//! Confluence API client for fetching pages and content.
//!
//! This module provides a client for interacting with the Confluence REST API,
//! including authentication, page fetching, and content retrieval.

use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{Deserialize, Serialize};
use url::Url;

/// Confluence API client
pub struct ConfluenceClient {
  base_url: String,
  username: String,
  token: String,
  client: reqwest::blocking::Client,
}

/// Confluence page metadata and content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
  pub id: String,
  pub title: String,
  #[serde(rename = "type")]
  pub page_type: String,
  pub status: String,
  pub body: Option<PageBody>,
  pub space: Option<PageSpace>,
  #[serde(rename = "_links")]
  pub links: Option<PageLinks>,
}

/// Page body content in various formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageBody {
  pub storage: Option<StorageFormat>,
  pub view: Option<ViewFormat>,
}

/// Storage format (Confluence's internal format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageFormat {
  pub value: String,
  pub representation: String,
}

/// View format (rendered HTML)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewFormat {
  pub value: String,
  pub representation: String,
}

/// Space information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSpace {
  pub key: String,
  pub name: String,
  #[serde(rename = "type")]
  pub space_type: String,
}

/// Page links
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageLinks {
  #[serde(rename = "webui")]
  pub web_ui: Option<String>,
  #[serde(rename = "self")]
  pub self_link: Option<String>,
}

/// Information extracted from a Confluence URL
#[derive(Debug, Clone)]
pub struct UrlInfo {
  pub base_url: String,
  pub page_id: String,
  pub space_key: Option<String>,
}

impl ConfluenceClient {
  /// Create a new Confluence client
  ///
  /// # Arguments
  /// * `base_url` - The base URL of the Confluence instance (e.g., https://example.atlassian.net)
  /// * `username` - The user's email address
  /// * `token` - The API token
  /// * `timeout_secs` - Request timeout in seconds
  pub fn new(
    base_url: impl Into<String>,
    username: impl Into<String>,
    token: impl Into<String>,
    timeout_secs: u64,
  ) -> Result<Self> {
    let base_url = base_url.into();
    let username = username.into();
    let token = token.into();

    // Normalize base URL (remove trailing slash)
    let base_url = base_url.trim_end_matches('/').to_string();

    // Create HTTP client with timeout
    let client = reqwest::blocking::Client::builder()
      .timeout(Duration::from_secs(timeout_secs))
      .user_agent(format!(
        "confluence-dl/{} ({})",
        env!("CARGO_PKG_VERSION"),
        env!("TARGET")
      ))
      .build()
      .context("Failed to create HTTP client")?;

    Ok(Self {
      base_url,
      username,
      token,
      client,
    })
  }

  /// Get the authorization header value (Basic auth)
  fn auth_header(&self) -> String {
    let credentials = format!("{}:{}", self.username, self.token);
    format!("Basic {}", BASE64.encode(credentials.as_bytes()))
  }

  /// Fetch a page by ID
  ///
  /// # Arguments
  /// * `page_id` - The numeric page ID
  pub fn get_page(&self, page_id: &str) -> Result<Page> {
    let url = format!(
      "{}/wiki/rest/api/content/{}?expand=body.storage,body.view,space",
      self.base_url, page_id
    );

    let response = self
      .client
      .get(&url)
      .header("Authorization", self.auth_header())
      .header("Accept", "application/json")
      .send()
      .context("Failed to send request to Confluence API")?;

    if !response.status().is_success() {
      let status = response.status();
      let error_text = response.text().unwrap_or_else(|_| String::from("(no error details)"));
      return Err(anyhow!("Confluence API returned error {status}: {error_text}"));
    }

    let page: Page = response
      .json()
      .context("Failed to parse page response from Confluence API")?;

    Ok(page)
  }

  /// Test authentication by fetching current user info
  #[allow(dead_code)]
  pub fn test_auth(&self) -> Result<()> {
    let url = format!("{}/wiki/rest/api/user/current", self.base_url);

    let response = self
      .client
      .get(&url)
      .header("Authorization", self.auth_header())
      .header("Accept", "application/json")
      .send()
      .context("Failed to send authentication test request")?;

    if !response.status().is_success() {
      let status = response.status();
      return Err(anyhow!("Authentication failed with status: {status}"));
    }

    Ok(())
  }
}

/// Parse a Confluence URL to extract page ID and other information
///
/// Supports various Confluence URL formats:
/// - https://example.atlassian.net/wiki/spaces/SPACE/pages/123456/Page+Title
/// - https://example.atlassian.net/wiki/pages/123456
/// - https://example.atlassian.net/pages/123456
pub fn parse_confluence_url(url: &str) -> Result<UrlInfo> {
  let parsed = Url::parse(url).context("Invalid URL format")?;

  // Extract base URL (scheme + host)
  let base_url = format!(
    "{}://{}",
    parsed.scheme(),
    parsed.host_str().context("URL missing host")?
  );

  // Extract path segments
  let path = parsed.path();
  let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

  // Look for "pages" segment followed by a numeric ID
  let page_id_pos = segments
    .iter()
    .position(|&s| s == "pages")
    .context("URL does not contain 'pages' segment")?;

  if page_id_pos + 1 >= segments.len() {
    return Err(anyhow!("URL does not contain page ID after 'pages' segment"));
  }

  let page_id = segments[page_id_pos + 1];

  // Verify page ID is numeric
  if !page_id.chars().all(|c| c.is_ascii_digit()) {
    return Err(anyhow!("Page ID is not numeric: {page_id}"));
  }

  // Try to extract space key (appears between "spaces" and "pages")
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
    let url = "https://eddieland.atlassian.net/wiki/spaces/~6320c26429083bbe8cc369b0/pages/229483/Getting+started+in+Confluence+from+Jira";
    let info = parse_confluence_url(url).unwrap();

    assert_eq!(info.base_url, "https://eddieland.atlassian.net");
    assert_eq!(info.page_id, "229483");
    assert_eq!(info.space_key, Some("~6320c26429083bbe8cc369b0".to_string()));
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
}
