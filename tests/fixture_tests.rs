use workspace_analyzer::parser::rust_parser::RustParser;
use std::path::PathBuf;

// Import the fixture constants
include!("../src/parser/tests/fixtures/actor_patterns.rs");
include!("../src/parser/tests/fixtures/function_defs.rs");
include!("../src/parser/tests/fixtures/type_defs.rs");
include!("../src/parser/tests/fixtures/trait_impls.rs");
include!("../src/parser/tests/fixtures/call_patterns.rs");
include!("../src/parser/tests/fixtures/macro_patterns.rs");

// Actor tests
#[test]
fn test_simple_actor() {
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_source(SIMPLE_ACTOR, &PathBuf::from("test.rs"), "test_crate").unwrap();
    
    assert!(!result.actors.is_empty(), "Should find SimpleActor");
    assert_eq!(result.actors[0].name, "SimpleActor");
}

#[test]
fn test_distributed_actor() {
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_source(DISTRIBUTED_ACTOR, &PathBuf::from("test.rs"), "test_crate").unwrap();
    
    assert!(!result.actors.is_empty(), "Should find CalculatorActor");
    let actor = &result.actors[0];
    assert_eq!(actor.name, "CalculatorActor");
    assert!(actor.is_distributed, "Should be marked as distributed");
}

// Function tests
#[test]
fn test_async_function() {
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_source(ASYNC_FUNCTION, &PathBuf::from("test.rs"), "test_crate").unwrap();
    
    println!("Functions found: {}", result.functions.len());
    for func in &result.functions {
        println!("  - {} (async: {})", func.name, func.is_async);
    }
    
    let async_funcs: Vec<_> = result.functions.iter().filter(|f| f.is_async).collect();
    assert!(!async_funcs.is_empty(), "Should find async functions");
    assert!(async_funcs.iter().any(|f| f.name == "fetch_data"));
}

#[test]
fn test_generic_function() {
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_source(GENERIC_FUNCTION, &PathBuf::from("test.rs"), "test_crate").unwrap();
    
    let generic_funcs: Vec<_> = result.functions.iter().filter(|f| f.is_generic).collect();
    assert!(!generic_funcs.is_empty(), "Should find generic functions");
}

#[test]
fn test_unsafe_function() {
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_source(UNSAFE_FUNCTION, &PathBuf::from("test.rs"), "test_crate").unwrap();
    
    let unsafe_funcs: Vec<_> = result.functions.iter().filter(|f| f.is_unsafe).collect();
    assert!(!unsafe_funcs.is_empty(), "Should find unsafe functions");
}

// Type tests
#[test]
fn test_simple_struct() {
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_source(SIMPLE_STRUCT, &PathBuf::from("test.rs"), "test_crate").unwrap();
    
    assert!(!result.types.is_empty(), "Should find struct");
    assert!(result.types.iter().any(|t| t.name == "Point"));
}

#[test]
fn test_generic_struct() {
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_source(GENERIC_STRUCT, &PathBuf::from("test.rs"), "test_crate").unwrap();
    
    println!("Types found: {}", result.types.len());
    for typ in &result.types {
        println!("  - {} (generic: {})", typ.name, typ.is_generic);
    }
    
    assert!(!result.types.is_empty(), "Should find generic struct");
    let container = result.types.iter().find(|t| t.name == "Container");
    assert!(container.is_some(), "Should find Container struct");
    assert!(container.unwrap().is_generic, "Container should be generic");
}

#[test]
fn test_simple_enum() {
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_source(SIMPLE_ENUM, &PathBuf::from("test.rs"), "test_crate").unwrap();
    
    assert!(!result.types.is_empty(), "Should find enum");
    assert!(result.types.iter().any(|t| matches!(t.kind, workspace_analyzer::parser::symbols::TypeKind::Enum)));
}

// Trait tests
#[test]
fn test_simple_trait_impl() {
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_source(SIMPLE_TRAIT_IMPL, &PathBuf::from("test.rs"), "test_crate").unwrap();
    
    assert!(!result.impls.is_empty(), "Should find trait impl");
}

#[test]
fn test_generic_trait_impl() {
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_source(GENERIC_TRAIT_IMPL, &PathBuf::from("test.rs"), "test_crate").unwrap();
    
    assert!(!result.impls.is_empty(), "Should find generic trait impl");
    assert!(result.impls.iter().any(|i| i.is_generic));
}

#[test]
fn test_inherent_impl() {
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_source(INHERENT_IMPL, &PathBuf::from("test.rs"), "test_crate").unwrap();
    
    assert!(!result.impls.is_empty(), "Should find inherent impl");
    assert!(result.impls.iter().any(|i| i.trait_name.is_none()));
}

// Call tests
#[test]
fn test_simple_calls() {
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_source(SIMPLE_CALLS, &PathBuf::from("test.rs"), "test_crate").unwrap();
    
    assert!(!result.calls.is_empty(), "Should find simple function calls");
}

#[test]
fn test_method_calls() {
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_source(METHOD_CALLS, &PathBuf::from("test.rs"), "test_crate").unwrap();
    
    assert!(!result.calls.is_empty(), "Should find method calls");
    let method_calls: Vec<_> = result.calls.iter()
        .filter(|c| matches!(c.call_type, workspace_analyzer::parser::symbols::CallType::Method))
        .collect();
    assert!(!method_calls.is_empty(), "Should find method-style calls");
}

#[test]
fn test_ufcs_calls() {
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_source(UFCS_CALLS, &PathBuf::from("test.rs"), "test_crate").unwrap();
    
    assert!(!result.calls.is_empty(), "Should find UFCS calls");
    let ufcs_calls: Vec<_> = result.calls.iter()
        .filter(|c| c.callee_name.contains("::"))
        .collect();
    assert!(!ufcs_calls.is_empty(), "Should find UFCS-style calls");
}

// Macro tests
#[test]
fn test_paste_macro_simple() {
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_source(PASTE_MACRO_SIMPLE, &PathBuf::from("test.rs"), "test_crate").unwrap();
    
    assert!(!result.macro_expansions.is_empty(), "Should find paste! macro expansions");
}

#[test]
fn test_paste_macro_complex() {
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_source(PASTE_MACRO_COMPLEX, &PathBuf::from("test.rs"), "test_crate").unwrap();
    
    assert!(!result.macro_expansions.is_empty(), "Should find complex paste! patterns");
    let synthetic_calls: Vec<_> = result.calls.iter()
        .filter(|c| c.is_synthetic)
        .collect();
    assert!(!synthetic_calls.is_empty(), "Should generate synthetic calls from paste! macros");
}

#[test]
fn test_attribute_macros() {
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_source(ATTRIBUTE_MACROS, &PathBuf::from("test.rs"), "test_crate").unwrap();
    
    // Check for async-trait functions
    let async_trait_funcs: Vec<_> = result.functions.iter()
        .filter(|f| f.is_async && f.is_trait_impl)
        .collect();
    assert!(!async_trait_funcs.is_empty(), "Should find async trait methods");
}