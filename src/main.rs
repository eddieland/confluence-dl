//! confluence-dl - Export Confluence pages to Markdown
//!
//! This is the main entry point for the CLI application.

mod cli;
mod color;
mod confluence;
mod credentials;
mod images;
mod markdown;

use std::path::Path;
use std::{fs, io, process};

use clap::CommandFactory;
use clap_complete::{Shell as CompletionShell, generate};
use cli::{AuthCommand, Cli, Command, Shell};
use color::ColorScheme;
use confluence::ConfluenceApi;
use credentials::{CredentialsProvider, NetrcProvider};

fn main() {
  let cli = Cli::parse_args();

  // Create color scheme based on user preference
  let colors = ColorScheme::new(cli.behavior.color);

  // Validate CLI arguments
  if let Err(e) = cli.validate() {
    eprintln!("{} {}", colors.error("Error:"), e);
    process::exit(4); // Invalid arguments exit code
  }

  // Handle subcommands
  if let Some(ref command) = cli.command {
    match command {
      Command::Auth { subcommand } => {
        handle_auth_command(subcommand, &cli, &colors);
      }
      Command::Version { json, short } => {
        handle_version_command(*json, *short, &colors);
      }
      Command::Completions { shell } => {
        handle_completions_command(*shell);
      }
    }
    return;
  }

  // Handle main page download functionality
  if let Some(ref page_input) = cli.page_input {
    handle_page_download(page_input, &cli, &colors);
  }
}

/// Handle authentication subcommands
fn handle_auth_command(subcommand: &AuthCommand, cli: &Cli, colors: &ColorScheme) {
  match subcommand {
    AuthCommand::Test => {
      println!("{}", colors.info("Testing authentication..."));
      // TODO: Implement authentication testing
      // 1. Load credentials from CLI args, env vars, or .netrc
      // 2. Make test API call to Confluence
      // 3. Display result with user info
      println!(
        "  {}: {}",
        colors.emphasis("URL"),
        colors.link(cli.auth.url.as_deref().unwrap_or("(not set)"))
      );
      println!(
        "  {}: {}",
        colors.emphasis("User"),
        cli.auth.user.as_deref().unwrap_or("(not set)")
      );
      println!(
        "  {}: {}",
        colors.emphasis("Token"),
        if cli.auth.token.is_some() {
          colors.dimmed("********")
        } else {
          colors.dimmed("(not set)")
        }
      );
      eprintln!("{} Authentication testing not yet implemented", colors.warning("Note:"));
    }
    AuthCommand::Show => {
      show_auth_config(cli, colors);
    }
  }
}

/// Display authentication configuration with source information
fn show_auth_config(cli: &Cli, colors: &ColorScheme) {
  println!("{}\n", colors.emphasis("Authentication Configuration"));

  // Determine the base URL
  let url = cli.auth.url.as_deref();
  let url_source = if std::env::var("CONFLUENCE_URL").is_ok() {
    "environment variable"
  } else if url.is_some() {
    "command-line flag"
  } else {
    "not set"
  };

  if let Some(url_value) = url {
    println!("{}: {}", colors.emphasis("Base URL"), colors.link(url_value));
    println!("  {}: {}", colors.dimmed("Source"), colors.dimmed(url_source));
  } else {
    println!("{}: {}", colors.emphasis("Base URL"), colors.dimmed("(not set)"));
  }

  // Determine username source
  let username = cli.auth.user.as_deref();
  let user_source = if std::env::var("CONFLUENCE_USER").is_ok() {
    "environment variable"
  } else if username.is_some() {
    "command-line flag"
  } else {
    "not set"
  };

  // Determine token source
  let token = cli.auth.token.as_deref();
  let token_source = if std::env::var("CONFLUENCE_TOKEN").is_ok() {
    "environment variable"
  } else if token.is_some() {
    "command-line flag"
  } else {
    "not set"
  };

  // Try to get credentials from .netrc if URL is provided but user/token are not
  let netrc_creds = if username.is_none() || token.is_none() {
    url.and_then(extract_host).and_then(|host| {
      let provider = NetrcProvider::new();
      provider.get_credentials(&host).ok().flatten()
    })
  } else {
    None
  };

  // Display username
  if let Some(user_value) = username {
    println!("\n{}: {}", colors.emphasis("Username"), user_value);
    println!("  {}: {}", colors.dimmed("Source"), colors.dimmed(user_source));
  } else if let Some(ref creds) = netrc_creds {
    println!("\n{}: {}", colors.emphasis("Username"), creds.username);
    println!("  {}: {}", colors.dimmed("Source"), colors.dimmed(".netrc file"));
  } else {
    println!("\n{}: {}", colors.emphasis("Username"), colors.dimmed("(not set)"));
  }

  // Display token
  if let Some(token_value) = token {
    let masked = if token_value.len() > 8 {
      format!("{}{}", &token_value[..4], "*".repeat(token_value.len() - 4))
    } else {
      "*".repeat(token_value.len())
    };
    println!("\n{}: {}", colors.emphasis("API Token"), colors.dimmed(&masked));
    println!(
      "  {}: {} characters",
      colors.dimmed("Length"),
      colors.number(token_value.len())
    );
    println!("  {}: {}", colors.dimmed("Source"), colors.dimmed(token_source));
  } else if netrc_creds.is_some() {
    // We have a password from .netrc but don't show it
    println!("\n{}: {}", colors.emphasis("API Token"), colors.dimmed("********"));
    println!("  {}: {}", colors.dimmed("Source"), colors.dimmed(".netrc file"));
  } else {
    println!("\n{}: {}", colors.emphasis("API Token"), colors.dimmed("(not set)"));
  }

  // Display .netrc information if found
  if netrc_creds.is_some() && (username.is_none() || token.is_none()) {
    println!("\n{} Credentials found in .netrc", colors.info("ℹ"));
    if let Some(host) = url.and_then(extract_host) {
      println!("  {}: {}", colors.dimmed("Host"), host);
    }
  }

  // Display warnings if credentials are incomplete
  if url.is_none() {
    println!(
      "\n{} {} is required for API access",
      colors.warning("⚠"),
      colors.emphasis("Base URL")
    );
    println!("  Set via --url flag or CONFLUENCE_URL environment variable");
  }

  let has_username = username.is_some() || netrc_creds.is_some();
  let has_token = token.is_some() || netrc_creds.is_some();

  if !has_username || !has_token {
    println!(
      "\n{} {} for API access",
      colors.warning("⚠"),
      colors.warning("Credentials incomplete")
    );
    if !has_username {
      println!("  Missing: username (use --user or CONFLUENCE_USER)");
    }
    if !has_token {
      println!("  Missing: API token (use --token or CONFLUENCE_TOKEN)");
    }
    println!("\n  Or add credentials to ~/.netrc:");
    if let Some(url_str) = url
      && let Some(host) = extract_host(url_str)
    {
      println!("    machine {host}");
    }
    println!("      login your.email@example.com");
    println!("      password your-api-token");
  } else {
    println!("\n{} {}", colors.success("✓"), colors.success("Credentials configured"));
  }
}

/// Extract hostname from a URL string
fn extract_host(url: &str) -> Option<String> {
  // Simple URL parsing to extract the host
  if let Some(start) = url.find("://") {
    let after_scheme = &url[start + 3..];
    if let Some(end) = after_scheme.find('/') {
      Some(after_scheme[..end].to_string())
    } else {
      Some(after_scheme.to_string())
    }
  } else {
    // No scheme, assume it's just a host
    if let Some(end) = url.find('/') {
      Some(url[..end].to_string())
    } else {
      Some(url.to_string())
    }
  }
}

/// Handle version command
fn handle_version_command(json: bool, short: bool, colors: &ColorScheme) {
  let version = env!("CARGO_PKG_VERSION");

  if short {
    println!("{version}");
    return;
  }

  if json {
    // Output JSON format (no colors in JSON)
    let git_hash = env!("GIT_HASH");
    let build_timestamp = env!("BUILD_TIMESTAMP");
    let target = env!("TARGET");

    println!("{{");
    println!("  \"version\": \"{version}\",");
    println!("  \"git_commit\": \"{git_hash}\",");
    println!("  \"build_timestamp\": \"{}\",", format_timestamp(build_timestamp));
    println!("  \"target\": \"{target}\",");
    println!("  \"rust_version\": \"{}\"", rustc_version());
    println!("}}");
  } else {
    // Output human-readable format with colors
    let git_hash = env!("GIT_HASH");
    let build_timestamp = env!("BUILD_TIMESTAMP");
    let target = env!("TARGET");

    println!("{} {}", colors.emphasis("confluence-dl"), colors.number(version));
    println!("{}: {}", colors.emphasis("Git commit"), colors.code(git_hash));
    println!(
      "{}: {}",
      colors.emphasis("Built"),
      colors.dimmed(format_timestamp(build_timestamp))
    );
    println!("{}: {}", colors.emphasis("Target"), target);
    println!("{}: {}", colors.emphasis("Rust version"), rustc_version());
  }
}

/// Handle completions command
fn handle_completions_command(shell: Shell) {
  let mut cmd = Cli::command();
  let bin_name = cmd.get_name().to_string();

  let clap_shell = match shell {
    Shell::Bash => CompletionShell::Bash,
    Shell::Zsh => CompletionShell::Zsh,
    Shell::Fish => CompletionShell::Fish,
    Shell::Powershell => CompletionShell::PowerShell,
    Shell::Elvish => CompletionShell::Elvish,
  };

  generate(clap_shell, &mut cmd, bin_name, &mut io::stdout());
}

/// Handle page download
fn handle_page_download(page_input: &str, cli: &Cli, colors: &ColorScheme) {
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

  // Fetch the page
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
  let mut markdown = markdown::storage_to_markdown(storage_content)?;

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

/// Load credentials from CLI args, env vars, or .netrc
fn load_credentials(base_url: &str, cli: &Cli) -> anyhow::Result<(String, String)> {
  // Try CLI args or env vars first
  let username = cli.auth.user.clone();
  let token = cli.auth.token.clone();

  // If both are provided, use them
  if let (Some(user), Some(tok)) = (username, token) {
    return Ok((user, tok));
  }

  // Try to load from .netrc
  let host = extract_host(base_url).ok_or_else(|| anyhow::anyhow!("Invalid base URL"))?;

  let provider = NetrcProvider::new();
  if let Some(creds) = provider.get_credentials(&host)? {
    let user = cli.auth.user.clone().unwrap_or(creds.username);
    let tok = cli.auth.token.clone().unwrap_or(creds.password);
    return Ok((user, tok));
  }

  anyhow::bail!(
    "Credentials not found. Provide --user and --token, set CONFLUENCE_USER and CONFLUENCE_TOKEN, or add to ~/.netrc"
  )
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

/// Format Unix timestamp as ISO 8601 UTC string
fn format_timestamp(timestamp: &str) -> String {
  timestamp
    .parse::<i64>()
    .ok()
    .and_then(|ts| {
      use std::time::{Duration, UNIX_EPOCH};
      UNIX_EPOCH.checked_add(Duration::from_secs(ts as u64))
    })
    .map(|time| {
      let datetime: chrono::DateTime<chrono::Utc> = time.into();
      datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    })
    .unwrap_or_else(|| timestamp.to_string())
}

/// Get Rust compiler version
fn rustc_version() -> String {
  // This could be enhanced to capture the actual rustc version at build time
  // For now, return a placeholder
  env!("CARGO_PKG_RUST_VERSION").to_string()
}
