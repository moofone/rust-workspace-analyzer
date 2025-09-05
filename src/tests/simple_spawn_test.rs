use crate::config::Config;
use std::path::PathBuf;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_dummy_workspace_spawn_detection() {
        // Load the current config which should point to dummy workspace
        let config_path = PathBuf::from("/Users/greg/Dev/git/rust-workspace-analyzer/config.toml");
        if config_path.exists() {
            match Config::from_file(config_path.to_str().expect("Valid path")) {
                Ok(config) => {
                    let mut analyzer = crate::analyzer::WorkspaceAnalyzer::new(&config.workspace.root)
                        .expect("Failed to create analyzer");
                    let snapshot = analyzer.create_snapshot().await
                        .expect("Failed to create snapshot");
                    
                    // Actor spawns are directly in the snapshot
                    let total_spawns = snapshot.actor_spawns.len();
                    
                    println!("Total actor spawns detected: {}", total_spawns);
                    
                    // With enhanced detection, we should find significantly more spawns
                    assert!(total_spawns > 0, "Should detect at least some actor spawns in dummy workspace");
                    
                    // Print some details about the spawns found
                    println!("Spawn details:");
                    for spawn in &snapshot.actor_spawns {
                        println!("  {} spawns {} via {:?} at {}:{}", 
                            spawn.parent_actor_name, 
                            spawn.child_actor_name, 
                            spawn.spawn_method,
                            spawn.file_path.split('/').last().unwrap_or("unknown"),
                            spawn.line);
                    }
                    
                }
                Err(e) => {
                    panic!("Failed to load config from {}: {}", config_path.display(), e);
                }
            }
        } else {
            panic!("Config file not found at {}", config_path.display());
        }
    }
}