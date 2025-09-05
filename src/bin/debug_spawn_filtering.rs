use anyhow::Result;
use std::path::Path;
use workspace_analyzer::parser::RustParser;

fn main() -> Result<()> {
    println!("🔍 Testing spawn filtering logic");
    
    let parser = RustParser::new();
    
    // Test the filtering function directly
    let test_contexts = vec![
        ("regular_function:test_accounting_new_position_long", "should be filtered"),
        ("regular_function:setup_adapter", "should be filtered"), 
        ("regular_function:mock_actor_spawn", "should be filtered"),
        ("regular_function:production_spawn", "should NOT be filtered"),
        ("on_start:on_start", "should NOT be filtered"),
        ("main_function:main", "should NOT be filtered"),
    ];
    
    let dummy_path = Path::new("/dummy/path.rs");
    
    for (context, expectation) in test_contexts {
        // We can't call the private method directly, so let's replicate the logic
        let is_filtered = is_spawn_in_test_context_logic(context, dummy_path);
        println!("Context: '{}' -> Filtered: {} ({})", context, is_filtered, expectation);
    }
    
    // Test with actual test files
    let test_file_paths = vec![
        "/some/path/tests/integration.rs",
        "/some/path/src/test_module.rs", 
        "/some/path/src/production.rs",
    ];
    
    for path_str in test_file_paths {
        let path = Path::new(path_str);
        let is_filtered = is_spawn_in_test_context_logic("regular_function:some_function", path);
        println!("File: '{}' -> Filtered: {}", path_str, is_filtered);
    }
    
    Ok(())
}

// Replicated logic from RustParser::is_spawn_in_test_context
fn is_spawn_in_test_context_logic(context: &str, file_path: &Path) -> bool {
    // Check if context indicates test function
    if context.contains("test") {
        return true;
    }
    
    // Check if file path indicates test file
    let file_str = file_path.to_string_lossy();
    if file_str.contains("test") || file_str.contains("/tests/") {
        return true;
    }
    
    // Check if function name patterns suggest test
    let function_name = context.split(':').last().unwrap_or("");
    if function_name.starts_with("test_") || 
       function_name.ends_with("_test") ||
       function_name.starts_with("setup_") ||
       function_name.contains("mock") {
        return true;
    }
    
    false
}