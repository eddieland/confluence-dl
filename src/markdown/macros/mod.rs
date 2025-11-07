//! Confluence macro conversion to Markdown.
//!
//! Handles structured macros like panels, notes, status badges, and more.

use roxmltree::Node;
use tracing::debug;

use crate::markdown::MarkdownOptions;
use crate::markdown::utils::{find_child_by_tag, get_attribute, get_element_text};

mod admonitions;
mod anchors;
mod basic;
mod code;
mod decisions;
mod emoji_macros;
mod excerpts;
mod expand;
mod jira;

pub(crate) use admonitions::render_admonition_block;
pub use decisions::convert_adf_extension_to_markdown;

/// Signature used by all macro handlers.
type MacroHandler = fn(&str, Node, &dyn Fn(Node) -> String, &MarkdownOptions) -> Option<String>;

struct Handler {
  names: &'static [&'static str],
  func: MacroHandler,
}

const HANDLERS: &[Handler] = &[
  Handler {
    names: &["toc", "panel", "status"],
    func: basic::handle_macro,
  },
  Handler {
    names: &["note", "info", "warning", "tip"],
    func: admonitions::handle_macro,
  },
  Handler {
    names: &["excerpt"],
    func: excerpts::handle_macro,
  },
  Handler {
    names: &["code", "code-block"],
    func: code::handle_macro,
  },
  Handler {
    names: &["expand"],
    func: expand::handle_macro,
  },
  Handler {
    names: &["emoji"],
    func: emoji_macros::handle_macro,
  },
  Handler {
    names: &["anchor"],
    func: anchors::handle_macro,
  },
  Handler {
    names: &["decisionreport", "decision", "decision-list"],
    func: decisions::handle_macro,
  },
  Handler {
    names: &["jira"],
    func: jira::handle_macro,
  },
];

/// Converts Confluence structured macros to Markdown.
///
/// Unknown macros fall back to returning their text content.
///
/// # Arguments
/// * `element` - The `<ac:structured-macro>` node being processed.
/// * `convert_node` - Callback used to render nested content into Markdown.
/// * `options` - Conversion behaviour flags that influence macro rendering.
///
/// # Returns
/// A Markdown fragment representing the macro, or the macro's text content when
/// unhandled.
pub fn convert_macro_to_markdown(
  element: Node,
  convert_node: &dyn Fn(Node) -> String,
  options: &MarkdownOptions,
) -> String {
  let macro_name = get_attribute(element, "ac:name").unwrap_or_default();

  for handler in HANDLERS {
    if handler.names.iter().any(|name| *name == macro_name)
      && let Some(result) = (handler.func)(&macro_name, element, convert_node, options)
    {
      return result;
    }
  }

  // For unknown macros, just extract the text content
  get_element_text(element)
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
    .filter(|child| crate::markdown::utils::matches_tag(*child, "ac:task"))
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
  use crate::markdown::MarkdownOptions;
  use crate::markdown::utils::{get_attribute, matches_tag, wrap_with_namespaces};

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
    let output = convert_macro_to_markdown(macro_node, &simple_convert_node, &MarkdownOptions::default());
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
    let output = convert_macro_to_markdown(macro_node, &simple_convert_node, &MarkdownOptions::default());
    assert!(output.contains("**Table of Contents**"));
  }

  #[test]
  fn test_anchor_macro_ignored_by_default() {
    let input = r#"
      <ac:structured-macro ac:name="anchor">
        <ac:parameter ac:name="anchor">section-1</ac:parameter>
      </ac:structured-macro>
    "#;

    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let macro_node = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:structured-macro"))
      .unwrap();
    let output = convert_macro_to_markdown(macro_node, &simple_convert_node, &MarkdownOptions::default());
    assert!(output.is_empty());
  }

  #[test]
  fn test_anchor_macro_preserved_when_requested() {
    let input = r#"
      <ac:structured-macro ac:name="anchor">
        <ac:parameter ac:name="anchor">section-1</ac:parameter>
      </ac:structured-macro>
    "#;

    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let macro_node = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:structured-macro"))
      .unwrap();
    let options = MarkdownOptions {
      preserve_anchors: true,
      ..Default::default()
    };
    let output = convert_macro_to_markdown(macro_node, &simple_convert_node, &options);
    assert_eq!(output, "<a id=\"section-1\"></a>");
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
    let output = convert_macro_to_markdown(macro_node, &simple_convert_node, &MarkdownOptions::default());

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
    let output = convert_macro_to_markdown(macro_node, &simple_convert_node, &MarkdownOptions::default());

    let expected = "\n```\nline 1\nline 2\n```\n\n";
    assert_eq!(output, expected);
  }

  #[test]
  fn test_convert_jira_macro_single_issue() {
    let input = r#"
      <ac:structured-macro ac:name="jira">
        <ac:parameter ac:name="key">ABC-123</ac:parameter>
        <ac:parameter ac:name="server">https://jira.example.com</ac:parameter>
        <ac:parameter ac:name="summary">Fix the login flow</ac:parameter>
      </ac:structured-macro>
    "#;

    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let macro_node = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:structured-macro"))
      .unwrap();
    let output = convert_macro_to_markdown(macro_node, &simple_convert_node, &MarkdownOptions::default());

    assert_eq!(
      output,
      "[ABC-123](https://jira.example.com/browse/ABC-123): Fix the login flow"
    );
  }

  #[test]
  fn test_convert_jira_macro_jql_message() {
    let input = r#"
      <ac:structured-macro ac:name="jira">
        <ac:plain-text-body><![CDATA[project = ABC ORDER BY created DESC]]></ac:plain-text-body>
      </ac:structured-macro>
    "#;

    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let macro_node = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:structured-macro"))
      .unwrap();
    let output = convert_macro_to_markdown(macro_node, &simple_convert_node, &MarkdownOptions::default());

    assert_eq!(
      output,
      "\n> _Jira issues macro (JQL: project = ABC ORDER BY created DESC). Dynamic content not exported._\n\n"
    );
  }

  #[test]
  fn test_convert_excerpt_macro_with_panel() {
    let input = r#"
      <ac:structured-macro ac:name="excerpt">
        <ac:rich-text-body>
          <p>This is an excerpt.</p>
        </ac:rich-text-body>
      </ac:structured-macro>
    "#;

    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let macro_node = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:structured-macro"))
      .unwrap();
    let output = convert_macro_to_markdown(macro_node, &simple_convert_node, &MarkdownOptions::default());

    assert_eq!(output, "\n> **Excerpt:** This is an excerpt.\n\n");
  }

  #[test]
  fn test_convert_excerpt_macro_without_panel() {
    let input = r#"
      <ac:structured-macro ac:name="excerpt">
        <ac:parameter ac:name="nopanel">true</ac:parameter>
        <ac:rich-text-body>
          <p>This is inline.</p>
        </ac:rich-text-body>
      </ac:structured-macro>
    "#;

    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let macro_node = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:structured-macro"))
      .unwrap();
    let output = convert_macro_to_markdown(macro_node, &simple_convert_node, &MarkdownOptions::default());

    assert_eq!(output, "This is inline.\n\n");
  }

  #[test]
  fn test_convert_hidden_excerpt_macro() {
    let input = r#"
      <ac:structured-macro ac:name="excerpt">
        <ac:parameter ac:name="hidden">true</ac:parameter>
        <ac:rich-text-body>
          <p>You should not see me.</p>
        </ac:rich-text-body>
      </ac:structured-macro>
    "#;

    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let macro_node = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:structured-macro"))
      .unwrap();
    let output = convert_macro_to_markdown(macro_node, &simple_convert_node, &MarkdownOptions::default());

    assert_eq!(output, "");
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

  #[test]
  fn test_convert_adf_extension_ignores_fallback_when_decisions_rendered() {
    let input = concat!(
      "<ac:adf-extension>",
      "<ac:adf-node type=\"paragraph\"><ac:adf-content>Intro text.</ac:adf-content></ac:adf-node>",
      "<ac:adf-node type=\"decision-list\">",
      "<ac:adf-node type=\"decision-item\">",
      "<ac:adf-attribute key=\"title\">Decision Title</ac:adf-attribute>",
      "</ac:adf-node>",
      "</ac:adf-node>",
      "<ac:adf-node type=\"paragraph\"><ac:adf-content>Outro text.</ac:adf-content></ac:adf-node>",
      "<ac:adf-fallback>Fallback markup.</ac:adf-fallback>",
      "</ac:adf-extension>"
    );
    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let extension = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:adf-extension"))
      .unwrap();
    let output = convert_adf_extension_to_markdown(extension, &simple_convert_node);
    assert_eq!(output, "Intro text.\n- **Decision:** Decision Title\n\nOutro text.");
  }

  #[test]
  fn test_convert_adf_extension_returns_fallback_when_no_supported_nodes() {
    let input = concat!(
      "<ac:adf-extension>",
      "<ac:adf-fallback>Fallback only.</ac:adf-fallback>",
      "</ac:adf-extension>"
    );
    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let extension = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:adf-extension"))
      .unwrap();
    let output = convert_adf_extension_to_markdown(extension, &simple_convert_node);
    assert_eq!(output, "Fallback only.");
  }

  #[test]
  fn test_convert_adf_panel_renders_note() {
    let input = concat!(
      "<ac:adf-extension>",
      "<ac:adf-node type=\"panel\">",
      "<ac:adf-attribute key=\"panel-type\">note</ac:adf-attribute>",
      "<ac:adf-content><p>This is Note.</p><p>Next line.</p></ac:adf-content>",
      "</ac:adf-node>",
      "<ac:adf-fallback>Fallback panel markup.</ac:adf-fallback>",
      "</ac:adf-extension>"
    );
    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let extension = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:adf-extension"))
      .unwrap();
    let output = convert_adf_extension_to_markdown(extension, &simple_convert_node);
    assert!(output.contains("> **Note:** This is Note.Next line."));
    assert!(!output.contains("Fallback panel markup"));
  }

  #[test]
  fn test_convert_adf_panel_uses_custom_title() {
    let input = concat!(
      "<ac:adf-extension>",
      "<ac:adf-node type=\"panel\">",
      "<ac:adf-attribute key=\"panel-type\">custom</ac:adf-attribute>",
      "<ac:adf-attribute key=\"panel-title\">Important</ac:adf-attribute>",
      "<ac:adf-content><p>Body copy.</p></ac:adf-content>",
      "</ac:adf-node>",
      "</ac:adf-extension>"
    );
    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let extension = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:adf-extension"))
      .unwrap();
    let output = convert_adf_extension_to_markdown(extension, &simple_convert_node);
    assert!(output.contains("> **Important:** Body copy."));
  }

  #[test]
  fn test_convert_decision_macro() {
    let input = r#"
      <ac:structured-macro ac:name="decision">
        <ac:parameter ac:name="title">Adopt Rust</ac:parameter>
        <ac:parameter ac:name="status">decided</ac:parameter>
        <ac:parameter ac:name="owner"><ri:user ri:account-id="12345" /></ac:parameter>
        <ac:parameter ac:name="date">2024-01-10</ac:parameter>
        <ac:parameter ac:name="due-date">2024-02-01</ac:parameter>
        <ac:parameter ac:name="outcome">Approved</ac:parameter>
        <ac:rich-text-body>
          <p>We will build the CLI tooling in Rust.</p>
        </ac:rich-text-body>
      </ac:structured-macro>
    "#;

    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let macro_node = document
      .descendants()
      .find(|node| {
        matches_tag(*node, "ac:structured-macro") && get_attribute(*node, "ac:name").as_deref() == Some("decision")
      })
      .unwrap();
    let output = convert_macro_to_markdown(macro_node, &simple_convert_node, &MarkdownOptions::default());

    assert!(output.contains("**Decision:** Adopt Rust"));
    assert!(output.contains("Status: decided"));
    assert!(output.contains("Owner: @user:12345"));
    assert!(output.contains("Date: 2024-01-10"));
    assert!(output.contains("Due date: 2024-02-01"));
    assert!(output.contains("Outcome: Approved"));
    assert!(output.contains("We will build the CLI tooling in Rust."));
  }

  #[test]
  fn test_convert_decision_list_macro() {
    let input = r#"
      <ac:structured-macro ac:name="decision-list">
        <ac:rich-text-body>
          <ul>
            <li>
              <ac:structured-macro ac:name="decision">
                <ac:parameter ac:name="title">Adopt Rust</ac:parameter>
                <ac:parameter ac:name="status">decided</ac:parameter>
                <ac:parameter ac:name="owner"><ri:user ri:account-id="12345" /></ac:parameter>
                <ac:rich-text-body>
                  <p>Use Rust for performance critical tooling.</p>
                </ac:rich-text-body>
              </ac:structured-macro>
            </li>
            <li>
              <ac:structured-macro ac:name="decision">
                <ac:parameter ac:name="title">Keep Python scripts</ac:parameter>
                <ac:parameter ac:name="status">in-progress</ac:parameter>
                <ac:parameter ac:name="owner">alice</ac:parameter>
                <ac:rich-text-body>
                  <p>Maintain Python automation where migration is not feasible.</p>
                </ac:rich-text-body>
              </ac:structured-macro>
            </li>
          </ul>
        </ac:rich-text-body>
      </ac:structured-macro>
    "#;

    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let macro_node = document
      .descendants()
      .find(|node| {
        matches_tag(*node, "ac:structured-macro") && get_attribute(*node, "ac:name").as_deref() == Some("decision-list")
      })
      .unwrap();
    let output = convert_macro_to_markdown(macro_node, &simple_convert_node, &MarkdownOptions::default());

    assert!(output.contains("- **Decision:** Adopt Rust"));
    assert!(output.contains("- **Decision:** Keep Python scripts"));
    assert!(output.contains("Owner: @user:12345"));
    assert!(output.contains("Owner: alice"));
    assert!(output.contains("Use Rust for performance critical tooling."));
    assert!(output.contains("Maintain Python automation where migration is not feasible."));
  }

  #[test]
  fn test_convert_decision_report_macro() {
    let input = r#"
      <ac:structured-macro ac:name="decisionreport">
        <ac:parameter ac:name="cql">space = "DOCS" and label = "meeting-notes"</ac:parameter>
      </ac:structured-macro>
    "#;

    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let macro_node = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:structured-macro"))
      .unwrap();
    let output = convert_macro_to_markdown(macro_node, &simple_convert_node, &MarkdownOptions::default());

    assert!(output.contains("Decision report macro"));
    assert!(output.contains("space = \"DOCS\" and label = \"meeting-notes\""));
  }
}
