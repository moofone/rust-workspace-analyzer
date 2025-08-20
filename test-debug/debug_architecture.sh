#!/bin/bash

echo "🔍 Debugging Architecture Analysis"
echo "=================================="

WORKSPACE_PATH="/Users/greg/dev/git/trading-backend-poc"

# Test 1: Check if workspace exists
echo "📁 Test 1: Workspace Path Check"
echo "------------------------------"
if [ -d "$WORKSPACE_PATH" ]; then
    echo "✅ Workspace exists: $WORKSPACE_PATH"
    ls -la "$WORKSPACE_PATH" | head -5
else
    echo "❌ Workspace does not exist: $WORKSPACE_PATH"
    exit 1
fi

echo ""
echo "📋 Test 2: Raw MCP Server Output (with stderr)"
echo "----------------------------------------------"
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"check_architecture_violations","arguments":{}}}' | \
  cargo run --bin mcp-server-stdio -- --workspace "$WORKSPACE_PATH"

echo ""
echo "📋 Test 3: Initialize First"
echo "---------------------------"
echo '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"debug","version":"1.0"}}}' | \
  cargo run --bin mcp-server-stdio -- --workspace "$WORKSPACE_PATH" 2>/dev/null

echo ""
echo "📋 Test 4: Build Check"
echo "----------------------"
cargo build --bin mcp-server-stdio

echo "🔍 Debug complete!"