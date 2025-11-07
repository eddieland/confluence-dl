use roxmltree::Node;

use crate::markdown::MarkdownOptions;
use crate::markdown::macros::render_admonition_block;
use crate::markdown::utils::{find_child_by_tag, find_child_by_tag_and_attr, get_element_text};

/// Converts the Confluence excerpt macro into Markdown.
///
/// When the macro is configured with `nopanel=true`, the excerpt body is
/// rendered inline without additional formatting. Otherwise, it is emitted as a
/// callout block so the exported Markdown conveys the same emphasis users see
/// in Confluence.
pub(super) fn handle_macro(
  _macro_name: &str,
  element: Node,
  convert_node: &dyn Fn(Node) -> String,
  _options: &MarkdownOptions,
) -> Option<String> {
  let body = find_child_by_tag(element, "ac:rich-text-body")
    .map(convert_node)
    .unwrap_or_else(|| get_element_text(element));

  let hidden = find_child_by_tag_and_attr(element, "ac:parameter", "ac:name", "hidden")
    .map(|param| {
      let value = get_element_text(param);
      value.trim().is_empty() || value.trim().eq_ignore_ascii_case("true")
    })
    .unwrap_or(false);

  if hidden {
    return Some(String::new());
  }

  let no_panel = find_child_by_tag_and_attr(element, "ac:parameter", "ac:name", "nopanel")
    .map(|param| get_element_text(param).trim().eq_ignore_ascii_case("true"))
    .unwrap_or(false);

  if no_panel {
    let trimmed = body.trim();
    if trimmed.is_empty() {
      Some(String::new())
    } else {
      Some(format!("{trimmed}\n\n"))
    }
  } else {
    Some(render_admonition_block("Excerpt", body.trim()))
  }
}
