// Test fixtures for Kameo actor patterns from trading-backend-poc

pub const KAMEO_ACTOR: &str = r#"
use kameo::Actor;
use kameo::actor::ActorRef;
use kameo::message::{Context, Message};
use tracing::info;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Bybit WebSocket Supervisor actor (from trading-backend-poc)
pub struct BybitWsSupervisor {
    rest_api_actor: Option<ActorRef<RestAPIActor>>,
    order_ws_actor: Option<ActorRef<OrderWSActor>>,
    respawn_attempts: u32,
}

impl Actor for BybitWsSupervisor {
    type Args = Self;
    type Error = BoxError;
    
    fn name() -> &'static str {
        "BybitWsSupervisor"
    }
    
    async fn on_start(mut actor: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, BoxError> {
        info!("Bybit WebSocket supervisor started");
        
        // Spawn child actors
        let rest_api_actor = RestAPIActor::spawn(RestAPIActor::new());
        let order_ws_actor = OrderWSActor::spawn(OrderWSActor::new());
        
        actor.rest_api_actor = Some(rest_api_actor);
        actor.order_ws_actor = Some(order_ws_actor);
        
        Ok(actor)
    }
}

/// REST API Actor
pub struct RestAPIActor {
    api_key: String,
}

impl RestAPIActor {
    pub fn new() -> Self {
        Self {
            api_key: String::new(),
        }
    }
    
    pub fn spawn(actor: Self) -> ActorRef<Self> {
        // Kameo spawn pattern
        kameo::spawn(actor)
    }
}

impl Actor for RestAPIActor {
    type Args = Self;
    type Error = BoxError;
    
    fn name() -> &'static str {
        "RestAPIActor"
    }
}

/// Order WebSocket Actor
pub struct OrderWSActor {
    connected: bool,
}

impl OrderWSActor {
    pub fn new() -> Self {
        Self { connected: false }
    }
    
    pub fn spawn(actor: Self) -> ActorRef<Self> {
        kameo::spawn(actor)
    }
}

impl Actor for OrderWSActor {
    type Args = Self;
    type Error = BoxError;
    
    fn name() -> &'static str {
        "OrderWSActor"
    }
}
"#;

pub const KAMEO_REMOTE_ACTOR: &str = r#"
use kameo::actor::RemoteActor;

// Remote message with kameo attribute
#[kameo(remote)]
#[derive(Debug, Clone)]
struct RemoteDataMessage {
    pub symbol: String,
    pub values: Vec<f64>,
}

// Distributed actor pattern
struct DistributedProcessor {
    node_id: String,
}

impl Actor for DistributedProcessor {
    type Msg = RemoteDataMessage;
    type Reply = ();
    
    async fn handle(&mut self, msg: Self::Msg, _ctx: &mut kameo::Context<Self>) -> Self::Reply {
        println!("Node {} processing: {}", self.node_id, msg.symbol);
    }
}
"#;

pub const KAMEO_MESSAGE_SENDING: &str = r#"
use kameo::actor::{ActorRef, spawn};
use kameo::Message;

async fn send_messages(processor_ref: ActorRef<DataProcessor>) {
    // Tell pattern (fire and forget)
    processor_ref.tell(ProcessMessage::Add(42.0)).await;
    processor_ref.tell(ProcessMessage::Clear).await;
    
    // Ask pattern (request-response)
    let result = processor_ref.ask(ProcessMessage::GetStats).await;
    match result {
        Ok(ProcessResult::Stats { count, sum }) => {
            println!("Stats: {} items, sum: {}", count, sum);
        }
        _ => {}
    }
}

async fn spawn_and_send() {
    // Spawn actor
    let processor = DataProcessor { buffer: vec![] };
    let actor_ref = spawn(processor);
    
    // Send messages
    actor_ref.tell(ProcessMessage::Add(1.0)).await;
    let reply = actor_ref.ask(ProcessMessage::GetStats).await.unwrap();
}
"#;

pub const KAMEO_COMPLEX_PATTERNS: &str = r#"
use kameo::{Actor, Message};
use kameo::actor::{ActorRef, spawn, ActorPool};
use std::collections::HashMap;

// Actor with state management
struct StateManager {
    state: HashMap<String, f64>,
    subscribers: Vec<ActorRef<NotificationActor>>,
}

impl Actor for StateManager {
    type Msg = StateMessage;
    type Reply = StateReply;
    
    async fn handle(&mut self, msg: Self::Msg, ctx: &mut kameo::Context<Self>) -> Self::Reply {
        match msg {
            StateMessage::Update { key, value } => {
                self.state.insert(key.clone(), value);
                
                // Notify subscribers
                for subscriber in &self.subscribers {
                    subscriber.tell(NotificationMessage::StateChanged { 
                        key: key.clone(), 
                        value 
                    }).await;
                }
                
                StateReply::Updated
            }
            StateMessage::Get { key } => {
                StateReply::Value(self.state.get(&key).copied())
            }
            StateMessage::Subscribe { actor } => {
                self.subscribers.push(actor);
                StateReply::Subscribed
            }
        }
    }
}

// Notification actor
struct NotificationActor {
    id: String,
}

impl Actor for NotificationActor {
    type Msg = NotificationMessage;
    type Reply = ();
    
    async fn handle(&mut self, msg: Self::Msg, _ctx: &mut kameo::Context<Self>) -> Self::Reply {
        match msg {
            NotificationMessage::StateChanged { key, value } => {
                println!("Actor {} notified: {} = {}", self.id, key, value);
            }
        }
    }
}

// Message types
#[derive(Debug)]
enum StateMessage {
    Update { key: String, value: f64 },
    Get { key: String },
    Subscribe { actor: ActorRef<NotificationActor> },
}

#[derive(Debug)]
enum StateReply {
    Updated,
    Value(Option<f64>),
    Subscribed,
}

#[derive(Debug, Clone)]
enum NotificationMessage {
    StateChanged { key: String, value: f64 },
}
"#;