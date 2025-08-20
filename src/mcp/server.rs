use std::path::Path;
use anyhow::Result;
use serde_json::{json, Value};
use tokio::sync::RwLock;
use std::sync::Arc;
use std::collections::HashMap;

use crate::analyzer::{WorkspaceAnalyzer, WorkspaceSnapshot};
use crate::graph::MemgraphClient;

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

#[derive(Debug, Clone)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

pub struct WorkspaceMcpServer {
    analyzer: Arc<RwLock<WorkspaceAnalyzer>>,
    graph_client: Arc<RwLock<MemgraphClient>>,
    workspace_root: std::path::PathBuf,
    current_snapshot: Arc<RwLock<Option<WorkspaceSnapshot>>>,
}

impl WorkspaceMcpServer {
    pub async fn new(workspace_root: &Path) -> Result<Self> {
        let analyzer = WorkspaceAnalyzer::new(workspace_root)?;
        let workspace_name = workspace_root.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown_workspace");
        
        let graph_client = MemgraphClient::new(workspace_name).await?;
        
        Ok(Self {
            analyzer: Arc::new(RwLock::new(analyzer)),
            graph_client: Arc::new(RwLock::new(graph_client)),
            workspace_root: workspace_root.to_path_buf(),
            current_snapshot: Arc::new(RwLock::new(None)),
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
        let snapshot = match self.analyzer.write().await.analyze_workspace() {
            Ok(snapshot) => snapshot,
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
        };
        
        // Store the snapshot
        *self.current_snapshot.write().await = Some(snapshot);
        
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
                            "properties": {}
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
                    }
                ]
            })),
            error: None,
        }
    }
    
    async fn handle_workspace_context(&self, request: McpRequest) -> McpResponse {
        let snapshot = self.current_snapshot.read().await;
        
        match snapshot.as_ref() {
            Some(snapshot) => {
                let context = json!({
                    "workspace_root": self.workspace_root.display().to_string(),
                    "analysis_timestamp": snapshot.timestamp.duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default().as_secs(),
                    "summary": {
                        "total_functions": snapshot.functions.len(),
                        "total_types": snapshot.types.len(),
                        "total_dependencies": snapshot.dependencies.len(),
                        "modules": self.get_module_summary(&snapshot)
                    },
                    "top_functions": self.get_representative_functions(&snapshot),
                    "top_types": self.get_representative_types(&snapshot)
                });
                
                McpResponse {
                    id: request.id,
                    result: Some(json!({
                        "content": [{
                            "type": "text",
                            "text": format!("# Workspace Analysis\n\n{}", serde_json::to_string_pretty(&context).unwrap())
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
        
        match snapshot.as_ref() {
            Some(snapshot) => {
                let violations = self.check_architectural_violations(snapshot);
                
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
               func.file_path.to_string_lossy().contains("/tests/") ||
               func.file_path.to_string_lossy().contains("test_") {
                test_functions.insert(func.qualified_name.clone());
            }
        }
        
        // Count function references using new data structure (only non-test references)
        for func_ref in &snapshot.function_references {
            if !func_ref.from_test { // Only count non-test references
                *function_refs.entry(func_ref.target_function.clone()).or_insert(0) += 1;
            }
        }
        
        // Count cross-crate references separately
        let mut cross_crate_refs = HashMap::new();
        for func_ref in &snapshot.function_references {
            if func_ref.cross_crate && !func_ref.from_test {
                *cross_crate_refs.entry(func_ref.target_function.clone()).or_insert(0) += 1;
            }
        }
        
        // Find production functions that are heavily used but not tested
        let mut untested_heavy_functions = Vec::new();
        let mut tested_functions = Vec::new();
        let mut test_coverage_stats = HashMap::new();
        
        for func in &snapshot.functions {
            let ref_count = function_refs.get(&func.qualified_name).unwrap_or(&0);
            let cross_crate_count = cross_crate_refs.get(&func.qualified_name).unwrap_or(&0);
            let crate_name = self.extract_crate_name(&func.file_path).unwrap_or_else(|| "unknown".to_string());
            
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
                        "file": func.file_path.to_string_lossy(),
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
                    "file": func.file_path.to_string_lossy(),
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
    
    fn get_representative_functions(&self, snapshot: &WorkspaceSnapshot) -> Vec<Value> {
        use std::collections::HashMap;
        let mut crate_functions: HashMap<String, Vec<&crate::analyzer::RustFunction>> = HashMap::new();
        
        // Group functions by crate
        for func in &snapshot.functions {
            if let Some(crate_name) = self.extract_crate_name(&func.file_path) {
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
                    "file": func.file_path.display().to_string(),
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
            if let Some(crate_name) = self.extract_crate_name(&typ.file_path) {
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
                    "file": typ.file_path.display().to_string(),
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
            impact_report.push(format!("- `{}` in {}", func.qualified_name, func.file_path.display()));
        }
        
        // Find dependencies (calls to this function)
        let mut dependents = Vec::new();
        for dep in &snapshot.dependencies {
            if dep.to_module.contains(target) || dep.to_module == target {
                dependents.push(dep);
            }
        }
        
        if !dependents.is_empty() {
            impact_report.push(format!("\\n## Potential Impact ({} dependents):", dependents.len()));
            for dep in dependents.iter().take(10) {
                impact_report.push(format!("- {} calls from {} (line {})", 
                    dep.dependency_type, dep.file_path.display(), dep.line));
            }
            if dependents.len() > 10 {
                impact_report.push(format!("... and {} more", dependents.len() - 10));
            }
        } else {
            impact_report.push("\\n## Impact: ✅ No direct dependents found".to_string());
        }
        
        impact_report.join("\\n")
    }
    
    fn check_architectural_violations(&self, snapshot: &WorkspaceSnapshot) -> String {
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
        
        // Check for layer violations
        violations.push("## 🏗️ Layer Dependency Analysis".to_string());
        
        let mut layer_violations = Vec::new();
        let mut cross_layer_jumps = Vec::new();
        let mut missing_prelude_usage = Vec::new();
        
        for dep in &snapshot.dependencies {
            let from_crate = self.extract_crate_from_dependency_path(&dep.from_module);
            let to_crate = self.extract_crate_from_dependency_path(&dep.to_module);
            
            if let (Some(from_layer), Some(to_layer)) = (layer_map.get(&from_crate), layer_map.get(&to_crate)) {
                // Check for upward dependencies (violation)
                if from_layer > to_layer {
                    layer_violations.push(format!(
                        "⚠️  **Upward Dependency**: `{}` (layer {}) → `{}` (layer {}) in {}:{}",
                        from_crate, from_layer, to_crate, to_layer, 
                        dep.file_path.display(), dep.line
                    ));
                }
                
                // Check for layer jumps > 1 without prelude
                let layer_jump = (to_layer - from_layer).abs();
                if layer_jump > 1 && !dep.to_module.contains("prelude") {
                    cross_layer_jumps.push(format!(
                        "🔀 **Layer Jump**: `{}` → `{}` (jumps {} layers) without prelude in {}:{}",
                        from_crate, to_crate, layer_jump,
                        dep.file_path.display(), dep.line  
                    ));
                }
            }
            
            // Check for missing prelude usage in cross-crate dependencies
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
        
        // Report violations
        if layer_violations.is_empty() && cross_layer_jumps.is_empty() && missing_prelude_usage.is_empty() {
            violations.push("✅ **No architecture violations detected**".to_string());
        } else {
            if !layer_violations.is_empty() {
                violations.push(format!("### 🚨 Layer Violations ({}):", layer_violations.len()));
                violations.extend(layer_violations.into_iter().take(10)); // Limit output
            }
            
            if !cross_layer_jumps.is_empty() {
                violations.push(format!("\\n### 🔀 Cross-Layer Jumps ({}):", cross_layer_jumps.len()));
                violations.extend(cross_layer_jumps.into_iter().take(10));
            }
            
            if !missing_prelude_usage.is_empty() {
                violations.push(format!("\\n### 📦 Consider Prelude Usage ({}):", missing_prelude_usage.len()));
                violations.extend(missing_prelude_usage.into_iter().take(5));
            }
        }
        
        // Add circular dependency check
        let circular_deps = self.find_circular_dependencies(snapshot);
        violations.push(format!("\\n## 🔄 Circular Dependencies\\n{}", circular_deps));
        
        violations.join("\\n")
    }
    
    fn extract_crate_from_dependency_path(&self, dep_path: &str) -> String {
        // Extract crate name from dependency path
        if dep_path.contains("trading-") {
            if let Some(start) = dep_path.find("trading-") {
                let remaining = &dep_path[start..];
                if let Some(end) = remaining.find("::").or_else(|| Some(remaining.len())) {
                    return remaining[..end].to_string();
                }
            }
        }
        
        // Fallback: use the first component
        dep_path.split("::").next().unwrap_or("unknown").to_string()
    }
    
    fn find_circular_dependencies(&self, snapshot: &WorkspaceSnapshot) -> String {
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
            format!("## Issues Found ({}):  \\n{}  ", issues.len(), issues.join("\\n"))
        }
    }
}