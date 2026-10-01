use std::{process::Stdio, time::Duration};

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{ChildStdin, ChildStdout, Command},
};

use super::line_process::read_tail;
use crate::error::{AppError, AppResult};

/// One message written to a JSON-RPC server over stdin.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonRpcMessage {
    /// Waits for the matching response before the next message is written.
    Request {
        method: String,
        params: Option<Value>,
    },
    Notification {
        method: String,
        params: Option<Value>,
    },
}

/// A short-lived JSON-RPC conversation with a stdio server: the messages are sent in order,
/// then the process is stopped. Stdin stays open until every response arrives, because some
/// servers exit on end of input before answering.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonRpcSession {
    pub program: String,
    pub arguments: Vec<String>,
    pub messages: Vec<JsonRpcMessage>,
    pub timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JsonRpcExit {
    /// One entry per request, in the order the requests were sent.
    Completed(Vec<Result<Value, JsonRpcError>>),
    /// The server closed stdout before answering every request.
    Exited {
        code: Option<i32>,
        stderr_tail: String,
    },
    TimedOut,
    ProgramNotFound,
}

#[async_trait]
pub trait JsonRpcProcessClient: Send + Sync {
    async fn exchange(&self, session: JsonRpcSession) -> AppResult<JsonRpcExit>;
}

#[derive(Debug, Default)]
pub struct TokioJsonRpcProcessClient;

#[async_trait]
impl JsonRpcProcessClient for TokioJsonRpcProcessClient {
    async fn exchange(&self, session: JsonRpcSession) -> AppResult<JsonRpcExit> {
        let mut child = match Command::new(&session.program)
            .args(&session.arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
        {
            Ok(child) => child,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(JsonRpcExit::ProgramNotFound);
            }
            Err(error) => {
                return Err(AppError::Process(format!(
                    "{} could not be started: {error}",
                    session.program
                )));
            }
        };
        let (Some(mut stdin), Some(stdout), Some(stderr)) =
            (child.stdin.take(), child.stdout.take(), child.stderr.take())
        else {
            return Err(AppError::Process("process pipes are unavailable".into()));
        };
        let stderr_task = tokio::spawn(read_tail(stderr));
        let mut lines = BufReader::new(stdout).lines();

        let outcome = tokio::time::timeout(
            session.timeout,
            converse(&mut stdin, &mut lines, &session.messages),
        )
        .await;
        drop(stdin);
        let exit = match outcome {
            Ok(Ok(Some(responses))) => JsonRpcExit::Completed(responses),
            Ok(Ok(None)) => {
                let status = child
                    .wait()
                    .await
                    .map_err(|error| AppError::Process(format!("process wait failed: {error}")))?;
                let stderr_tail = stderr_task.await.unwrap_or_default();
                return Ok(JsonRpcExit::Exited {
                    code: status.code(),
                    stderr_tail,
                });
            }
            Ok(Err(error)) => {
                stop(&mut child, &session.program).await;
                stderr_task.abort();
                return Err(error);
            }
            Err(_) => JsonRpcExit::TimedOut,
        };
        stop(&mut child, &session.program).await;
        stderr_task.abort();
        Ok(exit)
    }
}

async fn stop(child: &mut tokio::process::Child, program: &str) {
    if let Err(error) = child.kill().await {
        tracing::warn!(%error, program, "JSON-RPC server could not be stopped");
    }
}

/// Returns `None` when the server closes stdout before every request is answered.
async fn converse(
    stdin: &mut ChildStdin,
    lines: &mut Lines<BufReader<ChildStdout>>,
    messages: &[JsonRpcMessage],
) -> AppResult<Option<Vec<Result<Value, JsonRpcError>>>> {
    let mut responses = Vec::new();
    let mut next_id: u64 = 1;
    for message in messages {
        let (payload, request_id) = match message {
            JsonRpcMessage::Request { method, params } => {
                let id = next_id;
                next_id += 1;
                (envelope(Some(id), method, params), Some(id))
            }
            JsonRpcMessage::Notification { method, params } => {
                (envelope(None, method, params), None)
            }
        };
        let mut line = serde_json::to_string(&payload)?;
        line.push('\n');
        stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|error| AppError::Process(format!("stdin write failed: {error}")))?;
        stdin
            .flush()
            .await
            .map_err(|error| AppError::Process(format!("stdin flush failed: {error}")))?;
        let Some(id) = request_id else {
            continue;
        };
        loop {
            let line = lines
                .next_line()
                .await
                .map_err(|error| AppError::Process(format!("stdout read failed: {error}")))?;
            let Some(line) = line else {
                return Ok(None);
            };
            if let Some(response) = parse_response(&line, id) {
                responses.push(response);
                break;
            }
        }
    }
    Ok(Some(responses))
}

fn envelope(id: Option<u64>, method: &str, params: &Option<Value>) -> Value {
    let mut payload = json!({ "jsonrpc": "2.0", "method": method });
    if let Some(id) = id {
        payload["id"] = json!(id);
    }
    if let Some(params) = params {
        payload["params"] = params.clone();
    }
    payload
}

/// Matches the response for `id`. Notifications, server requests, other responses and
/// non-JSON lines are skipped.
fn parse_response(line: &str, id: u64) -> Option<Result<Value, JsonRpcError>> {
    let value: Value = serde_json::from_str(line.trim()).ok()?;
    if value.get("method").is_some() || value.get("id")?.as_u64()? != id {
        return None;
    }
    if let Some(error) = value.get("error") {
        return Some(Err(JsonRpcError {
            code: error
                .get("code")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
            message: error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("the server reported an error")
                .to_owned(),
        }));
    }
    Some(Ok(value.get("result").cloned().unwrap_or(Value::Null)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_only_the_awaited_response() {
        assert_eq!(parse_response(r#"{"method":"note","id":1}"#, 1), None);
        assert_eq!(parse_response(r#"{"id":2,"result":{}}"#, 1), None);
        assert_eq!(parse_response("not json", 1), None);
        assert_eq!(
            parse_response(r#"{"id":1,"result":{"ok":true}}"#, 1),
            Some(Ok(json!({ "ok": true })))
        );
        assert_eq!(
            parse_response(r#"{"id":1,"error":{"code":-32601,"message":"unknown"}}"#, 1),
            Some(Err(JsonRpcError {
                code: -32601,
                message: "unknown".into()
            }))
        );
    }

    #[test]
    fn encodes_requests_and_notifications() {
        assert_eq!(
            envelope(Some(3), "model/list", &Some(json!({ "limit": 5 }))),
            json!({ "jsonrpc": "2.0", "id": 3, "method": "model/list", "params": { "limit": 5 } })
        );
        assert_eq!(
            envelope(None, "initialized", &None),
            json!({ "jsonrpc": "2.0", "method": "initialized" })
        );
    }

    #[cfg(unix)]
    mod process {
        use super::*;

        fn session(
            script: &str,
            messages: Vec<JsonRpcMessage>,
            timeout: Duration,
        ) -> JsonRpcSession {
            JsonRpcSession {
                program: "sh".into(),
                arguments: vec!["-c".into(), script.into()],
                messages,
                timeout,
            }
        }

        fn request(method: &str) -> JsonRpcMessage {
            JsonRpcMessage::Request {
                method: method.into(),
                params: None,
            }
        }

        /// Answers each request only after reading it, like a stdio app server.
        const ECHO_SERVER: &str = r#"while read -r line; do
  case "$line" in
    *'"id":1'*) echo '{"method":"progress"}'; echo '{"id":1,"result":{"first":true}}' ;;
    *'"id":2'*) echo '{"id":2,"error":{"code":-32600,"message":"rejected"}}' ;;
  esac
done"#;

        #[tokio::test]
        async fn exchanges_requests_in_order_and_skips_notifications() {
            let exit = TokioJsonRpcProcessClient
                .exchange(session(
                    ECHO_SERVER,
                    vec![
                        request("initialize"),
                        JsonRpcMessage::Notification {
                            method: "initialized".into(),
                            params: None,
                        },
                        request("account/read"),
                    ],
                    Duration::from_secs(5),
                ))
                .await
                .expect("exchange");
            assert_eq!(
                exit,
                JsonRpcExit::Completed(vec![
                    Ok(json!({ "first": true })),
                    Err(JsonRpcError {
                        code: -32600,
                        message: "rejected".into()
                    })
                ])
            );
        }

        #[tokio::test]
        async fn reports_servers_that_exit_before_answering() {
            let exit = TokioJsonRpcProcessClient
                .exchange(session(
                    "read -r line; echo failed >&2; exit 4",
                    vec![request("initialize")],
                    Duration::from_secs(5),
                ))
                .await
                .expect("exchange");
            assert_eq!(
                exit,
                JsonRpcExit::Exited {
                    code: Some(4),
                    stderr_tail: "failed\n".into()
                }
            );
        }

        #[tokio::test]
        async fn stops_silent_servers_after_the_timeout() {
            let exit = tokio::time::timeout(
                Duration::from_secs(5),
                TokioJsonRpcProcessClient.exchange(session(
                    "sleep 30",
                    vec![request("initialize")],
                    Duration::from_millis(200),
                )),
            )
            .await
            .expect("returned promptly")
            .expect("exchange");
            assert_eq!(exit, JsonRpcExit::TimedOut);
        }

        #[tokio::test]
        async fn reports_missing_programs() {
            let mut missing = session("", vec![request("initialize")], Duration::from_secs(1));
            missing.program = "open-bots-missing-program".into();
            let exit = TokioJsonRpcProcessClient
                .exchange(missing)
                .await
                .expect("exchange");
            assert_eq!(exit, JsonRpcExit::ProgramNotFound);
        }
    }
}
