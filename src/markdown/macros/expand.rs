use roxmltree::Node;

use crate::markdown::MarkdownOptions;
use crate::markdown::utils::{find_child_by_tag, find_child_by_tag_and_attr, get_element_text};

/// Converts Confluence expand macros into HTML `<details>` elements.
///
/// # Arguments
/// * `_macro_name` - Included for signature parity; expand macros share the
///   same handler.
/// * `element` - The `<ac:structured-macro>` node describing the expand block.
/// * `convert_node` - Callback used to render the expand body into Markdown.
/// * `_options` - Conversion options (unused for expand macros).
///
/// # Returns
/// HTML `<details>` block containing the summary title and converted body.
pub(super) fn handle_macro(
  _macro_name: &str,
  element: Node,
  convert_node: &dyn Fn(Node) -> String,
  _options: &MarkdownOptions,
) -> Option<String> {
  Some(render_expand(element, convert_node))
}

/// Renders an expand macro to HTML, preserving title and body content.
///
/// # Arguments
/// * `element` - Expand macro node providing optional `title` and rich-text
///   body.
/// * `convert_node` - Callback for producing Markdown from the body.
///
/// # Returns
/// HTML `<details>` section wrapping the converted body.
fn render_expand(element: Node, convert_node: &dyn Fn(Node) -> String) -> String {
  let title = find_child_by_tag_and_attr(element, "ac:parameter", "ac:name", "title")
    .map(get_element_text)
    .unwrap_or_else(|| "Details".to_string());

  let body = find_child_by_tag(element, "ac:rich-text-body")
    .map(convert_node)
    .unwrap_or_else(|| get_element_text(element));

  format!(
    "\n<details>\n<summary>{}</summary>\n\n{}\n</details>\n\n",
    title,
    body.trim()
  )
}
