//! HTML entity encoding and decoding utilities.
//!
//! This module handles conversion of HTML entities to Unicode characters,
//! both named entities (like `&nbsp;`) and numeric entities (like `&#x1F642;`).

use std::sync::LazyLock;

/// Comprehensive map of HTML named entities to their Unicode characters.
///
/// Covers the full HTML 4 entity set: Latin supplements, Greek letters, math
/// operators, arrows, typographic symbols, currency, and card suits.
static ENTITY_MAP: LazyLock<std::collections::HashMap<&'static str, char>> = LazyLock::new(|| {
  std::collections::HashMap::from([
    // Latin supplement & typography
    ("nbsp", '\u{00A0}'),
    ("iexcl", '\u{00A1}'),
    ("cent", '\u{00A2}'),
    ("pound", '\u{00A3}'),
    ("curren", '\u{00A4}'),
    ("yen", '\u{00A5}'),
    ("brvbar", '\u{00A6}'),
    ("sect", '\u{00A7}'),
    ("uml", '\u{00A8}'),
    ("copy", '\u{00A9}'),
    ("ordf", '\u{00AA}'),
    ("laquo", '\u{00AB}'),
    ("not", '\u{00AC}'),
    ("shy", '\u{00AD}'),
    ("reg", '\u{00AE}'),
    ("macr", '\u{00AF}'),
    ("deg", '\u{00B0}'),
    ("plusmn", '\u{00B1}'),
    ("sup2", '\u{00B2}'),
    ("sup3", '\u{00B3}'),
    ("acute", '\u{00B4}'),
    ("micro", '\u{00B5}'),
    ("para", '\u{00B6}'),
    ("middot", '\u{00B7}'),
    ("cedil", '\u{00B8}'),
    ("sup1", '\u{00B9}'),
    ("ordm", '\u{00BA}'),
    ("raquo", '\u{00BB}'),
    ("frac14", '\u{00BC}'),
    ("frac12", '\u{00BD}'),
    ("frac34", '\u{00BE}'),
    ("iquest", '\u{00BF}'),
    // Latin capital letters with diacritics
    ("Agrave", '\u{00C0}'),
    ("Aacute", '\u{00C1}'),
    ("Acirc", '\u{00C2}'),
    ("Atilde", '\u{00C3}'),
    ("Auml", '\u{00C4}'),
    ("Aring", '\u{00C5}'),
    ("AElig", '\u{00C6}'),
    ("Ccedil", '\u{00C7}'),
    ("Egrave", '\u{00C8}'),
    ("Eacute", '\u{00C9}'),
    ("Ecirc", '\u{00CA}'),
    ("Euml", '\u{00CB}'),
    ("Igrave", '\u{00CC}'),
    ("Iacute", '\u{00CD}'),
    ("Icirc", '\u{00CE}'),
    ("Iuml", '\u{00CF}'),
    ("ETH", '\u{00D0}'),
    ("Ntilde", '\u{00D1}'),
    ("Ograve", '\u{00D2}'),
    ("Oacute", '\u{00D3}'),
    ("Ocirc", '\u{00D4}'),
    ("Otilde", '\u{00D5}'),
    ("Ouml", '\u{00D6}'),
    ("times", '\u{00D7}'),
    ("Oslash", '\u{00D8}'),
    ("Ugrave", '\u{00D9}'),
    ("Uacute", '\u{00DA}'),
    ("Ucirc", '\u{00DB}'),
    ("Uuml", '\u{00DC}'),
    ("Yacute", '\u{00DD}'),
    ("THORN", '\u{00DE}'),
    ("szlig", '\u{00DF}'),
    // Latin small letters with diacritics
    ("agrave", '\u{00E0}'),
    ("aacute", '\u{00E1}'),
    ("acirc", '\u{00E2}'),
    ("atilde", '\u{00E3}'),
    ("auml", '\u{00E4}'),
    ("aring", '\u{00E5}'),
    ("aelig", '\u{00E6}'),
    ("ccedil", '\u{00E7}'),
    ("egrave", '\u{00E8}'),
    ("eacute", '\u{00E9}'),
    ("ecirc", '\u{00EA}'),
    ("euml", '\u{00EB}'),
    ("igrave", '\u{00EC}'),
    ("iacute", '\u{00ED}'),
    ("icirc", '\u{00EE}'),
    ("iuml", '\u{00EF}'),
    ("eth", '\u{00F0}'),
    ("ntilde", '\u{00F1}'),
    ("ograve", '\u{00F2}'),
    ("oacute", '\u{00F3}'),
    ("ocirc", '\u{00F4}'),
    ("otilde", '\u{00F5}'),
    ("ouml", '\u{00F6}'),
    ("divide", '\u{00F7}'),
    ("oslash", '\u{00F8}'),
    ("ugrave", '\u{00F9}'),
    ("uacute", '\u{00FA}'),
    ("ucirc", '\u{00FB}'),
    ("uuml", '\u{00FC}'),
    ("yacute", '\u{00FD}'),
    ("thorn", '\u{00FE}'),
    ("yuml", '\u{00FF}'),
    // Latin extended
    ("OElig", '\u{0152}'),
    ("oelig", '\u{0153}'),
    ("Scaron", '\u{0160}'),
    ("scaron", '\u{0161}'),
    ("Yuml", '\u{0178}'),
    ("fnof", '\u{0192}'),
    ("circ", '\u{02C6}'),
    ("tilde", '\u{02DC}'),
    // Greek uppercase
    ("Alpha", '\u{0391}'),
    ("Beta", '\u{0392}'),
    ("Gamma", '\u{0393}'),
    ("Delta", '\u{0394}'),
    ("Epsilon", '\u{0395}'),
    ("Zeta", '\u{0396}'),
    ("Eta", '\u{0397}'),
    ("Theta", '\u{0398}'),
    ("Iota", '\u{0399}'),
    ("Kappa", '\u{039A}'),
    ("Lambda", '\u{039B}'),
    ("Mu", '\u{039C}'),
    ("Nu", '\u{039D}'),
    ("Xi", '\u{039E}'),
    ("Omicron", '\u{039F}'),
    ("Pi", '\u{03A0}'),
    ("Rho", '\u{03A1}'),
    ("Sigma", '\u{03A3}'),
    ("Tau", '\u{03A4}'),
    ("Upsilon", '\u{03A5}'),
    ("Phi", '\u{03A6}'),
    ("Chi", '\u{03A7}'),
    ("Psi", '\u{03A8}'),
    ("Omega", '\u{03A9}'),
    // Greek lowercase
    ("alpha", '\u{03B1}'),
    ("beta", '\u{03B2}'),
    ("gamma", '\u{03B3}'),
    ("delta", '\u{03B4}'),
    ("epsilon", '\u{03B5}'),
    ("zeta", '\u{03B6}'),
    ("eta", '\u{03B7}'),
    ("theta", '\u{03B8}'),
    ("iota", '\u{03B9}'),
    ("kappa", '\u{03BA}'),
    ("lambda", '\u{03BB}'),
    ("mu", '\u{03BC}'),
    ("nu", '\u{03BD}'),
    ("xi", '\u{03BE}'),
    ("omicron", '\u{03BF}'),
    ("pi", '\u{03C0}'),
    ("rho", '\u{03C1}'),
    ("sigmaf", '\u{03C2}'),
    ("sigma", '\u{03C3}'),
    ("tau", '\u{03C4}'),
    ("upsilon", '\u{03C5}'),
    ("phi", '\u{03C6}'),
    ("chi", '\u{03C7}'),
    ("psi", '\u{03C8}'),
    ("omega", '\u{03C9}'),
    ("thetasym", '\u{03D1}'),
    ("upsih", '\u{03D2}'),
    ("piv", '\u{03D6}'),
    // General punctuation
    ("ensp", '\u{2002}'),
    ("emsp", '\u{2003}'),
    ("thinsp", '\u{2009}'),
    ("zwnj", '\u{200C}'),
    ("zwj", '\u{200D}'),
    ("lrm", '\u{200E}'),
    ("rlm", '\u{200F}'),
    ("ndash", '\u{2013}'),
    ("mdash", '\u{2014}'),
    ("lsquo", '\u{2018}'),
    ("rsquo", '\u{2019}'),
    ("sbquo", '\u{201A}'),
    ("ldquo", '\u{201C}'),
    ("rdquo", '\u{201D}'),
    ("bdquo", '\u{201E}'),
    ("dagger", '\u{2020}'),
    ("Dagger", '\u{2021}'),
    ("bull", '\u{2022}'),
    ("hellip", '\u{2026}'),
    ("permil", '\u{2030}'),
    ("prime", '\u{2032}'),
    ("Prime", '\u{2033}'),
    ("lsaquo", '\u{2039}'),
    ("rsaquo", '\u{203A}'),
    ("oline", '\u{203E}'),
    ("frasl", '\u{2044}'),
    ("euro", '\u{20AC}'),
    // Letterlike symbols
    ("image", '\u{2111}'),
    ("weierp", '\u{2118}'),
    ("real", '\u{211C}'),
    ("trade", '\u{2122}'),
    ("alefsym", '\u{2135}'),
    // Arrows
    ("larr", '\u{2190}'),
    ("uarr", '\u{2191}'),
    ("rarr", '\u{2192}'),
    ("darr", '\u{2193}'),
    ("harr", '\u{2194}'),
    ("crarr", '\u{21B5}'),
    ("lArr", '\u{21D0}'),
    ("uArr", '\u{21D1}'),
    ("rArr", '\u{21D2}'),
    ("dArr", '\u{21D3}'),
    ("hArr", '\u{21D4}'),
    // Mathematical operators
    ("forall", '\u{2200}'),
    ("part", '\u{2202}'),
    ("exist", '\u{2203}'),
    ("empty", '\u{2205}'),
    ("nabla", '\u{2207}'),
    ("isin", '\u{2208}'),
    ("notin", '\u{2209}'),
    ("ni", '\u{220B}'),
    ("prod", '\u{220F}'),
    ("sum", '\u{2211}'),
    ("minus", '\u{2212}'),
    ("lowast", '\u{2217}'),
    ("radic", '\u{221A}'),
    ("prop", '\u{221D}'),
    ("infin", '\u{221E}'),
    ("ang", '\u{2220}'),
    ("and", '\u{2227}'),
    ("or", '\u{2228}'),
    ("cap", '\u{2229}'),
    ("cup", '\u{222A}'),
    ("int", '\u{222B}'),
    ("there4", '\u{2234}'),
    ("sim", '\u{223C}'),
    ("cong", '\u{2245}'),
    ("asymp", '\u{2248}'),
    ("ne", '\u{2260}'),
    ("equiv", '\u{2261}'),
    ("le", '\u{2264}'),
    ("ge", '\u{2265}'),
    ("sub", '\u{2282}'),
    ("sup", '\u{2283}'),
    ("nsub", '\u{2284}'),
    ("sube", '\u{2286}'),
    ("supe", '\u{2287}'),
    ("oplus", '\u{2295}'),
    ("otimes", '\u{2297}'),
    ("perp", '\u{22A5}'),
    ("sdot", '\u{22C5}'),
    // Miscellaneous technical
    ("lceil", '\u{2308}'),
    ("rceil", '\u{2309}'),
    ("lfloor", '\u{230A}'),
    ("rfloor", '\u{230B}'),
    ("lang", '\u{2329}'),
    ("rang", '\u{232A}'),
    // Geometric shapes
    ("loz", '\u{25CA}'),
    // Miscellaneous symbols
    ("spades", '\u{2660}'),
    ("clubs", '\u{2663}'),
    ("hearts", '\u{2665}'),
    ("diams", '\u{2666}'),
  ])
});

/// Replace HTML named entities with Unicode characters before XML parsing.
///
/// `roxmltree` only recognises XML's 5 predefined entities (`&lt;`, `&gt;`,
/// `&amp;`, `&quot;`, `&apos;`), so we convert all HTML named entities to their
/// literal Unicode characters in a single pass.
///
/// Any named entity not found in the lookup table is escaped (`&` → `&amp;`) so
/// the XML parser never encounters an unknown entity reference.
///
/// # Arguments
/// * `text` - Raw storage-format markup that may contain HTML entities.
///
/// # Returns
/// A `String` with HTML entities replaced by literal characters.
pub fn preprocess_html_entities(text: &str) -> String {
  let mut result = String::with_capacity(text.len());
  let mut remaining = text;

  while let Some(amp_pos) = remaining.find('&') {
    // Copy everything before the '&'
    result.push_str(&remaining[..amp_pos]);
    let after_amp = &remaining[amp_pos + 1..];

    // Look for the closing ';' (within a reasonable window to avoid scanning huge spans)
    let maybe_semi = after_amp
      .as_bytes()
      .iter()
      .position(|&b| b == b';' || b == b'&' || b == b'<' || b == b' ' || b == b'\n');

    if let Some(semi_pos) = maybe_semi
      && after_amp.as_bytes()[semi_pos] == b';' {
        let candidate = &after_amp[..semi_pos];

        // Numeric entities (#dec or #xHex) – pass through for roxmltree
        if candidate.starts_with('#') {
          result.push('&');
          remaining = after_amp;
          continue;
        }

        // XML's 5 predefined entities – pass through
        if matches!(candidate, "amp" | "lt" | "gt" | "quot" | "apos") {
          result.push('&');
          remaining = after_amp;
          continue;
        }

        // Named HTML entity – look up in table
        if !candidate.is_empty() && candidate.chars().all(|c| c.is_ascii_alphanumeric()) {
          if let Some(&ch) = ENTITY_MAP.get(candidate) {
            result.push(ch);
            remaining = &after_amp[semi_pos + 1..];
            continue;
          }
          // Unknown named entity – escape ampersand to prevent XML parse failure
          tracing::warn!("Escaping unrecognised HTML entity: &{candidate};");
          result.push_str("&amp;");
          remaining = after_amp;
          continue;
        }
      }

    // Not a recognised pattern – keep the '&' as-is
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
    .replace("&rsquo;", "\u{2019}")
    .replace("&lsquo;", "\u{2018}")
    .replace("&rdquo;", "\u{201D}")
    .replace("&ldquo;", "\u{201C}")
    .replace("&mdash;", "\u{2014}")
    .replace("&ndash;", "\u{2013}")
    .replace("&amp;", "&")
    .replace("&lt;", "<")
    .replace("&gt;", ">")
    .replace("&quot;", "\"")
    .replace("&rarr;", "\u{2192}")
    .replace("&larr;", "\u{2190}")
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
    assert_eq!(output, "There\u{2019}s a lot\u{2014}this & that \u{1F642} \u{1F44B}");
  }

  #[test]
  fn test_decode_all_entities() {
    let input = "&nbsp;&rsquo;&lsquo;&rdquo;&ldquo;&mdash;&ndash;&amp;&lt;&gt;&quot;&rarr;&larr;&#39;";
    let output = decode_html_entities(input);
    assert_eq!(
      output,
      " \u{2019}\u{2018}\u{201D}\u{201C}\u{2014}\u{2013}&<>\"\u{2192}\u{2190}'"
    );
  }

  #[test]
  fn test_preprocess_dagger_entities() {
    let input = "See note&dagger; and also&Dagger;";
    let output = preprocess_html_entities(input);
    assert_eq!(output, "See note\u{2020} and also\u{2021}");
  }

  #[test]
  fn test_preprocess_arrows() {
    let input = "&larr; &rarr; &uarr; &darr; &harr; &lArr; &rArr; &uArr; &dArr; &hArr;";
    let output = preprocess_html_entities(input);
    assert_eq!(
      output,
      "\u{2190} \u{2192} \u{2191} \u{2193} \u{2194} \u{21D0} \u{21D2} \u{21D1} \u{21D3} \u{21D4}"
    );
  }

  #[test]
  fn test_preprocess_greek() {
    let input = "&alpha;&beta;&Gamma;&Delta;";
    let output = preprocess_html_entities(input);
    assert_eq!(output, "\u{03B1}\u{03B2}\u{0393}\u{0394}");
  }

  #[test]
  fn test_preprocess_math_symbols() {
    let input = "&infin; &ne; &le; &ge; &sum; &prod; &radic;";
    let output = preprocess_html_entities(input);
    assert_eq!(output, "\u{221E} \u{2260} \u{2264} \u{2265} \u{2211} \u{220F} \u{221A}");
  }

  #[test]
  fn test_escape_unknown_entities() {
    let input = "foo &unknownEntity; bar";
    let output = preprocess_html_entities(input);
    assert_eq!(output, "foo &amp;unknownEntity; bar");
  }

  #[test]
  fn test_preserves_xml_predefined() {
    let input = "&amp; &lt; &gt; &quot; &apos;";
    let output = preprocess_html_entities(input);
    assert_eq!(output, "&amp; &lt; &gt; &quot; &apos;");
  }

  #[test]
  fn test_preserves_numeric_entities() {
    let input = "&#8224; &#x2020; hello";
    let output = preprocess_html_entities(input);
    assert_eq!(output, "&#8224; &#x2020; hello");
  }

  #[test]
  fn test_preprocess_then_parse_dagger() {
    let input = "<p>Note&dagger;</p>";
    let preprocessed = preprocess_html_entities(input);
    let wrapped = format!("<root>{preprocessed}</root>");
    assert!(roxmltree::Document::parse(&wrapped).is_ok());
  }

  #[test]
  fn test_preprocess_then_parse_harr() {
    let input = "<p>&hArr; &lArr; &rArr;</p>";
    let preprocessed = preprocess_html_entities(input);
    let wrapped = format!("<root>{preprocessed}</root>");
    assert!(roxmltree::Document::parse(&wrapped).is_ok());
  }

  #[test]
  fn test_bare_ampersand_passthrough() {
    let input = "Tom & Jerry";
    let output = preprocess_html_entities(input);
    assert_eq!(output, "Tom & Jerry");
  }

  #[test]
  fn test_mixed_entities() {
    let input = "&copy; 2024 &mdash; Price: &pound;5 &amp; &euro;6";
    let output = preprocess_html_entities(input);
    assert_eq!(output, "\u{00A9} 2024 \u{2014} Price: \u{00A3}5 &amp; \u{20AC}6");
  }
}
