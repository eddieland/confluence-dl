//! Emoji conversion utilities for Confluence content.
//!
//! Handles conversion of Confluence emoji elements and attributes to Unicode
//! emoji.

use roxmltree::Node;
use tracing::{debug, trace};

use super::utils::{get_attribute, get_element_text, get_plain_text};

/// Converts an emoji element to Markdown by resolving its codepoint.
///
/// Confluence stores emojis with various attributes:
/// - `ac:emoji-id`: Hex codepoint(s) like "1f44b" or "1f469-200d-1f4bb"
/// - `ac:shortcut`: Text shortcut like ":)"
/// - `ac:shortname`: Emoji name like ":wave:"
/// - `ac:emoji-fallback`: Fallback text representation
///
/// # Arguments
/// * `element` - The `<ac:emoji>` node to convert.
///
/// # Returns
/// The best matching emoji text or an empty string when the element cannot be
/// resolved.
pub fn convert_emoji_to_markdown(element: Node) -> String {
  let emoji_id = get_attribute(element, "ac:emoji-id");
  let shortcut = get_attribute(element, "ac:shortcut");
  let shortname = get_attribute(element, "ac:shortname").or_else(|| get_attribute(element, "ac:emoji-shortname"));
  let fallback = get_attribute(element, "ac:emoji-fallback");

  if let Some(id) = emoji_id.as_deref()
    && let Some(emoji) = emoji_id_to_unicode(id)
  {
    debug!("Emoji conversion: id={id} -> {emoji}");
    return emoji;
  }

  if let Some(fb) = fallback.as_deref() {
    debug!("Emoji fallback: {fb}");
    return fb.to_string();
  }

  if let Some(sc) = shortcut.as_deref() {
    debug!("Emoji shortcut: {sc}");
    return sc.to_string();
  }

  if let Some(sn) = shortname.as_deref() {
    debug!("Emoji shortname: {sn}");
    return sn.to_string();
  }

  let text = get_element_text(element);
  if text.trim().is_empty() {
    trace!("Emoji element with no resolvable content");
  }
  if !text.trim().is_empty() { text } else { String::new() }
}

/// Attempts to resolve emoji metadata stored on `<span>` elements.
///
/// Some Confluence content stores emoji information as data attributes on span
/// elements.
///
/// # Arguments
/// * `element` - The span node that may contain emoji metadata attributes.
///
/// # Returns
/// `Some(String)` containing the resolved emoji text, or `None` when no emoji
/// metadata is present.
pub fn convert_span_emoji(element: Node) -> Option<String> {
  let emoji_id = get_attribute(element, "data-emoji-id");
  let emoji_shortname = get_attribute(element, "data-emoji-shortname");
  let emoji_fallback = get_attribute(element, "data-emoji-fallback");

  let has_metadata = emoji_id.is_some() || emoji_shortname.is_some() || emoji_fallback.is_some();

  if !has_metadata {
    return None;
  }

  debug!("Span emoji: id={emoji_id:?}, shortname={emoji_shortname:?}, fallback={emoji_fallback:?}");

  if let Some(id) = emoji_id.as_deref()
    && let Some(emoji) = emoji_id_to_unicode(id)
  {
    debug!("Span emoji resolved: {id} -> {emoji}");
    return Some(emoji);
  }

  let text = get_plain_text(element);
  if !text.trim().is_empty() {
    debug!("Span emoji from text: {text}");
    return Some(text);
  }

  if let Some(shortname) = emoji_shortname.or(emoji_fallback).as_deref() {
    debug!("Span emoji from shortname/fallback: {shortname}");
    return Some(shortname.to_string());
  }

  trace!("Span emoji with no resolvable content");

  None
}

/// Converts an emoji identifier like "1f44b" or "1f469-200d-1f4bb" into
/// Unicode characters.
///
/// Confluence stores emoji IDs as hexadecimal Unicode codepoints, sometimes
/// with multiple codepoints joined by hyphens for compound emoji.
///
/// # Arguments
/// * `id` - Emoji identifier from Confluence metadata.
///
/// # Returns
/// `Some(String)` containing the Unicode emoji when parsing succeeds, or `None`
/// if the identifier is invalid.
pub fn emoji_id_to_unicode(id: &str) -> Option<String> {
  let trimmed = id.trim().trim_start_matches("emoji-").trim_start_matches("emoji/");
  if trimmed.is_empty() {
    trace!("Empty emoji ID after trimming: {id}");
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
        debug!("Failed to parse emoji hex '{part}': {e}");
        return None;
      }
    };

    let ch = match char::from_u32(code) {
      Some(c) => c,
      None => {
        debug!("Invalid unicode codepoint: U+{code:X}");
        return None;
      }
    };

    result.push(ch);
  }

  if result.is_empty() {
    trace!("No valid emoji characters from ID: {id}");
    None
  } else {
    debug!("Emoji ID {id} -> {result}");
    Some(result)
  }
}

#[cfg(test)]
mod tests {
  use roxmltree::Document;

  use super::*;
  use crate::markdown::utils::{matches_tag, wrap_with_namespaces};

  #[test]
  fn test_convert_confluence_emoji_from_id() {
    let input = r#"<p>Hello <ac:emoji ac:emoji-id="1f44b" /></p>"#;
    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let emoji_node = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:emoji"))
      .unwrap();
    let result = convert_emoji_to_markdown(emoji_node);
    assert_eq!(result, "👋");
  }

  #[test]
  fn test_convert_confluence_emoji_multi_codepoint() {
    let input = r#"<p><ac:emoji ac:emoji-id="1f469-200d-1f4bb" /></p>"#;
    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let emoji_node = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:emoji"))
      .unwrap();
    let result = convert_emoji_to_markdown(emoji_node);
    assert_eq!(result, "👩‍💻");
  }

  #[test]
  fn test_convert_confluence_emoji_shortcut_fallback() {
    let input = r#"<p><ac:emoji ac:shortcut=":)" /></p>"#;
    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let emoji_node = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:emoji"))
      .unwrap();
    let result = convert_emoji_to_markdown(emoji_node);
    assert_eq!(result, ":)");
  }

  #[test]
  fn test_emoji_id_to_unicode() {
    assert_eq!(emoji_id_to_unicode("1f44b"), Some("👋".to_string()));
    assert_eq!(emoji_id_to_unicode("1f469-200d-1f4bb"), Some("👩‍💻".to_string()));
    assert_eq!(emoji_id_to_unicode("emoji-1f60a"), Some("😊".to_string()));
  }
}
