use serde_json::json;

fn main() {
    println!("🔍 MCP Protocol Examples - Exact JSON-RPC Messages");
    println!("This shows what Claude Code sends/receives");
    
    println!("\\n{}", "=".repeat(80));
    
    // 1. INITIALIZE
    println!("\\n📤 CLAUDE CODE SENDS (Initialize):");
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });
    println!("{}", serde_json::to_string_pretty(&init_request).unwrap());
    
    println!("\\n📥 OUR SERVER RESPONDS (Initialize):");
    let init_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
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
    });
    println!("{}", serde_json::to_string_pretty(&init_response).unwrap());
    
    println!("\\n{}", "-".repeat(80));
    
    // 2. LIST TOOLS
    println!("\\n📤 CLAUDE CODE SENDS (List Tools):");
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });
    println!("{}", serde_json::to_string_pretty(&tools_request).unwrap());
    
    println!("\\n📥 OUR SERVER RESPONDS (List Tools):");
    let tools_response = json!({
        "jsonrpc": "2.0",
        "id": 2,
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
                }
            ]
        }
    });
    println!("{}", serde_json::to_string_pretty(&tools_response).unwrap());
    
    println!("\\n{}", "-".repeat(80));
    
    // 3. CALL WORKSPACE_CONTEXT TOOL
    println!("\\n📤 CLAUDE CODE SENDS (Get Workspace Context):");
    let context_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "workspace_context",
            "arguments": {}
        }
    });
    println!("{}", serde_json::to_string_pretty(&context_request).unwrap());
    
    println!("\\n📥 OUR SERVER RESPONDS (Workspace Context):");
    let context_response = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": "# Workspace Analysis\\n\\n{\\n  \\"analysis_timestamp\\": 1755718472,\\n  \\"summary\\": {\\n    \\"total_functions\\": 5172,\\n    \\"total_types\\": 2035,\\n    \\"total_dependencies\\": 16409,\\n    \\"modules\\": {\\n      \\"trading-core\\": 245,\\n      \\"trading-strategy\\": 892,\\n      \\"trading-runtime\\": 1234\\n    }\\n  },\\n  \\"top_functions\\": [\\n    {\\n      \\"name\\": \\"new\\",\\n      \\"crate\\": \\"trading-core\\",\\n      \\"visibility\\": \\"pub\\",\\n      \\"file\\": \\"/path/to/trading-core/src/position.rs\\",\\n      \\"line_start\\": 45\\n    }\\n  ],\\n  \\"top_types\\": [\\n    {\\n      \\"name\\": \\"Position\\",\\n      \\"type_kind\\": \\"struct\\",\\n      \\"crate\\": \\"trading-core\\",\\n      \\"visibility\\": \\"pub\\",\\n      \\"file\\": \\"/path/to/trading-core/src/position.rs\\",\\n      \\"line_start\\": 12\\n    }\\n  ]\\n}"
                }
            ]
        }
    });
    println!("{}", serde_json::to_string_pretty(&context_response).unwrap());
    
    println!("\\n{}", "-".repeat(80));
    
    // 4. CALL IMPACT ANALYSIS TOOL
    println!("\\n📤 CLAUDE CODE SENDS (Analyze Change Impact):");
    let impact_request = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "analyze_change_impact",
            "arguments": {
                "target": "PositionManager",
                "change_type": "signature"
            }
        }
    });
    println!("{}", serde_json::to_string_pretty(&impact_request).unwrap());
    
    println!("\\n📥 OUR SERVER RESPONDS (Impact Analysis):");
    let impact_response = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": "# Change Impact Analysis for 'PositionManager'\\n\\n## Matching Functions (3):\\n- `PositionManager::new` in /path/to/trading-strategy/src/position_manager.rs\\n- `PositionManager::update_position` in /path/to/trading-strategy/src/position_manager.rs\\n- `PositionManager::close_position` in /path/to/trading-strategy/src/position_manager.rs\\n\\n## Potential Impact (23 dependents):\\n- trading-runtime calls PositionManager::new (line 45)\\n- trading-backtest calls PositionManager::update_position (line 123)\\n- execution-engine calls PositionManager::close_position (line 67)\\n\\n⚠️ **High Risk**: Changes will affect critical execution paths"
                }
            ]
        }
    });
    println!("{}", serde_json::to_string_pretty(&impact_response).unwrap());
    
    println!("\\n{}", "-".repeat(80));
    
    // 5. CALL ARCHITECTURE VIOLATIONS TOOL
    println!("\\n📤 CLAUDE CODE SENDS (Check Architecture Violations):");
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
    
    println!("\\n📥 OUR SERVER RESPONDS (Architecture Violations):");
    let arch_response = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": "# Architecture Violations\\n\\n## 🏗️ Layer Dependency Analysis\\n\\n### 🚨 Layer Violations (0):\\n✅ No upward dependencies detected\\n\\n### 🔀 Cross-Layer Jumps (0):\\n✅ All layer jumps use proper abstractions\\n\\n### 📦 Consider Prelude Usage (513):\\n📦 **Deep Import**: `trading_exchanges::crypto::futures::binance::BinanceFuturesHistoricalCandleActor` (depth 4) - consider using prelude\\n📦 **Deep Import**: `trading_strategy::execution::crypto_futures_strategy_executor::CryptoFuturesStrategyExecutor` (depth 3) - consider using prelude\\n\\n## 🔄 Circular Dependencies\\n✅ No circular dependencies detected"
                }
            ]
        }
    });
    println!("{}", serde_json::to_string_pretty(&arch_response).unwrap());
    
    println!("\\n{}", "=".repeat(80));
    
    println!("\\n🎉 **MCP Protocol Summary:**");
    println!("   📡 **Transport**: JSON-RPC 2.0 over stdin/stdout");
    println!("   🔧 **Methods**: initialize, tools/list, tools/call");
    println!("   📋 **Tool Discovery**: Claude Code asks what tools are available");
    println!("   ⚡ **Tool Execution**: Claude Code calls tools with structured arguments");
    println!("   📊 **Rich Responses**: Formatted markdown text with analysis results");
    
    println!("\\n🚀 **For Claude Code Integration:**");
    println!("   1. Claude Code runs: `cargo run --bin mcp-server-stdio --workspace <path>`");
    println!("   2. Claude Code sends JSON-RPC requests to stdin");
    println!("   3. Our server responds with JSON-RPC responses on stdout");
    println!("   4. Claude Code gets deep workspace understanding! 🧠");
    
    println!("\\n✅ **What Claude Code Can Now Do:**");
    println!("   • \\"What's the architecture of this trading system?\\"");
    println!("   • \\"What happens if I change the PositionManager interface?\\"");
    println!("   • \\"Are there any layer violations in my code?\\"");
    println!("   • \\"Show me circular dependencies\\"");
    println!("   • \\"What types and functions are available across all crates?\\"");
}