use std::path::Path;
use workspace_analyzer::parser::rust_parser::RustParser;

#[tokio::main]
async fn main() {
    let mut parser = RustParser::new().unwrap();
    
    // Test parsing delta_vix.rs
    let path = Path::new("/Users/greg/Dev/git/trading-backend-poc/trading-ta/src/indicators/delta_vix.rs");
    
    let result = parser.parse_file(path, "trading-ta").unwrap();
    
    println!("=== delta_vix.rs impls ===");
    println!("Found {} impl blocks", result.impls.len());
    for impl_block in &result.impls {
        println!("Impl {} for {:?} at line {}", 
            impl_block.type_name, 
            impl_block.trait_name, 
            impl_block.line_start);
        for method in &impl_block.methods {
            println!("  {} (is_trait_impl: {})", method.name, method.is_trait_impl);
        }
    }
    
    // Also check standalone functions
    println!("\n=== Standalone functions ===");
    for func in &result.functions {
        if func.name == "fmt" || func.name == "na" || func.name == "nan" || func.name == "nz" || func.name == "from_ohlcv" {
            println!("{} at line {} (is_trait_impl: {})", 
                func.name, func.line_start, func.is_trait_impl);
        }
    }
    
    // Now test rma.rs for comparison
    println!("\n\n=== rma.rs impls (for comparison) ===");
    let rma_path = Path::new("/Users/greg/Dev/git/trading-backend-poc/trading-ta/src/indicators/rma.rs");
    let rma_result = parser.parse_file(rma_path, "trading-ta").unwrap();
    
    for impl_block in &rma_result.impls {
        if impl_block.trait_name.as_ref().map(|t| t.contains("Display")).unwrap_or(false) {
            println!("Impl {} for {:?} at line {}", 
                impl_block.type_name, 
                impl_block.trait_name, 
                impl_block.line_start);
            for method in &impl_block.methods {
                println!("  {} (is_trait_impl: {})", method.name, method.is_trait_impl);
            }
        }
    }
}