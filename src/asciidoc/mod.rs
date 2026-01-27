//! AsciiDoc conversion utilities for Confluence content.
//!
//! This module provides functionality to convert Confluence storage format
//! (XHTML-like) to AsciiDoc using proper HTML parsing, targeting
//! Asciidoctor-compatible output.
//!
//! # Architecture
//!
//! The conversion reuses parsing utilities from the markdown module:
//! - [`crate::markdown::html_entities`] - HTML entity encoding/decoding
//! - [`crate::markdown::utils`] - XML namespace handling and text extraction
//!
//! AsciiDoc-specific conversion is handled by:
//! - [`elements`] - AsciiDoc element converters
//! - [`utils`] - AsciiDoc-specific cleanup utilities

use std::time::Instant;

use anyhow::Result;
use roxmltree::Document;
use tracing::{debug, error, trace};

mod elements;
mod utils;

pub use elements::convert_node_to_asciidoc;

/// Options that control AsciiDoc conversion behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AsciiDocOptions {
  /// Preserve Confluence anchor macros as AsciiDoc anchors in the output.
  pub preserve_anchors: bool,
  /// Render tables in compact form without column width specs.
  pub compact_tables: bool,
}

/// Convert Confluence storage format to AsciiDoc using the provided options.
///
/// # Arguments
///
/// * `storage_content` - The Confluence storage format content (XHTML) to convert.
/// * `options` - Conversion behaviour flags that control optional features.
///
/// # Returns
///
/// `Result<String>` containing the converted AsciiDoc content, or an error if
/// parsing fails.
///
/// # Examples
///
/// ```
/// # use confluence_dl::asciidoc::{storage_to_asciidoc_with_options, AsciiDocOptions};
/// let input = "<p>Hello <strong>world</strong>!</p>";
/// let output = storage_to_asciidoc_with_options(input, &AsciiDocOptions::default()).unwrap();
/// assert_eq!(output.trim(), "Hello *world*!");
/// ```
pub fn storage_to_asciidoc_with_options(storage_content: &str, options: &AsciiDocOptions) -> Result<String> {
  // Reuse preprocessing from markdown module
  let preprocessed = crate::markdown::html_entities::preprocess_html_entities(storage_content);
  let wrapped = crate::markdown::utils::wrap_with_namespaces(&preprocessed);

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

  // Convert to AsciiDoc
  let asciidoc = convert_node_to_asciidoc(document.root_element(), options);

  // Clean up the result
  let cleaned = utils::clean_asciidoc(&asciidoc);

  Ok(cleaned)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn render(input: &str) -> String {
    storage_to_asciidoc_with_options(input, &AsciiDocOptions::default()).unwrap()
  }

  #[test]
  fn test_convert_headings() {
    let input = "<h1>Title</h1><h2>Subtitle</h2>";
    let output = render(input);
    assert!(output.contains("= Title"));
    assert!(output.contains("== Subtitle"));
  }

  #[test]
  fn test_convert_formatting() {
    let input = "<p><strong>bold</strong> <em>italic</em> <s>strike</s></p>";
    let output = render(input);
    assert!(output.contains("*bold*"));
    assert!(output.contains("_italic_"));
    assert!(output.contains("[line-through]#strike#"));
  }

  #[test]
  fn test_convert_links() {
    let input = r#"<a href="https://example.com">Example</a>"#;
    let output = render(input);
    assert!(output.contains("https://example.com[Example]"));
  }

  #[test]
  fn test_convert_code_block() {
    let input = "<pre>function test() {\n  return 42;\n}</pre>";
    let output = render(input);
    assert!(output.contains("----"));
    assert!(output.contains("function test()"));
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
    let output = render(input);
    assert!(output.contains("* Item 1"));
    assert!(output.contains("* Item 2"));
    assert!(output.contains(". First"));
    assert!(output.contains(". Second"));
  }

  #[test]
  fn test_convert_horizontal_rule() {
    let input = "<p>Before</p><hr /><p>After</p>";
    let output = render(input);
    assert!(output.contains("'''"));
  }
}
