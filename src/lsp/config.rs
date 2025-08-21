//! Configuration management for LSP integration
//! 
//! This module provides configuration structures and management for
//! the LSP integration, including rust-analyzer settings, caching
//! configuration, and fallback strategies.

use std::path::PathBuf;
use std::time::Duration;
use serde::{Deserialize, Serialize};

use super::{DEFAULT_INIT_TIMEOUT, DEFAULT_REQUEST_TIMEOUT, DEFAULT_CACHE_TTL, DEFAULT_MAX_MEMORY};

/// Main LSP configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspConfig {
    /// LSP server configuration
    pub server: LspServerConfig,
    /// Caching configuration
    pub cache: CacheConfig,
    /// Fallback configuration
    pub fallback: FallbackConfig,
    /// Performance tuning
    pub performance: PerformanceConfig,
    /// Feature flags
    pub features: FeatureConfig,
}

/// LSP server specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspServerConfig {
    /// Path to rust-analyzer executable
    pub executable_path: String,
    /// Arguments to pass to rust-analyzer
    pub args: Vec<String>,
    /// Initialization timeout
    pub init_timeout: Duration,
    /// Request timeout
    pub request_timeout: Duration,
    /// Number of retry attempts for failed requests
    pub retry_attempts: u32,
    /// Delay between retry attempts
    pub retry_delay: Duration,
    /// Whether to enable rust-analyzer's check on save
    pub check_on_save: bool,
    /// Whether to enable proc macro expansion
    pub proc_macro_enable: bool,
    /// Whether to enable build scripts
    pub build_scripts_enable: bool,
    /// Additional rust-analyzer settings
    pub additional_settings: serde_json::Value,
}

/// Caching configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Whether caching is enabled
    pub enabled: bool,
    /// Time-to-live for cache entries
    pub ttl: Duration,
    /// Maximum number of cache entries
    pub max_entries: usize,
    /// Maximum memory usage for cache
    pub max_memory_bytes: usize,
    /// Cache cleanup interval
    pub cleanup_interval: Duration,
    /// Whether to use persistent cache (Memgraph)
    pub persistent: bool,
    /// Cache invalidation strategy
    pub invalidation: CacheInvalidationConfig,
}

/// Cache invalidation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheInvalidationConfig {
    /// Invalidate on file changes
    pub on_file_change: bool,
    /// Invalidate on dependency changes
    pub on_dependency_change: bool,
    /// Invalidate on workspace changes
    pub on_workspace_change: bool,
    /// File patterns to watch for changes
    pub watch_patterns: Vec<String>,
    /// File patterns to ignore for invalidation
    pub ignore_patterns: Vec<String>,
}

/// Fallback configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackConfig {
    /// Enable graceful fallback to tree-sitter
    pub enable_graceful_fallback: bool,
    /// Timeout before falling back to tree-sitter
    pub fallback_timeout: Duration,
    /// Whether to show warnings when falling back
    pub show_warnings: bool,
    /// Whether to retry LSP on fallback
    pub retry_on_fallback: bool,
    /// Maximum number of fallback retries
    pub max_retry_attempts: u32,
    /// Retry interval for LSP recovery
    pub retry_interval: Duration,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Maximum concurrent LSP requests
    pub max_concurrent_requests: usize,
    /// Request batching size
    pub batch_size: usize,
    /// Request batching timeout
    pub batch_timeout: Duration,
    /// Background processing thread count
    pub background_threads: usize,
    /// Memory pressure threshold (bytes)
    pub memory_pressure_threshold: usize,
    /// CPU usage threshold (percentage)
    pub cpu_usage_threshold: f64,
}

/// Feature configuration flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureConfig {
    /// Enable semantic tokens
    pub semantic_tokens: bool,
    /// Enable document symbols
    pub document_symbols: bool,
    /// Enable workspace symbols
    pub workspace_symbols: bool,
    /// Enable references
    pub references: bool,
    /// Enable definition lookup
    pub definition: bool,
    /// Enable hover information
    pub hover: bool,
    /// Enable completion
    pub completion: bool,
    /// Enable diagnostics
    pub diagnostics: bool,
    /// Enable code actions
    pub code_actions: bool,
    /// Enable incremental sync
    pub incremental_sync: bool,
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            server: LspServerConfig::default(),
            cache: CacheConfig::default(),
            fallback: FallbackConfig::default(),
            performance: PerformanceConfig::default(),
            features: FeatureConfig::default(),
        }
    }
}

impl Default for LspServerConfig {
    fn default() -> Self {
        Self {
            executable_path: "rust-analyzer".to_string(),
            args: vec![],
            init_timeout: DEFAULT_INIT_TIMEOUT,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            retry_attempts: 3,
            retry_delay: Duration::from_millis(500),
            check_on_save: true,
            proc_macro_enable: true,
            build_scripts_enable: true,
            additional_settings: serde_json::json!({}),
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl: DEFAULT_CACHE_TTL,
            max_entries: 10000,
            max_memory_bytes: DEFAULT_MAX_MEMORY,
            cleanup_interval: Duration::from_secs(600), // 10 minutes
            persistent: true,
            invalidation: CacheInvalidationConfig::default(),
        }
    }
}

impl Default for CacheInvalidationConfig {
    fn default() -> Self {
        Self {
            on_file_change: true,
            on_dependency_change: true,
            on_workspace_change: true,
            watch_patterns: vec![
                "**/*.rs".to_string(),
                "**/Cargo.toml".to_string(),
                "**/Cargo.lock".to_string(),
            ],
            ignore_patterns: vec![
                "**/target/**".to_string(),
                "**/.git/**".to_string(),
                "**/node_modules/**".to_string(),
            ],
        }
    }
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            enable_graceful_fallback: true,
            fallback_timeout: Duration::from_secs(2),
            show_warnings: true,
            retry_on_fallback: true,
            max_retry_attempts: 3,
            retry_interval: Duration::from_secs(10),
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_concurrent_requests: 10,
            batch_size: 5,
            batch_timeout: Duration::from_millis(100),
            background_threads: num_cpus::get(),
            memory_pressure_threshold: 200 * 1024 * 1024, // 200MB
            cpu_usage_threshold: 80.0,
        }
    }
}

impl Default for FeatureConfig {
    fn default() -> Self {
        Self {
            semantic_tokens: true,
            document_symbols: true,
            workspace_symbols: true,
            references: true,
            definition: true,
            hover: true,
            completion: false, // Can be resource intensive
            diagnostics: true,
            code_actions: false, // Not needed for analysis
            incremental_sync: true,
        }
    }
}

/// Configuration builder for easier setup
pub struct LspConfigBuilder {
    config: LspConfig,
}

impl LspConfigBuilder {
    /// Create a new configuration builder
    pub fn new() -> Self {
        Self {
            config: LspConfig::default(),
        }
    }

    /// Set rust-analyzer executable path
    pub fn executable_path<P: Into<String>>(mut self, path: P) -> Self {
        self.config.server.executable_path = path.into();
        self
    }

    /// Set initialization timeout
    pub fn init_timeout(mut self, timeout: Duration) -> Self {
        self.config.server.init_timeout = timeout;
        self
    }

    /// Set request timeout
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.config.server.request_timeout = timeout;
        self
    }

    /// Enable or disable caching
    pub fn enable_cache(mut self, enabled: bool) -> Self {
        self.config.cache.enabled = enabled;
        self
    }

    /// Set cache TTL
    pub fn cache_ttl(mut self, ttl: Duration) -> Self {
        self.config.cache.ttl = ttl;
        self
    }

    /// Set maximum cache memory usage
    pub fn max_cache_memory(mut self, bytes: usize) -> Self {
        self.config.cache.max_memory_bytes = bytes;
        self
    }

    /// Enable or disable graceful fallback
    pub fn enable_fallback(mut self, enabled: bool) -> Self {
        self.config.fallback.enable_graceful_fallback = enabled;
        self
    }

    /// Set fallback timeout
    pub fn fallback_timeout(mut self, timeout: Duration) -> Self {
        self.config.fallback.fallback_timeout = timeout;
        self
    }

    /// Set maximum concurrent requests
    pub fn max_concurrent_requests(mut self, max: usize) -> Self {
        self.config.performance.max_concurrent_requests = max;
        self
    }

    /// Enable or disable specific features
    pub fn features(mut self, features: FeatureConfig) -> Self {
        self.config.features = features;
        self
    }

    /// Build the configuration
    pub fn build(self) -> LspConfig {
        self.config
    }
}

impl Default for LspConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Load configuration from file
pub fn load_config_from_file(path: &std::path::Path) -> Result<LspConfig, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let config: LspConfig = match path.extension().and_then(|s| s.to_str()) {
        Some("toml") => toml::from_str(&content)?,
        Some("json") => serde_json::from_str(&content)?,
        Some("yaml") | Some("yml") => serde_yaml::from_str(&content)?,
        _ => return Err("Unsupported configuration file format".into()),
    };
    Ok(config)
}

/// Save configuration to file
pub fn save_config_to_file(
    config: &LspConfig,
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = match path.extension().and_then(|s| s.to_str()) {
        Some("toml") => toml::to_string_pretty(config)?,
        Some("json") => serde_json::to_string_pretty(config)?,
        Some("yaml") | Some("yml") => serde_yaml::to_string(config)?,
        _ => return Err("Unsupported configuration file format".into()),
    };
    std::fs::write(path, content)?;
    Ok(())
}

/// Get default configuration file path
pub fn default_config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("rust-workspace-analyzer");
    path.push("lsp_config.toml");
    path
}

/// Create default configuration file if it doesn't exist
pub fn ensure_default_config() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let config_path = default_config_path();
    
    if !config_path.exists() {
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let default_config = LspConfig::default();
        save_config_to_file(&default_config, &config_path)?;
    }
    
    Ok(config_path)
}

/// Validate configuration
pub fn validate_config(config: &LspConfig) -> Result<(), String> {
    // Validate server configuration
    if config.server.executable_path.is_empty() {
        return Err("LSP executable path cannot be empty".to_string());
    }

    if config.server.init_timeout.as_secs() == 0 {
        return Err("Initialization timeout must be greater than 0".to_string());
    }

    if config.server.request_timeout.as_secs() == 0 {
        return Err("Request timeout must be greater than 0".to_string());
    }

    // Validate cache configuration
    if config.cache.enabled {
        if config.cache.max_entries == 0 {
            return Err("Cache max entries must be greater than 0".to_string());
        }

        if config.cache.max_memory_bytes == 0 {
            return Err("Cache max memory must be greater than 0".to_string());
        }

        if config.cache.ttl.as_secs() == 0 {
            return Err("Cache TTL must be greater than 0".to_string());
        }
    }

    // Validate performance configuration
    if config.performance.max_concurrent_requests == 0 {
        return Err("Max concurrent requests must be greater than 0".to_string());
    }

    if config.performance.batch_size == 0 {
        return Err("Batch size must be greater than 0".to_string());
    }

    if config.performance.background_threads == 0 {
        return Err("Background threads must be greater than 0".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = LspConfig::default();
        assert_eq!(config.server.executable_path, "rust-analyzer");
        assert!(config.cache.enabled);
        assert!(config.fallback.enable_graceful_fallback);
        assert!(config.features.semantic_tokens);
    }

    #[test]
    fn test_config_builder() {
        let config = LspConfigBuilder::new()
            .executable_path("/custom/rust-analyzer")
            .init_timeout(Duration::from_secs(60))
            .enable_cache(false)
            .max_concurrent_requests(20)
            .build();

        assert_eq!(config.server.executable_path, "/custom/rust-analyzer");
        assert_eq!(config.server.init_timeout, Duration::from_secs(60));
        assert!(!config.cache.enabled);
        assert_eq!(config.performance.max_concurrent_requests, 20);
    }

    #[test]
    fn test_config_validation() {
        let mut config = LspConfig::default();
        assert!(validate_config(&config).is_ok());

        config.server.executable_path = String::new();
        assert!(validate_config(&config).is_err());

        config = LspConfig::default();
        config.server.init_timeout = Duration::from_secs(0);
        assert!(validate_config(&config).is_err());

        config = LspConfig::default();
        config.cache.max_entries = 0;
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn test_config_serialization() {
        let config = LspConfig::default();
        
        // Test JSON serialization
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: LspConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.server.executable_path, deserialized.server.executable_path);

        // Test TOML serialization
        let toml = toml::to_string(&config).unwrap();
        let deserialized: LspConfig = toml::from_str(&toml).unwrap();
        assert_eq!(config.server.executable_path, deserialized.server.executable_path);
    }

    #[test]
    fn test_save_and_load_config() {
        let config = LspConfigBuilder::new()
            .executable_path("/test/rust-analyzer")
            .init_timeout(Duration::from_secs(45))
            .build();

        // Test with TOML file
        let temp_file = NamedTempFile::new().unwrap();
        let mut toml_path = temp_file.path().to_path_buf();
        toml_path.set_extension("toml");

        save_config_to_file(&config, &toml_path).unwrap();
        let loaded_config = load_config_from_file(&toml_path).unwrap();

        assert_eq!(config.server.executable_path, loaded_config.server.executable_path);
        assert_eq!(config.server.init_timeout, loaded_config.server.init_timeout);

        // Clean up
        let _ = std::fs::remove_file(&toml_path);
    }

    #[test]
    fn test_feature_config_defaults() {
        let features = FeatureConfig::default();
        assert!(features.semantic_tokens);
        assert!(features.document_symbols);
        assert!(features.references);
        assert!(features.definition);
        assert!(!features.completion); // Disabled by default
        assert!(!features.code_actions); // Disabled by default
    }

    #[test]
    fn test_cache_invalidation_config() {
        let invalidation = CacheInvalidationConfig::default();
        assert!(invalidation.on_file_change);
        assert!(invalidation.on_dependency_change);
        assert!(invalidation.watch_patterns.contains(&"**/*.rs".to_string()));
        assert!(invalidation.ignore_patterns.contains(&"**/target/**".to_string()));
    }
}