use anyhow::Result;
use tokio;

use workspace_analyzer::mcp::EnhancedMcpServer;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    eprintln!("🚀 Starting Enhanced MCP Server with Tree-sitter + Memgraph 3.0");
    eprintln!("📄 Using config file: config.toml");
    
    let server = EnhancedMcpServer::new("config.toml").await?;
    
    // Check for --no-auto-init flag
    let args: Vec<String> = std::env::args().collect();
    let skip_auto_init = args.contains(&"--no-auto-init".to_string());
    
    if !skip_auto_init {
        // Auto-initialize workspace on startup
        eprintln!("🔄 Auto-initializing workspace...");
        let start = std::time::Instant::now();
        server.auto_initialize().await?;
        eprintln!("⏱️ Auto-initialization took: {:?}", start.elapsed());
    } else {
        eprintln!("⚡ Skipping auto-initialization (use 'initialize' method to analyze workspace)");
    }
    
    eprintln!("✅ MCP Server ready for requests (call 'initialize' to analyze workspace)!");
    eprintln!("💡 Available methods:");
    eprintln!("   - initialize");
    eprintln!("   - workspace_context");
    eprintln!("   - analyze_change_impact");
    eprintln!("   - check_architecture_violations");
    eprintln!("   - semantic_search");
    eprintln!("   - get_function_details");
    eprintln!("   - get_type_details");
    eprintln!("   - get_crate_overview");
    eprintln!("   - get_layer_health");
    eprintln!("   - incremental_update");
    eprintln!("   - list_functions");
    eprintln!("   - debug_graph");
    
    let stdin = tokio::io::stdin();
    let mut reader = tokio::io::BufReader::new(stdin);
    let stdout = tokio::io::stdout();
    let mut writer = tokio::io::BufWriter::new(stdout);

    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
    
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {
                if let Ok(request_json) = serde_json::from_str::<serde_json::Value>(&line) {
                    let request = workspace_analyzer::mcp::McpRequest {
                        id: request_json.get("id").cloned(),
                        method: request_json.get("method")
                            .and_then(|m| m.as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        params: request_json.get("params").cloned(),
                    };
                    
                    let response = server.handle_request(request).await;
                    let response_json = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": response.id,
                        "result": response.result,
                        "error": response.error
                    });
                    
                    writer.write_all(response_json.to_string().as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                    writer.flush().await?;
                }
            }
            Err(e) => {
                eprintln!("Error reading from stdin: {}", e);
                break;
            }
        }
    }
    
    Ok(())
}