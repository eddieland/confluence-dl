//! Utility functions for XML/HTML parsing and manipulation.
//!
//! Provides helper functions for working with roxmltree nodes, including
//! namespace handling, attribute access, and text extraction.

use std::collections::BTreeSet;

use roxmltree::Node;

/// Synthetic namespace base URL for Confluence namespaces.
pub const SYNTHETIC_NS_BASE: &str = "https://confluence.example/";

/// Collects all decoded text content from an element and its descendants.
///
/// Recursively walks the node tree so that nested inline markup is flattened
/// into a single string that can be emitted as Markdown.
///
/// # Arguments
/// * `node` - The starting element to collect text from.
///
/// # Returns
/// A `String` containing all text nodes with HTML entities decoded.
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

/// Splits a qualified tag name into its namespace prefix and local name.
///
/// Names without a colon return `None` for the prefix.
///
/// # Arguments
/// * `name` - The tag or attribute name such as `ac:rich-text-body`.
///
/// # Returns
/// A tuple of `(prefix, local_name)` where `prefix` is `None` when the value
/// is not namespaced.
pub fn split_qualified_name(name: &str) -> (Option<&str>, &str) {
  if let Some((prefix, local)) = name.split_once(':') {
    (Some(prefix), local)
  } else {
    (None, name)
  }
}

/// Wraps storage format markup with synthetic namespace declarations.
///
/// Confluence storage format frequently references namespaces such as `ac:`
/// or `ri:` without declaring them. The wrapper element allows `roxmltree`
/// to resolve those prefixes during parsing.
///
/// # Arguments
/// * `storage_content` - Raw storage format XML/HTML snippet from Confluence.
///
/// # Returns
/// A `String` containing the original content nested inside a synthetic root
/// element with namespace declarations.
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
///
/// # Arguments
/// * `prefix` - Candidate namespace prefix to validate.
///
/// # Returns
/// `true` when the prefix only contains ASCII letters, digits, hyphen, or
/// underscore.
fn is_valid_prefix(prefix: &str) -> bool {
  if prefix.is_empty() {
    return false;
  }
  prefix
    .chars()
    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Builds the fully qualified tag name of a node, including namespace prefix.
///
/// # Arguments
/// * `node` - The XML node whose name should be normalized.
///
/// # Returns
/// A `String` in the form `ns:name` when the node has a namespace or just the
/// local name when it does not.
pub fn qualified_tag_name(node: Node) -> String {
  let tag = node.tag_name();
  let name = tag.name();
  if let Some(namespace) = tag.namespace() {
    format!("{namespace}:{name}")
  } else {
    name.to_string()
  }
}

/// Tests whether a node matches an expected tag name with optional namespace.
///
/// # Arguments
/// * `node` - The element to check.
/// * `name` - The expected tag name, optionally including a prefix such as
///   `ac:rich-text-body`.
///
/// # Returns
/// `true` when the element matches the supplied name and namespace, otherwise
/// `false`.
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

/// Retrieves an attribute value from a node, handling namespaced attributes.
///
/// # Arguments
/// * `node` - The element to inspect.
/// * `attr_name` - The attribute to retrieve, optionally namespaced like
///   `ri:filename`.
///
/// # Returns
/// `Some(String)` containing the attribute value when present, otherwise
/// `None`.
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

/// Finds the first child element with a given tag name.
///
/// This helper understands the synthetic namespaces injected by
/// [`wrap_with_namespaces`].
///
/// # Arguments
/// * `node` - The parent element whose children should be searched.
/// * `tag_name` - The qualified tag name to look for, e.g. `ac:rich-text-body`.
///
/// # Returns
/// `Some(Node)` when a matching child exists, or `None` if none are found.
pub fn find_child_by_tag<'a, 'input>(node: Node<'a, 'input>, tag_name: &str) -> Option<Node<'a, 'input>> {
  node.children().find(|child| matches_tag(*child, tag_name))
}

/// Finds a child element that matches both a tag name and attribute value.
///
/// # Arguments
/// * `node` - The parent element whose children should be inspected.
/// * `tag_name` - The qualified tag name to match against.
/// * `attr_name` - The attribute name to compare, optionally namespaced.
/// * `attr_value` - The expected attribute value.
///
/// # Returns
/// `Some(Node)` when a matching child exists, or `None` if nothing matches.
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

/// Clean up the markdown output for more predictable downstream processing.
///
/// - Removes excessive blank lines (more than 2 consecutive)
/// - Trims leading/trailing whitespace
/// - Ensures the file ends with a newline
///
/// # Arguments
/// * `content` - Raw Markdown emitted by the converter.
///
/// # Returns
/// A normalized Markdown string that is safe to write to disk.
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
