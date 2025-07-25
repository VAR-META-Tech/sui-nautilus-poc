use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefinementStats {
    pub total_messages: usize,
    pub messages_with_text: usize,
    pub messages_with_reactions: usize,
    pub date_range: DateRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start: Option<String>,
    pub end: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub user_id: String,
    pub chat_id: String,
    pub from_id: Option<String>,
    pub date: Option<String>,
    pub edit_date: Option<String>,
    pub message: Option<String>,
    pub out: Option<bool>,
    pub reactions: Option<Reactions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reactions {
    pub emoji: Option<String>,
    pub count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefinedData {
    pub revision: Option<serde_json::Value>,
    pub user: Option<serde_json::Value>,
    pub messages: Vec<Message>,
    pub refinement_stats: RefinementStats,
}

#[async_trait]
pub trait BaseRefinement: Send + Sync {
    async fn refine_data(&self, raw_data: &serde_json::Value) -> Result<RefinedData>;

    fn validate_input(&self, data: &serde_json::Value) -> Result<()> {
        if !data.is_object() {
            return Err(anyhow::anyhow!("Input data must be an object"));
        }
        Ok(())
    }

    fn sort_messages(&self, messages: &mut [Message]) {
        messages.sort_by(|a, b| {
            let date_a = a.date.as_deref().unwrap_or("").to_string();
            let date_b = b.date.as_deref().unwrap_or("").to_string();
            date_a.cmp(&date_b)
        });
    }

    fn filter_messages(&self, messages: Vec<Message>) -> Vec<Message> {
        messages
            .into_iter()
            .filter(|msg| {
                msg.message
                    .as_ref()
                    .map(|m| !m.trim().is_empty())
                    .unwrap_or(false)
            })
            .collect()
    }

    fn get_stats(&self, messages: &[Message]) -> RefinementStats {
        let messages_with_text = messages
            .iter()
            .filter(|msg| {
                msg.message
                    .as_ref()
                    .map(|m| !m.trim().is_empty())
                    .unwrap_or(false)
            })
            .count();

        let messages_with_reactions = messages
            .iter()
            .filter(|msg| msg.reactions.is_some())
            .count();

        let date_range = self.get_date_range(messages);

        RefinementStats {
            total_messages: messages.len(),
            messages_with_text,
            messages_with_reactions,
            date_range,
        }
    }

    fn get_date_range(&self, messages: &[Message]) -> DateRange {
        let valid_dates: Vec<&str> = messages
            .iter()
            .filter_map(|msg| msg.date.as_deref())
            .filter(|date| !date.is_empty())
            .collect();

        if valid_dates.is_empty() {
            return DateRange {
                start: None,
                end: None,
            };
        }

        let mut sorted_dates = valid_dates.clone();
        sorted_dates.sort();

        DateRange {
            start: sorted_dates.first().map(|s| s.to_string()),
            end: sorted_dates.last().map(|s| s.to_string()),
        }
    }
}