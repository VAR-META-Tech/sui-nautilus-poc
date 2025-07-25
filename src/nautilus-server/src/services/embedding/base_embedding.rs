use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResult {
    pub message: String,
    pub embedding: Option<Vec<f32>>,
    pub success: bool,
    pub error: Option<String>,
    pub dimensions: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingStats {
    pub batch_size: usize,
    pub max_retries: usize,
    pub timeout_ms: u64,
}

#[async_trait]
pub trait BaseEmbedding: Send + Sync {
    async fn embed_single(&self, message: &str) -> Result<Vec<f32>>;

    async fn embed_batch(&self, messages: &[String]) -> Result<Vec<EmbeddingResult>> {
        let batch_size = self.get_batch_size();
        let mut results = Vec::new();

        tracing::info!(
            "🔤 Starting batch embedding for {} messages (batch size: {})",
            messages.len(),
            batch_size
        );

        for (i, chunk) in messages.chunks(batch_size).enumerate() {
            let batch_num = i + 1;
            let total_batches = (messages.len() + batch_size - 1) / batch_size;
            
            tracing::info!(
                "🔤 Processing batch {}/{} ({} messages)",
                batch_num,
                total_batches,
                chunk.len()
            );

            let batch_results = self.process_batch(chunk).await;
            results.extend(batch_results);

            if i + 1 < total_batches {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        tracing::info!("✅ Completed batch embedding for {} messages", results.len());
        Ok(results)
    }

    async fn process_batch(&self, batch: &[String]) -> Vec<EmbeddingResult> {
        let mut results = Vec::new();

        for message in batch {
            match self.embed_single(message).await {
                Ok(embedding) => {
                    results.push(EmbeddingResult {
                        message: message.clone(),
                        embedding: Some(embedding.clone()),
                        success: true,
                        error: None,
                        dimensions: Some(embedding.len()),
                    });
                }
                Err(error) => {
                    tracing::error!("❌ Failed to embed message: {}", error);
                    results.push(EmbeddingResult {
                        message: message.clone(),
                        embedding: None,
                        success: false,
                        error: Some(error.to_string()),
                        dimensions: None,
                    });
                }
            }
        }

        results
    }

    async fn retry_operation<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut + Send,
        Fut: std::future::Future<Output = Result<T>> + Send,
        T: Send,
    {
        let max_retries = self.get_max_retries();
        let mut last_error = None;

        for attempt in 0..max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    last_error = Some(error);
                    if attempt < max_retries - 1 {
                        let delay = Duration::from_millis(2_u64.pow(attempt as u32) * 1000);
                        tracing::warn!(
                            "⚠️  Attempt {} failed, retrying in {}ms...",
                            attempt + 1,
                            delay.as_millis()
                        );
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }

    fn get_batch_size(&self) -> usize;
    fn get_max_retries(&self) -> usize;
    fn get_timeout(&self) -> Duration;
    fn get_stats(&self) -> EmbeddingStats;
}