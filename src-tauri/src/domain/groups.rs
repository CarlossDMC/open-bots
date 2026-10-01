use chrono::{DateTime, SubsecRound, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{conversations::MAX_MESSAGE_LENGTH, DomainError, DomainResult};

pub const MAX_GROUP_NAME_LENGTH: usize = 80;
pub const MAX_GROUP_TOPIC_LENGTH: usize = 2_000;
pub const MAX_GROUP_MEMBERS: usize = 12;

/// Who wrote a group message or created a group.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum GroupAuthor {
    User,
    #[serde(rename_all = "camelCase")]
    Agent {
        agent_id: Uuid,
    },
    /// Runtime notices such as failures or skipped turns; never attributed to a member.
    System,
}

impl GroupAuthor {
    pub fn agent_id(&self) -> Option<Uuid> {
        match self {
            Self::Agent { agent_id } => Some(*agent_id),
            Self::User | Self::System => None,
        }
    }
}

/// A member queued to take a turn in the group.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RoundSpeaker {
    pub agent_id: Uuid,
    /// Turns already chained before this one; members answering the user start at 0.
    pub chain_depth: u32,
}

/// The members still to answer in the group, in speaking order. Members answer one at a
/// time so each sees the replies before it.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GroupRound {
    pub queue: Vec<RoundSpeaker>,
    /// The wake queued for the current speaker; other wakes for this group are stale.
    pub active_wake_id: Option<Uuid>,
}

impl GroupRound {
    pub fn is_active(&self) -> bool {
        !self.queue.is_empty()
    }

    pub fn current(&self) -> Option<RoundSpeaker> {
        self.queue.first().copied()
    }

    /// Queues members to answer at `chain_depth`. A member already queued keeps its place
    /// and takes the shallower depth, so a user message resets the chain for everyone.
    pub fn enqueue(&mut self, agent_ids: &[Uuid], chain_depth: u32) {
        for agent_id in agent_ids {
            match self
                .queue
                .iter_mut()
                .find(|speaker| speaker.agent_id == *agent_id)
            {
                Some(speaker) => speaker.chain_depth = speaker.chain_depth.min(chain_depth),
                None => self.queue.push(RoundSpeaker {
                    agent_id: *agent_id,
                    chain_depth,
                }),
            }
        }
    }

    /// Ends the current speaker's turn. Returns false when `agent_id` was not speaking.
    pub fn finish(&mut self, agent_id: Uuid) -> bool {
        if self.current().map(|speaker| speaker.agent_id) != Some(agent_id) {
            return false;
        }
        self.queue.remove(0);
        self.active_wake_id = None;
        true
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.active_wake_id = None;
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NewGroup {
    pub name: String,
    pub topic: String,
    pub member_ids: Vec<Uuid>,
}

/// A conversation between the user and several agents about one topic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: Uuid,
    pub name: String,
    pub topic: String,
    /// Members in speaking order.
    pub member_ids: Vec<Uuid>,
    pub created_by: GroupAuthor,
    pub round: GroupRound,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Group {
    pub fn create(input: NewGroup, created_by: GroupAuthor) -> DomainResult<Self> {
        let name = input.name.trim();
        if name.is_empty() || name.chars().count() > MAX_GROUP_NAME_LENGTH {
            return Err(DomainError::Validation(format!(
                "group names must contain 1 to {MAX_GROUP_NAME_LENGTH} characters"
            )));
        }
        let topic = input.topic.trim();
        if topic.is_empty() || topic.chars().count() > MAX_GROUP_TOPIC_LENGTH {
            return Err(DomainError::Validation(format!(
                "group topics must contain 1 to {MAX_GROUP_TOPIC_LENGTH} characters"
            )));
        }
        let mut member_ids = Vec::with_capacity(input.member_ids.len());
        for id in input.member_ids {
            if member_ids.contains(&id) {
                return Err(DomainError::Validation(
                    "a group cannot list the same member twice".into(),
                ));
            }
            member_ids.push(id);
        }
        if member_ids.is_empty() || member_ids.len() > MAX_GROUP_MEMBERS {
            return Err(DomainError::Validation(format!(
                "groups need 1 to {MAX_GROUP_MEMBERS} members"
            )));
        }
        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            name: name.to_owned(),
            topic: topic.to_owned(),
            member_ids,
            created_by,
            round: GroupRound::default(),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn is_member(&self, agent_id: Uuid) -> bool {
        self.member_ids.contains(&agent_id)
    }

    /// Members who should answer a message, in speaking order. Mentioned members answer
    /// when there are any; otherwise everyone but the author answers when
    /// `everyone_by_default` is set, and nobody does when it is not.
    pub fn speakers_for(
        &self,
        author: GroupAuthor,
        content: &str,
        member_names: &[(Uuid, String)],
        everyone_by_default: bool,
    ) -> Vec<Uuid> {
        let author_id = author.agent_id();
        let candidates: Vec<(Uuid, String)> = member_names
            .iter()
            .filter(|(id, _)| self.is_member(*id) && Some(*id) != author_id)
            .cloned()
            .collect();
        let mentioned = mentioned_members(content, &candidates);
        let chosen: Vec<Uuid> = if !mentioned.is_empty() {
            mentioned
        } else if everyone_by_default {
            candidates.iter().map(|(id, _)| *id).collect()
        } else {
            Vec::new()
        };
        // Keep the group's speaking order rather than the order of mentions.
        self.member_ids
            .iter()
            .copied()
            .filter(|id| chosen.contains(id))
            .collect()
    }
}

/// Members named with `@Name` in `content`, ignoring case. Longer names win, so `@Ana Maria`
/// does not also mention `Ana`.
pub fn mentioned_members(content: &str, members: &[(Uuid, String)]) -> Vec<Uuid> {
    let text = content.to_lowercase();
    let mut by_length: Vec<&(Uuid, String)> = members.iter().collect();
    by_length.sort_by_key(|(_, name)| std::cmp::Reverse(name.chars().count()));
    let mut taken: Vec<(usize, usize)> = Vec::new();
    let mut mentioned = Vec::new();
    for (id, name) in by_length {
        let needle = format!("@{}", name.trim().to_lowercase());
        if needle.len() == 1 {
            continue;
        }
        for (start, _) in text.match_indices(&needle) {
            let end = start + needle.len();
            let boundary = text[end..]
                .chars()
                .next()
                .is_none_or(|next| !next.is_alphanumeric() && next != '_');
            let overlaps = taken.iter().any(|&(s, e)| start < e && s < end);
            if boundary && !overlaps {
                taken.push((start, end));
                if !mentioned.contains(id) {
                    mentioned.push(*id);
                }
            }
        }
    }
    mentioned
}

/// One entry in a group conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GroupMessage {
    pub id: Uuid,
    pub group_id: Uuid,
    pub author: GroupAuthor,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

impl GroupMessage {
    pub fn new(group_id: Uuid, author: GroupAuthor, content: &str) -> DomainResult<Self> {
        let content = content.trim();
        if content.is_empty() {
            return Err(DomainError::Validation("message cannot be empty".into()));
        }
        if content.chars().count() > MAX_MESSAGE_LENGTH {
            return Err(DomainError::Validation(format!(
                "message cannot exceed {MAX_MESSAGE_LENGTH} characters"
            )));
        }
        Ok(Self {
            id: Uuid::new_v4(),
            group_id,
            author,
            content: content.to_owned(),
            // Millisecond precision matches what persistence keeps.
            created_at: Utc::now().trunc_subsecs(3),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(members: &[Uuid]) -> Group {
        Group::create(
            NewGroup {
                name: " Release ".into(),
                topic: " Ship 0.4 ".into(),
                member_ids: members.to_vec(),
            },
            GroupAuthor::User,
        )
        .expect("group")
    }

    #[test]
    fn validates_name_topic_and_members() {
        let id = Uuid::new_v4();
        let created = group(&[id]);
        assert_eq!(created.name, "Release");
        assert_eq!(created.topic, "Ship 0.4");
        let new = |name: &str, topic: &str, members: Vec<Uuid>| {
            Group::create(
                NewGroup {
                    name: name.into(),
                    topic: topic.into(),
                    member_ids: members,
                },
                GroupAuthor::User,
            )
        };
        assert!(new(" ", "topic", vec![id]).is_err());
        assert!(new("name", "", vec![id]).is_err());
        assert!(new("name", "topic", vec![]).is_err());
        assert!(new("name", "topic", vec![id, id]).is_err());
        let many = (0..=MAX_GROUP_MEMBERS).map(|_| Uuid::new_v4()).collect();
        assert!(new("name", "topic", many).is_err());
    }

    #[test]
    fn rounds_run_in_order_and_keep_one_entry_per_member() {
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let mut round = GroupRound::default();
        round.enqueue(&[a, b], 2);
        round.enqueue(&[b], 0);
        assert_eq!(round.queue.len(), 2);
        assert_eq!(round.queue[1].chain_depth, 0);
        assert!(!round.finish(b));
        round.active_wake_id = Some(Uuid::new_v4());
        assert!(round.finish(a));
        assert_eq!(round.active_wake_id, None);
        assert_eq!(round.current().map(|s| s.agent_id), Some(b));
        round.clear();
        assert!(!round.is_active());
    }

    #[test]
    fn finds_mentions_ignoring_case_and_preferring_longer_names() {
        let (ana, ana_maria, bob) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let members = vec![
            (ana, "Ana".to_string()),
            (ana_maria, "Ana Maria".to_string()),
            (bob, "Bob".to_string()),
        ];
        assert_eq!(
            mentioned_members("@ana maria, what do you think?", &members),
            vec![ana_maria]
        );
        assert_eq!(
            mentioned_members("@BOB and @Ana please", &members),
            vec![ana, bob]
        );
        assert!(mentioned_members("@Bobby and email bob@x.dev", &members).is_empty());
    }

    #[test]
    fn user_messages_reach_everyone_unless_members_are_mentioned() {
        let (a, b, c) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let group = group(&[a, b, c]);
        let names = vec![
            (a, "Atlas".to_string()),
            (b, "Nova".to_string()),
            (c, "Orion".to_string()),
        ];
        assert_eq!(
            group.speakers_for(GroupAuthor::User, "Status?", &names, true),
            vec![a, b, c]
        );
        assert_eq!(
            group.speakers_for(GroupAuthor::User, "@Orion then @Atlas", &names, true),
            vec![a, c]
        );
    }

    #[test]
    fn agent_replies_wake_only_mentioned_members_and_never_the_author() {
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let group = group(&[a, b]);
        let names = vec![(a, "Atlas".to_string()), (b, "Nova".to_string())];
        let author = GroupAuthor::Agent { agent_id: a };
        assert!(group
            .speakers_for(author, "Done here.", &names, false)
            .is_empty());
        assert_eq!(
            group.speakers_for(author, "@Atlas @Nova review?", &names, false),
            vec![b]
        );
        assert_eq!(group.speakers_for(author, "Kickoff", &names, true), vec![b]);
    }

    #[test]
    fn rejects_empty_messages() {
        assert!(GroupMessage::new(Uuid::new_v4(), GroupAuthor::User, " ").is_err());
    }
}
