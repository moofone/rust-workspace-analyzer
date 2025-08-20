#!/bin/bash

# Test architecture violation analysis for trading-backend-poc workspace

echo "🏗️ Testing Architecture Violation Analysis..."
echo "============================================="

# Run the check_architecture_violations MCP function
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"check_architecture_violations","arguments":{}}}' | \
  cargo run --bin mcp-server-stdio -- --workspace /Users/greg/dev/git/trading-backend-poc \
  2>/dev/null | jq -r '.result.content[0].text'

echo ""
echo "✅ Architecture analysis complete!"