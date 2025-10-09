use roxmltree::Node;

use crate::markdown::MarkdownOptions;
use crate::markdown::utils::{find_child_by_tag, find_child_by_tag_and_attr, get_element_text};

/// Converts Confluence admonition macros (note, info, warning, tip) into
/// Markdown blockquotes.
///
/// # Arguments
/// * `macro_name` - The macro name that determines the default heading label.
/// * `element` - The `<ac:structured-macro>` node describing the admonition.
/// * `convert_node` - Callback used to render the rich text body into Markdown.
/// * `_options` - Conversion flags (currently unused; kept for signature
///   parity).
///
/// # Returns
/// Markdown blockquote for the admonition with an emphasized heading.
pub(super) fn handle_macro(
  macro_name: &str,
  element: Node,
  convert_node: &dyn Fn(Node) -> String,
  _options: &MarkdownOptions,
) -> Option<String> {
  let title = find_child_by_tag_and_attr(element, "ac:parameter", "ac:name", "title")
    .map(get_element_text)
    .unwrap_or_default();

  let body = find_child_by_tag(element, "ac:rich-text-body")
    .map(convert_node)
    .unwrap_or_else(|| get_element_text(element));

  Some(format_admonition_block(macro_name, title.trim(), body.trim()))
}

/// Formats the Markdown blockquote for an admonition macro.
///
/// # Arguments
/// * `macro_name` - Macro name used to pick the default title when none is
///   provided.
/// * `title` - Explicit title supplied by Confluence, already trimmed.
/// * `body` - Markdown body contents, expected to be trimmed and possibly
///   multiline.
///
/// # Returns
/// Markdown blockquote containing the heading and body lines.
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
