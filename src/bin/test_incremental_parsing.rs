use std::path::Path;
use workspace_analyzer::parser::RustParser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🧪 Testing incremental parsing to isolate crash location");
    
    // Read the full error.rs file
    let error_file = Path::new("/Users/greg/Dev/git/kameo/src/error.rs");
    let full_source = std::fs::read_to_string(error_file)?;
    let lines: Vec<&str> = full_source.lines().collect();
    
    println!("📁 Full file has {} lines", lines.len());
    
    let mut parser = RustParser::new()?;
    
    // Test progressively larger chunks
    let chunk_sizes = [50, 100, 200, 300, 400, 500, 600, 700, 800, lines.len()];
    
    for &chunk_size in &chunk_sizes {
        let chunk_size = chunk_size.min(lines.len());
        println!("\n🔍 Testing first {} lines...", chunk_size);
        
        let chunk_source = lines[..chunk_size].join("\n");
        
        match parser.parse_source(&chunk_source, error_file, "kameo") {
            Ok(symbols) => {
                println!("✅ Successfully parsed {} lines - Types: {}, Functions: {}", 
                    chunk_size, symbols.types.len(), symbols.functions.len());
                
                // Print types found for this chunk
                for rust_type in &symbols.types {
                    println!("   - Type: {}", rust_type.name);
                }
            }
            Err(e) => {
                eprintln!("❌ Failed parsing at {} lines: {}", chunk_size, e);
                break;
            }
        }
    }
    
    println!("🎉 Incremental test completed!");
    Ok(())
}