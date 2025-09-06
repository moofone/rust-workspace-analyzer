use workspace_analyzer::parser::rust_parser::RustParser;
use std::path::Path;
use tree_sitter::{Parser, Language};

extern "C" { fn tree_sitter_rust() -> Language; }

fn main() {
    let code = r#"
distributed_actor! {
    struct MarketDataDistributor {
        symbol: String,
    }
}
"#;
    
    // Parse with tree-sitter to see structure
    let mut ts_parser = Parser::new();
    let language = unsafe { tree_sitter_rust() };
    ts_parser.set_language(&language).unwrap();
    
    let tree = ts_parser.parse(code, None).unwrap();
    let root = tree.root_node();
    
    println!("=== Tree-sitter AST ===");
    print_tree(root, code.as_bytes(), 0);
    
    println!("\n=== Workspace Analyzer Parsing ===");
    let mut parser = RustParser::new().unwrap();
    let result = parser.parse_source(
        code,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();
    
    println!("Actors found: {}", result.actors.len());
    for actor in &result.actors {
        println!("  Actor: {} (distributed: {})", actor.name, actor.is_distributed);
    }
    
    println!("Distributed actors: {}", result.distributed_actors.len());
    for da in &result.distributed_actors {
        println!("  Distributed Actor: {}", da.actor_name);
    }
    
    println!("Macro expansions: {}", result.macro_expansions.len());
    for exp in &result.macro_expansions {
        println!("  Macro: {} - expanded: {:?}", exp.macro_name, exp.expanded_content);
    }
    
    println!("Types: {}", result.types.len());
    for ty in &result.types {
        println!("  Type: {}", ty.name);
    }
}

fn print_tree(node: tree_sitter::Node, source: &[u8], indent: usize) {
    let kind = node.kind();
    let text = node.utf8_text(source).unwrap_or("");
    let preview = if text.len() > 60 {
        format!("{}...", &text[..60])
    } else {
        text.to_string()
    };
    
    println!("{}{} [{}]", " ".repeat(indent), kind, preview.replace('\n', " "));
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        print_tree(child, source, indent + 2);
    }
}