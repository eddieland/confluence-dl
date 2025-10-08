//! Confluence module providing API abstractions, the HTTP client, data models,
//! URL parsing helpers, and higher-level traversal utilities.

pub mod api;
pub mod client;
pub mod models;
pub mod tree;
pub mod url;

pub use api::ConfluenceApi;
pub use client::ConfluenceClient;
#[allow(unused_imports)]
pub use models::{
  Attachment, AttachmentLinks, AttachmentsResponse, ChildPagesResponse, Page, PageBody, PageLinks, PageSpace,
  StorageFormat, UserInfo, ViewFormat,
};
pub use tree::{PageTree, get_page_tree};
pub use url::{UrlInfo, parse_confluence_url};
