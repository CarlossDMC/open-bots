use serde_json::{json, Map, Value};

use crate::{runtime::turn_tokens::TurnContext, tools::ToolHost};

/// Newest first; the first entry is offered when the client asks for an unknown version.
pub const SUPPORTED_PROTOCOL_VERSIONS: [&str; 4] =
    ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;

/// Handles one JSON-RPC message. Requests get a response; notifications and client
/// responses return `None`.
pub async fn handle_message(
    host: &dyn ToolHost,
    context: TurnContext,
    message: &Value,
) -> Option<Value> {
    let Some(object) = message.as_object() else {
        return Some(error(
            Value::Null,
            INVALID_REQUEST,
            "expected a JSON-RPC object",
        ));
    };
    let id = object.get("id").cloned();
    let Some(method) = object.get("method").and_then(Value::as_str) else {
        // A response to a server request; this server sends none, so there is nothing to do.
        return id
            .is_none()
            .then(|| error(Value::Null, INVALID_REQUEST, "missing method"));
    };
    let id = id?;
    let params = object.get("params").cloned().unwrap_or(Value::Null);
    let outcome = match method {
        "initialize" => Ok(initialize(&params)),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(list_tools(host)),
        "tools/call" => call_tool(host, context, &params).await,
        _ => Err((
            METHOD_NOT_FOUND,
            format!("method {method} is not supported"),
        )),
    };
    Some(match outcome {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err((code, text)) => error(id, code, &text),
    })
}

/// A JSON-RPC parse error for a body that is not JSON.
pub(super) fn parse_error() -> Value {
    error(Value::Null, PARSE_ERROR, "request body is not valid JSON")
}

fn initialize(params: &Value) -> Value {
    let requested = params.get("protocolVersion").and_then(Value::as_str);
    let version = requested
        .filter(|version| SUPPORTED_PROTOCOL_VERSIONS.contains(version))
        .unwrap_or(SUPPORTED_PROTOCOL_VERSIONS[0]);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": "open-bots", "version": env!("CARGO_PKG_VERSION") },
        "instructions": "Open Bots runtime tools. They act for the agent whose turn is running."
    })
}

fn list_tools(host: &dyn ToolHost) -> Value {
    let tools: Vec<Value> = host
        .describe()
        .into_iter()
        .map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "inputSchema": tool.input_schema,
            })
        })
        .collect();
    json!({ "tools": tools })
}

async fn call_tool(
    host: &dyn ToolHost,
    context: TurnContext,
    params: &Value,
) -> Result<Value, (i64, String)> {
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return Err((INVALID_PARAMS, "tools/call needs a tool name".into()));
    };
    let arguments = match params.get("arguments") {
        None | Some(Value::Null) => Value::Object(Map::new()),
        Some(value @ Value::Object(_)) => value.clone(),
        Some(_) => return Err((INVALID_PARAMS, "tool arguments must be an object".into())),
    };
    let outcome = host.call(context, name, arguments).await;
    let mut result = json!({
        "content": [{ "type": "text", "text": outcome.text }],
        "isError": outcome.is_error,
    });
    if let Some(structured @ Value::Object(_)) = outcome.structured {
        result["structuredContent"] = structured;
    }
    Ok(result)
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use uuid::Uuid;

    use super::*;
    use crate::tools::{ToolCallOutcome, ToolDescriptor};

    struct EchoHost;

    #[async_trait]
    impl ToolHost for EchoHost {
        fn describe(&self) -> Vec<ToolDescriptor> {
            vec![ToolDescriptor {
                name: "echo".into(),
                description: "Echo".into(),
                input_schema: json!({ "type": "object" }),
            }]
        }
        async fn call(&self, context: TurnContext, name: &str, input: Value) -> ToolCallOutcome {
            ToolCallOutcome {
                text: format!("{name} for {}", context.agent_id),
                structured: Some(input),
                is_error: false,
            }
        }
    }

    fn context() -> TurnContext {
        TurnContext {
            agent_id: Uuid::nil(),
            chain_depth: 0,
            group_id: None,
        }
    }

    async fn send(message: Value) -> Option<Value> {
        handle_message(&EchoHost, context(), &message).await
    }

    #[tokio::test]
    async fn negotiates_a_supported_protocol_version() {
        let response = send(json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "protocolVersion": "2025-06-18", "capabilities": {} } }))
        .await
        .expect("response");
        assert_eq!(response["result"]["protocolVersion"], "2025-06-18");
        assert_eq!(
            response["result"]["capabilities"]["tools"]["listChanged"],
            false
        );

        let unknown = send(json!({ "jsonrpc": "2.0", "id": 2, "method": "initialize",
            "params": { "protocolVersion": "1999-01-01" } }))
        .await
        .expect("response");
        assert_eq!(
            unknown["result"]["protocolVersion"],
            SUPPORTED_PROTOCOL_VERSIONS[0]
        );
    }

    #[tokio::test]
    async fn lists_and_calls_tools() {
        let listed = send(json!({ "jsonrpc": "2.0", "id": "a", "method": "tools/list" }))
            .await
            .expect("response");
        assert_eq!(listed["result"]["tools"][0]["name"], "echo");
        assert_eq!(listed["id"], "a");

        let called = send(json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": { "name": "echo", "arguments": { "value": 1 } } }))
        .await
        .expect("response");
        assert_eq!(called["result"]["isError"], false);
        assert_eq!(called["result"]["structuredContent"]["value"], 1);
        assert_eq!(called["result"]["content"][0]["type"], "text");
    }

    #[tokio::test]
    async fn ignores_notifications_and_rejects_unknown_methods() {
        assert_eq!(
            send(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })).await,
            None
        );
        let unknown = send(json!({ "jsonrpc": "2.0", "id": 4, "method": "resources/list" }))
            .await
            .expect("response");
        assert_eq!(unknown["error"]["code"], METHOD_NOT_FOUND);
        let bad = send(json!({ "jsonrpc": "2.0", "id": 5, "method": "tools/call",
            "params": { "name": "echo", "arguments": [1] } }))
        .await
        .expect("response");
        assert_eq!(bad["error"]["code"], INVALID_PARAMS);
        let batch = send(json!([{ "jsonrpc": "2.0", "id": 6, "method": "ping" }]))
            .await
            .expect("response");
        assert_eq!(batch["error"]["code"], INVALID_REQUEST);
    }
}
