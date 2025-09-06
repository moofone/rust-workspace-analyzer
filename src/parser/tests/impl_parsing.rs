use crate::parser::RustParser;
use std::path::Path;

/// Test impl block parsing including trait implementations
#[test]
pub fn test_impl_parsing() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    // Test case from trading-ta patterns
    let source = r#"
use std::ops::{Add, Sub};

pub struct Price(f64);

// Regular impl block
impl Price {
    pub fn new(value: f64) -> Self {
        Price(value)
    }
    
    pub fn value(&self) -> f64 {
        self.0
    }
}

// Trait implementation
impl Add for Price {
    type Output = Price;
    
    fn add(self, other: Price) -> Self::Output {
        Price(self.0 + other.0)
    }
}

// Generic trait implementation
impl<T> From<T> for Price 
where 
    T: Into<f64>
{
    fn from(value: T) -> Self {
        Price(value.into())
    }
}

// Trait implementation for primitive type (like in Pf64)
impl Add<Price> for f64 {
    type Output = Price;
    
    fn add(self, other: Price) -> Price {
        Price(self + other.0)
    }
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Debug output
    println!("Found {} functions", result.functions.len());
    for func in &result.functions {
        println!("  Function: {} (qualified: {}, is_trait_impl: {})", 
                 func.name, func.qualified_name, func.is_trait_impl);
    }
    
    // Check regular methods
    let new_method = result.functions.iter()
        .find(|f| f.name == "new")
        .expect("Should find new method");
    
    // Actually, methods in regular impl blocks should NOT be marked as trait_impl
    // Only methods in "impl Trait for Type" should be marked as trait_impl
    assert!(!new_method.is_trait_impl, "new() in regular impl block should not be trait_impl");
    assert_eq!(new_method.return_type, Some("Self".to_string()));
    
    // Check the value method too
    let value_method = result.functions.iter()
        .find(|f| f.name == "value")
        .expect("Should find value method");
    assert!(!value_method.is_trait_impl, "value() in regular impl block should not be trait_impl");
    
    // Check Add trait implementation
    let add_method = result.functions.iter()
        .find(|f| f.name == "add" && f.qualified_name.contains("Price"))
        .expect("Should find add method for Price");
    
    assert!(add_method.is_trait_impl);
    
    // Check From trait implementation
    let from_method = result.functions.iter()
        .find(|f| f.name == "from")
        .expect("Should find from method");
    
    assert!(from_method.is_trait_impl);
    
    // Check trait impl for primitive type
    let primitive_add = result.functions.iter()
        .find(|f| f.name == "add" && f.qualified_name.contains("f64"))
        .expect("Should find add implementation for f64");
    
    assert!(primitive_add.is_trait_impl);
}

/// Test async trait implementations
#[test]
fn test_async_impl_parsing() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    // Test case from trading-exchanges patterns
    let source = r#"
use async_trait::async_trait;

pub struct BinanceClient {
    api_key: String,
}

#[async_trait]
impl Exchange for BinanceClient {
    async fn connect(&mut self) -> Result<(), Error> {
        // Implementation
        Ok(())
    }
    
    async fn get_orderbook(&self, symbol: &str) -> Result<OrderBook, Error> {
        // Implementation
        unimplemented!()
    }
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Check async trait methods
    let connect_method = result.functions.iter()
        .find(|f| f.name == "connect")
        .expect("Should find connect method");
    
    assert!(connect_method.is_async);
    assert!(connect_method.is_trait_impl);
    
    let orderbook_method = result.functions.iter()
        .find(|f| f.name == "get_orderbook")
        .expect("Should find get_orderbook method");
    
    assert!(orderbook_method.is_async);
    assert!(orderbook_method.is_trait_impl);
}

/// Test impl block with generics and where clauses
#[test]
fn test_generic_impl_parsing() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    let source = r#"
pub struct Container<T> {
    data: Vec<T>,
}

impl<T> Container<T> 
where 
    T: Clone + Send
{
    pub fn new() -> Self {
        Container { data: Vec::new() }
    }
    
    pub fn add(&mut self, item: T) {
        self.data.push(item);
    }
}

impl<T: Default> Default for Container<T> {
    fn default() -> Self {
        Container { data: vec![T::default()] }
    }
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Debug output
    println!("Found {} functions", result.functions.len());
    for func in &result.functions {
        println!("  Function: {} (is_trait_impl: {})", func.name, func.is_trait_impl);
    }
    
    // Check regular impl methods (NOT trait implementations)
    let new_method = result.functions.iter()
        .find(|f| f.name == "new")
        .expect("Should find new method");
    assert!(!new_method.is_trait_impl, "new() in regular impl block should not be trait_impl");
    
    let add_method = result.functions.iter()
        .find(|f| f.name == "add")
        .expect("Should find add method");
    assert!(!add_method.is_trait_impl, "add() in regular impl block should not be trait_impl");
    
    // Check Default trait implementation
    let default_method = result.functions.iter()
        .find(|f| f.name == "default")
        .expect("Should find default method");
    
    assert!(default_method.is_trait_impl, "default() in Default trait impl should be trait_impl");
}