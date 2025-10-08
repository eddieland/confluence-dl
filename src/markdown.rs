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
              let content = convert_element_to_markdown(&child_element);
              let trimmed = content.trim();
              if !trimmed.is_empty() {
                result.push_str(&format!("{trimmed}\n\n"));
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
              if let Some(emoji) = convert_span_emoji(&child_element) {
                result.push_str(&emoji);
              } else {
                result.push_str(&convert_element_to_markdown(&child_element));
              }
            }

            // Emojis
            "ac:emoji" | "ac:emoticon" => {
              result.push_str(&convert_emoji_to_markdown(&child_element));
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
    "note" | "info" | "warning" | "tip" => {
      let title = find_child_by_tag_and_attr(element, "ac:parameter", "ac:name", "title")
        .map(|e| e.text().collect::<String>())
        .unwrap_or_default();

      let body = find_child_by_tag(element, "ac:rich-text-body")
        .map(|elem| convert_element_to_markdown(&elem))
        .unwrap_or_else(|| get_element_text(element));

      format_admonition_block(macro_name, title.trim(), body.trim())
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
    "emoji" => find_child_by_tag_and_attr(element, "ac:parameter", "ac:name", "emoji-id")
      .map(|e| e.text().collect::<String>())
      .and_then(|id| emoji_id_to_unicode(id.trim()))
      .or_else(|| {
        find_child_by_tag_and_attr(element, "ac:parameter", "ac:name", "emoji").map(|e| e.text().collect::<String>())
      })
      .or_else(|| {
        find_child_by_tag_and_attr(element, "ac:parameter", "ac:name", "shortname")
          .map(|e| e.text().collect::<String>())
      })
      .unwrap_or_default(),
    "anchor" => String::new(), // Skip anchors
    _ => {
      // For unknown macros, just extract the text content
      get_element_text(element)
    }
  }
}

/// Format Confluence admonition macros (note, info, warning, tip) as Markdown
/// blockquotes
fn format_admonition_block(macro_name: &str, title: &str, body: &str) -> String {
  let default_title = match macro_name {
    "info" => "Info",
    "warning" => "Warning",
    "tip" => "Tip",
    _ => "Note",
  };

  let heading = if title.is_empty() { default_title } else { title };

  if body.is_empty() {
    return format!("\n> **{heading}:**\n\n");
  }

  let mut result = String::new();
  let mut lines = body.lines();

  if let Some(first_line) = lines.next() {
    result.push_str(&format!("\n> **{heading}:** {}", first_line.trim()));
  }

  for line in lines {
    if line.trim().is_empty() {
      result.push_str("\n>");
    } else {
      result.push_str(&format!("\n> {}", line.trim()));
    }
  }

  result.push_str("\n\n");
  result
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

/// Convert an emoji element to markdown by resolving its codepoint
fn convert_emoji_to_markdown(element: &scraper::ElementRef) -> String {
  if let Some(id) = element.value().attr("ac:emoji-id")
    && let Some(emoji) = emoji_id_to_unicode(id)
  {
    return emoji;
  }

  if let Some(shortcut) = element.value().attr("ac:shortcut") {
    return shortcut.to_string();
  }

  if let Some(shortname) = element
    .value()
    .attr("ac:shortname")
    .or_else(|| element.value().attr("ac:emoji-shortname"))
  {
    return shortname.to_string();
  }

  let text = get_element_text(element);
  if !text.trim().is_empty() { text } else { String::new() }
}

/// Try to resolve emoji metadata stored on span elements
fn convert_span_emoji(element: &scraper::ElementRef) -> Option<String> {
  let value = element.value();

  let has_metadata = value.attr("data-emoji-id").is_some()
    || value.attr("data-emoji-shortname").is_some()
    || value.attr("data-emoji-fallback").is_some();

  if !has_metadata {
    return None;
  }

  if let Some(id) = value.attr("data-emoji-id")
    && let Some(emoji) = emoji_id_to_unicode(id)
  {
    return Some(emoji);
  }

  let text = get_element_text(element);
  if !text.trim().is_empty() {
    return Some(text);
  }

  if let Some(shortname) = value
    .attr("data-emoji-shortname")
    .or_else(|| value.attr("data-emoji-fallback"))
  {
    return Some(shortname.to_string());
  }

  None
}

/// Convert an emoji identifier like "1f44b" or "1f469-200d-1f4bb" into unicode
fn emoji_id_to_unicode(id: &str) -> Option<String> {
  let trimmed = id.trim().trim_start_matches("emoji-").trim_start_matches("emoji/");
  if trimmed.is_empty() {
    return None;
  }

  let mut result = String::new();
  let normalized = trimmed.replace('_', "-");

  for part in normalized.split('-') {
    let part = part.trim();
    if part.is_empty() {
      continue;
    }

    let code = u32::from_str_radix(part, 16).ok()?;
    let ch = char::from_u32(code)?;
    result.push(ch);
  }

  if result.is_empty() { None } else { Some(result) }
}

/// Convert HTML table to markdown table
fn convert_table_to_markdown(element: &scraper::ElementRef) -> String {
  let mut rows: Vec<Vec<String>> = Vec::new();

  // Extract all rows
  for tr in element.select(&Selector::parse("tr").unwrap()) {
    let mut cells: Vec<String> = Vec::new();

    // Get cells (th or td)
    for cell in tr.select(&Selector::parse("th, td").unwrap()) {
      let text = get_element_text(&cell)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
      cells.push(text);
    }

    if !cells.is_empty() {
      rows.push(cells);
    }
  }

  if rows.is_empty() {
    return String::new();
  }

  let column_count = rows.iter().map(|row| row.len()).max().unwrap_or(0);
  if column_count == 0 {
    return String::new();
  }

  for row in &mut rows {
    row.resize(column_count, String::new());
  }

  let mut column_widths = vec![0; column_count];
  for row in &rows {
    for (index, cell) in row.iter().enumerate() {
      column_widths[index] = column_widths[index].max(cell.len());
    }
  }

  let mut result = String::new();
  result.push('\n');

  fn format_row(row: &[String], column_widths: &[usize]) -> String {
    let mut line = String::new();
    line.push('|');

    for (cell, width) in row.iter().zip(column_widths) {
      line.push(' ');
      line.push_str(cell);
      if *width > cell.len() {
        line.push_str(&" ".repeat(width - cell.len()));
      }
      line.push(' ');
      line.push('|');
    }

    line.push('\n');
    line
  }

  // Write header row (or first row if no header)
  if let Some(first_row) = rows.first() {
    result.push_str(&format_row(first_row, &column_widths));

    // Write separator that matches the header width for prettier output
    result.push('|');
    for width in &column_widths {
      let dash_count = (*width).max(3);
      result.push(' ');
      result.push_str(&"-".repeat(dash_count));
      result.push(' ');
      result.push('|');
    }
    result.push('\n');
  }

  // Write remaining rows
  for row in rows.iter().skip(1) {
    result.push_str(&format_row(row, &column_widths));
  }

  result.push('\n');
  result
}

/// Decode common HTML entities
fn decode_html_entities(text: &str) -> String {
  let replaced = text
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
    .replace("&#39;", "'");

  decode_numeric_html_entities(&replaced)
}

/// Decode numeric HTML entities so emoji references render properly
fn decode_numeric_html_entities(text: &str) -> String {
  let mut result = String::with_capacity(text.len());
  let mut index = 0;
  let bytes = text.as_bytes();

  while index < text.len() {
    if bytes[index] == b'&'
      && let Some(semi_offset) = text[index..].find(';')
    {
      let end = index + semi_offset;
      if let Some(decoded) = decode_numeric_entity(&text[index + 1..end]) {
        result.push_str(&decoded);
        index = end + 1;
        continue;
      }
    }

    let ch = text[index..].chars().next().unwrap();
    result.push(ch);
    index += ch.len_utf8();
  }

  result
}

fn decode_numeric_entity(entity: &str) -> Option<String> {
  let body = entity.strip_prefix('#')?;

  let (radix, digits) = if let Some(hex) = body.strip_prefix('x').or_else(|| body.strip_prefix('X')) {
    (16, hex)
  } else {
    (10, body)
  };

  if digits.is_empty()
    || !digits.chars().all(|c| {
      if radix == 16 {
        c.is_ascii_hexdigit()
      } else {
        c.is_ascii_digit()
      }
    })
  {
    return None;
  }

  let value = u32::from_str_radix(digits, radix).ok()?;
  let ch = char::from_u32(value)?;
  Some(ch.to_string())
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
    let input = "There&rsquo;s a lot&mdash;this &amp; that &#x1F642; &#128075;";
    let output = decode_html_entities(input);
    assert_eq!(output, "There's a lot—this & that 🙂 👋");
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
  fn test_convert_note_macro() {
    let input = r#"
      <ac:structured-macro ac:name="note">
        <ac:rich-text-body>
          <p>This is a note block.</p>
        </ac:rich-text-body>
      </ac:structured-macro>
    "#;

    let output = storage_to_markdown(input).unwrap();
    assert!(output.contains("> **Note:** This is a note block."));
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
    insta::assert_snapshot!(output, @r###"
    - [ ] Task 1
    - [x] Task 2
    "###);
  }

  #[test]
  fn test_convert_image() {
    let input = r#"<ac:image ac:alt="test image"><ri:url ri:value="https://example.com/image.png" /></ac:image>"#;
    let output = storage_to_markdown(input).unwrap();
    assert!(output.contains("![test image](https://example.com/image.png)"));
  }

  #[test]
  fn test_convert_table() {
    let input = r#"
      <table>
        <tr><th>Header 1</th><th>Header 2</th></tr>
        <tr><td>Row 1 Col 1</td><td>Row 1 Col 2</td></tr>
        <tr><td>Row 2 Col 1</td><td>Row 2 Col 2</td></tr>
      </table>
    "#;
    let output = storage_to_markdown(input).unwrap();
    insta::assert_snapshot!(output, @r###"
    | Header 1    | Header 2    |
    | ----------- | ----------- |
    | Row 1 Col 1 | Row 1 Col 2 |
    | Row 2 Col 1 | Row 2 Col 2 |
    "###);
  }

  #[test]
  fn test_convert_table_empty() {
    let input = "<table></table>";
    let output = storage_to_markdown(input).unwrap();
    // Empty table should produce minimal output
    assert!(!output.contains("|"));
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
    let output = storage_to_markdown(input).unwrap();
    insta::assert_snapshot!(output, @r"
    - Item 1
    - Item 2

          
    1. First
    2. Second
    ");
  }

  #[test]
  fn test_convert_code_block() {
    let input = "<pre>function test() {\n  return 42;\n}</pre>";
    let output = storage_to_markdown(input).unwrap();
    assert!(output.contains("```"));
    assert!(output.contains("function test()"));
  }

  #[test]
  fn test_convert_inline_code() {
    let input = "<p>Use <code>git commit</code> to save</p>";
    let output = storage_to_markdown(input).unwrap();
    assert!(output.contains("`git commit`"));
  }

  #[test]
  fn test_convert_horizontal_rule() {
    let input = "<p>Before</p><hr /><p>After</p>";
    let output = storage_to_markdown(input).unwrap();
    assert!(output.contains("---"));
  }

  #[test]
  fn test_convert_line_break() {
    let input = "<p>Line 1<br />Line 2</p>";
    let output = storage_to_markdown(input).unwrap();
    assert!(output.contains("Line 1\nLine 2"));
  }

  #[test]
  fn test_convert_macro_toc() {
    let input = r#"<ac:structured-macro ac:name="toc"></ac:structured-macro>"#;
    let output = storage_to_markdown(input).unwrap();
    assert!(output.contains("**Table of Contents**"));
  }

  #[test]
  fn test_convert_macro_panel() {
    let input = r#"
      <ac:structured-macro ac:name="panel">
        <ac:rich-text-body>
          <p>Panel content here</p>
        </ac:rich-text-body>
      </ac:structured-macro>
    "#;
    let output = storage_to_markdown(input).unwrap();
    insta::assert_snapshot!(output, @r###"
    > Panel content here
    "###);
  }

  #[test]
  fn test_convert_macro_status() {
    let input = r#"
      <ac:structured-macro ac:name="status">
        <ac:parameter ac:name="title">In Progress</ac:parameter>
      </ac:structured-macro>
    "#;
    let output = storage_to_markdown(input).unwrap();
    assert!(output.contains("`[In Progress]`"));
  }

  #[test]
  fn test_convert_macro_expand() {
    let input = r#"
      <ac:structured-macro ac:name="expand">
        <ac:parameter ac:name="title">Click to expand</ac:parameter>
        <ac:rich-text-body>
          <p>Hidden content</p>
        </ac:rich-text-body>
      </ac:structured-macro>
    "#;
    let output = storage_to_markdown(input).unwrap();
    insta::assert_snapshot!(output, @r###"
    <details>
    <summary>Click to expand</summary>

    Hidden content
    </details>
    "###);
  }

  #[test]
  fn test_convert_macro_expand_default_title() {
    let input = r#"
      <ac:structured-macro ac:name="expand">
        <ac:rich-text-body>
          <p>Content without title</p>
        </ac:rich-text-body>
      </ac:structured-macro>
    "#;
    let output = storage_to_markdown(input).unwrap();
    assert!(output.contains("<summary>Details</summary>"));
  }

  #[test]
  fn test_convert_macro_anchor() {
    let input = r#"<ac:structured-macro ac:name="anchor"><ac:parameter ac:name="name">section1</ac:parameter></ac:structured-macro>"#;
    let output = storage_to_markdown(input).unwrap();
    // Anchor should produce empty output
    assert!(!output.contains("anchor"));
  }

  #[test]
  fn test_convert_macro_unknown() {
    let input = r#"<ac:structured-macro ac:name="unknown-macro">Some text content</ac:structured-macro>"#;
    let output = storage_to_markdown(input).unwrap();
    // Unknown macros should extract text content
    assert!(output.contains("Some text content"));
  }

  #[test]
  fn test_convert_underline() {
    let input = "<p><u>underlined text</u></p>";
    let output = storage_to_markdown(input).unwrap();
    assert!(output.contains("_underlined text_"));
  }

  #[test]
  fn test_convert_strikethrough() {
    let input = "<p><s>strike</s> and <del>delete</del></p>";
    let output = storage_to_markdown(input).unwrap();
    insta::assert_snapshot!(output, @"~~strike~~ and ~~delete~~");
  }

  #[test]
  fn test_convert_layout_sections() {
    let input = r#"
      <ac:layout>
        <ac:layout-section>
          <ac:layout-cell>
            <p>Cell content</p>
          </ac:layout-cell>
        </ac:layout-section>
      </ac:layout>
    "#;
    let output = storage_to_markdown(input).unwrap();
    insta::assert_snapshot!(output, @"Cell content");
  }

  #[test]
  fn test_convert_rich_text_body() {
    let input = r#"<ac:rich-text-body><p>Rich text</p></ac:rich-text-body>"#;
    let output = storage_to_markdown(input).unwrap();
    assert!(output.contains("Rich text"));
  }

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
  fn test_convert_image_without_url() {
    let input = r#"<ac:image ac:alt="no url"></ac:image>"#;
    let output = storage_to_markdown(input).unwrap();
    assert!(output.contains("![no url]()"));
  }

  #[test]
  fn test_convert_image_without_alt() {
    let input = r#"<ac:image><ri:url ri:value="https://example.com/img.png" /></ac:image>"#;
    let output = storage_to_markdown(input).unwrap();
    assert!(output.contains("![image](https://example.com/img.png)"));
  }

  #[test]
  fn test_convert_confluence_emoji_from_id() {
    let input = r#"<p>Hello <ac:emoji ac:emoji-id="1f44b" /></p>"#;
    let output = storage_to_markdown(input).unwrap();
    assert!(output.contains("Hello 👋"));
  }

  #[test]
  fn test_convert_confluence_emoji_multi_codepoint() {
    let input = r#"<p><ac:emoji ac:emoji-id="1f469-200d-1f4bb" /></p>"#;
    let output = storage_to_markdown(input).unwrap();
    assert!(output.contains("👩‍💻"));
  }

  #[test]
  fn test_convert_confluence_emoji_shortcut_fallback() {
    let input = r#"<p><ac:emoji ac:shortcut=":)" /></p>"#;
    let output = storage_to_markdown(input).unwrap();
    assert!(output.contains(":)"));
  }

  #[test]
  fn test_convert_emoji_macro() {
    let input = r#"
      <ac:structured-macro ac:name="emoji">
        <ac:parameter ac:name="emoji-id">1f60a</ac:parameter>
      </ac:structured-macro>
    "#;
    let output = storage_to_markdown(input).unwrap();
    assert!(output.contains("😊"));
  }

  #[test]
  fn test_convert_span_extracts_content() {
    let input = "<p><span>Span content</span></p>";
    let output = storage_to_markdown(input).unwrap();
    assert!(output.contains("Span content"));
  }

  #[test]
  fn test_convert_empty_paragraph() {
    let input = "<p></p><p>   </p>";
    let output = storage_to_markdown(input).unwrap();
    // Empty paragraphs should not produce extra newlines
    assert!(!output.contains("\n\n\n"));
  }

  #[test]
  fn test_get_element_text_recursive() {
    let input = "<div><span>Nested <strong>text</strong> content</span></div>";
    let document = Html::parse_document(input);
    let div = document.select(&Selector::parse("div").unwrap()).next().unwrap();
    let text = get_element_text(&div);
    assert_eq!(text, "Nested text content");
  }

  #[test]
  fn test_decode_all_entities() {
    let input = "&nbsp;&rsquo;&lsquo;&rdquo;&ldquo;&mdash;&ndash;&amp;&lt;&gt;&quot;&rarr;&larr;&#39;";
    let output = decode_html_entities(input);
    assert_eq!(output, " ''\"\"—–&<>\"→←'");
  }
}
