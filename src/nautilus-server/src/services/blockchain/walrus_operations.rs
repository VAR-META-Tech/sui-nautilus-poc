use crate::config::WalrusConfig;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct WalrusOperations {
    config: WalrusConfig,
    client: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalrusMetadata {
    pub walrus_url: String,
    pub size: u64,
    pub storage_size: u64,
    pub blob_id: String,
    pub publisher_url: String,
    pub aggregator_url: String,
    pub epochs: String,
    pub published_at: String,
}

#[derive(Debug, Deserialize)]
struct WalrusResponse {
    #[serde(rename = "newlyCreated")]
    newly_created: Option<NewlyCreated>,
    #[serde(rename = "alreadyCertified")]
    already_certified: Option<AlreadyCertified>,
}

#[derive(Debug, Deserialize)]
struct NewlyCreated {
    #[serde(rename = "blobObject")]
    blob_object: BlobObject,
}

#[derive(Debug, Deserialize)]
struct BlobObject {
    #[serde(rename = "blobId")]
    blob_id: String,
    size: Option<u64>,
    storage: Option<Storage>,
}

#[derive(Debug, Deserialize)]
struct Storage {
    #[serde(rename = "storageSize")]
    storage_size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct AlreadyCertified {
    #[serde(rename = "blobId")]
    blob_id: String,
}

impl WalrusOperations {
    pub fn new(config: WalrusConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    pub async fn fetch_encrypted_file(&self, blob_id: &str) -> Result<Vec<u8>> {
        let walrus_url = format!("{}/v1/blobs/{}", self.config.aggregator_url, blob_id);

        tracing::info!("📥 Fetching encrypted file from {}", walrus_url);

        let response = self
            .client
            .get(&walrus_url)
            .header("Content-Type", "application/octet-stream")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        let encrypted_file = response.bytes().await?.to_vec();

        if encrypted_file.is_empty() {
            return Err(anyhow!("Empty response from Walrus"));
        }

        tracing::info!(
            "✅ Successfully fetched encrypted file ({} bytes)",
            encrypted_file.len()
        );
        Ok(encrypted_file)
    }

    pub async fn publish_file(&self, encrypted_data: &[u8]) -> Result<WalrusMetadata> {
        let upload_url = format!(
            "{}/v1/blobs?epochs={}",
            self.config.publisher_url, self.config.epochs
        );

        tracing::info!("📤 Publishing file to {}", upload_url);

        let response = self
            .client
            .put(&upload_url)
            .header("Content-Type", "application/octet-stream")
            .body(encrypted_data.to_vec())
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        let data: WalrusResponse = response.json().await?;
        let blob_id = if let Some(ref newly_created) = data.newly_created {
            newly_created.blob_object.blob_id.clone()
        } else if let Some(ref already_certified) = data.already_certified {
            already_certified.blob_id.clone()
        } else {
            return Err(anyhow!("Invalid response format from Walrus"));
        };

        let metadata = WalrusMetadata {
            walrus_url: format!("{}/v1/blobs/{}", self.config.aggregator_url, blob_id),
            size: data
                .newly_created
                .as_ref()
                .and_then(|nc| nc.blob_object.size)
                .unwrap_or(0),
            storage_size: data
                .newly_created
                .as_ref()
                .and_then(|nc| nc.blob_object.storage.as_ref())
                .and_then(|s| s.storage_size)
                .unwrap_or(0),
            blob_id: blob_id.clone(),
            publisher_url: self.config.publisher_url.clone(),
            aggregator_url: self.config.aggregator_url.clone(),
            epochs: self.config.epochs.clone(),
            published_at: chrono::Utc::now().to_rfc3339(),
        };

        tracing::info!("✅ File published successfully. Blob ID: {}", blob_id);
        Ok(metadata)
    }

    pub async fn get_storage_info(&self) -> Result<serde_json::Value> {
        let response = self
            .client
            .get(&format!("{}/v1/info", self.config.aggregator_url))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        let info: serde_json::Value = response.json().await?;
        Ok(serde_json::json!({
            "aggregatorUrl": self.config.aggregator_url,
            "publisherUrl": self.config.publisher_url,
            "epochs": self.config.epochs,
            "networkInfo": info
        }))
    }

    pub async fn health_check(&self) -> Result<serde_json::Value> {
        let aggregator_future = self
            .client
            .get(&format!("{}/v1/info", self.config.aggregator_url))
            .send();
        let publisher_future = self
            .client
            .get(&format!("{}/v1/info", self.config.publisher_url))
            .send();

        let (aggregator_response, publisher_response) =
            tokio::try_join!(aggregator_future, publisher_future)?;

        Ok(serde_json::json!({
            "status": "healthy",
            "aggregator": {
                "url": self.config.aggregator_url,
                "healthy": aggregator_response.status().is_success()
            },
            "publisher": {
                "url": self.config.publisher_url,
                "healthy": publisher_response.status().is_success()
            },
            "epochs": self.config.epochs
        }))
    }
}