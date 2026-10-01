use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use super::{resolve_agent, RuntimeToolServices};
use crate::{
    application::GroupService,
    domain::{
        agents::PermissionLevel,
        conversations::MAX_MESSAGE_LENGTH,
        groups::{
            Group, NewGroup, MAX_GROUP_MEMBERS, MAX_GROUP_NAME_LENGTH, MAX_GROUP_TOPIC_LENGTH,
        },
    },
    error::{AppError, AppResult},
    tools::{parse_input, Tool, ToolRequest, ToolResult},
};

pub struct GroupCreateTool {
    services: RuntimeToolServices,
}

impl GroupCreateTool {
    pub fn new(services: RuntimeToolServices) -> Self {
        Self { services }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CreateInput {
    name: String,
    topic: String,
    members: Vec<String>,
    message: Option<String>,
}

#[async_trait]
impl Tool for GroupCreateTool {
    fn id(&self) -> &'static str {
        "group_create"
    }
    fn description(&self) -> &'static str {
        "Open a group conversation with other agents about one topic. You join it \
         automatically and the user can follow it. An opening message wakes the members to \
         answer one at a time; mention @Name to ask only some of them."
    }
    fn input_contract(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "maxLength": MAX_GROUP_NAME_LENGTH },
                "topic": { "type": "string", "maxLength": MAX_GROUP_TOPIC_LENGTH, "description": "What the group is for." },
                "members": {
                    "type": "array",
                    "items": { "type": "string", "description": "Agent name or id." },
                    "minItems": 1,
                    "maxItems": MAX_GROUP_MEMBERS
                },
                "message": { "type": "string", "maxLength": MAX_MESSAGE_LENGTH, "description": "Opening message to post as you." }
            },
            "required": ["name", "topic", "members"],
            "additionalProperties": false
        })
    }
    fn required_permission(&self) -> PermissionLevel {
        PermissionLevel::Allowed
    }
    fn action_id(&self) -> &'static str {
        "group.write"
    }
    async fn execute(&self, request: ToolRequest) -> AppResult<ToolResult> {
        let input: CreateInput = parse_input(request.input)?;
        let member_ids = input
            .members
            .iter()
            .map(|member| resolve_agent(self.services.agents.as_ref(), member).map(|a| a.id))
            .collect::<AppResult<Vec<_>>>()?;
        let group = self.services.groups.create_for_agent(
            request.context,
            NewGroup {
                name: input.name,
                topic: input.topic,
                member_ids,
            },
            input.message.as_deref(),
        )?;
        Ok(ToolResult {
            summary: format!("Group \"{}\" created.", group.name),
            output: json!({ "group": describe(&group) }),
        })
    }
}

pub struct GroupListTool {
    groups: Arc<GroupService>,
}

impl GroupListTool {
    pub fn new(groups: Arc<GroupService>) -> Self {
        Self { groups }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListInput {}

#[async_trait]
impl Tool for GroupListTool {
    fn id(&self) -> &'static str {
        "group_list"
    }
    fn description(&self) -> &'static str {
        "List the group conversations you belong to with their id, name, topic, and members."
    }
    fn input_contract(&self) -> Value {
        json!({ "type": "object", "properties": {}, "additionalProperties": false })
    }
    fn required_permission(&self) -> PermissionLevel {
        PermissionLevel::Allowed
    }
    fn action_id(&self) -> &'static str {
        "group.read"
    }
    async fn execute(&self, request: ToolRequest) -> AppResult<ToolResult> {
        let _: ListInput = parse_input(request.input)?;
        let groups: Vec<Value> = self
            .groups
            .list_for_agent(request.context.agent_id)?
            .iter()
            .map(describe)
            .collect();
        Ok(ToolResult {
            summary: format!("{} group(s).", groups.len()),
            output: json!({ "groups": groups }),
        })
    }
}

pub struct GroupPostTool {
    groups: Arc<GroupService>,
}

impl GroupPostTool {
    pub fn new(groups: Arc<GroupService>) -> Self {
        Self { groups }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PostInput {
    group: String,
    message: String,
}

#[async_trait]
impl Tool for GroupPostTool {
    fn id(&self) -> &'static str {
        "group_post"
    }
    fn description(&self) -> &'static str {
        "Post a message as you in another group you belong to, by name or id. Its members \
         answer one at a time; mention @Name to ask only some of them. Inside a group turn, \
         your normal reply is already posted to that group."
    }
    fn input_contract(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "group": { "type": "string", "description": "Group name or id." },
                "message": { "type": "string", "maxLength": MAX_MESSAGE_LENGTH }
            },
            "required": ["group", "message"],
            "additionalProperties": false
        })
    }
    fn required_permission(&self) -> PermissionLevel {
        PermissionLevel::Allowed
    }
    fn action_id(&self) -> &'static str {
        "group.write"
    }
    async fn execute(&self, request: ToolRequest) -> AppResult<ToolResult> {
        let input: PostInput = parse_input(request.input)?;
        let group = resolve_group(&self.groups, request.context.agent_id, &input.group)?;
        let message = self
            .groups
            .post_agent_message(request.context, group.id, &input.message)?;
        Ok(ToolResult {
            summary: format!("Posted in \"{}\".", group.name),
            output: json!({ "groupId": group.id, "messageId": message.id }),
        })
    }
}

/// Finds one of the agent's groups by id or by case-insensitive name.
fn resolve_group(groups: &GroupService, agent_id: Uuid, reference: &str) -> AppResult<Group> {
    let reference = reference.trim();
    let mine = groups.list_for_agent(agent_id)?;
    if let Ok(id) = reference.parse::<Uuid>() {
        return mine
            .into_iter()
            .find(|group| group.id == id)
            .ok_or_else(|| AppError::NotFound(format!("you are not in group {id}")));
    }
    let mut matches = mine
        .into_iter()
        .filter(|group| group.name.eq_ignore_ascii_case(reference));
    match (matches.next(), matches.next()) {
        (Some(group), None) => Ok(group),
        (Some(_), Some(_)) => Err(AppError::Validation(format!(
            "several of your groups are named \"{reference}\"; use the group id from group_list"
        ))),
        (None, _) => Err(AppError::NotFound(format!(
            "you are not in a group named \"{reference}\"; call group_list to see yours"
        ))),
    }
}

fn describe(group: &Group) -> Value {
    json!({
        "id": group.id,
        "name": group.name,
        "topic": group.topic,
        "memberIds": group.member_ids,
    })
}
