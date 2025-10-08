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

use cli::{Cli, ColorOption, Command};
use color::ColorScheme;
use commands::auth::handle_auth_command;
use commands::completions::handle_completions_command;
use commands::page::handle_page_download;
use commands::version::handle_version_command;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

fn init_tracing(cli: &Cli) {
  let cli_level = if cli.behavior.quiet {
    LevelFilter::ERROR
  } else {
    match cli.behavior.verbose {
      0 => LevelFilter::INFO,
      1 => LevelFilter::DEBUG,
      _ => LevelFilter::TRACE,
    }
  };

  let env_filter = EnvFilter::builder()
    .with_default_directive(cli_level.into())
    .from_env_lossy();

  let fmt_layer = tracing_subscriber::fmt::layer()
    .with_target(false)
    .without_time()
    .with_writer(std::io::stderr);

  let fmt_layer = match cli.behavior.color {
    ColorOption::Never => fmt_layer.with_ansi(false),
    ColorOption::Always => fmt_layer.with_ansi(true),
    ColorOption::Auto => fmt_layer,
  };

  tracing_subscriber::registry().with(env_filter).with(fmt_layer).init();
}

fn main() {
  let cli = Cli::parse_args();

  init_tracing(&cli);

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
