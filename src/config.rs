use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub workspace: WorkspaceConfig,
    pub analysis: AnalysisConfig,
    pub architecture: ArchitectureConfig,
    pub memgraph: MemgraphConfig,
    pub embeddings: EmbeddingsConfig,
    pub performance: PerformanceConfig,
    pub framework: FrameworkConfig,
    pub cross_crate: CrossCrateConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub root: PathBuf,
    #[serde(default)]
    pub additional_roots: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    #[serde(default = "default_true")]
    pub recursive_scan: bool,
    #[serde(default = "default_true")]
    pub include_dev_deps: bool,
    #[serde(default)]
    pub include_build_deps: bool,
    #[serde(default = "default_true")]
    pub workspace_members_only: bool,
    #[serde(default)]
    pub exclude_crates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureConfig {
    pub layers: Vec<Layer>,
    #[serde(skip)]
    layer_index_cache: Option<HashMap<String, usize>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    pub crates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemgraphConfig {
    pub uri: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub clean_start: bool,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    #[serde(default)]
    pub performance: MemgraphPerformanceConfig,
    #[serde(default)]
    pub retry: MemgraphRetryConfig,
    #[serde(default)]
    pub memory: MemgraphMemoryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemgraphPerformanceConfig {
    #[serde(default = "default_true")]
    pub use_analytical_mode: bool,
    #[serde(default = "default_connection_pool_size")]
    pub connection_pool_size: u32,
    #[serde(default = "default_connection_timeout_ms")]
    pub connection_timeout_ms: u64,
    #[serde(default = "default_query_timeout_ms")]
    pub query_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemgraphRetryConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    #[serde(default = "default_initial_delay_ms")]
    pub initial_delay_ms: u64,
    #[serde(default = "default_max_delay_ms")]
    pub max_delay_ms: u64,
    #[serde(default = "default_exponential_base")]
    pub exponential_base: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemgraphMemoryConfig {
    #[serde(default = "default_monitor_interval_ms")]
    pub monitor_interval_ms: u64,
    #[serde(default = "default_auto_free_threshold_mb")]
    pub auto_free_threshold_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_embedding_model")]
    pub model: String,
    #[serde(default = "default_embedding_fields")]
    pub include_in_embedding: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    #[serde(default = "default_max_threads")]
    pub max_threads: usize,
    #[serde(default = "default_cache_size")]
    pub cache_size_mb: usize,
    #[serde(default = "default_true")]
    pub incremental: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_framework_patterns_path")]
    pub patterns_path: Option<PathBuf>,
    #[serde(default)]
    pub excluded_functions: Vec<String>,
    #[serde(default = "default_true")]
    pub synthetic_call_generation: bool,
    #[serde(default)]
    pub custom_patterns: Vec<String>,
    #[serde(default = "default_supported_frameworks")]
    pub supported_frameworks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossCrateConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_global_index_path")]
    pub global_index_path: Option<PathBuf>,
    #[serde(default = "default_true")]
    pub use_cache: bool,
    #[serde(default = "default_true")]
    pub incremental_updates: bool,
    #[serde(default = "default_max_index_memory_mb")]
    pub max_index_memory_mb: usize,
}

fn default_true() -> bool {
    true
}

fn default_batch_size() -> usize {
    1000
}

fn default_embedding_model() -> String {
    "text-embedding-3-small".to_string()
}

fn default_embedding_fields() -> Vec<String> {
    vec![
        "function_name".to_string(),
        "module_path".to_string(),
        "crate_name".to_string(),
        "doc_comments".to_string(),
        "parameter_types".to_string(),
        "return_type".to_string(),
    ]
}

fn default_max_threads() -> usize {
    num_cpus::get()
}

fn default_cache_size() -> usize {
    100
}

fn default_connection_pool_size() -> u32 {
    4
}

fn default_connection_timeout_ms() -> u64 {
    5000
}

fn default_query_timeout_ms() -> u64 {
    30000
}

fn default_max_attempts() -> u32 {
    5
}

fn default_initial_delay_ms() -> u64 {
    100
}

fn default_max_delay_ms() -> u64 {
    5000
}

fn default_exponential_base() -> f64 {
    2.0
}

fn default_monitor_interval_ms() -> u64 {
    60000
}

fn default_auto_free_threshold_mb() -> f64 {
    1000.0
}

fn default_framework_patterns_path() -> Option<PathBuf> {
    None
}

fn default_supported_frameworks() -> Vec<String> {
    vec![
        "tokio".to_string(),
        "actix-web".to_string(),
        "async-std".to_string(),
        "websocket".to_string(),
    ]
}

fn default_global_index_path() -> Option<PathBuf> {
    None
}

fn default_max_index_memory_mb() -> usize {
    100
}

impl Default for MemgraphPerformanceConfig {
    fn default() -> Self {
        Self {
            use_analytical_mode: default_true(),
            connection_pool_size: default_connection_pool_size(),
            connection_timeout_ms: default_connection_timeout_ms(),
            query_timeout_ms: default_query_timeout_ms(),
        }
    }
}

impl Default for MemgraphRetryConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            max_attempts: default_max_attempts(),
            initial_delay_ms: default_initial_delay_ms(),
            max_delay_ms: default_max_delay_ms(),
            exponential_base: default_exponential_base(),
        }
    }
}

impl Default for MemgraphMemoryConfig {
    fn default() -> Self {
        Self {
            monitor_interval_ms: default_monitor_interval_ms(),
            auto_free_threshold_mb: default_auto_free_threshold_mb(),
        }
    }
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path))?;
        
        let mut config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path))?;
        
        config.init_caches();
        Ok(config)
    }

    pub fn from_workspace_root<P: AsRef<std::path::Path>>(workspace_root: P) -> Result<Self> {
        let mut config = Self::default();
        config.workspace.root = workspace_root.as_ref().to_path_buf();
        config.init_caches();
        Ok(config)
    }

    pub fn all_workspace_roots(&self) -> impl Iterator<Item = &PathBuf> {
        std::iter::once(&self.workspace.root).chain(self.workspace.additional_roots.iter())
    }

    fn init_caches(&mut self) {
        let mut layer_index = HashMap::new();
        for (idx, layer) in self.architecture.layers.iter().enumerate() {
            for crate_name in &layer.crates {
                layer_index.insert(crate_name.clone(), idx);
            }
        }
        self.architecture.layer_index_cache = Some(layer_index);
    }

    pub fn get_layer_index(&self, crate_name: &str) -> Option<usize> {
        self.architecture.layer_index_cache.as_ref()?.get(crate_name).copied()
    }

    pub fn get_layer_name(&self, index: usize) -> Option<&str> {
        self.architecture.layers.get(index).map(|layer| layer.name.as_str())
    }

    pub fn is_layer_violation(&self, from_crate: &str, to_crate: &str) -> bool {
        if let (Some(from_idx), Some(to_idx)) = 
            (self.get_layer_index(from_crate), self.get_layer_index(to_crate)) {
            from_idx < to_idx
        } else {
            false
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            workspace: WorkspaceConfig {
                root: PathBuf::from("."),
                additional_roots: Vec::new(),
            },
            analysis: AnalysisConfig {
                recursive_scan: true,
                include_dev_deps: true,
                include_build_deps: false,
                workspace_members_only: true,
                exclude_crates: Vec::new(),
            },
            architecture: ArchitectureConfig {
                layers: Vec::new(),
                layer_index_cache: None,
            },
            memgraph: MemgraphConfig {
                uri: "bolt://localhost:7687".to_string(),
                username: String::new(),
                password: String::new(),
                clean_start: false,
                batch_size: default_batch_size(),
                performance: MemgraphPerformanceConfig::default(),
                retry: MemgraphRetryConfig::default(),
                memory: MemgraphMemoryConfig::default(),
            },
            embeddings: EmbeddingsConfig {
                enabled: true,
                model: default_embedding_model(),
                include_in_embedding: default_embedding_fields(),
            },
            performance: PerformanceConfig {
                max_threads: default_max_threads(),
                cache_size_mb: default_cache_size(),
                incremental: true,
            },
            framework: FrameworkConfig {
                enabled: true,
                patterns_path: None,
                excluded_functions: Vec::new(),
                synthetic_call_generation: true,
                custom_patterns: Vec::new(),
                supported_frameworks: default_supported_frameworks(),
            },
            cross_crate: CrossCrateConfig {
                enabled: true,
                global_index_path: None,
                use_cache: true,
                incremental_updates: true,
                max_index_memory_mb: default_max_index_memory_mb(),
            },
        }
    }
}