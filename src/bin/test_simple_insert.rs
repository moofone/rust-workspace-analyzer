use neo4rs::*;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Testing simple Memgraph insertion...");
    
    // Connect to Memgraph
    let config = ConfigBuilder::default()
        .uri("bolt://localhost:7687")
        .user("")
        .password("")
        .db("memgraph")
        .build()?;
    let graph = Graph::connect(config).await?;
    
    // Test connection by running a simple query first
    println!("🔗 Testing connection...");
    
    // Try running a simple query directly without specifying database
    let test_conn_query = Query::new("RETURN 1 as test".to_string());
    match graph.run(test_conn_query).await {
        Ok(()) => println!("✅ Connection works using run()"),
        Err(e) => {
            println!("❌ Connection test failed with run(): {}", e);
            
            // Try execute as fallback
            let test_conn_query = Query::new("RETURN 1 as test".to_string());
            match graph.execute(test_conn_query).await {
                Ok(mut result) => {
                    if let Ok(Some(row)) = result.next().await {
                        if let Ok(test_val) = row.get::<i64>("test") {
                            println!("✅ Connection works with execute(), got: {}", test_val);
                        }
                    }
                }
                Err(e2) => {
                    println!("❌ Both run() and execute() failed: run={}, execute={}", e, e2);
                    return Err(e2.into());
                }
            }
        }
    }
    
    // Clear database
    println!("🧹 Clearing database...");
    let clear_query = Query::new("MATCH (n) DETACH DELETE n".to_string());
    match graph.run(clear_query).await {
        Ok(_) => println!("✅ Database cleared"),
        Err(e) => {
            println!("❌ Failed to clear database: {}", e);
            return Err(e.into());
        }
    }
    
    // Test 1: Simple node creation without transaction
    println!("📝 Creating simple test node without transaction...");
    let create_query = Query::new("CREATE (f:TestFunction {name: 'test_function', qualified_name: 'test::test_function'})".to_string());
    match graph.run(create_query).await {
        Ok(_) => println!("✅ Node creation query executed"),
        Err(e) => {
            println!("❌ Failed to create node: {}", e);
            return Err(e.into());
        }
    }
    
    // Test 2: Verify node exists
    println!("🔍 Checking if node exists...");
    let count_query = Query::new("MATCH (f:TestFunction) RETURN count(f) as count".to_string());
    match graph.execute(count_query).await {
        Ok(mut result) => {
            if let Ok(Some(row)) = result.next().await {
                if let Ok(count) = row.get::<i64>("count") {
                    println!("📊 Found {} TestFunction nodes", count);
                    if count > 0 {
                        println!("✅ Node insertion working!");
                    } else {
                        println!("❌ Node not found - insertion failed");
                    }
                }
            } else {
                println!("❌ No result returned from count query");
            }
        }
        Err(e) => {
            println!("❌ Failed to count nodes: {}", e);
            return Err(e.into());
        }
    }
    
    // Test 3: Try with parameters
    println!("🎯 Testing parameterized query...");
    let mut param_query = Query::new("CREATE (f:TestFunction {name: $name, qualified_name: $qualified_name})".to_string());
    param_query = param_query.param("name", "param_test_function");
    param_query = param_query.param("qualified_name", "test::param_test_function");
    
    match graph.run(param_query).await {
        Ok(_) => println!("✅ Parameterized query executed"),
        Err(e) => {
            println!("❌ Failed parameterized query: {}", e);
            return Err(e.into());
        }
    }
    
    // Test 4: Count all nodes
    println!("🔍 Final count...");
    let final_count_query = Query::new("MATCH (f:TestFunction) RETURN count(f) as count".to_string());
    match graph.execute(final_count_query).await {
        Ok(mut result) => {
            if let Ok(Some(row)) = result.next().await {
                if let Ok(count) = row.get::<i64>("count") {
                    println!("📊 Total TestFunction nodes: {}", count);
                }
            }
        }
        Err(e) => println!("❌ Failed final count: {}", e),
    }
    
    Ok(())
}