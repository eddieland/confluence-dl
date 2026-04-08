//! HTML entity encoding and decoding utilities.
//!
//! This module handles conversion of HTML entities to Unicode characters,
//! both named entities (like `&nbsp;`) and numeric entities (like `&#x1F642;`).

/// Replace common HTML entities with Unicode characters before XML parsing.
///
/// `roxmltree` only recognizes XML's 5 predefined entities (`&lt;`, `&gt;`,
/// `&amp;`, `&quot;`, `&apos;`), so we need to convert HTML entities to literal
/// characters or numeric references.
///
/// # Arguments
/// * `text` - Raw storage-format markup that may contain HTML entities.
///
/// # Returns
/// A `String` with common HTML entities replaced by literal characters.
pub fn preprocess_html_entities(text: &str) -> String {
  let text = text
    .replace("&nbsp;", "\u{00A0}") // non-breaking space
    .replace("&ndash;", "\u{2013}") // en dash
    .replace("&mdash;", "\u{2014}") // em dash
    .replace("&ldquo;", "\u{201C}") // left double quote
    .replace("&rdquo;", "\u{201D}") // right double quote
    .replace("&lsquo;", "\u{2018}") // left single quote
    .replace("&rsquo;", "\u{2019}") // right single quote
    .replace("&hellip;", "\u{2026}") // horizontal ellipsis
    .replace("&bull;", "\u{2022}") // bullet
    .replace("&middot;", "\u{00B7}") // middle dot
    .replace("&deg;", "\u{00B0}") // degree sign
    .replace("&copy;", "\u{00A9}") // copyright
    .replace("&reg;", "\u{00AE}") // registered trademark
    .replace("&trade;", "\u{2122}") // trademark
    .replace("&times;", "\u{00D7}") // multiplication sign
    .replace("&divide;", "\u{00F7}") // division sign
    .replace("&plusmn;", "\u{00B1}") // plus-minus sign
    .replace("&ne;", "\u{2260}") // not equal
    .replace("&le;", "\u{2264}") // less than or equal
    .replace("&ge;", "\u{2265}") // greater than or equal
    .replace("&larr;", "\u{2190}") // leftwards arrow
    .replace("&rarr;", "\u{2192}") // rightwards arrow
    .replace("&uarr;", "\u{2191}") // upwards arrow
    .replace("&darr;", "\u{2193}") // downwards arrow
    .replace("&dagger;", "\u{2020}") // dagger
    .replace("&Dagger;", "\u{2021}") // double dagger
    .replace("&sect;", "\u{00A7}") // section sign
    .replace("&para;", "\u{00B6}") // paragraph/pilcrow
    .replace("&micro;", "\u{00B5}") // micro sign
    .replace("&cent;", "\u{00A2}") // cent sign
    .replace("&pound;", "\u{00A3}") // pound sign
    .replace("&yen;", "\u{00A5}") // yen sign
    .replace("&euro;", "\u{20AC}") // euro sign
    .replace("&iexcl;", "\u{00A1}") // inverted exclamation mark
    .replace("&iquest;", "\u{00BF}") // inverted question mark
    .replace("&laquo;", "\u{00AB}") // left-pointing double angle quote
    .replace("&raquo;", "\u{00BB}") // right-pointing double angle quote
    .replace("&frac14;", "\u{00BC}") // fraction one quarter
    .replace("&frac12;", "\u{00BD}") // fraction one half
    .replace("&frac34;", "\u{00BE}") // fraction three quarters
    .replace("&sup1;", "\u{00B9}") // superscript one
    .replace("&sup2;", "\u{00B2}") // superscript two
    .replace("&sup3;", "\u{00B3}"); // superscript three

  // Escape any remaining unknown named entities so roxmltree doesn't fail.
  // XML only recognizes 5 predefined entities (lt, gt, amp, quot, apos).
  // Any other `&name;` left after our replacements would cause a parse error,
  // so we escape the `&` to `&amp;` turning them into literal text.
  escape_unknown_named_entities(&text)
}

/// Escape any remaining `&name;` references that aren't one of XML's five
/// predefined entities (`lt`, `gt`, `amp`, `quot`, `apos`).
///
/// Replaces the leading `&` with `&amp;` so the text passes through the XML
/// parser as literal content instead of triggering an "unknown entity" error.
fn escape_unknown_named_entities(text: &str) -> String {
  let xml_entities = ["lt", "gt", "amp", "quot", "apos"];
  let mut result = String::with_capacity(text.len());
  let mut remaining = text;

  while let Some(amp_pos) = remaining.find('&') {
    result.push_str(&remaining[..amp_pos]);

    let after_amp = &remaining[amp_pos + 1..];
    if let Some(semi_pos) = after_amp.find(';') {
      let entity_name = &after_amp[..semi_pos];
      // Numeric entities (&#123; or &#x1F44B;) are handled by roxmltree — leave them alone.
      if entity_name.starts_with('#') || xml_entities.contains(&entity_name) {
        result.push('&');
        result.push_str(&after_amp[..=semi_pos]);
      } else if !entity_name.is_empty() && entity_name.chars().all(|c| c.is_ascii_alphanumeric()) {
        // Unknown named entity — escape the ampersand so it becomes literal text.
        result.push_str("&amp;");
        result.push_str(&after_amp[..=semi_pos]);
      } else {
        // Not a valid entity pattern — pass through as-is.
        result.push('&');
        result.push_str(&after_amp[..=semi_pos]);
      }
      remaining = &after_amp[semi_pos + 1..];
    } else {
      // No closing `;` found — bare `&`, just pass it through.
      result.push('&');
      remaining = after_amp;
    }
  }

  result.push_str(remaining);
  result
}

/// Decode common HTML entities to their Unicode equivalents.
///
/// This handles both named entities and numeric entities (decimal and
/// hexadecimal).
///
/// # Arguments
/// * `text` - Text that may contain HTML entity references.
///
/// # Returns
/// A `String` with entity references expanded into their Unicode characters.
pub fn decode_html_entities(text: &str) -> String {
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

/// Decode numeric HTML entities so emoji references render properly.
///
/// Supports both decimal (`&#128075;`) and hexadecimal (`&#x1F44B;`) formats.
///
/// # Arguments
/// * `text` - Text that may contain numeric HTML entities.
///
/// # Returns
/// A `String` where numeric entity references are replaced with their
/// characters.
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

/// Decode a single numeric HTML entity (without the `&` and `;`).
///
/// # Arguments
/// * `entity` - Numeric entity body such as `#128075` or `#x1F44B`.
///
/// # Returns
/// `Some(String)` containing the decoded character, or `None` if parsing fails.
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
  fn test_decode_all_entities() {
    let input = "&nbsp;&rsquo;&lsquo;&rdquo;&ldquo;&mdash;&ndash;&amp;&lt;&gt;&quot;&rarr;&larr;&#39;";
    let output = decode_html_entities(input);
    assert_eq!(output, " ''\"\"—–&<>\"→←'");
  }

  #[test]
  fn test_preprocess_dagger_entity() {
    let input = "see note&dagger; and also&Dagger;";
    let output = preprocess_html_entities(input);
    assert_eq!(output, "see note\u{2020} and also\u{2021}");
  }

  #[test]
  fn test_preprocess_escapes_unknown_entities() {
    // Unknown named entities should be escaped so roxmltree doesn't fail.
    let input = "some &obscure; entity";
    let output = preprocess_html_entities(input);
    assert_eq!(output, "some &amp;obscure; entity");
  }

  #[test]
  fn test_preprocess_preserves_xml_entities() {
    // XML predefined entities must not be escaped.
    let input = "&lt;b&gt;bold&lt;/b&gt; &amp; &quot;quoted&quot;";
    let output = preprocess_html_entities(input);
    assert_eq!(output, "&lt;b&gt;bold&lt;/b&gt; &amp; &quot;quoted&quot;");
  }

  #[test]
  fn test_preprocess_preserves_numeric_entities() {
    let input = "emoji &#128075; and hex &#x1F642;";
    let output = preprocess_html_entities(input);
    assert_eq!(output, "emoji &#128075; and hex &#x1F642;");
  }
}
