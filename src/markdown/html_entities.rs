//! HTML entity encoding and decoding utilities.
//!
//! This module handles conversion of HTML entities to Unicode characters,
//! both named entities (like `&nbsp;`) and numeric entities (like `&#x1F642;`).

/// Replace common HTML entities with Unicode characters before XML parsing.
///
/// roxmltree only recognizes XML's 5 predefined entities (&lt; &gt; &amp;
/// &quot; &apos;) so we need to convert HTML entities to literal characters or
/// numeric references.
pub fn preprocess_html_entities(text: &str) -> String {
  replace_html_entities(text, PREPROCESS_ENTITIES, false)
}

/// Decode common HTML entities to their Unicode equivalents.
///
/// This handles both named entities and numeric entities (decimal and
/// hexadecimal).
pub fn decode_html_entities(text: &str) -> String {
  replace_html_entities(text, DECODE_ENTITIES, true)
}

/// Replace named (and optionally numeric) HTML entities in a single pass.
fn replace_html_entities(text: &str, entities: &[(&'static str, &'static str)], decode_numeric: bool) -> String {
  if !text.contains('&') {
    return text.to_owned();
  }

  let mut result = String::with_capacity(text.len());
  let mut index = 0;
  let bytes = text.as_bytes();

  while index < text.len() {
    if bytes[index] == b'&' {
      if let Some((entity, replacement)) = match_named_entity(&text[index..], entities) {
        result.push_str(replacement);
        index += entity.len();

        if decode_numeric {
          if replacement == "&"
            && let Some((nested_entity, nested_replacement)) =
              match_named_entity_following_amp(&text[index..], entities)
          {
            // Replace the `&` we just inserted with the decoded named entity.
            result.pop();
            result.push_str(nested_replacement);
            index += nested_entity.len() - 1;
            continue;
          }

          if replacement == "&"
            && let Some(remaining) = text.get(index..)
            && remaining.starts_with('#')
            && let Some(semi_offset) = remaining.find(';')
            && let Some(decoded) = decode_numeric_entity(&remaining[..semi_offset])
          {
            // Replace the `&` we just inserted with the decoded numeric entity.
            result.pop();
            result.push(decoded);
            index += semi_offset + 1;
            continue;
          }
        }

        continue;
      }

      if decode_numeric
        && let Some(semi_offset) = text[index..].find(';')
        && let Some(decoded) = decode_numeric_entity(&text[index + 1..index + semi_offset])
      {
        result.push(decoded);
        index += semi_offset + 1;
        continue;
      }
    }

    let ch = text[index..].chars().next().unwrap();
    result.push(ch);
    index += ch.len_utf8();
  }

  result
}

fn match_named_entity(text: &str, entities: &[(&'static str, &'static str)]) -> Option<(&'static str, &'static str)> {
  entities.iter().find(|(entity, _)| text.starts_with(entity)).copied()
}

fn match_named_entity_following_amp(
  text: &str,
  entities: &[(&'static str, &'static str)],
) -> Option<(&'static str, &'static str)> {
  entities.iter().find_map(|(entity, replacement)| {
    entity
      .strip_prefix('&')
      .filter(|suffix| text.starts_with(suffix))
      .map(|_| (*entity, *replacement))
  })
}

/// Decode a single numeric HTML entity (without the `&` and `;`).
fn decode_numeric_entity(entity: &str) -> Option<char> {
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
  Some(ch)
}

const PREPROCESS_ENTITIES: &[(&str, &str)] = &[
  ("&nbsp;", "\u{00A0}"),   // non-breaking space
  ("&ndash;", "\u{2013}"),  // en dash
  ("&mdash;", "\u{2014}"),  // em dash
  ("&ldquo;", "\u{201C}"),  // left double quote
  ("&rdquo;", "\u{201D}"),  // right double quote
  ("&lsquo;", "\u{2018}"),  // left single quote
  ("&rsquo;", "\u{2019}"),  // right single quote
  ("&hellip;", "\u{2026}"), // horizontal ellipsis
  ("&bull;", "\u{2022}"),   // bullet
  ("&middot;", "\u{00B7}"), // middle dot
  ("&deg;", "\u{00B0}"),    // degree sign
  ("&copy;", "\u{00A9}"),   // copyright
  ("&reg;", "\u{00AE}"),    // registered trademark
  ("&trade;", "\u{2122}"),  // trademark
  ("&times;", "\u{00D7}"),  // multiplication sign
  ("&divide;", "\u{00F7}"), // division sign
  ("&plusmn;", "\u{00B1}"), // plus-minus sign
  ("&ne;", "\u{2260}"),     // not equal
  ("&le;", "\u{2264}"),     // less than or equal
  ("&ge;", "\u{2265}"),     // greater than or equal
  ("&larr;", "\u{2190}"),   // leftwards arrow
  ("&rarr;", "\u{2192}"),   // rightwards arrow
  ("&uarr;", "\u{2191}"),   // upwards arrow
  ("&darr;", "\u{2193}"),   // downwards arrow
];

const DECODE_ENTITIES: &[(&str, &str)] = &[
  ("&nbsp;", " "),
  ("&rsquo;", "'"),
  ("&lsquo;", "'"),
  ("&rdquo;", "\""),
  ("&ldquo;", "\""),
  ("&mdash;", "—"),
  ("&ndash;", "–"),
  ("&amp;", "&"),
  ("&lt;", "<"),
  ("&gt;", ">"),
  ("&quot;", "\""),
  ("&rarr;", "→"),
  ("&larr;", "←"),
  ("&#39;", "'"),
];

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
  fn test_decode_double_encoded_numeric_entities() {
    assert_eq!(decode_html_entities("&amp;#39;"), "'");
    assert_eq!(decode_html_entities("&amp;#x1F44B;"), "👋");
  }

  #[test]
  fn test_decode_double_encoded_named_entities() {
    assert_eq!(decode_html_entities("&amp;lt;"), "<");
    assert_eq!(decode_html_entities("&amp;mdash;"), "—");
    assert_eq!(decode_html_entities("&amp;amp;"), "&");
    assert_eq!(decode_html_entities("&amp;nbsp;"), " ");
  }
}
