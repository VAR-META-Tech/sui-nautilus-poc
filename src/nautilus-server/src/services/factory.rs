use crate::services::{
    blockchain::{SealOperations, SuiOperations, WalrusOperations},
    embedding::OllamaEmbedding,
    refinement::ChatRefinement,
    vector_db::QdrantService,
    Services,
};
use crate::config::TaskConfig;
use anyhow::Result;

pub struct ServiceFactory {
    config: TaskConfig,
}

impl ServiceFactory {
    pub fn new(config: TaskConfig) -> Self {
        Self { config }
    }

    pub async fn create_all_services(&self) -> Result<Services> {
        tracing::info!("🔧 Initializing all services...");

        let sui = self.create_sui_service().await?;
        let walrus = self.create_walrus_service();
        let seal = self.create_seal_service().await?;
        let embedding = self.create_embedding_service();
        let vector_db = self.create_vector_db_service();
        let refinement = self.create_refinement_service();

        tracing::info!("✅ All services initialized successfully");

        Ok(Services::new(
            sui, walrus, seal, embedding, vector_db, refinement,
        ))
    }

    pub async fn create_sui_service(&self) -> Result<SuiOperations> {
        let mut sui = SuiOperations::new(
            self.config.move_package_id.clone(),
            self.config.sui_secret_key.clone(),
        )?;
        sui.initialize().await?;
        Ok(sui)
    }

    pub fn create_walrus_service(&self) -> WalrusOperations {
        WalrusOperations::new(self.config.walrus_config.clone())
    }

    pub async fn create_seal_service(&self) -> Result<SealOperations> {
        SealOperations::new(self.config.move_package_id.clone()).await
    }

    pub fn create_embedding_service(&self) -> OllamaEmbedding {
        OllamaEmbedding::new(self.config.ollama_config.clone())
    }

    pub fn create_vector_db_service(&self) -> QdrantService {
        QdrantService::new(self.config.qdrant_config.clone())
    }

    pub fn create_refinement_service(&self) -> ChatRefinement {
        ChatRefinement::new()
    }
}