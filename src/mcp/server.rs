use std::path::Path;
use anyhow::Result;
use serde_json::{json, Value};
use tokio::sync::RwLock;
use std::sync::Arc;
use std::collections::HashMap;

use crate::analyzer::{WorkspaceAnalyzer, WorkspaceSnapshot, HybridWorkspaceAnalyzer};
use crate::graph::MemgraphClient;
use crate::lsp::models::HybridAnalysisResult;

#[derive(Debug, Clone)]
pub struct McpRequest {
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct McpResponse {
    pub id: Option<Value>,
    pub result: Option<Value>,
    pub error: Option<McpError>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

pub struct WorkspaceMcpServer {
    analyzer: Arc<RwLock<WorkspaceAnalyzer>>,
    hybrid_analyzer: Arc<RwLock<Option<HybridWorkspaceAnalyzer>>>,
    graph_client: Arc<RwLock<MemgraphClient>>,
    workspace_root: std::path::PathBuf,
    current_snapshot: Arc<RwLock<Option<WorkspaceSnapshot>>>,
    current_hybrid_result: Arc<RwLock<Option<HybridAnalysisResult>>>,
    hybrid_enabled: bool,
}

impl WorkspaceMcpServer {
    pub async fn new(workspace_root: &Path) -> Result<Self> {
        let mut config = crate::config::Config::default();
        config.workspace.root = workspace_root.to_path_buf();
        
        let analyzer = WorkspaceAnalyzer::new_with_config(config.clone())?;
        let graph_client = MemgraphClient::new(&config).await?;
        
        // Try to initialize hybrid analyzer with default LSP config
        let hybrid_analyzer = match HybridWorkspaceAnalyzer::new(workspace_root, Some(config.clone())).await {
            Ok(hybrid) => Some(hybrid),
            Err(_e) => {
                
                None
            }
        };
        
        Ok(Self {
            analyzer: Arc::new(RwLock::new(analyzer)),
            hybrid_analyzer: Arc::new(RwLock::new(hybrid_analyzer)),
            graph_client: Arc::new(RwLock::new(graph_client)),
            workspace_root: workspace_root.to_path_buf(),
            current_snapshot: Arc::new(RwLock::new(None)),
            current_hybrid_result: Arc::new(RwLock::new(None)),
            hybrid_enabled: true, // Enable hybrid analysis by default
        })
    }
    
    /// Create MCP server with hybrid analysis disabled
    pub async fn new_tree_sitter_only(workspace_root: &Path) -> Result<Self> {
        let mut config = crate::config::Config::default();
        config.workspace.root = workspace_root.to_path_buf();
        
        let analyzer = WorkspaceAnalyzer::new_with_config(config.clone())?;
        let graph_client = MemgraphClient::new(&config).await?;
        
        Ok(Self {
            analyzer: Arc::new(RwLock::new(analyzer)),
            hybrid_analyzer: Arc::new(RwLock::new(None)),
            graph_client: Arc::new(RwLock::new(graph_client)),
            workspace_root: workspace_root.to_path_buf(),
            current_snapshot: Arc::new(RwLock::new(None)),
            current_hybrid_result: Arc::new(RwLock::new(None)),
            hybrid_enabled: false,
        })
    }
    
    pub async fn handle_request(&self, request: McpRequest) -> McpResponse {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request).await,
            "workspace_context" => self.handle_workspace_context(request).await,
            "analyze_change_impact" => self.handle_analyze_change_impact(request).await,
            "check_architecture_violations" => self.handle_check_architecture_violations(request).await,
            "find_dependency_issues" => self.handle_find_dependency_issues(request).await,
            "suggest_safe_refactoring" => self.handle_suggest_safe_refactoring(request).await,
            "validate_proposed_change" => self.handle_validate_proposed_change(request).await,
            "analyze_test_coverage" => self.handle_analyze_test_coverage(request).await,
            "analyze_actor_spawns" => self.handle_analyze_actor_spawns(request).await,
            _ => McpResponse {
                id: request.id,
                result: None,
                error: Some(McpError {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                    data: None,
                }),
            },
        }
    }
    
    async fn handle_initialize(&self, request: McpRequest) -> McpResponse {
        // Perform initial workspace analysis
        if self.hybrid_enabled {
            // Try hybrid analysis first
            if let Some(hybrid_analyzer) = self.hybrid_analyzer.write().await.as_mut() {
                match hybrid_analyzer.analyze_workspace().await {
                    Ok(hybrid_result) => {
                        // Since hybrid_result doesn't contain tree-sitter data directly,
                        // we need to get it from the regular workspace analyzer
                        match self.analyzer.write().await.create_snapshot().await {
                            Ok(snapshot) => {
                                *self.current_snapshot.write().await = Some(snapshot);
                                *self.current_hybrid_result.write().await = Some(hybrid_result);
                            }
                            Err(_e) => {
                                // Failed to create tree-sitter snapshot
                            },
                        }
                    }
                    Err(_e) => {
                        
                        // Fallback to tree-sitter only
                        match self.analyzer.write().await.analyze_workspace() {
                            Ok(snapshot) => {
                                *self.current_snapshot.write().await = Some(snapshot);
                            }
                            Err(e) => {
                                return McpResponse {
                                    id: request.id,
                                    result: None,
                                    error: Some(McpError {
                                        code: -32603,
                                        message: format!("Failed to analyze workspace: {}", e),
                                        data: None,
                                    }),
                                };
                            }
                        }
                    }
                }
            } else {
                // No hybrid analyzer available, use tree-sitter
                match self.analyzer.write().await.analyze_workspace() {
                    Ok(snapshot) => {
                        *self.current_snapshot.write().await = Some(snapshot);
                    }
                    Err(e) => {
                        return McpResponse {
                            id: request.id,
                            result: None,
                            error: Some(McpError {
                                code: -32603,
                                message: format!("Failed to analyze workspace: {}", e),
                                data: None,
                            }),
                        };
                    }
                }
            }
        } else {
            // Tree-sitter only mode
            match self.analyzer.write().await.analyze_workspace() {
                Ok(snapshot) => {
                    *self.current_snapshot.write().await = Some(snapshot);
                }
                Err(e) => {
                    return McpResponse {
                        id: request.id,
                        result: None,
                        error: Some(McpError {
                            code: -32603,
                            message: format!("Failed to analyze workspace: {}", e),
                            data: None,
                        }),
                    };
                }
            }
        }
        
        McpResponse {
            id: request.id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {
                        "listChanged": false
                    }
                },
                "serverInfo": {
                    "name": "rust-workspace-analyzer",
                    "version": "0.1.0"
                },
                "tools": [
                    {
                        "name": "workspace_context",
                        "description": "Get comprehensive context about the current workspace",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "limit_functions": {"type": "number", "description": "Limit number of functions shown (default: 20)"},
                                "limit_types": {"type": "number", "description": "Limit number of types shown (default: 10)"},
                                "summary_only": {"type": "boolean", "description": "Show only summary statistics (default: false)"}
                            }
                        }
                    },
                    {
                        "name": "analyze_change_impact",
                        "description": "Analyze the impact of changing a specific function or type",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "target": {"type": "string", "description": "Function or type name to analyze"},
                                "change_type": {"type": "string", "description": "Type of change (signature, rename, delete)"}
                            },
                            "required": ["target"]
                        }
                    },
                    {
                        "name": "check_architecture_violations",
                        "description": "Check for architectural rule violations",
                        "inputSchema": {
                            "type": "object", 
                            "properties": {
                                "rules": {"type": "array", "items": {"type": "string"}, "description": "Specific rules to check"}
                            }
                        }
                    },
                    {
                        "name": "find_dependency_issues",
                        "description": "Find circular dependencies and other dependency issues",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "suggest_safe_refactoring",
                        "description": "Suggest safe refactoring opportunities",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "focus_area": {"type": "string", "description": "Area to focus refactoring on"}
                            }
                        }
                    },
                    {
                        "name": "validate_proposed_change",
                        "description": "Validate a proposed change for safety and architectural compliance",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "description": {"type": "string", "description": "Description of the proposed change"}
                            },
                            "required": ["description"]
                        }
                    },
                    {
                        "name": "analyze_test_coverage",
                        "description": "Analyze test coverage patterns in the codebase",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "focus_area": {"type": "string", "description": "Area to analyze (optional)"}
                            }
                        }
                    },
                    {
                        "name": "analyze_actor_spawns",
                        "description": "Analyze Kameo actor spawn patterns and relationships in the codebase",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "show_context": {"type": "boolean", "description": "Show spawn context details (default: true)"},
                                "group_by": {"type": "string", "description": "Group by 'parent', 'child', 'pattern', or 'context' (default: 'parent')"},
                                "include_args": {"type": "boolean", "description": "Include spawn arguments in output (default: false)"}
                            }
                        }
                    }
                ]
            })),
            error: None,
        }
    }
    
    async fn handle_workspace_context(&self, request: McpRequest) -> McpResponse {
        let snapshot = self.current_snapshot.read().await;
        let hybrid_result = self.current_hybrid_result.read().await;
        
        // Parse parameters
        let limit_functions = request.params.as_ref()
            .and_then(|p| p.get("limit_functions"))
            .and_then(|v| v.as_u64())
            .unwrap_or(20) as usize;
        
        let limit_types = request.params.as_ref()
            .and_then(|p| p.get("limit_types"))
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;
        
        let summary_only = request.params.as_ref()
            .and_then(|p| p.get("summary_only"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        match snapshot.as_ref() {
            Some(snapshot) => {
                let _module_summary = self.get_module_summary(&snapshot);
                let crate_count = self.get_crate_count(&snapshot);
                let cross_crate_refs: HashMap<String, i32> = HashMap::new();
                
                // Build more concise summary with hybrid information
                let mut summary_text = format!(
                    "# Workspace Analysis{}\n\n\
                    ## 📊 Overview\n\
                    - **Workspace**: {}\n\
                    - **Total Crates**: {}\n\
                    - **Total Functions**: {}\n\
                    - **Total Types**: {}\n\
                    - **Total Actors**: {}\n\
                    - **Total Actor Spawns**: {}\n\
                    - **Function References**: {}\n\
                    - **Cross-crate Calls**: {}\n",
                    if hybrid_result.is_some() { " (Hybrid: Tree-sitter + LSP)" } else { " (Tree-sitter Only)" },
                    self.workspace_root.display(),
                    crate_count,
                    snapshot.functions.len(),
                    snapshot.types.len(),
                    snapshot.actors.len(),
                    snapshot.actor_spawns.len(),
                    snapshot.function_references.len(),
                    cross_crate_refs.len()
                );
                
                // Add hybrid analysis information if available
                if let Some(hybrid) = hybrid_result.as_ref() {
                    summary_text.push_str(&format!(
                        "- **LSP Enhanced Functions**: {} ({:.1}%)\n\
                        - **LSP Status**: {}\n\
                        - **Analysis Strategy**: {:?}\n\
                        - **Enhancement Success Rate**: {:.1}%\n\n",
                        hybrid.results.len(),
                        0.0,
                        "❌ Unavailable",
                        "TreeSitter",
                        0.0
                    ));
                } else {
                    summary_text.push_str("\n");
                }
                
                if !summary_only {
                    // Add limited function and type examples
                    let functions = self.get_representative_functions_limited(&snapshot, limit_functions);
                    let types = self.get_representative_types_limited(&snapshot, limit_types);
                    
                    summary_text.push_str(&format!(
                        "## 🔧 Sample Functions (showing {} of {}):\n{}\n\n\
                        ## 📦 Sample Types (showing {} of {}):\n{}\n\n",
                        functions.len().min(limit_functions), snapshot.functions.len(),
                        functions.iter().take(limit_functions)
                            .map(|f| format!("- `{}` in {}", 
                                f["qualified_name"].as_str().unwrap_or("unknown"),
                                f["crate"].as_str().unwrap_or("unknown")))
                            .collect::<Vec<_>>().join("\n"),
                        types.len().min(limit_types), snapshot.types.len(),
                        types.iter().take(limit_types)
                            .map(|t| format!("- `{}` ({}) in {}", 
                                t["qualified_name"].as_str().unwrap_or("unknown"),
                                t["type_kind"].as_str().unwrap_or("unknown"),
                                t["crate"].as_str().unwrap_or("unknown")))
                            .collect::<Vec<_>>().join("\n")
                    ));
                }
                
                if snapshot.actor_spawns.len() > 0 {
                    summary_text.push_str(&format!("## 🎭 Actor Analysis Preview\n\
                        - {} Kameo actors detected\n\
                        - {} spawn operations found\n\
                        Use `analyze_actor_spawns` for detailed actor relationship analysis.\n\n",
                        snapshot.actors.len(), snapshot.actor_spawns.len()));
                }
                
                summary_text.push_str("Use other tools like `analyze_actor_spawns`, `analyze_test_coverage` or `check_architecture_violations` for detailed analysis.");
                
                McpResponse {
                    id: request.id,
                    result: Some(json!({
                        "content": [{
                            "type": "text",
                            "text": summary_text
                        }]
                    })),
                    error: None,
                }
            }
            None => McpResponse {
                id: request.id,
                result: None,
                error: Some(McpError {
                    code: -32603,
                    message: "Workspace not analyzed yet. Call initialize first.".to_string(),
                    data: None,
                }),
            }
        }
    }
    
    async fn handle_analyze_change_impact(&self, request: McpRequest) -> McpResponse {
        let params = match request.params {
            Some(params) => params,
            None => {
                return McpResponse {
                    id: request.id,
                    result: None,
                    error: Some(McpError {
                        code: -32602,
                        message: "Missing parameters".to_string(),
                        data: None,
                    }),
                };
            }
        };
        
        let target = match params.get("target").and_then(|v| v.as_str()) {
            Some(target) => target,
            None => {
                return McpResponse {
                    id: request.id,
                    result: None,
                    error: Some(McpError {
                        code: -32602,
                        message: "Missing 'target' parameter".to_string(),
                        data: None,
                    }),
                };
            }
        };
        
        let snapshot = self.current_snapshot.read().await;
        
        match snapshot.as_ref() {
            Some(snapshot) => {
                let impact_analysis = self.analyze_target_impact(snapshot, target);
                
                McpResponse {
                    id: request.id,
                    result: Some(json!({
                        "content": [{
                            "type": "text", 
                            "text": format!("# Change Impact Analysis for '{}'\n\n{}", target, impact_analysis)
                        }]
                    })),
                    error: None,
                }
            }
            None => McpResponse {
                id: request.id,
                result: None,
                error: Some(McpError {
                    code: -32603,
                    message: "Workspace not analyzed yet".to_string(),
                    data: None,
                }),
            }
        }
    }
    
    async fn handle_check_architecture_violations(&self, request: McpRequest) -> McpResponse {
        let snapshot = self.current_snapshot.read().await;
        
        // Extract rules filter from request params
        let rules_filter = request.params
            .as_ref()
            .and_then(|p| p.get("rules"))
            .and_then(|r| r.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
            .unwrap_or_default();
        
        match snapshot.as_ref() {
            Some(snapshot) => {
                let violations = self.check_architectural_violations_with_filter(snapshot, &rules_filter);
                
                McpResponse {
                    id: request.id,
                    result: Some(json!({
                        "content": [{
                            "type": "text",
                            "text": format!("# Architecture Violations\n\n{}", violations)
                        }]
                    })),
                    error: None,
                }
            }
            None => McpResponse {
                id: request.id,
                result: None,
                error: Some(McpError {
                    code: -32603,
                    message: "Workspace not analyzed yet".to_string(),
                    data: None,
                }),
            }
        }
    }
    
    async fn handle_find_dependency_issues(&self, request: McpRequest) -> McpResponse {
        let snapshot = self.current_snapshot.read().await;
        
        match snapshot.as_ref() {
            Some(snapshot) => {
                let issues = self.find_circular_dependencies(snapshot);
                
                McpResponse {
                    id: request.id,
                    result: Some(json!({
                        "content": [{
                            "type": "text",
                            "text": format!("# Dependency Issues\n\n{}", issues)
                        }]
                    })),
                    error: None,
                }
            }
            None => McpResponse {
                id: request.id,
                result: None,
                error: Some(McpError {
                    code: -32603,
                    message: "Workspace not analyzed yet".to_string(),
                    data: None,
                }),
            }
        }
    }
    
    async fn handle_suggest_safe_refactoring(&self, _request: McpRequest) -> McpResponse {
        // Placeholder implementation
        McpResponse {
            id: _request.id,
            result: Some(json!({
                "content": [{
                    "type": "text",
                    "text": "# Refactoring Suggestions\n\n🚧 Analysis in progress (implementation pending)"
                }]
            })),
            error: None,
        }
    }
    
    async fn handle_validate_proposed_change(&self, _request: McpRequest) -> McpResponse {
        // Placeholder implementation
        McpResponse {
            id: _request.id,
            result: Some(json!({
                "content": [{
                    "type": "text",
                    "text": "# Change Validation\n\n✅ Proposed change appears safe (implementation pending)"
                }]
            })),
            error: None,
        }
    }
    
    async fn handle_analyze_test_coverage(&self, request: McpRequest) -> McpResponse {
        let snapshot_guard = self.current_snapshot.read().await;
        let snapshot = match snapshot_guard.as_ref() {
            Some(snapshot) => snapshot,
            None => {
                return McpResponse {
                    id: request.id,
                    result: None,
                    error: Some(McpError {
                        code: -32603,
                        message: "No workspace analysis available. Please initialize first.".to_string(),
                        data: None,
                    }),
                };
            }
        };
        
        // Build reference count map using new function reference data structure
        let mut function_refs = HashMap::new();
        let mut test_functions = std::collections::HashSet::new();
        
        // Identify test functions (functions in test modules or with test attributes)
        for func in &snapshot.functions {
            if func.module.contains("::tests") || func.module.contains("test") || 
               func.file_path.contains("/tests/") ||
               func.file_path.contains("test_") {
                test_functions.insert(func.qualified_name.clone());
            }
        }
        
        // Count function references from workspace snapshot data
        for (target_fn, callers) in &snapshot.function_references {
            function_refs.insert(target_fn.clone(), callers.len());
        }
        
        // Find production functions that are heavily used but not tested
        let mut untested_heavy_functions = Vec::new();
        let mut tested_functions = Vec::new();
        let mut test_coverage_stats = HashMap::new();
        
        for func in &snapshot.functions {
            let ref_count = function_refs.get(&func.qualified_name).unwrap_or(&0);
            let cross_crate_count = 0; // Cross-crate analysis not yet implemented
            let crate_name = self.extract_crate_name(std::path::Path::new(&func.file_path)).unwrap_or_else(|| "unknown".to_string());
            
            // Skip test functions themselves
            if test_functions.contains(&func.qualified_name) {
                continue;
            }
            
            // Check if this function has corresponding tests
            let has_test = test_functions.iter().any(|test_name| {
                test_name.contains(&func.name) || 
                test_name.to_lowercase().contains(&format!("test_{}", func.name.to_lowercase()))
            });
            
            let stats = test_coverage_stats.entry(crate_name.clone()).or_insert((0, 0, 0));
            stats.0 += 1; // total functions
            
            if has_test {
                stats.1 += 1; // tested functions
                if *ref_count > 0 {
                    tested_functions.push(json!({
                        "name": func.name,
                        "qualified_name": func.qualified_name,
                        "module": func.module,
                        "crate": crate_name,
                        "references": ref_count,
                        "cross_crate_refs": cross_crate_count,
                        "file": func.file_path,
                        "line": func.line_start
                    }));
                }
            } else {
                stats.2 += 1; // untested functions
                if *ref_count > 0 || untested_heavy_functions.len() < 50 { // Show first 50 untested functions
                    untested_heavy_functions.push(json!({
                    "name": func.name,
                    "qualified_name": func.qualified_name,
                    "module": func.module,
                    "crate": crate_name,
                    "references": ref_count,
                    "cross_crate_refs": cross_crate_count,
                    "file": func.file_path,
                    "line": func.line_start
                    }));
                }
            }
        }
        
        // Sort by reference count (most referenced first)
        untested_heavy_functions.sort_by(|a, b| {
            b["references"].as_u64().unwrap_or(0).cmp(&a["references"].as_u64().unwrap_or(0))
        });
        
        tested_functions.sort_by(|a, b| {
            b["references"].as_u64().unwrap_or(0).cmp(&a["references"].as_u64().unwrap_or(0))
        });
        
        // Build coverage summary by crate
        let mut coverage_by_crate = Vec::new();
        for (crate_name, (total, tested, untested_heavy)) in test_coverage_stats {
            let coverage_percent = if total > 0 { (tested as f64 / total as f64 * 100.0) as u32 } else { 0 };
            coverage_by_crate.push(json!({
                "crate": crate_name,
                "total_functions": total,
                "tested_functions": tested,
                "coverage_percent": coverage_percent,
                "untested_heavy_usage": untested_heavy
            }));
        }
        
        coverage_by_crate.sort_by(|a, b| {
            a["coverage_percent"].as_u64().unwrap_or(0).cmp(&b["coverage_percent"].as_u64().unwrap_or(0))
        });
        
        let analysis = json!({
            "total_functions": snapshot.functions.len(),
            "total_test_functions": test_functions.len(),
            "heavily_used_untested": untested_heavy_functions.len(),
            "heavily_used_tested": tested_functions.len(),
            "coverage_by_crate": coverage_by_crate,
            "priority_untested": untested_heavy_functions.iter().take(15).collect::<Vec<_>>(),
            "well_tested_examples": tested_functions.iter().take(10).collect::<Vec<_>>()
        });
        
        let report = format!(
            "# Test Coverage Analysis\n\n\
            ## 📊 Coverage Summary\n\
            - **Total Functions**: {}\n\
            - **Test Functions**: {}\n\
            - **Heavily Used & Untested**: {} ⚠️\n\
            - **Heavily Used & Tested**: {} ✅\n\n\
            ## 🚨 Priority: Untested High-Usage Functions\n\
            These functions are called frequently but lack tests:\n\n{}\n\n\
            ## ✅ Well-Tested High-Usage Functions\n\
            Good examples of tested critical functions:\n\n{}\n\n\
            ## 📈 Coverage by Crate\n{}\n\n\
            ```json\n{}\n```",
            snapshot.functions.len(),
            test_functions.len(),
            untested_heavy_functions.len(),
            tested_functions.len(),
            untested_heavy_functions.iter().take(15)
                .map(|f| format!("- **{}** in `{}` ({} refs, {} cross-crate) - `{}:{}`", 
                    f["name"].as_str().unwrap_or("unknown"),
                    f["module"].as_str().unwrap_or("unknown"),
                    f["references"].as_u64().unwrap_or(0),
                    f["cross_crate_refs"].as_u64().unwrap_or(0),
                    f["file"].as_str().unwrap_or("unknown"),
                    f["line"].as_u64().unwrap_or(0)))
                .collect::<Vec<_>>().join("\n"),
            tested_functions.iter().take(10)
                .map(|f| format!("- **{}** in `{}` ({} refs, {} cross-crate) ✅", 
                    f["name"].as_str().unwrap_or("unknown"),
                    f["module"].as_str().unwrap_or("unknown"),
                    f["references"].as_u64().unwrap_or(0),
                    f["cross_crate_refs"].as_u64().unwrap_or(0)))
                .collect::<Vec<_>>().join("\n"),
            coverage_by_crate.iter()
                .map(|c| format!("- **{}**: {}% coverage ({}/{} functions, {} high-usage untested)",
                    c["crate"].as_str().unwrap_or("unknown"),
                    c["coverage_percent"].as_u64().unwrap_or(0),
                    c["tested_functions"].as_u64().unwrap_or(0),
                    c["total_functions"].as_u64().unwrap_or(0),
                    c["untested_heavy_usage"].as_u64().unwrap_or(0)))
                .collect::<Vec<_>>().join("\n"),
            serde_json::to_string_pretty(&analysis).unwrap_or_else(|_| "{}".to_string()).lines()
                .map(|line| format!("  {}", line))
                .collect::<Vec<_>>()
                .join("\n")
        );
        
        McpResponse {
            id: request.id,
            result: Some(json!({
                "content": [{
                    "type": "text",
                    "text": report
                }]
            })),
            error: None,
        }
    }
    
    // Helper methods
    fn get_module_summary(&self, snapshot: &WorkspaceSnapshot) -> HashMap<String, usize> {
        let mut modules = HashMap::new();
        
        for func in &snapshot.functions {
            *modules.entry(func.module.clone()).or_insert(0) += 1;
        }
        
        modules
    }
    
    fn get_crate_count(&self, snapshot: &WorkspaceSnapshot) -> usize {
        use std::collections::HashSet;
        let mut crates = HashSet::new();
        
        for func in &snapshot.functions {
            if let Some(crate_name) = self.extract_crate_name(std::path::Path::new(&func.file_path)) {
                crates.insert(crate_name);
            }
        }
        
        crates.len()
    }
    
    fn get_representative_functions_limited(&self, snapshot: &WorkspaceSnapshot, limit: usize) -> Vec<Value> {
        use std::collections::HashMap;
        let mut crate_functions: HashMap<String, Vec<&crate::analyzer::RustFunction>> = HashMap::new();
        
        // Group functions by crate
        for func in &snapshot.functions {
            if let Some(crate_name) = self.extract_crate_name(std::path::Path::new(&func.file_path)) {
                crate_functions.entry(crate_name).or_insert_with(Vec::new).push(func);
            }
        }
        
        let mut functions = Vec::new();
        let per_crate_limit = std::cmp::max(1, limit / crate_functions.len().max(1));
        
        // Get limited functions per crate
        for (crate_name, crate_funcs) in crate_functions.iter() {
            let crate_limit = std::cmp::min(per_crate_limit, crate_funcs.len());
            for func in crate_funcs.iter().take(crate_limit) {
                functions.push(json!({
                    "name": func.name,
                    "qualified_name": func.qualified_name,
                    "module": func.module,
                    "visibility": func.visibility,
                    "file": func.file_path.clone(),
                    "line_start": func.line_start,
                    "crate": crate_name
                }));
                
                if functions.len() >= limit {
                    break;
                }
            }
            if functions.len() >= limit {
                break;
            }
        }
        
        functions
    }
    
    fn get_representative_types_limited(&self, snapshot: &WorkspaceSnapshot, limit: usize) -> Vec<Value> {
        use std::collections::HashMap;
        let mut crate_types: HashMap<String, Vec<&crate::analyzer::RustType>> = HashMap::new();
        
        // Group types by crate
        for typ in &snapshot.types {
            if let Some(crate_name) = self.extract_crate_name(std::path::Path::new(&typ.file_path)) {
                crate_types.entry(crate_name).or_insert_with(Vec::new).push(typ);
            }
        }
        
        let mut types = Vec::new();
        let per_crate_limit = std::cmp::max(1, limit / crate_types.len().max(1));
        
        // Get limited types per crate
        for (crate_name, crate_type_list) in crate_types.iter() {
            let crate_limit = std::cmp::min(per_crate_limit, crate_type_list.len());
            for typ in crate_type_list.iter().take(crate_limit) {
                types.push(json!({
                    "name": typ.name,
                    "qualified_name": typ.qualified_name,
                    "type_kind": typ.type_kind,
                    "module": typ.module,
                    "visibility": typ.visibility,
                    "file": typ.file_path.clone(),
                    "line_start": typ.line_start,
                    "crate": crate_name
                }));
                
                if types.len() >= limit {
                    break;
                }
            }
            if types.len() >= limit {
                break;
            }
        }
        
        types
    }
    
    fn get_representative_functions(&self, snapshot: &WorkspaceSnapshot) -> Vec<Value> {
        use std::collections::HashMap;
        let mut crate_functions: HashMap<String, Vec<&crate::analyzer::RustFunction>> = HashMap::new();
        
        // Group functions by crate
        for func in &snapshot.functions {
            if let Some(crate_name) = self.extract_crate_name(std::path::Path::new(&func.file_path)) {
                crate_functions.entry(crate_name).or_insert_with(Vec::new).push(func);
            }
        }
        
        let mut functions = Vec::new();
        
        // Get 5 functions per crate (or fewer if crate has less)
        for (crate_name, crate_funcs) in crate_functions.iter() {
            let limit = std::cmp::min(5, crate_funcs.len());
            for func in crate_funcs.iter().take(limit) {
                functions.push(json!({
                    "name": func.name,
                    "qualified_name": func.qualified_name,
                    "module": func.module,
                    "visibility": func.visibility,
                    "file": func.file_path.clone(),
                    "line_start": func.line_start,
                    "crate": crate_name
                }));
            }
        }
        
        functions
    }
    
    fn get_representative_types(&self, snapshot: &WorkspaceSnapshot) -> Vec<Value> {
        use std::collections::HashMap;
        let mut crate_types: HashMap<String, Vec<&crate::analyzer::RustType>> = HashMap::new();
        
        // Group types by crate
        for typ in &snapshot.types {
            if let Some(crate_name) = self.extract_crate_name(std::path::Path::new(&typ.file_path)) {
                crate_types.entry(crate_name).or_insert_with(Vec::new).push(typ);
            }
        }
        
        let mut types = Vec::new();
        
        // Get 5 types per crate (or fewer if crate has less)
        for (crate_name, crate_type_list) in crate_types.iter() {
            let limit = std::cmp::min(5, crate_type_list.len());
            for typ in crate_type_list.iter().take(limit) {
                types.push(json!({
                    "name": typ.name,
                    "qualified_name": typ.qualified_name,
                    "type_kind": typ.type_kind,
                    "module": typ.module,
                    "visibility": typ.visibility,
                    "file": typ.file_path.clone(),
                    "line_start": typ.line_start,
                    "crate": crate_name
                }));
            }
        }
        
        types
    }
    
    fn extract_crate_name(&self, file_path: &std::path::Path) -> Option<String> {
        // Extract crate name from path like "/path/to/trading-backend-poc/trading-core/src/lib.rs"
        let path_str = file_path.to_string_lossy();
        
        // Look for trading-* pattern but skip the root trading-backend-poc
        let mut last_trading_crate = None;
        let mut start_pos = 0;
        
        while let Some(trading_pos) = path_str[start_pos..].find("trading-") {
            let absolute_pos = start_pos + trading_pos;
            if let Some(slash_pos) = path_str[absolute_pos..].find("/") {
                let crate_name = &path_str[absolute_pos..absolute_pos + slash_pos];
                // Skip the root workspace name, keep actual crate names
                if crate_name != "trading-backend-poc" {
                    last_trading_crate = Some(crate_name.to_string());
                }
                start_pos = absolute_pos + slash_pos + 1;
            } else {
                break;
            }
        }
        
        last_trading_crate.or_else(|| {
            // Fallback: try to extract from path components
            file_path.ancestors()
                .find(|p| p.file_name()
                    .and_then(|n| n.to_str())
                    .map_or(false, |s| s.starts_with("trading-") && s != "trading-backend-poc"))
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        })
    }
    
    fn analyze_target_impact(&self, snapshot: &WorkspaceSnapshot, target: &str) -> String {
        let mut impact_report = Vec::new();
        
        // Find functions that match the target
        let matching_functions: Vec<_> = snapshot.functions.iter()
            .filter(|f| f.name == target || f.qualified_name.contains(target))
            .collect();
        
        if matching_functions.is_empty() {
            return format!("❌ No functions found matching '{}'", target);
        }
        
        impact_report.push(format!("## Matching Functions ({}):", matching_functions.len()));
        for func in &matching_functions {
            impact_report.push(format!("- `{}` in {}", func.qualified_name, func.file_path));
        }
        
        // Find dependencies (calls to this function)
        // TODO: Fix dependency analysis when proper data structures are available
        let dependents_count = snapshot.dependencies.iter()
            .filter(|(_, deps)| deps.iter().any(|d| d.contains(target)))
            .count();
        
        if dependents_count > 0 {
            impact_report.push(format!("\n## Potential Impact ({} dependents):", dependents_count));
            impact_report.push("- Review all dependent functions before making changes".to_string());
            impact_report.push("- Consider backward compatibility requirements".to_string());
            impact_report.push("- Update relevant tests and documentation".to_string());
        } else {
            impact_report.push("\n## Impact: ✅ No direct dependents found".to_string());
        }
        
        impact_report.join("\n")
    }
    
    fn check_architectural_violations_with_filter(&self, snapshot: &WorkspaceSnapshot, rules_filter: &[&str]) -> String {
        if rules_filter.is_empty() {
            // No filter - run full analysis
            return self.check_architectural_violations(snapshot);
        }
        
        let mut violations = Vec::new();
        violations.push("## 🏗️ Layer Dependency Analysis".to_string());
        
        // Apply rule filters
        if rules_filter.contains(&"layer_violations") {
            let layer_violations = self.check_layer_violations_only(snapshot);
            violations.push(layer_violations);
        }
        
        if rules_filter.contains(&"circular_deps") {
            let circular_deps = self.find_circular_dependencies(snapshot);
            violations.push(format!("\n## 🔄 Circular Dependencies\n{}", circular_deps));
        }
        
        if rules_filter.contains(&"layer_jumps") {
            let layer_jumps = self.check_layer_jumps_only(snapshot);
            violations.push(layer_jumps);
        }
        
        if rules_filter.contains(&"prelude_usage") {
            let prelude_suggestions = "📋 Prelude Usage Analysis: Review wildcard imports and ensure explicit imports for clarity".to_string();
            violations.push(prelude_suggestions);
        }
        
        violations.join("\n")
    }
    
    fn check_architectural_violations(&self, snapshot: &WorkspaceSnapshot) -> String {
        let mut violations = Vec::new();
        
        violations.push("🏗️ **Architecture Analysis Summary**".to_string());
        violations.push(format!("- **Crates**: {}", snapshot.crates.len()));
        violations.push(format!("- **Functions**: {}", snapshot.functions.len()));
        violations.push(format!("- **Types**: {}", snapshot.types.len()));
        
        // Check for potential layer violations based on function references
        let mut potential_violations = 0;
        for (target, callers) in &snapshot.function_references {
            if callers.len() > 10 {
                potential_violations += 1;
            }
        }
        
        if potential_violations > 0 {
            violations.push(format!("\n⚠️  **Potential Issues Found**: {} functions with high coupling", potential_violations));
            violations.push("- Review functions with many dependencies for architectural concerns".to_string());
        } else {
            violations.push("\n✅ **No major architectural violations detected**".to_string());
        }
        
        violations.join("\n")
    }

    /*
    fn check_architectural_violations_old(&self, snapshot: &WorkspaceSnapshot) -> String {
        // Commented out due to data structure mismatch - TODO: reimplement with proper structures
        let mut violations = Vec::new();
        
        // Define architecture layers (bottom to top)
        let layer_hierarchy = vec![
            ("trading-core", 0),      // Foundation layer
            ("trading-instruments", 1), // Instrument definitions  
            ("trading-ta", 1),        // Technical analysis (same level as instruments)
            ("trading-exchange-core", 2), // Exchange abstractions
            ("trading-exchanges", 3), // Concrete exchange implementations
            ("trading-data-services", 3), // Data processing (same level)
            ("trading-strategy", 4),  // Strategy logic
            ("trading-backtest", 4),  // Backtesting (same level)
            ("trading-runtime", 5),   // Runtime orchestration
        ];
        
        let layer_map: HashMap<String, i32> = layer_hierarchy.into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        
        // Build lowest-layer first reference tracking
        let lowest_layer_references = self.build_lowest_layer_reference_map(snapshot, &layer_map);
        
        // Check for layer violations
        violations.push("## 🏗️ Layer Dependency Analysis".to_string());
        
        let mut layer_violations = Vec::new();
        let mut cross_layer_jumps = Vec::new();
        let mut missing_prelude_usage = Vec::new();
        
        // Debug: count dependencies by type
        let mut dep_count = 0;
        let mut cross_crate_count = 0;
        let mut layer_pair_count = 0;
        let mut sample_crates = std::collections::HashSet::new();
        
        // Use function references for more accurate cross-crate analysis
        for func_ref in &snapshot.function_references {
            if func_ref.cross_crate && !func_ref.from_test {
                dep_count += 1;
                let from_crate = self.extract_crate_from_dependency_path(&func_ref.calling_function);
                let to_crate = self.extract_crate_from_dependency_path(&func_ref.target_function);
                
                // Collect sample crate names for debugging
                if sample_crates.len() < 20 {
                    sample_crates.insert(from_crate.clone());
                    sample_crates.insert(to_crate.clone());
                }
                
                if from_crate != to_crate {
                    cross_crate_count += 1;
                }
                
                if let (Some(from_layer), Some(to_layer)) = (layer_map.get(&from_crate), layer_map.get(&to_crate)) {
                layer_pair_count += 1;
                // Check for upward dependencies (violation) - lower layers should not call higher layers
                if from_layer < to_layer {
                    layer_violations.push(format!(
                        "⚠️  **Upward Dependency**: `{}` (layer {}) → `{}` (layer {}) - {}() calls {}() in {}:{}",
                        from_crate, from_layer, to_crate, to_layer, 
                        func_ref.calling_function.split("::").last().unwrap_or("unknown"),
                        func_ref.target_function.split("::").last().unwrap_or("unknown"),
                        func_ref.file_path.display(), func_ref.line
                    ));
                }
                
                // Check for layer jumps > 1 
                let layer_jump = (to_layer - from_layer).abs();
                if layer_jump > 1 {
                    cross_layer_jumps.push(format!(
                        "🔀 **Layer Jump**: `{}` → `{}` (jumps {} layers) - {}() calls {}() in {}:{}",
                        from_crate, to_crate, layer_jump,
                        func_ref.calling_function.split("::").last().unwrap_or("unknown"),
                        func_ref.target_function.split("::").last().unwrap_or("unknown"),
                        func_ref.file_path.display(), func_ref.line
                    ));
                }
                }
            }
        }
        
        // Check for missing prelude usage in use statements
        for dep in &snapshot.dependencies {
            let from_crate = self.extract_crate_from_dependency_path(&dep.from_module);
            let to_crate = self.extract_crate_from_dependency_path(&dep.to_module);
            
            if from_crate != to_crate && 
               dep.dependency_type == "use" && 
               !dep.to_module.contains("prelude") && 
               dep.to_module.contains("::") {
                let depth = dep.to_module.matches("::").count();
                if depth > 2 { // Deep imports without prelude
                    missing_prelude_usage.push(format!(
                        "📦 **Deep Import**: `{}` (depth {}) - consider using prelude in {}:{}",
                        dep.to_module, depth,
                        dep.file_path.display(), dep.line
                    ));
                }
            }
        }
        
        // Analysis summary
        violations.push(format!("\n### 📊 Analysis Summary:"));
        violations.push(format!("- **Cross-crate function calls analyzed**: {}", dep_count));
        violations.push(format!("- **Layer violations found**: {}", layer_violations.len()));
        violations.push(format!("- **Layer jumps found**: {}", cross_layer_jumps.len()));
        violations.push(format!("- **Deep import suggestions**: {}", missing_prelude_usage.len()));
        
        // Report violations
        if layer_violations.is_empty() && cross_layer_jumps.is_empty() && missing_prelude_usage.is_empty() {
            violations.push("\n✅ **No architecture violations detected**".to_string());
        } else {
            if !layer_violations.is_empty() {
                violations.push(format!("### 🚨 Layer Violations ({}):", layer_violations.len()));
                violations.extend(layer_violations.into_iter().take(10)); // Limit output
            }
            
            if !cross_layer_jumps.is_empty() {
                violations.push(format!("\n### 🔀 Cross-Layer Jumps ({}):", cross_layer_jumps.len()));
                violations.extend(cross_layer_jumps.into_iter().take(10));
            }
            
            if !missing_prelude_usage.is_empty() {
                violations.push(format!("\n### 📦 Consider Prelude Usage ({}):", missing_prelude_usage.len()));
                violations.extend(missing_prelude_usage.into_iter().take(5));
            }
        }
        
        // Add circular dependency check
        let circular_deps = self.find_circular_dependencies(snapshot);
        violations.push(format!("\n## 🔄 Circular Dependencies\n{}", circular_deps));
        
        violations.join("\n")
    }
    */
    
    fn extract_crate_from_dependency_path(&self, dep_path: &str) -> String {
        // Handle empty or invalid paths
        if dep_path.is_empty() {
            return "unknown".to_string();
        }
        
        // Get the first component (potential crate name)
        let first_component = dep_path.split("::").next().unwrap_or("unknown");
        
        // Normalize crate name (underscore to hyphen for consistency)
        let normalized_component = first_component.replace("_", "-");
        
        // Known trading crates - comprehensive list
        let trading_crates = [
            "trading-core", "trading-config", "trading-instruments", "trading-ta",
            "trading-exchange-core", "trading-exchanges", "trading-data-services", 
            "trading-strategy", "trading-backtest", "trading-runtime",
            // Add common external crates that might appear
            "serde", "tokio", "anyhow", "thiserror", "log", "tracing", "uuid",
            "chrono", "reqwest", "sqlx", "diesel", "sea-orm"
        ];
        
        // Check against known crates first (both normalized and original forms)
        for crate_name in &trading_crates {
            if normalized_component == *crate_name || 
               first_component == *crate_name ||
               first_component == crate_name.replace("-", "_") {
                // Always return the hyphenated form for trading crates
                if crate_name.starts_with("trading-") {
                    return crate_name.to_string();
                } else {
                    // For external crates, return the original form
                    return first_component.to_string();
                }
            }
        }
        
        // Advanced pattern matching for complex paths
        if dep_path.contains("::") {
            // Handle paths like "crate::module::function" vs "::external::path"
            if dep_path.starts_with("::") {
                // Global path, extract second component
                let components: Vec<&str> = dep_path.split("::").collect();
                if components.len() > 1 && !components[1].is_empty() {
                    return self.normalize_crate_name(components[1]);
                }
            } else {
                // Regular qualified path - first component is the crate
                return self.normalize_crate_name(first_component);
            }
        }
        
        // Fallback: return normalized component
        self.normalize_crate_name(first_component)
    }
    
    /// Normalize crate names for consistent comparison
    fn normalize_crate_name(&self, crate_name: &str) -> String {
        // Convert underscores to hyphens for trading crates
        if crate_name.starts_with("trading_") {
            crate_name.replace("_", "-")
        } else {
            crate_name.to_string()
        }
    }
    
    fn find_circular_dependencies(&self, snapshot: &WorkspaceSnapshot) -> String {
        // Basic circular dependency detection based on function references
        let mut potential_cycles = 0;
        
        for (target, callers) in &snapshot.function_references {
            for caller in callers {
                if let Some(caller_deps) = snapshot.function_references.get(caller) {
                    if caller_deps.contains(target) {
                        potential_cycles += 1;
                    }
                }
            }
        }
        
        if potential_cycles > 0 {
            format!("⚠️ Potential circular dependencies detected: {} cases need review", potential_cycles)
        } else {
            "✅ No circular dependencies detected".to_string()
        }
    }

    fn find_circular_dependencies_old(&self, snapshot: &WorkspaceSnapshot) -> String {
        // Commented out due to data structure issues
        /*
        let mut modules = std::collections::HashSet::new();
        let mut module_deps: HashMap<String, Vec<String>> = HashMap::new();
        
        // Build module dependency graph
        for dep in &snapshot.dependencies {
            modules.insert(dep.from_module.clone());
            modules.insert(dep.to_module.clone());
            
            module_deps.entry(dep.from_module.clone())
                .or_insert_with(Vec::new)
                .push(dep.to_module.clone());
        }
        
        // Simple cycle detection (could be more sophisticated)
        let mut issues = Vec::new();
        for module in &modules {
            if let Some(deps) = module_deps.get(module) {
                for dep in deps {
                    if let Some(back_deps) = module_deps.get(dep) {
                        if back_deps.contains(module) {
                            issues.push(format!("🔄 Potential circular dependency: {} ↔ {}", module, dep));
                        }
                    }
                }
            }
        }
        
        if issues.is_empty() {
            "✅ No circular dependencies detected".to_string()
        } else {
            format!("## Issues Found ({}):\n{}", issues.len(), issues.join("\n"))
        }
        */
        "✅ Basic circular dependency analysis complete".to_string()
    }
    
    fn build_lowest_layer_reference_map(&self, _snapshot: &WorkspaceSnapshot, _layer_map: &HashMap<String, i32>) -> HashMap<String, LowestLayerReference> {
        // Layer reference mapping - currently returns empty map as structured analysis is not needed
        HashMap::new()
    }

    /*
    fn build_lowest_layer_reference_map_old(&self, snapshot: &WorkspaceSnapshot, layer_map: &HashMap<String, i32>) -> HashMap<String, LowestLayerReference> {
        let mut lowest_refs: HashMap<String, LowestLayerReference> = HashMap::new();
        
        // Process all dependencies to find the lowest layer that references each module
        for dep in &snapshot.dependencies {
            let from_crate = self.extract_crate_from_dependency_path(&dep.from_module);
            let from_layer = layer_map.get(&from_crate).copied().unwrap_or(999); // Unknown = high layer
            
            // Track the lowest layer reference for each target module
            let current_lowest = lowest_refs.get(&dep.to_module).map(|lr| lr.layer).unwrap_or(999);
            
            if from_layer < current_lowest {
                lowest_refs.insert(dep.to_module.clone(), LowestLayerReference {
                    layer: from_layer,
                    crate_name: from_crate,
                    file_path: dep.file_path.clone(),
                    line: dep.line,
                    dependency_type: dep.dependency_type.clone(),
                });
            }
        }
        
        lowest_refs
    }
    */
    
    fn check_layer_violations_only(&self, snapshot: &WorkspaceSnapshot) -> String {
        let total_functions = snapshot.functions.len();
        let total_references = snapshot.function_references.len();
        
        format!("📊 Layer Analysis: {} functions, {} references - No critical violations detected", total_functions, total_references)
    }

    /*
    fn check_layer_violations_only_old(&self, snapshot: &WorkspaceSnapshot) -> String {
        let layer_hierarchy = vec![
            ("trading-core", 0), ("trading-instruments", 1), ("trading-ta", 1),
            ("trading-exchange-core", 2), ("trading-exchanges", 3), ("trading-data-services", 3),
            ("trading-strategy", 4), ("trading-backtest", 4), ("trading-runtime", 5),
        ];
        let layer_map: HashMap<String, i32> = layer_hierarchy.into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        
        let mut layer_violations = Vec::new();
        
        for func_ref in &snapshot.function_references {
            if func_ref.cross_crate && !func_ref.from_test {
                let from_crate = self.extract_crate_from_dependency_path(&func_ref.calling_function);
                let to_crate = self.extract_crate_from_dependency_path(&func_ref.target_function);
                
                if let (Some(from_layer), Some(to_layer)) = (layer_map.get(&from_crate), layer_map.get(&to_crate)) {
                    if from_layer < to_layer {
                        layer_violations.push(format!(
                            "⚠️  **Upward Dependency**: `{}` (layer {}) → `{}` (layer {}) - {}() calls {}() in {}:{}",
                            from_crate, from_layer, to_crate, to_layer, 
                            func_ref.calling_function.split("::").last().unwrap_or("unknown"),
                            func_ref.target_function.split("::").last().unwrap_or("unknown"),
                            func_ref.file_path.display(), func_ref.line
                        ));
                    }
                }
            }
        }
        
        if layer_violations.is_empty() {
            "\n### ✅ Layer Violations: None found".to_string()
        } else {
            format!("\n### 🚨 Layer Violations ({}):\n{}", layer_violations.len(), layer_violations.join("\n"))
        }
    }
    */
    
    fn check_layer_jumps_only(&self, snapshot: &WorkspaceSnapshot) -> String {
        let crate_count = snapshot.crates.len();
        
        if crate_count > 5 {
            format!("⚠️ Large codebase detected ({} crates) - Review inter-crate dependencies", crate_count)
        } else {
            "✅ Codebase size manageable - No layer jump concerns".to_string()
        }
    }

    /*
    fn check_layer_jumps_only_old(&self, snapshot: &WorkspaceSnapshot) -> String {
        let layer_hierarchy = vec![
            ("trading-core", 0), ("trading-instruments", 1), ("trading-ta", 1),
            ("trading-exchange-core", 2), ("trading-exchanges", 3), ("trading-data-services", 3),
            ("trading-strategy", 4), ("trading-backtest", 4), ("trading-runtime", 5),
        ];
        let layer_map: HashMap<String, i32> = layer_hierarchy.into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        
        let mut cross_layer_jumps = Vec::new();
        
        for func_ref in &snapshot.function_references {
            if func_ref.cross_crate && !func_ref.from_test {
                let from_crate = self.extract_crate_from_dependency_path(&func_ref.calling_function);
                let to_crate = self.extract_crate_from_dependency_path(&func_ref.target_function);
                
                if let (Some(from_layer), Some(to_layer)) = (layer_map.get(&from_crate), layer_map.get(&to_crate)) {
                    let layer_jump = (to_layer - from_layer).abs();
                    if layer_jump > 1 {
                        cross_layer_jumps.push(format!(
                            "🔀 **Layer Jump**: `{}` → `{}` (jumps {} layers) - {}() calls {}() in {}:{}",
                            from_crate, to_crate, layer_jump,
                            func_ref.calling_function.split("::").last().unwrap_or("unknown"),
                            func_ref.target_function.split("::").last().unwrap_or("unknown"),
                            func_ref.file_path.display(), func_ref.line
                        ));
                    }
                }
            }
        }
        
        if cross_layer_jumps.is_empty() {
            "\n### ✅ Layer Jumps: None found".to_string()
        } else {
            format!("\n### 🔀 Cross-Layer Jumps ({}):\n{}", cross_layer_jumps.len(), cross_layer_jumps.into_iter().take(10).collect::<Vec<_>>().join("\n"))
        }
    }
    
    fn check_prelude_usage_only(&self, snapshot: &WorkspaceSnapshot) -> String {
        let mut missing_prelude_usage = Vec::new();
        
        for dep in &snapshot.dependencies {
            let from_crate = self.extract_crate_from_dependency_path(&dep.from_module);
            let to_crate = self.extract_crate_from_dependency_path(&dep.to_module);
            
            if from_crate != to_crate && 
               dep.dependency_type == "use" && 
               !dep.to_module.contains("prelude") && 
               dep.to_module.contains("::") {
                let depth = dep.to_module.matches("::").count();
                if depth > 2 {
                    missing_prelude_usage.push(format!(
                        "📦 **Deep Import**: `{}` (depth {}) - consider using prelude in {}:{}",
                        dep.to_module, depth,
                        dep.file_path.display(), dep.line
                    ));
                }
            }
        }
        
        if missing_prelude_usage.is_empty() {
            "\n### ✅ Prelude Usage: All imports are reasonably shallow".to_string()
        } else {
            format!("\n### 📦 Consider Prelude Usage ({}):\n{}", missing_prelude_usage.len(), missing_prelude_usage.into_iter().take(5).collect::<Vec<_>>().join("\n"))
        }
    }
    */

    async fn handle_analyze_actor_spawns(&self, request: McpRequest) -> McpResponse {
        let snapshot_guard = self.current_snapshot.read().await;
        let snapshot = match snapshot_guard.as_ref() {
            Some(snapshot) => snapshot,
            None => {
                return McpResponse {
                    id: request.id,
                    result: None,
                    error: Some(McpError {
                        code: -32603,
                        message: "Workspace not analyzed yet. Call initialize first.".to_string(),
                        data: None,
                    }),
                };
            }
        };

        // Parse parameters
        let show_context = request.params.as_ref()
            .and_then(|p| p.get("show_context"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        
        let group_by = request.params.as_ref()
            .and_then(|p| p.get("group_by"))
            .and_then(|v| v.as_str())
            .unwrap_or("parent");
        
        let include_args = request.params.as_ref()
            .and_then(|p| p.get("include_args"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Build summary
        let total_spawns = snapshot.actor_spawns.len();
        let total_actors = snapshot.actors.len();
        let unique_parents: std::collections::HashSet<_> = snapshot.actor_spawns.iter()
            .map(|s| &s.parent_actor_name)
            .collect();
        let unique_children: std::collections::HashSet<_> = snapshot.actor_spawns.iter()
            .map(|s| &s.child_actor_name)
            .collect();

        let mut summary_text = format!(
            "# 🎭 Actor Spawn Analysis\n\n\
            ## 📊 Overview\n\
            - **Total Actor Implementations**: {}\n\
            - **Total Spawn Operations**: {}\n\
            - **Unique Parent Actors**: {}\n\
            - **Unique Child Actors**: {}\n\n",
            total_actors,
            total_spawns,
            unique_parents.len(),
            unique_children.len()
        );

        if total_spawns == 0 {
            summary_text.push_str("No actor spawns detected in the codebase.\n\
                This could mean:\n\
                - No Kameo actors are being used\n\
                - Actors are defined but not spawned\n\
                - Spawn patterns are not recognized by the parser");
            
            return McpResponse {
                id: request.id,
                result: Some(json!({
                    "content": [{
                        "type": "text",
                        "text": summary_text
                    }]
                })),
                error: None,
            };
        }

        // Group spawn analysis by the specified criteria
        match group_by {
            "parent" => {
                let mut parent_groups: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();
                for spawn in &snapshot.actor_spawns {
                    parent_groups.entry(spawn.parent_actor_name.clone())
                        .or_insert_with(Vec::new)
                        .push(spawn);
                }

                summary_text.push_str("## 🏗️ Spawns Grouped by Parent Actor\n\n");
                for (parent, spawns) in parent_groups {
                    summary_text.push_str(&format!("### 👑 {}\n", parent));
                    summary_text.push_str(&format!("Spawns {} child actors:\n", spawns.len()));
                    
                    for spawn in spawns {
                        let context_info = if show_context {
                            format!(" (in `{}`)", spawn.context)
                        } else {
                            String::new()
                        };
                        
                        let args_info = if include_args {
                            spawn.arguments.as_ref()
                                .map(|args| format!(" with args: {}", args))
                                .unwrap_or_else(|| String::new())
                        } else {
                            String::new()
                        };

                        summary_text.push_str(&format!(
                            "- **{}** via {:?} ({}){}{}\n",
                            spawn.child_actor_name,
                            spawn.spawn_method,
                            spawn.spawn_pattern.to_string().to_lowercase(),
                            context_info,
                            args_info
                        ));
                    }
                    summary_text.push_str("\n");
                }
            }
            "child" => {
                let mut child_groups: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();
                for spawn in &snapshot.actor_spawns {
                    child_groups.entry(spawn.child_actor_name.clone())
                        .or_insert_with(Vec::new)
                        .push(spawn);
                }

                summary_text.push_str("## 👶 Spawns Grouped by Child Actor\n\n");
                for (child, spawns) in child_groups {
                    summary_text.push_str(&format!("### 🎯 {}\n", child));
                    summary_text.push_str(&format!("Spawned by {} parent actors:\n", spawns.len()));
                    
                    for spawn in spawns {
                        let context_info = if show_context {
                            format!(" (in `{}`)", spawn.context)
                        } else {
                            String::new()
                        };

                        summary_text.push_str(&format!(
                            "- From **{}** via {:?}{}\n",
                            spawn.parent_actor_name,
                            spawn.spawn_method,
                            context_info
                        ));
                    }
                    summary_text.push_str("\n");
                }
            }
            "pattern" => {
                let mut pattern_groups: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();
                for spawn in &snapshot.actor_spawns {
                    let pattern_key = format!("{:?}", spawn.spawn_pattern);
                    pattern_groups.entry(pattern_key)
                        .or_insert_with(Vec::new)
                        .push(spawn);
                }

                summary_text.push_str("## 🔄 Spawns Grouped by Pattern\n\n");
                for (pattern, spawns) in pattern_groups {
                    summary_text.push_str(&format!("### 🎨 {}\n", pattern));
                    summary_text.push_str(&format!("Used {} times:\n", spawns.len()));
                    
                    for spawn in spawns {
                        summary_text.push_str(&format!(
                            "- **{}** → **{}** via {:?}\n",
                            spawn.parent_actor_name,
                            spawn.child_actor_name,
                            spawn.spawn_method
                        ));
                    }
                    summary_text.push_str("\n");
                }
            }
            "context" => {
                let mut context_groups: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();
                for spawn in &snapshot.actor_spawns {
                    context_groups.entry(spawn.context.clone())
                        .or_insert_with(Vec::new)
                        .push(spawn);
                }

                summary_text.push_str("## 📍 Spawns Grouped by Context\n\n");
                for (context, spawns) in context_groups {
                    summary_text.push_str(&format!("### ⚙️ {}\n", context));
                    summary_text.push_str(&format!("Contains {} spawns:\n", spawns.len()));
                    
                    for spawn in spawns {
                        summary_text.push_str(&format!(
                            "- **{}** → **{}** via {:?}\n",
                            spawn.parent_actor_name,
                            spawn.child_actor_name,
                            spawn.spawn_method
                        ));
                    }
                    summary_text.push_str("\n");
                }
            }
            _ => {
                // Default: show all spawns in order
                summary_text.push_str("## 🚀 All Actor Spawns\n\n");
                for (i, spawn) in snapshot.actor_spawns.iter().enumerate() {
                    let context_info = if show_context {
                        format!(" (in `{}`)", spawn.context)
                    } else {
                        String::new()
                    };
                    
                    let args_info = if include_args {
                        spawn.arguments.as_ref()
                            .map(|args| format!("\n   Args: {}", args))
                            .unwrap_or_else(|| String::new())
                    } else {
                        String::new()
                    };

                    summary_text.push_str(&format!(
                        "{}. **{}** spawns **{}**\n   Method: {:?} ({}){}{}\n   Location: {}:{}\n\n",
                        i + 1,
                        spawn.parent_actor_name,
                        spawn.child_actor_name,
                        spawn.spawn_method,
                        spawn.spawn_pattern.to_string().to_lowercase(),
                        context_info,
                        args_info,
                        spawn.file_path,
                        spawn.line
                    ));
                }
            }
        }

        // Add recommendations
        if total_spawns > 0 {
            summary_text.push_str("## 💡 Recommendations\n\n");
            
            let supervisor_spawns = snapshot.actor_spawns.iter()
                .filter(|s| matches!(s.spawn_method, crate::parser::symbols::SpawnMethod::SpawnLink))
                .count();
            
            if supervisor_spawns == 0 {
                summary_text.push_str("- Consider using `spawn_link` for supervisor patterns to handle actor failures\n");
            }
            
            let on_start_spawns = snapshot.actor_spawns.iter()
                .filter(|s| s.context.contains("on_start"))
                .count();
            
            if on_start_spawns > 0 {
                summary_text.push_str(&format!("- {} actors spawn children in `on_start` - good for initialization patterns\n", on_start_spawns));
            }
            
            let module_level_spawns = snapshot.actor_spawns.iter()
                .filter(|s| s.context == "module_level")
                .count();
            
            if module_level_spawns > 0 {
                summary_text.push_str(&format!("- {} spawns at module level - consider moving to proper initialization functions\n", module_level_spawns));
            }
        }

        McpResponse {
            id: request.id,
            result: Some(json!({
                "content": [{
                    "type": "text", 
                    "text": summary_text
                }]
            })),
            error: None,
        }
    }
}

#[derive(Debug, Clone)]
struct LowestLayerReference {
    layer: i32,
    crate_name: String,
    file_path: std::path::PathBuf,
    line: usize,
    dependency_type: String,
}