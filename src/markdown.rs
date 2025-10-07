//! Markdown conversion utilities for Confluence content.
//!
//! This module provides functionality to convert Confluence storage format
//! (XHTML-like) to Markdown using proper HTML parsing.

use anyhow::Result;
use scraper::{Html, Node, Selector};

/// Convert Confluence storage format to Markdown
///
/// This implementation uses proper HTML parsing to handle Confluence's
/// complex XML/HTML structure.
pub fn storage_to_markdown(storage_content: &str) -> Result<String> {
  // Parse the HTML/XML content
  let document = Html::parse_document(storage_content);

  // Convert to markdown
  let markdown = convert_element_to_markdown(&document.root_element());

  // Clean up the result
  let cleaned = clean_markdown(&markdown);

  Ok(cleaned)
}

/// Convert an element and its children to markdown recursively
fn convert_element_to_markdown(element: &scraper::ElementRef) -> String {
  let mut result = String::new();

  for child in element.children() {
    match child.value() {
      Node::Element(elem) => {
        let tag_name = elem.name();

        if let Some(child_element) = scraper::ElementRef::wrap(child) {
          match tag_name {
            // Headings
            "h1" => result.push_str(&format!("\n# {}\n\n", get_element_text(&child_element))),
            "h2" => result.push_str(&format!("\n## {}\n\n", get_element_text(&child_element))),
            "h3" => result.push_str(&format!("\n### {}\n\n", get_element_text(&child_element))),
            "h4" => result.push_str(&format!("\n#### {}\n\n", get_element_text(&child_element))),
            "h5" => result.push_str(&format!("\n##### {}\n\n", get_element_text(&child_element))),
            "h6" => result.push_str(&format!("\n###### {}\n\n", get_element_text(&child_element))),

            // Paragraphs
            "p" => {
              let text = get_element_text(&child_element).trim().to_string();
              if !text.is_empty() {
                result.push_str(&format!("{text}\n\n"));
              }
            }

            // Text formatting
            "strong" | "b" => result.push_str(&format!("**{}**", get_element_text(&child_element))),
            "em" | "i" => result.push_str(&format!("_{}_", get_element_text(&child_element))),
            "u" => result.push_str(&format!("_{}_", get_element_text(&child_element))),
            "s" | "del" => result.push_str(&format!("~~{}~~", get_element_text(&child_element))),
            "code" => result.push_str(&format!("`{}`", get_element_text(&child_element))),

            // Lists
            "ul" => {
              result.push('\n');
              for li in child_element.select(&Selector::parse("li").unwrap()) {
                result.push_str(&format!("- {}\n", get_element_text(&li).trim()));
              }
              result.push('\n');
            }
            "ol" => {
              result.push('\n');
              for (i, li) in child_element.select(&Selector::parse("li").unwrap()).enumerate() {
                result.push_str(&format!("{}. {}\n", i + 1, get_element_text(&li).trim()));
              }
              result.push('\n');
            }

            // Links
            "a" => {
              let text = get_element_text(&child_element);
              let href = child_element.value().attr("href").unwrap_or("");
              result.push_str(&format!("[{}]({})", text.trim(), href));
            }

            // Line breaks
            "br" => result.push('\n'),
            "hr" => result.push_str("\n---\n\n"),

            // Code blocks
            "pre" => {
              let code = get_element_text(&child_element);
              result.push_str(&format!("\n```\n{}\n```\n\n", code.trim()));
            }

            // Tables
            "table" => {
              result.push_str(&convert_table_to_markdown(&child_element));
            }

            // Confluence-specific macros
            "ac:structured-macro" => {
              result.push_str(&convert_macro_to_markdown(&child_element));
            }

            // Confluence task lists
            "ac:task-list" => {
              result.push_str(&convert_task_list_to_markdown(&child_element));
            }

            // Confluence images
            "ac:image" => {
              result.push_str(&convert_image_to_markdown(&child_element));
            }

            // Layout sections (just extract content)
            "ac:layout" | "ac:layout-section" | "ac:layout-cell" | "ac:rich-text-body" => {
              result.push_str(&convert_element_to_markdown(&child_element));
            }

            // Skip these Confluence-specific tags
            "ri:url" | "ac:parameter" | "ac:task-id" | "ac:task-status" | "ac:task-body" => {
              // For task-body, still extract the text
              if tag_name == "ac:task-body" {
                result.push_str(&get_element_text(&child_element));
              }
            }

            // Span elements - just extract content
            "span" => {
              result.push_str(&convert_element_to_markdown(&child_element));
            }

            // Default: recurse into children
            _ => {
              result.push_str(&convert_element_to_markdown(&child_element));
            }
          }
        }
      }
      Node::Text(text) => {
        // Decode HTML entities and add text
        let decoded = decode_html_entities(text);
        result.push_str(&decoded);
      }
      _ => {
        // Ignore comments, doctypes, etc.
      }
    }
  }

  result
}

/// Get all text content from an element and its children
fn get_element_text(element: &scraper::ElementRef) -> String {
  let mut text = String::new();

  for child in element.children() {
    match child.value() {
      Node::Text(t) => text.push_str(&decode_html_entities(t)),
      Node::Element(_) => {
        if let Some(child_elem) = scraper::ElementRef::wrap(child) {
          text.push_str(&get_element_text(&child_elem));
        }
      }
      _ => {}
    }
  }

  text
}

/// Convert Confluence structured macros to markdown
fn convert_macro_to_markdown(element: &scraper::ElementRef) -> String {
  let macro_name = element.value().attr("ac:name").unwrap_or("");

  match macro_name {
    "toc" => "\n**Table of Contents**\n\n".to_string(),
    "panel" => {
      // Extract rich text body if present - iterate children since namespaced
      // elements aren't valid CSS selectors
      let body = find_child_by_tag(element, "ac:rich-text-body")
        .map(|elem| convert_element_to_markdown(&elem))
        .unwrap_or_else(|| get_element_text(element));
      format!("\n> {}\n\n", body.trim())
    }
    "status" => {
      let title = find_child_by_tag_and_attr(element, "ac:parameter", "ac:name", "title")
        .map(|e| e.text().collect::<String>())
        .unwrap_or_default();
      format!("`[{title}]`")
    }
    "expand" => {
      let title = find_child_by_tag_and_attr(element, "ac:parameter", "ac:name", "title")
        .map(|e| e.text().collect::<String>())
        .unwrap_or_else(|| "Details".to_string());

      let body = find_child_by_tag(element, "ac:rich-text-body")
        .map(|elem| convert_element_to_markdown(&elem))
        .unwrap_or_else(|| get_element_text(element));

      format!(
        "\n<details>\n<summary>{}</summary>\n\n{}\n</details>\n\n",
        title,
        body.trim()
      )
    }
    "anchor" => String::new(), // Skip anchors
    _ => {
      // For unknown macros, just extract the text content
      get_element_text(element)
    }
  }
}

/// Find a child element by tag name (handles namespaced tags)
fn find_child_by_tag<'a>(element: &'a scraper::ElementRef, tag_name: &str) -> Option<scraper::ElementRef<'a>> {
  for child in element.children() {
    if let Node::Element(elem) = child.value()
      && elem.name() == tag_name
    {
      return scraper::ElementRef::wrap(child);
    }
  }
  None
}

/// Find a child element by tag name and attribute value
fn find_child_by_tag_and_attr<'a>(
  element: &'a scraper::ElementRef,
  tag_name: &str,
  attr_name: &str,
  attr_value: &str,
) -> Option<scraper::ElementRef<'a>> {
  for child in element.children() {
    if let Node::Element(elem) = child.value()
      && elem.name() == tag_name
      && let Some(child_elem) = scraper::ElementRef::wrap(child)
      && child_elem.value().attr(attr_name) == Some(attr_value)
    {
      return Some(child_elem);
    }
  }
  None
}

/// Convert Confluence task list to markdown checkboxes
fn convert_task_list_to_markdown(element: &scraper::ElementRef) -> String {
  let mut result = String::new();

  // Iterate through children to find ac:task elements
  for child in element.children() {
    if let Node::Element(elem) = child.value()
      && elem.name() == "ac:task"
      && let Some(task) = scraper::ElementRef::wrap(child)
    {
      let status = find_child_by_tag(&task, "ac:task-status")
        .map(|e| e.text().collect::<String>())
        .unwrap_or_else(|| "incomplete".to_string());

      let body = find_child_by_tag(&task, "ac:task-body")
        .map(|e| get_element_text(&e))
        .unwrap_or_default();

      let checkbox = if status.trim() == "complete" { "[x]" } else { "[ ]" };
      result.push_str(&format!("- {} {}\n", checkbox, body.trim()));
    }
  }

  result.push('\n');
  result
}

/// Convert Confluence image to markdown
fn convert_image_to_markdown(element: &scraper::ElementRef) -> String {
  // Try to find the image URL using child iteration
  let url = find_child_by_tag(element, "ri:url")
    .and_then(|e| e.value().attr("ri:value"))
    .unwrap_or("");

  let alt = element.value().attr("ac:alt").unwrap_or("image");

  if !url.is_empty() {
    format!("\n![{alt}]({url})\n\n")
  } else {
    format!("\n![{alt}]()\n\n")
  }
}

/// Convert HTML table to markdown table
fn convert_table_to_markdown(element: &scraper::ElementRef) -> String {
  let mut rows: Vec<Vec<String>> = Vec::new();

  // Extract all rows
  for tr in element.select(&Selector::parse("tr").unwrap()) {
    let mut cells: Vec<String> = Vec::new();

    // Get cells (th or td)
    for cell in tr.select(&Selector::parse("th, td").unwrap()) {
      let text = get_element_text(&cell).trim().to_string();
      cells.push(text);
    }

    if !cells.is_empty() {
      rows.push(cells);
    }
  }

  if rows.is_empty() {
    return String::new();
  }

  let mut result = String::new();
  result.push('\n');

  // Write header row (or first row if no header)
  if let Some(first_row) = rows.first() {
    result.push_str("| ");
    result.push_str(&first_row.join(" | "));
    result.push_str(" |\n");

    // Write separator
    result.push('|');
    for _ in 0..first_row.len() {
      result.push_str(" --- |");
    }
    result.push('\n');
  }

  // Write remaining rows
  for row in rows.iter().skip(1) {
    result.push_str("| ");
    result.push_str(&row.join(" | "));
    result.push_str(" |\n");
  }

  result.push('\n');
  result
}

/// Decode common HTML entities
fn decode_html_entities(text: &str) -> String {
  text
    .replace("&nbsp;", " ")
    .replace("&rsquo;", "'")
    .replace("&lsquo;", "'")
    .replace("&rdquo;", "\"")
    .replace("&ldquo;", "\"")
    .replace("&mdash;", "—")
    .replace("&ndash;", "–")
    .replace("&amp;", "&")
    .replace("&lt;", "<")
    .replace("&gt;", ">")
    .replace("&quot;", "\"")
    .replace("&rarr;", "→")
    .replace("&larr;", "←")
    .replace("&#39;", "'")
}

/// Clean up the markdown output
fn clean_markdown(content: &str) -> String {
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
  use super::*;

  #[test]
  fn test_decode_html_entities() {
    let input = "There&rsquo;s a lot&mdash;this &amp; that";
    let output = decode_html_entities(input);
    assert_eq!(output, "There's a lot—this & that");
  }

  #[test]
  fn test_convert_headings() {
    let input = "<h1>Title</h1><h2>Subtitle</h2>";
    let output = storage_to_markdown(input).unwrap();
    assert!(output.contains("# Title"));
    assert!(output.contains("## Subtitle"));
  }

  #[test]
  fn test_convert_formatting() {
    let input = "<p><strong>bold</strong> <em>italic</em> <s>strike</s></p>";
    let output = storage_to_markdown(input).unwrap();
    assert!(output.contains("**bold**"));
    assert!(output.contains("_italic_"));
    assert!(output.contains("~~strike~~"));
  }

  #[test]
  fn test_convert_links() {
    let input = r#"<a href="https://example.com">Example</a>"#;
    let output = storage_to_markdown(input).unwrap();
    assert!(output.contains("[Example](https://example.com)"));
  }

  #[test]
  fn test_convert_task_list() {
    let input = r#"
      <ac:task-list>
        <ac:task>
          <ac:task-status>incomplete</ac:task-status>
          <ac:task-body>Task 1</ac:task-body>
        </ac:task>
        <ac:task>
          <ac:task-status>complete</ac:task-status>
          <ac:task-body>Task 2</ac:task-body>
        </ac:task>
      </ac:task-list>
    "#;
    let output = storage_to_markdown(input).unwrap();
    assert!(output.contains("[ ] Task 1"));
    assert!(output.contains("[x] Task 2"));
  }

  #[test]
  fn test_convert_image() {
    let input = r#"<ac:image ac:alt="test image"><ri:url ri:value="https://example.com/image.png" /></ac:image>"#;
    let output = storage_to_markdown(input).unwrap();
    assert!(output.contains("![test image](https://example.com/image.png)"));
  }
}
