use std::path::Path;
use workspace_analyzer::parser::RustParser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🧪 Testing parser on actual trading-backend-poc file");
    
    let test_file = Path::new("/Users/greg/Dev/git/trading-backend-poc/trading-data-services/src/services/open_interest/messages.rs");
    
    if !test_file.exists() {
        eprintln!("❌ Test file not found: {:?}", test_file);
        return Ok(());
    }
    
    println!("📁 Testing file: {:?}", test_file);
    
    let mut parser = match RustParser::new() {
        Ok(p) => {
            println!("✅ RustParser created successfully");
            p
        }
        Err(e) => {
            eprintln!("❌ Failed to create parser: {}", e);
            return Err(e);
        }
    };
    
    println!("🔍 Attempting to parse trading file...");
    match parser.parse_file(test_file, "trading-exchanges") {
        Ok(symbols) => {
            println!("✅ Parsing completed successfully!");
            println!("📊 Results:");
            println!("   Functions: {}", symbols.functions.len());
            println!("   Types: {}", symbols.types.len());
            println!("   Impls: {}", symbols.impls.len());
            println!("   Calls: {}", symbols.calls.len());
            println!("   Imports: {}", symbols.imports.len());
            println!("   Modules: {}", symbols.modules.len());
            println!("   Message Handlers: {}", symbols.message_handlers.len());
            println!("   Actors: {}", symbols.actors.len());
            println!("   Distributed Actors: {}", symbols.distributed_actors.len());
            
            if !symbols.functions.is_empty() {
                println!("\n🔍 Functions found:");
                for func in &symbols.functions {
                    println!("   - {}::{} (line {})", func.module_path, func.name, func.line_start);
                }
            }
            
            if !symbols.types.is_empty() {
                println!("\n🔍 Types found:");
                for rust_type in &symbols.types {
                    println!("   - {}::{} ({}) (line {})", rust_type.module_path, rust_type.name, rust_type.type_kind, rust_type.line_start);
                }
            }
            
            if !symbols.message_handlers.is_empty() {
                println!("\n📨 Message handlers found:");
                for handler in &symbols.message_handlers {
                    println!("   - {} handles {}", handler.actor_name, handler.message_type);
                }
            }
            
            if !symbols.actors.is_empty() {
                println!("\n🎭 Actors found:");
                for actor in &symbols.actors {
                    println!("   - {} with {} local messages", actor.name, actor.local_messages.len());
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Parsing failed: {}", e);
            return Err(e);
        }
    }
    
    println!("🎉 Trading file test completed!");
    Ok(())
}