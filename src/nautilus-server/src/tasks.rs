use crate::config::{TaskConfig, TaskOperation, TaskResult, EmbeddingArgs, RetrieveArgs, RetrieveByBlobIdsArgs, DefaultArgs};
use crate::services::factory::ServiceFactory;
use crate::services::embedding::BaseEmbedding;
use crate::services::vector_db::BaseVectorDb;
use anyhow::Result;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct TaskRunner {
    pub config: TaskConfig,
}

impl TaskRunner {
    pub fn new(config: TaskConfig) -> Self {
        Self { config }
    }

    pub async fn run(&self, operation: TaskOperation) -> Result<TaskResult> {
        match operation {
            TaskOperation::Embedding(args) => run_embedding_operation(args, &self.config).await,
            TaskOperation::Retrieve(args) => run_retrieve_operation(args, &self.config).await,
            TaskOperation::RetrieveByBlobIds(args) => run_retrieve_by_blob_ids_operation(args, &self.config).await,
            TaskOperation::Default(args) => run_default_operation(args, &self.config).await,
        }
    }
}

pub async fn run_embedding_operation(
    args: EmbeddingArgs,
    config: &TaskConfig,
) -> Result<TaskResult> {
    tracing::info!("🔤 Running Embedding Operation...");

    let factory = ServiceFactory::new(config.clone());
    let services = factory.create_all_services().await?;

    // Step 1: Fetch encrypted refined data from Walrus
    tracing::info!("📥 Step 1: Fetching encrypted refined data from Walrus...");
    let refined_data_encrypted = services
        .walrus
        .fetch_encrypted_file(&args.walrus_blob_id)
        .await?;

    // Step 2: Parse encrypted object
    tracing::info!("📦 Step 2: Parsing encrypted refined data...");
    let encrypted_object = services.seal.parse_encrypted_object(&refined_data_encrypted)?;

    // Step 3: Register attestation for decryption
    tracing::info!("🔗 Step 3: Registering attestation...");
    let attestation_obj_id = services
        .sui
        .register_attestation(&encrypted_object.id, &args.enclave_id, &args.address)
        .await?;

    // Step 4: Decrypt refined data
    tracing::info!("🔓 Step 4: Decrypting refined data...");
    let decrypted_data = services
        .seal
        .decrypt_file(
            &encrypted_object.id,
            &attestation_obj_id,
            &refined_data_encrypted,
            &args.address,
            &args.on_chain_file_obj_id,
            &args.policy_object_id,
            &args.threshold,
            &services.sui,
        )
        .await?;

    // Step 5: Process messages individually with embeddings
    tracing::info!("🔤 Step 5: Processing messages individually with embeddings...");
    let result = process_messages_by_message(decrypted_data, &services, &args).await?;

    Ok(TaskResult::success("embedding", Some(result)))
}

pub async fn run_retrieve_operation(
    args: RetrieveArgs,
    config: &TaskConfig,
) -> Result<TaskResult> {
    tracing::info!("🔍 Running Message Retrieval Operation...");

    let factory = ServiceFactory::new(config.clone());
    let services = factory.create_all_services().await?;

    // Step 1: Connect to vector database
    tracing::info!("📦 Step 1: Connecting to vector database...");
    if !services.vector_db.is_connected() {
        services.vector_db.connect().await?;
    }

    // Step 2: Generate embedding for the query
    tracing::info!("🔤 Step 2: Generating embedding for query...");
    let query_embedding = services.embedding.embed_single(&args.query).await?;

    tracing::info!("✅ Query embedding generated ({} dimensions)", query_embedding.len());

    // Step 3: Search for similar vectors in the database
    tracing::info!("🔍 Step 3: Searching for similar messages...");
    let limit = args.processing_config.limit.unwrap_or(10);
    let search_results = services.vector_db.search(&query_embedding, limit, None).await?;

    tracing::info!("🔍 Found {} similar messages", search_results.len());

    if search_results.is_empty() {
        let result = serde_json::json!({
            "query": args.query,
            "results": [],
            "count": 0,
            "message": "No similar messages found"
        });
        return Ok(TaskResult::success("retrieve", Some(result)));
    }

    // Step 4: Decrypt each message
    tracing::info!("🔓 Step 4: Decrypting messages...");
    let mut decrypted_messages: Vec<serde_json::Value> = Vec::new();

    for (i, search_result) in search_results.iter().enumerate() {
        tracing::info!(
            "🔓 Decrypting message {}/{} (ID: {}, Score: {:.4})",
            i + 1,
            search_results.len(),
            search_result.metadata.get("message_id").unwrap_or(&serde_json::Value::Null),
            search_result.score
        );

        // Implementation would continue with decryption logic..
        // This is a simplified version for now
    }

    let result = serde_json::json!({
        "query": args.query,
        "results": decrypted_messages,
        "count": decrypted_messages.len()
    });

    Ok(TaskResult::success("retrieve", Some(result)))
}

pub async fn run_retrieve_by_blob_ids_operation(
    args: RetrieveByBlobIdsArgs,
    config: &TaskConfig,
) -> Result<TaskResult> {
    tracing::info!("📦 Running Message Retrieval by Blob IDs Operation...");

    let factory = ServiceFactory::new(config.clone());
    let _services = factory.create_all_services().await?;

    let decrypted_messages: Vec<serde_json::Value> = Vec::new();

    for (i, pair) in args.blob_file_pairs.iter().enumerate() {
        tracing::info!(
            "📦 Processing pair {}/{}: Blob ID: {}, File ID: {}, Policy ID: {}",
            i + 1,
            args.blob_file_pairs.len(),
            pair.walrus_blob_id,
            pair.on_chain_file_obj_id,
            pair.policy_object_id
        );

        // Implementation would continue with decryption logic..
    }

    let result = serde_json::json!({
        "requested_pairs": args.blob_file_pairs,
        "results": decrypted_messages,
        "total_requested": args.blob_file_pairs.len()
    });

    Ok(TaskResult::success("retrieve-by-blob-ids", Some(result)))
}

pub async fn run_default_operation(args: DefaultArgs, config: &TaskConfig) -> Result<TaskResult> {
    tracing::info!("📝 Running Default (Refinement) Operation...");

    let factory = ServiceFactory::new(config.clone());
    let services = factory.create_all_services().await?;

    // Step 1: Fetch encrypted file from Walrus
    tracing::info!("📥 Step 1: Fetching encrypted file...");
    let _encrypted_file = services.walrus.fetch_encrypted_file(&args.blob_id).await?;

    // Continue with implementation..

    let result = serde_json::json!({
        "message": "Default operation completed"
    });

    Ok(TaskResult::success("default", Some(result)))
}

async fn process_messages_by_message(
    _decrypted_data: Value,
    _services: &crate::services::Services,
    _args: &EmbeddingArgs,
) -> Result<Value> {
    // Implementation would process messages individually
    // This is a placeholder for now
    Ok(serde_json::json!({
        "status": "success",
        "operation": "embedding",
        "processedCount": 0,
        "message": "Processing completed"
    }))
}