use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 MCP Protocol Inspector - Raw JSON-RPC Messages");
    println!("This shows the exact messages Claude Code will send/receive");
    
    // Start the MCP server
    let mut server = Command::new("cargo")
        .args(&["run", "--bin", "mcp-server-stdio", "--", "--workspace", "/Users/greg/dev/git/trading-backend-poc"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    let mut stdin = server.stdin.take().unwrap();
    let stdout = server.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    
    println!("\\n{}", "=".repeat(80));
    
    // Helper function to send request and show full exchange
    let mut send_and_show = |name: &str, request: Value| -> Result<Value> {
        println!("\\n📤 {} REQUEST:", name);
        println!("{}", serde_json::to_string_pretty(&request)?);
        
        // Send request
        writeln!(stdin, "{}", serde_json::to_string(&request)?)?;
        stdin.flush()?;
        
        // Read response
        let mut response_line = String::new();
        reader.read_line(&mut response_line)?;
        
        let response: Value = serde_json::from_str(&response_line)?;
        
        println!("\\n📥 {} RESPONSE:", name);
        println!("{}", serde_json::to_string_pretty(&response)?);
        println!("\\n{}", "-".repeat(80));
        
        Ok(response)
    };
    
    // 1. Initialize
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });
    
    let init_response = send_and_show("INITIALIZE", init_request)?;
    
    // 2. List tools
    let tools_request = json!({
        "jsonrpc": "2.0", 
        "id": 2,
        "method": "tools/list",
        "params": {}
    });
    
    let tools_response = send_and_show("TOOLS/LIST", tools_request)?;
    
    // 3. Call workspace_context tool
    let context_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "workspace_context",
            "arguments": {}
        }
    });
    
    let context_response = send_and_show("WORKSPACE_CONTEXT", context_request)?;
    
    // 4. Call analyze_change_impact tool  
    let impact_request = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call", 
        "params": {
            "name": "analyze_change_impact",
            "arguments": {
                "target": "CryptoFuturesPositionManager"
            }
        }
    });
    
    let impact_response = send_and_show("ANALYZE_CHANGE_IMPACT", impact_request)?;
    
    // 5. Call check_architecture_violations tool
    let arch_request = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "check_architecture_violations", 
            "arguments": {}
        }
    });
    
    let arch_response = send_and_show("CHECK_ARCHITECTURE_VIOLATIONS", arch_request)?;
    
    println!("\\n🎉 MCP Protocol Inspection Complete!");
    println!("\\n📋 **What Claude Code sees:**");
    println!("   • JSON-RPC 2.0 protocol over stdin/stdout");
    println!("   • Initialize handshake with server capabilities");
    println!("   • Tool discovery via tools/list");
    println!("   • Tool execution via tools/call with structured arguments");
    println!("   • Rich responses with formatted text content");
    
    println!("\\n🔧 **Available Tools for Claude:**");
    if let Some(tools) = tools_response.get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array()) {
        for tool in tools {
            if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
                if let Some(desc) = tool.get("description").and_then(|d| d.as_str()) {
                    println!("   • {}: {}", name, desc);
                }
            }
        }
    }
    
    println!("\\n🚀 **Ready for Claude Code Integration!**");
    println!("   Configure Claude Code to run:");
    println!("   cargo run --bin mcp-server-stdio --workspace <your-workspace-path>");
    
    // Clean up
    let _ = server.kill();
    
    Ok(())
}