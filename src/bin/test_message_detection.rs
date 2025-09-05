use std::path::PathBuf;
use workspace_analyzer::parser::rust_parser::RustParser;
use workspace_analyzer::parser::symbols::SendMethod;

fn main() -> anyhow::Result<()> {
    let mut parser = RustParser::new()?;
    
    // Parse the dummy workspace crate_a
    let source = std::fs::read_to_string("/Users/greg/Dev/git/dummy-workspace/crate_a/src/lib.rs")?;
    let path = PathBuf::from("/Users/greg/Dev/git/dummy-workspace/crate_a/src/lib.rs");
    
    println!("Parsing dummy workspace crate_a...\n");
    let symbols = parser.parse_source(&source, &path, "crate_a")?;
    
    println!("\n=== Message Sends Detected ===");
    for send in &symbols.message_sends {
        println!("  {} -> {} via {} ({})", 
            send.sender_actor, 
            send.receiver_actor,
            match send.send_method {
                SendMethod::Tell => "tell",
                SendMethod::Ask => "ask",
            },
            send.message_type
        );
        println!("    at line {}", send.line);
    }
    
    if symbols.message_sends.is_empty() {
        println!("  No message sends detected");
    }
    
    Ok(())
}