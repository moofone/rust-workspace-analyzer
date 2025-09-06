use crate::parser::symbols::{ParsedSymbols, RustFunction, RustImpl, RustType};
use std::collections::HashMap;

/// Index structure for efficient trait and type method resolution
#[derive(Debug, Clone)]
pub struct TraitIndex {
    /// Maps trait names to methods they define
    pub trait_methods: HashMap<String, Vec<String>>,
    
    /// Maps (type_name, trait_name) to implementations
    pub trait_implementations: HashMap<(String, String), Vec<RustFunction>>,
    
    /// Maps type names to their inherent methods (non-trait methods)
    pub type_inherent_methods: HashMap<String, Vec<RustFunction>>,
    
    /// Maps type names to all traits they implement
    pub type_to_traits: HashMap<String, Vec<String>>,
    
    /// Maps fully qualified names to actual function definitions
    pub function_registry: HashMap<String, RustFunction>,
    
    /// Maps type names to their definitions
    pub type_registry: HashMap<String, RustType>,
}

impl TraitIndex {
    pub fn new() -> Self {
        Self {
            trait_methods: HashMap::new(),
            trait_implementations: HashMap::new(),
            type_inherent_methods: HashMap::new(),
            type_to_traits: HashMap::new(),
            function_registry: HashMap::new(),
            type_registry: HashMap::new(),
        }
    }

    /// Build the complete trait index from parsed symbols
    pub fn from_symbols(symbols: &ParsedSymbols) -> Self {
        let mut index = Self::new();
        
        // Build type registry
        for rust_type in &symbols.types {
            index.type_registry.insert(rust_type.name.clone(), rust_type.clone());
            index.type_registry.insert(rust_type.qualified_name.clone(), rust_type.clone());
        }
        
        // Build function registry
        for function in &symbols.functions {
            index.function_registry.insert(function.qualified_name.clone(), function.clone());
            index.function_registry.insert(function.name.clone(), function.clone());
        }
        
        
        // Process implementations
        for impl_block in &symbols.impls {
            // Process impl block
            
            if let Some(trait_name) = &impl_block.trait_name {
                // This is a trait implementation
                let key = (impl_block.type_name.clone(), trait_name.clone());
                index.trait_implementations.insert(key, impl_block.methods.clone());
                
                // Update type_to_traits mapping
                index.type_to_traits.entry(impl_block.type_name.clone())
                    .or_insert_with(Vec::new)
                    .push(trait_name.clone());
                
                // Update trait_methods registry
                for method in &impl_block.methods {
                    index.trait_methods.entry(trait_name.clone())
                        .or_insert_with(Vec::new)
                        .push(method.name.clone());
                }
            } else {
                // This is an inherent implementation
                index.type_inherent_methods.insert(impl_block.type_name.clone(), impl_block.methods.clone());
            }
        }
        
        index
    }
    
    /// Resolve a Type::method call to the actual function
    pub fn resolve_type_method(&self, type_name: &str, method_name: &str) -> Option<&RustFunction> {
        // First, try inherent methods (they have higher priority)
        if let Some(inherent_methods) = self.type_inherent_methods.get(type_name) {
            for method in inherent_methods {
                if method.name == method_name {
                    return Some(method);
                }
            }
        }
        
        // Then, try trait implementations
        if let Some(traits) = self.type_to_traits.get(type_name) {
            for trait_name in traits {
                let key = (type_name.to_string(), trait_name.clone());
                if let Some(trait_methods) = self.trait_implementations.get(&key) {
                    for method in trait_methods {
                        if method.name == method_name {
                            return Some(method);
                        }
                    }
                }
            }
        }
        
        None
    }
    
    /// Resolve a UFCS call <Type as Trait>::method to the actual function
    pub fn resolve_ufcs_call(&self, type_name: &str, trait_name: &str, method_name: &str) -> Option<&RustFunction> {
        let key = (type_name.to_string(), trait_name.to_string());
        if let Some(trait_methods) = self.trait_implementations.get(&key) {
            for method in trait_methods {
                if method.name == method_name {
                    return Some(method);
                }
            }
        }
        None
    }
    
    /// Get all methods available for a type (both inherent and trait methods)
    pub fn get_type_methods(&self, type_name: &str) -> Vec<&RustFunction> {
        let mut methods = Vec::new();
        
        // Add inherent methods
        if let Some(inherent_methods) = self.type_inherent_methods.get(type_name) {
            methods.extend(inherent_methods.iter());
        }
        
        // Add trait methods
        if let Some(traits) = self.type_to_traits.get(type_name) {
            for trait_name in traits {
                let key = (type_name.to_string(), trait_name.clone());
                if let Some(trait_methods) = self.trait_implementations.get(&key) {
                    methods.extend(trait_methods.iter());
                }
            }
        }
        
        methods
    }
    
    /// Check if a type implements a specific trait
    pub fn type_implements_trait(&self, type_name: &str, trait_name: &str) -> bool {
        if let Some(traits) = self.type_to_traits.get(type_name) {
            traits.contains(&trait_name.to_string())
        } else {
            false
        }
    }
    
    /// Get the type definition for a given type name
    pub fn get_type(&self, type_name: &str) -> Option<&RustType> {
        self.type_registry.get(type_name)
    }
    
    /// Resolve a qualified function name to the actual function
    pub fn resolve_function(&self, qualified_name: &str) -> Option<&RustFunction> {
        self.function_registry.get(qualified_name)
    }
}

impl Default for TraitIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::symbols::{ParsedSymbols, RustFunction, RustImpl, RustType, TypeKind};
    
    fn create_test_function(name: &str, module: &str) -> RustFunction {
        let qualified_name = format!("{}::{}", module, name);
        let mut func = RustFunction {
            id: format!("test_crate:{}:1", qualified_name),
            name: name.to_string(),
            qualified_name: qualified_name.clone(),
            crate_name: "test_crate".to_string(),
            module_path: module.to_string(),
            file_path: "test.rs".to_string(),
            line_start: 1,
            line_end: 5,
            visibility: "pub".to_string(),
            is_async: false,
            is_unsafe: false,
            is_generic: false,
            is_test: false,
            is_trait_impl: false,
            is_method: false,
            function_context: crate::parser::symbols::FunctionContext::Free,
            doc_comment: None,
            signature: format!("pub fn {}()", name),
            parameters: Vec::new(),
            return_type: None,
            embedding_text: None,
            module: module.to_string(),
        };
        func.generate_id();
        func
    }
    
    #[test]
    fn test_trait_index_creation() {
        let index = TraitIndex::new();
        assert!(index.type_inherent_methods.is_empty());
        assert!(index.trait_implementations.is_empty());
        assert!(index.type_to_traits.is_empty());
    }
    
    #[test]
    fn test_inherent_method_resolution() {
        let mut symbols = ParsedSymbols::new();
        
        // Create a type with inherent methods
        let new_method = create_test_function("new", "test::MyActor");
        let start_method = create_test_function("start", "test::MyActor");
        
        let impl_block = RustImpl {
            type_name: "MyActor".to_string(),
            trait_name: None,
            methods: vec![new_method.clone(), start_method.clone()],
            file_path: "test.rs".to_string(),
            line_start: 10,
            line_end: 20,
            is_generic: false,
        };
        
        symbols.impls.push(impl_block);
        symbols.functions.push(new_method.clone());
        symbols.functions.push(start_method.clone());
        
        let index = TraitIndex::from_symbols(&symbols);
        
        // Test resolving inherent methods
        let resolved = index.resolve_type_method("MyActor", "new");
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().name, "new");
        
        let resolved = index.resolve_type_method("MyActor", "start");
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().name, "start");
        
        let resolved = index.resolve_type_method("MyActor", "nonexistent");
        assert!(resolved.is_none());
    }
    
    #[test]
    fn test_trait_implementation_resolution() {
        let mut symbols = ParsedSymbols::new();
        
        // Create a type that implements a trait
        let on_start = create_test_function("on_start", "test::MyActor");
        let on_stop = create_test_function("on_stop", "test::MyActor");
        
        let impl_block = RustImpl {
            type_name: "MyActor".to_string(),
            trait_name: Some("Actor".to_string()),
            methods: vec![on_start.clone(), on_stop.clone()],
            file_path: "test.rs".to_string(),
            line_start: 30,
            line_end: 40,
            is_generic: false,
        };
        
        symbols.impls.push(impl_block);
        symbols.functions.push(on_start.clone());
        symbols.functions.push(on_stop.clone());
        
        let index = TraitIndex::from_symbols(&symbols);
        
        // Test that the type is marked as implementing the trait
        let traits = index.type_to_traits.get("MyActor");
        assert!(traits.is_some());
        assert!(traits.unwrap().contains(&"Actor".to_string()));
        assert!(!traits.unwrap().contains(&"WebSocketActor".to_string()));
        
        // Test resolving trait methods
        let resolved = index.resolve_type_method("MyActor", "on_start");
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().name, "on_start");
        
        let resolved = index.resolve_type_method("MyActor", "on_stop");
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().name, "on_stop");
    }
    
    #[test]
    fn test_ufcs_resolution() {
        let mut symbols = ParsedSymbols::new();
        
        // Create a type with trait implementation
        let handle = create_test_function("handle", "test::MyHandler");
        
        let impl_block = RustImpl {
            type_name: "MyHandler".to_string(),
            trait_name: Some("Handler".to_string()),
            methods: vec![handle.clone()],
            file_path: "test.rs".to_string(),
            line_start: 50,
            line_end: 60,
            is_generic: false,
        };
        
        symbols.impls.push(impl_block);
        symbols.functions.push(handle.clone());
        
        let index = TraitIndex::from_symbols(&symbols);
        
        // Test UFCS resolution
        let resolved = index.resolve_ufcs_call("MyHandler", "Handler", "handle");
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().name, "handle");
        
        // Test with wrong trait
        let resolved = index.resolve_ufcs_call("MyHandler", "Actor", "handle");
        assert!(resolved.is_none());
        
        // Test with wrong type
        let resolved = index.resolve_ufcs_call("WrongType", "Handler", "handle");
        assert!(resolved.is_none());
    }
    
    #[test]
    fn test_mixed_inherent_and_trait_methods() {
        let mut symbols = ParsedSymbols::new();
        
        // Create a type with both inherent and trait methods
        let new_method = create_test_function("new", "test::ComplexActor");
        let custom_method = create_test_function("custom", "test::ComplexActor");
        
        let inherent_impl = RustImpl {
            type_name: "ComplexActor".to_string(),
            trait_name: None,
            methods: vec![new_method.clone(), custom_method.clone()],
            file_path: "test.rs".to_string(),
            line_start: 70,
            line_end: 80,
            is_generic: false,
        };
        
        let on_start = create_test_function("on_start", "test::ComplexActor");
        
        let trait_impl = RustImpl {
            type_name: "ComplexActor".to_string(),
            trait_name: Some("Actor".to_string()),
            methods: vec![on_start.clone()],
            file_path: "test.rs".to_string(),
            line_start: 90,
            line_end: 100,
            is_generic: false,
        };
        
        symbols.impls.push(inherent_impl);
        symbols.impls.push(trait_impl);
        symbols.functions.extend(vec![new_method, custom_method, on_start]);
        
        let index = TraitIndex::from_symbols(&symbols);
        
        // Test that both inherent and trait methods are resolved
        assert!(index.resolve_type_method("ComplexActor", "new").is_some());
        assert!(index.resolve_type_method("ComplexActor", "custom").is_some());
        assert!(index.resolve_type_method("ComplexActor", "on_start").is_some());
        
        // Test that the type implements the trait
        let traits = index.type_to_traits.get("ComplexActor");
        assert!(traits.is_some());
        assert!(traits.unwrap().contains(&"Actor".to_string()));
    }
    
    #[test]
    fn test_get_type_and_function_registry() {
        let mut symbols = ParsedSymbols::new();
        
        // Add a type
        let rust_type = RustType {
            id: "test_crate:test::TestType:1".to_string(),
            name: "TestType".to_string(),
            qualified_name: "test::TestType".to_string(),
            crate_name: "test_crate".to_string(),
            module_path: "test".to_string(),
            file_path: "test.rs".to_string(),
            line_start: 1,
            line_end: 5,
            kind: TypeKind::Struct,
            visibility: "pub".to_string(),
            is_generic: false,
            is_test: false,
            doc_comment: None,
            fields: Vec::new(),
            variants: Vec::new(),
            methods: Vec::new(),
            embedding_text: None,
            type_kind: "struct".to_string(),
            module: "test".to_string(),
        };
        symbols.types.push(rust_type);
        
        // Add a function
        let function = create_test_function("test_fn", "test");
        symbols.functions.push(function.clone());
        
        let index = TraitIndex::from_symbols(&symbols);
        
        // Test type registry
        assert!(index.get_type("TestType").is_some());
        assert!(index.get_type("test::TestType").is_some());
        assert_eq!(index.get_type("TestType").unwrap().name, "TestType");
        
        // Test function registry
        assert!(index.resolve_function("test::test_fn").is_some());
        assert!(index.resolve_function("test_fn").is_some());
        assert_eq!(index.resolve_function("test_fn").unwrap().name, "test_fn");
    }
}

#[derive(Debug, Clone)]
pub struct CallResolution {
    pub target_function: RustFunction,
    pub resolution_type: ResolutionType,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub enum ResolutionType {
    /// Exact match - we know definitively this is the target
    Exact,
    /// Inherent method - resolved through inherent impl
    Inherent,
    /// Trait method - resolved through trait impl
    Trait { trait_name: String },
    /// UFCS - resolved through explicit trait qualification
    UFCS { trait_name: String },
}

impl CallResolution {
    pub fn exact(function: RustFunction) -> Self {
        Self {
            target_function: function,
            resolution_type: ResolutionType::Exact,
            confidence: 1.0,
        }
    }
    
    pub fn inherent(function: RustFunction) -> Self {
        Self {
            target_function: function,
            resolution_type: ResolutionType::Inherent,
            confidence: 1.0,
        }
    }
    
    pub fn trait_method(function: RustFunction, trait_name: String) -> Self {
        Self {
            target_function: function,
            resolution_type: ResolutionType::Trait { trait_name },
            confidence: 1.0,
        }
    }
    
    pub fn ufcs(function: RustFunction, trait_name: String) -> Self {
        Self {
            target_function: function,
            resolution_type: ResolutionType::UFCS { trait_name },
            confidence: 1.0,
        }
    }
}