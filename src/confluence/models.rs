//! Data transfer objects returned by the Confluence REST API.

use serde::{Deserialize, Serialize};

/// Confluence page metadata and content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
  /// Unique numeric identifier assigned by Confluence.
  pub id: String,
  /// Human-readable title displayed in the UI.
  pub title: String,
  #[serde(rename = "type")]
  /// Content type (typically `"page"` or `"blogpost"`).
  pub page_type: String,
  /// Publication status such as `"current"` or `"draft"`.
  pub status: String,
  /// Rich body content in different renderings.
  pub body: Option<PageBody>,
  /// Space metadata describing where the page lives.
  pub space: Option<PageSpace>,
  #[serde(rename = "_links")]
  /// Useful hyperlinks, including the canonical UI URL.
  pub links: Option<PageLinks>,
}

/// Page body content in various formats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageBody {
  /// Confluence storage-format XHTML representation.
  pub storage: Option<StorageFormat>,
  /// Rendered HTML view supplied by the API when expanded.
  pub view: Option<ViewFormat>,
}

/// Storage format (Confluence's internal format).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageFormat {
  /// Raw XHTML markup returned by the API.
  pub value: String,
  /// Representation name (typically `"storage"`).
  pub representation: String,
}

/// View format (rendered HTML).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewFormat {
  /// Rendered HTML snippet safe for display.
  pub value: String,
  /// Representation name (typically `"view"`).
  pub representation: String,
}

/// Space information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSpace {
  /// Short key that uniquely identifies the space.
  pub key: String,
  /// Human-readable space name.
  pub name: String,
  #[serde(rename = "type")]
  /// Space classification such as `"global"` or `"personal"`.
  pub space_type: String,
}

/// Page links.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageLinks {
  #[serde(rename = "webui")]
  /// Path to the page within the Confluence web UI.
  pub web_ui: Option<String>,
  #[serde(rename = "self")]
  /// Fully qualified API endpoint for the resource.
  pub self_link: Option<String>,
}

/// Attachment metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
  /// Unique attachment identifier.
  pub id: String,
  /// Original filename/title displayed in Confluence.
  pub title: String,
  #[serde(rename = "type")]
  /// Attachment content type (usually `"attachment"`).
  pub attachment_type: String,
  #[serde(rename = "mediaType")]
  /// MIME type reported by Confluence, when known.
  pub media_type: Option<String>,
  #[serde(rename = "fileSize")]
  /// Size of the attachment in bytes.
  pub file_size: Option<u64>,
  #[serde(rename = "_links")]
  /// Download and metadata links for the file.
  pub links: Option<AttachmentLinks>,
}

/// Attachment links.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentLinks {
  /// Direct download URL for the attachment bytes.
  pub download: Option<String>,
}

/// Attachments response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentsResponse {
  /// Attachments included in the API response page.
  pub results: Vec<Attachment>,
}

/// Child pages response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildPagesResponse {
  /// Child pages returned for the lookup request.
  pub results: Vec<Page>,
}

/// User information from authentication test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
  #[serde(rename = "accountId")]
  /// Stable Atlassian account identifier.
  pub account_id: String,
  /// Primary email address if the API caller is permitted to view it.
  pub email: Option<String>,
  #[serde(rename = "displayName")]
  /// Full display name configured in the Atlassian profile.
  pub display_name: String,
  #[serde(rename = "publicName")]
  /// Publicly visible name, which may differ from `display_name`.
  pub public_name: Option<String>,
}
