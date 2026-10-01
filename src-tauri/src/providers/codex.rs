//! Adapter for the official OpenAI Codex CLI.
//!
//! Verified against `codex-cli 0.159.2` help output and the Codex non-interactive mode
//! documentation:
//!
//! - New session: `codex exec --json --skip-git-repo-check --sandbox <mode> -C <workspace> -`
//! - Follow-up:   `codex exec resume --json --skip-git-repo-check -c sandbox_mode="<mode>" <id> -`
//!   (`resume` accepts neither `--sandbox` nor `-C`, so the sandbox is set through the
//!   documented `sandbox_mode` config key and the workspace through the process directory.)
//! - The prompt is read from stdin (`-`) so it never appears in a command line.
//! - `--json` prints JSONL events: `thread.started` carries the session id, `item.*` events
//!   describe agent messages and actions, and `turn.failed` / `error` report failures.
//!
//! Detection runs `codex --version` and `codex login status`; their output is never logged.
//! Authentication stays with the CLI; Open Bots never reads Codex credentials.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use super::{
    AgentProvider, DetectionStatus, ProviderCapability, ProviderKind, ProviderSummary, TurnEvent,
    TurnEvents, TurnOutcome, TurnRequest,
};
use crate::{
    domain::agents::WorkspaceAccess,
    error::{AppError, AppResult},
    infrastructure::process::{LineCommand, LineProcessExit, LineProcessRunner},
    runtime::cancellation::CancellationSignal,
};

const PROGRAM: &str = "codex";
/// Longest stderr excerpt included in a user-visible error.
const MAX_ERROR_EXCERPT: usize = 300;

pub struct CodexProvider {
    runner: Arc<dyn LineProcessRunner>,
}

impl CodexProvider {
    pub fn new(runner: Arc<dyn LineProcessRunner>) -> Self {
        Self { runner }
    }

    async fn run_quiet(&self, arguments: &[&str]) -> AppResult<(LineProcessExit, Vec<String>)> {
        let mut lines = Vec::new();
        let exit = self
            .runner
            .run(
                LineCommand {
                    program: PROGRAM.into(),
                    arguments: arguments
                        .iter()
                        .map(|argument| (*argument).into())
                        .collect(),
                    working_directory: None,
                    stdin: None,
                },
                &mut |line| lines.push(line.to_owned()),
                CancellationSignal::never(),
            )
            .await?;
        Ok((exit, lines))
    }
}

#[async_trait]
impl AgentProvider for CodexProvider {
    fn id(&self) -> &'static str {
        "codex"
    }

    fn name(&self) -> &'static str {
        "OpenAI Codex CLI"
    }

    async fn detect(&self) -> AppResult<ProviderSummary> {
        let summary = |status, detail: String| ProviderSummary {
            id: self.id().into(),
            name: self.name().into(),
            kind: ProviderKind::Cli,
            status,
            detail,
            capabilities: self.capabilities(),
        };
        let (version_exit, version_lines) = self.run_quiet(&["--version"]).await?;
        let version = match version_exit {
            LineProcessExit::ProgramNotFound => {
                return Ok(summary(
                    DetectionStatus::NotInstalled,
                    "The `codex` command was not found on PATH.".into(),
                ));
            }
            LineProcessExit::Finished { success: true, .. } => version_lines
                .first()
                .map(|line| line.trim().to_owned())
                .unwrap_or_else(|| "codex".into()),
            _ => {
                return Ok(summary(
                    DetectionStatus::Unknown,
                    "`codex --version` did not succeed.".into(),
                ));
            }
        };
        let (login_exit, _) = self.run_quiet(&["login", "status"]).await?;
        Ok(match login_exit {
            LineProcessExit::Finished { success: true, .. } => summary(
                DetectionStatus::Available,
                format!("{version}. Signed in through the Codex CLI."),
            ),
            _ => summary(
                DetectionStatus::Unknown,
                format!("{version}. Sign-in could not be confirmed; run `codex login`."),
            ),
        })
    }

    fn capabilities(&self) -> Vec<ProviderCapability> {
        vec![
            ProviderCapability::Sessions,
            ProviderCapability::Resume,
            ProviderCapability::Shell,
        ]
    }

    async fn run_turn(
        &self,
        request: TurnRequest,
        events: TurnEvents,
        cancellation: CancellationSignal,
    ) -> AppResult<TurnOutcome> {
        let mut session_id = request.session_id.clone();
        let mut failure: Option<String> = None;
        let command = turn_command(&request);
        let exit = self
            .runner
            .run(
                command,
                &mut |line| match parse_event(line) {
                    Some(CodexEvent::Turn(event)) => {
                        if let TurnEvent::SessionStarted { session_id: id } = &event {
                            session_id = Some(id.clone());
                        }
                        let _ = events.send(event);
                    }
                    Some(CodexEvent::Failed(message)) => failure = Some(message),
                    None => {}
                },
                cancellation,
            )
            .await?;

        match exit {
            LineProcessExit::Cancelled => Ok(TurnOutcome {
                session_id,
                cancelled: true,
            }),
            LineProcessExit::ProgramNotFound => Err(AppError::Provider(
                "the `codex` command was not found on PATH".into(),
            )),
            LineProcessExit::Finished {
                success,
                code,
                stderr_tail,
            } => {
                if let Some(message) = failure {
                    return Err(AppError::Provider(format!("Codex turn failed: {message}")));
                }
                if !success {
                    let excerpt = error_excerpt(&stderr_tail);
                    let code = code.map_or_else(|| "a signal".into(), |code| code.to_string());
                    return Err(AppError::Provider(if excerpt.is_empty() {
                        format!("Codex exited with {code}")
                    } else {
                        format!("Codex exited with {code}: {excerpt}")
                    }));
                }
                Ok(TurnOutcome {
                    session_id,
                    cancelled: false,
                })
            }
        }
    }
}

fn sandbox_mode(access: WorkspaceAccess) -> &'static str {
    match access {
        WorkspaceAccess::ReadOnly => "read-only",
        WorkspaceAccess::WorkspaceWrite => "workspace-write",
    }
}

fn turn_command(request: &TurnRequest) -> LineCommand {
    let mode = sandbox_mode(request.access);
    let arguments: Vec<String> = match &request.session_id {
        None => vec![
            "exec".into(),
            "--json".into(),
            "--skip-git-repo-check".into(),
            "--sandbox".into(),
            mode.into(),
            "-C".into(),
            request.workspace.to_string_lossy().into_owned(),
            "-".into(),
        ],
        Some(session_id) => vec![
            "exec".into(),
            "resume".into(),
            "--json".into(),
            "--skip-git-repo-check".into(),
            "-c".into(),
            format!("sandbox_mode=\"{mode}\""),
            session_id.clone(),
            "-".into(),
        ],
    };
    LineCommand {
        program: PROGRAM.into(),
        arguments,
        working_directory: Some(request.workspace.clone()),
        stdin: Some(request.prompt.clone()),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum CodexEvent {
    Turn(TurnEvent),
    Failed(String),
}

/// Maps one JSONL line to a provider-neutral event. Unknown or internal events (such as
/// reasoning items) are ignored so newer CLI versions do not break the adapter.
fn parse_event(line: &str) -> Option<CodexEvent> {
    let value: Value = serde_json::from_str(line.trim()).ok()?;
    let text = |value: &Value, key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
    match value.get("type")?.as_str()? {
        "thread.started" => Some(CodexEvent::Turn(TurnEvent::SessionStarted {
            session_id: text(&value, "thread_id")?,
        })),
        "turn.failed" => Some(CodexEvent::Failed(
            value
                .get("error")
                .and_then(|error| text(error, "message"))
                .unwrap_or_else(|| "the turn failed".into()),
        )),
        "error" => Some(CodexEvent::Failed(
            text(&value, "message").unwrap_or_else(|| "the CLI reported an error".into()),
        )),
        kind @ ("item.started" | "item.completed") => {
            let item = value.get("item")?;
            let item_type = item.get("type")?.as_str()?;
            let id = text(item, "id").unwrap_or_default();
            match (kind, item_type) {
                (_, "reasoning") => None,
                ("item.completed", "agent_message") => Some(CodexEvent::Turn(TurnEvent::Message {
                    text: text(item, "text").filter(|text| !text.trim().is_empty())?,
                })),
                ("item.started", "agent_message") => None,
                ("item.started", _) => Some(CodexEvent::Turn(TurnEvent::ActionStarted {
                    id,
                    summary: action_summary(item, item_type),
                })),
                _ => Some(CodexEvent::Turn(TurnEvent::ActionCompleted {
                    id,
                    summary: action_summary(item, item_type),
                    succeeded: action_succeeded(item),
                })),
            }
        }
        _ => None,
    }
}

fn action_summary(item: &Value, item_type: &str) -> String {
    match item.get("command").and_then(Value::as_str) {
        Some(command) if item_type == "command_execution" => command.to_owned(),
        _ => item_type.replace('_', " "),
    }
}

fn action_succeeded(item: &Value) -> bool {
    let status_ok = item
        .get("status")
        .and_then(Value::as_str)
        .is_none_or(|status| status == "completed");
    let exit_ok = item
        .get("exit_code")
        .and_then(Value::as_i64)
        .is_none_or(|code| code == 0);
    status_ok && exit_ok
}

fn error_excerpt(stderr: &str) -> String {
    let line = stderr
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default();
    line.chars().take(MAX_ERROR_EXCERPT).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tokio::sync::mpsc;

    /// Replays scripted stdout lines and records every command it receives.
    struct ScriptedRunner {
        lines: Vec<&'static str>,
        exit: LineProcessExit,
        commands: Mutex<Vec<LineCommand>>,
    }

    impl ScriptedRunner {
        fn new(lines: Vec<&'static str>, exit: LineProcessExit) -> Arc<Self> {
            Arc::new(Self {
                lines,
                exit,
                commands: Mutex::new(Vec::new()),
            })
        }
    }

    #[async_trait]
    impl LineProcessRunner for ScriptedRunner {
        async fn run(
            &self,
            command: LineCommand,
            on_line: &mut (dyn for<'line> FnMut(&'line str) + Send),
            _cancellation: CancellationSignal,
        ) -> AppResult<LineProcessExit> {
            self.commands.lock().expect("commands").push(command);
            for line in &self.lines {
                on_line(line);
            }
            Ok(self.exit.clone())
        }
    }

    fn finished(success: bool) -> LineProcessExit {
        LineProcessExit::Finished {
            success,
            code: Some(if success { 0 } else { 1 }),
            stderr_tail: String::new(),
        }
    }

    fn request(session_id: Option<&str>, access: WorkspaceAccess) -> TurnRequest {
        TurnRequest {
            session_id: session_id.map(str::to_owned),
            prompt: "List the files".into(),
            workspace: "/work/project".into(),
            access,
        }
    }

    #[test]
    fn starts_new_sessions_with_sandbox_and_workspace() {
        let command = turn_command(&request(None, WorkspaceAccess::ReadOnly));
        assert_eq!(command.program, "codex");
        assert_eq!(
            command.arguments,
            [
                "exec",
                "--json",
                "--skip-git-repo-check",
                "--sandbox",
                "read-only",
                "-C",
                "/work/project",
                "-"
            ]
        );
        assert_eq!(command.stdin.as_deref(), Some("List the files"));
        assert_eq!(command.working_directory, Some("/work/project".into()));
    }

    #[test]
    fn resumes_sessions_through_the_config_override() {
        let command = turn_command(&request(Some("thread-1"), WorkspaceAccess::WorkspaceWrite));
        assert_eq!(
            command.arguments,
            [
                "exec",
                "resume",
                "--json",
                "--skip-git-repo-check",
                "-c",
                "sandbox_mode=\"workspace-write\"",
                "thread-1",
                "-"
            ]
        );
        assert!(!command
            .arguments
            .iter()
            .any(|argument| argument.contains("List")));
    }

    #[test]
    fn parses_the_documented_event_stream() {
        assert_eq!(
            parse_event(r#"{"type":"thread.started","thread_id":"0199a213"}"#),
            Some(CodexEvent::Turn(TurnEvent::SessionStarted {
                session_id: "0199a213".into()
            }))
        );
        assert_eq!(
            parse_event(
                r#"{"type":"item.started","item":{"id":"item_1","type":"command_execution","command":"bash -lc ls","status":"in_progress"}}"#
            ),
            Some(CodexEvent::Turn(TurnEvent::ActionStarted {
                id: "item_1".into(),
                summary: "bash -lc ls".into()
            }))
        );
        assert_eq!(
            parse_event(
                r#"{"type":"item.completed","item":{"id":"item_1","type":"command_execution","command":"bash -lc ls","status":"failed","exit_code":2}}"#
            ),
            Some(CodexEvent::Turn(TurnEvent::ActionCompleted {
                id: "item_1".into(),
                summary: "bash -lc ls".into(),
                succeeded: false
            }))
        );
        assert_eq!(
            parse_event(
                r#"{"type":"item.completed","item":{"id":"item_3","type":"agent_message","text":"Repo contains docs."}}"#
            ),
            Some(CodexEvent::Turn(TurnEvent::Message {
                text: "Repo contains docs.".into()
            }))
        );
        assert_eq!(
            parse_event(r#"{"type":"turn.failed","error":{"message":"usage limit reached"}}"#),
            Some(CodexEvent::Failed("usage limit reached".into()))
        );
    }

    #[test]
    fn ignores_internal_unknown_and_malformed_lines() {
        for line in [
            r#"{"type":"turn.started"}"#,
            r#"{"type":"turn.completed","usage":{"input_tokens":1}}"#,
            r#"{"type":"item.completed","item":{"id":"r","type":"reasoning","text":"thinking"}}"#,
            r#"{"type":"future.event"}"#,
            "not json",
            "",
        ] {
            assert_eq!(parse_event(line), None, "{line}");
        }
    }

    #[tokio::test]
    async fn runs_a_turn_and_reports_the_new_session() {
        let runner = ScriptedRunner::new(
            vec![
                r#"{"type":"thread.started","thread_id":"thread-9"}"#,
                r#"{"type":"turn.started"}"#,
                r#"{"type":"item.completed","item":{"id":"m","type":"agent_message","text":"Done."}}"#,
                r#"{"type":"turn.completed","usage":{}}"#,
            ],
            finished(true),
        );
        let provider = CodexProvider::new(runner.clone());
        let (sender, mut receiver) = mpsc::unbounded_channel();

        let outcome = provider
            .run_turn(
                request(None, WorkspaceAccess::ReadOnly),
                sender,
                CancellationSignal::never(),
            )
            .await
            .expect("turn");

        assert_eq!(
            outcome,
            TurnOutcome {
                session_id: Some("thread-9".into()),
                cancelled: false
            }
        );
        assert!(matches!(
            receiver.recv().await,
            Some(TurnEvent::SessionStarted { .. })
        ));
        assert_eq!(
            receiver.recv().await,
            Some(TurnEvent::Message {
                text: "Done.".into()
            })
        );
    }

    #[tokio::test]
    async fn reports_turn_failures_and_exit_errors() {
        let failed = ScriptedRunner::new(
            vec![r#"{"type":"turn.failed","error":{"message":"usage limit reached"}}"#],
            finished(false),
        );
        let result = CodexProvider::new(failed)
            .run_turn(
                request(None, WorkspaceAccess::ReadOnly),
                mpsc::unbounded_channel().0,
                CancellationSignal::never(),
            )
            .await;
        assert!(
            matches!(result, Err(AppError::Provider(message)) if message.contains("usage limit"))
        );

        let crashed = ScriptedRunner::new(
            Vec::new(),
            LineProcessExit::Finished {
                success: false,
                code: Some(2),
                stderr_tail: "warning\nerror: not logged in\n".into(),
            },
        );
        let result = CodexProvider::new(crashed)
            .run_turn(
                request(Some("thread-1"), WorkspaceAccess::ReadOnly),
                mpsc::unbounded_channel().0,
                CancellationSignal::never(),
            )
            .await;
        assert!(
            matches!(result, Err(AppError::Provider(message)) if message == "Codex exited with 2: error: not logged in")
        );
    }

    #[tokio::test]
    async fn reports_cancelled_turns() {
        let runner = ScriptedRunner::new(Vec::new(), LineProcessExit::Cancelled);
        let outcome = CodexProvider::new(runner)
            .run_turn(
                request(Some("thread-1"), WorkspaceAccess::ReadOnly),
                mpsc::unbounded_channel().0,
                CancellationSignal::never(),
            )
            .await
            .expect("turn");
        assert!(outcome.cancelled);
        assert_eq!(outcome.session_id.as_deref(), Some("thread-1"));
    }

    #[tokio::test]
    async fn detects_installation_and_sign_in() {
        let missing = ScriptedRunner::new(Vec::new(), LineProcessExit::ProgramNotFound);
        let summary = CodexProvider::new(missing).detect().await.expect("detect");
        assert_eq!(summary.status, DetectionStatus::NotInstalled);

        let installed = ScriptedRunner::new(vec!["codex-cli 0.159.2"], finished(true));
        let summary = CodexProvider::new(installed.clone())
            .detect()
            .await
            .expect("detect");
        assert_eq!(summary.status, DetectionStatus::Available);
        assert!(summary.detail.starts_with("codex-cli 0.159.2"));
        let commands = installed.commands.lock().expect("commands");
        assert_eq!(commands[0].arguments, ["--version"]);
        assert_eq!(commands[1].arguments, ["login", "status"]);
    }
}
