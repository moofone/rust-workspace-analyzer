#!/usr/bin/env python3
"""
Test MCP Protocol Compliance for Rust Workspace Analyzer
Validates JSON-RPC 2.0 and MCP protocol adherence
"""

import json
import subprocess
import time
import sys
from typing import Dict, Any, Optional

class MCPTester:
    def __init__(self, binary_path: str, workspace_path: str = "."):
        self.binary_path = binary_path
        self.workspace_path = workspace_path
        self.process = None
        
    def start_server(self):
        """Start the MCP server process"""
        self.process = subprocess.Popen(
            [self.binary_path, "--workspace", self.workspace_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1
        )
        time.sleep(0.5)  # Give server time to initialize
        
    def stop_server(self):
        """Stop the MCP server process"""
        if self.process:
            self.process.terminate()
            self.process.wait()
            
    def send_request(self, request: Dict[str, Any]) -> Optional[Dict[str, Any]]:
        """Send a JSON-RPC request and get response"""
        if not self.process:
            return None
            
        request_str = json.dumps(request) + "\n"
        self.process.stdin.write(request_str)
        self.process.stdin.flush()
        
        # Read response
        response_line = self.process.stdout.readline()
        if response_line:
            try:
                return json.loads(response_line.strip())
            except json.JSONDecodeError as e:
                print(f"Failed to parse response: {e}")
                print(f"Raw response: {response_line}")
                return None
        return None
        
    def test_initialize(self) -> bool:
        """Test MCP initialize protocol"""
        print("🔄 Testing MCP initialize...")
        
        request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        }
        
        response = self.send_request(request)
        if not response:
            print("❌ No response to initialize")
            return False
            
        # Check JSON-RPC 2.0 compliance
        if response.get("jsonrpc") != "2.0":
            print(f"❌ Invalid jsonrpc version: {response.get('jsonrpc')}")
            return False
            
        if response.get("id") != 1:
            print(f"❌ ID mismatch: expected 1, got {response.get('id')}")
            return False
            
        if "error" in response:
            print(f"❌ Initialize error: {response['error']}")
            return False
            
        result = response.get("result", {})
        if "protocolVersion" not in result:
            print("❌ Missing protocolVersion in response")
            return False
            
        if "capabilities" not in result:
            print("❌ Missing capabilities in response")
            return False
            
        if "serverInfo" not in result:
            print("❌ Missing serverInfo in response")
            return False
            
        server_info = result["serverInfo"]
        if server_info.get("name") != "rust-workspace-analyzer":
            print(f"❌ Unexpected server name: {server_info.get('name')}")
            return False
            
        print("✅ Initialize test passed")
        return True
        
    def test_tools_list(self) -> bool:
        """Test tools/list method"""
        print("🔄 Testing tools/list...")
        
        request = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }
        
        response = self.send_request(request)
        if not response:
            print("❌ No response to tools/list")
            return False
            
        if "error" in response:
            print(f"❌ tools/list error: {response['error']}")
            return False
            
        result = response.get("result", {})
        tools = result.get("tools", [])
        
        if not tools:
            print("❌ No tools returned")
            return False
            
        # Check for required tools
        tool_names = [tool.get("name") for tool in tools]
        required_tools = [
            "workspace_context",
            "analyze_test_coverage", 
            "check_architecture_violations",
            "find_dependency_issues"
        ]
        
        for required_tool in required_tools:
            if required_tool not in tool_names:
                print(f"❌ Missing required tool: {required_tool}")
                return False
                
        # Validate tool schema
        for tool in tools:
            if "name" not in tool:
                print("❌ Tool missing name")
                return False
            if "description" not in tool:
                print(f"❌ Tool {tool['name']} missing description")
                return False
            if "inputSchema" not in tool:
                print(f"❌ Tool {tool['name']} missing inputSchema")
                return False
                
        print(f"✅ tools/list test passed ({len(tools)} tools)")
        return True
        
    def test_tool_call(self) -> bool:
        """Test tools/call method"""
        print("🔄 Testing tools/call...")
        
        request = {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "workspace_context",
                "arguments": {}
            }
        }
        
        response = self.send_request(request)
        if not response:
            print("❌ No response to tools/call")
            return False
            
        if "error" in response:
            print(f"❌ tools/call error: {response['error']}")
            return False
            
        result = response.get("result", {})
        if "content" not in result:
            print("❌ Missing content in tool response")
            return False
            
        content = result["content"]
        if not isinstance(content, list):
            print("❌ Content should be an array")
            return False
            
        if not content:
            print("❌ Empty content array")
            return False
            
        # Check first content item
        first_content = content[0]
        if "type" not in first_content:
            print("❌ Content item missing type")
            return False
            
        if first_content["type"] != "text":
            print(f"❌ Expected text content, got {first_content['type']}")
            return False
            
        if "text" not in first_content:
            print("❌ Text content item missing text field")
            return False
            
        print("✅ tools/call test passed")
        return True
        
    def test_error_handling(self) -> bool:
        """Test error handling"""
        print("🔄 Testing error handling...")
        
        # Test invalid method
        request = {
            "jsonrpc": "2.0",
            "id": 4,
            "method": "invalid_method",
            "params": {}
        }
        
        response = self.send_request(request)
        if not response:
            print("❌ No response to invalid method")
            return False
            
        if "error" not in response:
            print("❌ Expected error for invalid method")
            return False
            
        error = response["error"]
        if error.get("code") != -32601:
            print(f"❌ Expected error code -32601, got {error.get('code')}")
            return False
            
        # Test invalid JSON
        invalid_request = '{"jsonrpc": "2.0", "id": 5, "method"'  # Invalid JSON
        self.process.stdin.write(invalid_request + "\n")
        self.process.stdin.flush()
        
        response_line = self.process.stdout.readline()
        if response_line:
            try:
                response = json.loads(response_line.strip())
                if "error" not in response:
                    print("❌ Expected error for invalid JSON")
                    return False
                    
                error = response["error"]
                if error.get("code") != -32700:
                    print(f"❌ Expected error code -32700, got {error.get('code')}")
                    return False
            except:
                print("❌ Invalid response to invalid JSON")
                return False
        
        print("✅ Error handling test passed")
        return True
        
    def run_compliance_tests(self) -> bool:
        """Run all compliance tests"""
        print("🧪 Running MCP Protocol Compliance Tests...")
        print("=" * 50)
        
        try:
            self.start_server()
            
            tests = [
                self.test_initialize,
                self.test_tools_list,
                self.test_tool_call,
                self.test_error_handling
            ]
            
            results = []
            for test in tests:
                results.append(test())
                
            return all(results)
            
        finally:
            self.stop_server()

def main():
    binary_path = "./target/release/mcp-server-stdio"
    workspace_path = "/Users/greg/dev/git/trading-backend-poc"
    
    tester = MCPTester(binary_path, workspace_path)
    
    success = tester.run_compliance_tests()
    
    print("\n" + "=" * 50)
    if success:
        print("🎉 All MCP compliance tests passed!")
        print("✅ Ready for Claude Code integration")
    else:
        print("❌ Some tests failed")
        print("⚠️  Please fix issues before Claude Code integration")
        
    return 0 if success else 1

if __name__ == "__main__":
    sys.exit(main())