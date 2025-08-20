use anyhow::Result;
use serde_json::json;
use workspace_analyzer::mcp::{WorkspaceMcpServer, McpRequest};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 Testing Crate Diversity in Top Functions/Types...");
    
    let workspace_path = Path::new("/Users/greg/dev/git/trading-backend-poc");
    let server = WorkspaceMcpServer::new(&workspace_path).await?;
    
    // Initialize
    let init_request = McpRequest {
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: None,
    };
    
    let response = server.handle_request(init_request).await;
    if response.error.is_some() {
        println!("❌ Init failed: {:?}", response.error);
        return Ok(());
    }
    
    // Get workspace context
    let context_request = McpRequest {
        id: Some(json!(2)),
        method: "workspace_context".to_string(),
        params: None,
    };
    
    let context_response = server.handle_request(context_request).await;
    
    if let Some(result) = context_response.result {
        if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
            if let Some(text_content) = content.first().and_then(|c| c.get("text")).and_then(|t| t.as_str()) {
                // Parse the JSON from the workspace analysis
                if let Some(json_start) = text_content.find('{') {
                    if let Some(json_end) = text_content.rfind('}') {
                        let json_str = &text_content[json_start..=json_end];
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(json_str) {
                            
                            println!("\\n🔧 **Top Functions by Crate:**");
                            if let Some(functions) = data.get("top_functions").and_then(|f| f.as_array()) {
                                let mut crate_count = std::collections::HashMap::new();
                                for func in functions {
                                    if let Some(crate_name) = func.get("crate").and_then(|c| c.as_str()) {
                                        *crate_count.entry(crate_name).or_insert(0) += 1;
                                    }
                                }
                                for (crate_name, count) in crate_count {
                                    println!("  - {}: {} functions", crate_name, count);
                                }
                            }
                            
                            println!("\\n📊 **Top Types by Crate:**");  
                            if let Some(types) = data.get("top_types").and_then(|t| t.as_array()) {
                                let mut crate_count = std::collections::HashMap::new();
                                for typ in types {
                                    if let Some(crate_name) = typ.get("crate").and_then(|c| c.as_str()) {
                                        *crate_count.entry(crate_name).or_insert(0) += 1;
                                    }
                                }
                                for (crate_name, count) in crate_count {
                                    println!("  - {}: {} types", crate_name, count);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    println!("\\n🎉 Diversity analysis complete!");
    
    Ok(())
}