pub mod config;
pub mod workspace;
pub mod parser;
pub mod graph;
pub mod architecture;
pub mod embeddings;
pub mod incremental;
pub mod mcp;
pub mod analyzer;
pub mod lsp;

pub use config::Config;
pub use workspace::{WorkspaceDiscovery, CrateMetadata};
pub use parser::{RustParser, ParsedSymbols, RustFunction, RustType};
pub use graph::MemgraphClient;
pub use architecture::{ArchitectureAnalyzer, ArchitectureViolation};
pub use embeddings::{EmbeddingGenerator, SemanticSearch};
pub use incremental::IncrementalUpdater;
pub use analyzer::{WorkspaceAnalyzer, WorkspaceSnapshot, HybridWorkspaceAnalyzer};

#[cfg(test)]
pub mod tests;
