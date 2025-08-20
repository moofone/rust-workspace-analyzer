# IMPROVE_1: Cross-Crate Function Reference Counting Implementation

## Problem Statement
The current dependency analysis tracks module-level imports but fails to count actual function call references across crates, resulting in inaccurate "heavily used but untested" metrics showing all zeros.

## Implementation Tasks

### Task 1: Data Structure Updates
**File**: `src/analyzer/workspace.rs`

#### 1.1 Add FunctionReference struct
```rust
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
```

#### 1.2 Add FunctionRegistry struct
```rust
#[derive(Debug, Clone)]
pub struct FunctionRegistry {
    pub functions_by_name: HashMap<String, Vec<String>>,          // name -> qualified_names
    pub functions_by_qualified: HashMap<String, RustFunction>,    // qualified_name -> function
    pub public_functions: HashSet<String>,                        // qualified names of pub functions
}
```

#### 1.3 Update WorkspaceSnapshot
```rust
pub struct WorkspaceSnapshot {
    pub functions: Vec<RustFunction>,
    pub types: Vec<RustType>,
    pub dependencies: Vec<Dependency>,
    pub function_references: Vec<FunctionReference>,  // NEW
    pub function_registry: FunctionRegistry,          // NEW
    pub timestamp: std::time::SystemTime,
}
```

### Task 2: Function Registry Implementation
**File**: `src/analyzer/workspace.rs`

#### 2.1 Add registry building method
```rust
impl WorkspaceAnalyzer {
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
}
```

#### 2.2 Implement function lookup
```rust
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
```

### Task 3: Enhanced Call Detection
**File**: `src/analyzer/workspace.rs`

#### 3.1 Update traverse_for_dependencies method
```rust
fn traverse_for_dependencies(&self, cursor: &mut tree_sitter::TreeCursor, file_path: &Path, content: &str, dependencies: &mut Vec<Dependency>, function_references: &mut Vec<FunctionReference>, current_function: &str, file_imports: &[String]) {
    loop {
        let node = cursor.node();
        
        match node.kind() {
            "use_declaration" => {
                if let Some(dep) = self.parse_use_dependency(node, file_path, content) {
                    dependencies.push(dep);
                }
            }
            "call_expression" => {
                if let Some(func_ref) = self.parse_function_call(node, file_path, content, current_function, file_imports) {
                    function_references.push(func_ref);
                }
            }
            "method_call_expression" => {  // NEW
                if let Some(func_ref) = self.parse_method_call(node, file_path, content, current_function, file_imports) {
                    function_references.push(func_ref);
                }
            }
            _ => {}
        }
        
        if cursor.goto_first_child() {
            self.traverse_for_dependencies(cursor, file_path, content, dependencies, function_references, current_function, file_imports);
            cursor.goto_parent();
        }
        
        if !cursor.goto_next_sibling() {
            break;
        }
    }
}
```

#### 3.2 Implement function call parsing
```rust
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
        
        // Get the file module for cross-crate detection
        let file_module = self.get_file_module_path(file_path);
        let is_test_context = file_path.to_string_lossy().contains("/tests/") || 
                              file_path.to_string_lossy().contains("test_") ||
                              current_function.contains("test");
        
        Some(FunctionReference {
            target_function: call_text.to_string(), // Will be resolved later
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
```

#### 3.3 Implement method call parsing
```rust
fn parse_method_call(&self, node: tree_sitter::Node, file_path: &Path, content: &str, current_function: &str, file_imports: &[String]) -> Option<FunctionReference> {
    if let Some(method_node) = node.child_by_field_name("method") {
        let method_name = &content[method_node.start_byte()..method_node.end_byte()];
        
        let file_module = self.get_file_module_path(file_path);
        let is_test_context = file_path.to_string_lossy().contains("/tests/") || 
                              current_function.contains("test");
        
        Some(FunctionReference {
            target_function: method_name.to_string(), // Will be resolved later
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
```

### Task 4: Import Tracking
**File**: `src/analyzer/workspace.rs`

#### 4.1 Add import extraction per file
```rust
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
    // Handle various formats: use a::b::c, use a::b::{x, y}, use a::b::*
    // For now, simplified version:
    if let Some(path_part) = use_text.strip_prefix("use ").and_then(|s| s.strip_suffix(";")) {
        // Remove braces and wildcards for now - can be enhanced
        let cleaned = path_part.trim().replace("{", "").replace("}", "").replace("*", "");
        Some(cleaned)
    } else {
        None
    }
}
```

### Task 5: Two-Pass Analysis
**File**: `src/analyzer/workspace.rs`

#### 5.1 Update analyze_workspace method
```rust
pub fn analyze_workspace(&mut self) -> Result<WorkspaceSnapshot> {
    let mut functions = Vec::new();
    let mut types = Vec::new();
    let mut dependencies = Vec::new();
    let mut function_references = Vec::new();
    
    // [Existing file discovery and parsing code...]
    
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
```

#### 5.2 Add reference resolution
```rust
fn resolve_function_reference(&self, func_ref: &FunctionReference, registry: &FunctionRegistry) -> Option<FunctionReference> {
    // Extract current module from calling function
    let calling_module = if let Some(pos) = func_ref.calling_function.rfind("::") {
        &func_ref.calling_function[..pos]
    } else {
        ""
    };
    
    // Get file imports (would need to be passed or stored)
    let file_imports = vec![]; // TODO: Pass imports from analysis
    
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
```

### Task 6: Update Test Coverage Analysis
**File**: `src/mcp/server.rs`

#### 6.1 Modify handle_analyze_test_coverage
```rust
async fn handle_analyze_test_coverage(&self, request: McpRequest) -> McpResponse {
    let snapshot_guard = self.current_snapshot.read().await;
    let snapshot = match snapshot_guard.as_ref() {
        Some(snapshot) => snapshot,
        None => { /* error handling */ }
    };
    
    // Build reference count map using new data structure
    let mut function_refs = HashMap::new();
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
    
    // [Rest of analysis using accurate reference counts...]
}
```

### Task 7: Testing and Validation

#### 7.1 Create test cases
**File**: `tests/test_function_references.rs`
- Test simple function calls within same module
- Test cross-crate function calls
- Test method calls
- Test imported function calls
- Test reference counting accuracy

#### 7.2 Add debug logging
- Log function registry size
- Log number of references found
- Log cross-crate vs same-crate ratios
- Log resolution success rates

### Task 8: Integration Points

#### 8.1 Update extract_functions method
- Track current function context during traversal
- Pass current function to dependency extraction
- Extract imports per file during analysis

#### 8.2 Update file traversal
- Store per-file import lists
- Pass import context to call resolution
- Handle nested function contexts

## Expected Outcomes
After implementation:
1. Accurate cross-crate function reference counting
2. Proper identification of heavily-used untested functions
3. Meaningful `untested_heavy_usage` metrics in test coverage reports
4. Better prioritization of functions needing test coverage

## Validation Criteria
- Function reference counts > 0 for actually called functions
- Cross-crate references properly identified
- Test coverage tool shows realistic "heavily used" functions
- Reference resolution accuracy > 80%