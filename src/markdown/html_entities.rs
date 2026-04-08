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
  let replaced = text
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
    .replace("&harr;", "\u{2194}") // left right arrow
    .replace("&lArr;", "\u{21D0}") // leftwards double arrow
    .replace("&rArr;", "\u{21D2}") // rightwards double arrow
    .replace("&uArr;", "\u{21D1}") // upwards double arrow
    .replace("&dArr;", "\u{21D3}") // downwards double arrow
    .replace("&hArr;", "\u{21D4}") // left right double arrow
    .replace("&euro;", "\u{20AC}") // euro sign
    .replace("&pound;", "\u{00A3}") // pound sign
    .replace("&yen;", "\u{00A5}") // yen sign
    .replace("&cent;", "\u{00A2}") // cent sign
    .replace("&sect;", "\u{00A7}") // section sign
    .replace("&para;", "\u{00B6}") // pilcrow / paragraph sign
    .replace("&micro;", "\u{00B5}") // micro sign
    .replace("&frac14;", "\u{00BC}") // fraction one quarter
    .replace("&frac12;", "\u{00BD}") // fraction one half
    .replace("&frac34;", "\u{00BE}") // fraction three quarters
    .replace("&sup1;", "\u{00B9}") // superscript one
    .replace("&sup2;", "\u{00B2}") // superscript two
    .replace("&sup3;", "\u{00B3}") // superscript three
    .replace("&laquo;", "\u{00AB}") // left-pointing double angle quotation
    .replace("&raquo;", "\u{00BB}") // right-pointing double angle quotation
    .replace("&iquest;", "\u{00BF}") // inverted question mark
    .replace("&iexcl;", "\u{00A1}") // inverted exclamation mark
    .replace("&not;", "\u{00AC}") // not sign
    .replace("&shy;", "\u{00AD}") // soft hyphen
    .replace("&macr;", "\u{00AF}") // macron
    .replace("&acute;", "\u{00B4}") // acute accent
    .replace("&cedil;", "\u{00B8}") // cedilla
    .replace("&infin;", "\u{221E}") // infinity
    .replace("&fnof;", "\u{0192}") // latin small f with hook
    .replace("&alpha;", "\u{03B1}") // greek small alpha
    .replace("&beta;", "\u{03B2}") // greek small beta
    .replace("&gamma;", "\u{03B3}") // greek small gamma
    .replace("&delta;", "\u{03B4}") // greek small delta
    .replace("&epsilon;", "\u{03B5}") // greek small epsilon
    .replace("&lambda;", "\u{03BB}") // greek small lambda
    .replace("&mu;", "\u{03BC}") // greek small mu
    .replace("&pi;", "\u{03C0}") // greek small pi
    .replace("&sigma;", "\u{03C3}") // greek small sigma
    .replace("&omega;", "\u{03C9}") // greek small omega
    .replace("&sum;", "\u{2211}") // n-ary summation
    .replace("&prod;", "\u{220F}") // n-ary product
    .replace("&radic;", "\u{221A}") // square root
    .replace("&empty;", "\u{2205}") // empty set
    .replace("&sim;", "\u{223C}") // tilde operator
    .replace("&cong;", "\u{2245}") // approximately equal
    .replace("&asymp;", "\u{2248}") // almost equal
    .replace("&equiv;", "\u{2261}") // identical to
    .replace("&sub;", "\u{2282}") // subset of
    .replace("&sup;", "\u{2283}") // superset of
    .replace("&nsub;", "\u{2284}") // not a subset of
    .replace("&sube;", "\u{2286}") // subset of or equal to
    .replace("&supe;", "\u{2287}") // superset of or equal to
    .replace("&oplus;", "\u{2295}") // circled plus
    .replace("&otimes;", "\u{2297}") // circled times
    .replace("&perp;", "\u{22A5}") // up tack / perpendicular
    .replace("&and;", "\u{2227}") // logical and
    .replace("&or;", "\u{2228}") // logical or
    .replace("&cap;", "\u{2229}") // intersection
    .replace("&cup;", "\u{222A}") // union
    .replace("&int;", "\u{222B}") // integral
    .replace("&there4;", "\u{2234}") // therefore
    .replace("&part;", "\u{2202}") // partial differential
    .replace("&exist;", "\u{2203}") // there exists
    .replace("&forall;", "\u{2200}") // for all
    .replace("&nabla;", "\u{2207}") // nabla
    .replace("&isin;", "\u{2208}") // element of
    .replace("&notin;", "\u{2209}") // not an element of
    .replace("&minus;", "\u{2212}") // minus sign
    .replace("&lowast;", "\u{2217}") // asterisk operator
    .replace("&prop;", "\u{221D}") // proportional to
    .replace("&ang;", "\u{2220}") // angle
    .replace("&prime;", "\u{2032}") // prime
    .replace("&Prime;", "\u{2033}") // double prime
    .replace("&oline;", "\u{203E}") // overline
    .replace("&weierp;", "\u{2118}") // Weierstrass p
    .replace("&image;", "\u{2111}") // imaginary part
    .replace("&real;", "\u{211C}") // real part
    .replace("&alefsym;", "\u{2135}") // alef symbol
    .replace("&crarr;", "\u{21B5}") // downwards arrow with corner leftwards
    .replace("&loz;", "\u{25CA}") // lozenge
    .replace("&spades;", "\u{2660}") // black spade suit
    .replace("&clubs;", "\u{2663}") // black club suit
    .replace("&hearts;", "\u{2665}") // black heart suit
    .replace("&diams;", "\u{2666}"); // black diamond suit

  // Fallback: escape any remaining named HTML entities that the XML parser
  // would reject. The 5 XML predefined entities are left intact.
  escape_unknown_entities(&replaced)
}

/// Escape any remaining named HTML entities that aren't XML-predefined.
///
/// After known entities have been replaced with Unicode characters, this function
/// catches any remaining `&name;` patterns (e.g. rare or non-standard entities)
/// and escapes their `&` to `&amp;` so the XML parser doesn't reject them.
/// The 5 XML predefined entities (`&lt;`, `&gt;`, `&amp;`, `&quot;`, `&apos;`)
/// and numeric entities (`&#123;`, `&#x1F44B;`) are left intact.
fn escape_unknown_entities(text: &str) -> String {
  let mut result = String::with_capacity(text.len());
  let bytes = text.as_bytes();
  let len = bytes.len();
  let mut i = 0;

  while i < len {
    if bytes[i] == b'&'
      && let Some(semi_offset) = text[i..].find(';')
    {
      let entity_body = &text[i + 1..i + semi_offset];

      // Allow XML predefined entities
      let is_xml_predefined = matches!(entity_body, "lt" | "gt" | "amp" | "quot" | "apos");

      // Allow numeric entities (&#123; or &#x1F44B;)
      let is_numeric = entity_body.starts_with('#');

      if is_xml_predefined || is_numeric {
        // Pass through as-is
        result.push_str(&text[i..=i + semi_offset]);
        i += semi_offset + 1;
        continue;
      }

      // Check if this looks like a named entity (alphabetic characters only)
      let is_named_entity = !entity_body.is_empty() && entity_body.chars().all(|c| c.is_ascii_alphabetic());

      if is_named_entity {
        // Escape the & so the XML parser treats it as literal text
        tracing::debug!("Escaping unknown HTML entity: &{entity_body};");
        result.push_str("&amp;");
        result.push_str(&text[i + 1..=i + semi_offset]);
        i += semi_offset + 1;
        continue;
      }
    }

    let ch = text[i..].chars().next().unwrap();
    result.push(ch);
    i += ch.len_utf8();
  }

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
  fn test_preprocess_harr_entity() {
    let input = "A &harr; B";
    let output = preprocess_html_entities(input);
    assert_eq!(output, "A \u{2194} B");
  }

  #[test]
  fn test_preprocess_double_arrow_entities() {
    let input = "&lArr; &rArr; &hArr;";
    let output = preprocess_html_entities(input);
    assert_eq!(output, "\u{21D0} \u{21D2} \u{21D4}");
  }

  #[test]
  fn test_escape_unknown_entities() {
    // Unknown entity should be escaped so XML parser doesn't reject it
    let input = "foo &obscure; bar";
    let output = escape_unknown_entities(input);
    assert_eq!(output, "foo &amp;obscure; bar");
  }

  #[test]
  fn test_escape_preserves_xml_predefined() {
    // XML predefined entities must pass through unchanged
    let input = "&lt;tag&gt; &amp; &quot;hi&quot; &apos;x&apos;";
    let output = escape_unknown_entities(input);
    assert_eq!(output, input);
  }

  #[test]
  fn test_escape_preserves_numeric_entities() {
    let input = "&#128075; &#x1F642;";
    let output = escape_unknown_entities(input);
    assert_eq!(output, input);
  }

  #[test]
  fn test_preprocess_unknown_entity_does_not_crash_xml() {
    // After preprocessing, an unknown entity should be escaped, not left raw
    let input = "text with &weirdentity; inside";
    let output = preprocess_html_entities(input);
    assert_eq!(output, "text with &amp;weirdentity; inside");
  }
}
