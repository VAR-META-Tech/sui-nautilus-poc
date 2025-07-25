pub mod blockchain;
pub mod embedding;
pub mod factory;
pub mod refinement;
pub mod vector_db;

use crate::services::{
    blockchain::{SealOperations, SuiOperations, WalrusOperations},
    embedding::OllamaEmbedding,
    refinement::ChatRefinement,
    vector_db::QdrantService,
};

pub struct Services {
    pub sui: SuiOperations,
    pub walrus: WalrusOperations,
    pub seal: SealOperations,
    pub embedding: OllamaEmbedding,
    pub vector_db: QdrantService,
    pub refinement: ChatRefinement,
}

impl Services {
    pub fn new(
        sui: SuiOperations,
        walrus: WalrusOperations,
        seal: SealOperations,
        embedding: OllamaEmbedding,
        vector_db: QdrantService,
        refinement: ChatRefinement,
    ) -> Self {
        Self {
            sui,
            walrus,
            seal,
            embedding,
            vector_db,
            refinement,
        }
    }
}