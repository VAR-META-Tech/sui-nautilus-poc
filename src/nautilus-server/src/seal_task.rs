use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use tracing::{debug, info, warn};

/// Configuration for the seal task
#[derive(Debug, Clone)]
pub struct SealTaskConfig {
    pub move_package_id: String,
    pub sui_secret_key: String,
    pub walrus_aggregator_url: String,
    pub walrus_publisher_url: String,
    pub walrus_epochs: u64,
}

impl Default for SealTaskConfig {
    fn default() -> Self {
        Self {
            move_package_id: env::var("MOVE_PACKAGE_ID")
                .unwrap_or_else(|_| "0xf2433262bd55b30c1cddbae940a2355086cfe2850bd62583bdfcad7c57b17956".to_string()),
            sui_secret_key: env::var("SUI_SECRET_KEY")
                .unwrap_or_else(|_| "suiprivkey1qqd6sesfpyc7e9nds3aattvt073muxdchpcz7ad4064t0mgnfnna5ee977f".to_string()),
            walrus_aggregator_url: env::var("WALRUS_AGGREGATOR_URL")
                .unwrap_or_else(|_| "https://aggregator.walrus-testnet.walrus.space".to_string()),
            walrus_publisher_url: env::var("WALRUS_PUBLISHER_URL")
                .unwrap_or_else(|_| "https://publisher.walrus-testnet.walrus.space".to_string()),
            walrus_epochs: env::var("WALRUS_EPOCHS")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .unwrap_or(5),
        }
    }
}

/// Parameters for the seal task execution
#[derive(Debug, Clone)]
pub struct SealTaskParams {
    pub address: String,
    pub blob_id: String,
    pub on_chain_file_obj_id: String,
    pub policy_object_id: String,
    pub threshold: u32,
    pub enclave_id: String,
}

/// Result of the seal task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealTaskResult {
    pub walrus_url: String,
    pub attestation_obj_id: String,
    pub on_chain_file_obj_id: String,
    pub blob_id: String,
}

/// Refined message structure for processed data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefinedMessage {
    pub id: Option<String>,
    pub from_id: Option<String>,
    pub date: Option<String>,
    pub edit_date: Option<String>,
    pub message: Option<String>,
    pub out: Option<bool>,
    pub reactions: Option<MessageReactions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageReactions {
    pub emoji: Option<String>,
    pub count: Option<u32>,
}

/// Refined data structure after processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefinedData {
    pub revision: Option<serde_json::Value>,
    pub user: Option<serde_json::Value>,
    pub messages: Vec<RefinedMessage>,
}

/// Raw chat data structure from the encrypted file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawChatData {
    pub revision: Option<serde_json::Value>,
    pub user: Option<serde_json::Value>,
    pub chats: Option<Vec<ChatData>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatData {
    pub contents: Option<Vec<MessageContent>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContent {
    pub id: Option<String>,
    #[serde(rename = "fromId")]
    pub from_id: Option<FromId>,
    pub date: Option<i64>,
    #[serde(rename = "editDate")]
    pub edit_date: Option<i64>,
    pub message: Option<String>,
    pub out: Option<bool>,
    pub reactions: Option<RawReactions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FromId {
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawReactions {
    #[serde(rename = "recentReactions")]
    pub recent_reactions: Option<Vec<RecentReaction>>,
    pub results: Option<Vec<ReactionResult>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentReaction {
    pub reaction: Option<ReactionEmoji>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionEmoji {
    pub emoticon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionResult {
    pub count: Option<u32>,
}

/// Walrus publish response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalrusPublishResponse {
    #[serde(rename = "newlyCreated")]
    pub newly_created: Option<NewlyCreated>,
    #[serde(rename = "alreadyCertified")]
    pub already_certified: Option<AlreadyCertified>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewlyCreated {
    #[serde(rename = "blobObject")]
    pub blob_object: BlobObject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlreadyCertified {
    #[serde(rename = "blobId")]
    pub blob_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobObject {
    #[serde(rename = "blobId")]
    pub blob_id: String,
    pub size: Option<u64>,
    pub storage: Option<Storage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Storage {
    #[serde(rename = "storageSize")]
    pub storage_size: Option<u64>,
}

/// Metadata for published file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub walrus_url: String,
    pub size: u64,
    pub storage_size: u64,
    pub blob_id: String,
}

/// Main seal task runner
pub struct SealTaskRunner {
    config: SealTaskConfig,
    client: Client,
}

impl SealTaskRunner {
    pub fn new(config: SealTaskConfig) -> Self {
        Self {
            config,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Execute the complete seal task workflow
    pub async fn run(&self, params: SealTaskParams) -> Result<SealTaskResult> {
        info!("Starting seal task execution");
        
        // 1. Fetch encrypted file from Walrus
        let encrypted_file = self.fetch_encrypted_file(&params.blob_id).await?;
        info!("✓ Fetched encrypted file from Walrus");

        // 2. Register TEE attestation
        let attestation_obj_id = self.register_attestation(&params).await?;
        info!("✓ Registered TEE attestation: {}", attestation_obj_id);

        // 3. Decrypt file (simulated - actual decryption would use Seal SDK)
        let decrypted_data = self.decrypt_file(&params, &encrypted_file).await?;
        info!("✓ Decrypted file");

        // 4. Process and refine data
        let refined_data = self.process_data(decrypted_data)?;
        info!("✓ Processed and refined data");

        // 5. Encrypt refined data (simulated - actual encryption would use Seal SDK)
        let encrypted_refined_data = self.encrypt_file(&refined_data).await?;
        info!("✓ Encrypted refined data");

        // 6. Publish encrypted data to Walrus
        let metadata = self.publish_file(&encrypted_refined_data).await?;
        info!("✓ Published encrypted data to Walrus");

        // 7. Save encrypted file reference on-chain
        let on_chain_file_obj_id = self.save_encrypted_file_on_chain(
            &encrypted_refined_data,
            &metadata,
            &params.policy_object_id,
        ).await?;
        info!("✓ Saved encrypted file reference on-chain: {}", on_chain_file_obj_id);

        Ok(SealTaskResult {
            walrus_url: metadata.walrus_url,
            attestation_obj_id,
            on_chain_file_obj_id,
            blob_id: metadata.blob_id,
        })
    }

    /// Fetch encrypted file from Walrus
    async fn fetch_encrypted_file(&self, blob_id: &str) -> Result<Vec<u8>> {
        let url = format!("{}/v1/blobs/{}", self.config.walrus_aggregator_url, blob_id);
        
        let response = self.client
            .get(&url)
            .header("Content-Type", "application/octet-stream")
            .send()
            .await
            .context("Failed to fetch encrypted file from Walrus")?;

        if !response.status().is_success() {
            anyhow::bail!("HTTP {}: {}", response.status(), response.status());
        }

        let bytes = response.bytes().await?;
        if bytes.is_empty() {
            anyhow::bail!("Empty response from Walrus");
        }

        Ok(bytes.to_vec())
    }

    /// Register TEE attestation on-chain
    async fn register_attestation(&self, params: &SealTaskParams) -> Result<String> {
        // TODO: Implement actual Sui transaction for registering attestation
        // This is a placeholder implementation
        
        debug!("Registering TEE attestation for enclave: {}", params.enclave_id);
        
        // In the real implementation, this would:
        // 1. Create a Sui transaction
        // 2. Call the smart contract method: seal_manager::register_tee_attestation
        // 3. Pass enclave_id, file_object_id, and address as parameters
        // 4. Sign and execute the transaction
        // 5. Return the created attestation object ID
        
        // For now, return a mock attestation object ID
        let mock_attestation_id = format!("0x{}", hex::encode(rand::random::<[u8; 32]>()));
        
        Ok(mock_attestation_id)
    }

    /// Decrypt file using Seal SDK (simulated for now)
    async fn decrypt_file(&self, params: &SealTaskParams, encrypted_file: &[u8]) -> Result<RawChatData> {
        // TODO: Implement actual decryption using Seal SDK
        // This is a placeholder that assumes the encrypted file contains JSON
        
        debug!("Decrypting file with Seal SDK");
        
        // In the real implementation, this would:
        // 1. Initialize Seal client
        // 2. Create session key
        // 3. Sign personal message
        // 4. Build transaction for seal_approve
        // 5. Fetch keys from key servers
        // 6. Decrypt the data
        
        // For now, assume the file is already decrypted JSON
        let json_str = String::from_utf8(encrypted_file.to_vec())
            .context("Failed to convert decrypted bytes to string")?;
        
        let raw_data: RawChatData = serde_json::from_str(&json_str)
            .context("Failed to parse decrypted JSON")?;
        
        Ok(raw_data)
    }

    /// Process and refine raw chat data
    fn process_data(&self, raw_data: RawChatData) -> Result<RefinedData> {
        let mut refined_data = RefinedData {
            revision: raw_data.revision,
            user: raw_data.user,
            messages: Vec::new(),
        };

        if let Some(chats) = raw_data.chats {
            for chat in chats {
                if let Some(contents) = chat.contents {
                    for msg in contents {
                        let refined_msg = RefinedMessage {
                            id: msg.id,
                            from_id: msg.from_id.and_then(|f| f.user_id),
                            date: msg.date.map(|d| {
                                DateTime::from_timestamp(d, 0)
                                    .unwrap_or_else(|| Utc::now())
                                    .to_rfc3339()
                            }),
                            edit_date: msg.edit_date.map(|d| {
                                DateTime::from_timestamp(d, 0)
                                    .unwrap_or_else(|| Utc::now())
                                    .to_rfc3339()
                            }),
                            message: msg.message,
                            out: msg.out,
                            reactions: msg.reactions.map(|r| MessageReactions {
                                emoji: r.recent_reactions
                                    .and_then(|rr| rr.first().cloned())
                                    .and_then(|rr| rr.reaction)
                                    .and_then(|re| re.emoticon),
                                count: r.results
                                    .and_then(|res| res.first().cloned())
                                    .and_then(|res| res.count),
                            }),
                        };
                        refined_data.messages.push(refined_msg);
                    }
                }
            }
        }

        // Sort messages by date
        refined_data.messages.sort_by(|a, b| {
            match (a.date.as_ref(), b.date.as_ref()) {
                (Some(date_a), Some(date_b)) => date_a.cmp(date_b),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });

        Ok(refined_data)
    }

    /// Encrypt refined data using Seal SDK (simulated for now)
    async fn encrypt_file(&self, refined_data: &RefinedData) -> Result<Vec<u8>> {
        // TODO: Implement actual encryption using Seal SDK
        
        debug!("Encrypting refined data with Seal SDK");
        
        // In the real implementation, this would:
        // 1. Generate a unique ID for the encrypted object
        // 2. Use Seal client to encrypt the data
        // 3. Return the encrypted bytes
        
        // For now, just serialize to JSON and convert to bytes
        let json_str = serde_json::to_string(refined_data)
            .context("Failed to serialize refined data")?;
        
        Ok(json_str.into_bytes())
    }

    /// Publish encrypted file to Walrus
    async fn publish_file(&self, encrypted_data: &[u8]) -> Result<FileMetadata> {
        let url = format!("{}/v1/blobs?epochs={}", 
            self.config.walrus_publisher_url, 
            self.config.walrus_epochs);

        let response = self.client
            .put(&url)
            .header("Content-Type", "application/octet-stream")
            .body(encrypted_data.to_vec())
            .send()
            .await
            .context("Failed to publish file to Walrus")?;

        if !response.status().is_success() {
            anyhow::bail!("HTTP {}: {}", response.status(), response.status());
        }

        let publish_response: WalrusPublishResponse = response.json().await?;
        
        let (blob_id, size, storage_size) = if let Some(newly_created) = publish_response.newly_created {
            (
                newly_created.blob_object.blob_id,
                newly_created.blob_object.size.unwrap_or(0),
                newly_created.blob_object.storage
                    .and_then(|s| s.storage_size)
                    .unwrap_or(0),
            )
        } else if let Some(already_certified) = publish_response.already_certified {
            (already_certified.blob_id, 0, 0)
        } else {
            anyhow::bail!("Invalid response format from Walrus");
        };

        Ok(FileMetadata {
            walrus_url: format!("{}/v1/blobs/{}", self.config.walrus_aggregator_url, blob_id),
            size,
            storage_size,
            blob_id,
        })
    }

    /// Save encrypted file reference on-chain
    async fn save_encrypted_file_on_chain(
        &self,
        encrypted_data: &[u8],
        metadata: &FileMetadata,
        policy_obj_id: &str,
    ) -> Result<String> {
        // TODO: Implement actual Sui transaction for saving encrypted file reference
        
        debug!("Saving encrypted file reference on-chain");
        
        // In the real implementation, this would:
        // 1. Parse the encrypted object to get its ID
        // 2. Create a Sui transaction
        // 3. Call the smart contract method: seal_manager::save_encrypted_file
        // 4. Pass encrypted_object_id, policy_object_id, and metadata as parameters
        // 5. Sign and execute the transaction
        // 6. Return the created on-chain file object ID
        
        // For now, return a mock object ID
        let mock_obj_id = format!("0x{}", hex::encode(rand::random::<[u8; 32]>()));
        
        Ok(mock_obj_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_data() {
        let runner = SealTaskRunner::new(SealTaskConfig::default());
        
        // Create mock raw data
        let raw_data = RawChatData {
            revision: Some(serde_json::json!({"version": "1.0"})),
            user: Some(serde_json::json!({"id": "user123"})),
            chats: Some(vec![ChatData {
                contents: Some(vec![MessageContent {
                    id: Some("msg1".to_string()),
                    from_id: Some(FromId {
                        user_id: Some("user123".to_string()),
                    }),
                    date: Some(1640995200), // 2022-01-01 00:00:00 UTC
                    edit_date: None,
                    message: Some("Hello world".to_string()),
                    out: Some(true),
                    reactions: None,
                }]),
            }]),
        };

        let result = runner.process_data(raw_data).unwrap();
        
        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.messages[0].id, Some("msg1".to_string()));
        assert_eq!(result.messages[0].message, Some("Hello world".to_string()));
    }
} 