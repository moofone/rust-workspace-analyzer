use std::path::Path;
use workspace_analyzer::{WorkspaceAnalyzer, Config};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // Point to the trading backend workspace
    let config = Config::from_workspace_root("/Users/greg/Dev/git/trading-backend-poc")?;
    
    // Create analyzer
    let mut analyzer = WorkspaceAnalyzer::new_with_config(config)?;
    
    // Get snapshot
    let snapshot = analyzer.create_snapshot().await?;
    
    println!("📊 Found {} distributed actors:", snapshot.distributed_actors.len());
    for actor in &snapshot.distributed_actors {
        println!("  🎭 Actor: {} ({})", actor.actor_name, actor.crate_name);
        println!("     📁 File: {}", actor.file_path);
        println!("     📍 Line: {}", actor.line);
        println!("     📨 Distributed messages: {:?}", actor.distributed_messages);
        println!("     📧 Local messages: {:?}", actor.local_messages);
        println!();
    }
    
    Ok(())
}