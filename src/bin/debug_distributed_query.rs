use anyhow::Result;
use tree_sitter::{Language, Parser, Query, QueryCursor};

fn main() -> Result<()> {
    // Sample distributed actor code
    let source = r#"
kameo::distributed_actor! {
    TestActor {
        MessageA,
        MessageB,
        MessageC,
    }
}
"#;

    let language = tree_sitter_rust::language();
    let mut parser = Parser::new();
    parser.set_language(&language)?;

    let tree = parser.parse(source, None).unwrap();
    println!("Tree: {}", tree.root_node().to_sexp());

    // Test the query
    let query_text = r#"
    (macro_invocation
      (scoped_identifier
        path: (identifier) @namespace (#eq? @namespace "kameo")
        name: (identifier) @macro_name (#eq? @macro_name "distributed_actor"))
      (token_tree
        (identifier) @actor_name
        (token_tree
          (identifier) @message_type)*
      )
    ) @distributed_actor
    "#;

    match Query::new(&language, query_text) {
        Ok(query) => {
            let mut cursor = QueryCursor::new();
            let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
            
            let mut match_count = 0;
            for match_ in matches {
                match_count += 1;
                println!("Match {}: {} captures", match_count, match_.captures.len());
                for capture in match_.captures {
                    let name = &query.capture_names()[capture.index as usize];
                    if let Ok(text) = capture.node.utf8_text(source.as_bytes()) {
                        println!("  {}: {}", name, text);
                    }
                }
            }
            
            if match_count == 0 {
                println!("❌ No matches found!");
                
                // Let's try a simpler query
                let simple_query = r#"(macro_invocation) @macro"#;
                if let Ok(simple) = Query::new(&language, simple_query) {
                    let mut cursor = QueryCursor::new();
                    let matches = cursor.matches(&simple, tree.root_node(), source.as_bytes());
                    
                    for (i, match_) in matches.enumerate() {
                        println!("Simple match {}: {}", i, match_.captures[0].node.to_sexp());
                        if let Ok(text) = match_.captures[0].node.utf8_text(source.as_bytes()) {
                            println!("Text: {}", text);
                        }
                    }
                }
            } else {
                println!("✅ Found {} matches", match_count);
            }
        },
        Err(e) => {
            println!("❌ Query error: {}", e);
        }
    }

    Ok(())
}