//! Fake Confluence API client for testing
//!
//! This module provides a stub implementation of the Confluence API that
//! returns predefined responses without making any network requests.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use confluence_dl::confluence::{Attachment, ConfluenceApi, Page, UserInfo};

use crate::common::fixtures;

/// A fake Confluence client that returns predefined responses for testing
pub struct FakeConfluenceClient {
  pages: HashMap<String, Page>,
  attachments: HashMap<String, Vec<Attachment>>,
  child_pages: HashMap<String, Vec<String>>,
  auth_should_succeed: bool,
}

impl FakeConfluenceClient {
  /// Create a new fake client with no pages
  pub fn new() -> Self {
    Self {
      pages: HashMap::new(),
      attachments: HashMap::new(),
      child_pages: HashMap::new(),
      auth_should_succeed: true,
    }
  }

  /// Create a fake client with default sample pages
  pub fn with_sample_pages() -> Self {
    let mut client = Self::new();

    // Add sample pages from fixtures
    client.add_page_from_json("123456", fixtures::sample_page_response());
    client.add_page_from_json("789012", fixtures::sample_complex_page_response());
    client.add_page_from_json("345678", fixtures::sample_page_with_links_response());
    client.add_page_from_json("229483", fixtures::sample_personal_space_page_response());
    client.add_page_from_json("456789", fixtures::sample_page_with_images_response());

    client
  }

  /// Add a page from a JSON value
  pub fn add_page_from_json(&mut self, page_id: &str, json: serde_json::Value) {
    if let Ok(page) = serde_json::from_value::<Page>(json) {
      self.pages.insert(page_id.to_string(), page);
    }
  }

  /// Add a pre-constructed Page object
  #[allow(dead_code)]
  pub fn add_page(&mut self, page_id: &str, page: Page) {
    self.pages.insert(page_id.to_string(), page);
  }

  /// Configure whether authentication should succeed
  pub fn set_auth_success(&mut self, should_succeed: bool) {
    self.auth_should_succeed = should_succeed;
  }

  /// Add attachments for a page
  #[allow(dead_code)]
  pub fn add_attachments(&mut self, page_id: &str, attachments: Vec<Attachment>) {
    self.attachments.insert(page_id.to_string(), attachments);
  }

  /// Add child pages for a parent page
  #[allow(dead_code)]
  pub fn add_child_pages(&mut self, parent_id: &str, child_ids: Vec<String>) {
    self.child_pages.insert(parent_id.to_string(), child_ids);
  }
}

impl Default for FakeConfluenceClient {
  fn default() -> Self {
    Self::new()
  }
}

#[async_trait]
impl ConfluenceApi for FakeConfluenceClient {
  async fn get_page(&self, page_id: &str) -> Result<Page> {
    self
      .pages
      .get(page_id)
      .cloned()
      .ok_or_else(|| anyhow!("No content found with id: {}", page_id))
  }

  async fn get_child_pages(&self, page_id: &str) -> Result<Vec<Page>> {
    let child_ids = self.child_pages.get(page_id).cloned().unwrap_or_default();
    let mut children = Vec::new();

    for child_id in child_ids {
      if let Some(page) = self.pages.get(&child_id) {
        children.push(page.clone());
      }
    }

    Ok(children)
  }

  async fn get_attachments(&self, page_id: &str) -> Result<Vec<Attachment>> {
    Ok(self.attachments.get(page_id).cloned().unwrap_or_default())
  }

  async fn download_attachment(&self, _url: &str, output_path: &Path) -> Result<()> {
    // For testing, just create an empty file
    if let Some(parent) = output_path.parent() {
      tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(output_path, b"fake image data").await?;
    Ok(())
  }

  async fn fetch_attachment(&self, _url: &str) -> Result<Vec<u8>> {
    Ok(b"fake image data".to_vec())
  }

  async fn test_auth(&self) -> Result<UserInfo> {
    if self.auth_should_succeed {
      Ok(UserInfo {
        account_id: "test-account-id".to_string(),
        email: Some("test@example.com".to_string()),
        display_name: "Test User".to_string(),
        public_name: Some("Test User".to_string()),
      })
    } else {
      Err(anyhow!("Authentication failed with status: 401"))
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_fake_client_empty() {
    let client = FakeConfluenceClient::new();
    assert!(client.get_page("123456").await.is_err());
  }

  #[tokio::test]
  async fn test_fake_client_with_samples() {
    let client = FakeConfluenceClient::with_sample_pages();

    // Should be able to fetch sample pages
    let page = client.get_page("123456").await.unwrap();
    assert_eq!(page.id, "123456");
    assert_eq!(page.title, "Getting Started Guide");

    // Non-existent page should error
    assert!(client.get_page("999999").await.is_err());
  }

  #[tokio::test]
  async fn test_fake_client_add_page() {
    let mut client = FakeConfluenceClient::new();

    client.add_page_from_json("test123", fixtures::sample_page_response());

    let page = client.get_page("test123").await.unwrap();
    assert_eq!(page.title, "Getting Started Guide");
  }

  #[tokio::test]
  async fn test_fake_client_auth_success() {
    let client = FakeConfluenceClient::new();
    assert!(client.test_auth().await.is_ok());
  }

  #[tokio::test]
  async fn test_fake_client_auth_failure() {
    let mut client = FakeConfluenceClient::new();
    client.set_auth_success(false);
    assert!(client.test_auth().await.is_err());
  }

  #[tokio::test]
  async fn test_fake_client_complex_page() {
    let client = FakeConfluenceClient::with_sample_pages();

    let page = client.get_page("789012").await.unwrap();
    assert_eq!(page.title, "API Documentation");
    assert!(page.body.is_some());

    let body = page.body.unwrap();
    assert!(body.storage.is_some());

    let storage = body.storage.unwrap();
    assert!(storage.value.contains("API Documentation"));
    assert!(storage.value.contains("code"));
  }

  #[tokio::test]
  async fn test_fake_client_page_with_space() {
    let client = FakeConfluenceClient::with_sample_pages();

    let page = client.get_page("123456").await.unwrap();
    assert!(page.space.is_some());

    let space = page.space.unwrap();
    assert_eq!(space.key, "DOCS");
    assert_eq!(space.name, "Documentation");
  }
}
