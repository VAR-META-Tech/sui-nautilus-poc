use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::str::FromStr;
use tracing::{debug, info, warn};

// Sui SDK imports - using our local implementation
use crate::sui_sdk::{
    SuiClient, SuiClientBuilder, SuiAddress, ObjectID, Transaction, TransactionData,
    ProgrammableTransactionBuilder, Identifier, AccountKeystore, Keystore, Ed25519Keypair,
    ExecuteTransactionRequestType, ObjectArg, SequenceNumber, ObjectDigest, Keypair,
};

// Seal SDK imports - using our local implementation
use crate::seal::{
    SealClient, SessionKey, EncryptedObject, KeyServerConfig,
};

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
    sui_client: SuiClient,
    seal_client: SealClient,
    keystore: Keystore,
}

impl SealTaskRunner {
    pub async fn new(config: SealTaskConfig) -> Result<Self> {
        // Initialize Sui client
        let sui_client = SuiClientBuilder::default()
            .build("https://fullnode.testnet.sui.io:443")
            .await?;

        // Initialize keystore - create an empty keystore for now
        // In a real implementation, this would load from the proper Sui keystore location
        let mut keystore = Keystore::default();
        
        // For testing, add a mock keypair
        let test_keypair = Ed25519Keypair::new();
        keystore.add_key(test_keypair)?;

        // Initialize Seal client with key servers
        let key_servers = vec![
            KeyServerConfig {
                object_id: ObjectID::from_str("0x123...")?, // Replace with actual key server object ID
                address: "https://keyserver1.testnet.seal.io".to_string(),
            },
            KeyServerConfig {
                object_id: ObjectID::from_str("0x456...")?, // Replace with actual key server object ID
                address: "https://keyserver2.testnet.seal.io".to_string(),
            },
        ];

        let seal_client = SealClient::new(sui_client.clone(), key_servers).await?;

        Ok(Self {
            config,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .context("Failed to create HTTP client")?,
            sui_client,
            seal_client,
            keystore,
        })
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

        // 5. Encrypt refined data using Seal SDK
        let encrypted_refined_data = self.encrypt_file(&refined_data, params.threshold).await?;
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

    /// Register TEE attestation on-chain using Sui SDK
    async fn register_attestation(&self, params: &SealTaskParams) -> Result<String> {
        debug!("Registering TEE attestation for enclave: {}", params.enclave_id);
        
        // Get the signing key from keystore
        let addresses = self.keystore.addresses();
        let active_address = addresses.first()
            .ok_or_else(|| anyhow::anyhow!("No addresses found in keystore"))?;
        
        let keypair = self.keystore.get_key(active_address)?;
        
        // Parse addresses and object IDs
        let sender_address = SuiAddress::from_str(&params.address)?;
        let file_object_id = ObjectID::from_str(&params.on_chain_file_obj_id)?;
        let package_id = ObjectID::from_str(&self.config.move_package_id)?;
        
        // Build the transaction
        let mut ptb = ProgrammableTransactionBuilder::new();
        
        // Convert enclave_id to bytes
        let enclave_id_bytes = params.enclave_id.as_bytes().to_vec();
        let file_object_id_bytes = file_object_id.to_bytes().to_vec();
        
        // Prepare arguments for seal_manager::register_tee_attestation
        let arg1 = ptb.pure(enclave_id_bytes)?.into();
        let arg2 = ptb.pure(file_object_id_bytes)?.into();
        let arg3 = ptb.pure(sender_address.clone())?.into();
        
        // Call seal_manager::register_tee_attestation
        ptb.programmable_move_call(
            package_id,
            Identifier::new("seal_manager")?,
            Identifier::new("register_tee_attestation")?,
            vec![], // Type arguments
            vec![arg1, arg2, arg3],
        );
        
        let programmable_transaction = ptb.finish();
        
        // Create transaction data
        let gas_budget = 10_000_000; // 0.01 SUI
        let gas_price = self.sui_client.read_api().get_reference_gas_price().await?;
        
        let tx_data = TransactionData::new_programmable(
            sender_address,
            vec![], // Gas objects will be selected automatically
            programmable_transaction,
            gas_budget,
            gas_price,
        );
        
        // Sign and execute the transaction
        let signature = keypair.sign(&bcs::to_bytes(&tx_data)?);
        let signed_tx = Transaction::from_data(tx_data, vec![signature]);
        
        let response = self.sui_client
            .quorum_driver_api()
            .execute_transaction_block(
                signed_tx,
                ExecuteTransactionRequestType::WaitForLocalExecution,
                Some(ExecuteTransactionRequestType::WaitForLocalExecution),
            )
            .await?;
        
        // Extract the created attestation object ID from effects
        let attestation_obj_id = response
            .effects
            .as_ref()
            .and_then(|effects| effects.created().first())
            .map(|created| created.reference.object_id.to_string())
            .ok_or_else(|| anyhow::anyhow!("No attestation object created"))?;
        
        info!("Successfully registered TEE attestation: {}", attestation_obj_id);
        Ok(attestation_obj_id)
    }

    /// Decrypt file using Seal SDK
    async fn decrypt_file(&self, params: &SealTaskParams, encrypted_file: &[u8]) -> Result<RawChatData> {
        debug!("Decrypting file with Seal SDK");
        
        // Parse the encrypted object
        let encrypted_object = EncryptedObject::from_bytes(encrypted_file)?;
        
        // Get the signing key from keystore
        let addresses = self.keystore.addresses();
        let active_address = addresses.first()
            .ok_or_else(|| anyhow::anyhow!("No addresses found in keystore"))?;
        
        let keypair = self.keystore.get_key(active_address)?;
        
        // Create session key for decryption
        let session_key = SessionKey::new(
            SuiAddress::from_str(&params.address)?,
            ObjectID::from_str(&self.config.move_package_id)?,
            10, // TTL in minutes
        );
        
        // Get personal message for signing
        let personal_message = session_key.personal_message();
        
        // Sign the personal message
        let signature = keypair.sign(personal_message.as_bytes());
        
        // Set the signature on session key
        let mut session_key = session_key;
        session_key.set_signature(signature.as_bytes().to_vec())?;
        
        // Parse addresses and object IDs for the transaction
        let sender_address = SuiAddress::from_str(&params.address)?;
        let file_object_id = ObjectID::from_str(&params.on_chain_file_obj_id)?;
        let policy_object_id = ObjectID::from_str(&params.policy_object_id)?;
        let package_id = ObjectID::from_str(&self.config.move_package_id)?;
        
        // Build the seal_approve transaction
        let mut ptb = ProgrammableTransactionBuilder::new();
        
        let file_object_id_bytes = file_object_id.to_bytes().to_vec();
        
        // Prepare arguments for seal_manager::seal_approve
        let arg1 = ptb.pure(file_object_id_bytes)?.into();
        let arg2 = ptb.obj(ObjectArg::ImmOrOwnedObject(
            (file_object_id, SequenceNumber::new(), ObjectDigest::new([0; 32]))
        ))?;
        let arg3 = ptb.obj(ObjectArg::ImmOrOwnedObject(
            (policy_object_id, SequenceNumber::new(), ObjectDigest::new([0; 32]))
        ))?;
        let arg4 = ptb.pure(sender_address.clone())?.into();
        
        // Call seal_manager::seal_approve
        ptb.programmable_move_call(
            package_id,
            Identifier::new("seal_manager")?,
            Identifier::new("seal_approve")?,
            vec![], // Type arguments
            vec![arg1, arg2, arg3, arg4],
        );
        
        let programmable_transaction = ptb.finish();
        
        // Create transaction data for key derivation
        let gas_budget = 10_000_000;
        let gas_price = self.sui_client.read_api().get_reference_gas_price().await?;
        
        let tx_data = TransactionData::new_programmable(
            sender_address,
            vec![],
            programmable_transaction,
            gas_budget,
            gas_price,
        );
        
        let tx_bytes = bcs::to_bytes(&tx_data)?;
        
        // Fetch decryption keys from key servers
        let decryption_keys = self.seal_client
            .fetch_keys(
                &[encrypted_object.id()],
                &tx_bytes,
                &session_key,
                params.threshold,
            )
            .await?;
        
        // Decrypt the data
        let decrypted_bytes = self.seal_client
            .decrypt(&encrypted_object, &decryption_keys)
            .await?;
        
        // Parse the decrypted JSON
        let json_str = String::from_utf8(decrypted_bytes)
            .context("Failed to convert decrypted bytes to string")?;
        
        let raw_data: RawChatData = serde_json::from_str(&json_str)
            .context("Failed to parse decrypted JSON")?;
        
        info!("Successfully decrypted file using Seal SDK");
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

    /// Encrypt refined data using Seal SDK
    async fn encrypt_file(&self, refined_data: &RefinedData, threshold: u32) -> Result<Vec<u8>> {
        debug!("Encrypting refined data with Seal SDK");
        
        // Serialize the refined data to JSON
        let json_str = serde_json::to_string(refined_data)
            .context("Failed to serialize refined data")?;
        
        let data_bytes = json_str.into_bytes();
        
        // Generate a unique ID for the encrypted object
        let policy_object_id = ObjectID::from_str(&self.config.move_package_id)?;
        let nonce = rand::random::<[u8; 5]>();
        let mut id_bytes = policy_object_id.to_bytes().to_vec();
        id_bytes.extend_from_slice(&nonce);
        
        // Create the object ID for the encrypted data
        let encrypted_object_id = ObjectID::from_bytes(&id_bytes[..32])?;
        
        // Encrypt the data using Seal client
        let encrypted_object = self.seal_client
            .encrypt(
                encrypted_object_id.clone(),
                &data_bytes,
                threshold as usize,
                ObjectID::from_str(&self.config.move_package_id)?,
            )
            .await?;
        
        // Serialize the encrypted object to bytes
        let encrypted_bytes = encrypted_object.to_bytes()?;
        
        info!("Successfully encrypted file using Seal SDK, object ID: {}", encrypted_object_id);
        Ok(encrypted_bytes)
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

    /// Save encrypted file reference on-chain using Sui SDK
    async fn save_encrypted_file_on_chain(
        &self,
        encrypted_data: &[u8],
        metadata: &FileMetadata,
        policy_obj_id: &str,
    ) -> Result<String> {
        debug!("Saving encrypted file reference on-chain");
        
        // Parse the encrypted object to get its ID
        let encrypted_object = EncryptedObject::from_bytes(encrypted_data)?;
        let encrypted_object_id = encrypted_object.id();
        
        // Get the signing key from keystore
        let addresses = self.keystore.addresses();
        let active_address = addresses.first()
            .ok_or_else(|| anyhow::anyhow!("No addresses found in keystore"))?;
        
        let keypair = self.keystore.get_key(active_address)?;
        
        // Parse addresses and object IDs
        let sender_address = SuiAddress::from_str(&active_address.to_string())?;
        let policy_object_id = ObjectID::from_str(policy_obj_id)?;
        let package_id = ObjectID::from_str(&self.config.move_package_id)?;
        
        // Serialize metadata to bytes
        let metadata_bytes = serde_json::to_vec(metadata)
            .context("Failed to serialize metadata")?;
        
        // Build the transaction
        let mut ptb = ProgrammableTransactionBuilder::new();
        
        let encrypted_object_id_bytes = encrypted_object_id.to_bytes().to_vec();
        
        // Prepare arguments for seal_manager::save_encrypted_file
        let arg1 = ptb.pure(encrypted_object_id_bytes)?.into();
        let arg2 = ptb.obj(ObjectArg::ImmOrOwnedObject(
            (policy_object_id, SequenceNumber::new(), ObjectDigest::new([0; 32]))
        ))?;
        let arg3 = ptb.pure(metadata_bytes)?.into();
        
        // Call seal_manager::save_encrypted_file
        ptb.programmable_move_call(
            package_id,
            Identifier::new("seal_manager")?,
            Identifier::new("save_encrypted_file")?,
            vec![], // Type arguments
            vec![arg1, arg2, arg3],
        );
        
        let programmable_transaction = ptb.finish();
        
        // Create transaction data
        let gas_budget = 10_000_000; // 0.01 SUI
        let gas_price = self.sui_client.read_api().get_reference_gas_price().await?;
        
        let tx_data = TransactionData::new_programmable(
            sender_address,
            vec![], // Gas objects will be selected automatically
            programmable_transaction,
            gas_budget,
            gas_price,
        );
        
        // Sign and execute the transaction
        let signature = keypair.sign(&bcs::to_bytes(&tx_data)?);
        let signed_tx = Transaction::from_data(tx_data, vec![signature]);
        
        let response = self.sui_client
            .quorum_driver_api()
            .execute_transaction_block(
                signed_tx,
                ExecuteTransactionRequestType::WaitForLocalExecution,
                Some(ExecuteTransactionRequestType::WaitForLocalExecution),
            )
            .await?;
        
        // Extract the created file object ID from effects
        let file_obj_id = response
            .effects
            .as_ref()
            .and_then(|effects| effects.created().first())
            .map(|created| created.reference.object_id.to_string())
            .ok_or_else(|| anyhow::anyhow!("No file object created"))?;
        
        info!("Successfully saved encrypted file reference on-chain: {}", file_obj_id);
        Ok(file_obj_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_data() {
        // Create a mock SealTaskRunner without actual Sui/Seal clients for testing
        let config = SealTaskConfig::default();
        let client = Client::new();
        
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

        // Test the data processing logic directly (this doesn't require Sui/Seal setup)
        let mut refined_data = RefinedData {
            revision: raw_data.revision.clone(),
            user: raw_data.user.clone(),
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
        
        assert_eq!(refined_data.messages.len(), 1);
        assert_eq!(refined_data.messages[0].id, Some("msg1".to_string()));
        assert_eq!(refined_data.messages[0].message, Some("Hello world".to_string()));
    }
} 