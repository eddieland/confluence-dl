use roxmltree::Node;

use crate::markdown::MarkdownOptions;
use crate::markdown::utils::{find_child_by_tag, find_child_by_tag_and_attr, get_element_text};

/// Handles Confluence Jira issue macros.
///
/// Supports both single-issue macros (`key` parameter) and JQL-backed issue
/// tables. When the macro references a single issue we render a Markdown link
/// with the optional summary. JQL-based macros fall back to an informational
/// block noting that dynamic content is not exported.
pub(super) fn handle_macro(
  _macro_name: &str,
  element: Node,
  _convert_node: &dyn Fn(Node) -> String,
  _options: &MarkdownOptions,
) -> Option<String> {
  if let Some(key) = parameter_value(element, "key") {
    return Some(render_single_issue(element, &key));
  }

  let message = parameter_value(element, "jql")
    .or_else(|| {
      find_child_by_tag(element, "ac:plain-text-body")
        .map(get_element_text)
        .and_then(normalize_text)
    })
    .map(|query| format!("Jira issues macro (JQL: {query}). Dynamic content not exported."))
    .unwrap_or_else(|| "Jira issues macro (dynamic content not exported).".to_string());

  Some(format!("\n> _{message}_\n\n"))
}

/// Renders a single Jira issue reference into Markdown.
fn render_single_issue(element: Node, key: &str) -> String {
  let trimmed_key = key.trim();
  if trimmed_key.is_empty() {
    return String::new();
  }

  let summary = parameter_value(element, "summary");
  let server = parameter_value(element, "server")
    .or_else(|| parameter_value(element, "baseurl"))
    .or_else(|| parameter_value(element, "base-url"));

  let link = server
    .and_then(normalize_text)
    .map(|server_url| {
      let base = server_url.trim_end_matches('/');
      format!("{base}/browse/{trimmed_key}")
    })
    .unwrap_or_default();

  let mut result = if link.is_empty() {
    trimmed_key.to_string()
  } else {
    format!("[{trimmed_key}]({link})")
  };

  if let Some(summary) = summary.and_then(normalize_text)
    && !summary.is_empty()
  {
    result.push_str(": ");
    result.push_str(&summary);
  }

  result
}

/// Extracts and normalizes a parameter value.
fn parameter_value(element: Node, name: &str) -> Option<String> {
  find_child_by_tag_and_attr(element, "ac:parameter", "ac:name", name)
    .map(get_element_text)
    .and_then(normalize_text)
}

/// Trims whitespace and collapses empty strings to `None`.
fn normalize_text(value: String) -> Option<String> {
  let trimmed = value.trim();
  if trimmed.is_empty() {
    None
  } else {
    Some(trimmed.to_string())
  }
}

#[cfg(test)]
mod tests {
  use roxmltree::Document;

  use super::*;
  use crate::markdown::MarkdownOptions;
  use crate::markdown::utils::{matches_tag, wrap_with_namespaces};

  #[test]
  fn test_render_single_issue_with_link_and_summary() {
    let input = r#"
      <ac:structured-macro ac:name="jira">
        <ac:parameter ac:name="key">ABC-123</ac:parameter>
        <ac:parameter ac:name="server">https://jira.example.com/</ac:parameter>
        <ac:parameter ac:name="summary">Fix the login flow</ac:parameter>
      </ac:structured-macro>
    "#;

    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let macro_node = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:structured-macro"))
      .unwrap();

    let output = handle_macro("jira", macro_node, &|_| String::new(), &MarkdownOptions::default());
    assert_eq!(
      output,
      Some("[ABC-123](https://jira.example.com/browse/ABC-123): Fix the login flow".to_string())
    );
  }

  #[test]
  fn test_render_single_issue_without_server() {
    let input = r#"
      <ac:structured-macro ac:name="jira">
        <ac:parameter ac:name="key">ABC-123</ac:parameter>
      </ac:structured-macro>
    "#;

    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let macro_node = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:structured-macro"))
      .unwrap();

    let output = handle_macro("jira", macro_node, &|_| String::new(), &MarkdownOptions::default());
    assert_eq!(output, Some("ABC-123".to_string()));
  }

  #[test]
  fn test_render_jql_macro_message() {
    let input = r#"
      <ac:structured-macro ac:name="jira">
        <ac:plain-text-body><![CDATA[project = ABC ORDER BY created DESC]]></ac:plain-text-body>
      </ac:structured-macro>
    "#;

    let wrapped = wrap_with_namespaces(input);
    let document = Document::parse(&wrapped).unwrap();
    let macro_node = document
      .descendants()
      .find(|node| matches_tag(*node, "ac:structured-macro"))
      .unwrap();

    let output = handle_macro("jira", macro_node, &|_| String::new(), &MarkdownOptions::default());
    assert_eq!(
      output,
      Some(
        "\n> _Jira issues macro (JQL: project = ABC ORDER BY created DESC). Dynamic content not exported._\n\n"
          .to_string()
      )
    );
  }
}
