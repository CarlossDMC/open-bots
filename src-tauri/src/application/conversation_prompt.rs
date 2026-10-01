use crate::domain::{
    agents::{Agent, WorkspaceAccess},
    memories::AgentMemory,
};

/// Builds the prompt for the first turn of a provider session. Providers without a
/// verified system-instruction channel receive a runtime preamble followed by the
/// agent's identity, instructions, and memories as a leading context block; later turns
/// resume the session and send only the user's message.
pub fn first_turn_prompt(agent: &Agent, memories: &[AgentMemory], message: &str) -> String {
    let mut context = runtime_preamble(agent);
    context.push_str(&format!(
        "\n\nYou are {name}, working as {role} in an Open Bots workspace.",
        name = agent.name,
        role = agent.role
    ));
    if !agent.description.is_empty() {
        context.push_str(&format!("\nResponsibilities: {}", agent.description));
    }
    if !agent.instructions.is_empty() {
        context.push_str(&format!("\n\nInstructions:\n{}", agent.instructions));
    }
    if !memories.is_empty() {
        context.push_str("\n\nThings to remember:");
        // Oldest first reads naturally; the repository returns newest first.
        for memory in memories.iter().rev() {
            context.push_str(&format!("\n- {}", memory.content));
        }
    }
    format!("<agent_context>\n{context}\n</agent_context>\n\n{message}")
}

/// Describes how Open Bots runs the agent. It states only behavior the runtime enforces
/// today, so it must change when tools, approvals, or events reach the provider.
fn runtime_preamble(agent: &Agent) -> String {
    let access = match agent.permissions.workspace_access() {
        WorkspaceAccess::ReadOnly => "read-only: inspect files but do not modify them",
        WorkspaceAccess::WorkspaceWrite => {
            "workspace-write: you may edit files and run commands in the workspace"
        }
    };
    format!(
        "<runtime>\nYou are a persistent agent run by Open Bots, a local desktop runtime. \
         The user talks to you through the Open Bots app, and this conversation may continue \
         across many turns.\nEach turn runs non-interactively: you cannot ask for permission \
         or wait for input mid-turn. When something is ambiguous, state your assumption and \
         proceed, or end the turn with a question.\nWorkspace: {workspace}\nAccess: {access}\n\
         The identity and instructions below are defined by the user and take precedence over \
         general defaults.\n</runtime>",
        workspace = agent.workspace,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agents::{IdentityColor, NewAgent, PermissionLevel};
    use uuid::Uuid;

    fn agent(instructions: &str) -> Agent {
        Agent::create(NewAgent {
            name: "Atlas".into(),
            role: "Backend Engineer".into(),
            description: String::new(),
            provider_id: "codex".into(),
            identity_color: IdentityColor::Indigo,
            workspace: "/workspace".into(),
            instructions: instructions.into(),
            model_selection: Default::default(),
        })
        .expect("agent")
    }

    #[test]
    fn includes_instructions_and_memories_oldest_first() {
        let agent = agent("Prefer small changes.");
        let older = AgentMemory::create(Uuid::new_v4(), "Uses pnpm").expect("memory");
        let newer = AgentMemory::create(Uuid::new_v4(), "Deploys on Fridays").expect("memory");
        let prompt = first_turn_prompt(&agent, &[newer, older], "Fix the build");
        let (_, identity) = prompt
            .split_once("</runtime>\n\n")
            .expect("runtime preamble");
        assert_eq!(
            identity,
            "You are Atlas, working as Backend Engineer in an Open Bots workspace.\
             \n\nInstructions:\nPrefer small changes.\n\nThings to remember:\n- Uses pnpm\
             \n- Deploys on Fridays\n</agent_context>\n\nFix the build"
        );
    }

    #[test]
    fn omits_empty_sections() {
        let prompt = first_turn_prompt(&agent(""), &[], "Hello");
        assert!(!prompt.contains("Instructions"));
        assert!(!prompt.contains("remember"));
        assert!(prompt.ends_with("</agent_context>\n\nHello"));
    }

    #[test]
    fn runtime_preamble_leads_and_describes_workspace_and_access() {
        let mut agent = agent("Prefer small changes.");
        let prompt = first_turn_prompt(&agent, &[], "Hello");
        assert!(prompt.starts_with("<agent_context>\n<runtime>\n"));
        assert!(prompt.find("</runtime>") < prompt.find("Instructions:"));
        assert!(prompt.contains("Workspace: /workspace"));
        assert!(prompt.contains("non-interactively"));

        agent.permissions.filesystem = PermissionLevel::WorkspaceOnly;
        agent.permissions.shell = PermissionLevel::Allowed;
        assert!(first_turn_prompt(&agent, &[], "Hello").contains("Access: workspace-write"));
        agent.permissions.shell = PermissionLevel::ApprovalRequired;
        assert!(first_turn_prompt(&agent, &[], "Hello").contains("Access: read-only"));
    }
}
