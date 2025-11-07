//! HTML entity encoding and decoding utilities.
//!
//! This module handles conversion of HTML entities to Unicode characters,
//! both named entities (like `&nbsp;`) and numeric entities (like `&#x1F642;`).

use std::borrow::Cow;

use html_escape::decode_html_entities as decode_html_entities_cow;

const XML_BUILT_INS: [(&str, &str); 5] = [
  ("&lt;", "\u{0007}__CDL_XML_LT__\u{0007}"),
  ("&gt;", "\u{0007}__CDL_XML_GT__\u{0007}"),
  ("&amp;", "\u{0007}__CDL_XML_AMP__\u{0007}"),
  ("&quot;", "\u{0007}__CDL_XML_QUOT__\u{0007}"),
  ("&apos;", "\u{0007}__CDL_XML_APOS__\u{0007}"),
];

/// Replace HTML entities with Unicode characters before XML parsing.
///
/// `roxmltree` only recognizes XML's 5 predefined entities (`&lt;`, `&gt;`,
/// `&amp;`, `&quot;`, `&apos;`), so we need to convert other HTML entities to
/// literal characters or numeric references.
///
/// # Arguments
/// * `text` - Raw storage-format markup that may contain HTML entities.
///
/// # Returns
/// A `String` with common HTML entities replaced by literal characters.
pub fn preprocess_html_entities(text: &str) -> String {
  let mut protected: Cow<'_, str> = Cow::Borrowed(text);

  for (entity, placeholder) in XML_BUILT_INS.iter() {
    if protected.contains(entity) {
      protected = Cow::Owned(protected.into_owned().replace(entity, placeholder));
    }
  }

  let decoded = decode_html_entities_cow(&protected).into_owned();
  let mut restored = decoded;

  for (entity, placeholder) in XML_BUILT_INS.iter() {
    if restored.contains(placeholder) {
      restored = restored.replace(placeholder, entity);
    }
  }

  restored
}

/// Decode HTML entities to their Unicode equivalents, normalising non-breaking
/// spaces to plain spaces for Markdown output.
///
/// # Arguments
/// * `text` - Text that may contain HTML entity references.
///
/// # Returns
/// A `String` with entity references expanded into their Unicode characters.
pub fn decode_html_entities(text: &str) -> String {
  let decoded = decode_html_entities_cow(text).into_owned();
  decoded
    .replace('\u{00A0}', " ")
    .replace(['\u{2019}', '\u{2018}'], "'")
    .replace(['\u{201D}', '\u{201C}'], "\"")
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
  fn preprocess_preserves_xml_entities() {
    let input = "<p>1 &lt; 2 &amp; 4 &gt; 3 &quot;hi&quot; &apos;ok&apos;</p>";
    let output = preprocess_html_entities(input);
    assert_eq!(output, "<p>1 &lt; 2 &amp; 4 &gt; 3 &quot;hi&quot; &apos;ok&apos;</p>");
  }

  #[test]
  fn preprocess_decodes_other_entities() {
    let input = "<p>&nbsp; &mdash; &copy;</p>";
    let output = preprocess_html_entities(input);
    assert_eq!(output, "<p>\u{00A0} — ©</p>");
  }
}
