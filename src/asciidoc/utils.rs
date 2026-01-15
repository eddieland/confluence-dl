//! AsciiDoc-specific cleanup utilities.

/// Clean up AsciiDoc output for predictable formatting.
///
/// - Removes excessive blank lines (more than 2 consecutive)
/// - Trims leading/trailing whitespace
/// - Ensures the file ends with a newline
///
/// # Arguments
/// * `content` - Raw AsciiDoc emitted by the converter.
///
/// # Returns
/// A normalized AsciiDoc string that is safe to write to disk.
pub fn clean_asciidoc(content: &str) -> String {
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
  fn test_clean_asciidoc_removes_excessive_newlines() {
    let input = "Line 1\n\n\n\n\nLine 2";
    let output = clean_asciidoc(input);
    assert!(!output.contains("\n\n\n"));
    assert!(output.contains("Line 1\n\nLine 2"));
  }

  #[test]
  fn test_clean_asciidoc_adds_trailing_newline() {
    let input = "Some content";
    let output = clean_asciidoc(input);
    assert!(output.ends_with('\n'));
  }

  #[test]
  fn test_clean_asciidoc_preserves_double_newlines() {
    let input = "Paragraph 1\n\nParagraph 2";
    let output = clean_asciidoc(input);
    assert!(output.contains("Paragraph 1\n\nParagraph 2"));
  }
}
