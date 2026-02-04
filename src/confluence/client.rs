//! HTTP client implementation for talking to the Confluence REST API.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use tokio::sync::Mutex;
use tokio::time::sleep;

use super::api::ConfluenceApi;
use super::models::{Attachment, AttachmentsResponse, ChildPagesResponse, Page, UserInfo};

/// Confluence API client.
#[derive(Clone)]
pub struct ConfluenceClient {
  base_url: String,
  username: String,
  token: String,
  client: reqwest::Client,
  rate_limiter: Arc<RequestRateLimiter>,
}

/// Simple fixed-window rate limiter to cap the number of requests per interval.
#[derive(Debug)]
struct RequestRateLimiter {
  max_requests: usize,
  window: Duration,
  timestamps: Mutex<VecDeque<Instant>>,
}

impl RequestRateLimiter {
  /// Create a rate limiter with a fixed window.
  ///
  /// # Arguments
  /// * `max_requests` - Maximum number of requests permitted within the window.
  /// * `window` - Duration of the request window used to enforce throttling.
  ///
  /// # Returns
  /// A rate limiter that enforces the configured throughput ceiling.
  fn new(max_requests: usize, window: Duration) -> Self {
    Self {
      max_requests,
      window,
      timestamps: Mutex::new(VecDeque::with_capacity(max_requests)),
    }
  }

  /// Wait until the caller can perform another request without exceeding the
  /// rate limit.
  ///
  /// # Returns
  /// Completes when the rate limiter has reserved a slot for a request.
  async fn acquire(&self) {
    loop {
      let mut timestamps = self.timestamps.lock().await;
      let now = Instant::now();

      while let Some(earliest) = timestamps.front()
        && now.duration_since(*earliest) >= self.window
      {
        timestamps.pop_front();
      }

      if timestamps.len() < self.max_requests {
        timestamps.push_back(now);
        return;
      }

      let earliest = *timestamps.front().expect("rate limiter queue should never be empty");
      let elapsed = now.duration_since(earliest);
      let wait_duration = if elapsed >= self.window {
        Duration::from_secs(0)
      } else {
        self.window - elapsed
      };

      drop(timestamps);

      if wait_duration > Duration::from_secs(0) {
        sleep(wait_duration).await;
      }
    }
  }
}

impl ConfluenceClient {
  /// Create a new Confluence client.
  ///
  /// # Arguments
  /// * `base_url` - The base URL of the Confluence instance (e.g., https://example.atlassian.net)
  /// * `username` - The user's email address
  /// * `token` - The API token
  /// * `timeout_secs` - Request timeout in seconds
  /// * `rate_limit` - Maximum requests per second
  ///
  /// # Returns
  /// A configured `ConfluenceClient` ready for API calls when the provided
  /// options are valid.
  ///
  /// # Errors
  /// Returns an error if the rate limit is zero or if the underlying
  /// `reqwest::Client` cannot be built.
  pub fn new(
    base_url: impl Into<String>,
    username: impl Into<String>,
    token: impl Into<String>,
    timeout_secs: u64,
    rate_limit: usize,
  ) -> Result<Self> {
    let base_url = base_url.into();
    let username = username.into();
    let token = token.into();

    if rate_limit == 0 {
      return Err(anyhow!("Rate limit must be at least 1 request per second"));
    }

    let base_url = base_url.trim_end_matches('/').to_string();

    let client = reqwest::Client::builder()
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
      rate_limiter: Arc::new(RequestRateLimiter::new(rate_limit, Duration::from_secs(1))),
    })
  }

  /// Get the authorization header value (Basic auth).
  ///
  /// # Returns
  /// Encoded `Basic` authorization header string for the configured
  /// credentials.
  fn auth_header(&self) -> String {
    let credentials = format!("{}:{}", self.username, self.token);
    format!("Basic {}", BASE64.encode(credentials.as_bytes()))
  }
}

#[async_trait]
impl ConfluenceApi for ConfluenceClient {
  async fn get_page(&self, page_id: &str) -> Result<Page> {
    self.rate_limiter.acquire().await;

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
      .await
      .context("Failed to send request to Confluence API")?;

    if !response.status().is_success() {
      let status = response.status();
      let error_text = response
        .text()
        .await
        .unwrap_or_else(|_| String::from("(no error details)"));
      return Err(anyhow!("Confluence API returned error {status}: {error_text}"));
    }

    let page: Page = response
      .json()
      .await
      .context("Failed to parse page response from Confluence API")?;

    Ok(page)
  }

  async fn get_child_pages(&self, page_id: &str) -> Result<Vec<Page>> {
    self.rate_limiter.acquire().await;

    let url = format!("{}/wiki/rest/api/content/{}/child/page", self.base_url, page_id);

    let response = self
      .client
      .get(&url)
      .header("Authorization", self.auth_header())
      .header("Accept", "application/json")
      .send()
      .await
      .context("Failed to fetch child pages from Confluence API")?;

    if !response.status().is_success() {
      let status = response.status();
      let error_text = response
        .text()
        .await
        .unwrap_or_else(|_| String::from("(no error details)"));
      return Err(anyhow!("Confluence API returned error {status}: {error_text}"));
    }

    let child_pages: ChildPagesResponse = response
      .json()
      .await
      .context("Failed to parse child pages response from Confluence API")?;

    Ok(child_pages.results)
  }

  async fn get_attachments(&self, page_id: &str) -> Result<Vec<Attachment>> {
    self.rate_limiter.acquire().await;

    let url = format!("{}/wiki/rest/api/content/{}/child/attachment", self.base_url, page_id);

    let response = self
      .client
      .get(&url)
      .header("Authorization", self.auth_header())
      .header("Accept", "application/json")
      .send()
      .await
      .context("Failed to fetch attachments from Confluence API")?;

    if !response.status().is_success() {
      let status = response.status();
      let error_text = response
        .text()
        .await
        .unwrap_or_else(|_| String::from("(no error details)"));
      return Err(anyhow!("Confluence API returned error {status}: {error_text}"));
    }

    let attachments: AttachmentsResponse = response
      .json()
      .await
      .context("Failed to parse attachments response from Confluence API")?;

    Ok(attachments.results)
  }

  async fn download_attachment(&self, url: &str, output_path: &std::path::Path) -> Result<()> {
    let bytes = self.fetch_attachment(url).await?;

    if let Some(parent) = output_path.parent() {
      tokio::fs::create_dir_all(parent)
        .await
        .context("Failed to create output directory for attachment")?;
    }

    tokio::fs::write(output_path, bytes)
      .await
      .context("Failed to write attachment to file")?;

    Ok(())
  }

  async fn fetch_attachment(&self, url: &str) -> Result<Vec<u8>> {
    let full_url = self.resolve_attachment_url(url);

    self.rate_limiter.acquire().await;

    let response = self
      .client
      .get(&full_url)
      .header("Authorization", self.auth_header())
      .send()
      .await
      .context("Failed to download attachment")?;

    let status = response.status();
    if !status.is_success() {
      let error_text = response
        .text()
        .await
        .unwrap_or_else(|_| String::from("(no error details)"));
      return Err(anyhow!(
        "Failed to fetch attachment from {full_url}: {status} - {error_text}"
      ));
    }

    let bytes = response.bytes().await.context("Failed to read attachment bytes")?;
    Ok(bytes.to_vec())
  }

  async fn test_auth(&self) -> Result<UserInfo> {
    self.rate_limiter.acquire().await;

    let url = format!("{}/wiki/rest/api/user/current", self.base_url);

    let response = self
      .client
      .get(&url)
      .header("Authorization", self.auth_header())
      .header("Accept", "application/json")
      .send()
      .await
      .context("Failed to send authentication test request")?;

    if !response.status().is_success() {
      let status = response.status();
      let error_text = response
        .text()
        .await
        .unwrap_or_else(|_| String::from("(no error details)"));
      return Err(anyhow!("Authentication failed with status {status}: {error_text}"));
    }

    let user_info: UserInfo = response
      .json()
      .await
      .context("Failed to parse user information from Confluence API")?;

    Ok(user_info)
  }
}

impl ConfluenceClient {
  fn resolve_attachment_url(&self, url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
      return url.to_string();
    }

    if url.starts_with("/wiki/") {
      return format!("{}{}", self.base_url, url);
    }

    if url.starts_with("/download/") {
      return format!("{}/wiki{}", self.base_url, url);
    }

    format!("{}{}", self.base_url, url)
  }
}

#[cfg(test)]
mod tests {
  use base64::Engine as _;

  use super::*;

  #[test]
  fn test_confluence_client_new() {
    let client = ConfluenceClient::new("https://example.atlassian.net", "user@example.com", "test-token", 30, 5);
    assert!(client.is_ok());
    let client = client.unwrap();
    assert_eq!(client.base_url, "https://example.atlassian.net");
    assert_eq!(client.username, "user@example.com");
    assert_eq!(client.token, "test-token");
  }

  #[test]
  fn test_confluence_client_new_removes_trailing_slash() {
    let client = ConfluenceClient::new(
      "https://example.atlassian.net/",
      "user@example.com",
      "test-token",
      30,
      2,
    )
    .unwrap();
    assert_eq!(client.base_url, "https://example.atlassian.net");
  }

  #[test]
  fn test_auth_header_format() {
    let client =
      ConfluenceClient::new("https://example.atlassian.net", "user@example.com", "test-token", 30, 3).unwrap();

    let auth_header = client.auth_header();
    assert!(auth_header.starts_with("Basic "));

    let encoded = auth_header.strip_prefix("Basic ").unwrap();
    let decoded = BASE64.decode(encoded.as_bytes()).unwrap();
    let decoded_str = String::from_utf8(decoded).unwrap();
    assert_eq!(decoded_str, "user@example.com:test-token");
  }

  #[test]
  fn test_confluence_client_rejects_zero_rate_limit() {
    let client = ConfluenceClient::new("https://example.atlassian.net", "user@example.com", "test-token", 30, 0);
    assert!(client.is_err());
  }

  #[tokio::test]
  async fn test_rate_limiter_throttles_requests() {
    let limiter = RequestRateLimiter::new(2, Duration::from_secs(1));
    let start = Instant::now();

    limiter.acquire().await;
    limiter.acquire().await;
    limiter.acquire().await;

    assert!(
      start.elapsed() >= Duration::from_millis(900),
      "expected at least 900ms elapsed, got {:?}",
      start.elapsed()
    );
  }

  #[test]
  fn resolve_attachment_url_handles_absolute_urls() {
    let client =
      ConfluenceClient::new("https://example.atlassian.net", "user@example.com", "test-token", 30, 5).unwrap();

    let absolute = "https://cdn.example.com/files/image.png";
    assert_eq!(client.resolve_attachment_url(absolute), absolute);
  }

  #[test]
  fn resolve_attachment_url_prefixes_wiki_when_missing() {
    let client =
      ConfluenceClient::new("https://example.atlassian.net", "user@example.com", "test-token", 30, 5).unwrap();

    let relative = "/download/attachments/12345/image.png";
    assert_eq!(
      client.resolve_attachment_url(relative),
      "https://example.atlassian.net/wiki/download/attachments/12345/image.png"
    );
  }

  #[test]
  fn resolve_attachment_url_keeps_existing_wiki_prefix() {
    let client =
      ConfluenceClient::new("https://example.atlassian.net", "user@example.com", "test-token", 30, 5).unwrap();

    let relative = "/wiki/download/attachments/12345/image.png";
    assert_eq!(
      client.resolve_attachment_url(relative),
      "https://example.atlassian.net/wiki/download/attachments/12345/image.png"
    );
  }
}
