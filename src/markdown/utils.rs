//! Utility functions for XML/HTML parsing and manipulation.
//!
//! Provides helper functions for working with roxmltree nodes, including
//! namespace handling, attribute access, and text extraction.

use std::collections::BTreeSet;

use roxmltree::Node;

/// Synthetic namespace base URL for Confluence namespaces.
pub const SYNTHETIC_NS_BASE: &str = "https://confluence.example/";

/// Get all text content from an element and its children.
pub fn get_element_text(node: Node) -> String {
  let mut text = String::new();

  for child in node.children() {
    match child.node_type() {
      roxmltree::NodeType::Text => {
        if let Some(value) = child.text() {
          text.push_str(&super::html_entities::decode_html_entities(value));
        }
      }
      roxmltree::NodeType::Element => {
        text.push_str(&get_element_text(child));
      }
      _ => {}
    }
  }

  text
}

/// Split a qualified tag name into prefix and local name.
pub fn split_qualified_name(name: &str) -> (Option<&str>, &str) {
  if let Some((prefix, local)) = name.split_once(':') {
    (Some(prefix), local)
  } else {
    (None, name)
  }
}

/// Wrap content with synthetic namespace declarations.
///
/// This allows roxmltree to parse Confluence-specific namespaces
/// like `ac:`, `ri:` without them being declared in the XML.
pub fn wrap_with_namespaces(storage_content: &str) -> String {
  let mut prefixes = BTreeSet::new();

  // Scan for namespace prefixes in element names
  for segment in storage_content.split('<').skip(1) {
    let mut segment = segment;
    if let Some(idx) = segment.find('>') {
      segment = &segment[..idx];
    }

    let segment = segment.trim_start_matches('/');

    if let Some((prefix, _)) = segment.split_once(':')
      && is_valid_prefix(prefix)
    {
      prefixes.insert(prefix.to_string());
    }

    // Scan for namespace prefixes in attributes
    for attr in segment.split_whitespace() {
      if let Some((name, _)) = attr.split_once('=')
        && let Some((prefix, _)) = name.split_once(':')
        && is_valid_prefix(prefix)
      {
        prefixes.insert(prefix.to_string());
      }
    }
  }

  let mut result = String::from("<cdl-root");
  for prefix in prefixes {
    result.push_str(" xmlns:");
    result.push_str(&prefix);
    result.push_str("=\"");
    result.push_str(SYNTHETIC_NS_BASE);
    result.push_str(&prefix);
    result.push('"');
  }
  result.push('>');
  result.push_str(storage_content);
  result.push_str("</cdl-root>");
  result
}

/// Check if a string is a valid XML namespace prefix.
fn is_valid_prefix(prefix: &str) -> bool {
  if prefix.is_empty() {
    return false;
  }
  prefix
    .chars()
    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Get the qualified tag name of a node (including namespace).
pub fn qualified_tag_name(node: Node) -> String {
  let tag = node.tag_name();
  let name = tag.name();
  if let Some(namespace) = tag.namespace() {
    format!("{namespace}:{name}")
  } else {
    name.to_string()
  }
}

/// Check if a node matches a specific tag name (with optional namespace).
pub fn matches_tag(node: Node, name: &str) -> bool {
  if !node.is_element() {
    return false;
  }

  let (expected_prefix, expected_name) = split_qualified_name(name);
  let tag = node.tag_name();
  if tag.name() != expected_name {
    return false;
  }

  let expected_namespace = expected_prefix.map(|prefix| format!("{SYNTHETIC_NS_BASE}{prefix}"));

  match (expected_namespace.as_deref(), tag.namespace()) {
    (Some(expected), Some(actual)) => actual == expected,
    (None, None) => true,
    (Some(_), None) | (None, Some(_)) => false,
  }
}

/// Get an attribute value from a node (with optional namespace).
pub fn get_attribute(node: Node, attr_name: &str) -> Option<String> {
  if !node.is_element() {
    return None;
  }

  let (expected_prefix, expected_name) = split_qualified_name(attr_name);
  let expected_namespace = expected_prefix.map(|prefix| format!("{SYNTHETIC_NS_BASE}{prefix}"));

  for attr in node.attributes() {
    if attr.name() != expected_name {
      continue;
    }

    let namespace_matches = match (expected_namespace.as_deref(), attr.namespace()) {
      (Some(expected), Some(actual)) => actual == expected,
      (None, None) => true,
      (Some(_), None) | (None, Some(_)) => false,
    };

    if namespace_matches {
      return Some(attr.value().to_string());
    }
  }
  None
}

/// Find a child element by tag name (handles namespaced tags).
pub fn find_child_by_tag<'a, 'input>(node: Node<'a, 'input>, tag_name: &str) -> Option<Node<'a, 'input>> {
  node.children().find(|child| matches_tag(*child, tag_name))
}

/// Find a child element by tag name and attribute value.
pub fn find_child_by_tag_and_attr<'a, 'input>(
  node: Node<'a, 'input>,
  tag_name: &str,
  attr_name: &str,
  attr_value: &str,
) -> Option<Node<'a, 'input>> {
  node
    .children()
    .find(|child| matches_tag(*child, tag_name) && get_attribute(*child, attr_name).as_deref() == Some(attr_value))
}

/// Clean up the markdown output.
///
/// - Removes excessive blank lines (more than 2 consecutive)
/// - Trims leading/trailing whitespace
/// - Ensures file ends with a newline
pub fn clean_markdown(content: &str) -> String {
  let mut result = content.to_string();

  // Remove excessive blank lines (more than 2 consecutive)
  while result.contains("\n\n\n") {
    result = result.replace("\n\n\n", "\n\n");
  }

  // Remove leading/trailing whitespace
  result = result.trim().to_string();

  // Ensure file ends with newline
  if !result.ends_with('\n') {
    result.push('\n');
  }

  result
}

#[cfg(test)]
mod tests {
  use roxmltree::Document;

  use super::*;

  #[test]
  fn test_clean_markdown_removes_excessive_newlines() {
    let input = "Line 1\n\n\n\n\nLine 2";
    let output = clean_markdown(input);
    assert!(!output.contains("\n\n\n"));
    assert!(output.contains("Line 1\n\nLine 2"));
  }

  #[test]
  fn test_clean_markdown_adds_trailing_newline() {
    let input = "Some content";
    let output = clean_markdown(input);
    assert!(output.ends_with('\n'));
  }

  #[test]
  fn test_clean_markdown_preserves_double_newlines() {
    let input = "Paragraph 1\n\nParagraph 2";
    let output = clean_markdown(input);
    assert!(output.contains("Paragraph 1\n\nParagraph 2"));
  }

  #[test]
  fn test_get_element_text_recursive() {
    let input = "<div><span>Nested <strong>text</strong> content</span></div>";
    let document = Document::parse(input).unwrap();
    let div = document.descendants().find(|node| matches_tag(*node, "div")).unwrap();
    let text = get_element_text(div);
    assert_eq!(text, "Nested text content");
  }

  #[test]
  fn test_matches_tag() {
    let input = r#"<ac:structured-macro ac:name="test"></ac:structured-macro>"#;
    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let node = document
      .descendants()
      .find(|n| n.is_element() && n.tag_name().name() == "structured-macro")
      .unwrap();
    assert!(matches_tag(node, "ac:structured-macro"));
    assert!(!matches_tag(node, "structured-macro"));
  }

  #[test]
  fn test_get_attribute() {
    let input = r#"<ac:parameter ac:name="title">Test Title</ac:parameter>"#;
    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let node = document
      .descendants()
      .find(|n| n.is_element() && n.tag_name().name() == "parameter")
      .unwrap();
    assert_eq!(get_attribute(node, "ac:name"), Some("title".to_string()));
  }
}
