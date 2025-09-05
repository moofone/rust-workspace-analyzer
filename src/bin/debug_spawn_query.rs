use anyhow::Result;
use workspace_analyzer::parser::RustParser;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Debug spawn query parsing...");
    
    let mut parser = RustParser::new()?;
    
    // Test the exact spawn pattern from bybit_supervisor.rs
    let source = r#"
use kameo::Actor;
use trading_exchanges::bybit::{BybitFuturesRestAPIActor, BybitFuturesOrderWSActor, BybitFuturesTradeWSActor};

pub struct BybitWsSupervisor;

impl BybitWsSupervisor {
    pub async fn spawn_actors(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use kameo::Actor;

        // Spawn REST API actor  
        let rest_api_actor = BybitFuturesRestAPIActor::spawn(BybitFuturesRestAPIActor::new());

        // Spawn Order WS actor with REST API reference for fallback
        let order_ws_actor = BybitFuturesOrderWSActor::spawn(BybitFuturesOrderWSActor::with_rest_api_actor(rest_api_actor.clone()));

        // Spawn Trade WS actor with both references
        let trade_ws_actor = BybitFuturesTradeWSActor::spawn(BybitFuturesTradeWSActor::new(order_ws_actor.clone(), rest_api_actor.clone()));
        
        Ok(())
    }
}

impl Actor for BybitWsSupervisor {
    type Msg = ();
    type State = ();
    type Reply = ();
    
    async fn handle(&mut self, _msg: Self::Msg, _ctx: &mut kameo::Context<Self>) -> Self::Reply {
        
    }
}
    "#;
    
    let file_path = PathBuf::from("debug_test.rs");
    let symbols = parser.parse_source(source, &file_path, "debug_crate")?;
    
    println!("\n📊 Parsing Results:");
    println!("  🎭 Actors detected: {}", symbols.actors.len());
    for actor in &symbols.actors {
        println!("    - {} ({})", actor.name, actor.doc_comment.as_deref().unwrap_or("explicit"));
    }
    
    println!("  🎬 Spawns detected: {}", symbols.actor_spawns.len());
    for spawn in &symbols.actor_spawns {
        println!("    - {} spawns {} via {:?} ({:?})", 
            spawn.parent_actor_name, 
            spawn.child_actor_name, 
            spawn.spawn_method,
            spawn.spawn_pattern);
    }
    
    if symbols.actor_spawns.is_empty() {
        println!("\n❌ No spawns detected - debugging tree-sitter query...");
        
        // Let's try parsing just the spawn line manually
        let spawn_line = "let rest_api_actor = BybitFuturesRestAPIActor::spawn(BybitFuturesRestAPIActor::new());";
        println!("🔍 Testing simple spawn line: {}", spawn_line);
        
        let simple_symbols = parser.parse_source(spawn_line, &file_path, "debug_crate")?;
        println!("  Simple test spawns: {}", simple_symbols.actor_spawns.len());
    } else {
        println!("\n✅ Spawns detected successfully!");
    }
    
    Ok(())
}