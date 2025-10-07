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

/// Trait for Confluence API operations (enables testing with fake
/// implementations)
pub trait ConfluenceApi {
  /// Fetch a page by ID
  fn get_page(&self, page_id: &str) -> Result<Page>;

  /// Get attachments for a page
  fn get_attachments(&self, page_id: &str) -> Result<Vec<Attachment>>;

  /// Download an attachment by URL to a file
  fn download_attachment(&self, url: &str, output_path: &std::path::Path) -> Result<()>;

  /// Test authentication
  #[allow(dead_code)]
  fn test_auth(&self) -> Result<()>;
}

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

/// Attachment metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
  pub id: String,
  pub title: String,
  #[serde(rename = "type")]
  pub attachment_type: String,
  #[serde(rename = "mediaType")]
  pub media_type: Option<String>,
  #[serde(rename = "fileSize")]
  pub file_size: Option<u64>,
  #[serde(rename = "_links")]
  pub links: Option<AttachmentLinks>,
}

/// Attachment links
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentLinks {
  pub download: Option<String>,
}

/// Attachments response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentsResponse {
  pub results: Vec<Attachment>,
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
}

impl ConfluenceApi for ConfluenceClient {
  fn get_page(&self, page_id: &str) -> Result<Page> {
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

  fn get_attachments(&self, page_id: &str) -> Result<Vec<Attachment>> {
    let url = format!("{}/wiki/rest/api/content/{}/child/attachment", self.base_url, page_id);

    let response = self
      .client
      .get(&url)
      .header("Authorization", self.auth_header())
      .header("Accept", "application/json")
      .send()
      .context("Failed to fetch attachments from Confluence API")?;

    if !response.status().is_success() {
      let status = response.status();
      let error_text = response.text().unwrap_or_else(|_| String::from("(no error details)"));
      return Err(anyhow!("Confluence API returned error {status}: {error_text}"));
    }

    let attachments: AttachmentsResponse = response
      .json()
      .context("Failed to parse attachments response from Confluence API")?;

    Ok(attachments.results)
  }

  fn download_attachment(&self, url: &str, output_path: &std::path::Path) -> Result<()> {
    // Build full URL if it's a relative path
    let full_url = if url.starts_with("http://") || url.starts_with("https://") {
      url.to_string()
    } else {
      format!("{}{}", self.base_url, url)
    };

    let response = self
      .client
      .get(&full_url)
      .header("Authorization", self.auth_header())
      .send()
      .context("Failed to download attachment")?;

    if !response.status().is_success() {
      let status = response.status();
      return Err(anyhow!("Failed to download attachment: {status}"));
    }

    // Create parent directory if it doesn't exist
    if let Some(parent) = output_path.parent() {
      std::fs::create_dir_all(parent).context("Failed to create output directory for attachment")?;
    }

    // Write response bytes to file
    let bytes = response.bytes().context("Failed to read attachment bytes")?;
    std::fs::write(output_path, bytes).context("Failed to write attachment to file")?;

    Ok(())
  }

  fn test_auth(&self) -> Result<()> {
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

  #[test]
  fn test_confluence_client_new() {
    let client = ConfluenceClient::new("https://example.atlassian.net", "user@example.com", "test-token", 30);
    assert!(client.is_ok());
    let client = client.unwrap();
    assert_eq!(client.base_url, "https://example.atlassian.net");
    assert_eq!(client.username, "user@example.com");
    assert_eq!(client.token, "test-token");
  }

  #[test]
  fn test_confluence_client_new_removes_trailing_slash() {
    let client = ConfluenceClient::new("https://example.atlassian.net/", "user@example.com", "test-token", 30).unwrap();
    assert_eq!(client.base_url, "https://example.atlassian.net");
  }

  #[test]
  fn test_auth_header_format() {
    let client = ConfluenceClient::new("https://example.atlassian.net", "user@example.com", "test-token", 30).unwrap();

    let auth_header = client.auth_header();
    assert!(auth_header.starts_with("Basic "));

    // Decode and verify the Base64 encoded credentials
    let encoded = auth_header.strip_prefix("Basic ").unwrap();
    let decoded = BASE64.decode(encoded.as_bytes()).unwrap();
    let decoded_str = String::from_utf8(decoded).unwrap();
    assert_eq!(decoded_str, "user@example.com:test-token");
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
    let result = parse_confluence_url(url);
    // This should error because file:// scheme doesn't have a host
    assert!(result.is_err());
  }
}
