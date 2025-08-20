use anyhow::Result;
use serde_json::json;
use workspace_analyzer::mcp::{WorkspaceMcpServer, McpRequest};
use std::path::Path;
use clap::Parser;

#[derive(Parser)]
#[command(name = "mcp-server")]
#[command(about = "Rust Workspace MCP Server for Claude Code integration")]
struct Args {
    /// Path to the workspace to analyze
    #[arg(short, long, default_value = ".")]
    workspace: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    println!("🚀 Starting Rust Workspace MCP Server...");
    println!("📁 Target workspace: {}", args.workspace);
    
    // Initialize the server with specified workspace
    let workspace_path = Path::new(&args.workspace);
    if !workspace_path.exists() {
        eprintln!("❌ Error: Workspace path '{}' does not exist", args.workspace);
        std::process::exit(1);
    }
    
    let server = WorkspaceMcpServer::new(&workspace_path).await?;
    
    // Test initialization
    let init_request = McpRequest {
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: None,
    };
    
    println!("📊 Initializing workspace analysis...");
    let response = server.handle_request(init_request).await;
    
    if response.error.is_some() {
        println!("❌ Initialization failed: {:?}", response.error);
        return Ok(());
    }
    
    println!("✅ Server initialized successfully!");
    
    // Test workspace_context
    let context_request = McpRequest {
        id: Some(json!(2)),
        method: "workspace_context".to_string(),
        params: None,
    };
    
    println!("\\n📋 Getting workspace context...");
    let context_response = server.handle_request(context_request).await;
    
    match context_response.result {
        Some(result) => {
            if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
                if let Some(text_content) = content.first().and_then(|c| c.get("text")).and_then(|t| t.as_str()) {
                    println!("{}", text_content);
                }
            }
        }
        None => println!("❌ Failed to get context: {:?}", context_response.error),
    }
    
    // Test impact analysis
    println!("\\n🔍 Testing change impact analysis...");
    let impact_request = McpRequest {
        id: Some(json!(3)),
        method: "analyze_change_impact".to_string(),
        params: Some(json!({"target": "main"})),
    };
    
    let impact_response = server.handle_request(impact_request).await;
    match impact_response.result {
        Some(result) => {
            if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
                if let Some(text_content) = content.first().and_then(|c| c.get("text")).and_then(|t| t.as_str()) {
                    println!("{}", text_content);
                }
            }
        }
        None => println!("❌ Impact analysis failed: {:?}", impact_response.error),
    }
    
    println!("\\n🎉 MCP Server test completed!");
    println!("Next: Configure Claude Code to connect to this server");
    
    Ok(())
}