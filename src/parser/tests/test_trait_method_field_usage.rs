use crate::parser::RustParser;

#[test]
fn test_trait_method_accessing_struct_fields() {
    let code = r#"
/// Taker flow tracking for buy/sell volume
#[derive(Debug, Clone, Default)]
pub struct TakerFlow {
    pub taker_buy_volume: Option<f64>,
    pub taker_sell_volume: Option<f64>,
    pub taker_buy_base_volume: Option<f64>,
    pub taker_sell_base_volume: Option<f64>,
}

impl TakerFlow {
    pub fn value(&self) -> (Option<f64>, Option<f64>) {
        (self.taker_buy_volume, self.taker_sell_volume)
    }
    
    pub fn update_trade(&mut self, _price: f64, size: f64, is_buyer_maker: bool) {
        if !is_buyer_maker {
            self.taker_buy_volume = Some(self.taker_buy_volume.unwrap_or(0.0) + size);
        } else {
            self.taker_sell_volume = Some(self.taker_sell_volume.unwrap_or(0.0) + size);
        }
    }
}

/// Futures OHLCV candle with taker flow data
pub struct FuturesOHLCVCandle {
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
    taker_flow: TakerFlow,
}

impl FuturesOHLCVCandle {
    // Getters for taker volumes from TakerFlow
    pub fn taker_buy_volume(&self) -> Option<f64> {
        self.taker_flow.taker_buy_volume
    }
    
    pub fn taker_sell_volume(&self) -> Option<f64> {
        self.taker_flow.taker_sell_volume
    }
    
    pub fn taker_buy_base_volume(&self) -> Option<f64> {
        self.taker_flow.taker_buy_base_volume
    }
    
    pub fn taker_sell_base_volume(&self) -> Option<f64> {
        self.taker_flow.taker_sell_base_volume
    }
    
    pub fn set_taker_buy_volume(&mut self, value: f64) {
        self.taker_flow.taker_buy_volume = Some(value);
    }
    
    pub fn set_taker_sell_volume(&mut self, value: f64) {
        self.taker_flow.taker_sell_volume = Some(value);
    }
}

/// Trait for candles that include taker buy/sell volume data
pub trait TakerFlowCandle {
    fn taker_buy_volume(&self) -> Option<f64>;
    fn taker_sell_volume(&self) -> Option<f64>;
    fn taker_buy_base_volume(&self) -> Option<f64> {
        None
    }
    fn taker_sell_base_volume(&self) -> Option<f64> {
        None  
    }
}

// Implementation via inherent methods (not trait impl)
// This is the pattern used in trading-backend-poc
"#;

    let mut parser = RustParser::new().expect("Failed to create parser");
    let result = parser
        .parse_source(code, std::path::Path::new("test_trait_methods.rs"), "test_crate")
        .expect("parse failed");

    // Find TakerFlow struct and verify fields exist
    let taker_flow = result
        .types
        .iter()
        .find(|t| t.name == "TakerFlow")
        .expect("TakerFlow struct not found");

    let field_names: std::collections::HashSet<_> = taker_flow.fields.iter().map(|f| f.name.as_str()).collect();
    for expected in [
        "taker_buy_volume",
        "taker_sell_volume",
        "taker_buy_base_volume",
        "taker_sell_base_volume",
    ] {
        assert!(field_names.contains(expected), "missing field: {}", expected);
    }
}

#[test]
fn test_nested_field_access_through_methods() {
    let code = r#"
pub struct Inner {
    pub value: i32,
}

pub struct Outer {
    inner: Inner,
}

impl Outer {
    pub fn get_value(&self) -> i32 {
        self.inner.value
    }
    
    pub fn set_value(&mut self, val: i32) {
        self.inner.value = val;
    }
}
"#;

    let mut parser = RustParser::new().expect("Failed to create parser");
    let result = parser
        .parse_source(code, std::path::Path::new("test_nested.rs"), "test_crate")
        .expect("parse failed");

    // Find Inner struct and ensure field is present
    let inner = result
        .types
        .iter()
        .find(|t| t.name == "Inner")
        .expect("Inner struct not found");
    assert!(inner.fields.iter().any(|f| f.name == "value"));
}
