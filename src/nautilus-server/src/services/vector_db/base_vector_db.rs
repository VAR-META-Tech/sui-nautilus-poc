use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorData {
    pub id: String,
    pub vector: Vec<f32>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub score: f32,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageResult {
    pub id: String,
    pub success: bool,
    pub error: Option<String>,
}

#[async_trait]
pub trait BaseVectorDb: Send + Sync {
    async fn connect(&self) -> Result<()>;
    async fn disconnect(&self) -> Result<()>;
    
    async fn store(&self, id: &str, vector: &[f32], metadata: &HashMap<String, serde_json::Value>) -> Result<StorageResult>;
    
    async fn store_batch(&self, vectors: &[VectorData]) -> Result<Vec<StorageResult>> {
        let batch_size = self.get_batch_size();
        let mut results = Vec::new();

        tracing::info!("📊 Starting batch storage for {} vectors (batch size: {})", vectors.len(), batch_size);

        for (i, chunk) in vectors.chunks(batch_size).enumerate() {
            let batch_num = i + 1;
            let total_batches = (vectors.len() + batch_size - 1) / batch_size;
            
            tracing::info!("📊 Storing batch {}/{} ({} vectors)", batch_num, total_batches, chunk.len());

            let batch_results = self.store_batch_internal(chunk).await;
            results.extend(batch_results);

            if i + 1 < total_batches {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        tracing::info!("✅ Completed batch storage for {} vectors", results.len());
        Ok(results)
    }

    async fn store_batch_internal(&self, batch: &[VectorData]) -> Vec<StorageResult>;

    async fn search(&self, query_vector: &[f32], limit: usize, filter: Option<&serde_json::Value>) -> Result<Vec<SearchResult>>;
    
    async fn delete_by_id(&self, id: &str) -> Result<StorageResult>;
    
    async fn delete_batch(&self, ids: &[String]) -> Result<Vec<StorageResult>> {
        let mut results = Vec::new();
        
        for id in ids {
            let result = self.delete_by_id(id).await.unwrap_or(StorageResult {
                id: id.clone(),
                success: false,
                error: Some("Failed to delete".to_string()),
            });
            results.push(result);
        }
        
        Ok(results)
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
                        tracing::warn!("⚠️  Retrying in {}ms...", delay.as_millis());
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }

    fn is_connected(&self) -> bool;
    fn get_batch_size(&self) -> usize;
    fn get_max_retries(&self) -> usize;
    fn get_timeout(&self) -> Duration;
}