pub mod manager;
pub mod ollama;
pub mod registry;

pub use manager::LocalModelManager;
pub use ollama::OllamaClient;
pub use registry::{LocalModelMeta, LocalModelProfile, MemoryTier};
