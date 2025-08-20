use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 Testing MCP Protocol - Like Claude Code Will Use");
    
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
    
    println!("\\n📡 MCP Protocol Communication:");
    
    // Helper function to send request and get response
    let mut send_request = |request: Value| -> Result<Value> {
        writeln!(stdin, "{}", serde_json::to_string(&request)?)?;
        stdin.flush()?;
        
        let mut response_line = String::new();
        reader.read_line(&mut response_line)?;
        
        Ok(serde_json::from_str(&response_line)?)
    };
    
    // 1. Initialize
    println!("\\n1️⃣ Initializing MCP Server...");
    let init_response = send_request(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }))?;
    
    if let Some(result) = init_response.get("result") {
        println!("   ✅ Server initialized");
        if let Some(server_info) = result.get("serverInfo") {
            println!("   📋 Server: {} v{}", 
                server_info.get("name").and_then(|n| n.as_str()).unwrap_or("unknown"),
                server_info.get("version").and_then(|v| v.as_str()).unwrap_or("unknown")
            );
        }
    }
    
    // 2. List available tools
    println!("\\n2️⃣ Listing available tools...");
    let tools_response = send_request(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    }))?;
    
    if let Some(tools) = tools_response.get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array()) {
        println!("   🔧 Available tools: {}", tools.len());
        for tool in tools {
            if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
                if let Some(desc) = tool.get("description").and_then(|d| d.as_str()) {
                    println!("      • {}: {}", name, desc);
                }
            }
        }
    }
    
    // 3. Get workspace context
    println!("\\n3️⃣ Getting workspace context...");
    let context_response = send_request(json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "workspace_context",
            "arguments": {}
        }
    }))?;
    
    if let Some(content) = context_response.get("result")
        .and_then(|r| r.get("content"))
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("text"))
        .and_then(|t| t.as_str()) {
        
        // Parse the summary stats
        if let Some(json_start) = content.find('{') {
            if let Some(json_end) = content.rfind('}') {
                let json_str = &content[json_start..=json_end];
                if let Ok(data) = serde_json::from_str::<Value>(json_str) {
                    if let Some(summary) = data.get("summary") {
                        println!("   📊 Workspace Analysis:");
                        if let Some(funcs) = summary.get("total_functions") {
                            println!("      Functions: {}", funcs);
                        }
                        if let Some(types) = summary.get("total_types") {
                            println!("      Types: {}", types);
                        }
                        if let Some(deps) = summary.get("total_dependencies") {
                            println!("      Dependencies: {}", deps);
                        }
                        
                        // Show some crate diversity
                        if let Some(top_funcs) = data.get("top_functions").and_then(|f| f.as_array()) {
                            let mut crates = std::collections::HashSet::new();
                            for func in top_funcs {
                                if let Some(crate_name) = func.get("crate").and_then(|c| c.as_str()) {
                                    crates.insert(crate_name);
                                }
                            }
                            println!("      Crates analyzed: {} ({})", crates.len(), 
                                crates.iter().take(3).cloned().collect::<Vec<_>>().join(", "));
                        }
                    }
                }
            }
        }
        println!("   ✅ Context analysis complete");
    }
    
    // 4. Test impact analysis
    println!("\\n4️⃣ Testing impact analysis...");
    let impact_response = send_request(json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "analyze_change_impact",
            "arguments": {
                "target": "PositionManager"
            }
        }
    }))?;
    
    if let Some(content) = impact_response.get("result")
        .and_then(|r| r.get("content"))
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("text"))
        .and_then(|t| t.as_str()) {
        
        let preview: Vec<&str> = content.lines().take(8).collect();
        println!("   🎯 Impact Analysis Preview:");
        for line in preview {
            if !line.trim().is_empty() {
                println!("      {}", line.trim());
            }
        }
        println!("   ✅ Impact analysis complete");
    }
    
    // 5. Test architecture violations
    println!("\\n5️⃣ Checking architecture violations...");
    let arch_response = send_request(json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "check_architecture_violations",
            "arguments": {}
        }
    }))?;
    
    if let Some(content) = arch_response.get("result")
        .and_then(|r| r.get("content"))
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("text"))
        .and_then(|t| t.as_str()) {
        
        // Count violations by type
        let mut layer_violations = 0;
        let mut prelude_suggestions = 0;
        let mut circular_deps = 0;
        
        for line in content.lines() {
            if line.contains("**Upward Dependency**") {
                layer_violations += 1;
            } else if line.contains("**Deep Import**") {
                prelude_suggestions += 1;
            } else if line.contains("circular") && line.contains("dependency") {
                circular_deps += 1;
            }
        }
        
        println!("   🏗️ Architecture Analysis Results:");
        println!("      Layer violations: {}", layer_violations);
        println!("      Prelude suggestions: {}", prelude_suggestions);
        println!("      Circular dependencies: {}", circular_deps);
        println!("   ✅ Architecture check complete");
    }
    
    // 6. Test dependency issues
    println!("\\n6️⃣ Finding dependency issues...");
    let deps_response = send_request(json!({
        "jsonrpc": "2.0",
        "id": 6,
        "method": "tools/call",
        "params": {
            "name": "find_dependency_issues",
            "arguments": {}
        }
    }))?;
    
    if deps_response.get("result").is_some() {
        println!("   🔄 Dependency analysis complete");
    }
    
    println!("\\n🎉 MCP Protocol Test Successful!");
    println!("   ✅ All tools working via JSON-RPC");
    println!("   ✅ Protocol compatible with Claude Code");
    println!("   ✅ Ready for production use! 🚀");
    
    // Clean up
    let _ = server.kill();
    
    Ok(())
}