use std::collections::BTreeMap;

use roxmltree::{Node, NodeType};

use crate::markdown::MarkdownOptions;
use crate::markdown::utils::{
  find_child_by_tag, find_child_by_tag_and_attr, get_attribute, get_element_text, matches_tag,
};

/// Converts Confluence decision macros into descriptive Markdown blocks.
///
/// # Arguments
/// * `macro_name` - The specific decision macro variant (`decision`,
///   `decision-list`, `decisionreport`).
/// * `element` - The `<ac:structured-macro>` node containing decision metadata
///   and body.
/// * `convert_node` - Callback used to render nested rich text nodes into
///   Markdown.
/// * `_options` - Markdown rendering options (not currently used by decision
///   macros).
///
/// # Returns
/// Markdown representation for the decision macro, or `None` when unhandled.
pub(super) fn handle_macro(
  macro_name: &str,
  element: Node,
  convert_node: &dyn Fn(Node) -> String,
  _options: &MarkdownOptions,
) -> Option<String> {
  let rendered = match macro_name {
    "decisionreport" => format_decision_report(element),
    "decision" => format_decision(element, convert_node),
    "decision-list" => format_decision_list(element, convert_node),
    _ => return None,
  };

  Some(rendered)
}

/// Convert Atlassian Document Format decision extensions to Markdown.
///
/// # Arguments
/// * `element` - The `<ac:adf-extension>` node describing decision content.
/// * `convert_node` - Callback used to render nested rich text into Markdown.
///
/// # Returns
/// A Markdown fragment representing the decision content when available,
/// otherwise the fallback rendering of embedded nodes.
pub fn convert_adf_extension_to_markdown(element: Node, convert_node: &dyn Fn(Node) -> String) -> String {
  let mut result = String::new();
  let mut decision_rendered = false;
  let mut segments: Vec<(String, bool)> = Vec::new();

  for child in element.children().filter(|child| child.is_element()) {
    if matches_tag(child, "ac:adf-node") {
      match get_attribute(child, "type").as_deref() {
        Some("decision-list") => {
          let rendered = convert_adf_decision_list(child);
          if !rendered.is_empty() {
            flush_adf_segments(&mut result, &mut segments, false);
            result.push_str(&rendered);
            decision_rendered = true;
          }
        }
        _ => append_adf_segment(&mut segments, convert_node(child), false),
      }
    } else if matches_tag(child, "ac:adf-fallback") {
      append_adf_segment(&mut segments, convert_node(child), true);
    } else {
      append_adf_segment(&mut segments, convert_node(child), false);
    }
  }

  if decision_rendered {
    flush_adf_segments(&mut result, &mut segments, false);
    result
  } else {
    flush_adf_segments(&mut result, &mut segments, true);
    result
  }
}

fn append_adf_segment(segments: &mut Vec<(String, bool)>, content: String, is_fallback: bool) {
  if content.trim().is_empty() {
    return;
  }

  if let Some((existing, existing_fallback)) = segments.last_mut()
    && *existing_fallback == is_fallback
  {
    existing.push_str(&content);
    return;
  }

  segments.push((content, is_fallback));
}

fn flush_adf_segments(result: &mut String, segments: &mut Vec<(String, bool)>, include_fallback: bool) {
  if segments.is_empty() {
    return;
  }

  for (content, is_fallback) in segments.iter() {
    if include_fallback || !*is_fallback {
      result.push_str(content);
    }
  }

  segments.clear();
}

/// Aggregated decision metadata used for rendering Markdown output.
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

/// Renders the decision report macro, which links to dynamic Confluence
/// content.
///
/// # Arguments
/// * `element` - The `<ac:structured-macro>` node for `decisionreport`
///   containing an optional CQL query.
///
/// # Returns
/// Markdown note explaining that the dynamic content is not exported, with the
/// CQL query when provided.
fn format_decision_report(element: Node) -> String {
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

/// Appends trimmed text to the target buffer, collapsing whitespace boundaries.
///
/// # Arguments
/// * `target` - Accumulator receiving the trimmed segment.
/// * `segment` - Raw segment that may include extra whitespace.
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

/// Appends inline text to the buffer, inserting spaces or newlines as needed.
///
/// # Arguments
/// * `target` - Accumulator for inline decision metadata.
/// * `segment` - Segment that should be appended while preserving inline
///   readability.
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

/// Extracts and renders a parameter value from a decision macro.
///
/// # Arguments
/// * `element` - Decision macro element that may contain nested parameters.
/// * `name` - Parameter name to locate (`title`, `status`, etc.).
/// * `convert_node` - Callback used to render child elements into Markdown
///   text.
///
/// # Returns
/// Trimmed string value for the parameter, or `None` if not present.
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

/// Collects decision metadata and body content from a structured macro.
///
/// # Arguments
/// * `element` - The `<ac:structured-macro>` node describing a decision.
/// * `convert_node` - Callback used to render nested rich-text nodes.
///
/// # Returns
/// A `DecisionInfo` struct containing normalized metadata fields and body text.
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

/// Formats a parsed decision into human-readable Markdown lines.
///
/// # Arguments
/// * `info` - Parsed decision metadata and body content.
///
/// # Returns
/// Markdown string beginning with a bolded decision title followed by metadata
/// and body text.
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

/// Turns a multi-line decision description into a Markdown list item.
///
/// # Arguments
/// * `content` - Fully formatted decision content that may span multiple lines.
///
/// # Returns
/// List item string starting with `-` when content is non-empty, otherwise
/// `None`.
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

/// Determines whether a parsed decision contains meaningful data.
///
/// # Arguments
/// * `info` - Decision metadata to inspect.
///
/// # Returns
/// `true` when the decision has either a title or body content worth rendering.
fn decision_info_has_content(info: &DecisionInfo) -> bool {
  let has_title = !info.title.trim().is_empty();
  let has_body = info.body.as_ref().map(|body| !body.trim().is_empty()).unwrap_or(false);
  has_title || has_body
}

/// Renders a collection of decision descriptions as a Markdown list.
///
/// # Arguments
/// * `decisions` - Parsed decision entries to render in order.
/// * `skip_empty` - When true, suppresses entries without titles or body
///   content.
///
/// # Returns
/// Markdown list separated by blank lines, or an empty string when nothing
/// qualifies.
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

/// Renders a single `decision` macro into a Markdown block.
///
/// # Arguments
/// * `element` - The decision macro element to parse.
/// * `convert_node` - Callback used to render the rich-text body.
///
/// # Returns
/// Markdown block containing the formatted decision, or an empty string when it
/// lacks content.
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

/// Renders a `decision-list` macro by walking nested decision macros.
///
/// # Arguments
/// * `element` - The decision list macro element containing a rich-text body.
/// * `convert_node` - Callback used to render fallback content and nested
///   bodies.
///
/// # Returns
/// Markdown list of decisions or a fallback rendering when no structured
/// decisions exist.
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

/// Converts an ADF decision list node into Markdown list items.
///
/// # Arguments
/// * `element` - The `<ac:adf-node type=\"decision-list\">` element to process.
///
/// # Returns
/// Markdown bullet list or an empty string when no decision items are present.
fn convert_adf_decision_list(element: Node) -> String {
  let decisions: Vec<_> = element
    .children()
    .filter(|child| matches_tag(*child, "ac:adf-node"))
    .filter(|child| get_attribute(*child, "type").as_deref() == Some("decision-item"))
    .filter_map(parse_adf_decision)
    .collect();

  if decisions.is_empty() {
    return String::new();
  }

  render_decision_infos(decisions, true)
}

/// Parses a single ADF decision item node into `DecisionInfo`.
///
/// # Arguments
/// * `node` - The `<ac:adf-node type=\"decision-item\">` element describing the
///   decision.
///
/// # Returns
/// Populated `DecisionInfo` when content is meaningful, otherwise `None`.
fn parse_adf_decision(node: Node) -> Option<DecisionInfo> {
  let mut info = DecisionInfo::default();
  let attributes = collect_adf_attributes(node);

  if let Some(title) = attribute_lookup(&attributes, &["title", "text", "value"]) {
    info.title = title;
  }

  info.status = attribute_lookup(&attributes, &["state", "status"]);
  info.owner = attribute_lookup(
    &attributes,
    &[
      "owner",
      "owner-id",
      "ownerid",
      "assignee",
      "assignee-id",
      "decider",
      "atlassian:user-context",
    ],
  );
  info.date = attribute_lookup(&attributes, &["date", "decision-date", "created-date"]);
  info.due_date = attribute_lookup(&attributes, &["due-date", "duedate", "dueDate"]);
  info.outcome = attribute_lookup(&attributes, &["outcome", "result"]);

  let mut paragraphs = collect_adf_paragraphs(node);
  paragraphs.retain(|paragraph| !paragraph.trim().is_empty());

  if info.title.trim().is_empty()
    && let Some(first) = paragraphs.first().cloned()
  {
    info.title = first;
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

/// Collects key/value attribute pairs from ADF decision nodes.
///
/// # Arguments
/// * `node` - ADF node containing `ac:adf-attribute` children.
///
/// # Returns
/// Map of lowercased attribute keys to trimmed string values.
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

/// Retrieves the first matching attribute from a list of candidate keys.
///
/// # Arguments
/// * `attributes` - Attribute map produced by `collect_adf_attributes`.
/// * `keys` - Ordered list of potential attribute names (case-insensitive).
///
/// # Returns
/// Trimmed attribute value when found, otherwise `None`.
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

/// Walks an ADF node tree to collect paragraph-like text segments.
///
/// # Arguments
/// * `node` - Root node whose descendants may contain ADF content blocks.
///
/// # Returns
/// Vector of trimmed paragraph strings.
fn collect_adf_paragraphs(node: Node) -> Vec<String> {
  let mut paragraphs = Vec::new();

  for child in node.children().filter(|child| child.is_element()) {
    collect_adf_paragraphs_from(child, &mut paragraphs);
  }

  paragraphs
}

/// Recursively extracts paragraph text from ADF nodes.
///
/// # Arguments
/// * `node` - Current node being inspected for textual content.
/// * `paragraphs` - Accumulator receiving discovered paragraphs.
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

/// Flattens inline ADF nodes into a normalized Markdown-friendly string.
///
/// # Arguments
/// * `node` - The inline node (e.g., paragraph) whose children should be
///   flattened.
///
/// # Returns
/// Joined string with newlines preserved for hard breaks, or `None` when empty.
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

/// Traverses inline ADF nodes to accumulate text into a buffer.
///
/// # Arguments
/// * `node` - Node to inspect, which may be text, leaf, or nested structure.
/// * `buffer` - Mutable buffer receiving formatted inline content.
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
