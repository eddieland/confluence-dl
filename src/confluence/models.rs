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

/// Pagination links returned alongside paginated API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationLinks {
  /// Relative URL for the next page of results, if any.
  pub next: Option<String>,
}

/// Attachments response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentsResponse {
  /// Attachments included in the API response page.
  pub results: Vec<Attachment>,
  /// Number of items returned in this page.
  #[serde(default)]
  pub size: usize,
  /// Pagination links for traversing result pages.
  #[serde(rename = "_links")]
  pub links: Option<PaginationLinks>,
}

/// Child pages response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildPagesResponse {
  /// Child pages returned for the lookup request.
  pub results: Vec<Page>,
  /// Number of items returned in this page.
  #[serde(default)]
  pub size: usize,
  /// Pagination links for traversing result pages.
  #[serde(rename = "_links")]
  pub links: Option<PaginationLinks>,
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn child_pages_response_deserializes_with_pagination() {
    let json = serde_json::json!({
      "results": [
        {"id": "1", "title": "Child 1", "type": "page", "status": "current"},
        {"id": "2", "title": "Child 2", "type": "page", "status": "current"}
      ],
      "size": 2,
      "_links": {
        "next": "/wiki/rest/api/content/100/child/page?start=2&limit=2"
      }
    });

    let response: ChildPagesResponse = serde_json::from_value(json).unwrap();
    assert_eq!(response.results.len(), 2);
    assert_eq!(response.size, 2);
    let next = response.links.unwrap().next.unwrap();
    assert_eq!(next, "/wiki/rest/api/content/100/child/page?start=2&limit=2");
  }

  #[test]
  fn child_pages_response_deserializes_without_next_link() {
    let json = serde_json::json!({
      "results": [
        {"id": "1", "title": "Only Child", "type": "page", "status": "current"}
      ],
      "size": 1,
      "_links": {}
    });

    let response: ChildPagesResponse = serde_json::from_value(json).unwrap();
    assert_eq!(response.results.len(), 1);
    assert!(response.links.unwrap().next.is_none());
  }

  #[test]
  fn child_pages_response_deserializes_without_links() {
    let json = serde_json::json!({
      "results": []
    });

    let response: ChildPagesResponse = serde_json::from_value(json).unwrap();
    assert!(response.results.is_empty());
    assert_eq!(response.size, 0);
    assert!(response.links.is_none());
  }

  #[test]
  fn attachments_response_deserializes_with_pagination() {
    let json = serde_json::json!({
      "results": [
        {
          "id": "att1",
          "title": "doc.pdf",
          "type": "attachment",
          "_links": {"download": "/wiki/download/att1"}
        }
      ],
      "size": 1,
      "_links": {
        "next": "/wiki/rest/api/content/100/child/attachment?start=25&limit=25"
      }
    });

    let response: AttachmentsResponse = serde_json::from_value(json).unwrap();
    assert_eq!(response.results.len(), 1);
    let next = response.links.unwrap().next.unwrap();
    assert!(next.contains("start=25"));
  }
}
