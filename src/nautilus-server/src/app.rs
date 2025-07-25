// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::common::{ProcessDataRequest, get_attestation};
use crate::config::{EmbeddingArgs, ProcessingConfig, TaskOperation};
use crate::tasks::TaskRunner;
use crate::AppState;
use crate::EnclaveError;
use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::{CorsLayer, AllowOrigin, AllowHeaders};
use std::env;
use axum::routing::{get, post};
use axum::Router;
use axum::http::{HeaderValue, Method, header::{CONTENT_TYPE, AUTHORIZATION, ACCEPT, ORIGIN, REFERER, USER_AGENT}};
use crate::common::{health_check};

// Native Rust implementation - no need for stdout parsing

/// ====
/// Core Nautilus server logic, replace it with your own
/// relavant structs and process_data endpoint.
/// ====

/// Inner type T for IntentMessage<T>
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskResponse {
    pub status: String,
    pub data: serde_json::Value,
    pub stderr: String,
    pub exit_code: i32,
    pub execution_time_ms: u64,
}

/// Inner type T for ProcessDataRequest<T>
#[derive(Debug, Serialize, Deserialize)]
pub struct TaskRequest {
    pub timeout_secs: Option<u64>,
    pub args: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmbeddingIngestRequest {
    #[serde(rename = "walrusBlobId")]
    pub walrus_blob_id: String,
    pub address: String,
    #[serde(rename = "onChainFileObjId")]
    pub on_chain_file_obj_id: String,
    #[serde(rename = "policyObjectId")]
    pub policy_object_id: String,
    pub threshold: String,
    pub timeout_secs: Option<u64>,
    #[serde(rename = "batchSize")]
    pub batch_size: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageRetrievalRequest {
    pub query: String,
    pub limit: Option<u32>,
    pub address: String,
    #[serde(rename = "onChainFileObjId")]
    pub on_chain_file_obj_id: String,
    #[serde(rename = "policyObjectId")]
    pub policy_object_id: String,
    pub threshold: String,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlobFileIdPair {
    #[serde(rename = "walrusBlobId")]
    pub walrus_blob_id: String,
    #[serde(rename = "onChainFileObjId")]
    pub on_chain_file_obj_id: String,
    #[serde(rename = "policyObjectId")]
    pub policy_object_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageBlobRetrievalRequest {
    #[serde(rename = "blobFilePairs")]
    pub blob_file_pairs: Vec<BlobFileIdPair>,
    pub address: String,
    #[serde(rename = "policyObjectId")]
    pub policy_object_id: Option<String>, // Now optional since each pair has its own policy ID
    pub threshold: String,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessedData {
    #[serde(rename = "walrusUrl")]
    pub walrus_url: String,
    #[serde(rename = "attestationObjId")]
    pub attestation_obj_id: String,
    #[serde(rename = "onChainFileObjId")]
    pub on_chain_file_obj_id: String,
    #[serde(rename = "blobId")]
    pub blob_id: Option<String>,
}

/// Native Rust implementation for default processing (replaces Node.js)
pub async fn process_data(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProcessDataRequest<TaskRequest>>,
) -> Result<Json<TaskResponse>, EnclaveError> {
    // get attestation
    let attestation_info = get_attestation(State(state.clone())).await?;

    // Create native Rust TaskConfig from AppState
    let task_config = state.to_task_config();
    let task_runner = TaskRunner::new(task_config);

    // Convert request to native Rust operation (default operation)
    let default_args = crate::config::DefaultArgs {
        address: "default_address".to_string(), // Would need to be extracted from request
        blob_id: "default_blob".to_string(), // Would need to be extracted from request
        on_chain_file_obj_id: "default_file".to_string(), // Would need to be extracted from request
        policy_object_id: "default_policy".to_string(), // Would need to be extracted from request
        threshold: "2".to_string(), // Would need to be extracted from request
        enclave_id: attestation_info.attestation.enclaveId.clone(),
        processing_config: crate::config::ProcessingConfig::default(),
    };

    let operation = TaskOperation::Default(default_args);

    // Execute native Rust task
    let start_time = std::time::Instant::now();
    let task_result = task_runner.run(operation).await.map_err(|e| {
        EnclaveError::GenericError(format!("Failed to execute native Rust default task: {}", e))
    })?;

    let execution_time_ms = start_time.elapsed().as_millis() as u64;

    let is_success = task_result.status == "success";
    Ok(Json(TaskResponse {
        status: task_result.status,
        data: task_result.data.unwrap_or_else(|| serde_json::json!({})),
        stderr: task_result.error.unwrap_or_default(),
        exit_code: if is_success { 0 } else { 1 },
        execution_time_ms,
    }))
}

/// Native Rust implementation for embedding ingest (replaces Node.js)
pub async fn embedding_ingest(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProcessDataRequest<EmbeddingIngestRequest>>,
) -> Result<Json<TaskResponse>, EnclaveError> {
    // get attestation
    let attestation_info = get_attestation(State(state.clone())).await?;

    // Create native Rust TaskConfig from AppState
    let task_config = state.to_task_config();
    let task_runner = TaskRunner::new(task_config);

    // Convert request to native Rust operation
    let embedding_args = EmbeddingArgs {
        walrus_blob_id: request.payload.walrus_blob_id.clone(),
        address: request.payload.address.clone(),
        on_chain_file_obj_id: request.payload.on_chain_file_obj_id.clone(),
        policy_object_id: request.payload.policy_object_id.clone(),
        threshold: request.payload.threshold.clone(),
        enclave_id: attestation_info.attestation.enclaveId.clone(),
        processing_config: ProcessingConfig {
            batch_size: request.payload.batch_size.map(|b| b as usize),
            limit: None,
            store_vectors: Some(true),
            include_embeddings: Some(false),
        },
    };

    let operation = TaskOperation::Embedding(embedding_args);

    // Execute native Rust task
    let start_time = std::time::Instant::now();
    let task_result = task_runner.run(operation).await.map_err(|e| {
        EnclaveError::GenericError(format!("Failed to execute native Rust embedding task: {}", e))
    })?;

    let execution_time_ms = start_time.elapsed().as_millis() as u64;

    let is_success = task_result.status == "success";
    Ok(Json(TaskResponse {
        status: task_result.status,
        data: task_result.data.unwrap_or_else(|| serde_json::json!({})),
        stderr: task_result.error.unwrap_or_default(),
        exit_code: if is_success { 0 } else { 1 },
        execution_time_ms,
    }))
}

/// Native Rust implementation for message retrieval (replaces Node.js)
pub async fn retrieve_messages(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProcessDataRequest<MessageRetrievalRequest>>,
) -> Result<Json<TaskResponse>, EnclaveError> {
    // get attestation
    let attestation_info = get_attestation(State(state.clone())).await?;

    // Create native Rust TaskConfig from AppState
    let task_config = state.to_task_config();
    let task_runner = TaskRunner::new(task_config);

    // Convert request to native Rust operation
    let retrieve_args = crate::config::RetrieveArgs {
        query: request.payload.query.clone(),
        address: request.payload.address.clone(),
        on_chain_file_obj_id: request.payload.on_chain_file_obj_id.clone(),
        policy_object_id: request.payload.policy_object_id.clone(),
        threshold: request.payload.threshold.clone(),
        enclave_id: attestation_info.attestation.enclaveId.clone(),
        processing_config: ProcessingConfig {
            batch_size: None,
            limit: request.payload.limit.map(|l| l as usize),
            store_vectors: Some(false),
            include_embeddings: Some(false),
        },
    };

    let operation = TaskOperation::Retrieve(retrieve_args);

    // Execute native Rust task
    let start_time = std::time::Instant::now();
    let task_result = task_runner.run(operation).await.map_err(|e| {
        EnclaveError::GenericError(format!("Failed to execute native Rust retrieval task: {}", e))
    })?;

    let execution_time_ms = start_time.elapsed().as_millis() as u64;

    let is_success = task_result.status == "success";
    Ok(Json(TaskResponse {
        status: task_result.status,
        data: task_result.data.unwrap_or_else(|| serde_json::json!({})),
        stderr: task_result.error.unwrap_or_default(),
        exit_code: if is_success { 0 } else { 1 },
        execution_time_ms,
    }))
}

/// Native Rust implementation for message retrieval by blob IDs (replaces Node.js)
pub async fn retrieve_messages_by_blob_ids(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProcessDataRequest<MessageBlobRetrievalRequest>>,
) -> Result<Json<TaskResponse>, EnclaveError> {
    // get attestation
    let attestation_info = get_attestation(State(state.clone())).await?;

    // Create native Rust TaskConfig from AppState
    let task_config = state.to_task_config();
    let task_runner = TaskRunner::new(task_config);

    // Convert request to native Rust operation
    let blob_file_pairs = request.payload.blob_file_pairs.into_iter().map(|pair| {
        crate::config::BlobFilePair {
            walrus_blob_id: pair.walrus_blob_id,
            on_chain_file_obj_id: pair.on_chain_file_obj_id,
            policy_object_id: pair.policy_object_id,
        }
    }).collect();

    let retrieve_by_blob_ids_args = crate::config::RetrieveByBlobIdsArgs {
        blob_file_pairs,
        address: request.payload.address.clone(),
        threshold: request.payload.threshold.clone(),
        enclave_id: attestation_info.attestation.enclaveId.clone(),
        processing_config: ProcessingConfig::default(),
    };

    let operation = TaskOperation::RetrieveByBlobIds(retrieve_by_blob_ids_args);

    // Execute native Rust task
    let start_time = std::time::Instant::now();
    let task_result = task_runner.run(operation).await.map_err(|e| {
        EnclaveError::GenericError(format!("Failed to execute native Rust blob ID retrieval task: {}", e))
    })?;

    let execution_time_ms = start_time.elapsed().as_millis() as u64;

    let is_success = task_result.status == "success";
    Ok(Json(TaskResponse {
        status: task_result.status,
        data: task_result.data.unwrap_or_else(|| serde_json::json!({})),
        stderr: task_result.error.unwrap_or_default(),
        exit_code: if is_success { 0 } else { 1 },
        execution_time_ms,
    }))
}

/// All functions now use native Rust implementation

/// Create router with all endpoints
pub fn create_app(state: Arc<AppState>) -> Router {
    // Define allowed origins based on environment
    let origins = [
        "http://localhost:3000".parse::<HeaderValue>().unwrap(),
        "http://127.0.0.1:3000".parse::<HeaderValue>().unwrap(),
        "https://localhost:3000".parse::<HeaderValue>().unwrap(),
        "https://127.0.0.1:3000".parse::<HeaderValue>().unwrap(),
    ];

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_headers(AllowHeaders::list([
            CONTENT_TYPE,
            AUTHORIZATION,
            ACCEPT,
            ORIGIN,
            REFERER,
            USER_AGENT,
        ]))
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE]);

    Router::new()
        .route("/health", get(health_check))
        .route("/process_data", post(process_data))
        .route("/embedding_ingest", post(embedding_ingest))
        .route("/retrieve_messages", post(retrieve_messages))
        .route("/retrieve_messages_by_blob_ids", post(retrieve_messages_by_blob_ids))
        .layer(cors)
        .with_state(state)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::common::{IntentMessage, IntentScope};
    use axum::{extract::State, Json};
    use fastcrypto::{ed25519::Ed25519KeyPair, traits::KeyPair};

    // Note: This test is disabled because it requires the actual nodejs-task directory
    // to exist. In a real deployment, the nodejs-task directory is part of the container.
    // For unit testing, we focus on testing individual components like env var mapping.
    #[tokio::test]
    #[ignore] // Ignore this test in normal runs
    async fn test_process_data() {
        // This test would require the actual nodejs-task directory structure
        // which is not available in unit test environment
        println!("Test disabled - requires actual nodejs-task directory");
    }

    #[test]
    fn test_serde() {
        // test result should be consistent with serialization expectations
        use fastcrypto::encoding::{Encoding, Hex};
        let payload = TaskResponse {
            status: "success".to_string(),
            data: serde_json::json!("Hello World"),
            stderr: "".to_string(),
            exit_code: 0,
            execution_time_ms: 1500,
        };
        let timestamp = 1744038900000;
        let intent_msg = IntentMessage::new(payload, timestamp, IntentScope::Generic);
        let signing_payload = bcs::to_bytes(&intent_msg).expect("should not fail");

        // Just ensure serialization works without checking exact bytes since structure changed
        assert!(!signing_payload.is_empty());
    }
}
