pub mod base_embedding;
pub mod ollama_embedding;

pub use base_embedding::{BaseEmbedding, EmbeddingStats, EmbeddingResult};
pub use ollama_embedding::OllamaEmbedding;