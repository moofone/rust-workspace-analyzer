use std::collections::HashMap;
use std::path::PathBuf;
use tree_sitter::Node;

use crate::parser::ast_utils::*;
use crate::parser::symbols::*;
use crate::parser::symbols::DistributedActor;

/// Represents different contexts during AST traversal
#[derive(Debug, Clone)]
pub enum ContextFrame {
    Module { 
        name: String, 
        is_inline: bool 
    },
    Trait { 
        name: String, 
        generics: Vec<String> 
    },
    Impl { 
        type_name: String,
        trait_name: Option<String>,
        generics: Vec<String>,
    },
    Function { 
        name: String,
        is_async: bool,
        is_method: bool,
    },
    Macro { 
        name: String, 
        kind: MacroKind 
    },
}

/// Different kinds of macros we track
#[derive(Debug, Clone)]
pub enum MacroKind {
    Paste,
    AsyncTrait,
    DistributedActor,
    Derive(String),
    Custom(String),
}

impl std::fmt::Display for MacroKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MacroKind::Paste => write!(f, "paste"),
            MacroKind::AsyncTrait => write!(f, "async_trait"),
            MacroKind::DistributedActor => write!(f, "distributed_actor"),
            MacroKind::Derive(s) => write!(f, "derive:{}", s),
            MacroKind::Custom(s) => write!(f, "custom:{}", s),
        }
    }
}

/// Stack-based context tracking for AST traversal
#[derive(Debug, Clone)]
pub struct ScopeStack {
    frames: Vec<ContextFrame>,
    module_path: Vec<String>,
}

impl ScopeStack {
    pub fn new() -> Self {
        Self {
            frames: Vec::new(),
            module_path: Vec::new(),
        }
    }

    /// Push a new context frame onto the stack
    pub fn push(&mut self, frame: ContextFrame) {
        // Update module path if this is a module context
        match &frame {
            ContextFrame::Module { name, .. } => {
                self.module_path.push(name.clone());
            }
            _ => {}
        }
        
        self.frames.push(frame);
    }

    /// Pop the current context frame from the stack
    pub fn pop(&mut self) -> Option<ContextFrame> {
        if let Some(frame) = self.frames.pop() {
            // Update module path if we're leaving a module
            match &frame {
                ContextFrame::Module { .. } => {
                    self.module_path.pop();
                }
                _ => {}
            }
            Some(frame)
        } else {
            None
        }
    }

    /// Get the current function context based on stack state
    pub fn current_context(&self) -> FunctionContext {
        // Look for the most recent impl or trait context
        for frame in self.frames.iter().rev() {
            match frame {
                ContextFrame::Impl { type_name, trait_name, .. } => {
                    return if let Some(trait_name) = trait_name {
                        FunctionContext::TraitImpl {
                            trait_name: trait_name.clone(),
                            type_name: type_name.clone(),
                        }
                    } else {
                        FunctionContext::RegularImpl {
                            type_name: type_name.clone(),
                        }
                    };
                }
                ContextFrame::Trait { name, .. } => {
                    return FunctionContext::TraitDeclaration {
                        trait_name: name.clone(),
                    };
                }
                ContextFrame::Macro { name, kind } => {
                    return FunctionContext::MacroExpansion {
                        macro_info: MacroContext {
                            expansion_id: format!("inline_{}_{}", name, format!("{:?}", kind)),
                            macro_type: format!("{:?}", kind),
                            expansion_site_line: 0,  // Unknown in this context
                            name: name.clone(),
                            kind: format!("{:?}", kind),
                        }
                    };
                }
                _ => continue,
            }
        }

        FunctionContext::Free
    }

    /// Build qualified name from current context
    pub fn qualified_name(&self, name: &str) -> String {
        let mut parts = self.module_path.clone();
        
        // Add type name if we're in an impl block
        for frame in self.frames.iter().rev() {
            match frame {
                ContextFrame::Impl { type_name, .. } => {
                    parts.push(type_name.clone());
                    break;
                }
                ContextFrame::Trait { name, .. } => {
                    parts.push(name.clone());
                    break;
                }
                _ => continue,
            }
        }
        
        parts.push(name.to_string());
        parts.join("::")
    }

    /// Get the current module path
    pub fn module_path(&self) -> Vec<String> {
        self.module_path.clone()
    }

    /// Check if we're currently in a trait implementation
    pub fn in_trait_impl(&self) -> bool {
        self.frames.iter().any(|frame| {
            matches!(frame, ContextFrame::Impl { trait_name: Some(_), .. })
        })
    }

    /// Check if we're currently in any impl block
    pub fn in_impl_block(&self) -> bool {
        self.frames.iter().any(|frame| {
            matches!(frame, ContextFrame::Impl { .. })
        })
    }

    /// Get the current impl context if any
    pub fn current_impl_context(&self) -> Option<(String, Option<String>)> {
        for frame in self.frames.iter().rev() {
            if let ContextFrame::Impl { type_name, trait_name, .. } = frame {
                return Some((type_name.clone(), trait_name.clone()));
            }
        }
        None
    }
}

/// Unified AST walker that processes nodes in a single pass
pub struct UnifiedWalker<'a> {
    scope_stack: ScopeStack,
    source: &'a [u8],
    crate_name: String,
    file_path: PathBuf,
}

impl<'a> UnifiedWalker<'a> {
    pub fn new(source: &'a [u8], crate_name: String, file_path: PathBuf) -> Self {
        Self {
            scope_stack: ScopeStack::new(),
            source,
            crate_name,
            file_path,
        }
    }

    /// Main entry point for walking the AST
    pub fn walk(&mut self, node: Node<'a>) -> ParsedSymbols {
        let mut symbols = ParsedSymbols::new();
        self.walk_node(node, &mut symbols);
        symbols
    }

    /// Recursively walk a node and its children
    fn walk_node(&mut self, node: Node<'a>, symbols: &mut ParsedSymbols) {
        // Process the current node based on its type
        // These process_* methods handle their own recursion
        match node.kind() {
            "function_item" => {
                self.process_function(node, symbols);
                return; // Don't recurse - process_function handles it
            },
            "impl_item" => {
                self.process_impl_block(node, symbols);
                return; // Don't recurse - process_impl_block handles it
            },
            "trait_item" => {
                self.process_trait(node, symbols);
                return; // Don't recurse - process_trait handles it
            },
            "struct_item" | "enum_item" | "union_item" => self.process_type(node, symbols),
            "macro_invocation" => {
                self.process_macro(node, symbols);
                return; // Don't recurse - process_macro handles its own recursion
            },
            "attribute_item" => {
                self.process_attribute_macro(node, symbols);
                // Continue recursion for attributes - they may contain other nodes
            },
            "mod_item" => {
                self.process_module(node, symbols);
                return; // Don't recurse - process_module handles it
            },
            "call_expression" => self.process_call(node, symbols),
            "type_alias" | "type_item" => self.process_type_alias(node, symbols),
            "declaration_list" => {
                // Process trait method declarations inside trait bodies
                if self.in_trait_context() {
                    self.process_trait_body(node, symbols);
                    return;
                }
            },
            _ => {
                // For other nodes, just recurse through children
            }
        }

        // Recurse through all children (only for nodes not handled above)
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.walk_node(child, symbols);
        }
    }

    /// Process a function declaration
    fn process_function(&mut self, node: Node<'a>, symbols: &mut ParsedSymbols) {
        let context = self.scope_stack.current_context();
        let name = self.extract_function_name(node);
        let qualified = self.scope_stack.qualified_name(&name);

        // Check if this is an async function
        let is_async = is_async_function(node, self.source);

        // Determine if trait impl based on context
        let is_trait_impl = matches!(context, FunctionContext::TraitImpl { .. });

        // Determine if this is a method (has &self, &mut self, or self parameter)
        let is_method = self.is_method_function(node);

        // Extract parameters and return type
        let parameters = self.extract_parameters(node);
        let return_type = self.extract_return_type(node);
        let signature = safe_node_text(node, self.source).unwrap_or("").to_string();
        
        let function = RustFunction {
            id: format!("{}:{}:{}", self.file_path.display(), get_line_range(node).0, name),
            name: name.clone(),
            qualified_name: qualified,
            crate_name: self.crate_name.clone(),
            module_path: self.scope_stack.module_path().join("::"),
            file_path: self.file_path.to_string_lossy().to_string(),
            line_start: get_line_range(node).0,
            line_end: get_line_range(node).1,
            visibility: extract_visibility(node, self.source),
            is_async,
            is_unsafe: has_keyword(node, "unsafe", self.source),
            is_generic: !extract_generics(node, self.source).is_empty(),
            is_test: has_attribute(node, self.source, "test"),
            is_trait_impl,
            is_method,
            function_context: context,
            doc_comment: extract_doc_comment(node, self.source),
            signature,
            parameters,
            return_type,
            embedding_text: None,
            module: self.scope_stack.module_path().join("::"),
        };

        symbols.functions.push(function);

        // Push function context for nested processing
        self.scope_stack.push(ContextFrame::Function {
            name,
            is_async,
            is_method,
        });

        // Process children within function body
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.walk_node(child, symbols);
        }

        // Pop function context when done
        self.scope_stack.pop();
    }

    /// Process an impl block
    fn process_impl_block(&mut self, node: Node<'a>, symbols: &mut ParsedSymbols) {
        let type_name = self.extract_impl_type_name(node);
        let trait_name = self.extract_impl_trait_name(node);
        let generics = extract_generics(node, self.source);

        let impl_context = ContextFrame::Impl {
            type_name: type_name.clone(),
            trait_name: trait_name.clone(),
            generics,
        };

        self.scope_stack.push(impl_context);

        // Check if this is an Actor implementation
        if trait_name.as_deref() == Some("Actor") {
            // Extract associated types to detect Kameo patterns
            let associated_types = self.extract_associated_types(node);
            let has_args_type = associated_types.contains_key("Args");
            let has_error_type = associated_types.contains_key("Error");
            let has_context_type = associated_types.contains_key("Context");
            
            // Check if we're in a distributed_actor! macro context
            let in_distributed_macro = self.scope_stack.frames.iter().any(|f| {
                matches!(f, ContextFrame::Macro { name, .. } if name == "distributed_actor")
            });
            
            // Determine actor type based on patterns
            // All actors are Kameo, just differentiate between local and distributed
            // Mark as distributed if:
            // 1. In distributed_actor! macro
            // 2. Has distributed attribute
            // 3. Has kameo(remote) attribute on related messages (checked elsewhere)
            let actor_type = if in_distributed_macro 
                || has_attribute(node, self.source, "distributed")
                || has_attribute(node, self.source, "kameo(remote)") {
                ActorType::Distributed
            } else {
                // Default to local Kameo actor
                ActorType::Local
            };
            
            // Create an actor from the impl
            let actor = RustActor {
                id: format!("{}:{}:{}", self.file_path.display(), get_line_range(node).0, type_name),
                name: type_name.clone(),
                qualified_name: self.scope_stack.qualified_name(&type_name),
                crate_name: self.crate_name.clone(),
                module_path: self.scope_stack.module_path().join("::"),
                file_path: self.file_path.to_string_lossy().to_string(),
                line_start: get_line_range(node).0,
                line_end: get_line_range(node).1,
                actor_type,
                is_distributed: actor_type == ActorType::Distributed,
                is_test: has_attribute(node, self.source, "test"),
                local_messages: Vec::new(),
                inferred_from_message: false,
                visibility: "pub".to_string(), // Actors are typically public
                doc_comment: extract_doc_comment(node, self.source),
            };
            symbols.actors.push(actor.clone());
            
            // If it's a distributed actor, also add to distributed_actors
            if actor_type == ActorType::Distributed {
                let distributed_actor = DistributedActor {
                    id: format!("{}:{}:{}", self.file_path.display(), get_line_range(node).0, type_name),
                    actor_name: type_name.clone(),
                    crate_name: self.crate_name.clone(),
                    file_path: self.file_path.to_string_lossy().to_string(),
                    line: get_line_range(node).0,
                    is_test: has_attribute(node, self.source, "test"),
                    distributed_messages: Vec::new(),
                    local_messages: Vec::new(),
                };
                symbols.distributed_actors.push(distributed_actor);
            }
            
            // If it's a Kameo actor, track the message type
            if let Some(msg_type) = associated_types.get("Msg") {
                // Create a MessageType for the Kameo message
                let message_type = MessageType {
                    id: format!("{}:{}:{}", self.file_path.display(), get_line_range(node).0, msg_type),
                    name: msg_type.clone(),
                    qualified_name: self.scope_stack.qualified_name(msg_type),
                    crate_name: self.crate_name.clone(),
                    module_path: self.scope_stack.module_path().join("::"),
                    file_path: self.file_path.to_string_lossy().to_string(),
                    line_start: get_line_range(node).0,
                    line_end: get_line_range(node).1,
                    kind: MessageKind::Message,
                    visibility: "pub".to_string(),
                    doc_comment: None,
                };
                symbols.message_types.push(message_type);
                
                // Create a MessageHandler for the Kameo actor
                let handler = MessageHandler {
                    id: format!("{}:{}:handler", self.file_path.display(), get_line_range(node).0),
                    actor_name: type_name.clone(),
                    actor_qualified: self.scope_stack.qualified_name(&type_name),
                    message_type: msg_type.clone(),
                    message_qualified: self.scope_stack.qualified_name(msg_type),
                    reply_type: associated_types.get("Reply").unwrap_or(&"()".to_string()).clone(),
                    is_async: true, // Kameo handlers are async
                    file_path: self.file_path.to_string_lossy().to_string(),
                    line: get_line_range(node).0,
                    crate_name: self.crate_name.clone(),
                };
                symbols.message_handlers.push(handler);
            }
        }

        // Check for Message handler implementations
        if let Some(message_type) = self.extract_message_handler_type(&trait_name) {
            let handler = self.extract_message_handler(node, &type_name, message_type);
            symbols.message_handlers.push(handler);
        }

        // Process children (methods in impl block)
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.walk_node(child, symbols);
        }

        self.scope_stack.pop();
    }

    /// Process a trait declaration
    fn process_trait(&mut self, node: Node<'a>, symbols: &mut ParsedSymbols) {
        let name = extract_field_text(node, "name", self.source);
        let generics = extract_generics(node, self.source);

        let trait_context = ContextFrame::Trait {
            name: name.clone(),
            generics: generics.clone(),
        };

        self.scope_stack.push(trait_context);

        // Extract trait information
        let trait_def = RustTrait {
            name: name.clone(),
            qualified_name: self.scope_stack.qualified_name(&name),
            file_path: self.file_path.clone(),
            start_line: get_line_range(node).0,
            end_line: get_line_range(node).1,
        };

        symbols.traits.push(trait_def);
        
        // Also create a RustType entry for the trait
        let trait_type = RustType {
            id: format!("{}:{}:{}", self.file_path.display(), get_line_range(node).0, name),
            name: name.clone(),
            qualified_name: self.scope_stack.qualified_name(&name),
            crate_name: self.crate_name.clone(),
            module_path: self.scope_stack.module_path().join("::"),
            file_path: self.file_path.to_string_lossy().to_string(),
            line_start: get_line_range(node).0,
            line_end: get_line_range(node).1,
            kind: TypeKind::Trait,
            visibility: extract_visibility(node, self.source),
            is_generic: !generics.is_empty(),
            is_test: false,
            doc_comment: extract_doc_comment(node, self.source),
            fields: Vec::new(),
            variants: Vec::new(),
            methods: Vec::new(),
            embedding_text: None,
            type_kind: "Trait".to_string(),
            module: self.scope_stack.module_path().join("::"),
        };
        
        symbols.types.push(trait_type);

        // Process children (including trait method declarations)
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                match child.kind() {
                    "function_signature_item" => {
                        // Process trait method signature (no body)
                        self.process_trait_method_signature(child, symbols);
                    },
                    "function_item" => {
                        // Process trait method with default implementation
                        self.process_function(child, symbols);
                    },
                    _ => {
                        self.walk_node(child, symbols);
                    }
                }
            }
        }

        self.scope_stack.pop();
    }

    /// Process a type definition (struct, enum, union)
    fn process_type(&mut self, node: Node<'a>, symbols: &mut ParsedSymbols) {
        let name = extract_field_text(node, "name", self.source);
        let qualified = self.scope_stack.qualified_name(&name);

        // Check if we're in a distributed_actor! macro context
        let in_distributed_macro = self.scope_stack.frames.iter().any(|f| {
            matches!(f, ContextFrame::Macro { name, .. } if name == "distributed_actor")
        });

        // Check if this struct should be treated as an actor
        let is_distributed_actor = (node.kind() == "struct_item" && in_distributed_macro) ||
                                   has_attribute(node, self.source, "distributed_actor");
        
        if is_distributed_actor {
            // Create a distributed actor
            let actor = RustActor {
                id: format!("{}:{}:{}", self.file_path.display(), get_line_range(node).0, name),
                name: name.clone(),
                qualified_name: qualified.clone(),
                crate_name: self.crate_name.clone(),
                module_path: self.scope_stack.module_path().join("::"),
                file_path: self.file_path.to_string_lossy().to_string(),
                line_start: get_line_range(node).0,
                line_end: get_line_range(node).1,
                actor_type: ActorType::Distributed,
                is_distributed: true,
                is_test: has_attribute(node, self.source, "test"),
                local_messages: Vec::new(),
                inferred_from_message: false,
                visibility: extract_visibility(node, self.source),
                doc_comment: extract_doc_comment(node, self.source),
            };
            symbols.actors.push(actor);
            
            // Also create a DistributedActor entry for compatibility
            let distributed_actor = DistributedActor {
                id: format!("{}:{}:{}", self.file_path.display(), get_line_range(node).0, name),
                actor_name: name.clone(),
                crate_name: self.crate_name.clone(),
                file_path: self.file_path.to_string_lossy().to_string(),
                line: get_line_range(node).0,
                is_test: has_attribute(node, self.source, "test"),
                distributed_messages: Vec::new(),
                local_messages: Vec::new(),
            };
            symbols.distributed_actors.push(distributed_actor);
        }

        // Extract fields and variants based on type kind
        let fields = match node.kind() {
            "struct_item" | "union_item" => extract_struct_fields(node, self.source),
            _ => Vec::new(),
        };
        
        let variants = match node.kind() {
            "enum_item" => extract_enum_variants(node, self.source),
            _ => Vec::new(),
        };

        let rust_type = RustType {
            id: format!("{}:{}:{}", self.file_path.display(), get_line_range(node).0, name),
            name: name.clone(),
            qualified_name: qualified,
            crate_name: self.crate_name.clone(),
            module_path: self.scope_stack.module_path().join("::"),
            file_path: self.file_path.to_string_lossy().to_string(),
            line_start: get_line_range(node).0,
            line_end: get_line_range(node).1,
            kind: match node.kind() {
                "struct_item" => TypeKind::Struct,
                "enum_item" => TypeKind::Enum,
                "union_item" => TypeKind::Union,
                _ => TypeKind::Struct,
            },
            visibility: extract_visibility(node, self.source),
            is_generic: !extract_generics(node, self.source).is_empty(),
            is_test: has_attribute(node, self.source, "test"),
            doc_comment: extract_doc_comment(node, self.source),
            fields,
            variants,
            methods: Vec::new(), // Methods are added separately from impl blocks
            embedding_text: None,
            type_kind: match node.kind() {
                "struct_item" => "Struct".to_string(),
                "enum_item" => "Enum".to_string(),
                "union_item" => "Union".to_string(),
                _ => "Struct".to_string(),
            },
            module: self.scope_stack.module_path().join("::"),
        };

        symbols.types.push(rust_type);
    }

    /// Process attribute macros (e.g., #[criterion::criterion_group!(...)])
    fn process_attribute_macro(&mut self, node: Node<'a>, symbols: &mut ParsedSymbols) {
        // Get the attribute text
        if let Some(attr_text) = safe_node_text(node, self.source) {
            // Check if this is a macro-like attribute (contains '!')
            if attr_text.contains('!') {
                // Extract the macro name from patterns like #[criterion::criterion_group!(...)]
                // or #[macro_name!(...)]
                let cleaned = attr_text.trim_start_matches("#[").trim_end_matches(']');
                
                // Find the '!' to identify where the macro name ends
                if let Some(bang_pos) = cleaned.find('!') {
                    let macro_name = cleaned[..bang_pos].to_string();
                    
                    // Create a macro expansion entry for attribute macros
                    let line_range = get_line_range(node);
                    let expansion = MacroExpansion {
                        id: format!("{}:{}:{}", self.file_path.display(), line_range.0, macro_name),
                        crate_name: self.crate_name.clone(),
                        file_path: self.file_path.to_string_lossy().to_string(),
                        line_range: line_range.0..line_range.1,
                        macro_name: macro_name.clone(),
                        macro_type: self.classify_macro_type(&macro_name),
                        expansion_pattern: attr_text.to_string(),
                        expanded_content: Some(cleaned.to_string()),
                        target_functions: Vec::new(),
                        containing_function: self.get_containing_function(),
                        expansion_context: MacroContext {
                            expansion_id: format!("{}:{}:{}", self.file_path.display(), line_range.0, macro_name),
                            macro_type: self.classify_macro_type(&macro_name),
                            expansion_site_line: line_range.0,
                            name: macro_name.clone(),
                            kind: "attribute_macro".to_string(),
                        },
                    };
                    symbols.macro_expansions.push(expansion);
                }
            }
        }
    }

    /// Process a macro invocation
    fn process_macro(&mut self, node: Node<'a>, symbols: &mut ParsedSymbols) {
        if let Some(macro_name) = node.child_by_field_name("macro") {
            let name = extract_identifier(macro_name, self.source);
            
            // Also create a function call for the macro invocation itself
            // This captures macro calls like println!, panic!, etc.
            let macro_call = FunctionCall {
                caller_id: self.get_containing_function().unwrap_or_else(|| {
                    self.scope_stack.qualified_name("global")
                }),
                caller_module: self.scope_stack.module_path().join("::"),
                callee_name: format!("{}!", name),  // Include the ! to show it's a macro
                qualified_callee: None,
                call_type: CallType::Macro,
                line: get_line_range(node).0,
                cross_crate: false,
                from_crate: self.crate_name.clone(),
                to_crate: None,
                file_path: self.file_path.to_string_lossy().to_string(),
                is_synthetic: false,
                macro_context: Some(MacroContext {
                    expansion_id: format!("{}:{}:{}", self.file_path.display(), get_line_range(node).0, name),
                    macro_type: self.classify_macro_type(&name),
                    expansion_site_line: get_line_range(node).0,
                    name: name.clone(),
                    kind: "macro_invocation".to_string(),
                }),
                synthetic_confidence: 1.0,
            };
            symbols.calls.push(macro_call);
            
            // Special handling for distributed_actor! macro
            if name == "distributed_actor" {
                // Parse the content inside the macro to find struct definitions
                
                // Look for the token_tree child (usually the third child after identifier and !)
                let mut token_tree_node = None;
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "token_tree" {
                        token_tree_node = Some(child);
                        break;
                    }
                }
                
                if let Some(token_tree) = token_tree_node {
                    // Look for struct definitions within the token tree
                    let mut cursor = token_tree.walk();
                    for child in token_tree.children(&mut cursor) {
                        if child.kind() == "struct" {
                            // Found struct keyword, next sibling should be the struct name
                            if let Some(name_node) = child.next_sibling() {
                                if name_node.kind() == "identifier" {
                                    let struct_name = safe_node_text(name_node, self.source).unwrap_or("");
                                    
                                    // Create a distributed actor
                                    let line_range = get_line_range(node);
                                    let actor = RustActor {
                                        id: format!("{}:{}:{}", self.file_path.display(), line_range.0, struct_name),
                                        name: struct_name.to_string(),
                                        qualified_name: self.scope_stack.qualified_name(struct_name),
                                        crate_name: self.crate_name.clone(),
                                        module_path: self.scope_stack.module_path().join("::"),
                                        file_path: self.file_path.to_string_lossy().to_string(),
                                        line_start: line_range.0,
                                        line_end: line_range.1,
                                        actor_type: ActorType::Distributed,
                                        is_distributed: true,
                                        is_test: false,
                                        local_messages: Vec::new(),
                                        inferred_from_message: false,
                                        visibility: "pub".to_string(),
                                        doc_comment: None,
                                    };
                                    symbols.actors.push(actor.clone());
                                    
                                    // Also create a DistributedActor entry
                                    symbols.distributed_actors.push(DistributedActor {
                                        id: actor.id.clone(),
                                        actor_name: actor.name.clone(),
                                        crate_name: actor.crate_name.clone(),
                                        file_path: actor.file_path.clone(),
                                        line: actor.line_start,
                                        is_test: actor.is_test,
                                        distributed_messages: Vec::new(),
                                        local_messages: Vec::new(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
            
            let kind = self.detect_macro_kind(&name);

            let macro_context = ContextFrame::Macro {
                name: name.clone(),
                kind: kind.clone(),
            };

            self.scope_stack.push(macro_context);

            // Extract macro invocation details
            let macro_inv = MacroInvocation {
                name: name.clone(),
                range: get_node_range(node),
                line_range: get_line_range(node),
                containing_function: self.get_containing_function(),
                arguments: self.extract_macro_arguments(node),
            };

            symbols.macro_invocations.push(macro_inv.clone());
            
            // Create macro expansion for all tracked macros
            let line_range = get_line_range(node);
            let expanded_content = if name == "distributed_actor" {
                // For distributed_actor macros, show the struct definition in the expanded content
                if let Some(content) = self.extract_macro_content(node) {
                    // Clean up the content to show the struct definition more clearly
                    Some(format!("struct {}", content.trim()))
                } else {
                    None
                }
            } else {
                self.extract_macro_content(node)
            };
            
            let expansion = MacroExpansion {
                id: format!("{}:{}:{}", self.file_path.display(), line_range.0, name),
                crate_name: self.crate_name.clone(),
                file_path: self.file_path.to_string_lossy().to_string(),
                line_range: line_range.0..line_range.1,
                macro_name: name.clone(),
                macro_type: self.classify_macro_type(&name),
                expansion_pattern: safe_node_text(node, self.source).unwrap_or("").to_string(),
                expanded_content,
                target_functions: Vec::new(), // Would need semantic analysis
                containing_function: self.get_containing_function(),
                expansion_context: MacroContext {
                    expansion_id: format!("{}:{}:{}", self.file_path.display(), line_range.0, name),
                    macro_type: self.classify_macro_type(&name),
                    expansion_site_line: line_range.0,
                    name: name.clone(),
                    kind: kind.to_string(),
                },
            };
            symbols.macro_expansions.push(expansion);
            
            // Special handling for specific macros
            match name.as_str() {
                "paste" => {
                    // Generate synthetic calls from paste! patterns
                    self.generate_paste_synthetic_calls(node, symbols);
                },
                "define_indicator_enums" | "generate_builder" => {
                    // Track custom macro patterns from trading-backend-poc
                    self.process_custom_macro_pattern(node, &name, symbols);
                },
                "tokio::join" | "join" => {
                    // Track tokio::join! macro invocations
                    // These are async runtime macros
                },
                "test" | "tokio::test" | "async_test" => {
                    // Track test attribute macros
                    // Mark the containing function as a test
                },
                "dec" => {
                    // Track decimal macro usage
                    // These create Decimal literals
                },
                _ => {
                    // Standard library and other macros
                }
            }

            // Process children
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                self.walk_node(child, symbols);
            }

            self.scope_stack.pop();
        }
    }

    /// Process a module declaration
    fn process_module(&mut self, node: Node<'a>, symbols: &mut ParsedSymbols) {
        let name = extract_field_text(node, "name", self.source);
        let is_inline = node.child_by_field_name("body").is_some();

        let module_context = ContextFrame::Module {
            name: name.clone(),
            is_inline,
        };

        self.scope_stack.push(module_context);

        // Process children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.walk_node(child, symbols);
        }

        self.scope_stack.pop();
    }

    /// Process a function call
    fn process_call(&mut self, node: Node<'a>, symbols: &mut ParsedSymbols) {
        if let Some(function_node) = node.child_by_field_name("function") {
            // Parse the call to extract the actual function name and qualified path
            let (callee_name, qualified_callee) = self.parse_call_target(function_node);
            
            // Check for Kameo message send patterns
            if callee_name == "tell" || callee_name == "ask" || callee_name == "do_send" || callee_name == "send" {
                if let Some(message_send) = self.detect_message_send(node, &callee_name) {
                    symbols.message_sends.push(message_send);
                }
            }
            
            let call = FunctionCall {
                caller_id: self.get_containing_function().unwrap_or_else(|| {
                    self.scope_stack.qualified_name("global")
                }),
                caller_module: self.scope_stack.module_path().join("::"),
                callee_name,
                qualified_callee,
                call_type: self.determine_call_type(function_node),
                line: get_line_range(node).0,
                cross_crate: false, // Will be determined by import resolution
                from_crate: self.crate_name.clone(),
                to_crate: None,
                file_path: self.file_path.to_string_lossy().to_string(),
                is_synthetic: false,
                macro_context: None,
                synthetic_confidence: 0.0,
            };

            symbols.calls.push(call);
        }
    }
    
    /// Parse a call target to extract the function name and qualified path
    fn parse_call_target(&self, node: Node<'a>) -> (String, Option<String>) {
        match node.kind() {
            "scoped_identifier" => {
                // Type::method pattern - extract method name only
                // The qualified_callee should be None for these patterns
                // as they will be resolved by the reference resolver
                if let Some(name_node) = node.child_by_field_name("name") {
                    let method_name = safe_node_text(name_node, self.source)
                        .unwrap_or("<unknown>")
                        .to_string();
                    
                    // Return None for qualified_callee to match expected behavior
                    return (method_name, None);
                }
            }
            "field_expression" => {
                // object.method pattern
                if let Some(field_node) = node.child_by_field_name("field") {
                    let method_name = safe_node_text(field_node, self.source)
                        .unwrap_or("<unknown>")
                        .to_string();
                    return (method_name, None);
                }
            }
            "generic_function" => {
                // function::<T>() pattern
                if let Some(func_node) = node.child_by_field_name("function") {
                    return self.parse_call_target(func_node);
                }
            }
            "identifier" => {
                // Simple function call
                let func_name = safe_node_text(node, self.source)
                    .unwrap_or("<unknown>")
                    .to_string();
                return (func_name, None);
            }
            _ => {
                // For other patterns, try to extract something meaningful
                let full_text = safe_node_text(node, self.source)
                    .unwrap_or("<unknown>")
                    .to_string();
                
                // Check if it looks like a qualified call
                if full_text.contains("::") {
                    let parts: Vec<&str> = full_text.rsplitn(2, "::").collect();
                    if parts.len() == 2 {
                        return (parts[0].to_string(), Some(full_text));
                    }
                }
                
                return (full_text, None);
            }
        }
        
        // Fallback
        let text = safe_node_text(node, self.source)
            .unwrap_or("<unknown>")
            .to_string();
        (text, None)
    }

    // Helper methods

    fn extract_function_name(&self, node: Node<'a>) -> String {
        extract_field_text(node, "name", self.source)
    }

    fn extract_impl_type_name(&self, node: Node<'a>) -> String {
        if let Some(type_node) = node.child_by_field_name("type") {
            safe_node_text(type_node, self.source)
                .unwrap_or("<unknown>")
                .to_string()
        } else {
            "<unknown>".to_string()
        }
    }

    /// Detect message send patterns (tell, ask, etc.)
    fn detect_message_send(&self, call_node: Node<'a>, method_name: &str) -> Option<MessageSend> {
        // Get the receiver of the method call (the actor ref)
        let receiver = if let Some(function_node) = call_node.child_by_field_name("function") {
            if function_node.kind() == "field_expression" {
                if let Some(value_node) = function_node.child_by_field_name("value") {
                    safe_node_text(value_node, self.source)
                        .unwrap_or("<unknown>")
                        .to_string()
                } else {
                    "<unknown>".to_string()
                }
            } else {
                "<unknown>".to_string()
            }
        } else {
            "<unknown>".to_string()
        };
        
        // Try to extract the message type from arguments
        let message_type = if let Some(args_node) = call_node.child_by_field_name("arguments") {
            // Get the first argument which should be the message
            let mut cursor = args_node.walk();
            let mut message_type_str = "<unknown>".to_string();
            
            for child in args_node.children(&mut cursor) {
                if child.kind() != "(" && child.kind() != ")" && child.kind() != "," {
                    // Try to extract the type or constructor name
                    if let Some(text) = safe_node_text(child, self.source) {
                        // If it's a struct construction like ProcessMessage::Add(42.0)
                        if text.contains("::") {
                            message_type_str = text.split("::").next().unwrap_or("<unknown>").to_string();
                        } else {
                            message_type_str = text.to_string();
                        }
                        break;
                    }
                }
            }
            
            message_type_str
        } else {
            "<unknown>".to_string()
        };
        
        // Determine send method based on method name
        let send_method = match method_name {
            "tell" | "do_send" => SendMethod::Tell,
            "ask" | "send" => SendMethod::Ask,
            _ => SendMethod::Tell,
        };
        
        Some(MessageSend {
            id: format!("{}:{}:send", self.file_path.display(), get_line_range(call_node).0),
            sender_actor: self.get_containing_function().unwrap_or_else(|| "global".to_string()),
            sender_qualified: Some(self.scope_stack.qualified_name("sender")),
            receiver_actor: receiver.clone(),
            receiver_qualified: Some(receiver),
            message_type,
            message_qualified: None,
            send_method,
            line: get_line_range(call_node).0,
            file_path: self.file_path.to_string_lossy().to_string(),
            from_crate: self.crate_name.clone(),
            to_crate: None,
        })
    }
    
    fn extract_associated_types(&self, node: Node<'a>) -> HashMap<String, String> {
        let mut types = HashMap::new();
        
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            
            for child in body.children(&mut cursor) {
                // Check for both associated_type (in trait defs) and type_item (in impl blocks)
                if child.kind() == "associated_type" || child.kind() == "type_item" {
                    // Extract the name of the associated type (e.g., "Msg", "Reply")
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let type_name = safe_node_text(name_node, self.source)
                            .unwrap_or("")
                            .to_string();
                        
                        // Extract the type it's set to
                        if let Some(type_node) = child.child_by_field_name("type") {
                            let type_value = safe_node_text(type_node, self.source)
                                .unwrap_or("")
                                .to_string();
                            
                            types.insert(type_name, type_value);
                        }
                    } else {
                        // For type_item nodes, the structure is different
                        // type Reply = BacktestResult;
                        let mut type_name = String::new();
                        let mut type_value = String::new();
                        
                        let mut found_equals = false;
                        let mut child_cursor = child.walk();
                        for grandchild in child.children(&mut child_cursor) {
                            match grandchild.kind() {
                                "type_identifier" if !found_equals => {
                                    type_name = safe_node_text(grandchild, self.source)
                                        .unwrap_or("")
                                        .to_string();
                                },
                                "=" => found_equals = true,
                                "type_identifier" if found_equals => {
                                    type_value = safe_node_text(grandchild, self.source)
                                        .unwrap_or("")
                                        .to_string();
                                },
                                _ => {}
                            }
                        }
                        
                        if !type_name.is_empty() && !type_value.is_empty() {
                            types.insert(type_name, type_value);
                        }
                    }
                }
            }
        }
        
        types
    }
    
    fn extract_impl_trait_name(&self, node: Node<'a>) -> Option<String> {
        // Look for "impl Trait for Type" pattern
        let mut cursor = node.walk();
        let children: Vec<Node> = node.children(&mut cursor).collect();

        // Find "for" keyword to identify trait implementation
        let for_index = children.iter().position(|n| n.kind() == "for")?;

        // The trait should be before the "for" keyword
        if for_index > 0 {
            let trait_node = children[for_index - 1];
            Some(safe_node_text(trait_node, self.source)?.to_string())
        } else {
            None
        }
    }

    /// Extract function parameters
    fn extract_parameters(&self, node: Node<'a>) -> Vec<Parameter> {
        let mut parameters = Vec::new();
        
        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut cursor = params_node.walk();
            
            for child in params_node.children(&mut cursor) {
                if child.kind() == "parameter" || child.kind() == "self_parameter" {
                    let param_text = safe_node_text(child, self.source).unwrap_or("");
                    
                    // Check if it's a self parameter
                    let is_self = child.kind() == "self_parameter" || param_text.contains("self");
                    let is_mutable = param_text.contains("mut");
                    
                    // Extract parameter name and type
                    let (name, param_type) = if is_self {
                        ("self".to_string(), "Self".to_string())
                    } else if let Some(pattern) = child.child_by_field_name("pattern") {
                        let name = safe_node_text(pattern, self.source).unwrap_or("").to_string();
                        let param_type = if let Some(type_node) = child.child_by_field_name("type") {
                            safe_node_text(type_node, self.source).unwrap_or("").to_string()
                        } else {
                            String::new()
                        };
                        (name, param_type)
                    } else {
                        // Try to parse from text (fallback)
                        let parts: Vec<&str> = param_text.split(':').collect();
                        if parts.len() >= 2 {
                            (parts[0].trim().to_string(), parts[1].trim().to_string())
                        } else {
                            (param_text.to_string(), String::new())
                        }
                    };
                    
                    parameters.push(Parameter {
                        name,
                        param_type,
                        is_self,
                        is_mutable,
                    });
                }
            }
        }
        
        parameters
    }
    
    /// Extract function return type
    fn extract_return_type(&self, node: Node<'a>) -> Option<String> {
        if let Some(return_type_node) = node.child_by_field_name("return_type") {
            // The return type node contains "->" followed by the type
            // We want to extract everything after the "->"
            let full_text = safe_node_text(return_type_node, self.source)?;
            // Remove the "->" prefix
            let type_text = full_text.trim_start_matches("->").trim();
            if !type_text.is_empty() {
                return Some(type_text.to_string());
            }
        }
        None
    }
    
    fn is_method_function(&self, node: Node<'a>) -> bool {
        if let Some(params) = node.child_by_field_name("parameters") {
            let mut cursor = params.walk();
            for param in params.children(&mut cursor) {
                if param.kind() == "parameter" || param.kind() == "self_parameter" {
                    if let Some(param_text) = safe_node_text(param, self.source) {
                        if param_text.contains("self") {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn extract_actor_from_impl(&self, node: Node<'a>, type_name: &str) -> RustActor {
        RustActor {
            id: format!("{}:{}:{}", self.file_path.display(), get_line_range(node).0, type_name),
            name: type_name.to_string(),
            qualified_name: self.scope_stack.qualified_name(type_name),
            crate_name: self.crate_name.clone(),
            module_path: self.scope_stack.module_path().join("::"),
            file_path: self.file_path.to_string_lossy().to_string(),
            line_start: get_line_range(node).0,
            line_end: get_line_range(node).1,
            visibility: extract_visibility(node, self.source),
            doc_comment: extract_doc_comment(node, self.source),
            is_distributed: false, // TODO: Detect distributed actors
            is_test: has_attribute(node, self.source, "test"),
            actor_type: ActorType::Local, // Default to local Kameo actor
            local_messages: Vec::new(), // Will be populated later
            inferred_from_message: false,
        }
    }

    fn extract_message_handler_type(&self, trait_name: &Option<String>) -> Option<String> {
        if let Some(name) = trait_name {
            // Check for Kameo Message<T> pattern
            if name.starts_with("Message<") && name.ends_with(">") {
                let message_type = &name[8..name.len() - 1];
                Some(message_type.to_string())
            } else {
                None
            }
        } else {
            None
        }
    }

    fn extract_message_handler(&self, node: Node<'a>, actor_name: &str, message_type: String) -> MessageHandler {
        // Extract associated types from the impl block to get Reply type
        let associated_types = self.extract_associated_types(node);
        let reply_type = associated_types.get("Reply")
            .cloned()
            .unwrap_or_else(|| "()".to_string());
        
        MessageHandler {
            id: format!("{}:{}:handler", self.file_path.display(), get_line_range(node).0),
            actor_name: actor_name.to_string(),
            actor_qualified: self.scope_stack.qualified_name(actor_name),
            message_type: message_type.clone(),
            message_qualified: message_type, // TODO: Make this properly qualified
            reply_type,
            is_async: true, // Kameo handlers are always async
            file_path: self.file_path.to_string_lossy().to_string(),
            line: get_line_range(node).0,
            crate_name: self.crate_name.clone(),
        }
    }

    fn detect_macro_kind(&self, name: &str) -> MacroKind {
        match name {
            "paste" => MacroKind::Paste,
            "async_trait" => MacroKind::AsyncTrait,
            "distributed_actor" => MacroKind::DistributedActor,
            name if name == "derive" => MacroKind::Derive(name.to_string()),
            _ => MacroKind::Custom(name.to_string()),
        }
    }
    
    fn classify_macro_type(&self, name: &str) -> String {
        match name {
            "paste" => "paste".to_string(),
            "println" | "eprintln" | "print" | "eprint" | "format" | "write" | "writeln" => "builtin".to_string(),
            "vec" | "assert" | "assert_eq" | "assert_ne" | "debug_assert" => "builtin".to_string(),
            "panic" | "unreachable" | "unimplemented" | "todo" => "builtin".to_string(),
            "matches" | "cfg" | "thread_local" => "builtin".to_string(),
            "info" | "warn" | "error" | "debug" | "trace" => "logging".to_string(),
            "dec" => "decimal".to_string(),
            "select" => "async".to_string(),
            "distributed_actor" | "define_indicator_enums" | "generate_builder" => "custom".to_string(),
            _ => "declarative".to_string(),
        }
    }
    
    fn extract_macro_content(&self, node: Node<'a>) -> Option<String> {
        // Extract the token tree or content of the macro
        if let Some(token_tree) = node.child_by_field_name("token_tree") {
            safe_node_text(token_tree, self.source).map(|s| s.to_string())
        } else {
            None
        }
    }
    
    fn generate_paste_synthetic_calls(&self, node: Node<'a>, symbols: &mut ParsedSymbols) {
        // Generate synthetic calls from paste! macro patterns
        // This is a simplified implementation - full support would require macro expansion
        
        if let Some(content) = self.extract_macro_content(node) {
            // Look for patterns like [<$indicator>]::new or [<$indicator Input>]::from_ohlcv
            if content.contains("::new") || content.contains("::from_ohlcv") {
                // Generate a synthetic call
                let line = get_line_range(node).0;
                let synthetic_call = FunctionCall {
                    caller_id: self.get_containing_function().unwrap_or_else(|| "global".to_string()),
                    caller_module: self.scope_stack.module_path().join("::"),
                    callee_name: if content.contains("from_ohlcv") { "from_ohlcv" } else { "new" }.to_string(),
                    qualified_callee: None,
                    call_type: CallType::Direct,
                    line: line,
                    cross_crate: false,
                    from_crate: self.crate_name.clone(),
                    to_crate: None,
                    file_path: self.file_path.to_string_lossy().to_string(),
                    is_synthetic: true,
                    macro_context: Some(MacroContext {
                        expansion_id: format!("{}:{}:paste", self.file_path.display(), line),
                        macro_type: "paste".to_string(),
                        expansion_site_line: line,
                        name: "paste".to_string(),
                        kind: "paste".to_string(),
                    }),
                    synthetic_confidence: 0.9,
                };
                symbols.calls.push(synthetic_call);
            }
        }
    }
    
    fn process_custom_macro_pattern(&self, node: Node<'a>, name: &str, symbols: &mut ParsedSymbols) {
        // Process custom macro patterns from trading-backend-poc
        match name {
            "define_indicator_enums" => {
                // Extract indicator names from the macro arguments
                let args = self.extract_macro_arguments(node);
                if !args.is_empty() {
                    // Parse indicator names from pattern like "Rsi: \"description\", Ema: \"description\""
                    for part in args.split(',') {
                        if let Some(indicator_name) = part.split(':').next() {
                            let indicator_name = indicator_name.trim();
                            // Could generate synthetic types or track indicators
                        }
                    }
                }
            }
            "generate_builder" => {
                // Extract exchange and strategy names
                // Pattern: generate_builder!(bybit, BybitExchange, divergence_dev, DivergenceDevStrategy)
                let args = self.extract_macro_arguments(node);
                if !args.is_empty() {
                    let parts: Vec<&str> = args.split(',').map(|s| s.trim()).collect();
                    if parts.len() >= 4 {
                        // Could generate synthetic builder type
                        // e.g., BybitDivergenceDevBuilder
                    }
                }
            }
            _ => {}
        }
    }

    fn get_containing_function(&self) -> Option<String> {
        for frame in self.scope_stack.frames.iter().rev() {
            if let ContextFrame::Function { name, .. } = frame {
                return Some(self.scope_stack.qualified_name(name));
            }
        }
        None
    }

    fn extract_macro_arguments(&self, node: Node<'a>) -> String {
        // Extract the token tree containing macro arguments
        if let Some(token_tree) = find_child_of_type(node, &["token_tree"]) {
            safe_node_text(token_tree, self.source)
                .unwrap_or("")
                .to_string()
        } else {
            String::new()
        }
    }

    fn determine_call_type(&self, function_node: Node<'a>) -> CallType {
        if let Some(text) = safe_node_text(function_node, self.source) {
            if text.contains("::") {
                CallType::Associated
            } else if text.contains(".") {
                CallType::Method
            } else {
                CallType::Direct
            }
        } else {
            CallType::Direct
        }
    }
    
    /// Check if we're currently in a trait context
    fn in_trait_context(&self) -> bool {
        self.scope_stack.frames.iter().any(|frame| {
            matches!(frame, ContextFrame::Trait { .. })
        })
    }
    
    /// Process trait body for method declarations
    fn process_trait_body(&mut self, node: Node<'a>, symbols: &mut ParsedSymbols) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "function_signature_item" => {
                    self.process_trait_method_signature(child, symbols);
                },
                "function_item" => {
                    self.process_function(child, symbols);
                },
                _ => {
                    self.walk_node(child, symbols);
                }
            }
        }
    }
    
    /// Process a trait method signature (function without body)
    fn process_trait_method_signature(&mut self, node: Node<'a>, symbols: &mut ParsedSymbols) {
        let context = self.scope_stack.current_context();
        let name = self.extract_function_name(node);
        let qualified = self.scope_stack.qualified_name(&name);
        
        // Check if this is an async function
        let is_async = is_async_function(node, self.source);
        
        // Trait method signatures are always in a trait declaration context
        let is_trait_impl = false;
        
        // Determine if this is a method (has &self, &mut self, or self parameter)
        let is_method = self.is_method_function(node);
        
        // Extract parameters and return type
        let parameters = self.extract_parameters(node);
        let return_type = self.extract_return_type(node);
        let signature = safe_node_text(node, self.source).unwrap_or("").to_string();
        
        let function = RustFunction {
            id: format!("{}:{}:{}", self.file_path.display(), get_line_range(node).0, name),
            name: name.clone(),
            qualified_name: qualified,
            crate_name: self.crate_name.clone(),
            module_path: self.scope_stack.module_path().join("::"),
            file_path: self.file_path.to_string_lossy().to_string(),
            line_start: get_line_range(node).0,
            line_end: get_line_range(node).1,
            visibility: extract_visibility(node, self.source),
            is_async,
            is_unsafe: has_keyword(node, "unsafe", self.source),
            is_generic: !extract_generics(node, self.source).is_empty(),
            is_test: has_attribute(node, self.source, "test"),
            is_trait_impl,
            is_method,
            function_context: context,
            doc_comment: extract_doc_comment(node, self.source),
            signature,
            parameters,
            return_type,
            embedding_text: None,
            module: self.scope_stack.module_path().join("::"),
        };
        
        symbols.functions.push(function);
    }
    
    /// Process a type alias declaration
    fn process_type_alias(&mut self, node: Node<'a>, symbols: &mut ParsedSymbols) {
        let name = extract_field_text(node, "name", self.source);
        let qualified = self.scope_stack.qualified_name(&name);
        
        // Extract the aliased type
        let aliased_type = if let Some(type_node) = node.child_by_field_name("type") {
            safe_node_text(type_node, self.source).unwrap_or("").to_string()
        } else {
            String::new()
        };
        
        // Create a RustType for the type alias
        let type_alias = RustType {
            id: format!("{}:{}:{}", self.file_path.display(), get_line_range(node).0, name),
            name: name.clone(),
            qualified_name: qualified,
            crate_name: self.crate_name.clone(),
            module_path: self.scope_stack.module_path().join("::"),
            file_path: self.file_path.to_string_lossy().to_string(),
            line_start: get_line_range(node).0,
            line_end: get_line_range(node).1,
            kind: TypeKind::TypeAlias,
            visibility: extract_visibility(node, self.source),
            is_generic: !extract_generics(node, self.source).is_empty(),
            is_test: false,
            doc_comment: extract_doc_comment(node, self.source),
            fields: Vec::new(),
            variants: Vec::new(),
            methods: Vec::new(),
            embedding_text: None,
            type_kind: "TypeAlias".to_string(),
            module: self.scope_stack.module_path().join("::"),
        };
        
        symbols.types.push(type_alias);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    fn setup_parser() -> Parser {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::language()).unwrap();
        parser
    }

    #[test]
    fn test_scope_stack_basic() {
        let mut stack = ScopeStack::new();
        
        // Test module context
        stack.push(ContextFrame::Module { 
            name: "test_mod".to_string(), 
            is_inline: false 
        });
        
        assert_eq!(stack.module_path(), vec!["test_mod"]);
        assert_eq!(stack.qualified_name("func"), "test_mod::func");
        
        stack.pop();
        assert!(stack.module_path().is_empty());
    }

    #[test]
    fn test_impl_context() {
        let mut stack = ScopeStack::new();
        
        stack.push(ContextFrame::Impl {
            type_name: "MyStruct".to_string(),
            trait_name: Some("Display".to_string()),
            generics: vec![],
        });
        
        let context = stack.current_context();
        assert!(matches!(context, FunctionContext::TraitImpl { .. }));
        assert!(stack.in_trait_impl());
        
        stack.pop();
        let context = stack.current_context();
        assert!(matches!(context, FunctionContext::Free));
    }

    #[test]
    fn test_unified_walker_simple() {
        let mut parser = setup_parser();
        let source = b"fn test_function() { println!(\"Hello\"); }";
        let tree = parser.parse(source, None).unwrap();
        
        let mut walker = UnifiedWalker::new(
            source,
            "test_crate".to_string(),
            PathBuf::from("test.rs")
        );
        
        let symbols = walker.walk(tree.root_node());
        
        assert_eq!(symbols.functions.len(), 1);
        assert_eq!(symbols.functions[0].name, "test_function");
        assert!(!symbols.functions[0].is_trait_impl);
    }

    #[test]
    fn test_trait_impl_detection() {
        let mut parser = setup_parser();
        let source = b"impl Display for MyType { fn fmt(&self) -> String { String::new() } }";
        let tree = parser.parse(source, None).unwrap();
        
        let mut walker = UnifiedWalker::new(
            source,
            "test_crate".to_string(),
            PathBuf::from("test.rs")
        );
        
        let symbols = walker.walk(tree.root_node());
        
        assert_eq!(symbols.functions.len(), 1);
        assert_eq!(symbols.functions[0].name, "fmt");
        assert!(symbols.functions[0].is_trait_impl);
        assert!(symbols.functions[0].is_method);
    }
}