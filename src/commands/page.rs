//! Page download workflow.
//!
//! Implements the default command that fetches one or more Confluence pages,
//! converts them to Markdown, downloads assets, and persists everything to
//! disk according to the current CLI settings.

use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Arc;

use anyhow::Context;
use futures::future::join_all;
use tokio::sync::Semaphore;

use crate::cli::Cli;
use crate::color::ColorScheme;
use crate::commands::auth::load_credentials;
use crate::confluence::{self, ConfluenceApi};
use crate::markdown::{self, MarkdownOptions};
use crate::{attachments, images};

/// Execute the primary page download workflow.
///
/// The handler parses the supplied page identifier, resolves credentials, and
/// orchestrates downloads of pages, attachments, and images based on the
/// user's CLI flags.
///
/// # Arguments
/// * `page_input` - User-provided page URL or numeric Confluence page ID.
/// * `cli` - Parsed CLI options controlling behavior, output, and auth.
/// * `colors` - Shared color scheme for consistent terminal output.
pub(crate) async fn handle_page_download(page_input: &str, cli: &Cli, colors: &ColorScheme) {
  println!("{} {}", colors.progress("→"), colors.info("Downloading page"));
  println!("  {}: {}", colors.emphasis("URL"), colors.link(page_input));
  println!("  {}: {}", colors.emphasis("Output"), colors.path(&cli.output.output));

  if cli.page.children {
    println!("  {} {}", colors.success("✓"), colors.info("Including child pages"));
    if let Some(depth) = cli.page.max_depth {
      println!("    {} {}", colors.emphasis("Maximum depth:"), colors.number(depth));
    }
  }

  if cli.page.attachments {
    println!("  {} {}", colors.success("✓"), colors.info("Including attachments"));
  }

  if cli.behavior.dry_run {
    println!(
      "\n{} {}",
      colors.warning("⚠"),
      colors.warning("DRY RUN: No files will be downloaded")
    );
    return;
  }

  // Parse the input to extract page ID and base URL
  if let Err(e) = download_page(page_input, cli, colors).await {
    eprintln!("{} {}", colors.error("✗"), colors.error("Failed to download page"));
    eprintln!("  {}: {}", colors.emphasis("Error"), e);
    process::exit(1);
  }

  println!("\n{} {}", colors.success("✓"), colors.success("Download complete"));
}

/// Download a single Confluence page (optionally with attachments/children).
///
/// This helper performs the end-to-end export for one root page: authenticating
/// against Confluence, retrieving content, converting it to Markdown, and
/// writing files to disk. When `--children` is enabled it delegates to
/// [`download_page_tree`] after building the page tree.
///
/// # Arguments
/// * `page_input` - Page URL or numeric ID.
/// * `cli` - Parsed CLI options.
/// * `colors` - Color palette for progress output.
///
/// # Errors
/// Returns an error when any network call, filesystem write, or conversion
/// step fails.
async fn download_page(page_input: &str, cli: &Cli, colors: &ColorScheme) -> anyhow::Result<()> {
  // Parse URL to extract page ID and base URL
  let url_info = if page_input.contains("://") {
    // It's a URL
    confluence::parse_confluence_url(page_input)?
  } else {
    // It's a page ID - need base URL from --url
    if let Some(ref base_url) = cli.auth.url {
      confluence::UrlInfo {
        base_url: base_url.clone(),
        page_id: page_input.to_string(),
        space_key: None,
      }
    } else {
      anyhow::bail!("--url is required when using a numeric page ID");
    }
  };

  println!("\n{} {}", colors.info("→"), colors.info("Extracting page information"));
  println!("  {}: {}", colors.emphasis("Base URL"), colors.link(&url_info.base_url));
  println!("  {}: {}", colors.emphasis("Page ID"), colors.number(&url_info.page_id));
  if let Some(ref space) = url_info.space_key {
    println!("  {}: {}", colors.emphasis("Space"), space);
  }

  // Load credentials
  let (username, token) = load_credentials(&url_info.base_url, cli)?;

  // Create API client
  println!("\n{} {}", colors.info("→"), colors.info("Connecting to Confluence"));
  let client = confluence::ConfluenceClient::new(
    &url_info.base_url,
    &username,
    &token,
    cli.performance.timeout,
    cli.performance.rate_limit,
  )?;

  // Check if we should download children
  if cli.page.children {
    println!("{} {}", colors.info("→"), colors.info("Fetching page tree"));

    let max_depth = cli.page.max_depth;
    if let Some(depth) = max_depth {
      println!("  {}: {}", colors.emphasis("Max depth"), colors.number(depth));
    }

    let tree = confluence::get_page_tree(&client, &url_info.page_id, max_depth).await?;

    let total_pages = count_pages_in_tree(&tree);
    println!(
      "  {} Found {} {}",
      colors.success("✓"),
      colors.number(total_pages),
      if total_pages == 1 { "page" } else { "pages" }
    );

    // Download the entire tree
    println!("\n{} {}", colors.info("→"), colors.info("Downloading pages"));
    if cli.behavior.verbose > 0 {
      let parallel_label = cli.performance.parallel_label();
      println!(
        "  {}: {}",
        colors.dimmed("Parallel limit"),
        colors.number(parallel_label)
      );
    }
    let output_dir = Path::new(&cli.output.output);
    let parallel_limit = cli.performance.resolved_parallel();
    let semaphore = Arc::new(Semaphore::new(parallel_limit));
    download_page_tree(&client, &tree, output_dir, cli, colors, semaphore).await?;

    return Ok(());
  }

  // Fetch single page (non-children mode)
  println!("{} {}", colors.info("→"), colors.info("Fetching page content"));
  let page = client.get_page(&url_info.page_id).await?;

  println!("  {}: {}", colors.emphasis("Title"), colors.emphasis(&page.title));
  println!("  {}: {}", colors.emphasis("Type"), &page.page_type);
  println!("  {}: {}", colors.emphasis("Status"), &page.status);

  // Get the storage content
  let storage_content = page
    .body
    .as_ref()
    .and_then(|b| b.storage.as_ref())
    .map(|s| s.value.as_str())
    .ok_or_else(|| anyhow::anyhow!("Page has no storage content"))?;

  let filename = sanitize_filename(&page.title);

  if cli.output.save_raw {
    let raw_output_path = write_raw_storage(Path::new(&cli.output.output), &filename, storage_content)?;
    if cli.behavior.verbose > 0 {
      println!(
        "  {} {}",
        colors.dimmed("→"),
        colors.dimmed(format!("Raw: {}", raw_output_path.display()))
      );
    }
  }

  if cli.behavior.verbose > 0 {
    println!(
      "  {}: {} characters",
      colors.dimmed("Content size"),
      colors.number(storage_content.len())
    );
  }

  // Convert to Markdown
  println!("\n{} {}", colors.info("→"), colors.info("Converting to Markdown"));
  let options = build_markdown_options(cli);
  let mut markdown = markdown::storage_to_markdown_with_options(storage_content, &options)?;

  if cli.behavior.verbose > 0 {
    println!(
      "  {}: {} characters",
      colors.dimmed("Markdown size"),
      colors.number(markdown.len())
    );
  }

  let mut downloaded_image_filenames = HashSet::new();

  // Download images if requested
  if cli.images_links.download_images {
    println!("\n{} {}", colors.info("→"), colors.info("Processing images"));

    // Extract image references from storage content
    let image_refs = images::extract_image_references(storage_content)?;

    if !image_refs.is_empty() {
      println!(
        "  {}: {} {}",
        colors.emphasis("Found"),
        colors.number(image_refs.len()),
        if image_refs.len() == 1 { "image" } else { "images" }
      );

      // Download images
      let output_dir = Path::new(&cli.output.output);
      let filename_map = images::download_images(
        &client,
        &url_info.page_id,
        &image_refs,
        output_dir,
        &cli.images_links.images_dir,
        cli.output.overwrite,
      )
      .await?;

      println!(
        "  {} Downloaded {} {}",
        colors.success("✓"),
        colors.number(filename_map.len()),
        if filename_map.len() == 1 { "image" } else { "images" }
      );

      downloaded_image_filenames.extend(filename_map.keys().cloned());

      // Update markdown links to reference local files
      markdown = images::update_markdown_image_links(&markdown, &filename_map);
    } else {
      println!("  {}", colors.dimmed("No images found in page"));
    }
  }

  if cli.page.attachments {
    println!("\n{} {}", colors.info("→"), colors.info("Downloading attachments"));

    let skip_titles = if downloaded_image_filenames.is_empty() {
      None
    } else {
      Some(&downloaded_image_filenames)
    };

    let downloaded_attachments = attachments::download_attachments(
      &client,
      &url_info.page_id,
      Path::new(&cli.output.output),
      cli.output.overwrite,
      skip_titles,
    )
    .await?;

    if downloaded_attachments.is_empty() {
      println!("  {}", colors.dimmed("No attachments found in page"));
    } else {
      println!(
        "  {} Downloaded {} {}",
        colors.success("✓"),
        colors.number(downloaded_attachments.len()),
        if downloaded_attachments.len() == 1 {
          "attachment"
        } else {
          "attachments"
        }
      );

      markdown = attachments::update_markdown_attachment_links(&markdown, &downloaded_attachments);
    }
  }

  // Create output directory
  fs::create_dir_all(&cli.output.output).with_context(|| {
    format!(
      "Failed to create output directory {}",
      Path::new(&cli.output.output).display()
    )
  })?;

  let output_path = Path::new(&cli.output.output).join(format!("{filename}.md"));

  // Check if file exists and handle overwrite
  if output_path.exists() && !cli.output.overwrite {
    anyhow::bail!(
      "File already exists: {}. Use --overwrite to replace it.",
      output_path.display()
    );
  }

  // Write to file
  println!("\n{} {}", colors.info("→"), colors.info("Writing to disk"));
  println!("  {}: {}", colors.emphasis("File"), colors.path(output_path.display()));

  fs::write(&output_path, markdown)
    .with_context(|| format!("Failed to write markdown to {}", output_path.display()))?;

  Ok(())
}

/// Recursively download and render every node in a [`confluence::PageTree`].
///
/// The traversal enforces the configured parallelism with a semaphore so that
/// API calls and filesystem writes stay within resource constraints. Each page
/// is converted to Markdown, attachments/images are optionally downloaded, and
/// children are written to nested directories mirroring the tree shape.
///
/// # Arguments
/// * `client` - Confluence API implementation to fetch content from.
/// * `tree` - Current tree node describing the page and its descendants.
/// * `output_dir` - Root directory under which files for this node are stored.
/// * `cli` - Parsed CLI settings controlling behavior.
/// * `colors` - Color palette for log output.
/// * `semaphore` - Shared limiter controlling concurrent downloads.
///
/// # Returns
/// A future resolving once the tree rooted at `tree` is fully written.
///
/// # Errors
/// Returns an error when API calls fail, when data is missing required fields,
/// or when filesystem interactions cannot be completed.
fn download_page_tree<'a>(
  client: &'a dyn ConfluenceApi,
  tree: &'a confluence::PageTree,
  output_dir: &'a Path,
  cli: &'a Cli,
  colors: &'a ColorScheme,
  semaphore: Arc<Semaphore>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + 'a + Send>> {
  Box::pin(async move {
    let permit = semaphore
      .clone()
      .acquire_owned()
      .await
      .map_err(|_| anyhow::anyhow!("Parallel download limiter became unavailable"))?;

    // Download the current page
    let page = &tree.page;

    if cli.behavior.verbose > 0 {
      println!(
        "{}   {} {}",
        colors.progress("→"),
        colors.dimmed(format!("Depth {}", tree.depth)),
        colors.info(&page.title)
      );
    }

    // Get the storage content
    let storage_content = page
      .body
      .as_ref()
      .and_then(|b| b.storage.as_ref())
      .map(|s| s.value.as_str())
      .ok_or_else(|| anyhow::anyhow!("Page has no storage content"))?;

    // Generate filename from page title (needed for raw XML saving)
    let filename = sanitize_filename(&page.title);

    // Save raw Confluence storage format BEFORE parsing if requested
    // This ensures we can debug parse failures
    if cli.output.save_raw {
      let raw_output_path = write_raw_storage(output_dir, &filename, storage_content)?;

      if cli.behavior.verbose > 0 {
        println!(
          "    {} {}",
          colors.dimmed("→"),
          colors.dimmed(format!("Raw: {}", raw_output_path.display()))
        );
      }
    }

    // Convert to Markdown
    let options = build_markdown_options(cli);
    let mut markdown = markdown::storage_to_markdown_with_options(storage_content, &options)
      .map_err(|e| anyhow::anyhow!("Failed to convert page '{}' to markdown: {}", page.title, e))?;

    let mut downloaded_image_filenames = HashSet::new();

    // Download images if requested
    if cli.images_links.download_images {
      let image_refs = images::extract_image_references(storage_content)?;

      if !image_refs.is_empty() {
        let filename_map = images::download_images(
          client,
          &page.id,
          &image_refs,
          output_dir,
          &cli.images_links.images_dir,
          cli.output.overwrite,
        )
        .await?;

        downloaded_image_filenames.extend(filename_map.keys().cloned());

        markdown = images::update_markdown_image_links(&markdown, &filename_map);
      }
    }

    if cli.page.attachments {
      let skip_titles = if downloaded_image_filenames.is_empty() {
        None
      } else {
        Some(&downloaded_image_filenames)
      };

      let downloaded_attachments =
        attachments::download_attachments(client, &page.id, output_dir, cli.output.overwrite, skip_titles).await?;

      if !downloaded_attachments.is_empty() {
        if cli.behavior.verbose > 0 {
          println!(
            "    {} {}",
            colors.dimmed("Attachments:"),
            colors.number(downloaded_attachments.len())
          );
        }
        markdown = attachments::update_markdown_attachment_links(&markdown, &downloaded_attachments);
      } else if cli.behavior.verbose > 1 {
        println!("    {}", colors.dimmed("No attachments found"));
      }
    }

    // Generate output path
    let output_path = output_dir.join(format!("{filename}.md"));

    // Create parent directory
    if let Some(parent) = output_path.parent() {
      fs::create_dir_all(parent).with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    if cli.output.overwrite {
      // Write markdown to file
      fs::write(&output_path, &markdown)
        .with_context(|| format!("Failed to write markdown to {}", output_path.display()))?;
    } else {
      match OpenOptions::new().write(true).create_new(true).open(&output_path) {
        Ok(mut file) => {
          file
            .write_all(markdown.as_bytes())
            .with_context(|| format!("Failed to write markdown to {}", output_path.display()))?;
        }
        Err(err) if err.kind() == ErrorKind::AlreadyExists => {
          let message = format!(
            "File already exists: {}. Use --overwrite to replace it.",
            output_path.display()
          );

          eprintln!("{} {}", colors.error("✗"), colors.error(&message));

          anyhow::bail!(message);
        }
        Err(err) => {
          anyhow::bail!("Failed to create markdown file {}: {}", output_path.display(), err);
        }
      }
    }

    if !cli.behavior.quiet {
      println!("  {} {}", colors.success("✓"), colors.path(output_path.display()));
    }

    // Raw XML was already saved before parsing (if requested)

    // Release permit before scheduling children so they can use the slot.
    drop(permit);

    // Download child pages recursively
    if !tree.children.is_empty() {
      // Create subdirectory for children
      let child_dir = output_dir.join(&filename);
      fs::create_dir_all(&child_dir)
        .with_context(|| format!("Failed to create directory for child pages at {}", child_dir.display()))?;

      let child_futures = tree
        .children
        .iter()
        .map(|child_tree| download_page_tree(client, child_tree, &child_dir, cli, colors, Arc::clone(&semaphore)));

      for result in join_all(child_futures).await {
        result?;
      }
    }

    Ok(())
  })
}

/// Persist the raw Confluence storage payload next to the Markdown export.
fn write_raw_storage(output_dir: &Path, filename: &str, storage_content: &str) -> anyhow::Result<PathBuf> {
  let raw_output_path = output_dir.join(format!("{filename}.raw.xml"));
  if let Some(parent) = raw_output_path.parent() {
    fs::create_dir_all(parent)
      .with_context(|| format!("Failed to create directory for raw storage at {}", parent.display()))?;
  }

  fs::write(&raw_output_path, storage_content)
    .with_context(|| format!("Failed to write raw storage to {}", raw_output_path.display()))?;

  Ok(raw_output_path)
}

/// Build the Markdown conversion options from the CLI settings.
///
/// Currently propagates anchor preservation and compact table rendering flags.
fn build_markdown_options(cli: &Cli) -> MarkdownOptions {
  MarkdownOptions {
    preserve_anchors: cli.images_links.preserve_anchors,
    compact_tables: cli.output.compact_tables,
  }
}

/// Count the number of pages represented inside a [`confluence::PageTree`].
fn count_pages_in_tree(tree: &confluence::PageTree) -> usize {
  1 + tree.children.iter().map(count_pages_in_tree).sum::<usize>()
}

/// Sanitize a Confluence page title so it can be used as a filesystem name.
///
/// Removes/normalizes characters that are potentially unsafe across
/// platforms, collapsing repeated whitespace while keeping readability.
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

#[cfg(test)]
mod tests {
  use std::collections::HashMap;
  use std::path::PathBuf;
  use std::sync::Arc;
  use std::time::Duration;

  use anyhow::{Result, bail};
  use async_trait::async_trait;
  use tempfile::tempdir;
  use tokio::sync::Mutex;
  use tokio::time::sleep;

  use super::*;
  use crate::cli::{
    AuthOptions, BehaviorOptions, Cli, ColorOption, ImagesLinksOptions, OutputOptions, PageOptions, PerformanceOptions,
  };
  use crate::color::ColorScheme;
  use crate::confluence::{
    Attachment, AttachmentLinks, ConfluenceApi, Page, PageBody, PageTree, StorageFormat, UserInfo,
  };

  struct CountingClient {
    attachments: HashMap<String, Vec<Attachment>>,
    counter: Arc<Mutex<usize>>,
    max_counter: Arc<Mutex<usize>>,
    delay: Duration,
  }

  impl CountingClient {
    fn new(counter: Arc<Mutex<usize>>, max_counter: Arc<Mutex<usize>>, delay: Duration) -> Self {
      Self {
        attachments: HashMap::new(),
        counter,
        max_counter,
        delay,
      }
    }

    fn set_attachments(&mut self, page_id: &str, attachments: Vec<Attachment>) {
      self.attachments.insert(page_id.to_string(), attachments);
    }
  }

  #[async_trait]
  impl ConfluenceApi for CountingClient {
    async fn get_page(&self, page_id: &str) -> Result<Page> {
      bail!("get_page unexpectedly called for {}", page_id);
    }

    async fn get_child_pages(&self, _page_id: &str) -> Result<Vec<Page>> {
      Ok(Vec::new())
    }

    async fn get_attachments(&self, page_id: &str) -> Result<Vec<Attachment>> {
      Ok(self.attachments.get(page_id).cloned().unwrap_or_default())
    }

    async fn download_attachment(&self, _url: &str, output_path: &std::path::Path) -> Result<()> {
      let current = {
        let mut guard = self.counter.lock().await;
        *guard += 1;
        let current = *guard;
        drop(guard);
        current
      };

      {
        let mut max_guard = self.max_counter.lock().await;
        if current > *max_guard {
          *max_guard = current;
        }
      }

      let io_result = async {
        sleep(self.delay).await;

        if let Some(parent) = output_path.parent() {
          tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(output_path, b"test-data").await?;
        Result::<()>::Ok(())
      }
      .await;

      {
        let mut guard = self.counter.lock().await;
        *guard -= 1;
      }

      io_result
    }

    async fn test_auth(&self) -> Result<UserInfo> {
      bail!("test_auth unexpectedly called");
    }
  }

  fn make_page(id: &str, title: &str) -> Page {
    Page {
      id: id.to_string(),
      title: title.to_string(),
      page_type: "page".to_string(),
      status: "current".to_string(),
      body: Some(PageBody {
        storage: Some(StorageFormat {
          value: "<p>Example</p>".to_string(),
          representation: "storage".to_string(),
        }),
        view: None,
      }),
      space: None,
      links: None,
    }
  }

  fn make_attachment(page_id: &str) -> Attachment {
    Attachment {
      id: format!("{page_id}-attachment"),
      title: format!("{page_id}.dat"),
      attachment_type: "attachment".to_string(),
      media_type: Some("application/octet-stream".to_string()),
      file_size: Some(12),
      links: Some(AttachmentLinks {
        download: Some(format!("https://example.com/{page_id}")),
      }),
    }
  }

  fn build_tree() -> PageTree {
    let children: Vec<PageTree> = (0..4)
      .map(|idx| {
        let page_id = format!("child-{idx}");
        PageTree {
          page: make_page(&page_id, &format!("Child {}", idx)),
          children: Vec::new(),
          depth: 1,
        }
      })
      .collect();

    PageTree {
      page: make_page("root", "Root Page"),
      children,
      depth: 0,
    }
  }

  #[test]
  fn write_raw_storage_creates_file_with_content() {
    let temp_dir = tempdir().unwrap();
    let nested_dir = temp_dir.path().join("raw").join("pages");
    let content = "<p>Example</p>";

    let saved_path = write_raw_storage(&nested_dir, "Example Page", content).expect("raw storage should be saved");

    assert_eq!(
      saved_path,
      nested_dir.join("Example Page.raw.xml"),
      "raw output path should live under the provided directory"
    );
    assert_eq!(fs::read_to_string(&saved_path).unwrap(), content);
  }

  #[tokio::test]
  async fn download_page_tree_writes_raw_storage_when_enabled() {
    let temp_dir = tempdir().unwrap();
    let output_dir = temp_dir.path();

    let counter = Arc::new(Mutex::new(0));
    let max_counter = Arc::new(Mutex::new(0));
    let client = CountingClient::new(
      Arc::clone(&counter),
      Arc::clone(&max_counter),
      Duration::from_millis(10),
    );

    let tree = PageTree {
      page: make_page("root", "Root Page"),
      children: Vec::new(),
      depth: 0,
    };

    let colors = ColorScheme::new(ColorOption::Never);
    let cli = Cli {
      page_input: None,
      command: None,
      auth: AuthOptions {
        url: None,
        user: None,
        token: None,
      },
      output: OutputOptions {
        output: output_dir.to_string_lossy().to_string(),
        overwrite: true,
        save_raw: true,
        compact_tables: false,
      },
      behavior: BehaviorOptions {
        dry_run: false,
        verbose: 0,
        quiet: true,
        color: ColorOption::Never,
      },
      page: PageOptions {
        children: true,
        max_depth: None,
        attachments: false,
      },
      images_links: ImagesLinksOptions {
        download_images: false,
        images_dir: "images".to_string(),
        preserve_anchors: false,
      },
      performance: PerformanceOptions {
        parallel: 2,
        rate_limit: 10,
        timeout: 30,
      },
    };

    let semaphore = Arc::new(Semaphore::new(cli.performance.resolved_parallel()));
    download_page_tree(&client, &tree, output_dir, &cli, &colors, semaphore)
      .await
      .expect("download should succeed");

    let raw_file = output_dir.join("Root Page.raw.xml");
    assert!(raw_file.exists(), "raw storage file should be created");
    assert_eq!(fs::read_to_string(&raw_file).unwrap(), "<p>Example</p>");
  }

  #[tokio::test]
  async fn download_page_tree_respects_parallel_limit() {
    let temp_dir = tempdir().unwrap();
    let output_path = temp_dir.path();

    let counter = Arc::new(Mutex::new(0));
    let max_counter = Arc::new(Mutex::new(0));
    let mut client = CountingClient::new(
      Arc::clone(&counter),
      Arc::clone(&max_counter),
      Duration::from_millis(50),
    );

    let tree = build_tree();

    let mut attachment_map_ids = vec!["root".to_string()];
    attachment_map_ids.extend((0..4).map(|idx| format!("child-{idx}")));

    for page_id in attachment_map_ids {
      client.set_attachments(&page_id, vec![make_attachment(&page_id)]);
    }

    let client = client;
    let colors = ColorScheme::new(ColorOption::Never);

    let cli = Cli {
      page_input: None,
      command: None,
      auth: AuthOptions {
        url: None,
        user: None,
        token: None,
      },
      output: OutputOptions {
        output: output_path.to_string_lossy().to_string(),
        overwrite: true,
        save_raw: false,
        compact_tables: false,
      },
      behavior: BehaviorOptions {
        dry_run: false,
        verbose: 0,
        quiet: true,
        color: ColorOption::Never,
      },
      page: PageOptions {
        children: true,
        max_depth: None,
        attachments: true,
      },
      images_links: ImagesLinksOptions {
        download_images: false,
        images_dir: "images".to_string(),
        preserve_anchors: false,
      },
      performance: PerformanceOptions {
        parallel: 2,
        rate_limit: 10,
        timeout: 30,
      },
    };

    let limit = cli.performance.resolved_parallel();
    let semaphore = Arc::new(Semaphore::new(limit));
    download_page_tree(&client, &tree, output_path, &cli, &colors, semaphore)
      .await
      .expect("download should succeed");

    let max = *max_counter.lock().await;
    assert!(max <= limit, "observed concurrency {max} exceeds limit {}", limit);

    // Ensure files were written for each page.
    let expected_files: Vec<PathBuf> = vec![
      output_path.join("Root Page.md"),
      output_path.join("Root Page").join("Child 0.md"),
      output_path.join("Root Page").join("Child 1.md"),
      output_path.join("Root Page").join("Child 2.md"),
      output_path.join("Root Page").join("Child 3.md"),
    ];

    for file in expected_files {
      assert!(file.exists(), "expected output file {} to exist", file.display());
    }
  }
}
