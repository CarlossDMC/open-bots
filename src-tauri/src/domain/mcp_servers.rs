use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{DomainError, DomainResult};

/// Longest MCP server name accepted.
const MAX_MCP_SERVER_NAME_LENGTH: usize = 128;
/// Most MCP servers the catalog holds for one provider.
pub const MAX_CATALOG_SERVERS: usize = 64;

/// Trims and validates an MCP server name as the provider reports it, such as `github` or
/// `claude.ai Atlassian`. Names reach provider command lines, so only configuration-style
/// characters pass.
pub fn normalize_mcp_server_name(name: &str) -> DomainResult<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(DomainError::Validation(
            "MCP server names cannot be empty".into(),
        ));
    }
    if name.len() > MAX_MCP_SERVER_NAME_LENGTH {
        return Err(DomainError::Validation(format!(
            "MCP server names must contain at most {MAX_MCP_SERVER_NAME_LENGTH} characters"
        )));
    }
    if !name
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || " ._-".contains(character))
    {
        return Err(DomainError::Validation(
            "MCP server names may only contain letters, digits, spaces, '.', '_' and '-'".into(),
        ));
    }
    Ok(name.to_owned())
}

/// Trims, validates, and deduplicates server names, keeping their order.
pub fn normalize_mcp_server_names(names: Vec<String>, limit: usize) -> DomainResult<Vec<String>> {
    let mut servers: Vec<String> = Vec::new();
    for name in names {
        if name.trim().is_empty() {
            continue;
        }
        let name = normalize_mcp_server_name(&name)?;
        if !servers.contains(&name) {
            servers.push(name);
        }
    }
    if servers.len() > limit {
        return Err(DomainError::Validation(format!(
            "at most {limit} MCP servers can be selected"
        )));
    }
    Ok(servers)
}

/// An MCP server from a provider's own configuration that the user made available to the
/// agents of that provider. Only the name is kept: the server's command, URL, headers, and
/// credentials stay in the provider's configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpCatalogEntry {
    pub provider_id: String,
    pub name: String,
    pub added_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_server_names() {
        assert_eq!(
            normalize_mcp_server_name(" claude.ai Atlassian ").expect("valid"),
            "claude.ai Atlassian"
        );
        assert!(normalize_mcp_server_name("  ").is_err());
        assert!(normalize_mcp_server_name("git\"hub").is_err());
        assert!(normalize_mcp_server_name(&"a".repeat(129)).is_err());
    }

    #[test]
    fn deduplicates_names_and_enforces_the_limit() {
        let names = normalize_mcp_server_names(
            vec![
                "github".into(),
                String::new(),
                " github ".into(),
                "sentry".into(),
            ],
            4,
        )
        .expect("valid");
        assert_eq!(names, ["github", "sentry"]);
        assert!(normalize_mcp_server_names(vec!["a".into(), "b".into()], 1).is_err());
    }
}
