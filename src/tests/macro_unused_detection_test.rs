#[cfg(test)]
mod macro_unused_detection_tests {
    use crate::analyzer::WorkspaceAnalyzer;
    use crate::graph::memgraph_client::MemgraphClient;
    use crate::config::Config;
    use std::path::PathBuf;

    /// Test that indicator functions called through macro expansions 
    /// are NOT reported as unused in the graph database.
    /// 
    /// This test specifically checks that functions like `RSI::new()`, `RMA::new()`, `MACD::new()`, etc.
    /// that are called via paste! macros are properly linked with synthetic calls
    /// so they don't appear in the "unused functions" query.
    #[tokio::test]
    async fn test_indicator_functions_not_unused_with_macro_expansion() {
        // Load config from file
        let config = Config::from_file("config.toml")
            .expect("Failed to load config.toml");
        
        // Initialize the graph client
        let graph = MemgraphClient::new(&config).await
            .expect("Failed to connect to Memgraph");

        // Clear the graph to ensure clean state
        let clear_query = neo4rs::query("MATCH (n) DETACH DELETE n");
        graph.execute_query(clear_query).await
            .expect("Failed to clear graph");

        // Run the analyzer to parse the workspace
        let workspace_path = PathBuf::from("/Users/greg/Dev/git/trading-backend-poc");
        let mut analyzer = WorkspaceAnalyzer::new(workspace_path)
            .expect("Failed to create analyzer");
        
        let snapshot = analyzer.analyze_workspace()
            .expect("Failed to analyze workspace");
        
        // Count synthetic calls for diagnostics
        let mut synthetic_count = 0;
        for (_crate_name, symbols) in &snapshot.symbols {
            synthetic_count += symbols.calls.iter()
                .filter(|c| c.is_synthetic)
                .count();
        }
        
        println!("Analyzed {} functions", snapshot.functions.len());
        println!("Found {} synthetic calls", synthetic_count);

        // Import the symbols into the graph, including synthetic calls
        // We need to import each crate's symbols
        for (_crate_name, symbols) in &snapshot.symbols {
            graph.populate_from_symbols(symbols).await
                .expect("Failed to import symbols");
        }

        // Query for unused functions - functions with no incoming CALLS relationships
        // This should find indicator::new functions if macro expansion is NOT working
        let unused_query = r#"
            MATCH (f:Function)
            WHERE f.crate = 'trading-ta'
              AND f.name = 'new'
              AND f.module STARTS WITH 'trading_ta::indicators::'
              AND NOT EXISTS {
                MATCH ()-[:CALLS]->(f)
              }
            RETURN f.qualified_name as name, 
                   f.module as module, 
                   f.line_start as line
            ORDER BY f.module
        "#;

        let query = neo4rs::query(unused_query);
        let result = graph.execute_query(query).await
            .expect("Failed to query unused functions");
        
        // Log the total count of unused functions
        let total_unused_rows = result.len();
        println!("\n🔍 Query Results:");
        println!("Total unused trading_ta indicator::new functions found: {} rows", total_unused_rows);
        
        // Log each unused function found
        if total_unused_rows > 0 {
            println!("\nUnused indicator::new functions:");
            for row in &result {
                if let (Ok(name), Ok(module)) = (row.get::<String>("name"), row.get::<String>("module")) {
                    println!("  - {} (module: {})", name, module);
                }
            }
        }
        
        // Known indicators that should have synthetic calls from paste! macros
        let known_indicators = [
            "RSI", "RMA", "EMA", "SMA", "MACD", "BollingerBands", "ATR", "ADX", 
            "StochasticOscillator", "VWAP", "FisherTransform",
            "CCI", "ZScore", "CenterOfGravity", "Divergence",
            "OIIndicatorSuite", "VolatilityIndicator", "Alma", "Bb",
            "Rsi", "Rma", "Ema", "Sma", "Macd", "Atr", // Also check lowercase versions
        ];
        
        // Check if any known indicator::new functions are unused
        let mut unused_indicator_functions = Vec::new();
        
        for row in &result {
            if let Ok(name) = row.get::<String>("name") {
                // Check if this is a known indicator's new function
                for indicator in &known_indicators {
                    if name.contains(&format!("{}::new", indicator)) ||
                       name.contains(&format!("{}::new", indicator.to_lowercase())) {
                        unused_indicator_functions.push(name.to_string());
                        break;
                    }
                }
            }
        }
        
        // Always show diagnostics for debugging
        eprintln!("\n📊 Test Analysis:");
        eprintln!("Total unused indicator::new functions found: {}", unused_indicator_functions.len());
        
        if !unused_indicator_functions.is_empty() {
            eprintln!("\n❌ Unused indicator::new functions (should have synthetic calls):");
            for func in &unused_indicator_functions {
                eprintln!("  - {}", func);
            }
        }
        
        // Always check synthetic calls for debugging
        let synthetic_check = r#"
            MATCH ()-[c:CALLS]->(f:Function)
            WHERE c.is_synthetic = true
              AND f.crate = 'trading-ta'
              AND f.name = 'new'
            RETURN COUNT(c) as count
        "#;
            
        let query = neo4rs::query(synthetic_check);
        if let Ok(synthetic_result) = graph.execute_query(query).await {
            if let Some(row) = synthetic_result.first() {
                if let Ok(count) = row.get::<i64>("count") {
                    eprintln!("\nSynthetic calls to trading_ta indicator::new functions: {}", count);
                }
            }
        }

            // Show some example synthetic calls
            let example_synthetic = r#"
                MATCH (caller:Function)-[c:CALLS]->(target:Function)
                WHERE c.is_synthetic = true 
                  AND target.crate = 'trading-ta'
                  AND target.name ENDS WITH '::new'
                RETURN target.name as target, caller.name as caller
                LIMIT 5
            "#;
            
            let query = neo4rs::query(example_synthetic);
            if let Ok(example_result) = graph.execute_query(query).await {
                if !example_result.is_empty() {
                    eprintln!("\nExample synthetic calls to indicator ::new functions:");
                    for row in &example_result {
                        if let (Ok(target), Ok(caller)) = 
                            (row.get::<String>("target"),
                             row.get::<String>("caller")) {
                            eprintln!("  {} <- {}", target, caller);
                        }
                    }
                }
            }
        
        // ASSERTION: No indicator::new functions should be unused
        assert_eq!(
            unused_indicator_functions.len(),
            0,
            "Found {} indicator::new functions incorrectly marked as unused: {:?}. \
             These functions are called via paste! macros and should have synthetic CALLS relationships. \
             Total rows returned: {}",
            unused_indicator_functions.len(),
            unused_indicator_functions,
            total_unused_rows
        );
        
        // Additional assertion: we should have created synthetic calls
        assert!(
            synthetic_count > 0,
            "No synthetic calls were generated. Macro expansion is not working."
        );
        
        println!("✅ Test passed: {} synthetic calls prevent indicator functions from appearing unused", 
                 synthetic_count);
    }

    /// Test to get the total count of unused functions across the entire codebase
    #[tokio::test] 
    async fn test_count_all_unused_functions() {
        // Load config and initialize the graph client
        let config = Config::from_file("config.toml")
            .expect("Failed to load config.toml");
        let graph = MemgraphClient::new(&config).await
            .expect("Failed to connect to Memgraph");

        // Get all functions that have no direct calls (basic unused query)
        let unused_query = r#"
            MATCH (f:Function)
            WHERE NOT EXISTS {
                MATCH ()-[:CALLS]->(f)
            }
            RETURN f.qualified_name as name, 
                   f.crate as crate,
                   f.module as module,
                   f.name as function_name
            ORDER BY f.crate, f.module, f.name
        "#;

        let query = neo4rs::query(unused_query);
        let mut result = graph.execute_query(query).await
            .expect("Failed to query unused functions");
        
        println!("Raw unused functions found: {}", result.len());

        // Get calls to trait methods to filter false positives
        let trait_calls_query = r#"
            MATCH ()-[c:CALLS]->()
            WHERE c.callee_name IN ['nz', 'nan', 'na', 'all_values', 'fmt']
            RETURN DISTINCT c.callee_name as method_name
        "#;
        
        let trait_calls_query = neo4rs::query(trait_calls_query);
        let trait_calls = graph.execute_query(trait_calls_query).await
            .expect("Failed to query trait calls");
        
        let mut used_trait_methods = std::collections::HashSet::new();
        for row in &trait_calls {
            if let Ok(method_name) = row.get::<String>("method_name") {
                used_trait_methods.insert(method_name);
            }
        }
        
        println!("Found unqualified calls to: {:?}", used_trait_methods);
        
        // Filter out trait methods that have unqualified calls
        let original_count = result.len();
        result.retain(|row| {
            if let Ok(function_name) = row.get::<String>("function_name") {
                // If it's a trait method that has unqualified calls, filter it out
                !used_trait_methods.contains(&function_name)
            } else {
                true
            }
        });

        let filtered_count = original_count - result.len();
        println!("Filtered out {} trait method false positives", filtered_count);

        let total_unused = result.len();
        println!("\n📊 Total unused functions after filtering: {}", total_unused);
        
        // Group by crate for better visibility
        let mut unused_by_crate: std::collections::HashMap<String, Vec<String>> = 
            std::collections::HashMap::new();
        
        for row in &result {
            if let (Ok(name), Ok(crate_name)) = 
                (row.get::<String>("name"),
                 row.get::<String>("crate")) {
                unused_by_crate.entry(crate_name.to_string())
                    .or_insert_with(Vec::new)
                    .push(name.to_string());
            }
        }
        
        println!("\nBreakdown by crate:");
        
        for (crate_name, functions) in &unused_by_crate {
            println!("  {}: {} unused functions", crate_name, functions.len());
            
            // Show examples for trading-ta specifically
            if crate_name == "trading-ta" && !functions.is_empty() {
                println!("    Sample unused in trading-ta:");
                for func in functions.iter().take(5) {
                    println!("      - {}", func);
                }
                if functions.len() > 5 {
                    println!("      ... and {} more", functions.len() - 5);
                }
                
                // Check specifically for indicator ::new functions
                let unused_new: Vec<_> = functions.iter()
                    .filter(|f| f.ends_with("::new"))
                    .collect();
                
                if !unused_new.is_empty() {
                    println!("    ⚠️ Unused ::new functions in trading-ta: {}", unused_new.len());
                    for func in unused_new.iter().take(10) {
                        println!("      - {}", func);
                    }
                }
            }
        }
        
        // Specific assertion for trading-ta crate - indicator::new functions should not be unused
        let mut critical_unused = Vec::new();
        if let Some(trading_ta_funcs) = unused_by_crate.get("trading-ta") {
            for func in trading_ta_funcs {
                if func.ends_with("::new") {
                    // Check if it's a known indicator
                    let indicators = ["Rsi", "Rma", "Ema", "Sma", "Macd", "Atr", "Bb"];
                    if indicators.iter().any(|ind| func.contains(ind)) {
                        critical_unused.push(func.clone());
                    }
                }
            }
        }
        
        if !critical_unused.is_empty() {
            eprintln!("\n❌ Critical: Found {} indicator ::new functions marked as unused:", critical_unused.len());
            for func in &critical_unused {
                eprintln!("  - {}", func);
            }
        }
        
        assert_eq!(
            critical_unused.len(), 
            0, 
            "Found {} critical indicator ::new functions incorrectly marked as unused: {:?}",
            critical_unused.len(),
            critical_unused
        );
        
        println!("\n✅ Report generated. No critical indicator functions are incorrectly marked as unused.");
    }
}