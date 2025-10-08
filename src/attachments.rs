//! Attachment download helpers.
//!
//! Provides utilities for downloading Confluence attachments and updating
//! Markdown content to reference local files.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tokio::fs;
use tracing::warn;

use crate::confluence::{Attachment, ConfluenceApi};

/// Default directory name where attachments are stored relative to the page
/// output directory.
pub const ATTACHMENTS_DIR: &str = "attachments";

/// Represents an attachment downloaded from Confluence.
#[derive(Debug, Clone)]
pub struct DownloadedAttachment {
  /// Original filename reported by the Confluence API.
  pub original_name: String,
  /// Relative path (from the page output directory) to the downloaded file.
  pub relative_path: PathBuf,
}

/// Download all attachments associated with a page.
///
/// # Arguments
/// * `client` - Confluence API client used for metadata and downloads.
/// * `page_id` - Identifier of the page whose attachments should be fetched.
/// * `output_dir` - Directory where the Markdown file for the page is written.
/// * `overwrite` - When `true`, existing files are replaced.
/// * `skip_titles` - Optional set of attachment titles that should be skipped
///   (typically image filenames already handled separately).
pub async fn download_attachments(
  client: &dyn ConfluenceApi,
  page_id: &str,
  output_dir: &Path,
  overwrite: bool,
  skip_titles: Option<&HashSet<String>>,
) -> Result<Vec<DownloadedAttachment>> {
  let attachments = client
    .get_attachments(page_id)
    .await
    .context("Failed to fetch page attachments")?;

  if attachments.is_empty() {
    return Ok(Vec::new());
  }

  let attachments_dir = output_dir.join(ATTACHMENTS_DIR);
  fs::create_dir_all(&attachments_dir)
    .await
    .with_context(|| format!("Failed to create attachments directory {}", attachments_dir.display()))?;

  let mut downloaded = Vec::new();
  let mut used_filenames = HashSet::new();

  for attachment in attachments {
    if should_skip(&attachment, skip_titles) {
      continue;
    }

    let download_url = match attachment.links.as_ref().and_then(|links| links.download.as_ref()) {
      Some(url) => url,
      None => {
        warn!(
          "Skipping attachment '{}' because no download link was provided",
          attachment.title
        );
        continue;
      }
    };

    let unique_filename = generate_unique_filename(&attachment.title, &attachments_dir, &mut used_filenames);
    let output_path = attachments_dir.join(&unique_filename);

    if output_path.exists() && !overwrite {
      downloaded.push(DownloadedAttachment {
        original_name: attachment.title.clone(),
        relative_path: PathBuf::from(ATTACHMENTS_DIR).join(&unique_filename),
      });
      continue;
    }

    client
      .download_attachment(download_url, &output_path)
      .await
      .with_context(|| format!("Failed to download attachment {}", attachment.title))?;

    downloaded.push(DownloadedAttachment {
      original_name: attachment.title,
      relative_path: PathBuf::from(ATTACHMENTS_DIR).join(unique_filename),
    });
  }

  Ok(downloaded)
}

/// Update Markdown links that reference attachment filenames to point at the
/// downloaded files.
pub fn update_markdown_attachment_links(markdown: &str, attachments: &[DownloadedAttachment]) -> String {
  let mut result = markdown.to_string();

  for attachment in attachments {
    let local_path = attachment
      .relative_path
      .to_str()
      .map(|s| s.replace('\\', "/"))
      .unwrap_or_default();

    let search = format!("]({})", attachment.original_name);
    let replacement = format!("]({local_path})");
    result = result.replace(&search, &replacement);
  }

  result
}

fn should_skip(attachment: &Attachment, skip_titles: Option<&HashSet<String>>) -> bool {
  if let Some(skip) = skip_titles {
    skip.contains(&attachment.title)
  } else {
    false
  }
}

fn generate_unique_filename(name: &str, target_dir: &Path, used_filenames: &mut HashSet<String>) -> String {
  let mut candidate = sanitize_filename(name);
  let (base, ext) = split_name_and_extension(&candidate);
  let mut counter = 1;

  while used_filenames.contains(&candidate) || target_dir.join(&candidate).exists() {
    candidate = if ext.is_empty() {
      format!("{base}-{counter}")
    } else {
      format!("{base}-{counter}.{ext}")
    };
    counter += 1;
  }

  used_filenames.insert(candidate.clone());
  candidate
}

fn split_name_and_extension(name: &str) -> (String, String) {
  if let Some((stem, ext)) = name.rsplit_once('.') {
    (stem.to_string(), ext.to_string())
  } else {
    (name.to_string(), String::new())
  }
}

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
  fn test_split_name_and_extension_with_extension() {
    let (base, ext) = split_name_and_extension("report.pdf");
    assert_eq!(base, "report");
    assert_eq!(ext, "pdf");
  }

  #[test]
  fn test_split_name_and_extension_without_extension() {
    let (base, ext) = split_name_and_extension("README");
    assert_eq!(base, "README");
    assert_eq!(ext, "");
  }

  #[test]
  fn test_sanitize_filename_removes_illegal_chars() {
    let sanitized = sanitize_filename("report:<draft>.pdf");
    assert_eq!(sanitized, "report__draft_.pdf");
  }
}
