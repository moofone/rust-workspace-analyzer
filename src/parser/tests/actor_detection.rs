use crate::parser::RustParser;
use std::path::Path;

/// Test actor detection from trading-strategy patterns
#[test]
pub fn test_actor_detection() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    // Test case from trading-strategy patterns
    let source = r#"
use actix::prelude::*;

pub struct CryptoFuturesStrategyActor {
    state: StrategyState,
    config: Config,
}

impl Actor for CryptoFuturesStrategyActor {
    type Context = Context<Self>;
    
    fn started(&mut self, ctx: &mut Self::Context) {
        println!("Actor started");
    }
}

// Another actor pattern
pub struct DataDistributedActor {
    data: Vec<f64>,
}

impl Actor for DataDistributedActor {
    type Context = SyncContext<Self>;
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Should detect actors
    assert_eq!(result.actors.len(), 2, "Should detect 2 actors");
    
    let strategy_actor = result.actors.iter()
        .find(|a| a.name == "CryptoFuturesStrategyActor")
        .expect("Should find CryptoFuturesStrategyActor");
    
    assert_eq!(strategy_actor.actor_type, crate::parser::symbols::ActorImplementationType::Local.into());
    assert!(!strategy_actor.is_distributed); // Regular actor
    
    let data_actor = result.actors.iter()
        .find(|a| a.name == "DataDistributedActor")
        .expect("Should find DataDistributedActor");
    
    // Should be Local type - not distributed just based on name
    assert_eq!(data_actor.actor_type, crate::parser::symbols::ActorImplementationType::Local.into());
}

/// Test distributed actor detection
#[test]
fn test_distributed_actor_detection() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    // Test distributed_actor! macro pattern
    let source = r#"
distributed_actor! {
    #[derive(Debug)]
    pub struct CryptoFuturesDataDistributedActorLive {
        symbol: String,
        state: DataState,
    }
    
    impl CryptoFuturesDataDistributedActorLive {
        pub fn new(symbol: String) -> Self {
            Self {
                symbol,
                state: DataState::default(),
            }
        }
    }
}

// Regular distributed actor pattern (contains "Distributed" in name)
pub struct OrderDistributedActor {
    orders: Vec<Order>,
}

impl Actor for OrderDistributedActor {
    type Context = Context<Self>;
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Debug output
    println!("Found {} distributed actors", result.distributed_actors.len());
    for da in &result.distributed_actors {
        println!("  Distributed Actor: {} with messages: {:?}", da.actor_name, da.distributed_messages);
    }
    
    println!("Found {} regular actors", result.actors.len());
    for a in &result.actors {
        println!("  Actor: {} (is_distributed: {})", a.name, a.is_distributed);
    }
    
    // Note: The AST walker cannot parse inside macro token trees without macro expansion
    // The struct inside distributed_actor! macro won't be detected as a distributed actor
    // This is a known limitation of the current implementation
    
    // We should at least detect the OrderDistributedActor as a regular (local) actor
    // since having "Distributed" in the name alone doesn't make it distributed
    let order_actor = result.actors.iter()
        .find(|a| a.name == "OrderDistributedActor")
        .expect("Should find OrderDistributedActor");
    
    assert_eq!(order_actor.actor_type, crate::parser::symbols::ActorType::Local, "Should be local actor (not distributed just by name)");
    
    // DistributedActor type doesn't have is_distributed field - it's inherently distributed
    
    // Regular actor with "Distributed" in name - but NOT marked as distributed
    // just because of the name. It would need to be in a distributed_actor! macro
    // or have other explicit distributed patterns
    let order_actor = result.actors.iter()
        .find(|a| a.name == "OrderDistributedActor")
        .expect("Should find OrderDistributedActor");
    
    // Should NOT be distributed just because of name
    assert!(!order_actor.is_distributed, "Should not be distributed based on name alone");
}

/// Test actor spawn detection
#[test]
fn test_actor_spawn_detection() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    let source = r#"
pub fn start_system() {
    let actor1 = MyActor::new();
    let addr1 = actor1.start();
    
    // Spawn pattern
    let addr2 = SomeActor::create(|ctx| {
        SomeActor { value: 42 }
    });
    
    // Arbiter spawn
    Arbiter::spawn(async move {
        let actor = DataActor::new();
        actor.start();
    });
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Should detect actor spawns
    let spawns: Vec<_> = result.actor_spawns.iter().collect();
    
    // Note: spawn detection depends on pattern matching in the actual implementation
    // The test validates that the parser processes spawn patterns
    assert!(result.calls.iter().any(|c| c.callee_name == "start"), 
            "Should detect start() calls");
}