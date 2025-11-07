//! Basic HTML element to Markdown converters.
//!
//! Handles conversion of standard HTML elements like headings, paragraphs,
//! links, lists, code blocks, and formatting.

use roxmltree::Node;
use tracing::debug;

use super::MarkdownOptions;
use super::emoji::{convert_emoji_to_markdown, convert_span_emoji};
use super::html_entities::decode_html_entities;
use super::macros::{
  convert_adf_extension_to_markdown, convert_confluence_link_to_markdown, convert_image_to_markdown,
  convert_macro_to_markdown, convert_task_list_to_markdown, render_admonition_block,
};
use super::tables::{convert_table_to_markdown, render_markdown_table};
use super::utils::{get_attribute, get_element_text, matches_tag};

/// Checks whether a line appears to start with a Markdown list marker.
///
/// This guards against accidentally duplicating list prefixes when nested
/// content already includes list syntax.
///
/// # Arguments
/// * `line` - Raw line text to examine for leading list indicators.
///
/// # Returns
/// `true` when the line begins with a Markdown unordered or ordered list
/// marker, otherwise `false`.
fn looks_like_list_marker(line: &str) -> bool {
  let trimmed = line.trim_start();

  if trimmed.starts_with(['-', '*', '+']) {
    return trimmed.len() > 1 && trimmed.as_bytes()[1] == b' ';
  }

  let mut chars = trimmed.chars();
  let mut saw_digit = false;

  while let Some(ch) = chars.next() {
    if ch.is_ascii_digit() {
      saw_digit = true;
      continue;
    }

    if ch == '.' {
      return saw_digit && matches!(chars.next(), Some(' '));
    }

    break;
  }

  false
}

/// Formats a converted list item, preserving nested list structure.
///
/// The helper ensures that existing list markers remain untouched while
/// normalizing indentation for newly created prefixes.
///
/// # Arguments
/// * `item` - Converted Markdown representing the list item's body.
/// * `prefix` - The list marker (e.g., `"- "` or `"1. "`) applied to the first
///   visible line.
///
/// # Returns
/// Rendered Markdown snippet for the list item with normalized indentation.
fn format_list_item(item: &str, prefix: &str) -> String {
  let mut formatted = String::new();
  let lines = item.trim_end().lines();
  let indentation = " ".repeat(prefix.chars().count());
  let mut wrote_first_line = false;

  for line in lines {
    if !wrote_first_line {
      if line.trim().is_empty() {
        continue;
      }

      let line_content = line.trim_start();

      if looks_like_list_marker(line_content) {
        formatted.push_str(prefix.trim_end());
        formatted.push('\n');
        formatted.push_str(&indentation);
        formatted.push_str(line_content);
        formatted.push('\n');
      } else {
        formatted.push_str(prefix);
        formatted.push_str(line_content);
        formatted.push('\n');
      }

      wrote_first_line = true;
    } else if line.trim().is_empty() {
      formatted.push('\n');
    } else {
      formatted.push_str(&indentation);
      formatted.push_str(line);
      formatted.push('\n');
    }
  }

  if !wrote_first_line {
    formatted.push_str(prefix.trim_end());
    formatted.push('\n');
  }

  formatted
}

fn convert_layout_cell(cell: Node, options: &MarkdownOptions) -> String {
  let mut content = String::new();

  for child in cell.children() {
    content.push_str(&convert_node_to_markdown(child, options));
  }

  content
}

fn convert_legacy_admonition_block(node: Node, options: &MarkdownOptions, heading: &str) -> String {
  let body = node
    .children()
    .find(|child| matches_tag(*child, "ac:rich-text-body"))
    .map(|body| convert_node_to_markdown(body, options))
    .unwrap_or_else(|| get_element_text(node));

  render_admonition_block(heading, body.trim())
}

fn convert_layout_section(section: Node, options: &MarkdownOptions) -> String {
  let mut content = String::new();

  for child in section.children() {
    content.push_str(&convert_node_to_markdown(child, options));
  }

  content
}

fn sanitize_layout_cell_content(content: &str) -> String {
  let trimmed = content.trim();

  if trimmed.is_empty() {
    return String::new();
  }

  trimmed
    .replace('|', "\\|")
    .split('\n')
    .map(str::trim_end)
    .collect::<Vec<_>>()
    .join("<br />")
}

fn layout_cell_contains_block_markdown(content: &str) -> bool {
  let trimmed = content.trim();

  if trimmed.is_empty() {
    return false;
  }

  if trimmed.contains("```") || trimmed.contains("\n\n") {
    return true;
  }

  trimmed.lines().any(|line| {
    let stripped = line.trim_start();
    stripped.starts_with('>') || stripped.starts_with('#') || looks_like_list_marker(stripped)
  })
}

fn convert_layout_to_markdown(layout: Node, options: &MarkdownOptions) -> String {
  let mut rows: Vec<Vec<String>> = Vec::new();
  let mut max_columns = 0;
  let mut has_block_markdown = false;

  for section in layout.children().filter(|n| matches_tag(*n, "ac:layout-section")) {
    let mut cells = Vec::new();

    for cell in section.children().filter(|n| matches_tag(*n, "ac:layout-cell")) {
      let cell_content = convert_layout_cell(cell, options);
      has_block_markdown |= layout_cell_contains_block_markdown(&cell_content);
      cells.push(cell_content);
    }

    if !cells.is_empty() {
      max_columns = max_columns.max(cells.len());
      rows.push(cells);
    }
  }

  if rows.is_empty() {
    return convert_layout_section(layout, options);
  }

  if has_block_markdown {
    return convert_layout_section(layout, options);
  }

  for row in &mut rows {
    if row.len() < max_columns {
      row.resize_with(max_columns, String::new);
    }
  }

  let mut table_rows = Vec::with_capacity(rows.len() + 1);
  table_rows.push(vec![String::new(); max_columns]);

  for row in rows {
    let sanitized: Vec<String> = row
      .into_iter()
      .map(|cell| sanitize_layout_cell_content(&cell))
      .collect();
    table_rows.push(sanitized);
  }

  render_markdown_table(table_rows, options.compact_tables).unwrap_or_else(|| convert_layout_section(layout, options))
}

fn render_styled_span(node: Node, options: &MarkdownOptions) -> Option<String> {
  let style = collect_span_color_styles(node)?;

  let mut content = String::new();
  for child in node.children() {
    match child.node_type() {
      roxmltree::NodeType::Text => {
        if let Some(text) = child.text() {
          content.push_str(&decode_html_entities(text));
        }
      }
      _ => content.push_str(&convert_node_to_markdown(child, options)),
    }
  }

  if content.trim().is_empty() {
    return None;
  }

  Some(format!("<span style=\"{style}\">{content}</span>"))
}

fn collect_span_color_styles(node: Node) -> Option<String> {
  let color = extract_style_color(node)
    .or_else(|| get_attribute(node, "data-color").and_then(|value| sanitize_css_value(&value)));

  let background = extract_style_background(node)
    .or_else(|| get_attribute(node, "data-background-color").and_then(|value| sanitize_css_value(&value)));

  if color.is_none() && background.is_none() {
    return None;
  }

  let mut declarations = Vec::new();
  if let Some(color) = color {
    declarations.push(format!("color: {color}"));
  }
  if let Some(background) = background {
    declarations.push(format!("background-color: {background}"));
  }

  Some(declarations.join("; "))
}

fn extract_style_color(node: Node) -> Option<String> {
  get_attribute(node, "style").and_then(|style| extract_style_property(&style, "color"))
}

fn extract_style_background(node: Node) -> Option<String> {
  get_attribute(node, "style").and_then(|style| extract_style_property(&style, "background-color"))
}

fn extract_style_property(style_attr: &str, property: &str) -> Option<String> {
  style_attr.split(';').find_map(|declaration| {
    let (name, value) = declaration.split_once(':')?;
    if name.trim().eq_ignore_ascii_case(property) {
      sanitize_css_value(value)
    } else {
      None
    }
  })
}

fn sanitize_css_value(raw_value: &str) -> Option<String> {
  let trimmed = raw_value.trim();
  if trimmed.is_empty() {
    return None;
  }

  let without_important = trimmed
    .strip_suffix("!important")
    .map(|value| value.trim_end())
    .unwrap_or(trimmed);

  if without_important.is_empty() {
    return None;
  }

  if without_important.chars().all(|ch| {
    ch.is_ascii_alphanumeric() || matches!(ch, '#' | '(' | ')' | ',' | '.' | ' ' | '/' | '%' | '-' | '+' | '\'')
  }) {
    Some(without_important.to_string())
  } else {
    None
  }
}

/// Converts an element and its children to Markdown recursively.
///
/// # Arguments
/// * `node` - Root node whose descendants should be rendered.
/// * `options` - Conversion behaviour flags that control optional features,
///   such as anchor preservation.
///
/// # Returns
/// A Markdown string representing the element and its descendants.
pub fn convert_node_to_markdown(node: Node, options: &MarkdownOptions) -> String {
  let mut result = String::new();

  for child in node.children() {
    match child.node_type() {
      roxmltree::NodeType::Element => {
        let tag = child.tag_name();
        let local_name = tag.name();

        match local_name {
          // Headings
          "h1" => result.push_str(&format!("\n# {}\n\n", convert_node_to_markdown(child, options).trim())),
          "h2" => result.push_str(&format!("\n## {}\n\n", convert_node_to_markdown(child, options).trim())),
          "h3" => result.push_str(&format!(
            "\n### {}\n\n",
            convert_node_to_markdown(child, options).trim()
          )),
          "h4" => result.push_str(&format!(
            "\n#### {}\n\n",
            convert_node_to_markdown(child, options).trim()
          )),
          "h5" => result.push_str(&format!(
            "\n##### {}\n\n",
            convert_node_to_markdown(child, options).trim()
          )),
          "h6" => result.push_str(&format!(
            "\n###### {}\n\n",
            convert_node_to_markdown(child, options).trim()
          )),

          // Paragraphs
          "p" => {
            let content = convert_node_to_markdown(child, options);
            let trimmed = content.trim();
            if !trimmed.is_empty() {
              result.push_str(&format!("{trimmed}\n\n"));
            }
          }

          // Text formatting
          "strong" | "b" => result.push_str(&format!("**{}**", convert_node_to_markdown(child, options))),
          "em" | "i" => result.push_str(&format!("_{}_", convert_node_to_markdown(child, options))),
          "u" => result.push_str(&format!("_{}_", convert_node_to_markdown(child, options))),
          "s" | "del" => result.push_str(&format!("~~{}~~", convert_node_to_markdown(child, options))),
          "code" => result.push_str(&format!("`{}`", convert_node_to_markdown(child, options))),

          // Lists
          "ul" => {
            result.push('\n');
            for li in child.children().filter(|n| matches_tag(*n, "li")) {
              let item = convert_node_to_markdown(li, options);
              result.push_str(&format_list_item(&item, "- "));
            }
            result.push('\n');
          }
          "ol" => {
            result.push('\n');
            for (index, li) in child.children().filter(|n| matches_tag(*n, "li")).enumerate() {
              let item = convert_node_to_markdown(li, options);
              let prefix = format!("{}. ", index + 1);
              result.push_str(&format_list_item(&item, &prefix));
            }
            result.push('\n');
          }

          // Links
          "a" => {
            let text = convert_node_to_markdown(child, options);
            let href = get_attribute(child, "href").unwrap_or_default();
            result.push_str(&format!("[{}]({})", text.trim(), href));
          }

          // Line breaks and horizontal rules
          "br" => result.push('\n'),
          "hr" => result.push_str("\n---\n\n"),

          // Code blocks
          "pre" => {
            let code = get_element_text(child);
            result.push_str(&format!("\n```\n{}\n```\n\n", code.trim()));
          }

          // Tables
          "table" => result.push_str(&convert_table_to_markdown(child, options)),

          // Confluence-specific elements
          "link" if matches_tag(child, "ac:link") => {
            result.push_str(&convert_confluence_link_to_markdown(child));
          }
          "note" if matches_tag(child, "ac:note") => {
            result.push_str(&convert_legacy_admonition_block(child, options, "Note"));
          }
          "info" if matches_tag(child, "ac:info") => {
            result.push_str(&convert_legacy_admonition_block(child, options, "Info"));
          }
          "tip" if matches_tag(child, "ac:tip") => {
            result.push_str(&convert_legacy_admonition_block(child, options, "Tip"));
          }
          "warning" if matches_tag(child, "ac:warning") => {
            result.push_str(&convert_legacy_admonition_block(child, options, "Warning"));
          }
          "structured-macro" if matches_tag(child, "ac:structured-macro") => {
            result.push_str(&convert_macro_to_markdown(
              child,
              &|node| convert_node_to_markdown(node, options),
              options,
            ));
          }
          "task-list" if matches_tag(child, "ac:task-list") => {
            result.push_str(&convert_task_list_to_markdown(child));
          }
          "image" if matches_tag(child, "ac:image") => {
            result.push_str(&convert_image_to_markdown(child));
          }
          "adf-extension" if matches_tag(child, "ac:adf-extension") => {
            result.push_str(&convert_adf_extension_to_markdown(child, &|node| {
              convert_node_to_markdown(node, options)
            }));
          }

          // Layout elements
          "layout" if matches_tag(child, "ac:layout") => {
            result.push_str(&convert_layout_to_markdown(child, options));
          }
          "layout-section" if matches_tag(child, "ac:layout-section") => {
            result.push_str(&convert_layout_section(child, options));
          }
          "layout-cell" if matches_tag(child, "ac:layout-cell") => {
            result.push_str(&convert_layout_cell(child, options));
          }
          "rich-text-body" if matches_tag(child, "ac:rich-text-body") => {
            result.push_str(&convert_node_to_markdown(child, options));
          }

          // Skip these internal elements
          "url" if matches_tag(child, "ri:url") => {}
          "parameter" if matches_tag(child, "ac:parameter") => {}
          "task-id" if matches_tag(child, "ac:task-id") => {}
          "task-status" if matches_tag(child, "ac:task-status") => {}
          "task-body" if matches_tag(child, "ac:task-body") => {
            result.push_str(&get_element_text(child));
          }
          "placeholder" if matches_tag(child, "ac:placeholder") => {}

          // Time elements - prefer visible text, fall back to datetime attribute
          "time" => {
            let text = get_element_text(child);
            if !text.trim().is_empty() {
              result.push_str(&text);
            } else if let Some(datetime) = get_attribute(child, "datetime") {
              result.push_str(&datetime);
            }
          }

          // Span elements (check for emoji metadata)
          "span" => {
            if let Some(emoji) = convert_span_emoji(child) {
              result.push_str(&emoji);
            } else if let Some(styled) = render_styled_span(child, options) {
              result.push_str(&styled);
            } else {
              result.push_str(&convert_node_to_markdown(child, options));
            }
          }

          // Emoji elements
          "emoji" if matches_tag(child, "ac:emoji") => {
            result.push_str(&convert_emoji_to_markdown(child));
          }
          "emoticon" if matches_tag(child, "ac:emoticon") => {
            result.push_str(&convert_emoji_to_markdown(child));
          }

          // Unknown elements - extract content
          _ => {
            let debug_name = super::utils::qualified_tag_name(child);
            debug!("Unknown tag: {debug_name}");
            result.push_str(&convert_node_to_markdown(child, options));
          }
        }
      }
      roxmltree::NodeType::Text => {
        if let Some(text) = child.text() {
          let decoded = decode_html_entities(text);
          result.push_str(&decoded);
        }
      }
      _ => {}
    }
  }

  result
}

#[cfg(test)]
mod tests {
  use super::*;

  fn convert_to_markdown(input: &str) -> String {
    use roxmltree::Document;

    use crate::markdown::MarkdownOptions;
    use crate::markdown::utils::wrap_with_namespaces;

    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let markdown = convert_node_to_markdown(document.root_element(), &MarkdownOptions::default());
    crate::markdown::utils::clean_markdown(&markdown)
  }

  #[test]
  fn test_convert_headings() {
    let input = "<h1>Title</h1><h2>Subtitle</h2>";
    let output = convert_to_markdown(input);
    assert!(output.contains("# Title"));
    assert!(output.contains("## Subtitle"));
  }

  #[test]
  fn test_convert_formatting() {
    let input = "<p><strong>bold</strong> <em>italic</em> <s>strike</s></p>";
    let output = convert_to_markdown(input);
    assert!(output.contains("**bold**"));
    assert!(output.contains("_italic_"));
    assert!(output.contains("~~strike~~"));
  }

  #[test]
  fn test_convert_links() {
    let input = r#"<a href="https://example.com">Example</a>"#;
    let output = convert_to_markdown(input);
    assert!(output.contains("[Example](https://example.com)"));
  }

  #[test]
  fn test_convert_time_with_text_content() {
    let input = "<p>Meeting at <time datetime=\"2025-10-07\">October 7, 2025</time></p>";
    let output = convert_to_markdown(input);
    assert!(output.contains("Meeting at October 7, 2025"));
  }

  #[test]
  fn test_convert_time_with_datetime_attribute() {
    let input = "<p>Meeting at <time datetime=\"2025-10-07\" /></p>";
    let output = convert_to_markdown(input);
    assert!(output.contains("Meeting at 2025-10-07"));
  }

  #[test]
  fn test_convert_lists() {
    let input = r#"
      <ul>
        <li>Item 1</li>
        <li>Item 2</li>
      </ul>
      <ol>
        <li>First</li>
        <li>Second</li>
      </ol>
    "#;
    let result = convert_to_markdown(input);
    // Multiline inline snapshots with funky spacing confuse rustfmt, so keep this
    // escaped.
    let output = result.escape_default();
    insta::assert_snapshot!(output, @r"- Item 1\n- Item 2\n\n      \n1. First\n2. Second\n");
  }

  #[test]
  fn test_convert_nested_lists() {
    let input = r#"
      <ul>
        <li>Parent
          <ul>
            <li>Child</li>
            <li>Nested
              <ul>
                <li>Grandchild</li>
              </ul>
            </li>
          </ul>
        </li>
      </ul>
    "#;

    let result = convert_to_markdown(input);
    let output = result.escape_default();

    insta::assert_snapshot!(
      output,
      @r"- Parent\n\n  - Child\n  - Nested\n\n    - Grandchild\n"
    );
  }

  #[test]
  fn test_convert_code_block() {
    let input = "<pre>function test() {\n  return 42;\n}</pre>";
    let output = convert_to_markdown(input);
    assert!(output.contains("```"));
    assert!(output.contains("function test()"));
  }

  #[test]
  fn test_convert_inline_code() {
    let input = "<p>Use <code>git commit</code> to save</p>";
    let output = convert_to_markdown(input);
    assert!(output.contains("`git commit`"));
  }

  #[test]
  fn test_convert_adf_decision_list_placeholder_is_ignored() {
    let input = r#"
      <ac:adf-extension>
        <ac:adf-node type="decision-list">
          <ac:adf-attribute key="local-id">5a86a7de</ac:adf-attribute>
          <ac:adf-node type="decision-item">
            <ac:adf-attribute key="local-id">51a042d6</ac:adf-attribute>
            <ac:adf-attribute key="state">DECIDED</ac:adf-attribute>
          </ac:adf-node>
        </ac:adf-node>
      </ac:adf-extension>
  "#;

    let output = convert_to_markdown(input);
    assert!(output.trim().is_empty());
  }

  #[test]
  fn test_convert_adf_decision_list_with_content() {
    let input = r#"
      <ac:adf-extension>
        <ac:adf-node type="decision-list">
          <ac:adf-node type="decision-item">
            <ac:adf-attribute key="state">DECIDED</ac:adf-attribute>
            <ac:adf-attribute key="owner">Alice</ac:adf-attribute>
            <ac:adf-node type="paragraph">
              <ac:adf-leaf type="text">
                <ac:adf-attribute key="text">Adopt Rust for CLI tooling</ac:adf-attribute>
              </ac:adf-leaf>
            </ac:adf-node>
            <ac:adf-node type="paragraph">
              <ac:adf-leaf type="text">
                <ac:adf-attribute key="text">Roll out to all teams in Q3.</ac:adf-attribute>
              </ac:adf-leaf>
            </ac:adf-node>
          </ac:adf-node>
        </ac:adf-node>
      </ac:adf-extension>
    "#;

    let output = convert_to_markdown(input);
    assert!(output.contains("- **Decision:** Adopt Rust for CLI tooling"));
    assert!(output.contains("Status: DECIDED"));
    assert!(output.contains("Owner: Alice"));
    assert!(output.contains("Roll out to all teams in Q3."));
  }

  #[test]
  fn test_convert_adf_decision_list_with_plain_content() {
    let input = r#"
      <ac:adf-extension>
        <ac:adf-node type="decision-list">
          <ac:adf-node type="decision-item">
            <ac:adf-content>Hotel? Trivago</ac:adf-content>
          </ac:adf-node>
        </ac:adf-node>
      </ac:adf-extension>
    "#;

    let output = convert_to_markdown(input);
    assert!(output.contains("- **Decision:** Hotel? Trivago"));
  }

  #[test]
  fn test_convert_layout_to_markdown_single_row() {
    let input = r#"
      <ac:layout>
        <ac:layout-section>
          <ac:layout-cell>
            <p>Left column</p>
          </ac:layout-cell>
          <ac:layout-cell>
            <p>Right column</p>
          </ac:layout-cell>
        </ac:layout-section>
      </ac:layout>
    "#;

    let output = convert_to_markdown(input);
    assert_eq!(
      output,
      "|             |              |\n| ----------- | ------------ |\n| Left column | Right column |\n"
    );
  }

  #[test]
  fn test_convert_layout_to_markdown_multiple_rows() {
    let input = r#"
      <ac:layout>
        <ac:layout-section>
          <ac:layout-cell><p>One</p></ac:layout-cell>
          <ac:layout-cell><p>Two</p></ac:layout-cell>
        </ac:layout-section>
        <ac:layout-section>
          <ac:layout-cell><p>Three</p></ac:layout-cell>
          <ac:layout-cell><p>Four</p></ac:layout-cell>
        </ac:layout-section>
      </ac:layout>
    "#;

    let output = convert_to_markdown(input);
    assert_eq!(
      output,
      "|       |      |\n| ----- | ---- |\n| One   | Two  |\n| Three | Four |\n"
    );
  }

  #[test]
  fn test_convert_layout_escapes_table_characters() {
    let input = r#"
      <ac:layout>
        <ac:layout-section>
          <ac:layout-cell>
            <p>Pipe | Value</p>
          </ac:layout-cell>
          <ac:layout-cell>
            <p>Multi
line</p>
          </ac:layout-cell>
        </ac:layout-section>
      </ac:layout>
    "#;

    let output = convert_to_markdown(input);
    assert_eq!(
      output,
      "|               |                 |\n| ------------- | --------------- |\n| Pipe \\| Value | Multi<br />line |\n"
    );
  }

  #[test]
  fn test_span_color_style_is_preserved() {
    let input = r#"<p><span style="color: rgb(97, 189, 109);">Green</span></p>"#;
    let output = convert_to_markdown(input);
    assert!(
      output.contains(r#"<span style="color: rgb(97, 189, 109)">Green</span>"#),
      "{output:?}"
    );
  }

  #[test]
  fn test_span_background_style_is_preserved() {
    let input = r#"<p><span style="background-color: #ffeeaa;">Highlight</span></p>"#;
    let output = convert_to_markdown(input);
    assert!(
      output.contains(r#"<span style="background-color: #ffeeaa">Highlight</span>"#),
      "{output:?}"
    );
  }

  #[test]
  fn test_span_data_color_attributes_are_preserved() {
    let input = r##"<p><span data-color="#ff0000" data-background-color="#ffeeee">Alert</span></p>"##;
    let output = convert_to_markdown(input);
    assert!(
      output.contains(r#"<span style="color: #ff0000; background-color: #ffeeee">Alert</span>"#),
      "{output:?}"
    );
  }
}
