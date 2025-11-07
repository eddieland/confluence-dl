//! Helpers for parsing Confluence URLs into actionable identifiers.

use anyhow::{Context, Result, anyhow};
use url::Url;

/// Information extracted from a Confluence URL.
#[derive(Debug, Clone)]
pub struct UrlInfo {
  /// Scheme and host of the Confluence instance (e.g., `https://example.atlassian.net`).
  pub base_url: String,
  /// Numeric identifier of the page derived from the URL.
  pub page_id: String,
  /// Optional Confluence space key when the URL encodes one.
  pub space_key: Option<String>,
}

/// Parse a Confluence URL to extract page ID, base URL, and optional space key.
///
/// Supports various Confluence URL formats:
/// - https://example.atlassian.net/wiki/spaces/SPACE/pages/123456/Page+Title
/// - https://example.atlassian.net/wiki/pages/123456
/// - https://example.atlassian.net/pages/123456
///
/// # Arguments
/// * `url` - User-supplied Confluence URL that should resolve to a specific
///   page.
///
/// # Returns
/// Structured [`UrlInfo`] describing the base instance URL, page identifier,
/// and space key if present.
///
/// # Errors
/// Returns an error when the URL is malformed, missing the expected `pages`
/// segment, or contains a non-numeric page ID.
pub fn parse_confluence_url(url: &str) -> Result<UrlInfo> {
  let parsed = Url::parse(url).context("Invalid URL format")?;

  let base_url = format!(
    "{}://{}",
    parsed.scheme(),
    parsed.host_str().context("URL missing host")?
  );

  let path = parsed.path();
  let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

  let page_id_pos = segments
    .iter()
    .position(|&s| s == "pages")
    .context("URL does not contain 'pages' segment")?;

  if page_id_pos + 1 >= segments.len() {
    return Err(anyhow!("URL does not contain page ID after 'pages' segment"));
  }

  let page_id = segments[page_id_pos + 1];

  if !page_id.chars().all(|c| c.is_ascii_digit()) {
    return Err(anyhow!("Page ID is not numeric: {page_id}"));
  }

  let space_key = segments.iter().position(|&s| s == "spaces").and_then(|pos| {
    if pos + 1 < segments.len() && pos + 1 < page_id_pos {
      Some(segments[pos + 1].to_string())
    } else {
      None
    }
  });

  Ok(UrlInfo {
    base_url,
    page_id: page_id.to_string(),
    space_key,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_confluence_url_with_space() {
    let url =
      "https://example.atlassian.net/wiki/spaces/~example-user/pages/229483/Getting+started+in+Confluence+from+Jira";
    let info = parse_confluence_url(url).unwrap();

    assert_eq!(info.base_url, "https://example.atlassian.net");
    assert_eq!(info.page_id, "229483");
    assert_eq!(info.space_key, Some("~example-user".to_string()));
  }

  #[test]
  fn test_parse_confluence_url_without_space() {
    let url = "https://example.atlassian.net/wiki/pages/123456";
    let info = parse_confluence_url(url).unwrap();

    assert_eq!(info.base_url, "https://example.atlassian.net");
    assert_eq!(info.page_id, "123456");
    assert_eq!(info.space_key, None);
  }

  #[test]
  fn test_parse_confluence_url_invalid() {
    let url = "https://example.com/not-a-confluence-url";
    assert!(parse_confluence_url(url).is_err());
  }

  #[test]
  fn test_parse_confluence_url_non_numeric_id() {
    let url = "https://example.atlassian.net/wiki/pages/notanumber";
    assert!(parse_confluence_url(url).is_err());
  }

  #[test]
  fn test_parse_confluence_url_missing_pages_segment() {
    let url = "https://example.atlassian.net/wiki/spaces/SPACE/123456";
    assert!(parse_confluence_url(url).is_err());
  }

  #[test]
  fn test_parse_confluence_url_pages_at_end() {
    let url = "https://example.atlassian.net/wiki/pages";
    let result = parse_confluence_url(url);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not contain page ID"));
  }

  #[test]
  fn test_parse_confluence_url_invalid_scheme() {
    let url = "not-a-url";
    assert!(parse_confluence_url(url).is_err());
  }

  #[test]
  fn test_parse_confluence_url_no_host() {
    let url = "file:///wiki/pages/123";
    assert!(parse_confluence_url(url).is_err());
  }
}
