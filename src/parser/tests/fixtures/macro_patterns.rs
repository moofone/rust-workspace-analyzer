// Comprehensive macro patterns from trading-backend-poc

/// Test fixture for paste! macro usage with identifier concatenation
pub const PASTE_MACRO_PATTERN: &str = r#"
use paste::paste;

// Pattern from trading-runtime/src/global_builder_macro.rs
macro_rules! generate_builder {
    ($exchange:ident, $exchange_type:ty, $strategy:ident, $strategy_type:ty) => {
        paste! {
            pub struct [<$exchange:camel $strategy:camel Builder>] {
                config_path: String,
            }

            impl [<$exchange:camel $strategy:camel Builder>] {
                pub fn new(config_path: String) -> Self {
                    Self { config_path }
                }

                pub async fn build(self) -> Result<CryptoFuturesRuntime<$exchange_type, $strategy_type>, anyhow::Error> {
                    let _config = load_config_macro(&self.config_path)?;

                    Ok(CryptoFuturesRuntime {
                        ta_manager: "CryptoFuturesTAManager".to_string(),
                        execution_router: "CryptoFuturesExecutionRouter".to_string(),
                        strategy_manager: "CryptoFuturesStrategyManager".to_string(),
                        _exchange: PhantomData,
                        _strategy: PhantomData,
                    })
                }
            }
        }
    };
}

// Invocation of the macro
generate_builder!(bybit, BybitExchange, divergence_dev, DivergenceDevStrategy);
generate_builder!(binance, BinanceExchange, momentum, MomentumStrategy);
"#;

/// Test fixture for complex nested macro with paste!
pub const NESTED_PASTE_MACRO: &str = r#"
use paste::paste;

// Pattern from trading-ta/src/types/indicator_types.rs
macro_rules! define_indicator_enums {
    ($($indicator:ident: $description:literal),*) => {
        paste! {
            // Define IndicatorConfigKind enum with auto-assigned explicit discriminants
            #[derive(Debug, Clone)]
            pub enum IndicatorConfigKind {
                $(
                    $indicator( [<$indicator Config>] ),
                )*
            }

            // Define IndicatorOutputKind enum
            #[derive(Debug, Clone)]
            pub enum IndicatorOutputKind {
                $(
                    $indicator( [<$indicator Output>] ),
                )*
            }

            // Implementation methods for IndicatorConfigKind
            impl IndicatorConfigKind {
                pub fn get_name(&self) -> String {
                    match self {
                        $( IndicatorConfigKind::$indicator(_) => stringify!($indicator).to_string() ),*
                    }
                }

                pub async fn compute_batch_parallel<C>(
                    self,
                    candles: Vec<C>,
                    timeframe: Seconds
                ) -> Result<Vec<(IndicatorOutputKind, TimestampMS)>, String>
                where
                    C: OHLCV + Clone + Send + 'static
                {
                    tokio::task::spawn_blocking(move || {
                        let mut results = Vec::with_capacity(candles.len());

                        match self {
                            $(
                                IndicatorConfigKind::$indicator(config) => {
                                    // Create indicator instance using paste! generated type
                                    let mut indicator = paste! { [<$indicator>]::new(config) };

                                    for candle in &candles {
                                        let timestamp_ms = candle.timestamp().timestamp_millis();
                                        
                                        // Map input using paste! generated Input type
                                        let input = paste! { [<$indicator Input>]::from_ohlcv(candle) };
                                        
                                        let output = indicator.compute(timestamp_ms, input, timeframe);
                                        results.push((IndicatorOutputKind::$indicator(output), timestamp_ms));
                                        indicator.close_candle(timeframe);
                                    }
                                }
                            )*
                        }
                        Ok(results)
                    }).await.unwrap()
                }
            }
        }
    };
}

// Invocation with actual indicators
define_indicator_enums!(
    Alma: "Arnaud Legoux Moving Average",
    Atr: "Average True Range",
    Bb: "Bollinger Bands",
    Cvd: "Cumulative Volume Delta",
    DeltaVix: "Delta VIX indicator",
    Ema: "Exponential Moving Average",
    Macd: "MACD indicator",
    Rsi: "Relative Strength Index",
    Sma: "Simple Moving Average"
);
"#;

/// Test fixture for macro_rules! patterns
pub const MACRO_RULES_PATTERNS: &str = r#"
// Simple macro_rules! pattern
macro_rules! log_error {
    ($msg:expr) => {
        eprintln!("ERROR: {}", $msg);
    };
    ($msg:expr, $($arg:tt)*) => {
        eprintln!("ERROR: {}", format!($msg, $($arg)*));
    };
}

// More complex macro with repetition
macro_rules! generate_crypto_futures_builders {
    () => {
        pub mod builders {
            use super::*;

            // Generate all builder combinations
            generate_builder!(bybit, BybitExchange, divergence_dev, DivergenceDevStrategy);
            generate_builder!(bybit, BybitExchange, momentum, MomentumStrategy);
            generate_builder!(binance, BinanceExchange, divergence_dev, DivergenceDevStrategy);

            pub fn get_builder(exchange: &str, strategy: &str, config_path: String) -> Result<Box<dyn std::any::Any>, anyhow::Error> {
                match (exchange, strategy) {
                    ("bybit", "divergence_dev") => Ok(Box::new(BybitDivergenceDevBuilder::new(config_path))),
                    ("bybit", "momentum") => Ok(Box::new(BybitMomentumBuilder::new(config_path))),
                    _ => anyhow::bail!("Unsupported combination"),
                }
            }
        }
    };
}

// Usage
generate_crypto_futures_builders!();
"#;

/// Test fixture for standard library macros
pub const STDLIB_MACROS: &str = r#"
use std::collections::HashMap;

fn test_stdlib_macros() {
    // assert! family
    assert!(true);
    assert_eq!(1, 1);
    assert_ne!(1, 2);
    debug_assert!(true);
    
    // println! family
    println!("Hello, world!");
    eprintln!("Error message");
    print!("No newline");
    
    // format! family
    let s = format!("Value: {}", 42);
    
    // vec! macro
    let v = vec![1, 2, 3];
    let v2 = vec![0; 10];
    
    // matches! macro
    let x = Some(5);
    let is_some = matches!(x, Some(_));
    
    // panic! and related
    if false {
        panic!("This should not happen");
        unreachable!("This code is unreachable");
        unimplemented!("Not yet implemented");
        todo!("Implement this later");
    }
    
    // write! macros
    use std::fmt::Write;
    let mut s = String::new();
    write!(s, "Hello {}", "world").unwrap();
    writeln!(s, "New line").unwrap();
    
    // cfg! macro
    if cfg!(target_os = "linux") {
        println!("Running on Linux");
    }
    
    // thread_local! macro
    thread_local! {
        static COUNTER: std::cell::RefCell<u32> = std::cell::RefCell::new(0);
    }
}
"#;

/// Test fixture for async and Kameo-specific macros
pub const ASYNC_AND_KAMEO_MACROS: &str = r#"
use kameo::Actor;
use tokio::select;

async fn test_async_macros() {
    // tokio::select! macro
    let fut1 = async { 1 };
    let fut2 = async { 2 };
    
    select! {
        val = fut1 => println!("fut1 completed with {}", val),
        val = fut2 => println!("fut2 completed with {}", val),
    }
}

// Kameo-specific patterns (though not macros, included for context)
impl Actor for MyActor {
    type Args = Self;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    
    fn name() -> &'static str {
        "MyActor"
    }
}
"#;

/// Test fixture for derive macros
pub const DERIVE_MACROS: &str = r#"
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub name: String,
    pub value: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Status {
    Active,
    Inactive,
}

// Derive with attributes
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse {
    pub status_code: u16,
    pub message_text: String,
}
"#;

/// Test fixture for attribute macros (non-derive)
pub const ATTRIBUTE_MACROS: &str = r#"
// Test attribute
#[test]
fn test_something() {
    assert_eq!(1 + 1, 2);
}

// Async trait (if using async-trait crate)
#[async_trait::async_trait]
trait MyAsyncTrait {
    async fn do_something(&self) -> Result<(), Box<dyn std::error::Error>>;
}

// Kameo remote attribute
#[kameo(remote)]
#[derive(Debug, Clone)]
struct RemoteMessage {
    pub data: String,
}

// Criterion benchmarks
#[criterion::criterion_group!(benches, benchmark_function)]
#[criterion::criterion_main!(benches)]
"#;

/// Test fixture for logging macros
pub const LOGGING_MACROS: &str = r#"
use tracing::{info, warn, error, debug, trace};

fn test_logging() {
    info!("Information message");
    warn!("Warning message");
    error!("Error message");
    debug!("Debug message");
    trace!("Trace message");
    
    // With formatting
    let value = 42;
    info!("Value is {}", value);
    error!("Failed to process: {}", "reason");
    
    // Structured logging
    info!(value = 42, "Processing started");
    error!(error = %"some error", "Operation failed");
}
"#;

/// Test fixture for custom macro invocations from trading-backend-poc
pub const CUSTOM_TRADING_MACROS: &str = r#"
// Custom macros from trading-backend-poc

// Decimal macro (from rust_decimal)
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn test_decimal() {
    let price = dec!(42.50);
    let quantity = dec!(0.001);
    let fee = dec!(0.0002);
}

// Custom builder macro invocation
generate_builder!(bybit, BybitExchange, divergence_dev, DivergenceDevStrategy);

// Custom indicator enum macro invocation
define_indicator_enums!(
    Rsi: "Relative Strength Index",
    Macd: "MACD indicator"
);

// Strategy definition macro from trading-runtime
define_strategies! {
    ImportantPoints(ImportantPointsData, ImportantPointsLogic) => "important_points",
    MultiRsi(MultiRsiData, MultiRsiLogic) => "multi_rsi",
    DivergenceDev(DivergenceDevData, DivergenceDevLogic) => "divergence_dev",
    Testing(TestingData, TestingLogic) => "testing",
}

// Distributed actor macro (hypothetical, as actual implementation may vary)
distributed_actor! {
    pub struct DataDistributedActor {
        data: Vec<f64>,
    }
    
    impl DataDistributedActor {
        pub async fn process(&self) {
            println!("Processing data");
        }
    }
}
"#;

/// Test fixture for macro expansion results (synthetic calls)
pub const MACRO_EXPANSION_RESULTS: &str = r#"
// This represents what the paste! macro would expand to

// Original macro invocation:
// paste! { [<Bybit $strategy:camel Builder>] }
// Expands to:
pub struct BybitDivergenceDevBuilder {
    config_path: String,
}

// These synthetic calls should be detected by the parser
impl BybitDivergenceDevBuilder {
    pub fn new(config_path: String) -> Self {
        Self { config_path }
    }
}

// Synthetic method calls generated by macros
// Original: paste! { [<$indicator>]::new(config) }
// Original: paste! { [<$indicator Input>]::from_ohlcv(candle) }
// Expands to the following function:
fn process_indicators() {
    let rsi = Rsi::new(RsiConfig::default());
    let input = RsiInput::from_ohlcv(&candle);
    let output = rsi.compute(timestamp, input, timeframe);
}
"#;