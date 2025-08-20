use serde_json::json;

fn main() {
    println!("🔍 MCP Protocol - Raw JSON-RPC Messages");
    println!("This is exactly what Claude Code sends/receives");
    
    println!("\n{}", "=".repeat(80));
    
    // 1. INITIALIZE REQUEST/RESPONSE
    println!("\n📤 CLAUDE CODE SENDS (Initialize):");
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });
    println!("{}", serde_json::to_string_pretty(&init_request).unwrap());
    
    println!("\n📥 OUR SERVER RESPONDS:");
    let init_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "rust-workspace-analyzer",
                "version": "0.1.0"
            }
        }
    });
    println!("{}", serde_json::to_string_pretty(&init_response).unwrap());
    
    // 2. TOOLS LIST REQUEST/RESPONSE
    println!("\n{}", "-".repeat(80));
    println!("\n📤 CLAUDE CODE SENDS (List Tools):");
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });
    println!("{}", serde_json::to_string_pretty(&tools_request).unwrap());
    
    println!("\n📥 OUR SERVER RESPONDS (Available Tools):");
    let tools_response = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "result": {
            "tools": [
                {
                    "name": "workspace_context",
                    "description": "Get comprehensive context about the current workspace"
                },
                {
                    "name": "analyze_change_impact",
                    "description": "Analyze the impact of changing a specific function or type",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "target": {"type": "string"}
                        },
                        "required": ["target"]
                    }
                },
                {
                    "name": "check_architecture_violations",
                    "description": "Check for architectural rule violations"
                }
            ]
        }
    });
    println!("{}", serde_json::to_string_pretty(&tools_response).unwrap());
    
    // 3. TOOL CALL REQUEST/RESPONSE
    println!("\n{}", "-".repeat(80));
    println!("\n📤 CLAUDE CODE SENDS (Call workspace_context):");
    let call_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "workspace_context",
            "arguments": {}
        }
    });
    println!("{}", serde_json::to_string_pretty(&call_request).unwrap());
    
    println!("\n📥 OUR SERVER RESPONDS (Workspace Analysis):");
    let call_response = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": "# Workspace Analysis\n\n- Functions: 5172\n- Types: 2035 \n- Dependencies: 16409\n- Crates: 11 (trading-core, trading-strategy, etc.)\n\nArchitecture: Clean layered design with no circular dependencies"
                }
            ]
        }
    });
    println!("{}", serde_json::to_string_pretty(&call_response).unwrap());
    
    // 4. IMPACT ANALYSIS TOOL CALL
    println!("\n{}", "-".repeat(80));
    println!("\n📤 CLAUDE CODE SENDS (Analyze Change Impact):");
    let impact_request = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "analyze_change_impact",
            "arguments": {
                "target": "PositionManager"
            }
        }
    });
    println!("{}", serde_json::to_string_pretty(&impact_request).unwrap());
    
    println!("\n📥 OUR SERVER RESPONDS (Impact Analysis):");
    let impact_response = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": "# Change Impact Analysis for 'PositionManager'\n\n## Matching Functions (3):\n- PositionManager::new in trading-strategy/src/position_manager.rs\n- PositionManager::update_position \n- PositionManager::close_position\n\n## Potential Impact (23 dependents):\n- trading-runtime calls PositionManager (high risk)\n- trading-backtest uses PositionManager (medium risk)\n\n⚠️ Critical path dependencies detected"
                }
            ]
        }
    });
    println!("{}", serde_json::to_string_pretty(&impact_response).unwrap());
    
    // 5. ARCHITECTURE VIOLATIONS TOOL CALL
    println!("\n{}", "-".repeat(80));
    println!("\n📤 CLAUDE CODE SENDS (Check Architecture):");
    let arch_request = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "check_architecture_violations",
            "arguments": {}
        }
    });
    println!("{}", serde_json::to_string_pretty(&arch_request).unwrap());
    
    println!("\n📥 OUR SERVER RESPONDS (Architecture Analysis):");
    let arch_response = json!({
        "jsonrpc": "2.0", 
        "id": 5,
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": "# Architecture Violations\n\n## Layer Analysis:\n✅ No upward dependencies (clean layers)\n✅ No circular dependencies\n\n## Suggestions (513 found):\n📦 Consider using prelude for deep imports:\n- trading_exchanges::crypto::futures::binance::BinanceFuturesActor\n- trading_strategy::execution::CryptoFuturesStrategyExecutor\n\nOverall: ✅ Clean architecture with good layer separation"
                }
            ]
        }
    });
    println!("{}", serde_json::to_string_pretty(&arch_response).unwrap());
    
    println!("\n{}", "=".repeat(80));
    
    println!("\n🎉 **MCP Protocol Summary:**");
    println!("   📡 Transport: JSON-RPC 2.0 over stdin/stdout");
    println!("   🔧 Methods: initialize, tools/list, tools/call");
    println!("   📋 Discovery: Claude Code discovers available tools");
    println!("   ⚡ Execution: Claude Code calls tools with arguments");
    println!("   📊 Response: Rich markdown analysis results");
    
    println!("\n🚀 **Claude Code Integration:**");
    println!("   1. Configure MCP server in Claude Code settings");
    println!("   2. Command: cargo run --bin mcp-server-stdio --workspace <path>");
    println!("   3. Claude Code gets deep workspace understanding!");
    
    println!("\n✅ **What Claude Code Can Now Ask:**");
    println!("   • 'What's the architecture of this trading system?'");
    println!("   • 'What happens if I change PositionManager?'"); 
    println!("   • 'Are there any layer violations?'");
    println!("   • 'Show me circular dependencies'");
    println!("   • 'What types are available across all crates?'");
    
    println!("\n🧠 **Claude Code's New Capabilities:**");
    println!("   • Deep codebase understanding");
    println!("   • Architecture-aware suggestions");
    println!("   • Impact analysis before changes");
    println!("   • Layer violation detection");
    println!("   • Cross-crate dependency insights");
}