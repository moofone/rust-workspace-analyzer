use workspace_analyzer::parser::rust_parser::RustParser;
use std::path::Path;

fn main() {
    let mut parser = RustParser::new().unwrap();
    
    let code = r#"
use kameo::actor::{Actor, ActorRef};
use kameo::message::{Message, MessageHandler};
use kameo::spawn;

// Distributed actor macro
distributed_actor! {
    struct MarketDataDistributor {
        symbol: String,
        clients: Vec<ActorRef<PriceClient>>,
    }
}

// Regular actor  
#[derive(Actor)]
struct PriceClient {
    id: String,
}

// Another distributed actor
distributed_actor! {
    struct OrderRouter {
        exchanges: Vec<String>,
    }
}

// Message types
#[derive(Message)]
struct PriceUpdate {
    symbol: String,
    price: f64,
}

impl MessageHandler<PriceUpdate> for MarketDataDistributor {
    type Reply = ();
    
    async fn handle(&mut self, msg: PriceUpdate, ctx: &mut ActorContext<Self>) -> Self::Reply {
        for client in &self.clients {
            client.send(msg.clone()).await;
        }
    }
}

#[tokio::main]
async fn main() {
    // Spawn distributed actor
    let distributor = spawn(MarketDataDistributor {
        symbol: "AAPL".to_string(),
        clients: vec![],
    });
    
    // Spawn regular actor
    let client = spawn(PriceClient {
        id: "client1".to_string(),
    });
}
"#;
    
    let result = parser.parse_source(
        code,
        Path::new("test_distributed.rs"),
        "test_crate"
    ).unwrap();
    
    println!("=== PARSING RESULTS ===\n");
    
    println!("Actors found: {}", result.actors.len());
    for actor in &result.actors {
        println!("  Actor: {} (distributed: {}, type: {:?})", 
                 actor.name, actor.is_distributed, actor.actor_type);
    }
    
    println!("\nDistributed actors: {}", result.distributed_actors.len());
    for da in &result.distributed_actors {
        println!("  Distributed Actor: {}", da.actor_name);
    }
    
    println!("\nMacro expansions found: {}", result.macro_expansions.len());
    for expansion in &result.macro_expansions {
        println!("  Macro: {} (expanded: {})", 
                 expansion.macro_name, 
                 expansion.expanded_content.is_some());
        if let Some(content) = &expansion.expanded_content {
            println!("    Content preview: {}", 
                     content.chars().take(100).collect::<String>());
        }
    }
    
    println!("\nFunctions found: {}", result.functions.len());
    for func in &result.functions {
        println!("  Function: {}", func.name);
    }
    
    println!("\nTypes found: {}", result.types.len());
    for ty in &result.types {
        println!("  Type: {}", ty.name);
    }
}