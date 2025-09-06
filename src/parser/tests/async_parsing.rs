use crate::parser::RustParser;
use std::path::Path;

/// Test async function parsing from trading-exchanges patterns
#[test]
pub fn test_async_parsing() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    // Test case from trading-exchanges patterns
    let source = r#"
use async_trait::async_trait;
use tokio::sync::RwLock;

pub struct ExchangeClient {
    api_key: String,
    websocket: Option<WebSocket>,
}

impl ExchangeClient {
    // Async method
    pub async fn connect(&mut self) -> Result<(), Error> {
        self.websocket = Some(WebSocket::connect("wss://stream.binance.com").await?);
        Ok(())
    }
    
    // Async method with complex return type
    pub async fn get_orderbook(&self, symbol: &str) -> Result<OrderBook, Error> {
        let response = self.fetch_data(format!("/api/v3/depth?symbol={}", symbol)).await?;
        Ok(serde_json::from_str(&response)?)
    }
    
    // Private async method
    async fn fetch_data(&self, endpoint: String) -> Result<String, Error> {
        // Implementation
        Ok(String::new())
    }
}

// Async trait implementation
#[async_trait]
pub trait DataStream: Send + Sync {
    async fn subscribe(&mut self, symbols: Vec<String>) -> Result<(), Error>;
    async fn next_data(&mut self) -> Option<MarketData>;
}

#[async_trait]
impl DataStream for ExchangeClient {
    async fn subscribe(&mut self, symbols: Vec<String>) -> Result<(), Error> {
        // Implementation
        Ok(())
    }
    
    async fn next_data(&mut self) -> Option<MarketData> {
        None
    }
}

// Standalone async functions
pub async fn fetch_all_symbols(exchange: &str) -> Vec<String> {
    // Implementation
    vec![]
}

async fn internal_process() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

// Async closure
pub async fn process_with_timeout() {
    let handle = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        42
    });
    
    let result = handle.await;
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Check async methods
    let connect = result.functions.iter()
        .find(|f| f.name == "connect")
        .expect("Should find connect method");
    
    assert!(connect.is_async, "connect should be async");
    assert_eq!(connect.return_type, Some("Result<(), Error>".to_string()));
    
    let orderbook = result.functions.iter()
        .find(|f| f.name == "get_orderbook")
        .expect("Should find get_orderbook method");
    
    assert!(orderbook.is_async, "get_orderbook should be async");
    
    // Check async trait methods
    let subscribe = result.functions.iter()
        .find(|f| f.name == "subscribe" && f.is_trait_impl)
        .expect("Should find subscribe trait method");
    
    assert!(subscribe.is_async, "subscribe trait method should be async");
    
    // Check standalone async function
    let fetch_all = result.functions.iter()
        .find(|f| f.name == "fetch_all_symbols")
        .expect("Should find fetch_all_symbols");
    
    assert!(fetch_all.is_async, "fetch_all_symbols should be async");
    assert_eq!(fetch_all.visibility, "pub");
    
    // Check private async function
    let internal = result.functions.iter()
        .find(|f| f.name == "internal_process")
        .expect("Should find internal_process");
    
    assert!(internal.is_async, "internal_process should be async");
    assert_eq!(internal.visibility, "private");
}

/// Test async block and spawn detection
#[test]
fn test_async_blocks_and_spawns() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    let source = r#"
pub async fn complex_async_operations() {
    // Async block
    let future1 = async {
        calculate_indicators().await
    };
    
    // Spawn task
    let handle1 = tokio::spawn(async move {
        process_data().await
    });
    
    // Spawn with named function
    let handle2 = tokio::spawn(fetch_market_data());
    
    // Join handles
    let (result1, result2) = tokio::join!(handle1, handle2);
    
    // Select on multiple futures
    tokio::select! {
        val = future1 => println!("Future 1 completed"),
        _ = tokio::time::sleep(Duration::from_secs(1)) => println!("Timeout"),
    }
}

async fn calculate_indicators() -> f64 {
    42.0
}

async fn process_data() -> Result<(), Error> {
    Ok(())
}

async fn fetch_market_data() -> Vec<MarketData> {
    vec![]
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Check main async function
    let complex_ops = result.functions.iter()
        .find(|f| f.name == "complex_async_operations")
        .expect("Should find complex_async_operations");
    
    assert!(complex_ops.is_async);
    
    // Debug: print all detected calls
    println!("Detected {} calls:", result.calls.len());
    for call in &result.calls {
        println!("  - {}", call.callee_name);
    }
    
    // Check for tokio::spawn calls
    assert!(result.calls.iter().any(|c| c.callee_name == "spawn"),
            "Should detect tokio::spawn calls");
    
    // Check for join! macro call - may be detected as "join!" or "tokio::join!" with the exclamation
    assert!(result.calls.iter().any(|c| c.callee_name == "join" || c.callee_name == "join!" || c.callee_name == "tokio::join!"),
            "Should detect tokio::join! macro");
    
    // All helper functions should be async
    let helpers = ["calculate_indicators", "process_data", "fetch_market_data"];
    for helper in &helpers {
        let func = result.functions.iter()
            .find(|f| f.name == *helper)
            .expect(&format!("Should find {} function", helper));
        
        assert!(func.is_async, "{} should be async", helper);
    }
}