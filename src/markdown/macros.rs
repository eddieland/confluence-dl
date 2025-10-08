//! Confluence macro conversion to Markdown.
//!
//! Handles structured macros like panels, notes, status badges, and more.

use roxmltree::Node;
use tracing::debug;

use super::emoji::emoji_id_to_unicode;
use super::utils::{find_child_by_tag, find_child_by_tag_and_attr, get_attribute, get_element_text};

/// Converts Confluence structured macros to Markdown.
///
/// Handles various macro types including:
/// - `toc`: Table of contents
/// - `panel`: Blockquote panels
/// - `note`, `info`, `warning`, `tip`: Admonition blocks
/// - `status`: Status badges
/// - `expand`: Collapsible sections
/// - `emoji`: Emoji macros
/// - `anchor`: Link anchors (rendered as empty)
///
/// # Arguments
/// * `element` - The `<ac:structured-macro>` node to convert.
/// * `convert_node` - Callback used to recursively render nested nodes.
///
/// # Returns
/// A Markdown representation of the macro content. Unknown macros fall back to
/// returning their text content.
pub fn convert_macro_to_markdown(element: Node, convert_node: &dyn Fn(Node) -> String) -> String {
  let macro_name = get_attribute(element, "ac:name").unwrap_or_default();

  match macro_name.as_str() {
    "toc" => "\n**Table of Contents**\n\n".to_string(),
    "panel" => {
      // Extract rich text body if present
      let body = find_child_by_tag(element, "ac:rich-text-body")
        .map(convert_node)
        .unwrap_or_else(|| get_element_text(element));
      format!("\n> {}\n\n", body.trim())
    }
    "note" | "info" | "warning" | "tip" => {
      let title = find_child_by_tag_and_attr(element, "ac:parameter", "ac:name", "title")
        .map(get_element_text)
        .unwrap_or_default();

      let body = find_child_by_tag(element, "ac:rich-text-body")
        .map(convert_node)
        .unwrap_or_else(|| get_element_text(element));

      format_admonition_block(&macro_name, title.trim(), body.trim())
    }
    "code" | "code-block" => format_code_block(element),
    "status" => {
      let title = find_child_by_tag_and_attr(element, "ac:parameter", "ac:name", "title")
        .map(get_element_text)
        .unwrap_or_default();
      format!("`[{title}]`")
    }
    "expand" => {
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
    "emoji" => {
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
      result
    }
    "anchor" => String::new(), // Skip anchors
    _ => {
      // For unknown macros, just extract the text content
      get_element_text(element)
    }
  }
}

/// Formats Confluence admonition macros (note, info, warning, tip) as Markdown
/// blockquotes.
///
/// # Arguments
/// * `macro_name` - The macro identifier such as `note` or `warning`.
/// * `title` - Optional custom title displayed in the admonition heading.
/// * `body` - Rich text content inside the admonition block.
///
/// # Returns
/// A Markdown string containing a blockquote-style admonition.
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

/// Converts Confluence code macros to fenced code blocks.
///
/// # Arguments
/// * `element` - The structured macro node describing the code block.
///
/// # Returns
/// A Markdown string using triple backticks and optional language annotation.
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

/// Converts Confluence task list macros to Markdown checkboxes.
///
/// # Arguments
/// * `element` - The `<ac:task-list>` node to convert.
///
/// # Returns
/// Markdown representing each task as a checkbox list item.
pub fn convert_task_list_to_markdown(element: Node) -> String {
  let mut result = String::new();

  for task in element
    .children()
    .filter(|child| super::utils::matches_tag(*child, "ac:task"))
  {
    let status = find_child_by_tag(task, "ac:task-status")
      .map(get_element_text)
      .unwrap_or_else(|| "incomplete".to_string());

    let body = find_child_by_tag(task, "ac:task-body")
      .map(get_element_text)
      .unwrap_or_default();

    let checkbox = if status.trim() == "complete" { "[x]" } else { "[ ]" };
    result.push_str(&format!("- {} {}\n", checkbox, body.trim()));
  }

  result.push('\n');
  result
}

/// Converts Confluence image macros to Markdown image syntax.
///
/// # Arguments
/// * `element` - The `<ac:image>` node to convert.
///
/// # Returns
/// Markdown `![alt](source)` markup using either attachment filenames or URLs.
pub fn convert_image_to_markdown(element: Node) -> String {
  let alt = get_attribute(element, "ac:alt").unwrap_or_else(|| "image".to_string());

  if let Some(url) = find_child_by_tag(element, "ri:url").and_then(|e| get_attribute(e, "ri:value"))
    && !url.is_empty()
  {
    return format!("\n![{alt}]({url})\n\n");
  }

  if let Some(filename) = find_child_by_tag(element, "ri:attachment").and_then(|e| get_attribute(e, "ri:filename"))
    && !filename.is_empty()
  {
    return format!("\n![{alt}]({filename})\n\n");
  }

  format!("\n![{alt}]()\n\n")
}

/// Converts Confluence links to Markdown.
///
/// Handles user mentions (`<ac:link><ri:user .../></ac:link>`) and internal
/// page links.
///
/// # Arguments
/// * `element` - The `<ac:link>` node to convert.
///
/// # Returns
/// Markdown-formatted text representing the link target or mention.
pub fn convert_confluence_link_to_markdown(element: Node) -> String {
  // Check for user mention
  if let Some(user_node) = find_child_by_tag(element, "ri:user") {
    let account_id = get_attribute(user_node, "ri:account-id").unwrap_or_default();

    debug!("User mention: account_id={account_id}");

    // Format as @mention with account ID as fallback
    // In the future, we could look up display names via API
    return format!("@user:{account_id}");
  }

  // Check for page link
  if let Some(page_node) = find_child_by_tag(element, "ri:page") {
    let title = get_attribute(page_node, "ri:content-title").unwrap_or_default();

    debug!("Page link: title={title}");

    // Format as wiki-style link
    return format!("[[{title}]]");
  }

  // Check for attachment link
  if let Some(attachment_node) = find_child_by_tag(element, "ri:attachment") {
    let filename = get_attribute(attachment_node, "ri:filename").unwrap_or_default();

    if !filename.is_empty() {
      let link_text = find_child_by_tag(element, "ac:plain-text-link-body")
        .map(get_element_text)
        .filter(|text| !text.trim().is_empty())
        .unwrap_or_else(|| filename.clone());

      return format!("[{}]({filename})", link_text.trim());
    }
  }

  // Fall back to regular link handling if it has an href
  let text = get_element_text(element);
  if let Some(href) = get_attribute(element, "href") {
    return format!("[{text}]({href})");
  }

  // If no special handling matched, just return the text content
  text
}

#[cfg(test)]
mod tests {
  use roxmltree::Document;

  use super::*;
  use crate::markdown::utils::{matches_tag, wrap_with_namespaces};

  // Simple converter for tests that doesn't do recursion
  fn simple_convert_node(node: Node) -> String {
    get_element_text(node)
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

    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let macro_node = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:structured-macro"))
      .unwrap();
    let output = convert_macro_to_markdown(macro_node, &simple_convert_node);
    assert!(output.contains("> **Note:** This is a note block."));
  }

  #[test]
  fn test_convert_macro_toc() {
    let input = r#"<ac:structured-macro ac:name="toc"></ac:structured-macro>"#;
    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let macro_node = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:structured-macro"))
      .unwrap();
    let output = convert_macro_to_markdown(macro_node, &simple_convert_node);
    assert!(output.contains("**Table of Contents**"));
  }

  #[test]
  fn test_convert_code_macro_with_language() {
    let input = r#"
      <ac:structured-macro ac:name="code">
        <ac:parameter ac:name="language">rust</ac:parameter>
        <ac:plain-text-body><![CDATA[fn main() {
  println!("hi");
}
]]></ac:plain-text-body>
      </ac:structured-macro>
    "#;

    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let macro_node = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:structured-macro"))
      .unwrap();
    let output = convert_macro_to_markdown(macro_node, &simple_convert_node);

    let expected = "\n```rust\nfn main() {\n  println!(\"hi\");\n}\n```\n\n";
    assert_eq!(output, expected);
  }

  #[test]
  fn test_convert_code_macro_without_language() {
    let input = r#"
      <ac:structured-macro ac:name="code">
        <ac:plain-text-body><![CDATA[line 1
line 2]]></ac:plain-text-body>
      </ac:structured-macro>
    "#;

    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let macro_node = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:structured-macro"))
      .unwrap();
    let output = convert_macro_to_markdown(macro_node, &simple_convert_node);

    let expected = "\n```\nline 1\nline 2\n```\n\n";
    assert_eq!(output, expected);
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
    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let task_list = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:task-list"))
      .unwrap();
    let output = convert_task_list_to_markdown(task_list);
    insta::assert_snapshot!(output, @r###"
    - [ ] Task 1
    - [x] Task 2
    "###);
  }

  #[test]
  fn test_convert_image() {
    let input = r#"<ac:image ac:alt="test image"><ri:url ri:value="https://example.com/image.png" /></ac:image>"#;
    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let image = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:image"))
      .unwrap();
    let output = convert_image_to_markdown(image);
    assert!(output.contains("![test image](https://example.com/image.png)"));
  }

  #[test]
  fn test_convert_image_with_attachment() {
    let input = r#"<ac:image ac:alt="diagram"><ri:attachment ri:filename="diagram.png" /></ac:image>"#;
    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let image = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:image"))
      .unwrap();
    let output = convert_image_to_markdown(image);
    assert!(output.contains("![diagram](diagram.png)"));
  }

  #[test]
  fn test_convert_attachment_link_to_markdown() {
    let input = r#"
      <ac:link>
        <ri:attachment ri:filename="spec.pdf" />
        <ac:plain-text-link-body>Download spec</ac:plain-text-link-body>
      </ac:link>
    "#;
    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let link = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:link"))
      .unwrap();
    let output = convert_confluence_link_to_markdown(link);
    assert_eq!(output, "[Download spec](spec.pdf)");
  }
}
