#!/bin/bash

echo "🧪 Simple MCP Server Test"
echo "========================="

WORKSPACE_PATH="/Users/greg/dev/git/trading-backend-poc"

echo "📋 Test 1: Initialize MCP Server"
echo "--------------------------------"
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' | \
  timeout 30s cargo run --bin mcp-server-stdio -- --workspace "$WORKSPACE_PATH" 2>&1

echo ""
echo "📋 Test 2: Direct Architecture Check (with timeout)"
echo "--------------------------------------------------"
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"check_architecture_violations","arguments":{}}}' | \
  timeout 30s cargo run --bin mcp-server-stdio -- --workspace "$WORKSPACE_PATH" 2>&1

echo ""
echo "✅ Simple test complete!"