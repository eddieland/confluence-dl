//! AsciiDoc element converters for Confluence content.
//!
//! Handles conversion of standard HTML elements to AsciiDoc format,
//! including headings, paragraphs, links, lists, code blocks, and formatting.

use roxmltree::Node;
use tracing::debug;

use crate::asciidoc::AsciiDocOptions;
use crate::markdown::html_entities::decode_html_entities;
use crate::markdown::utils::{get_attribute, get_element_text, matches_tag, qualified_tag_name};

/// Converts an element and its children to AsciiDoc recursively.
///
/// # Arguments
/// * `node` - Root node whose descendants should be rendered.
/// * `options` - Conversion behaviour flags that control optional features.
///
/// # Returns
/// An AsciiDoc string representing the element and its descendants.
pub fn convert_node_to_asciidoc(node: Node, options: &AsciiDocOptions) -> String {
  let mut result = String::new();

  for child in node.children() {
    match child.node_type() {
      roxmltree::NodeType::Element => result.push_str(&convert_element_node(child, options)),
      roxmltree::NodeType::Text => {
        if let Some(text) = child.text() {
          let decoded = decode_html_entities(text);
          result.push_str(&decoded);
        }
      }
      _ => {}
    }
  }

  result
}

/// Formats a converted list item for AsciiDoc with proper indentation.
///
/// # Arguments
/// * `item` - Converted AsciiDoc representing the list item's body.
/// * `marker` - The list marker (e.g., `"*"` or `"."`) to use.
/// * `depth` - The nesting depth (1 = top level).
///
/// # Returns
/// Rendered AsciiDoc snippet for the list item.
fn format_list_item(item: &str, marker: &str, depth: usize) -> String {
  let prefix = marker.repeat(depth);
  let trimmed = item.trim();

  if trimmed.is_empty() {
    return format!("{prefix}\n");
  }

  // For simple content, output on same line
  let has_multiple_lines = trimmed.lines().count() > 1;

  if !has_multiple_lines {
    format!("{prefix} {trimmed}\n")
  } else {
    // For multi-line content, handle continuation properly
    let mut result = String::new();
    let mut first = true;
    for line in trimmed.lines() {
      if first {
        result.push_str(&format!("{prefix} {}\n", line.trim()));
        first = false;
      } else if line.trim().is_empty() {
        result.push('\n');
      } else {
        // Continuation lines - AsciiDoc uses + for list continuation
        result.push_str(&format!("+\n{}\n", line.trim()));
      }
    }
    result
  }
}

/// Converts arbitrary content into an AsciiDoc blockquote.
///
/// # Arguments
/// * `content` - Raw text that should be wrapped in blockquote syntax.
///
/// # Returns
/// AsciiDoc-formatted blockquote with proper delimiters.
fn render_blockquote(content: &str) -> String {
  let trimmed = content.trim();

  if trimmed.is_empty() {
    return "\n[quote]\n____\n____\n\n".to_string();
  }

  format!("\n[quote]\n____\n{trimmed}\n____\n\n")
}

fn convert_element_node(child: Node, options: &AsciiDocOptions) -> String {
  let mut result = String::new();
  let tag = child.tag_name();
  let local_name = tag.name();

  match local_name {
    // Headings - AsciiDoc uses = for headings
    "h1" => {
      let content = convert_node_to_asciidoc(child, options);
      result.push_str(&format!("\n= {}\n\n", content.trim()));
    }
    "h2" => {
      let content = convert_node_to_asciidoc(child, options);
      result.push_str(&format!("\n== {}\n\n", content.trim()));
    }
    "h3" => {
      let content = convert_node_to_asciidoc(child, options);
      result.push_str(&format!("\n=== {}\n\n", content.trim()));
    }
    "h4" => {
      let content = convert_node_to_asciidoc(child, options);
      result.push_str(&format!("\n==== {}\n\n", content.trim()));
    }
    "h5" => {
      let content = convert_node_to_asciidoc(child, options);
      result.push_str(&format!("\n===== {}\n\n", content.trim()));
    }
    "h6" => {
      let content = convert_node_to_asciidoc(child, options);
      result.push_str(&format!("\n====== {}\n\n", content.trim()));
    }

    // Paragraphs
    "p" => {
      let content = convert_node_to_asciidoc(child, options);
      let trimmed = content.trim();
      if !trimmed.is_empty() {
        result.push_str(&format!("{trimmed}\n\n"));
      }
    }

    // Text formatting - AsciiDoc uses single * for bold
    "strong" | "b" => {
      let content = convert_node_to_asciidoc(child, options);
      result.push_str(&format!("*{}*", content));
    }
    "em" | "i" => {
      let content = convert_node_to_asciidoc(child, options);
      result.push_str(&format!("_{}_", content));
    }
    "u" => {
      // AsciiDoc underline using role
      let content = convert_node_to_asciidoc(child, options);
      result.push_str(&format!("[underline]#{}#", content));
    }
    "s" | "del" => {
      // AsciiDoc strikethrough using role
      let content = convert_node_to_asciidoc(child, options);
      result.push_str(&format!("[line-through]#{}#", content));
    }
    "code" => {
      let content = convert_node_to_asciidoc(child, options);
      result.push_str(&format!("`{}`", content));
    }
    "sub" => {
      // AsciiDoc native subscript
      let content = convert_node_to_asciidoc(child, options);
      let trimmed = content.trim();
      if !trimmed.is_empty() {
        result.push_str(&format!("~{trimmed}~"));
      }
    }
    "sup" => {
      // AsciiDoc native superscript
      let content = convert_node_to_asciidoc(child, options);
      let trimmed = content.trim();
      if !trimmed.is_empty() {
        result.push_str(&format!("^{trimmed}^"));
      }
    }

    // Blockquotes
    "blockquote" => {
      let inner = convert_node_to_asciidoc(child, options);
      result.push_str(&render_blockquote(&inner));
    }

    // Lists - AsciiDoc uses * for unordered and . for ordered
    "ul" => {
      result.push('\n');
      convert_list_items(child, options, "*", 1, &mut result);
      result.push('\n');
    }
    "ol" => {
      result.push('\n');
      convert_list_items(child, options, ".", 1, &mut result);
      result.push('\n');
    }

    // Links - AsciiDoc uses url[text] format
    "a" => {
      let text = convert_node_to_asciidoc(child, options);
      let href = get_attribute(child, "href").unwrap_or_default();
      let trimmed_text = text.trim();

      if let Some(anchor) = href.strip_prefix('#') {
        // Internal anchor link - use AsciiDoc cross-reference
        result.push_str(&format!("<<{anchor},{trimmed_text}>>"));
      } else if trimmed_text.is_empty() || trimmed_text == href {
        // URL only or text matches URL
        result.push_str(&href);
      } else {
        // External link with different text
        result.push_str(&format!("{href}[{trimmed_text}]"));
      }
    }

    // Line breaks and horizontal rules
    "br" => result.push('\n'),
    "hr" => result.push_str("\n'''\n\n"),

    // Code blocks - AsciiDoc uses ---- delimiters
    "pre" => {
      let code = get_element_text(child);
      result.push_str(&format!("\n----\n{}\n----\n\n", code.trim()));
    }

    // Tables - basic support (tables are complex, basic impl for now)
    "table" => {
      result.push_str(&convert_table_to_asciidoc(child, options));
    }

    // Confluence-specific elements
    "link" if matches_tag(child, "ac:link") => {
      result.push_str(&convert_confluence_link(child));
    }

    // Images
    "image" if matches_tag(child, "ac:image") => {
      result.push_str(&convert_image_to_asciidoc(child));
    }

    // Layout elements - pass through content
    "layout" if matches_tag(child, "ac:layout") => {
      result.push_str(&convert_node_to_asciidoc(child, options));
    }
    "layout-section" if matches_tag(child, "ac:layout-section") => {
      result.push_str(&convert_node_to_asciidoc(child, options));
    }
    "layout-cell" if matches_tag(child, "ac:layout-cell") => {
      result.push_str(&convert_node_to_asciidoc(child, options));
    }
    "rich-text-body" if matches_tag(child, "ac:rich-text-body") => {
      result.push_str(&convert_node_to_asciidoc(child, options));
    }

    // Skip internal elements
    "url" if matches_tag(child, "ri:url") => {}
    "parameter" if matches_tag(child, "ac:parameter") => {}
    "task-id" if matches_tag(child, "ac:task-id") => {}
    "task-status" if matches_tag(child, "ac:task-status") => {}
    "task-body" if matches_tag(child, "ac:task-body") => {
      result.push_str(&get_element_text(child));
    }
    "placeholder" if matches_tag(child, "ac:placeholder") => {}

    // Time elements
    "time" => {
      let text = get_element_text(child);
      if !text.trim().is_empty() {
        result.push_str(&text);
      } else if let Some(datetime) = get_attribute(child, "datetime") {
        result.push_str(&datetime);
      }
    }

    // Span elements - pass through content
    "span" => {
      result.push_str(&convert_node_to_asciidoc(child, options));
    }

    // Unknown elements - extract content
    _ => {
      let debug_name = qualified_tag_name(child);
      debug!("Unknown AsciiDoc tag: {debug_name}");
      result.push_str(&convert_node_to_asciidoc(child, options));
    }
  }

  result
}

/// Convert list items recursively with proper depth tracking.
fn convert_list_items(node: Node, options: &AsciiDocOptions, marker: &str, depth: usize, result: &mut String) {
  for li in node.children().filter(|n| matches_tag(*n, "li")) {
    // Check for nested lists
    let mut item_content = String::new();
    let mut has_nested_list = false;

    for child in li.children() {
      match child.node_type() {
        roxmltree::NodeType::Text => {
          if let Some(text) = child.text() {
            item_content.push_str(&decode_html_entities(text));
          }
        }
        roxmltree::NodeType::Element => {
          if matches_tag(child, "ul") {
            has_nested_list = true;
            // Output current item content first
            if !item_content.trim().is_empty() {
              result.push_str(&format_list_item(&item_content, marker, depth));
              item_content.clear();
            }
            // Recursively handle nested unordered list
            convert_list_items(child, options, "*", depth + 1, result);
          } else if matches_tag(child, "ol") {
            has_nested_list = true;
            // Output current item content first
            if !item_content.trim().is_empty() {
              result.push_str(&format_list_item(&item_content, marker, depth));
              item_content.clear();
            }
            // Recursively handle nested ordered list
            convert_list_items(child, options, ".", depth + 1, result);
          } else {
            // For other elements, convert them recursively
            item_content.push_str(&convert_element_node(child, options));
          }
        }
        _ => {}
      }
    }

    // Output any remaining content
    if !item_content.trim().is_empty() || !has_nested_list {
      result.push_str(&format_list_item(&item_content, marker, depth));
    }
  }
}

/// Convert Confluence link to AsciiDoc.
fn convert_confluence_link(node: Node) -> String {
  // Try to find the link text
  let link_text = node
    .children()
    .find(|child| matches_tag(*child, "ac:link-body") || matches_tag(*child, "ac:plain-text-link-body"))
    .map(get_element_text)
    .unwrap_or_default();

  // Try to find the URL
  let url = node
    .children()
    .find(|child| matches_tag(*child, "ri:url"))
    .and_then(|url_node| get_attribute(url_node, "ri:value"))
    .unwrap_or_default();

  if url.is_empty() {
    link_text
  } else if link_text.is_empty() || link_text == url {
    url
  } else {
    format!("{url}[{link_text}]")
  }
}

/// Convert Confluence image to AsciiDoc.
fn convert_image_to_asciidoc(node: Node) -> String {
  let alt = get_attribute(node, "ac:alt").unwrap_or_default();

  // Try ri:url first
  if let Some(url_node) = node.children().find(|child| matches_tag(*child, "ri:url"))
    && let Some(src) = get_attribute(url_node, "ri:value") {
      return format!("image::{src}[{alt}]");
    }

  // Try ri:attachment
  if let Some(attachment_node) = node.children().find(|child| matches_tag(*child, "ri:attachment"))
    && let Some(filename) = get_attribute(attachment_node, "ri:filename") {
      return format!("image::{filename}[{alt}]");
    }

  // Fallback - return empty if no source found
  String::new()
}

/// Convert HTML table to AsciiDoc format.
fn convert_table_to_asciidoc(node: Node, options: &AsciiDocOptions) -> String {
  let mut rows: Vec<Vec<String>> = Vec::new();
  let mut has_header = false;

  // Find tbody, thead, or direct tr children
  let row_containers: Vec<Node> = node
    .children()
    .filter(|n| {
      matches_tag(*n, "tbody") || matches_tag(*n, "thead") || matches_tag(*n, "tfoot") || matches_tag(*n, "tr")
    })
    .collect();

  for container in row_containers {
    if matches_tag(container, "tr") {
      // Direct tr child
      let row = extract_table_row(container, options);
      if !row.is_empty() {
        // Check if this row has th elements (header)
        if container.children().any(|cell| matches_tag(cell, "th")) {
          has_header = true;
        }
        rows.push(row);
      }
    } else {
      // tbody/thead/tfoot container
      if matches_tag(container, "thead") {
        has_header = true;
      }
      for tr in container.children().filter(|n| matches_tag(*n, "tr")) {
        let row = extract_table_row(tr, options);
        if !row.is_empty() {
          rows.push(row);
        }
      }
    }
  }

  if rows.is_empty() {
    return String::new();
  }

  // Build AsciiDoc table
  let mut result = String::new();
  result.push_str("\n|===\n");

  for (i, row) in rows.iter().enumerate() {
    // Output cells
    for cell in row {
      result.push_str(&format!("| {cell} "));
    }
    result.push('\n');

    // Add blank line after header row
    if i == 0 && has_header {
      result.push('\n');
    }
  }

  result.push_str("|===\n\n");
  result
}

/// Extract cells from a table row.
fn extract_table_row(tr: Node, options: &AsciiDocOptions) -> Vec<String> {
  tr.children()
    .filter(|n| matches_tag(*n, "td") || matches_tag(*n, "th"))
    .map(|cell| {
      let content = convert_node_to_asciidoc(cell, options);
      // Clean up cell content - remove newlines and extra whitespace
      content.trim().replace('\n', " ")
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use roxmltree::Document;

  use super::*;
  use crate::asciidoc::AsciiDocOptions;
  use crate::markdown::utils::wrap_with_namespaces;

  fn convert_to_asciidoc(input: &str) -> String {
    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let asciidoc = convert_node_to_asciidoc(document.root_element(), &AsciiDocOptions::default());
    crate::asciidoc::utils::clean_asciidoc(&asciidoc)
  }

  #[test]
  fn test_convert_headings() {
    let input = "<h1>Title</h1><h2>Subtitle</h2><h3>Section</h3>";
    let output = convert_to_asciidoc(input);
    assert!(output.contains("= Title"));
    assert!(output.contains("== Subtitle"));
    assert!(output.contains("=== Section"));
  }

  #[test]
  fn test_convert_bold() {
    let input = "<p><strong>important</strong></p>";
    let output = convert_to_asciidoc(input);
    assert!(output.contains("*important*"));
  }

  #[test]
  fn test_convert_italic() {
    let input = "<p><em>emphasis</em></p>";
    let output = convert_to_asciidoc(input);
    assert!(output.contains("_emphasis_"));
  }

  #[test]
  fn test_convert_strikethrough() {
    let input = "<p><s>deleted</s></p>";
    let output = convert_to_asciidoc(input);
    assert!(output.contains("[line-through]#deleted#"));
  }

  #[test]
  fn test_convert_subscript() {
    let input = "<p>H<sub>2</sub>O</p>";
    let output = convert_to_asciidoc(input);
    assert!(output.contains("H~2~O"));
  }

  #[test]
  fn test_convert_superscript() {
    let input = "<p>x<sup>2</sup></p>";
    let output = convert_to_asciidoc(input);
    assert!(output.contains("x^2^"));
  }

  #[test]
  fn test_convert_link() {
    let input = r#"<a href="https://example.com">Example</a>"#;
    let output = convert_to_asciidoc(input);
    assert!(output.contains("https://example.com[Example]"));
  }

  #[test]
  fn test_convert_internal_link() {
    let input = r##"<a href="#section">Jump to section</a>"##;
    let output = convert_to_asciidoc(input);
    assert!(output.contains("<<section,Jump to section>>"));
  }

  #[test]
  fn test_convert_code_block() {
    let input = "<pre>fn main() {}</pre>";
    let output = convert_to_asciidoc(input);
    assert!(output.contains("----"));
    assert!(output.contains("fn main()"));
  }

  #[test]
  fn test_convert_unordered_list() {
    let input = "<ul><li>Item 1</li><li>Item 2</li></ul>";
    let output = convert_to_asciidoc(input);
    assert!(output.contains("* Item 1"));
    assert!(output.contains("* Item 2"));
  }

  #[test]
  fn test_convert_ordered_list() {
    let input = "<ol><li>First</li><li>Second</li></ol>";
    let output = convert_to_asciidoc(input);
    assert!(output.contains(". First"));
    assert!(output.contains(". Second"));
  }

  #[test]
  fn test_convert_nested_list() {
    let input = "<ul><li>Parent<ul><li>Child</li></ul></li></ul>";
    let output = convert_to_asciidoc(input);
    assert!(output.contains("* Parent"));
    assert!(output.contains("** Child"));
  }

  #[test]
  fn test_convert_horizontal_rule() {
    let input = "<p>Before</p><hr /><p>After</p>";
    let output = convert_to_asciidoc(input);
    assert!(output.contains("'''"));
  }

  #[test]
  fn test_convert_blockquote() {
    let input = "<blockquote><p>Quote text</p></blockquote>";
    let output = convert_to_asciidoc(input);
    assert!(output.contains("[quote]"));
    assert!(output.contains("____"));
    assert!(output.contains("Quote text"));
  }

  #[test]
  fn test_convert_image() {
    let input = r#"<ac:image ac:alt="test image"><ri:url ri:value="https://example.com/image.png" /></ac:image>"#;
    let output = convert_to_asciidoc(input);
    assert!(output.contains("image::https://example.com/image.png[test image]"));
  }

  #[test]
  fn test_convert_table() {
    let input = r#"
      <table>
        <tr><th>Header 1</th><th>Header 2</th></tr>
        <tr><td>Cell 1</td><td>Cell 2</td></tr>
      </table>
    "#;
    let output = convert_to_asciidoc(input);
    assert!(output.contains("|==="));
    assert!(output.contains("| Header 1"));
    assert!(output.contains("| Cell 1"));
  }
}
