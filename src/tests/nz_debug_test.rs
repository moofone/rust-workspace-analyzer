#[cfg(test)]
mod nz_debug_tests {
    use crate::parser::rust_parser::RustParser;
    use std::path::PathBuf;

    #[test]
    fn test_nz_method_call_detection() {
        let mut parser = RustParser::new().unwrap();
        
        let source = r#"
            impl SomeType {
                fn calculate(&self) -> f64 {
                    self.value.nz(0.0)
                }
            }
            
            trait NZ {
                fn nz(&self, default: f64) -> f64;
            }
            
            impl NZ for f64 {
                fn nz(&self, default: f64) -> f64 {
                    if self.is_nan() { default } else { *self }
                }
            }
        "#;

        let symbols = parser
            .parse_source(source, &PathBuf::from("test.rs"), "test")
            .unwrap();
        
        println!("Functions found:");
        for func in &symbols.functions {
            println!("  - {}", func.qualified_name);
        }
        
        println!("\nCalls found:");
        for call in &symbols.calls {
            println!("  - {} calls {} (qualified: {:?})", 
                     call.caller_id, 
                     call.callee_name,
                     call.qualified_callee);
        }
        
        // Check that we found the nz method call
        let nz_calls: Vec<_> = symbols.calls.iter()
            .filter(|call| call.callee_name == "nz")
            .collect();
            
        println!("\nnz() calls found: {}", nz_calls.len());
        for call in &nz_calls {
            println!("  - Caller: {}", call.caller_id);
            println!("    Callee: {}", call.callee_name);
            println!("    Qualified: {:?}", call.qualified_callee);
        }
        
        // Should find at least the .nz() call
        assert!(!nz_calls.is_empty(), "Should detect .nz() method call");
    }
}