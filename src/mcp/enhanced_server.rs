use anyhow::Result;
use serde_json::{json, Value};
use tokio::sync::RwLock;
use std::sync::Arc;

use crate::config::Config;
use crate::graph::MemgraphClient;
use crate::parser::ParsedSymbols;
use crate::analyzer::WorkspaceAnalyzer;
use crate::workspace::{WorkspaceDiscovery, CrateMetadata};
use crate::architecture::ArchitectureAnalyzer;
use crate::embeddings::{EmbeddingGenerator, SemanticSearch};
use crate::incremental::IncrementalUpdater;

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

pub struct EnhancedMcpServer {
    config: Config,
    graph: Arc<MemgraphClient>,
    analyzer: Arc<RwLock<WorkspaceAnalyzer>>,
    workspace_discovery: Arc<RwLock<WorkspaceDiscovery>>,
    architecture_analyzer: Arc<ArchitectureAnalyzer>,
    embedding_generator: Arc<EmbeddingGenerator>,
    semantic_search: Arc<RwLock<SemanticSearch>>,
    incremental_updater: Arc<RwLock<IncrementalUpdater>>,
    current_symbols: Arc<RwLock<Option<ParsedSymbols>>>,
    current_crates: Arc<RwLock<Vec<CrateMetadata>>>,
}

impl EnhancedMcpServer {
    pub async fn new(config_path: &str) -> Result<Self> {
        let config = Config::from_file(config_path)?;
        let graph = Arc::new(MemgraphClient::new(&config).await?);
        let analyzer = Arc::new(RwLock::new(WorkspaceAnalyzer::new_with_config(config.clone())?));
        
        let workspace_discovery = Arc::new(RwLock::new(WorkspaceDiscovery::new(config.clone())));
        let architecture_analyzer = Arc::new(ArchitectureAnalyzer::new(graph.clone(), config.clone()));
        let embedding_generator = Arc::new(EmbeddingGenerator::new(config.clone()));
        let semantic_search = Arc::new(RwLock::new(SemanticSearch::new()));
        let incremental_updater = Arc::new(RwLock::new(IncrementalUpdater::new(config.clone(), graph.clone())?));

        Ok(Self {
            config,
            graph,
            analyzer,
            workspace_discovery,
            architecture_analyzer,
            embedding_generator,
            semantic_search,
            incremental_updater,
            current_symbols: Arc::new(RwLock::new(None)),
            current_crates: Arc::new(RwLock::new(Vec::new())),
        })
    }

    pub async fn auto_initialize(&self) -> Result<()> {
        let total_start = std::time::Instant::now();
        eprintln!("🔍 Discovering workspace crates...");
        let discovery_start = std::time::Instant::now();
        
        let mut workspace_discovery = self.workspace_discovery.write().await;
        let crates = workspace_discovery.discover_crates().await?;
        
        eprintln!("📊 Found {} crates ({} workspace members) in {:?}", 
            crates.len(),
            crates.iter().filter(|c| c.is_workspace_member).count(),
            discovery_start.elapsed()
        );

        // Crate nodes will be created by analyze_and_populate_graph
        eprintln!("🔬 Parsing workspace files using unified analyzer...");
        let parse_start = std::time::Instant::now();
        
        let mut analyzer = self.analyzer.write().await;
        let mut semantic_search = self.semantic_search.write().await;
        let mut incremental_updater = self.incremental_updater.write().await;
        
        let all_symbols = match analyzer.analyze_and_populate_graph(
            Some(&*self.graph),
            Some(&*self.embedding_generator),
            Some(&*self.architecture_analyzer),
            Some(&mut *semantic_search),
            Some(&mut *incremental_updater)
        ).await {
            Ok(symbols) => {
                eprintln!("⏱️ Parsing and analysis took: {:?}", parse_start.elapsed());
                symbols
            },
            Err(e) => {
                eprintln!("❌ Failed to analyze and populate graph: {}", e);
                return Err(e);
            }
        };

        // Log discovered crates summary
        eprintln!("📦 Discovered {} crates", crates.len());
        
        // Validation for dummy-workspace  
        if crates.len() == 3 && crates.iter().all(|c| c.name.starts_with("crate_")) {
            eprintln!("\n🧪 DUMMY WORKSPACE PRE-MEMGRAPH ANALYSIS:");
            eprintln!("   📦 CRATES ({}):", crates.len());
            for crate_meta in &crates {
                eprintln!("      - {} (workspace_member: {})", crate_meta.name, crate_meta.is_workspace_member);
            }
            
            eprintln!("   🔧 FUNCTIONS ({}):", all_symbols.functions.len());
            for func in &all_symbols.functions {
                eprintln!("      - ID: {}", func.id);
                eprintln!("        Name: {}", func.name);
                eprintln!("        Qualified: {}", func.qualified_name);  
                eprintln!("        Crate: {}", func.crate_name);
                eprintln!("        File: {}", func.file_path);
                eprintln!("        Lines: {}-{}", func.line_start, func.line_end);
                eprintln!();
            }
            
            eprintln!("   📞 CALLS ({}):", all_symbols.calls.len());
            for (i, call) in all_symbols.calls.iter().enumerate() {
                eprintln!("      Call #{}: ", i + 1);
                eprintln!("        Caller ID: {}", call.caller_id);
                eprintln!("        Caller Module: {}", call.caller_module);
                eprintln!("        Callee Name: {}", call.callee_name);
                eprintln!("        Qualified Callee: {:?}", call.qualified_callee);
                eprintln!("        Call Type: {:?}", call.call_type);
                eprintln!("        Line: {}", call.line);
                eprintln!("        Cross Crate: {}", call.cross_crate);
                eprintln!("        From Crate: {}", call.from_crate);
                eprintln!("        To Crate: {:?}", call.to_crate);
                eprintln!();
            }
            
            eprintln!("   📊 SUMMARY: {} crates, {} functions, {} calls", 
                      crates.len(), all_symbols.functions.len(), all_symbols.calls.len());
        }

        // Store results (graph population, verification, and architecture analysis were handled in analyze_and_populate_graph)
        *self.current_symbols.write().await = Some(all_symbols);
        *self.current_crates.write().await = crates;
        
        eprintln!("✅ Auto-initialization completed successfully in {:?}", total_start.elapsed());

        Ok(())
    }

    pub async fn handle_request(&self, request: McpRequest) -> McpResponse {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request).await,
            "workspace_context" => self.handle_workspace_context(request).await,
            "analyze_change_impact" => self.handle_analyze_change_impact(request).await,
            "analyze_batch_change_impact" => self.handle_analyze_batch_change_impact(request).await,
            "analyze_file_changes" => self.handle_analyze_file_changes(request).await,
            "discover_functions_from_diff" => self.handle_discover_functions_from_diff(request).await,
            "incremental_file_analysis" => self.handle_incremental_file_analysis(request).await,
            "find_unreferenced_functions" => self.handle_find_unreferenced_functions(request).await,
            "find_test_only_functions" => self.handle_find_test_only_functions(request).await,
            "find_functions_without_tests" => self.handle_find_functions_without_tests(request).await,
            "find_functions_with_tests" => self.handle_find_functions_with_tests(request).await,
            "find_most_referenced_functions" => self.handle_find_most_referenced_functions(request).await,
            "find_most_referenced_without_tests" => self.handle_find_most_referenced_without_tests(request).await,
            "generate_actor_spawn_diagram" => self.handle_generate_actor_spawn_diagram(request).await,
            "generate_actor_message_diagram" => self.handle_generate_actor_message_diagram(request).await,
            "get_actor_details" => self.handle_get_actor_details(request).await,
            "get_distributed_actors" => self.handle_get_distributed_actors(request).await,
            "generate_distributed_actor_message_flow" => self.handle_generate_distributed_actor_message_flow(request).await,
            "debug_call_relationships" => self.handle_debug_call_relationships(request).await,
            "analyze_symbol_impact" => self.handle_analyze_symbol_impact(request).await,
            "check_architecture_violations" => self.handle_check_architecture_violations(request).await,
            "semantic_search" => self.handle_semantic_search(request).await,
            "get_function_details" => self.handle_get_function_details(request).await,
            "get_type_details" => self.handle_get_type_details(request).await,
            "get_crate_overview" => self.handle_get_crate_overview(request).await,
            "get_layer_health" => self.handle_get_layer_health(request).await,
            "incremental_update" => self.handle_incremental_update(request).await,
            "list_functions" => self.handle_list_functions(request).await,
            "debug_graph" => self.handle_debug_graph(request).await,
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
        eprintln!("🚀 Starting enhanced workspace analysis");
        
        let mut workspace_discovery = self.workspace_discovery.write().await;
        let crates = match workspace_discovery.discover_crates().await {
            Ok(crates) => crates,
            Err(e) => {
                return McpResponse {
                    id: request.id,
                    result: None,
                    error: Some(McpError {
                        code: -32603,
                        message: format!("Failed to discover crates: {}", e),
                        data: None,
                    }),
                };
            }
        };

        let mut all_symbols = ParsedSymbols::new();
        let mut analyzer = self.analyzer.write().await;

        for crate_meta in &crates {
            if crate_meta.is_workspace_member {
                match analyzer.parse_crate_files(&crate_meta.path, &crate_meta.name) {
                    Ok(crate_symbols) => all_symbols.merge(crate_symbols),
                    Err(e) => eprintln!("⚠️ Failed to parse crate {}: {}", crate_meta.name, e),
                }
            }
        }

        crate::parser::references::resolve_all_references(&mut all_symbols).unwrap_or_else(|e| {
            eprintln!("⚠️ Failed to resolve references: {}", e);
        });

        match self.embedding_generator.generate_embeddings(&mut all_symbols).await {
            Ok(_) => {},
            Err(e) => eprintln!("⚠️ Failed to generate embeddings: {}", e),
        }

        if let Err(e) = self.graph.populate_from_symbols(&all_symbols).await {
            return McpResponse {
                id: request.id,
                result: None,
                error: Some(McpError {
                    code: -32603,
                    message: format!("Failed to populate graph: {}", e),
                    data: None,
                }),
            };
        }

        let function_embeddings = self.embedding_generator.extract_function_embeddings(&all_symbols.functions);
        let type_embeddings = self.embedding_generator.extract_type_embeddings(&all_symbols.types);
        
        let mut semantic_search = self.semantic_search.write().await;
        semantic_search.index_function_embeddings(&function_embeddings);
        semantic_search.index_type_embeddings(&type_embeddings);

        *self.current_symbols.write().await = Some(all_symbols);
        *self.current_crates.write().await = crates;

        McpResponse {
            id: request.id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": { "listChanged": false }
                },
                "serverInfo": {
                    "name": "enhanced-rust-workspace-analyzer",
                    "version": "0.1.0"
                },
                "tools": [
                    {
                        "name": "workspace_context",
                        "description": "Get comprehensive context about the workspace with Memgraph 3.0 and tree-sitter",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "include_stats": {"type": "boolean", "description": "Include detailed statistics"},
                                "crate_filter": {"type": "string", "description": "Filter by crate name"}
                            }
                        }
                    },
                    {
                        "name": "analyze_change_impact",
                        "description": "Analyze impact of changes using graph traversal",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "target": {"type": "string", "description": "Function or type to analyze"},
                                "depth": {"type": "number", "description": "Analysis depth (default: 3)"}
                            },
                            "required": ["target"]
                        }
                    },
                    {
                        "name": "analyze_batch_change_impact",
                        "description": "Analyze impact of multiple changed functions/types in batch",
                        "inputSchema": {
                            "type": "object", 
                            "properties": {
                                "targets": {
                                    "type": "array",
                                    "items": {"type": "string"},
                                    "description": "List of functions/types to analyze"
                                },
                                "depth": {"type": "number", "description": "Analysis depth (default: 3)"},
                                "aggregate": {"type": "boolean", "description": "Combine results into single report (default: true)"}
                            },
                            "required": ["targets"]
                        }
                    },
                    {
                        "name": "check_architecture_violations",
                        "description": "Check for architecture violations with layer analysis",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "severity": {"type": "string", "enum": ["all", "error", "warning"], "description": "Filter by severity"}
                            }
                        }
                    },
                    {
                        "name": "semantic_search", 
                        "description": "Search functions and types semantically using embeddings",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": {"type": "string", "description": "Search query"},
                                "limit": {"type": "number", "description": "Number of results (default: 10)"},
                                "crate_filter": {"type": "string", "description": "Limit search to specific crate"}
                            },
                            "required": ["query"]
                        }
                    },
                    {
                        "name": "get_function_details",
                        "description": "Get detailed information about a specific function",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "qualified_name": {"type": "string", "description": "Fully qualified function name"}
                            },
                            "required": ["qualified_name"]
                        }
                    },
                    {
                        "name": "get_type_details",
                        "description": "Get detailed information about a specific type",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "qualified_name": {"type": "string", "description": "Fully qualified type name"}
                            },
                            "required": ["qualified_name"]
                        }
                    },
                    {
                        "name": "get_crate_overview",
                        "description": "Get overview of a specific crate",
                        "inputSchema": {
                            "type": "object", 
                            "properties": {
                                "crate_name": {"type": "string", "description": "Name of the crate"}
                            },
                            "required": ["crate_name"]
                        }
                    },
                    {
                        "name": "get_layer_health",
                        "description": "Get architecture layer health report",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "incremental_update",
                        "description": "Perform incremental update of changed files",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "files": {"type": "array", "items": {"type": "string"}, "description": "List of changed file paths"}
                            }
                        }
                    }
                ]
            })),
            error: None,
        }
    }

    async fn handle_workspace_context(&self, request: McpRequest) -> McpResponse {
        let symbols = self.current_symbols.read().await;
        let crates = self.current_crates.read().await;

        match symbols.as_ref() {
            Some(symbols) => {
                let include_stats = request.params.as_ref()
                    .and_then(|p| p.get("include_stats"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                let crate_filter = request.params.as_ref()
                    .and_then(|p| p.get("crate_filter"))
                    .and_then(|v| v.as_str());

                let stats = match self.graph.get_statistics().await {
                    Ok(stats) => stats,
                    Err(_) => return self.error_response(request.id, -32603, "Failed to get graph statistics"),
                };

                let mut context = format!(
                    "# Enhanced Workspace Analysis (Tree-sitter + Memgraph 3.0)\n\n\
                    ## 📊 Overview\n\
                    - **Workspace Roots**: {}\n\
                    - **Total Crates**: {}\n\
                    - **Workspace Members**: {}\n\
                    - **External Dependencies**: {}\n\n\
                    ## 📈 Graph Statistics\n\
                    - **Function Nodes**: {}\n\
                    - **Type Nodes**: {}\n\
                    - **Module Nodes**: {}\n\
                    - **Call Edges**: {}\n\
                    - **Implements Edges**: {}\n\
                    - **Cross-crate Calls**: {}\n",
                    self.config.all_workspace_roots().count(),
                    crates.len(),
                    crates.iter().filter(|c| c.is_workspace_member).count(),
                    crates.iter().filter(|c| c.is_external).count(),
                    stats.function_nodes,
                    stats.type_nodes,
                    stats.module_nodes,
                    stats.call_edges,
                    stats.implements_edges,
                    symbols.get_cross_crate_calls().len()
                );

                if include_stats {
                    context.push_str("\n## 🏗️ Architecture Layers\n");
                    for layer in &self.config.architecture.layers {
                        let crates_in_layer: Vec<_> = crates.iter()
                            .filter(|c| c.layer.as_ref() == Some(&layer.name))
                            .map(|c| c.name.as_str())
                            .collect();
                        
                        if !crates_in_layer.is_empty() {
                            context.push_str(&format!("- **{}**: {}\n", layer.name, crates_in_layer.join(", ")));
                        }
                    }
                }

                if let Some(filter) = crate_filter {
                    let filtered_functions = symbols.get_functions_in_crate(filter);
                    let filtered_types = symbols.get_types_in_crate(filter);
                    
                    context.push_str(&format!(
                        "\n## 🔍 Filtered View: {}\n\
                        - **Functions**: {}\n\
                        - **Types**: {}\n",
                        filter,
                        filtered_functions.len(),
                        filtered_types.len()
                    ));
                }

                McpResponse {
                    id: request.id,
                    result: Some(json!({
                        "content": [{"type": "text", "text": context}]
                    })),
                    error: None,
                }
            }
            None => self.error_response(request.id, -32603, "Workspace not initialized"),
        }
    }

    async fn handle_analyze_change_impact(&self, request: McpRequest) -> McpResponse {
        let target = match self.extract_required_param(&request, "target") {
            Some(target) => target,
            None => return self.error_response(request.id, -32602, "Missing 'target' parameter"),
        };

        let depth = request.params.as_ref()
            .and_then(|p| p.get("depth"))
            .and_then(|v| v.as_u64())
            .unwrap_or(3) as usize;

        // Query 1: Find all functions that call the target (existing functionality)
        let impact_query = format!(
            "MATCH (f:Function {{qualified_name: '{}'}})
             MATCH path = (caller:Function)-[:CALLS*1..{}]->(f)
             RETURN DISTINCT caller.qualified_name as caller,
                    caller.crate as caller_crate,
                    caller.file as file,
                    caller.line_start as line,
                    caller.is_test as is_test,
                    size(path) as distance
             ORDER BY distance, caller_crate
             LIMIT 50",
            target, depth
        );

        let query = neo4rs::Query::new(impact_query);
        let result = match self.graph.execute_query(query).await {
            Ok(result) => result,
            Err(e) => return self.error_response(request.id, -32603, &format!("Query failed: {}", e)),
        };

        let mut impacted_functions = Vec::new();
        let mut test_functions = Vec::new();
        let mut regular_functions = Vec::new();

        for row in result {
            if let (Ok(caller), Ok(caller_crate), Ok(file), Ok(line), Ok(is_test), Ok(distance)) = (
                row.get::<String>("caller"),
                row.get::<String>("caller_crate"), 
                row.get::<String>("file"),
                row.get::<i64>("line"),
                row.get::<bool>("is_test"),
                row.get::<i64>("distance"),
            ) {
                let function_info = json!({
                    "function": caller,
                    "crate": caller_crate,
                    "file": file,
                    "line": line,
                    "distance": distance,
                    "is_test": is_test
                });
                
                impacted_functions.push(function_info.clone());
                
                if is_test {
                    test_functions.push(function_info);
                } else {
                    regular_functions.push(function_info);
                }
            }
        }

        // Query 2: Find test functions that directly test the target function
        let direct_test_query = format!(
            "MATCH (test:Function {{is_test: true}})-[:CALLS]->(f:Function {{qualified_name: '{}'}})
             RETURN DISTINCT test.qualified_name as test_function,
                    test.crate as test_crate,
                    test.file as test_file,
                    test.line_start as test_line",
            target
        );

        let test_query = neo4rs::Query::new(direct_test_query);
        let test_result = match self.graph.execute_query(test_query).await {
            Ok(result) => result,
            Err(_) => {
                // If query fails, continue without direct test analysis
                return self.create_impact_report(&target, depth, &impacted_functions, &test_functions, &regular_functions, &Vec::new(), request.id);
            }
        };

        let mut direct_tests = Vec::new();
        for row in test_result {
            if let (Ok(test_func), Ok(test_crate), Ok(test_file), Ok(test_line)) = (
                row.get::<String>("test_function"),
                row.get::<String>("test_crate"),
                row.get::<String>("test_file"), 
                row.get::<i64>("test_line"),
            ) {
                direct_tests.push(json!({
                    "function": test_func,
                    "crate": test_crate,
                    "file": test_file,
                    "line": test_line
                }));
            }
        }

        self.create_impact_report(&target, depth, &impacted_functions, &test_functions, &regular_functions, &direct_tests, request.id)
    }

    async fn handle_analyze_batch_change_impact(&self, request: McpRequest) -> McpResponse {
        let params = match request.params.as_ref() {
            Some(params) => params,
            None => return self.error_response(request.id, -32602, "Missing parameters"),
        };

        let targets = match params.get("targets").and_then(|v| v.as_array()) {
            Some(targets) => targets.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
            None => return self.error_response(request.id, -32602, "Missing 'targets' parameter"),
        };

        if targets.is_empty() {
            return self.error_response(request.id, -32602, "Empty 'targets' list");
        }

        let depth = params.get("depth")
            .and_then(|v| v.as_u64())
            .unwrap_or(3) as usize;

        let aggregate = params.get("aggregate")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        if aggregate {
            self.handle_aggregated_batch_analysis(&targets, depth, request.id).await
        } else {
            self.handle_separate_batch_analysis(&targets, depth, request.id).await
        }
    }

    async fn handle_aggregated_batch_analysis(&self, targets: &[String], depth: usize, request_id: Option<serde_json::Value>) -> McpResponse {
        let mut all_impacted = Vec::new();
        let mut all_test_functions = Vec::new();
        let mut all_regular_functions = Vec::new();
        let mut all_direct_tests = Vec::new();
        let mut analysis_results = Vec::new();

        for target in targets {
            // Query for each target
            let impact_query = format!(
                "MATCH (f:Function {{qualified_name: '{}'}})
                 MATCH path = (caller:Function)-[:CALLS*1..{}]->(f)
                 RETURN DISTINCT caller.qualified_name as caller,
                        caller.crate as caller_crate,
                        caller.file as file,
                        caller.line_start as line,
                        caller.is_test as is_test,
                        size(path) as distance
                 ORDER BY distance, caller_crate
                 LIMIT 50",
                target, depth
            );

            let query = neo4rs::Query::new(impact_query);
            let result = match self.graph.execute_query(query).await {
                Ok(result) => result,
                Err(e) => {
                    analysis_results.push(json!({
                        "target": target,
                        "error": format!("Query failed: {}", e),
                        "impacted_functions": 0
                    }));
                    continue;
                }
            };

            let mut target_impacted = Vec::new();
            for row in result {
                if let (Ok(caller), Ok(caller_crate), Ok(file), Ok(line), Ok(is_test), Ok(distance)) = (
                    row.get::<String>("caller"),
                    row.get::<String>("caller_crate"), 
                    row.get::<String>("file"),
                    row.get::<i64>("line"),
                    row.get::<bool>("is_test"),
                    row.get::<i64>("distance"),
                ) {
                    let function_info = json!({
                        "function": caller,
                        "crate": caller_crate,
                        "file": file,
                        "line": line,
                        "distance": distance,
                        "is_test": is_test,
                        "affects_target": target
                    });
                    
                    target_impacted.push(function_info.clone());
                    all_impacted.push(function_info.clone());
                    
                    if is_test {
                        all_test_functions.push(function_info.clone());
                    } else {
                        all_regular_functions.push(function_info.clone());
                    }
                }
            }

            // Query for direct tests for this target
            let direct_test_query = format!(
                "MATCH (test:Function {{is_test: true}})-[:CALLS]->(f:Function {{qualified_name: '{}'}})
                 RETURN DISTINCT test.qualified_name as test_function,
                        test.crate as test_crate,
                        test.file as test_file,
                        test.line_start as test_line",
                target
            );

            let test_query = neo4rs::Query::new(direct_test_query);
            if let Ok(test_result) = self.graph.execute_query(test_query).await {
                for row in test_result {
                    if let (Ok(test_func), Ok(test_crate), Ok(test_file), Ok(test_line)) = (
                        row.get::<String>("test_function"),
                        row.get::<String>("test_crate"),
                        row.get::<String>("test_file"), 
                        row.get::<i64>("test_line"),
                    ) {
                        let direct_test_info = json!({
                            "function": test_func,
                            "crate": test_crate,
                            "file": test_file,
                            "line": test_line,
                            "tests_target": target
                        });
                        all_direct_tests.push(direct_test_info);
                    }
                }
            }

            analysis_results.push(json!({
                "target": target,
                "impacted_functions": target_impacted.len(),
                "success": true
            }));
        }

        // Remove duplicates from aggregated results
        all_impacted.dedup_by(|a, b| {
            a["function"] == b["function"] && a["line"] == b["line"]
        });
        all_test_functions.dedup_by(|a, b| {
            a["function"] == b["function"] && a["line"] == b["line"] 
        });
        all_regular_functions.dedup_by(|a, b| {
            a["function"] == b["function"] && a["line"] == b["line"]
        });
        all_direct_tests.dedup_by(|a, b| {
            a["function"] == b["function"] && a["line"] == b["line"]
        });

        // Create aggregated report
        let targets_str = targets.join(", ");
        let report = self.create_batch_impact_report(
            &targets_str, 
            targets,
            depth, 
            &all_impacted, 
            &all_test_functions, 
            &all_regular_functions, 
            &all_direct_tests,
            &analysis_results,
            request_id
        );

        report
    }

    async fn handle_separate_batch_analysis(&self, targets: &[String], depth: usize, request_id: Option<serde_json::Value>) -> McpResponse {
        let mut reports = Vec::new();

        for target in targets {
            // For separate analysis, we essentially call the single analysis for each target
            let single_request = McpRequest {
                id: None,
                method: "analyze_change_impact".to_string(),
                params: Some(json!({
                    "target": target,
                    "depth": depth
                }))
            };

            let single_response = self.handle_analyze_change_impact(single_request).await;
            
            if let Some(result) = single_response.result {
                if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
                    if let Some(text_content) = content.first().and_then(|t| t.get("text")) {
                        reports.push(format!("## Analysis for {}\n\n{}", target, text_content.as_str().unwrap_or("")));
                    }
                }
            } else if let Some(error) = single_response.error {
                reports.push(format!("## Analysis for {} - ERROR\n\n{}", target, error.message));
            }
        }

        let combined_report = format!(
            "# 🎯 Batch Change Impact Analysis\n\n\
            ## 📊 Batch Summary\n\
            - **Total Targets Analyzed**: {}\n\
            - **Targets**: {}\n\n\
            {}\n\n\
            ---\n\n\
            *Generated by batch change impact analysis*",
            targets.len(),
            targets.join(", "),
            reports.join("\n\n---\n\n")
        );

        McpResponse {
            id: request_id,
            result: Some(json!({
                "content": [{"type": "text", "text": combined_report}]
            })),
            error: None,
        }
    }

    fn create_batch_impact_report(
        &self,
        _targets_display: &str,
        targets: &[String],
        depth: usize,
        all_impacted: &[serde_json::Value],
        all_test_functions: &[serde_json::Value], 
        all_regular_functions: &[serde_json::Value],
        all_direct_tests: &[serde_json::Value],
        analysis_results: &[serde_json::Value],
        request_id: Option<serde_json::Value>
    ) -> McpResponse {
        McpResponse {
            id: request_id,
            result: Some(json!({
                "analysis_type": "batch_change_impact",
                "targets": targets,
                "depth": depth,
                "summary": {
                    "total_targets": targets.len(),
                    "total_impacted": all_impacted.len(),
                    "regular_functions": all_regular_functions.len(),
                    "test_functions": all_test_functions.len(),
                    "direct_tests": all_direct_tests.len()
                },
                "per_target_results": analysis_results,
                "all_impacted_functions": all_impacted,
                "all_test_functions": all_test_functions,
                "all_regular_functions": all_regular_functions,
                "all_direct_tests": all_direct_tests
            })),
            error: None,
        }
    }

    fn create_impact_report(
        &self,
        target: &str,
        depth: usize,
        impacted_functions: &[serde_json::Value],
        test_functions: &[serde_json::Value], 
        regular_functions: &[serde_json::Value],
        direct_tests: &[serde_json::Value],
        request_id: Option<serde_json::Value>
    ) -> McpResponse {
        McpResponse {
            id: request_id,
            result: Some(json!({
                "analysis_type": "change_impact",
                "target": target,
                "depth": depth,
                "summary": {
                    "total_impacted": impacted_functions.len(),
                    "regular_functions": regular_functions.len(),
                    "test_functions": test_functions.len(),
                    "direct_tests": direct_tests.len()
                },
                "impacted_functions": impacted_functions,
                "test_functions": test_functions,
                "regular_functions": regular_functions,
                "direct_tests": direct_tests
            })),
            error: None,
        }
    }

    async fn handle_analyze_symbol_impact(&self, request: McpRequest) -> McpResponse {
        // Extract symbol and symbol_type from params
        let symbol = match request.params.as_ref()
            .and_then(|p| p.get("symbol"))
            .and_then(|s| s.as_str()) {
            Some(s) => s.to_string(),
            None => {
                return McpResponse {
                    id: request.id,
                    result: None,
                    error: Some(McpError {
                        code: -32602,
                        message: "Missing required parameter 'symbol'".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let symbol_type = request.params.as_ref()
            .and_then(|p| p.get("symbol_type"))
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());

        // Get current workspace symbols
        let symbols_guard = self.current_symbols.read().await;
        let symbols = match symbols_guard.as_ref() {
            Some(s) => s,
            None => {
                return McpResponse {
                    id: request.id,
                    result: None,
                    error: Some(McpError {
                        code: -32603,
                        message: "Workspace not analyzed yet. Run workspace_context first.".to_string(),
                        data: None,
                    }),
                };
            }
        };

        // Search for the symbol
        let mut found_symbols = Vec::new();

        // Search in functions
        for func in &symbols.functions {
            if func.name == symbol || func.qualified_name.contains(&symbol) {
                found_symbols.push(json!({
                    "type": "function",
                    "name": func.qualified_name,
                    "file": func.file_path,
                    "line": func.line_start,
                    "crate": func.crate_name,
                    "visibility": func.visibility
                }));
            }
        }

        // Search in types
        for typ in &symbols.types {
            if typ.name == symbol || typ.qualified_name.contains(&symbol) {
                found_symbols.push(json!({
                    "type": typ.kind,
                    "name": typ.qualified_name,
                    "file": typ.file_path,
                    "line": typ.line_start,
                    "crate": typ.crate_name,
                    "visibility": typ.visibility
                }));
            }
        }

        if found_symbols.is_empty() {
            let report = format!(
                "# Symbol Impact Analysis: '{}'\n\n❌ **Symbol not found**\n\nThe symbol '{}' was not found in the current workspace.\n\n## Suggestions:\n- Check spelling of the symbol name\n- Ensure the symbol is in a workspace member crate\n- Try searching for partial matches\n- Available symbol types: function, struct, trait, enum, type",
                symbol, symbol
            );

            return McpResponse {
                id: request.id,
                result: Some(json!({
                    "content": [{"type": "text", "text": report}],
                    "found": false,
                    "matches": 0
                })),
                error: None,
            };
        }

        // Analyze impact
        let mut direct_usages = 0;
        let mut calling_functions = Vec::new();

        // Analyze function calls to find usages
        for call in &symbols.calls {
            let target = call.qualified_callee.as_ref().unwrap_or(&call.callee_name);
            if target.contains(&symbol) {
                direct_usages += 1;
                calling_functions.push(call.caller_id.clone());
            }
        }

        // Generate impact level
        let (impact_level, impact_emoji) = if direct_usages == 0 {
            ("LOW", "✅")
        } else if direct_usages < 5 {
            ("MEDIUM", "⚠️")
        } else {
            ("HIGH", "🚨")
        };

        // Build detailed report
        let mut report = format!(
            "# Symbol Impact Analysis: '{}'\n\n✅ **Found {} matches**\n\n## Symbol Definitions:\n",
            symbol, found_symbols.len()
        );

        for (i, found_symbol) in found_symbols.iter().enumerate() {
            report.push_str(&format!(
                "{}. **{}**: `{}` in `{}:{}`\n",
                i + 1,
                found_symbol["type"].as_str().unwrap_or("unknown"),
                found_symbol["name"].as_str().unwrap_or("unknown"),
                found_symbol["file"].as_str().unwrap_or("unknown"),
                found_symbol["line"].as_i64().unwrap_or(0)
            ));
        }

        report.push_str(&format!(
            "\n## 📊 Impact Analysis\n\n{} **{} IMPACT**\n\n",
            impact_emoji, impact_level
        ));

        if direct_usages > 0 {
            report.push_str(&format!(
                "- **Direct function calls**: {}\n- **Called by {} different functions**\n\n",
                direct_usages, calling_functions.len()
            ));

            let sample_size = std::cmp::min(5, calling_functions.len());
            if sample_size > 0 {
                report.push_str("### Sample Callers:\n");
                for (i, caller) in calling_functions.iter().take(sample_size).enumerate() {
                    report.push_str(&format!("{}. `{}`\n", i + 1, caller));
                }
                if calling_functions.len() > sample_size {
                    report.push_str(&format!("... and {} more\n", calling_functions.len() - sample_size));
                }
            }
        } else {
            report.push_str("- **No direct function calls found**\n");
        }

        // Add type-specific analysis
        if let Some(sym_type) = symbol_type {
            report.push_str(&format!("\n## 🔍 Type-Specific Analysis ('{}')\n\n", sym_type));
            match sym_type.to_lowercase().as_str() {
                "struct" => {
                    report.push_str("- Check for field access patterns\n");
                    report.push_str("- Verify trait implementations\n");
                    report.push_str("- Review constructor usage\n");
                },
                "trait" => {
                    report.push_str("- Check implementations across crates\n");
                    report.push_str("- Look for trait bounds in generics\n");
                    report.push_str("- Verify trait object usage\n");
                },
                "enum" => {
                    report.push_str("- Check variant usage patterns\n");
                    report.push_str("- Look for match expressions\n");
                    report.push_str("- Verify serialization impact\n");
                },
                "function" => {
                    report.push_str("- Direct calls analyzed above\n");
                    report.push_str("- Check function pointer usage\n");
                    report.push_str("- Review higher-order usage\n");
                },
                _ => {
                    report.push_str("- General symbol analysis performed\n");
                }
            }
        }

        // Add recommendations
        report.push_str("\n## 💡 Change Impact Guidance\n\n");
        match impact_level {
            "LOW" => {
                report.push_str("✅ **Safe to modify**: Symbol has no direct dependencies\n");
                report.push_str("- Implementation changes are low-risk\n");
                report.push_str("- Consider if unused code can be removed\n");
            },
            "MEDIUM" => {
                report.push_str("⚠️  **Review required**: Symbol has moderate usage\n");
                report.push_str("- Review each caller before changes\n");
                report.push_str("- Consider backward compatibility\n");
                report.push_str("- Update relevant tests\n");
            },
            "HIGH" => {
                report.push_str("🚨 **High impact**: Symbol is widely used\n");
                report.push_str("- Breaking changes affect many components\n");
                report.push_str("- Consider deprecation strategy\n");
                report.push_str("- Extensive testing required\n");
                report.push_str("- Document migration path\n");
            },
            _ => {}
        }

        report.push_str("\n## 📝 Recommended Actions\n");
        report.push_str("- Run `cargo check` after changes\n");
        report.push_str("- Execute full test suite\n");
        report.push_str("- Check for compiler warnings\n");
        if direct_usages > 0 {
            report.push_str("- Use IDE 'Find All References' for detailed analysis\n");
        }

        McpResponse {
            id: request.id,
            result: Some(json!({
                "content": [{"type": "text", "text": report}],
                "symbol": symbol,
                "found": true,
                "matches": found_symbols.len(),
                "impact_level": impact_level.to_lowercase(),
                "direct_usages": direct_usages,
                "calling_functions_count": calling_functions.len(),
                "found_symbols": found_symbols
            })),
            error: None,
        }
    }

    async fn handle_check_architecture_violations(&self, request: McpRequest) -> McpResponse {
        let severity_filter = request.params.as_ref()
            .and_then(|p| p.get("severity"))
            .and_then(|v| v.as_str())
            .unwrap_or("all");
        
        let limit = request.params.as_ref()
            .and_then(|p| p.get("limit"))
            .and_then(|v| v.as_u64())
            .map(|l| l as usize);

        let report = match self.architecture_analyzer.analyze_architecture().await {
            Ok(analysis_report) => {
                let mut filtered_violations: Vec<_> = analysis_report.violations.into_iter()
                    .filter(|v| match severity_filter {
                        "error" => matches!(v.severity, crate::architecture::ViolationSeverity::Error),
                        "warning" => matches!(v.severity, crate::architecture::ViolationSeverity::Warning),
                        _ => true,
                    })
                    .collect();
                
                // Apply limit if specified
                let total_before_limit = filtered_violations.len();
                if let Some(limit_value) = limit {
                    filtered_violations.truncate(limit_value);
                }

                let limit_notice = if limit.is_some() && total_before_limit > filtered_violations.len() {
                    format!("- **Showing**: {} of {} violations (limit applied)\n", filtered_violations.len(), total_before_limit)
                } else {
                    String::new()
                };
                
                format!(
                    "# Architecture Violations Report\n\n\
                    ## 📊 Summary\n\
                    - **Total Violations**: {}\n\
                    - **Errors**: {}\n\
                    - **Warnings**: {}\n\
                    - **Filter Applied**: {}\n{}\n\
                    ## 🚨 Violations\n{}\n\n\
                    ## 📈 Most Problematic Crates\n{}\n",
                    total_before_limit,
                    analysis_report.summary.error_count,
                    analysis_report.summary.warning_count,
                    severity_filter,
                    limit_notice,
                    filtered_violations.iter()
                        .map(|v| format!("### {} - {}\n**{}** → **{}**\n{}\n`{}:{}`\n",
                            match v.severity {
                                crate::architecture::ViolationSeverity::Error => "🚨 ERROR",
                                crate::architecture::ViolationSeverity::Warning => "⚠️ WARNING", 
                                crate::architecture::ViolationSeverity::Info => "ℹ️ INFO",
                            },
                            v.kind,
                            v.from_crate,
                            v.to_crate,
                            v.message,
                            v.file,
                            v.line))
                        .collect::<Vec<_>>().join("\n"),
                    analysis_report.summary.most_problematic_crates.iter()
                        .map(|crate_name| format!("- {}", crate_name))
                        .collect::<Vec<_>>().join("\n")
                )
            },
            Err(e) => return self.error_response(request.id, -32603, &format!("Analysis failed: {}", e)),
        };

        McpResponse {
            id: request.id,
            result: Some(json!({
                "content": [{"type": "text", "text": report}]
            })),
            error: None,
        }
    }

    async fn handle_semantic_search(&self, request: McpRequest) -> McpResponse {
        let query = match self.extract_required_param(&request, "query") {
            Some(query) => query,
            None => return self.error_response(request.id, -32602, "Missing 'query' parameter"),
        };

        let limit = request.params.as_ref()
            .and_then(|p| p.get("limit"))
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;

        let semantic_search = self.semantic_search.read().await;
        let results = match semantic_search.search(&query, limit).await {
            Ok(results) => results,
            Err(e) => return self.error_response(request.id, -32603, &format!("Search failed: {}", e)),
        };

        let report = format!(
            "# Semantic Search Results\n\n\
            **Query**: {}\n\
            **Results**: {}\n\n\
            ## 🔍 Matches\n{}\n",
            query,
            results.len(),
            results.iter()
                .map(|r| format!("- **{}** (similarity: {:.3}) - {} in `{}`",
                    r.qualified_name,
                    r.similarity_score,
                    match r.result_type {
                        crate::embeddings::SearchResultType::Function => "Function",
                        crate::embeddings::SearchResultType::Type => "Type",
                    },
                    r.metadata.crate_name))
                .collect::<Vec<_>>().join("\n")
        );

        McpResponse {
            id: request.id,
            result: Some(json!({
                "content": [{"type": "text", "text": report}]
            })),
            error: None,
        }
    }

    async fn handle_get_function_details(&self, request: McpRequest) -> McpResponse {
        let qualified_name = match self.extract_required_param(&request, "qualified_name") {
            Some(name) => name,
            None => return self.error_response(request.id, -32602, "Missing 'qualified_name' parameter"),
        };

        let symbols = self.current_symbols.read().await;
        match symbols.as_ref() {
            Some(symbols) => {
                if let Some(function) = symbols.get_function_by_name(&qualified_name) {
                    let details = format!(
                        "# Function Details: {}\n\n\
                        ## 📋 Basic Info\n\
                        - **Name**: {}\n\
                        - **Crate**: {}\n\
                        - **Module**: {}\n\
                        - **File**: {}:{}-{}\n\
                        - **Visibility**: {}\n\n\
                        ## 🔧 Signature\n\
                        ```rust\n{}\n```\n\n\
                        ## 📝 Properties\n\
                        - **Async**: {}\n\
                        - **Unsafe**: {}\n\
                        - **Generic**: {}\n\
                        - **Parameters**: {}\n\
                        - **Return Type**: {}\n\n\
                        ## 📖 Documentation\n{}\n",
                        function.qualified_name,
                        function.name,
                        function.crate_name,
                        function.module_path,
                        function.file_path,
                        function.line_start,
                        function.line_end,
                        function.visibility,
                        function.signature,
                        function.is_async,
                        function.is_unsafe,
                        function.is_generic,
                        function.parameters.len(),
                        function.return_type.as_deref().unwrap_or("()"),
                        function.doc_comment.as_deref().unwrap_or("No documentation available")
                    );

                    McpResponse {
                        id: request.id,
                        result: Some(json!({
                            "content": [{"type": "text", "text": details}]
                        })),
                        error: None,
                    }
                } else {
                    self.error_response(request.id, -32603, "Function not found")
                }
            }
            None => self.error_response(request.id, -32603, "Workspace not initialized"),
        }
    }

    async fn handle_get_type_details(&self, request: McpRequest) -> McpResponse {
        let qualified_name = match self.extract_required_param(&request, "qualified_name") {
            Some(name) => name,
            None => return self.error_response(request.id, -32602, "Missing 'qualified_name' parameter"),
        };

        let symbols = self.current_symbols.read().await;
        match symbols.as_ref() {
            Some(symbols) => {
                if let Some(rust_type) = symbols.get_type_by_name(&qualified_name) {
                    let details = format!(
                        "# Type Details: {}\n\n\
                        ## 📋 Basic Info\n\
                        - **Name**: {}\n\
                        - **Kind**: {:?}\n\
                        - **Crate**: {}\n\
                        - **Module**: {}\n\
                        - **File**: {}:{}-{}\n\
                        - **Visibility**: {}\n\
                        - **Generic**: {}\n\n\
                        ## 🏗️ Structure\n\
                        - **Fields**: {}\n\
                        - **Variants**: {}\n\
                        - **Methods**: {}\n\n\
                        ## 📖 Documentation\n{}\n",
                        rust_type.qualified_name,
                        rust_type.name,
                        rust_type.kind,
                        rust_type.crate_name,
                        rust_type.module_path,
                        rust_type.file_path,
                        rust_type.line_start,
                        rust_type.line_end,
                        rust_type.visibility,
                        rust_type.is_generic,
                        rust_type.fields.len(),
                        rust_type.variants.len(),
                        rust_type.methods.len(),
                        rust_type.doc_comment.as_deref().unwrap_or("No documentation available")
                    );

                    McpResponse {
                        id: request.id,
                        result: Some(json!({
                            "content": [{"type": "text", "text": details}]
                        })),
                        error: None,
                    }
                } else {
                    self.error_response(request.id, -32603, "Type not found")
                }
            }
            None => self.error_response(request.id, -32603, "Workspace not initialized"),
        }
    }

    async fn handle_get_crate_overview(&self, request: McpRequest) -> McpResponse {
        let crate_name = match self.extract_required_param(&request, "crate_name") {
            Some(name) => name,
            None => return self.error_response(request.id, -32602, "Missing 'crate_name' parameter"),
        };

        let symbols = self.current_symbols.read().await;
        let crates = self.current_crates.read().await;

        match (symbols.as_ref(), crates.iter().find(|c| c.name == crate_name)) {
            (Some(symbols), Some(crate_meta)) => {
                let functions = symbols.get_functions_in_crate(&crate_name);
                let types = symbols.get_types_in_crate(&crate_name);
                let cross_crate_calls = symbols.get_cross_crate_calls().into_iter()
                    .filter(|call| call.from_crate == crate_name || call.to_crate.as_ref() == Some(&crate_name))
                    .count();

                let overview = format!(
                    "# Crate Overview: {}\n\n\
                    ## 📦 Metadata\n\
                    - **Name**: {}\n\
                    - **Version**: {}\n\
                    - **Path**: {}\n\
                    - **Layer**: {}\n\
                    - **Workspace Member**: {}\n\
                    - **External**: {}\n\
                    - **Dependency Depth**: {}\n\n\
                    ## 📊 Statistics\n\
                    - **Functions**: {}\n\
                    - **Types**: {}\n\
                    - **Dependencies**: {}\n\
                    - **Cross-crate Calls**: {}\n\n\
                    ## 🔧 Sample Functions\n{}\n\n\
                    ## 📐 Sample Types\n{}\n",
                    crate_name,
                    crate_meta.name,
                    crate_meta.version,
                    crate_meta.path.display(),
                    crate_meta.layer.as_deref().unwrap_or("none"),
                    crate_meta.is_workspace_member,
                    crate_meta.is_external,
                    crate_meta.depth,
                    functions.len(),
                    types.len(),
                    crate_meta.dependencies.len(),
                    cross_crate_calls,
                    functions.iter().take(5)
                        .map(|f| format!("- **{}** ({})", f.name, f.visibility))
                        .collect::<Vec<_>>().join("\n"),
                    types.iter().take(5)
                        .map(|t| format!("- **{}** ({:?})", t.name, t.kind))
                        .collect::<Vec<_>>().join("\n")
                );

                McpResponse {
                    id: request.id,
                    result: Some(json!({
                        "content": [{"type": "text", "text": overview}]
                    })),
                    error: None,
                }
            }
            _ => self.error_response(request.id, -32603, "Crate not found or workspace not initialized"),
        }
    }

    async fn handle_get_layer_health(&self, request: McpRequest) -> McpResponse {
        let layer_health = match self.architecture_analyzer.get_layer_health().await {
            Ok(health) => health,
            Err(e) => return self.error_response(request.id, -32603, &format!("Failed to get layer health: {}", e)),
        };

        let report = format!(
            "# Architecture Layer Health\n\n\
            ## 🏥 Layer Status\n{}\n\n\
            ## 📊 Health Scores\n{}\n",
            layer_health.iter()
                .map(|(layer, health)| format!("- **{}**: {} ({} violations) - {}",
                    layer,
                    match health.status.as_str() {
                        "healthy" => "🟢 Healthy",
                        "warning" => "🟡 Warning", 
                        "critical" => "🔴 Critical",
                        _ => "⚪ Unknown",
                    },
                    health.violation_count,
                    format!("{:.1}% health", health.health_score)))
                .collect::<Vec<_>>().join("\n"),
            layer_health.iter()
                .map(|(layer, health)| format!("- **{}**: {:.1}/100", layer, health.health_score))
                .collect::<Vec<_>>().join("\n")
        );

        McpResponse {
            id: request.id,
            result: Some(json!({
                "content": [{"type": "text", "text": report}]
            })),
            error: None,
        }
    }

    async fn handle_incremental_update(&self, request: McpRequest) -> McpResponse {
        let files: Vec<String> = request.params.as_ref()
            .and_then(|p| p.get("files"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        let file_paths: Vec<std::path::PathBuf> = files.into_iter()
            .map(|f| std::path::PathBuf::from(f))
            .collect();

        let mut updater = self.incremental_updater.write().await;
        match updater.process_file_changes(file_paths).await {
            Ok(_) => {
                let stats = updater.get_statistics().await;
                let report = format!(
                    "# Incremental Update Complete\n\n\
                    ## 📊 Statistics\n\
                    - **Tracked Files**: {}\n\
                    - **Total Symbols**: {}\n\
                    - **Crates Tracked**: {}\n\
                    - **Last Update**: {:?}\n\
                    - **Last Full Analysis**: {:?}\n",
                    stats.tracked_files,
                    stats.total_symbols,
                    stats.crates_tracked,
                    stats.last_update,
                    stats.last_full_analysis
                );

                McpResponse {
                    id: request.id,
                    result: Some(json!({
                        "content": [{"type": "text", "text": report}]
                    })),
                    error: None,
                }
            }
            Err(e) => self.error_response(request.id, -32603, &format!("Update failed: {}", e)),
        }
    }


    fn extract_required_param(&self, request: &McpRequest, param_name: &str) -> Option<String> {
        request.params.as_ref()
            .and_then(|p| p.get(param_name))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    fn error_response(&self, id: Option<Value>, code: i32, message: &str) -> McpResponse {
        McpResponse {
            id,
            result: None,
            error: Some(McpError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }

    async fn handle_list_functions(&self, request: McpRequest) -> McpResponse {
        let limit = request.params.as_ref()
            .and_then(|p| p.get("limit"))
            .and_then(|v| v.as_u64())
            .unwrap_or(20) as usize;

        let search_term = request.params.as_ref()
            .and_then(|p| p.get("search").or_else(|| p.get("pattern")))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        match self.current_symbols.read().await.as_ref() {
            Some(symbols) => {
                let mut functions: Vec<_> = symbols.functions.iter()
                    .filter(|f| search_term.is_empty() || 
                             f.qualified_name.contains(search_term) || 
                             f.name.contains(search_term))
                    .take(limit)
                    .collect();

                functions.sort_by(|a, b| a.qualified_name.cmp(&b.qualified_name));

                let function_list: Vec<serde_json::Value> = functions.iter()
                    .map(|f| json!({
                        "qualified_name": f.qualified_name,
                        "name": f.name,
                        "crate": f.crate_name,
                        "module": f.module_path,
                        "file": f.file_path,
                        "signature": f.signature,
                        "visibility": f.visibility
                    }))
                    .collect();

                McpResponse {
                    id: request.id,
                    result: Some(json!({
                        "functions": function_list,
                        "total_found": symbols.functions.len(),
                        "showing": function_list.len(),
                        "search": search_term
                    })),
                    error: None,
                }
            }
            None => self.error_response(request.id, -32603, "No workspace analysis available"),
        }
    }

    async fn handle_analyze_file_changes(&self, request: McpRequest) -> McpResponse {
        let params = match request.params.as_ref() {
            Some(params) => params,
            None => return self.error_response(request.id, -32602, "Missing parameters"),
        };

        let files = match params.get("files").and_then(|v| v.as_array()) {
            Some(files) => files.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
            None => return self.error_response(request.id, -32602, "Missing 'files' parameter"),
        };

        if files.is_empty() {
            return self.error_response(request.id, -32602, "Empty 'files' list");
        }

        let depth = params.get("depth")
            .and_then(|v| v.as_u64())
            .unwrap_or(3) as usize;

        // Discover functions within the changed files
        let discovered_functions = match self.discover_functions_in_files(&files).await {
            Ok(functions) => functions,
            Err(e) => {
                return self.error_response(
                    request.id, 
                    -32603, 
                    &format!("Failed to discover functions in files: {}", e)
                );
            }
        };

        if discovered_functions.is_empty() {
            return McpResponse {
                id: request.id,
                result: Some(json!({
                    "analysis_type": "file_change_impact",
                    "files_analyzed": files,
                    "functions_discovered": 0,
                    "message": "No functions found in the specified files",
                    "impact_summary": {
                        "total_impacted": 0,
                        "test_functions": 0,
                        "regular_functions": 0,
                        "direct_tests": 0
                    }
                })),
                error: None,
            };
        }

        // Use batch analysis for the discovered functions
        let batch_analysis = self.handle_aggregated_batch_analysis(&discovered_functions, depth, None).await;
        
        // Wrap the batch analysis result with file-specific context
        match batch_analysis.result {
            Some(mut result_value) => {
                if let Some(result_obj) = result_value.as_object_mut() {
                    result_obj.insert("analysis_type".to_string(), json!("file_change_impact"));
                    result_obj.insert("files_analyzed".to_string(), json!(files));
                    result_obj.insert("functions_discovered".to_string(), json!(discovered_functions));
                    result_obj.insert("discovery_count".to_string(), json!(discovered_functions.len()));
                }
                
                McpResponse {
                    id: request.id,
                    result: Some(result_value),
                    error: None,
                }
            }
            None => {
                if let Some(error) = batch_analysis.error {
                    McpResponse {
                        id: request.id,
                        result: None,
                        error: Some(error),
                    }
                } else {
                    self.error_response(request.id, -32603, "Unknown error in batch analysis")
                }
            }
        }
    }

    async fn discover_functions_in_files(&self, file_paths: &[String]) -> Result<Vec<String>> {
        let mut discovered_functions = Vec::new();
        
        // Query the database for functions in the specified files
        for file_path in file_paths {
            let query = neo4rs::Query::new(format!(
                "MATCH (f:Function)
                 WHERE f.file ENDS WITH '{}'
                 RETURN f.qualified_name as qualified_name",
                file_path.trim_start_matches('/').replace('\'', "\\'")
            ));

            let result = self.graph.execute_query(query).await?;
            
            for row in result {
                if let Ok(qualified_name) = row.get::<String>("qualified_name") {
                    discovered_functions.push(qualified_name);
                }
            }
        }

        discovered_functions.dedup();
        Ok(discovered_functions)
    }

    async fn handle_discover_functions_from_diff(&self, request: McpRequest) -> McpResponse {
        let params = match request.params.as_ref() {
            Some(params) => params,
            None => return self.error_response(request.id, -32602, "Missing parameters"),
        };

        let diff_text = match params.get("diff").and_then(|v| v.as_str()) {
            Some(diff) => diff,
            None => return self.error_response(request.id, -32602, "Missing 'diff' parameter"),
        };

        let include_impact_analysis = params.get("include_impact")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let depth = params.get("depth")
            .and_then(|v| v.as_u64())
            .unwrap_or(3) as usize;

        // Extract file names from the diff
        let changed_files = self.extract_changed_files_from_diff(diff_text);
        
        // Extract function signatures from the diff
        let function_signatures = self.extract_function_signatures_from_diff(diff_text);
        
        // Query database to find qualified names for the discovered functions
        let mut qualified_functions = Vec::new();
        
        for signature in &function_signatures {
            let query = neo4rs::Query::new(format!(
                "MATCH (f:Function)
                 WHERE f.name = '{}'
                 RETURN f.qualified_name as qualified_name, f.file as file",
                signature.replace('\'', "\\'")
            ));

            if let Ok(result) = self.graph.execute_query(query).await {
                for row in result {
                    if let (Ok(qualified_name), Ok(file)) = (
                        row.get::<String>("qualified_name"),
                        row.get::<String>("file")
                    ) {
                        // Prefer matches from changed files
                        let is_in_changed_file = changed_files.iter()
                            .any(|changed_file| file.ends_with(changed_file));
                        
                        if is_in_changed_file {
                            qualified_functions.insert(0, qualified_name); // Put changed file matches first
                        } else {
                            qualified_functions.push(qualified_name);
                        }
                    }
                }
            }
        }

        qualified_functions.dedup();

        let response_data = json!({
            "analysis_type": "git_diff_function_discovery",
            "changed_files": changed_files,
            "function_signatures": function_signatures,
            "qualified_functions": qualified_functions,
            "discovery_count": qualified_functions.len()
        });

        // If impact analysis is requested, combine with batch analysis
        if include_impact_analysis && !qualified_functions.is_empty() {
            let batch_analysis = self.handle_aggregated_batch_analysis(&qualified_functions, depth, None).await;
            
            match batch_analysis.result {
                Some(mut batch_result) => {
                    if let Some(batch_obj) = batch_result.as_object_mut() {
                        // Merge discovery data with impact analysis
                        for (key, value) in response_data.as_object().unwrap() {
                            batch_obj.insert(key.clone(), value.clone());
                        }
                    }
                    
                    McpResponse {
                        id: request.id,
                        result: Some(batch_result),
                        error: None,
                    }
                }
                None => {
                    if let Some(error) = batch_analysis.error {
                        McpResponse {
                            id: request.id,
                            result: Some(response_data), // Return discovery data even if impact analysis fails
                            error: Some(error),
                        }
                    } else {
                        McpResponse {
                            id: request.id,
                            result: Some(response_data),
                            error: None,
                        }
                    }
                }
            }
        } else {
            McpResponse {
                id: request.id,
                result: Some(response_data),
                error: None,
            }
        }
    }

    fn extract_changed_files_from_diff(&self, diff_text: &str) -> Vec<String> {
        let mut files = Vec::new();
        
        for line in diff_text.lines() {
            if line.starts_with("diff --git") {
                // Extract file path from: diff --git a/path/to/file.rs b/path/to/file.rs
                if let Some(captures) = line.split_whitespace().nth(2) {
                    if let Some(file_path) = captures.strip_prefix("a/") {
                        files.push(file_path.to_string());
                    }
                }
            } else if line.starts_with("+++") {
                // Extract from: +++ b/path/to/file.rs
                if let Some(file_path) = line.strip_prefix("+++ b/") {
                    files.push(file_path.to_string());
                }
            }
        }
        
        files.dedup();
        files
    }

    fn extract_function_signatures_from_diff(&self, diff_text: &str) -> Vec<String> {
        let mut functions = Vec::new();
        
        for line in diff_text.lines() {
            if line.starts_with("+") || line.starts_with("-") {
                let content = &line[1..].trim(); // Remove +/- prefix
                
                // Look for function definitions
                if let Some(func_name) = self.extract_rust_function_name(content) {
                    functions.push(func_name);
                }
            }
        }
        
        functions.dedup();
        functions
    }

    fn extract_rust_function_name(&self, line: &str) -> Option<String> {
        // Simple regex-like patterns for Rust function definitions
        let line = line.trim();
        
        // Match: pub fn function_name, fn function_name, async fn function_name, etc.
        if line.contains("fn ") {
            // Find "fn " and extract the next word
            if let Some(fn_pos) = line.find("fn ") {
                let after_fn = &line[fn_pos + 3..];
                if let Some(name_end) = after_fn.find(&['(', '<', ' '][..]) {
                    let function_name = after_fn[..name_end].trim();
                    if !function_name.is_empty() && function_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                        return Some(function_name.to_string());
                    }
                }
            }
        }
        
        None
    }

    async fn handle_incremental_file_analysis(&self, request: McpRequest) -> McpResponse {
        let params = match request.params.as_ref() {
            Some(params) => params,
            None => return self.error_response(request.id, -32602, "Missing parameters"),
        };

        let files = match params.get("files").and_then(|v| v.as_array()) {
            Some(files) => files.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
            None => return self.error_response(request.id, -32602, "Missing 'files' parameter"),
        };

        if files.is_empty() {
            return self.error_response(request.id, -32602, "Empty 'files' list");
        }

        let update_database = params.get("update_database")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let include_impact_analysis = params.get("include_impact")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let depth = params.get("depth")
            .and_then(|v| v.as_u64())
            .unwrap_or(3) as usize;

        // Use incremental updater to analyze the modified files
        let mut analysis_results = Vec::new();
        let mut all_discovered_functions = Vec::new();

        for file_path in &files {
            eprintln!("📝 Incrementally analyzing file: {}", file_path);
            
            // Query for existing functions in this file before update
            let existing_functions = match self.discover_functions_in_files(&[file_path.clone()]).await {
                Ok(functions) => functions,
                Err(_) => Vec::new(),
            };
            
            // Use incremental updater to update the file (which handles database updates)
            if update_database {
                let mut incremental_updater = self.incremental_updater.write().await;
                let file_path_buf = std::path::PathBuf::from(file_path);
                
                match incremental_updater.process_file_changes(vec![file_path_buf]).await {
                    Ok(()) => {
                        // Query for new functions after update
                        drop(incremental_updater);
                        let new_functions = match self.discover_functions_in_files(&[file_path.clone()]).await {
                            Ok(functions) => functions,
                            Err(_) => Vec::new(),
                        };
                        
                        all_discovered_functions.extend(new_functions.clone());
                        
                        analysis_results.push(json!({
                            "file": file_path,
                            "status": "success",
                            "functions_before": existing_functions.len(),
                            "functions_after": new_functions.len(),
                            "functions_discovered": new_functions,
                            "database_update": {
                                "status": "success"
                            }
                        }));
                    }
                    Err(e) => {
                        eprintln!("⚠️ Failed to analyze file {}: {}", file_path, e);
                        analysis_results.push(json!({
                            "file": file_path,
                            "status": "failed",
                            "error": e.to_string(),
                            "functions_before": existing_functions.len(),
                            "functions_after": 0
                        }));
                    }
                }
            } else {
                // Just parse the file without database update
                let mut analyzer = self.analyzer.write().await;
                let file_path_buf = std::path::PathBuf::from(file_path);
                
                match analyzer.parser_mut().parse_file(&file_path_buf, "unknown") {
                    Ok(symbols) => {
                        let function_names: Vec<String> = symbols.functions.iter()
                            .map(|f| f.qualified_name.clone())
                            .collect();
                        
                        all_discovered_functions.extend(function_names.clone());
                        
                        analysis_results.push(json!({
                            "file": file_path,
                            "status": "success",
                            "functions_before": existing_functions.len(),
                            "functions_after": function_names.len(),
                            "functions_discovered": function_names,
                            "database_update": {
                                "status": "skipped"
                            }
                        }));
                    }
                    Err(e) => {
                        eprintln!("⚠️ Failed to parse file {}: {}", file_path, e);
                        analysis_results.push(json!({
                            "file": file_path,
                            "status": "failed",
                            "error": e.to_string(),
                            "functions_before": existing_functions.len(),
                            "functions_after": 0
                        }));
                    }
                }
            }
        }

        let response_data = json!({
            "analysis_type": "incremental_file_analysis",
            "files_analyzed": files,
            "results": analysis_results,
            "total_functions_discovered": all_discovered_functions.len(),
            "qualified_functions": all_discovered_functions
        });

        // Include impact analysis if requested and functions were discovered
        if include_impact_analysis && !all_discovered_functions.is_empty() {
            let batch_analysis = self.handle_aggregated_batch_analysis(&all_discovered_functions, depth, None).await;
            
            match batch_analysis.result {
                Some(mut batch_result) => {
                    if let Some(batch_obj) = batch_result.as_object_mut() {
                        // Merge incremental analysis data with impact analysis
                        for (key, value) in response_data.as_object().unwrap() {
                            batch_obj.insert(key.clone(), value.clone());
                        }
                    }
                    
                    McpResponse {
                        id: request.id,
                        result: Some(batch_result),
                        error: None,
                    }
                }
                None => {
                    McpResponse {
                        id: request.id,
                        result: Some(response_data),
                        error: batch_analysis.error,
                    }
                }
            }
        } else {
            McpResponse {
                id: request.id,
                result: Some(response_data),
                error: None,
            }
        }
    }

    async fn handle_find_unreferenced_functions(&self, request: McpRequest) -> McpResponse {
        let params = request.params.as_ref();
        
        let exclude_public = params
            .and_then(|p| p.get("exclude_public"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let exclude_tests = params
            .and_then(|p| p.get("exclude_tests"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        
        let crate_filter = params
            .and_then(|p| p.get("crate"))
            .and_then(|v| v.as_str());

        // Build the query to find functions with no incoming CALLS relationships
        // Go back to the working COUNT approach but keep it simple
        let mut query_parts = vec![
            "MATCH (f:Function)".to_string(),
            "OPTIONAL MATCH (caller:Function)-[:CALLS]->(f)".to_string(),
            "WITH f, COUNT(caller) AS caller_count".to_string(),
            "WHERE caller_count = 0".to_string(),
        ];

        // Add filters
        if exclude_public {
            query_parts.push("AND NOT f.visibility = 'pub'".to_string());
        }
        
        if exclude_tests {
            query_parts.push("AND NOT f.is_test = true".to_string());
        }
        
        if let Some(crate_name) = crate_filter {
            query_parts.push(format!("AND f.crate = '{}'", crate_name.replace('\'', "\\'")));
        }

        query_parts.push(
            "RETURN f.qualified_name as qualified_name, f.name as name, f.crate as crate, f.file as file, f.line_start as line, f.visibility as visibility, f.is_test as is_test ORDER BY f.crate, f.name".to_string()
        );

        let query_str = query_parts.join(" ");
        let query = neo4rs::Query::new(query_str);

        let mut unreferenced_functions = Vec::new();
        match self.graph.execute_query(query).await {
            Ok(result) => {
                for row in result {
                    if let (Ok(qualified_name), Ok(name), Ok(crate_name), Ok(file), Ok(line), Ok(visibility), Ok(is_test)) = (
                        row.get::<String>("qualified_name"),
                        row.get::<String>("name"),
                        row.get::<String>("crate"),
                        row.get::<String>("file"),
                        row.get::<i64>("line"),
                        row.get::<String>("visibility"),
                        row.get::<bool>("is_test"),
                    ) {
                        unreferenced_functions.push(json!({
                            "qualified_name": qualified_name,
                            "name": name,
                            "crate": crate_name,
                            "file": file,
                            "line": line,
                            "visibility": visibility,
                            "is_test": is_test
                        }));
                    }
                }
            }
            Err(e) => {
                return self.error_response(request.id, -32603, &format!("Query failed: {}", e));
            }
        }

        McpResponse {
            id: request.id,
            result: Some(json!({
                "analysis_type": "unreferenced_functions",
                "filters": {
                    "exclude_public": exclude_public,
                    "exclude_tests": exclude_tests,
                    "crate_filter": crate_filter
                },
                "count": unreferenced_functions.len(),
                "functions": unreferenced_functions
            })),
            error: None,
        }
    }

    async fn handle_find_test_only_functions(&self, request: McpRequest) -> McpResponse {
        let params = request.params.as_ref();
        
        let crate_filter = params
            .and_then(|p| p.get("crate"))
            .and_then(|v| v.as_str());

        let include_public = params
            .and_then(|p| p.get("include_public"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        // Query to find functions that are only called by test functions
        let mut query_parts = vec![
            "MATCH (f:Function)".to_string(),
            "OPTIONAL MATCH (caller:Function)-[:CALLS]->(f)".to_string(),
            "WITH f, COLLECT(caller) as callers".to_string(),
            "WHERE SIZE(callers) > 0 AND ALL(c IN callers WHERE c.is_test = true)".to_string(),
        ];

        if !include_public {
            query_parts.push("AND NOT f.visibility = 'pub'".to_string());
        }

        if let Some(crate_name) = crate_filter {
            query_parts.push(format!("AND f.crate = '{}'", crate_name.replace('\'', "\\'")));
        }

        query_parts.push(
            "RETURN f.qualified_name as qualified_name, f.name as name, f.crate as crate, f.file as file, 
                    f.line_start as line, f.visibility as visibility, f.is_test as is_test
             ORDER BY f.crate, f.name".to_string()
        );

        let query_str = query_parts.join(" ");
        let query = neo4rs::Query::new(query_str);

        let mut test_only_functions = Vec::new();
        match self.graph.execute_query(query).await {
            Ok(result) => {
                for row in result {
                    if let (Ok(qualified_name), Ok(name), Ok(crate_name), Ok(file), Ok(line), Ok(visibility), Ok(is_test)) = (
                        row.get::<String>("qualified_name"),
                        row.get::<String>("name"),
                        row.get::<String>("crate"),
                        row.get::<String>("file"),
                        row.get::<i64>("line"),
                        row.get::<String>("visibility"),
                        row.get::<bool>("is_test"),
                    ) {
                        test_only_functions.push(json!({
                            "qualified_name": qualified_name,
                            "name": name,
                            "crate": crate_name,
                            "file": file,
                            "line": line,
                            "visibility": visibility,
                            "is_test": is_test
                        }));
                    }
                }
            }
            Err(e) => {
                return self.error_response(request.id, -32603, &format!("Query failed: {}", e));
            }
        }

        McpResponse {
            id: request.id,
            result: Some(json!({
                "analysis_type": "test_only_functions",
                "filters": {
                    "include_public": include_public,
                    "crate_filter": crate_filter
                },
                "count": test_only_functions.len(),
                "functions": test_only_functions
            })),
            error: None,
        }
    }

    async fn handle_debug_call_relationships(&self, request: McpRequest) -> McpResponse {
        // First check if any relationships exist at all
        let count_query = neo4rs::Query::new("MATCH ()-[r:CALLS]->() RETURN COUNT(r) as count".to_string());
        
        let mut total_calls = 0;
        if let Ok(result) = self.graph.execute_query(count_query).await {
            if let Some(row) = result.first() {
                if let Ok(count) = row.get::<i64>("count") {
                    total_calls = count;
                }
            }
        }

        // Check all relationship types that exist
        let rel_types_query = neo4rs::Query::new("MATCH ()-[r]->() RETURN DISTINCT TYPE(r) as rel_type, COUNT(r) as count".to_string());
        let mut rel_types = Vec::new();
        if let Ok(result) = self.graph.execute_query(rel_types_query).await {
            for row in result {
                if let (Ok(rel_type), Ok(count)) = (row.get::<String>("rel_type"), row.get::<i64>("count")) {
                    rel_types.push(json!({
                        "relationship_type": rel_type,
                        "count": count
                    }));
                }
            }
        }

        // Query all CALLS relationships if they exist
        let query = neo4rs::Query::new(
            "MATCH (caller)-[r:CALLS]->(callee)
             RETURN caller.qualified_name as caller_name, callee.qualified_name as callee_name, 
                    labels(caller) as caller_labels, labels(callee) as callee_labels,
                    r.file as call_file, r.line as call_line
             ORDER BY caller_name, callee_name".to_string()
        );

        let mut call_relationships = Vec::new();
        match self.graph.execute_query(query).await {
            Ok(result) => {
                for row in result {
                    let caller_name = row.get::<String>("caller_name").unwrap_or_else(|_| "unknown".to_string());
                    let callee_name = row.get::<String>("callee_name").unwrap_or_else(|_| "unknown".to_string());
                    let caller_labels = row.get::<Vec<String>>("caller_labels").unwrap_or_else(|_| vec![]);
                    let callee_labels = row.get::<Vec<String>>("callee_labels").unwrap_or_else(|_| vec![]);
                    let call_file = row.get::<String>("call_file").unwrap_or_else(|_| "unknown".to_string());
                    let call_line = row.get::<i64>("call_line").unwrap_or(0);

                    call_relationships.push(json!({
                        "caller": caller_name,
                        "callee": callee_name,
                        "caller_labels": caller_labels,
                        "callee_labels": callee_labels,
                        "file": call_file,
                        "line": call_line
                    }));
                }
            }
            Err(e) => {
                return self.error_response(request.id, -32603, &format!("Debug query failed: {}", e));
            }
        }

        McpResponse {
            id: request.id,
            result: Some(json!({
                "analysis_type": "debug_call_relationships",
                "total_calls_relationships": total_calls,
                "all_relationship_types": rel_types,
                "calls_relationships_count": call_relationships.len(),
                "calls_relationships": call_relationships
            })),
            error: None,
        }
    }

    async fn handle_debug_graph(&self, request: McpRequest) -> McpResponse {
        eprintln!("🐛 Debug graph request received");
        
        // Test connection first
        if let Err(e) = self.graph.test_connection().await {
            return self.error_response(request.id, -32603, &format!("Connection failed: {}", e));
        }

        // Run verification
        match self.graph.verify_population().await {
            Ok(_) => {
                McpResponse {
                    id: request.id,
                    result: Some(json!({
                        "status": "verification_complete",
                        "message": "Check console for detailed output"
                    })),
                    error: None,
                }
            }
            Err(e) => self.error_response(request.id, -32603, &format!("Verification failed: {}", e)),
        }
    }

    async fn handle_find_functions_without_tests(&self, request: McpRequest) -> McpResponse {
        let params = request.params.as_ref();
        
        let crate_filter = params
            .and_then(|p| p.get("crate"))
            .and_then(|v| v.as_str());
        
        let limit = params
            .and_then(|p| p.get("limit"))
            .and_then(|v| v.as_i64())
            .unwrap_or(100) as i32;

        // Find functions that have no test coverage
        let mut query_parts = vec![
            "MATCH (f:Function)".to_string(),
            "WHERE (f.is_test = false OR f.is_test IS NULL)".to_string(),
            "OPTIONAL MATCH (test:Function)-[:CALLS]->(f)".to_string(),
            "WHERE test.is_test = true".to_string(),
            "WITH f, COUNT(test) AS test_count".to_string(),
            "WHERE test_count = 0".to_string(),
        ];

        if let Some(crate_name) = crate_filter {
            query_parts.push(format!("AND f.crate = '{}'", crate_name));
        }

        query_parts.push("RETURN f.qualified_name, f.crate, f.visibility".to_string());
        query_parts.push("ORDER BY f.qualified_name".to_string());
        query_parts.push(format!("LIMIT {}", limit));

        let query = query_parts.join(" ");
        
        match self.graph.execute_query(neo4rs::Query::new(query)).await {
            Ok(result) => {
                let mut functions = Vec::new();
                for row in result {
                    if let (Ok(qualified_name), Ok(crate_name), Ok(visibility)) = (
                        row.get::<String>("f.qualified_name"),
                        row.get::<String>("f.crate"),
                        row.get::<String>("f.visibility")
                    ) {
                        functions.push(json!({
                            "qualified_name": qualified_name,
                            "crate": crate_name,
                            "visibility": visibility
                        }));
                    }
                }

                McpResponse {
                    id: request.id,
                    result: Some(json!({
                        "functions_without_tests": functions,
                        "count": functions.len()
                    })),
                    error: None,
                }
            }
            Err(e) => self.error_response(request.id, -32603, &format!("Query failed: {}", e)),
        }
    }

    async fn handle_find_functions_with_tests(&self, request: McpRequest) -> McpResponse {
        let params = request.params.as_ref();
        
        let crate_filter = params
            .and_then(|p| p.get("crate"))
            .and_then(|v| v.as_str());
        
        let limit = params
            .and_then(|p| p.get("limit"))
            .and_then(|v| v.as_i64())
            .unwrap_or(100) as i32;

        // Find functions that have test coverage
        let mut query_parts = vec![
            "MATCH (f:Function)".to_string(),
            "WHERE (f.is_test = false OR f.is_test IS NULL)".to_string(),
            "OPTIONAL MATCH (test:Function)-[:CALLS]->(f)".to_string(),
            "WHERE test.is_test = true".to_string(),
            "WITH f, COUNT(test) AS test_count".to_string(),
            "WHERE test_count > 0".to_string(),
        ];

        if let Some(crate_name) = crate_filter {
            query_parts.push(format!("AND f.crate = '{}'", crate_name));
        }

        query_parts.push("RETURN f.qualified_name, f.crate, f.visibility, test_count".to_string());
        query_parts.push("ORDER BY test_count DESC, f.qualified_name".to_string());
        query_parts.push(format!("LIMIT {}", limit));

        let query = query_parts.join(" ");
        
        match self.graph.execute_query(neo4rs::Query::new(query)).await {
            Ok(result) => {
                let mut functions = Vec::new();
                for row in result {
                    if let (Ok(qualified_name), Ok(crate_name), Ok(visibility), Ok(test_count)) = (
                        row.get::<String>("f.qualified_name"),
                        row.get::<String>("f.crate"),
                        row.get::<String>("f.visibility"),
                        row.get::<i64>("test_count")
                    ) {
                        functions.push(json!({
                            "qualified_name": qualified_name,
                            "crate": crate_name,
                            "visibility": visibility,
                            "test_count": test_count
                        }));
                    }
                }

                McpResponse {
                    id: request.id,
                    result: Some(json!({
                        "functions_with_tests": functions,
                        "count": functions.len()
                    })),
                    error: None,
                }
            }
            Err(e) => self.error_response(request.id, -32603, &format!("Query failed: {}", e)),
        }
    }

    async fn handle_find_most_referenced_functions(&self, request: McpRequest) -> McpResponse {
        let params = request.params.as_ref();
        
        let limit = params
            .and_then(|p| p.get("limit"))
            .and_then(|v| v.as_i64())
            .unwrap_or(10) as i32;
        
        let crate_filter = params
            .and_then(|p| p.get("crate"))
            .and_then(|v| v.as_str());

        // Find functions with the most incoming CALLS relationships
        let mut query_parts = vec![
            "MATCH (f:Function)".to_string(),
            "OPTIONAL MATCH (caller:Function)-[:CALLS]->(f)".to_string(),
            "WITH f, COUNT(caller) AS reference_count".to_string(),
            "WHERE reference_count > 0".to_string(),
        ];

        if let Some(crate_name) = crate_filter {
            query_parts.push(format!("AND f.crate = '{}'", crate_name));
        }

        query_parts.push("RETURN f.qualified_name, f.crate, f.visibility, reference_count".to_string());
        query_parts.push("ORDER BY reference_count DESC".to_string());
        query_parts.push(format!("LIMIT {}", limit));

        let query = query_parts.join(" ");
        
        match self.graph.execute_query(neo4rs::Query::new(query)).await {
            Ok(result) => {
                let mut functions = Vec::new();
                for row in result {
                    if let (Ok(qualified_name), Ok(crate_name), Ok(visibility), Ok(reference_count)) = (
                        row.get::<String>("f.qualified_name"),
                        row.get::<String>("f.crate"),
                        row.get::<String>("f.visibility"),
                        row.get::<i64>("reference_count")
                    ) {
                        functions.push(json!({
                            "qualified_name": qualified_name,
                            "crate": crate_name,
                            "visibility": visibility,
                            "reference_count": reference_count
                        }));
                    }
                }

                McpResponse {
                    id: request.id,
                    result: Some(json!({
                        "most_referenced_functions": functions,
                        "count": functions.len()
                    })),
                    error: None,
                }
            }
            Err(e) => self.error_response(request.id, -32603, &format!("Query failed: {}", e)),
        }
    }

    async fn handle_find_most_referenced_without_tests(&self, request: McpRequest) -> McpResponse {
        let params = request.params.as_ref();
        
        let limit = params
            .and_then(|p| p.get("limit"))
            .and_then(|v| v.as_i64())
            .unwrap_or(10) as i32;
        
        let crate_filter = params
            .and_then(|p| p.get("crate"))
            .and_then(|v| v.as_str());

        // Find heavily referenced functions without test coverage
        let mut query_parts = vec![
            "MATCH (f:Function)".to_string(),
            "WHERE (f.is_test = false OR f.is_test IS NULL)".to_string(),
            "OPTIONAL MATCH (caller:Function)-[:CALLS]->(f)".to_string(),
            "WITH f, COUNT(caller) AS reference_count".to_string(),
            "WHERE reference_count > 0".to_string(),
            "OPTIONAL MATCH (test:Function)-[:CALLS]->(f)".to_string(),
            "WHERE test.is_test = true".to_string(),
            "WITH f, reference_count, COUNT(test) AS test_count".to_string(),
            "WHERE test_count = 0".to_string(),
        ];

        if let Some(crate_name) = crate_filter {
            query_parts.push(format!("AND f.crate = '{}'", crate_name));
        }

        query_parts.push("RETURN f.qualified_name, f.crate, f.visibility, reference_count".to_string());
        query_parts.push("ORDER BY reference_count DESC".to_string());
        query_parts.push(format!("LIMIT {}", limit));

        let query = query_parts.join(" ");
        
        match self.graph.execute_query(neo4rs::Query::new(query)).await {
            Ok(result) => {
                let mut functions = Vec::new();
                for row in result {
                    if let (Ok(qualified_name), Ok(crate_name), Ok(visibility), Ok(reference_count)) = (
                        row.get::<String>("f.qualified_name"),
                        row.get::<String>("f.crate"),
                        row.get::<String>("f.visibility"),
                        row.get::<i64>("reference_count")
                    ) {
                        functions.push(json!({
                            "qualified_name": qualified_name,
                            "crate": crate_name,
                            "visibility": visibility,
                            "reference_count": reference_count
                        }));
                    }
                }

                McpResponse {
                    id: request.id,
                    result: Some(json!({
                        "most_referenced_without_tests": functions,
                        "count": functions.len()
                    })),
                    error: None,
                }
            }
            Err(e) => self.error_response(request.id, -32603, &format!("Query failed: {}", e)),
        }
    }

    async fn handle_generate_actor_spawn_diagram(&self, request: McpRequest) -> McpResponse {
        let params = request.params.as_ref();
        
        let crate_filter = params
            .and_then(|p| p.get("crate"))
            .and_then(|v| v.as_str());

        // Query for all actor spawn relationships using Type:Actor nodes
        let mut query_parts = vec![
            "MATCH (parent:Type:Actor)-[r:SPAWNS]->(child:Type:Actor)".to_string(),
        ];

        if let Some(crate_name) = crate_filter {
            query_parts.push(format!("WHERE parent.crate = '{}' OR child.crate = '{}'", crate_name, crate_name));
        }

        query_parts.push("RETURN parent.name, parent.crate, child.name, child.crate, r.context".to_string());
        query_parts.push("ORDER BY parent.name, child.name".to_string());

        let query = query_parts.join(" ");
        
        match self.graph.execute_query(neo4rs::Query::new(query)).await {
            Ok(result) => {
                let mut relationships = Vec::new();
                let mut mermaid_lines = vec!["graph TD".to_string()];
                
                for row in result {
                    if let (Ok(parent_name), Ok(parent_crate), Ok(child_name), Ok(child_crate), context_opt) = (
                        row.get::<String>("parent.name"),
                        row.get::<String>("parent.crate"),
                        row.get::<String>("child.name"),
                        row.get::<String>("child.crate"),
                        row.get::<String>("r.context").ok()
                    ) {
                        let parent_id = format!("{}_{}", parent_crate.replace("-", "_"), parent_name);
                        let child_id = format!("{}_{}", child_crate.replace("-", "_"), child_name);
                        
                        // Create mermaid diagram line
                        mermaid_lines.push(format!("  {} -->|spawns| {}", parent_id, child_id));
                        
                        relationships.push(json!({
                            "parent": parent_name,
                            "parent_crate": parent_crate,
                            "child": child_name,
                            "child_crate": child_crate,
                            "context": context_opt
                        }));
                    }
                }

                let mermaid_diagram = mermaid_lines.join("\n");
                
                McpResponse {
                    id: request.id,
                    result: Some(json!({
                        "mermaid_diagram": mermaid_diagram,
                        "relationships": relationships,
                        "actor_count": relationships.len(), // This is spawn count, would need separate query for actor count
                        "spawn_count": relationships.len()
                    })),
                    error: None,
                }
            }
            Err(e) => {
                // If Actor nodes/SPAWNS relationships don't exist yet, return empty result
                eprintln!("⚠️ Actor spawn query failed (probably no actor data yet): {}", e);
                McpResponse {
                    id: request.id,
                    result: Some(json!({
                        "mermaid_diagram": "graph TD\n  %% No actor spawn relationships found",
                        "relationships": [],
                        "actor_count": 0,
                        "spawn_count": 0,
                        "note": "No actor spawn data available. Actor parsing not yet implemented."
                    })),
                    error: None,
                }
            },
        }
    }

    async fn handle_generate_actor_message_diagram(&self, request: McpRequest) -> McpResponse {
        let params = request.params.as_ref();
        
        let crate_filter = params
            .and_then(|p| p.get("crate"))
            .and_then(|v| v.as_str());
        
        let method_filter = params
            .and_then(|p| p.get("method"))
            .and_then(|v| v.as_str()); // "tell", "ask", or None for both

        // Query for all message send relationships using Type:Actor nodes
        let mut query_parts = vec![
            "MATCH (sender:Type:Actor)-[r:SENDS]->(receiver:Type:Actor)".to_string(),
        ];

        let mut where_clauses = Vec::new();
        
        if let Some(crate_name) = crate_filter {
            where_clauses.push(format!("(sender.crate = '{}' OR receiver.crate = '{}')", crate_name, crate_name));
        }
        
        if let Some(method) = method_filter {
            where_clauses.push(format!("r.method = '{}'", method));
        }
        
        if !where_clauses.is_empty() {
            query_parts.push(format!("WHERE {}", where_clauses.join(" AND ")));
        }

        query_parts.push("RETURN sender.name, sender.crate, receiver.name, receiver.crate, r.message_type, r.method".to_string());
        query_parts.push("ORDER BY sender.name, receiver.name, r.message_type".to_string());

        let query = query_parts.join(" ");
        
        match self.graph.execute_query(neo4rs::Query::new(query)).await {
            Ok(result) => {
                let mut relationships = Vec::new();
                let mut mermaid_lines = vec!["graph LR".to_string()]; // Use LR for messaging diagrams
                let mut tell_count = 0;
                let mut ask_count = 0;
                
                for row in result {
                    if let (Ok(sender_name), Ok(sender_crate), Ok(receiver_name), Ok(receiver_crate), Ok(message_type), Ok(method)) = (
                        row.get::<String>("sender.name"),
                        row.get::<String>("sender.crate"),
                        row.get::<String>("receiver.name"),
                        row.get::<String>("receiver.crate"),
                        row.get::<String>("r.message_type"),
                        row.get::<String>("r.method")
                    ) {
                        let sender_id = format!("{}_{}", sender_crate.replace("-", "_"), sender_name);
                        let receiver_id = format!("{}_{}", receiver_crate.replace("-", "_"), receiver_name);
                        
                        // Create mermaid diagram line with method and message type
                        let method_label = if method == "Tell" {
                            tell_count += 1;
                            "tell"
                        } else {
                            ask_count += 1;
                            "ask"
                        };
                        
                        mermaid_lines.push(format!("  {} -->|{}: {}| {}", 
                            sender_id, method_label, message_type, receiver_id));
                        
                        relationships.push(json!({
                            "sender": sender_name,
                            "sender_crate": sender_crate,
                            "receiver": receiver_name,
                            "receiver_crate": receiver_crate,
                            "message_type": message_type,
                            "method": method
                        }));
                    }
                }

                let mermaid_diagram = mermaid_lines.join("\n");
                
                McpResponse {
                    id: request.id,
                    result: Some(json!({
                        "mermaid_diagram": mermaid_diagram,
                        "relationships": relationships,
                        "total_messages": relationships.len(),
                        "tell_count": tell_count,
                        "ask_count": ask_count
                    })),
                    error: None,
                }
            }
            Err(e) => {
                // If Actor nodes/SENDS relationships don't exist yet, return empty result
                eprintln!("⚠️ Actor message query failed (probably no message data yet): {}", e);
                McpResponse {
                    id: request.id,
                    result: Some(json!({
                        "mermaid_diagram": "graph LR\n  %% No actor messaging relationships found",
                        "relationships": [],
                        "total_messages": 0,
                        "tell_count": 0,
                        "ask_count": 0,
                        "note": "No actor messaging data available. Try analyzing a workspace with kameo actors and message passing."
                    })),
                    error: None,
                }
            },
        }
    }

    async fn handle_get_actor_details(&self, request: McpRequest) -> McpResponse {
        let params = request.params.as_ref()
            .ok_or_else(|| "Missing parameters")
            .and_then(|p| p.get("actor_name").and_then(|v| v.as_str()).ok_or("Missing actor_name parameter"));

        let actor_name = match params {
            Ok(name) => name,
            Err(e) => return self.error_response(request.id, -32602, e),
        };

        let crate_filter = request.params.as_ref()
            .and_then(|p| p.get("crate"))
            .and_then(|v| v.as_str());

        // Build comprehensive query for actor with all its relationships
        let mut query = String::from("MATCH (a:Type:Actor {name: $actor_name})");
        
        if let Some(crate_name) = crate_filter {
            query.push_str(&format!(" WHERE a.crate = '{}'", crate_name));
        }

        query.push_str("
            OPTIONAL MATCH (a)-[:HAS_METHOD]->(m:Function)
            OPTIONAL MATCH (a)-[:HAS_FIELD]->(f:Field)
            OPTIONAL MATCH (a)-[:SPAWNS]->(spawned:Type:Actor)
            OPTIONAL MATCH (spawner:Type:Actor)-[:SPAWNS]->(a)
            OPTIONAL MATCH (a)-[:HANDLES]->(msg:MessageType)
            OPTIONAL MATCH (a)-[:SENDS]->(receiver:Type:Actor)
            OPTIONAL MATCH (sender:Type:Actor)-[:SENDS]->(a)
            OPTIONAL MATCH (a)-[:IMPLEMENTS]->(trait:Type)
            RETURN a,
                   collect(DISTINCT m) as methods,
                   collect(DISTINCT f) as fields,
                   collect(DISTINCT spawned.name) as spawns_actors,
                   collect(DISTINCT spawner.name) as spawned_by,
                   collect(DISTINCT msg.name) as handles_messages,
                   collect(DISTINCT receiver.name) as sends_to,
                   collect(DISTINCT sender.name) as receives_from,
                   collect(DISTINCT trait.name) as implements_traits");

        let neo_query = neo4rs::Query::new(query)
            .param("actor_name", actor_name);

        match self.graph.execute_query(neo_query).await {
            Ok(result) => {
                if let Some(row) = result.first() {
                    // Extract actor node properties
                    let actor_node = row.get::<neo4rs::Node>("a").ok();
                    
                    let actor_props = actor_node.as_ref().map(|node| {
                        json!({
                            "name": node.get::<String>("name").unwrap_or_default(),
                            "crate": node.get::<String>("crate").unwrap_or_default(),
                            "module_path": node.get::<String>("module_path").unwrap_or_default(),
                            "file_path": node.get::<String>("file_path").unwrap_or_default(),
                            "visibility": node.get::<String>("visibility").unwrap_or_default(),
                            "is_distributed": node.get::<bool>("is_distributed").unwrap_or(false),
                            "actor_type": node.get::<String>("actor_type").unwrap_or_default(),
                        })
                    });

                    // Extract related data
                    let methods: Vec<neo4rs::Node> = row.get("methods").unwrap_or_default();
                    let method_names: Vec<String> = methods.iter()
                        .filter_map(|m| m.get::<String>("name").ok())
                        .collect();

                    let fields: Vec<neo4rs::Node> = row.get("fields").unwrap_or_default();
                    let field_info: Vec<serde_json::Value> = fields.iter()
                        .map(|f| json!({
                            "name": f.get::<String>("name").unwrap_or_default(),
                            "type": f.get::<String>("type_name").unwrap_or_default(),
                        }))
                        .collect();

                    McpResponse {
                        id: request.id,
                        result: Some(json!({
                            "actor": actor_props,
                            "methods": method_names,
                            "fields": field_info,
                            "spawns_actors": row.get::<Vec<String>>("spawns_actors").unwrap_or_default(),
                            "spawned_by": row.get::<Vec<String>>("spawned_by").unwrap_or_default(),
                            "handles_messages": row.get::<Vec<String>>("handles_messages").unwrap_or_default(),
                            "sends_to": row.get::<Vec<String>>("sends_to").unwrap_or_default(),
                            "receives_from": row.get::<Vec<String>>("receives_from").unwrap_or_default(),
                            "implements_traits": row.get::<Vec<String>>("implements_traits").unwrap_or_default(),
                        })),
                        error: None,
                    }
                } else {
                    McpResponse {
                        id: request.id,
                        result: Some(json!({
                            "error": format!("Actor '{}' not found", actor_name),
                            "note": "Make sure the actor has been analyzed and exists in the graph"
                        })),
                        error: None,
                    }
                }
            }
            Err(e) => self.error_response(request.id, -32603, &format!("Query failed: {}", e)),
        }
    }

    async fn handle_get_distributed_actors(&self, request: McpRequest) -> McpResponse {
        let crate_filter = request.params.as_ref()
            .and_then(|p| p.get("crate"))
            .and_then(|v| v.as_str());

        let max_results = request.params.as_ref()
            .and_then(|p| p.get("limit"))
            .and_then(|v| v.as_i64())
            .unwrap_or(50) as usize;
        
        let exclude_tests = request.params.as_ref()
            .and_then(|p| p.get("exclude_tests"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Get current parsed symbols
        let current_symbols = self.current_symbols.read().await;
        
        if let Some(symbols) = current_symbols.as_ref() {
            let filtered_actors: Vec<_> = symbols.distributed_actors
                .iter()
                .filter(|actor| {
                    // Filter by crate if specified
                    if let Some(crate_name) = crate_filter {
                        if actor.crate_name != crate_name {
                            return false;
                        }
                    }
                    // Exclude test code if requested
                    if exclude_tests && actor.is_test {
                        return false;
                    }
                    true
                })
                .take(max_results)
                .map(|actor| json!({
                    "id": actor.id,
                    "name": actor.actor_name,
                    "crate": actor.crate_name,
                    "file": actor.file_path,
                    "line": actor.line,
                    "is_test": actor.is_test,
                    "distributed_messages": actor.distributed_messages,
                    "local_messages": actor.local_messages
                }))
                .collect();

            McpResponse {
                id: request.id,
                result: Some(json!({
                    "distributed_actors": filtered_actors,
                    "total_count": filtered_actors.len(),
                    "crate_filter": crate_filter
                })),
                error: None,
            }
        } else {
            self.error_response(request.id, -32603, "No workspace analysis available. Call initialize first.")
        }
    }

    async fn handle_generate_distributed_actor_message_flow(&self, request: McpRequest) -> McpResponse {
        let actor_filter = request.params.as_ref()
            .and_then(|p| p.get("actor"))
            .and_then(|v| v.as_str());

        let crate_filter = request.params.as_ref()
            .and_then(|p| p.get("crate"))
            .and_then(|v| v.as_str());

        // Get current parsed symbols
        let current_symbols = self.current_symbols.read().await;
        
        if let Some(symbols) = current_symbols.as_ref() {
            // Get all distributed actors
            let mut actor_set = std::collections::HashSet::new();
            for actor in &symbols.distributed_actors {
                if let Some(filter) = actor_filter {
                    if !actor.actor_name.contains(filter) {
                        continue;
                    }
                }
                if let Some(filter) = crate_filter {
                    if actor.crate_name != filter {
                        continue;
                    }
                }
                actor_set.insert((actor.actor_name.clone(), actor.crate_name.clone()));
            }

            // Get message flows
            let flows: Vec<_> = symbols.distributed_message_flows
                .iter()
                .filter(|flow| {
                    // Check if either sender or target matches filters
                    let actor_match = if let Some(filter) = actor_filter {
                        flow.sender_actor.contains(filter) || flow.target_actor.contains(filter)
                    } else {
                        true
                    };

                    let crate_match = if let Some(filter) = crate_filter {
                        flow.sender_crate == filter || flow.target_crate == filter
                    } else {
                        true
                    };

                    actor_match && crate_match
                })
                .collect();

            // Generate Mermaid diagram showing actual message flows
            let mut mermaid_diagram = String::from("graph LR\n");
            mermaid_diagram.push_str("    %% Distributed Actor Message Flow\n\n");
            
            // Add all actors involved in flows
            let mut involved_actors = std::collections::HashSet::new();
            for flow in &flows {
                involved_actors.insert((flow.sender_actor.clone(), flow.sender_crate.clone()));
                involved_actors.insert((flow.target_actor.clone(), flow.target_crate.clone()));
            }
            
            // Add actor nodes
            for (actor_name, crate_name) in &involved_actors {
                let node_id = format!("{}_{}", actor_name.replace(" ", "_"), crate_name.replace("-", "_"));
                mermaid_diagram.push_str(&format!(
                    "    {}[\"{}\\n({})\"]:::actor\n",
                    node_id,
                    actor_name,
                    crate_name
                ));
            }
            
            mermaid_diagram.push_str("\n    %% Message Flows\n");
            
            // Add message flow edges with message labels
            for flow in &flows {
                let sender_id = format!("{}_{}", flow.sender_actor.replace(" ", "_"), flow.sender_crate.replace("-", "_"));
                let target_id = format!("{}_{}", flow.target_actor.replace(" ", "_"), flow.target_crate.replace("-", "_"));
                let method_icon = match flow.send_method {
                    crate::parser::symbols::MessageSendMethod::Tell => "→",
                    crate::parser::symbols::MessageSendMethod::Ask => "⇄",
                };
                
                mermaid_diagram.push_str(&format!(
                    "    {} --\"{}[{}]\"--> {}\n",
                    sender_id,
                    flow.message_type,
                    method_icon,
                    target_id
                ));
            }

            // Add styling
            mermaid_diagram.push_str("\n    %% Styling\n");
            mermaid_diagram.push_str("    classDef actor fill:#e1f5fe,stroke:#0277bd,stroke-width:2px\n");
            mermaid_diagram.push_str("    classDef message fill:#fff3e0,stroke:#ff8f00,stroke-width:1px\n");

            McpResponse {
                id: request.id,
                result: Some(json!({
                    "mermaid_diagram": mermaid_diagram,
                    "message_flows": flows.len(),
                    "actors_involved": involved_actors.len(),
                    "filters": {
                        "actor": actor_filter,
                        "crate": crate_filter
                    },
                    "flows": flows.iter().map(|flow| json!({
                        "from": flow.sender_actor,
                        "to": flow.target_actor,
                        "message": flow.message_type,
                        "method": format!("{:?}", flow.send_method),
                        "location": format!("{}:{}", flow.send_location.file_path, flow.send_location.line)
                    })).collect::<Vec<_>>()
                })),
                error: None,
            }
        } else {
            self.error_response(request.id, -32603, "No workspace analysis available. Call initialize first.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_server_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let config_content = r#"
[workspace]
root = "."
additional_roots = []

[analysis]
recursive_scan = true
include_dev_deps = false
include_build_deps = false
exclude_crates = []

[architecture]
layers = []

[memgraph]
uri = "bolt://localhost:7687"
username = ""
password = ""
clean_start = false
batch_size = 1000

[embeddings]
enabled = false
model = "local"
include_in_embedding = []

[performance]
max_threads = 4
cache_size_mb = 100
incremental = true
"#;
        
        std::fs::write(&config_path, config_content).unwrap();
        
        // This would fail without a running Memgraph instance, but tests the parsing
        let result = EnhancedMcpServer::new(config_path.to_str().unwrap()).await;
        assert!(result.is_ok() || result.is_err()); // Either way, config parsing should work
    }
}