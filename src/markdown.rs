//! Markdown conversion utilities for Confluence content.
//!
//! This module provides functionality to convert Confluence storage format
//! (XHTML-like) to Markdown using proper HTML parsing.

use anyhow::{Context, Result};
use roxmltree::{Document, Node, NodeType};

/// Convert Confluence storage format to Markdown
///
/// This implementation uses proper HTML parsing to handle Confluence's
/// complex XML/HTML structure.
pub fn storage_to_markdown(storage_content: &str, verbose: u8) -> Result<String> {
  // Parse the HTML/XML content using a namespace-aware XML parser.
  //
  // Confluence storage format frequently omits namespace declarations in
  // snippets, so we wrap the payload in a lightweight root element that
  // provides the standard prefix definitions used throughout the format.
  let wrapped = wrap_storage_content(storage_content);
  let document = Document::parse(&wrapped).context("failed to parse Confluence storage content as XML")?;

  // Convert to markdown
  let markdown = convert_element_to_markdown(document.root_element(), verbose);

  // Clean up the result
  let cleaned = clean_markdown(&markdown);

  Ok(cleaned)
}

const AC_NAMESPACE: &str = "http://atlassian.com/content";
const RI_NAMESPACE: &str = "http://atlassian.com/resource/identifier";
const EMOJI_NAMESPACE: &str = "http://atlassian.com/emoji";

fn wrap_storage_content(storage_content: &str) -> String {
  format!(
    "<confluence-root xmlns:ac=\"{AC_NAMESPACE}\" xmlns:ri=\"{RI_NAMESPACE}\" xmlns:emoji=\"{EMOJI_NAMESPACE}\">{storage_content}</confluence-root>"
  )
}

fn qualified_name<'a, 'input>(node: Node<'a, 'input>) -> Option<String> {
  if !node.is_element() {
    return None;
  }

  let local = node.tag_name().name();
  match node.tag_name().namespace() {
    Some(namespace) => node
      .lookup_prefix(namespace)
      .map(|prefix| format!("{prefix}:{local}"))
      .or_else(|| Some(local.to_string())),
    None => Some(local.to_string()),
  }
}

fn has_tag<'a, 'input>(node: Node<'a, 'input>, name: &str) -> bool {
  if !node.is_element() {
    return false;
  }

  match name.split_once(':') {
    Some((prefix, local)) => {
      if node.tag_name().name() != local {
        return false;
      }

      match (node.lookup_namespace_uri(Some(prefix)), node.tag_name().namespace()) {
        (Some(expected), Some(actual)) => expected == actual,
        (None, None) => true,
        _ => false,
      }
    }
    None => node.tag_name().name() == name,
  }
}

fn get_attribute<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<&'input str>
where
  'a: 'input,
{
  if let Some((prefix, local)) = name.split_once(':') {
    let expected_namespace = node.lookup_namespace_uri(Some(prefix));
    node
      .attributes()
      .find(|attr| attr.name() == local && expected_namespace.is_none_or(|ns| attr.namespace() == Some(ns)))
      .map(|attr| attr.value())
  } else {
    node.attribute(name)
  }
}

/// Convert an element and its children to markdown recursively
fn convert_element_to_markdown<'a, 'input>(node: Node<'a, 'input>, verbose: u8) -> String {
  let mut result = String::new();

  for child in node.children() {
    match child.node_type() {
      NodeType::Element => {
        if has_tag(child, "h1") {
          result.push_str(&format!("\n# {}\n\n", get_element_text(child)));
        } else if has_tag(child, "h2") {
          result.push_str(&format!("\n## {}\n\n", get_element_text(child)));
        } else if has_tag(child, "h3") {
          result.push_str(&format!("\n### {}\n\n", get_element_text(child)));
        } else if has_tag(child, "h4") {
          result.push_str(&format!("\n#### {}\n\n", get_element_text(child)));
        } else if has_tag(child, "h5") {
          result.push_str(&format!("\n##### {}\n\n", get_element_text(child)));
        } else if has_tag(child, "h6") {
          result.push_str(&format!("\n###### {}\n\n", get_element_text(child)));
        } else if has_tag(child, "p") {
          let content = convert_element_to_markdown(child, verbose);
          let trimmed = content.trim();
          if !trimmed.is_empty() {
            result.push_str(trimmed);
            result.push_str("\n\n");
          }
        } else if has_tag(child, "strong") || has_tag(child, "b") {
          result.push_str(&format!("**{}**", get_element_text(child)));
        } else if has_tag(child, "em") || has_tag(child, "i") || has_tag(child, "u") {
          result.push_str(&format!("_{}_", get_element_text(child)));
        } else if has_tag(child, "s") || has_tag(child, "del") {
          result.push_str(&format!("~~{}~~", get_element_text(child)));
        } else if has_tag(child, "code") {
          result.push_str(&format!("`{}`", get_element_text(child)));
        } else if has_tag(child, "ul") {
          result.push('\n');
          for li in child.children().filter(|n| n.is_element() && has_tag(*n, "li")) {
            let text = get_element_text(li);
            let trimmed = text.trim();
            if !trimmed.is_empty() {
              result.push_str("- ");
              result.push_str(trimmed);
              result.push('\n');
            }
          }
          result.push('\n');
        } else if has_tag(child, "ol") {
          result.push('\n');
          for (index, li) in child
            .children()
            .filter(|n| n.is_element() && has_tag(*n, "li"))
            .enumerate()
          {
            let text = get_element_text(li);
            let trimmed = text.trim();
            if !trimmed.is_empty() {
              result.push_str(&format!("{}. {}\n", index + 1, trimmed));
            }
          }
          result.push('\n');
        } else if has_tag(child, "a") {
          let text = get_element_text(child);
          let href = get_attribute(child, "href").unwrap_or("");
          result.push_str(&format!("[{}]({})", text.trim(), href));
        } else if has_tag(child, "br") {
          result.push('\n');
        } else if has_tag(child, "hr") {
          result.push_str("\n---\n\n");
        } else if has_tag(child, "pre") {
          let code = get_element_text(child);
          result.push_str("\n```\n");
          result.push_str(code.trim());
          result.push_str("\n```\n\n");
        } else if has_tag(child, "table") {
          result.push_str(&convert_table_to_markdown(child));
        } else if has_tag(child, "ac:structured-macro") {
          result.push_str(&convert_macro_to_markdown(child, verbose));
        } else if has_tag(child, "ac:task-list") {
          result.push_str(&convert_task_list_to_markdown(child));
        } else if has_tag(child, "ac:image") {
          result.push_str(&convert_image_to_markdown(child));
        } else if has_tag(child, "ac:layout")
          || has_tag(child, "ac:layout-section")
          || has_tag(child, "ac:layout-cell")
          || has_tag(child, "ac:rich-text-body")
        {
          result.push_str(&convert_element_to_markdown(child, verbose));
        } else if has_tag(child, "ri:url")
          || has_tag(child, "ac:parameter")
          || has_tag(child, "ac:task-id")
          || has_tag(child, "ac:task-status")
          || has_tag(child, "ac:task-body")
        {
          if has_tag(child, "ac:task-body") {
            result.push_str(&get_element_text(child));
          }
        } else if has_tag(child, "span") {
          if let Some(emoji) = convert_span_emoji(child, verbose) {
            result.push_str(&emoji);
          } else {
            result.push_str(&convert_element_to_markdown(child, verbose));
          }
        } else if has_tag(child, "ac:emoji") || has_tag(child, "ac:emoticon") {
          result.push_str(&convert_emoji_to_markdown(child, verbose));
        } else {
          if verbose >= 3
            && let Some(name) = qualified_name(child)
          {
            eprintln!("[DEBUG] Unknown tag: {name}");
          }
          result.push_str(&convert_element_to_markdown(child, verbose));
        }
      }
      NodeType::Text => {
        if let Some(text) = child.text() {
          let decoded = decode_html_entities(text);
          result.push_str(&decoded);
        }
      }
      _ => {
        // Ignore comments, doctypes, processing instructions, etc.
      }
    }
  }

  result
}

/// Get all text content from an element and its children
fn get_element_text<'a, 'input>(node: Node<'a, 'input>) -> String {
  let mut text = String::new();

  for child in node.children() {
    match child.node_type() {
      NodeType::Text => {
        if let Some(value) = child.text() {
          text.push_str(&decode_html_entities(value));
        }
      }
      NodeType::Element => {
        text.push_str(&get_element_text(child));
      }
      _ => {}
    }
  }

  text
}

/// Convert Confluence structured macros to markdown
fn convert_macro_to_markdown<'a, 'input>(element: Node<'a, 'input>, verbose: u8) -> String {
  let macro_name = get_attribute(element, "ac:name").unwrap_or("");

  match macro_name {
    "toc" => "\n**Table of Contents**\n\n".to_string(),
    "panel" => {
      let body = find_child_by_tag(element, "ac:rich-text-body")
        .map(|elem| convert_element_to_markdown(elem, verbose))
        .unwrap_or_else(|| get_element_text(element));
      format!("\n> {}\n\n", body.trim())
    }
    "note" | "info" | "warning" | "tip" => {
      let title = find_child_by_tag_and_attr(element, "ac:parameter", "ac:name", "title")
        .map(get_element_text)
        .unwrap_or_default();

      let body = find_child_by_tag(element, "ac:rich-text-body")
        .map(|elem| convert_element_to_markdown(elem, verbose))
        .unwrap_or_else(|| get_element_text(element));

      format_admonition_block(macro_name, title.trim(), body.trim())
    }
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
        .map(|elem| convert_element_to_markdown(elem, verbose))
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
        .and_then(|id: &str| emoji_id_to_unicode(id.trim(), verbose))
        .or_else(|| find_child_by_tag_and_attr(element, "ac:parameter", "ac:name", "emoji").map(get_element_text))
        .or_else(|| find_child_by_tag_and_attr(element, "ac:parameter", "ac:name", "shortname").map(get_element_text))
        .unwrap_or_default();

      if verbose >= 2 && !result.is_empty() {
        eprintln!("[DEBUG] Macro emoji: id={emoji_id:?} -> {result}");
      }
      result
    }
    "anchor" => String::new(),
    _ => get_element_text(element),
  }
}

/// Format Confluence admonition macros (note, info, warning, tip) as Markdown
/// blockquotes
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

/// Find a child element by tag name (handles namespaced tags)
fn find_child_by_tag<'a, 'input>(node: Node<'a, 'input>, tag_name: &str) -> Option<Node<'a, 'input>> {
  node
    .children()
    .find(|child| child.is_element() && has_tag(*child, tag_name))
}

/// Find a child element by tag name and attribute value
fn find_child_by_tag_and_attr<'a, 'input>(
  node: Node<'a, 'input>,
  tag_name: &str,
  attr_name: &str,
  attr_value: &str,
) -> Option<Node<'a, 'input>> {
  node.children().find(|child| {
    child.is_element() && has_tag(*child, tag_name) && (get_attribute(*child, attr_name) == Some(attr_value))
  })
}

/// Convert Confluence task list to markdown checkboxes
fn convert_task_list_to_markdown<'a, 'input>(node: Node<'a, 'input>) -> String {
  let mut result = String::new();

  for task in node
    .children()
    .filter(|child| child.is_element() && has_tag(*child, "ac:task"))
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

/// Convert Confluence image to markdown
fn convert_image_to_markdown<'a, 'input>(node: Node<'a, 'input>) -> String {
  let url = find_child_by_tag(node, "ri:url")
    .and_then(|child| get_attribute(child, "ri:value"))
    .unwrap_or("");

  let alt = get_attribute(node, "ac:alt").unwrap_or("image");

  if !url.is_empty() {
    format!("\n![{alt}]({url})\n\n")
  } else {
    format!("\n![{alt}]()\n\n")
  }
}

/// Convert an emoji element to markdown by resolving its codepoint
fn convert_emoji_to_markdown<'a, 'input>(node: Node<'a, 'input>, verbose: u8) -> String {
  let emoji_id = get_attribute(node, "ac:emoji-id");
  let shortcut = get_attribute(node, "ac:shortcut");
  let shortname = get_attribute(node, "ac:shortname").or_else(|| get_attribute(node, "ac:emoji-shortname"));

  if let Some(id) = emoji_id
    && let Some(emoji) = emoji_id_to_unicode(id, verbose)
  {
    if verbose >= 2 {
      eprintln!("[DEBUG] Emoji conversion: id={id} -> {emoji}");
    }
    return emoji;
  }

  if let Some(sc) = shortcut {
    if verbose >= 2 {
      eprintln!("[DEBUG] Emoji shortcut: {sc}");
    }
    return sc.to_owned();
  }

  if let Some(sn) = shortname {
    if verbose >= 2 {
      eprintln!("[DEBUG] Emoji shortname: {sn}");
    }
    return sn.to_owned();
  }

  let text = get_element_text(node);
  if verbose >= 3 && text.trim().is_empty() {
    eprintln!("[DEBUG] Emoji element with no resolvable content");
  }
  if !text.trim().is_empty() { text } else { String::new() }
}

/// Try to resolve emoji metadata stored on span elements
fn convert_span_emoji<'a, 'input>(node: Node<'a, 'input>, verbose: u8) -> Option<String> {
  let emoji_id = get_attribute(node, "data-emoji-id");
  let emoji_shortname = get_attribute(node, "data-emoji-shortname");
  let emoji_fallback = get_attribute(node, "data-emoji-fallback");

  let has_metadata = emoji_id.is_some() || emoji_shortname.is_some() || emoji_fallback.is_some();

  if !has_metadata {
    return None;
  }

  if verbose >= 2 {
    eprintln!("[DEBUG] Span emoji: id={emoji_id:?}, shortname={emoji_shortname:?}, fallback={emoji_fallback:?}");
  }

  if let Some(id) = emoji_id
    && let Some(emoji) = emoji_id_to_unicode(id, verbose)
  {
    if verbose >= 2 {
      eprintln!("[DEBUG] Span emoji resolved: {id} -> {emoji}");
    }
    return Some(emoji);
  }

  let text = get_element_text(node);
  if !text.trim().is_empty() {
    if verbose >= 2 {
      eprintln!("[DEBUG] Span emoji from text: {text}");
    }
    return Some(text);
  }

  if let Some(shortname) = emoji_shortname.or(emoji_fallback) {
    if verbose >= 2 {
      eprintln!("[DEBUG] Span emoji from shortname/fallback: {shortname}");
    }
    return Some(shortname.to_owned());
  }

  if verbose >= 3 {
    eprintln!("[DEBUG] Span emoji with no resolvable content");
  }

  None
}

/// Convert an emoji identifier like "1f44b" or "1f469-200d-1f4bb" into unicode
fn emoji_id_to_unicode(id: &str, verbose: u8) -> Option<String> {
  let trimmed = id.trim().trim_start_matches("emoji-").trim_start_matches("emoji/");
  if trimmed.is_empty() {
    if verbose >= 3 {
      eprintln!("[DEBUG] Empty emoji ID after trimming: {id}");
    }
    return None;
  }

  let mut result = String::new();
  let normalized = trimmed.replace('_', "-");

  for part in normalized.split('-') {
    let part = part.trim();
    if part.is_empty() {
      continue;
    }

    let code = match u32::from_str_radix(part, 16) {
      Ok(c) => c,
      Err(e) => {
        if verbose >= 2 {
          eprintln!("[DEBUG] Failed to parse emoji hex '{part}': {e}");
        }
        return None;
      }
    };

    let ch = match char::from_u32(code) {
      Some(c) => c,
      None => {
        if verbose >= 2 {
          eprintln!("[DEBUG] Invalid unicode codepoint: U+{code:X}");
        }
        return None;
      }
    };

    result.push(ch);
  }

  if result.is_empty() {
    if verbose >= 3 {
      eprintln!("[DEBUG] No valid emoji characters from ID: {id}");
    }
    None
  } else {
    if verbose >= 2 {
      eprintln!("[DEBUG] Emoji ID {id} -> {result}");
    }
    Some(result)
  }
}

/// Convert HTML table to markdown table
fn convert_table_to_markdown<'a, 'input>(node: Node<'a, 'input>) -> String {
  let mut rows: Vec<Vec<String>> = Vec::new();

  for tr in node
    .descendants()
    .filter(|child| child.is_element() && has_tag(*child, "tr"))
  {
    let mut cells: Vec<String> = Vec::new();

    for cell in tr
      .children()
      .filter(|child| child.is_element() && (has_tag(*child, "th") || has_tag(*child, "td")))
    {
      let text = get_element_text(cell)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
      cells.push(text);
    }

    if !cells.is_empty() {
      rows.push(cells);
    }
  }

  if rows.is_empty() {
    return String::new();
  }

  let column_count = rows.iter().map(|row| row.len()).max().unwrap_or(0);
  if column_count == 0 {
    return String::new();
  }

  for row in &mut rows {
    row.resize(column_count, String::new());
  }

  let mut column_widths = vec![0; column_count];
  for row in &rows {
    for (index, cell) in row.iter().enumerate() {
      column_widths[index] = column_widths[index].max(cell.len());
    }
  }

  let mut result = String::new();
  result.push('\n');

  fn format_row(row: &[String], column_widths: &[usize]) -> String {
    let mut line = String::new();
    line.push('|');

    for (cell, width) in row.iter().zip(column_widths) {
      line.push(' ');
      line.push_str(cell);
      if *width > cell.len() {
        line.push_str(&" ".repeat(width - cell.len()));
      }
      line.push(' ');
      line.push('|');
    }

    line.push('\n');
    line
  }

  if let Some(first_row) = rows.first() {
    result.push_str(&format_row(first_row, &column_widths));

    result.push('|');
    for width in &column_widths {
      let dash_count = (*width).max(3);
      result.push(' ');
      result.push_str(&"-".repeat(dash_count));
      result.push(' ');
      result.push('|');
    }
    result.push('\n');
  }

  for row in rows.iter().skip(1) {
    result.push_str(&format_row(row, &column_widths));
  }

  result.push('\n');
  result
}

/// Decode common HTML entities
fn decode_html_entities(text: &str) -> String {
  let replaced = text
    .replace("&nbsp;", " ")
    .replace("&rsquo;", "'")
    .replace("&lsquo;", "'")
    .replace("&rdquo;", "\"")
    .replace("&ldquo;", "\"")
    .replace("&mdash;", "—")
    .replace("&ndash;", "–")
    .replace("&amp;", "&")
    .replace("&lt;", "<")
    .replace("&gt;", ">")
    .replace("&quot;", "\"")
    .replace("&rarr;", "→")
    .replace("&larr;", "←")
    .replace("&#39;", "'");

  decode_numeric_html_entities(&replaced)
}

/// Decode numeric HTML entities so emoji references render properly
fn decode_numeric_html_entities(text: &str) -> String {
  let mut result = String::with_capacity(text.len());
  let mut index = 0;
  let bytes = text.as_bytes();

  while index < text.len() {
    if bytes[index] == b'&'
      && let Some(semi_offset) = text[index..].find(';')
    {
      let end = index + semi_offset;
      if let Some(decoded) = decode_numeric_entity(&text[index + 1..end]) {
        result.push_str(&decoded);
        index = end + 1;
        continue;
      }
    }

    let ch = text[index..].chars().next().unwrap();
    result.push(ch);
    index += ch.len_utf8();
  }

  result
}

fn decode_numeric_entity(entity: &str) -> Option<String> {
  let body = entity.strip_prefix('#')?;

  let (radix, digits) = if let Some(hex) = body.strip_prefix('x').or_else(|| body.strip_prefix('X')) {
    (16, hex)
  } else {
    (10, body)
  };

  if digits.is_empty()
    || !digits.chars().all(|c| {
      if radix == 16 {
        c.is_ascii_hexdigit()
      } else {
        c.is_ascii_digit()
      }
    })
  {
    return None;
  }

  let value = u32::from_str_radix(digits, radix).ok()?;
  let ch = char::from_u32(value)?;
  Some(ch.to_string())
}

/// Clean up the markdown output
fn clean_markdown(content: &str) -> String {
  let mut result = content.to_string();

  // Remove excessive blank lines (more than 2 consecutive)
  while result.contains("\n\n\n") {
    result = result.replace("\n\n\n", "\n\n");
  }

  // Remove leading/trailing whitespace
  result = result.trim().to_string();

  // Ensure file ends with newline
  if !result.ends_with('\n') {
    result.push('\n');
  }

  result
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_decode_html_entities() {
    let input = "There&rsquo;s a lot&mdash;this &amp; that &#x1F642; &#128075;";
    let output = decode_html_entities(input);
    assert_eq!(output, "There's a lot—this & that 🙂 👋");
  }

  #[test]
  fn test_convert_headings() {
    let input = "<h1>Title</h1><h2>Subtitle</h2>";
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("# Title"));
    assert!(output.contains("## Subtitle"));
  }

  #[test]
  fn test_convert_formatting() {
    let input = "<p><strong>bold</strong> <em>italic</em> <s>strike</s></p>";
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("**bold**"));
    assert!(output.contains("_italic_"));
    assert!(output.contains("~~strike~~"));
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

    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("> **Note:** This is a note block."));
  }

  #[test]
  fn test_convert_links() {
    let input = r#"<a href="https://example.com">Example</a>"#;
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("[Example](https://example.com)"));
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
    let output = storage_to_markdown(input, 0).unwrap();
    insta::assert_snapshot!(output, @r###"
    - [ ] Task 1
    - [x] Task 2
    "###);
  }

  #[test]
  fn test_convert_image() {
    let input = r#"<ac:image ac:alt="test image"><ri:url ri:value="https://example.com/image.png" /></ac:image>"#;
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("![test image](https://example.com/image.png)"));
  }

  #[test]
  fn test_convert_table() {
    let input = r#"
      <table>
        <tr><th>Header 1</th><th>Header 2</th></tr>
        <tr><td>Row 1 Col 1</td><td>Row 1 Col 2</td></tr>
        <tr><td>Row 2 Col 1</td><td>Row 2 Col 2</td></tr>
      </table>
    "#;
    let output = storage_to_markdown(input, 0).unwrap();
    insta::assert_snapshot!(output, @r###"
    | Header 1    | Header 2    |
    | ----------- | ----------- |
    | Row 1 Col 1 | Row 1 Col 2 |
    | Row 2 Col 1 | Row 2 Col 2 |
    "###);
  }

  #[test]
  fn test_convert_table_empty() {
    let input = "<table></table>";
    let output = storage_to_markdown(input, 0).unwrap();
    // Empty table should produce minimal output
    assert!(!output.contains("|"));
  }

  #[test]
  fn test_convert_lists() {
    let input = r#"
      <ul>
        <li>Item 1</li>
        <li>Item 2</li>
      </ul>
      <ol>
        <li>First</li>
        <li>Second</li>
      </ol>
    "#;
    let output = storage_to_markdown(input, 0).unwrap();
    insta::assert_snapshot!(output, @r"
    - Item 1
    - Item 2

          
    1. First
    2. Second
    ");
  }

  #[test]
  fn test_convert_code_block() {
    let input = "<pre>function test() {\n  return 42;\n}</pre>";
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("```"));
    assert!(output.contains("function test()"));
  }

  #[test]
  fn test_convert_inline_code() {
    let input = "<p>Use <code>git commit</code> to save</p>";
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("`git commit`"));
  }

  #[test]
  fn test_convert_horizontal_rule() {
    let input = "<p>Before</p><hr /><p>After</p>";
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("---"));
  }

  #[test]
  fn test_convert_line_break() {
    let input = "<p>Line 1<br />Line 2</p>";
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("Line 1\nLine 2"));
  }

  #[test]
  fn test_convert_macro_toc() {
    let input = r#"<ac:structured-macro ac:name="toc"></ac:structured-macro>"#;
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("**Table of Contents**"));
  }

  #[test]
  fn test_convert_macro_panel() {
    let input = r#"
      <ac:structured-macro ac:name="panel">
        <ac:rich-text-body>
          <p>Panel content here</p>
        </ac:rich-text-body>
      </ac:structured-macro>
    "#;
    let output = storage_to_markdown(input, 0).unwrap();
    insta::assert_snapshot!(output, @r###"
    > Panel content here
    "###);
  }

  #[test]
  fn test_convert_macro_status() {
    let input = r#"
      <ac:structured-macro ac:name="status">
        <ac:parameter ac:name="title">In Progress</ac:parameter>
      </ac:structured-macro>
    "#;
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("`[In Progress]`"));
  }

  #[test]
  fn test_convert_macro_expand() {
    let input = r#"
      <ac:structured-macro ac:name="expand">
        <ac:parameter ac:name="title">Click to expand</ac:parameter>
        <ac:rich-text-body>
          <p>Hidden content</p>
        </ac:rich-text-body>
      </ac:structured-macro>
    "#;
    let output = storage_to_markdown(input, 0).unwrap();
    insta::assert_snapshot!(output, @r###"
    <details>
    <summary>Click to expand</summary>

    Hidden content
    </details>
    "###);
  }

  #[test]
  fn test_convert_macro_expand_default_title() {
    let input = r#"
      <ac:structured-macro ac:name="expand">
        <ac:rich-text-body>
          <p>Content without title</p>
        </ac:rich-text-body>
      </ac:structured-macro>
    "#;
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("<summary>Details</summary>"));
  }

  #[test]
  fn test_convert_macro_anchor() {
    let input = r#"<ac:structured-macro ac:name="anchor"><ac:parameter ac:name="name">section1</ac:parameter></ac:structured-macro>"#;
    let output = storage_to_markdown(input, 0).unwrap();
    // Anchor should produce empty output
    assert!(!output.contains("anchor"));
  }

  #[test]
  fn test_convert_macro_unknown() {
    let input = r#"<ac:structured-macro ac:name="unknown-macro">Some text content</ac:structured-macro>"#;
    let output = storage_to_markdown(input, 0).unwrap();
    // Unknown macros should extract text content
    assert!(output.contains("Some text content"));
  }

  #[test]
  fn test_convert_underline() {
    let input = "<p><u>underlined text</u></p>";
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("_underlined text_"));
  }

  #[test]
  fn test_convert_strikethrough() {
    let input = "<p><s>strike</s> and <del>delete</del></p>";
    let output = storage_to_markdown(input, 0).unwrap();
    insta::assert_snapshot!(output, @"~~strike~~ and ~~delete~~");
  }

  #[test]
  fn test_convert_layout_sections() {
    let input = r#"
      <ac:layout>
        <ac:layout-section>
          <ac:layout-cell>
            <p>Cell content</p>
          </ac:layout-cell>
        </ac:layout-section>
      </ac:layout>
    "#;
    let output = storage_to_markdown(input, 0).unwrap();
    insta::assert_snapshot!(output, @"Cell content");
  }

  #[test]
  fn test_convert_rich_text_body() {
    let input = r#"<ac:rich-text-body><p>Rich text</p></ac:rich-text-body>"#;
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("Rich text"));
  }

  #[test]
  fn test_clean_markdown_removes_excessive_newlines() {
    let input = "Line 1\n\n\n\n\nLine 2";
    let output = clean_markdown(input);
    assert!(!output.contains("\n\n\n"));
    assert!(output.contains("Line 1\n\nLine 2"));
  }

  #[test]
  fn test_clean_markdown_adds_trailing_newline() {
    let input = "Some content";
    let output = clean_markdown(input);
    assert!(output.ends_with('\n'));
  }

  #[test]
  fn test_clean_markdown_preserves_double_newlines() {
    let input = "Paragraph 1\n\nParagraph 2";
    let output = clean_markdown(input);
    assert!(output.contains("Paragraph 1\n\nParagraph 2"));
  }

  #[test]
  fn test_convert_image_without_url() {
    let input = r#"<ac:image ac:alt="no url"></ac:image>"#;
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("![no url]()"));
  }

  #[test]
  fn test_convert_image_without_alt() {
    let input = r#"<ac:image><ri:url ri:value="https://example.com/img.png" /></ac:image>"#;
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("![image](https://example.com/img.png)"));
  }

  #[test]
  fn test_convert_confluence_emoji_from_id() {
    let input = r#"<p>Hello <ac:emoji ac:emoji-id="1f44b" /></p>"#;
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("Hello 👋"));
  }

  #[test]
  fn test_convert_confluence_emoji_multi_codepoint() {
    let input = r#"<p><ac:emoji ac:emoji-id="1f469-200d-1f4bb" /></p>"#;
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("👩‍💻"));
  }

  #[test]
  fn test_convert_confluence_emoji_shortcut_fallback() {
    let input = r#"<p><ac:emoji ac:shortcut=":)" /></p>"#;
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains(":)"));
  }

  #[test]
  fn test_convert_emoji_macro() {
    let input = r#"
      <ac:structured-macro ac:name="emoji">
        <ac:parameter ac:name="emoji-id">1f60a</ac:parameter>
      </ac:structured-macro>
    "#;
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("😊"));
  }

  #[test]
  fn test_convert_span_extracts_content() {
    let input = "<p><span>Span content</span></p>";
    let output = storage_to_markdown(input, 0).unwrap();
    assert!(output.contains("Span content"));
  }

  #[test]
  fn test_convert_empty_paragraph() {
    let input = "<p></p><p>   </p>";
    let output = storage_to_markdown(input, 0).unwrap();
    // Empty paragraphs should not produce extra newlines
    assert!(!output.contains("\n\n\n"));
  }

  #[test]
  fn test_get_element_text_recursive() {
    let input = "<div><span>Nested <strong>text</strong> content</span></div>";
    let wrapped = wrap_storage_content(input);
    let document = Document::parse(&wrapped).unwrap();
    let div = document
      .root_element()
      .descendants()
      .find(|node| node.is_element() && has_tag(*node, "div"))
      .unwrap();
    let text = get_element_text(div);
    assert_eq!(text, "Nested text content");
  }

  #[test]
  fn test_decode_all_entities() {
    let input = "&nbsp;&rsquo;&lsquo;&rdquo;&ldquo;&mdash;&ndash;&amp;&lt;&gt;&quot;&rarr;&larr;&#39;";
    let output = decode_html_entities(input);
    assert_eq!(output, " ''\"\"—–&<>\"→←'");
  }
}
