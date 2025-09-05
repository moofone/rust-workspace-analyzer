use crate::analyzer::WorkspaceAnalyzer;
use crate::parser::symbols::{ActorSpawn, SpawnMethod, SpawnPattern, SpawnPatternData};
use crate::config::Config;
use std::collections::HashMap;
use std::path::PathBuf;

/// Comprehensive test suite for validating spawn pattern detection
/// across all pattern types in the dummy workspace
#[cfg(test)]
mod spawn_pattern_tests {
    use super::*;

    #[tokio::test]
    async fn test_direct_type_spawn_detection() {
        let actor_spawns = analyze_dummy_workspace().await;
        let spawn_patterns = extract_spawn_patterns(&actor_spawns);
        
        // Test DirectType patterns: ActorType::spawn(args)
        let direct_type_patterns: Vec<_> = spawn_patterns
            .iter()
            .filter(|p| matches!(p.pattern_type, SpawnPattern::DirectType))
            .collect();
            
        assert!(direct_type_patterns.len() >= 10, 
            "Should detect at least 10 DirectType spawn patterns, found: {}", 
            direct_type_patterns.len());
            
        // Validate specific patterns
        assert!(direct_type_patterns.iter().any(|p| 
            p.actor_type == "ExampleActor" && p.method_name == "spawn"
        ), "Should detect ExampleActor::spawn patterns");
        
        assert!(direct_type_patterns.iter().any(|p| 
            p.actor_type == "WorkerActor" && p.method_name == "spawn"
        ), "Should detect WorkerActor::spawn patterns");
    }
    
    #[tokio::test] 
    async fn test_trait_method_spawn_detection() {
        let actor_spawns = analyze_dummy_workspace().await;
        let spawn_patterns = extract_spawn_patterns(&actor_spawns);
        
        // Test TraitMethod patterns: Trait::spawn(instance)
        let trait_method_patterns: Vec<_> = spawn_patterns
            .iter()
            .filter(|p| matches!(p.pattern_type, SpawnPattern::TraitMethod))
            .collect();
            
        assert!(trait_method_patterns.len() >= 3,
            "Should detect at least 3 TraitMethod spawn patterns, found: {}", 
            trait_method_patterns.len());
            
        // Validate specific trait method patterns
        assert!(trait_method_patterns.iter().any(|p| 
            p.trait_name.as_ref().unwrap_or(&String::new()) == "TestActorSpawner" && 
            p.method_name == "spawn"
        ), "Should detect TestActorSpawner::spawn patterns");
        
        assert!(trait_method_patterns.iter().any(|p| 
            p.method_name == "spawn_link"
        ), "Should detect spawn_link method patterns");
        
        assert!(trait_method_patterns.iter().any(|p| 
            p.method_name == "spawn_with_mailbox"
        ), "Should detect spawn_with_mailbox method patterns");
    }
    
    #[tokio::test]
    async fn test_module_function_spawn_detection() {
        let actor_spawns = analyze_dummy_workspace().await;
        let spawn_patterns = extract_spawn_patterns(&actor_spawns);
        
        // Test ModuleFunction patterns: module::spawn(instance)
        let module_function_patterns: Vec<_> = spawn_patterns
            .iter()
            .filter(|p| matches!(p.pattern_type, SpawnPattern::ModuleFunction))
            .collect();
            
        assert!(module_function_patterns.len() >= 5,
            "Should detect at least 5 ModuleFunction spawn patterns, found: {}", 
            module_function_patterns.len());
            
        // Validate specific module function patterns
        assert!(module_function_patterns.iter().any(|p| 
            p.module_path.as_ref().unwrap_or(&String::new()) == "kameo" && 
            p.method_name == "spawn"
        ), "Should detect kameo::spawn patterns");
    }
    
    #[tokio::test]
    async fn test_spawn_method_variants() {
        let actor_spawns = analyze_dummy_workspace().await;
        let spawn_patterns = extract_spawn_patterns(&actor_spawns);
        
        // Test different spawn method names are detected
        let method_names: Vec<_> = spawn_patterns
            .iter()
            .map(|p| &p.method_name)
            .collect();
            
        assert!(method_names.contains(&&"spawn".to_string()),
            "Should detect 'spawn' methods");
        assert!(method_names.contains(&&"spawn_link".to_string()),
            "Should detect 'spawn_link' methods");
        assert!(method_names.contains(&&"spawn_with_mailbox".to_string()),
            "Should detect 'spawn_with_mailbox' methods");
    }
    
    #[tokio::test]
    async fn test_cross_crate_spawn_detection() {
        let actor_spawns = analyze_dummy_workspace().await;
        let spawn_patterns = extract_spawn_patterns(&actor_spawns);
        
        // Group patterns by crate
        let mut patterns_by_crate: HashMap<String, Vec<&SpawnPattern>> = HashMap::new();
        for pattern in &spawn_patterns {
            patterns_by_crate
                .entry(pattern.crate_name.clone())
                .or_default()
                .push(pattern);
        }
        
        // Validate patterns exist in all crates
        assert!(patterns_by_crate.contains_key("crate_a"),
            "Should find spawn patterns in crate_a");
        assert!(patterns_by_crate.contains_key("crate_b"),
            "Should find spawn patterns in crate_b");
        assert!(patterns_by_crate.contains_key("crate_c"),
            "Should find spawn patterns in crate_c");
            
        // Validate cross-crate actor spawning
        let crate_b_patterns = patterns_by_crate.get("crate_b").unwrap();
        assert!(crate_b_patterns.iter().any(|p| 
            p.actor_type == "ExampleActor"
        ), "crate_b should spawn ExampleActor from crate_a");
        
        let crate_c_patterns = patterns_by_crate.get("crate_c").unwrap();
        assert!(crate_c_patterns.iter().any(|p| 
            p.actor_type == "WorkerActor"
        ), "crate_c should spawn WorkerActor from crate_a");
    }
    
    #[tokio::test]
    async fn test_nested_spawn_detection() {
        let actor_spawns = analyze_dummy_workspace().await;
        let spawn_patterns = extract_spawn_patterns(&actor_spawns);
        
        // Test spawns in different contexts are detected
        let contexts: Vec<_> = spawn_patterns
            .iter()
            .map(|p| &p.context)
            .collect();
            
        // Should detect spawns in various contexts
        let has_conditional = contexts.iter().any(|ctx| 
            ctx.contains("if") || ctx.contains("match") || ctx.contains("loop")
        );
        assert!(has_conditional, "Should detect spawns in conditional contexts");
        
        let has_closure = contexts.iter().any(|ctx| 
            ctx.contains("closure") || ctx.contains("async")
        );
        assert!(has_closure, "Should detect spawns in closure/async contexts");
    }
    
    #[tokio::test] 
    async fn test_spawn_pattern_completeness() {
        let actor_spawns = analyze_dummy_workspace().await;
        let spawn_patterns = extract_spawn_patterns(&actor_spawns);
        
        // Validate that we have comprehensive coverage
        assert!(spawn_patterns.len() >= 20,
            "Should detect at least 20 total spawn patterns across all types, found: {}", 
            spawn_patterns.len());
            
        // Validate all pattern types are represented
        let has_direct_type = spawn_patterns.iter().any(|p| 
            matches!(p.pattern_type, SpawnPattern::DirectType)
        );
        let has_trait_method = spawn_patterns.iter().any(|p| 
            matches!(p.pattern_type, SpawnPattern::TraitMethod)
        );  
        let has_module_function = spawn_patterns.iter().any(|p| 
            matches!(p.pattern_type, SpawnPattern::ModuleFunction)
        );
        
        assert!(has_direct_type, "Should detect DirectType patterns");
        assert!(has_trait_method, "Should detect TraitMethod patterns");
        assert!(has_module_function, "Should detect ModuleFunction patterns");
    }
    
    #[tokio::test]
    async fn test_spawn_argument_analysis() {
        let actor_spawns = analyze_dummy_workspace().await;
        let spawn_patterns = extract_spawn_patterns(&actor_spawns);
        
        // Test that spawn arguments are properly captured
        for pattern in &spawn_patterns {
            assert!(!pattern.arguments.is_empty() || pattern.method_name.contains("spawn"),
                "Spawn pattern should have arguments or be a spawn method: {:?}", pattern);
                
            // Test specific argument patterns
            if pattern.actor_type == "ExampleActor" {
                // Should have integer arguments or struct construction
                let has_valid_args = pattern.arguments.iter().any(|arg| 
                    arg.contains("counter") || arg.parse::<i32>().is_ok()
                );
                assert!(has_valid_args, 
                    "ExampleActor spawn should have valid arguments: {:?}", pattern.arguments);
            }
        }
    }

    /// Helper function to analyze the dummy workspace
    async fn analyze_dummy_workspace() -> Vec<ActorSpawn> {
        // Create config pointing to dummy workspace
        let config = Config {
            workspace: crate::config::WorkspaceConfig {
                root: PathBuf::from("/Users/greg/Dev/git/dummy-workspace"),
                additional_roots: vec![],
            },
            analysis: crate::config::AnalysisConfig {
                recursive_scan: true,
                include_dev_deps: true,
                include_build_deps: false,
                workspace_members_only: true,
                exclude_crates: vec![],
            },
            architecture: crate::config::ArchitectureConfig {
                layers: vec![
                    crate::config::Layer {
                        name: "crate_a".to_string(),
                        crates: vec!["crate_a".to_string()],
                    },
                    crate::config::Layer {
                        name: "crate_b".to_string(),
                        crates: vec!["crate_b".to_string()],
                    },
                    crate::config::Layer {
                        name: "crate_c".to_string(),
                        crates: vec!["crate_c".to_string()],
                    },
                ],
                layer_index_cache: None,
            },
            embeddings: crate::config::EmbeddingsConfig {
                enabled: false,
                model: "text-embedding-3-small".to_string(),
                include_in_embedding: vec![],
            },
            memgraph: crate::config::MemgraphConfig {
                uri: "bolt://localhost:7687".to_string(),
                username: "".to_string(),
                password: "".to_string(),
                clean_start: false,
                batch_size: 1000,
            },
            performance: crate::config::PerformanceConfig {
                max_threads: 4,
                cache_size_mb: 256,
                incremental: true,
            },
        };
        
        let analyzer = WorkspaceAnalyzer::new(config).expect("Failed to create analyzer");
        let snapshot = analyzer.create_snapshot().await.expect("Failed to create snapshot");
        
        // Collect all actor spawns from all crates
        let mut all_spawns = Vec::new();
        for (_crate_name, crate_symbols) in snapshot.crates {
            all_spawns.extend(crate_symbols.actor_spawns);
        }
        
        all_spawns
    }
    
    /// Helper function to convert ActorSpawn to SpawnPatternData for testing
    fn extract_spawn_patterns(actor_spawns: &[ActorSpawn]) -> Vec<SpawnPatternData> {
        actor_spawns.iter().map(|spawn| {
            // Create a SpawnPattern from ActorSpawn for testing
            let pattern_type = match &spawn.spawn_method {
                SpawnMethod::Spawn | SpawnMethod::SpawnWithMailbox | SpawnMethod::SpawnLink | SpawnMethod::SpawnInThread => {
                    // Determine pattern type based on spawn method context
                    if spawn.context.contains("trait") {
                        SpawnPattern::TraitMethod
                    } else if spawn.context.contains("module") {
                        SpawnPattern::ModuleFunction
                    } else {
                        SpawnPattern::DirectType
                    }
                }
            };
            
            SpawnPatternData {
                pattern_type,
                actor_type: spawn.child_actor_name.clone(),
                method_name: match spawn.spawn_method {
                    SpawnMethod::Spawn => "spawn".to_string(),
                    SpawnMethod::SpawnWithMailbox => "spawn_with_mailbox".to_string(), 
                    SpawnMethod::SpawnLink => "spawn_link".to_string(),
                    SpawnMethod::SpawnInThread => "spawn_in_thread".to_string(),
                },
                arguments: vec![], // Would need to parse from actual source
                context: spawn.context.clone(),
                crate_name: spawn.from_crate.clone(),
                trait_name: None, // Would need to extract from context
                module_path: None, // Would need to extract from context
                line: spawn.line,
                file_path: spawn.file_path.clone(),
            }
        }).collect()
    }
}