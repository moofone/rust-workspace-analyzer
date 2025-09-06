use workspace_analyzer::parser::rust_parser::RustParser;
use std::path::Path;
use tree_sitter::{Parser, Language};

extern "C" { fn tree_sitter_rust() -> Language; }

fn main() {
    let mut parser = RustParser::new().unwrap();
    
    let code = r#"
// Test attribute
#[test]
fn test_something() {
    assert_eq!(1 + 1, 2);
}

// Async trait (if using async-trait crate)
#[async_trait::async_trait]
trait MyAsyncTrait {
    async fn do_something(&self) -> Result<(), Box<dyn std::error::Error>>;
}

// Kameo remote attribute
#[kameo(remote)]
#[derive(Debug, Clone)]
struct RemoteMessage {
    pub data: String,
}

// Criterion benchmarks
#[criterion::criterion_group!(benches, benchmark_function)]
#[criterion::criterion_main!(benches)]
"#;
    
    let result = parser.parse_source(
        code,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();
    
    println!("Functions found: {}", result.functions.len());
    for func in &result.functions {
        println!("  Function: {} (is_test: {})", func.name, func.is_test);
    }
    
    println!("\nTypes found: {}", result.types.len());
    for ty in &result.types {
        println!("  Type: {}", ty.name);
    }
    
    println!("\nMacro expansions found: {}", result.macro_expansions.len());
    for expansion in &result.macro_expansions {
        println!("  Macro: {}", expansion.macro_name);
    }
    
    // Now let's analyze the tree-sitter AST directly
    println!("\n--- Direct Tree-sitter Analysis ---");
    let mut ts_parser = Parser::new();
    let language = unsafe { tree_sitter_rust() };
    ts_parser.set_language(&language).unwrap();
    
    let tree = ts_parser.parse(code, None).unwrap();
    let root = tree.root_node();
    
    fn print_tree(node: tree_sitter::Node, source: &[u8], indent: usize) {
        let kind = node.kind();
        let text = node.utf8_text(source).unwrap_or("");
        let preview = if text.len() > 50 {
            format!("{}...", &text[..50])
        } else {
            text.to_string()
        };
        
        println!("{}{} [{}]", " ".repeat(indent), kind, preview.replace('\n', " "));
        
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "function_item" || child.kind() == "attribute_item" || child.kind().contains("test") {
                print_tree(child, source, indent + 2);
            }
        }
    }
    
    print_tree(root, code.as_bytes(), 0);
}