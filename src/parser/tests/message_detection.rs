use crate::parser::RustParser;
use std::path::Path;

/// Test Kameo message send and handle detection from trading patterns
#[test]
pub fn test_kameo_message_detection() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    // Test case using Kameo patterns from trading-backend-poc
    let source = r#"
use kameo::Actor;
use kameo::actor::ActorRef;
use kameo::message::{Context, Message};

/// Message types for Kameo
#[derive(Debug, Clone)]
pub struct PreBacktestMessage {
    pub symbol: String,
    pub data: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct BacktestIterationMessage {
    pub candle: Candle,
    pub timestamp: i64,
}

#[derive(Debug)]
pub struct BacktestResult {
    pub pnl: f64,
}

pub struct StrategyActor {
    state: HashMap<String, f64>,
}

impl Actor for StrategyActor {
    type Args = Self;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    
    fn name() -> &'static str {
        "StrategyActor"
    }
}

// Message handler implementation for Kameo
impl Message<PreBacktestMessage> for StrategyActor {
    type Reply = ();
    
    async fn handle(&mut self, msg: PreBacktestMessage, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        println!("Handling pre-backtest for {}", msg.symbol);
    }
}

impl Message<BacktestIterationMessage> for StrategyActor {
    type Reply = BacktestResult;
    
    async fn handle(&mut self, msg: BacktestIterationMessage, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        BacktestResult { pnl: 0.0 }
    }
}

// Message sending patterns in Kameo
pub async fn send_messages(actor_ref: ActorRef<StrategyActor>) {
    // tell pattern - fire and forget
    actor_ref.tell(PreBacktestMessage {
        symbol: "BTC/USDT".to_string(),
        data: vec![],
    }).await;
    
    // ask pattern - wait for response
    let result = actor_ref.ask(BacktestIterationMessage {
        candle: Default::default(),
        timestamp: 0,
    }).await;
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Should detect the actor
    assert!(!result.actors.is_empty(), "Should detect actor");
    let actor = result.actors.iter()
        .find(|a| a.name == "StrategyActor")
        .expect("Should find StrategyActor");
    
    assert_eq!(actor.actor_type, crate::parser::symbols::ActorType::Local);
    
    // Should detect message types
    assert!(result.types.iter().any(|t| t.name == "PreBacktestMessage"),
            "Should detect PreBacktestMessage");
    assert!(result.types.iter().any(|t| t.name == "BacktestIterationMessage"),
            "Should detect BacktestIterationMessage");
    
    // Should detect message handlers
    assert!(result.message_handlers.len() >= 2, "Should detect message handlers");
    
    let pre_handler = result.message_handlers.iter()
        .find(|h| h.actor_name == "StrategyActor" && h.message_type == "PreBacktestMessage")
        .expect("Should find PreBacktestMessage handler");
    
    assert_eq!(pre_handler.reply_type, "()");
    assert!(pre_handler.is_async);
    
    let backtest_handler = result.message_handlers.iter()
        .find(|h| h.actor_name == "StrategyActor" && h.message_type == "BacktestIterationMessage")
        .expect("Should find BacktestIterationMessage handler");
    
    assert_eq!(backtest_handler.reply_type, "BacktestResult");
    assert!(backtest_handler.is_async);
    
    // Should detect message sends
    let tell_sends: Vec<_> = result.message_sends.iter()
        .filter(|s| s.send_method == crate::parser::symbols::SendMethod::Tell)
        .collect();
    
    assert!(!tell_sends.is_empty(), "Should detect tell() calls");
    
    let ask_sends: Vec<_> = result.message_sends.iter()
        .filter(|s| s.send_method == crate::parser::symbols::SendMethod::Ask)
        .collect();
    
    assert!(!ask_sends.is_empty(), "Should detect ask() calls");
}

/// Test distributed Kameo patterns
#[test]
fn test_kameo_distributed_patterns() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    let source = r#"
use kameo::Actor;
use kameo::actor::{ActorRef, RemoteActor};

// Remote message with kameo attribute
#[kameo(remote)]
#[derive(Debug, Clone)]
pub struct UpdateMessage {
    pub value: f64,
}

// Distributed actor
distributed_actor! {
    pub struct DistributedDataActor {
        data: Vec<f64>,
        node_id: String,
    }
}

impl Actor for DistributedDataActor {
    type Args = Self;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    
    fn name() -> &'static str {
        "DistributedDataActor"
    }
}

impl Message<UpdateMessage> for DistributedDataActor {
    type Reply = ();
    
    async fn handle(&mut self, msg: UpdateMessage, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.data.push(msg.value);
    }
}

pub async fn send_to_distributed(actor_ref: ActorRef<DistributedDataActor>) {
    actor_ref.tell(UpdateMessage { value: 42.0 }).await;
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Should detect distributed actor
    let actor = result.actors.iter()
        .find(|a| a.name == "DistributedDataActor")
        .expect("Should find DistributedDataActor");
    
    // Actor type detection based on name containing "Distributed"
    assert!(actor.is_distributed || actor.actor_type == crate::parser::symbols::ActorType::Distributed,
            "Should be marked as distributed");
    
    // Should detect message type
    assert!(result.types.iter().any(|t| t.name == "UpdateMessage"),
            "Should detect UpdateMessage");
}

/// Test Kameo supervisor patterns
#[test]
fn test_kameo_supervisor_patterns() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    let source = r#"
use kameo::Actor;
use kameo::actor::{ActorRef, WeakActorRef};
use kameo::error::ActorStopReason;

pub struct SupervisorActor {
    child_actors: Vec<ActorRef<WorkerActor>>,
    respawn_count: u32,
}

pub struct WorkerActor {
    id: String,
}

impl Actor for SupervisorActor {
    type Args = Self;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    
    fn name() -> &'static str {
        "SupervisorActor"
    }
    
    async fn on_start(mut actor: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        // Spawn child actors
        for i in 0..3 {
            let worker = WorkerActor { id: format!("worker-{}", i) };
            let worker_ref = kameo::spawn(worker);
            
            // Link child to supervisor
            worker_ref.link(&actor_ref).await;
            
            actor.child_actors.push(worker_ref);
        }
        
        Ok(actor)
    }
    
    async fn on_link_died(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        id: ActorID,
        reason: ActorStopReason,
    ) -> Result<std::ops::ControlFlow<ActorStopReason>, Self::Error> {
        // Respawn dead actor
        self.respawn_count += 1;
        
        if self.respawn_count < 10 {
            let worker = WorkerActor { id: format!("respawned-{}", self.respawn_count) };
            let worker_ref = kameo::spawn(worker);
            self.child_actors.push(worker_ref);
        }
        
        Ok(std::ops::ControlFlow::Continue(()))
    }
}

impl Actor for WorkerActor {
    type Args = Self;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    
    fn name() -> &'static str {
        "WorkerActor"
    }
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Should detect supervisor and worker actors
    let supervisor = result.actors.iter()
        .find(|a| a.name == "SupervisorActor")
        .expect("Should find SupervisorActor");
    
    assert_eq!(supervisor.actor_type, crate::parser::symbols::ActorType::Local);
    
    let worker = result.actors.iter()
        .find(|a| a.name == "WorkerActor")
        .expect("Should find WorkerActor");
    
    assert_eq!(worker.actor_type, crate::parser::symbols::ActorType::Local);
    
    // Should detect spawn calls
    let spawn_calls: Vec<_> = result.calls.iter()
        .filter(|c| c.callee_name == "spawn" || c.callee_name == "kameo::spawn")
        .collect();
    
    assert!(!spawn_calls.is_empty(), "Should detect spawn calls");
    
    // Should detect link calls
    let link_calls: Vec<_> = result.calls.iter()
        .filter(|c| c.callee_name == "link")
        .collect();
    
    assert!(!link_calls.is_empty(), "Should detect link calls");
}