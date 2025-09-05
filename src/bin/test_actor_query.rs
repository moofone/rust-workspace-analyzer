use anyhow::Result;
use workspace_analyzer::{Config, MemgraphClient};
use neo4rs::Query;

#[tokio::main]
async fn main() -> Result<()> {
    let mut config = Config::from_file("config.toml")?;
    
    // Don't clear the database when testing queries
    config.memgraph.clean_start = false;
    
    let client = MemgraphClient::new(&config).await?;
    
    println!("🔍 Testing Actor Communication Queries");
    println!("=====================================\n");
    
    // Count actors
    let actor_count_query = Query::new("MATCH (a:Actor) RETURN COUNT(a) as count".to_string());
    match client.execute_query(actor_count_query).await {
        Ok(rows) => {
            if let Some(row) = rows.first() {
                if let Ok(count) = row.get::<i64>("count") {
                    println!("✅ Actor nodes: {}", count);
                }
            }
        }
        Err(e) => println!("❌ Failed to count actors: {}", e),
    }
    
    // Count SENDS relationships
    let sends_query = Query::new("MATCH ()-[s:SENDS]->() RETURN COUNT(s) as count".to_string());
    match client.execute_query(sends_query).await {
        Ok(rows) => {
            if let Some(row) = rows.first() {
                if let Ok(count) = row.get::<i64>("count") {
                    println!("✅ SENDS relationships: {}", count);
                }
            }
        }
        Err(e) => println!("❌ Failed to count SENDS: {}", e),
    }
    
    // Count HANDLES relationships
    let handles_query = Query::new("MATCH ()-[h:HANDLES]->() RETURN COUNT(h) as count".to_string());
    match client.execute_query(handles_query).await {
        Ok(rows) => {
            if let Some(row) = rows.first() {
                if let Ok(count) = row.get::<i64>("count") {
                    println!("✅ HANDLES relationships: {}", count);
                }
            }
        }
        Err(e) => println!("❌ Failed to count HANDLES: {}", e),
    }
    
    // Test actor communication query
    println!("\n📡 Actor Communication Paths:");
    let comm_query = Query::new("MATCH (a1:Actor)-[s:SENDS]->(m:MessageType)<-[h:HANDLES]-(a2:Actor)
                      RETURN a1.name as sender, m.name as message, a2.name as receiver, 
                             s.is_distributed as distributed
                      LIMIT 10".to_string());
    match client.execute_query(comm_query).await {
        Ok(rows) => {
            if rows.is_empty() {
                println!("  No actor communication paths found");
            } else {
                for row in rows.iter() {
                    let sender = row.get::<String>("sender").unwrap_or_else(|_| "null".to_string());
                    let message = row.get::<String>("message").unwrap_or_else(|_| "null".to_string());
                    let receiver = row.get::<String>("receiver").unwrap_or_else(|_| "null".to_string());
                    let distributed = row.get::<bool>("distributed").unwrap_or(false);
                    
                    let dist_label = if distributed { 
                        " [DISTRIBUTED]" 
                    } else { 
                        "" 
                    };
                    
                    println!("  {} -> {} -> {}{}", sender, message, receiver, dist_label);
                }
            }
        }
        Err(e) => println!("❌ Failed to query actor communication: {}", e),
    }
    
    // Show all message flows (including non-actors)
    println!("\n📬 All Message Flows:");
    let all_flows_query = Query::new("MATCH (sender)-[s:SENDS]->(m:MessageType)
                           OPTIONAL MATCH (m)<-[h:HANDLES]-(receiver)
                           RETURN sender.name as sender_name,
                                  labels(sender) as sender_labels,
                                  m.name as message,
                                  receiver.name as receiver_name,
                                  labels(receiver) as receiver_labels,
                                  s.is_distributed as distributed
                           LIMIT 20".to_string());
    match client.execute_query(all_flows_query).await {
        Ok(rows) => {
            if rows.is_empty() {
                println!("  No message flows found");
            } else {
                for row in rows.iter() {
                    let sender = row.get::<String>("sender_name").unwrap_or_else(|_| "null".to_string());
                    let message = row.get::<String>("message").unwrap_or_else(|_| "null".to_string());
                    let receiver = row.get::<Option<String>>("receiver_name").unwrap_or(None);
                    let sender_labels = row.get::<Vec<String>>("sender_labels").unwrap_or_else(|_| vec![]);
                    let receiver_labels = row.get::<Vec<String>>("receiver_labels").unwrap_or_else(|_| vec![]);
                    let distributed = row.get::<bool>("distributed").unwrap_or(false);
                    
                    let sender_type = if sender_labels.contains(&"Actor".to_string()) { 
                        "[Actor]" 
                    } else { 
                        "[Function]" 
                    };
                    let receiver_str = receiver.as_deref().unwrap_or("(none)");
                    let receiver_type = if receiver_labels.contains(&"Actor".to_string()) { 
                        "[Actor]" 
                    } else { 
                        "[Function]" 
                    };
                    let dist_label = if distributed { " DIST" } else { "" };
                    
                    println!("  {}{} -> {}{} -> {}{}", 
                        sender, sender_type, 
                        message, dist_label,
                        receiver_str, receiver_type);
                }
            }
        }
        Err(e) => println!("❌ Failed to query all message flows: {}", e),
    }
    
    Ok(())
}