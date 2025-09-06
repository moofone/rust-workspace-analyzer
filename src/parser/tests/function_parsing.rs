use crate::parser::RustParser;
use std::path::Path;

/// Test parsing of regular functions from trading-backend-poc
#[test]
pub fn test_function_parsing() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    // Test case from trading-ta/src/indicators/rsi.rs
    let source = r#"
pub fn calculate_rsi(prices: &[f64], period: usize) -> Vec<f64> {
    let mut rsi_values = Vec::new();
    
    if prices.len() < period {
        return rsi_values;
    }
    
    // Calculate price changes
    let mut gains = Vec::new();
    let mut losses = Vec::new();
    
    for i in 1..prices.len() {
        let change = prices[i] - prices[i - 1];
        if change > 0.0 {
            gains.push(change);
            losses.push(0.0);
        } else {
            gains.push(0.0);
            losses.push(change.abs());
        }
    }
    
    rsi_values
}

fn helper_function(value: f64) -> f64 {
    value * 2.0
}

pub(crate) fn internal_calc(data: &[f64]) -> Option<f64> {
    data.first().copied()
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Verify functions were parsed
    assert_eq!(result.functions.len(), 3, "Should parse 3 functions");
    
    // Check main function
    let calculate_rsi = result.functions.iter()
        .find(|f| f.name == "calculate_rsi")
        .expect("Should find calculate_rsi function");
    
    assert_eq!(calculate_rsi.visibility, "pub");
    assert_eq!(calculate_rsi.return_type, Some("Vec<f64>".to_string()));
    assert!(!calculate_rsi.is_async);
    assert!(!calculate_rsi.is_trait_impl);
    
    // Check parameters
    assert_eq!(calculate_rsi.parameters.len(), 2);
    assert!(calculate_rsi.parameters.iter().any(|p| p.name == "prices" && p.param_type == "&[f64]"));
    assert!(calculate_rsi.parameters.iter().any(|p| p.name == "period" && p.param_type == "usize"));
    
    // Check private function
    let helper = result.functions.iter()
        .find(|f| f.name == "helper_function")
        .expect("Should find helper_function");
    
    assert_eq!(helper.visibility, "private");
    assert_eq!(helper.return_type, Some("f64".to_string()));
    
    // Check crate-visible function
    let internal = result.functions.iter()
        .find(|f| f.name == "internal_calc")
        .expect("Should find internal_calc");
    
    assert_eq!(internal.visibility, "pub(crate)");
    assert_eq!(internal.return_type, Some("Option<f64>".to_string()));
}

/// Test method parsing from impl blocks
#[test]
fn test_method_parsing() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    // Test case from trading-core/src/types.rs
    let source = r#"
pub struct Order {
    pub id: String,
    pub price: f64,
}

impl Order {
    pub fn new(id: String, price: f64) -> Self {
        Self { id, price }
    }
    
    pub fn update_price(&mut self, new_price: f64) {
        self.price = new_price;
    }
    
    fn validate(&self) -> bool {
        self.price > 0.0
    }
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // All methods should be captured
    assert_eq!(result.functions.len(), 3, "Should parse 3 methods");
    
    // Check constructor
    let new_fn = result.functions.iter()
        .find(|f| f.name == "new")
        .expect("Should find new method");
    
    assert_eq!(new_fn.visibility, "pub");
    assert_eq!(new_fn.return_type, Some("Self".to_string()));
    assert!(!new_fn.is_trait_impl); // Regular impl methods are NOT trait implementations
    
    // Check mutable self method
    let update_fn = result.functions.iter()
        .find(|f| f.name == "update_price")
        .expect("Should find update_price method");
    
    assert!(update_fn.parameters.iter().any(|p| p.is_self && p.is_mutable));
    
    // Check private method
    let validate_fn = result.functions.iter()
        .find(|f| f.name == "validate")
        .expect("Should find validate method");
    
    assert_eq!(validate_fn.visibility, "private");
    assert!(validate_fn.parameters.iter().any(|p| p.is_self && !p.is_mutable));
}

/// Test generic function parsing
#[test]
fn test_generic_function_parsing() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    // Test case inspired by trading-core patterns
    let source = r#"
pub fn convert<T, U>(value: T) -> U 
where
    T: Into<U>,
{
    value.into()
}

pub fn process_data<'a, T: Clone>(data: &'a [T], count: usize) -> Vec<&'a T> {
    data.iter().take(count).collect()
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    assert_eq!(result.functions.len(), 2, "Should parse 2 generic functions");
    
    let convert_fn = result.functions.iter()
        .find(|f| f.name == "convert")
        .expect("Should find convert function");
    
    assert!(convert_fn.is_generic, "convert should be marked as generic");
    
    let process_fn = result.functions.iter()
        .find(|f| f.name == "process_data")
        .expect("Should find process_data function");
    
    assert!(process_fn.is_generic, "process should be marked as generic");
}