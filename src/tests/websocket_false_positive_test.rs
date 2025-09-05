#[cfg(test)]
mod websocket_false_positive_tests {
    use crate::analyzer::workspace_analyzer::WorkspaceAnalyzer;
    use crate::graph::memgraph_client::MemgraphClient;
    use crate::config::Config;
    use std::path::PathBuf;

    /// Test that demonstrates false positives for trading-ta rma indicator methods.
    /// 
    /// This test checks if na(), nan(), and new() methods from trading-ta RMA indicator
    /// are incorrectly reported as unused, and also tracks trait implementations.
    #[tokio::test]
    async fn test_websocket_actor_false_positives() {
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
        
        // Use the new enhanced analysis with global context and framework patterns
        let snapshot = analyzer.analyze_with_global_context().await
            .expect("Failed to analyze workspace with global context");
        
        println!("Analyzed {} functions", snapshot.functions.len());

        // Import the symbols into the graph
        for (_crate_name, symbols) in &snapshot.symbols {
            graph.populate_from_symbols(symbols).await
                .expect("Failed to import symbols");
        }

        // Query for unused functions from trading-ta RMA indicator
        let unused_query = r#"
            MATCH (f:Function)
            WHERE NOT EXISTS {
                MATCH ()-[:CALLS]->(f)
            }
            AND f.crate = 'trading-ta'
            AND f.module CONTAINS 'rma'
            AND f.name IN ['na', 'nan', 'new']
            RETURN f.name as name, 
                   f.crate as crate, 
                   f.module as module, 
                   f.qualified_name as qualified_name,
                   f.is_trait_impl as is_trait_impl
            ORDER BY f.name
        "#;

        let query = neo4rs::query(unused_query);
        let result = graph.execute_query(query).await
            .expect("Failed to query unused functions");
        
        println!("\n🔍 Query Results for trading-ta RMA indicator:");
        println!("Found {} potentially unused RMA methods", result.len());

        // Track which methods were found as unused and if they're trait impls
        let mut na_found_unused = false;
        let mut nan_found_unused = false;
        let mut new_found_unused = false;
        let mut na_is_trait_impl = false;
        let mut nan_is_trait_impl = false;
        let mut new_is_trait_impl = false;
        
        for row in &result {
            if let Ok(name) = row.get::<String>("name") {
                let is_trait_impl = row.get::<bool>("is_trait_impl").unwrap_or(false);
                
                match name.as_str() {
                    "na" => {
                        na_found_unused = true;
                        na_is_trait_impl = is_trait_impl;
                        println!("  - na() marked as unused (trait impl: {})", is_trait_impl);
                    },
                    "nan" => {
                        nan_found_unused = true;
                        nan_is_trait_impl = is_trait_impl;
                        println!("  - nan() marked as unused (trait impl: {})", is_trait_impl);
                    },
                    "new" => {
                        new_found_unused = true;
                        new_is_trait_impl = is_trait_impl;
                        println!("  - new() marked as unused (trait impl: {})", is_trait_impl);
                    },
                    _ => {}
                }
            }
        }

        // Check for trait implementations
        let trait_impl_query = r#"
            MATCH (t:Type)-[:IMPLEMENTS]->(trait:Trait)
            WHERE t.crate = 'trading-ta' 
            AND t.name CONTAINS 'RMA'
            RETURN t.name as type_name, trait.name as trait_name
        "#;
        
        let query = neo4rs::query(trait_impl_query);
        let trait_result = graph.execute_query(query).await
            .expect("Failed to query trait implementations");
        
        if !trait_result.is_empty() {
            println!("\n📚 Trait implementations found:");
            for row in &trait_result {
                if let (Ok(type_name), Ok(trait_name)) = 
                    (row.get::<String>("type_name"), row.get::<String>("trait_name")) {
                    println!("  - {} implements {}", type_name, trait_name);
                }
            }
        }

        // Simple assertions for the 3 methods
        assert!(!na_found_unused, "RMA::na() should not be marked as unused");
        assert!(!nan_found_unused, "RMA::nan() should not be marked as unused");
        assert!(!new_found_unused, "RMA::new() should not be marked as unused");
    }
}