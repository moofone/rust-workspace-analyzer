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
        println!("=== Delta Vix Functions Analysis ===\n");
        
        // Check standalone functions for delta_vix
        println!("Standalone functions in delta_vix module:");
        for func in &symbols.functions {
            if func.module_path.contains("delta_vix") && 
               (func.name == "na" || func.name == "nan" || func.name == "nz") {
                println!("  {} at line {} | is_trait_impl: {} | qualified: {}", 
                    func.name, func.line_start, func.is_trait_impl, func.qualified_name);
            }
        }
        
        // Check impl block methods  
        println!("\nImpl block methods for delta_vix types:");
        for impl_block in &symbols.impls {
            if impl_block.type_name.contains("DeltaVix") {
                println!("  Impl {} for {:?}:", impl_block.type_name, impl_block.trait_name);
                for method in &impl_block.methods {
                    if method.name == "na" || method.name == "nan" || method.name == "nz" {
                        println!("    {} at line {} | is_trait_impl: {} | qualified: {}", 
                            method.name, method.line_start, method.is_trait_impl, method.qualified_name);
                    }
                }
            }
        }
        
        // Now simulate the deduplication logic
        println!("\n=== Simulating Graph Population Logic ===");
        let mut all_functions = symbols.functions.clone();
        
        for impl_block in &symbols.impls {
            if impl_block.type_name.contains("DeltaVix") {
                for method in &impl_block.methods {
                    if method.name == "na" || method.name == "nan" || method.name == "nz" {
                        // Find if this function already exists
                        if let Some(existing) = all_functions.iter().find(|f| 
                            f.qualified_name == method.qualified_name && 
                            (f.line_start as i32 - method.line_start as i32).abs() <= 5
                        ) {
                            println!("  Found match for {} at line {}:", method.name, method.line_start);
                            println!("    Existing: line {} | is_trait_impl: {}", 
                                existing.line_start, existing.is_trait_impl);
                            println!("    Method: line {} | is_trait_impl: {}", 
                                method.line_start, method.is_trait_impl);
                            println!("    Qualified names match: {}", 
                                existing.qualified_name == method.qualified_name);
                            println!("    Line diff: {}", 
                                (existing.line_start as i32 - method.line_start as i32).abs());
                        } else {
                            println!("  No match found for {} at line {} (would be added as new)", 
                                method.name, method.line_start);
                        }
                    }
                }
            }
        }
        
        // Clear and populate graph to see final result
        println!("\n=== Populating Graph ===");
        let graph = MemgraphClient::new(&config).await.expect("Failed to connect to Memgraph");
        
        // Clear first
        graph.clear_workspace().await.expect("Failed to clear");
        
        // Populate
        graph.populate_from_symbols(symbols).await.expect("Failed to populate");
        
        // Query the graph to verify
        println!("\n=== Querying Graph for delta_vix na/nan/nz ===");
        let query = neo4rs::query(r#"
            MATCH (f:Function)
            WHERE f.crate = 'trading-ta' 
              AND f.module CONTAINS 'delta_vix'
              AND f.name IN ['na', 'nan', 'nz']
            RETURN f.name as name, f.line_start as line, f.is_trait_impl as is_trait_impl
            ORDER BY f.line_start
        "#);
        
        let result = graph.execute_query(query).await.expect("Query failed");
        
        println!("\nFunctions in graph:");
        for row in result {
            let name = row.get::<String>("name").unwrap_or_else(|_| "?".to_string());
            let line = row.get::<i64>("line").unwrap_or(-1);
            let is_trait_impl = row.get::<bool>("is_trait_impl").unwrap_or(false);
            println!("  {} at line {} | is_trait_impl: {}", name, line, is_trait_impl);
        }
    }
}