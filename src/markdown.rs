//! Markdown conversion utilities for Confluence content.
//!
//! This module provides functionality to convert Confluence storage format
//! (XHTML-like) to Markdown.

use anyhow::Result;

/// Convert Confluence storage format to Markdown
///
/// This is a basic implementation that handles common Confluence elements.
/// More sophisticated conversion can be added later.
pub fn storage_to_markdown(storage_content: &str) -> Result<String> {
  let mut markdown = storage_content.to_string();

  // Remove XML declaration if present
  markdown = markdown
    .trim_start_matches("<?xml version=\"1.0\" encoding=\"UTF-8\"?>")
    .trim()
    .to_string();

  // Convert headings
  markdown = convert_headings(&markdown);

  // Convert formatting
  markdown = convert_formatting(&markdown);

  // Convert links
  markdown = convert_links(&markdown);

  // Convert lists
  markdown = convert_lists(&markdown);

  // Convert code blocks
  markdown = convert_code_blocks(&markdown);

  // Convert images
  markdown = convert_images(&markdown);

  // Clean up remaining HTML tags (simple approach)
  markdown = strip_common_tags(&markdown);

  // Clean up excessive newlines
  markdown = clean_whitespace(&markdown);

  Ok(markdown)
}

/// Convert Confluence headings to Markdown
fn convert_headings(content: &str) -> String {
  let mut result = content.to_string();

  // h1-h6 tags
  for level in 1..=6 {
    let opening = format!("<h{level}>");
    let closing = format!("</h{level}>");
    let prefix = "#".repeat(level);

    while let Some(start) = result.find(&opening) {
      if let Some(end) = result[start..].find(&closing) {
        let full_end = start + end + closing.len();
        let content = &result[start + opening.len()..start + end];
        let replacement = format!("\n{} {}\n", prefix, content.trim());
        result.replace_range(start..full_end, &replacement);
      } else {
        break;
      }
    }
  }

  result
}

/// Convert text formatting (bold, italic, etc.)
fn convert_formatting(content: &str) -> String {
  let mut result = content.to_string();

  // Bold (<strong> or <b>)
  result = result.replace("<strong>", "**").replace("</strong>", "**");
  result = result.replace("<b>", "**").replace("</b>", "**");

  // Italic (<em> or <i>)
  result = result.replace("<em>", "_").replace("</em>", "_");
  result = result.replace("<i>", "_").replace("</i>", "_");

  // Underline (no direct markdown equivalent, use emphasis)
  result = result.replace("<u>", "_").replace("</u>", "_");

  // Strikethrough
  result = result.replace("<s>", "~~").replace("</s>", "~~");
  result = result.replace("<del>", "~~").replace("</del>", "~~");

  result
}

/// Convert links to Markdown
fn convert_links(content: &str) -> String {
  let mut result = content.to_string();

  // Simple regex-like replacement for <a href="url">text</a>
  // This is a simplified version; a proper implementation would use a parser
  while let Some(start) = result.find("<a href=\"") {
    if let Some(quote_end) = result[start + 9..].find('"') {
      let url_start = start + 9;
      let url_end = url_start + quote_end;
      let url = &result[url_start..url_end].to_string();

      if let Some(close_tag) = result[url_end..].find('>') {
        let text_start = url_end + close_tag + 1;
        if let Some(end_tag) = result[text_start..].find("</a>") {
          let text_end = text_start + end_tag;
          let text = &result[text_start..text_end].to_string();
          let replacement = format!("[{text}]({url})");
          result.replace_range(start..text_end + 4, &replacement);
        } else {
          break;
        }
      } else {
        break;
      }
    } else {
      break;
    }
  }

  result
}

/// Convert lists to Markdown
fn convert_lists(content: &str) -> String {
  let mut result = content.to_string();

  // Unordered lists
  result = result.replace("<ul>", "\n").replace("</ul>", "\n");
  result = result.replace("<li>", "- ").replace("</li>", "\n");

  // Ordered lists (simplified - doesn't handle numbering)
  result = result.replace("<ol>", "\n").replace("</ol>", "\n");

  result
}

/// Convert code blocks to Markdown
fn convert_code_blocks(content: &str) -> String {
  let mut result = content.to_string();

  // Inline code
  result = result.replace("<code>", "`").replace("</code>", "`");

  // Code blocks (simplified)
  result = result.replace("<pre>", "\n```\n").replace("</pre>", "\n```\n");

  result
}

/// Convert images to Markdown
fn convert_images(content: &str) -> String {
  let mut result = content.to_string();

  // ac:image is Confluence's image macro
  // This is a simplified conversion - proper implementation would extract actual
  // image URLs
  result = result.replace("<ac:image>", "![image]");
  result = result.replace("</ac:image>", "");

  result
}

/// Strip common HTML tags that don't have direct Markdown equivalents
fn strip_common_tags(content: &str) -> String {
  let mut result = content.to_string();

  // Paragraph tags (just use blank lines in markdown)
  result = result.replace("<p>", "\n").replace("</p>", "\n");

  // Div tags
  result = result.replace("<div>", "\n").replace("</div>", "\n");

  // Span tags
  result = result.replace("<span>", "").replace("</span>", "");

  // Break tags
  result = result
    .replace("<br>", "\n")
    .replace("<br/>", "\n")
    .replace("<br />", "\n");

  // Table tags (very basic conversion)
  result = result.replace("<table>", "\n").replace("</table>", "\n");
  result = result.replace("<tr>", "| ").replace("</tr>", " |\n");
  result = result.replace("<td>", " ").replace("</td>", " | ");
  result = result.replace("<th>", " ").replace("</th>", " | ");

  result
}

/// Clean up excessive whitespace
fn clean_whitespace(content: &str) -> String {
  let mut result = content.to_string();

  // Replace multiple consecutive newlines with at most 2
  while result.contains("\n\n\n") {
    result = result.replace("\n\n\n", "\n\n");
  }

  // Trim leading/trailing whitespace
  result = result.trim().to_string();

  // Ensure file ends with a newline
  if !result.ends_with('\n') {
    result.push('\n');
  }

  result
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_convert_headings() {
    let input = "<h1>Title</h1><h2>Subtitle</h2>";
    let output = convert_headings(input);
    assert!(output.contains("# Title"));
    assert!(output.contains("## Subtitle"));
  }

  #[test]
  fn test_convert_formatting() {
    let input = "<strong>bold</strong> <em>italic</em> <s>strike</s>";
    let output = convert_formatting(input);
    assert_eq!(output, "**bold** _italic_ ~~strike~~");
  }

  #[test]
  fn test_convert_links() {
    let input = "<a href=\"https://example.com\">Example</a>";
    let output = convert_links(input);
    assert_eq!(output, "[Example](https://example.com)");
  }

  #[test]
  fn test_convert_code() {
    let input = "Some <code>inline code</code> here";
    let output = convert_code_blocks(input);
    assert_eq!(output, "Some `inline code` here");
  }
}
