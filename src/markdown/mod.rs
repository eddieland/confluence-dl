//! Markdown conversion utilities for Confluence content.
//!
//! This module provides functionality to convert Confluence storage format
//! (XHTML-like) to Markdown using proper HTML parsing.
//!
//! # Architecture
//!
//! The conversion is split into focused modules:
//! - [`html_entities`] - HTML entity encoding/decoding
//! - [`emoji`] - Emoji conversion from Confluence format
//! - [`tables`] - HTML table to Markdown table conversion
//! - [`macros`] - Confluence macro handling (panels, notes, etc.)
//! - [`elements`] - Basic HTML element converters
//! - [`utils`] - Utility functions for XML parsing and manipulation
//!
//! # Example
//!
//! ```no_run
//! use confluence_dl::markdown::storage_to_markdown;
//!
//! let confluence_html = r#"<h1>Title</h1><p><strong>Bold text</strong></p>"#;
//! let markdown = storage_to_markdown(confluence_html, 0).unwrap();
//! assert!(markdown.contains("# Title"));
//! assert!(markdown.contains("**Bold text**"));
//! ```

use anyhow::Result;
use roxmltree::Document;

// Module declarations
mod elements;
mod emoji;
mod html_entities;
mod macros;
mod tables;
mod utils;

// Public API - re-export main conversion function
pub use elements::convert_node_to_markdown;

/// Convert Confluence storage format to Markdown.
///
/// This implementation uses proper HTML parsing to handle Confluence's
/// complex XML/HTML structure.
///
/// # Arguments
///
/// * `storage_content` - The Confluence storage format content (XHTML)
/// * `verbose` - Verbosity level for debug output (0 = silent, 1+ = increasing
///   verbosity)
///
/// # Returns
///
/// The converted Markdown content, or an error if parsing fails.
///
/// # Examples
///
/// ```
/// # use confluence_dl::markdown::storage_to_markdown;
/// let input = "<p>Hello <strong>world</strong>!</p>";
/// let output = storage_to_markdown(input, 0).unwrap();
/// assert_eq!(output.trim(), "Hello **world**!");
/// ```
pub fn storage_to_markdown(storage_content: &str, verbose: u8) -> Result<String> {
  // Pre-process: Replace HTML entities with numeric character references
  // roxmltree only supports XML's 5 predefined entities, not HTML entities
  let preprocessed = html_entities::preprocess_html_entities(storage_content);

  // Wrap with synthetic namespace declarations for Confluence namespaces
  let wrapped = utils::wrap_with_namespaces(&preprocessed);

  if verbose >= 4 {
    eprintln!(
      "[DEBUG] Wrapped XML (first 500 chars):\n{}",
      &wrapped.chars().take(500).collect::<String>()
    );
  }

  // Parse the HTML/XML content
  let document = Document::parse(&wrapped).map_err(|e| {
    if verbose >= 1 {
      eprintln!("[ERROR] XML parse error: {e}");
      eprintln!("[ERROR] Wrapped XML length: {} chars", wrapped.len());
      if verbose >= 3 {
        eprintln!("[ERROR] Full wrapped XML:\n{wrapped}");
      }
    }
    anyhow::anyhow!("Failed to parse Confluence storage content: {e}")
  })?;

  // Convert to markdown
  let markdown = convert_node_to_markdown(document.root_element(), verbose);

  // Clean up the result
  let cleaned = utils::clean_markdown(&markdown);

  Ok(cleaned)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_convert_headings() {
    let input = "<h1>Title</h1><h2>Subtitle</h2>";
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("# Title"));
    assert!(output.contains("## Subtitle"));
  }

  #[test]
  fn test_convert_formatting() {
    let input = "<p><strong>bold</strong> <em>italic</em> <s>strike</s></p>";
    let output = storage_to_markdown(input, 0).unwrap();
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

    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("> **Note:** This is a note block."));
  }

  #[test]
  fn test_convert_links() {
    let input = r#"<a href="https://example.com">Example</a>"#;
    let output = storage_to_markdown(input, 0).unwrap();
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
    let output = storage_to_markdown(input, 0).unwrap();
    insta::assert_snapshot!(output, @r###"
    - [ ] Task 1
    - [x] Task 2
    "###);
  }

  #[test]
  fn test_convert_image() {
    let input = r#"<ac:image ac:alt="test image"><ri:url ri:value="https://example.com/image.png" /></ac:image>"#;
    let output = storage_to_markdown(input, 0).unwrap();
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
    let output = storage_to_markdown(input, 0).unwrap();
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
    let output = storage_to_markdown(input, 0).unwrap();
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
    let result = storage_to_markdown(input, 0).unwrap();
    let output = result.escape_default();
    insta::assert_snapshot!(output, @r"- Item 1\n- Item 2\n\n      \n1. First\n2. Second\n");
  }

  #[test]
  fn test_convert_code_block() {
    let input = "<pre>function test() {\n  return 42;\n}</pre>";
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("```"));
    assert!(output.contains("function test()"));
  }

  #[test]
  fn test_convert_inline_code() {
    let input = "<p>Use <code>git commit</code> to save</p>";
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("`git commit`"));
  }

  #[test]
  fn test_convert_horizontal_rule() {
    let input = "<p>Before</p><hr /><p>After</p>";
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("---"));
  }

  #[test]
  fn test_convert_line_break() {
    let input = "<p>Line 1<br />Line 2</p>";
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("Line 1\nLine 2"));
  }
}
