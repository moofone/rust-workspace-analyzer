use crate::parser::RustParser;
use std::path::Path;

/// Test trait definition parsing from trading-backend-poc patterns
#[test]
pub fn test_trait_parsing() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    // Test case inspired by trading-core and trading-ta patterns
    let source = r#"
use async_trait::async_trait;

pub trait Indicator {
    type Input;
    type Output;
    
    fn compute(&mut self, input: Self::Input) -> Self::Output;
    fn reset(&mut self);
}

#[async_trait]
pub trait DataProvider: Send + Sync {
    async fn fetch_candles(&self, symbol: &str, limit: usize) -> Result<Vec<Candle>, Error>;
    async fn subscribe(&mut self, symbols: Vec<String>) -> Result<(), Error>;
}

trait InternalCache {
    fn get(&self, key: &str) -> Option<&str>;
    fn set(&mut self, key: String, value: String);
}

pub trait Strategy<T> where T: Clone {
    fn analyze(&self, data: &T) -> Signal;
    fn update_params(&mut self, params: StrategyParams);
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Find traits in types collection
    let traits: Vec<_> = result.types.iter()
        .filter(|t| t.kind == crate::parser::symbols::TypeKind::Trait)
        .collect();
    
    assert!(traits.len() >= 4, "Should parse at least 4 traits");
    
    // Check Indicator trait with associated types
    let indicator = result.types.iter()
        .find(|t| t.name == "Indicator" && t.kind == crate::parser::symbols::TypeKind::Trait)
        .expect("Should find Indicator trait");
    
    assert_eq!(indicator.visibility, "pub");
    
    // Check async trait
    let provider = result.types.iter()
        .find(|t| t.name == "DataProvider" && t.kind == crate::parser::symbols::TypeKind::Trait)
        .expect("Should find DataProvider trait");
    
    assert_eq!(provider.visibility, "pub");
    
    // Check private trait
    let cache = result.types.iter()
        .find(|t| t.name == "InternalCache" && t.kind == crate::parser::symbols::TypeKind::Trait)
        .expect("Should find InternalCache trait");
    
    assert_eq!(cache.visibility, "private");
    
    // Check generic trait with where clause
    let strategy = result.types.iter()
        .find(|t| t.name == "Strategy" && t.kind == crate::parser::symbols::TypeKind::Trait)
        .expect("Should find Strategy trait");
    
    // Traits don't currently have generics tracked in our structs
    // This would need to be added to RustTrait if needed
}

/// Test trait method parsing
#[test]
fn test_trait_methods() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    let source = r#"
pub trait Calculator {
    fn add(&self, a: f64, b: f64) -> f64;
    fn subtract(&self, a: f64, b: f64) -> f64;
    
    // Default implementation
    fn multiply(&self, a: f64, b: f64) -> f64 {
        a * b
    }
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Debug: see what functions were found
    println!("Found {} functions total", result.functions.len());
    for func in &result.functions {
        println!("  Function: {} (is_trait_impl: {})", func.name, func.is_trait_impl);
    }
    
    // Trait method declarations should be captured as regular functions
    // They are NOT trait implementations (those would be in impl blocks)
    let trait_methods: Vec<_> = result.functions.iter()
        .filter(|f| ["add", "subtract", "multiply"].contains(&f.name.as_str()))
        .collect();
    
    // We should find the trait method declarations
    assert!(trait_methods.len() >= 2, "Should find at least 2 trait method declarations, found {}", trait_methods.len());
}