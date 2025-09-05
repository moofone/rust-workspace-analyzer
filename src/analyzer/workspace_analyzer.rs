use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

use crate::config::Config;
use crate::parser::{RustParser, ParsedSymbols};
pub use crate::parser::{RustFunction, RustType};
use crate::workspace::{WorkspaceDiscovery, CrateMetadata};
use crate::analyzer::{GlobalSymbolIndex, CrateFunctionInfo, CrateTypeInfo, CrateTraitInfo, CrateExports, Visibility, TypeKind, TraitMethodInfo};

#[derive(Debug, Clone)]
pub struct WorkspaceSnapshot {
    pub crates: Vec<CrateMetadata>,
    pub symbols: HashMap<String, ParsedSymbols>,
    pub functions: Vec<RustFunction>,
    pub types: Vec<RustType>,
    pub actors: Vec<crate::parser::symbols::RustActor>,
    pub actor_spawns: Vec<crate::parser::symbols::ActorSpawn>,
    pub distributed_actors: Vec<crate::parser::symbols::DistributedActor>,
    pub distributed_message_flows: Vec<crate::parser::symbols::DistributedMessageFlow>,
    pub function_references: HashMap<String, Vec<String>>,
    pub dependencies: HashMap<String, Vec<String>>,
}

pub struct WorkspaceAnalyzer {
    config: Config,
    parser: RustParser,
    workspace_discovery: WorkspaceDiscovery,
    global_index: Option<GlobalSymbolIndex>,
}

impl WorkspaceAnalyzer {
    pub fn new<P: AsRef<Path>>(workspace_root: P) -> Result<Self> {
        let config = Config::from_workspace_root(workspace_root.as_ref())?;
        let parser = RustParser::new()?;
        let workspace_discovery = WorkspaceDiscovery::new(config.clone());

        Ok(Self {
            config,
            parser,
            workspace_discovery,
            global_index: None,
        })
    }

    pub fn new_with_config(config: Config) -> Result<Self> {
        let parser = RustParser::new()?;
        let workspace_discovery = WorkspaceDiscovery::new(config.clone());

        Ok(Self {
            config,
            parser,
            workspace_discovery,
            global_index: None,
        })
    }

    pub async fn create_snapshot(&mut self) -> Result<WorkspaceSnapshot> {
        let all_crates = self.workspace_discovery.discover_crates().await?;
        
        // Filter crates based on workspace_members_only configuration
        let crates_to_analyze = if self.config.analysis.workspace_members_only {
            all_crates.iter().filter(|crate_meta| crate_meta.is_workspace_member).cloned().collect::<Vec<_>>()
        } else {
            all_crates.clone()
        };
        
        let mut symbols = HashMap::new();
        let mut all_functions = Vec::new();
        let mut all_types = Vec::new();
        let mut all_actors = Vec::new();
        let mut all_actor_spawns = Vec::new();
        let mut all_distributed_actors = Vec::new();
        let mut all_distributed_message_flows = Vec::new();

        for crate_meta in &crates_to_analyze {
            if let Ok(mut parsed) = self.parse_crate_files_internal(&crate_meta.path, &crate_meta.name) {
                // Resolve references and generate synthetic trait method calls before merging
                eprintln!("  🔗 Resolving references for crate: {}", crate_meta.name);
                crate::parser::references::resolve_all_references(&mut parsed).unwrap_or_else(|e| {
                    eprintln!("  ⚠️ WARNING: Reference resolution failed for {}: {}", crate_meta.name, e);
                });
                
                all_functions.extend(parsed.functions.clone());
                all_types.extend(parsed.types.clone());
                all_actors.extend(parsed.actors.clone());
                all_actor_spawns.extend(parsed.actor_spawns.clone());
                all_distributed_actors.extend(parsed.distributed_actors.clone());
                all_distributed_message_flows.extend(parsed.distributed_message_flows.clone());
                symbols.insert(crate_meta.name.clone(), parsed);
            }
        }

        Ok(WorkspaceSnapshot {
            crates: crates_to_analyze,
            symbols,
            functions: all_functions,
            types: all_types,
            actors: all_actors,
            actor_spawns: all_actor_spawns,
            distributed_actors: all_distributed_actors,
            distributed_message_flows: all_distributed_message_flows,
            function_references: HashMap::new(),
            dependencies: HashMap::new(),
        })
    }

    pub fn analyze_workspace(&mut self) -> Result<WorkspaceSnapshot> {
        // For synchronous version, we'll use a simple blocking approach
        // This is a placeholder - in a real implementation you might want to use tokio::task::block_in_place
        futures::executor::block_on(self.create_snapshot())
    }

    // New method to parse and populate graph (for MCP use)
    pub async fn analyze_and_populate_graph(
        &mut self,
        graph: Option<&crate::graph::MemgraphClient>,
        embedding_gen: Option<&crate::embeddings::EmbeddingGenerator>,
        architecture: Option<&crate::architecture::ArchitectureAnalyzer>,
        semantic_search: Option<&mut crate::embeddings::SemanticSearch>,
        incremental_updater: Option<&mut crate::incremental::IncrementalUpdater>
    ) -> Result<ParsedSymbols> {
        // 1. Discover crates
        let timer = std::time::Instant::now();
        let all_crates = self.workspace_discovery.discover_crates().await?;
        
        // Filter crates based on workspace_members_only configuration
        let crates_to_analyze = if self.config.analysis.workspace_members_only {
            all_crates.iter().filter(|crate_meta| crate_meta.is_workspace_member).cloned().collect::<Vec<_>>()
        } else {
            all_crates.clone()
        };
        eprintln!("  ⏱️ Crate discovery: {:?}", timer.elapsed());
        
        // 2. Create crate nodes in graph
        let graph_timer = std::time::Instant::now();
        if let Some(graph) = graph {
            eprintln!("  ✅ Graph client available, creating crate nodes");
            graph.create_crate_nodes(&crates_to_analyze).await?;
        } else {
            eprintln!("  ⚠️ WARNING: Graph client is None - no data will be written to Memgraph!");
        }
        eprintln!("  ⏱️ Graph node creation: {:?}", graph_timer.elapsed());

        // 3. Parse all files using existing parser
        let parse_timer = std::time::Instant::now();
        let mut all_symbols = ParsedSymbols::new();
        let total_files = crates_to_analyze.len();
        for (i, crate_meta) in crates_to_analyze.iter().enumerate() {
            eprintln!("  📦 Parsing crate {}/{}: {}", i+1, total_files, crate_meta.name);
            let crate_timer = std::time::Instant::now();
            if let Ok(parsed) = self.parse_crate_files_internal(&crate_meta.path, &crate_meta.name) {
                eprintln!("    ⏱️ {} parsed in {:?} ({} files)", 
                    crate_meta.name, 
                    crate_timer.elapsed(),
                    parsed.functions.len());
                all_symbols.merge(parsed);
            }
        }
        eprintln!("  ⏱️ Total parsing time: {:?}", parse_timer.elapsed());

        // 4. Resolve references and generate synthetic calls (including trait methods)
        let resolve_timer = std::time::Instant::now();
        eprintln!("  🔗 Resolving references and generating synthetic calls...");
        crate::parser::references::resolve_all_references(&mut all_symbols).unwrap_or_else(|e| {
            eprintln!("  ⚠️ WARNING: Reference resolution failed: {}", e);
        });
        eprintln!("  ⏱️ Reference resolution: {:?}", resolve_timer.elapsed());

        // 5. Generate embeddings if provided
        let embed_timer = std::time::Instant::now();
        if let Some(embedding_gen) = embedding_gen {
            embedding_gen.generate_embeddings(&mut all_symbols).await?;
            eprintln!("  ⏱️ Embedding generation: {:?}", embed_timer.elapsed());
        }

        // 6. Populate graph
        let populate_timer = std::time::Instant::now();
        if let Some(graph) = graph {
            eprintln!("  ✅ Graph client available, populating with symbols");
            graph.populate_from_symbols(&all_symbols).await?;
            graph.verify_population().await?;
            eprintln!("  ⏱️ Graph population: {:?}", populate_timer.elapsed());
        } else {
            eprintln!("  ⚠️ WARNING: Graph client is None - symbols not written to Memgraph!");
        }

        // 7. Index for semantic search (handled separately as it requires embeddings, not raw symbols)
        // Note: Semantic search indexing should be done after embeddings are generated

        // 8. Run architecture analysis if provided
        let arch_timer = std::time::Instant::now();
        if let Some(architecture) = architecture {
            architecture.analyze_architecture().await?;
            eprintln!("  ⏱️ Architecture analysis: {:?}", arch_timer.elapsed());
        }

        // 9. Handle incremental updates if provided (skip for full analysis)
        // Note: Incremental updates are only relevant when specific files have changed

        Ok(all_symbols)
    }

    // Get access to internal parser (for incremental updates)
    pub fn parser_mut(&mut self) -> &mut RustParser {
        &mut self.parser
    }

    // Make parse_crate_files public so it can be used by others
    pub fn parse_crate_files(&mut self, crate_path: &Path, crate_name: &str) -> Result<ParsedSymbols> {
        self.parse_crate_files_internal(crate_path, crate_name)
    }

    fn parse_crate_files_internal(&mut self, crate_path: &Path, crate_name: &str) -> Result<ParsedSymbols> {
        let mut symbols = ParsedSymbols::new();
        let src_dir = crate_path.join("src");
        
        if !src_dir.exists() {
            return Ok(symbols);
        }

        let walker = walkdir::WalkDir::new(&src_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|entry| {
                entry.path().extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "rs")
                    .unwrap_or(false)
            });

        for entry in walker {
            let file_path = entry.path();
            
            // Skip known problematic files that cause segfaults
            let file_name = file_path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            
            // Note: Previously problematic files now handled by robust error handling in parser
            // No longer need to skip entire files - the parser will handle crashes gracefully
            
            match self.parser.parse_file(file_path, crate_name) {
                Ok(file_symbols) => {
                    symbols.merge(file_symbols)
                },
                Err(_e) => {
                    // Continue parsing other files
                }
            }
        }

        Ok(symbols)
    }

    /// Build global symbol index for cross-crate resolution
    pub async fn build_global_symbol_index(&mut self) -> Result<()> {
        if !self.config.cross_crate.enabled {
            return Ok(());
        }

        eprintln!("🔍 Building global symbol index...");
        
        // Try to load from cache first
        let workspace_root = self.config.workspace.root.clone();
        let mut index = GlobalSymbolIndex::new(workspace_root.clone());
        
        if self.config.cross_crate.use_cache {
            if let Ok(Some(cached_index)) = index.try_load_from_cache() {
                eprintln!("  ✅ Loaded global index from cache");
                self.global_index = Some(cached_index);
                return Ok(());
            }
        }

        // Build fresh index
        let start_time = std::time::Instant::now();
        let all_crates = self.workspace_discovery.discover_crates().await?;
        
        // Filter crates based on workspace_members_only configuration
        let crates_to_analyze = if self.config.analysis.workspace_members_only {
            all_crates.iter().filter(|crate_meta| crate_meta.is_workspace_member).cloned().collect::<Vec<_>>()
        } else {
            all_crates.clone()
        };

        eprintln!("  📦 Indexing {} crates...", crates_to_analyze.len());

        for (i, crate_meta) in crates_to_analyze.iter().enumerate() {
            eprintln!("    {}/{}: {}", i + 1, crates_to_analyze.len(), crate_meta.name);
            
            // Parse crate symbols
            if let Ok(parsed) = self.parse_crate_files_internal(&crate_meta.path, &crate_meta.name) {
                // Extract function information
                for function in &parsed.functions {
                    let func_info = self.convert_to_crate_function_info(function, &crate_meta.name)?;
                    index.add_function(func_info);
                }

                // Extract type information
                for rust_type in &parsed.types {
                    let type_info = self.convert_to_crate_type_info(rust_type, &crate_meta.name, &parsed)?;
                    index.add_type(type_info);
                }

                // Extract trait information from types with TypeKind::Trait
                for rust_type in &parsed.types {
                    if matches!(rust_type.kind, crate::parser::symbols::TypeKind::Trait) {
                        let trait_info = self.convert_to_crate_trait_info(rust_type, &crate_meta.name, &parsed)?;
                        index.add_trait(trait_info);
                    }
                }

                // Build crate exports
                let exports = self.build_crate_exports(&crate_meta.name, &parsed)?;
                index.add_crate_exports(exports);
            }
        }

        let stats = index.stats();
        eprintln!("  ⏱️ Index built in {:?}", start_time.elapsed());
        eprintln!("  📊 {}", stats);

        // Save to cache if enabled
        if self.config.cross_crate.use_cache {
            if let Err(e) = index.save_to_cache() {
                eprintln!("  ⚠️ WARNING: Failed to save index cache: {}", e);
            } else {
                eprintln!("  💾 Index cached for future use");
            }
        }

        self.global_index = Some(index);
        Ok(())
    }

    /// Convert RustFunction to CrateFunctionInfo
    fn convert_to_crate_function_info(&self, function: &RustFunction, crate_name: &str) -> Result<CrateFunctionInfo> {
        // Detect if this is a method by checking if first parameter is self
        let has_self_param = function.parameters.iter().any(|p| p.is_self);
        
        // Extract associated type from qualified name if it contains ::
        let associated_type = if function.qualified_name.contains("::") && has_self_param {
            // Extract the type name from qualified name like "MyType::method"
            let parts: Vec<&str> = function.qualified_name.split("::").collect();
            if parts.len() >= 2 {
                Some(parts[parts.len() - 2].to_string())
            } else {
                None
            }
        } else {
            None
        };

        Ok(CrateFunctionInfo {
            name: function.name.clone(),
            crate_name: crate_name.to_string(),
            module_path: vec![function.module_path.clone()],
            signature: function.signature.clone(),
            visibility: self.convert_visibility(&function.visibility),
            is_async: function.is_async,
            is_unsafe: function.is_unsafe,
            is_extern: false, // TODO: Extract from parsed data
            associated_type,
            trait_impl: None, // TODO: Extract from impl blocks
            file_path: function.file_path.clone().into(),
            line_number: Some(function.line_start as u32),
        })
    }

    /// Convert RustType to CrateTypeInfo
    fn convert_to_crate_type_info(&self, rust_type: &RustType, crate_name: &str, parsed: &ParsedSymbols) -> Result<CrateTypeInfo> {
        // Find methods and associated functions for this type from parsed functions
        let methods = parsed.functions.iter()
            .filter(|f| {
                // Check if function is associated with this type and has self param
                f.qualified_name.starts_with(&format!("{}::", rust_type.name)) &&
                f.parameters.iter().any(|p| p.is_self)
            })
            .map(|f| f.name.clone())
            .collect();

        let associated_functions = parsed.functions.iter()
            .filter(|f| {
                // Check if function is associated with this type but no self param
                f.qualified_name.starts_with(&format!("{}::", rust_type.name)) &&
                !f.parameters.iter().any(|p| p.is_self)
            })
            .map(|f| f.name.clone())
            .collect();

        // Extract trait implementations from impl blocks in parsed data
        let trait_impls = parsed.impls.iter()
            .filter(|impl_block| impl_block.type_name == rust_type.name)
            .filter_map(|impl_block| impl_block.trait_name.clone())
            .collect();

        Ok(CrateTypeInfo {
            name: rust_type.name.clone(),
            crate_name: crate_name.to_string(),
            module_path: vec![rust_type.module_path.clone()],
            type_kind: self.convert_type_kind(&rust_type.kind),
            visibility: self.convert_visibility(&rust_type.visibility),
            methods,
            associated_functions,
            trait_impls,
            generic_params: Vec::new(), // TODO: Extract generic parameters
            file_path: rust_type.file_path.clone().into(),
            line_number: Some(rust_type.line_start as u32),
        })
    }

    /// Convert RustType (trait) to CrateTraitInfo  
    fn convert_to_crate_trait_info(&self, trait_def: &RustType, crate_name: &str, parsed: &ParsedSymbols) -> Result<CrateTraitInfo> {
        // Find trait methods from impl blocks that implement this trait
        let methods = parsed.impls.iter()
            .filter(|impl_block| impl_block.trait_name.as_deref() == Some(&trait_def.name))
            .flat_map(|impl_block| &impl_block.methods)
            .map(|method| TraitMethodInfo {
                name: method.name.clone(),
                signature: method.signature.clone(),
                is_async: method.is_async,
                is_unsafe: method.is_unsafe,
                has_default_impl: false, // TODO: Detect default implementations
            })
            .collect();

        Ok(CrateTraitInfo {
            name: trait_def.name.clone(),
            crate_name: crate_name.to_string(),
            module_path: vec![trait_def.module_path.clone()],
            visibility: self.convert_visibility(&trait_def.visibility),
            methods,
            associated_types: Vec::new(), // TODO: Extract from trait definition
            super_traits: Vec::new(), // TODO: Extract from trait definition
            generic_params: Vec::new(), // TODO: Extract generic parameters
            file_path: trait_def.file_path.clone().into(),
            line_number: Some(trait_def.line_start as u32),
        })
    }

    /// Build crate exports information
    fn build_crate_exports(&self, crate_name: &str, parsed: &ParsedSymbols) -> Result<CrateExports> {
        let public_functions = parsed.functions.iter()
            .filter(|f| f.visibility == "pub")
            .map(|f| f.name.clone())
            .collect();

        let public_types = parsed.types.iter()
            .filter(|t| t.visibility == "pub")
            .map(|t| t.name.clone())
            .collect();

        let public_traits = parsed.types.iter()
            .filter(|t| t.visibility == "pub" && matches!(t.kind, crate::parser::symbols::TypeKind::Trait))
            .map(|t| t.name.clone())
            .collect();

        // TODO: Extract re-exports and glob exports from parsed data
        let re_exports = HashMap::new();
        let glob_exports = Vec::new();

        Ok(CrateExports {
            crate_name: crate_name.to_string(),
            public_functions,
            public_types,
            public_traits,
            re_exports,
            glob_exports,
        })
    }

    /// Convert visibility string to enum
    fn convert_visibility(&self, visibility: &str) -> Visibility {
        match visibility {
            "pub" => Visibility::Public,
            "pub(crate)" => Visibility::Crate,
            "pub(super)" => Visibility::SuperScope,
            _ => Visibility::Private,
        }
    }

    /// Convert type kind enum to enum
    fn convert_type_kind(&self, kind: &crate::parser::symbols::TypeKind) -> TypeKind {
        match kind {
            crate::parser::symbols::TypeKind::Struct => TypeKind::Struct,
            crate::parser::symbols::TypeKind::Enum => TypeKind::Enum,
            crate::parser::symbols::TypeKind::Union => TypeKind::Union,
            crate::parser::symbols::TypeKind::TypeAlias => TypeKind::Alias,
            crate::parser::symbols::TypeKind::Trait => TypeKind::Struct, // Traits treated as struct for now
        }
    }

    /// Get reference to global index
    pub fn global_index(&self) -> Option<&GlobalSymbolIndex> {
        self.global_index.as_ref()
    }

    /// Get mutable reference to global index
    pub fn global_index_mut(&mut self) -> Option<&mut GlobalSymbolIndex> {
        self.global_index.as_mut()
    }

    /// Two-pass analysis with global context - the key method needed by the test
    pub async fn analyze_with_global_context(&mut self) -> Result<WorkspaceSnapshot> {
        eprintln!("🔍 Starting two-pass analysis with global context...");
        
        // Phase 1: Build global symbol index for cross-crate resolution
        if self.config.cross_crate.enabled {
            self.build_global_symbol_index().await?;
        }

        // Phase 2: Analyze with global context and framework patterns
        let all_crates = self.workspace_discovery.discover_crates().await?;
        
        // Filter crates based on workspace_members_only configuration
        let crates_to_analyze = if self.config.analysis.workspace_members_only {
            all_crates.iter().filter(|crate_meta| crate_meta.is_workspace_member).cloned().collect::<Vec<_>>()
        } else {
            all_crates.clone()
        };
        
        let mut symbols = HashMap::new();
        let mut all_functions = Vec::new();
        let mut all_types = Vec::new();
        let mut all_actors = Vec::new();
        let mut all_actor_spawns = Vec::new();
        let mut all_distributed_actors = Vec::new();
        let mut all_distributed_message_flows = Vec::new();

        for crate_meta in &crates_to_analyze {
            if let Ok(mut parsed) = self.parse_crate_files_internal(&crate_meta.path, &crate_meta.name) {
                // Enhanced reference resolution with global context
                eprintln!("  🔗 Resolving references for crate: {} (with global context)", crate_meta.name);
                
                // First run standard reference resolution
                crate::parser::references::resolve_all_references(&mut parsed).unwrap_or_else(|e| {
                    eprintln!("  ⚠️ WARNING: Reference resolution failed for {}: {}", crate_meta.name, e);
                });
                
                // Then apply framework knowledge and global context resolution
                self.apply_framework_knowledge(&mut parsed).await?;
                self.apply_cross_crate_resolution(&mut parsed).await?;
                
                all_functions.extend(parsed.functions.clone());
                all_types.extend(parsed.types.clone());
                all_actors.extend(parsed.actors.clone());
                all_actor_spawns.extend(parsed.actor_spawns.clone());
                all_distributed_actors.extend(parsed.distributed_actors.clone());
                all_distributed_message_flows.extend(parsed.distributed_message_flows.clone());
                symbols.insert(crate_meta.name.clone(), parsed);
            }
        }

        Ok(WorkspaceSnapshot {
            crates: crates_to_analyze,
            symbols,
            functions: all_functions,
            types: all_types,
            actors: all_actors,
            actor_spawns: all_actor_spawns,
            distributed_actors: all_distributed_actors,
            distributed_message_flows: all_distributed_message_flows,
            function_references: HashMap::new(),
            dependencies: HashMap::new(),
        })
    }

    /// Apply framework knowledge to detect runtime calls and mark functions as used
    async fn apply_framework_knowledge(&mut self, parsed: &mut ParsedSymbols) -> Result<()> {
        if !self.config.framework.enabled {
            return Ok(());
        }

        eprintln!("  🎯 Applying framework pattern recognition...");
        
        // Load framework patterns
        use crate::analyzer::FrameworkPatterns;
        let patterns = FrameworkPatterns::with_default_patterns();
        
        // Track synthetic calls and framework functions we generate
        let mut synthetic_calls = Vec::new();
        let mut framework_functions = Vec::new();

        // Create framework dispatch functions that will call trait methods
        let websocket_dispatch_function = crate::parser::symbols::RustFunction {
            id: "websocket_framework_dispatch".to_string(),
            name: "websocket_dispatch".to_string(),
            qualified_name: "websocket_framework::websocket_dispatch".to_string(),
            crate_name: "websocket_framework".to_string(),
            module_path: "websocket_framework".to_string(),
            file_path: "<synthetic>".to_string(),
            line_start: 0,
            line_end: 0,
            visibility: "pub".to_string(),
            is_async: true,
            is_unsafe: false,
            is_generic: false,
            is_test: false,
            is_trait_impl: false,
            doc_comment: Some("Synthetic framework dispatcher for WebSocket trait methods".to_string()),
            signature: "async fn websocket_dispatch()".to_string(),
            parameters: Vec::new(),
            return_type: Some("()".to_string()),
            embedding_text: None,
            module: "websocket_framework".to_string(),
        };
        framework_functions.push(websocket_dispatch_function);

        // Create actix framework lifecycle dispatcher
        let actix_lifecycle_function = crate::parser::symbols::RustFunction {
            id: "actix_framework_lifecycle".to_string(),
            name: "lifecycle_dispatch".to_string(),
            qualified_name: "actix_framework::lifecycle_dispatch".to_string(),
            crate_name: "actix_framework".to_string(),
            module_path: "actix_framework".to_string(),
            file_path: "<synthetic>".to_string(),
            line_start: 0,
            line_end: 0,
            visibility: "pub".to_string(),
            is_async: false,
            is_unsafe: false,
            is_generic: false,
            is_test: false,
            is_trait_impl: false,
            doc_comment: Some("Synthetic framework dispatcher for Actor lifecycle methods".to_string()),
            signature: "fn lifecycle_dispatch()".to_string(),
            parameters: Vec::new(),
            return_type: Some("()".to_string()),
            embedding_text: None,
            module: "actix_framework".to_string(),
        };
        framework_functions.push(actix_lifecycle_function);

        // For each source file, check for framework patterns
        for function in &parsed.functions {
            if let Some(file_content) = self.get_file_content(&function.file_path) {
                // Check for WebSocket actor trait implementations
                if self.is_websocket_actor_implementation(&file_content, &function.qualified_name) {
                    // Mark WebSocketActor trait methods as used via synthetic calls
                    if function.name == "event_stream" || function.name == "handle_message" {
                        eprintln!("    🎯 Marking WebSocketActor method as used: {}", function.qualified_name);
                        
                        // Create synthetic call from framework dispatch function to this method
                        synthetic_calls.push(crate::parser::symbols::FunctionCall {
                            caller_id: "websocket_framework_dispatch".to_string(),
                            caller_module: "websocket_framework".to_string(),
                            callee_name: function.name.clone(),
                            qualified_callee: Some(function.qualified_name.clone()),
                            call_type: crate::parser::symbols::CallType::Method,
                            line: function.line_start,
                            cross_crate: true,
                            from_crate: "websocket_framework".to_string(),
                            to_crate: Some(function.crate_name.clone()),
                            file_path: function.file_path.clone(),
                            is_synthetic: true,
                            macro_context: None,
                            synthetic_confidence: 0.95,
                        });
                    }
                }

                // Check for Actor::start() patterns that trigger lifecycle methods  
                if self.contains_actor_spawn_pattern(&file_content) {
                    // Find actor lifecycle methods and mark them as used
                    if function.name == "started" || function.name == "stopping" || function.name == "stopped" {
                        eprintln!("    🎯 Marking Actor lifecycle method as used: {}", function.qualified_name);
                        
                        synthetic_calls.push(crate::parser::symbols::FunctionCall {
                            caller_id: "actix_framework_lifecycle".to_string(),
                            caller_module: "actix_framework".to_string(),
                            callee_name: function.name.clone(),
                            qualified_callee: Some(function.qualified_name.clone()),
                            call_type: crate::parser::symbols::CallType::Method,
                            line: function.line_start,
                            cross_crate: true,
                            from_crate: "actix_framework".to_string(),
                            to_crate: Some(function.crate_name.clone()),
                            file_path: function.file_path.clone(),
                            is_synthetic: true,
                            macro_context: None,
                            synthetic_confidence: 0.90,
                        });
                    }
                }
            }
        }

        // Add synthetic calls and framework functions to parsed symbols
        let synthetic_count = synthetic_calls.len();
        parsed.calls.extend(synthetic_calls);
        parsed.functions.extend(framework_functions);
        
        eprintln!("    ✅ Generated {} synthetic framework calls", synthetic_count);
        Ok(())
    }

    /// Apply cross-crate resolution using global index
    async fn apply_cross_crate_resolution(&mut self, parsed: &mut ParsedSymbols) -> Result<()> {
        if !self.config.cross_crate.enabled || self.global_index.is_none() {
            return Ok(());
        }

        eprintln!("  🌐 Applying cross-crate resolution...");
        
        let index = self.global_index.as_ref().unwrap();
        let mut resolved_calls = 0;

        // Look for function calls that might be cross-crate Type::method calls
        for call in &mut parsed.calls {
            
            // Check if this looks like a Type::method call that might be cross-crate
            if let Some(qualified_callee) = &call.qualified_callee {
                if qualified_callee.contains("::") && call.to_crate.is_none() {
                    let parts: Vec<&str> = qualified_callee.split("::").collect();
                    if parts.len() >= 2 {
                        let type_name = parts[parts.len() - 2];
                        let method_name = parts[parts.len() - 1];
                        
                        // Try to resolve using global index
                        let resolved_functions = index.resolve_associated_function(type_name, method_name);
                        
                        if !resolved_functions.is_empty() {
                            // Use the first match (could be enhanced to pick best match)
                            let resolved = &resolved_functions[0];
                            
                            eprintln!("    🎯 Resolved cross-crate call: {}::{} -> {}", 
                                      type_name, method_name, resolved.crate_name);
                            
                            call.to_crate = Some(resolved.crate_name.clone());
                            call.cross_crate = true;
                            resolved_calls += 1;
                        }
                    }
                }
            }
        }

        eprintln!("    ✅ Resolved {} cross-crate calls", resolved_calls);
        Ok(())
    }

    /// Check if file content contains WebSocketActor trait implementation
    fn is_websocket_actor_implementation(&self, file_content: &str, qualified_name: &str) -> bool {
        file_content.contains("impl WebSocketActor") || 
        file_content.contains("impl<") && file_content.contains("WebSocketActor") ||
        qualified_name.contains("WebSocketActor")
    }

    /// Check if file content contains actor spawn patterns
    fn contains_actor_spawn_pattern(&self, file_content: &str) -> bool {
        file_content.contains(".start()") || 
        file_content.contains("Actor::start") ||
        file_content.contains("actor.start")
    }

    /// Get file content for pattern matching
    fn get_file_content(&self, file_path: &str) -> Option<String> {
        std::fs::read_to_string(file_path).ok()
    }
}

pub struct HybridWorkspaceAnalyzer {
    workspace_analyzer: WorkspaceAnalyzer,
}

impl HybridWorkspaceAnalyzer {
    pub async fn new<P: AsRef<Path>>(workspace_root: P, _lsp_config: Option<crate::config::Config>) -> Result<Self> {
        let workspace_analyzer = WorkspaceAnalyzer::new(workspace_root)?;
        
        Ok(Self {
            workspace_analyzer,
        })
    }

    pub async fn create_snapshot(&mut self) -> Result<WorkspaceSnapshot> {
        self.workspace_analyzer.create_snapshot().await
    }

    pub async fn analyze_workspace(&mut self) -> Result<crate::lsp::models::HybridAnalysisResult> {
        let snapshot = self.create_snapshot().await?;
        Ok(crate::lsp::models::HybridAnalysisResult {
            analysis_type: "hybrid".to_string(),
            results: vec![],
        })
    }
}