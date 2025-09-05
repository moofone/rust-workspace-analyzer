use workspace_analyzer::graph::memgraph_client::MemgraphClient;
use workspace_analyzer::config::Config;

#[tokio::main]
async fn main() {
    // Load config
    let config = Config::from_file("config.toml").expect("Failed to load config");
    
    // Connect to graph
    let graph = MemgraphClient::new(&config).await.expect("Failed to connect to Memgraph");
    
    // Clear graph
    println!("Clearing graph...");
    graph.clear_workspace().await.expect("Failed to clear");
    
    // Create a test actor manually
    println!("Creating test actor...");
    let query = neo4rs::query(r#"
        CREATE (a:Actor {
            name: 'TestActor',
            crate: 'test-crate',
            id: 'test-actor-1',
            is_distributed: false
        })
        RETURN a
    "#);
    
    let result = graph.execute_query(query).await.expect("Failed to create test actor");
    println!("Created {} test actors", result.len());
    
    // Query to verify
    println!("\nQuerying Actor nodes:");
    let query = neo4rs::query("MATCH (a:Actor) RETURN a.name as name, a.crate as crate");
    let result = graph.execute_query(query).await.expect("Query failed");
    
    println!("  Query returned {} rows", result.len());
    for row in result {
        let name = row.get::<String>("name").unwrap_or_else(|_| "?".to_string());
        let crate_name = row.get::<String>("crate").unwrap_or_else(|_| "?".to_string());
        println!("  Found Actor: {} in crate {}", name, crate_name);
    }
    
    // Try querying all nodes
    println!("\nQuerying ALL nodes:");
    let query_all = neo4rs::query("MATCH (n) RETURN labels(n) as labels, n.name as name LIMIT 10");
    let result_all = graph.execute_query(query_all).await.expect("Query all failed");
    
    println!("  All nodes query returned {} rows", result_all.len());
    for row in result_all {
        let labels: Vec<String> = row.get("labels").unwrap_or_else(|_| vec!["?".to_string()]);
        let name = row.get::<String>("name").unwrap_or_else(|_| "?".to_string());
        println!("  Node: {} with labels {:?}", name, labels);
    }
    
    // Now test the MERGE query that our code uses
    println!("\nTesting MERGE query:");
    let merge_query = neo4rs::query(r#"
        MERGE (a:Actor {name: 'MergeTestActor', crate: 'test-crate'})
        ON CREATE SET
            a.id = 'merge-test-1',
            a.is_distributed = false
        ON MATCH SET
            a.is_distributed = false
        RETURN a
    "#);
    
    let result = graph.execute_query(merge_query).await;
    match result {
        Ok(rows) => {
            println!("MERGE succeeded, created {} rows", rows.len());
        },
        Err(e) => {
            println!("MERGE failed: {}", e);
        }
    }
    
    // Query again to verify
    println!("\nFinal query of Actor nodes:");
    let query = neo4rs::query("MATCH (a:Actor) RETURN a.name as name, a.crate as crate");
    let result = graph.execute_query(query).await.expect("Query failed");
    
    for row in result {
        let name = row.get::<String>("name").unwrap_or_else(|_| "?".to_string());
        let crate_name = row.get::<String>("crate").unwrap_or_else(|_| "?".to_string());
        println!("  Found Actor: {} in crate {}", name, crate_name);
    }
}