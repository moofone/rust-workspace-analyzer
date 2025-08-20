#!/bin/bash

# Comprehensive architecture violation testing with multiple rule sets

echo "🏗️ Comprehensive Architecture Testing"
echo "====================================="

WORKSPACE_PATH="/Users/greg/dev/git/trading-backend-poc"

# Test 1: Full architecture violation check
echo "📋 Test 1: Full Architecture Analysis"
echo "-------------------------------------"
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"check_architecture_violations","arguments":{}}}' | \
  cargo run --bin mcp-server-stdio -- --workspace "$WORKSPACE_PATH" \
  2>/dev/null | jq -r '.result.content[0].text'

echo ""
echo "📋 Test 2: Layer Violations Only"  
echo "--------------------------------"
echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"check_architecture_violations","arguments":{"rules":["layer_violations"]}}}' | \
  cargo run --bin mcp-server-stdio -- --workspace "$WORKSPACE_PATH" \
  2>/dev/null | jq -r '.result.content[0].text'

echo ""
echo "📋 Test 3: Circular Dependencies Only"
echo "-------------------------------------"
echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"check_architecture_violations","arguments":{"rules":["circular_deps"]}}}' | \
  cargo run --bin mcp-server-stdio -- --workspace "$WORKSPACE_PATH" \
  2>/dev/null | jq -r '.result.content[0].text'

echo ""
echo "📋 Test 4: Dependency Issues Analysis"
echo "------------------------------------"
echo '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"find_dependency_issues","arguments":{}}}' | \
  cargo run --bin mcp-server-stdio -- --workspace "$WORKSPACE_PATH" \
  2>/dev/null | jq -r '.result.content[0].text'

echo ""
echo "✅ All architecture tests complete!"