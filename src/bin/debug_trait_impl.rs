use std::path::PathBuf;
use workspace_analyzer::analyzer::workspace_analyzer::WorkspaceAnalyzer;

#[tokio::main]
async fn main() {
    let workspace_path = PathBuf::from("/Users/greg/Dev/git/trading-backend-poc");
    let mut analyzer = WorkspaceAnalyzer::new(workspace_path).unwrap();
    
    let snapshot = analyzer.analyze_with_global_context().await.unwrap();
    
    // Look for RMA functions
    for (crate_name, symbols) in &snapshot.symbols {
        if crate_name == "trading-ta" {
            println!("\n=== Functions in trading-ta ===");
            let mut rma_funcs = 0;
            for func in &symbols.functions {
                if func.module_path.contains("rma") {
                    println!("Function: {} | is_trait_impl: {} | module: {}", 
                        func.name, func.is_trait_impl, func.module_path);
                    rma_funcs += 1;
                }
            }
            println!("Total RMA functions: {}", rma_funcs);
            
            println!("\n=== Impls in trading-ta ===");
            for impl_block in &symbols.impls {
                if impl_block.type_name == "Rma" || impl_block.trait_name.as_ref().map_or(false, |t| t.contains("SeriesIndicator")) {
                    println!("Impl: {} for {:?}", impl_block.type_name, impl_block.trait_name);
                    for method in &impl_block.methods {
                        println!("  Method: {} | is_trait_impl: {}", method.name, method.is_trait_impl);
                    }
                }
            }
        }
    }
}