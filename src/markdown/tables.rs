//! HTML table to Markdown table conversion.
//!
//! Converts Confluence HTML tables to properly formatted Markdown tables.

use roxmltree::Node;

use super::utils::{get_element_text, matches_tag};

/// Convert an HTML table element into Markdown table syntax.
///
/// Handles tables with `thead`, `tbody`, `tfoot` sections, or direct `tr`
/// children. Automatically aligns columns and formats with consistent spacing.
///
/// # Arguments
/// * `element` - The `<table>` node whose content should be rendered.
///
/// # Returns
/// A Markdown fragment beginning with a newline that contains the formatted
/// table, or an empty string when the table has no meaningful content.
pub fn convert_table_to_markdown(element: Node) -> String {
  let mut rows: Vec<Vec<String>> = Vec::new();

  // Collect all <tr> elements from the table
  // In HTML tables, rows are typically wrapped in <tbody>, <thead>, or <tfoot>
  let mut tr_elements = Vec::new();

  // Check for direct <tr> children (edge case) or table section elements
  for child in element.children() {
    if matches_tag(child, "tr") {
      tr_elements.push(child);
    } else if matches_tag(child, "tbody") || matches_tag(child, "thead") || matches_tag(child, "tfoot") {
      // Collect <tr> elements from table sections
      for tr in child.children().filter(|n| matches_tag(*n, "tr")) {
        tr_elements.push(tr);
      }
    }
  }

  // Process all collected rows
  for tr in tr_elements {
    let mut cells: Vec<String> = Vec::new();

    for cell in tr
      .children()
      .filter(|child| matches_tag(*child, "th") || matches_tag(*child, "td"))
    {
      let text = get_element_text(cell)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
      cells.push(text);
    }

    if !cells.is_empty() {
      rows.push(cells);
    }
  }

  render_markdown_table(rows).unwrap_or_default()
}

/// Pretty-print Markdown tables with aligned columns.
///
/// Accepts a collection of rows (each a vector of cell strings) and formats
/// them into a Markdown table with padded columns. The first row is treated as
/// the header.
///
/// # Arguments
/// * `rows` - Table rows in display order.
///
/// # Returns
/// `Some(String)` containing the rendered Markdown table (surrounded by leading
/// and trailing newlines) or `None` when the supplied rows are insufficient to
/// produce a valid table.
pub fn render_markdown_table(mut rows: Vec<Vec<String>>) -> Option<String> {
  if rows.is_empty() {
    return None;
  }

  let column_count = rows.iter().map(|row| row.len()).max()?;
  if column_count == 0 {
    return None;
  }

  for row in &mut rows {
    row.resize(column_count, String::new());
  }

  let mut column_widths = vec![0; column_count];
  for row in &rows {
    for (index, cell) in row.iter().enumerate() {
      column_widths[index] = column_widths[index].max(cell.len());
    }
  }

  let mut result = String::new();
  result.push('\n');

  if let Some(first_row) = rows.first() {
    result.push_str(&format_row(first_row, &column_widths));

    result.push('|');
    for width in &column_widths {
      let dash_count = (*width).max(3);
      result.push(' ');
      result.push_str(&"-".repeat(dash_count));
      result.push(' ');
      result.push('|');
    }
    result.push('\n');
  }

  for row in rows.iter().skip(1) {
    result.push_str(&format_row(row, &column_widths));
  }

  result.push('\n');
  Some(result)
}

/// Format a single table row with proper column alignment.
///
/// # Arguments
/// * `row` - The cell values to render, in column order.
/// * `column_widths` - Precomputed column widths used to pad each cell.
///
/// # Returns
/// A Markdown table row ending with a newline.
fn format_row(row: &[String], column_widths: &[usize]) -> String {
  let mut line = String::new();
  line.push('|');

  for (cell, width) in row.iter().zip(column_widths) {
    line.push(' ');
    line.push_str(cell);
    if *width > cell.len() {
      line.push_str(&" ".repeat(width - cell.len()));
    }
    line.push(' ');
    line.push('|');
  }

  line.push('\n');
  line
}

#[cfg(test)]
mod tests {
  use roxmltree::Document;

  use super::*;
  use crate::markdown::utils::wrap_with_namespaces;

  #[test]
  fn test_convert_table() {
    let input = r#"
      <table>
        <tr><th>Header 1</th><th>Header 2</th></tr>
        <tr><td>Row 1 Col 1</td><td>Row 1 Col 2</td></tr>
        <tr><td>Row 2 Col 1</td><td>Row 2 Col 2</td></tr>
      </table>
    "#;
    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let table = document.descendants().find(|node| matches_tag(*node, "table")).unwrap();
    let output = convert_table_to_markdown(table);
    insta::assert_snapshot!(output, @r###"
    | Header 1    | Header 2    |
    | ----------- | ----------- |
    | Row 1 Col 1 | Row 1 Col 2 |
    | Row 2 Col 1 | Row 2 Col 2 |
    "###);
  }

  #[test]
  fn test_convert_table_empty() {
    let input = "<table></table>";
    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let table = document.descendants().find(|node| matches_tag(*node, "table")).unwrap();
    let output = convert_table_to_markdown(table);
    // Empty table should produce minimal output
    assert!(!output.contains("|"));
  }
}
