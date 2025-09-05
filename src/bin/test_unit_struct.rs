use std::path::Path;
use workspace_analyzer::parser::RustParser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🧪 Testing unit struct parsing that may cause crash");
    
    let test_source = r#"
use std::any;

type ErrorHookFn = fn(&PanicError);

pub enum SendError<M = (), E = std::convert::Infallible> {
    ActorNotRunning(M),
    ActorStopped,
    MailboxFull(M),
}

pub struct ActorNotRunning;

pub enum ActorStopReason {
    Normal,
    Panic(String),
}

pub struct PanicError;
"#;
    
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
    
    println!("🔍 Attempting to parse source with unit struct and enum...");
    match parser.parse_source(test_source, Path::new("test.rs"), "test") {
        Ok(symbols) => {
            println!("✅ Parsing completed successfully!");
            println!("📊 Results:");
            println!("   Functions: {}", symbols.functions.len());
            println!("   Types: {}", symbols.types.len());
            println!("   Impls: {}", symbols.impls.len());
            println!("   Calls: {}", symbols.calls.len());
            println!("   Imports: {}", symbols.imports.len());
            println!("   Modules: {}", symbols.modules.len());
            
            for rust_type in &symbols.types {
                println!("   - Type: {} (kind: {:?})", rust_type.name, rust_type.kind);
            }
        }
        Err(e) => {
            eprintln!("❌ Parsing failed: {}", e);
            return Err(e);
        }
    }
    
    println!("🎉 Unit struct test completed!");
    Ok(())
}