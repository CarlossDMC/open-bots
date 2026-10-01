use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    domain::inbox::{validate_max_chain_turns, DEFAULT_MAX_CHAIN_TURNS},
    error::AppResult,
    infrastructure::database::SettingsRepository,
};

const MAX_CHAIN_TURNS_KEY: &str = "runtime.maxChainTurns";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSettings {
    /// Turns that may chain from one user message or routine without the user stepping in.
    pub max_chain_turns: u32,
}

pub struct SettingsService {
    settings: Arc<dyn SettingsRepository>,
}

impl SettingsService {
    pub fn new(settings: Arc<dyn SettingsRepository>) -> Self {
        Self { settings }
    }

    pub fn runtime(&self) -> AppResult<RuntimeSettings> {
        let stored = self
            .settings
            .get(MAX_CHAIN_TURNS_KEY)?
            .and_then(|value| value.as_u64())
            .and_then(|value| u32::try_from(value).ok())
            .and_then(|value| validate_max_chain_turns(value).ok());
        Ok(RuntimeSettings {
            max_chain_turns: stored.unwrap_or(DEFAULT_MAX_CHAIN_TURNS),
        })
    }

    pub fn update_runtime(&self, input: RuntimeSettings) -> AppResult<RuntimeSettings> {
        let max_chain_turns = validate_max_chain_turns(input.max_chain_turns)?;
        self.settings
            .set(MAX_CHAIN_TURNS_KEY, &json!(max_chain_turns))?;
        tracing::info!(max_chain_turns, "runtime settings updated");
        Ok(RuntimeSettings { max_chain_turns })
    }
}
