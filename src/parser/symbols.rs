use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSymbols {
    pub functions: Vec<RustFunction>,
    pub types: Vec<RustType>,
    pub modules: Vec<RustModule>,
    pub imports: Vec<RustImport>,
    pub impls: Vec<RustImpl>,
    pub calls: Vec<FunctionCall>,
    pub actors: Vec<RustActor>,
    pub actor_spawns: Vec<ActorSpawn>,
    pub message_types: Vec<MessageType>,
    pub message_handlers: Vec<MessageHandler>,
    pub message_sends: Vec<MessageSend>,
    pub distributed_actors: Vec<DistributedActor>,
    pub distributed_message_flows: Vec<DistributedMessageFlow>,
    pub macro_expansions: Vec<MacroExpansion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustFunction {
    pub id: String,
    pub name: String,
    pub qualified_name: String,
    pub crate_name: String,
    pub module_path: String,
    pub file_path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub visibility: String,
    pub is_async: bool,
    pub is_unsafe: bool,
    pub is_generic: bool,
    pub is_test: bool,
    pub is_trait_impl: bool,
    pub doc_comment: Option<String>,
    pub signature: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub embedding_text: Option<String>,
    pub module: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub param_type: String,
    pub is_self: bool,
    pub is_mutable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustType {
    pub id: String,
    pub name: String,
    pub qualified_name: String,
    pub crate_name: String,
    pub module_path: String,
    pub file_path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub kind: TypeKind,
    pub visibility: String,
    pub is_generic: bool,
    pub is_test: bool,
    pub doc_comment: Option<String>,
    pub fields: Vec<Field>,
    pub variants: Vec<Variant>,
    pub methods: Vec<String>,
    pub embedding_text: Option<String>,
    pub type_kind: String,
    pub module: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TypeKind {
    Struct,
    Enum,
    Trait,
    TypeAlias,
    Union,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub field_type: String,
    pub visibility: String,
    pub doc_comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<Field>,
    pub doc_comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustModule {
    pub name: String,
    pub path: String,
    pub crate_name: String,
    pub file_path: String,
    pub is_public: bool,
    pub parent_module: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustImport {
    pub module_path: String,  // The path being imported (e.g., "crate_a::function_a")
    pub imported_items: Vec<ImportedItem>,  // What items are imported
    pub import_type: ImportType,
    pub file_path: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedItem {
    pub name: String,           // Original name (e.g., "function_a")
    pub alias: Option<String>,  // Alias if renamed (e.g., "as func_a")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImportType {
    Simple,    // use crate_a::function_a;
    Grouped,   // use crate_a::{function_a, utility_function};
    Glob,      // use crate_a::*;
    Module,    // use crate_a;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustImpl {
    pub type_name: String,
    pub trait_name: Option<String>,
    pub methods: Vec<RustFunction>,
    pub file_path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub is_generic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub caller_id: String,                   // Real function ID, not MACRO_EXPANSION
    pub caller_module: String,
    pub callee_name: String,
    pub qualified_callee: Option<String>,
    pub call_type: CallType,
    pub line: usize,
    pub cross_crate: bool,
    pub from_crate: String,
    pub to_crate: Option<String>,
    pub file_path: String,
    pub is_synthetic: bool,
    pub macro_context: Option<MacroContext>, // Links to originating macro
    pub synthetic_confidence: f32,           // Confidence in synthetic call
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CallType {
    Direct,
    Method,
    Associated,
    Macro,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustActor {
    pub id: String,
    pub name: String,
    pub qualified_name: String,
    pub crate_name: String,
    pub module_path: String,
    pub file_path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub visibility: String,
    pub doc_comment: Option<String>,
    pub is_distributed: bool,
    pub is_test: bool,
    pub actor_type: ActorImplementationType,
    pub local_messages: Vec<String>,
    pub inferred_from_message: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActorImplementationType {
    Local,         // Standard kameo actor
    Distributed,   // Distributed actor
    Supervisor,    // Supervisor actor
    Unknown,       // Could not determine type
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorSpawn {
    pub parent_actor_id: String,     // The actor doing the spawning
    pub parent_actor_name: String,
    pub child_actor_name: String,    // The actor being spawned
    pub spawn_method: SpawnMethod,   // How it's being spawned
    pub spawn_pattern: SpawnPattern, // Which pattern was matched
    pub context: String,             // Where the spawn happens (on_start, message handler, etc.)
    pub arguments: Option<String>,   // Arguments passed to spawn (for analysis)
    pub line: usize,
    pub file_path: String,
    pub from_crate: String,
    pub to_crate: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpawnMethod {
    Spawn,              // ActorType::spawn()
    SpawnWithMailbox,   // ActorType::spawn_with_mailbox()
    SpawnLink,          // ActorType::spawn_link()
    SpawnInThread,      // ActorType::spawn_in_thread()
    SpawnWithStorage,   // Custom spawn variant like spawn_with_storage()
    Actor,              // Actor::spawn(instance) - trait method
    ModuleSpawn,        // kameo::actor::spawn(instance) - module function
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpawnPattern {
    DirectType,         // ActorType::spawn() pattern
    TraitMethod,        // Actor::spawn(instance) pattern
    ModuleFunction,     // kameo::actor::spawn(instance) pattern
}

impl std::fmt::Display for SpawnPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpawnPattern::DirectType => write!(f, "DirectType"),
            SpawnPattern::TraitMethod => write!(f, "TraitMethod"),
            SpawnPattern::ModuleFunction => write!(f, "ModuleFunction"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageType {
    pub id: String,
    pub name: String,
    pub qualified_name: String,
    pub crate_name: String,
    pub module_path: String,
    pub file_path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub kind: MessageKind,
    pub visibility: String,
    pub doc_comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageKind {
    Tell,       // One-way message (no response)
    Ask,        // Two-way message (expects response)
    Message,    // Generic message (could be either)
    Query,      // Query message (expects response)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageHandler {
    pub id: String,
    pub actor_name: String,        // The actor that handles the message
    pub actor_qualified: String,
    pub message_type: String,      // The message type being handled
    pub message_qualified: String,
    pub reply_type: String,         // The Reply type (e.g., "()" for tell, "Result<...>" for ask)
    pub is_async: bool,
    pub file_path: String,
    pub line: usize,
    pub crate_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSend {
    pub id: String,
    pub sender_actor: String,      // Actor or context sending the message
    pub sender_qualified: Option<String>,
    pub receiver_actor: String,    // Actor receiving the message (from ActorRef)
    pub receiver_qualified: Option<String>,
    pub message_type: String,      // Type of message being sent
    pub message_qualified: Option<String>,
    pub send_method: SendMethod,   // tell or ask
    pub line: usize,
    pub file_path: String,
    pub from_crate: String,
    pub to_crate: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SendMethod {
    Tell,   // actor_ref.tell(message)
    Ask,    // actor_ref.ask(message)
}

impl ParsedSymbols {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            types: Vec::new(),
            modules: Vec::new(),
            imports: Vec::new(),
            impls: Vec::new(),
            calls: Vec::new(),
            actors: Vec::new(),
            actor_spawns: Vec::new(),
            message_types: Vec::new(),
            message_handlers: Vec::new(),
            message_sends: Vec::new(),
            distributed_actors: Vec::new(),
            distributed_message_flows: Vec::new(),
            macro_expansions: Vec::new(),
        }
    }

    pub fn merge(&mut self, other: ParsedSymbols) {
        // Deduplicate functions during merge
        let existing_functions: std::collections::HashSet<String> = self.functions.iter()
            .map(|f| format!("{}:{}", f.qualified_name, f.line_start))
            .collect();
        
        for function in other.functions {
            let function_key = format!("{}:{}", function.qualified_name, function.line_start);
            if !existing_functions.contains(&function_key) {
                self.functions.push(function);
            }
        }
        
        // Deduplicate types during merge  
        let existing_types: std::collections::HashSet<String> = self.types.iter()
            .map(|t| format!("{}:{}", t.qualified_name, t.line_start))
            .collect();
            
        for rust_type in other.types {
            let type_key = format!("{}:{}", rust_type.qualified_name, rust_type.line_start);
            if !existing_types.contains(&type_key) {
                self.types.push(rust_type);
            }
        }
        
        // Deduplicate actor spawns during merge to avoid cross-file duplicates
        let existing_spawns: std::collections::HashSet<String> = self.actor_spawns.iter()
            .map(|s| format!("{}:{}:{}:{}", s.parent_actor_name, s.child_actor_name, s.file_path, s.line))
            .collect();
        
        for spawn in other.actor_spawns {
            let spawn_key = format!("{}:{}:{}:{}", spawn.parent_actor_name, spawn.child_actor_name, spawn.file_path, spawn.line);
            if !existing_spawns.contains(&spawn_key) {
                self.actor_spawns.push(spawn);
            }
        }
        
        // Other collections can use simple extend (less likely to have problematic duplicates)
        self.modules.extend(other.modules);
        self.imports.extend(other.imports);
        self.impls.extend(other.impls);
        self.calls.extend(other.calls);
        self.actors.extend(other.actors);
        self.message_types.extend(other.message_types);
        self.message_handlers.extend(other.message_handlers);
        self.message_sends.extend(other.message_sends);
        self.distributed_actors.extend(other.distributed_actors);
        self.distributed_message_flows.extend(other.distributed_message_flows);
        self.macro_expansions.extend(other.macro_expansions);
    }

    pub fn get_function_by_name(&self, name: &str) -> Option<&RustFunction> {
        self.functions.iter().find(|f| f.qualified_name == name)
    }

    pub fn get_type_by_name(&self, name: &str) -> Option<&RustType> {
        self.types.iter().find(|t| t.qualified_name == name)
    }

    pub fn get_functions_in_crate(&self, crate_name: &str) -> Vec<&RustFunction> {
        self.functions.iter().filter(|f| f.crate_name == crate_name).collect()
    }

    pub fn get_types_in_crate(&self, crate_name: &str) -> Vec<&RustType> {
        self.types.iter().filter(|t| t.crate_name == crate_name).collect()
    }

    pub fn get_cross_crate_calls(&self) -> Vec<&FunctionCall> {
        self.calls.iter().filter(|c| c.cross_crate).collect()
    }
}

impl RustFunction {
    pub fn generate_id(&mut self) {
        self.id = format!("{}:{}:{}", self.crate_name, self.qualified_name, self.line_start);
    }

    pub fn generate_embedding_text(&mut self, fields: &[String]) -> String {
        let mut parts = Vec::new();
        
        parts.push(format!("crate: {}", self.crate_name));
        
        for field in fields {
            match field.as_str() {
                "function_name" => parts.push(format!("function: {}", self.name)),
                "module_path" => parts.push(format!("module: {}", self.module_path)),
                "doc_comments" => {
                    if let Some(doc) = &self.doc_comment {
                        parts.push(format!("docs: {}", doc));
                    }
                }
                "parameter_types" => {
                    let param_types: Vec<String> = self.parameters.iter()
                        .map(|p| p.param_type.clone())
                        .collect();
                    if !param_types.is_empty() {
                        parts.push(format!("params: {}", param_types.join(", ")));
                    }
                }
                "return_type" => {
                    if let Some(ret) = &self.return_type {
                        parts.push(format!("returns: {}", ret));
                    }
                }
                _ => {}
            }
        }
        
        let text = parts.join(" | ");
        self.embedding_text = Some(text.clone());
        text
    }
}

impl RustType {
    pub fn generate_id(&mut self) {
        self.id = format!("{}:{}:{}", self.crate_name, self.qualified_name, self.line_start);
    }

    pub fn generate_embedding_text(&mut self, fields: &[String]) -> String {
        let mut parts = Vec::new();
        
        parts.push(format!("crate: {}", self.crate_name));
        parts.push(format!("type: {}", self.name));
        parts.push(format!("kind: {:?}", self.kind));
        
        for field in fields {
            match field.as_str() {
                "module_path" => parts.push(format!("module: {}", self.module_path)),
                "doc_comments" => {
                    if let Some(doc) = &self.doc_comment {
                        parts.push(format!("docs: {}", doc));
                    }
                }
                "fields" => {
                    if !self.fields.is_empty() {
                        let field_names: Vec<String> = self.fields.iter()
                            .map(|f| f.name.clone())
                            .collect();
                        parts.push(format!("fields: {}", field_names.join(", ")));
                    }
                }
                "variants" => {
                    if !self.variants.is_empty() {
                        let variant_names: Vec<String> = self.variants.iter()
                            .map(|v| v.name.clone())
                            .collect();
                        parts.push(format!("variants: {}", variant_names.join(", ")));
                    }
                }
                _ => {}
            }
        }
        
        let text = parts.join(" | ");
        self.embedding_text = Some(text.clone());
        text
    }
}

impl Default for ParsedSymbols {
    fn default() -> Self {
        Self::new()
    }
}

/// Structure representing a detected spawn pattern for testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnPatternData {
    pub pattern_type: SpawnPattern,
    pub actor_type: String,
    pub method_name: String,
    pub arguments: Vec<String>,
    pub context: String,
    pub crate_name: String,
    pub trait_name: Option<String>,
    pub module_path: Option<String>,
    pub line: usize,
    pub file_path: String,
}

/// Distributed actor definition detected in code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedActor {
    pub id: String,
    pub actor_name: String,
    pub crate_name: String,
    pub file_path: String,
    pub line: usize,
    pub is_test: bool,
    pub distributed_messages: Vec<String>, // Distributed message types this actor handles
    pub local_messages: Vec<String>, // Local message types this actor handles via impl Message<T>
}

/// Distributed message flow - message sent from one actor to another
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedMessageFlow {
    pub id: String,
    pub message_type: String,
    pub sender_actor: String,      // Actor sending the message (from context/function)
    pub sender_context: String,    // Where the message was sent from
    pub sender_crate: String,
    pub target_actor: String,      // Target distributed actor (from variable name)
    pub target_crate: String,
    pub send_method: MessageSendMethod,  // tell or ask
    pub send_location: MessageSendLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageSendMethod {
    Tell,   // Fire-and-forget message
    Ask,    // Request-response message
}

/// Location where a distributed message was sent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSendLocation {
    pub file_path: String,
    pub line: usize,
    pub function_context: String,  // Function that sent the message
}

/// Context information for macro expansions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroContext {
    pub expansion_id: String,
    pub macro_type: String,
    pub expansion_site_line: usize,
}

/// Represents a macro expansion detected in the source code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroExpansion {
    pub id: String,                          // {file_path}:{line}:{macro_type}
    pub crate_name: String,
    pub file_path: String,
    pub line_range: std::ops::Range<usize>,  // Enhanced: line range vs single line
    pub macro_type: String,                  // "paste", "proc_macro", "declarative"
    pub expansion_pattern: String,
    pub target_functions: Vec<String>,       // Resolved target functions
    pub containing_function: Option<String>, // Containing function ID
    pub expansion_context: MacroContext,     // Richer context
}

impl MacroExpansion {
    /// Get the primary line number (start of range) for backward compatibility
    pub fn line(&self) -> usize {
        self.line_range.start
    }
}