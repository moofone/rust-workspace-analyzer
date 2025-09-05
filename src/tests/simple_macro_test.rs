#[cfg(test)]
mod simple_macro_tests {
    use crate::parser::rust_parser::RustParser;
    use std::path::PathBuf;

    #[test]
    fn test_basic_synthetic_call_generation() {
        let mut parser = RustParser::new().unwrap();
        
        let source = r#"
            use paste::paste;
            
            pub fn process_indicators(config: &Config) -> Result<()> {
                paste! {
                    let indicator = [<$indicator>]::new(config)?;
                }
                Ok(())
            }
        "#;

        let symbols = parser
            .parse_source(source, &PathBuf::from("test_indicators.rs"), "trading_ta")
            .unwrap();

        // Should detect synthetic calls
        let synthetic_calls: Vec<_> = symbols.calls.iter()
            .filter(|call| call.is_synthetic)
            .collect();
        
        eprintln!("Generated {} synthetic calls", synthetic_calls.len());
        for call in &synthetic_calls {
            eprintln!("  Synthetic call: {} -> {}", 
                     call.caller_id, 
                     call.qualified_callee.as_ref().unwrap_or(&call.callee_name));
        }
        
        // Basic validation - should generate some synthetic calls
        assert!(!synthetic_calls.is_empty(), "Should generate synthetic calls for macro patterns");
        
        // Check that synthetic calls have real function IDs as callers
        for call in &synthetic_calls {
            assert!(!call.caller_id.contains("MACRO_EXPANSION"), 
                   "Caller should be real function, got: {}", call.caller_id);
            assert!(call.is_synthetic, "Call should be marked as synthetic");
        }
        
        eprintln!("✅ Basic synthetic call generation test passed");
    }

    #[test]
    fn test_from_ohlcv_pattern() {
        let mut parser = RustParser::new().unwrap();
        
        let source = r#"
            use paste::paste;
            
            pub fn process_data(data: &OhlcvData) -> Result<()> {
                paste! {
                    let input = [<$indicator Input>]::from_ohlcv(data);
                }
                Ok(())
            }
        "#;

        let symbols = parser
            .parse_source(source, &PathBuf::from("test_from_ohlcv.rs"), "trading_ta")
            .unwrap();

        // Should detect synthetic calls for from_ohlcv pattern
        let synthetic_calls: Vec<_> = symbols.calls.iter()
            .filter(|call| call.is_synthetic)
            .collect();
        
        eprintln!("Generated {} synthetic calls for from_ohlcv pattern", synthetic_calls.len());
        
        // Should generate calls for from_ohlcv pattern
        let from_ohlcv_calls = synthetic_calls.iter()
            .filter(|call| call.qualified_callee.as_ref().map(|q| q.contains("from_ohlcv")).unwrap_or(false))
            .count();
            
        assert!(from_ohlcv_calls > 0, "Should generate synthetic calls for from_ohlcv pattern");
        
        eprintln!("✅ from_ohlcv pattern test passed with {} calls", from_ohlcv_calls);
    }
}