use std::path::Path;
use std::{fs, process};

use crate::cli::Cli;
use crate::color::ColorScheme;
use crate::commands::auth::load_credentials;
use crate::confluence::{self, ConfluenceApi};
use crate::{images, markdown};

/// Handle page download
pub(crate) fn handle_page_download(page_input: &str, cli: &Cli, colors: &ColorScheme) {
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

  if cli.page.comments {
    println!("  {} {}", colors.success("✓"), colors.info("Including comments"));
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
  if let Err(e) = download_page(page_input, cli, colors) {
    eprintln!("{} {}", colors.error("✗"), colors.error("Failed to download page"));
    eprintln!("  {}: {}", colors.emphasis("Error"), e);
    process::exit(1);
  }

  println!("\n{} {}", colors.success("✓"), colors.success("Download complete"));
}

/// Download a page and save it to disk
fn download_page(page_input: &str, cli: &Cli, colors: &ColorScheme) -> anyhow::Result<()> {
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
  let client = confluence::ConfluenceClient::new(&url_info.base_url, &username, &token, cli.performance.timeout)?;

  // Check if we should download children
  if cli.page.children {
    println!("{} {}", colors.info("→"), colors.info("Fetching page tree"));

    let max_depth = cli.page.max_depth;
    if let Some(depth) = max_depth {
      println!("  {}: {}", colors.emphasis("Max depth"), colors.number(depth));
    }

    let tree = confluence::get_page_tree(&client, &url_info.page_id, max_depth)?;

    let total_pages = count_pages_in_tree(&tree);
    println!(
      "  {} Found {} {}",
      colors.success("✓"),
      colors.number(total_pages),
      if total_pages == 1 { "page" } else { "pages" }
    );

    // Download the entire tree
    println!("\n{} {}", colors.info("→"), colors.info("Downloading pages"));
    let output_dir = Path::new(&cli.output.output);
    download_page_tree(&client, &tree, output_dir, cli, colors)?;

    return Ok(());
  }

  // Fetch single page (non-children mode)
  println!("{} {}", colors.info("→"), colors.info("Fetching page content"));
  let page = client.get_page(&url_info.page_id)?;

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

  if cli.behavior.verbose > 0 {
    println!(
      "  {}: {} characters",
      colors.dimmed("Content size"),
      colors.number(storage_content.len())
    );
  }

  // Convert to Markdown
  println!("\n{} {}", colors.info("→"), colors.info("Converting to Markdown"));
  let mut markdown = markdown::storage_to_markdown(storage_content, cli.behavior.verbose)?;

  if cli.behavior.verbose > 0 {
    println!(
      "  {}: {} characters",
      colors.dimmed("Markdown size"),
      colors.number(markdown.len())
    );
  }

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
      )?;

      println!(
        "  {} Downloaded {} {}",
        colors.success("✓"),
        colors.number(filename_map.len()),
        if filename_map.len() == 1 { "image" } else { "images" }
      );

      // Update markdown links to reference local files
      markdown = images::update_markdown_image_links(&markdown, &filename_map);
    } else {
      println!("  {}", colors.dimmed("No images found in page"));
    }
  }

  // Create output directory
  fs::create_dir_all(&cli.output.output)?;

  // Generate filename from page title
  let filename = sanitize_filename(&page.title);
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

  fs::write(&output_path, markdown)?;

  Ok(())
}

/// Download a page tree recursively
fn download_page_tree(
  client: &dyn ConfluenceApi,
  tree: &confluence::PageTree,
  output_dir: &Path,
  cli: &Cli,
  colors: &ColorScheme,
) -> anyhow::Result<()> {
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
    let raw_output_path = output_dir.join(format!("{filename}.raw.xml"));
    if let Some(parent) = raw_output_path.parent() {
      fs::create_dir_all(parent)?;
    }
    fs::write(&raw_output_path, storage_content)?;

    if cli.behavior.verbose > 0 {
      println!(
        "    {} {}",
        colors.dimmed("→"),
        colors.dimmed(format!("Raw: {}", raw_output_path.display()))
      );
    }
  }

  // Convert to Markdown
  let mut markdown = markdown::storage_to_markdown(storage_content, cli.behavior.verbose)
    .map_err(|e| anyhow::anyhow!("Failed to convert page '{}' to markdown: {}", page.title, e))?;

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
      )?;

      markdown = images::update_markdown_image_links(&markdown, &filename_map);
    }
  }

  // Generate output path
  let output_path = output_dir.join(format!("{filename}.md"));

  // Check if file exists and handle overwrite
  if output_path.exists() && !cli.output.overwrite {
    let message = format!(
      "File already exists: {}. Use --overwrite to replace it.",
      output_path.display()
    );

    eprintln!("{} {}", colors.error("✗"), colors.error(&message));

    anyhow::bail!(message);
  } else {
    // Create parent directory
    if let Some(parent) = output_path.parent() {
      fs::create_dir_all(parent)?;
    }

    // Write markdown to file
    fs::write(&output_path, markdown)?;

    if !cli.behavior.quiet {
      println!("  {} {}", colors.success("✓"), colors.path(output_path.display()));
    }

    // Raw XML was already saved before parsing (if requested)
  }

  // Download child pages recursively
  if !tree.children.is_empty() {
    // Create subdirectory for children
    let child_dir = output_dir.join(&filename);
    fs::create_dir_all(&child_dir)?;

    for child_tree in &tree.children {
      download_page_tree(client, child_tree, &child_dir, cli, colors)?;
    }
  }

  Ok(())
}

/// Count total pages in a page tree (including root and all descendants)
fn count_pages_in_tree(tree: &confluence::PageTree) -> usize {
  1 + tree.children.iter().map(count_pages_in_tree).sum::<usize>()
}

/// Sanitize a page title to create a valid filename
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
