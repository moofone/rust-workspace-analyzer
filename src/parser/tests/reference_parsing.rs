use crate::parser::RustParser;
use std::path::Path;

/// Test reference resolution between functions and types
#[test]
pub fn test_reference_parsing() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    // Test case with various reference patterns
    let source = r#"
pub struct Order {
    pub symbol: String,
    pub price: f64,
}

impl Order {
    pub fn new(symbol: String, price: f64) -> Self {
        // Constructor references the struct
        Self { symbol, price }
    }
    
    pub fn update(&mut self, new_price: f64) {
        self.price = new_price;
    }
}

pub fn process_order(order: &Order) -> f64 {
    // Function references Order type
    order.price * 1.01
}

pub fn create_and_process() {
    // References Order::new
    let order = Order::new("BTC/USDT".to_string(), 50000.0);
    
    // References process_order function
    let processed_price = process_order(&order);
    
    // Method call reference
    let mut mutable_order = order;
    mutable_order.update(51000.0);
}

// Generic function referencing trait bounds
pub fn compare_orders<T: PartialOrd>(a: &T, b: &T) -> bool {
    a < b
}

// Type alias reference
type Price = f64;

pub fn calculate_total(prices: Vec<Price>) -> Price {
    prices.iter().sum()
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Verify type references in function signatures
    let process_order_fn = result.functions.iter()
        .find(|f| f.name == "process_order")
        .expect("Should find process_order function");
    
    // Check that Order type is referenced in parameters
    assert!(process_order_fn.parameters.iter()
            .any(|p| p.param_type.contains("Order")),
            "process_order should reference Order type");
    
    // Debug: see what calls were found
    println!("Found {} calls total", result.calls.len());
    for call in &result.calls {
        println!("  Call: {} (qualified: {:?})", call.callee_name, call.qualified_callee);
    }
    
    // Verify function calls create references
    // Order::new is a Type::method pattern - qualified_callee will be None
    assert!(result.calls.iter().any(|c| c.callee_name == "new"),
        "Should detect Order::new call");
    
    assert!(result.calls.iter().any(|c| 
        c.callee_name == "process_order"),
        "Should detect process_order call");
    
    assert!(result.calls.iter().any(|c| 
        c.callee_name == "update"),
        "Should detect update method call");
}

/// Test cross-module references
#[test]
fn test_cross_module_references() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    let source = r#"
mod indicators {
    pub struct RSI {
        period: usize,
    }
    
    impl RSI {
        pub fn new(period: usize) -> Self {
            RSI { period }
        }
        
        pub fn calculate(&self, data: &[f64]) -> f64 {
            // Implementation
            0.0
        }
    }
}

mod strategy {
    use super::indicators::RSI;
    
    pub fn analyze_with_rsi(data: &[f64]) -> f64 {
        // Cross-module reference to RSI
        let rsi = RSI::new(14);
        rsi.calculate(data)
    }
}

pub use indicators::RSI;
pub use strategy::analyze_with_rsi;

pub fn main_analysis() {
    let data = vec![1.0, 2.0, 3.0];
    // References re-exported function
    let result = analyze_with_rsi(&data);
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Should detect cross-module calls - RSI is a type, so qualified_callee will be None
    assert!(result.calls.iter().any(|c| c.callee_name == "new"),
        "Should detect RSI::new cross-module call");
    
    assert!(result.calls.iter().any(|c|
        c.callee_name == "calculate"),
        "Should detect calculate method call");
    
    assert!(result.calls.iter().any(|c|
        c.callee_name == "analyze_with_rsi"),
        "Should detect analyze_with_rsi call");
}

/// Test trait implementation references
#[test]
fn test_trait_impl_references() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    let source = r#"
pub trait Indicator {
    fn compute(&self, data: &[f64]) -> f64;
}

pub struct SMA {
    period: usize,
}

// Trait implementation creates reference
impl Indicator for SMA {
    fn compute(&self, data: &[f64]) -> f64 {
        data.iter().sum::<f64>() / self.period as f64
    }
}

pub fn process_indicator<T: Indicator>(indicator: &T, data: &[f64]) -> f64 {
    // Dynamic dispatch through trait
    indicator.compute(data)
}

pub fn use_trait() {
    let sma = SMA { period: 20 };
    let data = vec![1.0, 2.0, 3.0];
    
    // Reference through trait bound
    let result = process_indicator(&sma, &data);
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Check trait implementation
    let compute_impl = result.functions.iter()
        .find(|f| f.name == "compute" && f.is_trait_impl)
        .expect("Should find compute trait implementation");
    
    assert!(compute_impl.is_trait_impl, "compute should be marked as trait implementation");
    
    // Check trait method call
    assert!(result.calls.iter().any(|c|
        c.callee_name == "compute"),
        "Should detect compute trait method call");
}

/// Test type references in complex scenarios
#[test]
fn test_complex_type_references() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    let source = r#"
use std::collections::HashMap;

pub type Cache = HashMap<String, Vec<f64>>;

pub struct DataProcessor {
    cache: Cache,
    processor: Box<dyn Fn(&[f64]) -> f64>,
}

impl DataProcessor {
    pub fn new<F>(processor: F) -> Self 
    where
        F: Fn(&[f64]) -> f64 + 'static,
    {
        Self {
            cache: HashMap::new(),
            processor: Box::new(processor),
        }
    }
    
    pub fn process(&mut self, key: String, data: Vec<f64>) -> f64 {
        let result = (self.processor)(&data);
        self.cache.insert(key, data);
        result
    }
}

pub fn create_processor() -> DataProcessor {
    DataProcessor::new(|data| data.iter().sum())
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Check type alias
    let cache_alias = result.types.iter()
        .find(|t| t.name == "Cache")
        .expect("Should find Cache type alias");
    
    assert_eq!(cache_alias.kind, crate::parser::symbols::TypeKind::TypeAlias);
    
    // Check that DataProcessor references Cache type
    let processor_type = result.types.iter()
        .find(|t| t.name == "DataProcessor")
        .expect("Should find DataProcessor struct");
    
    assert!(processor_type.fields.iter().any(|f| f.field_type.contains("Cache")),
            "DataProcessor should have Cache field");
    
    // Check HashMap::new call - HashMap is a type, so qualified_callee will be None  
    assert!(result.calls.iter().any(|c| c.callee_name == "new"),
        "Should detect HashMap::new call");
}