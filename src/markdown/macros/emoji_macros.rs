use roxmltree::Node;
use tracing::debug;

use crate::markdown::MarkdownOptions;
use crate::markdown::emoji::emoji_id_to_unicode;
use crate::markdown::utils::{find_child_by_tag_and_attr, get_element_text};

/// Renders Confluence emoji macros into plain Unicode characters.
///
/// # Arguments
/// * `_macro_name` - Present for signature compatibility; not used.
/// * `element` - The `<ac:structured-macro>` node that contains emoji parameters.
/// * `_convert_node` - Unused callback since emoji macros have no inner content.
/// * `_options` - Markdown conversion options (unused for emoji rendering).
///
/// # Returns
/// Unicode emoji or shortname text when a matching mapping is found.
pub(super) fn handle_macro(
  _macro_name: &str,
  element: Node,
  _convert_node: &dyn Fn(Node) -> String,
  _options: &MarkdownOptions,
) -> Option<String> {
  let emoji_id = find_child_by_tag_and_attr(element, "ac:parameter", "ac:name", "emoji-id").map(get_element_text);

  let result = emoji_id
    .as_deref()
    .and_then(|id| emoji_id_to_unicode(id.trim()))
    .or_else(|| find_child_by_tag_and_attr(element, "ac:parameter", "ac:name", "emoji").map(get_element_text))
    .or_else(|| find_child_by_tag_and_attr(element, "ac:parameter", "ac:name", "shortname").map(get_element_text))
    .unwrap_or_default();

  if !result.is_empty() {
    debug!("Macro emoji: id={emoji_id:?} -> {result}");
  }

  Some(result)
}
