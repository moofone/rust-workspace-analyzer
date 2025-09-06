use crate::parser::RustParser;
use std::path::Path;

/// Test parsing of structs from trading-backend-poc patterns
#[test]
pub fn test_type_parsing() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    // Test case inspired by trading-core/src/common/types.rs
    let source = r#"
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Clone)]
pub struct OrderBook {
    pub bids: Vec<(f64, f64)>,
    pub asks: Vec<(f64, f64)>,
    symbol: String,
    last_update: Option<i64>,
}

pub struct TradingEngine<T: Clone> {
    data: Vec<T>,
    processor: Box<dyn Fn(&T) -> f64>,
}

// Tuple struct
pub struct TimestampMS(pub i64);

// Unit struct
pub struct MarketClosed;
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Verify all struct types were parsed
    assert!(result.types.len() >= 5, "Should parse at least 5 structs");
    
    // Check Candle struct
    let candle = result.types.iter()
        .find(|t| t.name == "Candle")
        .expect("Should find Candle struct");
    
    assert_eq!(candle.kind, crate::parser::symbols::TypeKind::Struct);
    assert_eq!(candle.visibility, "pub");
    println!("Parsed {} types", result.types.len());
    println!("Candle struct has {} fields", candle.fields.len());
    for field in &candle.fields {
        println!("  Field: {} : {}", field.name, field.field_type);
    }
    assert!(candle.fields.len() >= 6, "Candle should have 6 fields but found {}", candle.fields.len());
    
    // Verify all fields are public
    for field in &candle.fields {
        assert_eq!(field.visibility, "pub", "All Candle fields should be public");
    }
    
    // Check OrderBook with mixed visibility
    let orderbook = result.types.iter()
        .find(|t| t.name == "OrderBook")
        .expect("Should find OrderBook struct");
    
    assert_eq!(orderbook.kind, crate::parser::symbols::TypeKind::Struct);
    assert!(orderbook.fields.iter().any(|f| f.visibility == "pub"));
    assert!(orderbook.fields.iter().any(|f| f.visibility != "pub"));
    
    // Check generic struct
    let engine = result.types.iter()
        .find(|t| t.name == "TradingEngine")
        .expect("Should find TradingEngine struct");
    
    // RustType doesn't currently have generics field, only is_generic
    assert!(engine.is_generic, "Generic struct should be marked as generic");
    
    // Check tuple struct
    let timestamp = result.types.iter()
        .find(|t| t.name == "TimestampMS")
        .expect("Should find TimestampMS tuple struct");
    
    assert_eq!(timestamp.kind, crate::parser::symbols::TypeKind::Struct);
    
    // Check unit struct  
    let closed = result.types.iter()
        .find(|t| t.name == "MarketClosed")
        .expect("Should find MarketClosed unit struct");
    
    assert_eq!(closed.kind, crate::parser::symbols::TypeKind::Struct);
}

/// Test enum parsing
#[test]
fn test_enum_parsing() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    // Test case from trading-strategy patterns
    let source = r#"
#[derive(Debug, Clone, PartialEq)]
pub enum OrderType {
    Market,
    Limit { price: f64 },
    StopLoss { trigger: f64, limit: Option<f64> },
}

pub enum TradingSignal {
    Buy(f64),
    Sell(f64),
    Hold,
}

enum InternalState {
    Ready,
    Processing { id: u64 },
    Error(String),
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    assert!(result.types.len() >= 3, "Should parse at least 3 enums");
    
    // Check OrderType enum
    let order_type = result.types.iter()
        .find(|t| t.name == "OrderType")
        .expect("Should find OrderType enum");
    
    assert_eq!(order_type.kind, crate::parser::symbols::TypeKind::Enum);
    assert_eq!(order_type.visibility, "pub");
    
    // Check TradingSignal with tuple variants
    let signal = result.types.iter()
        .find(|t| t.name == "TradingSignal")
        .expect("Should find TradingSignal enum");
    
    assert_eq!(signal.kind, crate::parser::symbols::TypeKind::Enum);
    
    // Check private enum
    let internal = result.types.iter()
        .find(|t| t.name == "InternalState")
        .expect("Should find InternalState enum");
    
    assert_eq!(internal.visibility, "private");
}

/// Test type alias parsing
#[test]
fn test_type_alias_parsing() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    // Test case from trading-core patterns
    let source = r#"
pub type Price = f64;
pub type Volume = f64;
pub type Timestamp = i64;

type InternalCache = std::collections::HashMap<String, Vec<f64>>;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub type OrderCallback = Box<dyn Fn(&Order) -> bool + Send + Sync>;
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Check Price type alias
    let price = result.types.iter()
        .find(|t| t.name == "Price")
        .expect("Should find Price type alias");
    
    assert_eq!(price.kind, crate::parser::symbols::TypeKind::TypeAlias);
    assert_eq!(price.visibility, "pub");
    
    // Check generic type alias
    let result_alias = result.types.iter()
        .find(|t| t.name == "Result")
        .expect("Should find Result type alias");
    
    // Type aliases can have generics but field not tracked
    // assert!(result_alias.generics.is_some());
    
    // Check complex type alias with trait bounds
    let callback = result.types.iter()
        .find(|t| t.name == "OrderCallback")
        .expect("Should find OrderCallback type alias");
    
    assert_eq!(callback.kind, crate::parser::symbols::TypeKind::TypeAlias);
}