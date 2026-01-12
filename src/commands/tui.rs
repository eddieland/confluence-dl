//! Experimental TUI for browsing and downloading Confluence content.
//!
//! This module provides a lightweight, prompt-driven terminal UI that lets
//! users inspect a page tree and kick off downloads on demand. The initial
//! implementation favors simplicity over richness so the experience can be
//! iterated quickly.

use std::io::{self, Write};
use std::process;

use anyhow::{Result, anyhow};

use crate::cli::Cli;
use crate::color::ColorScheme;
use crate::commands::auth::load_credentials;
use crate::commands::page::handle_page_download;
use crate::confluence::{self, PageTree};

/// Launch the experimental TUI for a space homepage or root page.
///
/// # Arguments
/// * `target` - Page URL or numeric page ID to use as the browsing root.
/// * `max_depth` - Maximum depth when building the page tree for navigation.
/// * `cli` - Parsed CLI options for authentication, behavior, and defaults.
/// * `colors` - Shared color palette used for output.
pub async fn handle_tui_command(target: &str, max_depth: usize, cli: &Cli, colors: &ColorScheme) {
  if let Err(error) = run_tui_command(target, max_depth, cli, colors).await {
    eprintln!("{} {}", colors.error("✗"), colors.error("Failed to launch TUI"));
    eprintln!("  {}: {}", colors.emphasis("Error"), error);
    process::exit(1);
  }
}

async fn run_tui_command(target: &str, max_depth: usize, cli: &Cli, colors: &ColorScheme) -> Result<()> {
  println!("{} {}", colors.progress("→"), colors.info("Launching TUI"));

  let url_info = resolve_url_info(target.trim(), cli)?;

  println!("  {}: {}", colors.emphasis("Base URL"), colors.link(&url_info.base_url));
  println!("  {}: {}", colors.emphasis("Page ID"), colors.number(&url_info.page_id));
  if let Some(space) = &url_info.space_key {
    println!("  {}: {}", colors.emphasis("Space"), colors.emphasis(space));
  }
  println!("  {}: {}", colors.emphasis("Max depth"), colors.number(max_depth));

  let (username, token) = load_credentials(&url_info.base_url, cli)?;

  println!("\n{} {}", colors.info("→"), colors.info("Connecting to Confluence"));
  let client = confluence::ConfluenceClient::new(
    &url_info.base_url,
    &username,
    &token,
    cli.performance.timeout,
    cli.performance.rate_limit,
  )?;

  println!("{} {}", colors.info("→"), colors.info("Fetching page tree"));
  let tree = confluence::get_page_tree(&client, &url_info.page_id, Some(max_depth)).await?;

  let entries = flatten_tree(&tree);
  let mut preferences = DownloadPreferences::from_cli(cli);

  loop {
    println!("\n{}", colors.emphasis("Pages"));
    render_entries(&entries, colors);
    render_preferences(&preferences, colors);

    let prompt = format!(
      "{} {}",
      colors.emphasis("Select a page number"),
      colors.dimmed("(s=settings, q=quit)")
    );
    let input = read_line(&prompt)?;

    if input.eq_ignore_ascii_case("q") {
      println!("{}", colors.info("Exiting TUI."));
      return Ok(());
    }

    if input.eq_ignore_ascii_case("s") {
      update_preferences(&mut preferences)?;
      continue;
    }

    let index: usize = match input.parse() {
      Ok(value) => value,
      Err(_) => {
        println!(
          "{} {}",
          colors.warning("⚠"),
          colors.warning("Enter a valid number, s, or q.")
        );
        continue;
      }
    };

    if index == 0 || index > entries.len() {
      println!(
        "{} {}",
        colors.warning("⚠"),
        colors.warning("Selection out of range; choose a valid page number.")
      );
      continue;
    }

    let entry = &entries[index - 1];
    println!(
      "\n{} {}",
      colors.progress("→"),
      colors.info(format!("Downloading \"{}\"", entry.title))
    );

    let mut cli_override = cli.clone();
    cli_override.auth.url = Some(url_info.base_url.clone());
    preferences.apply_to_cli(&mut cli_override);
    handle_page_download(&entry.id, &cli_override, colors).await;
  }
}

fn resolve_url_info(target: &str, cli: &Cli) -> Result<confluence::UrlInfo> {
  if target.contains("://") {
    return confluence::parse_confluence_url(target);
  }

  if let Some(base_url) = &cli.auth.url {
    return Ok(confluence::UrlInfo {
      base_url: base_url.trim_end_matches('/').to_string(),
      page_id: target.to_string(),
      space_key: None,
    });
  }

  Err(anyhow!(
    "--url is required when using a numeric page ID (e.g., confluence-dl tui 123456 --url https://example.atlassian.net)"
  ))
}

#[derive(Debug, Clone)]
struct DownloadPreferences {
  children: bool,
  max_depth: Option<usize>,
  attachments: bool,
  download_images: bool,
}

impl DownloadPreferences {
  /// Seed preferences from the current CLI defaults.
  fn from_cli(cli: &Cli) -> Self {
    Self {
      children: cli.page.children,
      max_depth: cli.page.max_depth,
      attachments: cli.page.attachments,
      download_images: cli.images_links.download_images,
    }
  }

  /// Apply the current preferences to a mutable CLI clone.
  fn apply_to_cli(&self, cli: &mut Cli) {
    cli.page.children = self.children;
    cli.page.max_depth = if self.children { self.max_depth } else { None };
    cli.page.attachments = self.attachments;
    cli.images_links.download_images = self.download_images;
  }
}

#[derive(Debug, Clone)]
struct TuiPageEntry {
  id: String,
  title: String,
  depth: usize,
}

/// Flatten a page tree into a pre-order list for menu rendering.
fn flatten_tree(tree: &PageTree) -> Vec<TuiPageEntry> {
  let mut entries = Vec::new();
  flatten_tree_recursive(tree, &mut entries);
  entries
}

fn flatten_tree_recursive(tree: &PageTree, entries: &mut Vec<TuiPageEntry>) {
  entries.push(TuiPageEntry {
    id: tree.page.id.clone(),
    title: tree.page.title.clone(),
    depth: tree.depth,
  });

  for child in &tree.children {
    flatten_tree_recursive(child, entries);
  }
}

fn render_entries(entries: &[TuiPageEntry], colors: &ColorScheme) {
  for (idx, entry) in entries.iter().enumerate() {
    let indent = "  ".repeat(entry.depth);
    println!(
      "{}{} {}",
      indent,
      colors.number(format!("[{}]", idx + 1)),
      colors.emphasis(&entry.title)
    );
  }
}

fn render_preferences(preferences: &DownloadPreferences, colors: &ColorScheme) {
  println!("\n{}", colors.emphasis("Download settings"));
  println!(
    "  {}: {}",
    colors.emphasis("Children"),
    colors.info(preferences.children.to_string())
  );
  println!(
    "  {}: {}",
    colors.emphasis("Max depth"),
    colors.info(
      preferences
        .max_depth
        .map_or("none".to_string(), |value| value.to_string())
    )
  );
  println!(
    "  {}: {}",
    colors.emphasis("Attachments"),
    colors.info(preferences.attachments.to_string())
  );
  println!(
    "  {}: {}",
    colors.emphasis("Images"),
    colors.info(preferences.download_images.to_string())
  );
}

fn update_preferences(preferences: &mut DownloadPreferences) -> Result<()> {
  println!("\nUpdate settings (press Enter to keep defaults)");
  preferences.children = prompt_bool("Download child pages", preferences.children)?;
  if preferences.children {
    preferences.max_depth = prompt_optional_usize("Max depth", preferences.max_depth)?;
  } else {
    preferences.max_depth = None;
  }
  preferences.attachments = prompt_bool("Download attachments", preferences.attachments)?;
  preferences.download_images = prompt_bool("Download images", preferences.download_images)?;
  Ok(())
}

fn prompt_bool(label: &str, default: bool) -> Result<bool> {
  let suffix = if default { "Y/n" } else { "y/N" };
  loop {
    let response = read_line(&format!("{label} [{suffix}]: "))?;
    if response.is_empty() {
      return Ok(default);
    }
    match response.as_str() {
      "y" | "Y" | "yes" | "YES" | "Yes" => return Ok(true),
      "n" | "N" | "no" | "NO" | "No" => return Ok(false),
      _ => println!("Please enter y or n."),
    }
  }
}

fn prompt_optional_usize(label: &str, default: Option<usize>) -> Result<Option<usize>> {
  let suffix = default.map_or("none".to_string(), |value| value.to_string());
  loop {
    let response = read_line(&format!("{label} [{suffix}]: "))?;
    if response.is_empty() {
      return Ok(default);
    }
    match response.parse::<usize>() {
      Ok(value) => return Ok(Some(value)),
      Err(_) => println!("Please enter a valid number or leave blank."),
    }
  }
}

fn read_line(prompt: &str) -> Result<String> {
  let mut input = String::new();
  print!("{prompt} ");
  io::stdout().flush()?;
  io::stdin().read_line(&mut input)?;
  Ok(input.trim().to_string())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::confluence::Page;

  fn make_page(id: &str, title: &str) -> Page {
    Page {
      id: id.to_string(),
      title: title.to_string(),
      page_type: "page".to_string(),
      status: "current".to_string(),
      body: None,
      space: None,
      links: None,
    }
  }

  #[test]
  fn test_flatten_tree_preorder() {
    let tree = PageTree {
      page: make_page("1", "Root"),
      depth: 0,
      children: vec![
        PageTree {
          page: make_page("2", "Child A"),
          depth: 1,
          children: vec![],
        },
        PageTree {
          page: make_page("3", "Child B"),
          depth: 1,
          children: vec![PageTree {
            page: make_page("4", "Grandchild"),
            depth: 2,
            children: vec![],
          }],
        },
      ],
    };

    let entries = flatten_tree(&tree);
    let titles: Vec<&str> = entries.iter().map(|entry| entry.title.as_str()).collect();
    assert_eq!(titles, vec!["Root", "Child A", "Child B", "Grandchild"]);
  }
}
