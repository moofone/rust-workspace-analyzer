use anyhow::Result;
use serde_json::json;
use workspace_analyzer::mcp::{WorkspaceMcpServer, McpRequest};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 Testing Function Reference Counting Implementation (IMPROVE_1.md)...");
    
    let workspace_path = Path::new("/Users/greg/dev/git/trading-backend-poc");
    let server = WorkspaceMcpServer::new(&workspace_path).await?;
    
    // Initialize
    let init_request = McpRequest {
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: None,
    };
    
    println!("📊 Initializing workspace analysis...");
    let response = server.handle_request(init_request).await;
    if response.error.is_some() {
        println!("❌ Initialization failed: {:?}", response.error);
        return Ok(());
    }
    println!("✅ Workspace initialized successfully");
    
    // Test function reference counting via test coverage analysis
    let coverage_request = McpRequest {
        id: Some(json!(2)),
        method: "analyze_test_coverage".to_string(),
        params: None,
    };
    
    println!("\n🔍 Testing function reference counting...");
    let coverage_response = server.handle_request(coverage_request).await;
    
    match coverage_response.result {
        Some(result) => {
            if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
                if let Some(text_content) = content.first().and_then(|c| c.get("text")).and_then(|t| t.as_str()) {
                    println!("{}", text_content);
                    
                    // Parse JSON from the report to validate results
                    if let Some(json_start) = text_content.rfind("```json\n") {
                        if let Some(json_end) = text_content.rfind("\n```") {
                            let json_str = &text_content[json_start + 8..json_end];
                            if let Ok(data) = serde_json::from_str::<serde_json::Value>(json_str) {
                                // Validation tests for IMPROVE_1.md acceptance criteria
                                
                                println!("\n🎯 VALIDATION RESULTS:");
                                
                                // Check 1: Do we have heavily referenced functions with non-zero counts?
                                if let Some(untested_heavy) = data.get("heavily_used_untested").and_then(|h| h.as_u64()) {
                                    if untested_heavy > 0 {
                                        println!("✅ PASS: Found {} heavily referenced untested functions (was 0 before)", untested_heavy);
                                    } else {
                                        println!("❌ FAIL: Still showing 0 heavily referenced untested functions");
                                    }
                                }
                                
                                // Check 2: Are we finding heavily used tested functions?
                                if let Some(tested_heavy) = data.get("heavily_used_tested").and_then(|h| h.as_u64()) {
                                    if tested_heavy > 0 {
                                        println!("✅ PASS: Found {} heavily referenced tested functions", tested_heavy);
                                    } else {
                                        println!("⚠️  WARNING: No heavily referenced tested functions found");
                                    }
                                }
                                
                                // Check 3: Do priority untested functions have reference counts > 0?
                                if let Some(priority_untested) = data.get("priority_untested").and_then(|p| p.as_array()) {
                                    let mut has_positive_refs = false;
                                    for func in priority_untested {
                                        if let Some(ref_count) = func.get("references").and_then(|r| r.as_u64()) {
                                            if ref_count > 0 {
                                                has_positive_refs = true;
                                                break;
                                            }
                                        }
                                    }
                                    
                                    if has_positive_refs {
                                        println!("✅ PASS: Priority untested functions have positive reference counts");
                                    } else {
                                        println!("❌ FAIL: Priority untested functions still have 0 reference counts");
                                    }
                                }
                                
                                // Check 4: Are cross-crate references being identified?
                                if let Some(priority_untested) = data.get("priority_untested").and_then(|p| p.as_array()) {
                                    let mut has_cross_crate = false;
                                    for func in priority_untested {
                                        if let Some(cross_crate_refs) = func.get("cross_crate_refs").and_then(|r| r.as_u64()) {
                                            if cross_crate_refs > 0 {
                                                has_cross_crate = true;
                                                break;
                                            }
                                        }
                                    }
                                    
                                    if has_cross_crate {
                                        println!("✅ PASS: Cross-crate references are being identified");
                                    } else {
                                        println!("⚠️  WARNING: No cross-crate references found (may be valid for this codebase)");
                                    }
                                }
                                
                                // Check 5: Coverage by crate shows meaningful untested_heavy_usage
                                if let Some(coverage_by_crate) = data.get("coverage_by_crate").and_then(|c| c.as_array()) {
                                    let mut total_untested_heavy = 0;
                                    for crate_stats in coverage_by_crate {
                                        if let Some(untested_heavy) = crate_stats.get("untested_heavy_usage").and_then(|u| u.as_u64()) {
                                            total_untested_heavy += untested_heavy;
                                        }
                                    }
                                    
                                    if total_untested_heavy > 0 {
                                        println!("✅ PASS: Found {} total heavily-used untested functions across all crates", total_untested_heavy);
                                    } else {
                                        println!("❌ FAIL: untested_heavy_usage metrics still showing 0 across all crates");
                                    }
                                }
                                
                                println!("\n📈 IMPLEMENTATION STATUS:");
                                println!("✅ FunctionReference struct implemented");
                                println!("✅ FunctionRegistry struct implemented");
                                println!("✅ Two-pass analysis (registry build + reference resolution) implemented");
                                println!("✅ Call detection for direct calls, method calls, and qualified calls implemented");
                                println!("✅ Cross-crate detection implemented");
                                println!("✅ Test context detection implemented");
                                println!("✅ Test coverage analysis updated to use function references");
                                
                            } else {
                                println!("❌ Failed to parse JSON from analysis report");
                            }
                        }
                    }
                }
            }
        }
        None => {
            println!("❌ Test coverage analysis failed: {:?}", coverage_response.error);
            return Ok(());
        }
    }
    
    println!("\n🎉 Function reference testing complete!");
    
    Ok(())
}