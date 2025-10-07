//! End-to-end tests using the fake Confluence client
//!
//! These tests demonstrate complete workflows using the fake client,
//! including page fetching, error handling, and data validation.

mod common;

use common::fake_confluence::FakeConfluenceClient;
use common::fixtures;
use confluence_dl::confluence::ConfluenceApi;

#[test]
fn test_fetch_basic_page() {
  let client = FakeConfluenceClient::with_sample_pages();

  let page = client.get_page("123456").unwrap();

  assert_eq!(page.id, "123456");
  assert_eq!(page.title, "Getting Started Guide");
  assert_eq!(page.page_type, "page");
  assert_eq!(page.status, "current");

  // Verify body content
  assert!(page.body.is_some());
  let body = page.body.unwrap();
  assert!(body.storage.is_some());

  let storage = body.storage.unwrap();
  assert!(storage.value.contains("Getting Started"));
  assert!(storage.value.contains("Welcome to our documentation"));
}

#[test]
fn test_fetch_complex_page_with_code() {
  let client = FakeConfluenceClient::with_sample_pages();

  let page = client.get_page("789012").unwrap();

  assert_eq!(page.title, "API Documentation");

  let body = page.body.unwrap();
  let storage = body.storage.unwrap();

  // Verify complex content is preserved
  assert!(storage.value.contains("API Documentation"));
  assert!(storage.value.contains("ac:structured-macro"));
  assert!(storage.value.contains("python"));
  assert!(storage.value.contains("import requests"));
}

#[test]
fn test_fetch_page_with_internal_links() {
  let client = FakeConfluenceClient::with_sample_pages();

  let page = client.get_page("345678").unwrap();

  assert_eq!(page.title, "Installation Guide");

  let body = page.body.unwrap();
  let storage = body.storage.unwrap();

  // Verify internal links are present
  assert!(storage.value.contains("ac:link"));
  assert!(storage.value.contains("ri:page"));
  assert!(storage.value.contains("Getting Started Guide"));
  assert!(storage.value.contains("API Documentation"));
}

#[test]
fn test_fetch_personal_space_page() {
  let client = FakeConfluenceClient::with_sample_pages();

  let page = client.get_page("229483").unwrap();

  assert_eq!(page.title, "Getting started in Confluence from Jira");

  // Verify space information
  let space = page.space.unwrap();
  assert_eq!(space.space_type, "personal");
  assert_eq!(space.key, "~6320c26429083bbe8cc369b0");
  assert_eq!(space.name, "Edward Jones");
}

#[test]
fn test_fetch_page_with_images() {
  let client = FakeConfluenceClient::with_sample_pages();

  let page = client.get_page("456789").unwrap();

  assert_eq!(page.title, "Architecture Diagram");

  let body = page.body.unwrap();
  let storage = body.storage.unwrap();

  // Verify image attachments are present
  assert!(storage.value.contains("ac:image"));
  assert!(storage.value.contains("ri:attachment"));
  assert!(storage.value.contains("architecture.png"));
}

#[test]
fn test_fetch_nonexistent_page() {
  let client = FakeConfluenceClient::with_sample_pages();

  let result = client.get_page("999999");

  assert!(result.is_err());
  let err = result.unwrap_err();
  assert!(err.to_string().contains("No content found"));
  assert!(err.to_string().contains("999999"));
}

#[test]
fn test_authentication_success() {
  let client = FakeConfluenceClient::with_sample_pages();

  let result = client.test_auth();
  assert!(result.is_ok());
}

#[test]
fn test_authentication_failure() {
  let mut client = FakeConfluenceClient::with_sample_pages();
  client.set_auth_success(false);

  let result = client.test_auth();
  assert!(result.is_err());

  let err = result.unwrap_err();
  assert!(err.to_string().contains("Authentication failed"));
}

#[test]
fn test_custom_page_workflow() {
  let mut client = FakeConfluenceClient::new();

  // Start with empty client
  assert!(client.get_page("custom123").is_err());

  // Add a custom page
  client.add_page_from_json("custom123", fixtures::sample_page_response());

  // Now it should be fetchable
  let page = client.get_page("custom123").unwrap();
  assert_eq!(page.title, "Getting Started Guide");
}

#[test]
fn test_multiple_pages_workflow() {
  let client = FakeConfluenceClient::with_sample_pages();

  // Fetch multiple pages in sequence
  let pages = vec!["123456", "789012", "345678"];

  for page_id in pages {
    let page = client.get_page(page_id).unwrap();
    assert_eq!(page.id, page_id);
    assert!(page.body.is_some());
  }
}

#[test]
fn test_page_links_metadata() {
  let client = FakeConfluenceClient::with_sample_pages();

  let page = client.get_page("123456").unwrap();

  // Verify links metadata
  assert!(page.links.is_some());
  let links = page.links.unwrap();

  assert!(links.web_ui.is_some());
  assert!(links.self_link.is_some());

  let web_ui = links.web_ui.unwrap();
  assert!(web_ui.contains("/wiki/spaces/DOCS/pages/123456"));
}

#[test]
fn test_page_space_metadata() {
  let client = FakeConfluenceClient::with_sample_pages();

  let page = client.get_page("789012").unwrap();

  let space = page.space.unwrap();
  assert_eq!(space.key, "DEV");
  assert_eq!(space.name, "Developer Portal");
  assert_eq!(space.space_type, "global");
}

#[test]
fn test_storage_format_representation() {
  let client = FakeConfluenceClient::with_sample_pages();

  let page = client.get_page("123456").unwrap();
  let body = page.body.unwrap();
  let storage = body.storage.unwrap();

  assert_eq!(storage.representation, "storage");
  assert!(!storage.value.is_empty());
}

#[test]
fn test_view_format_representation() {
  let client = FakeConfluenceClient::with_sample_pages();

  let page = client.get_page("123456").unwrap();
  let body = page.body.unwrap();

  assert!(body.view.is_some());
  let view = body.view.unwrap();

  assert_eq!(view.representation, "view");
  assert!(!view.value.is_empty());
}

#[test]
fn test_error_handling_workflow() {
  let mut client = FakeConfluenceClient::with_sample_pages();

  // Test auth failure
  client.set_auth_success(false);
  assert!(client.test_auth().is_err());

  // Re-enable auth
  client.set_auth_success(true);
  assert!(client.test_auth().is_ok());

  // Test missing page
  assert!(client.get_page("nonexistent").is_err());

  // Test existing page
  assert!(client.get_page("123456").is_ok());
}

use tempfile::TempDir;

#[test]
fn test_image_download_workflow() {
  use confluence_dl::confluence::{Attachment, AttachmentLinks};
  use confluence_dl::images;

  // Create a temporary directory for the test
  let temp_dir = TempDir::new().unwrap();
  let output_path = temp_dir.path();

  // Create a fake client with sample pages
  let mut client = FakeConfluenceClient::with_sample_pages();

  // Add some attachments for the page with images (must match filenames in the
  // fixture)
  let attachments = vec![Attachment {
    id: "att1".to_string(),
    title: "architecture.png".to_string(),
    attachment_type: "attachment".to_string(),
    media_type: Some("image/png".to_string()),
    file_size: Some(1024),
    links: Some(AttachmentLinks {
      download: Some("/wiki/download/attachments/456789/architecture.png".to_string()),
    }),
  }];
  client.add_attachments("456789", attachments);

  // Get the page with images
  let page = client.get_page("456789").unwrap();
  let storage_content = page
    .body
    .as_ref()
    .and_then(|b| b.storage.as_ref())
    .map(|s| s.value.as_str())
    .unwrap();

  // Extract image references
  let image_refs = images::extract_image_references(storage_content).unwrap();
  assert!(!image_refs.is_empty(), "Should find images in the page");

  // Download images
  let filename_map = images::download_images(&client, "456789", &image_refs, output_path, "images", false).unwrap();

  // Verify images were "downloaded" (fake client creates empty files)
  assert!(!filename_map.is_empty(), "Should have downloaded images");

  // Verify files exist
  for (original_filename, local_path) in &filename_map {
    let full_path = output_path.join(local_path);
    assert!(full_path.exists(), "Image file should exist: {:?}", full_path);
    println!("Downloaded {} -> {}", original_filename, local_path.display());
  }

  // Test markdown link updating
  let markdown = "![architecture](architecture.png)";
  let updated_markdown = images::update_markdown_image_links(markdown, &filename_map);

  // Verify links were updated to point to the images directory
  assert!(
    updated_markdown.contains("](images/"),
    "Links should be updated to images directory: {}",
    updated_markdown
  );
}
