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

  let heading = resolve_heading(macro_name, title.trim());
  Some(render_admonition_block(&heading, body.trim()))
}

fn resolve_heading(macro_name: &str, explicit_title: &str) -> String {
  let default_title = match macro_name {
    "info" => "Info",
    "warning" => "Warning",
    "tip" => "Tip",
    _ => "Note",
  };

  if explicit_title.is_empty() {
    default_title.to_string()
  } else {
    explicit_title.to_string()
  }
}

/// Formats the Markdown blockquote for an admonition macro.
///
/// # Arguments
/// * `heading` - Title to display for the admonition.
/// * `body` - Markdown body contents, expected to be trimmed and possibly
///   multiline.
///
/// # Returns
/// Markdown blockquote containing the heading and body lines.
pub(crate) fn render_admonition_block(heading: &str, body: &str) -> String {
  let body = body.trim();

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
