use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Language, Parser, Query, QueryCursor, Tree};

use crate::parser::symbols::*;

// Static list of indicators from the trading-ta crate
// Using static array to avoid allocations per file
static INDICATORS: &[&str] = &[
    "Alma", "ApproximateQuartiles", "Atr", "Bb", "Cvd", "CvdTrend",
    "DeltaVix", "Divergence", "Dmi", "Ema", "Lwpi", "Macd",
    "MultiLengthRsi", "OIIndicatorSuite", "Qama", "Rma", "Rsi", 
    "Sma", "Supertrail", "Supertrend", "Tdfi", "Trendilo", "Vwma"
];

pub struct RustParser {
    parser: Parser,
    query_cursor: QueryCursor,
    queries: QuerySet,
}

struct QuerySet {
    function_query: Query,
    type_query: Query,
    impl_query: Query,
    call_query: Query,
    import_query: Query,
    module_query: Query,
    actor_impl_query: Query,
    actor_spawn_query: Query,
    message_type_query: Query,
    message_handler_query: Query,
    actor_ref_query: Query,
    message_send_query: Query,
}

/// Represents a detected macro pattern in source code
#[derive(Debug, Clone)]
struct MacroPattern {
    line: usize,
    macro_type: String,
    pattern: String,
    method: String,  // The method being called (new, na, nan)
}

/// Helper struct for enhanced function context resolution
#[derive(Debug, Clone)]
struct FunctionWithRange {
    id: String,
    name: String,
    line_range: std::ops::Range<usize>,
}

/// Indicator resolution for trading indicators
#[derive(Debug, Clone)]
pub struct IndicatorResolver {
    static_indicators: Vec<String>,
}

impl IndicatorResolver {
    pub fn new() -> Self {
        Self {
            static_indicators: vec![
                "adx", "atr", "bb", "cci", "dmi", "ema", "fibonacci_retracement",
                "ichimoku_cloud", "keltner_channel", "macd", "mfi", "obv", "parabolic_sar",
                "roc", "rsi", "sma", "stochastic_oscillator", "trix", "ultimate_oscillator",
                "volume_profile", "williams_r", "wma", "zigzag"
            ].into_iter().map(String::from).collect()
        }
    }
    
    pub fn resolve_indicators(&self) -> &[String] {
        &self.static_indicators
    }
}

/// Enhanced pattern extraction from macro patterns
#[derive(Debug, Clone)]
struct PastePattern {
    method: String,
    variable: String,  // The $variable part
}

/// Synthetic call generator with proper caller context resolution
pub struct SyntheticCallGenerator {
    indicator_resolver: IndicatorResolver,
}

impl SyntheticCallGenerator {
    pub fn new() -> Self {
        Self {
            indicator_resolver: IndicatorResolver::new(),
        }
    }
    
    pub fn generate_calls_from_paste_macro(
        &self,
        expansion: &MacroExpansion,
        containing_function_id: &str,
    ) -> Vec<FunctionCall> {
        let pattern = self.extract_paste_pattern(&expansion.expansion_pattern);
        let indicators = self.indicator_resolver.resolve_indicators();
        
        indicators.iter().flat_map(|indicator| {
            // Convert CamelCase to snake_case for module name
            let snake_case_name = indicator.chars().enumerate().fold(String::new(), |mut acc, (i, c)| {
                if i > 0 && c.is_uppercase() {
                    acc.push('_');
                }
                acc.push(c.to_lowercase().next().unwrap());
                acc
            });
            
            // Special case mappings
            let module_name = match indicator.as_str() {
                "OIIndicatorSuite" => "oi_indicator_suite",
                "Divergence" => "divergences", // Plural form
                _ => &snake_case_name
            };
            
            let mut calls = Vec::new();
            
            // Determine the target type(s) based on the method being called
            if expansion.expansion_pattern.contains("Input>]") {
                // This is for Input struct (e.g., [<$indicator Input>]::from_ohlcv)
                let struct_name = format!("{}Input", indicator);
                let qualified = format!("{}::{}::{}::{}", 
                    expansion.crate_name.replace('-', "_"), module_name, struct_name, pattern.method);
                
                calls.push(FunctionCall {
                    caller_id: containing_function_id.to_string(),
                    caller_module: self.extract_module_from_expansion(expansion),
                    callee_name: pattern.method.clone(),
                    qualified_callee: Some(qualified),
                    call_type: CallType::Direct,
                    line: expansion.line_range.start,
                    cross_crate: false,
                    from_crate: expansion.crate_name.replace('-', "_"),
                    to_crate: Some(expansion.crate_name.replace('-', "_")),
                    file_path: expansion.file_path.clone(),
                    is_synthetic: true,
                    macro_context: Some(MacroContext {
                        expansion_id: expansion.id.clone(),
                        macro_type: expansion.macro_type.clone(),
                        expansion_site_line: expansion.line_range.start,
                    }),
                    synthetic_confidence: 0.95,
                });
            } else if pattern.method == "nan" || pattern.method == "na" || pattern.method == "nz" {
                // For NAN/NZ trait methods, generate calls to multiple possible output types
                let output_variants = vec![
                    format!("{}Output", indicator),           // Standard: CvdOutput
                    format!("{}Input", indicator),            // Input: CvdInput  
                    format!("{}TrendOutput", indicator),      // Trend: CvdTrendOutput
                    format!("{}UnifiedOutput", indicator),    // Unified: OIUnifiedOutput (for OIIndicatorSuite)
                ];
                
                for output_type in output_variants {
                    let qualified = format!("{}::{}::{}::{}", 
                        expansion.crate_name.replace('-', "_"), module_name, output_type, pattern.method);
                    
                    calls.push(FunctionCall {
                        caller_id: containing_function_id.to_string(),
                        caller_module: self.extract_module_from_expansion(expansion),
                        callee_name: pattern.method.clone(),
                        qualified_callee: Some(qualified),
                        call_type: CallType::Direct,
                        line: expansion.line_range.start,
                        cross_crate: false,
                        from_crate: expansion.crate_name.replace('-', "_"),
                        to_crate: Some(expansion.crate_name.replace('-', "_")),
                        file_path: expansion.file_path.clone(),
                        is_synthetic: true,
                        macro_context: Some(MacroContext {
                            expansion_id: expansion.id.clone(),
                            macro_type: expansion.macro_type.clone(),
                            expansion_site_line: expansion.line_range.start,
                        }),
                        synthetic_confidence: 0.7, // Lower confidence since we're guessing types
                    });
                }
            } else {
                // This is for the indicator itself (e.g., [<$indicator>]::new)
                let qualified = format!("{}::{}::{}::{}", 
                    expansion.crate_name.replace('-', "_"), module_name, indicator, pattern.method);
                
                calls.push(FunctionCall {
                    caller_id: containing_function_id.to_string(),
                    caller_module: self.extract_module_from_expansion(expansion),
                    callee_name: pattern.method.clone(),
                    qualified_callee: Some(qualified),
                    call_type: CallType::Direct,
                    line: expansion.line_range.start,
                    cross_crate: false,
                    from_crate: expansion.crate_name.replace('-', "_"),
                    to_crate: Some(expansion.crate_name.replace('-', "_")),
                    file_path: expansion.file_path.clone(),
                    is_synthetic: true,
                    macro_context: Some(MacroContext {
                        expansion_id: expansion.id.clone(),
                        macro_type: expansion.macro_type.clone(),
                        expansion_site_line: expansion.line_range.start,
                    }),
                    synthetic_confidence: 0.95,
                });
            }
            
            calls
        }).collect()
    }
    
    fn extract_paste_pattern(&self, pattern: &str) -> PastePattern {
        // Extract method name from patterns like: paste! { [<$indicator>]::new(config) }
        let method = if pattern.contains("::new(") {
            "new"
        } else if pattern.contains("::from_ohlcv(") || pattern.contains("::from_ohlcv ") {
            "from_ohlcv"
        } else if pattern.contains("::na(") {
            "na"
        } else if pattern.contains("::nan(") {
            "nan"
        } else if pattern.contains("::nz(") {
            "nz"
        } else {
            "new" // Default fallback
        };
        
        // Extract variable (simplified - could be enhanced with regex)
        let variable = if pattern.contains("$indicator") {
            "indicator"
        } else {
            "unknown"
        };
        
        PastePattern {
            method: method.to_string(),
            variable: variable.to_string(),
        }
    }
    
    fn extract_module_from_expansion(&self, expansion: &MacroExpansion) -> String {
        // Extract module from file path - simplified implementation
        std::path::Path::new(&expansion.file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string()
    }
}

impl RustParser {
    pub fn new() -> Result<Self> {
        let language = tree_sitter_rust::language();
        let mut parser = Parser::new();
        parser
            .set_language(&language)
            .map_err(|e| anyhow::anyhow!("Failed to set language: {}", e))?;

        let queries = QuerySet::new(language)?;

        Ok(Self {
            parser,
            query_cursor: QueryCursor::new(),
            queries,
        })
    }

    pub fn parse_file(&mut self, file_path: &Path, crate_name: &str) -> Result<ParsedSymbols> {
        // Debug logging removed to reduce verbosity
        let source = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {:?}", file_path))?;

        self.parse_source(&source, file_path, crate_name)
    }

    pub fn parse_source(
        &mut self,
        source: &str,
        file_path: &Path,
        crate_name: &str,
    ) -> Result<ParsedSymbols> {
        let tree = self.parser.parse(source, None).ok_or_else(|| {
            anyhow::anyhow!("Failed to parse source for: {}", file_path.display())
        })?;

        let mut symbols = ParsedSymbols::new();
        let source_bytes = source.as_bytes();

        // Detect if this is a test file
        let is_test = self.is_test_file(file_path);

        symbols
            .modules
            .extend(self.extract_modules(&tree, source_bytes, file_path, crate_name)?);

        symbols
            .imports
            .extend(self.extract_imports(&tree, source_bytes, file_path)?);

        symbols.functions.extend(self.extract_functions(
            &tree,
            source_bytes,
            file_path,
            crate_name,
            is_test,
        )?);

        symbols.types.extend(self.extract_types(
            &tree,
            source_bytes,
            file_path,
            crate_name,
            is_test,
        )?);

        let impls = self.extract_impls(&tree, source_bytes, file_path, crate_name)?;
        
        // Add impl methods to the functions list (they are properly marked as trait implementations)
        for impl_block in &impls {
            symbols.functions.extend(impl_block.methods.clone());
        }
        
        symbols.impls.extend(impls);

        // Extract calls, passing functions so macro expansion can find containing functions
        let calls = self.extract_calls(&tree, source_bytes, file_path, crate_name, &symbols.functions)?;
        symbols.calls.extend(calls);

        symbols.actors.extend(self.extract_actors(
            &tree,
            source_bytes,
            file_path,
            crate_name,
            is_test,
        )?);

        symbols.actor_spawns.extend(self.extract_actor_spawns(
            &tree,
            source_bytes,
            file_path,
            crate_name,
        )?);

        // Extract distributed actors and message flows
        symbols
            .distributed_actors
            .extend(self.extract_distributed_actors(
                &tree,
                source_bytes,
                file_path,
                crate_name,
                is_test,
            )?);

        symbols.message_types.extend(self.extract_message_types(
            &tree,
            source_bytes,
            file_path,
            crate_name,
        )?);

        symbols
            .message_handlers
            .extend(self.extract_message_handlers(&tree, source_bytes, file_path, crate_name)?);

        // Link message handlers to actors' local_messages field
        self.link_message_handlers_to_actors(&mut symbols);

        // Extract distributed message flows (messages being sent between actors)
        symbols
            .distributed_message_flows
            .extend(self.extract_distributed_message_flows(
                &tree,
                source_bytes,
                file_path,
                crate_name,
            )?);

        let actor_ref_map = self.detect_actor_ref_variables(&tree, source_bytes)?;

        let message_sends = self.extract_message_sends(
            &tree,
            source_bytes,
            file_path,
            crate_name,
            &actor_ref_map,
        )?;
        
        // Process message sends silently
        
        symbols.message_sends.extend(message_sends);

        // Extract macro expansions (specifically paste! macros for trading indicators)
        symbols.macro_expansions.extend(self.extract_macro_expansions(
            source_bytes,
            file_path,
            crate_name,
        )?);

        Ok(symbols)
    }

    fn is_test_file(&self, file_path: &Path) -> bool {
        // Check if file is in tests/ or examples/ directory
        if file_path.components().any(|c| c.as_os_str() == "tests" || c.as_os_str() == "examples") {
            return true;
        }

        // Check if file name ends with _test.rs or _tests.rs
        if let Some(file_name) = file_path.file_name() {
            let name = file_name.to_string_lossy();
            if name.ends_with("_test.rs")
                || name.ends_with("_tests.rs")
                || name == "test.rs"
                || name == "tests.rs"
            {
                return true;
            }
        }

        // Check if file is in benches/ directory
        if file_path.components().any(|c| c.as_os_str() == "benches") {
            return true;
        }

        false
    }

    fn extract_functions(
        &mut self,
        tree: &Tree,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
        is_test: bool,
    ) -> Result<Vec<RustFunction>> {
        let mut functions = Vec::new();
        let mut seen_functions = std::collections::HashSet::new();

        // Create a new QueryCursor to avoid borrow conflicts
        let mut cursor = tree_sitter::QueryCursor::new();
        let mut match_count = 0;

        for query_match in cursor.matches(&self.queries.function_query, tree.root_node(), source) {
            match_count += 1;
            
            // Skip functions that are inside impl blocks - they'll be handled by extract_impls
            let mut is_in_impl = false;
            for capture in query_match.captures.iter() {
                if capture.node.parent().map_or(false, |p| p.kind() == "declaration_list") {
                    if capture.node.parent()
                        .and_then(|p| p.parent())
                        .map_or(false, |pp| pp.kind() == "impl_item") {
                        is_in_impl = true;
                        break;
                    }
                }
            }
            
            if is_in_impl {
                continue; // Skip this function, it will be handled by extract_impls
            }
            
            if let Some(function) =
                self.parse_function_match(query_match, source, file_path, crate_name, is_test)?
            {
                // Use qualified name + line number as unique identifier
                let function_key = format!("{}:{}", function.qualified_name, function.line_start);
                if seen_functions.insert(function_key) {
                    functions.push(function);
                }
            }
        }

        Ok(functions)
    }

    fn extract_types(
        &mut self,
        tree: &Tree,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
        is_test: bool,
    ) -> Result<Vec<RustType>> {
        let mut types = Vec::new();
        let mut seen_types = std::collections::HashSet::new();

        let mut cursor = tree_sitter::QueryCursor::new();
        let mut match_count = 0;

        for query_match in cursor.matches(&self.queries.type_query, tree.root_node(), source) {
            match_count += 1;

            // Use std::panic::catch_unwind to handle segfaults gracefully
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.parse_type_match(query_match, source, file_path, crate_name, is_test)
            }));

            match result {
                Ok(Ok(Some(rust_type))) => {
                    // Use qualified name + line number as unique identifier
                    let type_key = format!("{}:{}", rust_type.qualified_name, rust_type.line_start);
                    if seen_types.insert(type_key) {
                        types.push(rust_type);
                    } else {
                    }
                }
                Ok(Ok(None)) => {}
                Ok(Err(e)) => {
                    // Continue processing other types instead of failing entirely
                    continue;
                }
                Err(_panic_info) => {
                    // Continue processing other types despite the panic
                    continue;
                }
            }
        }

        Ok(types)
    }

    fn extract_impls(
        &mut self,
        tree: &Tree,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
    ) -> Result<Vec<RustImpl>> {
        let mut impls = Vec::new();
        let mut seen_impl_locations = std::collections::HashSet::new();

        let mut cursor = tree_sitter::QueryCursor::new();

        for query_match in cursor.matches(&self.queries.impl_query, tree.root_node(), source) {
            if let Some(rust_impl) =
                self.parse_impl_match(query_match, source, file_path, crate_name)?
            {
                // Deduplicate impl blocks by their location (type_name + line_start)
                // When duplicates exist, prefer the one with a trait name
                let impl_key = format!("{}:{}", rust_impl.type_name, rust_impl.line_start);
                
                // Check if we already have an impl at this location
                if let Some(existing_idx) = impls.iter().position(|i: &RustImpl| 
                    format!("{}:{}", i.type_name, i.line_start) == impl_key
                ) {
                    // If the new one has a trait name and the existing doesn't, replace it
                    if rust_impl.trait_name.is_some() && impls[existing_idx].trait_name.is_none() {
                        impls[existing_idx] = rust_impl;
                    }
                    // Otherwise keep the existing one (which might already have a trait name)
                } else {
                    // New impl block, add it
                    seen_impl_locations.insert(impl_key);
                    impls.push(rust_impl);
                }
            }
        }

        Ok(impls)
    }

    fn extract_calls(
        &mut self,
        tree: &Tree,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
        functions: &[RustFunction],
    ) -> Result<Vec<FunctionCall>> {
        let mut calls = Vec::new();

        let mut cursor = tree_sitter::QueryCursor::new();

        for query_match in cursor.matches(&self.queries.call_query, tree.root_node(), source) {
            if let Some(call) = self.parse_call_match(query_match, source, file_path, crate_name)? {
                calls.push(call);
            }
        }
        
        // Add synthetic calls from macro expansions
        let synthetic_calls = self.extract_macro_generated_calls(source, file_path, crate_name, functions)?;
        calls.extend(synthetic_calls);

        Ok(calls)
    }

    fn extract_imports(
        &mut self,
        tree: &Tree,
        source: &[u8],
        file_path: &Path,
    ) -> Result<Vec<RustImport>> {
        let mut imports = Vec::new();

        let mut cursor = tree_sitter::QueryCursor::new();

        for query_match in cursor.matches(&self.queries.import_query, tree.root_node(), source) {
            if let Some(import) = self.parse_import_match(query_match, source, file_path)? {
                imports.push(import);
            }
        }

        Ok(imports)
    }

    fn extract_modules(
        &mut self,
        tree: &Tree,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
    ) -> Result<Vec<RustModule>> {
        let mut modules = Vec::new();

        let mut cursor = tree_sitter::QueryCursor::new();

        for query_match in cursor.matches(&self.queries.module_query, tree.root_node(), source) {
            if let Some(module) =
                self.parse_module_match(query_match, source, file_path, crate_name)?
            {
                modules.push(module);
            }
        }

        Ok(modules)
    }

    fn extract_actors(
        &mut self,
        tree: &Tree,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
        is_test: bool,
    ) -> Result<Vec<RustActor>> {
        let mut actors = Vec::new();
        let mut seen_actors = std::collections::HashSet::new();

        let mut cursor = tree_sitter::QueryCursor::new();

        // Find impl Actor for SomeActor blocks
        for query_match in cursor.matches(&self.queries.actor_impl_query, tree.root_node(), source)
        {
            if let Some(actor) =
                self.parse_actor_impl_match(query_match, source, file_path, crate_name, is_test)?
            {
                // CRITICAL FIX: Deduplicate actors by (actor_name, crate_name)
                let actor_key = (actor.name.clone(), actor.crate_name.clone());
                if seen_actors.insert(actor_key) {
                    actors.push(actor);
                }
            }
        }

        // DISABLED: Actor inference creates too many false positives
        // Only detect actors from explicit "impl Actor for Type" declarations
        // This prevents marking regular types like Signal, Trend, DataPoint as actors

        // Task 1.2: Infer actors from spawn calls - DISABLED
        // Causes false positives when non-actors are spawned
        // let inferred_actors = self.infer_actors_from_spawns(tree, source, file_path, crate_name)?;

        // Task 1.1: Find actors with #[derive(Actor)] macro pattern - DISABLED
        // Not all derives are for actors
        // let derive_actors = self.extract_derive_actors(tree, source, file_path, crate_name)?;

        // Task 1.3: Find actors from ActorRef<T> usage - DISABLED
        // Creates false positives from type names that happen to contain "Actor"
        // let type_usage_actors = self.extract_actors_from_type_usage(tree, source, file_path, crate_name)?;

        // Extract actors from Message implementations
        let message_impl_actors = self.extract_actors_from_message_impls(tree, source, file_path, crate_name, is_test)?;
        
        // Merge actors, avoiding duplicates
        for message_actor in message_impl_actors {
            let actor_key = (message_actor.name.clone(), message_actor.crate_name.clone());
            if seen_actors.insert(actor_key) {
                actors.push(message_actor);
            }
        }

        Ok(actors)
    }
    
    fn extract_actors_from_message_impls(
        &mut self,
        tree: &Tree,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
        is_test: bool,
    ) -> Result<Vec<RustActor>> {
        let mut actors = Vec::new();
        let mut seen_actors = std::collections::HashSet::new();
        let mut cursor = tree_sitter::QueryCursor::new();
        
        // Find all impl Message<T> for Type patterns
        for query_match in cursor.matches(&self.queries.message_handler_query, tree.root_node(), source) {
            let captures: HashMap<&str, (tree_sitter::Node, &str)> = query_match
                .captures
                .iter()
                .filter_map(|capture| {
                    let name = &self.queries.message_handler_query.capture_names()[capture.index as usize];
                    match capture.node.utf8_text(source) {
                        Ok(text) => Some((*name, (capture.node, text))),
                        Err(_) => None,
                    }
                })
                .collect();
            
            // Check if this is a Message trait implementation
            if let Some((_, trait_name)) = captures.get("trait_name") {
                if *trait_name != "Message" {
                    continue;
                }
            } else {
                continue;
            }
            
            // Get the actor type from the impl
            if let Some((actor_node, actor_type_raw)) = captures.get("actor_type") {
                // Extract just the type name from scoped paths like super::data::OpenInterestDataActor
                let actor_type = if actor_type_raw.contains("::") {
                    actor_type_raw.split("::").last().unwrap_or(actor_type_raw).to_string()
                } else {
                    actor_type_raw.to_string()
                };
                
                let actor_key = (actor_type.clone(), crate_name.to_string());
                if seen_actors.insert(actor_key) {
                    // Get the message type being handled
                    let message_type = captures
                        .get("message_type")
                        .map(|(_, t)| t.to_string())
                        .unwrap_or_default();
                    
                    // Infer module path from file location
                    let module_path = self.infer_module_path_with_crate(file_path, crate_name)?;
                    let qualified_name = if module_path == "crate" {
                        format!("{}::{}", crate_name, &actor_type)
                    } else {
                        format!("{}::{}", module_path, &actor_type)
                    };
                    
                    let line_start = actor_node.start_position().row + 1;
                    let line_end = actor_node.end_position().row + 1;
                    
                    let actor = RustActor {
                        id: format!("{}::{}", crate_name, &actor_type),
                        name: actor_type.clone(),
                        qualified_name,
                        crate_name: crate_name.to_string(),
                        module_path,
                        file_path: file_path.display().to_string(),
                        line_start,
                        line_end,
                        visibility: "pub".to_string(), // Default to pub for inferred actors
                        doc_comment: None,
                        is_distributed: false, // Will be updated if we find distributed_actor! macro
                        is_test,
                        actor_type: ActorImplementationType::Local,
                        local_messages: vec![message_type], // Start with the message we found
                        inferred_from_message: true, // Mark as inferred from Message impl
                    };
                    
                    actors.push(actor);
                }
            }
        }
        
        Ok(actors)
    }

    fn extract_actor_spawns(
        &mut self,
        tree: &Tree,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
    ) -> Result<Vec<ActorSpawn>> {
        let mut spawns = Vec::new();
        let mut seen_spawns = std::collections::HashSet::new();

        let mut cursor = tree_sitter::QueryCursor::new();

        // Find SomeActor::spawn() calls
        for query_match in cursor.matches(&self.queries.actor_spawn_query, tree.root_node(), source)
        {
            if let Some(spawn) =
                self.parse_actor_spawn_match(query_match, source, file_path, crate_name)?
            {
                // CRITICAL FIX: Deduplicate spawn relationships by (parent, child, file, line)
                let spawn_key = (
                    spawn.parent_actor_name.clone(),
                    spawn.child_actor_name.clone(),
                    spawn.file_path.clone(),
                    spawn.line,
                );

                if seen_spawns.insert(spawn_key) {
                    spawns.push(spawn);
                }
            }
        }

        Ok(spawns)
    }

    fn extract_distributed_actors(
        &mut self,
        tree: &Tree,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
        is_test: bool,
    ) -> Result<Vec<crate::parser::symbols::DistributedActor>> {
        let mut distributed_actors = Vec::new();

        // Look for distributed_actor! macro invocations
        let query_text = r#"
        (macro_invocation
          (scoped_identifier
            path: (identifier) @namespace (#eq? @namespace "kameo")
            name: (identifier) @macro_name (#eq? @macro_name "distributed_actor"))
          (token_tree
            (identifier) @actor_name
            (token_tree
              (identifier) @message_type)*
          )
        ) @distributed_actor
        "#;

        let query = tree_sitter::Query::new(&tree.language(), query_text)
            .map_err(|e| anyhow::anyhow!("Failed to create distributed actor query: {}", e))?;

        let mut cursor = tree_sitter::QueryCursor::new();
        let matches: Vec<_> = cursor.matches(&query, tree.root_node(), source).collect();

        // Group matches by actor (same actor can have multiple matches for different message types)
        let mut actor_map: HashMap<String, (usize, Vec<String>)> = HashMap::new();

        for match_ in matches {
            let captures: HashMap<&str, (tree_sitter::Node, &str)> = match_
                .captures
                .iter()
                .filter_map(|capture| {
                    let name = &query.capture_names()[capture.index as usize];
                    match capture.node.utf8_text(source) {
                        Ok(text) => Some((*name, (capture.node, text))),
                        Err(_) => None,
                    }
                })
                .collect();

            if let Some((_, actor_name)) = captures.get("actor_name") {
                let line = match_
                    .captures
                    .first()
                    .map(|c| c.node.start_position().row + 1)
                    .unwrap_or(0);

                // Collect all message types for this match
                let mut message_types = Vec::new();
                for capture in match_.captures {
                    let name = &query.capture_names()[capture.index as usize];
                    if *name == "message_type" {
                        if let Ok(msg_type) = capture.node.utf8_text(source) {
                            message_types.push(msg_type.to_string());
                        }
                    }
                }

                // Add to actor map, merging message types
                let actor_key = format!("{}:{}", actor_name, line);
                match actor_map.get_mut(&actor_key) {
                    Some((_, existing_messages)) => {
                        existing_messages.extend(message_types);
                    }
                    None => {
                        actor_map.insert(actor_key.clone(), (line, message_types));
                    }
                }
            }
        }

        // Convert actor map to DistributedActor structs
        for (actor_key, (line, distributed_messages)) in actor_map {
            let actor_name = actor_key.split(':').next().unwrap_or("").to_string();
            let distributed_actor = crate::parser::symbols::DistributedActor {
                id: format!("{}::{}:{}", crate_name, actor_name, line),
                actor_name,
                crate_name: crate_name.to_string(),
                file_path: file_path.to_string_lossy().to_string(),
                line,
                is_test,
                distributed_messages,
                local_messages: Vec::new(), // Will be populated later by link_message_handlers_to_actors
            };
            distributed_actors.push(distributed_actor);
        }

        Ok(distributed_actors)
    }

    fn extract_distributed_message_flows(
        &mut self,
        tree: &Tree,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
    ) -> Result<Vec<crate::parser::symbols::DistributedMessageFlow>> {
        let mut message_flows = Vec::new();

        // First, collect all variable assignments to message types
        let message_var_types = self.extract_message_variable_types(tree, source)?;

        // Query for distributed_ref.tell(message) and distributed_ref.ask(message) patterns
        // Also captures struct expressions to find the actual message type
        let query_text = r#"
        (call_expression
          function: (field_expression
            value: (identifier) @ref_name
            field: (field_identifier) @method (#match? @method "^(tell|ask)$")
          )
          arguments: (arguments
            [
              (identifier) @message_var
              (struct_expression
                name: (type_identifier) @message_type
              )
            ]
          )
        ) @message_send
        "#;

        let query = tree_sitter::Query::new(&tree.language(), query_text)
            .map_err(|e| anyhow::anyhow!("Failed to create message flow query: {}", e))?;

        let mut cursor = tree_sitter::QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), source);

        for match_ in matches {
            let captures: HashMap<&str, &str> = match_
                .captures
                .iter()
                .filter_map(|capture| {
                    let name = &query.capture_names()[capture.index as usize];
                    match capture.node.utf8_text(source) {
                        Ok(text) => Some((*name, text)),
                        Err(_) => None,
                    }
                })
                .collect();

            // Get the message type - either from direct struct or from variable
            let message_type_str = captures.get("message_type");
            let message_var_str = captures.get("message_var");

            if let (Some(ref_name), Some(method)) =
                (captures.get("ref_name"), captures.get("method"))
            {
                let line = match_
                    .captures
                    .first()
                    .map(|c| c.node.start_position().row + 1)
                    .unwrap_or(0);

                // Determine the actual message type
                let actual_message_type = if let Some(msg_type) = message_type_str {
                    // Direct struct expression - use the type name
                    msg_type.to_string()
                } else if let Some(var_name) = message_var_str {
                    // Variable reference - look up the type
                    message_var_types
                        .get(*var_name)
                        .cloned()
                        .unwrap_or_else(|| var_name.to_string())
                } else {
                    continue;
                };

                // Determine send method
                let send_method = match *method {
                    "tell" => crate::parser::symbols::MessageSendMethod::Tell,
                    "ask" => crate::parser::symbols::MessageSendMethod::Ask,
                    _ => continue,
                };

                // Extract context (function/method name)
                let sender_context = self
                    .get_enclosing_function_name(tree, match_.captures[0].node, source)
                    .unwrap_or_else(|| "unknown".to_string());

                // Try to infer sender actor from context
                let sender_actor = if sender_context.contains("::") {
                    sender_context
                        .split("::")
                        .next()
                        .unwrap_or("unknown")
                        .to_string()
                } else {
                    sender_context.clone()
                };

                // Target actor is inferred from the variable name
                let target_actor = ref_name
                    .to_string()
                    .replace("_distributed_ref", "")
                    .replace("_ref", "")
                    .replace("executor", "CryptoFuturesStrategyExecutor")
                    .replace("ta_manager", "TAManagerActor");

                let message_flow = crate::parser::symbols::DistributedMessageFlow {
                    id: format!(
                        "{}::{}::{}:{}",
                        crate_name, sender_actor, actual_message_type, line
                    ),
                    message_type: actual_message_type,
                    sender_actor,
                    sender_context: sender_context.clone(),
                    sender_crate: crate_name.to_string(),
                    target_actor,
                    target_crate: crate_name.to_string(), // Assume same crate for now
                    send_method,
                    send_location: crate::parser::symbols::MessageSendLocation {
                        file_path: file_path.to_string_lossy().to_string(),
                        line,
                        function_context: sender_context,
                    },
                };

                message_flows.push(message_flow);
            }
        }

        Ok(message_flows)
    }

    fn extract_message_variable_types(
        &self,
        tree: &Tree,
        source: &[u8],
    ) -> Result<HashMap<String, String>> {
        let mut var_types = HashMap::new();

        // Query for let statements that assign message structs
        let query_text = r#"
        (let_declaration
          pattern: (identifier) @var_name
          value: (struct_expression
            name: (type_identifier) @type_name
          )
        ) @assignment
        "#;

        let query = tree_sitter::Query::new(&tree.language(), query_text)?;
        let mut cursor = tree_sitter::QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), source);

        for match_ in matches {
            let mut var_name = None;
            let mut type_name = None;

            for capture in match_.captures {
                let name = &query.capture_names()[capture.index as usize];
                if let Ok(text) = capture.node.utf8_text(source) {
                    match *name {
                        "var_name" => var_name = Some(text.to_string()),
                        "type_name" => type_name = Some(text.to_string()),
                        _ => {}
                    }
                }
            }

            if let (Some(var), Some(typ)) = (var_name, type_name) {
                var_types.insert(var, typ);
            }
        }

        Ok(var_types)
    }

    fn get_enclosing_function_name(
        &self,
        _tree: &Tree,
        node: tree_sitter::Node,
        source: &[u8],
    ) -> Option<String> {
        let mut current = node;

        while let Some(parent) = current.parent() {
            if parent.kind() == "function_item" || parent.kind() == "impl_item" {
                // Find the name of the function
                for child in parent.children(&mut parent.walk()) {
                    if child.kind() == "identifier" {
                        if let Ok(name) = child.utf8_text(source) {
                            return Some(name.to_string());
                        }
                    }
                }
            }
            current = parent;
        }

        None
    }

    fn parse_function_match(
        &self,
        match_: tree_sitter::QueryMatch,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
        is_test_file: bool,
    ) -> Result<Option<RustFunction>> {
        let mut captures = HashMap::new();
        for capture in match_.captures {
            let node = capture.node;
            let text = node.utf8_text(source)?;
            let capture_name = self.queries.function_query.capture_names()[capture.index as usize];
            captures.insert(capture_name, (node, text));
        }

        if let Some((name_node, name)) = captures.get("name") {
            let visibility = captures
                .get("visibility")
                .map(|(_, vis)| vis.to_string())
                .unwrap_or_else(|| "private".to_string());

            let doc_comment = self.extract_doc_comment(name_node.parent(), source)?;
            let is_async = captures.contains_key("async");
            let is_unsafe = captures.contains_key("unsafe");
            let is_generic = captures.contains_key("generic");

            // Check if function has test attributes or is in a test file
            let is_test = is_test_file || self.is_test_function(&captures, source)?;

            let parameters = self.extract_parameters(&captures, source)?;
            let return_type = captures.get("return_type").map(|(_, ret)| ret.to_string());
            let signature = self.extract_function_signature(&captures, source)?;

            let module_path = self.infer_module_path_with_crate(file_path, crate_name)?;
            let qualified_name = format!("{}::{}", module_path, name);

            let mut function = RustFunction {
                id: String::new(),
                name: name.to_string(),
                qualified_name,
                crate_name: crate_name.to_string(),
                module_path: module_path.clone(),
                file_path: file_path.to_string_lossy().to_string(),
                line_start: name_node.start_position().row + 1,
                line_end: name_node.end_position().row + 1,
                visibility,
                is_async,
                is_unsafe,
                is_generic,
                is_test,
                is_trait_impl: false,  // This will be set to true when processing impl blocks
                doc_comment,
                signature,
                parameters,
                return_type,
                embedding_text: None,
                module: module_path.clone(),
            };

            function.generate_id();
            return Ok(Some(function));
        }

        Ok(None)
    }

    fn parse_type_match(
        &self,
        match_: tree_sitter::QueryMatch,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
        is_test: bool,
    ) -> Result<Option<RustType>> {
        // Wrap capture processing in catch_unwind to handle segfaults gracefully
        let captures_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || -> Result<HashMap<&str, (tree_sitter::Node, &str)>> {
                let mut captures = HashMap::new();
                for capture in match_.captures {
                    let node = capture.node;
                    let text = node.utf8_text(source)?;
                    let capture_name =
                        self.queries.type_query.capture_names()[capture.index as usize];
                    captures.insert(capture_name, (node, text));
                }
                Ok(captures)
            },
        ));

        let captures = match captures_result {
            Ok(Ok(c)) => c,
            Ok(Err(_e)) => {
                return Ok(None);
            }
            Err(_) => {
                return Ok(None);
            }
        };

        if let Some((name_node, name)) = captures.get("name") {
            let kind = if captures.contains_key("struct") {
                TypeKind::Struct
            } else if captures.contains_key("enum") {
                TypeKind::Enum
            } else if captures.contains_key("trait") {
                TypeKind::Trait
            } else if captures.contains_key("type_alias") {
                TypeKind::TypeAlias
            } else if captures.contains_key("union") {
                TypeKind::Union
            } else {
                TypeKind::Struct
            };

            let visibility = captures
                .get("visibility")
                .map(|(_, vis)| vis.to_string())
                .unwrap_or_else(|| "private".to_string());

            let doc_comment = None; // Skip doc comment extraction to avoid segfaults
            let is_generic = false; // Skip generic detection since we removed it from query

            let fields = Vec::new(); // Skip field extraction to avoid segfaults
            let variants = Vec::new(); // Skip variant extraction to avoid segfaults

            let module_path = self.infer_module_path_with_crate(file_path, crate_name)?;
            let qualified_name = format!("{}::{}", module_path, name);

            let mut rust_type = RustType {
                id: String::new(),
                name: name.to_string(),
                qualified_name,
                crate_name: crate_name.to_string(),
                module_path: module_path.clone(),
                file_path: file_path.to_string_lossy().to_string(),
                line_start: name_node.start_position().row + 1,
                line_end: name_node.end_position().row + 1,
                kind,
                visibility,
                is_generic,
                is_test,
                doc_comment,
                fields,
                variants,
                methods: Vec::new(),
                embedding_text: None,
                type_kind: format!("{:?}", kind),
                module: module_path.clone(),
            };

            rust_type.generate_id();
            return Ok(Some(rust_type));
        }

        Ok(None)
    }

    fn parse_impl_match(
        &self,
        match_: tree_sitter::QueryMatch,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
    ) -> Result<Option<RustImpl>> {
        let mut captures = HashMap::new();
        for capture in match_.captures {
            let node = capture.node;
            let text = node.utf8_text(source)?;
            let capture_name = self.queries.impl_query.capture_names()[capture.index as usize];
            captures.insert(capture_name, (node, text));
        }

        if let Some((type_name_node, type_name)) = captures.get("type_name") {
            let trait_name = captures.get("trait_name").map(|(_, name)| name.to_string());
            let module_path = self.infer_module_path_with_crate(file_path, crate_name)?;
            let qualified_type_name = format!("{}::{}", module_path, type_name);

            // Extract ALL methods from the impl block body
            let mut methods = Vec::new();
            if let Some((body_node, _)) = captures.get("body") {
                // Parse each function item in the impl body
                for child_idx in 0..body_node.child_count() {
                    if let Some(child) = body_node.child(child_idx) {
                        if child.kind() == "function_item" {
                            if let Some(function) = self.parse_function_from_node_with_trait(child, source, file_path, crate_name, false, trait_name.as_deref())? {
                                methods.push(function);
                            }
                        }
                    }
                }
            }

            let rust_impl = RustImpl {
                type_name: type_name.to_string(),
                trait_name: trait_name.clone(),
                methods,
                file_path: file_path.to_string_lossy().to_string(),
                line_start: type_name_node.start_position().row + 1,
                line_end: type_name_node.end_position().row + 1,
                is_generic: captures.contains_key("generic"),
            };

            return Ok(Some(rust_impl));
        }

        Ok(None)
    }

    /// Parse a function directly from a tree-sitter function_item node
    fn parse_function_from_node(
        &self,
        function_node: tree_sitter::Node,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
        is_test_file: bool,
    ) -> Result<Option<RustFunction>> {
        self.parse_function_from_node_with_trait(function_node, source, file_path, crate_name, is_test_file, None)
    }

    /// Parse a function directly from a tree-sitter function_item node with optional trait context
    fn parse_function_from_node_with_trait(
        &self,
        function_node: tree_sitter::Node,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
        is_test_file: bool,
        trait_name: Option<&str>,
    ) -> Result<Option<RustFunction>> {
        // Extract function name
        let name_node = function_node.child_by_field_name("name");
        if let Some(name_node) = name_node {
            let name = name_node.utf8_text(source)?;
            
            // Extract visibility (look for pub keyword before function)
            let visibility = if let Some(visibility_node) = function_node.child(0) {
                if visibility_node.kind() == "visibility_modifier" {
                    "pub".to_string()
                } else {
                    "private".to_string()
                }
            } else {
                "private".to_string()
            };

            // Check for async and unsafe keywords
            let mut is_async = false;
            let mut is_unsafe = false;
            let mut is_generic = false;
            
            for child_idx in 0..function_node.child_count() {
                if let Some(child) = function_node.child(child_idx) {
                    match child.kind() {
                        "async" => is_async = true,
                        "unsafe" => is_unsafe = true,
                        "type_parameters" => is_generic = true,
                        _ => {}
                    }
                }
            }

            // Extract parameters
            let parameters = if let Some(params_node) = function_node.child_by_field_name("parameters") {
                self.extract_parameters_from_node(params_node, source)?
            } else {
                Vec::new()
            };

            // Extract return type
            let return_type = if let Some(return_node) = function_node.child_by_field_name("return_type") {
                Some(return_node.utf8_text(source)?.to_string())
            } else {
                None
            };

            // Build function signature
            let signature = self.build_signature(name, &parameters, &return_type, is_async, is_unsafe);

            let module_path = self.infer_module_path_with_crate(file_path, crate_name)?;
            let qualified_name = format!("{}::{}", module_path, name);

            let mut function = RustFunction {
                id: String::new(),
                name: name.to_string(),
                qualified_name,
                crate_name: crate_name.to_string(),
                module_path: module_path.clone(),
                file_path: file_path.to_string_lossy().to_string(),
                line_start: name_node.start_position().row + 1,
                line_end: function_node.end_position().row + 1,
                visibility,
                is_async,
                is_unsafe,
                is_generic,
                is_test: is_test_file || name == "test" || name.starts_with("test_"),
                is_trait_impl: trait_name.is_some(),  // True if this is part of a trait implementation
                doc_comment: None, // We'll extract this if needed
                signature,
                parameters,
                return_type,
                embedding_text: None,
                module: module_path,
            };

            function.generate_id();
            return Ok(Some(function));
        }

        Ok(None)
    }

    /// Extract parameters from a parameters node
    fn extract_parameters_from_node(
        &self,
        params_node: tree_sitter::Node,
        source: &[u8],
    ) -> Result<Vec<Parameter>> {
        let mut parameters = Vec::new();

        for child_idx in 0..params_node.child_count() {
            if let Some(child) = params_node.child(child_idx) {
                if child.kind() == "parameter" {
                    if let Some(param) = self.parse_parameter_node(child, source)? {
                        parameters.push(param);
                    }
                }
            }
        }

        Ok(parameters)
    }

    /// Parse a single parameter node
    fn parse_parameter_node(
        &self,
        param_node: tree_sitter::Node,
        source: &[u8],
    ) -> Result<Option<Parameter>> {
        let mut pattern_node = None;
        let mut type_node = None;

        for child_idx in 0..param_node.child_count() {
            if let Some(child) = param_node.child(child_idx) {
                match child.kind() {
                    "identifier" => pattern_node = Some(child),
                    "self" => {
                        return Ok(Some(Parameter {
                            name: "self".to_string(),
                            param_type: "Self".to_string(),
                            is_self: true,
                            is_mutable: false,
                        }));
                    }
                    _ if child.kind().contains("type") => type_node = Some(child),
                    _ => {}
                }
            }
        }

        if let (Some(pattern), Some(type_node)) = (pattern_node, type_node) {
            let name = pattern.utf8_text(source)?;
            let param_type = type_node.utf8_text(source)?;

            Ok(Some(Parameter {
                name: name.to_string(),
                param_type: param_type.to_string(),
                is_self: false,
                is_mutable: name.starts_with("mut "),
            }))
        } else {
            Ok(None)
        }
    }

    /// Build function signature string
    fn build_signature(
        &self,
        name: &str,
        parameters: &[Parameter],
        return_type: &Option<String>,
        is_async: bool,
        is_unsafe: bool,
    ) -> String {
        let mut sig = String::new();
        
        if is_unsafe {
            sig.push_str("unsafe ");
        }
        if is_async {
            sig.push_str("async ");
        }
        
        sig.push_str("fn ");
        sig.push_str(name);
        sig.push('(');
        
        for (i, param) in parameters.iter().enumerate() {
            if i > 0 {
                sig.push_str(", ");
            }
            sig.push_str(&param.name);
            sig.push_str(": ");
            sig.push_str(&param.param_type);
        }
        
        sig.push(')');
        
        if let Some(ret) = return_type {
            sig.push_str(" -> ");
            sig.push_str(ret);
        }
        
        sig
    }

    fn find_containing_context(
        &self,
        mut node: tree_sitter::Node,
        source: &[u8],
        file_path: &Path,
    ) -> Result<String> {
        // Find the containing function and use it as context
        while let Some(parent) = node.parent() {
            if parent.kind() == "function_item" {
                // Get the function name
                for child_idx in 0..parent.child_count() {
                    if let Some(child) = parent.child(child_idx) {
                        if child.kind() == "identifier" {
                            if let Ok(func_name) = child.utf8_text(source) {
                                // Extract module name from file path
                                let module = self.extract_module_name_from_path(file_path);
                                
                                // Special handling for main functions
                                if func_name == "main" {
                                    return Ok(format!("{}::Main", self.pascal_case(&module)));
                                }
                                
                                // For other functions, use Module::FunctionName format
                                return Ok(format!("{}::{}", 
                                    self.pascal_case(&module),
                                    self.pascal_case(func_name)
                                ));
                            }
                        }
                    }
                }
            }
            node = parent;
        }
        
        // If no function found, use module context
        let module = self.extract_module_name_from_path(file_path);
        Ok(format!("{}::Module", self.pascal_case(&module)))
    }

    fn find_containing_actor_impl(
        &self,
        mut node: tree_sitter::Node,
        source: &[u8],
    ) -> Result<String> {
        // CRITICAL FIX: Enhanced parent actor detection with multiple fallbacks

        // Walk up the tree to find the containing impl block
        // Skip through closures and async blocks
        while let Some(parent) = node.parent() {
            // Skip through closure and async block nodes
            if parent.kind() == "closure_expression" || 
               parent.kind() == "async_block_expression" ||
               parent.kind() == "block" {
                node = parent;
                continue;
            }
            
            if parent.kind() == "impl_item" {
                // Pattern 1: impl TraitName for TypeName (e.g., impl Actor for MyActor)
                let mut found_trait = false;
                let mut trait_name = String::new();
                let mut found_for = false;
                let mut type_name = String::new();
                let mut simple_impl_type = String::new();

                for child_idx in 0..parent.child_count() {
                    if let Some(child) = parent.child(child_idx) {
                        if child.kind() == "type_identifier" {
                            if let Ok(text) = child.utf8_text(source) {
                                if !found_for && !found_trait {
                                    // Could be simple impl (impl SomeType) without trait
                                    simple_impl_type = text.to_string();
                                }

                                if !found_for {
                                    // This is the trait name
                                    trait_name = text.to_string();
                                    found_trait = true;
                                } else {
                                    // This is the type name (after "for")
                                    type_name = text.to_string();
                                }
                            }
                        } else if child.kind() == "generic_type" {
                            // Handle generic types like DelegatedReplyAdapter<TActor>
                            if let Ok(text) = child.utf8_text(source) {
                                // Extract the base type name (before the <)
                                if let Some(base) = text.split('<').next() {
                                    if !found_for && !found_trait {
                                        // Simple impl with generics: impl DelegatedReplyAdapter<T>
                                        simple_impl_type = base.to_string();
                                    } else if found_for {
                                        // After "for": impl Trait for DelegatedReplyAdapter<T>
                                        type_name = base.to_string();
                                    }
                                }
                            }
                        } else if child.kind() == "for" {
                            found_for = true;
                        }
                    }
                }

                // Priority 1: Actor trait implementations
                if found_trait && trait_name == "Actor" && !type_name.is_empty() {
                    return Ok(type_name);
                }

                // Priority 2: Message trait implementations (Message<T> for Actor)
                if found_trait && trait_name == "Message" && !type_name.is_empty() {
                    if self.is_likely_actor_type(&type_name) {
                        return Ok(type_name);
                    }
                }

                // Priority 3: Any trait implementation (impl Trait for Type)
                // This handles cases like impl CryptoFuturesAPIAdapter for BybitFuturesAdapter
                if found_for && !type_name.is_empty() {
                    return Ok(type_name);
                }

                // Priority 4: Simple impl blocks for actor-looking types (impl SomeActor)
                if !simple_impl_type.is_empty() && self.is_likely_actor_type(&simple_impl_type) {
                    return Ok(simple_impl_type);
                }

                // Priority 5: Simple impl blocks for any type (better than Unknown)
                if !simple_impl_type.is_empty() {
                    return Ok(simple_impl_type);
                }

                // Priority 6: Any impl block with actor-like type name
                if !type_name.is_empty() && self.is_likely_actor_type(&type_name) {
                    return Ok(type_name);
                }
            }

            // Also check for function-level context to provide better names
            if parent.kind() == "function_item" {
                if let Some(function_name) = self.find_containing_function(parent, source) {
                    // If we're in a function that looks actor-related, use it as context
                    if function_name.contains("actor")
                        || function_name.contains("Actor")
                        || function_name == "on_start"
                        || function_name == "on_stop"
                        || function_name.starts_with("handle_")
                        || function_name.starts_with("reply")
                    {
                        // Try to infer actor name from function context
                        if let Some(context_actor) =
                            self.infer_actor_from_function_context(parent, source)?
                        {
                            return Ok(context_actor);
                        }
                    }
                }
            }

            node = parent;
        }

        // Final fallback: try to infer from file name or module structure
        let fallback_name = self.infer_actor_from_context(node, source)?;
        if fallback_name != "Unknown" {
            return Ok(fallback_name);
        }

        Ok("Unknown".to_string())
    }

    fn find_spawning_context(
        &self,
        node: tree_sitter::Node,
        source: &[u8],
        file_path: &Path,
    ) -> Result<String> {
        // First try to find if we're in an actor implementation
        if let Ok(actor_name) = self.find_containing_actor_impl(node, source) {
            if actor_name != "Unknown" {
                return Ok(actor_name);
            }
        }

        // If not in an actor impl, create a context based on module + function
        let module_name = self.extract_module_name_from_path(file_path);

        if let Some(function_name) = self.find_containing_function(node, source) {
            // For main functions, use the crate/module name as the spawner
            if function_name == "main" {
                return Ok(format!("{}::Main", module_name));
            }

            // For other functions, use module::FunctionName format
            return Ok(format!(
                "{}::{}",
                module_name,
                self.pascal_case(&function_name)
            ));
        }

        // Fallback to module name only
        Ok(format!("{}::Unknown", module_name))
    }

    fn extract_module_name_from_path(&self, file_path: &Path) -> String {
        // Extract meaningful module name from file path
        if let Some(file_name) = file_path.file_stem() {
            if let Some(file_str) = file_name.to_str() {
                match file_str {
                    "main" | "lib" => {
                        // Use the parent directory name
                        if let Some(parent) = file_path.parent() {
                            if let Some(parent_name) = parent.file_name() {
                                if let Some(parent_str) = parent_name.to_str() {
                                    return self.pascal_case(parent_str);
                                }
                            }
                        }
                        return "Runtime".to_string();
                    }
                    _ => return self.pascal_case(file_str),
                }
            }
        }
        "Unknown".to_string()
    }

    fn pascal_case(&self, snake_case: &str) -> String {
        snake_case
            .split('_')
            .map(|word| {
                if word.is_empty() {
                    String::new()
                } else {
                    let mut chars = word.chars();
                    chars.next().unwrap().to_uppercase().collect::<String>() + chars.as_str()
                }
            })
            .collect()
    }

    fn find_containing_function(
        &self,
        mut node: tree_sitter::Node,
        source: &[u8],
    ) -> Option<String> {
        while let Some(parent) = node.parent() {
            if parent.kind() == "function_item" {
                // Find the function name within this function_item
                let mut cursor = parent.walk();
                for child in parent.children(&mut cursor) {
                    if child.kind() == "identifier"
                        && child
                            .parent()
                            .map_or(false, |p| p.kind() == "function_item")
                    {
                        if let Ok(name) = child.utf8_text(source) {
                            return Some(name.to_string());
                        }
                    }
                }
            }
            node = parent;
        }
        None
    }

    fn find_containing_function_with_line(
        &self,
        mut node: tree_sitter::Node,
        source: &[u8],
    ) -> Option<(String, usize)> {
        while let Some(parent) = node.parent() {
            if parent.kind() == "function_item" {
                // Find the function name within this function_item
                let mut cursor = parent.walk();
                for child in parent.children(&mut cursor) {
                    if child.kind() == "identifier"
                        && child
                            .parent()
                            .map_or(false, |p| p.kind() == "function_item")
                    {
                        if let Ok(name) = child.utf8_text(source) {
                            return Some((name.to_string(), child.start_position().row + 1));
                        }
                    }
                }
            }
            node = parent;
        }
        None
    }

    /// Find the type being implemented in the containing impl block
    fn find_containing_impl_type(&self, node: tree_sitter::Node, source: &[u8]) -> Option<String> {
        let mut current = node;
        while let Some(parent) = current.parent() {
            if parent.kind() == "impl_item" {
                // Look for the type being implemented
                // Pattern 1: impl Type {...}
                // Pattern 2: impl Trait for Type {...}
                
                let mut found_for = false;
                for child in parent.children(&mut parent.walk()) {
                    if child.kind() == "for" {
                        found_for = true;
                        continue;
                    }
                    
                    if child.kind() == "type_identifier" {
                        // If we haven't seen "for" yet, this is the trait name (skip it)
                        // If we have seen "for", this is the type being implemented
                        if found_for || !parent.child_by_field_name("trait").is_some() {
                            if let Ok(type_name) = child.utf8_text(source) {
                                return Some(type_name.to_string());
                            }
                        }
                    }
                }
            }
            current = parent;
        }
        None
    }

    /// Find the actual function containing a macro expansion at the given line
    fn find_containing_function_for_macro(&self, macro_line: usize, functions: &[RustFunction], file_path: &Path) -> Option<String> {
        // Find function that contains the macro based on line ranges
        for function in functions {
            if function.file_path == file_path.to_string_lossy()
                && function.line_start <= macro_line
                && function.line_end >= macro_line {
                return Some(function.id.clone());
            }
        }
        None
    }

    /// Extract module path from function ID (e.g., "crate::module::function" -> "crate::module")
    fn extract_module_path_from_function_id(&self, function_id: &str) -> String {
        if let Some(pos) = function_id.rfind("::") {
            function_id[..pos].to_string()
        } else {
            function_id.to_string()
        }
    }

    /// Extract synthetic function calls from known macro patterns
    fn extract_macro_generated_calls(
        &mut self,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
        functions: &[RustFunction],
    ) -> Result<Vec<FunctionCall>> {
        let mut synthetic_calls = Vec::new();
        let source_str = std::str::from_utf8(source)?;
        let module_path = self.infer_module_path_with_crate(file_path, crate_name)?;
        
        // Create SyntheticCallGenerator for modular call generation
        let generator = SyntheticCallGenerator::new();
        
        // Pattern 1: paste! { [<$indicator>]::new(config) } - check all files, not just trading-ta
        let path_str = file_path.to_string_lossy();
        // Remove restrictive path check to allow pattern detection in any file
        
        // First, check for general paste! macros in any file
        let macro_expansions = self.extract_macro_expansions(source, file_path, crate_name)?;
        
        for expansion in &macro_expansions {
            if expansion.macro_type == "paste" {
                // Find the containing function for proper caller context
                let default_caller = format!("{}::unknown_function", crate_name);
                let caller_id = expansion.containing_function.as_ref()
                    .unwrap_or(&default_caller);
                
                // Generate synthetic calls from this paste macro expansion
                let generated = generator.generate_calls_from_paste_macro(&expansion, caller_id);
                
                // Debug: Check for nan/nz methods
                for call in &generated {
                    if call.callee_name == "nan" || call.callee_name == "nz" || call.callee_name == "na" {
                        eprintln!("🔍 DEBUG: Generated synthetic call for {}: {} -> {}", 
                                call.callee_name, call.caller_id, call.qualified_callee.as_ref().unwrap_or(&"None".to_string()));
                    }
                }
                
                synthetic_calls.extend(generated);
            }
        }
        
        // Also check if this is the indicator_types.rs file - special handling for trading_ta
        if file_path.to_string_lossy().contains("indicator_types.rs") && (crate_name == "trading-ta" || crate_name == "trading_ta") {
            
            // Look for define_indicator_enums! macro invocation
            if source_str.contains("define_indicator_enums!") {
                
                // Find all indicator ::new functions in the analyzed functions
                let indicator_functions = functions.iter()
                    .filter(|func| {
                        (func.crate_name == crate_name || func.crate_name == "trading_ta" || func.crate_name == "trading-ta") &&
                        func.name == "new" &&
                        func.module_path.contains("indicators::")
                    })
                    .collect::<Vec<_>>();
                    
                
                // Find the function containing the define_indicator_enums! call (likely compute_batch_parallel)
                let mut containing_function_id = None;
                for (line_num, line) in source_str.lines().enumerate() {
                    if line.contains("define_indicator_enums!") {
                        let line_number = line_num + 1;
                        containing_function_id = self.find_containing_function_for_macro(line_number, functions, file_path);
                        break;
                    }
                }
                
                // Also look for the actual usage sites of the generated code (compute_batch_parallel method)
                for (line_num, line) in source_str.lines().enumerate() {
                    if line.contains("IndicatorConfigKind::$indicator") {
                        let line_number = line_num + 1;
                        let caller_id = self.find_containing_function_for_macro(line_number, functions, file_path)
                            .or_else(|| {
                                // Try to find the compute_batch_parallel function in the same file
                                functions.iter()
                                    .find(|f| f.file_path == file_path.to_string_lossy() && f.name.contains("compute_batch_parallel"))
                                    .map(|f| f.id.clone())
                            })
                            .unwrap_or_else(|| format!("{}::types::indicator_types::IndicatorConfigKind::compute_batch_parallel", crate_name.replace('-', "_")));
                            
                        
                        // Create synthetic calls to all indicator functions
                        for indicator_func in &indicator_functions {
                            let synthetic_call = FunctionCall {
                                caller_id: caller_id.clone(),
                                caller_module: self.extract_module_path_from_function_id(&caller_id),
                                callee_name: indicator_func.name.clone(),
                                qualified_callee: Some(indicator_func.qualified_name.clone()),
                                call_type: CallType::Macro,
                                line: line_number,
                                cross_crate: false,
                                from_crate: crate_name.to_string(),
                                to_crate: Some(crate_name.to_string()),
                                file_path: file_path.to_string_lossy().to_string(),
                                is_synthetic: true,
                                macro_context: Some(MacroContext {
                                    expansion_id: format!("{}:{}:define_indicator_enums", file_path.display(), line_number),
                                    macro_type: "define_indicator_enums".to_string(),
                                    expansion_site_line: line_number,
                                }),
                                synthetic_confidence: 0.95,
                            };
                            
                            synthetic_calls.push(synthetic_call);
                        }
                        break; // Only create once per file
                    }
                }
            }
        }
        
        // Use static indicator list to avoid allocations
        
        // Look for the macro expansion pattern by scanning lines to find exact location
        for (line_num, line) in source_str.lines().enumerate() {
            let line_number = line_num + 1;
            
            // Pattern 1: paste! macro with [<$indicator>]::new pattern
            // Looking for: paste! { [<$indicator>]::new(config) }
            if line.contains("paste!") && line.contains("[<$indicator>]::new") {
                
                // Find the actual containing function instead of using a fake MACRO_EXPANSION node
                let caller_id = self.find_containing_function_for_macro(line_number, functions, file_path)
                    .unwrap_or_else(|| {
                        let normalized_crate = crate_name.replace('-', "_");
                        format!("{}::unknown_macro_context", normalized_crate)
                    });
                
                
                // Create MacroExpansion object for proper context tracking
                let expansion = MacroExpansion {
                    id: format!("{}:{}:{}", crate_name, file_path.display(), line_number),
                    crate_name: crate_name.to_string(),
                    file_path: file_path.to_string_lossy().to_string(),
                    line_range: line_number..line_number + 1,
                    macro_type: "paste".to_string(),
                    expansion_pattern: line.to_string(),
                    target_functions: vec![],
                    containing_function: Some(caller_id.clone()),
                    expansion_context: MacroContext {
                        expansion_id: format!("{}:{}:{}", crate_name, file_path.display(), line_number),
                        macro_type: "paste".to_string(),
                        expansion_site_line: line_number,
                    },
                };
                
                // Use SyntheticCallGenerator to create calls with proper context
                let mut generated = generator.generate_calls_from_paste_macro(&expansion, &caller_id);
                
                // Update generated calls with proper macro context
                for call in &mut generated {
                    // Set proper macro context linking to the expansion
                    call.macro_context = Some(MacroContext {
                        expansion_id: expansion.id.clone(),
                        macro_type: expansion.macro_type.clone(),
                        expansion_site_line: line_number,
                    });
                    call.file_path = file_path.to_string_lossy().to_string();
                    call.from_crate = crate_name.replace('-', "_");
                    call.to_crate = Some(crate_name.replace('-', "_"));
                }
                
                synthetic_calls.extend(generated);
            }
            
            // Pattern 2: paste! macro with [<$indicator Input>]::from_ohlcv pattern
            // Looking for: paste! { [<$indicator Input>]::from_ohlcv(candle) }
            if line.contains("paste!") && line.contains("[<$indicator Input>]::from_ohlcv") {
                
                // Find the actual containing function
                let caller_id = self.find_containing_function_for_macro(line_number, functions, file_path)
                    .unwrap_or_else(|| {
                        let normalized_crate = crate_name.replace('-', "_");
                        format!("{}::unknown_macro_context", normalized_crate)
                    });
                
                
                // Create MacroExpansion object for proper context tracking
                let expansion = MacroExpansion {
                    id: format!("{}:{}:{}_input", crate_name, file_path.display(), line_number),
                    crate_name: crate_name.to_string(),
                    file_path: file_path.to_string_lossy().to_string(),
                    line_range: line_number..line_number + 1,
                    macro_type: "paste".to_string(),
                    expansion_pattern: line.to_string(),
                    target_functions: vec![],
                    containing_function: Some(caller_id.clone()),
                    expansion_context: MacroContext {
                        expansion_id: format!("{}:{}:{}_input", crate_name, file_path.display(), line_number),
                        macro_type: "paste".to_string(),
                        expansion_site_line: line_number,
                    },
                };
                
                // Use SyntheticCallGenerator to create calls with proper context
                // Note: The generator's generate_calls_from_paste_macro needs to handle Input patterns
                let mut generated = generator.generate_calls_from_paste_macro(&expansion, &caller_id);
                
                // Update generated calls with proper macro context
                for call in &mut generated {
                    // Set proper macro context linking to the expansion
                    call.macro_context = Some(MacroContext {
                        expansion_id: expansion.id.clone(),
                        macro_type: expansion.macro_type.clone(),
                        expansion_site_line: line_number,
                    });
                    call.file_path = file_path.to_string_lossy().to_string();
                    call.from_crate = crate_name.replace('-', "_");
                    call.to_crate = Some(crate_name.replace('-', "_"));
                }
                
                let generated_count = generated.len();
                synthetic_calls.extend(generated);
            }
        }
        
        Ok(synthetic_calls)
    }

    fn parse_call_match(
        &self,
        match_: tree_sitter::QueryMatch,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
    ) -> Result<Option<FunctionCall>> {
        let mut captures = HashMap::new();
        for capture in match_.captures {
            let node = capture.node;
            let text = node.utf8_text(source)?;
            let capture_name = self.queries.call_query.capture_names()[capture.index as usize];
            captures.insert(capture_name, (node, text));
        }

        let call_node = match_
            .captures
            .first()
            .map(|c| c.node)
            .ok_or_else(|| anyhow::anyhow!("No captures in call match"))?;

        let callee_name = if let Some((_, function_name)) = captures.get("function") {
            function_name.to_string()
        } else if let Some((_, method_name)) = captures.get("method") {
            method_name.to_string()
        } else if let Some((_, scoped_call)) = captures.get("scoped_call") {
            // Debug: Log scoped calls that contain bybit
            if scoped_call.to_lowercase().contains("bybit") && scoped_call.contains("new") {
            }
            // Check if it's a Self:: call and resolve it
            if scoped_call.starts_with("Self::") {
                let method_name = &scoped_call[6..]; // Skip "Self::"
                if let Some(impl_type) = self.find_containing_impl_type(call_node, source) {
                    // Resolve Self to the concrete type
                    let resolved = format!("{}::{}", impl_type, method_name);
                    resolved
                } else {
                    scoped_call.to_string()
                }
            } else {
                scoped_call.to_string()
            }
        } else if let Some((_, generic_call)) = captures.get("generic_call") {
            generic_call.to_string()
        } else if let Some((_, macro_name)) = captures.get("macro_name") {
            format!("{}!", macro_name)
        } else {
            return Ok(None);
        };

        let module_path = self.infer_module_path_with_crate(file_path, crate_name)?;

        // Find the actual containing function instead of using "unknown_caller"
        let caller_id = if let Some((function_name, function_line)) =
            self.find_containing_function_with_line(call_node, source)
        {
            // Generate ID in same format as RustFunction::generate_id()
            let qualified_name = format!("{}::{}", module_path, function_name);
            format!("{}:{}:{}", crate_name, qualified_name, function_line)
        } else {
            // Fallback to module-level if not inside a function (e.g., const expressions)
            format!("{}::module_level", module_path)
        };

        // qualified_callee would be resolved in reference resolution phase
        let qualified_callee = None;

        let function_call = FunctionCall {
            caller_id,
            caller_module: module_path.clone(),
            callee_name,
            qualified_callee,
            call_type: if captures.contains_key("method") {
                CallType::Method
            } else if captures.contains_key("scoped_call") {
                // Determine if this is an associated function call
                if let Some((_, scoped_call)) = captures.get("scoped_call") {
                    if scoped_call.contains("::") {
                        CallType::Associated
                    } else {
                        CallType::Direct
                    }
                } else {
                    CallType::Direct
                }
            } else if captures.contains_key("generic_call") {
                CallType::Direct
            } else if captures.contains_key("macro_name") {
                CallType::Macro
            } else {
                CallType::Direct
            },
            line: call_node.start_position().row + 1,
            cross_crate: false, // Would be determined during resolution
            from_crate: crate_name.to_string(),
            to_crate: None, // Would be resolved later
            file_path: file_path.to_string_lossy().to_string(),
            is_synthetic: false,
            macro_context: None, // Regular calls don't have macro context
            synthetic_confidence: 1.0, // Regular calls have full confidence
        };

        Ok(Some(function_call))
    }

    fn parse_import_match(
        &self,
        match_: tree_sitter::QueryMatch,
        source: &[u8],
        file_path: &Path,
    ) -> Result<Option<RustImport>> {
        let mut captures = HashMap::new();
        for capture in match_.captures {
            let node = capture.node;
            let text = node.utf8_text(source)?;
            let capture_name = self.queries.import_query.capture_names()[capture.index as usize];
            captures.insert(capture_name, (node, text));
        }

        if let Some((use_node, use_text)) = captures.get("use_decl") {
            return self.parse_use_statement(use_text, use_node, file_path);
        }

        Ok(None)
    }

    fn parse_module_match(
        &self,
        match_: tree_sitter::QueryMatch,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
    ) -> Result<Option<RustModule>> {
        let mut captures = HashMap::new();
        for capture in match_.captures {
            let node = capture.node;
            let text = node.utf8_text(source)?;
            let capture_name = self.queries.module_query.capture_names()[capture.index as usize];
            captures.insert(capture_name, (node, text));
        }

        if let Some((name_node, name)) = captures.get("name") {
            let visibility = captures
                .get("visibility")
                .map(|(_, vis)| vis.to_string())
                .unwrap_or_else(|| "private".to_string());

            let parent_module_path = self.infer_module_path_with_crate(file_path, crate_name)?;
            let module_path = format!("{}::{}", parent_module_path, name);

            let rust_module = RustModule {
                name: name.to_string(),
                path: module_path.clone(),
                crate_name: crate_name.to_string(),
                file_path: file_path.to_string_lossy().to_string(),
                is_public: visibility == "pub",
                parent_module: Some(parent_module_path.clone()),
            };

            return Ok(Some(rust_module));
        }

        Ok(None)
    }

    fn extract_doc_comment(
        &self,
        node: Option<tree_sitter::Node>,
        source: &[u8],
    ) -> Result<Option<String>> {
        // Wrap AST traversal in catch_unwind to handle segfaults gracefully
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || -> Result<Option<String>> {
                if let Some(parent) = node {
                    let prev_sibling = parent.prev_sibling();
                    if let Some(comment_node) = prev_sibling {
                        if comment_node.kind() == "line_comment"
                            || comment_node.kind() == "block_comment"
                        {
                            let comment_text = comment_node.utf8_text(source)?;
                            if comment_text.starts_with("///") || comment_text.starts_with("/**") {
                                return Ok(Some(comment_text.to_string()));
                            }
                        }
                    }
                }
                Ok(None)
            },
        ));

        match result {
            Ok(doc_result) => doc_result,
            Err(_) => {
                Ok(None) // Return None instead of crashing
            }
        }
    }

    fn extract_parameters(
        &self,
        captures: &HashMap<&str, (tree_sitter::Node, &str)>,
        source: &[u8],
    ) -> Result<Vec<Parameter>> {
        let mut parameters = Vec::new();

        if let Some((params_node, _)) = captures.get("params") {
            for child in params_node.children(&mut params_node.walk()) {
                if child.kind() == "parameter" {
                    if let Ok(param_text) = child.utf8_text(source) {
                        // Simple parameter parsing - just store the text for now
                        let parts: Vec<&str> = param_text.split(':').map(|s| s.trim()).collect();
                        if parts.len() >= 2 {
                            parameters.push(Parameter {
                                name: parts[0].to_string(),
                                param_type: parts[1].to_string(),
                                is_self: parts[0] == "self"
                                    || parts[0] == "&self"
                                    || parts[0] == "&mut self",
                                is_mutable: param_text.contains("mut "),
                            });
                        }
                    }
                }
            }
        }

        Ok(parameters)
    }

    fn extract_function_signature(
        &self,
        captures: &HashMap<&str, (tree_sitter::Node, &str)>,
        _source: &[u8],
    ) -> Result<String> {
        // Build a simplified function signature
        let mut parts = Vec::new();

        if let Some((_, visibility)) = captures.get("visibility") {
            parts.push(visibility.to_string());
        }

        if captures.contains_key("async") {
            parts.push("async".to_string());
        }

        if captures.contains_key("unsafe") {
            parts.push("unsafe".to_string());
        }

        parts.push("fn".to_string());

        if let Some((_, name)) = captures.get("name") {
            parts.push(name.to_string());
        }

        if let Some((_, params)) = captures.get("params") {
            parts.push(format!("({})", params));
        }

        if let Some((_, return_type)) = captures.get("return_type") {
            parts.push(format!("-> {}", return_type));
        }

        Ok(parts.join(" "))
    }

    fn extract_fields(
        &self,
        captures: &HashMap<&str, (tree_sitter::Node, &str)>,
        source: &[u8],
    ) -> Result<Vec<Field>> {
        let mut fields = Vec::new();

        if let Some((fields_node, _)) = captures.get("fields") {
            // Wrap AST traversal in catch_unwind to handle segfaults gracefully
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<Vec<Field>> {
                    let mut inner_fields = Vec::new();
                    for child in fields_node.children(&mut fields_node.walk()) {
                        if child.kind() == "field_declaration" {
                            if let Ok(field_text) = child.utf8_text(source) {
                                let parts: Vec<&str> =
                                    field_text.split(':').map(|s| s.trim()).collect();
                                if parts.len() >= 2 {
                                    inner_fields.push(Field {
                                        name: parts[0].to_string(),
                                        field_type: parts[1].to_string(),
                                        visibility: "private".to_string(),
                                        doc_comment: None,
                                    });
                                }
                            }
                        }
                    }
                    Ok(inner_fields)
                }));

            match result {
                Ok(Ok(extracted_fields)) => fields.extend(extracted_fields),
                Ok(Err(_e)) => {
                    // Return empty fields rather than propagating error
                }
                Err(_) => {

                    // Return empty fields rather than crashing
                }
            }
        }

        Ok(fields)
    }

    fn extract_variants(
        &self,
        captures: &HashMap<&str, (tree_sitter::Node, &str)>,
        source: &[u8],
    ) -> Result<Vec<Variant>> {
        let mut variants = Vec::new();

        if let Some((variants_node, _)) = captures.get("variants") {
            // Wrap AST traversal in catch_unwind to handle segfaults gracefully
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || -> Result<Vec<Variant>> {
                    let mut inner_variants = Vec::new();
                    for child in variants_node.children(&mut variants_node.walk()) {
                        if child.kind() == "enum_variant" {
                            if let Ok(variant_text) = child.utf8_text(source) {
                                let name = variant_text
                                    .split('(')
                                    .next()
                                    .unwrap_or(variant_text)
                                    .trim();
                                inner_variants.push(Variant {
                                    name: name.to_string(),
                                    fields: Vec::new(),
                                    doc_comment: None,
                                });
                            }
                        }
                    }
                    Ok(inner_variants)
                },
            ));

            match result {
                Ok(Ok(extracted_variants)) => variants.extend(extracted_variants),
                Ok(Err(_e)) => {
                    // Return empty variants rather than propagating error
                }
                Err(_) => {

                    // Return empty variants rather than crashing
                }
            }
        }

        Ok(variants)
    }

    fn parse_use_statement(
        &self,
        use_text: &str,
        use_node: &tree_sitter::Node,
        file_path: &Path,
    ) -> Result<Option<RustImport>> {
        // Parse the use statement text to extract path and items
        let use_text = use_text.trim();
        let path = if use_text.starts_with("use ") {
            use_text[4..].trim_end_matches(';').trim()
        } else {
            use_text.trim_end_matches(';').trim()
        };

        let (module_path, import_type, imported_items) = if path.contains('{') {
            // Grouped import: use crate_a::{function_a, utility_function};
            let parts: Vec<&str> = path.splitn(2, '{').collect();
            let base_path = parts[0].trim_end_matches(':').trim();
            let items_str = parts.get(1).unwrap_or(&"").trim_end_matches('}').trim();

            let items: Vec<ImportedItem> = items_str
                .split(',')
                .map(|item| {
                    let item = item.trim();
                    if item.contains(" as ") {
                        let parts: Vec<&str> = item.split(" as ").collect();
                        ImportedItem {
                            name: parts[0].trim().to_string(),
                            alias: Some(parts[1].trim().to_string()),
                        }
                    } else {
                        ImportedItem {
                            name: item.to_string(),
                            alias: None,
                        }
                    }
                })
                .filter(|item| !item.name.is_empty())
                .collect();

            (base_path.to_string(), ImportType::Grouped, items)
        } else if path.ends_with('*') {
            // Glob import: use crate_a::*;
            let base_path = path.trim_end_matches("::*").trim();
            (base_path.to_string(), ImportType::Glob, vec![])
        } else if path.contains(" as ") {
            // Aliased simple import: use crate_a::function_a as func_a;
            let parts: Vec<&str> = path.split(" as ").collect();
            let full_path = parts[0].trim();
            let alias = parts[1].trim();

            // Extract the item name from the end of the path
            let item_name = full_path.split("::").last().unwrap_or(full_path);
            let module_path = full_path.trim_end_matches(&format!("::{}", item_name));

            let items = vec![ImportedItem {
                name: item_name.to_string(),
                alias: Some(alias.to_string()),
            }];

            (module_path.to_string(), ImportType::Simple, items)
        } else if path.contains("::") {
            // Simple import with path: use crate_a::function_a;
            let parts: Vec<&str> = path.split("::").collect();
            if parts.len() >= 2 {
                let item_name = parts.last().unwrap();
                let module_path = parts[..parts.len() - 1].join("::");

                let items = vec![ImportedItem {
                    name: item_name.to_string(),
                    alias: None,
                }];

                (module_path, ImportType::Simple, items)
            } else {
                // Module import: use some_module;
                (path.to_string(), ImportType::Module, vec![])
            }
        } else {
            // Module import: use crate_a;
            (path.to_string(), ImportType::Module, vec![])
        };

        let rust_import = RustImport {
            module_path,
            imported_items,
            import_type,
            file_path: file_path.to_string_lossy().to_string(),
            line: use_node.start_position().row + 1,
        };

        Ok(Some(rust_import))
    }

    fn infer_module_path(&self, file_path: &Path) -> Result<String> {
        self.infer_module_path_with_crate(file_path, "crate")
    }

    fn infer_module_path_with_crate(&self, file_path: &Path, crate_name: &str) -> Result<String> {
        // Normalize crate name (replace - with _)
        let normalized_crate = crate_name.replace('-', "_");
        
        // Find the "src" directory in the path to determine the module structure
        let path_str = file_path.to_string_lossy();
        
        // Split the path on "src/" to get the relative path within the crate
        let parts: Vec<&str> = path_str.split("/src/").collect();
        let relative_path = if parts.len() >= 2 {
            parts.last().unwrap().to_string()
        } else {
            // Try Windows-style path
            let parts: Vec<&str> = path_str.split("\\src\\").collect();
            if parts.len() >= 2 {
                parts.last().unwrap().to_string()
            } else {
                // Fallback to old behavior if we can't find src/
                let stem = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
                return if stem == "lib" || stem == "main" {
                    Ok(normalized_crate.clone())
                } else {
                    Ok(format!("{}::{}", normalized_crate, stem))
                };
            }
        };
        
        // Convert the file path to module path
        let module_path = if relative_path == "lib.rs" || relative_path == "main.rs" {
            normalized_crate.clone()
        } else {
            // Remove the .rs extension
            let path_without_ext = relative_path.trim_end_matches(".rs");
            
            // Split by path separator
            let segments: Vec<&str> = path_without_ext
                .split('/')
                .filter(|s| !s.is_empty())
                .collect();
            
            // Handle mod.rs files - they represent their parent directory
            let mut final_segments = segments.clone();
            if final_segments.last() == Some(&"mod") {
                final_segments.pop();
            }
            
            if final_segments.is_empty() {
                normalized_crate.clone()
            } else {
                format!("{}::{}", normalized_crate, final_segments.join("::"))
            }
        };
        
        Ok(module_path)
    }

    fn is_test_function(
        &self,
        captures: &HashMap<&str, (tree_sitter::Node, &str)>,
        source: &[u8],
    ) -> Result<bool> {
        // Get the function name from captures
        let function_name = if let Some((_, name)) = captures.get("name") {
            name
        } else {
            return Ok(false);
        };

        // TEMPORARY: Simple name-based detection for testing the pipeline
        if function_name.contains("test") {
            return Ok(true);
        }

        if let Some((attributes_node, _)) = captures.get("attributes") {
            // Check for test attributes like #[test], #[tokio::test], etc.
            let mut cursor = attributes_node.walk();
            for child in attributes_node.children(&mut cursor) {
                if child.kind() == "attribute_item" {
                    if let Ok(attr_text) = child.utf8_text(source) {
                        // Check for common test attributes
                        if attr_text.contains("#[test]")
                            || attr_text.contains("#[tokio::test]")
                            || attr_text.contains("#[async_test]")
                        {
                            return Ok(true);
                        }
                    }
                }
            }
        }

        // TODO: Also check if function is inside #[cfg(test)] module
        // For now, just return false for non-attribute tests
        Ok(false)
    }

    fn is_spawn_in_test_context(&self, context: &str, file_path: &Path) -> bool {
        // Check if context indicates test function
        if context.contains("test") {
            return true;
        }

        // Check if file path indicates test file
        let file_str = file_path.to_string_lossy();
        if file_str.contains("test") || file_str.contains("/tests/") {
            return true;
        }

        // Check if function name patterns suggest test
        let function_name = context.split(':').last().unwrap_or("");
        if function_name.starts_with("test_")
            || function_name.ends_with("_test")
            || function_name.starts_with("setup_")
            || function_name.contains("mock")
        {
            return true;
        }

        false
    }

    fn parse_actor_impl_match(
        &self,
        match_: tree_sitter::QueryMatch,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
        is_test: bool,
    ) -> Result<Option<RustActor>> {
        let captures: HashMap<&str, (tree_sitter::Node, &str)> = match_
            .captures
            .iter()
            .filter_map(|capture| {
                let name = &self.queries.actor_impl_query.capture_names()[capture.index as usize];
                match capture.node.utf8_text(source) {
                    Ok(text) => Some((*name, (capture.node, text))),
                    Err(_) => None,
                }
            })
            .collect();

        // Check if this is actually an Actor implementation
        let trait_name = captures
            .get("trait_name")
            .map(|(_, text)| *text)
            .unwrap_or("");
        if trait_name != "Actor" {
            return Ok(None);
        }

        let actor_name = captures
            .get("actor_name")
            .map(|(_, text)| *text)
            .unwrap_or("Unknown");

        // CRITICAL FIX: Reject qualified type names (e.g., SomeType::Variant)
        // These are enum variants or associated types, not actual actor types
        if actor_name.contains("::") {
            return Ok(None);
        }

        let line_start = match_
            .captures
            .first()
            .map(|c| c.node.start_position().row + 1)
            .unwrap_or(0);
        let line_end = match_
            .captures
            .first()
            .map(|c| c.node.end_position().row + 1)
            .unwrap_or(0);

        // Determine actor type based on naming patterns
        let actor_type = if actor_name.contains("Supervisor") {
            ActorImplementationType::Supervisor
        } else if actor_name.contains("Distributed") {
            ActorImplementationType::Distributed
        } else {
            ActorImplementationType::Local
        };

        let module_path = self.infer_module_path_with_crate(file_path, crate_name)?;
        let qualified_name = format!("{}::{}", module_path, actor_name);

        let actor = RustActor {
            id: format!("{}::{}:{}", qualified_name, file_path.display(), line_start),
            name: actor_name.to_string(),
            qualified_name,
            crate_name: crate_name.to_string(),
            module_path,
            file_path: file_path.to_string_lossy().to_string(),
            line_start,
            line_end,
            visibility: "pub".to_string(), // Most actors are public
            doc_comment: None,
            is_distributed: matches!(actor_type, ActorImplementationType::Distributed),
            is_test,
            actor_type,
            local_messages: Vec::new(), // Will be populated by message handler detection
            inferred_from_message: false, // Explicitly declared with impl Actor
        };

        Ok(Some(actor))
    }

    fn parse_actor_spawn_match(
        &self,
        match_: tree_sitter::QueryMatch,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
    ) -> Result<Option<ActorSpawn>> {
        let captures: HashMap<&str, (tree_sitter::Node, &str)> = match_
            .captures
            .iter()
            .filter_map(|capture| {
                let name = &self.queries.actor_spawn_query.capture_names()[capture.index as usize];
                match capture.node.utf8_text(source) {
                    Ok(text) => Some((*name, (capture.node, text))),
                    Err(_) => None,
                }
            })
            .collect();

        // CRITICAL FIX: Actor Framework Filtering - reject non-actor spawns immediately
        if let Some((_, actor_type)) = captures.get("actor_type") {
            if self.is_non_actor_framework(actor_type) {
                return Ok(None); // Reject tokio, std, async_std spawns
            }
        }

        if let Some((_, trait_type)) = captures.get("trait_type") {
            if self.is_non_actor_framework(trait_type) {
                return Ok(None); // Reject tokio, std, async_std spawns
            }
        }

        if let Some((_, module)) = captures.get("module_path") {
            if self.is_non_actor_framework(module) {
                return Ok(None); // Reject tokio, std, async_std spawns
            }
        }

        // Extract arguments for analysis
        let arguments = captures.get("spawn_args").map(|(_, args)| args.to_string());

        let line = match_
            .captures
            .first()
            .map(|c| c.node.start_position().row + 1)
            .unwrap_or(0);

        // Find the actual spawning context (module + function, not just actor impl)
        let parent_actor_name = if let Some(first_capture) = match_.captures.first() {
            self.find_spawning_context(first_capture.node, source, file_path)?
        } else {
            "Unknown".to_string()
        };

        // Find the context (function name where spawn occurs with enhanced detection)
        let context = if let Some(first_capture) = match_.captures.first() {
            self.detect_spawn_context(first_capture.node, source)
        } else {
            "unknown".to_string()
        };

        // CRITICAL FIX: Filter out spawns from test functions to avoid inflating counts
        if self.is_spawn_in_test_context(&context, file_path) {
            return Ok(None);
        }

        let parent_actor_id = format!("{}::{}:{}", parent_actor_name, file_path.display(), line);

        // Pattern 1: DirectType - ActorType::spawn(args) - ONLY for actual actors
        if let (Some((_, actor_type)), Some((_, method_name))) =
            (captures.get("actor_type"), captures.get("spawn_method"))
        {
            let spawn_method = self.parse_spawn_method_name(method_name)?;
            if spawn_method.is_none() {
                return Ok(None); // Not a spawn method
            }

            // Additional validation: actor type should look like an actor
            if !self.is_likely_actor_type(actor_type) {
                return Ok(None); // Reject generic identifiers that aren't actors
            }

            let spawn = ActorSpawn {
                parent_actor_id,
                parent_actor_name: parent_actor_name.clone(),
                child_actor_name: actor_type.to_string(),
                spawn_method: spawn_method.unwrap(),
                spawn_pattern: SpawnPattern::DirectType,
                context,
                arguments,
                line,
                file_path: file_path.to_string_lossy().to_string(),
                from_crate: crate_name.to_string(),
                to_crate: crate_name.to_string(),
            };

            return Ok(Some(spawn));
        }

        // Pattern 2: TraitMethod - Actor::spawn(instance) - ONLY for Actor trait
        if let (Some((_, trait_type)), Some((_, method_name))) =
            (captures.get("trait_type"), captures.get("spawn_method"))
        {
            if *trait_type == "Actor" && *method_name == "spawn" {
                // Extract actor type from arguments (e.g., SomeActor::new())
                let child_actor_name = self
                    .extract_actor_type_from_args(&captures, source)
                    .unwrap_or_else(|| "Unknown".to_string());

                // Reject if we couldn't extract a meaningful actor type or if it's a trait name
                if child_actor_name == "Unknown" || child_actor_name == "Actor" {
                    return Ok(None);
                }

                // The extracted name might be a type or an inferred type from a variable
                // Both are valid at this point since extract_actor_type_from_args handles the conversion

                let spawn = ActorSpawn {
                    parent_actor_id,
                    parent_actor_name: parent_actor_name.clone(),
                    child_actor_name: child_actor_name.clone(),
                    spawn_method: SpawnMethod::Actor,
                    spawn_pattern: SpawnPattern::TraitMethod,
                    context,
                    arguments,
                    line,
                    file_path: file_path.to_string_lossy().to_string(),
                    from_crate: crate_name.to_string(),
                    to_crate: crate_name.to_string(),
                };

                return Ok(Some(spawn));
            }
        }

        // Pattern 3: ModuleFunction - kameo::actor::spawn(instance) - ONLY for actor frameworks
        if let (Some((_, module)), Some((_, actor_mod)), Some((_, function))) = (
            captures.get("module_path"),
            captures.get("actor_module"),
            captures.get("spawn_function"),
        ) {
            if self.is_actor_framework_spawn(module, Some(*actor_mod), function) {
                // Extract actor type from arguments
                let child_actor_name = self
                    .extract_actor_type_from_args(&captures, source)
                    .unwrap_or_else(|| "Unknown".to_string());

                // Reject if we couldn't extract a meaningful actor type
                if child_actor_name == "Unknown" {
                    return Ok(None);
                }

                // The extracted name might be a type or an inferred type from a variable
                // Both are valid at this point since extract_actor_type_from_args handles the conversion

                let spawn = ActorSpawn {
                    parent_actor_id,
                    parent_actor_name: parent_actor_name.clone(),
                    child_actor_name,
                    spawn_method: SpawnMethod::ModuleSpawn,
                    spawn_pattern: SpawnPattern::ModuleFunction,
                    context,
                    arguments,
                    line,
                    file_path: file_path.to_string_lossy().to_string(),
                    from_crate: crate_name.to_string(),
                    to_crate: crate_name.to_string(),
                };

                return Ok(Some(spawn));
            }
        }

        // Legacy fallback for old scoped_spawn_call capture - WITH FILTERING
        if let Some((_, scoped_call_text)) = captures.get("scoped_spawn_call") {
            if let Some((actor_type_text, spawn_method_text)) = scoped_call_text.rsplit_once("::") {
                // Apply framework filtering to legacy patterns too
                if self.is_non_actor_framework(actor_type_text) {
                    return Ok(None);
                }

                if let Some(spawn_method) = self.parse_spawn_method_name(spawn_method_text)? {
                    if !self.is_likely_actor_type(actor_type_text) {
                        return Ok(None);
                    }

                    let spawn = ActorSpawn {
                        parent_actor_id,
                        parent_actor_name: parent_actor_name.clone(),
                        child_actor_name: actor_type_text.to_string(),
                        spawn_method,
                        spawn_pattern: SpawnPattern::DirectType,
                        context,
                        arguments,
                        line,
                        file_path: file_path.to_string_lossy().to_string(),
                        from_crate: crate_name.to_string(),
                        to_crate: crate_name.to_string(),
                    };

                    return Ok(Some(spawn));
                }
            }
        }

        Ok(None)
    }

    fn extract_message_types(
        &mut self,
        tree: &Tree,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
    ) -> Result<Vec<MessageType>> {
        let mut message_types = Vec::new();
        let mut cursor = tree_sitter::QueryCursor::new();

        for query_match in
            cursor.matches(&self.queries.message_type_query, tree.root_node(), source)
        {
            let captures: HashMap<&str, (tree_sitter::Node, &str)> = query_match
                .captures
                .iter()
                .filter_map(|capture| {
                    let name =
                        &self.queries.message_type_query.capture_names()[capture.index as usize];
                    match capture.node.utf8_text(source) {
                        Ok(text) => Some((*name, (capture.node, text))),
                        Err(_) => None,
                    }
                })
                .collect();

            if let Some((_, name)) = captures.get("name") {
                // Check if the name ends with Tell, Ask, Message, or Query
                let kind = if name.ends_with("Tell") {
                    MessageKind::Tell
                } else if name.ends_with("Ask") {
                    MessageKind::Ask
                } else if name.ends_with("Message") {
                    MessageKind::Message
                } else if name.ends_with("Query") {
                    MessageKind::Query
                } else {
                    continue; // Not a message type
                };

                let visibility = captures
                    .get("visibility")
                    .map(|(_, vis)| vis.to_string())
                    .unwrap_or_else(|| "private".to_string());

                let module_path = self.infer_module_path_with_crate(file_path, crate_name)?;
                let qualified_name = format!("{}::{}", module_path, name);

                let mut message_type = MessageType {
                    id: String::new(),
                    name: name.to_string(),
                    qualified_name,
                    crate_name: crate_name.to_string(),
                    module_path: module_path.clone(),
                    file_path: file_path.to_string_lossy().to_string(),
                    line_start: captures
                        .get("name")
                        .map(|(n, _)| n.start_position().row + 1)
                        .unwrap_or(0),
                    line_end: captures
                        .get("name")
                        .map(|(n, _)| n.end_position().row + 1)
                        .unwrap_or(0),
                    kind,
                    visibility,
                    doc_comment: None,
                };

                message_type.id = format!(
                    "{}:{}:{}",
                    crate_name, message_type.qualified_name, message_type.line_start
                );
                message_types.push(message_type);
            }
        }

        Ok(message_types)
    }

    fn extract_message_handlers(
        &mut self,
        tree: &Tree,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
    ) -> Result<Vec<MessageHandler>> {
        let mut handlers = Vec::new();
        let mut cursor = tree_sitter::QueryCursor::new();
        

        for query_match in cursor.matches(
            &self.queries.message_handler_query,
            tree.root_node(),
            source,
        ) {
            let captures: HashMap<&str, (tree_sitter::Node, &str)> = query_match
                .captures
                .iter()
                .filter_map(|capture| {
                    let name =
                        &self.queries.message_handler_query.capture_names()[capture.index as usize];
                    match capture.node.utf8_text(source) {
                        Ok(text) => Some((*name, (capture.node, text))),
                        Err(_) => None,
                    }
                })
                .collect();

            // Check if this is a Message trait implementation
            if let Some((_, trait_name)) = captures.get("trait_name") {
                if *trait_name != "Message" {
                    continue;
                }
            } else {
                continue;
            }

            let actor_type_raw = captures
                .get("actor_type")
                .map(|(_, t)| t.to_string())
                .unwrap_or_default();
            
            // Extract just the type name from scoped paths like super::data::OpenInterestDataActor
            let actor_type = if actor_type_raw.contains("::") {
                actor_type_raw.split("::").last().unwrap_or(&actor_type_raw).to_string()
            } else {
                actor_type_raw
            };
            let message_type = captures
                .get("message_type")
                .map(|(_, t)| t.to_string())
                .unwrap_or_default();

            if actor_type.is_empty() || message_type.is_empty() {
                continue;
            }

            // Try to find the Reply type in the impl block
            let reply_type = self.extract_reply_type(&captures, source)?;

            let module_path = self.infer_module_path_with_crate(file_path, crate_name)?;
            let actor_qualified = format!("{}::{}", module_path, actor_type);
            let message_qualified = format!("{}::{}", module_path, message_type);

            let line = captures
                .get("trait_name")
                .map(|(n, _)| n.start_position().row + 1)
                .unwrap_or(0);

            let mut handler = MessageHandler {
                id: String::new(),
                actor_name: actor_type.clone(),
                actor_qualified,
                message_type: message_type.clone(),
                message_qualified,
                reply_type,
                is_async: true, // kameo handlers are always async
                file_path: file_path.to_string_lossy().to_string(),
                line,
                crate_name: crate_name.to_string(),
            };

            handler.id = format!("{}:{}->{}:{}", crate_name, actor_type, message_type, line);
            handlers.push(handler);
        }

        Ok(handlers)
    }

    fn link_message_handlers_to_actors(&self, symbols: &mut ParsedSymbols) {
        
        // Create a map from actor qualified name to message types they handle
        let mut actor_messages: HashMap<String, Vec<String>> = HashMap::new();
        
        for handler in &symbols.message_handlers {
            actor_messages
                .entry(handler.actor_qualified.clone())
                .or_default()
                .push(handler.message_type.clone());
        }
        
        // Update local actors with their message handlers
        for actor in &mut symbols.actors {
            if let Some(messages) = actor_messages.get(&actor.qualified_name) {
                actor.local_messages = messages.clone();
            }
        }
        
        // Update distributed actors with their message handlers (they can have both distributed and local)
        for actor in &mut symbols.distributed_actors {
            // Create qualified name for distributed actor using same pattern as message handlers
            let module_path = self.infer_module_path_with_crate(&std::path::Path::new(&actor.file_path), &actor.crate_name).unwrap_or_else(|_| actor.crate_name.replace('-', "_"));
            let qualified_name = format!("{}::{}", module_path, actor.actor_name);
            
            if let Some(messages) = actor_messages.get(&qualified_name) {
                actor.local_messages = messages.clone();
            }
        }
    }

    fn extract_struct_field_actor_refs(
        &self,
        tree: &Tree,
        source: &[u8],
        actor_ref_map: &mut HashMap<String, String>,
    ) -> Result<()> {
        // Query for struct definitions with ActorRef fields
        let query_text = r#"
            (struct_item
              name: (type_identifier) @struct_name
              body: (field_declaration_list
                (field_declaration
                  name: (field_identifier) @field_name
                  type: [
                    (generic_type
                      type: (type_identifier) @wrapper_type
                      type_arguments: (type_arguments
                        (generic_type
                          type: (type_identifier) @ref_type
                          type_arguments: (type_arguments
                            (type_identifier) @actor_type
                          )
                        )
                      )
                    )
                    (generic_type
                      type: (type_identifier) @ref_type
                      type_arguments: (type_arguments
                        (type_identifier) @actor_type
                      )
                    )
                  ]
                )
              )
            )
        "#;
        
        let query = Query::new(&tree_sitter_rust::language(), query_text)?;
        let mut cursor = tree_sitter::QueryCursor::new();
        
        for query_match in cursor.matches(&query, tree.root_node(), source) {
            let mut field_name = None;
            let mut wrapper_type = None;
            let mut ref_type = None;
            let mut actor_type = None;
            
            for capture in query_match.captures {
                let name = &query.capture_names()[capture.index as usize];
                let text = capture.node.utf8_text(source)?;
                
                match *name {
                    "field_name" => field_name = Some(text),
                    "wrapper_type" => wrapper_type = Some(text),
                    "ref_type" => ref_type = Some(text),
                    "actor_type" => actor_type = Some(text),
                    _ => {}
                }
            }
            
            // Check if this is an ActorRef field (with or without Option wrapper)
            if let (Some(field), Some(ref_t), Some(actor)) = (field_name, ref_type, actor_type) {
                if ref_t == "ActorRef" {
                    // Map both the full field name and the stripped version
                    actor_ref_map.insert(field.to_string(), actor.to_string());
                    
                    // Also map without _ref suffix if present
                    if field.ends_with("_ref") {
                        let stripped = field.trim_end_matches("_ref");
                        actor_ref_map.insert(stripped.to_string(), actor.to_string());
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn detect_actor_ref_variables(
        &mut self,
        tree: &Tree,
        source: &[u8],
    ) -> Result<HashMap<String, String>> {
        let mut actor_ref_map = HashMap::new();
        
        // First, extract struct fields that are ActorRef types
        self.extract_struct_field_actor_refs(tree, source, &mut actor_ref_map)?;
        
        let mut cursor = tree_sitter::QueryCursor::new();

        for query_match in cursor.matches(&self.queries.actor_ref_query, tree.root_node(), source) {
            let captures: HashMap<&str, (tree_sitter::Node, &str)> = query_match
                .captures
                .iter()
                .filter_map(|capture| {
                    let name =
                        &self.queries.actor_ref_query.capture_names()[capture.index as usize];
                    match capture.node.utf8_text(source) {
                        Ok(text) => Some((*name, (capture.node, text))),
                        Err(_) => None,
                    }
                })
                .collect();

            // Check for ActorRef<T> type declaration
            if let (Some((_, actor_type)), Some((_, var_name))) =
                (captures.get("actor_type"), captures.get("var_name"))
            {
                // Special case: ActorRef<Self> should map to "Self" which will be resolved later
                if *actor_type == "Self" {
                    actor_ref_map.insert(var_name.to_string(), "Self".to_string());
                } else {
                    actor_ref_map.insert(var_name.to_string(), actor_type.to_string());
                }
            }
            // Check for spawn call assignments
            else if let (Some((_, spawn_call)), Some((_, var_name))) =
                (captures.get("spawn_call"), captures.get("var_name"))
            {
                // Extract actor name from spawn call (e.g., ExampleActor::spawn -> ExampleActor)
                if spawn_call.contains("::spawn") {
                    let actor_name = spawn_call.split("::").next().unwrap_or("Unknown");
                    actor_ref_map.insert(var_name.to_string(), actor_name.to_string());
                }
            }
        }

        Ok(actor_ref_map)
    }

    fn extract_message_sends(
        &mut self,
        tree: &Tree,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
        actor_ref_map: &HashMap<String, String>,
    ) -> Result<Vec<MessageSend>> {
        let mut sends = Vec::new();
        
        // First pass: collect message variable types (let msg: MessageType = ...)
        let message_var_types = self.extract_message_variable_types(tree, source)?;
        
        let mut cursor = tree_sitter::QueryCursor::new();

        for query_match in cursor.matches(&self.queries.message_send_query, tree.root_node(), source) {
            let captures: HashMap<&str, (tree_sitter::Node, &str)> = query_match
                .captures
                .iter()
                .filter_map(|capture| {
                    let name =
                        &self.queries.message_send_query.capture_names()[capture.index as usize];
                    match capture.node.utf8_text(source) {
                        Ok(text) => Some((*name, (capture.node, text))),
                        Err(_) => None,
                    }
                })
                .collect();

            // Check if this is a tell or ask call
            if let Some((_, method)) = captures.get("method") {
                let send_method = match *method {
                    "tell" => SendMethod::Tell,
                    "ask" => SendMethod::Ask,
                    _ => continue,
                };

                // Get the receiver variable name (handle self.field patterns)
                let receiver_var = if let Some((_, receiver_text)) = captures.get("receiver") {
                    // Handle patterns like self.field or self.nested.field
                    if receiver_text.starts_with("self.") {
                        // Extract the field name, keeping underscores
                        let field = receiver_text.strip_prefix("self.")
                            .unwrap_or(receiver_text);
                        // For nested access like self.foo.bar, keep the last part
                        field.split('.').last().unwrap_or(field).to_string()
                    } else if receiver_text.contains('.') {
                        // Handle other field access patterns
                        receiver_text.split('.').last().unwrap_or(receiver_text).to_string()
                    } else {
                        receiver_text.to_string()
                    }
                } else {
                    "Unknown".to_string()
                };

                // Try to resolve the receiver actor type from the actor_ref_map
                let mut receiver_actor = actor_ref_map
                    .get(&receiver_var)
                    .cloned()
                    .unwrap_or_else(|| {
                        // Special handling for common context variable patterns
                        if receiver_var == "ctx" || receiver_var.ends_with("_ctx") || receiver_var.contains("ctx") {
                            // ctx and ctx.clone() patterns are self-references
                            "Self".to_string()
                        } else if receiver_var.ends_with("_actor") || receiver_var.ends_with("_ref") || receiver_var.ends_with("_actor_ref") {
                            // Try to convert snake_case to PascalCase for actor name
                            let name = receiver_var
                                .trim_end_matches("_actor_ref")
                                .trim_end_matches("_actor")
                                .trim_end_matches("_ref");
                            self.pascal_case(name)
                        } else {
                            "Unknown".to_string()
                        }
                    });
                
                // If receiver is "Self", try to resolve it to the actual actor name
                if receiver_actor == "Self" {
                    if let Some(node) = captures.get("method").map(|(n, _)| *n) {
                        if let Ok(actor_name) = self.find_containing_actor_impl(node, source) {
                            if actor_name != "Unknown" {
                                receiver_actor = actor_name;
                            }
                        }
                    }
                }

                // Try to find the sender actor from context
                let sender = if let Some(node) = captures.get("method").map(|(n, _)| *n) {
                    // First try to find if we're in an Actor impl
                    let actor_sender = self.find_containing_actor_impl(node, source)?;
                    if actor_sender != "Unknown" {
                        actor_sender
                    } else {
                        // For non-actor contexts, check if it's a test or main function
                        // These are legitimate senders even though they're not actors
                        let context = self.find_containing_context(node, source, file_path)?;
                        
                        // Only keep contexts that look like test functions or main functions
                        // Reject utility/helper functions that shouldn't be creating message sends
                        if context.ends_with("::Main") || context.contains("Test") || context.contains("test") {
                            context
                        } else {
                            "Unknown".to_string()
                        }
                    }
                } else {
                    "Unknown".to_string()
                };

                // Try to extract the message type from arguments
                let mut message_type = self.extract_message_type_from_args(&captures, source)?;
                
                // If we got a variable name, try to resolve it to a type
                if message_type != "Unknown" && !message_type.contains("::") && message_type.chars().next().map_or(false, |c| c.is_lowercase()) {
                    // This looks like a variable, try to resolve it
                    if let Some(resolved_type) = message_var_types.get(&message_type) {
                        message_type = resolved_type.clone();
                    }
                }

                let line = captures
                    .get("method")
                    .map(|(n, _)| n.start_position().row + 1)
                    .unwrap_or(0);

                let mut send = MessageSend {
                    id: String::new(),
                    sender_actor: sender.clone(),
                    sender_qualified: if sender != "Unknown" {
                        Some(format!(
                            "{}::{}",
                            self.infer_module_path_with_crate(file_path, crate_name)?,
                            sender
                        ))
                    } else {
                        None
                    },
                    receiver_actor: receiver_actor.clone(),
                    receiver_qualified: if receiver_actor != "Unknown" {
                        Some(format!(
                            "{}::{}",
                            self.infer_module_path_with_crate(file_path, crate_name)?,
                            receiver_actor
                        ))
                    } else {
                        None
                    },
                    message_type: message_type.clone(),
                    message_qualified: None, // Could be enhanced
                    send_method,
                    line,
                    file_path: file_path.to_string_lossy().to_string(),
                    from_crate: crate_name.to_string(),
                    to_crate: None, // Could be enhanced
                };

                send.id = format!(
                    "{}:{}->{}:{}:{}",
                    crate_name, sender, receiver_actor, message_type, line
                );
                sends.push(send);
            }
        }

        Ok(sends)
    }

    fn extract_reply_type(
        &self,
        captures: &HashMap<&str, (tree_sitter::Node, &str)>,
        source: &[u8],
    ) -> Result<String> {
        // Look for type Reply = ... in the impl block body
        if let Some((body_node, _)) = captures.get("body") {
            let mut cursor = body_node.walk();
            for child in body_node.children(&mut cursor) {
                if child.kind() == "type_item" {
                    let mut type_cursor = child.walk();
                    for type_child in child.children(&mut type_cursor) {
                        if let Ok(text) = type_child.utf8_text(source) {
                            if text == "Reply" {
                                // Found Reply type, get the value after =
                                if let Some(next_sibling) = type_child.next_sibling() {
                                    if let Some(value_sibling) = next_sibling.next_sibling() {
                                        if let Ok(reply_type) = value_sibling.utf8_text(source) {
                                            return Ok(reply_type
                                                .trim_end_matches(';')
                                                .trim()
                                                .to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok("Unknown".to_string())
    }

    fn extract_message_type_from_args(
        &self,
        captures: &HashMap<&str, (tree_sitter::Node, &str)>,
        source: &[u8],
    ) -> Result<String> {
        // Try to extract the message type from the arguments
        if let Some((args_node, args_text)) = captures.get("args") {
            // First, check if the entire argument text contains a scoped identifier with struct fields
            // Pattern: EnumType::Variant { ... }
            if args_text.contains("::") {
                // Extract the part before any { or (
                let clean_text = args_text
                    .trim_start_matches('(')
                    .trim_end_matches(')')
                    .trim();
                
                if let Some(type_part) = clean_text.split(|c| c == '{' || c == '(').next() {
                    let type_str = type_part.trim();
                    if type_str.contains("::") {
                        // Return the full Enum::Variant pattern
                        return Ok(type_str.to_string());
                    }
                }
            }
            
            let mut cursor = args_node.walk();
            for child in args_node.children(&mut cursor) {
                // Skip parentheses
                if child.kind() == "(" || child.kind() == ")" || child.kind() == "," {
                    continue;
                }
                
                match child.kind() {
                    // Struct expression: MyMessage { ... } or Enum::Variant { ... }
                    "struct_expression" => {
                        let mut struct_cursor = child.walk();
                        for struct_child in child.children(&mut struct_cursor) {
                            // Look for scoped_identifier (Enum::Variant) or type_identifier
                            match struct_child.kind() {
                                "scoped_identifier" => {
                                    if let Ok(text) = struct_child.utf8_text(source) {
                                        // Return the full Enum::Variant pattern
                                        return Ok(text.to_string());
                                    }
                                },
                                "type_identifier" => {
                                    if let Ok(text) = struct_child.utf8_text(source) {
                                        return Ok(text.to_string());
                                    }
                                },
                                _ => continue,
                            }
                        }
                    },
                    // Scoped identifier: MessageEnum::Variant or Module::Message
                    // This handles cases like BybitAccountingMessage::GetPosition (without fields)
                    // OR the start of BybitAccountingMessage::GetPosition { fields }
                    "scoped_identifier" => {
                        if let Ok(text) = child.utf8_text(source) {
                            // Return the full Enum::Variant pattern
                            return Ok(text.to_string());
                        }
                    },
                    // Simple identifier: msg or message_var
                    "identifier" => {
                        if let Ok(text) = child.utf8_text(source) {
                            // Common message variable names - we'll return the identifier
                            // The caller should resolve this to an actual type
                            return Ok(text.to_string());
                        }
                    },
                    // Call expression: Message::new() or MessageEnum::Variant(data)
                    "call_expression" => {
                        // Try to extract type from constructor call
                        let mut call_cursor = child.walk();
                        for call_child in child.children(&mut call_cursor) {
                            if call_child.kind() == "scoped_identifier" {
                                if let Ok(text) = call_child.utf8_text(source) {
                                    // For patterns like MessageEnum::Variant(...) or constructors
                                    // Return the full type
                                    return Ok(text.to_string());
                                }
                            }
                        }
                    },
                    _ => continue,
                }
            }
        }
        Ok("Unknown".to_string())
    }

    fn parse_spawn_method_name(&self, method_name: &str) -> Result<Option<SpawnMethod>> {
        let spawn_method = match method_name {
            "spawn" => SpawnMethod::Spawn,
            "spawn_with_mailbox" => SpawnMethod::SpawnWithMailbox,
            "spawn_link" => SpawnMethod::SpawnLink,
            "spawn_in_thread" => SpawnMethod::SpawnInThread,
            "spawn_with_storage" => SpawnMethod::SpawnWithStorage,
            _ => return Ok(None), // Not a spawn method
        };
        Ok(Some(spawn_method))
    }

    fn extract_actor_type_from_args(
        &self,
        captures: &HashMap<&str, (tree_sitter::Node, &str)>,
        source: &[u8],
    ) -> Option<String> {
        if let Some((args_node, _)) = captures.get("spawn_args") {
            // Priority 1: Look for constructor calls like SomeActor::new()
            let mut cursor = args_node.walk();
            for child in args_node.children(&mut cursor) {
                if child.kind() == "call_expression" {
                    // Look for scoped_identifier in the function part
                    let mut call_cursor = child.walk();
                    for call_child in child.children(&mut call_cursor) {
                        if call_child.kind() == "scoped_identifier" {
                            if let Ok(scoped_text) = call_child.utf8_text(source) {
                                // Extract actor type from "SomeActor::new"
                                if let Some((actor_type, method)) = scoped_text.rsplit_once("::") {
                                    if method == "new" || method == "default" || method == "create"
                                    {
                                        return Some(actor_type.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
                // Priority 2: Check for simple identifiers (variable names)
                else if child.kind() == "identifier" {
                    if let Ok(text) = child.utf8_text(source) {
                        // If it's a variable that looks like an actor, try to infer the type
                        if self.is_likely_actor_variable(text) {
                            // Convert snake_case to PascalCase for type inference
                            let inferred_type = self.infer_type_from_variable_name(text);
                            return Some(inferred_type);
                        }
                        // If it already looks like a type (PascalCase), use it directly
                        else if self.is_likely_actor_type(text) {
                            return Some(text.to_string());
                        }
                    }
                }
            }
        }
        None
    }

    /// Infer actor type from variable name (e.g., accounting_actor -> AccountingActor)
    fn infer_type_from_variable_name(&self, var_name: &str) -> String {
        // Handle common patterns
        if var_name.ends_with("_actor") {
            let base = &var_name[..var_name.len() - 6]; // Remove "_actor"
            return self.snake_to_pascal_case(base) + "Actor";
        }
        if var_name.ends_with("_supervisor") {
            let base = &var_name[..var_name.len() - 11]; // Remove "_supervisor"
            return self.snake_to_pascal_case(base) + "Supervisor";
        }
        if var_name.ends_with("_worker") {
            let base = &var_name[..var_name.len() - 7]; // Remove "_worker"
            return self.snake_to_pascal_case(base) + "Worker";
        }
        if var_name.ends_with("_handler") {
            let base = &var_name[..var_name.len() - 8]; // Remove "_handler"
            return self.snake_to_pascal_case(base) + "Handler";
        }

        // Fallback: just convert to PascalCase
        self.snake_to_pascal_case(var_name)
    }

    /// Convert snake_case to PascalCase
    fn snake_to_pascal_case(&self, snake: &str) -> String {
        snake
            .split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect()
    }

    fn detect_spawn_context(&self, mut node: tree_sitter::Node, source: &[u8]) -> String {
        // Walk up the tree to find the containing function and determine context
        while let Some(parent) = node.parent() {
            if parent.kind() == "function_item" {
                // Found the containing function, get its name
                let mut cursor = parent.walk();
                for child in parent.children(&mut cursor) {
                    if child.kind() == "identifier"
                        && child
                            .parent()
                            .map_or(false, |p| p.kind() == "function_item")
                    {
                        if let Ok(name) = child.utf8_text(source) {
                            // Classify the function context
                            let context_type = match name {
                                "on_start" => "on_start",
                                "on_stop" => "on_stop",
                                "reply" => "message_handler",
                                name if name.starts_with("handle_") => "message_handler",
                                name if name.starts_with("on_") => "lifecycle_handler",
                                name if name.contains("spawn") => "spawn_function",
                                "main" => "main_function",
                                "run" => "run_function",
                                _ => "regular_function",
                            };
                            return format!("{}:{}", context_type, name);
                        }
                    }
                }
                // Fallback if we can't get the function name
                return "function:unknown".to_string();
            }

            // Check if we're inside a impl block to get more context
            if parent.kind() == "impl_item" {
                // Check if this is an Actor impl block
                let mut impl_cursor = parent.walk();
                let mut is_actor_impl = false;
                for child in parent.children(&mut impl_cursor) {
                    if child.kind() == "type_identifier" {
                        if let Ok(text) = child.utf8_text(source) {
                            if text == "Actor" {
                                is_actor_impl = true;
                                break;
                            }
                        }
                    }
                }

                if is_actor_impl {
                    // Continue walking up to find the function name within Actor impl
                    node = parent;
                    continue;
                }
            }

            node = parent;
        }

        // If we didn't find a function, check if we're at module level
        "module_level".to_string()
    }

    // Task 1.2: Infer actors from spawn calls
    fn infer_actors_from_spawns(
        &mut self,
        tree: &Tree,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
    ) -> Result<Vec<RustActor>> {
        let mut inferred_actors = Vec::new();
        let mut cursor = tree_sitter::QueryCursor::new();
        let mut seen_actors = std::collections::HashSet::new();

        // First extract all spawn calls to identify actor types being spawned
        for query_match in cursor.matches(&self.queries.actor_spawn_query, tree.root_node(), source)
        {
            let captures: HashMap<&str, (tree_sitter::Node, &str)> = query_match
                .captures
                .iter()
                .filter_map(|capture| {
                    let name =
                        &self.queries.actor_spawn_query.capture_names()[capture.index as usize];
                    match capture.node.utf8_text(source) {
                        Ok(text) => Some((*name, (capture.node, text))),
                        Err(_) => None,
                    }
                })
                .collect();

            // Pattern 1: DirectType - ActorType::spawn(args)
            if let (Some((_, actor_type)), Some((_, method_name))) =
                (captures.get("actor_type"), captures.get("spawn_method"))
            {
                if self.parse_spawn_method_name(method_name)?.is_some() {
                    // Validate actor type before creating inferred actor
                    if self.is_likely_actor_type(actor_type)
                        && seen_actors.insert(actor_type.to_string())
                    {
                        let inferred_actor = self.create_inferred_actor(
                            actor_type,
                            file_path,
                            crate_name,
                            query_match
                                .captures
                                .first()
                                .map(|c| c.node.start_position().row + 1)
                                .unwrap_or(0),
                        )?;
                        inferred_actors.push(inferred_actor);
                    }
                }
            }
            // Pattern 2: Extract from arguments in Actor::spawn(instance) calls
            else if let (Some((_, trait_type)), Some((_, method_name))) =
                (captures.get("trait_type"), captures.get("spawn_method"))
            {
                if *trait_type == "Actor" && *method_name == "spawn" {
                    if let Some(child_actor_name) =
                        self.extract_actor_type_from_args(&captures, source)
                    {
                        // Don't create inferred actors for invalid types like "Actor", "Unknown", etc.
                        if child_actor_name != "Unknown"
                            && child_actor_name != "Actor"
                            && seen_actors.insert(child_actor_name.clone())
                        {
                            let inferred_actor = self.create_inferred_actor(
                                &child_actor_name,
                                file_path,
                                crate_name,
                                query_match
                                    .captures
                                    .first()
                                    .map(|c| c.node.start_position().row + 1)
                                    .unwrap_or(0),
                            )?;
                            inferred_actors.push(inferred_actor);
                        }
                    }
                }
            }
        }

        Ok(inferred_actors)
    }

    fn create_inferred_actor(
        &self,
        actor_name: &str,
        file_path: &Path,
        crate_name: &str,
        line: usize,
    ) -> Result<RustActor> {
        let module_path = self.infer_module_path_with_crate(file_path, crate_name)?;
        let qualified_name = format!("{}::{}", module_path, actor_name);

        Ok(RustActor {
            id: format!(
                "{}::{}:{} (inferred)",
                qualified_name,
                file_path.display(),
                line
            ),
            name: actor_name.to_string(),
            qualified_name,
            crate_name: crate_name.to_string(),
            module_path,
            file_path: file_path.to_string_lossy().to_string(),
            line_start: line,
            line_end: line,
            visibility: "unknown".to_string(), // Inferred actors have unknown visibility
            doc_comment: Some("Inferred from spawn call".to_string()),
            is_distributed: false,
            is_test: false, // Can't determine test status for inferred actors
            actor_type: ActorImplementationType::Unknown,
            local_messages: Vec::new(), // Inferred actors have no known local message handlers
            inferred_from_message: false, // Inferred from spawn, not Message impl
        })
    }

    // Task 1.1: Find actors with #[derive(Actor)] macro pattern
    fn extract_derive_actors(
        &mut self,
        tree: &Tree,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
    ) -> Result<Vec<RustActor>> {
        let mut derive_actors = Vec::new();

        // Use a simple struct query and then check for derive attributes in source text
        let struct_query = tree_sitter::Query::new(
            &tree_sitter_rust::language(),
            r#"
            (struct_item
              name: (type_identifier) @struct_name
            )
            "#,
        )?;

        let source_str = std::str::from_utf8(source)?;

        let mut cursor = tree_sitter::QueryCursor::new();
        for query_match in cursor.matches(&struct_query, tree.root_node(), source) {
            let captures: HashMap<&str, (tree_sitter::Node, &str)> = query_match
                .captures
                .iter()
                .filter_map(|capture| {
                    let name = &struct_query.capture_names()[capture.index as usize];
                    match capture.node.utf8_text(source) {
                        Ok(text) => Some((*name, (capture.node, text))),
                        Err(_) => None,
                    }
                })
                .collect();

            if let Some((struct_node, struct_name)) = captures.get("struct_name") {
                // Check if there's a #[derive(...)] attribute before this struct that contains Actor
                let struct_start_byte = struct_node.start_byte();
                let preceding_text = &source_str[..struct_start_byte];

                // Look for derive attributes in the preceding lines (simple heuristic)
                if let Some(last_derive_line) = preceding_text
                    .lines()
                    .rev()
                    .take(10)
                    .find(|line| line.trim().contains("#[derive"))
                {
                    if last_derive_line.contains("Actor") {
                        let module_path = self.infer_module_path_with_crate(file_path, crate_name)?;
                        let qualified_name = format!("{}::{}", module_path, struct_name);

                        let actor = RustActor {
                            id: format!(
                                "{}::{}:{}",
                                qualified_name,
                                file_path.display(),
                                struct_node.start_position().row + 1
                            ),
                            name: struct_name.to_string(),
                            qualified_name,
                            crate_name: crate_name.to_string(),
                            module_path,
                            file_path: file_path.to_string_lossy().to_string(),
                            line_start: struct_node.start_position().row + 1,
                            line_end: struct_node.end_position().row + 1,
                            visibility: "pub".to_string(), // Most actors are public
                            doc_comment: Some("Derived Actor".to_string()),
                            is_distributed: false,
                            is_test: false, // Can't determine test status for derived actors
                            actor_type: ActorImplementationType::Local,
                            local_messages: Vec::new(), // Derived actors need separate analysis for message handlers
                            inferred_from_message: false, // Explicitly derived with #[derive(Actor)]
                        };
                        derive_actors.push(actor);
                    }
                }
            }
        }

        Ok(derive_actors)
    }

    // CRITICAL FIX: Actor Framework Filtering Helper Functions

    /// Checks if a given identifier represents a non-actor framework that should be filtered out
    fn is_non_actor_framework(&self, identifier: &str) -> bool {
        // Blacklist of non-actor frameworks and standard library spawn patterns
        matches!(
            identifier,
            "tokio"
                | "std"
                | "async_std"
                | "futures"
                | "runtime"
                | "task"
                | "thread"
                | "executor"
                | "spawn_blocking"
                | "smol"
                | "async_global_executor"
                | "blocking"
                | "rayon"
        )
    }

    /// Checks if the module/function combo represents an actor framework spawn
    fn is_actor_framework_spawn(
        &self,
        module: &str,
        actor_module: Option<&str>,
        function: &str,
    ) -> bool {
        // Known actor framework spawn patterns
        match (module, actor_module, function) {
            ("kameo", Some("actor"), "spawn") => true,
            ("kameo", Some("actor"), "spawn_with_mailbox") => true,
            ("actix", Some("actor"), "spawn") => true,
            ("actix", Some("spawn"), _) => true,
            ("riker", Some("actor"), "spawn") => true,
            ("bastion", _, "spawn") => true,
            ("coerce", _, "spawn") => true,
            // Add more actor frameworks as needed
            _ => false,
        }
    }

    /// Heuristic to determine if an identifier looks like an actor type
    fn is_likely_actor_type(&self, identifier: &str) -> bool {
        // Must not be a known non-actor framework first
        if self.is_non_actor_framework(identifier) {
            return false;
        }

        // CRITICAL FIX: Reject generic trait names
        if identifier == "Actor" || identifier == "Message" || identifier == "Handler" {
            return false; // These are trait names, not actor types
        }

        // Heuristics for actor type detection
        let is_actor_named = identifier.ends_with("Actor")
            || identifier.ends_with("Supervisor")
            || identifier.ends_with("Worker")
            || identifier.ends_with("Handler")
            || identifier.ends_with("Agent")
            || identifier.ends_with("Service");

        let is_proper_case = identifier
            .chars()
            .next()
            .map_or(false, |c| c.is_uppercase());

        let has_actor_context = identifier.contains("Actor")
            || identifier.contains("Supervisor")
            || identifier.contains("Manager");

        // Must be proper case (like a type name) and either have actor naming or context
        is_proper_case && (is_actor_named || has_actor_context)
    }

    /// Check if an identifier looks like an actor variable (snake_case)
    fn is_likely_actor_variable(&self, identifier: &str) -> bool {
        // Variable names that look like actors
        identifier.ends_with("_actor")
            || identifier.ends_with("_supervisor")
            || identifier.ends_with("_worker")
            || identifier.ends_with("_handler")
            || identifier.ends_with("_agent")
            || identifier.ends_with("_service")
            || identifier.contains("actor_")
            || identifier.contains("supervisor_")
            || identifier.contains("manager_")
    }

    /// Try to infer actor name from function context
    fn infer_actor_from_function_context(
        &self,
        function_node: tree_sitter::Node,
        source: &[u8],
    ) -> Result<Option<String>> {
        // Look for surrounding impl blocks or struct definitions
        let mut current = function_node;
        while let Some(parent) = current.parent() {
            // Check if we're inside an impl block
            if parent.kind() == "impl_item" {
                // Try to extract the type being implemented
                for child_idx in 0..parent.child_count() {
                    if let Some(child) = parent.child(child_idx) {
                        if child.kind() == "type_identifier" {
                            if let Ok(text) = child.utf8_text(source) {
                                if self.is_likely_actor_type(text) {
                                    return Ok(Some(text.to_string()));
                                }
                            }
                        }
                    }
                }
            }
            current = parent;
        }
        Ok(None)
    }

    /// Final fallback to infer actor context from surrounding code
    fn infer_actor_from_context(&self, node: tree_sitter::Node, source: &[u8]) -> Result<String> {
        // Look for actor-related patterns in the immediate context
        let mut current = node;

        // Walk up a few levels to find any actor-related identifiers
        for _ in 0..5 {
            if let Some(parent) = current.parent() {
                // Check all child nodes for actor-like type names
                for child_idx in 0..parent.child_count() {
                    if let Some(child) = parent.child(child_idx) {
                        if child.kind() == "type_identifier" {
                            if let Ok(text) = child.utf8_text(source) {
                                if self.is_likely_actor_type(text) {
                                    return Ok(text.to_string());
                                }
                            }
                        }
                    }
                }
                current = parent;
            } else {
                break;
            }
        }

        // No actor context found
        Ok("Unknown".to_string())
    }

    // Task 1.3: Find actors from ActorRef<T> and other type usage
    fn extract_actors_from_type_usage(
        &mut self,
        tree: &Tree,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
    ) -> Result<Vec<RustActor>> {
        let mut type_usage_actors = Vec::new();
        let mut seen_actors = std::collections::HashSet::new();

        // Use existing actor_ref_query to find ActorRef<T> patterns
        let mut cursor = tree_sitter::QueryCursor::new();
        for query_match in cursor.matches(&self.queries.actor_ref_query, tree.root_node(), source) {
            let captures: HashMap<&str, (tree_sitter::Node, &str)> = query_match
                .captures
                .iter()
                .filter_map(|capture| {
                    let name =
                        &self.queries.actor_ref_query.capture_names()[capture.index as usize];
                    match capture.node.utf8_text(source) {
                        Ok(text) => Some((*name, (capture.node, text))),
                        Err(_) => None,
                    }
                })
                .collect();

            // Check for ActorRef<T> type usage
            if let Some((actor_node, actor_type)) = captures.get("actor_type") {
                if seen_actors.insert(actor_type.to_string()) {
                    let module_path = self.infer_module_path_with_crate(file_path, crate_name)?;
                    let qualified_name = format!("{}::{}", module_path, actor_type);

                    let actor = RustActor {
                        id: format!(
                            "{}::{}:{} (type_usage)",
                            qualified_name,
                            file_path.display(),
                            actor_node.start_position().row + 1
                        ),
                        name: actor_type.to_string(),
                        qualified_name,
                        crate_name: crate_name.to_string(),
                        module_path,
                        file_path: file_path.to_string_lossy().to_string(),
                        line_start: actor_node.start_position().row + 1,
                        line_end: actor_node.end_position().row + 1,
                        visibility: "unknown".to_string(),
                        doc_comment: Some("Inferred from ActorRef usage".to_string()),
                        is_distributed: false,
                        is_test: false, // Can't determine test status for type usage
                        actor_type: ActorImplementationType::Unknown,
                        local_messages: Vec::new(), // Type usage actors have no known local message handlers
                        inferred_from_message: false, // Inferred from ActorRef usage, not Message impl
                    };
                    type_usage_actors.push(actor);
                }
            }
        }

        Ok(type_usage_actors)
    }

    /// Extract macro expansions, specifically paste! macros that generate function calls
    fn extract_macro_expansions(
        &mut self,
        source: &[u8],
        file_path: &Path,
        crate_name: &str,
    ) -> Result<Vec<MacroExpansion>> {
        let mut macro_expansions = Vec::new();
        let source_str = String::from_utf8_lossy(source);
        
        // Detect paste! macro patterns for trading indicators
        let patterns = self.detect_paste_macro_patterns(&source_str);
        
        for pattern in patterns {
            
            // Use enhanced function context resolution
            let containing_function = self.find_containing_function_enhanced(pattern.line, &source_str);
            
            let expansion = MacroExpansion {
                id: format!("{}:{}:{}", file_path.display(), pattern.line, pattern.macro_type),
                crate_name: crate_name.to_string(),
                file_path: file_path.to_string_lossy().to_string(),
                line_range: pattern.line..pattern.line+1,
                macro_type: pattern.macro_type.clone(),
                expansion_pattern: pattern.pattern.clone(),
                target_functions: Vec::new(), // Will be populated by SyntheticCallGenerator
                containing_function,          // Enhanced: resolved from function context
                expansion_context: MacroContext {
                    expansion_id: format!("{}:{}:{}", file_path.display(), pattern.line, pattern.macro_type),
                    macro_type: pattern.macro_type.clone(),
                    expansion_site_line: pattern.line,
                },
            };
            
            macro_expansions.push(expansion);
        }
        
        Ok(macro_expansions)
    }

    /// Detect paste! macro patterns in source code using enhanced pattern matching
    fn detect_paste_macro_patterns(&self, source: &str) -> Vec<MacroPattern> {
        let mut patterns = Vec::new();
        
        // Early exit if source doesn't contain paste! or define_indicator_enums! at all
        if !source.contains("paste!") && !source.contains("define_indicator_enums!") {
            return patterns;
        }
        
        
        // Look for the actual pattern: define_indicator_enums! macro invocation
        // This generates all the paste! patterns internally
        static PASTE_PATTERNS: &[&str] = &[
            r"define_indicator_enums!\s*\(",                             // define_indicator_enums!(
            r"paste!\s*\{\s*\[\s*<\s*\$\w+\s*>\s*\]::\w+\s*\(",        // paste! { [<$indicator>]::new(
            r"\[\s*<\s*\$\w+\s*>\s*\]::\w+\s*\(",                       // [<$indicator>]::new( (without paste!)
            r"paste!\s*\{\s*\[\s*<\s*\$\w+\s+\w+\s*>\s*\]::\w+\s*\(",  // paste! { [<$indicator Input>]::from_ohlcv(
            r"\[\s*<\s*\$\w+\s+\w+\s*>\s*\]::\w+\s*\(",                 // [<$indicator Input>]::from_ohlcv( (without paste!)
        ];
        
        use regex::Regex;
        let mut compiled_patterns = Vec::new();
        for pattern in PASTE_PATTERNS {
            if let Ok(regex) = Regex::new(pattern) {
                compiled_patterns.push(regex);
            }
        }
        
        // Pre-allocate pattern vector with reasonable capacity
        patterns.reserve(8); // Most files won't have more than a few macros
        
        // Multi-line paste! macro detection
        let lines: Vec<&str> = source.lines().collect();
        let mut i = 0;
        
        while i < lines.len() {
            let line = lines[i];
            let line_number = i + 1;
            
            // Look for paste! macro start
            if line.trim().contains("paste!") && line.trim().contains("{") {
                // Capture the full paste! block
                if let Some(pattern) = self.parse_paste_macro_block(&lines, i, &compiled_patterns) {
                    // Skip to the end of this block (pattern.line contains the end line)
                    let skip_to = pattern.line;
                    patterns.push(pattern);
                    i = skip_to;
                }
            }
            // Also handle single-line patterns and define_indicator_enums!
            else if line.contains("paste!") || line.contains("[<$") || line.contains("define_indicator_enums!") {
                if let Some(pattern) = self.parse_paste_macro_line_enhanced(line, line_number, &compiled_patterns) {
                    patterns.push(pattern);
                }
            }
            
            i += 1;
        }
        
        
        patterns.shrink_to_fit(); // Free unused capacity
        patterns
    }

    /// Parse a multi-line paste! macro block
    fn parse_paste_macro_block(&self, lines: &[&str], start_line_idx: usize, compiled_patterns: &[regex::Regex]) -> Option<MacroPattern> {
        let start_line_number = start_line_idx + 1;
        let mut block_content = Vec::new();
        let mut brace_count = 0;
        let mut method = "new"; // Default method
        let mut found_pattern = false;
        
        // Process the paste! block
        for (i, line) in lines.iter().enumerate().skip(start_line_idx) {
            let trimmed = line.trim();
            block_content.push(*line);
            
            // Count braces to find the end of the block
            brace_count += trimmed.chars().filter(|c| *c == '{').count() as i32;
            brace_count -= trimmed.chars().filter(|c| *c == '}').count() as i32;
            
            // Look for the pattern inside the block
            if trimmed.contains("[<$") && trimmed.contains(">]::") {
                found_pattern = true;
                
                // Determine the method being called
                if trimmed.contains("::new(") {
                    method = "new";
                } else if trimmed.contains("::from_ohlcv(") {
                    method = "from_ohlcv";
                } else if trimmed.contains("::na(") {
                    method = "na";
                } else if trimmed.contains("::nan(") {
                    method = "nan";
                }
            }
            
            // End of block reached
            if brace_count == 0 && i > start_line_idx {
                let full_pattern = block_content.join("\n");
                
                if found_pattern {
                    return Some(MacroPattern {
                        line: i + 1, // End line for skipping
                        macro_type: "paste".to_string(),
                        pattern: full_pattern,
                        method: method.to_string(),
                    });
                }
                break;
            }
        }
        
        None
    }

    /// Enhanced parse function for paste! macro lines using regex patterns
    fn parse_paste_macro_line_enhanced(&self, line: &str, line_number: usize, compiled_patterns: &[regex::Regex]) -> Option<MacroPattern> {
        let trimmed = line.trim();
        
        // Test against enhanced regex patterns
        for pattern in compiled_patterns {
            if pattern.is_match(trimmed) {
                // Extract method name from the matched pattern
                let method = if trimmed.contains("::new(") {
                    "new"
                } else if trimmed.contains("::from_ohlcv(") || trimmed.contains("::from_ohlcv ") {
                    "from_ohlcv"
                } else if trimmed.contains("::na(") {
                    "na"
                } else if trimmed.contains("::nan(") {
                    "nan"
                } else {
                    // Default to "new" for unrecognized patterns
                    "new"
                };
                
                return Some(MacroPattern {
                    line: line_number,
                    macro_type: "paste".to_string(),
                    pattern: trimmed.to_string(),
                    method: method.to_string(),
                });
            }
        }
        
        // Fallback to original simple parsing for compatibility
        self.parse_paste_macro_line_fallback(line, line_number)
    }

    /// Fallback parse function (original implementation) for compatibility
    fn parse_paste_macro_line_fallback(&self, line: &str, line_number: usize) -> Option<MacroPattern> {
        // Remove whitespace and look for paste! patterns
        let trimmed = line.trim();
        
        // Match patterns like: paste! { [<$indicator>]::new(config) }
        if trimmed.contains("paste!") && trimmed.contains("[<$") && trimmed.contains(">]::") {
            // Determine the method being called
            let method = if trimmed.contains("::new(") {
                "new"
            } else if trimmed.contains("::from_ohlcv(") || trimmed.contains("::from_ohlcv ") {
                "from_ohlcv"
            } else {
                return None;
            };
            
            return Some(MacroPattern {
                line: line_number,
                macro_type: "paste".to_string(),
                pattern: trimmed.to_string(),
                method: method.to_string(),
            });
        }
        
        None
    }

    /// Enhanced function context resolution as per specification
    fn find_containing_function_enhanced(&mut self, macro_line: usize, source: &str) -> Option<String> {
        // Use AST node ranges for precise function boundary detection
        match self.parse_functions_with_ranges(source) {
            Ok(functions) => {
                functions.into_iter()
                    .find(|func| func.line_range.contains(&macro_line))
                    .map(|func| func.id)
            }
            Err(_) => None
        }
    }

    /// Parse functions with their line ranges for better context resolution
    fn parse_functions_with_ranges(&mut self, source: &str) -> Result<Vec<FunctionWithRange>> {
        let mut cursor = tree_sitter::QueryCursor::new();
        let tree = self.parser.parse(source.as_bytes(), None).ok_or_else(|| {
            anyhow::anyhow!("Failed to parse source code")
        })?;

        // Query to find all function definitions
        let function_query = tree_sitter::Query::new(
            &tree_sitter_rust::language(),
            r#"
            (function_item
                name: (identifier) @name
                parameters: (parameters) @params
                body: (block) @body
            ) @function
            "#,
        )?;

        let mut functions = Vec::new();
        let source_bytes = source.as_bytes();

        for query_match in cursor.matches(&function_query, tree.root_node(), source_bytes) {
            let mut function_name = None;
            let mut function_node = None;

            for capture in query_match.captures {
                match function_query.capture_names()[capture.index as usize] {
                    "name" => {
                        function_name = Some(capture.node.utf8_text(source_bytes).unwrap_or("").to_string());
                    }
                    "function" => {
                        function_node = Some(capture.node);
                    }
                    _ => {}
                }
            }

            if let (Some(name), Some(node)) = (function_name, function_node) {
                let line_range = (node.start_position().row + 1)..(node.end_position().row + 1);
                
                // Generate function ID in same format as existing code
                let function_id = format!("function::{}", name); // Simplified for now
                
                functions.push(FunctionWithRange {
                    id: function_id,
                    name,
                    line_range,
                });
            }
        }

        Ok(functions)
    }

    /// Extract macro expansion range for multi-line support
    fn extract_macro_expansion_range(&self, node: tree_sitter::Node, _source: &str) -> std::ops::Range<usize> {
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;
        start_line..end_line
    }
}

impl QuerySet {
    fn new(language: Language) -> Result<Self> {
        let function_query = Query::new(
            &language,
            r#"
            (function_item
              (visibility_modifier)? @visibility
              "async"? @async
              "unsafe"? @unsafe
              "fn" @fn
              name: (identifier) @name
              type_parameters: (type_parameters)? @generic
              parameters: (parameters) @params
              return_type: (_)? @return_type
              body: (block) @body
            )
            "#,
        )?;

        let type_query = Query::new(
            &language,
            r#"
            [
              (struct_item
                (visibility_modifier)? @visibility
                "struct" @struct
                name: (type_identifier) @name
              )
              (enum_item
                (visibility_modifier)? @visibility
                "enum" @enum
                name: (type_identifier) @name
              )
              (trait_item
                (visibility_modifier)? @visibility
                "trait" @trait
                name: (type_identifier) @name
              )
              (type_item
                (visibility_modifier)? @visibility
                "type" @type_alias
                name: (type_identifier) @name
              )
            ]
            "#,
        )?;

        let impl_query = Query::new(
            &language,
            r#"
            (impl_item
              trait: (type_identifier) @trait_name
              "for"
              type: (type_identifier) @type_name
              body: (declaration_list) @body
            ) @impl_block
            (impl_item
              trait: (type_identifier) @trait_name
              "for"
              type: (primitive_type) @type_name
              body: (declaration_list) @body
            ) @impl_block_primitive
            (impl_item
              trait: (generic_type
                type: (type_identifier) @trait_name
              )
              "for"
              type: (type_identifier) @type_name
              body: (declaration_list) @body
            ) @impl_block_generic
            (impl_item
              trait: (generic_type
                type: (type_identifier) @trait_name
              )
              "for"
              type: (primitive_type) @type_name
              body: (declaration_list) @body
            ) @impl_block_generic_primitive
            (impl_item
              trait: (scoped_type_identifier
                path: (identifier) @trait_path
                name: (type_identifier) @trait_name
              )
              "for"
              type: (type_identifier) @type_name
              body: (declaration_list) @body
            ) @impl_block_scoped
            (impl_item
              trait: (scoped_type_identifier
                path: (scoped_identifier) @trait_path
                name: (type_identifier) @trait_name
              )
              "for"
              type: (type_identifier) @type_name
              body: (declaration_list) @body
            ) @impl_block_nested_scoped
            (impl_item
              trait: (type_identifier) @trait_name
              "for"
              type: (primitive_type) @type_name
              body: (declaration_list) @body
            ) @impl_block_primitive
            (impl_item
              trait: (scoped_type_identifier
                name: (type_identifier) @trait_name
              )
              "for"
              type: (primitive_type) @type_name
              body: (declaration_list) @body
            ) @impl_block_scoped_primitive
            (impl_item
              type: (type_identifier) @type_name
              body: (declaration_list) @body
            ) @simple_impl_block
            "#,
        )?;

        let call_query = Query::new(
            &language,
            r#"
            [
              (call_expression
                function: (identifier) @function
              )
              (call_expression
                function: (field_expression
                  field: (field_identifier) @method
                )
              )
              (call_expression
                function: (scoped_identifier) @scoped_call
              )
              (call_expression
                function: (generic_function) @generic_call
              )
              ; Type::method patterns (associated functions) - already handled by scoped_identifier
              (macro_invocation
                macro: (identifier) @macro_name
              )
            ]
            "#,
        )?;

        let import_query = Query::new(
            &language,
            r#"
            (use_declaration) @use_decl
            "#,
        )?;

        let module_query = Query::new(
            &language,
            r#"
            (mod_item
              (visibility_modifier)? @visibility
              "mod" @mod
              name: (identifier) @name
              body: (declaration_list)? @body
            )
            "#,
        )?;

        let actor_impl_query = Query::new(
            &language,
            r#"
            (impl_item
              trait: (type_identifier) @trait_name (#eq? @trait_name "Actor")
              "for"
              type: (type_identifier) @actor_name
              body: (declaration_list) @body
            ) @impl
            "#,
        )?;

        let actor_spawn_query = Query::new(
            &language,
            r#"
            (call_expression
              function: [
                ; Pattern 1: ActorType::spawn(args) - direct type method call
                (scoped_identifier
                  path: (identifier) @actor_type
                  name: (identifier) @spawn_method
                ) @scoped_spawn_call
                ; Pattern 2: Actor::spawn(instance) - trait method call  
                (scoped_identifier
                  path: (identifier) @trait_type
                  name: (identifier) @spawn_method
                ) @trait_spawn_call
                ; Pattern 3: kameo::actor::spawn(instance) - module function call
                (scoped_identifier
                  path: (scoped_identifier
                    path: (identifier) @module_path
                    name: (identifier) @actor_module
                  )
                  name: (identifier) @spawn_function
                ) @module_spawn_call
              ]
              arguments: (arguments) @spawn_args
            )
            "#,
        )?;

        // Query for message type definitions (enums/structs ending with Tell, Ask, Message, Query)
        let message_type_query = Query::new(
            &language,
            r#"
            [
              (enum_item
                (visibility_modifier)? @visibility
                "enum" @enum
                name: (type_identifier) @name
                body: (enum_variant_list) @body
              )
              (struct_item
                (visibility_modifier)? @visibility
                "struct" @struct
                name: (type_identifier) @name
                body: (field_declaration_list)? @body
              )
            ]
            "#,
        )?;

        // Query for Message trait implementations
        let message_handler_query = Query::new(
            &language,
            r#"
            (impl_item
              trait: (generic_type
                type: [
                  (type_identifier) @trait_name
                  (scoped_type_identifier
                    name: (type_identifier) @trait_name
                  )
                ]
                type_arguments: (type_arguments
                  (type_identifier) @message_type
                )
              )
              "for"
              type: [
                (type_identifier) @actor_type
                (scoped_type_identifier
                  name: (type_identifier) @actor_type
                )
              ]
              body: (declaration_list) @body
            )
            "#,
        )?;

        // Query for ActorRef variable declarations to track actor types
        let actor_ref_query = Query::new(
            &language,
            r#"
            [
              (let_declaration
                pattern: (identifier) @var_name
                value: (call_expression
                  function: (scoped_identifier) @spawn_call
                )
              )
              (let_declaration
                pattern: (identifier) @var_name
                type: (generic_type
                  type: (type_identifier) @ref_type
                  type_arguments: (type_arguments
                    (type_identifier) @actor_type
                  )
                )
              )
            ]
            "#,
        )?;

        // Query for tell and ask calls
        // This handles both direct calls and chained calls like .tell(msg).send()
        let message_send_query = Query::new(
            &language,
            r#"
            (call_expression
              function: (field_expression
                value: (_) @receiver
                field: (field_identifier) @method
              )
              arguments: (arguments) @args
            )
            "#,
        )?;

        Ok(Self {
            function_query,
            type_query,
            impl_query,
            call_query,
            import_query,
            module_query,
            actor_impl_query,
            actor_spawn_query,
            message_type_query,
            message_handler_query,
            actor_ref_query,
            message_send_query,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_simple_function() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"
            pub fn add(x: i32, y: i32) -> i32 {
                x + y
            }
        "#;

        let symbols = parser
            .parse_source(source, &PathBuf::from("test.rs"), "test_crate")
            .unwrap();

        assert_eq!(symbols.functions.len(), 1);
        let func = &symbols.functions[0];
        assert_eq!(func.name, "add");
        assert_eq!(func.visibility, "pub");
        assert!(func.return_type.is_some());
    }

    #[test]
    fn test_parse_struct() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"
            pub struct Person {
                name: String,
                age: u32,
            }
        "#;

        let symbols = parser
            .parse_source(source, &PathBuf::from("test.rs"), "test_crate")
            .unwrap();

        assert_eq!(symbols.types.len(), 1);
        let type_def = &symbols.types[0];
        assert_eq!(type_def.name, "Person");
        assert_eq!(type_def.visibility, "pub");
        assert!(matches!(type_def.kind, TypeKind::Struct));
    }
}
