//! Adapter for the official Anthropic Claude Code CLI.
//!
//! Verified against `2.1.286 (Claude Code)` help output and recorded `stream-json` runs:
//!
//! - Turn: `claude -p --output-format stream-json --verbose --permission-mode dontAsk
//!   --permission-prompts none --tools <list> --allowed-tools <list> [--model <m>]
//!   [--effort <e>] [--resume <id>]`, run in the workspace directory. `--print` rejects
//!   `stream-json` without `--verbose`.
//! - The prompt is read from stdin so it never appears in a command line.
//! - `--tools` removes every built-in tool outside the list; `--allowed-tools` alone is not
//!   enough because Claude Code auto-approves read-only shell commands. `dontAsk` with
//!   `--permission-prompts none` denies anything that would otherwise prompt, so a turn never
//!   waits for input. Read-only agents get `Read,Glob,Grep`; workspace-write agents also get
//!   `Edit,Write,Bash`, and agents with internet access also get `WebFetch,WebSearch`
//!   (checked against `2.1.287`). Claude Code's Bash tool is not sandboxed by the operating
//!   system, so workspace-write commands can reach files outside the workspace and the
//!   network.
//! - Events are JSONL: `system`/`init` carries the session id, `assistant` messages carry
//!   `text` and `tool_use` blocks, `user` messages carry `tool_result` blocks, and `result`
//!   ends the turn (`is_error` reports failures). Errors raised before a model call arrive as
//!   a synthetic assistant message (`model: "<synthetic>"`) followed by an error result.
//! - `--resume <id>` continues a session under the same id.
//! - Runtime tools: `--mcp-config <json> --strict-mcp-config` registers only the Open Bots
//!   HTTP MCP server, with `Authorization: Bearer ${OPEN_BOTS_MCP_TOKEN}` expanded by the CLI
//!   from the environment, and `mcp__open_bots` in `--allowed-tools` pre-approves its tools.
//! - Configured MCP servers: when the agent selects servers from the user's own Claude Code
//!   configuration (user, project and claude.ai connectors, as `claude mcp list` names them),
//!   `--strict-mcp-config` is left out so those servers load next to Open Bots, and each
//!   selected server gets `mcp__<server>__*` in `--allowed-tools`, the documented per-server
//!   wildcard. A bare `mcp__*` was verified not to pre-approve anything. Tool names replace
//!   characters outside `[A-Za-z0-9_-]` with `_`, so `claude.ai Atlassian` becomes
//!   `mcp__claude_ai_Atlassian__<tool>`. Servers that are not selected still load but every
//!   call is denied by `dontAsk`. Selected tools run without prompts, including writes to
//!   external services. claude.ai connectors connect asynchronously in `--print` mode, so
//!   `CLAUDE_CODE_MCP_STARTUP_WAIT_MS` (documented, v2.1.274 or later) makes the first turn
//!   wait for every pending server. `--tools` turns tool search off, so connected servers'
//!   tools are listed directly.
//! - The catalog import runs `claude mcp list`, which health-checks each server and prints
//!   `<name>: <command or URL> - <symbol> <status>` per line (verified: `✔ Connected`,
//!   `! Needs authentication`). Only the name and status are kept; the command or URL may
//!   hold secrets and is dropped unread. The command has no JSON output, so a run whose
//!   lines all fail to parse is an error. Project-scoped servers depend on the directory, so
//!   only user-scope servers and claude.ai connectors are listed reliably.
//!
//! Models come from a static list of aliases resolved by the CLI (`--model` documents
//! `fable`, `opus` and `sonnet`; `haiku` was verified in a recorded run). The CLI exposes no
//! model catalog.
//!
//! Usage limits come from the built-in `/usage` command, sent on stdin to
//! `claude -p --safe-mode --output-format stream-json --verbose --no-session-persistence
//! --tools ""`. It is answered locally without a model call (`total_cost_usd: 0`), and
//! `--safe-mode` keeps user hooks and plugins out of the read. The answer is human-readable
//! text in the `result` event, such as `Current session: 14% used · resets Oct 1, 1:40pm
//! (America/Sao_Paulo)`; `Current session` is the five-hour window and `Current week (<scope>)`
//! a weekly one. Reset times are printed in the system time zone without a year, so they are
//! read as local time on the next matching date. Unrecognized lines are skipped, and a report
//! without any window is an error rather than an empty success. The plan comes from
//! `subscriptionType` in `claude auth status --json`.
//!
//! Detection runs `claude --version` and `claude auth status --json` and reads only the
//! `loggedIn` field; the rest of that output contains account details and is never stored or
//! logged. Authentication stays with the CLI; Open Bots never reads Claude credentials.

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Utc};
use serde_json::{json, Value};

use super::{
    cli::{CliOutput, CliProgram, SignInCheck, StreamEvent, TurnStreamParser},
    AgentProvider, ConfiguredMcpServer, McpServerStatus, ProviderCapability, ProviderModel,
    ProviderSummary, ProviderUsage, TurnEvent, TurnEvents, TurnOutcome, TurnRequest, UsageWindow,
    RUNTIME_TOOLS_SERVER_NAME, RUNTIME_TOOLS_TOKEN_ENV,
};
use crate::{
    domain::{agents::WorkspaceAccess, mcp_servers::normalize_mcp_server_name},
    error::{AppError, AppResult},
    infrastructure::process::{
        LineCommand, LineProcessExit, LineProcessRunner, ProcessEnvironment,
    },
    runtime::cancellation::CancellationSignal,
};

const PROGRAM: &str = "claude";
const READ_ONLY_TOOLS: &str = "Read,Glob,Grep";
const WORKSPACE_WRITE_TOOLS: &str = "Read,Glob,Grep,Edit,Write,Bash";
/// Claude Code's built-in web tools, added when the agent has internet access.
const NETWORK_TOOLS: &str = "WebFetch,WebSearch";
/// How long the first turn waits for configured MCP servers that are still connecting.
const MCP_STARTUP_WAIT_ENV: &str = "CLAUDE_CODE_MCP_STARTUP_WAIT_MS";
const MCP_STARTUP_WAIT_MS: &str = "15000";
/// Effort levels documented by `claude --help` for `--effort`.
const EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];
/// Reads `/usage` locally: no tools, no saved session, and no user hooks or plugins.
const USAGE_ARGUMENTS: [&str; 8] = [
    "-p",
    "--safe-mode",
    "--output-format",
    "stream-json",
    "--verbose",
    "--no-session-persistence",
    "--tools",
    "",
];
const SESSION_WINDOW_MINUTES: i64 = 5 * 60;
const WEEK_WINDOW_MINUTES: i64 = 7 * 24 * 60;

pub struct ClaudeProvider {
    cli: CliProgram,
}

impl ClaudeProvider {
    pub fn new(runner: Arc<dyn LineProcessRunner>) -> Self {
        Self {
            cli: CliProgram::new(PROGRAM, "Claude Code", runner),
        }
    }
}

#[async_trait]
impl AgentProvider for ClaudeProvider {
    fn id(&self) -> &'static str {
        "claude-code"
    }

    fn name(&self) -> &'static str {
        "Claude Code"
    }

    async fn detect(&self) -> AppResult<ProviderSummary> {
        self.cli
            .detect(
                self,
                SignInCheck {
                    arguments: &["auth", "status", "--json"],
                    confirmed: |output| output.succeeded() && logged_in(output),
                    login_hint: "claude auth login",
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
            ProviderCapability::ConfiguredMcpServers,
        ]
    }

    async fn list_models(&self) -> AppResult<Vec<ProviderModel>> {
        Ok(model_catalog())
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
                &mut ClaudeStreamParser::default(),
                &events,
                cancellation,
            )
            .await
    }

    async fn list_configured_mcp_servers(&self) -> AppResult<Vec<ConfiguredMcpServer>> {
        let output = self.cli.run_quiet(&["mcp", "list"], None).await?;
        match &output.exit {
            LineProcessExit::ProgramNotFound => return Err(self.cli.not_found()),
            LineProcessExit::Finished {
                success: false,
                code,
                stderr_tail,
            } => {
                return Err(self
                    .cli
                    .exit_error("Claude Code mcp list", *code, stderr_tail))
            }
            _ => {}
        }
        parse_mcp_list(&output.lines)
    }

    async fn read_usage(&self) -> AppResult<ProviderUsage> {
        let output = self.cli.run_quiet(&USAGE_ARGUMENTS, Some("/usage")).await?;
        match &output.exit {
            LineProcessExit::ProgramNotFound => return Err(self.cli.not_found()),
            LineProcessExit::Finished {
                success: false,
                code,
                stderr_tail,
            } => {
                return Err(self
                    .cli
                    .exit_error("Claude Code /usage", *code, stderr_tail))
            }
            _ => {}
        }
        let windows = parse_usage_windows(&usage_report(&output)?, &Local::now())?;
        // The plan is optional context; a failed lookup must not hide the usage itself.
        let plan = self
            .cli
            .run_quiet(&["auth", "status", "--json"], None)
            .await
            .ok()
            .filter(CliOutput::succeeded)
            .and_then(|output| auth_field(&output, "subscriptionType"));
        Ok(ProviderUsage {
            provider_id: self.id().into(),
            plan,
            limit_reached: windows.iter().any(|window| window.used_percent >= 100),
            windows,
            checked_at: Utc::now(),
        })
    }
}

/// Reads only `loggedIn` from `claude auth status --json`.
fn logged_in(output: &CliOutput) -> bool {
    auth_status(output)
        .and_then(|value| value.get("loggedIn").and_then(Value::as_bool))
        .unwrap_or(false)
}

/// Reads one string field from `claude auth status --json`.
fn auth_field(output: &CliOutput, key: &str) -> Option<String> {
    auth_status(output)?
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn auth_status(output: &CliOutput) -> Option<Value> {
    serde_json::from_str(output.lines.join("\n").trim()).ok()
}

/// Extracts the `/usage` answer from the `result` event.
fn usage_report(output: &CliOutput) -> AppResult<String> {
    let result = output
        .lines
        .iter()
        .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
        .find(|value| value.get("type").and_then(Value::as_str) == Some("result"))
        .ok_or_else(|| AppError::Provider("Claude Code did not answer `/usage`".into()))?;
    let text = result
        .get("result")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    if result.get("is_error").and_then(Value::as_bool) == Some(true) {
        return Err(AppError::Provider(format!(
            "Claude Code could not read usage: {}",
            first_line(&text)
        )));
    }
    Ok(text)
}

/// Maps `/usage` text to usage windows. `now` sets the time zone and the reference date for
/// reset times, which the CLI prints without a year.
fn parse_usage_windows<Tz: TimeZone>(
    text: &str,
    now: &DateTime<Tz>,
) -> AppResult<Vec<UsageWindow>> {
    let windows: Vec<UsageWindow> = text
        .lines()
        .filter_map(|line| parse_usage_window(line.trim(), now))
        .collect();
    if windows.is_empty() {
        return Err(AppError::Provider(format!(
            "Claude Code did not report usage limits: {}",
            first_line(text)
        )));
    }
    Ok(windows)
}

/// Parses `Current <window>: <n>% used · resets <when>`.
fn parse_usage_window<Tz: TimeZone>(line: &str, now: &DateTime<Tz>) -> Option<UsageWindow> {
    let (name, detail) = line.strip_prefix("Current ")?.split_once(": ")?;
    let (percent, rest) = detail.split_once("% used")?;
    let used: f64 = percent.trim().parse().ok()?;
    let (duration, scope) = if name == "session" {
        (SESSION_WINDOW_MINUTES, None)
    } else {
        let scope = name
            .strip_prefix("week")?
            .trim()
            .strip_prefix('(')
            .and_then(|scope| scope.strip_suffix(')'))
            .map(str::trim)
            .filter(|scope| !scope.is_empty() && !scope.eq_ignore_ascii_case("all models"))
            .map(str::to_owned);
        (WEEK_WINDOW_MINUTES, scope)
    };
    Some(UsageWindow {
        duration_minutes: Some(duration),
        scope,
        // Clamped to 0..=100 first, so the conversion cannot truncate.
        used_percent: used.round().clamp(0.0, 100.0) as u8,
        resets_at: rest
            .split_once("resets ")
            .and_then(|(_, reset)| parse_reset(reset, now)),
    })
}

/// Parses `Oct 5, 12am (Zone)` or `1:40pm (Zone)` as the next matching local time.
fn parse_reset<Tz: TimeZone>(text: &str, now: &DateTime<Tz>) -> Option<DateTime<Utc>> {
    let text = text.split(" (").next()?.trim();
    let (date, clock) = match text.split_once(", ") {
        Some((date, clock)) => (Some(date), clock),
        None => (None, text),
    };
    let (hour, minute) = parse_clock(clock.trim())?;
    let today = now.naive_local().date();
    let dates: Vec<NaiveDate> = match date {
        Some(date) => {
            let (month, day) = date.trim().split_once(' ')?;
            let month = month_number(month)?;
            let day: u32 = day.trim().parse().ok()?;
            (today.year() - 1..=today.year() + 1)
                .filter_map(|year| NaiveDate::from_ymd_opt(year, month, day))
                .collect()
        }
        None => vec![today, today.succ_opt()?],
    };
    // A reset moments ago may still be printed, so allow a small grace period.
    let earliest = now.clone() - Duration::hours(1);
    dates
        .into_iter()
        .filter_map(|date| date.and_hms_opt(hour, minute, 0))
        .filter_map(|local| now.timezone().from_local_datetime(&local).earliest())
        .find(|at| *at >= earliest)
        .map(|at| at.with_timezone(&Utc))
}

/// Parses `1:40pm` or `12am` into a 24-hour clock.
fn parse_clock(clock: &str) -> Option<(u32, u32)> {
    let clock = clock.to_ascii_lowercase();
    let (clock, afternoon) = match (clock.strip_suffix("am"), clock.strip_suffix("pm")) {
        (Some(clock), _) => (clock.to_owned(), false),
        (_, Some(clock)) => (clock.to_owned(), true),
        _ => return None,
    };
    let (hour, minute) = match clock.split_once(':') {
        Some((hour, minute)) => (hour.parse::<u32>().ok()?, minute.parse::<u32>().ok()?),
        None => (clock.parse::<u32>().ok()?, 0),
    };
    if !(1..=12).contains(&hour) || minute > 59 {
        return None;
    }
    Some((hour % 12 + if afternoon { 12 } else { 0 }, minute))
}

fn month_number(name: &str) -> Option<u32> {
    const MONTHS: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let prefix = name.get(..3)?.to_ascii_lowercase();
    let index = MONTHS.iter().position(|month| *month == prefix)?;
    u32::try_from(index + 1).ok()
}

fn first_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("no details")
        .chars()
        .take(200)
        .collect()
}

fn model_catalog() -> Vec<ProviderModel> {
    let alias = |id: &str, display_name: &str, efforts: bool| ProviderModel {
        id: id.into(),
        display_name: display_name.into(),
        description: "Alias resolved by the Claude Code CLI to its latest model.".into(),
        is_default: false,
        reasoning_efforts: if efforts {
            EFFORTS.iter().map(|effort| (*effort).into()).collect()
        } else {
            Vec::new()
        },
        default_reasoning_effort: None,
    };
    vec![
        alias("fable", "Fable", true),
        alias("opus", "Opus", true),
        alias("sonnet", "Sonnet", true),
        // Effort support for Haiku has not been verified, so none is offered.
        alias("haiku", "Haiku", false),
    ]
}

fn allowed_tools(access: WorkspaceAccess, network: bool) -> String {
    let base = match access {
        WorkspaceAccess::ReadOnly => READ_ONLY_TOOLS,
        WorkspaceAccess::WorkspaceWrite => WORKSPACE_WRITE_TOOLS,
    };
    if network {
        format!("{base},{NETWORK_TOOLS}")
    } else {
        base.to_owned()
    }
}

/// Reads `claude mcp list` output. Lines that are not server entries, such as the health
/// check banner, are skipped; the command or URL part of each entry is never kept.
fn parse_mcp_list(lines: &[String]) -> AppResult<Vec<ConfiguredMcpServer>> {
    let mut servers: Vec<ConfiguredMcpServer> = Vec::new();
    let mut unrecognized = 0;
    for line in lines.iter().map(|line| line.trim()) {
        if line.is_empty() || line.starts_with("Checking MCP server health") {
            continue;
        }
        if line.starts_with("No MCP servers configured") {
            return Ok(Vec::new());
        }
        let entry = line
            .rsplit_once(" - ")
            .and_then(|(server, status)| Some((server.split_once(": ")?.0, status)));
        let Some((name, status)) = entry else {
            unrecognized += 1;
            continue;
        };
        let Ok(name) = normalize_mcp_server_name(name) else {
            unrecognized += 1;
            continue;
        };
        if servers.iter().all(|server| server.name != name) {
            servers.push(ConfiguredMcpServer {
                name,
                status: mcp_status(status),
            });
        }
    }
    if servers.is_empty() && unrecognized > 0 {
        return Err(AppError::Provider(
            "the output of `claude mcp list` was not recognized".into(),
        ));
    }
    if unrecognized > 0 {
        tracing::warn!(unrecognized, "skipped unrecognized `claude mcp list` lines");
    }
    Ok(servers)
}

fn mcp_status(status: &str) -> McpServerStatus {
    let status = status.to_ascii_lowercase();
    if status.contains("needs authentication") {
        McpServerStatus::NeedsAuthentication
    } else if status.contains("pending approval") {
        McpServerStatus::PendingApproval
    } else if status.contains("failed") {
        McpServerStatus::Failed
    } else if status.contains("connected") {
        McpServerStatus::Connected
    } else {
        McpServerStatus::Unknown
    }
}

/// The tool-name prefix Claude Code derives from an MCP server name.
fn mcp_tool_prefix(server: &str) -> String {
    let name: String = server
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "_-".contains(character) {
                character
            } else {
                '_'
            }
        })
        .collect();
    format!("mcp__{name}")
}

fn turn_command(request: &TurnRequest) -> LineCommand {
    let tools = allowed_tools(request.access, request.network);
    let mut allowed = vec![tools.clone()];
    // A server name in `--allowed-tools` pre-approves every tool that server provides.
    let runtime_prefix = format!("mcp__{RUNTIME_TOOLS_SERVER_NAME}");
    if request.runtime_tools.is_some() {
        allowed.push(runtime_prefix.clone());
    }
    for server in &request.mcp_servers {
        let prefix = mcp_tool_prefix(server);
        // A configured server cannot stand in for the Open Bots runtime server.
        if prefix != runtime_prefix {
            allowed.push(format!("{prefix}__*"));
        }
    }
    let allowed = allowed.join(",");
    let mut arguments: Vec<String> = [
        "-p",
        "--output-format",
        "stream-json",
        "--verbose",
        "--permission-mode",
        "dontAsk",
        "--permission-prompts",
        "none",
        "--tools",
        &tools,
        "--allowed-tools",
        &allowed,
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    // Identifiers are validated by the domain and passed as separate arguments.
    if let Some(model) = &request.model {
        arguments.extend(["--model".into(), model.clone()]);
    }
    if let Some(effort) = &request.reasoning_effort {
        arguments.extend(["--effort".into(), effort.clone()]);
    }
    if let Some(session_id) = &request.session_id {
        arguments.extend(["--resume".into(), session_id.clone()]);
    }
    let mut environment = ProcessEnvironment::default();
    if let Some(endpoint) = &request.runtime_tools {
        // `${VAR}` is expanded by Claude Code, so the token stays out of the arguments.
        let config = json!({
            "mcpServers": {
                RUNTIME_TOOLS_SERVER_NAME: {
                    "type": "http",
                    "url": endpoint.url,
                    "headers": { "Authorization": format!("Bearer ${{{RUNTIME_TOOLS_TOKEN_ENV}}}") }
                }
            }
        });
        arguments.extend(["--mcp-config".into(), config.to_string()]);
        // Without selected servers, the user's own configuration stays out of the turn.
        if request.mcp_servers.is_empty() {
            arguments.push("--strict-mcp-config".into());
        }
        environment.set(RUNTIME_TOOLS_TOKEN_ENV, endpoint.token.clone());
    }
    if !request.mcp_servers.is_empty() {
        environment.set(MCP_STARTUP_WAIT_ENV, MCP_STARTUP_WAIT_MS);
    }
    LineCommand {
        environment,
        program: PROGRAM.into(),
        arguments,
        working_directory: Some(request.workspace.clone()),
        stdin: Some(request.prompt.clone()),
    }
}

/// Maps `stream-json` lines to provider-neutral events. It remembers tool summaries so a
/// `tool_result` can be reported with the summary of the `tool_use` it answers. Unknown events
/// (hooks, thinking, rate limits) are ignored so newer CLI versions do not break the adapter.
#[derive(Default)]
struct ClaudeStreamParser {
    session_started: bool,
    actions: HashMap<String, String>,
}

impl TurnStreamParser for ClaudeStreamParser {
    fn parse_line(&mut self, line: &str) -> Vec<StreamEvent> {
        let Ok(value) = serde_json::from_str::<Value>(line.trim()) else {
            return Vec::new();
        };
        let text =
            |value: &Value, key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
        match value.get("type").and_then(Value::as_str) {
            Some("system") if text(&value, "subtype").as_deref() == Some("init") => {
                match text(&value, "session_id") {
                    Some(session_id) if !self.session_started => {
                        self.session_started = true;
                        vec![StreamEvent::Turn(TurnEvent::SessionStarted { session_id })]
                    }
                    _ => Vec::new(),
                }
            }
            Some("assistant") => {
                let message = value.get("message");
                // Synthetic messages repeat an error that the result event reports.
                if message
                    .and_then(|message| text(message, "model"))
                    .as_deref()
                    == Some("<synthetic>")
                {
                    return Vec::new();
                }
                content_blocks(message)
                    .filter_map(|block| match block.get("type")?.as_str()? {
                        "text" => Some(StreamEvent::Turn(TurnEvent::Message {
                            text: text(block, "text").filter(|text| !text.trim().is_empty())?,
                        })),
                        "tool_use" => {
                            let id = text(block, "id").unwrap_or_default();
                            let summary = tool_summary(block);
                            self.actions.insert(id.clone(), summary.clone());
                            Some(StreamEvent::Turn(TurnEvent::ActionStarted { id, summary }))
                        }
                        _ => None,
                    })
                    .collect()
            }
            Some("user") => content_blocks(value.get("message"))
                .filter(|block| block.get("type").and_then(Value::as_str) == Some("tool_result"))
                .map(|block| {
                    let id = text(block, "tool_use_id").unwrap_or_default();
                    let summary = self.actions.remove(&id).unwrap_or_else(|| "tool".into());
                    StreamEvent::Turn(TurnEvent::ActionCompleted {
                        id,
                        summary,
                        succeeded: !block
                            .get("is_error")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                    })
                })
                .collect(),
            Some("result") if value.get("is_error").and_then(Value::as_bool) == Some(true) => {
                vec![StreamEvent::Failed(
                    text(&value, "result")
                        .filter(|result| !result.trim().is_empty())
                        .or_else(|| text(&value, "subtype"))
                        .unwrap_or_else(|| "the CLI reported an error".into()),
                )]
            }
            _ => Vec::new(),
        }
    }
}

fn content_blocks(message: Option<&Value>) -> impl Iterator<Item = &Value> {
    message
        .and_then(|message| message.get("content"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

fn tool_summary(block: &Value) -> String {
    let name = block.get("name").and_then(Value::as_str).unwrap_or("tool");
    let input = block.get("input");
    let field = |key: &str| {
        input
            .and_then(|input| input.get(key))
            .and_then(Value::as_str)
    };
    match name {
        "Bash" => field("command").map(str::to_owned),
        "Read" | "Edit" | "Write" => field("file_path").map(|path| format!("{name} {path}")),
        "Glob" | "Grep" => field("pattern").map(|pattern| format!("{name} {pattern}")),
        _ => None,
    }
    .unwrap_or_else(|| name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{
        testing::{finished, ScriptedRunner},
        DetectionStatus,
    };
    use chrono::FixedOffset;
    use tokio::sync::mpsc;

    #[test]
    fn registers_runtime_tools_with_the_token_in_the_environment() {
        let mut with_tools = request(Some("session-1"), WorkspaceAccess::ReadOnly);
        with_tools.runtime_tools = Some(crate::providers::RuntimeToolsEndpoint {
            url: "http://127.0.0.1:4100/mcp".into(),
            token: "secret-token".into(),
        });
        let command = turn_command(&with_tools);
        let arguments = command.arguments.join(" ");
        assert!(!arguments.contains("secret-token"));
        assert_eq!(
            command.environment.get("OPEN_BOTS_MCP_TOKEN"),
            Some("secret-token")
        );
        let allowed = command
            .arguments
            .iter()
            .position(|argument| argument == "--allowed-tools")
            .map(|index| command.arguments[index + 1].clone());
        assert_eq!(allowed.as_deref(), Some("Read,Glob,Grep,mcp__open_bots"));
        let config_index = command
            .arguments
            .iter()
            .position(|argument| argument == "--mcp-config")
            .expect("mcp config");
        let config: Value =
            serde_json::from_str(&command.arguments[config_index + 1]).expect("json config");
        assert_eq!(
            config["mcpServers"]["open_bots"],
            json!({
                "type": "http",
                "url": "http://127.0.0.1:4100/mcp",
                "headers": { "Authorization": "Bearer ${OPEN_BOTS_MCP_TOKEN}" }
            })
        );
        assert!(command
            .arguments
            .contains(&"--strict-mcp-config".to_owned()));
        assert!(!format!("{command:?}").contains("secret-token"));
    }

    #[test]
    fn pre_approves_selected_configured_mcp_servers() {
        let mut with_servers = request(None, WorkspaceAccess::ReadOnly);
        with_servers.runtime_tools = Some(crate::providers::RuntimeToolsEndpoint {
            url: "http://127.0.0.1:4100/mcp".into(),
            token: "secret-token".into(),
        });
        with_servers.mcp_servers = vec![
            "claude.ai Atlassian".into(),
            "github".into(),
            "open_bots".into(),
        ];
        let command = turn_command(&with_servers);
        let allowed = command
            .arguments
            .iter()
            .position(|argument| argument == "--allowed-tools")
            .map(|index| command.arguments[index + 1].clone());
        assert_eq!(
            allowed.as_deref(),
            Some("Read,Glob,Grep,mcp__open_bots,mcp__claude_ai_Atlassian__*,mcp__github__*")
        );
        assert!(command.arguments.contains(&"--mcp-config".to_owned()));
        assert!(!command
            .arguments
            .contains(&"--strict-mcp-config".to_owned()));
        assert_eq!(
            command.environment.get("CLAUDE_CODE_MCP_STARTUP_WAIT_MS"),
            Some("15000")
        );
    }

    #[test]
    fn parses_mcp_list_names_and_statuses_without_targets() {
        let lines: Vec<String> = [
            "Checking MCP server health…",
            "",
            "claude.ai Atlassian: https://mcp.atlassian.com/v1/mcp - ✔ Connected",
            "claude.ai Notion: https://mcp.notion.com/mcp - ! Needs authentication",
            "github: npx -y server-github --token=secret - ✗ Failed to connect",
            "weird\"name: x - ✔ Connected",
        ]
        .map(String::from)
        .to_vec();
        let servers = parse_mcp_list(&lines).expect("servers");
        assert_eq!(
            servers,
            vec![
                ConfiguredMcpServer {
                    name: "claude.ai Atlassian".into(),
                    status: McpServerStatus::Connected
                },
                ConfiguredMcpServer {
                    name: "claude.ai Notion".into(),
                    status: McpServerStatus::NeedsAuthentication
                },
                ConfiguredMcpServer {
                    name: "github".into(),
                    status: McpServerStatus::Failed
                },
            ]
        );
        assert!(!format!("{servers:?}").contains("secret"));
    }

    #[test]
    fn rejects_unrecognized_mcp_list_output() {
        let empty = parse_mcp_list(&["No MCP servers configured. Use `claude mcp add`.".into()]);
        assert_eq!(empty.expect("empty"), Vec::new());
        assert!(parse_mcp_list(&["something else entirely".into()]).is_err());
    }

    #[test]
    fn keeps_configured_mcp_servers_out_by_default() {
        let mut with_tools = request(None, WorkspaceAccess::ReadOnly);
        with_tools.runtime_tools = Some(crate::providers::RuntimeToolsEndpoint {
            url: "http://127.0.0.1:4100/mcp".into(),
            token: "secret-token".into(),
        });
        let command = turn_command(&with_tools);
        assert!(command
            .arguments
            .contains(&"--strict-mcp-config".to_owned()));
        assert!(!command.arguments.join(" ").contains("__*"));
        assert_eq!(
            command.environment.get("CLAUDE_CODE_MCP_STARTUP_WAIT_MS"),
            None
        );
    }

    #[test]
    fn adds_web_tools_only_with_internet_access() {
        let tools = |network: bool| {
            let mut request = request(None, WorkspaceAccess::WorkspaceWrite);
            request.network = network;
            let command = turn_command(&request);
            let at = |flag: &str| {
                let index = command
                    .arguments
                    .iter()
                    .position(|argument| argument == flag)
                    .expect("flag");
                command.arguments[index + 1].clone()
            };
            (at("--tools"), at("--allowed-tools"))
        };
        assert_eq!(
            tools(true),
            (
                "Read,Glob,Grep,Edit,Write,Bash,WebFetch,WebSearch".into(),
                "Read,Glob,Grep,Edit,Write,Bash,WebFetch,WebSearch".into()
            )
        );
        assert!(!tools(false).0.contains("Web"));
    }

    #[test]
    fn omits_runtime_tools_when_not_offered() {
        let command = turn_command(&request(None, WorkspaceAccess::ReadOnly));
        assert!(!command.arguments.contains(&"--mcp-config".to_owned()));
        assert_eq!(command.environment.get("OPEN_BOTS_MCP_TOKEN"), None);
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

    const BASE_ARGUMENTS: [&str; 8] = [
        "-p",
        "--output-format",
        "stream-json",
        "--verbose",
        "--permission-mode",
        "dontAsk",
        "--permission-prompts",
        "none",
    ];

    #[test]
    fn starts_read_only_sessions_with_restricted_tools() {
        let command = turn_command(&request(None, WorkspaceAccess::ReadOnly));
        let mut expected: Vec<&str> = BASE_ARGUMENTS.to_vec();
        expected.extend([
            "--tools",
            "Read,Glob,Grep",
            "--allowed-tools",
            "Read,Glob,Grep",
        ]);
        assert_eq!(command.program, "claude");
        assert_eq!(command.arguments, expected);
        assert_eq!(command.stdin.as_deref(), Some("List the files"));
        assert_eq!(command.working_directory, Some("/work/project".into()));
    }

    #[test]
    fn resumes_workspace_write_sessions_with_model_and_effort() {
        let mut resumed = request(Some("session-1"), WorkspaceAccess::WorkspaceWrite);
        resumed.model = Some("opus".into());
        resumed.reasoning_effort = Some("high".into());
        let command = turn_command(&resumed);
        let mut expected: Vec<&str> = BASE_ARGUMENTS.to_vec();
        expected.extend([
            "--tools",
            "Read,Glob,Grep,Edit,Write,Bash",
            "--allowed-tools",
            "Read,Glob,Grep,Edit,Write,Bash",
            "--model",
            "opus",
            "--effort",
            "high",
            "--resume",
            "session-1",
        ]);
        assert_eq!(command.arguments, expected);
        assert!(!command
            .arguments
            .iter()
            .any(|argument| argument.contains("List")));
    }

    #[test]
    fn parses_the_recorded_event_stream() {
        let mut parser = ClaudeStreamParser::default();
        assert_eq!(
            parser.parse_line(r#"{"type":"system","subtype":"init","session_id":"e890e179","model":"claude-haiku-4-5"}"#),
            [StreamEvent::Turn(TurnEvent::SessionStarted {
                session_id: "e890e179".into()
            })]
        );
        assert_eq!(
            parser.parse_line(
                r#"{"type":"assistant","message":{"model":"claude-haiku-4-5","content":[{"type":"thinking","thinking":""},{"type":"text","text":"I'll read the note."},{"type":"tool_use","id":"toolu_1","name":"Read","input":{"file_path":"/work/note.txt"}}]}}"#
            ),
            [
                StreamEvent::Turn(TurnEvent::Message {
                    text: "I'll read the note.".into()
                }),
                StreamEvent::Turn(TurnEvent::ActionStarted {
                    id: "toolu_1".into(),
                    summary: "Read /work/note.txt".into()
                })
            ]
        );
        assert_eq!(
            parser.parse_line(
                r#"{"type":"user","message":{"content":[{"tool_use_id":"toolu_1","type":"tool_result","content":"hello"}]}}"#
            ),
            [StreamEvent::Turn(TurnEvent::ActionCompleted {
                id: "toolu_1".into(),
                summary: "Read /work/note.txt".into(),
                succeeded: true
            })]
        );
        parser.parse_line(
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_2","name":"Bash","input":{"command":"ls","description":"List files"}}]}}"#,
        );
        assert_eq!(
            parser.parse_line(
                r#"{"type":"user","message":{"content":[{"tool_use_id":"toolu_2","type":"tool_result","content":"denied","is_error":true}]}}"#
            ),
            [StreamEvent::Turn(TurnEvent::ActionCompleted {
                id: "toolu_2".into(),
                summary: "ls".into(),
                succeeded: false
            })]
        );
        assert_eq!(
            parser.parse_line(
                r#"{"type":"result","subtype":"success","is_error":true,"result":"There's an issue with the selected model.","session_id":"e890e179"}"#
            ),
            [StreamEvent::Failed(
                "There's an issue with the selected model.".into()
            )]
        );
    }

    #[test]
    fn ignores_internal_synthetic_unknown_and_malformed_lines() {
        let mut parser = ClaudeStreamParser::default();
        parser.parse_line(r#"{"type":"system","subtype":"init","session_id":"s"}"#);
        for line in [
            r#"{"type":"system","subtype":"init","session_id":"s"}"#,
            r#"{"type":"system","subtype":"hook_started","session_id":"s"}"#,
            r#"{"type":"system","subtype":"thinking_tokens","estimated_tokens":10}"#,
            r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed"}}"#,
            r#"{"type":"assistant","message":{"model":"<synthetic>","content":[{"type":"text","text":"error"}]}}"#,
            r#"{"type":"result","subtype":"success","is_error":false,"result":"Done."}"#,
            r#"{"type":"future.event"}"#,
            "not json",
            "",
        ] {
            assert!(parser.parse_line(line).is_empty(), "{line}");
        }
    }

    #[tokio::test]
    async fn runs_a_turn_and_reports_the_new_session() {
        let runner = ScriptedRunner::single(
            vec![
                r#"{"type":"system","subtype":"hook_started","session_id":"session-9"}"#,
                r#"{"type":"system","subtype":"init","session_id":"session-9"}"#,
                r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Done."}]}}"#,
                r#"{"type":"result","subtype":"success","is_error":false,"result":"Done.","session_id":"session-9"}"#,
            ],
            finished(true),
        );
        let provider = ClaudeProvider::new(runner);
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
                session_id: Some("session-9".into()),
                cancelled: false
            }
        );
        assert_eq!(
            receiver.recv().await,
            Some(TurnEvent::SessionStarted {
                session_id: "session-9".into()
            })
        );
        assert_eq!(
            receiver.recv().await,
            Some(TurnEvent::Message {
                text: "Done.".into()
            })
        );
        assert_eq!(receiver.recv().await, None);
    }

    #[tokio::test]
    async fn reports_turn_failures_and_exit_errors() {
        let failed = ScriptedRunner::single(
            vec![
                r#"{"type":"assistant","message":{"model":"<synthetic>","content":[{"type":"text","text":"Model issue"}]}}"#,
                r#"{"type":"result","subtype":"success","is_error":true,"result":"Model issue"}"#,
            ],
            finished(false),
        );
        let result = ClaudeProvider::new(failed)
            .run_turn(
                request(None, WorkspaceAccess::ReadOnly),
                mpsc::unbounded_channel().0,
                CancellationSignal::never(),
            )
            .await;
        assert!(
            matches!(result, Err(AppError::Provider(message)) if message == "Claude Code turn failed: Model issue")
        );

        let crashed = ScriptedRunner::single(
            Vec::new(),
            LineProcessExit::Finished {
                success: false,
                code: Some(1),
                stderr_tail:
                    "Error: When using --print, --output-format=stream-json requires --verbose\n"
                        .into(),
            },
        );
        let result = ClaudeProvider::new(crashed)
            .run_turn(
                request(Some("session-1"), WorkspaceAccess::ReadOnly),
                mpsc::unbounded_channel().0,
                CancellationSignal::never(),
            )
            .await;
        assert!(
            matches!(result, Err(AppError::Provider(message)) if message.starts_with("Claude Code exited with 1: Error: When using --print"))
        );

        let missing = ScriptedRunner::single(Vec::new(), LineProcessExit::ProgramNotFound);
        let result = ClaudeProvider::new(missing)
            .run_turn(
                request(None, WorkspaceAccess::ReadOnly),
                mpsc::unbounded_channel().0,
                CancellationSignal::never(),
            )
            .await;
        assert!(matches!(result, Err(AppError::Provider(message)) if message.contains("PATH")));
    }

    #[tokio::test]
    async fn reports_cancelled_turns() {
        let runner = ScriptedRunner::single(Vec::new(), LineProcessExit::Cancelled);
        let outcome = ClaudeProvider::new(runner)
            .run_turn(
                request(Some("session-1"), WorkspaceAccess::ReadOnly),
                mpsc::unbounded_channel().0,
                CancellationSignal::never(),
            )
            .await
            .expect("turn");
        assert!(outcome.cancelled);
        assert_eq!(outcome.session_id.as_deref(), Some("session-1"));
    }

    #[tokio::test]
    async fn detects_installation_and_sign_in() {
        let missing = ScriptedRunner::single(Vec::new(), LineProcessExit::ProgramNotFound);
        let summary = ClaudeProvider::new(missing).detect().await.expect("detect");
        assert_eq!(summary.status, DetectionStatus::NotInstalled);

        let signed_in = ScriptedRunner::new(vec![
            (vec!["2.1.286 (Claude Code)"], finished(true)),
            (
                vec![r#"{"loggedIn": true, "authMethod": "claude.ai", "email": "x@example.com"}"#],
                finished(true),
            ),
        ]);
        let summary = ClaudeProvider::new(signed_in.clone())
            .detect()
            .await
            .expect("detect");
        assert_eq!(summary.status, DetectionStatus::Available);
        assert_eq!(
            summary.detail,
            "2.1.286 (Claude Code). Signed in through the Claude Code CLI."
        );
        let commands = signed_in.commands.lock().expect("commands");
        assert_eq!(commands[0].arguments, ["--version"]);
        assert_eq!(commands[1].arguments, ["auth", "status", "--json"]);
    }

    #[tokio::test]
    async fn reports_unconfirmed_sign_in_as_unknown() {
        for (lines, exit) in [
            (vec![r#"{"loggedIn": false}"#], finished(true)),
            (vec!["not json"], finished(true)),
            (Vec::new(), finished(false)),
        ] {
            let runner = ScriptedRunner::new(vec![
                (vec!["2.1.286 (Claude Code)"], finished(true)),
                (lines, exit),
            ]);
            let summary = ClaudeProvider::new(runner).detect().await.expect("detect");
            assert_eq!(summary.status, DetectionStatus::Unknown);
            assert!(summary.detail.contains("claude auth login"));
        }
    }

    #[tokio::test]
    async fn lists_the_static_alias_catalog() {
        let models = ClaudeProvider::new(ScriptedRunner::new(Vec::new()))
            .list_models()
            .await
            .expect("models");
        let ids: Vec<&str> = models.iter().map(|model| model.id.as_str()).collect();
        assert_eq!(ids, ["fable", "opus", "sonnet", "haiku"]);
        assert!(models.iter().all(|model| !model.is_default));
        assert_eq!(models[0].reasoning_efforts, EFFORTS);
        assert!(models[3].reasoning_efforts.is_empty());
    }

    const USAGE_TEXT: &str =
        "You are currently using your subscription to power your Claude Code usage

Current session: 14% used · resets Oct 1, 1:40pm (America/Sao_Paulo)
Current week (all models): 20% used · resets Oct 5, 12am (America/Sao_Paulo)
Current week (Fable): 0% used · resets Oct 5, 12am (America/Sao_Paulo)

What's contributing to your limits usage?
Last 24h · 760 requests · 16 sessions
  78% of your usage came from subagent-heavy sessions";

    fn sao_paulo(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<FixedOffset> {
        FixedOffset::west_opt(3 * 3600)
            .expect("offset")
            .with_ymd_and_hms(year, month, day, hour, minute, 0)
            .single()
            .expect("time")
    }

    #[test]
    fn parses_usage_windows_with_scopes_and_local_reset_times() {
        let now = sao_paulo(2026, 10, 1, 11, 42);
        let windows = parse_usage_windows(USAGE_TEXT, &now).expect("windows");
        let utc = |date: DateTime<FixedOffset>| Some(date.with_timezone(&Utc));
        assert_eq!(
            windows,
            [
                UsageWindow {
                    duration_minutes: Some(300),
                    scope: None,
                    used_percent: 14,
                    resets_at: utc(sao_paulo(2026, 10, 1, 13, 40)),
                },
                UsageWindow {
                    duration_minutes: Some(10080),
                    scope: None,
                    used_percent: 20,
                    resets_at: utc(sao_paulo(2026, 10, 5, 0, 0)),
                },
                UsageWindow {
                    duration_minutes: Some(10080),
                    scope: Some("Fable".into()),
                    used_percent: 0,
                    resets_at: utc(sao_paulo(2026, 10, 5, 0, 0)),
                },
            ]
        );
    }

    #[test]
    fn reads_reset_times_across_years_and_without_dates() {
        let new_year = sao_paulo(2026, 12, 31, 22, 0);
        assert_eq!(
            parse_reset("Jan 2, 12am (America/Sao_Paulo)", &new_year),
            Some(sao_paulo(2027, 1, 2, 0, 0).with_timezone(&Utc))
        );
        let morning = sao_paulo(2026, 10, 1, 9, 0);
        assert_eq!(
            parse_reset("1:40pm (America/Sao_Paulo)", &morning),
            Some(sao_paulo(2026, 10, 1, 13, 40).with_timezone(&Utc))
        );
        assert_eq!(
            parse_reset("8am", &sao_paulo(2026, 10, 1, 23, 0)),
            Some(sao_paulo(2026, 10, 2, 8, 0).with_timezone(&Utc))
        );
        for unreadable in ["soon", "Foo 3, 1pm", "13pm", "Oct 1, 1:75pm"] {
            assert_eq!(parse_reset(unreadable, &morning), None, "{unreadable}");
        }
    }

    #[test]
    fn keeps_windows_whose_reset_time_is_unreadable_and_rejects_reports_without_windows() {
        let now = sao_paulo(2026, 10, 1, 11, 42);
        let windows = parse_usage_windows("Current session: 100% used · resets later", &now)
            .expect("windows");
        assert_eq!(windows[0].used_percent, 100);
        assert_eq!(windows[0].resets_at, None);

        let result = parse_usage_windows("You are using an API key.\n", &now);
        assert!(
            matches!(result, Err(AppError::Provider(message)) if message.ends_with("You are using an API key."))
        );
    }

    #[tokio::test]
    async fn reads_usage_through_the_local_usage_command() {
        let runner = ScriptedRunner::new(vec![
            (
                vec![
                    r#"{"type":"system","subtype":"init","session_id":"s"}"#,
                    r#"{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0,"result":"Current session: 100% used · resets Oct 1, 1:40pm (America/Sao_Paulo)\nCurrent week (all models): 20% used · resets Oct 5, 12am (America/Sao_Paulo)"}"#,
                ],
                finished(true),
            ),
            (
                vec![r#"{"loggedIn": true, "subscriptionType": "team"}"#],
                finished(true),
            ),
        ]);
        let usage = ClaudeProvider::new(runner.clone())
            .read_usage()
            .await
            .expect("usage");
        assert_eq!(usage.provider_id, "claude-code");
        assert_eq!(usage.plan.as_deref(), Some("team"));
        assert!(usage.limit_reached);
        assert_eq!(usage.windows.len(), 2);

        let commands = runner.commands();
        assert_eq!(commands[0].arguments, USAGE_ARGUMENTS);
        assert_eq!(commands[0].stdin.as_deref(), Some("/usage"));
        assert_eq!(commands[1].arguments, ["auth", "status", "--json"]);
    }

    #[tokio::test]
    async fn reports_usage_command_failures() {
        let failed = ScriptedRunner::single(
            vec![r#"{"type":"result","is_error":true,"result":"Not logged in"}"#],
            finished(true),
        );
        let result = ClaudeProvider::new(failed).read_usage().await;
        assert!(
            matches!(result, Err(AppError::Provider(message)) if message == "Claude Code could not read usage: Not logged in")
        );

        let silent = ScriptedRunner::single(Vec::new(), finished(true));
        assert!(ClaudeProvider::new(silent).read_usage().await.is_err());

        let missing = ScriptedRunner::single(Vec::new(), LineProcessExit::ProgramNotFound);
        assert!(ClaudeProvider::new(missing).read_usage().await.is_err());
    }
}
