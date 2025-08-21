//! Fallback strategies for MCP server when LSP is unavailable
//! 
//! This module provides intelligent fallback mechanisms that ensure
//! the MCP server continues to function gracefully when LSP (rust-analyzer)
//! is unavailable, providing degraded but still useful analysis.

use std::time::{Duration, Instant};
use anyhow::Result;
use serde_json::json;
use tracing::{debug, warn};

use super::{McpResponse, McpError};
use crate::lsp::LspStatus;

/// Fallback manager for handling LSP unavailability
pub struct FallbackManager {
    /// Current fallback strategy
    strategy: FallbackStrategy,
    /// Number of consecutive failures
    consecutive_failures: u32,
    /// Last successful LSP operation time
    last_success: Option<Instant>,
    /// Fallback timeout configuration
    timeout_config: TimeoutConfig,
    /// Performance degradation tracking
    degradation_tracker: DegradationTracker,
}

/// Available fallback strategies
#[derive(Debug, Clone, PartialEq)]
pub enum FallbackStrategy {
    /// Use tree-sitter only, no warnings
    TreeSitterOnly,
    /// Use tree-sitter with warnings about reduced accuracy
    TreeSitterWithWarnings,
    /// Use cached LSP data when available, tree-sitter otherwise
    CachedLspWithTreeSitterFallback,
    /// Intelligent strategy that adapts based on success rate
    Adaptive,
    /// Progressive degradation with retry logic
    ProgressiveDegradation,
}

/// Timeout configuration for different operations
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// Fast response timeout (for immediate responses)
    pub fast_timeout: Duration,
    /// Normal response timeout
    pub normal_timeout: Duration,
    /// Extended timeout for complex operations
    pub extended_timeout: Duration,
    /// Maximum wait time for LSP initialization
    pub max_init_wait: Duration,
}

/// Tracks performance degradation
#[derive(Debug, Clone)]
pub struct DegradationTracker {
    /// Success rate over recent operations
    pub success_rate: f64,
    /// Average response time without LSP
    pub fallback_response_time: Duration,
    /// Number of operations since last LSP success
    pub operations_since_lsp: u32,
    /// Quality score of fallback responses
    pub quality_score: f64,
}

/// Fallback context for decision making
#[derive(Debug)]
pub struct FallbackContext {
    /// Current LSP status
    pub lsp_status: LspStatus,
    /// Time since request started
    pub elapsed_time: Duration,
    /// Whether this is a critical operation
    pub is_critical: bool,
    /// User tolerance for waiting
    pub user_patience: UserPatience,
    /// Available cached data
    pub has_cached_data: bool,
}

/// User patience level for operations
#[derive(Debug, Clone, PartialEq)]
pub enum UserPatience {
    /// User wants immediate response
    Immediate,
    /// User can wait a short time for better results
    Short,
    /// User is willing to wait for best results
    Patient,
}

/// Result of fallback operation
#[derive(Debug)]
pub struct FallbackResult {
    /// The response to return
    pub response: McpResponse,
    /// Whether this used LSP data
    pub used_lsp: bool,
    /// Quality score of the response
    pub quality_score: f64,
    /// Strategy that was used
    pub strategy_used: FallbackStrategy,
    /// Warning messages for the user
    pub warnings: Vec<String>,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            fast_timeout: Duration::from_millis(500),
            normal_timeout: Duration::from_secs(2),
            extended_timeout: Duration::from_secs(5),
            max_init_wait: Duration::from_secs(30),
        }
    }
}

impl Default for DegradationTracker {
    fn default() -> Self {
        Self {
            success_rate: 1.0,
            fallback_response_time: Duration::from_millis(100),
            operations_since_lsp: 0,
            quality_score: 0.8, // Reasonable default for tree-sitter only
        }
    }
}

impl FallbackManager {
    /// Create a new fallback manager
    pub fn new(strategy: FallbackStrategy) -> Self {
        Self {
            strategy,
            consecutive_failures: 0,
            last_success: None,
            timeout_config: TimeoutConfig::default(),
            degradation_tracker: DegradationTracker::default(),
        }
    }

    /// Handle a request with fallback logic
    pub async fn handle_with_fallback<F, Fut>(
        &mut self,
        context: FallbackContext,
        lsp_operation: F,
        tree_sitter_operation: impl Fn() -> Result<McpResponse>,
    ) -> FallbackResult
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<McpResponse>>,
    {
        let start_time = Instant::now();
        
        // Determine the best strategy for this context
        let active_strategy = self.determine_strategy(&context);
        
        match active_strategy {
            FallbackStrategy::TreeSitterOnly => {
                self.handle_tree_sitter_only(tree_sitter_operation, start_time)
            }
            FallbackStrategy::TreeSitterWithWarnings => {
                self.handle_tree_sitter_with_warnings(tree_sitter_operation, start_time)
            }
            FallbackStrategy::CachedLspWithTreeSitterFallback => {
                self.handle_cached_lsp_fallback(context, lsp_operation, tree_sitter_operation, start_time).await
            }
            FallbackStrategy::Adaptive => {
                self.handle_adaptive(context, lsp_operation, tree_sitter_operation, start_time).await
            }
            FallbackStrategy::ProgressiveDegradation => {
                self.handle_progressive_degradation(context, lsp_operation, tree_sitter_operation, start_time).await
            }
        }
    }

    /// Handle tree-sitter only strategy
    fn handle_tree_sitter_only<F>(
        &mut self,
        tree_sitter_operation: F,
        start_time: Instant,
    ) -> FallbackResult
    where
        F: Fn() -> Result<McpResponse>,
    {
        match tree_sitter_operation() {
            Ok(response) => {
                self.update_success_metrics(start_time, false);
                FallbackResult {
                    response,
                    used_lsp: false,
                    quality_score: 0.7, // Good but not excellent for tree-sitter
                    strategy_used: FallbackStrategy::TreeSitterOnly,
                    warnings: vec![],
                }
            }
            Err(e) => {
                self.update_failure_metrics();
                FallbackResult {
                    response: self.create_error_response(e),
                    used_lsp: false,
                    quality_score: 0.0,
                    strategy_used: FallbackStrategy::TreeSitterOnly,
                    warnings: vec!["Tree-sitter analysis failed".to_string()],
                }
            }
        }
    }

    /// Handle tree-sitter with warnings strategy
    fn handle_tree_sitter_with_warnings<F>(
        &mut self,
        tree_sitter_operation: F,
        start_time: Instant,
    ) -> FallbackResult
    where
        F: Fn() -> Result<McpResponse>,
    {
        match tree_sitter_operation() {
            Ok(mut response) => {
                // Add warning to the response
                if let Some(result) = response.result.as_mut() {
                    if let Some(content) = result.get_mut("content") {
                        if let Some(content_array) = content.as_array_mut() {
                            if let Some(first_content) = content_array.first_mut() {
                                if let Some(text) = first_content.get_mut("text") {
                                    if let Some(text_str) = text.as_str() {
                                        let enhanced_text = format!(
                                            "⚠️ **Analysis Notice**: LSP (rust-analyzer) unavailable. Using tree-sitter analysis only - results may be less accurate for cross-crate references.\n\n{}",
                                            text_str
                                        );
                                        *text = json!(enhanced_text);
                                    }
                                }
                            }
                        }
                    }
                }

                self.update_success_metrics(start_time, false);
                FallbackResult {
                    response,
                    used_lsp: false,
                    quality_score: 0.6, // Slightly lower due to warnings
                    strategy_used: FallbackStrategy::TreeSitterWithWarnings,
                    warnings: vec!["LSP unavailable, using tree-sitter analysis only".to_string()],
                }
            }
            Err(e) => {
                self.update_failure_metrics();
                FallbackResult {
                    response: self.create_error_response(e),
                    used_lsp: false,
                    quality_score: 0.0,
                    strategy_used: FallbackStrategy::TreeSitterWithWarnings,
                    warnings: vec!["Both LSP and tree-sitter analysis failed".to_string()],
                }
            }
        }
    }

    /// Handle cached LSP with tree-sitter fallback
    async fn handle_cached_lsp_fallback<F, Fut>(
        &mut self,
        context: FallbackContext,
        lsp_operation: F,
        tree_sitter_operation: impl Fn() -> Result<McpResponse>,
        start_time: Instant,
    ) -> FallbackResult
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<McpResponse>>,
    {
        // Try LSP with short timeout if cached data might be available
        if context.has_cached_data {
            match tokio::time::timeout(self.timeout_config.fast_timeout, lsp_operation()).await {
                Ok(Ok(response)) => {
                    self.update_success_metrics(start_time, true);
                    return FallbackResult {
                        response,
                        used_lsp: true,
                        quality_score: 0.95,
                        strategy_used: FallbackStrategy::CachedLspWithTreeSitterFallback,
                        warnings: vec![],
                    };
                }
                Ok(Err(e)) => {
                    debug!("LSP operation failed: {}", e);
                }
                Err(_) => {
                    debug!("LSP operation timed out");
                }
            }
        }

        // Fallback to tree-sitter
        self.handle_tree_sitter_with_warnings(tree_sitter_operation, start_time)
    }

    /// Handle adaptive strategy
    async fn handle_adaptive<F, Fut>(
        &mut self,
        context: FallbackContext,
        lsp_operation: F,
        tree_sitter_operation: impl Fn() -> Result<McpResponse>,
        start_time: Instant,
    ) -> FallbackResult
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<McpResponse>>,
    {
        // Adapt strategy based on recent success rate
        if self.degradation_tracker.success_rate > 0.8 {
            // High success rate, try LSP with normal timeout
            let timeout = if context.is_critical {
                self.timeout_config.extended_timeout
            } else {
                self.timeout_config.normal_timeout
            };

            match tokio::time::timeout(timeout, lsp_operation()).await {
                Ok(Ok(response)) => {
                    self.update_success_metrics(start_time, true);
                    return FallbackResult {
                        response,
                        used_lsp: true,
                        quality_score: 0.95,
                        strategy_used: FallbackStrategy::Adaptive,
                        warnings: vec![],
                    };
                }
                Ok(Err(e)) => {
                    warn!("LSP operation failed in adaptive mode: {}", e);
                }
                Err(_) => {
                    warn!("LSP operation timed out in adaptive mode");
                }
            }
        }

        // Low success rate or LSP failed, use tree-sitter
        self.handle_tree_sitter_with_warnings(tree_sitter_operation, start_time)
    }

    /// Handle progressive degradation strategy
    async fn handle_progressive_degradation<F, Fut>(
        &mut self,
        context: FallbackContext,
        lsp_operation: F,
        tree_sitter_operation: impl Fn() -> Result<McpResponse>,
        start_time: Instant,
    ) -> FallbackResult
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<McpResponse>>,
    {
        // Progressive timeout based on user patience and operation criticality
        let timeout = match (context.user_patience, context.is_critical) {
            (UserPatience::Immediate, _) => self.timeout_config.fast_timeout,
            (UserPatience::Short, false) => self.timeout_config.normal_timeout,
            (UserPatience::Short, true) => self.timeout_config.extended_timeout,
            (UserPatience::Patient, _) => self.timeout_config.extended_timeout,
        };

        match tokio::time::timeout(timeout, lsp_operation()).await {
            Ok(Ok(response)) => {
                self.update_success_metrics(start_time, true);
                FallbackResult {
                    response,
                    used_lsp: true,
                    quality_score: 0.95,
                    strategy_used: FallbackStrategy::ProgressiveDegradation,
                    warnings: vec![],
                }
            }
            Ok(Err(e)) => {
                debug!("LSP operation failed: {}", e);
                self.handle_tree_sitter_with_warnings(tree_sitter_operation, start_time)
            }
            Err(_) => {
                debug!("LSP operation timed out after {:?}", timeout);
                self.handle_tree_sitter_with_warnings(tree_sitter_operation, start_time)
            }
        }
    }

    /// Determine the best strategy for current context
    fn determine_strategy(&self, context: &FallbackContext) -> FallbackStrategy {
        match &self.strategy {
            FallbackStrategy::Adaptive => {
                // Choose strategy based on LSP status and history
                match context.lsp_status {
                    LspStatus::Ready => {
                        if self.degradation_tracker.success_rate > 0.9 {
                            FallbackStrategy::ProgressiveDegradation
                        } else {
                            FallbackStrategy::CachedLspWithTreeSitterFallback
                        }
                    }
                    LspStatus::Initializing => {
                        if context.user_patience == UserPatience::Patient {
                            FallbackStrategy::ProgressiveDegradation
                        } else {
                            FallbackStrategy::TreeSitterWithWarnings
                        }
                    }
                    LspStatus::Unavailable(_) | LspStatus::Failed(_) => {
                        FallbackStrategy::TreeSitterWithWarnings
                    }
                    LspStatus::Uninitialized => {
                        FallbackStrategy::TreeSitterOnly
                    }
                }
            }
            other => other.clone(),
        }
    }

    /// Update metrics on successful operation
    fn update_success_metrics(&mut self, start_time: Instant, used_lsp: bool) {
        let response_time = start_time.elapsed();
        
        if used_lsp {
            self.consecutive_failures = 0;
            self.last_success = Some(Instant::now());
            self.degradation_tracker.operations_since_lsp = 0;
            
            // Update success rate (weighted average)
            self.degradation_tracker.success_rate = 
                self.degradation_tracker.success_rate * 0.9 + 0.1;
        } else {
            self.degradation_tracker.operations_since_lsp += 1;
            
            // Update fallback response time
            self.degradation_tracker.fallback_response_time = Duration::from_millis(
                (self.degradation_tracker.fallback_response_time.as_millis() as u64 + 
                 response_time.as_millis() as u64) / 2
            );
        }
    }

    /// Update metrics on failed operation
    fn update_failure_metrics(&mut self) {
        self.consecutive_failures += 1;
        self.degradation_tracker.operations_since_lsp += 1;
        
        // Decrease success rate
        self.degradation_tracker.success_rate = 
            self.degradation_tracker.success_rate * 0.9;
        
        // Decrease quality score
        self.degradation_tracker.quality_score = 
            (self.degradation_tracker.quality_score * 0.95).max(0.3);
    }

    /// Create error response
    fn create_error_response(&self, error: anyhow::Error) -> McpResponse {
        McpResponse {
            id: None,
            result: None,
            error: Some(McpError {
                code: -32603,
                message: format!("Analysis failed: {}", error),
                data: Some(json!({
                    "fallback_used": true,
                    "consecutive_failures": self.consecutive_failures,
                    "strategy": format!("{:?}", self.strategy)
                })),
            }),
        }
    }

    /// Get current degradation status
    pub fn get_degradation_status(&self) -> DegradationStatus {
        if self.degradation_tracker.success_rate > 0.9 {
            DegradationStatus::Optimal
        } else if self.degradation_tracker.success_rate > 0.7 {
            DegradationStatus::Minor
        } else if self.degradation_tracker.success_rate > 0.5 {
            DegradationStatus::Moderate
        } else {
            DegradationStatus::Severe
        }
    }

    /// Set fallback strategy
    pub fn set_strategy(&mut self, strategy: FallbackStrategy) {
        self.strategy = strategy;
    }

    /// Get current strategy
    pub fn get_strategy(&self) -> &FallbackStrategy {
        &self.strategy
    }

    /// Reset metrics (useful after LSP recovery)
    pub fn reset_metrics(&mut self) {
        self.consecutive_failures = 0;
        self.degradation_tracker = DegradationTracker::default();
    }
}

/// Current degradation status
#[derive(Debug, Clone, PartialEq)]
pub enum DegradationStatus {
    /// Operating at full capacity
    Optimal,
    /// Minor degradation
    Minor,
    /// Moderate degradation
    Moderate,
    /// Severe degradation
    Severe,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_manager_creation() {
        let manager = FallbackManager::new(FallbackStrategy::Adaptive);
        assert_eq!(manager.strategy, FallbackStrategy::Adaptive);
        assert_eq!(manager.consecutive_failures, 0);
    }

    #[test]
    fn test_degradation_status() {
        let mut manager = FallbackManager::new(FallbackStrategy::TreeSitterOnly);
        
        // Initially optimal
        assert_eq!(manager.get_degradation_status(), DegradationStatus::Optimal);
        
        // Simulate failures
        for _ in 0..5 {
            manager.update_failure_metrics();
        }
        
        // Should be degraded now
        assert_ne!(manager.get_degradation_status(), DegradationStatus::Optimal);
    }

    #[test]
    fn test_strategy_determination() {
        let manager = FallbackManager::new(FallbackStrategy::Adaptive);
        
        let context = FallbackContext {
            lsp_status: LspStatus::Ready,
            elapsed_time: Duration::from_millis(100),
            is_critical: false,
            user_patience: UserPatience::Short,
            has_cached_data: true,
        };
        
        let strategy = manager.determine_strategy(&context);
        assert!(matches!(
            strategy,
            FallbackStrategy::ProgressiveDegradation | FallbackStrategy::CachedLspWithTreeSitterFallback
        ));
    }

    #[test]
    fn test_timeout_config_defaults() {
        let config = TimeoutConfig::default();
        assert!(config.fast_timeout < config.normal_timeout);
        assert!(config.normal_timeout < config.extended_timeout);
        assert!(config.extended_timeout < config.max_init_wait);
    }
}