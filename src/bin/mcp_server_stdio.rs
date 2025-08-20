use anyhow::Result;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use workspace_analyzer::mcp::{WorkspaceMcpServer, McpRequest};
use std::path::Path;
use clap::Parser;

#[derive(Parser)]
#[command(name = "mcp-server-stdio")]
#[command(about = "MCP Server with JSON-RPC over stdio for Claude Code")]
struct Args {
    /// Path to the workspace to analyze
    #[arg(short, long, default_value = ".")]
    workspace: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize the workspace analyzer
    let workspace_path = Path::new(&args.workspace);
    if !workspace_path.exists() {
        eprintln!("Error: Workspace path '{}' does not exist", args.workspace);
        std::process::exit(1);
    }
    
    let server = WorkspaceMcpServer::new(&workspace_path).await?;
    
    // Initialize the server automatically
    let init_request = McpRequest {
        id: Some(json!(0)),
        method: "initialize".to_string(),
        params: None,
    };
    let _ = server.handle_request(init_request).await;
    
    // Start JSON-RPC loop over stdio
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        
        // Parse JSON-RPC request
        match serde_json::from_str::<Value>(&line) {
            Ok(json_request) => {
                let response = handle_jsonrpc_request(&server, json_request).await;
                
                // Send response
                writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
                stdout.flush()?;
            }
            Err(e) => {
                let error_response = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {
                        "code": -32700,
                        "message": format!("Parse error: {}", e)
                    }
                });
                writeln!(stdout, "{}", serde_json::to_string(&error_response)?)?;
                stdout.flush()?;
            }
        }
    }
    
    Ok(())
}

async fn handle_jsonrpc_request(server: &WorkspaceMcpServer, request: Value) -> Value {
    let id = request.get("id");
    let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = request.get("params");
    
    match method {
        "initialize" => {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "rust-workspace-analyzer",
                        "version": "0.1.0"
                    }
                }
            })
        }
        "tools/list" => {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": [
                        {
                            "name": "workspace_context",
                            "description": "Get comprehensive context about the current workspace",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        },
                        {
                            "name": "analyze_change_impact",
                            "description": "Analyze the impact of changing a specific function or type",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "target": {"type": "string", "description": "Function or type name to analyze"}
                                },
                                "required": ["target"]
                            }
                        },
                        {
                            "name": "check_architecture_violations", 
                            "description": "Check for architectural rule violations",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        },
                        {
                            "name": "find_dependency_issues",
                            "description": "Find circular dependencies and other dependency issues",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        },
                        {
                            "name": "suggest_safe_refactoring",
                            "description": "Suggest safe refactoring opportunities",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "focus_area": {"type": "string", "description": "Area to focus refactoring on"}
                                }
                            }
                        },
                        {
                            "name": "validate_proposed_change",
                            "description": "Validate a proposed change for safety and architectural compliance",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "description": {"type": "string", "description": "Description of the proposed change"}
                                },
                                "required": ["description"]
                            }
                        },
                        {
                            "name": "analyze_test_coverage",
                            "description": "Analyze test coverage and find heavily-used functions without tests",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        }
                    ]
                }
            })
        }
        "tools/call" => {
            if let Some(params) = params {
                let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
                
                // Convert to our internal request format
                let mcp_request = McpRequest {
                    id: id.cloned(),
                    method: tool_name.to_string(),
                    params: Some(arguments),
                };
                
                let response = server.handle_request(mcp_request).await;
                
                if let Some(error) = response.error {
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": error.code,
                            "message": error.message,
                            "data": error.data
                        }
                    })
                } else {
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": response.result.unwrap_or(json!({}))
                    })
                }
            } else {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32602,
                        "message": "Invalid params"
                    }
                })
            }
        }
        _ => {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": format!("Method not found: {}", method)
                }
            })
        }
    }
}