echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"analyze_test_coverage","arguments":{}}}' | cargo run --bin mcp-server-stdio -- --workspace /Users/greg/dev/git/trading-backend-poc
   2>/dev/null | tail -n +4 | jq -r '.result.content[0].text'
