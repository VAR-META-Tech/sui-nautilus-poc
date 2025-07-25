use crate::services::vector_db::{BaseVectorDb, SearchResult, StorageResult, VectorData};
use crate::config::QdrantConfig;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[derive(Debug)]
pub struct QdrantService {
    config: QdrantConfig,
    client: reqwest::Client,
    connected: AtomicBool,
    vector_size: Option<usize>,
    batch_size: usize,
    max_retries: usize,
    timeout: Duration,
}

#[derive(Debug, Clone, Serialize)]
struct QdrantPoint {
    id: String,
    vector: Vec<f32>,
    payload: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct QdrantUpsertRequest {
    points: Vec<QdrantPoint>,
}

#[derive(Debug, Serialize)]
struct QdrantSearchRequest {
    vector: Vec<f32>,
    limit: usize,
    with_payload: bool,
    with_vector: bool,
    filter: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct QdrantSearchResponse {
    result: Vec<QdrantSearchResult>,
}

#[derive(Debug, Deserialize)]
struct QdrantSearchResult {
    id: String,
    score: f32,
    payload: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct QdrantCollectionsResponse {
    result: QdrantCollectionsList,
}

#[derive(Debug, Deserialize)]
struct QdrantCollectionsList {
    collections: Vec<QdrantCollection>,
}

#[derive(Debug, Deserialize)]
struct QdrantCollection {
    name: String,
}

#[derive(Debug, Serialize)]
struct QdrantCreateCollectionRequest {
    vectors: QdrantVectorConfig,
}

#[derive(Debug, Serialize)]
struct QdrantVectorConfig {
    size: usize,
    distance: String,
}

impl QdrantService {
    pub fn new(config: QdrantConfig) -> Self {
        let mut client_builder = reqwest::Client::builder().timeout(Duration::from_secs(10));

        if let Some(api_key) = &config.api_key {
            let mut headers = reqwest::header::HeaderMap::new();
            let mut auth_value = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", api_key))
                .expect("Invalid API key format");
            auth_value.set_sensitive(true);
            headers.insert(reqwest::header::AUTHORIZATION, auth_value);
            client_builder = client_builder.default_headers(headers);
        }

        let client = client_builder.build().expect("Failed to create HTTP client");

        Self {
            config,
            client,
            connected: AtomicBool::new(false),
            vector_size: None,
            batch_size: 100,
            max_retries: 3,
            timeout: Duration::from_secs(10),
        }
    }

    async fn ensure_collection(&self) -> Result<()> {
        let collections_url = format!("{}/collections", self.config.url);
        let response = self.client.get(&collections_url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to get collections: {}", response.status()));
        }

        let collections_response: QdrantCollectionsResponse = response.json().await?;
        let collection_exists = collections_response
            .result
            .collections
            .iter()
            .any(|col| col.name == self.config.collection_name);

        if !collection_exists {
            if let Some(vector_size) = self.vector_size {
                tracing::info!(
                    "📦 Creating Qdrant collection: {} with vector size {}",
                    self.config.collection_name,
                    vector_size
                );

                let create_url = format!("{}/collections/{}", self.config.url, self.config.collection_name);
                let create_request = QdrantCreateCollectionRequest {
                    vectors: QdrantVectorConfig {
                        size: vector_size,
                        distance: "Cosine".to_string(),
                    },
                };

                let response = self
                    .client
                    .put(&create_url)
                    .json(&create_request)
                    .send()
                    .await?;

                if !response.status().is_success() {
                    return Err(anyhow!("Failed to create collection: {}", response.status()));
                }

                tracing::info!("✅ Created Qdrant collection: {}", self.config.collection_name);
            } else {
                tracing::info!(
                    "⏳ Collection {} will be created when first vector is stored",
                    self.config.collection_name
                );
            }
        } else {
            tracing::info!("✅ Qdrant collection already exists: {}", self.config.collection_name);
        }

        Ok(())
    }
}

#[async_trait]
impl BaseVectorDb for QdrantService {
    async fn connect(&self) -> Result<()> {
        tracing::info!("🔗 Connecting to Qdrant at {} ...", self.config.url);

        let health_url = format!("{}/", self.config.url);
        let response = self.client.get(&health_url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("Qdrant health check failed: {}", response.status()));
        }

        tracing::info!("✅ Qdrant health check passed");

        self.ensure_collection().await?;

        self.connected.store(true, Ordering::Relaxed);
        tracing::info!(
            "✅ Successfully connected to Qdrant collection: {}",
            self.config.collection_name
        );

        Ok(())
    }

    async fn disconnect(&self) -> Result<()> {
        self.connected.store(false, Ordering::Relaxed);
        tracing::info!("✅ Disconnected from Qdrant");
        Ok(())
    }

    async fn store(&self, id: &str, vector: &[f32], metadata: &HashMap<String, serde_json::Value>) -> Result<StorageResult> {
        if !self.is_connected() {
            self.connect().await?;
        }

        if vector.is_empty() {
            return Ok(StorageResult {
                id: id.to_string(),
                success: false,
                error: Some("Vector cannot be empty".to_string()),
            });
        }

        let operation = || {
            Box::pin(async {
                let mut payload = metadata.clone();
                payload.insert("timestamp".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));

                let point = QdrantPoint {
                    id: uuid::Uuid::new_v4().to_string(),
                    vector: vector.to_vec(),
                    payload,
                };

                let request = QdrantUpsertRequest {
                    points: vec![point],
                };

                let upsert_url = format!("{}/collections/{}/points", self.config.url, self.config.collection_name);
                let response = self
                    .client
                    .put(&upsert_url)
                    .json(&request)
                    .send()
                    .await?;

                if !response.status().is_success() {
                    return Err(anyhow!("Failed to store vector: {}", response.status()));
                }

                tracing::info!("✅ Stored vector {} in Qdrant", id);
                Ok(StorageResult {
                    id: id.to_string(),
                    success: true,
                    error: None,
                })
            })
        };

        self.retry_operation(operation).await
    }

    async fn store_batch_internal(&self, batch: &[VectorData]) -> Vec<StorageResult> {
        if !self.is_connected() {
            if let Err(e) = self.connect().await {
                return batch.iter().map(|v| StorageResult {
                    id: v.id.clone(),
                    success: false,
                    error: Some(format!("Connection failed: {}", e)),
                }).collect();
            }
        }

        let operation = || {
            Box::pin(async {
                let mut points = Vec::new();

                for item in batch {
                    if item.vector.is_empty() {
                        continue;
                    }

                    // Validate vector values
                    for (i, &val) in item.vector.iter().enumerate() {
                        if !val.is_finite() {
                            return Err(anyhow!("Invalid vector value at index {}: {}", i, val));
                        }
                    }

                    let mut payload = item.metadata.clone();
                    payload.insert("timestamp".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
                    payload.insert("original_id".to_string(), serde_json::Value::String(item.id.clone()));

                    points.push(QdrantPoint {
                        id: uuid::Uuid::new_v4().to_string(),
                        vector: item.vector.clone(),
                        payload,
                    });
                }

                if points.is_empty() {
                    return Ok(Vec::new());
                }

                tracing::info!("🔍 Upserting {} points to collection '{}'", points.len(), self.config.collection_name);

                let request = QdrantUpsertRequest { points: points.clone() };
                let upsert_url = format!("{}/collections/{}/points", self.config.url, self.config.collection_name);
                
                let response = self
                    .client
                    .put(&upsert_url)
                    .json(&request)
                    .send()
                    .await?;

                if !response.status().is_success() {
                    return Err(anyhow!("Failed to store batch: {}", response.status()));
                }

                tracing::info!("✅ Stored batch of {} vectors in Qdrant", points.len());
                Ok(points.into_iter().map(|point| StorageResult {
                    id: point.id,
                    success: true,
                    error: None,
                }).collect())
            })
        };

        match self.retry_operation(operation).await {
            Ok(results) => results,
            Err(e) => batch.iter().map(|v| StorageResult {
                id: v.id.clone(),
                success: false,
                error: Some(e.to_string()),
            }).collect(),
        }
    }

    async fn search(&self, query_vector: &[f32], limit: usize, filter: Option<&serde_json::Value>) -> Result<Vec<SearchResult>> {
        if !self.is_connected() {
            self.connect().await?;
        }

        if query_vector.is_empty() {
            return Err(anyhow!("Query vector cannot be empty"));
        }

        let operation = || {
            Box::pin(async {
                let search_request = QdrantSearchRequest {
                    vector: query_vector.to_vec(),
                    limit,
                    with_payload: true,
                    with_vector: false,
                    filter: filter.cloned(),
                };

                let search_url = format!("{}/collections/{}/points/search", self.config.url, self.config.collection_name);
                let response = self
                    .client
                    .post(&search_url)
                    .json(&search_request)
                    .send()
                    .await?;

                if !response.status().is_success() {
                    return Err(anyhow!("Search failed: {}", response.status()));
                }

                let search_response: QdrantSearchResponse = response.json().await?;

                let results: Vec<SearchResult> = search_response
                    .result
                    .into_iter()
                    .map(|result| SearchResult {
                        id: result.id,
                        score: result.score,
                        metadata: result.payload.unwrap_or_default(),
                    })
                    .collect();

                tracing::info!("🔍 Found {} similar vectors", results.len());
                Ok(results)
            })
        };

        self.retry_operation(operation).await
    }

    async fn delete_by_id(&self, id: &str) -> Result<StorageResult> {
        if !self.is_connected() {
            self.connect().await?;
        }

        let operation = || {
            Box::pin(async {
                let delete_url = format!("{}/collections/{}/points/delete", self.config.url, self.config.collection_name);
                let delete_request = serde_json::json!({
                    "points": [id]
                });

                let response = self
                    .client
                    .post(&delete_url)
                    .json(&delete_request)
                    .send()
                    .await?;

                if !response.status().is_success() {
                    return Err(anyhow!("Delete failed: {}", response.status()));
                }

                tracing::info!("🗑️  Deleted vector {} from Qdrant", id);
                Ok(StorageResult {
                    id: id.to_string(),
                    success: true,
                    error: None,
                })
            })
        };

        self.retry_operation(operation).await
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
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
}