//! Trait definitions for interacting with Confluence.

use std::path::Path;

use anyhow::Result;
use async_trait::async_trait;

use super::models::{Attachment, Page, UserInfo};

/// Trait for Confluence API operations (enables testing with fake
/// implementations).
#[async_trait]
pub trait ConfluenceApi: Send + Sync {
  /// Fetch a page by ID.
  ///
  /// # Arguments
  /// * `page_id` - Unique Confluence identifier for the page to retrieve.
  ///
  /// # Returns
  /// The full `Page` record including metadata and any expanded fields.
  async fn get_page(&self, page_id: &str) -> Result<Page>;

  /// Get child pages for a given page ID.
  ///
  /// # Arguments
  /// * `page_id` - Identifier of the parent page whose children should be
  ///   listed.
  ///
  /// # Returns
  /// A vector of `Page` records representing each direct child of the parent.
  async fn get_child_pages(&self, page_id: &str) -> Result<Vec<Page>>;

  /// Get attachments for a page.
  ///
  /// # Arguments
  /// * `page_id` - Identifier of the page whose attachments should be fetched.
  ///
  /// # Returns
  /// A vector of attachment metadata describing each file attached to the page.
  async fn get_attachments(&self, page_id: &str) -> Result<Vec<Attachment>>;

  /// Download an attachment by URL to a file.
  ///
  /// # Arguments
  /// * `url` - Direct or relative link to the attachment download endpoint.
  /// * `output_path` - Filesystem location where the downloaded bytes should be
  ///   written.
  ///
  /// # Returns
  /// `Ok(())` on success, or an error detailing why the download failed.
  async fn download_attachment(&self, url: &str, output_path: &Path) -> Result<()>;

  /// Test authentication and return user information.
  ///
  /// # Returns
  /// The authenticated user's profile details, confirming credentials are
  /// valid.
  async fn test_auth(&self) -> Result<UserInfo>;
}
