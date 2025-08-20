use neo4rs::*;
use anyhow::Result;
use std::time::Instant;

#[derive(Clone)]
pub struct MemgraphClient {
    pub graph: Graph,
    pub workspace_name: String,
}

impl MemgraphClient {
    pub async fn new(workspace_name: &str) -> Result<Self> {
        // Connect to Memgraph's default database  
        let config = ConfigBuilder::default()
            .uri("bolt://localhost:7687")
            .user("")
            .password("")
            .db("memgraph") // Specify Memgraph's default database name
            .build()?;
        let graph = Graph::connect(config).await?;
        
        let client = Self {
            graph,
            workspace_name: workspace_name.to_string(),
        };
        
        // Initialize optimized schema
        client.setup_performance_schema().await?;
        
        Ok(client)
    }
    
    async fn setup_performance_schema(&self) -> Result<()> {
        let schema_queries = vec![
            // Constraints for uniqueness
            "CREATE CONSTRAINT workspace_function_unique IF NOT EXISTS ON (f:Function) ASSERT (f.workspace, f.qualified_name) IS UNIQUE",
            "CREATE CONSTRAINT workspace_type_unique IF NOT EXISTS ON (t:Type) ASSERT (t.workspace, t.qualified_name) IS UNIQUE",
            
            // Indexes for fast queries
            "CREATE INDEX function_name_idx IF NOT EXISTS FOR (f:Function) ON (f.name)",
            "CREATE INDEX function_module_idx IF NOT EXISTS FOR (f:Function) ON (f.module)",
            "CREATE INDEX type_name_idx IF NOT EXISTS FOR (t:Type) ON (t.name)",
        ];
        
        for query in schema_queries {
            let _ = self.graph.execute(Query::new(query.to_string())).await;
        }
        
        eprintln!("✅ Memgraph schema initialized for workspace: {}", self.workspace_name);
        Ok(())
    }
    
    pub async fn health_check(&self) -> Result<bool> {
        let start = Instant::now();
        let mut result = self.graph.execute(Query::new("RETURN 1 as health".to_string())).await?;
        result.next().await?;
        let duration = start.elapsed();
        
        eprintln!("🏥 Memgraph health check: {}ms", duration.as_millis());
        Ok(duration.as_millis() < 50) // Should be sub-50ms for good performance
    }
}
