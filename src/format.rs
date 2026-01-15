//! Output format definitions and utilities.

use clap::ValueEnum;

/// Supported output formats for Confluence content conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum OutputFormat {
  /// Markdown output (default)
  #[default]
  Markdown,
  /// AsciiDoc output (Asciidoctor-compatible)
  #[value(alias = "adoc")]
  AsciiDoc,
}

impl OutputFormat {
  /// Returns the file extension for this output format.
  pub fn file_extension(&self) -> &'static str {
    match self {
      OutputFormat::Markdown => "md",
      OutputFormat::AsciiDoc => "adoc",
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_file_extension() {
    assert_eq!(OutputFormat::Markdown.file_extension(), "md");
    assert_eq!(OutputFormat::AsciiDoc.file_extension(), "adoc");
  }

  #[test]
  fn test_default_is_markdown() {
    assert_eq!(OutputFormat::default(), OutputFormat::Markdown);
  }
}
