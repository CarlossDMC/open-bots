//! Shared plumbing for adapters that drive an official provider CLI.
//!
//! A CLI adapter owns a [`CliProgram`] and supplies only what is specific to its CLI: the
//! command line for a turn, a [`TurnStreamParser`] for its output, and the sign-in check used
//! by detection. Process execution, detection wording, the turn loop, cancellation, and error
//! mapping are shared so every CLI provider behaves the same way. See `docs/providers.md`.

use std::{path::PathBuf, sync::Arc};

use super::{
    AgentProvider, DetectionStatus, ProviderKind, ProviderSummary, TurnEvent, TurnEvents,
    TurnOutcome,
};
use crate::{
    error::{AppError, AppResult},
    infrastructure::process::{LineCommand, LineProcessExit, LineProcessRunner},
    runtime::cancellation::CancellationSignal,
};

/// Longest stderr excerpt included in a user-visible error.
const MAX_ERROR_EXCERPT: usize = 300;

/// One provider event decoded from a CLI output line.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum StreamEvent {
    Turn(TurnEvent),
    /// The CLI reported that the turn failed; the message is shown to the user.
    Failed(String),
}

/// Decodes a CLI's turn output, one stdout line at a time. Implementations must ignore
/// unknown or malformed lines so newer CLI versions degrade gracefully.
pub(super) trait TurnStreamParser: Send {
    fn parse_line(&mut self, line: &str) -> Vec<StreamEvent>;
}

/// Collected output of a short, non-interactive CLI invocation.
pub(super) struct CliOutput {
    pub exit: LineProcessExit,
    pub lines: Vec<String>,
}

impl CliOutput {
    pub fn succeeded(&self) -> bool {
        matches!(self.exit, LineProcessExit::Finished { success: true, .. })
    }
}

/// How detection confirms that the CLI is signed in. Output is inspected in memory only and
/// never logged or stored.
pub(super) struct SignInCheck<'a> {
    pub arguments: &'a [&'a str],
    pub confirmed: fn(&CliOutput) -> bool,
    /// Command suggested to the user when sign-in cannot be confirmed.
    pub login_hint: &'a str,
}

/// An official CLI program, run through process infrastructure.
pub(super) struct CliProgram {
    program: &'static str,
    /// Name used in user-visible messages, such as "Codex" or "Claude Code".
    display_name: &'static str,
    runner: Arc<dyn LineProcessRunner>,
}

impl CliProgram {
    pub fn new(
        program: &'static str,
        display_name: &'static str,
        runner: Arc<dyn LineProcessRunner>,
    ) -> Self {
        Self {
            program,
            display_name,
            runner,
        }
    }

    pub fn command(
        &self,
        arguments: Vec<String>,
        working_directory: Option<PathBuf>,
        stdin: Option<String>,
    ) -> LineCommand {
        LineCommand {
            program: self.program.into(),
            arguments,
            working_directory,
            stdin,
        }
    }

    /// Runs a short command to completion and collects its stdout lines.
    pub async fn run_quiet(&self, arguments: &[&str], stdin: Option<&str>) -> AppResult<CliOutput> {
        let mut lines = Vec::new();
        let command = self.command(
            arguments
                .iter()
                .map(|argument| (*argument).into())
                .collect(),
            None,
            stdin.map(str::to_owned),
        );
        let exit = self
            .runner
            .run(
                command,
                &mut |line| lines.push(line.to_owned()),
                CancellationSignal::never(),
            )
            .await?;
        Ok(CliOutput { exit, lines })
    }

    /// Standard detection: `<program> --version`, then the provider's sign-in check.
    pub async fn detect(
        &self,
        provider: &dyn AgentProvider,
        sign_in: SignInCheck<'_>,
    ) -> AppResult<ProviderSummary> {
        let summary = |status, detail: String| ProviderSummary {
            id: provider.id().into(),
            name: provider.name().into(),
            kind: ProviderKind::Cli,
            status,
            detail,
            capabilities: provider.capabilities(),
        };
        let version = self.run_quiet(&["--version"], None).await?;
        let version = match version.exit {
            LineProcessExit::ProgramNotFound => {
                return Ok(summary(
                    DetectionStatus::NotInstalled,
                    format!("The `{}` command was not found on PATH.", self.program),
                ));
            }
            LineProcessExit::Finished { success: true, .. } => version
                .lines
                .first()
                .map(|line| line.trim().to_owned())
                .unwrap_or_else(|| self.program.into()),
            _ => {
                return Ok(summary(
                    DetectionStatus::Unknown,
                    format!("`{} --version` did not succeed.", self.program),
                ));
            }
        };
        let output = self.run_quiet(sign_in.arguments, None).await?;
        Ok(if (sign_in.confirmed)(&output) {
            summary(
                DetectionStatus::Available,
                format!(
                    "{version}. Signed in through the {} CLI.",
                    self.display_name
                ),
            )
        } else {
            summary(
                DetectionStatus::Unknown,
                format!(
                    "{version}. Sign-in could not be confirmed; run `{}`.",
                    sign_in.login_hint
                ),
            )
        })
    }

    /// Standard turn loop: streams `command` through `parser`, forwards turn events, tracks
    /// the session id, and maps cancellation, failures, and exit codes to a turn outcome.
    pub async fn run_turn(
        &self,
        command: LineCommand,
        session_id: Option<String>,
        parser: &mut dyn TurnStreamParser,
        events: &TurnEvents,
        cancellation: CancellationSignal,
    ) -> AppResult<TurnOutcome> {
        let mut session_id = session_id;
        let mut failure: Option<String> = None;
        let exit = self
            .runner
            .run(
                command,
                &mut |line| {
                    for event in parser.parse_line(line) {
                        match event {
                            StreamEvent::Turn(event) => {
                                if let TurnEvent::SessionStarted { session_id: id } = &event {
                                    session_id = Some(id.clone());
                                }
                                let _ = events.send(event);
                            }
                            StreamEvent::Failed(message) => failure = Some(message),
                        }
                    }
                },
                cancellation,
            )
            .await?;

        match exit {
            LineProcessExit::Cancelled => Ok(TurnOutcome {
                session_id,
                cancelled: true,
            }),
            LineProcessExit::ProgramNotFound => Err(self.not_found()),
            LineProcessExit::Finished {
                success,
                code,
                stderr_tail,
            } => {
                if let Some(message) = failure {
                    return Err(AppError::Provider(format!(
                        "{} turn failed: {message}",
                        self.display_name
                    )));
                }
                if !success {
                    return Err(self.exit_error(self.display_name, code, &stderr_tail));
                }
                Ok(TurnOutcome {
                    session_id,
                    cancelled: false,
                })
            }
        }
    }

    pub fn not_found(&self) -> AppError {
        AppError::Provider(format!(
            "the `{}` command was not found on PATH",
            self.program
        ))
    }

    /// Error for a non-zero exit, such as "Codex app-server exited with 2: <stderr line>".
    pub fn exit_error(&self, subject: &str, code: Option<i32>, stderr_tail: &str) -> AppError {
        let excerpt = error_excerpt(stderr_tail);
        let code = code.map_or_else(|| "a signal".into(), |code| code.to_string());
        AppError::Provider(if excerpt.is_empty() {
            format!("{subject} exited with {code}")
        } else {
            format!("{subject} exited with {code}: {excerpt}")
        })
    }
}

/// Last non-empty stderr line, truncated, for user-visible CLI errors.
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

    #[test]
    fn excerpts_the_last_non_empty_stderr_line() {
        assert_eq!(error_excerpt("warning\nerror: denied\n\n"), "error: denied");
        assert_eq!(error_excerpt(""), "");
        assert_eq!(error_excerpt(&"x".repeat(500)).len(), MAX_ERROR_EXCERPT);
    }
}
