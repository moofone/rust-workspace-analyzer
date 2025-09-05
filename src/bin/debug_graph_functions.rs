use std::path::PathBuf;
use workspace_analyzer::analyzer::workspace_analyzer::WorkspaceAnalyzer;
use workspace_analyzer::graph::memgraph_client::MemgraphClient;
use workspace_analyzer::config::Config;

#[tokio::main]
async fn main() {
    // Load config
    let config = Config::from_file("config.toml").expect("Failed to load config");
    
    // Analyze workspace
    let workspace_path = PathBuf::from("/Users/greg/Dev/git/trading-backend-poc");
    let mut analyzer = WorkspaceAnalyzer::new(workspace_path).unwrap();
    let snapshot = analyzer.analyze_with_global_context().await.unwrap();
    
    // Check trading-ta symbols
    if let Some(symbols) = snapshot.symbols.get("trading-ta") {
        println!("=== Trading-ta RMA Functions ===");
        
        // Check standalone functions
        println!("\nStandalone functions in RMA module:");
        for func in &symbols.functions {
            if func.module_path.contains("rma") {
                println!("  {} | is_trait_impl: {}", func.name, func.is_trait_impl);
            }
        }
        
        // Check impl block methods  
        println!("\nImpl block methods:");
        for impl_block in &symbols.impls {
            if impl_block.type_name == "Rma" {
                println!("  Impl {} for {:?}:", impl_block.type_name, impl_block.trait_name);
                for method in &impl_block.methods {
                    println!("    {} | is_trait_impl: {}", method.name, method.is_trait_impl);
                }
            }
        }
        
        // Now check what's actually being sent to the graph
        println!("\n=== What gets sent to populate_from_symbols ===");
        
        // Simulate the deduplication logic
        let mut all_functions = symbols.functions.clone();
        
        for impl_block in &symbols.impls {
            for method in &impl_block.methods {
                if let Some(existing) = all_functions.iter_mut().find(|f| 
                    f.qualified_name == method.qualified_name && 
                    f.line_start == method.line_start
                ) {
                    println!("  Found duplicate: {} | existing.is_trait_impl: {} | method.is_trait_impl: {}", 
                        method.name, existing.is_trait_impl, method.is_trait_impl);
                    if method.is_trait_impl && !existing.is_trait_impl {
                        existing.is_trait_impl = true;
                        println!("    Updated to is_trait_impl: true");
                    }
                } else {
                    println!("  Adding new method: {} | is_trait_impl: {}", method.name, method.is_trait_impl);
                    all_functions.push(method.clone());
                }
            }
        }
        
        println!("\nFinal RMA functions to be created in graph:");
        for func in &all_functions {
            if func.module_path.contains("rma") {
                println!("  {} | is_trait_impl: {} | line: {}", func.name, func.is_trait_impl, func.line_start);
            }
        }
        
        // Now actually populate the graph
        println!("\n=== Populating Graph ===");
        let graph = MemgraphClient::new(&config).await.expect("Failed to connect to Memgraph");
        
        // Clear first
        graph.clear_workspace().await.expect("Failed to clear");
        
        // Populate
        graph.populate_from_symbols(symbols).await.expect("Failed to populate");
        
        // Query the graph to verify
        println!("\n=== Querying Graph ===");
        let query = neo4rs::query(r#"
            MATCH (f:Function)
            WHERE f.crate = 'trading-ta' AND f.module CONTAINS 'rma'
            RETURN f.name as name, f.is_trait_impl as is_trait_impl
            ORDER BY f.name
        "#);
        
        let result = graph.execute_query(query).await.expect("Query failed");
        
        println!("\nFunctions in graph:");
        for row in result {
            let name = row.get::<String>("name").unwrap_or_else(|_| "?".to_string());
            let is_trait_impl = row.get::<bool>("is_trait_impl").unwrap_or(false);
            println!("  {} | is_trait_impl: {}", name, is_trait_impl);
        }
    }
}