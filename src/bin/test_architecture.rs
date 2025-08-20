use anyhow::Result;
use serde_json::json;
use workspace_analyzer::mcp::{WorkspaceMcpServer, McpRequest};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 Testing Architecture Violation Detection...");
    
    // Initialize the server with trading backend workspace
    let workspace_path = Path::new("/Users/greg/dev/git/trading-backend-poc");
    let server = WorkspaceMcpServer::new(&workspace_path).await?;
    
    // Initialize
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
    
    // Test architecture violations
    let violations_request = McpRequest {
        id: Some(json!(2)),
        method: "check_architecture_violations".to_string(),
        params: None,
    };
    
    println!("\\n🏗️ Checking architecture violations...");
    let violations_response = server.handle_request(violations_request).await;
    
    match violations_response.result {
        Some(result) => {
            if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
                if let Some(text_content) = content.first().and_then(|c| c.get("text")).and_then(|t| t.as_str()) {
                    println!("{}", text_content);
                }
            }
        }
        None => println!("❌ Architecture check failed: {:?}", violations_response.error),
    }
    
    // Test circular dependencies
    let circular_request = McpRequest {
        id: Some(json!(3)),
        method: "find_dependency_issues".to_string(),
        params: None,
    };
    
    println!("\\n🔄 Checking for circular dependencies...");
    let circular_response = server.handle_request(circular_request).await;
    
    match circular_response.result {
        Some(result) => {
            if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
                if let Some(text_content) = content.first().and_then(|c| c.get("text")).and_then(|t| t.as_str()) {
                    println!("{}", text_content);
                }
            }
        }
        None => println!("❌ Circular dependency check failed: {:?}", circular_response.error),
    }
    
    println!("\\n🎉 Architecture analysis complete!");
    
    Ok(())
}