use roxmltree::Node;

use crate::markdown::MarkdownOptions;
use crate::markdown::utils::{find_child_by_tag, find_child_by_tag_and_attr, get_element_text};

/// Handles basic Confluence macros such as table of contents, panels, and
/// status badges.
///
/// # Arguments
/// * `macro_name` - Name of the macro to dispatch (e.g., `"toc"`, `"panel"`).
/// * `element` - The `<ac:structured-macro>` node containing macro content.
/// * `convert_node` - Callback used for rendering nested rich text where needed.
/// * `_options` - Markdown rendering options (unused for these basic macros).
///
/// # Returns
/// Markdown string representing the macro when handled, otherwise `None`.
pub(super) fn handle_macro(
  macro_name: &str,
  element: Node,
  convert_node: &dyn Fn(Node) -> String,
  _options: &MarkdownOptions,
) -> Option<String> {
  match macro_name {
    "toc" => Some("\n**Table of Contents**\n\n".to_string()),
    "panel" => Some(render_panel(element, convert_node)),
    "status" => Some(render_status(element)),
    _ => None,
  }
}

/// Renders a Confluence panel macro into a Markdown blockquote-style section.
///
/// # Arguments
/// * `element` - The `<ac:structured-macro>` node representing the panel.
/// * `convert_node` - Callback used to turn the panel body into Markdown.
///
/// # Returns
/// Markdown fragment wrapped in `>` lines that preserves panel content.
fn render_panel(element: Node, convert_node: &dyn Fn(Node) -> String) -> String {
  let body = find_child_by_tag(element, "ac:rich-text-body")
    .map(convert_node)
    .unwrap_or_else(|| get_element_text(element));
  format!("\n> {}\n\n", body.trim())
}

/// Renders the Confluence status macro into inline code-style Markdown.
///
/// # Arguments
/// * `element` - The `<ac:structured-macro>` node that may include a `title` parameter.
///
/// # Returns
/// Inline code span such as `` `[In Progress]` `` representing the status
/// badge.
fn render_status(element: Node) -> String {
  let title = find_child_by_tag_and_attr(element, "ac:parameter", "ac:name", "title")
    .map(get_element_text)
    .unwrap_or_default();
  format!("`[{title}]`")
}
