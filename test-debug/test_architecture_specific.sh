#!/bin/bash

# Test architecture violations for specific functions/modules

echo "🎯 Targeted Architecture Testing"
echo "================================"

WORKSPACE_PATH="/Users/greg/dev/git/trading-backend-poc"

# Test specific function impact analysis
echo "📋 Test 1: Analyze Impact of 'execute_trade' function"
echo "----------------------------------------------------"
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"analyze_change_impact","arguments":{"target":"execute_trade","change_type":"signature"}}}' | \
  cargo run --bin mcp-server-stdio -- --workspace "$WORKSPACE_PATH" \
  2>/dev/null | tail -n +4 | jq -r '.result.content[0].text'

echo ""
echo "📋 Test 2: Analyze Impact of 'Strategy' type"
echo "--------------------------------------------"
echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"analyze_change_impact","arguments":{"target":"Strategy","change_type":"rename"}}}' | \
  cargo run --bin mcp-server-stdio -- --workspace "$WORKSPACE_PATH" \
  2>/dev/null | tail -n +4 | jq -r '.result.content[0].text'

echo ""
echo "📋 Test 3: Validate Proposed Change"
echo "----------------------------------"
echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"validate_proposed_change","arguments":{"description":"Move price calculation logic from trading-runtime to trading-core"}}}' | \
  cargo run --bin mcp-server-stdio -- --workspace "$WORKSPACE_PATH" \
  2>/dev/null | tail -n +4 | jq -r '.result.content[0].text'

echo ""
echo "📋 Test 4: Workspace Context Overview"
echo "------------------------------------"
echo '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"workspace_context","arguments":{}}}' | \
  cargo run --bin mcp-server-stdio -- --workspace "$WORKSPACE_PATH" \
  2>/dev/null | tail -n +4 | jq -r '.result.content[0].text'

echo ""
echo "✅ Targeted architecture tests complete!"