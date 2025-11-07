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
//! use confluence_dl::markdown::{MarkdownOptions, storage_to_markdown_with_options};
//!
//! let confluence_html = r#"<h1>Title</h1><p><strong>Bold text</strong></p>"#;
//! let markdown =
//!   storage_to_markdown_with_options(confluence_html, &MarkdownOptions::default()).unwrap();
//! assert!(markdown.contains("# Title"));
//! assert!(markdown.contains("**Bold text**"));
//! ```

use std::time::Instant;

use anyhow::Result;
use roxmltree::Document;
use tracing::{debug, error, trace};

// Module declarations
mod elements;
mod emoji;
mod html_entities;
mod macros;
mod tables;
mod utils;

// Public API - re-export main conversion function
pub use elements::convert_node_to_markdown;

/// Options that control Markdown conversion behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MarkdownOptions {
  /// Preserve Confluence anchor macros as HTML anchors in the output.
  pub preserve_anchors: bool,
  /// Render Markdown tables without padding cells to align columns.
  pub compact_tables: bool,
}

/// Convert Confluence storage format to Markdown using the provided options.
///
/// # Arguments
///
/// * `storage_content` - The Confluence storage format content (XHTML) to
///   convert.
/// * `options` - Conversion behaviour flags that control optional features,
///   such as anchor preservation.
///
/// # Returns
///
/// `Result<String>` containing the converted Markdown content, or an error if
/// parsing fails.
///
/// # Examples
///
/// ```
/// # use confluence_dl::markdown::{storage_to_markdown_with_options, MarkdownOptions};
/// let input = "<p>Hello <strong>world</strong>!</p>";
/// let output = storage_to_markdown_with_options(input, &MarkdownOptions::default()).unwrap();
/// assert_eq!(output.trim(), "Hello **world**!");
/// ```
pub fn storage_to_markdown_with_options(storage_content: &str, options: &MarkdownOptions) -> Result<String> {
  // Pre-process: Replace HTML entities with numeric character references
  // roxmltree only supports XML's 5 predefined entities, not HTML entities
  let preprocessed = html_entities::preprocess_html_entities(storage_content);

  // Wrap with synthetic namespace declarations for Confluence namespaces
  let wrapped = utils::wrap_with_namespaces(&preprocessed);

  trace!(
    "Wrapped XML (first 500 chars):\n{}",
    wrapped.chars().take(500).collect::<String>()
  );

  // Parse the HTML/XML content
  let parse_start = Instant::now();
  let document = Document::parse(&wrapped).map_err(|e| {
    error!("XML parse error: {e}");
    error!("Wrapped XML length: {} chars", wrapped.len());
    trace!("Full wrapped XML:\n{wrapped}");
    anyhow::anyhow!("Failed to parse Confluence storage content: {e}")
  })?;

  debug!(
    "Parsed Confluence storage document in {duration:?} (length: {length} chars)",
    duration = parse_start.elapsed(),
    length = wrapped.len()
  );

  // Convert to markdown
  let markdown = convert_node_to_markdown(document.root_element(), options);

  // Clean up the result
  let cleaned = utils::clean_markdown(&markdown);

  Ok(cleaned)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn render(input: &str) -> String {
    storage_to_markdown_with_options(input, &MarkdownOptions::default()).unwrap()
  }

  #[test]
  fn test_convert_headings() {
    let input = "<h1>Title</h1><h2>Subtitle</h2>";
    let output = render(input);
    assert!(output.contains("# Title"));
    assert!(output.contains("## Subtitle"));
  }

  #[test]
  fn test_convert_formatting() {
    let input = "<p><strong>bold</strong> <em>italic</em> <s>strike</s></p>";
    let output = render(input);
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

    let output = render(input);
    assert!(output.contains("> **Note:** This is a note block."));
  }

  #[test]
  fn test_convert_excerpt_macro() {
    let input = r#"
      <ac:structured-macro ac:name="excerpt">
        <ac:rich-text-body>
          <p>This is an excerpt.</p>
        </ac:rich-text-body>
      </ac:structured-macro>
    "#;

    let output = render(input);
    assert!(output.contains("> **Excerpt:** This is an excerpt."));
  }

  #[test]
  fn test_convert_excerpt_macro_without_panel() {
    let input = r#"
      <ac:structured-macro ac:name="excerpt">
        <ac:parameter ac:name="nopanel">true</ac:parameter>
        <ac:rich-text-body>
          <p>This is inline.</p>
        </ac:rich-text-body>
      </ac:structured-macro>
    "#;

    let output = render(input);
    assert!(output.contains("This is inline."));
    assert!(!output.contains("**Excerpt:**"));
  }

  #[test]
  fn test_convert_hidden_excerpt_macro() {
    let input = r#"
      <ac:structured-macro ac:name="excerpt">
        <ac:parameter ac:name="hidden">true</ac:parameter>
        <ac:rich-text-body>
          <p>Hidden excerpt content.</p>
        </ac:rich-text-body>
      </ac:structured-macro>
    "#;

    let output = render(input);
    assert!(!output.contains("Hidden excerpt content."));
    assert!(!output.contains("**Excerpt:**"));
  }

  #[test]
  fn test_convert_legacy_note_block() {
    let input = r#"
      <ac:note>
        <ac:rich-text-body>
          <p>This is a legacy note.</p>
        </ac:rich-text-body>
      </ac:note>
    "#;

    let output = render(input);
    assert!(output.contains("> **Note:** This is a legacy note."));
  }

  #[test]
  fn test_convert_links() {
    let input = r#"<a href="https://example.com">Example</a>"#;
    let output = render(input);
    assert!(output.contains("[Example](https://example.com)"));
  }

  #[test]
  fn test_anchor_macro_not_preserved_by_default() {
    let input = r#"
      <ac:structured-macro ac:name="anchor">
        <ac:parameter ac:name="anchor">my-anchor</ac:parameter>
      </ac:structured-macro>
    "#;

    let output = render(input);
    assert!(!output.contains("<a id=\"my-anchor\"></a>"));
  }

  #[test]
  fn test_anchor_macro_preserved_when_requested() {
    let input = r#"
      <ac:structured-macro ac:name="anchor">
        <ac:parameter ac:name="anchor">my-anchor</ac:parameter>
      </ac:structured-macro>
    "#;

    let options = MarkdownOptions {
      preserve_anchors: true,
      ..Default::default()
    };
    let output = storage_to_markdown_with_options(input, &options).unwrap();
    assert!(output.contains("<a id=\"my-anchor\"></a>"));
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
    let output = render(input);
    insta::assert_snapshot!(output, @r###"
    - [ ] Task 1
    - [x] Task 2
    "###);
  }

  #[test]
  fn test_convert_image() {
    let input = r#"<ac:image ac:alt="test image"><ri:url ri:value="https://example.com/image.png" /></ac:image>"#;
    let output = render(input);
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
    let output = render(input);
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
    let output = render(input);
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
    let result = render(input);
    let output = result.escape_default();
    insta::assert_snapshot!(output, @r"- Item 1\n- Item 2\n\n      \n1. First\n2. Second\n");
  }

  #[test]
  fn test_convert_code_block() {
    let input = "<pre>function test() {\n  return 42;\n}</pre>";
    let output = render(input);
    assert!(output.contains("```"));
    assert!(output.contains("function test()"));
  }

  #[test]
  fn test_convert_inline_code() {
    let input = "<p>Use <code>git commit</code> to save</p>";
    let output = render(input);
    assert!(output.contains("`git commit`"));
  }

  #[test]
  fn test_convert_horizontal_rule() {
    let input = "<p>Before</p><hr /><p>After</p>";
    let output = render(input);
    assert!(output.contains("---"));
  }

  #[test]
  fn test_convert_line_break() {
    let input = "<p>Line 1<br />Line 2</p>";
    let output = render(input);
    assert!(output.contains("Line 1\nLine 2"));
  }
}
