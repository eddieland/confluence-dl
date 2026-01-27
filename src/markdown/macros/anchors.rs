use roxmltree::Node;

use crate::markdown::MarkdownOptions;
use crate::markdown::utils::{find_child_by_tag_and_attr, get_element_text};

/// Converts Confluence anchor macros into optional HTML anchor tags.
///
/// # Arguments
/// * `_macro_name` - Present for signature compatibility; anchors do not branch on name.
/// * `element` - The `<ac:structured-macro>` node that may contain an anchor parameter.
/// * `_convert_node` - Unused callback retained to match handler signature.
/// * `options` - Markdown conversion options indicating whether to keep anchors.
///
/// # Returns
/// Empty string when anchors are suppressed, otherwise an HTML `<a id=\"...\">`
/// tag.
pub(super) fn handle_macro(
  _macro_name: &str,
  element: Node,
  _convert_node: &dyn Fn(Node) -> String,
  options: &MarkdownOptions,
) -> Option<String> {
  if !options.preserve_anchors {
    return Some(String::new());
  }

  let anchor_id = find_child_by_tag_and_attr(element, "ac:parameter", "ac:name", "anchor")
    .map(get_element_text)
    .map(|value| value.trim().to_string())
    .unwrap_or_default();

  Some(if anchor_id.is_empty() {
    String::new()
  } else {
    format!("<a id=\"{anchor_id}\"></a>")
  })
}
