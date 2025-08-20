use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use anyhow::Result;
use walkdir::WalkDir;
use tree_sitter::{Parser, Tree};
use cargo_metadata::{MetadataCommand, Package};
use tree_sitter_rust;

#[derive(Debug, Clone)]
pub struct RustFunction {
    pub name: String,
    pub qualified_name: String,
    pub file_path: PathBuf,
    pub line_start: usize,
    pub line_end: usize,
    pub module: String,
    pub visibility: String,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RustType {
    pub name: String,
    pub qualified_name: String,
    pub file_path: PathBuf,
    pub line_start: usize,
    pub line_end: usize,
    pub module: String,
    pub visibility: String,
    pub type_kind: String, // struct, enum, trait, type_alias
}

#[derive(Debug, Clone)]
pub struct Dependency {
    pub from_module: String,
    pub to_module: String,
    pub dependency_type: String, // use, call, field_access
    pub file_path: PathBuf,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct FunctionReference {
    pub target_function: String,     // qualified name of called function
    pub calling_function: String,    // qualified name of caller
    pub call_type: CallType,         // Direct, Method, Qualified, Import
    pub cross_crate: bool,           // true if calling across crate boundaries
    pub from_test: bool,             // true if call is from test code
    pub file_path: PathBuf,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub enum CallType {
    Direct,      // function_name()
    Method,      // obj.method_name()
    Qualified,   // crate::module::function_name()
    Import,      // use crate::func; func()
}

#[derive(Debug, Clone)]
pub struct FunctionRegistry {
    pub functions_by_name: HashMap<String, Vec<String>>,          // name -> qualified_names
    pub functions_by_qualified: HashMap<String, RustFunction>,    // qualified_name -> function
    pub public_functions: HashSet<String>,                        // qualified names of pub functions
}

pub struct WorkspaceAnalyzer {
    root_path: PathBuf,
    parser: Parser,
    packages: Vec<Package>,
}

impl FunctionRegistry {
    pub fn resolve_function_call(&self, call_name: &str, current_module: &str, imports: &[String]) -> Option<String> {
        // 1. Check if it's already qualified
        if call_name.contains("::") {
            if self.functions_by_qualified.contains_key(call_name) {
                return Some(call_name.to_string());
            }
        }
        
        // 2. Check imports first
        for import in imports {
            if import.ends_with(&format!("::{}", call_name)) {
                if self.functions_by_qualified.contains_key(import) {
                    return Some(import.clone());
                }
            }
        }
        
        // 3. Check same module
        let same_module_qualified = format!("{}::{}", current_module, call_name);
        if self.functions_by_qualified.contains_key(&same_module_qualified) {
            return Some(same_module_qualified);
        }
        
        // 4. Check all functions with this name
        if let Some(candidates) = self.functions_by_name.get(call_name) {
            if candidates.len() == 1 {
                return Some(candidates[0].clone());
            }
        }
        
        None
    }
}

impl WorkspaceAnalyzer {
    pub fn new(root_path: &Path) -> Result<Self> {
        let mut parser = Parser::new();
        let language = tree_sitter_rust::language();
        parser.set_language(&language)?;
        
        // Load Cargo workspace metadata
        let metadata = MetadataCommand::new()
            .manifest_path(root_path.join("Cargo.toml"))
            .exec()?;
        
        Ok(Self {
            root_path: root_path.to_path_buf(),
            parser,
            packages: metadata.packages,
        })
    }
    
    fn build_function_registry(&self, functions: &[RustFunction]) -> FunctionRegistry {
        let mut functions_by_name = HashMap::new();
        let mut functions_by_qualified = HashMap::new();
        let mut public_functions = HashSet::new();
        
        for func in functions {
            // Add to name index
            functions_by_name
                .entry(func.name.clone())
                .or_insert_with(Vec::new)
                .push(func.qualified_name.clone());
            
            // Add to qualified index
            functions_by_qualified.insert(func.qualified_name.clone(), func.clone());
            
            // Track public functions
            if func.visibility.contains("pub") {
                public_functions.insert(func.qualified_name.clone());
            }
        }
        
        FunctionRegistry {
            functions_by_name,
            functions_by_qualified,
            public_functions,
        }
    }

    pub fn analyze_workspace(&mut self) -> Result<WorkspaceSnapshot> {
        let mut functions = Vec::new();
        let mut types = Vec::new();
        let mut dependencies = Vec::new();
        let mut function_references = Vec::new();
        
        // Find Rust files in source directories, handling workspace structure
        let mut all_entries = Vec::new();
        
        // Check if it's a workspace by looking for workspace crates
        let workspace_crates = WalkDir::new(&self.root_path)
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_dir())
            .filter(|e| e.path().join("Cargo.toml").exists())
            .filter(|e| e.path() != self.root_path) // Exclude root
            .collect::<Vec<_>>();
        
        if !workspace_crates.is_empty() {
            println!("📦 Found workspace with {} crates", workspace_crates.len());
            // Scan workspace crates
            for crate_entry in workspace_crates {
                let crate_path = crate_entry.path();
                let source_dirs = ["src", "examples", "tests", "benches"];
                
                for dir in &source_dirs {
                    let dir_path = crate_path.join(dir);
                    if dir_path.exists() {
                        for entry in WalkDir::new(dir_path)
                            .into_iter()
                            .filter_map(|e| e.ok())
                            .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
                            // Skip large generated files or data files
                            .filter(|e| e.metadata().map_or(true, |m| m.len() < 1_000_000)) // Skip files > 1MB
                        {
                            all_entries.push(entry);
                        }
                    }
                }
            }
        } else {
            // Single crate structure
            let source_dirs = ["src", "examples", "tests", "benches"];
            for dir in &source_dirs {
                let dir_path = self.root_path.join(dir);
                if dir_path.exists() {
                    for entry in WalkDir::new(dir_path)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
                        .filter(|e| e.metadata().map_or(true, |m| m.len() < 1_000_000)) // Skip files > 1MB
                    {
                        all_entries.push(entry);
                    }
                }
            }
        }
        
        println!("📁 Found {} Rust files to analyze", all_entries.len());
        
        for entry in all_entries
        {
            let file_path = entry.path();
            match std::fs::read_to_string(file_path) {
                Ok(content) => {
                    // Skip very large files or files that look like data
                    if content.len() > 500_000 {
                        println!("⚠️  Skipping large file: {} ({} bytes)", file_path.display(), content.len());
                        continue;
                    }
                    
                    // Parse with tree-sitter
                    match self.parser.parse(&content, None) {
                        Some(tree) => {
                            // Extract functions, types, and dependencies
                            if let Err(e) = self.extract_functions(&tree, file_path, &content, &mut functions) {
                                println!("⚠️  Error extracting functions from {}: {}", file_path.display(), e);
                            }
                            if let Err(e) = self.extract_types(&tree, file_path, &content, &mut types) {
                                println!("⚠️  Error extracting types from {}: {}", file_path.display(), e);
                            }
                            if let Err(e) = self.extract_dependencies_and_references(&tree, file_path, &content, &mut dependencies, &mut function_references) {
                                println!("⚠️  Error extracting dependencies from {}: {}", file_path.display(), e);
                            }
                        }
                        None => {
                            println!("⚠️  Failed to parse: {}", file_path.display());
                        }
                    }
                }
                Err(e) => {
                    println!("⚠️  Cannot read file {}: {}", file_path.display(), e);
                }
            }
        }
        
        // PASS 1: Build function registry
        let function_registry = self.build_function_registry(&functions);
        
        // PASS 2: Resolve function references
        for func_ref in &mut function_references {
            if let Some(resolved) = self.resolve_function_reference(func_ref, &function_registry) {
                func_ref.target_function = resolved.target_function;
                func_ref.cross_crate = resolved.cross_crate;
            }
        }
        
        Ok(WorkspaceSnapshot {
            functions,
            types,
            dependencies,
            function_references,
            function_registry,
            timestamp: std::time::SystemTime::now(),
        })
    }
    
    fn extract_functions(&self, tree: &Tree, file_path: &Path, content: &str, functions: &mut Vec<RustFunction>) -> Result<()> {
        let root_node = tree.root_node();
        let mut cursor = root_node.walk();
        
        // Get the file-level module path from file structure
        let file_module = self.get_file_module_path(file_path);
        
        self.traverse_for_functions(&mut cursor, file_path, content, functions, &file_module);
        Ok(())
    }
    
    fn traverse_for_functions(&self, cursor: &mut tree_sitter::TreeCursor, file_path: &Path, content: &str, functions: &mut Vec<RustFunction>, module_path: &str) {
        loop {
            let node = cursor.node();
            
            if node.kind() == "function_item" {
                if let Some(function) = self.parse_function(node, file_path, content, module_path) {
                    functions.push(function);
                }
            } else if node.kind() == "mod_item" {
                // Handle nested modules
                if let Some(mod_name) = self.get_identifier_name(node, content) {
                    let new_module_path = if module_path.is_empty() {
                        mod_name
                    } else {
                        format!("{}::{}", module_path, mod_name)
                    };
                    
                    if cursor.goto_first_child() {
                        self.traverse_for_functions(cursor, file_path, content, functions, &new_module_path);
                        cursor.goto_parent();
                    }
                }
            }
            
            if cursor.goto_first_child() {
                self.traverse_for_functions(cursor, file_path, content, functions, module_path);
                cursor.goto_parent();
            }
            
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    
    fn parse_function(&self, node: tree_sitter::Node, file_path: &Path, content: &str, module_path: &str) -> Option<RustFunction> {
        let name = self.get_function_name(node, content)?;
        let visibility = self.get_visibility(node, content).unwrap_or_else(|| "private".to_string());
        let parameters = self.get_function_parameters(node, content);
        let return_type = self.get_function_return_type(node, content);
        
        let qualified_name = if module_path.is_empty() {
            name.clone()
        } else {
            format!("{}::{}", module_path, name)
        };
        
        Some(RustFunction {
            name,
            qualified_name,
            file_path: file_path.to_path_buf(),
            line_start: node.start_position().row + 1,
            line_end: node.end_position().row + 1,
            module: module_path.to_string(),
            visibility,
            parameters,
            return_type,
        })
    }
    
    fn extract_types(&self, tree: &Tree, file_path: &Path, content: &str, types: &mut Vec<RustType>) -> Result<()> {
        let root_node = tree.root_node();
        let mut cursor = root_node.walk();
        
        // Get the file-level module path from file structure
        let file_module = self.get_file_module_path(file_path);
        
        self.traverse_for_types(&mut cursor, file_path, content, types, &file_module);
        Ok(())
    }
    
    fn traverse_for_types(&self, cursor: &mut tree_sitter::TreeCursor, file_path: &Path, content: &str, types: &mut Vec<RustType>, module_path: &str) {
        loop {
            let node = cursor.node();
            
            match node.kind() {
                "struct_item" | "enum_item" | "trait_item" | "type_item" => {
                    if let Some(rust_type) = self.parse_type(node, file_path, content, module_path) {
                        types.push(rust_type);
                    }
                }
                "mod_item" => {
                    if let Some(mod_name) = self.get_identifier_name(node, content) {
                        let new_module_path = if module_path.is_empty() {
                            mod_name
                        } else {
                            format!("{}::{}", module_path, mod_name)
                        };
                        
                        if cursor.goto_first_child() {
                            self.traverse_for_types(cursor, file_path, content, types, &new_module_path);
                            cursor.goto_parent();
                        }
                    }
                }
                _ => {
                    if cursor.goto_first_child() {
                        self.traverse_for_types(cursor, file_path, content, types, module_path);
                        cursor.goto_parent();
                    }
                }
            }
            
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    
    fn parse_type(&self, node: tree_sitter::Node, file_path: &Path, content: &str, module_path: &str) -> Option<RustType> {
        let name = self.get_identifier_name(node, content)?;
        let visibility = self.get_visibility(node, content).unwrap_or_else(|| "private".to_string());
        let type_kind = match node.kind() {
            "struct_item" => "struct",
            "enum_item" => "enum", 
            "trait_item" => "trait",
            "type_item" => "type_alias",
            _ => "unknown"
        }.to_string();
        
        let qualified_name = if module_path.is_empty() {
            name.clone()
        } else {
            format!("{}::{}", module_path, name)
        };
        
        Some(RustType {
            name,
            qualified_name,
            file_path: file_path.to_path_buf(),
            line_start: node.start_position().row + 1,
            line_end: node.end_position().row + 1,
            module: module_path.to_string(),
            visibility,
            type_kind,
        })
    }
    
    fn extract_dependencies_and_references(&self, tree: &Tree, file_path: &Path, content: &str, dependencies: &mut Vec<Dependency>, function_references: &mut Vec<FunctionReference>) -> Result<()> {
        // Extract use statements and function calls for dependency analysis
        let root_node = tree.root_node();
        let mut cursor = root_node.walk();
        
        // Extract imports for this file
        let file_imports = self.extract_file_imports(tree, content);
        
        // Get file module for current function tracking
        let file_module = self.get_file_module_path(file_path);
        
        self.traverse_for_dependencies(&mut cursor, file_path, content, dependencies, function_references, &file_module, &file_imports);
        Ok(())
    }

    fn resolve_function_reference(&self, func_ref: &FunctionReference, registry: &FunctionRegistry) -> Option<FunctionReference> {
        // Extract current module from calling function
        let calling_module = if let Some(pos) = func_ref.calling_function.rfind("::") {
            &func_ref.calling_function[..pos]
        } else {
            ""
        };
        
        // Get file imports (simplified - in full implementation would store per-file)
        let file_imports = vec![];
        
        if let Some(resolved_target) = registry.resolve_function_call(&func_ref.target_function, calling_module, &file_imports) {
            let target_crate = self.extract_crate_name_from_qualified(&resolved_target);
            let calling_crate = self.extract_crate_name_from_qualified(&func_ref.calling_function);
            
            Some(FunctionReference {
                target_function: resolved_target,
                cross_crate: target_crate != calling_crate,
                ..func_ref.clone()
            })
        } else {
            None
        }
    }

    fn extract_crate_name_from_qualified(&self, qualified_name: &str) -> String {
        qualified_name.split("::").next().unwrap_or("unknown").to_string()
    }
    
    fn extract_file_imports(&self, tree: &Tree, content: &str) -> Vec<String> {
        let mut imports = Vec::new();
        let root_node = tree.root_node();
        let mut cursor = root_node.walk();
        
        self.traverse_for_imports(&mut cursor, content, &mut imports);
        imports
    }

    fn traverse_for_imports(&self, cursor: &mut tree_sitter::TreeCursor, content: &str, imports: &mut Vec<String>) {
        loop {
            let node = cursor.node();
            
            if node.kind() == "use_declaration" {
                let use_text = &content[node.start_byte()..node.end_byte()];
                if let Some(import) = self.parse_import_path(use_text) {
                    imports.push(import);
                }
            }
            
            if cursor.goto_first_child() {
                self.traverse_for_imports(cursor, content, imports);
                cursor.goto_parent();
            }
            
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    fn parse_import_path(&self, use_text: &str) -> Option<String> {
        // Extract the imported path from "use path::to::item;"
        if let Some(path_part) = use_text.strip_prefix("use ").and_then(|s| s.strip_suffix(";")) {
            // Simple cleaning - can be enhanced
            let cleaned = path_part.trim().replace(['{', '}', '*'], "");
            Some(cleaned)
        } else {
            None
        }
    }

    fn traverse_for_dependencies(&self, cursor: &mut tree_sitter::TreeCursor, file_path: &Path, content: &str, dependencies: &mut Vec<Dependency>, function_references: &mut Vec<FunctionReference>, current_module: &str, file_imports: &[String]) {
        // Track current function context for proper attribution
        let mut current_function = current_module.to_string();
        
        loop {
            let node = cursor.node();
            
            match node.kind() {
                "use_declaration" => {
                    if let Some(dep) = self.parse_use_dependency(node, file_path, content) {
                        dependencies.push(dep);
                    }
                }
                "call_expression" => {
                    if let Some(func_ref) = self.parse_function_call(node, file_path, content, &current_function, file_imports) {
                        function_references.push(func_ref);
                    }
                    if let Some(dep) = self.parse_call_dependency(node, file_path, content) {
                        dependencies.push(dep);
                    }
                }
                "method_call_expression" => {
                    if let Some(func_ref) = self.parse_method_call(node, file_path, content, &current_function, file_imports) {
                        function_references.push(func_ref);
                    }
                }
                "function_item" => {
                    // Update current function context
                    if let Some(func_name) = self.get_function_name(node, content) {
                        current_function = if current_module.is_empty() {
                            func_name
                        } else {
                            format!("{}::{}", current_module, func_name)
                        };
                    }
                }
                _ => {}
            }
            
            if cursor.goto_first_child() {
                self.traverse_for_dependencies(cursor, file_path, content, dependencies, function_references, current_module, file_imports);
                cursor.goto_parent();
            }
            
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    
    fn parse_use_dependency(&self, node: tree_sitter::Node, file_path: &Path, content: &str) -> Option<Dependency> {
        // Extract the module being imported
        let use_text = &content[node.start_byte()..node.end_byte()];
        
        // Simple parsing - could be more sophisticated
        if let Some(module) = use_text.split_whitespace().nth(1) {
            let module = module.trim_end_matches(';');
            
            Some(Dependency {
                from_module: self.get_file_module(file_path),
                to_module: module.to_string(),
                dependency_type: "use".to_string(),
                file_path: file_path.to_path_buf(),
                line: node.start_position().row + 1,
            })
        } else {
            None
        }
    }
    
    fn parse_call_dependency(&self, node: tree_sitter::Node, file_path: &Path, content: &str) -> Option<Dependency> {
        // Extract function call dependencies
        if let Some(function_node) = node.child_by_field_name("function") {
            let call_text = &content[function_node.start_byte()..function_node.end_byte()];
            
            // Check if it's a qualified call (contains ::)
            if call_text.contains("::") {
                let parts: Vec<&str> = call_text.rsplitn(2, "::").collect();
                if parts.len() == 2 {
                    return Some(Dependency {
                        from_module: self.get_file_module(file_path),
                        to_module: parts[1].to_string(),
                        dependency_type: "call".to_string(),
                        file_path: file_path.to_path_buf(),
                        line: node.start_position().row + 1,
                    });
                }
            }
        }
        None
    }

    fn parse_function_call(&self, node: tree_sitter::Node, file_path: &Path, content: &str, current_function: &str, file_imports: &[String]) -> Option<FunctionReference> {
        if let Some(function_node) = node.child_by_field_name("function") {
            let call_text = &content[function_node.start_byte()..function_node.end_byte()];
            
            let call_type = if call_text.contains("::") {
                CallType::Qualified
            } else if file_imports.iter().any(|imp| imp.ends_with(&format!("::{}", call_text))) {
                CallType::Import
            } else {
                CallType::Direct
            };
            
            let is_test_context = file_path.to_string_lossy().contains("/tests/") || 
                                  file_path.to_string_lossy().contains("test_") ||
                                  current_function.contains("test");
            
            Some(FunctionReference {
                target_function: call_text.to_string(),
                calling_function: current_function.to_string(),
                call_type,
                cross_crate: false, // Will be determined after resolution
                from_test: is_test_context,
                file_path: file_path.to_path_buf(),
                line: node.start_position().row + 1,
            })
        } else {
            None
        }
    }

    fn parse_method_call(&self, node: tree_sitter::Node, file_path: &Path, content: &str, current_function: &str, _file_imports: &[String]) -> Option<FunctionReference> {
        if let Some(method_node) = node.child_by_field_name("method") {
            let method_name = &content[method_node.start_byte()..method_node.end_byte()];
            
            let is_test_context = file_path.to_string_lossy().contains("/tests/") || 
                                  current_function.contains("test");
            
            Some(FunctionReference {
                target_function: method_name.to_string(),
                calling_function: current_function.to_string(),
                call_type: CallType::Method,
                cross_crate: false, // Will be determined after resolution
                from_test: is_test_context,
                file_path: file_path.to_path_buf(),
                line: node.start_position().row + 1,
            })
        } else {
            None
        }
    }
    
    // Helper methods
    fn get_function_name(&self, node: tree_sitter::Node, content: &str) -> Option<String> {
        node.child_by_field_name("name")
            .map(|name_node| content[name_node.start_byte()..name_node.end_byte()].to_string())
    }
    
    fn get_identifier_name(&self, node: tree_sitter::Node, content: &str) -> Option<String> {
        node.child_by_field_name("name")
            .map(|name_node| content[name_node.start_byte()..name_node.end_byte()].to_string())
    }
    
    fn get_visibility(&self, node: tree_sitter::Node, content: &str) -> Option<String> {
        // Look for visibility modifiers
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "visibility_modifier" {
                    return Some(content[child.start_byte()..child.end_byte()].to_string());
                }
            }
        }
        None
    }
    
    fn get_function_parameters(&self, node: tree_sitter::Node, content: &str) -> Vec<String> {
        if let Some(params_node) = node.child_by_field_name("parameters") {
            let params_text = &content[params_node.start_byte()..params_node.end_byte()];
            // Simple parameter extraction - could be more sophisticated
            params_text.trim_start_matches('(').trim_end_matches(')')
                .split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect()
        } else {
            Vec::new()
        }
    }
    
    fn get_function_return_type(&self, node: tree_sitter::Node, content: &str) -> Option<String> {
        node.child_by_field_name("return_type")
            .map(|return_node| content[return_node.start_byte()..return_node.end_byte()].to_string())
    }
    
    fn get_file_module(&self, file_path: &Path) -> String {
        file_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string()
    }
    
    fn get_file_module_path(&self, file_path: &Path) -> String {
        // Extract crate name and module path from file structure
        let mut components = Vec::new();
        
        // Find the crate name (directory containing Cargo.toml)
        let mut current_path = file_path;
        let mut crate_name = None;
        
        while let Some(parent) = current_path.parent() {
            if parent.join("Cargo.toml").exists() {
                crate_name = parent.file_name().and_then(|n| n.to_str()).map(|s| s.to_string());
                break;
            }
            current_path = parent;
        }
        
        if let Some(crate_name) = crate_name {
            components.push(crate_name);
            
            // Get the relative path from the crate root to this file
            if let Some(crate_root) = current_path.parent() {
                if let Ok(relative_path) = file_path.strip_prefix(crate_root) {
                    // Skip src/ and get module path
                    let path_components: Vec<&str> = relative_path
                        .components()
                        .filter_map(|c| c.as_os_str().to_str())
                        .filter(|&s| s != "src" && s != "examples" && s != "tests" && s != "benches")
                        .collect();
                    
                    // Convert file path to module path
                    for (i, component) in path_components.iter().enumerate() {
                        if i == path_components.len() - 1 {
                            // Last component is the file - use stem unless it's mod.rs or lib.rs
                            if *component == "mod.rs" || *component == "lib.rs" || *component == "main.rs" {
                                // Don't add the filename for these special files
                                continue;
                            } else {
                                // Remove .rs extension
                                if let Some(stem) = Path::new(component).file_stem().and_then(|s| s.to_str()) {
                                    components.push(stem.to_string());
                                }
                            }
                        } else {
                            // Directory component
                            components.push(component.to_string());
                        }
                    }
                }
            }
        }
        
        if components.is_empty() {
            "unknown".to_string()
        } else {
            components.join("::")
        }
    }
}

#[derive(Debug)]
pub struct WorkspaceSnapshot {
    pub functions: Vec<RustFunction>,
    pub types: Vec<RustType>,
    pub dependencies: Vec<Dependency>,
    pub function_references: Vec<FunctionReference>,
    pub function_registry: FunctionRegistry,
    pub timestamp: std::time::SystemTime,
}