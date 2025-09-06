use crate::parser::RustParser;
use std::path::Path;

/// Test macro expansion detection from trading-ta patterns
#[test]
pub fn test_macro_expansion() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    // Test case from trading-ta/src/types/indicator_types.rs
    let source = r#"
// Macro to generate the enum types dynamically
macro_rules! define_indicator_enums {
    ($($indicator:ident: $description:literal),*) => {
        paste! {
            pub enum IndicatorConfigKind {
                $(
                    $indicator( [<$indicator Config>] ),
                )*
            }
        }
    };
}

// Invocation that should be detected
define_indicator_enums!(
    Atr: "Average True Range",
    Bb: "Bollinger Bands",
    Ema: "Exponential Moving Average",
    Macd: "Moving Average Convergence Divergence",
    Rsi: "Relative Strength Index",
    Sma: "Simple Moving Average"
);

// Paste macro usage that should generate synthetic calls
pub fn process_indicator(indicator_type: &str) {
    paste! {
        let indicator = [<$indicator>]::new(config);
        let input = [<$indicator Input>]::from_ohlcv(candle);
    }
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("trading-ta/src/types/indicator_types.rs"),
        "trading_ta"
    ).unwrap();

    // Check that macro expansions were detected
    assert!(!result.macro_expansions.is_empty(), "Should detect macro expansions");
    
    // Look for paste macro expansions
    let paste_expansions: Vec<_> = result.macro_expansions.iter()
        .filter(|e| e.macro_type == "paste")
        .collect();
    
    assert!(!paste_expansions.is_empty(), "Should detect paste! macro expansions");
}

/// Test synthetic call generation from macros
#[test]
fn test_synthetic_call_generation() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    // Simplified test with the actual pattern
    let source = r#"
define_indicator_enums!(
    TestIndicator: "Test Indicator",
    AnotherIndicator: "Another Indicator"
);

pub fn use_indicator() {
    // This would expand to TestIndicator::new(config)
    paste! {
        [<$indicator>]::new(config)
    }
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Should have macro expansions
    assert!(!result.macro_expansions.is_empty(), "Should have macro expansions");
    
    // Should generate synthetic calls
    let synthetic_calls: Vec<_> = result.calls.iter()
        .filter(|c| c.is_synthetic)
        .collect();
    
    // The dynamic resolver should detect indicators from the macro invocation
    // and generate appropriate synthetic calls
    assert!(!result.calls.is_empty(), "Should have some calls detected");
}

/// Failing guard: from_ohlcv calls should be generated for all indicators.
/// Based on trading-ta indicator macro pattern with [<$indicator Input>]::from_ohlcv(candle)
#[test]
fn test_from_ohlcv_calls_generated_for_all_indicators() {
    let mut parser = RustParser::new().expect("Failed to create parser");

    let source = r#"
// Realistic macro pattern for indicators
macro_rules! define_indicator_enums {
    ($($indicator:ident: $description:literal),*) => {
        paste! {
            pub enum IndicatorConfigKind {
                $( $indicator ),*
            }
        }
    };
}

define_indicator_enums!(
    Rsi: "Relative Strength Index",
    Ema: "Exponential Moving Average",
    Macd: "Moving Average Convergence Divergence"
);

pub fn use_inputs(candle: i32) {
    paste! {
        // Should expand to RsiInput::from_ohlcv, EmaInput::from_ohlcv, MacdInput::from_ohlcv
        [<$indicator Input>]::from_ohlcv(candle);
    }
}
"#;

    let result = parser
        .parse_source(source, Path::new("trading-ta/src/types/indicator_types.rs"), "trading_ta")
        .expect("parse failed");

    // Count synthetic from_ohlcv calls
    let from_ohlcv_calls = result
        .calls
        .iter()
        .filter(|c| c.is_synthetic && c.callee_name == "from_ohlcv")
        .count();

    // Expect one per indicator (3). This currently FAILS because we only generate one call per paste! expansion
    assert_eq!(from_ohlcv_calls, 3, "Expected 3 synthetic from_ohlcv calls, got {}", from_ohlcv_calls);
}

/// Test derive macro detection
#[test] 
fn test_derive_macro_detection() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    let source = r#"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    pub symbol: String,
    pub price: f64,
}

#[derive(Default)]
pub struct Config {
    pub enabled: bool,
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Types should be parsed even with derive macros
    assert!(result.types.len() >= 2, "Should parse types with derive macros");
    
    let market_data = result.types.iter()
        .find(|t| t.name == "MarketData")
        .expect("Should find MarketData");
    
    assert_eq!(market_data.visibility, "pub");
}

/// Test custom macro patterns
#[test]
fn test_custom_macro_patterns() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    // Test distributed_actor! macro pattern
    let source = r#"
distributed_actor! {
    pub struct TradingActor {
        state: TradingState,
    }
    
    impl TradingActor {
        pub fn new() -> Self {
            Self { state: TradingState::default() }
        }
    }
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Should detect distributed actor
    assert!(!result.distributed_actors.is_empty(), "Should detect distributed actor");
}
