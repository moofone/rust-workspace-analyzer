use std::path::PathBuf;
use workspace_analyzer::analyzer::workspace_analyzer::WorkspaceAnalyzer;

#[tokio::main]
async fn main() {
    let workspace_path = PathBuf::from("/Users/greg/Dev/git/trading-backend-poc");
    let mut analyzer = WorkspaceAnalyzer::new(workspace_path).unwrap();
    
    let snapshot = analyzer.analyze_with_global_context().await.unwrap();
    
    println!("=== ACTOR DETECTION REPORT ===\n");
    
    let mut total_actors = 0;
    for (crate_name, symbols) in &snapshot.symbols {
        if !symbols.actors.is_empty() {
            println!("Crate: {}", crate_name);
            println!("  Found {} actors:", symbols.actors.len());
            for actor in &symbols.actors {
                println!("    - {} (type: {:?}, distributed: {})", 
                    actor.name, actor.actor_type, actor.is_distributed);
                total_actors += 1;
            }
            println!();
        }
    }
    
    println!("Total actors found: {}\n", total_actors);
    
    // Check distributed actors
    println!("=== DISTRIBUTED ACTORS ===");
    for (crate_name, symbols) in &snapshot.symbols {
        if !symbols.distributed_actors.is_empty() {
            println!("Crate {}: {} distributed actors", 
                crate_name, symbols.distributed_actors.len());
            for dist_actor in &symbols.distributed_actors {
                println!("  - {}", dist_actor.actor_name);
            }
        }
    }
    
    // Check message types
    println!("\n=== MESSAGE TYPES ===");
    let mut total_messages = 0;
    for (crate_name, symbols) in &snapshot.symbols {
        if !symbols.message_types.is_empty() {
            println!("Crate {}: {} message types", 
                crate_name, symbols.message_types.len());
            total_messages += symbols.message_types.len();
        }
    }
    println!("Total message types: {}", total_messages);
    
    // Check message handlers  
    println!("\n=== MESSAGE HANDLERS ===");
    let mut total_handlers = 0;
    for (crate_name, symbols) in &snapshot.symbols {
        if !symbols.message_handlers.is_empty() {
            println!("Crate {}: {} message handlers", 
                crate_name, symbols.message_handlers.len());
            total_handlers += symbols.message_handlers.len();
        }
    }
    println!("Total message handlers: {}", total_handlers);
    
    // Check message sends
    println!("\n=== MESSAGE SENDS ===");
    let mut total_sends = 0;
    for (crate_name, symbols) in &snapshot.symbols {
        if !symbols.message_sends.is_empty() {
            println!("Crate {}: {} message sends", 
                crate_name, symbols.message_sends.len());
            total_sends += symbols.message_sends.len();
        }
    }
    println!("Total message sends: {}", total_sends);
}