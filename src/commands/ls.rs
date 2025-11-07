//! `ls` subcommand for printing Confluence page hierarchies.
//!
//! This module powers `confluence-dl ls`, which connects to Confluence, builds
//! the page tree for a target page, and renders the hierarchy in a friendly
//! ASCII tree without downloading any content.

use std::process;

use anyhow::{Context, Result, anyhow};

use crate::cli::Cli;
use crate::color::ColorScheme;
use crate::commands::auth::load_credentials;
use crate::confluence::{self, PageTree};

/// Execute the `ls` subcommand to display a page tree.
///
/// This handler parses the page reference, resolves credentials, fetches the
/// remote page hierarchy, and prints the structure using Unix `ls -R`-like
/// formatting. The command never writes to disk, making it safe to run with or
/// without `--dry-run`.
///
/// # Arguments
/// * `target` - Page URL or numeric page ID supplied on the CLI.
/// * `max_depth` - Optional traversal depth limit (0 lists only the root).
/// * `cli` - Top-level CLI options for auth, behavior, and networking.
/// * `colors` - Shared color palette used to render terminal output.
pub(crate) async fn handle_ls_command(target: &str, max_depth: Option<usize>, cli: &Cli, colors: &ColorScheme) {
  if let Err(error) = run_ls_command(target, max_depth, cli, colors).await {
    eprintln!("{} {}", colors.error("✗"), colors.error("Failed to list page tree"));
    eprintln!("  {}: {}", colors.emphasis("Error"), error);
    process::exit(1);
  }
}

async fn run_ls_command(target: &str, max_depth: Option<usize>, cli: &Cli, colors: &ColorScheme) -> Result<()> {
  println!("{} {}", colors.progress("→"), colors.info("Inspecting page tree"));

  let url_info = resolve_url_info(target.trim(), cli).context("Could not determine page identifier")?;

  println!("  {}: {}", colors.emphasis("Base URL"), colors.link(&url_info.base_url));
  println!("  {}: {}", colors.emphasis("Page ID"), colors.number(&url_info.page_id));
  if let Some(space) = &url_info.space_key {
    println!("  {}: {}", colors.emphasis("Space"), colors.emphasis(space));
  }
  if let Some(depth) = max_depth {
    println!("  {}: {}", colors.emphasis("Max depth"), colors.number(depth));
  }

  let (username, token) = load_credentials(&url_info.base_url, cli)
    .context("Failed to resolve credentials. Provide --user/--token, env vars, or configure ~/.netrc")?;

  println!("\n{} {}", colors.info("→"), colors.info("Connecting to Confluence"));
  let client = confluence::ConfluenceClient::new(
    &url_info.base_url,
    &username,
    &token,
    cli.performance.timeout,
    cli.performance.rate_limit,
  )
  .context("Unable to construct Confluence API client")?;

  println!("{} {}", colors.info("→"), colors.info("Fetching page tree"));
  let tree = confluence::get_page_tree(&client, &url_info.page_id, max_depth).await?;

  let total_pages = count_nodes(&tree);
  println!(
    "  {} {}",
    colors.success("✓"),
    colors.info(format!(
      "Found {} {}",
      colors.number(total_pages),
      if total_pages == 1 { "page" } else { "pages" }
    ))
  );

  if cli.behavior.dry_run {
    println!(
      "\n{} {}",
      colors.warning("⚠"),
      colors.warning("--dry-run has no effect for `ls`; nothing is written to disk")
    );
  }

  println!("\n{}", colors.emphasis("Page Tree"));
  for line in format_tree_lines(&tree, colors) {
    println!("  {line}");
  }

  Ok(())
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
    "--url is required when using a numeric page ID (e.g., confluence-dl ls 123456 --url https://example.atlassian.net)"
  ))
}

fn format_tree_lines(tree: &PageTree, colors: &ColorScheme) -> Vec<String> {
  let mut lines = Vec::new();
  format_tree_lines_recursive(tree, String::new(), true, true, colors, &mut lines);
  lines
}

fn format_tree_lines_recursive(
  node: &PageTree,
  prefix: String,
  is_last: bool,
  is_root: bool,
  colors: &ColorScheme,
  lines: &mut Vec<String>,
) {
  let connector = if is_root {
    String::new()
  } else if is_last {
    format!("{prefix}└── ")
  } else {
    format!("{prefix}├── ")
  };

  let line = if is_root {
    format!(
      "{} {}",
      colors.emphasis(&node.page.title),
      format_metadata(node, colors)
    )
  } else {
    format!(
      "{}{} {}",
      connector,
      colors.emphasis(&node.page.title),
      format_metadata(node, colors)
    )
  };
  lines.push(line);

  let next_prefix = if is_root {
    prefix
  } else if is_last {
    format!("{prefix}    ")
  } else {
    format!("{prefix}│   ")
  };

  for (idx, child) in node.children.iter().enumerate() {
    let child_is_last = idx + 1 == node.children.len();
    format_tree_lines_recursive(child, next_prefix.clone(), child_is_last, false, colors, lines);
  }
}

fn format_metadata(node: &PageTree, colors: &ColorScheme) -> String {
  format!(
    "[id {} | depth {} | status {} | type {}]",
    colors.number(&node.page.id),
    colors.number(node.depth),
    colors.dimmed(&node.page.status),
    colors.dimmed(&node.page.page_type)
  )
}

fn count_nodes(tree: &PageTree) -> usize {
  1 + tree.children.iter().map(count_nodes).sum::<usize>()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::cli::ColorOption;
  use crate::color::ColorScheme;
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

  fn make_tree() -> PageTree {
    PageTree {
      page: make_page("1", "Root"),
      depth: 0,
      children: vec![
        PageTree {
          page: make_page("2", "Child A"),
          depth: 1,
          children: vec![PageTree {
            page: make_page("3", "Grandchild"),
            depth: 2,
            children: vec![],
          }],
        },
        PageTree {
          page: make_page("4", "Child B"),
          depth: 1,
          children: vec![],
        },
      ],
    }
  }

  #[test]
  fn test_format_tree_lines_structure() {
    let colors = ColorScheme::new(ColorOption::Never);
    let tree = make_tree();

    let lines = format_tree_lines(&tree, &colors);
    assert_eq!(lines.len(), 4);
    assert!(lines[0].starts_with("Root [id 1"));
    assert_eq!(
      lines[1].trim_start(),
      "├── Child A [id 2 | depth 1 | status current | type page]"
    );
    assert_eq!(
      lines[2].trim_start(),
      "│   └── Grandchild [id 3 | depth 2 | status current | type page]"
    );
    assert_eq!(
      lines[3].trim_start(),
      "└── Child B [id 4 | depth 1 | status current | type page]"
    );
  }

  #[test]
  fn test_count_nodes() {
    let tree = make_tree();
    assert_eq!(count_nodes(&tree), 4);
  }
}
