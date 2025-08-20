#!/bin/bash

echo "🔍 Debug Function Call Detection"
echo "================================"

WORKSPACE_PATH="/Users/greg/dev/git/trading-backend-poc"

echo "📋 Checking if function calls are being detected from trading-core utils.rs"
echo "------------------------------------------------------------------------"

# First check that the file exists
echo "📁 File check:"
ls -la "$WORKSPACE_PATH/trading-core/src/utils.rs"

echo ""
echo "📋 Content around line 114:"
sed -n '110,120p' "$WORKSPACE_PATH/trading-core/src/utils.rs"

echo ""
echo "📋 Search for the violation call in analyzed data:"
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"workspace_context","arguments":{}}}' | \
  cargo run --bin mcp-server-stdio -- --workspace "$WORKSPACE_PATH" 2>/dev/null | \
  jq -r '.result.content[0].text' | grep -i "trading_exchanges" | head -5

echo ""
echo "🔍 Debug complete!"