use roxmltree::Node;
use tracing::debug;

use crate::markdown::MarkdownOptions;
use crate::markdown::utils::{find_child_by_tag, find_child_by_tag_and_attr, get_element_text};

/// Renders Confluence code macros into fenced Markdown code blocks.
///
/// # Arguments
/// * `_macro_name` - Present for signature compatibility; only `"code"` variants reach here.
/// * `element` - The `<ac:structured-macro>` node that contains code parameters and body.
/// * `_convert_node` - Ignored callback because code bodies are plain text.
/// * `_options` - Markdown conversion options (not currently used for code blocks).
///
/// # Returns
/// Markdown fenced code block using the detected language when provided.
pub(super) fn handle_macro(
  _macro_name: &str,
  element: Node,
  _convert_node: &dyn Fn(Node) -> String,
  _options: &MarkdownOptions,
) -> Option<String> {
  Some(format_code_block(element))
}

/// Builds a fenced code block from a Confluence code macro element.
///
/// # Arguments
/// * `element` - The `<ac:structured-macro>` node containing `language` parameters and body text.
///
/// # Returns
/// A fenced code block surrounded by blank lines, including the language hint
/// when available.
fn format_code_block(element: Node) -> String {
  let language = find_child_by_tag_and_attr(element, "ac:parameter", "ac:name", "language")
    .map(get_element_text)
    .unwrap_or_default();

  if !language.trim().is_empty() {
    debug!("Code block language: {}", language.trim());
  }

  let body = find_child_by_tag(element, "ac:plain-text-body")
    .map(get_element_text)
    .or_else(|| find_child_by_tag(element, "ac:rich-text-body").map(get_element_text))
    .unwrap_or_else(|| get_element_text(element));

  let mut result = String::new();
  result.push('\n');
  result.push_str("```");
  let trimmed_language = language.trim();
  if !trimmed_language.is_empty() {
    result.push_str(trimmed_language);
  }
  result.push('\n');

  let trimmed_body = body.trim_matches(|c| matches!(c, '\n' | '\r'));
  result.push_str(trimmed_body);
  if !trimmed_body.ends_with('\n') && !trimmed_body.is_empty() {
    result.push('\n');
  }

  result.push_str("```\n\n");
  result
}
