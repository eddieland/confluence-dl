//! Page processing types that separate API/conversion logic from file I/O.
//!
//! This module provides the [`ProcessedPage`] struct which represents a fully
//! processed Confluence page ready to be written to disk. It separates the
//! concerns of fetching and converting content from persisting it to the
//! filesystem.

use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write as IoWrite};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::asciidoc::{self, AsciiDocOptions};
use crate::attachments::{self, ATTACHMENTS_DIR, DownloadedAttachment};
use crate::confluence::{ConfluenceApi, Page};
use crate::format::OutputFormat;
use crate::images::{self, ImageReference};
use crate::markdown::{self, MarkdownOptions};

/// Data about an asset (image or attachment) ready to be written to disk.
#[derive(Debug, Clone)]
pub struct AssetData {
  /// The relative path where this asset should be written (e.g., "images/photo.png").
  pub relative_path: PathBuf,
  /// The raw bytes of the asset.
  pub content: Vec<u8>,
}

/// A fully processed page ready to be written to disk.
///
/// This struct contains all the data needed to persist a page and its assets
/// without requiring any further API calls or transformations. It enables
/// clean separation between the processing phase (API calls + conversion) and
/// the I/O phase (writing files to disk).
#[derive(Debug, Clone)]
pub struct ProcessedPage {
  /// Sanitized filename (without extension) for the output file.
  pub filename: String,
  /// The final converted content (Markdown or AsciiDoc) with all links
  /// rewritten to reference local asset files.
  pub content: String,
  /// Optional raw Confluence storage format content for debugging.
  pub raw_storage: Option<String>,
  /// Images to write to disk.
  pub images: Vec<AssetData>,
  /// Attachments to write to disk.
  pub attachments: Vec<AssetData>,
}

/// Options controlling how a page should be processed.
#[derive(Debug, Clone)]
pub struct ProcessOptions {
  /// The target output format (Markdown or AsciiDoc).
  pub format: OutputFormat,
  /// Whether to preserve raw storage content for debugging.
  pub save_raw: bool,
  /// Whether to download images referenced in the page.
  pub download_images: bool,
  /// Subdirectory name for storing downloaded images.
  pub images_dir: String,
  /// Whether to download attachments.
  pub download_attachments: bool,
  /// Markdown-specific conversion options.
  pub markdown_options: MarkdownOptions,
  /// AsciiDoc-specific conversion options.
  pub asciidoc_options: AsciiDocOptions,
}

impl Default for ProcessOptions {
  fn default() -> Self {
    Self {
      format: OutputFormat::Markdown,
      save_raw: false,
      download_images: false,
      images_dir: "images".to_string(),
      download_attachments: false,
      markdown_options: MarkdownOptions::default(),
      asciidoc_options: AsciiDocOptions::default(),
    }
  }
}

/// Process a Confluence page into a [`ProcessedPage`] ready for writing.
///
/// This function performs all API calls needed to fetch images and attachments,
/// converts the storage content to the target format, and rewrites links to
/// reference local files. The result can be written to disk using
/// [`write_processed_page`].
///
/// # Arguments
/// * `client` - Confluence API client for fetching attachments.
/// * `page` - The page to process (must have storage content).
/// * `options` - Processing options controlling conversion and downloads.
///
/// # Returns
/// A [`ProcessedPage`] containing all data needed to write the page to disk.
pub async fn process_page(client: &dyn ConfluenceApi, page: &Page, options: &ProcessOptions) -> Result<ProcessedPage> {
  let storage_content = page
    .body
    .as_ref()
    .and_then(|b| b.storage.as_ref())
    .map(|s| s.value.as_str())
    .ok_or_else(|| anyhow::anyhow!("Page '{}' has no storage content", page.title))?;

  let filename = sanitize_filename(&page.title);

  // Convert to target format
  let mut output_content = match options.format {
    OutputFormat::Markdown => markdown::storage_to_markdown_with_options(storage_content, &options.markdown_options)
      .map_err(|e| anyhow::anyhow!("Failed to convert page '{}' to markdown: {}", page.title, e))?,
    OutputFormat::AsciiDoc => asciidoc::storage_to_asciidoc_with_options(storage_content, &options.asciidoc_options)
      .map_err(|e| anyhow::anyhow!("Failed to convert page '{}' to asciidoc: {}", page.title, e))?,
  };

  let mut images = Vec::new();
  let mut downloaded_image_filenames = HashSet::new();

  // Process images if requested
  if options.download_images {
    let image_refs = images::extract_image_references(storage_content)?;

    if !image_refs.is_empty() {
      let (downloaded_images, filename_map) = fetch_images(client, &page.id, &image_refs, &options.images_dir).await?;

      images = downloaded_images;
      downloaded_image_filenames.extend(filename_map.keys().cloned());

      // Update content with local image paths
      output_content = match options.format {
        OutputFormat::Markdown => images::update_markdown_image_links(&output_content, &filename_map),
        OutputFormat::AsciiDoc => images::update_asciidoc_image_links(&output_content, &filename_map),
      };
    }
  }

  let mut attachments_data = Vec::new();

  // Process attachments if requested
  if options.download_attachments {
    let skip_titles = if downloaded_image_filenames.is_empty() {
      None
    } else {
      Some(&downloaded_image_filenames)
    };

    let (fetched_attachments, downloaded_info) = fetch_attachments(client, &page.id, skip_titles).await?;

    attachments_data = fetched_attachments;

    if !downloaded_info.is_empty() {
      output_content = attachments::update_markdown_attachment_links(&output_content, &downloaded_info);
    }
  }

  let raw_storage = if options.save_raw {
    Some(storage_content.to_string())
  } else {
    None
  };

  Ok(ProcessedPage {
    filename,
    content: output_content,
    raw_storage,
    images,
    attachments: attachments_data,
  })
}

/// Write a processed page to disk.
///
/// This function handles all filesystem I/O for persisting a page and its
/// assets. It creates necessary directories, writes asset files, and writes
/// the main page content.
///
/// # Arguments
/// * `page` - The processed page data to write.
/// * `output_dir` - The directory where the page and assets should be written.
/// * `format` - The output format (determines file extension).
/// * `overwrite` - Whether to overwrite existing files.
///
/// # Returns
/// The path to the written page file on success.
pub fn write_processed_page(
  page: &ProcessedPage,
  output_dir: &Path,
  format: OutputFormat,
  overwrite: bool,
) -> Result<PathBuf> {
  // Create output directory
  fs::create_dir_all(output_dir)
    .with_context(|| format!("Failed to create output directory {}", output_dir.display()))?;

  // Write images
  for image in &page.images {
    let image_path = output_dir.join(&image.relative_path);
    write_asset(&image_path, &image.content, overwrite)?;
  }

  // Write attachments
  for attachment in &page.attachments {
    let attachment_path = output_dir.join(&attachment.relative_path);
    write_asset(&attachment_path, &attachment.content, overwrite)?;
  }

  // Write raw storage if present
  if let Some(ref raw_storage) = page.raw_storage {
    let raw_path = output_dir.join(format!("{}.raw.xml", page.filename));
    write_file(&raw_path, raw_storage.as_bytes(), overwrite)?;
  }

  // Write main content
  let extension = format.file_extension();
  let output_path = output_dir.join(format!("{}.{}", page.filename, extension));
  write_file(&output_path, page.content.as_bytes(), overwrite)?;

  Ok(output_path)
}

/// Result of processing a page, containing both the processed page and
/// information about what was downloaded (useful for logging).
#[derive(Debug)]
pub struct ProcessResult {
  /// The processed page ready for writing.
  pub page: ProcessedPage,
  /// Number of images that were fetched.
  pub image_count: usize,
  /// Number of attachments that were fetched.
  pub attachment_count: usize,
}

/// Fetch images referenced in a page and return their data along with a
/// filename mapping for link rewriting.
async fn fetch_images(
  client: &dyn ConfluenceApi,
  page_id: &str,
  image_refs: &[ImageReference],
  images_subdir: &str,
) -> Result<(Vec<AssetData>, HashMap<String, PathBuf>)> {
  let mut assets = Vec::new();
  let mut filename_map = HashMap::new();

  if image_refs.is_empty() {
    return Ok((assets, filename_map));
  }

  // Get all attachments for the page to find image URLs
  let attachments = client
    .get_attachments(page_id)
    .await
    .context("Failed to fetch page attachments")?;

  for image_ref in image_refs {
    // Find the attachment matching this image
    let attachment = attachments
      .iter()
      .find(|a| a.title == image_ref.filename)
      .with_context(|| format!("Attachment not found: {}", image_ref.filename))?;

    let download_url = attachment
      .links
      .as_ref()
      .and_then(|l| l.download.as_ref())
      .with_context(|| format!("No download link for attachment: {}", image_ref.filename))?;

    // Fetch the image bytes
    let bytes = client
      .fetch_attachment(download_url)
      .await
      .with_context(|| format!("Failed to fetch image: {}", image_ref.filename))?;

    // Sanitize filename and build relative path
    let safe_filename = sanitize_asset_filename(&image_ref.filename);
    let relative_path = PathBuf::from(images_subdir).join(&safe_filename);

    filename_map.insert(image_ref.filename.clone(), relative_path.clone());
    assets.push(AssetData {
      relative_path,
      content: bytes,
    });
  }

  Ok((assets, filename_map))
}

/// Fetch attachments for a page and return their data along with metadata for
/// link rewriting.
async fn fetch_attachments(
  client: &dyn ConfluenceApi,
  page_id: &str,
  skip_titles: Option<&HashSet<String>>,
) -> Result<(Vec<AssetData>, Vec<DownloadedAttachment>)> {
  let mut assets = Vec::new();
  let mut downloaded_info = Vec::new();

  let attachments = client
    .get_attachments(page_id)
    .await
    .context("Failed to fetch page attachments")?;

  if attachments.is_empty() {
    return Ok((assets, downloaded_info));
  }

  let mut used_filenames = HashSet::new();

  for attachment in attachments {
    // Skip if this attachment was already downloaded as an image
    if let Some(skip) = skip_titles
      && skip.contains(&attachment.title) {
        continue;
      }

    let download_url = match attachment.links.as_ref().and_then(|links| links.download.as_ref()) {
      Some(url) => url,
      None => continue,
    };

    // Sanitize and deduplicate filename
    let sanitized = sanitize_asset_filename(&attachment.title);
    let (base, ext) = split_name_and_extension(&sanitized);
    let mut filename = sanitized.clone();
    let mut counter = 1;

    while used_filenames.contains(&filename) {
      filename = next_candidate(&base, &ext, counter);
      counter += 1;
    }
    used_filenames.insert(filename.clone());

    // Fetch the attachment bytes
    let bytes = client
      .fetch_attachment(download_url)
      .await
      .with_context(|| format!("Failed to fetch attachment: {}", attachment.title))?;

    let relative_path = PathBuf::from(ATTACHMENTS_DIR).join(&filename);

    downloaded_info.push(DownloadedAttachment {
      original_name: attachment.title.clone(),
      relative_path: relative_path.clone(),
    });

    assets.push(AssetData {
      relative_path,
      content: bytes,
    });
  }

  Ok((assets, downloaded_info))
}

/// Write an asset file to disk, creating parent directories as needed.
fn write_asset(path: &Path, content: &[u8], overwrite: bool) -> Result<()> {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).with_context(|| format!("Failed to create directory {}", parent.display()))?;
  }
  write_file(path, content, overwrite)
}

/// Write a file to disk, respecting the overwrite setting.
fn write_file(path: &Path, content: &[u8], overwrite: bool) -> Result<()> {
  if overwrite {
    fs::write(path, content).with_context(|| format!("Failed to write {}", path.display()))?;
  } else {
    match OpenOptions::new().write(true).create_new(true).open(path) {
      Ok(mut file) => {
        file
          .write_all(content)
          .with_context(|| format!("Failed to write {}", path.display()))?;
      }
      Err(err) if err.kind() == ErrorKind::AlreadyExists => {
        bail!(
          "File already exists: {}. Use --overwrite to replace it.",
          path.display()
        );
      }
      Err(err) => {
        bail!("Failed to create file {}: {}", path.display(), err);
      }
    }
  }
  Ok(())
}

/// Sanitize a Confluence page title so it can be used as a filesystem name.
fn sanitize_filename(title: &str) -> String {
  title
    .chars()
    .map(|c| {
      if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
        c
      } else {
        '_'
      }
    })
    .collect::<String>()
    .replace("  ", " ")
    .trim()
    .to_string()
}

/// Sanitize an asset filename for safe filesystem storage.
fn sanitize_asset_filename(filename: &str) -> String {
  filename
    .chars()
    .map(|c| match c {
      '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
      c => c,
    })
    .collect()
}

fn split_name_and_extension(name: &str) -> (String, String) {
  if let Some((stem, ext)) = name.rsplit_once('.') {
    (stem.to_string(), ext.to_string())
  } else {
    (name.to_string(), String::new())
  }
}

fn next_candidate(base: &str, ext: &str, counter: usize) -> String {
  if ext.is_empty() {
    format!("{base}-{counter}")
  } else {
    format!("{base}-{counter}.{ext}")
  }
}

#[cfg(test)]
mod tests {
  use tempfile::tempdir;

  use super::*;

  #[test]
  fn test_sanitize_filename() {
    assert_eq!(sanitize_filename("Hello World"), "Hello World");
    assert_eq!(sanitize_filename("Test/Page"), "Test_Page");
    assert_eq!(sanitize_filename("Page: Overview"), "Page_ Overview");
    assert_eq!(sanitize_filename("  Spaced  "), "Spaced");
  }

  #[test]
  fn test_sanitize_asset_filename() {
    assert_eq!(sanitize_asset_filename("normal.png"), "normal.png");
    assert_eq!(
      sanitize_asset_filename("file/with/slashes.png"),
      "file_with_slashes.png"
    );
    assert_eq!(sanitize_asset_filename("file:with:colons.png"), "file_with_colons.png");
  }

  #[test]
  fn test_write_file_creates_new_file() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    write_file(&file_path, b"hello", false).unwrap();

    assert!(file_path.exists());
    assert_eq!(fs::read_to_string(&file_path).unwrap(), "hello");
  }

  #[test]
  fn test_write_file_fails_without_overwrite() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("existing.txt");

    fs::write(&file_path, "original").unwrap();

    let result = write_file(&file_path, b"new content", false);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
  }

  #[test]
  fn test_write_file_overwrites_with_flag() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("existing.txt");

    fs::write(&file_path, "original").unwrap();
    write_file(&file_path, b"new content", true).unwrap();

    assert_eq!(fs::read_to_string(&file_path).unwrap(), "new content");
  }

  #[test]
  fn test_write_processed_page_creates_all_files() {
    let temp_dir = tempdir().unwrap();
    let output_dir = temp_dir.path();

    let page = ProcessedPage {
      filename: "Test Page".to_string(),
      content: "# Test\n\nContent".to_string(),
      raw_storage: Some("<p>Test</p>".to_string()),
      images: vec![AssetData {
        relative_path: PathBuf::from("images/test.png"),
        content: b"PNG".to_vec(),
      }],
      attachments: vec![AssetData {
        relative_path: PathBuf::from("attachments/doc.pdf"),
        content: b"PDF".to_vec(),
      }],
    };

    let result = write_processed_page(&page, output_dir, OutputFormat::Markdown, true);
    assert!(result.is_ok());

    let written_path = result.unwrap();
    assert_eq!(written_path, output_dir.join("Test Page.md"));
    assert!(written_path.exists());
    assert_eq!(fs::read_to_string(&written_path).unwrap(), "# Test\n\nContent");

    // Check raw storage
    let raw_path = output_dir.join("Test Page.raw.xml");
    assert!(raw_path.exists());
    assert_eq!(fs::read_to_string(&raw_path).unwrap(), "<p>Test</p>");

    // Check images
    let image_path = output_dir.join("images/test.png");
    assert!(image_path.exists());
    assert_eq!(fs::read(&image_path).unwrap(), b"PNG");

    // Check attachments
    let attachment_path = output_dir.join("attachments/doc.pdf");
    assert!(attachment_path.exists());
    assert_eq!(fs::read(&attachment_path).unwrap(), b"PDF");
  }

  #[test]
  fn test_write_processed_page_asciidoc_extension() {
    let temp_dir = tempdir().unwrap();
    let output_dir = temp_dir.path();

    let page = ProcessedPage {
      filename: "Test".to_string(),
      content: "= Test".to_string(),
      raw_storage: None,
      images: vec![],
      attachments: vec![],
    };

    let result = write_processed_page(&page, output_dir, OutputFormat::AsciiDoc, true);
    assert!(result.is_ok());

    let written_path = result.unwrap();
    assert_eq!(written_path, output_dir.join("Test.adoc"));
  }

  #[test]
  fn test_split_name_and_extension() {
    let (base, ext) = split_name_and_extension("report.pdf");
    assert_eq!(base, "report");
    assert_eq!(ext, "pdf");

    let (base, ext) = split_name_and_extension("README");
    assert_eq!(base, "README");
    assert_eq!(ext, "");
  }

  #[test]
  fn test_next_candidate() {
    assert_eq!(next_candidate("file", "txt", 1), "file-1.txt");
    assert_eq!(next_candidate("file", "txt", 2), "file-2.txt");
    assert_eq!(next_candidate("file", "", 1), "file-1");
  }
}
