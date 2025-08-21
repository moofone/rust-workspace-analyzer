//! LSP (Language Server Protocol) integration module
//! 
//! This module provides integration with rust-analyzer via LSP to enhance
//! the tree-sitter based analysis with semantic information.
//! 
//! # Architecture
//! 
//! The LSP integration follows a hybrid approach:
//! 1. Tree-sitter provides fast initial analysis
//! 2. LSP enhances results with semantic accuracy
//! 3. Results are cached in Memgraph for performance
//! 4. Graceful fallback when LSP is unavailable
//! 
//! # Modules
//! 
//! - `models`: Data structures for LSP-enhanced analysis
//! - `client`: LSP client wrapper for rust-analyzer
//! - `manager`: Background LSP lifecycle management
//! - `config`: Configuration management for LSP settings

pub mod models;
pub mod client;
pub mod manager;
pub mod config;

// Re-exports for convenient access
pub use models::{
    LspEnhancedFunction, LspSymbol, TypeInfo, LspReference,
    HybridAnalysisResult, LspEnhancements, MergeStrategy,
    ReferenceContext, SymbolKind, Range, Location
};

pub use client::{LspClient, LspClientError, ConnectionStatus};
pub use manager::{LspManager, LspManagerConfig, BackgroundTask};
pub use config::{LspConfig, CacheConfig, FallbackConfig};

use std::time::Duration;
use anyhow::Result;

/// Default timeout for LSP initialization
pub const DEFAULT_INIT_TIMEOUT: Duration = Duration::from_secs(30);

/// Default timeout for LSP requests
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Default cache TTL
pub const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60); // 24 hours

/// Maximum memory usage for LSP caching (100MB)
pub const DEFAULT_MAX_MEMORY: usize = 100 * 1024 * 1024;

/// LSP integration error types
#[derive(Debug, thiserror::Error)]
pub enum LspError {
    #[error("LSP server initialization failed: {0}")]
    InitializationFailed(String),
    
    #[error("LSP server connection lost: {0}")]
    ConnectionLost(String),
    
    #[error("LSP request timeout after {timeout:?}")]
    RequestTimeout { timeout: Duration },
    
    #[error("LSP server returned error: {0}")]
    ServerError(String),
    
    #[error("LSP response parsing failed: {0}")]
    ParseError(String),
    
    #[error("Cache operation failed: {0}")]
    CacheError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

/// Result type for LSP operations
pub type LspResult<T> = Result<T, LspError>;

// Implement From trait for common error conversions
impl From<serde_json::Error> for LspError {
    fn from(err: serde_json::Error) -> Self {
        LspError::ParseError(err.to_string())
    }
}

/// Status of the LSP integration
#[derive(Debug, Clone, PartialEq)]
pub enum LspStatus {
    /// LSP is not initialized
    Uninitialized,
    /// LSP is initializing
    Initializing,
    /// LSP is ready and available
    Ready,
    /// LSP is temporarily unavailable
    Unavailable(String),
    /// LSP has failed and requires restart
    Failed(String),
}

/// Strategy for analyzing with LSP
#[derive(Debug, Clone, PartialEq)]
pub enum AnalysisStrategy {
    /// Use only tree-sitter (fast but limited semantic info)
    TreeSitterOnly,
    /// Use tree-sitter first, enhance with LSP if available
    Progressive,
    /// Use cached LSP data only (fastest when available)
    CachedLspOnly,
    /// Use tree-sitter with warnings about LSP unavailability
    TreeSitterWithWarning,
    /// Hybrid approach with intelligent merging
    HybridIntelligent,
}

impl Default for AnalysisStrategy {
    fn default() -> Self {
        Self::Progressive
    }
}

/// Entry point for LSP-enhanced workspace analysis
pub async fn create_hybrid_analyzer(
    workspace_path: &std::path::Path,
    config: Option<LspConfig>,
) -> LspResult<manager::LspManager> {
    let config = config.unwrap_or_default();
    manager::LspManager::new(workspace_path, config).await
}

/// Utility function to check if LSP is available on the system
pub async fn check_lsp_availability() -> bool {
    // Try to run rust-analyzer --version
    match tokio::process::Command::new("rust-analyzer")
        .arg("--version")
        .output()
        .await
    {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// Initialize LSP with default configuration
pub async fn init_default_lsp(workspace_path: &std::path::Path) -> LspResult<manager::LspManager> {
    let config = LspConfig::default();
    manager::LspManager::new(workspace_path, config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[tokio::test]
    async fn test_lsp_availability_check() {
        // This test checks if rust-analyzer is available
        // Result depends on system setup, so we just ensure it doesn't panic
        let _available = check_lsp_availability().await;
    }

    #[test]
    fn test_default_values() {
        assert_eq!(DEFAULT_INIT_TIMEOUT, Duration::from_secs(30));
        assert_eq!(DEFAULT_REQUEST_TIMEOUT, Duration::from_secs(5));
        assert_eq!(DEFAULT_CACHE_TTL, Duration::from_secs(24 * 60 * 60));
        assert_eq!(DEFAULT_MAX_MEMORY, 100 * 1024 * 1024);
    }

    #[test]
    fn test_analysis_strategy_default() {
        assert_eq!(AnalysisStrategy::default(), AnalysisStrategy::Progressive);
    }

    #[test]
    fn test_lsp_status_variants() {
        let status = LspStatus::Uninitialized;
        assert_eq!(status, LspStatus::Uninitialized);
        
        let status = LspStatus::Failed("Test error".to_string());
        match status {
            LspStatus::Failed(msg) => assert_eq!(msg, "Test error"),
            _ => panic!("Expected Failed status"),
        }
    }
}