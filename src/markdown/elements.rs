//! Basic HTML element to Markdown converters.
//!
//! Handles conversion of standard HTML elements like headings, paragraphs,
//! links, lists, code blocks, and formatting.

use roxmltree::Node;
use tracing::debug;

use super::emoji::{convert_emoji_to_markdown, convert_span_emoji};
use super::html_entities::decode_html_entities;
use super::macros::{
  convert_confluence_link_to_markdown, convert_image_to_markdown, convert_macro_to_markdown,
  convert_task_list_to_markdown,
};
use super::tables::convert_table_to_markdown;
use super::utils::{get_attribute, get_element_text, matches_tag};

fn looks_like_list_marker(line: &str) -> bool {
  let trimmed = line.trim_start();

  if trimmed.starts_with(['-', '*', '+']) {
    return trimmed.len() > 1 && trimmed.as_bytes()[1] == b' ';
  }

  let mut chars = trimmed.chars();
  let mut saw_digit = false;

  while let Some(ch) = chars.next() {
    if ch.is_ascii_digit() {
      saw_digit = true;
      continue;
    }

    if ch == '.' {
      return saw_digit && matches!(chars.next(), Some(' '));
    }

    break;
  }

  false
}

fn format_list_item(item: &str, prefix: &str) -> String {
  let mut formatted = String::new();
  let lines = item.trim_end().lines();
  let indentation = " ".repeat(prefix.chars().count());
  let mut wrote_first_line = false;

  for line in lines {
    if !wrote_first_line {
      if line.trim().is_empty() {
        continue;
      }

      let line_content = line.trim_start();

      if looks_like_list_marker(line_content) {
        formatted.push_str(prefix.trim_end());
        formatted.push('\n');
        formatted.push_str(&indentation);
        formatted.push_str(line_content);
        formatted.push('\n');
      } else {
        formatted.push_str(prefix);
        formatted.push_str(line_content);
        formatted.push('\n');
      }

      wrote_first_line = true;
    } else if line.trim().is_empty() {
      formatted.push('\n');
    } else {
      formatted.push_str(&indentation);
      formatted.push_str(line);
      formatted.push('\n');
    }
  }

  if !wrote_first_line {
    formatted.push_str(prefix.trim_end());
    formatted.push('\n');
  }

  formatted
}

/// Converts an element and its children to Markdown recursively.
///
/// # Arguments
/// * `node` - Root node whose descendants should be rendered.
///
/// # Returns
/// A Markdown string representing the element and its descendants.
pub fn convert_node_to_markdown(node: Node) -> String {
  let mut result = String::new();

  for child in node.children() {
    match child.node_type() {
      roxmltree::NodeType::Element => {
        let tag = child.tag_name();
        let local_name = tag.name();

        match local_name {
          // Headings
          "h1" => result.push_str(&format!("\n# {}\n\n", convert_node_to_markdown(child).trim())),
          "h2" => result.push_str(&format!("\n## {}\n\n", convert_node_to_markdown(child).trim())),
          "h3" => result.push_str(&format!("\n### {}\n\n", convert_node_to_markdown(child).trim())),
          "h4" => result.push_str(&format!("\n#### {}\n\n", convert_node_to_markdown(child).trim())),
          "h5" => result.push_str(&format!("\n##### {}\n\n", convert_node_to_markdown(child).trim())),
          "h6" => result.push_str(&format!("\n###### {}\n\n", convert_node_to_markdown(child).trim())),

          // Paragraphs
          "p" => {
            let content = convert_node_to_markdown(child);
            let trimmed = content.trim();
            if !trimmed.is_empty() {
              result.push_str(&format!("{trimmed}\n\n"));
            }
          }

          // Text formatting
          "strong" | "b" => result.push_str(&format!("**{}**", convert_node_to_markdown(child))),
          "em" | "i" => result.push_str(&format!("_{}_", convert_node_to_markdown(child))),
          "u" => result.push_str(&format!("_{}_", convert_node_to_markdown(child))),
          "s" | "del" => result.push_str(&format!("~~{}~~", convert_node_to_markdown(child))),
          "code" => result.push_str(&format!("`{}`", convert_node_to_markdown(child))),

          // Lists
          "ul" => {
            result.push('\n');
            for li in child.children().filter(|n| matches_tag(*n, "li")) {
              let item = convert_node_to_markdown(li);
              result.push_str(&format_list_item(&item, "- "));
            }
            result.push('\n');
          }
          "ol" => {
            result.push('\n');
            for (index, li) in child.children().filter(|n| matches_tag(*n, "li")).enumerate() {
              let item = convert_node_to_markdown(li);
              let prefix = format!("{}. ", index + 1);
              result.push_str(&format_list_item(&item, &prefix));
            }
            result.push('\n');
          }

          // Links
          "a" => {
            let text = convert_node_to_markdown(child);
            let href = get_attribute(child, "href").unwrap_or_default();
            result.push_str(&format!("[{}]({})", text.trim(), href));
          }

          // Line breaks and horizontal rules
          "br" => result.push('\n'),
          "hr" => result.push_str("\n---\n\n"),

          // Code blocks
          "pre" => {
            let code = get_element_text(child);
            result.push_str(&format!("\n```\n{}\n```\n\n", code.trim()));
          }

          // Tables
          "table" => result.push_str(&convert_table_to_markdown(child)),

          // Confluence-specific elements
          "link" if matches_tag(child, "ac:link") => {
            result.push_str(&convert_confluence_link_to_markdown(child));
          }
          "structured-macro" if matches_tag(child, "ac:structured-macro") => {
            result.push_str(&convert_macro_to_markdown(child, &convert_node_to_markdown));
          }
          "task-list" if matches_tag(child, "ac:task-list") => {
            result.push_str(&convert_task_list_to_markdown(child));
          }
          "image" if matches_tag(child, "ac:image") => {
            result.push_str(&convert_image_to_markdown(child));
          }

          // Layout elements (just extract content)
          "layout" if matches_tag(child, "ac:layout") => {
            result.push_str(&convert_node_to_markdown(child));
          }
          "layout-section" if matches_tag(child, "ac:layout-section") => {
            result.push_str(&convert_node_to_markdown(child));
          }
          "layout-cell" if matches_tag(child, "ac:layout-cell") => {
            result.push_str(&convert_node_to_markdown(child));
          }
          "rich-text-body" if matches_tag(child, "ac:rich-text-body") => {
            result.push_str(&convert_node_to_markdown(child));
          }

          // Skip these internal elements
          "url" if matches_tag(child, "ri:url") => {}
          "parameter" if matches_tag(child, "ac:parameter") => {}
          "task-id" if matches_tag(child, "ac:task-id") => {}
          "task-status" if matches_tag(child, "ac:task-status") => {}
          "task-body" if matches_tag(child, "ac:task-body") => {
            result.push_str(&get_element_text(child));
          }
          "placeholder" if matches_tag(child, "ac:placeholder") => {}

          // Time elements - prefer visible text, fall back to datetime attribute
          "time" => {
            let text = get_element_text(child);
            if !text.trim().is_empty() {
              result.push_str(&text);
            } else if let Some(datetime) = get_attribute(child, "datetime") {
              result.push_str(&datetime);
            }
          }

          // Span elements (check for emoji metadata)
          "span" => {
            if let Some(emoji) = convert_span_emoji(child) {
              result.push_str(&emoji);
            } else {
              result.push_str(&convert_node_to_markdown(child));
            }
          }

          // Emoji elements
          "emoji" if matches_tag(child, "ac:emoji") => {
            result.push_str(&convert_emoji_to_markdown(child));
          }
          "emoticon" if matches_tag(child, "ac:emoticon") => {
            result.push_str(&convert_emoji_to_markdown(child));
          }

          // Unknown elements - extract content
          _ => {
            let debug_name = super::utils::qualified_tag_name(child);
            debug!("Unknown tag: {debug_name}");
            result.push_str(&convert_node_to_markdown(child));
          }
        }
      }
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

#[cfg(test)]
mod tests {
  use super::*;

  fn convert_to_markdown(input: &str) -> String {
    use roxmltree::Document;

    use crate::markdown::utils::wrap_with_namespaces;

    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let markdown = convert_node_to_markdown(document.root_element());
    crate::markdown::utils::clean_markdown(&markdown)
  }

  #[test]
  fn test_convert_headings() {
    let input = "<h1>Title</h1><h2>Subtitle</h2>";
    let output = convert_to_markdown(input);
    assert!(output.contains("# Title"));
    assert!(output.contains("## Subtitle"));
  }

  #[test]
  fn test_convert_formatting() {
    let input = "<p><strong>bold</strong> <em>italic</em> <s>strike</s></p>";
    let output = convert_to_markdown(input);
    assert!(output.contains("**bold**"));
    assert!(output.contains("_italic_"));
    assert!(output.contains("~~strike~~"));
  }

  #[test]
  fn test_convert_links() {
    let input = r#"<a href="https://example.com">Example</a>"#;
    let output = convert_to_markdown(input);
    assert!(output.contains("[Example](https://example.com)"));
  }

  #[test]
  fn test_convert_time_with_text_content() {
    let input = "<p>Meeting at <time datetime=\"2025-10-07\">October 7, 2025</time></p>";
    let output = convert_to_markdown(input);
    assert!(output.contains("Meeting at October 7, 2025"));
  }

  #[test]
  fn test_convert_time_with_datetime_attribute() {
    let input = "<p>Meeting at <time datetime=\"2025-10-07\" /></p>";
    let output = convert_to_markdown(input);
    assert!(output.contains("Meeting at 2025-10-07"));
  }

  #[test]
  fn test_convert_lists() {
    let input = r#"
      <ul>
        <li>Item 1</li>
        <li>Item 2</li>
      </ul>
      <ol>
        <li>First</li>
        <li>Second</li>
      </ol>
    "#;
    let result = convert_to_markdown(input);
    // Multiline inline snapshots with funky spacing confuse rustfmt, so keep this
    // escaped.
    let output = result.escape_default();
    insta::assert_snapshot!(output, @r"- Item 1\n- Item 2\n\n      \n1. First\n2. Second\n");
  }

  #[test]
  fn test_convert_nested_lists() {
    let input = r#"
      <ul>
        <li>Parent
          <ul>
            <li>Child</li>
            <li>Nested
              <ul>
                <li>Grandchild</li>
              </ul>
            </li>
          </ul>
        </li>
      </ul>
    "#;

    let result = convert_to_markdown(input);
    let output = result.escape_default();

    insta::assert_snapshot!(
      output,
      @r"- Parent\n\n  - Child\n  - Nested\n\n    - Grandchild\n"
    );
  }

  #[test]
  fn test_convert_code_block() {
    let input = "<pre>function test() {\n  return 42;\n}</pre>";
    let output = convert_to_markdown(input);
    assert!(output.contains("```"));
    assert!(output.contains("function test()"));
  }

  #[test]
  fn test_convert_inline_code() {
    let input = "<p>Use <code>git commit</code> to save</p>";
    let output = convert_to_markdown(input);
    assert!(output.contains("`git commit`"));
  }
}
