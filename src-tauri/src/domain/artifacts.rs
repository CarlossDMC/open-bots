use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub id: Uuid,
    pub task_id: Option<Uuid>,
    pub agent_id: Uuid,
    pub name: String,
    pub media_type: String,
    pub relative_path: String,
    pub byte_size: u64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactReference {
    pub artifact_id: Uuid,
    pub name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ArtifactStoreError {
    #[error("artifact storage failed: {0}")]
    Storage(String),
}

pub trait ArtifactStore: Send + Sync {
    fn save_metadata(&self, artifact: &Artifact) -> Result<(), ArtifactStoreError>;
    fn find(&self, id: Uuid) -> Result<Option<Artifact>, ArtifactStoreError>;
}
