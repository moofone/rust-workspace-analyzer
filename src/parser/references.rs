use anyhow::Result;
use std::collections::HashMap;

use crate::parser::symbols::*;

pub struct ReferenceResolver {
    symbol_table: HashMap<String, ResolvedSymbol>,
    import_table: HashMap<String, Vec<ImportedSymbol>>, // file_path -> imported symbols
}

#[derive(Debug, Clone)]
pub struct ImportedSymbol {
    pub local_name: String,        // The name to use locally (could be original or alias)
    pub qualified_name: String,    // The fully qualified name of the symbol
    pub crate_name: String,        // Which crate it comes from
}

#[derive(Debug, Clone)]
pub struct ResolvedSymbol {
    pub qualified_name: String,
    pub crate_name: String,
    pub symbol_type: SymbolType,
}

#[derive(Debug, Clone)]
pub enum SymbolType {
    Function,
    Type,
    Module,
    Constant,
}

impl ReferenceResolver {
    pub fn new() -> Self {
        Self {
            symbol_table: HashMap::new(),
            import_table: HashMap::new(),
        }
    }

    pub fn build_symbol_table(&mut self, symbols: &ParsedSymbols) -> Result<()> {
        // Build import table first
        self.build_import_table(&symbols.imports)?;
        
        // Then build symbol table
        for function in &symbols.functions {
            let resolved_symbol = ResolvedSymbol {
                qualified_name: function.qualified_name.clone(),
                crate_name: function.crate_name.clone(),
                symbol_type: SymbolType::Function,
            };

            // Add the original qualified name (e.g., "crate::function_a")
            self.symbol_table.insert(
                function.qualified_name.clone(),
                resolved_symbol.clone(),
            );

            // Add the simple name (e.g., "function_a")
            self.symbol_table.insert(
                function.name.clone(),
                resolved_symbol.clone(),
            );

            // Add cross-crate qualified name (e.g., "crate_a::function_a")
            if function.qualified_name.starts_with("crate::") {
                let cross_crate_name = function.qualified_name.replace("crate::", &format!("{}::", function.crate_name));
                self.symbol_table.insert(
                    cross_crate_name,
                    resolved_symbol.clone(),
                );
            }
        }

        for rust_type in &symbols.types {
            let resolved_symbol = ResolvedSymbol {
                qualified_name: rust_type.qualified_name.clone(),
                crate_name: rust_type.crate_name.clone(),
                symbol_type: SymbolType::Type,
            };

            // Add the original qualified name (e.g., "crate::MyType")
            self.symbol_table.insert(
                rust_type.qualified_name.clone(),
                resolved_symbol.clone(),
            );

            // Add the simple name (e.g., "MyType")
            self.symbol_table.insert(
                rust_type.name.clone(),
                resolved_symbol.clone(),
            );

            // Add cross-crate qualified name (e.g., "crate_a::MyType")
            if rust_type.qualified_name.starts_with("crate::") {
                let cross_crate_name = rust_type.qualified_name.replace("crate::", &format!("{}::", rust_type.crate_name));
                self.symbol_table.insert(
                    cross_crate_name,
                    resolved_symbol.clone(),
                );
            }
        }

        for module in &symbols.modules {
            self.symbol_table.insert(
                module.path.clone(),
                ResolvedSymbol {
                    qualified_name: module.path.clone(),
                    crate_name: module.crate_name.clone(),
                    symbol_type: SymbolType::Module,
                },
            );
        }

        Ok(())
    }

    fn build_import_table(&mut self, imports: &[RustImport]) -> Result<()> {
        for import in imports {
            let mut imported_symbols = Vec::new();

            match &import.import_type {
                ImportType::Simple => {
                    // use crate_a::function_a;
                    for item in &import.imported_items {
                        let local_name = item.alias.as_ref().unwrap_or(&item.name);
                        let qualified_name = format!("{}::{}", import.module_path, item.name);
                        
                        imported_symbols.push(ImportedSymbol {
                            local_name: local_name.clone(),
                            qualified_name: qualified_name.clone(),
                            crate_name: import.module_path.split("::").next().unwrap_or(&import.module_path).to_string(),
                        });
                    }
                }
                ImportType::Grouped => {
                    // use crate_a::{function_a, utility_function};
                    for item in &import.imported_items {
                        let local_name = item.alias.as_ref().unwrap_or(&item.name);
                        let qualified_name = format!("{}::{}", import.module_path, item.name);
                        
                        imported_symbols.push(ImportedSymbol {
                            local_name: local_name.clone(),
                            qualified_name: qualified_name.clone(),
                            crate_name: import.module_path.split("::").next().unwrap_or(&import.module_path).to_string(),
                        });
                    }
                }
                ImportType::Module => {
                    // use crate_a; - makes crate_a available as a namespace
                    let module_parts: Vec<&str> = import.module_path.split("::").collect();
                    let local_name = module_parts.last().unwrap_or(&"module").to_string();
                    
                    imported_symbols.push(ImportedSymbol {
                        local_name,
                        qualified_name: import.module_path.clone(),
                        crate_name: import.module_path.split("::").next().unwrap_or(&import.module_path).to_string(),
                    });
                }
                ImportType::Glob => {
                    // use crate_a::*; - we'll need to resolve this later when we know all symbols
                    // For now, we'll mark this as needing global resolution
                    continue;
                }
            }

            // Add to import table for this file
            self.import_table.entry(import.file_path.clone())
                .or_insert_with(Vec::new)
                .extend(imported_symbols);
        }

        Ok(())
    }

    pub fn resolve_call(&self, call_name: &str, context_module: &str, context_crate: &str, context_file: &str) -> Option<ResolvedSymbol> {
        if call_name.contains("::") {
            return self.resolve_qualified_call(call_name);
        }

        if call_name.contains('.') {
            return self.resolve_method_call(call_name, context_module, context_crate);
        }

        // First, check imports for this file
        if let Some(imported) = self.resolve_from_imports(call_name, context_file) {
            return Some(imported);
        }

        self.resolve_simple_call(call_name, context_module, context_crate)
    }

    fn resolve_from_imports(&self, call_name: &str, file_path: &str) -> Option<ResolvedSymbol> {
        if let Some(imports) = self.import_table.get(file_path) {
            for imported in imports {
                if imported.local_name == call_name {
                    // Found a match! Now resolve the qualified name
                    if let Some(symbol) = self.symbol_table.get(&imported.qualified_name) {
                        return Some(symbol.clone());
                    }
                    
                    // If not found in symbol table, create a resolved symbol from import info
                    return Some(ResolvedSymbol {
                        qualified_name: imported.qualified_name.clone(),
                        crate_name: imported.crate_name.clone(),
                        symbol_type: SymbolType::Function, // Assume function for now
                    });
                }
            }
        }
        None
    }

    fn resolve_qualified_call(&self, call_name: &str) -> Option<ResolvedSymbol> {
        // First try direct lookup
        if let Some(symbol) = self.symbol_table.get(call_name) {
            return Some(symbol.clone());
        }

        if call_name.starts_with("crate::") {
            let local_name = call_name.strip_prefix("crate::").unwrap();
            return self.symbol_table.get(local_name).cloned();
        }

        if call_name.starts_with("super::") {
            return None;
        }

        if call_name.contains("::") {
            let parts: Vec<&str> = call_name.split("::").collect();
            if parts.len() >= 2 {
                // Direct lookup first
                if let Some(symbol) = self.symbol_table.get(call_name) {
                    return Some(symbol.clone());
                }
                
                // Try pattern matching for potential crate::module::Type::method patterns
                for (key, symbol) in &self.symbol_table {
                    if key.ends_with(&format!("::{}", call_name)) {
                        return Some(symbol.clone());
                    }
                }
                
                let potential_crate = parts[0];
                let rest = parts[1..].join("::");
                let full_name = format!("{}::{}", potential_crate, rest);
                return self.symbol_table.get(&full_name).cloned();
            }
        }

        None
    }

    fn resolve_method_call(&self, call_name: &str, context_module: &str, context_crate: &str) -> Option<ResolvedSymbol> {
        let parts: Vec<&str> = call_name.split('.').collect();
        if parts.len() != 2 {
            return None;
        }

        let _object = parts[0];
        let method = parts[1];

        let candidates = vec![
            format!("{}::{}", context_module, method),
            format!("{}::{}", context_crate, method),
            method.to_string(),
        ];

        for candidate in candidates {
            if let Some(symbol) = self.symbol_table.get(&candidate) {
                return Some(symbol.clone());
            }
        }

        None
    }

    fn resolve_simple_call(&self, call_name: &str, context_module: &str, context_crate: &str) -> Option<ResolvedSymbol> {
        let candidates = vec![
            format!("{}::{}", context_module, call_name),
            format!("{}::{}", context_crate, call_name),
            call_name.to_string(),
        ];

        for candidate in candidates {
            if let Some(symbol) = self.symbol_table.get(&candidate) {
                return Some(symbol.clone());
            }
        }

        None
    }

    pub fn resolve_type_reference(&self, type_name: &str, context_module: &str, context_crate: &str, context_file: &str) -> Option<ResolvedSymbol> {
        self.resolve_call(type_name, context_module, context_crate, context_file)
    }

    pub fn get_symbol_by_qualified_name(&self, qualified_name: &str) -> Option<&ResolvedSymbol> {
        self.symbol_table.get(qualified_name)
    }

    pub fn get_all_symbols(&self) -> &HashMap<String, ResolvedSymbol> {
        &self.symbol_table
    }

    pub fn get_functions_in_crate(&self, crate_name: &str) -> Vec<&ResolvedSymbol> {
        self.symbol_table.values()
            .filter(|symbol| {
                symbol.crate_name == crate_name && matches!(symbol.symbol_type, SymbolType::Function)
            })
            .collect()
    }

    pub fn get_types_in_crate(&self, crate_name: &str) -> Vec<&ResolvedSymbol> {
        self.symbol_table.values()
            .filter(|symbol| {
                symbol.crate_name == crate_name && matches!(symbol.symbol_type, SymbolType::Type)
            })
            .collect()
    }
    
    /// Generate synthetic trait method calls for all trait implementations
    /// This creates call edges for trait methods that can be called through dynamic dispatch
    pub fn generate_trait_method_calls(&self, symbols: &ParsedSymbols) -> Result<Vec<FunctionCall>> {
        let mut synthetic_calls = Vec::new();
        
        
        let mut trait_impls = 0;
        let mut simple_impls = 0;
        
        for impl_block in &symbols.impls {
            // Only process trait implementations (impl Trait for Type)
            if let Some(ref trait_name) = impl_block.trait_name {
                trait_impls += 1;
                
                // Generate synthetic calls for known trait methods, even if we don't have the methods parsed
                let trait_methods = self.get_known_trait_methods(trait_name);
                
                for method_name in trait_methods {
                    // Create a synthetic call indicating this trait method can be called
                    // We'll create a generic caller that represents "framework/runtime"
                    let caller_id = format!("trait_caller::{}::{}", trait_name, method_name);
                    
                    // Build the qualified callee name for this method using full module path
                    // Need to determine the actual qualified name of the function from the symbol table
                    let mut qualified_callee = None;
                    
                    // Search for the actual function in the symbol table
                    for (symbol_name, symbol) in &self.symbol_table {
                        // Look for method names that end with the method name
                        if symbol_name.ends_with(method_name) {
                            // Try to match based on file path patterns
                            // If the impl_block file path contains pattern and the symbol contains similar pattern
                            let impl_file_stem = if let Some(file_name) = impl_block.file_path.split('/').last() {
                                file_name.replace(".rs", "")
                            } else {
                                continue;
                            };
                            
                            if symbol_name.contains(&impl_file_stem) {
                                qualified_callee = Some(symbol_name.clone());
                                break;
                            }
                        }
                    }
                    
                    // If not found, use a fallback pattern
                    if qualified_callee.is_none() {
                        let fallback = format!("{}::{}", impl_block.type_name, method_name);
                        qualified_callee = Some(fallback);
                    }
                    
                    let synthetic_call = FunctionCall {
                        caller_id,
                        caller_module: "trait_framework".to_string(),
                        callee_name: method_name.to_string(),
                        qualified_callee: qualified_callee.clone(),
                        call_type: CallType::Method,
                        line: impl_block.line_start,
                        cross_crate: false,
                        from_crate: "trait_framework".to_string(),
                        to_crate: None, // Will be resolved later
                        file_path: "<trait_synthetic>".to_string(),
                        is_synthetic: true,
                        macro_context: Some(MacroContext {
                            expansion_id: format!("trait_method::{}::{}", trait_name, method_name),
                            macro_type: "trait_method".to_string(),
                            expansion_site_line: impl_block.line_start,
                        }),
                        synthetic_confidence: 0.8, // Lower confidence since it's potential dispatch
                    };
                    
                    synthetic_calls.push(synthetic_call);
                }
            } else {
                simple_impls += 1;
            }
        }
        
        
        Ok(synthetic_calls)
    }
    
    /// Get known methods for common traits to generate synthetic calls
    fn get_known_trait_methods(&self, trait_name: &str) -> Vec<&'static str> {
        match trait_name {
            "WebSocketActor" => vec!["handle_message", "event_stream", "connect", "disconnect", "subscribe", "is_connected"],
            "Actor" => vec!["on_start", "on_stop"],
            "Handler" => vec!["handle"],
            "StreamHandler" => vec!["handle", "started", "finished"],
            "Default" => vec!["default"],
            "Clone" => vec!["clone"],
            "Debug" => vec!["fmt"],
            "Display" => vec!["fmt"],
            "PartialEq" => vec!["eq"],
            "Eq" => vec![],
            "PartialOrd" => vec!["partial_cmp"],
            "Ord" => vec!["cmp"],
            "Hash" => vec!["hash"],
            "From" => vec!["from"],
            "Into" => vec!["into"],
            "TryFrom" => vec!["try_from"],
            "TryInto" => vec!["try_into"],
            "AsRef" => vec!["as_ref"],
            "AsMut" => vec!["as_mut"],
            "Deref" => vec!["deref"],
            "DerefMut" => vec!["deref_mut"],
            "Iterator" => vec!["next"],
            "IntoIterator" => vec!["into_iter"],
            "Serialize" => vec!["serialize"],
            "Deserialize" => vec!["deserialize"],
            _ => vec![], // For unknown traits, don't generate synthetic calls
        }
    }
}

pub fn resolve_all_references(symbols: &mut ParsedSymbols) -> Result<()> {
    let mut resolver = ReferenceResolver::new();
    resolver.build_symbol_table(symbols)?;

    for call in &mut symbols.calls {
        if let Some(resolved) = resolver.resolve_call(
            &call.callee_name,
            &call.caller_module,
            &call.from_crate,
            &call.file_path,
        ) {
            call.qualified_callee = Some(resolved.qualified_name.clone());
            call.to_crate = Some(resolved.crate_name.clone());
            call.cross_crate = call.from_crate != resolved.crate_name;
        }
    }
    
    // Note: Trait method call generation is now handled by framework patterns in WorkspaceAnalyzer

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_symbols() -> ParsedSymbols {
        let mut symbols = ParsedSymbols::new();
        
        let mut func = RustFunction {
            id: "test_crate:crate::module::test_fn:10".to_string(),
            name: "test_fn".to_string(),
            qualified_name: "crate::module::test_fn".to_string(),
            crate_name: "test_crate".to_string(),
            module_path: "crate::module".to_string(),
            file_path: "src/module.rs".to_string(),
            line_start: 10,
            line_end: 15,
            visibility: "pub".to_string(),
            is_async: false,
            is_unsafe: false,
            is_generic: false,
            is_test: false,
            is_trait_impl: false,
            doc_comment: None,
            signature: "pub fn test_fn()".to_string(),
            parameters: Vec::new(),
            return_type: None,
            embedding_text: None,
            module: "crate::module".to_string(),
        };
        func.generate_id();
        symbols.functions.push(func);

        symbols
    }

    #[test]
    fn test_build_symbol_table() {
        let symbols = create_test_symbols();
        let mut resolver = ReferenceResolver::new();
        
        resolver.build_symbol_table(&symbols).unwrap();
        
        assert!(resolver.get_symbol_by_qualified_name("crate::module::test_fn").is_some());
        assert!(resolver.get_symbol_by_qualified_name("test_fn").is_some());
    }

    #[test]
    fn test_resolve_qualified_call() {
        let symbols = create_test_symbols();
        let mut resolver = ReferenceResolver::new();
        resolver.build_symbol_table(&symbols).unwrap();
        
        let resolved = resolver.resolve_call("crate::module::test_fn", "crate", "test_crate", "src/test.rs");
        assert!(resolved.is_some());
        
        let symbol = resolved.unwrap();
        assert_eq!(symbol.qualified_name, "crate::module::test_fn");
        assert_eq!(symbol.crate_name, "test_crate");
    }

    #[test]
    fn test_resolve_simple_call() {
        let symbols = create_test_symbols();
        let mut resolver = ReferenceResolver::new();
        resolver.build_symbol_table(&symbols).unwrap();
        
        let resolved = resolver.resolve_call("test_fn", "crate::module", "test_crate", "src/test.rs");
        assert!(resolved.is_some());
        
        let symbol = resolved.unwrap();
        assert_eq!(symbol.qualified_name, "crate::module::test_fn");
    }

    #[test]
    fn test_resolve_all_references() {
        let mut symbols = create_test_symbols();
        
        let call = FunctionCall {
            caller_id: "caller_id".to_string(),
            caller_module: "crate::module".to_string(),
            callee_name: "test_fn".to_string(),
            qualified_callee: None,
            call_type: CallType::Direct,
            line: 20,
            cross_crate: false,
            from_crate: "test_crate".to_string(),
            to_crate: None,
            file_path: "src/test.rs".to_string(),
            is_synthetic: false,
            macro_context: None,
            synthetic_confidence: 1.0,
        };
        symbols.calls.push(call);

        resolve_all_references(&mut symbols).unwrap();

        assert_eq!(symbols.calls.len(), 1);
        let resolved_call = &symbols.calls[0];
        assert!(resolved_call.qualified_callee.is_some());
        assert_eq!(resolved_call.qualified_callee.as_ref().unwrap(), "crate::module::test_fn");
        assert!(!resolved_call.cross_crate);
    }
}