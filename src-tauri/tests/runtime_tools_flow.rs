use std::sync::Arc;

use open_bots_lib::{
    application::{MemoryService, TaskService, ToolService},
    domain::{
        agents::{Agent, IdentityColor, NewAgent},
        approvals::DefaultApprovalPolicy,
        tasks::TaskStatus,
    },
    infrastructure::{
        database::{
            AgentRepository, Database, SqliteAgentRepository, SqliteEventRepository,
            SqliteMemoryRepository, SqliteTaskRepository,
        },
        mcp::McpListener,
    },
    runtime::{
        event_bus::EventBus,
        turn_tokens::{TurnContext, TurnTokens},
    },
    tools::{
        runtime::{register_runtime_tools, RuntimeToolServices},
        ToolRegistry,
    },
};
use serde_json::{json, Value};
use tempfile::TempDir;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

struct TestHarness {
    _directory: TempDir,
    url: String,
    tokens: Arc<TurnTokens>,
    atlas: Agent,
    nova: Agent,
    memories: Arc<MemoryService>,
    tasks: Arc<TaskService>,
}

fn agent(agents: &SqliteAgentRepository, name: &str) -> Agent {
    let agent = Agent::create(NewAgent {
        name: name.into(),
        role: "Engineer".into(),
        description: String::new(),
        provider_id: "mock".into(),
        identity_color: IdentityColor::Indigo,
        workspace: format!("/work/{name}"),
        instructions: String::new(),
        model_selection: Default::default(),
    })
    .expect("agent");
    agents.save(&agent).expect("save agent");
    agent
}

impl TestHarness {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database =
            Arc::new(Database::open(&directory.path().join("tools.sqlite3")).expect("database"));
        let agents = Arc::new(SqliteAgentRepository::new(Arc::clone(&database)));
        let atlas = agent(&agents, "Atlas");
        let nova = agent(&agents, "Nova");
        let events = Arc::new(SqliteEventRepository::new(Arc::clone(&database)));
        let event_bus = EventBus::new(64);
        let memories = Arc::new(MemoryService::new(
            Arc::new(SqliteMemoryRepository::new(Arc::clone(&database))),
            agents.clone(),
            events.clone(),
            event_bus.clone(),
        ));
        let tasks = Arc::new(TaskService::new(
            Arc::new(SqliteTaskRepository::new(database)),
            agents.clone(),
            events,
            event_bus,
        ));
        let mut registry = ToolRegistry::default();
        register_runtime_tools(
            &mut registry,
            &RuntimeToolServices {
                agents,
                memories: Arc::clone(&memories),
                tasks: Arc::clone(&tasks),
            },
        )
        .expect("register tools");
        let host = Arc::new(ToolService::new(registry, Arc::new(DefaultApprovalPolicy)));
        let tokens = TurnTokens::new();
        let listener = McpListener::bind().expect("bind");
        let url = listener.url();
        tokio::spawn(listener.serve(host, Arc::clone(&tokens)));
        Self {
            _directory: directory,
            url,
            tokens,
            atlas,
            nova,
            memories,
            tasks,
        }
    }

    fn token_for(&self, agent: &Agent) -> String {
        self.tokens.issue(TurnContext {
            agent_id: agent.id,
            chain_depth: 0,
        })
    }

    async fn call(&self, token: &str, name: &str, arguments: Value) -> Value {
        let (status, body) = post(
            &self.url,
            Some(token),
            &json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                "params": { "name": name, "arguments": arguments } }),
        )
        .await;
        assert_eq!(status, 200, "{body}");
        serde_json::from_str::<Value>(&body).expect("json")["result"].clone()
    }
}

/// Minimal HTTP/1.1 client; the server answers with a fixed-length body and closes.
async fn post(url: &str, token: Option<&str>, body: &Value) -> (u16, String) {
    let address = url
        .strip_prefix("http://")
        .and_then(|rest| rest.strip_suffix("/mcp"))
        .expect("address");
    let mut stream = TcpStream::connect(address).await.expect("connect");
    let body = body.to_string();
    let authorization = token
        .map(|token| format!("Authorization: Bearer {token}\r\n"))
        .unwrap_or_default();
    let request = format!(
        "POST /mcp HTTP/1.1\r\nHost: {address}\r\n{authorization}Content-Type: application/json\r\n\
         Accept: application/json, text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).await.expect("write");
    let mut response = String::new();
    stream.read_to_string(&mut response).await.expect("read");
    let status = response
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .expect("status");
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_owned())
        .unwrap_or_default();
    (status, body)
}

#[tokio::test]
async fn rejects_requests_without_a_live_token() {
    let harness = TestHarness::new();
    let ping = json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" });
    assert_eq!(post(&harness.url, None, &ping).await.0, 401);
    assert_eq!(post(&harness.url, Some("guess"), &ping).await.0, 401);

    let token = harness.token_for(&harness.atlas);
    assert_eq!(post(&harness.url, Some(&token), &ping).await.0, 200);
    harness.tokens.revoke(&token);
    assert_eq!(post(&harness.url, Some(&token), &ping).await.0, 401);
}

#[tokio::test]
async fn initializes_and_lists_the_runtime_tools() {
    let harness = TestHarness::new();
    let token = harness.token_for(&harness.atlas);
    let (status, body) = post(
        &harness.url,
        Some(&token),
        &json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "protocolVersion": "2025-06-18", "capabilities": {},
                "clientInfo": { "name": "test", "version": "1" } } }),
    )
    .await;
    assert_eq!(status, 200);
    let initialized: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(initialized["result"]["protocolVersion"], "2025-06-18");

    let notification = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
    assert_eq!(post(&harness.url, Some(&token), &notification).await.0, 202);

    let (_, body) = post(
        &harness.url,
        Some(&token),
        &json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
    )
    .await;
    let listed: Value = serde_json::from_str(&body).expect("json");
    let names: Vec<&str> = listed["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    assert_eq!(
        names,
        [
            "agent_list",
            "memory_save",
            "task_create",
            "task_list",
            "task_update"
        ]
    );
}

#[tokio::test]
async fn tools_act_for_the_calling_agent() {
    let harness = TestHarness::new();
    let atlas_token = harness.token_for(&harness.atlas);

    let created = harness
        .call(
            &atlas_token,
            "task_create",
            json!({ "title": "Review the API", "assignee": "nova" }),
        )
        .await;
    assert_eq!(created["isError"], false, "{created}");
    let task_id = created["structuredContent"]["id"]
        .as_str()
        .expect("task id")
        .parse()
        .expect("uuid");
    let task = harness.tasks.find(task_id).expect("task");
    assert_eq!(task.assigned_agent_id, Some(harness.nova.id));
    assert_eq!(task.created_by_agent_id, Some(harness.atlas.id));

    let saved = harness
        .call(
            &atlas_token,
            "memory_save",
            json!({ "content": "The user prefers small PRs." }),
        )
        .await;
    assert_eq!(saved["isError"], false);
    assert_eq!(
        harness.memories.list(harness.atlas.id).expect("memories")[0].content,
        "The user prefers small PRs."
    );

    let listed = harness.call(&atlas_token, "agent_list", json!({})).await;
    assert_eq!(listed["structuredContent"]["agents"][0]["name"], "Nova");

    let nova_token = harness.token_for(&harness.nova);
    let finished = harness
        .call(
            &nova_token,
            "task_update",
            json!({ "taskId": task_id, "status": "completed", "result": "Looks good" }),
        )
        .await;
    assert_eq!(finished["isError"], false, "{finished}");
    assert_eq!(
        harness.tasks.find(task_id).expect("task").status,
        TaskStatus::Completed
    );
}

#[tokio::test]
async fn reports_invalid_input_and_foreign_tasks_as_tool_errors() {
    let harness = TestHarness::new();
    let atlas_token = harness.token_for(&harness.atlas);
    let invalid = harness
        .call(
            &atlas_token,
            "task_create",
            json!({ "name": "wrong field" }),
        )
        .await;
    assert_eq!(invalid["isError"], true);
    assert!(invalid["content"][0]["text"]
        .as_str()
        .expect("text")
        .contains("invalid tool input"));

    let created = harness
        .call(&atlas_token, "task_create", json!({ "title": "Private" }))
        .await;
    let task_id = created["structuredContent"]["id"].clone();
    let nova_token = harness.token_for(&harness.nova);
    let foreign = harness
        .call(
            &nova_token,
            "task_update",
            json!({ "taskId": task_id, "status": "cancelled" }),
        )
        .await;
    assert_eq!(foreign["isError"], true);

    let unknown = harness.call(&atlas_token, "shell_run", json!({})).await;
    assert_eq!(unknown["isError"], true);
}
