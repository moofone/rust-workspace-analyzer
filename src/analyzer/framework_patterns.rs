use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Framework-specific patterns for detecting runtime calls and synthetic references
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkPatterns {
    pub entry_points: Vec<EntryPointPattern>,
    pub runtime_calls: Vec<RuntimeCallPattern>,
    pub trait_dispatch: Vec<TraitDispatchPattern>,
    pub actor_patterns: Vec<ActorPattern>,
    #[serde(skip)]
    pub compiled_patterns: Option<CompiledPatterns>,
}

/// Pattern for framework entry points (main functions, server starts, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPointPattern {
    pub name: String,
    pub description: String,
    pub framework: String,
    pub pattern_type: EntryPointType,
    pub function_pattern: String,
    pub triggers_methods: Vec<String>,
    pub context_conditions: Vec<String>,
}

/// Pattern for runtime method invocations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeCallPattern {
    pub name: String,
    pub description: String,
    pub framework: String,
    pub pattern_type: RuntimeCallType,
    pub caller_pattern: String,
    pub target_pattern: String,
    pub method_patterns: Vec<String>,
    pub conditions: Vec<String>,
}

/// Pattern for trait method dispatch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitDispatchPattern {
    pub name: String,
    pub description: String,
    pub framework: String,
    pub trait_name: String,
    pub dispatch_method: String,
    pub target_methods: Vec<String>,
    pub dispatch_conditions: Vec<String>,
}

/// Pattern for actor system method calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorPattern {
    pub name: String,
    pub description: String,
    pub framework: String,
    pub actor_trait: String,
    pub lifecycle_methods: Vec<String>,
    pub message_handlers: Vec<String>,
    pub spawn_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntryPointType {
    Main,
    ServerStart,
    ActorSpawn,
    TaskSpawn,
    RuntimeBuilder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeCallType {
    AsyncTaskSpawn,
    ActorMessage,
    TraitObjectCall,
    FrameworkCallback,
    EventHandler,
}

/// Compiled regex patterns for performance
#[derive(Debug)]
pub struct CompiledPatterns {
    pub entry_point_regexes: HashMap<String, Regex>,
    pub runtime_call_regexes: HashMap<String, Regex>,
    pub trait_dispatch_regexes: HashMap<String, Regex>,
    pub actor_regexes: HashMap<String, Regex>,
}

impl Clone for CompiledPatterns {
    fn clone(&self) -> Self {
        Self {
            entry_point_regexes: self.entry_point_regexes.iter()
                .map(|(k, v)| (k.clone(), Regex::new(v.as_str()).unwrap_or_else(|_| Regex::new("").unwrap())))
                .collect(),
            runtime_call_regexes: self.runtime_call_regexes.iter()
                .map(|(k, v)| (k.clone(), Regex::new(v.as_str()).unwrap_or_else(|_| Regex::new("").unwrap())))
                .collect(),
            trait_dispatch_regexes: self.trait_dispatch_regexes.iter()
                .map(|(k, v)| (k.clone(), Regex::new(v.as_str()).unwrap_or_else(|_| Regex::new("").unwrap())))
                .collect(),
            actor_regexes: self.actor_regexes.iter()
                .map(|(k, v)| (k.clone(), Regex::new(v.as_str()).unwrap_or_else(|_| Regex::new("").unwrap())))
                .collect(),
        }
    }
}

impl FrameworkPatterns {
    /// Create new empty framework patterns
    pub fn new() -> Self {
        Self {
            entry_points: Vec::new(),
            runtime_calls: Vec::new(),
            trait_dispatch: Vec::new(),
            actor_patterns: Vec::new(),
            compiled_patterns: None,
        }
    }

    /// Create patterns with default frameworks (actix-web, tokio, async-std)
    pub fn with_default_patterns() -> Self {
        let mut patterns = Self::new();
        patterns.add_tokio_patterns();
        patterns.add_actix_web_patterns();
        patterns.add_async_std_patterns();
        patterns.add_websocket_patterns();
        patterns.compile_patterns().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to compile framework patterns: {}", e);
        });
        patterns
    }

    /// Add Tokio framework patterns
    pub fn add_tokio_patterns(&mut self) {
        // Tokio spawn patterns
        self.runtime_calls.push(RuntimeCallPattern {
            name: "tokio_spawn".to_string(),
            description: "Tokio task spawning".to_string(),
            framework: "tokio".to_string(),
            pattern_type: RuntimeCallType::AsyncTaskSpawn,
            caller_pattern: r"tokio::spawn|spawn".to_string(),
            target_pattern: r"async\s+move\s*\{|async\s+\|".to_string(),
            method_patterns: vec![".*".to_string()],
            conditions: vec!["use tokio".to_string(), "tokio::".to_string()],
        });

        // Tokio runtime builder
        self.entry_points.push(EntryPointPattern {
            name: "tokio_main".to_string(),
            description: "Tokio main function".to_string(),
            framework: "tokio".to_string(),
            pattern_type: EntryPointType::Main,
            function_pattern: r"#\[tokio::main\]".to_string(),
            triggers_methods: vec!["main".to_string()],
            context_conditions: vec!["tokio".to_string()],
        });

        // Tokio select patterns
        self.runtime_calls.push(RuntimeCallPattern {
            name: "tokio_select".to_string(),
            description: "Tokio select! macro".to_string(),
            framework: "tokio".to_string(),
            pattern_type: RuntimeCallType::EventHandler,
            caller_pattern: r"tokio::select!|select!".to_string(),
            target_pattern: r"=>".to_string(),
            method_patterns: vec![".*".to_string()],
            conditions: vec!["tokio".to_string()],
        });
    }

    /// Add Actix-Web framework patterns
    pub fn add_actix_web_patterns(&mut self) {
        // Actor start pattern
        self.entry_points.push(EntryPointPattern {
            name: "actix_actor_start".to_string(),
            description: "Actix Actor start method".to_string(),
            framework: "actix-web".to_string(),
            pattern_type: EntryPointType::ActorSpawn,
            function_pattern: r"\.start\(\)".to_string(),
            triggers_methods: vec!["started".to_string(), "stopping".to_string(), "stopped".to_string()],
            context_conditions: vec!["actix".to_string(), "Actor".to_string()],
        });

        // Actor trait dispatch
        self.trait_dispatch.push(TraitDispatchPattern {
            name: "actix_actor_trait".to_string(),
            description: "Actix Actor trait methods".to_string(),
            framework: "actix-web".to_string(),
            trait_name: "Actor".to_string(),
            dispatch_method: "started|stopping|stopped".to_string(),
            target_methods: vec!["started".to_string(), "stopping".to_string(), "stopped".to_string()],
            dispatch_conditions: vec!["impl.*Actor".to_string()],
        });

        // Handler trait dispatch
        self.trait_dispatch.push(TraitDispatchPattern {
            name: "actix_handler_trait".to_string(),
            description: "Actix Handler trait methods".to_string(),
            framework: "actix-web".to_string(),
            trait_name: "Handler".to_string(),
            dispatch_method: "handle".to_string(),
            target_methods: vec!["handle".to_string()],
            dispatch_conditions: vec!["impl.*Handler".to_string()],
        });

        // Actor pattern
        self.actor_patterns.push(ActorPattern {
            name: "actix_actor".to_string(),
            description: "Actix Actor system".to_string(),
            framework: "actix-web".to_string(),
            actor_trait: "Actor".to_string(),
            lifecycle_methods: vec!["started".to_string(), "stopping".to_string(), "stopped".to_string()],
            message_handlers: vec!["handle".to_string()],
            spawn_patterns: vec![r"\.start\(\)".to_string()],
        });
    }

    /// Add async-std framework patterns
    pub fn add_async_std_patterns(&mut self) {
        self.runtime_calls.push(RuntimeCallPattern {
            name: "async_std_spawn".to_string(),
            description: "async-std task spawning".to_string(),
            framework: "async-std".to_string(),
            pattern_type: RuntimeCallType::AsyncTaskSpawn,
            caller_pattern: r"async_std::task::spawn|task::spawn".to_string(),
            target_pattern: r"async\s+move\s*\{|async\s+\|".to_string(),
            method_patterns: vec![".*".to_string()],
            conditions: vec!["async_std".to_string()],
        });
    }

    /// Add WebSocket framework patterns
    pub fn add_websocket_patterns(&mut self) {
        // WebSocket actor trait
        self.trait_dispatch.push(TraitDispatchPattern {
            name: "websocket_actor".to_string(),
            description: "WebSocket actor trait methods".to_string(),
            framework: "websocket".to_string(),
            trait_name: "WebSocketActor".to_string(),
            dispatch_method: "event_stream|handle_message".to_string(),
            target_methods: vec!["event_stream".to_string(), "handle_message".to_string()],
            dispatch_conditions: vec!["impl.*WebSocketActor".to_string()],
        });

        // WebSocket actor pattern
        self.actor_patterns.push(ActorPattern {
            name: "websocket_actor".to_string(),
            description: "WebSocket actor system".to_string(),
            framework: "websocket".to_string(),
            actor_trait: "WebSocketActor".to_string(),
            lifecycle_methods: vec![],
            message_handlers: vec!["event_stream".to_string(), "handle_message".to_string()],
            spawn_patterns: vec![],
        });
    }

    /// Compile all patterns into regex for performance
    pub fn compile_patterns(&mut self) -> Result<()> {
        let mut entry_point_regexes = HashMap::new();
        let mut runtime_call_regexes = HashMap::new();
        let mut trait_dispatch_regexes = HashMap::new();
        let mut actor_regexes = HashMap::new();

        // Compile entry point patterns
        for pattern in &self.entry_points {
            let regex = Regex::new(&pattern.function_pattern)
                .map_err(|e| anyhow::anyhow!("Invalid entry point regex '{}': {}", pattern.function_pattern, e))?;
            entry_point_regexes.insert(pattern.name.clone(), regex);
        }

        // Compile runtime call patterns
        for pattern in &self.runtime_calls {
            let caller_regex = Regex::new(&pattern.caller_pattern)
                .map_err(|e| anyhow::anyhow!("Invalid runtime call regex '{}': {}", pattern.caller_pattern, e))?;
            runtime_call_regexes.insert(format!("{}_caller", pattern.name), caller_regex);
            
            let target_regex = Regex::new(&pattern.target_pattern)
                .map_err(|e| anyhow::anyhow!("Invalid runtime target regex '{}': {}", pattern.target_pattern, e))?;
            runtime_call_regexes.insert(format!("{}_target", pattern.name), target_regex);
        }

        // Compile trait dispatch patterns
        for pattern in &self.trait_dispatch {
            let dispatch_regex = Regex::new(&pattern.dispatch_method)
                .map_err(|e| anyhow::anyhow!("Invalid trait dispatch regex '{}': {}", pattern.dispatch_method, e))?;
            trait_dispatch_regexes.insert(pattern.name.clone(), dispatch_regex);
        }

        // Compile actor patterns
        for pattern in &self.actor_patterns {
            for spawn_pattern in &pattern.spawn_patterns {
                let spawn_regex = Regex::new(spawn_pattern)
                    .map_err(|e| anyhow::anyhow!("Invalid actor spawn regex '{}': {}", spawn_pattern, e))?;
                actor_regexes.insert(format!("{}_spawn", pattern.name), spawn_regex);
            }
        }

        self.compiled_patterns = Some(CompiledPatterns {
            entry_point_regexes,
            runtime_call_regexes,
            trait_dispatch_regexes,
            actor_regexes,
        });

        Ok(())
    }

    /// Validate pattern complexity to prevent regex DoS
    pub fn validate_patterns(&self) -> Result<()> {
        for pattern in &self.entry_points {
            self.validate_pattern_complexity(&pattern.function_pattern, "entry point")?;
        }

        for pattern in &self.runtime_calls {
            self.validate_pattern_complexity(&pattern.caller_pattern, "runtime call caller")?;
            self.validate_pattern_complexity(&pattern.target_pattern, "runtime call target")?;
        }

        for pattern in &self.trait_dispatch {
            self.validate_pattern_complexity(&pattern.dispatch_method, "trait dispatch")?;
        }

        for pattern in &self.actor_patterns {
            for spawn_pattern in &pattern.spawn_patterns {
                self.validate_pattern_complexity(spawn_pattern, "actor spawn")?;
            }
        }

        Ok(())
    }

    /// Validate individual pattern complexity
    fn validate_pattern_complexity(&self, pattern: &str, pattern_type: &str) -> Result<()> {
        // Check for dangerous patterns that could cause ReDoS
        let dangerous_patterns = [
            r"\.\*\.\*",
            r"\.\+\.\+",
            r"\(\.\*\)\+",
            r"\(\.\+\)\+",
            r"\[\.\*\]",
            r"\[\.\+\]",
        ];

        for dangerous in &dangerous_patterns {
            if pattern.contains(dangerous) {
                return Err(anyhow::anyhow!(
                    "Pattern '{}' for {} contains potentially dangerous regex '{}' that could cause ReDoS",
                    pattern, pattern_type, dangerous
                ));
            }
        }

        // Limit pattern length
        if pattern.len() > 1000 {
            return Err(anyhow::anyhow!(
                "Pattern '{}' for {} is too long ({} chars), maximum is 1000",
                pattern, pattern_type, pattern.len()
            ));
        }

        Ok(())
    }

    /// Find matching entry points in code
    pub fn find_entry_points(&self, code: &str) -> Vec<&EntryPointPattern> {
        let mut matches = Vec::new();

        if let Some(compiled) = &self.compiled_patterns {
            for pattern in &self.entry_points {
                if let Some(regex) = compiled.entry_point_regexes.get(&pattern.name) {
                    if regex.is_match(code) {
                        matches.push(pattern);
                    }
                }
            }
        }

        matches
    }

    /// Find matching runtime calls in code
    pub fn find_runtime_calls(&self, code: &str) -> Vec<&RuntimeCallPattern> {
        let mut matches = Vec::new();

        if let Some(compiled) = &self.compiled_patterns {
            for pattern in &self.runtime_calls {
                if let Some(caller_regex) = compiled.runtime_call_regexes.get(&format!("{}_caller", pattern.name)) {
                    if caller_regex.is_match(code) {
                        matches.push(pattern);
                    }
                }
            }
        }

        matches
    }

    /// Find matching trait dispatch patterns in code
    pub fn find_trait_dispatches(&self, code: &str) -> Vec<&TraitDispatchPattern> {
        let mut matches = Vec::new();

        if let Some(compiled) = &self.compiled_patterns {
            for pattern in &self.trait_dispatch {
                if let Some(regex) = compiled.trait_dispatch_regexes.get(&pattern.name) {
                    if regex.is_match(code) {
                        matches.push(pattern);
                    }
                }
            }
        }

        matches
    }

    /// Find matching actor patterns in code
    pub fn find_actor_patterns(&self, code: &str) -> Vec<&ActorPattern> {
        let mut matches = Vec::new();

        if let Some(compiled) = &self.compiled_patterns {
            for pattern in &self.actor_patterns {
                for spawn_pattern in &pattern.spawn_patterns {
                    if let Some(regex) = compiled.actor_regexes.get(&format!("{}_spawn", pattern.name)) {
                        if regex.is_match(code) {
                            matches.push(pattern);
                            break; // Don't add the same pattern multiple times
                        }
                    }
                }
            }
        }

        matches
    }

    /// Get statistics about the patterns
    pub fn stats(&self) -> PatternStats {
        PatternStats {
            entry_points: self.entry_points.len(),
            runtime_calls: self.runtime_calls.len(),
            trait_dispatches: self.trait_dispatch.len(),
            actor_patterns: self.actor_patterns.len(),
            compiled: self.compiled_patterns.is_some(),
        }
    }
}

impl Default for FrameworkPatterns {
    fn default() -> Self {
        Self::with_default_patterns()
    }
}

#[derive(Debug)]
pub struct PatternStats {
    pub entry_points: usize,
    pub runtime_calls: usize,
    pub trait_dispatches: usize,
    pub actor_patterns: usize,
    pub compiled: bool,
}

impl std::fmt::Display for PatternStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "Framework Pattern Stats:\n\
             - Entry Points: {}\n\
             - Runtime Calls: {}\n\
             - Trait Dispatches: {}\n\
             - Actor Patterns: {}\n\
             - Compiled: {}",
            self.entry_points,
            self.runtime_calls,
            self.trait_dispatches,
            self.actor_patterns,
            self.compiled
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framework_patterns_creation() {
        let patterns = FrameworkPatterns::with_default_patterns();
        assert!(patterns.entry_points.len() > 0);
        assert!(patterns.runtime_calls.len() > 0);
        assert!(patterns.trait_dispatch.len() > 0);
        assert!(patterns.actor_patterns.len() > 0);
        assert!(patterns.compiled_patterns.is_some());
    }

    #[test]
    fn test_pattern_validation() {
        let patterns = FrameworkPatterns::with_default_patterns();
        assert!(patterns.validate_patterns().is_ok());
    }

    #[test]
    fn test_tokio_spawn_detection() {
        let patterns = FrameworkPatterns::with_default_patterns();
        let code = r#"
            tokio::spawn(async move {
                some_function().await;
            });
        "#;
        
        let matches = patterns.find_runtime_calls(code);
        assert!(matches.len() > 0);
        assert!(matches.iter().any(|p| p.name == "tokio_spawn"));
    }

    #[test]
    fn test_websocket_actor_detection() {
        let patterns = FrameworkPatterns::with_default_patterns();
        let code = r#"
            impl WebSocketActor for MyActor {
                fn event_stream(&mut self) -> BoxStream<'_, Message> {
                    // implementation
                }
            }
        "#;
        
        let matches = patterns.find_trait_dispatches(code);
        assert!(matches.len() > 0);
        assert!(matches.iter().any(|p| p.name == "websocket_actor"));
    }

    #[test]
    fn test_dangerous_pattern_validation() {
        let mut patterns = FrameworkPatterns::new();
        patterns.entry_points.push(EntryPointPattern {
            name: "dangerous".to_string(),
            description: "Test".to_string(),
            framework: "test".to_string(),
            pattern_type: EntryPointType::Main,
            function_pattern: r"(.*)(.*)".to_string(), // Dangerous pattern
            triggers_methods: vec![],
            context_conditions: vec![],
        });

        assert!(patterns.validate_patterns().is_err());
    }
}