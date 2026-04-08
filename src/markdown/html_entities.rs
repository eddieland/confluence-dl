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
  let result = text
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
    .replace("&laquo;", "\u{00AB}") // left angle quote
    .replace("&raquo;", "\u{00BB}") // right angle quote
    .replace("&iquest;", "\u{00BF}") // inverted question mark
    .replace("&iexcl;", "\u{00A1}") // inverted exclamation mark
    .replace("&sect;", "\u{00A7}") // section sign
    .replace("&para;", "\u{00B6}") // pilcrow / paragraph sign
    .replace("&micro;", "\u{00B5}") // micro sign
    .replace("&cent;", "\u{00A2}") // cent sign
    .replace("&pound;", "\u{00A3}") // pound sign
    .replace("&yen;", "\u{00A5}") // yen sign
    .replace("&euro;", "\u{20AC}") // euro sign
    .replace("&curren;", "\u{00A4}") // currency sign
    .replace("&fnof;", "\u{0192}") // latin small f with hook
    .replace("&permil;", "\u{2030}") // per mille sign
    .replace("&prime;", "\u{2032}") // prime
    .replace("&Prime;", "\u{2033}") // double prime
    .replace("&frasl;", "\u{2044}") // fraction slash
    .replace("&frac14;", "\u{00BC}") // fraction one quarter
    .replace("&frac12;", "\u{00BD}") // fraction one half
    .replace("&frac34;", "\u{00BE}"); // fraction three quarters

  escape_unknown_entities(&result)
}

/// Escape any remaining HTML named entities that are not among XML's 5 predefined entities.
///
/// This is a safety net that prevents `roxmltree` from failing on unrecognised named entities.
/// Unknown entities are escaped (`&` → `&amp;`) so the literal entity text appears in the output
/// rather than crashing the parser.
fn escape_unknown_entities(text: &str) -> String {
  let mut result = String::with_capacity(text.len());
  let mut remaining = text;

  while let Some(amp_pos) = remaining.find('&') {
    // Copy everything before the '&'
    result.push_str(&remaining[..amp_pos]);

    let after_amp = &remaining[amp_pos + 1..];

    // Check if this looks like a named entity reference (&word;)
    if let Some(semi_pos) = after_amp.find(';') {
      let candidate = &after_amp[..semi_pos];

      // Named entities: purely alphabetic. Numeric entities (#123, #xAB) are already handled by roxmltree.
      if !candidate.is_empty() && !candidate.starts_with('#') && candidate.chars().all(|c| c.is_ascii_alphabetic()) {
        // Allow XML's 5 predefined entities through
        match candidate {
          "amp" | "lt" | "gt" | "quot" | "apos" => {
            result.push('&');
            remaining = after_amp;
            continue;
          }
          _ => {
            // Unknown named entity – escape the ampersand so the parser sees literal text
            tracing::warn!("Escaping unrecognised HTML entity: &{candidate};");
            result.push_str("&amp;");
            remaining = after_amp;
            continue;
          }
        }
      }
    }

    // Not a named entity – keep the '&' as-is
    result.push('&');
    remaining = after_amp;
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
  fn test_preprocess_dagger_entities() {
    let input = "See note&dagger; and also&Dagger;";
    let output = preprocess_html_entities(input);
    assert_eq!(output, "See note\u{2020} and also\u{2021}");
  }

  #[test]
  fn test_escape_unknown_entities() {
    // Unknown entity gets escaped so XML parser won't choke
    let input = "foo &unknownEntity; bar";
    let output = escape_unknown_entities(input);
    assert_eq!(output, "foo &amp;unknownEntity; bar");
  }

  #[test]
  fn test_escape_preserves_xml_predefined() {
    // The 5 XML predefined entities must pass through untouched
    let input = "&amp; &lt; &gt; &quot; &apos;";
    let output = escape_unknown_entities(input);
    assert_eq!(output, "&amp; &lt; &gt; &quot; &apos;");
  }

  #[test]
  fn test_escape_preserves_numeric_entities() {
    let input = "&#8224; &#x2020; hello";
    let output = escape_unknown_entities(input);
    assert_eq!(output, "&#8224; &#x2020; hello");
  }

  #[test]
  fn test_preprocess_then_parse_dagger() {
    // Integration: preprocessing + XML parse should succeed
    let input = "<p>Note&dagger;</p>";
    let preprocessed = preprocess_html_entities(input);
    let wrapped = format!("<root>{preprocessed}</root>");
    assert!(roxmltree::Document::parse(&wrapped).is_ok());
  }
}
