use std::path::Path;
use workspace_analyzer::parser::RustParser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🧪 Testing parser on error.rs file that causes crash");
    
    let error_file = Path::new("/Users/greg/Dev/git/kameo/src/error.rs");
    if !error_file.exists() {
        eprintln!("❌ Error file not found: {:?}", error_file);
        return Ok(());
    }
    
    println!("📁 Parsing file: {:?}", error_file);
    
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
    
    println!("🔍 Attempting to parse error.rs...");
    match parser.parse_file(error_file, "kameo") {
        Ok(symbols) => {
            println!("✅ Parsing completed successfully!");
            println!("📊 Results:");
            println!("   Functions: {}", symbols.functions.len());
            println!("   Types: {}", symbols.types.len());
            println!("   Impls: {}", symbols.impls.len());
            println!("   Calls: {}", symbols.calls.len());
            println!("   Imports: {}", symbols.imports.len());
            println!("   Modules: {}", symbols.modules.len());
        }
        Err(e) => {
            eprintln!("❌ Parsing failed: {}", e);
            return Err(e);
        }
    }
    
    println!("🎉 Test completed without crash!");
    Ok(())
}