//! Page download workflow.
//!
//! Implements the default command that fetches one or more Confluence pages,
//! converts them to Markdown, downloads assets, and persists everything to
//! disk according to the current CLI settings.

use std::path::Path;
use std::sync::Arc;
use std::{fs, process};

use anyhow::Context;
use futures::future::join_all;
use tokio::sync::Semaphore;

use crate::asciidoc::AsciiDocOptions;
use crate::cli::Cli;
use crate::color::ColorScheme;
use crate::commands::auth::load_credentials;
use crate::confluence::{self, ConfluenceApi};
use crate::format::OutputFormat;
use crate::markdown::MarkdownOptions;
use crate::processed_page::{ProcessOptions, process_page, sanitize_filename, write_processed_page, write_raw_storage};

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
pub async fn handle_page_download(page_input: &str, cli: &Cli, colors: &ColorScheme) {
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

  // Get storage content for size display and raw saving
  let storage_content = page
    .body
    .as_ref()
    .and_then(|b| b.storage.as_ref())
    .map(|s| s.value.as_str());

  if cli.behavior.verbose > 0
    && let Some(content) = storage_content
  {
    println!(
      "  {}: {} characters",
      colors.dimmed("Content size"),
      colors.number(content.len())
    );
  }

  let output_dir = Path::new(&cli.output.output);

  // Save raw Confluence storage format BEFORE parsing if requested.
  // This ensures we can debug parse failures.
  if cli.output.save_raw
    && let Some(content) = storage_content
  {
    let filename = sanitize_filename(&page.title);
    let raw_path = write_raw_storage(output_dir, &filename, content, cli.output.overwrite)?;
    if cli.behavior.verbose > 0 {
      println!(
        "  {} {}",
        colors.dimmed("→"),
        colors.dimmed(format!("Raw: {}", raw_path.display()))
      );
    }
  }

  // Convert to target format
  let format_name = match cli.output.format {
    OutputFormat::Markdown => "Markdown",
    OutputFormat::AsciiDoc => "AsciiDoc",
  };
  println!(
    "\n{} {}",
    colors.info("→"),
    colors.info(format!("Converting to {format_name}"))
  );

  // Process the page (API calls + conversion)
  // Note: save_raw=false since we've already saved it above
  let mut process_options = build_process_options(cli);
  process_options.save_raw = false;
  let processed = process_page(&client, &page, &process_options).await?;

  if cli.behavior.verbose > 0 {
    println!(
      "  {}: {} characters",
      colors.dimmed(format!("{format_name} size")),
      colors.number(processed.content.len())
    );
  }

  // Log image/attachment processing
  if cli.images_links.download_images {
    println!("\n{} {}", colors.info("→"), colors.info("Processing images"));
    if !processed.images.is_empty() {
      println!(
        "  {} Processed {} {}",
        colors.success("✓"),
        colors.number(processed.images.len()),
        if processed.images.len() == 1 { "image" } else { "images" }
      );
    } else {
      println!("  {}", colors.dimmed("No images found in page"));
    }
  }

  if cli.page.attachments {
    println!("\n{} {}", colors.info("→"), colors.info("Processing attachments"));
    if !processed.attachments.is_empty() {
      println!(
        "  {} Processed {} {}",
        colors.success("✓"),
        colors.number(processed.attachments.len()),
        if processed.attachments.len() == 1 {
          "attachment"
        } else {
          "attachments"
        }
      );
    } else {
      println!("  {}", colors.dimmed("No attachments found in page"));
    }
  }

  // Write to disk (I/O phase)
  println!("\n{} {}", colors.info("→"), colors.info("Writing to disk"));
  let output_path = write_processed_page(&processed, output_dir, cli.output.format, cli.output.overwrite)?;
  println!("  {}: {}", colors.emphasis("File"), colors.path(output_path.display()));

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

    let page = &tree.page;

    if cli.behavior.verbose > 0 {
      println!(
        "{}   {} {}",
        colors.progress("→"),
        colors.dimmed(format!("Depth {}", tree.depth)),
        colors.info(&page.title)
      );
    }

    // Save raw Confluence storage format BEFORE parsing if requested.
    // This ensures we can debug parse failures.
    let filename = sanitize_filename(&page.title);
    if cli.output.save_raw
      && let Some(storage_content) = page.body.as_ref().and_then(|b| b.storage.as_ref())
    {
      let raw_path = write_raw_storage(output_dir, &filename, &storage_content.value, cli.output.overwrite)?;
      if cli.behavior.verbose > 0 {
        println!(
          "    {} {}",
          colors.dimmed("→"),
          colors.dimmed(format!("Raw: {}", raw_path.display()))
        );
      }
    }

    // Process the page (API calls + conversion)
    // Note: save_raw=false since we've already saved it above
    let mut process_options = build_process_options(cli);
    process_options.save_raw = false;
    let processed = process_page(client, page, &process_options).await?;

    if cli.behavior.verbose > 0 && !processed.attachments.is_empty() {
      println!(
        "    {} {}",
        colors.dimmed("Attachments:"),
        colors.number(processed.attachments.len())
      );
    } else if cli.behavior.verbose > 1 && cli.page.attachments && processed.attachments.is_empty() {
      println!("    {}", colors.dimmed("No attachments found"));
    }

    // Write processed page to disk (I/O phase)
    let output_path = write_processed_page(&processed, output_dir, cli.output.format, cli.output.overwrite)?;

    if !cli.behavior.quiet {
      println!("  {} {}", colors.success("✓"), colors.path(output_path.display()));
    }

    // Release permit before scheduling children so they can use the slot.
    drop(permit);

    // Download child pages recursively
    if !tree.children.is_empty() {
      // Create subdirectory for children (use our local filename since it matches processed.filename)
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

/// Build the processing options from CLI settings.
///
/// Creates a [`ProcessOptions`] struct that controls how pages are converted
/// and what assets are downloaded.
fn build_process_options(cli: &Cli) -> ProcessOptions {
  ProcessOptions {
    format: cli.output.format,
    save_raw: cli.output.save_raw,
    download_images: cli.images_links.download_images,
    images_dir: cli.images_links.images_dir.clone(),
    download_attachments: cli.page.attachments,
    markdown_options: build_markdown_options(cli),
    asciidoc_options: build_asciidoc_options(cli),
  }
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

/// Build the AsciiDoc conversion options from the CLI settings.
///
/// Currently propagates anchor preservation and compact table rendering flags.
fn build_asciidoc_options(cli: &Cli) -> AsciiDocOptions {
  AsciiDocOptions {
    preserve_anchors: cli.images_links.preserve_anchors,
    compact_tables: cli.output.compact_tables,
  }
}

/// Count the number of pages represented inside a [`confluence::PageTree`].
fn count_pages_in_tree(tree: &confluence::PageTree) -> usize {
  1 + tree.children.iter().map(count_pages_in_tree).sum::<usize>()
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
      let bytes = self.fetch_attachment(_url).await?;

      if let Some(parent) = output_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
      }

      tokio::fs::write(output_path, bytes).await?;
      Ok(())
    }

    async fn fetch_attachment(&self, _url: &str) -> Result<Vec<u8>> {
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

      let result = async {
        sleep(self.delay).await;
        Result::<Vec<u8>>::Ok(b"test-data".to_vec())
      }
      .await;

      {
        let mut guard = self.counter.lock().await;
        *guard -= 1;
      }

      result
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
        format: OutputFormat::Markdown,
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
        format: OutputFormat::Markdown,
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
