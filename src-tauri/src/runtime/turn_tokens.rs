use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use uuid::Uuid;

/// Who a runtime-tool call acts for: the agent whose turn issued the token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnContext {
    pub agent_id: Uuid,
    /// Chained turns behind the turn that holds the token.
    pub chain_depth: u32,
    /// The group the turn answers in; absent for the agent's direct conversation.
    pub group_id: Option<Uuid>,
}

/// Bearer tokens for the local MCP server, one per running turn. A token only works while
/// its turn runs, so a provider process cannot act for an agent after the turn ends.
#[derive(Default)]
pub struct TurnTokens {
    tokens: Mutex<HashMap<String, TurnContext>>,
}

impl TurnTokens {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Issues a token with 244 random bits from two v4 UUIDs.
    pub fn issue(&self, context: TurnContext) -> String {
        let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        if let Ok(mut tokens) = self.tokens.lock() {
            tokens.insert(token.clone(), context);
        }
        token
    }

    pub fn resolve(&self, token: &str) -> Option<TurnContext> {
        self.tokens.lock().ok()?.get(token).copied()
    }

    pub fn revoke(&self, token: &str) {
        if let Ok(mut tokens) = self.tokens.lock() {
            tokens.remove(token);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_resolve_until_revoked() {
        let tokens = TurnTokens::new();
        let context = TurnContext {
            agent_id: Uuid::new_v4(),
            chain_depth: 2,
            group_id: None,
        };
        let token = tokens.issue(context);
        assert_eq!(token.len(), 64);
        assert_eq!(tokens.resolve(&token), Some(context));
        assert_eq!(tokens.resolve("guess"), None);
        tokens.revoke(&token);
        assert_eq!(tokens.resolve(&token), None);
    }
}
