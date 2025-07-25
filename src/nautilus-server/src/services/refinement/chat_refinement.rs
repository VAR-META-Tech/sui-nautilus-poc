use crate::services::refinement::{BaseRefinement, Message, Reactions, RefinedData};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ChatRefinement {
    sort_by_date: bool,
    filter_empty_messages: bool,
}

impl ChatRefinement {
    pub fn new() -> Self {
        Self {
            sort_by_date: true,
            filter_empty_messages: true,
        }
    }

    fn transform_message(&self, msg: &Value, user_id: &str, chat_id: &str) -> Result<Message> {
        if !msg.is_object() {
            return Err(anyhow::anyhow!("Message must be an object"));
        }

        let id = msg
            .get("id")
            .and_then(|v| {
                v.as_str().map(|s| s.to_string())
                    .or_else(|| v.as_u64().map(|n| n.to_string()))
            })
            .unwrap_or_else(|| "unknown".to_string());

        let from_id = msg
            .get("fromId")
            .and_then(|v| v.get("userId"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let date = msg
            .get("date")
            .and_then(|v| v.as_u64())
            .map(|timestamp| {
                chrono::DateTime::from_timestamp(timestamp as i64, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_else(|| timestamp.to_string())
            });

        let edit_date = msg
            .get("editDate")
            .and_then(|v| v.as_u64())
            .map(|timestamp| {
                chrono::DateTime::from_timestamp(timestamp as i64, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_else(|| timestamp.to_string())
            });

        let message_text = msg
            .get("message")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let out = msg.get("out").and_then(|v| v.as_bool());

        let reactions = self.transform_reactions(msg.get("reactions"));

        Ok(Message {
            id,
            user_id: user_id.to_string(),
            chat_id: chat_id.to_string(),
            from_id,
            date,
            edit_date,
            message: message_text,
            out,
            reactions,
        })
    }

    fn transform_reactions(&self, reactions_value: Option<&Value>) -> Option<Reactions> {
        reactions_value.and_then(|reactions| {
            let emoji = reactions
                .get("recentReactions")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.get("reaction"))
                .and_then(|v| v.get("emoticon"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let count = reactions
                .get("results")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.get("count"))
                .and_then(|v| v.as_u64())
                .map(|n| n as u32);

            if emoji.is_some() || count.is_some() {
                Some(Reactions { emoji, count })
            } else {
                None
            }
        })
    }
}

impl Default for ChatRefinement {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BaseRefinement for ChatRefinement {
    async fn refine_data(&self, raw_data: &Value) -> Result<RefinedData> {
        self.validate_input(raw_data)?;

        tracing::info!("📝 Starting chat data refinement...");

        let mut messages = Vec::new();

        let user_id = raw_data
            .get("user")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown_user");

        if let Some(chats) = raw_data.get("chats").and_then(|v| v.as_array()) {
            tracing::info!("📊 Processing {} chat conversations...", chats.len());

            for chat in chats {
                let chat_id = chat
                    .get("chat_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown_chat");

                if let Some(contents) = chat.get("contents").and_then(|v| v.as_array()) {
                    tracing::info!("💬 Processing chat with {} messages...", contents.len());

                    for msg in contents {
                        match self.transform_message(msg, user_id, chat_id) {
                            Ok(transformed_message) => {
                                messages.push(transformed_message);
                            }
                            Err(e) => {
                                tracing::warn!("⚠️  Failed to transform message: {}", e);
                            }
                        }
                    }
                }
            }
        }

        // Sort messages by date if enabled
        if self.sort_by_date {
            self.sort_messages(&mut messages);
        }

        // Filter empty messages if enabled
        if self.filter_empty_messages {
            messages = self.filter_messages(messages);
        }

        let stats = self.get_stats(&messages);
        tracing::info!("✅ Chat refinement completed. Stats: {:?}", stats);

        Ok(RefinedData {
            revision: raw_data.get("revision").cloned(),
            user: raw_data.get("user").cloned(),
            messages,
            refinement_stats: stats,
        })
    }
}