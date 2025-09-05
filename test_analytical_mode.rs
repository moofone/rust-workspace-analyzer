use anyhow::Result;
use workspace_analyzer::{Config, MemgraphClient};
use workspace_analyzer::graph::StorageMode;

#[tokio::main]
async fn main() -> Result<()> {
    // Load config
    let config = Config::from_file("config.toml")?;
    
    println!("Connecting to Memgraph...");
    let client = MemgraphClient::new(&config).await?;
    
    println!("Testing ANALYTICAL mode switch...");
    println!("Switching to ANALYTICAL mode (10 second timeout)...");
    
    match client.set_storage_mode(StorageMode::InMemoryAnalytical).await {
        Ok(_) => println!("✅ Successfully switched to ANALYTICAL mode!"),
        Err(e) => println!("❌ Failed to switch to ANALYTICAL mode: {}", e),
    }
    
    println!("\nTesting TRANSACTIONAL mode switch...");
    println!("Switching to TRANSACTIONAL mode (10 second timeout)...");
    
    match client.set_storage_mode(StorageMode::InMemoryTransactional).await {
        Ok(_) => println!("✅ Successfully switched to TRANSACTIONAL mode!"),
        Err(e) => println!("❌ Failed to switch to TRANSACTIONAL mode: {}", e),
    }
    
    println!("\nAll tests completed!");
    Ok(())
}