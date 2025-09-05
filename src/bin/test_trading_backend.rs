use std::path::Path;
use workspace_analyzer::analyzer::WorkspaceAnalyzer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🧪 Testing parser on trading-backend-poc workspace");
    
    let workspace_root = Path::new("/Users/greg/Dev/git/trading-backend-poc");
    
    if !workspace_root.exists() {
        eprintln!("❌ Trading backend workspace not found at: {:?}", workspace_root);
        return Ok(());
    }
    
    println!("📁 Testing workspace: {:?}", workspace_root);
    
    let mut analyzer = match WorkspaceAnalyzer::new(workspace_root) {
        Ok(a) => {
            println!("✅ WorkspaceAnalyzer created successfully");
            a
        }
        Err(e) => {
            eprintln!("❌ Failed to create analyzer: {}", e);
            return Err(e);
        }
    };
    
    println!("🔍 Creating workspace snapshot...");
    match analyzer.create_snapshot().await {
        Ok(snapshot) => {
            println!("✅ Workspace analysis completed successfully!");
            println!("📊 Results:");
            println!("   Crates: {}", snapshot.crates.len());
            println!("   Total Functions: {}", snapshot.functions.len());
            println!("   Total Types: {}", snapshot.types.len());
            println!("   Parsed Files: {}", snapshot.symbols.len());
            
            println!("\n🔍 Crates found:");
            for crate_meta in &snapshot.crates {
                let symbols = snapshot.symbols.get(&crate_meta.name);
                match symbols {
                    Some(symbols) => {
                        println!("   - {} ({}) - {} functions, {} types", 
                            crate_meta.name, 
                            crate_meta.path.display(),
                            symbols.functions.len(), 
                            symbols.types.len()
                        );
                    }
                    None => {
                        println!("   - {} ({}) - no symbols parsed", 
                            crate_meta.name, 
                            crate_meta.path.display()
                        );
                    }
                }
            }
            
            if !snapshot.functions.is_empty() {
                println!("\n🔍 Sample functions:");
                for func in snapshot.functions.iter().take(5) {
                    println!("   - {}::{}", func.module_path, func.name);
                }
            }
            
            if !snapshot.types.is_empty() {
                println!("\n🔍 Sample types:");
                for rust_type in snapshot.types.iter().take(5) {
                    println!("   - {}::{} ({})", rust_type.module_path, rust_type.name, rust_type.type_kind);
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Workspace analysis failed: {}", e);
            return Err(e);
        }
    }
    
    println!("🎉 Trading backend test completed successfully!");
    Ok(())
}