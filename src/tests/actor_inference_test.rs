use crate::parser::RustParser;
use std::path::PathBuf;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_actor_inference_from_spawns() {
        let mut parser = RustParser::new().expect("Failed to create parser");
        
        // Test source code with various spawn patterns
        let source = r#"
use kameo::Actor;

// This actor has an explicit impl Actor block
pub struct ExplicitActor;

impl Actor for ExplicitActor {
    type Msg = String;
    type State = ();
    type Reply = ();

    async fn handle(&mut self, msg: Self::Msg, ctx: &mut kameo::Context<Self>) -> Self::Reply {
        // Handle message
    }
}

// This actor is inferred from spawn calls but has no explicit impl
pub struct InferredActor {
    pub data: String,
}

// This actor uses derive macro
#[derive(Actor)]
pub struct DerivedActor {
    pub value: i32,
}

// Function that spawns various actors
pub fn spawn_actors() {
    // Direct spawn pattern - should infer InferredActor
    let inferred_ref = InferredActor::spawn(InferredActor { 
        data: "test".to_string() 
    });
    
    // Explicit actor spawn  
    let explicit_ref = ExplicitActor::spawn(ExplicitActor);
    
    // Derived actor spawn
    let derived_ref = DerivedActor::spawn(DerivedActor { value: 42 });
    
    // Actor trait method spawn
    let trait_ref = Actor::spawn(InferredActor {
        data: "trait_spawn".to_string()
    });
}

// Function using ActorRef<T> - should infer actor types
pub fn use_actor_refs() {
    let actor_ref: kameo::ActorRef<InferredActor> = get_actor_ref();
    let another_ref: kameo::ActorRef<DerivedActor> = get_another_ref();
}

fn get_actor_ref() -> kameo::ActorRef<InferredActor> {
    // Return a test actor reference
    InferredActor::spawn(InferredActor {
        data: "test_ref".to_string()
    })
}

fn get_another_ref() -> kameo::ActorRef<DerivedActor> {
    // Return a test actor reference
    DerivedActor::spawn(DerivedActor {
        value: 100
    })
}
        "#;
        
        let file_path = PathBuf::from("test_file.rs");
        let symbols = parser.parse_source(source, &file_path, "test_crate")
            .expect("Failed to parse source");
        
        // Check that actors were detected
        println!("Total actors detected: {}", symbols.actors.len());
        for actor in &symbols.actors {
            println!("  Actor: {} ({})", actor.name, actor.doc_comment.as_deref().unwrap_or(""));
        }
        
        // We should have detected at least:
        // 1. ExplicitActor (from impl Actor)
        // 2. DerivedActor (from #[derive(Actor)])  
        // 3. InferredActor (from spawn calls and ActorRef usage)
        assert!(symbols.actors.len() >= 3, 
            "Should detect at least 3 actors, found {}: {:?}", 
            symbols.actors.len(),
            symbols.actors.iter().map(|a| &a.name).collect::<Vec<_>>()
        );
        
        // Check that we have explicit actor
        let explicit_actor = symbols.actors.iter().find(|a| a.name == "ExplicitActor");
        assert!(explicit_actor.is_some(), "Should find ExplicitActor");
        
        // Check that we have derived actor
        let derived_actor = symbols.actors.iter().find(|a| a.name == "DerivedActor");
        assert!(derived_actor.is_some(), "Should find DerivedActor");
        
        // Check that we have inferred actor
        let inferred_actor = symbols.actors.iter().find(|a| a.name == "InferredActor");
        assert!(inferred_actor.is_some(), "Should find InferredActor");
        
        if let Some(inferred) = inferred_actor {
            assert!(inferred.doc_comment.is_some(), "Inferred actor should have doc comment");
            assert!(inferred.doc_comment.as_ref().unwrap().contains("Inferred"), 
                "Doc comment should indicate it's inferred");
        }
        
        // Check spawn patterns were detected
        println!("Total spawns detected: {}", symbols.actor_spawns.len());
        for spawn in &symbols.actor_spawns {
            println!("  Spawn: {} spawns {} via {:?} ({})", 
                spawn.parent_actor_name, 
                spawn.child_actor_name, 
                spawn.spawn_method,
                spawn.spawn_pattern);
        }
        
        // We should detect multiple spawn calls
        assert!(symbols.actor_spawns.len() >= 3, 
            "Should detect at least 3 spawn calls, found {}", 
            symbols.actor_spawns.len()
        );
        
        // Verify we have both DirectType and TraitMethod patterns
        let direct_patterns = symbols.actor_spawns.iter()
            .filter(|s| matches!(s.spawn_pattern, crate::parser::symbols::SpawnPattern::DirectType))
            .count();
        let trait_patterns = symbols.actor_spawns.iter()
            .filter(|s| matches!(s.spawn_pattern, crate::parser::symbols::SpawnPattern::TraitMethod))
            .count();
            
        assert!(direct_patterns > 0, "Should find DirectType spawn patterns");
        assert!(trait_patterns > 0, "Should find TraitMethod spawn patterns");
        
        println!("✅ Actor inference test passed!");
        println!("   - {} actors detected ({} explicit, {} inferred)", 
            symbols.actors.len(), 
            symbols.actors.iter().filter(|a| !a.doc_comment.as_ref().map_or(false, |d| d.contains("Inferred"))).count(),
            symbols.actors.iter().filter(|a| a.doc_comment.as_ref().map_or(false, |d| d.contains("Inferred"))).count()
        );
        println!("   - {} spawn patterns detected", symbols.actor_spawns.len());
    }
}