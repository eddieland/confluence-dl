//! Image extraction and download utilities for Confluence content.
//!
//! This module handles extracting image references from Confluence storage
//! format, downloading them, and updating markdown links to reference local
//! files.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use roxmltree::{Document, Node};

const SYNTHETIC_NS_BASE: &str = "https://confluence.example/";

use crate::confluence::ConfluenceApi;

/// Information about an image found in Confluence content
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageReference {
  /// The filename of the image attachment
  pub filename: String,
  /// The alt text for the image
  pub alt_text: String,
}

/// Extract image references from Confluence storage format content
///
/// Parses the HTML/XML content to find `<ac:image>` tags and extracts
/// the attachment filenames and alt text.
pub fn extract_image_references(storage_content: &str) -> Result<Vec<ImageReference>> {
  // Pre-process: Replace HTML entities with Unicode characters
  // roxmltree only supports XML's 5 predefined entities, not HTML entities
  let preprocessed = preprocess_html_entities(storage_content);
  let wrapped = wrap_with_namespaces(&preprocessed);
  let document = Document::parse(&wrapped).context("Failed to parse Confluence storage content for images")?;
  let mut images = Vec::new();

  for image_elem in document.descendants().filter(|node| matches_tag(*node, "ac:image")) {
    let alt_text = get_attribute(image_elem, "ac:alt").unwrap_or_else(|| "image".to_string());

    for attachment in image_elem
      .children()
      .filter(|child| matches_tag(*child, "ri:attachment"))
    {
      if let Some(filename) = get_attribute(attachment, "ri:filename") {
        images.push(ImageReference {
          filename,
          alt_text: alt_text.clone(),
        });
      }
    }
  }

  Ok(images)
}

fn split_qualified_name(name: &str) -> (Option<&str>, &str) {
  if let Some((prefix, local)) = name.split_once(':') {
    (Some(prefix), local)
  } else {
    (None, name)
  }
}

fn matches_tag<'a, 'input>(node: Node<'a, 'input>, name: &str) -> bool {
  if !node.is_element() {
    return false;
  }

  let (expected_prefix, expected_name) = split_qualified_name(name);
  let tag = node.tag_name();
  if tag.name() != expected_name {
    return false;
  }

  let expected_namespace = expected_prefix.map(|prefix| format!("{SYNTHETIC_NS_BASE}{prefix}"));

  match (expected_namespace.as_deref(), tag.namespace()) {
    (Some(expected), Some(actual)) => actual == expected,
    (None, None) => true,
    (Some(_), None) | (None, Some(_)) => false,
  }
}

fn get_attribute<'a, 'input>(node: Node<'a, 'input>, attr_name: &str) -> Option<String> {
  if !node.is_element() {
    return None;
  }

  let (expected_prefix, expected_name) = split_qualified_name(attr_name);
  let expected_namespace = expected_prefix.map(|prefix| format!("{SYNTHETIC_NS_BASE}{prefix}"));

  for attr in node.attributes() {
    if attr.name() != expected_name {
      continue;
    }

    let namespace_matches = match (expected_namespace.as_deref(), attr.namespace()) {
      (Some(expected), Some(actual)) => actual == expected,
      (None, None) => true,
      (Some(_), None) | (None, Some(_)) => false,
    };

    if namespace_matches {
      return Some(attr.value().to_string());
    }
  }
  None
}

fn wrap_with_namespaces(storage_content: &str) -> String {
  let mut prefixes = BTreeSet::new();

  for segment in storage_content.split('<').skip(1) {
    let mut segment = segment;
    if let Some(idx) = segment.find('>') {
      segment = &segment[..idx];
    }

    let segment = segment.trim_start_matches('/');

    if let Some((prefix, _)) = segment.split_once(':')
      && is_valid_prefix(prefix)
    {
      prefixes.insert(prefix.to_string());
    }

    for attr in segment.split_whitespace() {
      if let Some((name, _)) = attr.split_once('=')
        && let Some((prefix, _)) = name.split_once(':')
        && is_valid_prefix(prefix)
      {
        prefixes.insert(prefix.to_string());
      }
    }
  }

  let mut result = String::from("<cdl-root");
  for prefix in prefixes {
    result.push_str(" xmlns:");
    result.push_str(&prefix);
    result.push_str("=\"");
    result.push_str(SYNTHETIC_NS_BASE);
    result.push_str(&prefix);
    result.push('"');
  }
  result.push('>');
  result.push_str(storage_content);
  result.push_str("</cdl-root>");
  result
}

fn is_valid_prefix(prefix: &str) -> bool {
  if prefix.is_empty() {
    return false;
  }
  prefix
    .chars()
    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Replace common HTML entities with their Unicode characters before XML
/// parsing roxmltree only recognizes XML's 5 predefined entities (&lt; &gt;
/// &amp; &quot; &apos;) so we need to convert HTML entities to literal
/// characters
fn preprocess_html_entities(text: &str) -> String {
  text
    .replace("&nbsp;", "\u{00A0}") // non-breaking space
    .replace("&ndash;", "\u{2013}") // en dash
    .replace("&mdash;", "\u{2014}") // em dash
    .replace("&ldquo;", "\u{201C}") // left double quote
    .replace("&rdquo;", "\u{201D}") // right double quote
    .replace("&lsquo;", "\u{2018}") // left single quote
    .replace("&rsquo;", "\u{2019}") // right single quote
    .replace("&hellip;", "\u{2026}") // horizontal ellipsis
    .replace("&bull;", "\u{2022}") // bullet
    .replace("&middot;", "\u{00B7}") // middle dot
    .replace("&deg;", "\u{00B0}") // degree sign
    .replace("&copy;", "\u{00A9}") // copyright
    .replace("&reg;", "\u{00AE}") // registered trademark
    .replace("&trade;", "\u{2122}") // trademark
    .replace("&times;", "\u{00D7}") // multiplication sign
    .replace("&divide;", "\u{00F7}") // division sign
    .replace("&plusmn;", "\u{00B1}") // plus-minus sign
    .replace("&ne;", "\u{2260}") // not equal
    .replace("&le;", "\u{2264}") // less than or equal
    .replace("&ge;", "\u{2265}") // greater than or equal
    .replace("&larr;", "\u{2190}") // leftwards arrow
    .replace("&rarr;", "\u{2192}") // rightwards arrow
    .replace("&uarr;", "\u{2191}") // upwards arrow
    .replace("&darr;", "\u{2193}") // downwards arrow
}

/// Download images for a page
///
/// Downloads all images referenced in the page content and saves them
/// to the specified output directory.
///
/// Returns a map of original filenames to local file paths (relative to output
/// root).
pub fn download_images(
  client: &dyn ConfluenceApi,
  page_id: &str,
  image_refs: &[ImageReference],
  output_dir: &Path,
  images_subdir: &str,
  overwrite: bool,
) -> Result<HashMap<String, PathBuf>> {
  let mut filename_map = HashMap::new();

  if image_refs.is_empty() {
    return Ok(filename_map);
  }

  // Get all attachments for the page
  let attachments = client
    .get_attachments(page_id)
    .context("Failed to fetch page attachments")?;

  // Create images directory
  let images_dir = output_dir.join(images_subdir);
  std::fs::create_dir_all(&images_dir).context("Failed to create images directory")?;

  // Download each image
  for image_ref in image_refs {
    // Find the attachment matching this image
    let attachment = attachments
      .iter()
      .find(|a| a.title == image_ref.filename)
      .with_context(|| format!("Attachment not found: {}", image_ref.filename))?;

    // Sanitize filename for filesystem
    let safe_filename = sanitize_filename(&image_ref.filename);
    let output_path = images_dir.join(&safe_filename);

    // Skip if file exists and overwrite is false
    if output_path.exists() && !overwrite {
      // Still add to map
      let relative_path = PathBuf::from(images_subdir).join(&safe_filename);
      filename_map.insert(image_ref.filename.clone(), relative_path);
      continue;
    }

    // Get download URL from attachment links
    let download_url = attachment
      .links
      .as_ref()
      .and_then(|l| l.download.as_ref())
      .with_context(|| format!("No download link for attachment: {}", image_ref.filename))?;

    // Download the image
    client
      .download_attachment(download_url, &output_path)
      .with_context(|| format!("Failed to download image: {}", image_ref.filename))?;

    // Store the relative path (relative to output_dir)
    let relative_path = PathBuf::from(images_subdir).join(&safe_filename);
    filename_map.insert(image_ref.filename.clone(), relative_path);
  }

  Ok(filename_map)
}

/// Update markdown image links to reference local files
///
/// Replaces image URLs in the markdown with local file paths based on
/// the provided filename map.
pub fn update_markdown_image_links(markdown: &str, filename_map: &HashMap<String, PathBuf>) -> String {
  let mut result = markdown.to_string();

  // For each image in the map, replace the markdown link
  for (original_filename, local_path) in filename_map {
    // Convert local path to forward slashes for markdown
    let local_path_str = local_path.to_str().unwrap_or("").replace('\\', "/");

    // Pattern: ![alt text](anything containing original_filename)
    // We need to be careful to match the right image references
    // The markdown converter creates links like: ![alt text]()
    // We need to replace the empty () with the local path

    // Find all occurrences of the filename in the markdown
    let pattern = format!("]({original_filename})");
    result = result.replace(&pattern, &format!("]({local_path_str})"));

    // Also handle the case where it might be wrapped in other URL context
    let pattern_empty = "![]()";
    if result.contains(pattern_empty) {
      // This is trickier - we'd need to match alt text to filename
      // For now, we'll handle the simpler case where filename is in the URL
    }
  }

  result
}

/// Sanitize a filename for safe filesystem storage
///
/// Removes or replaces characters that might cause issues on various
/// filesystems.
fn sanitize_filename(filename: &str) -> String {
  filename
    .chars()
    .map(|c| match c {
      '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
      c => c,
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_extract_image_references_with_attachment() {
    let storage = r#"
      <ac:image ac:alt="diagram">
        <ri:attachment ri:filename="architecture-diagram.png" />
      </ac:image>
    "#;

    let refs = extract_image_references(storage).unwrap();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].filename, "architecture-diagram.png");
    assert_eq!(refs[0].alt_text, "diagram");
  }

  #[test]
  fn test_extract_image_references_no_images() {
    let storage = "<p>Just some text</p>";
    let refs = extract_image_references(storage).unwrap();
    assert!(refs.is_empty());
  }

  #[test]
  fn test_extract_image_references_multiple() {
    let storage = r#"
      <ac:image ac:alt="first">
        <ri:attachment ri:filename="image1.png" />
      </ac:image>
      <p>Some text</p>
      <ac:image ac:alt="second">
        <ri:attachment ri:filename="image2.jpg" />
      </ac:image>
    "#;

    let refs = extract_image_references(storage).unwrap();
    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0].filename, "image1.png");
    assert_eq!(refs[1].filename, "image2.jpg");
  }

  #[test]
  fn test_extract_image_references_default_alt() {
    let storage = r#"
      <ac:image>
        <ri:attachment ri:filename="test.png" />
      </ac:image>
    "#;

    let refs = extract_image_references(storage).unwrap();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].alt_text, "image");
  }

  #[test]
  fn test_sanitize_filename() {
    assert_eq!(sanitize_filename("normal.png"), "normal.png");
    assert_eq!(sanitize_filename("file/with/slashes.png"), "file_with_slashes.png");
    assert_eq!(sanitize_filename("file:with:colons.png"), "file_with_colons.png");
    assert_eq!(sanitize_filename("file*with?chars.png"), "file_with_chars.png");
  }

  #[test]
  fn test_update_markdown_image_links() {
    let markdown = "![diagram](architecture-diagram.png)\n![photo](photo.jpg)";
    let mut map = HashMap::new();
    map.insert(
      "architecture-diagram.png".to_string(),
      PathBuf::from("images/architecture-diagram.png"),
    );
    map.insert("photo.jpg".to_string(), PathBuf::from("images/photo.jpg"));

    let result = update_markdown_image_links(markdown, &map);
    assert!(result.contains("](images/architecture-diagram.png)"));
    assert!(result.contains("](images/photo.jpg)"));
  }

  #[test]
  fn test_update_markdown_no_images() {
    let markdown = "Just some text without images";
    let map = HashMap::new();
    let result = update_markdown_image_links(markdown, &map);
    assert_eq!(result, markdown);
  }
}
