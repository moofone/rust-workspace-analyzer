// Test fixtures for actor patterns and message detection

pub const SIMPLE_ACTOR: &str = r#"
use actix::prelude::*;

#[derive(Debug)]
struct SimpleActor {
    count: usize,
}

impl Actor for SimpleActor {
    type Context = Context<Self>;
    
    fn started(&mut self, _ctx: &mut Self::Context) {
        println!("SimpleActor started");
    }
}

#[derive(Message)]
#[rtype(result = "usize")]
struct GetCount;

impl Handler<GetCount> for SimpleActor {
    type Result = usize;
    
    fn handle(&mut self, _msg: GetCount, _ctx: &mut Self::Context) -> Self::Result {
        self.count
    }
}
"#;

pub const DISTRIBUTED_ACTOR: &str = r#"
use distributed_actor::prelude::*;

#[distributed_actor]
#[derive(Debug)]
struct CalculatorActor {
    value: i32,
}

#[distributed_actor]
impl CalculatorActor {
    pub fn new(initial_value: i32) -> Self {
        Self {
            value: initial_value,
        }
    }
    
    #[message]
    pub async fn add(&mut self, amount: i32) -> i32 {
        self.value += amount;
        self.value
    }
    
    #[message]
    pub async fn get_value(&self) -> i32 {
        self.value
    }
    
    #[message]
    pub async fn reset(&mut self) {
        self.value = 0;
    }
}
"#;

pub const ACTOR_SPAWNING: &str = r#"
use actix::prelude::*;
use distributed_actor::prelude::*;

async fn spawn_actors() {
    // Kameo spawning
    let simple_actor = kameo::spawn(SimpleActor { count: 0 });
    let worker = kameo::spawn(WorkerActor::new());
    
    // Distributed actor spawning
    let calc_actor = CalculatorActor::new(42).spawn().await;
    let worker_pool = WorkerActor::spawn_pool(4).await;
    
    // Send messages
    let result = simple_addr.send(GetCount).await;
    let sum = calc_actor.add(10).await;
    let current_value = calc_actor.get_value().await;
}
"#;

pub const MESSAGE_DEFINITIONS: &str = r#"
use kameo::Actor;
use kameo::message::{Message, Context};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ProcessData {
    pub input: String,
    pub options: ProcessingOptions,
}

#[derive(Debug)]
pub struct Shutdown;

#[derive(Serialize, Deserialize)]
pub struct FetchItems {
    pub filter: ItemFilter,
    pub limit: Option<usize>,
}

// Distributed actor messages are methods, not structs
#[distributed_actor]
impl DataProcessor {
    #[message]
    pub async fn process_batch(&mut self, items: Vec<DataItem>) -> ProcessingResult {
        // processing logic
    }
    
    #[message]
    pub async fn get_stats(&self) -> ProcessingStats {
        // stats logic
    }
}
"#;

pub const MESSAGE_HANDLERS: &str = r#"
use kameo::{Actor, Context};
use kameo::message::Message;

struct DataProcessor {
    processed_count: usize,
}

impl Actor for DataProcessor {
    type Msg = ProcessorMessage;
    type Reply = ProcessorReply;
    type Args = ();
    type Error = ProcessingError;
    
    async fn on_start(&mut self, _ctx: &Context<Self>) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Message<ProcessData> for DataProcessor {
    type Reply = Result<String, ProcessingError>;
    
    async fn handle(&mut self, msg: ProcessData, _ctx: &Context<Self>) -> Self::Reply {
        self.processed_count += 1;
        
        // Simulate processing
        match self.process_internal(&msg.input).await {
            Ok(result) => Ok(result),
            Err(e) => Err(e)
        }
    }
}

impl Message<Shutdown> for DataProcessor {
    type Reply = ();
    
    async fn handle(&mut self, _: Shutdown, ctx: &Context<Self>) -> Self::Reply {
        println!("Shutting down processor after {} items", self.processed_count);
        ctx.shutdown();
    }
}
"#;

pub const COMPLEX_ACTOR_SYSTEM: &str = r#"
use actix::prelude::*;
use distributed_actor::prelude::*;

// Kameo actors
#[derive(Debug)]
struct ManagerActor {
    workers: Vec<Addr<WorkerActor>>,
}

#[derive(Debug)]
struct WorkerActor {
    id: usize,
}

impl Actor for ManagerActor {
    type Context = Context<Self>;
}

impl Actor for WorkerActor {
    type Context = Context<Self>;
}

// Distributed actors
#[distributed_actor]
struct ServiceManager {
    services: HashMap<String, ServiceInfo>,
}

#[distributed_actor]
impl ServiceManager {
    #[message]
    pub async fn register_service(&mut self, name: String, info: ServiceInfo) {
        self.services.insert(name, info);
    }
    
    #[message]
    pub async fn discover_service(&self, name: &str) -> Option<ServiceInfo> {
        self.services.get(name).cloned()
    }
    
    #[message]
    pub async fn list_services(&self) -> Vec<String> {
        self.services.keys().cloned().collect()
    }
}

async fn setup_actor_system() {
    let manager = ManagerActor::start_default();
    let service_mgr = ServiceManager::new().spawn().await;
    
    // Cross-communication
    let services = service_mgr.list_services().await;
    manager.do_send(UpdateServices { services });
}
"#;