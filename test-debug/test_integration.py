#!/usr/bin/env python3
"""
End-to-end integration test for Claude Code MCP integration
"""

import json
import subprocess
import time
import sys

def test_full_workflow():
    """Test the complete workflow of MCP tools"""
    print("🧪 Testing End-to-End Claude Code Integration...")
    print("=" * 50)
    
    binary_path = "./target/release/mcp-server-stdio"
    workspace_path = "/Users/greg/dev/git/trading-backend-poc"
    
    # Start server
    process = subprocess.Popen(
        [binary_path, "--workspace", workspace_path],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )
    
    try:
        # Test 1: Initialize
        print("🔄 Testing initialization...")
        init_request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }
        
        response = send_request(process, init_request)
        if not validate_response(response, 1):
            return False
        print("✅ Initialization successful")
        
        # Test 2: List tools
        print("🔄 Testing tool discovery...")
        tools_request = {
            "jsonrpc": "2.0", 
            "id": 2,
            "method": "tools/list",
            "params": {}
        }
        
        response = send_request(process, tools_request)
        if not validate_response(response, 2):
            return False
            
        tools = response["result"]["tools"]
        tool_names = [tool["name"] for tool in tools]
        expected_tools = ["workspace_context", "analyze_test_coverage", "check_architecture_violations"]
        
        for tool in expected_tools:
            if tool not in tool_names:
                print(f"❌ Missing expected tool: {tool}")
                return False
        print(f"✅ Tool discovery successful ({len(tools)} tools)")
        
        # Test 3: Workspace context
        print("🔄 Testing workspace context...")
        context_request = {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "workspace_context",
                "arguments": {}
            }
        }
        
        response = send_request(process, context_request, timeout=30)
        if not validate_response(response, 3):
            return False
            
        content = response["result"]["content"][0]["text"]
        if "analysis_timestamp" not in content or "summary" not in content:
            print("❌ Workspace context missing expected content")
            return False
        print("✅ Workspace context successful")
        
        # Test 4: Test coverage analysis
        print("🔄 Testing test coverage analysis...")
        coverage_request = {
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "analyze_test_coverage",
                "arguments": {}
            }
        }
        
        response = send_request(process, coverage_request, timeout=30)
        if not validate_response(response, 4):
            return False
            
        content = response["result"]["content"][0]["text"]
        if "Heavily Used & Untested" not in content:
            print("❌ Test coverage analysis missing expected content")
            return False
        print("✅ Test coverage analysis successful")
        
        # Test 5: Architecture violations
        print("🔄 Testing architecture violation detection...")
        arch_request = {
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "check_architecture_violations",
                "arguments": {}
            }
        }
        
        response = send_request(process, arch_request, timeout=20)
        if not validate_response(response, 5):
            return False
            
        content = response["result"]["content"][0]["text"]
        if "Architecture Violations" not in content:
            print("❌ Architecture analysis missing expected content")
            return False
        print("✅ Architecture violation detection successful")
        
        print("\n🎉 All integration tests passed!")
        print("✅ Ready for Claude Code production use")
        return True
        
    finally:
        process.terminate()
        process.wait()

def send_request(process, request, timeout=10):
    """Send a request and get response"""
    request_str = json.dumps(request) + "\n"
    process.stdin.write(request_str)
    process.stdin.flush()
    
    # Read response with timeout
    start_time = time.time()
    while time.time() - start_time < timeout:
        if process.poll() is not None:
            print("❌ Process terminated unexpectedly")
            return None
            
        try:
            response_line = process.stdout.readline()
            if response_line:
                return json.loads(response_line.strip())
        except json.JSONDecodeError as e:
            print(f"❌ Failed to parse response: {e}")
            return None
        except Exception as e:
            print(f"❌ Error reading response: {e}")
            return None
            
        time.sleep(0.1)
    
    print(f"❌ Request timed out after {timeout}s")
    return None

def validate_response(response, expected_id):
    """Validate response structure"""
    if not response:
        print("❌ No response received")
        return False
        
    if "error" in response:
        print(f"❌ Error in response: {response['error']}")
        return False
        
    if response.get("id") != expected_id:
        print(f"❌ Wrong response ID: expected {expected_id}, got {response.get('id')}")
        return False
        
    if "result" not in response:
        print("❌ Missing result in response")
        return False
        
    return True

def main():
    try:
        success = test_full_workflow()
        return 0 if success else 1
    except KeyboardInterrupt:
        print("\n❌ Test interrupted")
        return 1
    except Exception as e:
        print(f"❌ Test failed with exception: {e}")
        return 1

if __name__ == "__main__":
    sys.exit(main())