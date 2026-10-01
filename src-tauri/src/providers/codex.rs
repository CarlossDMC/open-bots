//! Adapter for the official OpenAI Codex CLI.
//!
//! Verified against `codex-cli 0.159.2` help output and the Codex non-interactive mode
//! documentation:
//!
//! - New session: `codex exec --json --skip-git-repo-check --sandbox <mode> -C <workspace> -`
//! - Follow-up:   `codex exec resume --json --skip-git-repo-check -c sandbox_mode="<mode>" <id> -`
//!   (`resume` accepts neither `--sandbox` nor `-C`, so the sandbox is set through the
//!   documented `sandbox_mode` config key and the workspace through the process directory.)
//! - Model: `-m <model>` on both commands; reasoning effort through the documented
//!   `model_reasoning_effort` config key (`-c model_reasoning_effort="<effort>"`).
//! - Internet access: `-c web_search="live"` turns on the native web search tool (the
//!   interactive `--search` flag is not accepted by `exec`), and in the workspace-write
//!   sandbox `-c sandbox_workspace_write.network_access=true` lets commands reach the
//!   network. The read-only sandbox keeps commands offline; web search still works there.
//!   Both keys were checked against `codex-cli 0.159.3`.
//! - The prompt is read from stdin (`-`) so it never appears in a command line.
//! - Runtime tools: `-c mcp_servers.open_bots.url="<url>"`,
//!   `-c mcp_servers.open_bots.bearer_token_env_var="OPEN_BOTS_MCP_TOKEN"` and
//!   `-c mcp_servers.open_bots.default_tools_approval_mode="approve"` on new and resumed
//!   turns (`codex -c … mcp list` shows the server enabled with bearer auth). `exec` never
//!   prompts, so without a pre-approval mode its MCP calls would be rejected.
//! - `--json` prints JSONL events: `thread.started` carries the session id, `item.*` events
//!   describe agent messages and actions, and `turn.failed` / `error` report failures.
//!
//! Models and usage limits come from `codex app-server` (stdio JSON-RPC, marked experimental by
//! the CLI): `initialize`, the `initialized` notification, then `model/list` or
//! `account/rateLimits/read`. The server is started per read and stopped afterwards; turns
//! never go through it. Parsing tolerates missing fields so newer CLI versions degrade to an
//! error instead of wrong numbers.
//!
//! Detection runs `codex --version` and `codex login status`; their output is never logged.
//! Authentication stays with the CLI; Open Bots never reads Codex credentials.

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};

use super::{
    cli::{CliOutput, CliProgram, SignInCheck, StreamEvent, TurnStreamParser},
    AgentProvider, ProviderCapability, ProviderModel, ProviderSummary, ProviderUsage, TurnEvent,
    TurnEvents, TurnOutcome, TurnRequest, UsageWindow, RUNTIME_TOOLS_SERVER_NAME,
    RUNTIME_TOOLS_TOKEN_ENV,
};
use crate::{
    domain::agents::WorkspaceAccess,
    error::{AppError, AppResult},
    infrastructure::process::{
        JsonRpcError, JsonRpcExit, JsonRpcMessage, JsonRpcProcessClient, JsonRpcSession,
        LineCommand, LineProcessRunner, ProcessEnvironment,
    },
    runtime::cancellation::CancellationSignal,
};

const PROGRAM: &str = "codex";
/// How long one app-server read may take, including server start-up.
const APP_SERVER_TIMEOUT: Duration = Duration::from_secs(15);
/// Upper bound on `model/list` pages, so a misbehaving cursor cannot loop forever.
const MAX_MODEL_PAGES: usize = 5;

pub struct CodexProvider {
    cli: CliProgram,
    rpc: Arc<dyn JsonRpcProcessClient>,
}

impl CodexProvider {
    pub fn new(runner: Arc<dyn LineProcessRunner>, rpc: Arc<dyn JsonRpcProcessClient>) -> Self {
        Self {
            cli: CliProgram::new(PROGRAM, "Codex", runner),
            rpc,
        }
    }

    /// Sends one request to a fresh `codex app-server` and returns its result.
    async fn app_server_request(&self, method: &str, params: Option<Value>) -> AppResult<Value> {
        let exit = self
            .rpc
            .exchange(JsonRpcSession {
                program: PROGRAM.into(),
                arguments: vec!["app-server".into()],
                messages: vec![
                    JsonRpcMessage::Request {
                        method: "initialize".into(),
                        params: Some(json!({
                            "clientInfo": {
                                "name": "open-bots",
                                "title": "Open Bots",
                                "version": env!("CARGO_PKG_VERSION"),
                            }
                        })),
                    },
                    JsonRpcMessage::Notification {
                        method: "initialized".into(),
                        params: None,
                    },
                    JsonRpcMessage::Request {
                        method: method.into(),
                        params,
                    },
                ],
                timeout: APP_SERVER_TIMEOUT,
            })
            .await?;
        match exit {
            JsonRpcExit::Completed(responses) => {
                let mut responses = responses.into_iter();
                let failure = |error: JsonRpcError| {
                    AppError::Provider(format!(
                        "Codex app-server rejected `{method}`: {}",
                        error.message
                    ))
                };
                responses
                    .next()
                    .ok_or_else(|| {
                        AppError::Provider("Codex app-server did not initialize".into())
                    })?
                    .map_err(failure)?;
                responses
                    .next()
                    .ok_or_else(|| {
                        AppError::Provider(format!("Codex app-server did not answer `{method}`"))
                    })?
                    .map_err(failure)
            }
            JsonRpcExit::ProgramNotFound => Err(self.cli.not_found()),
            JsonRpcExit::TimedOut => Err(AppError::Provider(format!(
                "Codex app-server did not answer within {} seconds",
                APP_SERVER_TIMEOUT.as_secs()
            ))),
            JsonRpcExit::Exited { code, stderr_tail } => {
                Err(self.cli.exit_error("Codex app-server", code, &stderr_tail))
            }
        }
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
        self.cli
            .detect(
                self,
                SignInCheck {
                    arguments: &["login", "status"],
                    confirmed: CliOutput::succeeded,
                    login_hint: "codex login",
                },
            )
            .await
    }

    fn capabilities(&self) -> Vec<ProviderCapability> {
        vec![
            ProviderCapability::Sessions,
            ProviderCapability::Resume,
            ProviderCapability::Shell,
            ProviderCapability::ModelSelection,
            ProviderCapability::UsageLimits,
            ProviderCapability::RuntimeTools,
        ]
    }

    async fn list_models(&self) -> AppResult<Vec<ProviderModel>> {
        let mut models = Vec::new();
        let mut cursor: Option<String> = None;
        for _ in 0..MAX_MODEL_PAGES {
            let page = self
                .app_server_request(
                    "model/list",
                    Some(json!({ "includeHidden": false, "cursor": cursor })),
                )
                .await?;
            let (page_models, next) = parse_model_page(&page)?;
            models.extend(page_models);
            match next {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }
        Ok(models)
    }

    async fn read_usage(&self) -> AppResult<ProviderUsage> {
        let result = self
            .app_server_request("account/rateLimits/read", None)
            .await?;
        parse_usage(self.id(), &result, Utc::now())
    }

    async fn run_turn(
        &self,
        request: TurnRequest,
        events: TurnEvents,
        cancellation: CancellationSignal,
    ) -> AppResult<TurnOutcome> {
        self.cli
            .run_turn(
                turn_command(&request),
                request.session_id.clone(),
                &mut CodexStreamParser,
                &events,
                cancellation,
            )
            .await
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
    let mut arguments: Vec<String> = match &request.session_id {
        None => vec![
            "exec".into(),
            "--json".into(),
            "--skip-git-repo-check".into(),
            "--sandbox".into(),
            mode.into(),
        ],
        Some(_) => vec![
            "exec".into(),
            "resume".into(),
            "--json".into(),
            "--skip-git-repo-check".into(),
            "-c".into(),
            format!("sandbox_mode=\"{mode}\""),
        ],
    };
    // Identifiers are validated by the domain, so they cannot break out of the TOML string.
    if let Some(model) = &request.model {
        arguments.extend(["-m".into(), model.clone()]);
    }
    if let Some(effort) = &request.reasoning_effort {
        arguments.extend(["-c".into(), format!("model_reasoning_effort=\"{effort}\"")]);
    }
    if request.network {
        arguments.extend(["-c".into(), "web_search=\"live\"".into()]);
        if request.access == WorkspaceAccess::WorkspaceWrite {
            arguments.extend([
                "-c".into(),
                "sandbox_workspace_write.network_access=true".into(),
            ]);
        }
    }
    match &request.session_id {
        None => arguments.extend([
            "-C".into(),
            request.workspace.to_string_lossy().into_owned(),
        ]),
        Some(session_id) => arguments.push(session_id.clone()),
    }
    let mut environment = ProcessEnvironment::default();
    if let Some(endpoint) = &request.runtime_tools {
        // `exec` has no approval prompt, so the server's tools are pre-approved; the URL is
        // a loopback address built by the runtime and the token travels in the environment.
        let server = format!("mcp_servers.{RUNTIME_TOOLS_SERVER_NAME}");
        arguments.extend([
            "-c".into(),
            format!("{server}.url=\"{}\"", endpoint.url),
            "-c".into(),
            format!("{server}.bearer_token_env_var=\"{RUNTIME_TOOLS_TOKEN_ENV}\""),
            "-c".into(),
            format!("{server}.default_tools_approval_mode=\"approve\""),
        ]);
        environment.set(RUNTIME_TOOLS_TOKEN_ENV, endpoint.token.clone());
    }
    arguments.push("-".into());
    LineCommand {
        environment,
        program: PROGRAM.into(),
        arguments,
        working_directory: Some(request.workspace.clone()),
        stdin: Some(request.prompt.clone()),
    }
}

/// Codex events are self-contained, so the parser keeps no state between lines.
struct CodexStreamParser;

impl TurnStreamParser for CodexStreamParser {
    fn parse_line(&mut self, line: &str) -> Vec<StreamEvent> {
        parse_event(line).into_iter().collect()
    }
}

/// Maps one JSONL line to a provider-neutral event. Unknown or internal events (such as
/// reasoning items) are ignored so newer CLI versions do not break the adapter.
fn parse_event(line: &str) -> Option<StreamEvent> {
    let value: Value = serde_json::from_str(line.trim()).ok()?;
    let text = |value: &Value, key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
    match value.get("type")?.as_str()? {
        "thread.started" => Some(StreamEvent::Turn(TurnEvent::SessionStarted {
            session_id: text(&value, "thread_id")?,
        })),
        "turn.failed" => Some(StreamEvent::Failed(
            value
                .get("error")
                .and_then(|error| text(error, "message"))
                .unwrap_or_else(|| "the turn failed".into()),
        )),
        "error" => Some(StreamEvent::Failed(
            text(&value, "message").unwrap_or_else(|| "the CLI reported an error".into()),
        )),
        kind @ ("item.started" | "item.completed") => {
            let item = value.get("item")?;
            let item_type = item.get("type")?.as_str()?;
            let id = text(item, "id").unwrap_or_default();
            match (kind, item_type) {
                (_, "reasoning") => None,
                ("item.completed", "agent_message") => {
                    Some(StreamEvent::Turn(TurnEvent::Message {
                        text: text(item, "text").filter(|text| !text.trim().is_empty())?,
                    }))
                }
                ("item.started", "agent_message") => None,
                ("item.started", _) => Some(StreamEvent::Turn(TurnEvent::ActionStarted {
                    id,
                    summary: action_summary(item, item_type),
                })),
                _ => Some(StreamEvent::Turn(TurnEvent::ActionCompleted {
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

/// Maps one `model/list` page to visible models and the next cursor.
fn parse_model_page(page: &Value) -> AppResult<(Vec<ProviderModel>, Option<String>)> {
    let entries = page
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::Provider("Codex returned an unexpected model list".into()))?;
    let text = |value: &Value, key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
    let models = entries
        .iter()
        .filter(|entry| {
            !entry
                .get("hidden")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .filter_map(|entry| {
            let id = text(entry, "model").or_else(|| text(entry, "id"))?;
            Some(ProviderModel {
                display_name: text(entry, "displayName").unwrap_or_else(|| id.clone()),
                description: text(entry, "description").unwrap_or_default(),
                is_default: entry
                    .get("isDefault")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                reasoning_efforts: entry
                    .get("supportedReasoningEfforts")
                    .and_then(Value::as_array)
                    .map(|efforts| {
                        efforts
                            .iter()
                            .filter_map(|effort| text(effort, "reasoningEffort"))
                            .collect()
                    })
                    .unwrap_or_default(),
                default_reasoning_effort: text(entry, "defaultReasoningEffort"),
                id,
            })
        })
        .collect();
    Ok((models, text(page, "nextCursor")))
}

/// Maps `account/rateLimits/read` to provider-neutral usage windows.
fn parse_usage(
    provider_id: &str,
    result: &Value,
    checked_at: DateTime<Utc>,
) -> AppResult<ProviderUsage> {
    let snapshot = result
        .get("rateLimits")
        .filter(|snapshot| snapshot.is_object())
        .ok_or_else(|| AppError::Provider("Codex did not report usage limits".into()))?;
    let windows = ["primary", "secondary"]
        .into_iter()
        .filter_map(|key| snapshot.get(key).filter(|window| window.is_object()))
        .filter_map(|window| {
            let used = window.get("usedPercent").and_then(Value::as_i64)?;
            Some(UsageWindow {
                duration_minutes: window.get("windowDurationMins").and_then(Value::as_i64),
                scope: None,
                used_percent: u8::try_from(used.clamp(0, 100)).unwrap_or(100),
                resets_at: window
                    .get("resetsAt")
                    .and_then(Value::as_i64)
                    .and_then(|seconds| DateTime::from_timestamp(seconds, 0)),
            })
        })
        .collect();
    let reached = |value: Option<&Value>| value.is_some_and(|value| !value.is_null());
    Ok(ProviderUsage {
        provider_id: provider_id.into(),
        plan: snapshot
            .get("planType")
            .and_then(Value::as_str)
            .map(str::to_owned),
        windows,
        limit_reached: reached(snapshot.get("rateLimitReachedType"))
            || result.get("ordinaryUsageAllowed").and_then(Value::as_bool) == Some(false),
        checked_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        infrastructure::process::LineProcessExit,
        providers::{
            testing::{finished, ScriptedRunner},
            DetectionStatus,
        },
    };
    use std::sync::Mutex;
    use tokio::sync::mpsc;

    /// Returns one scripted app-server exit and records every session it receives.
    struct ScriptedRpc {
        exit: JsonRpcExit,
        sessions: Mutex<Vec<JsonRpcSession>>,
    }

    impl ScriptedRpc {
        fn new(exit: JsonRpcExit) -> Arc<Self> {
            Arc::new(Self {
                exit,
                sessions: Mutex::new(Vec::new()),
            })
        }
    }

    #[async_trait]
    impl JsonRpcProcessClient for ScriptedRpc {
        async fn exchange(&self, session: JsonRpcSession) -> AppResult<JsonRpcExit> {
            self.sessions.lock().expect("sessions").push(session);
            Ok(self.exit.clone())
        }
    }

    fn codex(runner: Arc<ScriptedRunner>) -> CodexProvider {
        CodexProvider::new(runner, ScriptedRpc::new(JsonRpcExit::TimedOut))
    }

    fn codex_rpc(rpc: Arc<ScriptedRpc>) -> CodexProvider {
        CodexProvider::new(ScriptedRunner::single(Vec::new(), finished(true)), rpc)
    }

    fn answered(result: Value) -> JsonRpcExit {
        JsonRpcExit::Completed(vec![Ok(json!({})), Ok(result)])
    }

    #[test]
    fn registers_runtime_tools_on_new_and_resumed_turns() {
        for session in [None, Some("thread-1")] {
            let mut with_tools = request(session, WorkspaceAccess::ReadOnly);
            with_tools.runtime_tools = Some(crate::providers::RuntimeToolsEndpoint {
                url: "http://127.0.0.1:4100/mcp".into(),
                token: "secret-token".into(),
            });
            let command = turn_command(&with_tools);
            let arguments = command.arguments.join(" ");
            assert!(
                arguments.contains("-c mcp_servers.open_bots.url=\"http://127.0.0.1:4100/mcp\"")
            );
            assert!(arguments
                .contains("-c mcp_servers.open_bots.bearer_token_env_var=\"OPEN_BOTS_MCP_TOKEN\""));
            assert!(arguments
                .contains("-c mcp_servers.open_bots.default_tools_approval_mode=\"approve\""));
            assert!(!arguments.contains("secret-token"));
            assert_eq!(command.arguments.last().map(String::as_str), Some("-"));
            assert_eq!(
                command.environment.get("OPEN_BOTS_MCP_TOKEN"),
                Some("secret-token")
            );
        }
    }

    fn request(session_id: Option<&str>, access: WorkspaceAccess) -> TurnRequest {
        TurnRequest {
            session_id: session_id.map(str::to_owned),
            prompt: "List the files".into(),
            workspace: "/work/project".into(),
            access,
            network: false,
            model: None,
            reasoning_effort: None,
            runtime_tools: None,
            mcp_servers: Vec::new(),
        }
    }

    #[test]
    fn enables_web_search_and_sandbox_network_with_internet_access() {
        let mut writable = request(None, WorkspaceAccess::WorkspaceWrite);
        writable.network = true;
        let arguments = turn_command(&writable).arguments.join(" ");
        assert!(arguments.contains("-c web_search=\"live\""), "{arguments}");
        assert!(arguments.contains("-c sandbox_workspace_write.network_access=true"));

        let mut read_only = request(None, WorkspaceAccess::ReadOnly);
        read_only.network = true;
        let arguments = turn_command(&read_only).arguments.join(" ");
        assert!(arguments.contains("web_search"));
        assert!(!arguments.contains("network_access"));
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
            Some(StreamEvent::Turn(TurnEvent::SessionStarted {
                session_id: "0199a213".into()
            }))
        );
        assert_eq!(
            parse_event(
                r#"{"type":"item.started","item":{"id":"item_1","type":"command_execution","command":"bash -lc ls","status":"in_progress"}}"#
            ),
            Some(StreamEvent::Turn(TurnEvent::ActionStarted {
                id: "item_1".into(),
                summary: "bash -lc ls".into()
            }))
        );
        assert_eq!(
            parse_event(
                r#"{"type":"item.completed","item":{"id":"item_1","type":"command_execution","command":"bash -lc ls","status":"failed","exit_code":2}}"#
            ),
            Some(StreamEvent::Turn(TurnEvent::ActionCompleted {
                id: "item_1".into(),
                summary: "bash -lc ls".into(),
                succeeded: false
            }))
        );
        assert_eq!(
            parse_event(
                r#"{"type":"item.completed","item":{"id":"item_3","type":"agent_message","text":"Repo contains docs."}}"#
            ),
            Some(StreamEvent::Turn(TurnEvent::Message {
                text: "Repo contains docs.".into()
            }))
        );
        assert_eq!(
            parse_event(r#"{"type":"turn.failed","error":{"message":"usage limit reached"}}"#),
            Some(StreamEvent::Failed("usage limit reached".into()))
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
        let runner = ScriptedRunner::single(
            vec![
                r#"{"type":"thread.started","thread_id":"thread-9"}"#,
                r#"{"type":"turn.started"}"#,
                r#"{"type":"item.completed","item":{"id":"m","type":"agent_message","text":"Done."}}"#,
                r#"{"type":"turn.completed","usage":{}}"#,
            ],
            finished(true),
        );
        let provider = codex(runner.clone());
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
        let failed = ScriptedRunner::single(
            vec![r#"{"type":"turn.failed","error":{"message":"usage limit reached"}}"#],
            finished(false),
        );
        let result = codex(failed)
            .run_turn(
                request(None, WorkspaceAccess::ReadOnly),
                mpsc::unbounded_channel().0,
                CancellationSignal::never(),
            )
            .await;
        assert!(
            matches!(result, Err(AppError::Provider(message)) if message.contains("usage limit"))
        );

        let crashed = ScriptedRunner::single(
            Vec::new(),
            LineProcessExit::Finished {
                success: false,
                code: Some(2),
                stderr_tail: "warning\nerror: not logged in\n".into(),
            },
        );
        let result = codex(crashed)
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
        let runner = ScriptedRunner::single(Vec::new(), LineProcessExit::Cancelled);
        let outcome = codex(runner)
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
        let missing = ScriptedRunner::single(Vec::new(), LineProcessExit::ProgramNotFound);
        let summary = codex(missing).detect().await.expect("detect");
        assert_eq!(summary.status, DetectionStatus::NotInstalled);

        let installed = ScriptedRunner::single(vec!["codex-cli 0.159.2"], finished(true));
        let summary = codex(installed.clone()).detect().await.expect("detect");
        assert_eq!(summary.status, DetectionStatus::Available);
        assert!(summary.detail.starts_with("codex-cli 0.159.2"));
        let commands = installed.commands.lock().expect("commands");
        assert_eq!(commands[0].arguments, ["--version"]);
        assert_eq!(commands[1].arguments, ["login", "status"]);
    }

    #[test]
    fn passes_the_selected_model_to_new_and_resumed_sessions() {
        let mut new_session = request(None, WorkspaceAccess::ReadOnly);
        new_session.model = Some("gpt-5.5".into());
        new_session.reasoning_effort = Some("high".into());
        assert_eq!(
            turn_command(&new_session).arguments,
            [
                "exec",
                "--json",
                "--skip-git-repo-check",
                "--sandbox",
                "read-only",
                "-m",
                "gpt-5.5",
                "-c",
                "model_reasoning_effort=\"high\"",
                "-C",
                "/work/project",
                "-"
            ]
        );

        let mut resumed = request(Some("thread-1"), WorkspaceAccess::ReadOnly);
        resumed.model = Some("gpt-5.5".into());
        assert_eq!(
            turn_command(&resumed).arguments,
            [
                "exec",
                "resume",
                "--json",
                "--skip-git-repo-check",
                "-c",
                "sandbox_mode=\"read-only\"",
                "-m",
                "gpt-5.5",
                "thread-1",
                "-"
            ]
        );
    }

    #[tokio::test]
    async fn lists_visible_models_from_the_app_server() {
        let rpc = ScriptedRpc::new(answered(json!({
            "data": [
                {
                    "id": "gpt-5.5",
                    "model": "gpt-5.5",
                    "displayName": "GPT-5.5",
                    "description": "Legacy coding model.",
                    "isDefault": true,
                    "hidden": false,
                    "supportedReasoningEfforts": [
                        { "reasoningEffort": "low", "description": "Fast" },
                        { "reasoningEffort": "high", "description": "Deep" }
                    ],
                    "defaultReasoningEffort": "low"
                },
                { "id": "internal", "model": "internal", "hidden": true },
                { "id": "bare" }
            ],
            "nextCursor": null
        })));
        let models = codex_rpc(rpc.clone()).list_models().await.expect("models");
        assert_eq!(
            models,
            [
                ProviderModel {
                    id: "gpt-5.5".into(),
                    display_name: "GPT-5.5".into(),
                    description: "Legacy coding model.".into(),
                    is_default: true,
                    reasoning_efforts: vec!["low".into(), "high".into()],
                    default_reasoning_effort: Some("low".into()),
                },
                ProviderModel {
                    id: "bare".into(),
                    display_name: "bare".into(),
                    description: String::new(),
                    is_default: false,
                    reasoning_efforts: Vec::new(),
                    default_reasoning_effort: None,
                }
            ]
        );
        let sessions = rpc.sessions.lock().expect("sessions");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].program, "codex");
        assert_eq!(sessions[0].arguments, ["app-server"]);
        let methods: Vec<_> = sessions[0]
            .messages
            .iter()
            .map(|message| match message {
                JsonRpcMessage::Request { method, .. } => method.as_str(),
                JsonRpcMessage::Notification { method, .. } => method.as_str(),
            })
            .collect();
        assert_eq!(methods, ["initialize", "initialized", "model/list"]);
    }

    #[tokio::test]
    async fn reads_usage_windows_from_the_app_server() {
        let rpc = ScriptedRpc::new(answered(json!({
            "ordinaryUsageAllowed": true,
            "rateLimits": {
                "limitId": "codex",
                "primary": { "usedPercent": 2, "windowDurationMins": 300, "resetsAt": 1790880511 },
                "secondary": { "usedPercent": 130, "windowDurationMins": 10080, "resetsAt": null },
                "planType": "plus",
                "rateLimitReachedType": null
            }
        })));
        let usage = codex_rpc(rpc).read_usage().await.expect("usage");
        assert_eq!(usage.provider_id, "codex");
        assert_eq!(usage.plan.as_deref(), Some("plus"));
        assert!(!usage.limit_reached);
        assert_eq!(
            usage.windows,
            [
                UsageWindow {
                    duration_minutes: Some(300),
                    scope: None,
                    used_percent: 2,
                    resets_at: DateTime::from_timestamp(1_790_880_511, 0),
                },
                UsageWindow {
                    duration_minutes: Some(10080),
                    scope: None,
                    used_percent: 100,
                    resets_at: None,
                }
            ]
        );
    }

    #[test]
    fn flags_reached_limits_and_rejects_missing_snapshots() {
        let now = Utc::now();
        let reached = parse_usage(
            "codex",
            &json!({ "rateLimits": { "primary": null, "rateLimitReachedType": "rate_limit_reached" } }),
            now,
        )
        .expect("usage");
        assert!(reached.limit_reached);
        assert!(reached.windows.is_empty());
        assert!(parse_usage("codex", &json!({}), now).is_err());
    }

    #[tokio::test]
    async fn reports_app_server_failures() {
        let rejected = ScriptedRpc::new(JsonRpcExit::Completed(vec![
            Ok(json!({})),
            Err(JsonRpcError {
                code: -32600,
                message: "not signed in with ChatGPT".into(),
            }),
        ]));
        let result = codex_rpc(rejected).read_usage().await;
        assert!(matches!(
            result,
            Err(AppError::Provider(message))
                if message == "Codex app-server rejected `account/rateLimits/read`: not signed in with ChatGPT"
        ));

        let missing = ScriptedRpc::new(JsonRpcExit::ProgramNotFound);
        assert!(codex_rpc(missing).list_models().await.is_err());

        let timed_out = ScriptedRpc::new(JsonRpcExit::TimedOut);
        assert!(matches!(
            codex_rpc(timed_out).read_usage().await,
            Err(AppError::Provider(message)) if message.contains("did not answer")
        ));
    }
}
