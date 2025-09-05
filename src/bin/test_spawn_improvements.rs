use anyhow::Result;
use workspace_analyzer::config::Config;
use workspace_analyzer::analyzer::WorkspaceAnalyzer;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Testing spawn detection improvements on trading-backend-poc...");
    
    // Load config pointing to trading-backend-poc
    let config = Config::from_workspace_root("/Users/greg/Dev/git/trading-backend-poc")?;
    
    let mut analyzer = WorkspaceAnalyzer::new_with_config(config)?;
    let snapshot = analyzer.create_snapshot().await?;
    
    println!("\n📊 Detection Results:");
    println!("  📦 Total crates analyzed: {}", snapshot.crates.len());
    for crate_meta in &snapshot.crates {
        println!("    - {}", crate_meta.name);
    }
    
    println!("  🎭 Total actors detected: {}", snapshot.actors.len());
    println!("  🎬 Total spawns detected: {}", snapshot.actor_spawns.len());
    
    // Let's check if we're finding any Rust files at all
    println!("\n🔍 Sample of detected actor names:");
    for actor in snapshot.actors.iter().take(10) {
        println!("    - {} (from {})", 
            actor.name, 
            actor.file_path.split('/').last().unwrap_or("unknown")
        );
    }
    
    // Group actors by how they were detected
    let explicit_actors = snapshot.actors.iter().filter(|a| {
        !a.doc_comment.as_ref().map_or(false, |d| d.contains("Inferred") || d.contains("Derived"))
    }).count();
    
    let inferred_actors = snapshot.actors.iter().filter(|a| {
        a.doc_comment.as_ref().map_or(false, |d| d.contains("Inferred"))
    }).count();
    
    let derived_actors = snapshot.actors.iter().filter(|a| {
        a.doc_comment.as_ref().map_or(false, |d| d.contains("Derived"))
    }).count();
    
    println!("\n🔍 Actor Detection Breakdown:");
    println!("  ✅ Explicit (impl Actor): {}", explicit_actors);
    println!("  🔮 Inferred (from spawns/refs): {}", inferred_actors);
    println!("  🏗️  Derived (#[derive(Actor)]): {}", derived_actors);
    
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
    
    // Show some specific spawn examples
    println!("\n📋 Sample Spawn Detections:");
    for (i, spawn) in snapshot.actor_spawns.iter().take(10).enumerate() {
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
    
    if snapshot.actor_spawns.len() > 10 {
        println!("  ... and {} more", snapshot.actor_spawns.len() - 10);
    }
    
    // Check for exact duplicates
    use std::collections::HashSet;
    let mut unique_spawns = HashSet::new();
    let mut duplicates = 0;
    
    for spawn in &snapshot.actor_spawns {
        let spawn_key = (
            spawn.parent_actor_name.clone(),
            spawn.child_actor_name.clone(),
            spawn.file_path.clone(),
            spawn.line
        );
        
        if !unique_spawns.insert(spawn_key) {
            duplicates += 1;
        }
    }
    
    println!("\n🔍 Duplicate Analysis:");
    println!("  📊 Total spawn relationships: {}", snapshot.actor_spawns.len());
    println!("  ✅ Unique spawn relationships: {}", unique_spawns.len());
    println!("  ⚠️  Exact duplicates: {}", duplicates);
    
    // Show actors spawned multiple times
    let mut child_counts: HashMap<String, usize> = HashMap::new();
    for spawn in &snapshot.actor_spawns {
        *child_counts.entry(spawn.child_actor_name.clone()).or_insert(0) += 1;
    }
    
    let mut duplicate_actors: Vec<_> = child_counts.iter()
        .filter(|(_, count)| **count > 1)
        .collect();
    duplicate_actors.sort_by(|a, b| b.1.cmp(a.1));
    
    println!("\n🔄 Actors spawned multiple times:");
    for (actor_name, count) in duplicate_actors.iter().take(10) {
        println!("  {} appears {} times", actor_name, count);
    }
    
    // Show detailed locations for top duplicate actor
    if let Some((top_actor, _)) = duplicate_actors.first() {
        println!("\n📍 Locations where {} is spawned:", top_actor);
        let mut locations: Vec<_> = snapshot.actor_spawns.iter()
            .filter(|spawn| spawn.child_actor_name == **top_actor)
            .collect();
        locations.sort_by(|a, b| a.file_path.cmp(&b.file_path).then(a.line.cmp(&b.line)));
        
        for (i, spawn) in locations.iter().take(15).enumerate() {
            let file_name = spawn.file_path.split('/').last().unwrap_or("unknown");
            let is_test = spawn.context.contains("test") || spawn.file_path.contains("test") || spawn.context.contains("Test");
            let marker = if is_test { "🧪" } else { "🏢" };
            println!("  {}. {} {}:{} in {} (context: {})", 
                i + 1, marker, file_name, spawn.line, spawn.parent_actor_name, spawn.context);
        }
        if locations.len() > 15 {
            println!("  ... and {} more locations", locations.len() - 15);
        }
    }
    
    println!("\n✨ Analysis Complete!");
    println!("🎯 Expected ~42 unique spawn relationships per specification");
    println!("🔥 Detected: {} spawn relationships", snapshot.actor_spawns.len());
    
    if duplicates > 0 {
        println!("❌ ISSUE: Found {} exact duplicate spawn relationships!", duplicates);
    } else if snapshot.actor_spawns.len() >= 30 {
        println!("🎉 SUCCESS: Significant improvement in spawn detection!");
    } else if snapshot.actor_spawns.len() >= 15 {
        println!("📈 GOOD: Moderate improvement in spawn detection");
    } else {
        println!("⚠️  NEEDS_WORK: Still room for improvement");
    }
    
    Ok(())
}