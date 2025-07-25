use crate::services::embedding::{BaseEmbedding, EmbeddingStats};
use crate::config::OllamaConfig;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct OllamaEmbedding {
    config: OllamaConfig,
    client: reqwest::Client,
    batch_size: usize,
    max_retries: usize,
    timeout: Duration,
}

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    embedding: Option<Vec<f32>>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelsResponse {
    models: Option<Vec<OllamaModel>>,
}

#[derive(Debug, Deserialize)]
struct OllamaModel {
    name: String,
}

impl OllamaEmbedding {
    pub fn new(config: OllamaConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            client,
            batch_size: 10,
            max_retries: 3,
            timeout: Duration::from_secs(30),
        }
    }

    pub async fn health_check(&self) -> Result<serde_json::Value> {
        let response = self
            .client
            .get(&format!("{}/api/tags", self.config.api_url))
            .timeout(Duration::from_secs(5))
            .send()
            .await?;

        let models_response: OllamaModelsResponse = response.json().await?;
        let model_exists = models_response
            .models
            .as_ref()
            .map(|models| models.iter().any(|model| model.name.contains(&self.config.model)))
            .unwrap_or(false);

        Ok(serde_json::json!({
            "status": "healthy",
            "apiUrl": self.config.api_url,
            "model": self.config.model,
            "modelAvailable": model_exists,
            "models": models_response.models.unwrap_or_default().into_iter().map(|m| m.name).collect::<Vec<_>>()
        }))
    }
}

#[async_trait]
impl BaseEmbedding for OllamaEmbedding {
    async fn embed_single(&self, message: &str) -> Result<Vec<f32>> {
        if message.trim().is_empty() {
            return Err(anyhow!("Message must be a non-empty string"));
        }

        let operation = || async {
                tracing::info!(
                    "🔤 Generating embedding for message: {}...",
                    &message[..message.len().min(50)]
                );

                let request = OllamaRequest {
                    model: self.config.model.clone(),
                    prompt: message.trim().to_string(),
                };

                let response = self
                    .client
                    .post(&format!("{}/api/embeddings", self.config.api_url))
                    .json(&request)
                    .timeout(self.timeout)
                    .send()
                    .await?;

                if !response.status().is_success() {
                    return Err(anyhow!(
                        "Ollama API error: {} - {}",
                        response.status(),
                        response.text().await.unwrap_or_default()
                    ));
                }

                let ollama_response: OllamaResponse = response.json().await?;

                let embedding = ollama_response
                    .embedding
                    .ok_or_else(|| anyhow!("No embedding in Ollama response"))?;

                tracing::info!(
                    "✅ Successfully generated embedding ({} dimensions)",
                    embedding.len()
                );

                Ok(embedding)
        };

        self.retry_operation(operation).await
    }

    fn get_batch_size(&self) -> usize {
        self.batch_size
    }

    fn get_max_retries(&self) -> usize {
        self.max_retries
    }

    fn get_timeout(&self) -> Duration {
        self.timeout
    }

    fn get_stats(&self) -> EmbeddingStats {
        EmbeddingStats {
            batch_size: self.batch_size,
            max_retries: self.max_retries,
            timeout_ms: self.timeout.as_millis() as u64,
        }
    }
}