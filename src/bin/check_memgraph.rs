use anyhow::Result;
use workspace_analyzer::graph::MemgraphClient;
use neo4rs::Query;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Checking Memgraph database contents...");
    
    let memgraph = MemgraphClient::new("check_db").await?;
    
    // Check what's actually stored in the database
    let queries = vec![
        ("Functions", "MATCH (f:Function) RETURN count(f) as count"),
        ("Types", "MATCH (t:Type) RETURN count(t) as count"),
        ("Dependencies", "MATCH ()-[r]->() RETURN count(r) as count"),
        ("All Nodes", "MATCH (n) RETURN count(n) as count"),
        ("All Relationships", "MATCH ()-[r]->() RETURN type(r), count(r) ORDER BY count(r) DESC"),
    ];
    
    for (name, query_str) in queries {
        println!("\n📊 {}: ", name);
        
        let query = Query::new(query_str.to_string());
        let mut result = memgraph.graph.execute(query).await?;
        
        while let Some(row) = result.next().await? {
            if query_str.contains("type(r)") {
                let rel_type: String = row.get("type(r)").unwrap_or_else(|_| "unknown".to_string());
                let count: i64 = row.get("count(r)").unwrap_or(0);
                println!("  {} relationships: {}", rel_type, count);
            } else {
                let count: i64 = row.get("count").unwrap_or(0);
                println!("  Count: {}", count);
            }
        }
    }
    
    // Check workspaces
    println!("\n🏢 Workspaces:");
    let query = Query::new("MATCH (n) WHERE n.workspace IS NOT NULL RETURN DISTINCT n.workspace as workspace, count(n) as nodes".to_string());
    let mut result = memgraph.graph.execute(query).await?;
    
    while let Some(row) = result.next().await? {
        let workspace: String = row.get("workspace").unwrap_or_else(|_| "unknown".to_string());
        let nodes: i64 = row.get("nodes").unwrap_or(0);
        println!("  {}: {} nodes", workspace, nodes);
    }
    
    Ok(())
}