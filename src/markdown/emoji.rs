//! Emoji conversion utilities for Confluence content.
//!
//! Handles conversion of Confluence emoji elements and attributes to Unicode
//! emoji.

use roxmltree::Node;

use super::utils::{get_attribute, get_element_text};

/// Convert an emoji element to markdown by resolving its codepoint.
///
/// Confluence stores emojis with various attributes:
/// - `ac:emoji-id`: Hex codepoint(s) like "1f44b" or "1f469-200d-1f4bb"
/// - `ac:shortcut`: Text shortcut like ":)"
/// - `ac:shortname`: Emoji name like ":wave:"
/// - `ac:emoji-fallback`: Fallback text representation
pub fn convert_emoji_to_markdown(element: Node, verbose: u8) -> String {
  let emoji_id = get_attribute(element, "ac:emoji-id");
  let shortcut = get_attribute(element, "ac:shortcut");
  let shortname = get_attribute(element, "ac:shortname").or_else(|| get_attribute(element, "ac:emoji-shortname"));
  let fallback = get_attribute(element, "ac:emoji-fallback");

  if let Some(id) = emoji_id.as_deref()
    && let Some(emoji) = emoji_id_to_unicode(id, verbose)
  {
    if verbose >= 2 {
      eprintln!("[DEBUG] Emoji conversion: id={id} -> {emoji}");
    }
    return emoji;
  }

  if let Some(fb) = fallback.as_deref() {
    if verbose >= 2 {
      eprintln!("[DEBUG] Emoji fallback: {fb}");
    }
    return fb.to_string();
  }

  if let Some(sc) = shortcut.as_deref() {
    if verbose >= 2 {
      eprintln!("[DEBUG] Emoji shortcut: {sc}");
    }
    return sc.to_string();
  }

  if let Some(sn) = shortname.as_deref() {
    if verbose >= 2 {
      eprintln!("[DEBUG] Emoji shortname: {sn}");
    }
    return sn.to_string();
  }

  let text = get_element_text(element);
  if verbose >= 3 && text.trim().is_empty() {
    eprintln!("[DEBUG] Emoji element with no resolvable content");
  }
  if !text.trim().is_empty() { text } else { String::new() }
}

/// Try to resolve emoji metadata stored on span elements.
///
/// Some Confluence content stores emoji information as data attributes on span
/// elements.
pub fn convert_span_emoji(element: Node, verbose: u8) -> Option<String> {
  let emoji_id = get_attribute(element, "data-emoji-id");
  let emoji_shortname = get_attribute(element, "data-emoji-shortname");
  let emoji_fallback = get_attribute(element, "data-emoji-fallback");

  let has_metadata = emoji_id.is_some() || emoji_shortname.is_some() || emoji_fallback.is_some();

  if !has_metadata {
    return None;
  }

  if verbose >= 2 {
    eprintln!("[DEBUG] Span emoji: id={emoji_id:?}, shortname={emoji_shortname:?}, fallback={emoji_fallback:?}");
  }

  if let Some(id) = emoji_id.as_deref()
    && let Some(emoji) = emoji_id_to_unicode(id, verbose)
  {
    if verbose >= 2 {
      eprintln!("[DEBUG] Span emoji resolved: {id} -> {emoji}");
    }
    return Some(emoji);
  }

  let text = get_element_text(element);
  if !text.trim().is_empty() {
    if verbose >= 2 {
      eprintln!("[DEBUG] Span emoji from text: {text}");
    }
    return Some(text);
  }

  if let Some(shortname) = emoji_shortname.or(emoji_fallback).as_deref() {
    if verbose >= 2 {
      eprintln!("[DEBUG] Span emoji from shortname/fallback: {shortname}");
    }
    return Some(shortname.to_string());
  }

  if verbose >= 3 {
    eprintln!("[DEBUG] Span emoji with no resolvable content");
  }

  None
}

/// Convert an emoji identifier like "1f44b" or "1f469-200d-1f4bb" into unicode.
///
/// Confluence stores emoji IDs as hexadecimal Unicode codepoints, sometimes
/// with multiple codepoints joined by hyphens for compound emoji.
pub fn emoji_id_to_unicode(id: &str, verbose: u8) -> Option<String> {
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
    let result = convert_emoji_to_markdown(emoji_node, 0);
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
    let result = convert_emoji_to_markdown(emoji_node, 0);
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
    let result = convert_emoji_to_markdown(emoji_node, 0);
    assert_eq!(result, ":)");
  }

  #[test]
  fn test_emoji_id_to_unicode() {
    assert_eq!(emoji_id_to_unicode("1f44b", 0), Some("👋".to_string()));
    assert_eq!(emoji_id_to_unicode("1f469-200d-1f4bb", 0), Some("👩‍💻".to_string()));
    assert_eq!(emoji_id_to_unicode("emoji-1f60a", 0), Some("😊".to_string()));
  }
}
