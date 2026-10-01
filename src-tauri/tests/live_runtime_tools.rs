//! Live checks that the real provider CLIs reach the Open Bots MCP server. They make real
//! model calls on the signed-in account, so they are ignored by default:
//!
//! ```text
//! cargo test --test live_runtime_tools -- --ignored --nocapture
//! ```

use std::sync::Arc;

use open_bots_lib::{
    application::{MemoryService, TaskService, ToolService},
    domain::{
        agents::{Agent, IdentityColor, NewAgent, WorkspaceAccess},
        approvals::DefaultApprovalPolicy,
    },
    infrastructure::{
        database::{
            AgentRepository, Database, SqliteAgentRepository, SqliteEventRepository,
            SqliteMemoryRepository, SqliteTaskRepository,
        },
        mcp::McpListener,
        process::{TokioJsonRpcProcessClient, TokioLineProcessRunner},
    },
    providers::{
        AgentProvider, ClaudeProvider, CodexProvider, RuntimeToolsEndpoint, TurnEvent, TurnRequest,
    },
    runtime::{
        cancellation::CancellationSignal,
        event_bus::EventBus,
        turn_tokens::{TurnContext, TurnTokens},
    },
    tools::{
        runtime::{register_runtime_tools, RuntimeToolServices},
        ToolRegistry,
    },
};
use tokio::sync::mpsc;

const PROMPT: &str = "Call the Open Bots tool mcp__open_bots__memory_save exactly once with \
    the content \"live check\". If it is not listed directly, look for it under that name. \
    Then reply with the single word DONE.";

async fn run_live_turn(provider: &dyn AgentProvider, model: Option<&str>) {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database =
        Arc::new(Database::open(&directory.path().join("live.sqlite3")).expect("database"));
    let agents = Arc::new(SqliteAgentRepository::new(Arc::clone(&database)));
    let agent = Agent::create(NewAgent {
        name: "Live".into(),
        role: "Tester".into(),
        description: String::new(),
        provider_id: provider.id().into(),
        identity_color: IdentityColor::Indigo,
        workspace: directory.path().to_string_lossy().into_owned(),
        instructions: String::new(),
        model_selection: Default::default(),
    })
    .expect("agent");
    agents.save(&agent).expect("save agent");
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
            tasks,
        },
    )
    .expect("register tools");
    let tokens = TurnTokens::new();
    let listener = McpListener::bind().expect("bind");
    let url = listener.url();
    tokio::spawn(listener.serve(
        Arc::new(ToolService::new(registry, Arc::new(DefaultApprovalPolicy))),
        Arc::clone(&tokens),
    ));

    let token = tokens.issue(TurnContext {
        agent_id: agent.id,
        chain_depth: 0,
    });
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let outcome = provider
        .run_turn(
            TurnRequest {
                session_id: None,
                prompt: PROMPT.into(),
                workspace: directory.path().to_path_buf(),
                access: WorkspaceAccess::ReadOnly,
                model: model.map(str::to_owned),
                reasoning_effort: None,
                runtime_tools: Some(RuntimeToolsEndpoint { url, token }),
            },
            sender,
            CancellationSignal::never(),
        )
        .await;
    while let Ok(event) = receiver.try_recv() {
        match event {
            TurnEvent::Message { text } => println!("message: {text}"),
            TurnEvent::ActionStarted { summary, .. } => println!("action: {summary}"),
            TurnEvent::ActionCompleted {
                summary, succeeded, ..
            } => println!("action done ({succeeded}): {summary}"),
            TurnEvent::SessionStarted { .. } => {}
        }
    }
    println!("outcome: {outcome:?}");
    let saved = memories.list(agent.id).expect("memories");
    assert_eq!(
        saved.first().map(|memory| memory.content.as_str()),
        Some("live check"),
        "the provider did not save the memory through the MCP server"
    );
}

#[tokio::test]
#[ignore = "makes a real Claude Code model call"]
async fn claude_code_calls_runtime_tools() {
    let provider = ClaudeProvider::new(Arc::new(TokioLineProcessRunner));
    run_live_turn(&provider, Some("haiku")).await;
}

#[tokio::test]
#[ignore = "makes a real Codex model call"]
async fn codex_calls_runtime_tools() {
    let provider = CodexProvider::new(
        Arc::new(TokioLineProcessRunner),
        Arc::new(TokioJsonRpcProcessClient),
    );
    run_live_turn(&provider, None).await;
}
