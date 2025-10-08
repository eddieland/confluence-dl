//! Data transfer objects returned by the Confluence REST API.

use serde::{Deserialize, Serialize};

/// Confluence page metadata and content.
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

/// Page body content in various formats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageBody {
  pub storage: Option<StorageFormat>,
  pub view: Option<ViewFormat>,
}

/// Storage format (Confluence's internal format).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageFormat {
  pub value: String,
  pub representation: String,
}

/// View format (rendered HTML).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewFormat {
  pub value: String,
  pub representation: String,
}

/// Space information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSpace {
  pub key: String,
  pub name: String,
  #[serde(rename = "type")]
  pub space_type: String,
}

/// Page links.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageLinks {
  #[serde(rename = "webui")]
  pub web_ui: Option<String>,
  #[serde(rename = "self")]
  pub self_link: Option<String>,
}

/// Attachment metadata.
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

/// Attachment links.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentLinks {
  pub download: Option<String>,
}

/// Attachments response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentsResponse {
  pub results: Vec<Attachment>,
}

/// Child pages response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildPagesResponse {
  pub results: Vec<Page>,
}

/// User information from authentication test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
  #[serde(rename = "accountId")]
  pub account_id: String,
  pub email: Option<String>,
  #[serde(rename = "displayName")]
  pub display_name: String,
  #[serde(rename = "publicName")]
  pub public_name: Option<String>,
}
