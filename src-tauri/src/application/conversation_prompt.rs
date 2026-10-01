use crate::domain::{agents::Agent, memories::AgentMemory};

/// Builds the prompt for the first turn of a provider session. Providers without a
/// verified system-instruction channel receive the agent's identity, instructions, and
/// memories as a leading context block; later turns resume the session and send only
/// the user's message.
pub fn first_turn_prompt(agent: &Agent, memories: &[AgentMemory], message: &str) -> String {
    let mut context = format!(
        "You are {name}, working as {role} in an Open Bots workspace.",
        name = agent.name,
        role = agent.role
    );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agents::{IdentityColor, NewAgent};
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
        assert_eq!(
            prompt,
            "<agent_context>\nYou are Atlas, working as Backend Engineer in an Open Bots workspace.\
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
}
