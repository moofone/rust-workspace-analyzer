// Parser regression tests - ensures we don't break existing functionality
// Run with: cargo test --test parser_regression

use workspace_analyzer::parser::RustParser;
use std::path::Path;

#[test]
fn parser_regression_test_suite() {
    println!("\n=== Running Parser Regression Test Suite ===\n");
    
    // Run individual test categories
    test_basic_function_parsing();
    test_complex_type_hierarchies();
    test_trait_implementations(); 
    test_async_patterns();
    test_actor_message_patterns();
    test_macro_expansions();
    test_operator_overloading();
    
    println!("\n✅ All parser regression tests passed!\n");
}

fn test_basic_function_parsing() {
    println!("Testing basic function parsing...");
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    let source = r#"
pub fn calculate_price(base: f64, multiplier: f64) -> f64 {
    base * multiplier
}

async fn fetch_data(url: &str) -> Result<String, Error> {
    Ok(String::new())
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).expect("Failed to parse source");

    assert_eq!(result.functions.len(), 2, "Should parse 2 functions");
    assert!(result.functions.iter().any(|f| f.is_async), "Should detect async function");
}

fn test_complex_type_hierarchies() {
    println!("Testing complex type hierarchies...");
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    let source = r#"
pub struct Order<T> where T: Clone {
    id: String,
    data: T,
}

pub enum OrderStatus {
    Pending,
    Filled { price: f64, quantity: f64 },
    Cancelled(String),
}

pub type OrderId = String;
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).expect("Failed to parse source");

    assert!(result.types.len() >= 3, "Should parse struct, enum, and type alias");
}

fn test_trait_implementations() {
    println!("Testing trait implementations...");
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    // Critical test for operator overloading on primitive types
    let source = r#"
use std::ops::Add;

pub struct Price(f64);

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
    ).expect("Failed to parse source");

    let add_impl = result.functions.iter()
        .find(|f| f.name == "add")
        .expect("Should find add implementation");
    
    assert!(add_impl.is_trait_impl, "Operator should be marked as trait implementation");
}

fn test_async_patterns() {
    println!("Testing async patterns...");
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    let source = r#"
use async_trait::async_trait;

#[async_trait]
pub trait Exchange {
    async fn connect(&mut self) -> Result<(), Error>;
}

pub struct BinanceClient;

#[async_trait]
impl Exchange for BinanceClient {
    async fn connect(&mut self) -> Result<(), Error> {
        Ok(())
    }
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).expect("Failed to parse source");

    let connect = result.functions.iter()
        .find(|f| f.name == "connect" && f.is_trait_impl)
        .expect("Should find trait implementation");
    
    assert!(connect.is_async, "Trait method should be async");
}

fn test_actor_message_patterns() {
    println!("Testing actor and message patterns...");
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    let source = r#"
use actix::prelude::*;

pub struct TradingActor;

impl Actor for TradingActor {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct UpdateMessage {
    value: f64,
}

impl Handler<UpdateMessage> for TradingActor {
    type Result = ();
    
    fn handle(&mut self, msg: UpdateMessage, ctx: &mut Context<Self>) {
        // Handle message
    }
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).expect("Failed to parse source");

    assert!(!result.actors.is_empty(), "Should detect actor");
    assert!(!result.message_types.is_empty(), "Should detect message type");
    assert!(!result.message_handlers.is_empty(), "Should detect message handler");
}

fn test_macro_expansions() {
    println!("Testing macro expansion detection...");
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    let source = r#"
define_indicator_enums!(
    Rsi: "Relative Strength Index",
    Macd: "MACD",
    Sma: "Simple Moving Average"
);

distributed_actor! {
    pub struct DataActor {
        data: Vec<f64>,
    }
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).expect("Failed to parse source");

    assert!(!result.distributed_actors.is_empty(), "Should detect distributed actor");
}

fn test_operator_overloading() {
    println!("Testing operator overloading...");
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    let source = r#"
use std::ops::{Add, Sub, Mul, Div};

#[derive(Clone, Copy)]
pub struct Pf64(pub f64);

impl Add for Pf64 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Pf64(self.0 + other.0)
    }
}

impl Sub for Pf64 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Pf64(self.0 - other.0)
    }
}

impl Mul<Pf64> for f64 {
    type Output = Pf64;
    fn mul(self, other: Pf64) -> Pf64 {
        Pf64(self * other.0)
    }
}

impl Div<Pf64> for f64 {
    type Output = Pf64;
    fn div(self, other: Pf64) -> Pf64 {
        Pf64(self / other.0)
    }
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).expect("Failed to parse source");

    // All operator implementations should be marked as trait implementations
    let operators: Vec<_> = result.functions.iter()
        .filter(|f| ["add", "sub", "mul", "div"].contains(&f.name.as_str()))
        .collect();
    
    assert!(operators.len() >= 4, "Should find all operator implementations");
    
    for op in &operators {
        assert!(op.is_trait_impl, 
                "Operator {} should be marked as trait implementation", op.name);
    }
    
    // Specifically check f64 implementations
    let f64_ops: Vec<_> = operators.iter()
        .filter(|f| f.qualified_name.contains("f64"))
        .collect();
    
    assert!(!f64_ops.is_empty(), "Should find f64 operator implementations");
}