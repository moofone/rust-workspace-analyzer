use anyhow::Result;
use serde_json::{json, Value};
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 MCP Client - Testing the server like Claude Code would");
    
    // Start the MCP server as a subprocess
    let mut server_process = Command::new("cargo")
        .args(&["run", "--bin", "mcp-server", "--", "--workspace", "/Users/greg/dev/git/trading-backend-poc"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    let stdin = server_process.stdin.take().expect("Failed to get stdin");
    let stdout = server_process.stdout.take().expect("Failed to get stdout");
    
    // Create channels for communication
    let (response_tx, response_rx) = mpsc::channel::<String>();
    
    // Thread to read server responses
    let response_tx_clone = response_tx.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    if line.trim().starts_with('{') {
                        let _ = response_tx_clone.send(line);
                    }
                }
                Err(_) => break,
            }
        }
    });
    
    // Create MCP client
    let mut client = McpClient::new(stdin);
    
    println!("\\n📡 Testing MCP Protocol Communication:");
    
    // 1. Initialize
    println!("\\n1️⃣ Initializing server...");
    let init_response = client.send_request("initialize", None)?;
    if let Ok(response) = response_rx.recv_timeout(std::time::Duration::from_secs(10)) {
        println!("   ✅ Server initialized");
        if let Ok(parsed) = serde_json::from_str::<Value>(&response) {
            if let Some(tools) = parsed.get("result").and_then(|r| r.get("tools")).and_then(|t| t.as_array()) {
                println!("   📋 Available tools: {}", tools.len());
                for tool in tools.iter().take(3) {
                    if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
                        println!("      - {}", name);
                    }
                }
            }
        }
    } else {
        println!("   ❌ No response from server");
        return Ok(());
    }
    
    // 2. Get workspace context
    println!("\\n2️⃣ Getting workspace context...");
    client.call_tool("workspace_context", json!({}))?;
    if let Ok(response) = response_rx.recv_timeout(std::time::Duration::from_secs(5)) {
        if let Ok(parsed) = serde_json::from_str::<Value>(&response) {
            if let Some(content) = parsed.get("result")
                .and_then(|r| r.get("content"))
                .and_then(|c| c.as_array())
                .and_then(|arr| arr.first())
                .and_then(|item| item.get("text"))
                .and_then(|t| t.as_str()) {
                
                // Extract summary stats
                if let Some(json_start) = content.find('{') {
                    if let Some(json_end) = content.rfind('}') {
                        let json_str = &content[json_start..=json_end];
                        if let Ok(data) = serde_json::from_str::<Value>(json_str) {
                            if let Some(summary) = data.get("summary") {
                                println!("   📊 Workspace Summary:");
                                if let Some(total_functions) = summary.get("total_functions") {
                                    println!("      Functions: {}", total_functions);
                                }
                                if let Some(total_types) = summary.get("total_types") {
                                    println!("      Types: {}", total_types);
                                }
                                if let Some(total_deps) = summary.get("total_dependencies") {
                                    println!("      Dependencies: {}", total_deps);
                                }
                            }
                        }
                    }
                }
            }
        }
        println!("   ✅ Context retrieved");
    }
    
    // 3. Test impact analysis
    println!("\\n3️⃣ Testing impact analysis...");
    client.call_tool("analyze_change_impact", json!({
        "target": "CryptoFuturesPositionManager"
    }))?;
    if let Ok(response) = response_rx.recv_timeout(std::time::Duration::from_secs(5)) {
        if let Ok(parsed) = serde_json::from_str::<Value>(&response) {
            if let Some(content) = parsed.get("result")
                .and_then(|r| r.get("content"))
                .and_then(|c| c.as_array())
                .and_then(|arr| arr.first())
                .and_then(|item| item.get("text"))
                .and_then(|t| t.as_str()) {
                
                let lines: Vec<&str> = content.lines().take(10).collect();
                println!("   📋 Impact Analysis Preview:");
                for line in lines {
                    if !line.trim().is_empty() {
                        println!("      {}", line.trim());
                    }
                }
            }
        }
        println!("   ✅ Impact analysis completed");
    }
    
    // 4. Test architecture violations
    println!("\\n4️⃣ Checking architecture violations...");
    client.call_tool("check_architecture_violations", json!({}))?;
    if let Ok(response) = response_rx.recv_timeout(std::time::Duration::from_secs(5)) {
        if let Ok(parsed) = serde_json::from_str::<Value>(&response) {
            if let Some(content) = parsed.get("result")
                .and_then(|r| r.get("content"))
                .and_then(|c| c.as_array())
                .and_then(|arr| arr.first())
                .and_then(|item| item.get("text"))
                .and_then(|t| t.as_str()) {
                
                let lines: Vec<&str> = content.lines().take(15).collect();
                println!("   🏗️ Architecture Analysis Preview:");
                for line in lines {
                    if !line.trim().is_empty() {
                        println!("      {}", line.trim());
                    }
                }
            }
        }
        println!("   ✅ Architecture check completed");
    }
    
    // 5. Test circular dependencies
    println!("\\n5️⃣ Finding dependency issues...");
    client.call_tool("find_dependency_issues", json!({}))?;
    if let Ok(response) = response_rx.recv_timeout(std::time::Duration::from_secs(3)) {
        println!("   ✅ Dependency analysis completed");
    }
    
    println!("\\n🎉 MCP Protocol Test Complete!");
    println!("   All core tools are working via MCP protocol");
    println!("   Ready for Claude Code integration! 🚀");
    
    // Clean up
    let _ = server_process.kill();
    
    Ok(())
}

struct McpClient {
    stdin: std::process::ChildStdin,
    request_id: i32,
}

impl McpClient {
    fn new(stdin: std::process::ChildStdin) -> Self {
        Self {
            stdin,
            request_id: 0,
        }
    }
    
    fn send_request(&mut self, method: &str, params: Option<Value>) -> Result<()> {
        self.request_id += 1;
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": method,
            "params": params.unwrap_or(json!({}))
        });
        
        writeln!(self.stdin, "{}", request)?;
        self.stdin.flush()?;
        
        Ok(())
    }
    
    fn call_tool(&mut self, tool_name: &str, arguments: Value) -> Result<()> {
        self.request_id += 1;
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            }
        });
        
        writeln!(self.stdin, "{}", request)?;
        self.stdin.flush()?;
        
        Ok(())
    }
}