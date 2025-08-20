#!/bin/bash

echo "🔍 Testing MCP Server with Full Responses"
echo "========================================"

# Start the server and capture both input and output
{
    echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
    echo '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
    echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"workspace_context","arguments":{}}}'
} | cargo run --bin mcp-server-stdio -- --workspace /Users/greg/dev/git/trading-backend-poc 2>/dev/null