use crate::tpl::ast::AstNode;

enum TagFrame {
    If { test: String },
    For { 
        item: String, 
        collection: String, 
        open: String, 
        sep: String, 
        close: String 
    },
}

pub fn parse_template(template: &str) -> Vec<AstNode> {
    let mut nodes_stack: Vec<Vec<AstNode>> = vec![Vec::new()];
    let mut tag_stack: Vec<TagFrame> = Vec::new();
    let mut pos = 0;
    let len = template.len();

    while pos < len {
        let remaining = &template[pos..];

        // 1. Check for <if ...>
        if remaining.starts_with("<if ") {
             if let Some(end_tag) = find_tag_end(remaining) {
                let tag_content = &remaining[4..end_tag]; // skip "<if "
                // Extract test="..."
                if let Some(test) = extract_attr(tag_content, "test") {
                    nodes_stack.push(Vec::new());
                    tag_stack.push(TagFrame::If { test: test.to_string() });
                    pos += end_tag + 1;
                    continue;
                }
             }
        }
        
        // 2. Check for </if>
        if remaining.starts_with("</if>") {
            // Check if the current open tag is <if>
            if let Some(TagFrame::If { .. }) = tag_stack.last() {
                if let Some(TagFrame::If { test }) = tag_stack.pop() {
                     let body = nodes_stack.pop().unwrap_or_default();
                     append_node(nodes_stack.last_mut().expect("Stack underflow"), AstNode::If { test, body });
                     pos += 5;
                     continue;
                }
            }
        }

        // 3. Check for <for ...>
        if remaining.starts_with("<for ") {
             if let Some(end_tag) = find_tag_end(remaining) {
                let tag_content = &remaining[5..end_tag]; // skip "<for "
                if let (Some(item), Some(collection)) = (extract_attr(tag_content, "item"), extract_attr(tag_content, "collection")) {
                    let open = extract_attr(tag_content, "open").unwrap_or("");
                    let sep = extract_attr(tag_content, "sep").unwrap_or(",");
                    let close = extract_attr(tag_content, "close").unwrap_or("");
                    
                    nodes_stack.push(Vec::new());
                    tag_stack.push(TagFrame::For { 
                        item: item.to_string(), 
                        collection: collection.to_string(), 
                        open: open.to_string(), 
                        sep: sep.to_string(), 
                        close: close.to_string() 
                    });
                    pos += end_tag + 1;
                    continue;
                }
             }
        }
        
        // 4. Check for </for>
        if remaining.starts_with("</for>") {
             if let Some(TagFrame::For { .. }) = tag_stack.last() {
                 if let Some(TagFrame::For { item, collection, open, sep, close }) = tag_stack.pop() {
                     let body = nodes_stack.pop().unwrap_or_default();
                     append_node(nodes_stack.last_mut().expect("Stack underflow"), AstNode::For { 
                        item, collection, open, sep, close, body 
                     });
                     pos += 6;
                     continue;
                }
             }
        }

        // 5. Check for <include ... />
        if remaining.starts_with("<include") {
             if let Some(end_tag) = find_tag_end(remaining) {
                let tag_content = &remaining[8..end_tag]; // skip "<include"
                if let Some(refid) = extract_attr(tag_content, "refid") {
                    append_node(nodes_stack.last_mut().expect("Stack underflow"), AstNode::Include { refid: refid.to_string() });
                    pos += end_tag + 1;
                    continue;
                }
             }
        }
        
        // 6. Check for #{var}
        if remaining.starts_with("#{") {
             if let Some(end) = remaining.find('}') {
                 let var_name = remaining[2..end].trim();
                 if !var_name.is_empty() {
                    append_node(nodes_stack.last_mut().expect("Stack underflow"), AstNode::Var(var_name.to_string()));
                    pos += end + 1;
                    continue;
                 }
             }
        }
        
        // 6. Text
        let next_tag = remaining.find('<').unwrap_or(remaining.len());
        let next_var = remaining.find("#{").unwrap_or(remaining.len());
        let next_stop = std::cmp::min(next_tag, next_var);
        
        if next_stop > 0 {
             append_text(nodes_stack.last_mut().expect("Stack underflow"), &remaining[..next_stop]);
             pos += next_stop;
        } else {
             // Handle unmatched '<' or '#{'
             append_text(nodes_stack.last_mut().expect("Stack underflow"), &remaining[0..1]);
             pos += 1;
        }
    }

    // Auto-close unclosed tags
    while let Some(tag) = tag_stack.pop() {
        let body = nodes_stack.pop().unwrap_or_default();
        let node = match tag {
            TagFrame::If { test } => AstNode::If { test, body },
            TagFrame::For { item, collection, open, sep, close } => AstNode::For { item, collection, open, sep, close, body },
        };
        // Add to parent (if exists)
        if let Some(parent) = nodes_stack.last_mut() {
            append_node(parent, node);
        }
    }

    nodes_stack.pop().unwrap_or_default()
}

fn append_node(nodes: &mut Vec<AstNode>, node: AstNode) {
    nodes.push(node);
}

fn append_text(nodes: &mut Vec<AstNode>, text: &str) {
    if let Some(AstNode::Text(last_text)) = nodes.last_mut() {
        last_text.push_str(text);
    } else {
        nodes.push(AstNode::Text(text.to_string()));
    }
}

fn find_tag_end(s: &str) -> Option<usize> {
    let mut in_quote = false;
    for (i, c) in s.char_indices() {
        if c == '"' {
            in_quote = !in_quote;
        } else if c == '>' && !in_quote {
            return Some(i);
        }
    }
    None
}

fn extract_attr<'a>(tag_content: &'a str, key: &str) -> Option<&'a str> {
    let key_eq = format!("{}=\"", key);
    if let Some(idx) = tag_content.find(&key_eq) {
        let start = idx + key_eq.len();
        if let Some(end) = tag_content[start..].find('"') {
            return Some(&tag_content[start..start+end]);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_text() {
        let tpl = "hello world";
        let nodes = parse_template(tpl);
        assert_eq!(nodes.len(), 1);
        match &nodes[0] {
            AstNode::Text(t) => assert_eq!(t, "hello world"),
            _ => panic!("Expected Text"),
        }
    }

    #[test]
    fn test_parse_merged_text() {
        let tpl = "hello < world"; 
        let nodes = parse_template(tpl);
        assert_eq!(nodes.len(), 1);
        match &nodes[0] {
            AstNode::Text(t) => assert_eq!(t, "hello < world"),
            _ => panic!("Expected Text"),
        }
    }

    #[test]
    fn test_parse_var() {
        let tpl = "hello #{name}!";
        let nodes = parse_template(tpl);
        assert_eq!(nodes.len(), 3);
        match &nodes[0] { AstNode::Text(t) => assert_eq!(t, "hello "), _ => panic!() }
        match &nodes[1] { AstNode::Var(v) => assert_eq!(v, "name"), _ => panic!() }
        match &nodes[2] { AstNode::Text(t) => assert_eq!(t, "!"), _ => panic!() }
    }

    #[test]
    fn test_parse_if() {
        let tpl = "<if test=\"a > 1\">content</if>";
        let nodes = parse_template(tpl);
        assert_eq!(nodes.len(), 1);
        match &nodes[0] {
            AstNode::If { test, body } => {
                assert_eq!(test, "a > 1");
                assert_eq!(body.len(), 1);
                match &body[0] { AstNode::Text(t) => assert_eq!(t, "content"), _ => panic!() }
            }
            _ => panic!("Expected If"),
        }
    }

    #[test]
    fn test_parse_nested() {
        let tpl = "<if test=\"x\"><for item=\"i\" collection=\"list\">#{i}</for></if>";
        let nodes = parse_template(tpl);
        assert_eq!(nodes.len(), 1);
        match &nodes[0] {
            AstNode::If { body, .. } => {
                assert_eq!(body.len(), 1);
                match &body[0] {
                    AstNode::For { item, body, .. } => {
                        assert_eq!(item, "i");
                        assert_eq!(body.len(), 1);
                    }
                    _ => panic!("Expected For"),
                }
            }
            _ => panic!("Expected If"),
        }
    }

    #[test]
    fn test_auto_close() {
        let tpl = "<if test=\"x\">content";
        let nodes = parse_template(tpl);
        assert_eq!(nodes.len(), 1);
        match &nodes[0] {
            AstNode::If { test, body } => {
                assert_eq!(test, "x");
                assert_eq!(body.len(), 1);
                 match &body[0] { AstNode::Text(t) => assert_eq!(t, "content"), _ => panic!() }
            }
            _ => panic!("Expected If"),
        }
    }

    #[test]
    fn test_malformed_tags() {
        let tpl = "<if test=\"x\"> <unknown> #{ unclosed";
        let nodes = parse_template(tpl);
        assert_eq!(nodes.len(), 1);
        match &nodes[0] {
            AstNode::If { body, .. } => {
                assert_eq!(body.len(), 1);
                match &body[0] {
                    AstNode::Text(t) => assert_eq!(t, " <unknown> #{ unclosed"),
                    _ => panic!("Expected Text, got {:?}", body[0]),
                }
            }
            _ => panic!("Expected If"),
        }
    }
}
