use anyhow::Result;
use workspace_analyzer::config::Config;
use workspace_analyzer::analyzer::WorkspaceAnalyzer;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Testing all actor detection patterns on updated dummy workspace...");
    
    // Load config pointing to dummy workspace  
    let config = Config::from_workspace_root("/Users/greg/Dev/git/dummy-workspace")?;
    
    let mut analyzer = WorkspaceAnalyzer::new_with_config(config)?;
    let snapshot = analyzer.create_snapshot().await?;
    
    println!("\n📊 Dummy Workspace Detection Results:");
    println!("  📦 Total crates analyzed: {}", snapshot.crates.len());
    for crate_meta in &snapshot.crates {
        println!("    - {}", crate_meta.name);
    }
    
    println!("  🎭 Total actors detected: {}", snapshot.actors.len());
    println!("  🎬 Total spawns detected: {}", snapshot.actor_spawns.len());
    
    // Group actors by detection method
    let explicit_actors: Vec<_> = snapshot.actors.iter().filter(|a| {
        !a.doc_comment.as_ref().map_or(false, |d| d.contains("Inferred") || d.contains("Derived"))
    }).collect();
    
    let inferred_actors: Vec<_> = snapshot.actors.iter().filter(|a| {
        a.doc_comment.as_ref().map_or(false, |d| d.contains("Inferred"))
    }).collect();
    
    let derived_actors: Vec<_> = snapshot.actors.iter().filter(|a| {
        a.doc_comment.as_ref().map_or(false, |d| d.contains("Derived"))
    }).collect();
    
    println!("\n🔍 Actor Detection Breakdown:");
    println!("  ✅ Explicit (impl Actor): {}", explicit_actors.len());
    for actor in &explicit_actors {
        println!("    - {} (from {})", actor.name, actor.file_path.split('/').last().unwrap_or("unknown"));
    }
    
    println!("  🔮 Inferred (from spawns/refs): {}", inferred_actors.len());
    for actor in &inferred_actors {
        println!("    - {} (from {})", actor.name, actor.file_path.split('/').last().unwrap_or("unknown"));
    }
    
    println!("  🏗️  Derived (#[derive(Actor)]): {}", derived_actors.len());
    for actor in &derived_actors {
        println!("    - {} (from {})", actor.name, actor.file_path.split('/').last().unwrap_or("unknown"));
    }
    
    // Show spawn pattern breakdown
    use std::collections::HashMap;
    let mut spawn_patterns: HashMap<String, usize> = HashMap::new();
    let mut spawn_methods: HashMap<String, usize> = HashMap::new();
    
    for spawn in &snapshot.actor_spawns {
        *spawn_patterns.entry(format!("{:?}", spawn.spawn_pattern)).or_insert(0) += 1;
        *spawn_methods.entry(format!("{:?}", spawn.spawn_method)).or_insert(0) += 1;
    }
    
    println!("\n🎬 Spawn Pattern Breakdown:");
    for (pattern, count) in spawn_patterns {
        println!("  📝 {}: {}", pattern, count);
    }
    
    println!("\n🚀 Spawn Method Breakdown:");
    for (method, count) in spawn_methods {
        println!("  ⚙️  {}: {}", method, count);
    }
    
    // Show specific spawn examples
    println!("\n📋 All Spawn Detections:");
    for (i, spawn) in snapshot.actor_spawns.iter().enumerate() {
        let file_name = spawn.file_path.split('/').last().unwrap_or("unknown");
        println!("  {}. {} spawns {} via {:?} at {}:{}", 
            i + 1,
            spawn.parent_actor_name, 
            spawn.child_actor_name, 
            spawn.spawn_method,
            file_name,
            spawn.line
        );
    }
    
    println!("\n✨ Expected Pattern Verification:");
    println!("🎯 Task 1.1 (Derive): Expected >=2 derived actors, found {}", derived_actors.len());
    println!("🎯 Task 1.2 (Inference): Expected >=4 inferred actors, found {}", inferred_actors.len()); 
    println!("🎯 Task 1.3 (ActorRef): Should see actors from ActorRef<T> usage");
    println!("🎯 Task 2.1 (Fallback): Should see NonExistentActor, MissingActor from spawn calls");
    
    // Expected actors we should find:
    let expected_derived = ["DerivedActor", "ComplexDerivedActor"];
    let expected_explicit = ["ExampleActor", "WorkerActor"];
    let expected_inferred = ["NonExistentActor", "MissingActor"];
    
    println!("\n🔍 Verification Results:");
    for expected in &expected_derived {
        let found = derived_actors.iter().any(|a| a.name == *expected);
        println!("  {} Derived Actor '{}': {}", if found { "✅" } else { "❌" }, expected, if found { "FOUND" } else { "MISSING" });
    }
    
    for expected in &expected_explicit {
        let found = explicit_actors.iter().any(|a| a.name == *expected);
        println!("  {} Explicit Actor '{}': {}", if found { "✅" } else { "❌" }, expected, if found { "FOUND" } else { "MISSING" });
    }
    
    for expected in &expected_inferred {
        let found = inferred_actors.iter().any(|a| a.name == *expected);
        println!("  {} Inferred Actor '{}': {}", if found { "✅" } else { "❌" }, expected, if found { "FOUND" } else { "MISSING" });
    }
    
    Ok(())
}