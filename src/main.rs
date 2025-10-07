//! confluence-dl - Export Confluence pages to Markdown
//!
//! This is the main entry point for the CLI application.

mod cli;
mod color;

use std::{io, process};

use clap::CommandFactory;
use clap_complete::{Shell as CompletionShell, generate};
use cli::{AuthCommand, Cli, Command, Shell};
use color::ColorScheme;

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
      println!("{}", colors.emphasis("Authentication Configuration:"));
      // TODO: Implement authentication configuration display
      // 1. Determine source (CLI, env, .netrc)
      // 2. Display URL, user, and masked token
      // 3. Show token length and source
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
      if let Some(ref token) = cli.auth.token {
        println!(
          "  {}: {} ({} chars)",
          colors.emphasis("Token"),
          colors.dimmed("*".repeat(8)),
          colors.number(token.len())
        );
      } else {
        println!("  {}: {}", colors.emphasis("Token"), colors.dimmed("(not set)"));
      }
      eprintln!(
        "{} Full authentication configuration display not yet implemented",
        colors.warning("Note:")
      );
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
