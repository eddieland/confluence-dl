//! Confluence macro conversion to Markdown.
//!
//! Handles structured macros like panels, notes, status badges, and more.

use std::collections::BTreeMap;

use roxmltree::{Node, NodeType};
use tracing::debug;

use super::emoji::emoji_id_to_unicode;
use super::utils::{find_child_by_tag, find_child_by_tag_and_attr, get_attribute, get_element_text, matches_tag};

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
    "decisionreport" => {
      let query = find_child_by_tag_and_attr(element, "ac:parameter", "ac:name", "cql")
        .map(get_element_text)
        .unwrap_or_default();
      if query.is_empty() {
        "\n> _Decision report macro (dynamic content not exported)._ \n\n".to_string()
      } else {
        format!(
          "\n> _Decision report macro (CQL: {}). Dynamic content not exported._\n\n",
          query.trim()
        )
      }
    }
    "decision" => format_decision(element, convert_node),
    "decision-list" => format_decision_list(element, convert_node),
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

#[derive(Default)]
struct DecisionInfo {
  title: String,
  status: Option<String>,
  owner: Option<String>,
  date: Option<String>,
  due_date: Option<String>,
  outcome: Option<String>,
  body: Option<String>,
}

fn append_segment(target: &mut String, segment: &str) {
  let trimmed = segment.trim();
  if trimmed.is_empty() {
    return;
  }
  if !target.is_empty() && !target.ends_with(' ') {
    target.push(' ');
  }
  target.push_str(trimmed);
}

fn append_inline_text(target: &mut String, segment: &str) {
  let trimmed = segment.trim();
  if trimmed.is_empty() {
    return;
  }
  if !target.is_empty() && !target.ends_with([' ', '\n']) {
    target.push(' ');
  }
  target.push_str(trimmed);
}

fn get_parameter_value(element: Node, name: &str, convert_node: &dyn Fn(Node) -> String) -> Option<String> {
  let parameter = find_child_by_tag_and_attr(element, "ac:parameter", "ac:name", name)?;
  let mut value = String::new();

  for child in parameter.children() {
    match child.node_type() {
      NodeType::Text => {
        if let Some(text) = child.text() {
          append_segment(&mut value, text);
        }
      }
      NodeType::Element => {
        let converted = convert_node(child);
        append_segment(&mut value, &converted);
      }
      _ => {}
    }
  }

  if value.trim().is_empty() {
    if let Some(user) = find_child_by_tag(parameter, "ri:user") {
      if let Some(account_id) = get_attribute(user, "ri:account-id") {
        value = format!("@user:{account_id}");
      } else if let Some(username) = get_attribute(user, "ri:username") {
        value = format!("@{username}");
      } else if let Some(display) = get_attribute(user, "ri:display-name") {
        value = display;
      }
    } else if let Some(page) = find_child_by_tag(parameter, "ri:page") {
      if let Some(title) = get_attribute(page, "ri:content-title") {
        value = format!("[[{title}]]");
      } else if let Some(target) = get_attribute(page, "ri:value") {
        value = target;
      }
    }
  }

  if value.trim().is_empty() {
    value = get_element_text(parameter);
  }

  let trimmed = value.trim();
  if trimmed.is_empty() {
    None
  } else {
    Some(trimmed.to_string())
  }
}

fn parse_decision(element: Node, convert_node: &dyn Fn(Node) -> String) -> DecisionInfo {
  DecisionInfo {
    title: get_parameter_value(element, "title", convert_node).unwrap_or_else(|| "Untitled decision".to_string()),
    status: get_parameter_value(element, "status", convert_node),
    owner: get_parameter_value(element, "owner", convert_node),
    date: get_parameter_value(element, "date", convert_node),
    due_date: get_parameter_value(element, "due-date", convert_node)
      .or_else(|| get_parameter_value(element, "dueDate", convert_node)),
    outcome: get_parameter_value(element, "outcome", convert_node),
    body: find_child_by_tag(element, "ac:rich-text-body")
      .map(convert_node)
      .map(|body| body.trim().to_string())
      .filter(|body| !body.is_empty()),
  }
}

fn format_decision_content(info: &DecisionInfo) -> String {
  let mut content = String::new();

  let title = info.title.trim();
  content.push_str("**Decision:** ");
  content.push_str(if title.is_empty() { "Untitled decision" } else { title });

  let mut metadata = Vec::new();
  if let Some(status) = info.status.as_deref().filter(|s| !s.trim().is_empty()) {
    metadata.push(format!("Status: {}", status.trim()));
  }
  if let Some(owner) = info.owner.as_deref().filter(|s| !s.trim().is_empty()) {
    metadata.push(format!("Owner: {}", owner.trim()));
  }
  if let Some(date) = info.date.as_deref().filter(|s| !s.trim().is_empty()) {
    metadata.push(format!("Date: {}", date.trim()));
  }
  if let Some(due) = info.due_date.as_deref().filter(|s| !s.trim().is_empty()) {
    metadata.push(format!("Due date: {}", due.trim()));
  }
  if let Some(outcome) = info.outcome.as_deref().filter(|s| !s.trim().is_empty()) {
    metadata.push(format!("Outcome: {}", outcome.trim()));
  }

  if !metadata.is_empty() {
    content.push(' ');
    content.push('(');
    content.push_str(&metadata.join("; "));
    content.push(')');
  }

  if let Some(body) = info.body.as_deref().filter(|s| !s.trim().is_empty()) {
    content.push_str("\n\n");
    content.push_str(body.trim());
  }

  content
}

fn render_list_item(content: &str) -> Option<String> {
  let mut lines = content.lines();
  let first_line = lines.next()?.trim();
  if first_line.is_empty() {
    return None;
  }

  let mut result = String::new();
  result.push_str("- ");
  result.push_str(first_line);
  result.push('\n');

  for line in lines {
    if line.trim().is_empty() {
      result.push('\n');
    } else {
      result.push_str("  ");
      result.push_str(line.trim_end());
      result.push('\n');
    }
  }

  if !result.ends_with('\n') {
    result.push('\n');
  }

  Some(result)
}

fn decision_info_has_content(info: &DecisionInfo) -> bool {
  let has_title = !info.title.trim().is_empty();
  let has_body = info.body.as_ref().map(|body| !body.trim().is_empty()).unwrap_or(false);
  has_title || has_body
}

fn render_decision_infos(decisions: Vec<DecisionInfo>, skip_empty: bool) -> String {
  let mut result = String::new();
  let mut wrote_any = false;

  for info in decisions {
    if skip_empty && !decision_info_has_content(&info) {
      continue;
    }

    let content = format_decision_content(&info);
    if let Some(item) = render_list_item(&content) {
      if !wrote_any {
        result.push('\n');
        wrote_any = true;
      }
      result.push_str(&item);
    }
  }

  if wrote_any {
    if !result.ends_with('\n') {
      result.push('\n');
    }
    result.push('\n');
    result
  } else {
    String::new()
  }
}

fn format_decision(element: Node, convert_node: &dyn Fn(Node) -> String) -> String {
  let info = parse_decision(element, convert_node);
  let content = format_decision_content(&info);

  if content.trim().is_empty() {
    return String::new();
  }

  let mut result = String::new();
  result.push('\n');
  result.push_str(content.trim_end());
  result.push('\n');
  result.push('\n');
  result
}

fn format_decision_list(element: Node, convert_node: &dyn Fn(Node) -> String) -> String {
  let body = match find_child_by_tag(element, "ac:rich-text-body") {
    Some(body) => body,
    None => {
      let fallback = get_element_text(element);
      return if fallback.trim().is_empty() {
        String::new()
      } else {
        format!("\n{}\n\n", fallback.trim())
      };
    }
  };

  let decisions: Vec<_> = body
    .descendants()
    .filter(|node| matches_tag(*node, "ac:structured-macro"))
    .filter(|node| get_attribute(*node, "ac:name").as_deref() == Some("decision"))
    .collect();

  if decisions.is_empty() {
    let converted = convert_node(body);
    return if converted.trim().is_empty() {
      String::new()
    } else {
      format!("\n{}\n\n", converted.trim())
    };
  }

  let infos = decisions
    .into_iter()
    .map(|decision| parse_decision(decision, convert_node))
    .collect();
  render_decision_infos(infos, false)
}

/// Converts Atlassian Document Format (ADF) extensions into Markdown output.
///
/// Currently this understands decision lists coming from the newer editor
/// blocks.
pub fn convert_adf_extension_to_markdown(element: Node, convert_node: &dyn Fn(Node) -> String) -> String {
  let mut result = String::new();
  let mut decision_rendered = false;
  let mut fallback_buffer = String::new();

  for child in element.children().filter(|child| child.is_element()) {
    if matches_tag(child, "ac:adf-node") {
      match get_attribute(child, "type").as_deref() {
        Some("decision-list") => {
          let rendered = convert_adf_decision_list(child);
          if !rendered.is_empty() {
            result.push_str(&rendered);
            decision_rendered = true;
          }
        }
        _ => {
          let converted = convert_node(child);
          if !converted.trim().is_empty() {
            result.push_str(&converted);
          }
        }
      }
    } else if matches_tag(child, "ac:adf-fallback") {
      let converted = convert_node(child);
      if !converted.trim().is_empty() {
        fallback_buffer.push_str(&converted);
      }
    }
  }

  if !decision_rendered && !fallback_buffer.trim().is_empty() {
    result.push_str(fallback_buffer.trim());
    result.push('\n');
  }

  result
}

fn convert_adf_decision_list(node: Node) -> String {
  let mut decisions = Vec::new();

  for child in node.children().filter(|child| matches_tag(*child, "ac:adf-node")) {
    if get_attribute(child, "type").as_deref() == Some("decision-item")
      && let Some(info) = parse_adf_decision_item(child)
    {
      decisions.push(info);
    }
  }

  render_decision_infos(decisions, true)
}

fn parse_adf_decision_item(node: Node) -> Option<DecisionInfo> {
  let attrs = collect_adf_attributes(node);

  let mut info = DecisionInfo::default();
  if let Some(title) = attribute_lookup(&attrs, &["title"]) {
    info.title = title;
  }
  info.status = attribute_lookup(&attrs, &["state", "status"]);
  info.owner = attribute_lookup(
    &attrs,
    &["owner", "owner-id", "ownerid", "assignee", "assignee-id", "decider"],
  );
  info.date = attribute_lookup(&attrs, &["date", "decision-date", "created-date"]);
  info.due_date = attribute_lookup(&attrs, &["due-date", "duedate"]);
  info.outcome = attribute_lookup(&attrs, &["outcome", "result"]);

  let mut paragraphs = collect_adf_paragraphs(node);
  paragraphs.retain(|paragraph| !paragraph.trim().is_empty());

  if info.title.trim().is_empty()
    && let Some(first) = paragraphs.first()
  {
    info.title = first.clone();
    paragraphs.remove(0);
  }

  if !paragraphs.is_empty() {
    info.body = Some(paragraphs.join("\n\n"));
  }

  if !decision_info_has_content(&info) {
    return None;
  }

  Some(info)
}

fn collect_adf_attributes(node: Node) -> BTreeMap<String, String> {
  let mut attributes = BTreeMap::new();

  for child in node.children().filter(|child| matches_tag(*child, "ac:adf-attribute")) {
    if let Some(key) = get_attribute(child, "key") {
      let normalized_key = key.to_ascii_lowercase();
      let value = get_element_text(child).trim().to_string();
      if !value.is_empty() {
        attributes.insert(normalized_key, value);
      }
    }
  }

  attributes
}

fn attribute_lookup(attributes: &BTreeMap<String, String>, keys: &[&str]) -> Option<String> {
  for key in keys {
    let normalized = key.to_ascii_lowercase();
    if let Some(value) = attributes.get(&normalized) {
      let trimmed = value.trim();
      if !trimmed.is_empty() {
        return Some(trimmed.to_string());
      }
    }
  }
  None
}

fn collect_adf_paragraphs(node: Node) -> Vec<String> {
  let mut paragraphs = Vec::new();

  for child in node.children().filter(|child| child.is_element()) {
    collect_adf_paragraphs_from(child, &mut paragraphs);
  }

  paragraphs
}

fn collect_adf_paragraphs_from(node: Node, paragraphs: &mut Vec<String>) {
  if matches_tag(node, "ac:adf-content") {
    let text = get_element_text(node);
    let trimmed = text.trim();
    if !trimmed.is_empty() {
      paragraphs.push(trimmed.to_string());
    }
    return;
  }

  if matches_tag(node, "ac:adf-node")
    && let Some(node_type) = get_attribute(node, "type")
  {
    match node_type.as_str() {
      "paragraph" | "heading" | "blockquote" => {
        if let Some(text) = collect_adf_inline_text(node) {
          paragraphs.push(text);
        }
        return;
      }
      "listItem" => {
        if let Some(text) = collect_adf_inline_text(node) {
          paragraphs.push(text);
        }
        return;
      }
      "bulletList" | "orderedList" => {
        for child in node.children().filter(|child| child.is_element()) {
          collect_adf_paragraphs_from(child, paragraphs);
        }
        return;
      }
      _ => {}
    }
  }

  if matches_tag(node, "ac:adf-fallback") {
    let text = get_element_text(node);
    let trimmed = text.trim();
    if !trimmed.is_empty() {
      paragraphs.push(trimmed.to_string());
    }
    return;
  }

  for child in node.children().filter(|child| child.is_element()) {
    collect_adf_paragraphs_from(child, paragraphs);
  }
}

fn collect_adf_inline_text(node: Node) -> Option<String> {
  let mut buffer = String::new();

  for child in node.children() {
    collect_adf_inline(child, &mut buffer);
  }

  let lines: Vec<String> = buffer
    .split('\n')
    .map(|line| line.trim())
    .filter(|line| !line.is_empty())
    .map(|line| line.to_string())
    .collect();

  if lines.is_empty() { None } else { Some(lines.join("\n")) }
}

fn collect_adf_inline(node: Node, buffer: &mut String) {
  match node.node_type() {
    NodeType::Text => {
      if let Some(text) = node.text() {
        append_inline_text(buffer, text);
      }
    }
    NodeType::Element => {
      if matches_tag(node, "ac:adf-attribute") {
        if let Some(key) = get_attribute(node, "key") {
          let value = get_element_text(node);
          match key.to_ascii_lowercase().as_str() {
            "text" | "title" | "emoji-fallback" => append_inline_text(buffer, &value),
            "emoji-shortname" => {
              if !value.trim().is_empty() {
                append_inline_text(buffer, &value);
              }
            }
            _ => {}
          }
        }
        return;
      }

      if (matches_tag(node, "ac:adf-node") || matches_tag(node, "ac:adf-leaf"))
        && let Some(node_type) = get_attribute(node, "type")
        && node_type == "hardBreak"
        && !buffer.ends_with('\n')
      {
        buffer.push('\n');
      }

      for child in node.children() {
        collect_adf_inline(child, buffer);
      }
    }
    _ => {}
  }
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
    let output = convert_macro_to_markdown(macro_node, &simple_convert_node);

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
    let output = convert_macro_to_markdown(macro_node, &simple_convert_node);

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
    let output = convert_macro_to_markdown(macro_node, &simple_convert_node);

    assert!(output.contains("Decision report macro"));
    assert!(output.contains("space = \"DOCS\" and label = \"meeting-notes\""));
  }
}
