use tree_sitter::Node;

/// Safe wrapper around node.utf8_text that handles encoding errors gracefully
pub fn safe_node_text<'a>(node: Node, source: &'a [u8]) -> Option<&'a str> {
    node.utf8_text(source).ok()
}

/// Safe wrapper around node.child_by_field_name that handles edge cases
pub fn get_child_by_field_name<'a>(node: Node<'a>, field: &str) -> Option<Node<'a>> {
    node.child_by_field_name(field)
}

/// Extract documentation comment from the node preceding the given node
pub fn extract_doc_comment(node: Node, source: &[u8]) -> Option<String> {
    let mut cursor = node.walk();
    let parent = node.parent()?;
    
    // Look for comment nodes before this node
    let mut comments = Vec::new();
    let mut found_target = false;
    
    for child in parent.children(&mut cursor) {
        if child == node {
            found_target = true;
            break;
        }
        
        if child.kind() == "line_comment" {
            if let Some(text) = safe_node_text(child, source) {
                // Only collect doc comments (starting with ///)
                if text.trim_start().starts_with("///") {
                    let doc_text = text.trim_start()
                        .strip_prefix("///")
                        .unwrap_or("")
                        .trim_start();
                    comments.push(doc_text.to_string());
                }
            }
        } else if child.kind() == "block_comment" {
            if let Some(text) = safe_node_text(child, source) {
                // Only collect doc comments (starting with /**)
                if text.trim_start().starts_with("/**") {
                    let doc_text = text.trim_start()
                        .strip_prefix("/**")
                        .and_then(|s| s.strip_suffix("*/"))
                        .unwrap_or("")
                        .trim();
                    comments.push(doc_text.to_string());
                }
            }
        }
    }
    
    if found_target && !comments.is_empty() {
        Some(comments.join(" "))
    } else {
        None
    }
}

/// Find the first child node of any of the specified types
pub fn find_child_of_type<'a>(node: Node<'a>, types: &[&str]) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    
    for child in node.children(&mut cursor) {
        if types.contains(&child.kind()) {
            return Some(child);
        }
    }
    
    None
}

/// Find all child nodes of the specified type
pub fn find_children_of_type<'a>(node: Node<'a>, node_type: &str) -> Vec<Node<'a>> {
    let mut cursor = node.walk();
    let mut children = Vec::new();
    
    for child in node.children(&mut cursor) {
        if child.kind() == node_type {
            children.push(child);
        }
    }
    
    children
}

/// Safely extract identifier name from a node
pub fn extract_identifier(node: Node, source: &[u8]) -> String {
    safe_node_text(node, source)
        .unwrap_or("<unknown>")
        .to_string()
}

/// Check if a node has a specific keyword as a child or in modifiers
pub fn has_keyword(node: Node, keyword: &str, source: &[u8]) -> bool {
    let mut cursor = node.walk();
    
    for child in node.children(&mut cursor) {
        if child.kind() == keyword {
            return true;
        }
        
        // Check for keyword in modifier nodes
        if child.kind() == "function_modifiers" || child.kind() == "visibility_modifier" {
            if let Some(text) = safe_node_text(child, source) {
                if text.contains(keyword) {
                    return true;
                }
            }
        }
    }
    
    false
}

/// Extract the visibility modifier from a node (pub, pub(crate), etc.)
pub fn extract_visibility(node: Node, source: &[u8]) -> String {
    // First try to find visibility modifier as a direct child
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "visibility_modifier" {
            if let Some(vis_text) = safe_node_text(child, source) {
                return vis_text.to_string();
            }
        }
    }
    
    // Fallback to checking field name (some node types might have it as a field)
    if let Some(vis_node) = node.child_by_field_name("visibility") {
        safe_node_text(vis_node, source)
            .unwrap_or("private")
            .to_string()
    } else {
        "private".to_string()
    }
}

/// Check if a node has an attribute with the given name
pub fn has_attribute(node: Node, source: &[u8], attribute_name: &str) -> bool {
    // First check if the node itself has attribute children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "attribute_item" {
            if let Some(attr_text) = safe_node_text(child, source) {
                if attr_text.contains(attribute_name) {
                    return true;
                }
            }
        }
    }
    
    // Check for preceding sibling attribute items
    // Attributes typically appear as preceding siblings in the AST
    if let Some(parent) = node.parent() {
        let mut cursor = parent.walk();
        let children: Vec<_> = parent.children(&mut cursor).collect();
        
        // Find the index of the current node
        if let Some(node_index) = children.iter().position(|&child| child.id() == node.id()) {
            // Look backwards from the node to find preceding attributes
            for i in (0..node_index).rev() {
                let child = children[i];
                if child.kind() == "attribute_item" {
                    if let Some(attr_text) = safe_node_text(child, source) {
                        if attr_text.contains(attribute_name) {
                            return true;
                        }
                    }
                } else if child.kind() != "line_comment" && child.kind() != "block_comment" {
                    // Stop if we hit a non-attribute, non-comment node
                    break;
                }
            }
        }
    }
    
    false
}

/// Safely traverse up the parent chain looking for a node of the given type
pub fn find_ancestor_of_type<'a>(mut node: Node<'a>, node_type: &str) -> Option<Node<'a>> {
    while let Some(parent) = node.parent() {
        if parent.kind() == node_type {
            return Some(parent);
        }
        node = parent;
    }
    None
}

/// Extract text content from a named field, falling back to empty string
pub fn extract_field_text(node: Node, field_name: &str, source: &[u8]) -> String {
    node.child_by_field_name(field_name)
        .and_then(|n| safe_node_text(n, source))
        .unwrap_or("")
        .to_string()
}

/// Check if a node represents an async function
pub fn is_async_function(node: Node, source: &[u8]) -> bool {
    // Check for async keyword in modifiers
    if has_keyword(node, "async", source) {
        return true;
    }
    
    // Check for #[async_trait] attribute
    has_attribute(node, source, "async_trait")
}

/// Extract generic parameters from a node
pub fn extract_generics(node: Node, source: &[u8]) -> Vec<String> {
    let mut generics = Vec::new();
    
    if let Some(type_params) = node.child_by_field_name("type_parameters") {
        let mut cursor = type_params.walk();
        
        for child in type_params.children(&mut cursor) {
            match child.kind() {
                "type_identifier" => {
                    // Simple type parameter like T
                    if let Some(param) = safe_node_text(child, source) {
                        generics.push(param.to_string());
                    }
                }
                "lifetime" => {
                    // Lifetime parameter like 'a
                    if let Some(param) = safe_node_text(child, source) {
                        generics.push(param.to_string());
                    }
                }
                "constrained_type_parameter" => {
                    // Type parameter with bounds like T: Clone
                    if let Some(name) = child.child_by_field_name("left") {
                        if let Some(param) = safe_node_text(name, source) {
                            generics.push(param.to_string());
                        }
                    }
                }
                "optional_type_parameter" => {
                    // Optional type parameter like E = Error
                    if let Some(name) = child.child_by_field_name("name") {
                        if let Some(param) = safe_node_text(name, source) {
                            generics.push(param.to_string());
                        }
                    }
                }
                _ => {
                    // For any other patterns, try to extract something meaningful
                    if child.kind().contains("parameter") {
                        if let Some(text) = safe_node_text(child, source) {
                            // Extract the first identifier-like token
                            if let Some(first_word) = text.split(':').next() {
                                let cleaned = first_word.trim().trim_start_matches('\'');
                                if !cleaned.is_empty() && !cleaned.starts_with('<') {
                                    generics.push(cleaned.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    generics
}

/// Safely get the range (start, end) of a node
pub fn get_node_range(node: Node) -> (usize, usize) {
    (node.start_byte(), node.end_byte())
}

/// Extract fields from a struct or union node
pub fn extract_struct_fields(node: Node, source: &[u8]) -> Vec<crate::parser::symbols::Field> {
    let mut fields = Vec::new();
    
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        
        for child in body.children(&mut cursor) {
            if child.kind() == "field_declaration" {
                let field_name = extract_field_text(child, "name", source);
                let field_type = extract_field_text(child, "type", source);
                
                // Check for visibility modifier
                let visibility = {
                    let mut vis = "private".to_string();
                    for sub_child in child.children(&mut child.walk()) {
                        if sub_child.kind() == "visibility_modifier" {
                            if let Some(vis_text) = safe_node_text(sub_child, source) {
                                vis = vis_text.to_string();
                                break;
                            }
                        }
                    }
                    vis
                };
                
                let doc_comment = extract_doc_comment(child, source);
                
                if !field_name.is_empty() {
                    fields.push(crate::parser::symbols::Field {
                        name: field_name,
                        field_type,
                        visibility,
                        doc_comment,
                    });
                }
            }
        }
    }
    
    fields
}

/// Extract variants from an enum node
pub fn extract_enum_variants(node: Node, source: &[u8]) -> Vec<crate::parser::symbols::Variant> {
    let mut variants = Vec::new();
    
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        
        for child in body.children(&mut cursor) {
            if child.kind() == "enum_variant" {
                let variant_name = extract_field_text(child, "name", source);
                let doc_comment = extract_doc_comment(child, source);
                
                // Check if variant has fields (tuple or struct variant)
                let has_fields = child.child_by_field_name("body").is_some() 
                    || child.child_by_field_name("parameters").is_some();
                
                if !variant_name.is_empty() {
                    variants.push(crate::parser::symbols::Variant {
                        name: variant_name,
                        fields: Vec::new(), // Could extract variant fields here if needed
                        doc_comment,
                    });
                }
            }
        }
    }
    
    variants
}

/// Get the line range of a node
pub fn get_line_range(node: Node) -> (usize, usize) {
    (node.start_position().row, node.end_position().row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    fn setup_parser() -> Parser {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::language()).unwrap();
        parser
    }

    #[test]
    fn test_safe_node_text() {
        let mut parser = setup_parser();
        let source = b"fn test() {}";
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        
        let text = safe_node_text(root, source);
        assert!(text.is_some());
        assert_eq!(text.unwrap(), "fn test() {}");
    }

    #[test]
    fn test_find_child_of_type() {
        let mut parser = setup_parser();
        let source = b"fn test() { let x = 42; }";
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        
        let function_node = find_child_of_type(root, &["function_item"]);
        assert!(function_node.is_some());
        assert_eq!(function_node.unwrap().kind(), "function_item");
    }

    #[test]
    fn test_has_keyword() {
        let mut parser = setup_parser();
        let source = b"pub async fn test() {}";
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        
        let function_node = find_child_of_type(root, &["function_item"]).unwrap();
        
        // Debug: print the AST structure
        println!("Function node kind: {}", function_node.kind());
        let mut cursor = function_node.walk();
        for child in function_node.children(&mut cursor) {
            println!("  Child: {} = {:?}", child.kind(), safe_node_text(child, source));
        }
        
        // Test has_keyword with the updated signature  
        assert!(has_keyword(function_node, "async", source));
        assert!(has_keyword(function_node, "pub", source));
        assert!(!has_keyword(function_node, "const", source));
    }

    #[test]
    fn test_extract_identifier() {
        let mut parser = setup_parser();
        let source = b"fn test_function() {}";
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        
        let function_node = find_child_of_type(root, &["function_item"]).unwrap();
        let name_node = function_node.child_by_field_name("name").unwrap();
        let name = extract_identifier(name_node, source);
        assert_eq!(name, "test_function");
    }

    #[test]
    fn test_is_async_function() {
        let mut parser = setup_parser();
        
        // Test async keyword
        let source = b"async fn test() {}";
        let tree = parser.parse(source, None).unwrap();
        let function_node = find_child_of_type(tree.root_node(), &["function_item"]).unwrap();
        assert!(is_async_function(function_node, source));
        
        // Test sync function
        let source = b"fn test() {}";
        let tree = parser.parse(source, None).unwrap();
        let function_node = find_child_of_type(tree.root_node(), &["function_item"]).unwrap();
        assert!(!is_async_function(function_node, source));
    }

    #[test]
    fn test_extract_generics() {
        let mut parser = setup_parser();
        let source = b"fn test<T, U>() {}";
        let tree = parser.parse(source, None).unwrap();
        let function_node = find_child_of_type(tree.root_node(), &["function_item"]).unwrap();
        
        let generics = extract_generics(function_node, source);
        assert_eq!(generics, vec!["T", "U"]);
    }
}