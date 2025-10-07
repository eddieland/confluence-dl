//! confluence-dl - Export Confluence pages to Markdown
//!
//! This is the main entry point for the CLI application.

mod cli;
mod color;
mod credentials;

use std::{io, process};

use clap::CommandFactory;
use clap_complete::{Shell as CompletionShell, generate};
use cli::{AuthCommand, Cli, Command, Shell};
use color::ColorScheme;
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
  println!("{} {}", colors.progress("Downloading page:"), colors.link(page_input));
  println!(
    "{} {}",
    colors.emphasis("Output directory:"),
    colors.path(&cli.output.output)
  );
  println!("{} {:?}", colors.emphasis("Format:"), cli.output.format);

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
  }

  // TODO: Implement actual page download functionality
  // 1. Parse page input (URL vs ID)
  // 2. Load credentials
  // 3. Connect to Confluence API
  // 4. Download page content
  // 5. Download children if requested
  // 6. Download images and attachments
  // 7. Convert to Markdown
  // 8. Write to output directory

  eprintln!(
    "\n{} Page download functionality not yet implemented",
    colors.warning("Note:")
  );
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
