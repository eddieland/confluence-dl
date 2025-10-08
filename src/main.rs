//! confluence-dl - Export Confluence pages to Markdown
//!
//! This is the main entry point for the CLI application.

mod cli;
mod color;
mod commands;
mod confluence;
mod credentials;
mod images;
mod markdown;

use std::process;

use cli::{Cli, Command};
use color::ColorScheme;
use commands::auth::handle_auth_command;
use commands::completions::handle_completions_command;
use commands::page::handle_page_download;
use commands::version::handle_version_command;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;

fn main() {
  let cli = Cli::parse_args();

  init_tracing(&cli.behavior);

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

fn init_tracing(behavior: &cli::BehaviorOptions) {
  let level = if behavior.quiet {
    LevelFilter::ERROR
  } else {
    match behavior.verbose {
      0 => LevelFilter::WARN,
      1 => LevelFilter::INFO,
      2 => LevelFilter::DEBUG,
      _ => LevelFilter::TRACE,
    }
  };

  let env_filter = EnvFilter::builder()
    .with_default_directive(level.into())
    .from_env_lossy();

  let _ = tracing_subscriber::fmt()
    .with_env_filter(env_filter)
    .with_target(false)
    .with_writer(std::io::stderr)
    .try_init();
}
