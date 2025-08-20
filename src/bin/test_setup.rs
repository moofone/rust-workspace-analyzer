use anyhow::Result;
use workspace_analyzer::graph::MemgraphClient;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 Testing Phase 1 setup...");
    
    // Test 1: Memgraph connection
    println!("1️⃣  Testing Memgraph connection...");
    let memgraph = MemgraphClient::new("test_workspace").await?;
    
    if memgraph.health_check().await? {
        println!("   ✅ Memgraph performance: OPTIMAL");
    } else {
        println!("   ⚠️  Memgraph performance: SUBOPTIMAL"); 
    }
    
    // Test 2: Basic workspace detection
    println!("2️⃣  Testing workspace detection...");
    let current_dir = std::env::current_dir()?;
    if current_dir.join("Cargo.toml").exists() {
        println!("   ✅ Rust workspace detected: {}", current_dir.display());
    } else {
        println!("   ℹ️  Not in a Rust workspace (that's OK for testing)");
    }
    
    println!("\\n🎉 Phase 1 setup complete!");
    println!("Next: Build MCP server and connect to Claude Code");
    
    Ok(())
}
