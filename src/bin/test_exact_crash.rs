use std::path::Path;
use workspace_analyzer::parser::RustParser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🧪 Testing exact content from error.rs lines 1-400");
    
    // Read the actual crash-inducing content
    let error_file = Path::new("/Users/greg/Dev/git/kameo/src/error.rs");
    let full_source = std::fs::read_to_string(error_file)?;
    let lines: Vec<&str> = full_source.lines().collect();
    
    // Test the first 400 lines (where crash occurs)
    let crash_source = lines[..400].join("\n");
    
    println!("📁 Testing first 400 lines ({} chars)", crash_source.len());
    
    let mut parser = RustParser::new()?;
    
    println!("🔍 Attempting to parse crash-inducing content...");
    match parser.parse_source(&crash_source, Path::new("error.rs"), "kameo") {
        Ok(symbols) => {
            println!("✅ Parsing completed successfully!");
            println!("📊 Results:");
            println!("   Functions: {}", symbols.functions.len());
            println!("   Types: {}", symbols.types.len());
            println!("   Impls: {}", symbols.impls.len());
            println!("   Calls: {}", symbols.calls.len());
            
            for rust_type in &symbols.types {
                println!("   - Type: {} (kind: {:?})", rust_type.name, rust_type.kind);
            }
        }
        Err(e) => {
            eprintln!("❌ Parsing failed: {}", e);
            return Err(e);
        }
    }
    
    println!("🎉 Exact crash test completed!");
    Ok(())
}