use std::path::Path;
use workspace_analyzer::parser::RustParser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🧪 Testing tree-sitter parser on simple Rust file");
    
    let test_file = Path::new("test_simple.rs");
    if !test_file.exists() {
        eprintln!("❌ Test file not found: {:?}", test_file);
        return Ok(());
    }
    
    println!("📁 Parsing file: {:?}", test_file);
    
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
    
    println!("🔍 Parsing file content...");
    match parser.parse_file(test_file, "test_crate") {
        Ok(symbols) => {
            println!("✅ Parsing completed successfully!");
            println!("📊 Results:");
            println!("   Functions: {}", symbols.functions.len());
            println!("   Types: {}", symbols.types.len());
            println!("   Impls: {}", symbols.impls.len());
            println!("   Calls: {}", symbols.calls.len());
            println!("   Imports: {}", symbols.imports.len());
            println!("   Modules: {}", symbols.modules.len());
            
            if !symbols.functions.is_empty() {
                println!("📝 Functions found:");
                for func in &symbols.functions {
                    println!("   - {}", func.qualified_name);
                }
            }
            
            if !symbols.types.is_empty() {
                println!("📝 Types found:");
                for ty in &symbols.types {
                    println!("   - {}", ty.qualified_name);
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Parsing failed: {}", e);
            return Err(e);
        }
    }
    
    println!("🎉 Test completed successfully!");
    Ok(())
}