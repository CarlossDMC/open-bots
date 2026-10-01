use std::{path::PathBuf, process::Stdio};

use async_trait::async_trait;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::Command,
};

use crate::{
    error::{AppError, AppResult},
    runtime::cancellation::CancellationSignal,
};

/// Bytes of stderr kept for error reporting.
const STDERR_TAIL_BYTES: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineCommand {
    pub program: String,
    pub arguments: Vec<String>,
    pub working_directory: Option<PathBuf>,
    /// Written to stdin, which is then closed. Keeps prompts out of the command line.
    pub stdin: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineProcessExit {
    Finished {
        success: bool,
        code: Option<i32>,
        stderr_tail: String,
    },
    Cancelled,
    ProgramNotFound,
}

/// Runs a process and hands each stdout line to `on_line` as it arrives.
#[async_trait]
pub trait LineProcessRunner: Send + Sync {
    async fn run(
        &self,
        command: LineCommand,
        on_line: &mut (dyn for<'line> FnMut(&'line str) + Send),
        cancellation: CancellationSignal,
    ) -> AppResult<LineProcessExit>;
}

#[derive(Debug, Default)]
pub struct TokioLineProcessRunner;

#[async_trait]
impl LineProcessRunner for TokioLineProcessRunner {
    async fn run(
        &self,
        command: LineCommand,
        on_line: &mut (dyn for<'line> FnMut(&'line str) + Send),
        mut cancellation: CancellationSignal,
    ) -> AppResult<LineProcessExit> {
        if let Some(directory) = &command.working_directory {
            if !directory.is_dir() {
                return Err(AppError::Process(format!(
                    "working directory {} does not exist",
                    directory.display()
                )));
            }
        }
        let mut process = Command::new(&command.program);
        process
            .args(&command.arguments)
            .stdin(if command.stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if let Some(directory) = &command.working_directory {
            process.current_dir(directory);
        }
        let mut child = match process.spawn() {
            Ok(child) => child,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(LineProcessExit::ProgramNotFound);
            }
            Err(error) => {
                return Err(AppError::Process(format!(
                    "{} could not be started: {error}",
                    command.program
                )));
            }
        };

        if let (Some(input), Some(mut stdin)) = (command.stdin, child.stdin.take()) {
            stdin
                .write_all(input.as_bytes())
                .await
                .map_err(|error| AppError::Process(format!("stdin write failed: {error}")))?;
            drop(stdin);
        }
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AppError::Process("stdout is unavailable".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| AppError::Process("stderr is unavailable".into()))?;
        let stderr_task = tokio::spawn(read_tail(stderr));

        let mut lines = BufReader::new(stdout).lines();
        loop {
            tokio::select! {
                line = lines.next_line() => match line {
                    Ok(Some(line)) => on_line(&line),
                    Ok(None) => break,
                    Err(error) => {
                        return Err(AppError::Process(format!("stdout read failed: {error}")));
                    }
                },
                () = cancellation.cancelled() => {
                    if let Err(error) = child.kill().await {
                        tracing::warn!(%error, program = %command.program, "cancelled process could not be killed");
                    }
                    stderr_task.abort();
                    return Ok(LineProcessExit::Cancelled);
                }
            }
        }

        let status = child
            .wait()
            .await
            .map_err(|error| AppError::Process(format!("process wait failed: {error}")))?;
        let stderr_tail = stderr_task.await.unwrap_or_default();
        Ok(LineProcessExit::Finished {
            success: status.success(),
            code: status.code(),
            stderr_tail,
        })
    }
}

async fn read_tail(mut stderr: tokio::process::ChildStderr) -> String {
    let mut tail = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        match stderr.read(&mut buffer).await {
            Ok(0) | Err(_) => break,
            Ok(read) => {
                tail.extend_from_slice(&buffer[..read]);
                if tail.len() > STDERR_TAIL_BYTES {
                    tail.drain(..tail.len() - STDERR_TAIL_BYTES);
                }
            }
        }
    }
    String::from_utf8_lossy(&tail).into_owned()
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::runtime::cancellation::cancellation_pair;
    use std::time::Duration;

    fn shell(script: &str, stdin: Option<&str>) -> LineCommand {
        LineCommand {
            program: "sh".into(),
            arguments: vec!["-c".into(), script.into()],
            working_directory: None,
            stdin: stdin.map(str::to_owned),
        }
    }

    #[tokio::test]
    async fn streams_stdout_lines_and_reports_exit() {
        let mut lines = Vec::new();
        let exit = TokioLineProcessRunner
            .run(
                shell(
                    "cat; echo done; echo oops >&2; exit 3",
                    Some("first\nsecond\n"),
                ),
                &mut |line| lines.push(line.to_owned()),
                CancellationSignal::never(),
            )
            .await
            .expect("run");
        assert_eq!(lines, ["first", "second", "done"]);
        assert_eq!(
            exit,
            LineProcessExit::Finished {
                success: false,
                code: Some(3),
                stderr_tail: "oops\n".into()
            }
        );
    }

    #[tokio::test]
    async fn kills_the_process_when_cancelled() {
        let (canceller, signal) = cancellation_pair();
        let run = tokio::spawn(async move {
            TokioLineProcessRunner
                .run(shell("echo started; sleep 30", None), &mut |_| {}, signal)
                .await
        });
        tokio::time::sleep(Duration::from_millis(100)).await;
        canceller.cancel();
        let exit = tokio::time::timeout(Duration::from_secs(5), run)
            .await
            .expect("finished promptly")
            .expect("join")
            .expect("run");
        assert_eq!(exit, LineProcessExit::Cancelled);
    }

    #[tokio::test]
    async fn rejects_missing_working_directories() {
        let mut command = shell("true", None);
        command.working_directory = Some("/open-bots/missing/directory".into());
        let result = TokioLineProcessRunner
            .run(command, &mut |_| {}, CancellationSignal::never())
            .await;
        assert!(
            matches!(result, Err(AppError::Process(message)) if message.contains("does not exist"))
        );
    }

    #[tokio::test]
    async fn reports_missing_programs() {
        let exit = TokioLineProcessRunner
            .run(
                LineCommand {
                    program: "open-bots-missing-program".into(),
                    arguments: Vec::new(),
                    working_directory: None,
                    stdin: None,
                },
                &mut |_| {},
                CancellationSignal::never(),
            )
            .await
            .expect("run");
        assert_eq!(exit, LineProcessExit::ProgramNotFound);
    }
}
