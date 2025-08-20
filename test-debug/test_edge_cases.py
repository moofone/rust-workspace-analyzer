#!/usr/bin/env python3
"""
Test edge cases and error handling for Rust Workspace Analyzer
"""

import json
import subprocess
import tempfile
import os
import sys
from pathlib import Path

class EdgeCaseTester:
    def __init__(self, binary_path: str):
        self.binary_path = binary_path
        
    def test_invalid_workspace(self):
        """Test behavior with invalid workspace path"""
        print("🔄 Testing invalid workspace path...")
        
        result = subprocess.run(
            [self.binary_path, "--workspace", "/nonexistent/path"],
            capture_output=True,
            text=True,
            timeout=10
        )
        
        if result.returncode == 0:
            print("❌ Should fail with invalid workspace path")
            return False
            
        if "does not exist" not in result.stderr:
            print(f"❌ Expected error message about non-existent path, got: {result.stderr}")
            return False
            
        print("✅ Invalid workspace test passed")
        return True
        
    def test_empty_workspace(self):
        """Test behavior with empty workspace"""
        print("🔄 Testing empty workspace...")
        
        with tempfile.TemporaryDirectory() as temp_dir:
            process = subprocess.Popen(
                [self.binary_path, "--workspace", temp_dir],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True
            )
            
            # Send initialize request
            init_request = {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {}
            }
            
            request_str = json.dumps(init_request) + "\n"
            stdout, stderr = process.communicate(input=request_str, timeout=10)
            
            if not stdout:
                print("❌ No response from empty workspace")
                return False
                
            try:
                response = json.loads(stdout.strip())
                if "error" in response:
                    print(f"❌ Unexpected error with empty workspace: {response['error']}")
                    return False
            except json.JSONDecodeError:
                print(f"❌ Invalid JSON response: {stdout}")
                return False
                
        print("✅ Empty workspace test passed")
        return True
        
    def test_malformed_json_requests(self):
        """Test handling of malformed JSON requests"""
        print("🔄 Testing malformed JSON requests...")
        
        process = subprocess.Popen(
            [self.binary_path, "--workspace", "."],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )
        
        # Test various malformed requests
        malformed_requests = [
            '{"invalid": json}',  # Invalid JSON syntax
            '{"jsonrpc": "1.0"}',  # Wrong JSON-RPC version
            '{"jsonrpc": "2.0"}',  # Missing method
            '{"jsonrpc": "2.0", "method": ""}',  # Empty method
            '',  # Empty request
            'not json at all',
        ]
        
        all_passed = True
        for i, bad_request in enumerate(malformed_requests):
            process.stdin.write(bad_request + "\n")
            process.stdin.flush()
            
            try:
                response_line = process.stdout.readline()
                if response_line:
                    response = json.loads(response_line.strip())
                    if "error" not in response:
                        print(f"❌ Expected error for malformed request {i+1}")
                        all_passed = False
                else:
                    print(f"❌ No response for malformed request {i+1}")
                    all_passed = False
            except json.JSONDecodeError:
                print(f"❌ Server returned invalid JSON for malformed request {i+1}")
                all_passed = False
                
        process.terminate()
        process.wait()
        
        if all_passed:
            print("✅ Malformed JSON requests test passed")
        return all_passed
        
    def test_large_workspace(self):
        """Test performance with a large workspace simulation"""
        print("🔄 Testing large workspace simulation...")
        
        with tempfile.TemporaryDirectory() as temp_dir:
            # Create a simulated large workspace
            src_dir = Path(temp_dir) / "src"
            src_dir.mkdir()
            
            # Create many files to simulate large workspace
            for i in range(50):  # Reasonable test size
                file_path = src_dir / f"module_{i}.rs"
                with open(file_path, 'w') as f:
                    f.write(f"""
// Module {i}
pub struct Struct{i} {{
    field: i32,
}}

impl Struct{i} {{
    pub fn new() -> Self {{
        Self {{ field: {i} }}
    }}
    
    pub fn process(&self) -> i32 {{
        self.field * 2
    }}
}}

pub fn function_{i}() -> i32 {{
    {i}
}}

#[cfg(test)]
mod tests {{
    use super::*;
    
    #[test]
    fn test_struct_{i}() {{
        let s = Struct{i}::new();
        assert_eq!(s.process(), {i * 2});
    }}
}}
""")
            
            # Create Cargo.toml
            cargo_toml = Path(temp_dir) / "Cargo.toml"
            with open(cargo_toml, 'w') as f:
                f.write("""
[package]
name = "test-workspace"
version = "0.1.0"
edition = "2021"
""")
            
            # Test with this large workspace
            import time
            start_time = time.time()
            
            process = subprocess.Popen(
                [self.binary_path, "--workspace", temp_dir],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True
            )
            
            # Send workspace context request
            request = {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {
                    "name": "workspace_context",
                    "arguments": {}
                }
            }
            
            request_str = json.dumps(request) + "\n"
            stdout, stderr = process.communicate(input=request_str, timeout=30)
            
            elapsed = time.time() - start_time
            
            if elapsed > 25:  # Should complete within 25 seconds
                print(f"❌ Large workspace test too slow: {elapsed:.2f}s")
                return False
                
            if not stdout:
                print("❌ No response for large workspace")
                return False
                
            try:
                response = json.loads(stdout.strip())
                if "error" in response:
                    print(f"❌ Error processing large workspace: {response['error']}")
                    return False
            except json.JSONDecodeError:
                print(f"❌ Invalid response for large workspace: {stdout}")
                return False
                
        print(f"✅ Large workspace test passed ({elapsed:.2f}s)")
        return True
        
    def test_concurrent_requests(self):
        """Test handling of concurrent requests"""
        print("🔄 Testing concurrent request handling...")
        
        process = subprocess.Popen(
            [self.binary_path, "--workspace", "."],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )
        
        # Send multiple requests rapidly
        requests = []
        for i in range(5):
            request = {
                "jsonrpc": "2.0",
                "id": i + 1,
                "method": "tools/list",
                "params": {}
            }
            requests.append(json.dumps(request) + "\n")
            
        # Send all requests at once
        for request_str in requests:
            process.stdin.write(request_str)
        process.stdin.flush()
        
        # Read responses
        responses = []
        for _ in range(len(requests)):
            response_line = process.stdout.readline()
            if response_line:
                try:
                    response = json.loads(response_line.strip())
                    responses.append(response)
                except json.JSONDecodeError:
                    print(f"❌ Invalid JSON in concurrent response: {response_line}")
                    process.terminate()
                    return False
                    
        process.terminate()
        process.wait()
        
        if len(responses) != len(requests):
            print(f"❌ Expected {len(requests)} responses, got {len(responses)}")
            return False
            
        # Check all responses are valid
        for i, response in enumerate(responses):
            if "error" in response:
                print(f"❌ Error in concurrent response {i+1}: {response['error']}")
                return False
            if response.get("id") != i + 1:
                print(f"❌ Wrong ID in concurrent response {i+1}")
                return False
                
        print("✅ Concurrent requests test passed")
        return True
        
    def run_all_tests(self):
        """Run all edge case tests"""
        print("🧪 Running Edge Case and Error Handling Tests...")
        print("=" * 50)
        
        tests = [
            self.test_invalid_workspace,
            self.test_empty_workspace,
            self.test_malformed_json_requests,
            self.test_large_workspace,
            self.test_concurrent_requests,
        ]
        
        results = []
        for test in tests:
            try:
                results.append(test())
            except Exception as e:
                print(f"❌ Test failed with exception: {e}")
                results.append(False)
                
        return all(results)

def main():
    binary_path = "./target/release/mcp-server-stdio"
    
    if not os.path.exists(binary_path):
        print(f"❌ Binary not found: {binary_path}")
        print("Please run: cargo build --release --bin mcp-server-stdio")
        return 1
        
    tester = EdgeCaseTester(binary_path)
    success = tester.run_all_tests()
    
    print("\n" + "=" * 50)
    if success:
        print("🎉 All edge case tests passed!")
        print("✅ Error handling is robust")
    else:
        print("❌ Some edge case tests failed")
        print("⚠️  Please review error handling")
        
    return 0 if success else 1

if __name__ == "__main__":
    sys.exit(main())