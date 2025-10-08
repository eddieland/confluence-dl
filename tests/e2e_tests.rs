//! End-to-end tests using the fake Confluence client
//!
//! These tests demonstrate complete workflows using the fake client,
//! including page fetching, error handling, and data validation.

mod common;

use common::fake_confluence::FakeConfluenceClient;
use common::fixtures;
use confluence_dl::confluence::ConfluenceApi;
use insta::assert_snapshot;

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

  let user_info = result.unwrap();
  assert_eq!(user_info.account_id, "test-account-id");
  assert_eq!(user_info.email, Some("test@example.com".to_string()));
  assert_eq!(user_info.display_name, "Test User");
  assert_eq!(user_info.public_name, Some("Test User".to_string()));
}

#[test]
fn test_authentication_failure() {
  let mut client = FakeConfluenceClient::with_sample_pages();
  client.set_auth_success(false);

  let result = client.test_auth();
  assert!(result.is_err());

  let err = result.unwrap_err();
  assert!(err.to_string().contains("Authentication failed"));
  assert!(err.to_string().contains("401"));
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
    assert!(full_path.exists(), "Image file should exist: {full_path:?}");
    println!("Downloaded {} -> {}", original_filename, local_path.display());
  }

  // Test markdown link updating
  let markdown = "![architecture](architecture.png)";
  let updated_markdown = images::update_markdown_image_links(markdown, &filename_map);

  // Verify links were updated to point to the images directory
  assert!(
    updated_markdown.contains("](images/"),
    "Links should be updated to images directory: {updated_markdown}"
  );
}

#[test]
fn test_get_child_pages_empty() {
  let client = FakeConfluenceClient::with_sample_pages();

  // Page with no children should return empty vec
  let children = client.get_child_pages("123456").unwrap();
  assert!(children.is_empty(), "Page should have no children");
}

#[test]
fn test_get_child_pages_with_children() {
  let mut client = FakeConfluenceClient::with_sample_pages();

  // Add child pages
  client.add_page_from_json("111111", fixtures::sample_child_page_1_response());
  client.add_page_from_json("222222", fixtures::sample_child_page_2_response());

  // Set up parent-child relationship
  client.add_child_pages("123456", vec!["111111".to_string(), "222222".to_string()]);

  // Get children
  let children = client.get_child_pages("123456").unwrap();
  assert_eq!(children.len(), 2, "Should have 2 children");

  // Verify child titles
  assert_eq!(children[0].title, "Child Page 1");
  assert_eq!(children[1].title, "Child Page 2");
}

#[test]
fn test_page_tree_single_page() {
  use confluence_dl::confluence::get_page_tree;

  let client = FakeConfluenceClient::with_sample_pages();

  // Build tree for page with no children
  let tree = get_page_tree(&client, "123456", None).unwrap();

  assert_eq!(tree.page.id, "123456");
  assert_eq!(tree.page.title, "Getting Started Guide");
  assert_eq!(tree.depth, 0);
  assert!(tree.children.is_empty());
}

#[test]
fn test_page_tree_with_children() {
  use confluence_dl::confluence::get_page_tree;

  let mut client = FakeConfluenceClient::with_sample_pages();

  // Set up page hierarchy: 123456 -> [111111, 222222]
  client.add_page_from_json("111111", fixtures::sample_child_page_1_response());
  client.add_page_from_json("222222", fixtures::sample_child_page_2_response());
  client.add_child_pages("123456", vec!["111111".to_string(), "222222".to_string()]);

  // Build tree
  let tree = get_page_tree(&client, "123456", None).unwrap();

  assert_eq!(tree.page.title, "Getting Started Guide");
  assert_eq!(tree.depth, 0);
  assert_eq!(tree.children.len(), 2);

  // Verify first child
  assert_eq!(tree.children[0].page.title, "Child Page 1");
  assert_eq!(tree.children[0].depth, 1);
  assert!(tree.children[0].children.is_empty());

  // Verify second child
  assert_eq!(tree.children[1].page.title, "Child Page 2");
  assert_eq!(tree.children[1].depth, 1);
  assert!(tree.children[1].children.is_empty());
}

#[test]
fn test_page_tree_with_grandchildren() {
  use confluence_dl::confluence::get_page_tree;

  let mut client = FakeConfluenceClient::with_sample_pages();

  // Set up page hierarchy: 123456 -> 111111 -> 333333
  client.add_page_from_json("111111", fixtures::sample_child_page_1_response());
  client.add_page_from_json("333333", fixtures::sample_grandchild_page_response());
  client.add_child_pages("123456", vec!["111111".to_string()]);
  client.add_child_pages("111111", vec!["333333".to_string()]);

  // Build tree with unlimited depth
  let tree = get_page_tree(&client, "123456", None).unwrap();

  assert_eq!(tree.depth, 0);
  assert_eq!(tree.children.len(), 1);

  // Verify child
  let child = &tree.children[0];
  assert_eq!(child.page.title, "Child Page 1");
  assert_eq!(child.depth, 1);
  assert_eq!(child.children.len(), 1);

  // Verify grandchild
  let grandchild = &child.children[0];
  assert_eq!(grandchild.page.title, "Grandchild Page");
  assert_eq!(grandchild.depth, 2);
  assert!(grandchild.children.is_empty());
}

#[test]
fn test_page_tree_max_depth_limit() {
  use confluence_dl::confluence::get_page_tree;

  let mut client = FakeConfluenceClient::with_sample_pages();

  // Set up page hierarchy: 123456 -> 111111 -> 333333
  client.add_page_from_json("111111", fixtures::sample_child_page_1_response());
  client.add_page_from_json("333333", fixtures::sample_grandchild_page_response());
  client.add_child_pages("123456", vec!["111111".to_string()]);
  client.add_child_pages("111111", vec!["333333".to_string()]);

  // Build tree with max_depth = 1 (should stop at children, not grandchildren)
  let tree = get_page_tree(&client, "123456", Some(1)).unwrap();

  assert_eq!(tree.depth, 0);
  assert_eq!(tree.children.len(), 1);

  // Verify child exists
  let child = &tree.children[0];
  assert_eq!(child.page.title, "Child Page 1");
  assert_eq!(child.depth, 1);

  // Grandchild should NOT be included due to depth limit
  assert!(
    child.children.is_empty(),
    "Should not fetch grandchildren when max_depth=1"
  );
}

#[test]
fn test_page_tree_depth_zero() {
  use confluence_dl::confluence::get_page_tree;

  let mut client = FakeConfluenceClient::with_sample_pages();

  // Set up children
  client.add_page_from_json("111111", fixtures::sample_child_page_1_response());
  client.add_child_pages("123456", vec!["111111".to_string()]);

  // Build tree with max_depth = 0 (should include only root page)
  let tree = get_page_tree(&client, "123456", Some(0)).unwrap();

  assert_eq!(tree.depth, 0);
  assert!(tree.children.is_empty(), "Should not fetch children when max_depth=0");
}

#[test]
fn test_page_tree_circular_reference_detection() {
  use confluence_dl::confluence::get_page_tree;

  let mut client = FakeConfluenceClient::with_sample_pages();

  // Create circular reference: 123456 -> 111111 -> 123456
  client.add_page_from_json("111111", fixtures::sample_child_page_1_response());
  client.add_child_pages("123456", vec!["111111".to_string()]);
  client.add_child_pages("111111", vec!["123456".to_string()]);

  // The function should successfully build the tree but skip the circular
  // reference (it logs a warning and continues with other children)
  let result = get_page_tree(&client, "123456", None);

  assert!(result.is_ok(), "Should handle circular reference gracefully");
  let tree = result.unwrap();

  // Root page should be present
  assert_eq!(tree.page.title, "Getting Started Guide");
  assert_eq!(tree.children.len(), 1);

  // Child should be present but without the circular reference back to parent
  let child = &tree.children[0];
  assert_eq!(child.page.title, "Child Page 1");
  assert!(child.children.is_empty(), "Circular reference should be skipped");
}

#[test]
fn test_convert_comprehensive_features_page_to_markdown() {
  use confluence_dl::markdown;

  let mut client = FakeConfluenceClient::with_sample_pages();

  // Add the comprehensive test page
  client.add_page_from_json("776655", fixtures::sample_comprehensive_features_response());

  // Fetch the page
  let page = client.get_page("776655").unwrap();

  // Extract storage content
  let storage_content = page
    .body
    .as_ref()
    .and_then(|b| b.storage.as_ref())
    .map(|s| s.value.as_str())
    .unwrap();

  // Convert to markdown
  let markdown = markdown::storage_to_markdown(storage_content).unwrap();

  assert_snapshot!(markdown);
}

#[test]
fn test_convert_meeting_notes_overview_to_markdown() {
  use confluence_dl::markdown;

  let mut client = FakeConfluenceClient::with_sample_pages();

  // Add the meeting notes overview page
  client.add_page_from_json("998877", fixtures::sample_meeting_notes_overview_response());

  // Fetch the page
  let page = client.get_page("998877").unwrap();

  // Extract storage content
  let storage_content = page
    .body
    .as_ref()
    .and_then(|b| b.storage.as_ref())
    .map(|s| s.value.as_str())
    .unwrap();

  // Convert to markdown
  let markdown = markdown::storage_to_markdown(storage_content).unwrap();

  assert_snapshot!(markdown);
}

#[test]
fn test_convert_meeting_notes_with_tasks_to_markdown() {
  use confluence_dl::markdown;

  let mut client = FakeConfluenceClient::with_sample_pages();

  // Add the meeting notes with tasks page
  client.add_page_from_json("887766", fixtures::sample_meeting_notes_with_tasks_response());

  // Fetch the page
  let page = client.get_page("887766").unwrap();

  // Extract storage content
  let storage_content = page
    .body
    .as_ref()
    .and_then(|b| b.storage.as_ref())
    .map(|s| s.value.as_str())
    .unwrap();

  // Convert to markdown
  let markdown = markdown::storage_to_markdown(storage_content).unwrap();

  assert_snapshot!(markdown);
}

#[test]
fn test_convert_complex_page_with_code_to_markdown() {
  use confluence_dl::markdown;

  let client = FakeConfluenceClient::with_sample_pages();

  // Fetch the complex page (already in sample_pages)
  let page = client.get_page("789012").unwrap();

  // Extract storage content
  let storage_content = page
    .body
    .as_ref()
    .and_then(|b| b.storage.as_ref())
    .map(|s| s.value.as_str())
    .unwrap();

  // Convert to markdown
  let markdown = markdown::storage_to_markdown(storage_content).unwrap();

  assert_snapshot!(markdown);
}

#[test]
fn test_convert_page_with_internal_links_to_markdown() {
  use confluence_dl::markdown;

  let client = FakeConfluenceClient::with_sample_pages();

  // Fetch the page with internal links
  let page = client.get_page("345678").unwrap();

  // Extract storage content
  let storage_content = page
    .body
    .as_ref()
    .and_then(|b| b.storage.as_ref())
    .map(|s| s.value.as_str())
    .unwrap();

  // Convert to markdown
  let markdown = markdown::storage_to_markdown(storage_content).unwrap();

  assert_snapshot!(markdown);
}

#[test]
fn test_end_to_end_page_fetch_and_convert() {
  use confluence_dl::markdown;

  let client = FakeConfluenceClient::with_sample_pages();

  // Test a complete workflow: fetch page -> extract content -> convert to
  // markdown
  let page_id = "123456";
  let page = client.get_page(page_id).unwrap();

  assert_eq!(page.id, page_id);
  assert_eq!(page.title, "Getting Started Guide");

  // Extract and convert
  let storage_content = page
    .body
    .as_ref()
    .and_then(|b| b.storage.as_ref())
    .map(|s| s.value.as_str())
    .expect("Page should have storage content");

  let markdown = markdown::storage_to_markdown(storage_content).unwrap();

  assert_snapshot!(markdown);
}

#[test]
fn test_markdown_conversion_handles_empty_content() {
  use confluence_dl::markdown;

  // Test that empty/minimal content doesn't crash
  let empty_storage = "";
  let result = markdown::storage_to_markdown(empty_storage);
  assert!(result.is_ok(), "Should handle empty content");

  let minimal_storage = "<p>Test</p>";
  let markdown = markdown::storage_to_markdown(minimal_storage).unwrap();
  assert!(markdown.contains("Test"), "Should contain text");
}

#[test]
fn test_markdown_conversion_preserves_structure() {
  use confluence_dl::markdown;

  let mut client = FakeConfluenceClient::with_sample_pages();

  // Add the comprehensive page
  client.add_page_from_json("776655", fixtures::sample_comprehensive_features_response());
  let page = client.get_page("776655").unwrap();

  let storage_content = page
    .body
    .as_ref()
    .and_then(|b| b.storage.as_ref())
    .map(|s| s.value.as_str())
    .unwrap();

  let markdown = markdown::storage_to_markdown(storage_content).unwrap();

  assert_snapshot!(markdown);
}
