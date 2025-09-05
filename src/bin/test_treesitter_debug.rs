use tree_sitter::{Parser, Query, QueryCursor};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::language())?;

    let source_code = r#"/// Function in crate_b that calls both crate_a and itself
pub fn function_b() -> i32 {
    let result_a = crate_a::function_a();  // Call to crate_a::function_a
    let result_self = function_b_helper();  // Call to own helper function
    result_a + result_self
}

/// Helper function in crate_b
pub fn function_b_helper() -> i32 {
    10
}"#;

    let tree = parser.parse(source_code, None).unwrap();
    
    println!("=== AST Debug ===");
    println!("{}", tree.root_node().to_sexp());
    
    let function_query = Query::new(
        &tree_sitter_rust::language(),
        r#"(function_item name: (identifier) @name)"#,
    )?;

    // First, let's walk the AST manually to find function_item nodes
    println!("\n=== Manual AST Walk ===");
    let mut cursor = tree.walk();
    let mut function_nodes = Vec::new();
    
    fn collect_function_nodes(cursor: &mut tree_sitter::TreeCursor, source: &[u8], nodes: &mut Vec<(usize, String)>) {
        loop {
            let node = cursor.node();
            if node.kind() == "function_item" {
                // Find the identifier node for the name
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        if child.kind() == "identifier" {
                            let name = child.utf8_text(source).unwrap_or("?");
                            nodes.push((child.start_position().row + 1, name.to_string()));
                            break;
                        }
                    }
                }
            }
            if cursor.goto_first_child() {
                collect_function_nodes(cursor, source, nodes);
                cursor.goto_parent();
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    
    collect_function_nodes(&mut cursor, source_code.as_bytes(), &mut function_nodes);
    
    println!("Found {} function nodes in AST:", function_nodes.len());
    for (line, name) in &function_nodes {
        println!("  Line {}: {}", line, name);
    }
    
    // Now let's test the query using the cursor approach without collecting
    println!("\n=== Function Query Matches (Stream) ===");
    let mut cursor = QueryCursor::new();
    let mut match_count = 0;
    
    for query_match in cursor.matches(&function_query, tree.root_node(), source_code.as_bytes()) {
        match_count += 1;
        println!("\nMatch #{}", match_count);
        for capture in query_match.captures {
            let node = capture.node;
            let text = node.utf8_text(source_code.as_bytes()).unwrap();
            let capture_name = function_query.capture_names()[capture.index as usize];
            println!("  Capture '{}': '{}' (line: {})", capture_name, text.replace('\n', "\\n"), node.start_position().row + 1);
        }
    }
    
    println!("\nTotal matches: {}", match_count);
    
    Ok(())
}