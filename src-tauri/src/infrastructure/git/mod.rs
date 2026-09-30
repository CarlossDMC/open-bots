use async_trait::async_trait;
use std::path::Path;

use crate::error::AppResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitStatus {
    pub branch: String,
    pub staged_files: Vec<String>,
    pub changed_files: Vec<String>,
    pub untracked_files: Vec<String>,
}

#[async_trait]
pub trait GitService: Send + Sync {
    async fn status(&self, workspace: &Path) -> AppResult<GitStatus>;
    async fn diff(&self, workspace: &Path, staged: bool) -> AppResult<String>;
    async fn create_branch(&self, workspace: &Path, name: &str) -> AppResult<()>;
    async fn commit(&self, workspace: &Path, message: &str) -> AppResult<String>;
}

/// Placeholder boundary. Git CLI execution will be implemented behind this type.
#[derive(Debug, Default)]
pub struct GitCliService;
