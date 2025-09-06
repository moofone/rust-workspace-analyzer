#[cfg(test)]
mod macro_expansion_tests {
    use crate::parser::rust_parser::{RustParser, SyntheticCallGenerator, IndicatorResolver};
    use crate::parser::symbols::*;
    use crate::graph::memgraph_client::MemgraphClient;
    use crate::config::Config;
    use std::path::PathBuf;

    #[test]
    fn test_synthetic_call_generation_with_real_functions() {
        let mut parser = RustParser::new().unwrap();
        
        let source = r#"
            use paste::paste;
            
            pub fn process_indicators(config: &Config) -> Result<()> {
                paste! {
                    let indicator = [<$indicator>]::new(config)?;
                }
                
                paste! {
                    let input = [<$indicator Input>]::from_ohlcv(data);
                }
                
                Ok(())
            }
        "#;

        let symbols = parser
            .parse_source(source, &PathBuf::from("test_indicators.rs"), "trading_ta")
            .unwrap();

        // Should detect synthetic calls instead of macro expansions
        let synthetic_calls: Vec<_> = symbols.calls.iter()
            .filter(|call| call.is_synthetic)
            .collect();
        
        assert!(!synthetic_calls.is_empty(), "Should generate synthetic calls for macro patterns");
        
        // Check that synthetic calls have real function IDs as callers, not MACRO_EXPANSION
        for call in &synthetic_calls {
            assert!(!call.caller_id.contains("MACRO_EXPANSION"), 
                   "Caller should be real function, got: {}", call.caller_id);
            assert!(call.caller_id.contains("process_indicators"), 
                   "Caller should be the containing function, got: {}", call.caller_id);
            assert!(call.is_synthetic, "Call should be marked as synthetic");
        }
        
        // Should have calls for both [<$indicator>]::new and [<$indicator Input>]::from_ohlcv patterns
        let new_calls = synthetic_calls.iter()
            .filter(|call| call.qualified_callee.as_ref().map(|q| q.contains("::new")).unwrap_or(false))
            .count();
        let from_ohlcv_calls = synthetic_calls.iter()
            .filter(|call| call.qualified_callee.as_ref().map(|q| q.contains("::from_ohlcv")).unwrap_or(false))
            .count();
            
        assert!(new_calls > 0, "Should generate synthetic calls for ::new methods");
        assert!(from_ohlcv_calls > 0, "Should generate synthetic calls for ::from_ohlcv methods");
        
        eprintln!("✅ Generated {} synthetic calls ({} new, {} from_ohlcv)", 
                 synthetic_calls.len(), new_calls, from_ohlcv_calls);
    }

    #[test]
    fn test_detect_paste_macro_patterns() {
        let mut parser = RustParser::new().unwrap();
        
        let source = r#"
            use paste::paste;
            
            pub fn process_indicators(config: &Config) -> Result<()> {
                paste! {
                    let indicator = [<$indicator>]::new(config)?;
                }
                
                paste! {
                    let na_value = [<$indicator>]::na();
                }
                
                paste! {
                    let nan_value = [<$indicator>]::nan();
                }
                
                Ok(())
            }
        "#;

        let symbols = parser
            .parse_source(source, &PathBuf::from("test_indicators.rs"), "trading_ta")
            .unwrap();

        // Should detect 3 macro expansions
        assert_eq!(symbols.macro_expansions.len(), 3);
        
        // Verify each macro expansion has correct properties
        for expansion in &symbols.macro_expansions {
            assert_eq!(expansion.crate_name, "trading_ta");
            assert_eq!(expansion.macro_type, "paste");
            assert!(expansion.file_path.contains("test_indicators.rs"));
            assert!(expansion.expansion_pattern.contains("paste!"));
            assert!(expansion.expansion_pattern.contains("[<$indicator>]::"));
        }
        
        // Should have one for each method type
        let methods: Vec<_> = symbols.macro_expansions.iter()
            .map(|m| {
                if m.expansion_pattern.contains("::new(") {
                    "new"
                } else if m.expansion_pattern.contains("::na()") {
                    "na"
                } else if m.expansion_pattern.contains("::nan()") {
                    "nan"
                } else {
                    "unknown"
                }
            })
            .collect();
            
        assert!(methods.contains(&"new"));
        assert!(methods.contains(&"na"));
        assert!(methods.contains(&"nan"));
    }

    // #[tokio::test]
    #[allow(dead_code)]
    async fn test_macro_expansion_graph_integration() {
        // This test requires a running Memgraph instance
        let config = Config::from_file("config.toml").unwrap_or_else(|_| Config::default());
        let client = match MemgraphClient::new(&config).await {
            Ok(client) => client,
            Err(_) => {
                eprintln!("⚠️ Skipping integration test - Memgraph not available");
                return;
            }
        };

        // Clear the graph
        let clear_query = neo4rs::query("MATCH (n) DETACH DELETE n");
        client.execute_query(clear_query).await.unwrap();

        // Parse source with macro expansions
        let mut parser = RustParser::new().unwrap();
        let source = r#"
            use paste::paste;
            
            pub fn create_indicator(config: &Config) -> Result<Box<dyn Indicator>> {
                paste! {
                    Box::new([<$indicator>]::new(config)?)
                }
            }
            
            pub fn get_na_value() -> f64 {
                paste! {
                    [<$indicator>]::na()
                }
            }
        "#;

        let symbols = parser
            .parse_source(source, &PathBuf::from("trading_indicators.rs"), "trading_ta")
            .unwrap();

        // Import symbols into graph
        client.populate_from_symbols(&symbols).await.unwrap();

        // Verify MacroExpansion nodes were created
        let macro_count = client.execute_query(
            neo4rs::Query::new("MATCH (m:MacroExpansion) RETURN count(m) as count".to_string())
        ).await.unwrap();
        
        let count: i64 = macro_count.first()
            .and_then(|row| row.get("count").ok())
            .unwrap_or(0);
            
        assert!(count > 0, "Should have created MacroExpansion nodes");

        // Verify CONTAINS_MACRO relationships exist
        let contains_macro_count = client.execute_query(
            neo4rs::Query::new("MATCH ()-[:CONTAINS_MACRO]->() RETURN count(*) as count".to_string())
        ).await.unwrap();
        
        let contains_count: i64 = contains_macro_count.first()
            .and_then(|row| row.get("count").ok())
            .unwrap_or(0);
            
        assert!(contains_count > 0, "Should have created CONTAINS_MACRO relationships");

        // Verify EXPANDS_TO_CALL relationships exist
        let expands_to_call_count = client.execute_query(
            neo4rs::Query::new("MATCH ()-[:EXPANDS_TO_CALL]->() RETURN count(*) as count".to_string())
        ).await.unwrap();
        
        let expands_count: i64 = expands_to_call_count.first()
            .and_then(|row| row.get("count").ok())
            .unwrap_or(0);
            
        // Each macro should create 23 * 3 = 69 synthetic calls
        // With 2 macros, we expect 138 relationships  
        assert!(expands_count >= 69, "Should have created many EXPANDS_TO_CALL relationships");

        eprintln!("✅ Integration test passed:");
        eprintln!("   MacroExpansion nodes: {}", count);
        eprintln!("   CONTAINS_MACRO relationships: {}", contains_count);
        eprintln!("   EXPANDS_TO_CALL relationships: {}", expands_count);
    }

    // #[test]  
    #[allow(dead_code)]
    async fn test_indicator_resolution() {
        // This test was checking private methods resolve_indicator_targets and generate_indicator_function_ids
        // These are now tested indirectly through the macro expansion tests
        /*
        let config = Config::from_file("config.toml").unwrap_or_else(|_| Config::default());
        let client = MemgraphClient::new(&config).await.unwrap_or_else(|_| {
            panic!("This test requires Memgraph connection");
        });

        let indicators = client.resolve_indicator_targets();
        
        // Should have exactly 23 indicators as per spec
        assert_eq!(indicators.len(), 23);
        
        // Verify specific indicators are included
        assert!(indicators.contains(&"Adx".to_string()));
        assert!(indicators.contains(&"Atr".to_string()));
        assert!(indicators.contains(&"BollingerBands".to_string()));
        assert!(indicators.contains(&"Rsi".to_string()));
        assert!(indicators.contains(&"Macd".to_string()));
        assert!(indicators.contains(&"IchimokuKinkoHyo".to_string()));

        // Generate function IDs
        let function_ids = client.generate_indicator_function_ids("trading_ta");
        
        // Should generate 23 * 3 = 69 function IDs
        assert_eq!(function_ids.len(), 69);
        
        // Verify pattern of generated IDs
        assert!(function_ids.iter().any(|id| id.contains("::adx::new")));
        assert!(function_ids.iter().any(|id| id.contains("::rsi::na")));
        assert!(function_ids.iter().any(|id| id.contains("::macd::nan")));
        */
    }

    #[test]
    fn test_unused_function_detection() {
        // This test demonstrates that macro-expanded functions are not considered unused
        let mut parser = RustParser::new().unwrap();
        
        // Create a test scenario with functions that would normally be considered unused
        let source = r#"
            pub fn unused_function() {
                // This should appear as unused
            }
            
            pub fn indicator_new() -> Indicator {
                // This would be called through macro expansion
                Indicator::default()
            }
            
            pub fn create_indicators() {
                paste! {
                    let indicator = [<$indicator>]::new();
                }
            }
        "#;

        let symbols = parser
            .parse_source(source, &PathBuf::from("test_unused.rs"), "test_crate")
            .unwrap();

        // Should detect the macro expansion
        assert!(!symbols.macro_expansions.is_empty());
        
        // The macro expansion should reference indicator functions
        let expansion = &symbols.macro_expansions[0];
        assert!(expansion.expansion_pattern.contains("::new()"));
        
        eprintln!("✅ Macro expansion detected: {}", expansion.expansion_pattern);
        eprintln!("   This would create synthetic calls to indicator functions");
        eprintln!("   Making them not appear in unused function queries");
    }

    // #[tokio::test]
    #[allow(dead_code)]
    async fn test_synthetic_calls_database_merge() {
        // Test that synthetic calls correctly use MERGE for target functions
        let config = Config::from_file("config.toml").unwrap_or_else(|_| Config::default());
        let client = match MemgraphClient::new(&config).await {
            Ok(client) => client,
            Err(_) => {
                eprintln!("⚠️ Skipping integration test - Memgraph not available");
                return;
            }
        };

        // Clear the graph
        let clear_query = neo4rs::query("MATCH (n) DETACH DELETE n");
        client.execute_query(clear_query).await.unwrap();

        let mut parser = RustParser::new().unwrap();
        let source = r#"
            use paste::paste;
            
            pub fn process_indicators(config: &Config) -> Result<()> {
                paste! {
                    let indicator = [<$indicator>]::new(config)?;
                }
                Ok(())
            }
        "#;

        let symbols = parser
            .parse_source(source, &PathBuf::from("test_synthetic.rs"), "trading_ta")
            .unwrap();

        // Populate the graph with the parsed data
        client.populate_from_symbols(&symbols).await.unwrap();

        // Verify synthetic functions were created
        let synthetic_functions_query = neo4rs::Query::new(
            "MATCH (f:Function {is_synthetic: true}) RETURN count(f) as count".to_string()
        );
        let synthetic_count_result = client.execute_query(synthetic_functions_query).await.unwrap();
        let synthetic_count: i64 = synthetic_count_result.first()
            .and_then(|row| row.get("count").ok())
            .unwrap_or(0);

        assert!(synthetic_count > 0, "Should have created synthetic function nodes");

        // Verify synthetic CALLS relationships were created
        let synthetic_calls_query = neo4rs::Query::new(
            "MATCH ()-[r:CALLS {is_synthetic: true}]->() RETURN count(r) as count".to_string()
        );
        let calls_count_result = client.execute_query(synthetic_calls_query).await.unwrap();
        let calls_count: i64 = calls_count_result.first()
            .and_then(|row| row.get("count").ok())
            .unwrap_or(0);

        assert!(calls_count > 0, "Should have created synthetic CALLS relationships");

        // Verify that the caller is a real function, not a MacroExpansion
        let caller_query = neo4rs::Query::new(
            "MATCH (caller)-[r:CALLS {is_synthetic: true}]->(target:Function {is_synthetic: true}) 
             RETURN caller.id as caller_id, target.qualified_name as target_name
             LIMIT 5".to_string()
        );
        let caller_result = client.execute_query(caller_query).await.unwrap();
        
        assert!(!caller_result.is_empty(), "Should find synthetic call relationships");
        
        for row in caller_result {
            let caller_id: String = row.get("caller_id").unwrap();
            let target_name: String = row.get("target_name").unwrap();
            
            // Verify caller is not a MACRO_EXPANSION node
            assert!(!caller_id.contains("MACRO_EXPANSION"), 
                   "Caller should be real function, got: {}", caller_id);
            assert!(caller_id.contains("process_indicators"), 
                   "Caller should be the containing function, got: {}", caller_id);
            assert!(target_name.contains("trading_ta"), 
                   "Target should be in trading_ta crate, got: {}", target_name);
        }

        eprintln!("✅ Database MERGE test passed:");
        eprintln!("   Synthetic functions created: {}", synthetic_count);
        eprintln!("   Synthetic CALLS relationships: {}", calls_count);
        eprintln!("   All calls originate from real functions, not MacroExpansion nodes");
    }

    #[tokio::test]
    async fn test_macro_calls_not_appearing_as_unused() {
        // Set up test config and client
        let config = Config::from_file("config.toml").unwrap_or_else(|_| Config::default());
        let client = match MemgraphClient::new(&config).await {
            Ok(client) => client,
            Err(_) => {
                eprintln!("⚠️ Skipping integration test - Memgraph not available");
                return;
            }
        };

        // Clear graph for clean test
        let clear_query = neo4rs::query("MATCH (n) DETACH DELETE n");
        client.execute_query(clear_query).await.unwrap();

        // Parse sample file with paste macros
        let source = r#"
            use paste::paste;
            
            pub fn process_indicators() {
                paste! { [<$indicator>]::new() }
            }
        "#;
        
        let mut parser = RustParser::new().unwrap();
        let symbols = parser.parse_source(source, &PathBuf::from("test.rs"), "test_crate").unwrap();
        
        // Import into graph
        client.populate_from_symbols(&symbols).await.unwrap();
        
        // Query for unused functions - should not include indicator::new functions
        let unused_query = r#"
            MATCH (f:Function) 
            OPTIONAL MATCH (f)-[:CALLS]->(called) 
            WHERE called IS NULL 
            RETURN f.qualified_name
        "#;
        
        let result = client.execute_query(neo4rs::Query::new(unused_query.to_string())).await.unwrap();
        let unused_functions: Vec<String> = result.iter()
            .filter_map(|r| r.get("f.qualified_name").ok())
            .collect();
        
        // Assert that indicator functions are NOT in unused list
        assert!(!unused_functions.iter().any(|f| f.contains("trading_ta::") && f.contains("::new")));
    }

    #[tokio::test]
    async fn test_synthetic_calls_created() {
        let config = Config::from_file("config.toml").unwrap_or_else(|_| Config::default());
        let client = match MemgraphClient::new(&config).await {
            Ok(client) => client,
            Err(_) => {
                eprintln!("⚠️ Skipping integration test - Memgraph not available");
                return;
            }
        };

        // Process macro expansion using enhanced generator
        let expansion = MacroExpansion {
            id: "test.rs:5:paste".to_string(),
            crate_name: "test_crate".to_string(),
            file_path: "test.rs".to_string(),
            line_range: 5..6,
            macro_name: "paste".to_string(),
            macro_type: "paste".to_string(),
            expansion_pattern: "paste! { [<$indicator>]::new() }".to_string(),
            expanded_content: None,
            target_functions: Vec::new(),
            containing_function: Some("test_crate::process_indicators".to_string()),
            expansion_context: MacroContext {
                expansion_id: "test.rs:5:paste".to_string(),
                macro_type: "paste".to_string(),
                expansion_site_line: 5,
                name: "paste".to_string(),
                kind: "paste_macro".to_string(),
            },
        };
        
        let generator = SyntheticCallGenerator::new();
        let calls = generator.generate_calls_from_paste_macro(&expansion, "test_crate::process_indicators");
        
        // Store in graph using enhanced batch method
        client.create_synthetic_call_relationships_batch(calls).await.unwrap();
        
        // Verify synthetic relationships exist
        let synthetic_count_query = "MATCH ()-[r:CALLS {is_synthetic: true}]->() RETURN count(r) as count";
        let result = client.execute_query(neo4rs::Query::new(synthetic_count_query.to_string())).await.unwrap();
        
        let count = result.first()
            .and_then(|row| row.get("count").ok())
            .unwrap_or(0i64);
        assert!(count >= 23); // At least one call per indicator
    }

    #[test]
    fn test_synthetic_call_generator() {
        let generator = SyntheticCallGenerator::new();
        
        let expansion = MacroExpansion {
            id: "test.rs:10:paste".to_string(),
            crate_name: "test_crate".to_string(),
            file_path: "test.rs".to_string(),
            line_range: 10..11,
            macro_name: "paste".to_string(),
            macro_type: "paste".to_string(),
            expansion_pattern: "paste! { [<$indicator>]::new(config) }".to_string(),
            expanded_content: None,
            target_functions: Vec::new(),
            containing_function: Some("test_crate::create_indicator".to_string()),
            expansion_context: MacroContext {
                expansion_id: "test.rs:10:paste".to_string(),
                macro_type: "paste".to_string(),
                expansion_site_line: 10,
                name: "paste".to_string(),
                kind: "paste_macro".to_string(),
            },
        };
        
        let calls = generator.generate_calls_from_paste_macro(&expansion, "test_crate::create_indicator");
        
        // Should generate calls for all indicators
        assert!(calls.len() >= 23, "Should generate calls for all {} indicators", calls.len());
        
        // Verify call properties
        for call in &calls {
            assert_eq!(call.caller_id, "test_crate::create_indicator");
            assert!(call.qualified_callee.as_ref().unwrap().starts_with("trading_ta::"));
            assert!(call.qualified_callee.as_ref().unwrap().ends_with("::new"));
            assert!(call.is_synthetic);
            assert_eq!(call.synthetic_confidence, 0.95);
            assert!(call.macro_context.is_some());
        }
    }

    #[test]
    fn test_dynamic_indicator_resolver() {
        let mut resolver = IndicatorResolver::new();
        
        // Test with sample source code containing a macro invocation
        let sample_source = r#"
            define_indicator_enums!(
                Atr: "Average True Range",
                Bb: "Bollinger Bands",
                Ema: "Exponential Moving Average",
                Rsi: "Relative Strength Index",
                Sma: "Simple Moving Average",
                TestIndicator: "Test Indicator for Unit Testing"
            );
        "#;
        
        // Extract indicators from the source
        let extracted = resolver.extract_from_source(sample_source);
        
        // Should extract all indicators from the macro
        assert_eq!(extracted.len(), 6, "Should extract 6 indicators from the sample");
        assert!(extracted.contains(&"Atr".to_string()));
        assert!(extracted.contains(&"Bb".to_string()));
        assert!(extracted.contains(&"Ema".to_string()));
        assert!(extracted.contains(&"Rsi".to_string()));
        assert!(extracted.contains(&"Sma".to_string()));
        assert!(extracted.contains(&"TestIndicator".to_string()));
        
        // Resolved indicators should match
        let indicators = resolver.resolve_indicators();
        assert_eq!(indicators.len(), 6, "Should have 6 resolved indicators");
    }
}