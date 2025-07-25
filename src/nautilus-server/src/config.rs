use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct TaskConfig {
    pub move_package_id: String,
    pub sui_secret_key: String,
    pub walrus_config: WalrusConfig,
    pub ollama_config: OllamaConfig,
    pub qdrant_config: QdrantConfig,
}

#[derive(Debug, Clone)]
pub struct WalrusConfig {
    pub aggregator_url: String,
    pub publisher_url: String,
    pub epochs: String,
}

#[derive(Debug, Clone)]
pub struct OllamaConfig {
    pub api_url: String,
    pub model: String,
}

#[derive(Debug, Clone)]
pub struct QdrantConfig {
    pub url: String,
    pub collection_name: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    pub batch_size: Option<usize>,
    pub limit: Option<usize>,
    pub store_vectors: Option<bool>,
    pub include_embeddings: Option<bool>,
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            batch_size: Some(50),
            limit: Some(10),
            store_vectors: Some(true),
            include_embeddings: Some(false),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingArgs {
    pub walrus_blob_id: String,
    pub address: String,
    pub on_chain_file_obj_id: String,
    pub policy_object_id: String,
    pub threshold: String,
    pub enclave_id: String,
    pub processing_config: ProcessingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrieveArgs {
    pub query: String,
    pub address: String,
    pub on_chain_file_obj_id: String,
    pub policy_object_id: String,
    pub threshold: String,
    pub enclave_id: String,
    pub processing_config: ProcessingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrieveByBlobIdsArgs {
    pub blob_file_pairs: Vec<BlobFilePair>,
    pub address: String,
    pub threshold: String,
    pub enclave_id: String,
    pub processing_config: ProcessingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultArgs {
    pub address: String,
    pub blob_id: String,
    pub on_chain_file_obj_id: String,
    pub policy_object_id: String,
    pub threshold: String,
    pub enclave_id: String,
    pub processing_config: ProcessingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobFilePair {
    pub walrus_blob_id: String,
    pub on_chain_file_obj_id: String,
    pub policy_object_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation")]
pub enum TaskOperation {
    #[serde(rename = "embedding")]
    Embedding(EmbeddingArgs),
    #[serde(rename = "retrieve")]
    Retrieve(RetrieveArgs),
    #[serde(rename = "retrieve-by-blob-ids")]
    RetrieveByBlobIds(RetrieveByBlobIdsArgs),
    #[serde(rename = "default")]
    Default(DefaultArgs),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub status: String,
    pub operation: String,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

impl TaskResult {
    pub fn success(operation: &str, data: Option<serde_json::Value>) -> Self {
        Self {
            status: "success".to_string(),
            operation: operation.to_string(),
            data,
            error: None,
        }
    }

    pub fn error(operation: &str, error: String) -> Self {
        Self {
            status: "error".to_string(),
            operation: operation.to_string(),
            data: None,
            error: Some(error),
        }
    }
}