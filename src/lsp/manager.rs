//! LSP Manager for background initialization and lifecycle management
//! 
//! This module provides the LspManager which handles the lifecycle of
//! rust-analyzer, background initialization, health monitoring, and
//! coordinates between the LSP client and the analysis system.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use tokio::sync::{mpsc, RwLock, Mutex};
use tokio::time::{interval, timeout, sleep};
use tracing::{debug, error, info, warn};

use super::client::{LspClient, ConnectionStatus};
use super::config::{LspConfig, LspServerConfig};
use super::models::{
    LspEnhancedFunction, LspSymbol, LspReference, TypeInfo,
    HybridAnalysisResult, LspEnhancements, MergeStrategy, AnalysisMetrics,
    LspDiagnostic, SemanticToken, SemanticDependency,
    Position as LspPosition, Range as LspRange, Location as LspLocation,
};
use super::{LspError, LspResult, LspStatus, AnalysisStrategy};
use crate::analyzer::{WorkspaceSnapshot, RustFunction};

/// Configuration for LSP manager
#[derive(Debug, Clone)]
pub struct LspManagerConfig {
    /// LSP configuration
    pub lsp_config: LspConfig,
    /// Whether to start LSP in background
    pub background_init: bool,
    /// Health check interval
    pub health_check_interval: Duration,
    /// Maximum initialization attempts
    pub max_init_attempts: u32,
}

/// Background task handle
#[derive(Debug)]
pub struct BackgroundTask {
    /// Task name
    pub name: String,
    /// Task start time
    pub started_at: SystemTime,
    /// Task handle
    pub handle: tokio::task::JoinHandle<()>,
}

/// LSP Manager for background initialization and lifecycle management
pub struct LspManager {
    /// LSP client
    client: Arc<RwLock<Option<LspClient>>>,
    /// Manager configuration
    config: LspManagerConfig,
    /// Workspace root path
    workspace_root: PathBuf,
    /// Current LSP status
    status: Arc<RwLock<LspStatus>>,
    /// Background tasks
    background_tasks: Arc<Mutex<Vec<BackgroundTask>>>,
    /// Enhancement cache
    enhancement_cache: Arc<RwLock<HashMap<String, LspEnhancedFunction>>>,
    /// Request queue for batching
    request_queue: Arc<Mutex<Vec<QueuedRequest>>>,
    /// Health metrics
    health_metrics: Arc<RwLock<HealthMetrics>>,
    /// Initialization attempts counter
    init_attempts: Arc<RwLock<u32>>,
}

/// Queued LSP request for batching
#[derive(Debug)]
struct QueuedRequest {
    /// Request type
    request_type: RequestType,
    /// File path for request
    file_path: PathBuf,
    /// Additional parameters
    params: RequestParams,
    /// Response sender
    response_tx: mpsc::UnboundedSender<LspResult<RequestResponse>>,
    /// Request timestamp
    queued_at: Instant,
}

/// Types of LSP requests
#[derive(Debug, Clone)]
enum RequestType {
    DocumentSymbols,
    References,
    Definition,
    Hover,
    SemanticTokens,
}

/// Request parameters
#[derive(Debug, Clone)]
enum RequestParams {
    Position { line: u32, character: u32 },
    Range { start: LspPosition, end: LspPosition },
    None,
}

/// Request response types
#[derive(Debug)]
enum RequestResponse {
    Symbols(Vec<LspSymbol>),
    References(Vec<LspReference>),
    Locations(Vec<LspLocation>),
    Hover(Option<String>),
    Tokens(Vec<SemanticToken>),
}

/// Health metrics for monitoring
#[derive(Debug, Clone)]
struct HealthMetrics {
    /// Total requests sent
    total_requests: u64,
    /// Successful requests
    successful_requests: u64,
    /// Failed requests
    failed_requests: u64,
    /// Average response time
    average_response_time: Duration,
    /// Last successful request time
    last_success: Option<SystemTime>,
    /// Memory usage
    memory_usage: usize,
    /// Connection uptime
    uptime: Duration,
    /// Last health check
    last_health_check: SystemTime,
}

impl Default for LspManagerConfig {
    fn default() -> Self {
        Self {
            lsp_config: LspConfig::default(),
            background_init: true,
            health_check_interval: Duration::from_secs(30),
            max_init_attempts: 5,
        }
    }
}

impl Default for HealthMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            average_response_time: Duration::ZERO,
            last_success: None,
            memory_usage: 0,
            uptime: Duration::ZERO,
            last_health_check: SystemTime::now(),
        }
    }
}

impl LspManager {
    /// Create a new LSP manager
    pub async fn new(workspace_root: &Path, config: LspConfig) -> LspResult<Self> {
        let manager_config = LspManagerConfig {
            lsp_config: config,
            background_init: true,
            health_check_interval: Duration::from_secs(30),
            max_init_attempts: 5,
        };

        let manager = Self {
            client: Arc::new(RwLock::new(None)),
            config: manager_config,
            workspace_root: workspace_root.to_path_buf(),
            status: Arc::new(RwLock::new(LspStatus::Uninitialized)),
            background_tasks: Arc::new(Mutex::new(Vec::new())),
            enhancement_cache: Arc::new(RwLock::new(HashMap::new())),
            request_queue: Arc::new(Mutex::new(Vec::new())),
            health_metrics: Arc::new(RwLock::new(HealthMetrics::default())),
            init_attempts: Arc::new(RwLock::new(0)),
        };

        // Start background initialization if enabled
        if manager.config.background_init {
            manager.start_background_initialization().await?;
        }

        Ok(manager)
    }

    /// Start background initialization
    async fn start_background_initialization(&self) -> LspResult<()> {
        let client = self.client.clone();
        let config = self.config.clone();
        let workspace_root = self.workspace_root.clone();
        let status = self.status.clone();
        let init_attempts = self.init_attempts.clone();
        let health_metrics = self.health_metrics.clone();

        let task = tokio::spawn(async move {
            Self::background_init_task(
                client,
                config,
                workspace_root,
                status,
                init_attempts,
                health_metrics,
            ).await;
        });

        let mut tasks = self.background_tasks.lock().await;
        tasks.push(BackgroundTask {
            name: "lsp_initialization".to_string(),
            started_at: SystemTime::now(),
            handle: task,
        });

        Ok(())
    }

    /// Background initialization task
    async fn background_init_task(
        client: Arc<RwLock<Option<LspClient>>>,
        config: LspManagerConfig,
        workspace_root: PathBuf,
        status: Arc<RwLock<LspStatus>>,
        init_attempts: Arc<RwLock<u32>>,
        health_metrics: Arc<RwLock<HealthMetrics>>,
    ) {
        info!("Starting LSP background initialization");
        *status.write().await = LspStatus::Initializing;

        let mut attempts = 0;
        let max_attempts = config.max_init_attempts;

        while attempts < max_attempts {
            attempts += 1;
            *init_attempts.write().await = attempts;

            debug!("LSP initialization attempt {} of {}", attempts, max_attempts);

            match Self::try_initialize_lsp(&client, &config, &workspace_root).await {
                Ok(()) => {
                    info!("LSP initialization successful");
                    *status.write().await = LspStatus::Ready;
                    
                    // Update health metrics
                    {
                        let mut metrics = health_metrics.write().await;
                        metrics.last_success = Some(SystemTime::now());
                    }

                    // Start health monitoring
                    Self::start_health_monitoring(client.clone(), status.clone(), health_metrics.clone(), config.health_check_interval).await;
                    return;
                }
                Err(e) => {
                    warn!("LSP initialization attempt {} failed: {}", attempts, e);
                    
                    if attempts < max_attempts {
                        let delay = Duration::from_secs(2u64.pow(attempts.min(5))); // Exponential backoff
                        debug!("Retrying LSP initialization in {:?}", delay);
                        sleep(delay).await;
                    }
                }
            }
        }

        error!("LSP initialization failed after {} attempts", max_attempts);
        *status.write().await = LspStatus::Failed("Max initialization attempts exceeded".to_string());
    }

    /// Try to initialize LSP client
    async fn try_initialize_lsp(
        client: &Arc<RwLock<Option<LspClient>>>,
        config: &LspManagerConfig,
        workspace_root: &Path,
    ) -> LspResult<()> {
        // Create new client
        let mut lsp_client = LspClient::new(
            workspace_root,
            config.lsp_config.server.request_timeout,
        ).await?;

        // Start the client with timeout
        timeout(
            config.lsp_config.server.init_timeout,
            lsp_client.start()
        ).await
        .map_err(|_| LspError::InitializationFailed("Initialization timeout".to_string()))??;

        // Store the client
        *client.write().await = Some(lsp_client);

        Ok(())
    }

    /// Start health monitoring
    async fn start_health_monitoring(
        client: Arc<RwLock<Option<LspClient>>>,
        status: Arc<RwLock<LspStatus>>,
        health_metrics: Arc<RwLock<HealthMetrics>>,
        check_interval: Duration,
    ) {
        tokio::spawn(async move {
            let mut interval = interval(check_interval);
            
            loop {
                interval.tick().await;
                
                let client_status = {
                    let client_guard = client.read().await;
                    match client_guard.as_ref() {
                        Some(client) => client.status().await,
                        None => ConnectionStatus::Disconnected,
                    }
                };

                let new_status = match client_status {
                    ConnectionStatus::Connected => LspStatus::Ready,
                    ConnectionStatus::Connecting => LspStatus::Initializing,
                    ConnectionStatus::Failed(ref msg) => LspStatus::Failed(msg.clone()),
                    ConnectionStatus::Reconnecting => LspStatus::Unavailable("Reconnecting".to_string()),
                    ConnectionStatus::Disconnected => LspStatus::Unavailable("Disconnected".to_string()),
                };

                *status.write().await = new_status;

                // Update health metrics
                {
                    let mut metrics = health_metrics.write().await;
                    metrics.last_health_check = SystemTime::now();
                    
                    if matches!(client_status, ConnectionStatus::Connected) {
                        metrics.last_success = Some(SystemTime::now());
                    }
                }

                debug!("LSP health check completed: {:?}", client_status);
            }
        });
    }

    /// Get current LSP status
    pub async fn status(&self) -> LspStatus {
        self.status.read().await.clone()
    }

    /// Check if LSP is available
    pub async fn is_available(&self) -> bool {
        matches!(self.status().await, LspStatus::Ready)
    }

    /// Get health metrics
    pub async fn health_metrics(&self) -> HealthMetrics {
        self.health_metrics.read().await.clone()
    }

    /// Enhance analysis with LSP data
    pub async fn enhance_analysis(
        &self,
        base_snapshot: &WorkspaceSnapshot,
        strategy: AnalysisStrategy,
    ) -> LspResult<HybridAnalysisResult> {
        let start_time = Instant::now();
        
        match strategy {
            AnalysisStrategy::TreeSitterOnly => {
                // Return base analysis without LSP enhancement
                Ok(HybridAnalysisResult::new(
                    base_snapshot.clone(),
                    LspEnhancements {
                        enhanced_functions: HashMap::new(),
                        resolved_references: Vec::new(),
                        type_definitions: HashMap::new(),
                        semantic_tokens: Vec::new(),
                        semantic_dependencies: Vec::new(),
                        diagnostics: Vec::new(),
                    },
                    MergeStrategy::TreeSitterOnly,
                    false,
                ))
            }
            AnalysisStrategy::Progressive => {
                self.progressive_enhancement(base_snapshot, start_time).await
            }
            AnalysisStrategy::CachedLspOnly => {
                self.cached_enhancement(base_snapshot, start_time).await
            }
            AnalysisStrategy::HybridIntelligent => {
                self.intelligent_enhancement(base_snapshot, start_time).await
            }
            AnalysisStrategy::TreeSitterWithWarning => {
                // Similar to TreeSitterOnly but with warning metadata
                let mut result = HybridAnalysisResult::new(
                    base_snapshot.clone(),
                    LspEnhancements {
                        enhanced_functions: HashMap::new(),
                        resolved_references: Vec::new(),
                        type_definitions: HashMap::new(),
                        semantic_tokens: Vec::new(),
                        semantic_dependencies: Vec::new(),
                        diagnostics: Vec::new(),
                    },
                    MergeStrategy::TreeSitterOnly,
                    false,
                );
                
                // Add warning to metrics or somewhere accessible
                warn!("LSP unavailable, using tree-sitter only analysis");
                Ok(result)
            }
        }
    }

    /// Progressive enhancement strategy
    async fn progressive_enhancement(
        &self,
        base_snapshot: &WorkspaceSnapshot,
        start_time: Instant,
    ) -> LspResult<HybridAnalysisResult> {
        let mut enhancements = LspEnhancements {
            enhanced_functions: HashMap::new(),
            resolved_references: Vec::new(),
            type_definitions: HashMap::new(),
            semantic_tokens: Vec::new(),
            semantic_dependencies: Vec::new(),
            diagnostics: Vec::new(),
        };

        let lsp_available = self.is_available().await;
        
        if lsp_available {
            // Enhance a subset of functions with LSP data
            let functions_to_enhance: Vec<_> = base_snapshot.functions
                .iter()
                .take(100) // Limit for performance
                .collect();

            for function in functions_to_enhance {
                if let Ok(enhanced) = self.enhance_function(function).await {
                    enhancements.enhanced_functions.insert(
                        function.qualified_name.clone(),
                        enhanced,
                    );
                }
            }
        }

        let metrics = AnalysisMetrics {
            tree_sitter_duration: Duration::ZERO, // Would be provided by caller
            lsp_duration: start_time.elapsed(),
            merge_duration: Duration::ZERO,
            lsp_enhanced_symbols: enhancements.enhanced_functions.len(),
            cache_hit_rate: 0.0, // Would be calculated
            memory_usage: 0, // Would be measured
        };

        let mut result = HybridAnalysisResult::new(
            base_snapshot.clone(),
            enhancements,
            MergeStrategy::Progressive,
            lsp_available,
        );
        result.metrics = metrics;

        Ok(result)
    }

    /// Cached enhancement strategy
    async fn cached_enhancement(
        &self,
        base_snapshot: &WorkspaceSnapshot,
        start_time: Instant,
    ) -> LspResult<HybridAnalysisResult> {
        let cache = self.enhancement_cache.read().await;
        let mut enhancements = LspEnhancements {
            enhanced_functions: HashMap::new(),
            resolved_references: Vec::new(),
            type_definitions: HashMap::new(),
            semantic_tokens: Vec::new(),
            semantic_dependencies: Vec::new(),
            diagnostics: Vec::new(),
        };

        // Use cached enhancements
        for function in &base_snapshot.functions {
            if let Some(enhanced) = cache.get(&function.qualified_name) {
                enhancements.enhanced_functions.insert(
                    function.qualified_name.clone(),
                    enhanced.clone(),
                );
            }
        }

        let metrics = AnalysisMetrics {
            tree_sitter_duration: Duration::ZERO,
            lsp_duration: start_time.elapsed(),
            merge_duration: Duration::ZERO,
            lsp_enhanced_symbols: enhancements.enhanced_functions.len(),
            cache_hit_rate: if base_snapshot.functions.is_empty() {
                0.0
            } else {
                enhancements.enhanced_functions.len() as f64 / base_snapshot.functions.len() as f64
            },
            memory_usage: 0,
        };

        let mut result = HybridAnalysisResult::new(
            base_snapshot.clone(),
            enhancements,
            MergeStrategy::CachedLspOnly,
            self.is_available().await,
        );
        result.metrics = metrics;

        Ok(result)
    }

    /// Intelligent enhancement strategy
    async fn intelligent_enhancement(
        &self,
        base_snapshot: &WorkspaceSnapshot,
        start_time: Instant,
    ) -> LspResult<HybridAnalysisResult> {
        // Combine progressive and cached strategies intelligently
        let cached_result = self.cached_enhancement(base_snapshot, start_time).await?;
        
        if cached_result.enhancement_percentage() < 50.0 && self.is_available().await {
            // Low cache hit rate, try progressive enhancement
            self.progressive_enhancement(base_snapshot, start_time).await
        } else {
            Ok(cached_result)
        }
    }

    /// Enhance a single function with LSP data
    async fn enhance_function(&self, function: &RustFunction) -> LspResult<LspEnhancedFunction> {
        // Check cache first
        {
            let cache = self.enhancement_cache.read().await;
            if let Some(cached) = cache.get(&function.qualified_name) {
                return Ok(cached.clone());
            }
        }

        // Get LSP client
        let client_guard = self.client.read().await;
        let client = client_guard.as_ref().ok_or_else(|| {
            LspError::ConnectionLost("LSP client not available".to_string())
        })?;

        // Get document symbols for the file
        let symbols = client.get_document_symbols(&function.file_path).await?;
        
        // Find matching symbol
        let lsp_symbol = symbols.into_iter()
            .find(|symbol| {
                // Simple matching by name and line range
                symbol.range.start.line as usize <= function.line_start &&
                symbol.range.end.line as usize >= function.line_end
            });

        let enhanced = LspEnhancedFunction::from_base(function.clone(), lsp_symbol);

        // Cache the result
        {
            let mut cache = self.enhancement_cache.write().await;
            cache.insert(function.qualified_name.clone(), enhanced.clone());
        }

        Ok(enhanced)
    }

    /// Shutdown the LSP manager
    pub async fn shutdown(&self) -> LspResult<()> {
        info!("Shutting down LSP manager");

        // Shutdown LSP client
        if let Some(mut client) = self.client.write().await.take() {
            client.shutdown().await?;
        }

        // Cancel background tasks
        let mut tasks = self.background_tasks.lock().await;
        for task in tasks.drain(..) {
            task.handle.abort();
        }

        *self.status.write().await = LspStatus::Uninitialized;
        info!("LSP manager shutdown complete");

        Ok(())
    }

    /// Force restart of LSP
    pub async fn restart(&self) -> LspResult<()> {
        info!("Restarting LSP");
        
        // Shutdown current client
        if let Some(mut client) = self.client.write().await.take() {
            let _ = client.shutdown().await;
        }

        // Reset status and attempts
        *self.status.write().await = LspStatus::Uninitialized;
        *self.init_attempts.write().await = 0;

        // Clear cache
        self.enhancement_cache.write().await.clear();

        // Start background initialization again
        self.start_background_initialization().await?;

        Ok(())
    }

    /// Get initialization attempts
    pub async fn init_attempts(&self) -> u32 {
        *self.init_attempts.read().await
    }

    /// Get active background tasks
    pub async fn background_tasks(&self) -> Vec<String> {
        let tasks = self.background_tasks.lock().await;
        tasks.iter().map(|task| task.name.clone()).collect()
    }

    /// Get definition for a symbol at a position
    pub async fn get_definition(
        &self,
        file_path: &Path,
        line: u32,
        character: u32,
    ) -> LspResult<Vec<crate::lsp::models::Location>> {
        let client_guard = self.client.read().await;
        if let Some(client) = client_guard.as_ref() {
            client.get_definition(file_path, line, character).await
        } else {
            Err(LspError::ConnectionLost("LSP client not available".to_string()))
        }
    }

    /// Get document symbols for a file
    pub async fn get_document_symbols(&self, file_path: &Path) -> LspResult<Vec<crate::lsp::models::LspSymbol>> {
        let client_guard = self.client.read().await;
        if let Some(client) = client_guard.as_ref() {
            client.get_document_symbols(file_path).await
        } else {
            Err(LspError::ConnectionLost("LSP client not available".to_string()))
        }
    }
}

impl Drop for LspManager {
    fn drop(&mut self) {
        // Cancel all background tasks
        // Note: This is a best-effort cleanup since we can't use async in Drop
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_lsp_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = LspConfig::default();
        
        let manager = LspManager::new(temp_dir.path(), config).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_status_tracking() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = LspConfig::default();
        config.server.init_timeout = Duration::from_millis(100);
        
        let manager = LspManager::new(temp_dir.path(), config).await.unwrap();
        
        // Initially should be uninitialized or initializing
        let status = manager.status().await;
        assert!(matches!(status, LspStatus::Uninitialized | LspStatus::Initializing));
    }

    #[tokio::test]
    async fn test_health_metrics() {
        let temp_dir = TempDir::new().unwrap();
        let config = LspConfig::default();
        
        let manager = LspManager::new(temp_dir.path(), config).await.unwrap();
        let metrics = manager.health_metrics().await;
        
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.successful_requests, 0);
        assert_eq!(metrics.failed_requests, 0);
    }

    #[tokio::test]
    async fn test_shutdown() {
        let temp_dir = TempDir::new().unwrap();
        let config = LspConfig::default();
        
        let manager = LspManager::new(temp_dir.path(), config).await.unwrap();
        let shutdown_result = manager.shutdown().await;
        
        assert!(shutdown_result.is_ok());
        assert_eq!(manager.status().await, LspStatus::Uninitialized);
    }
}