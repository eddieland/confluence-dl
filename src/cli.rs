//! Command-line interface definitions for confluence-dl.
//!
//! This module defines the CLI structure using clap derives, organizing
//! commands and arguments according to the design in CLI_DESIGN.md.

use std::process;

use clap::{Parser, Subcommand, ValueEnum};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;
use url::Url;

use crate::color::ColorScheme;
use crate::commands::auth::{AuthCommand, handle_auth_command};
use crate::commands::completions::{Shell, handle_completions_command};
use crate::commands::ls::handle_ls_command;
use crate::commands::page::handle_page_download;
use crate::commands::version::handle_version_command;

/// confluence-dl - Export Confluence pages to Markdown
#[derive(Debug, Parser)]
#[command(
  name = "confluence-dl",
  version,
  about = "Export Confluence pages to Markdown",
  long_about = "A command-line tool for exporting Confluence pages and spaces to Markdown format.\n\
                Downloads pages, child pages, images, and attachments while preserving structure.",
  styles = get_clap_styles()
)]
pub struct Cli {
  /// Page URL or numeric page ID to download
  #[arg(value_name = "PAGE_URL_OR_ID")]
  pub page_input: Option<String>,

  /// Subcommand to execute
  #[command(subcommand)]
  pub command: Option<Command>,

  /// Authentication options
  #[command(flatten)]
  pub auth: AuthOptions,

  /// Output options
  #[command(flatten)]
  pub output: OutputOptions,

  /// Behavior options
  #[command(flatten)]
  pub behavior: BehaviorOptions,

  /// Page-specific options
  #[command(flatten)]
  pub page: PageOptions,

  /// Image and link options
  #[command(flatten)]
  pub images_links: ImagesLinksOptions,

  /// Performance options
  #[command(flatten)]
  pub performance: PerformanceOptions,
}

/// Subcommands for debugging and introspection
#[derive(Debug, Subcommand)]
pub enum Command {
  /// Print the Confluence page tree without downloading content
  Ls {
    /// Page URL or numeric page ID whose descendants should be displayed
    #[arg(value_name = "PAGE_URL_OR_ID")]
    target: String,

    /// Maximum depth when traversing children (0 lists only the root page)
    #[arg(long, value_name = "N")]
    max_depth: Option<usize>,
  },

  /// Authentication testing and inspection
  Auth {
    #[command(subcommand)]
    subcommand: AuthCommand,
  },

  /// Display version and build information
  Version {
    /// Output in JSON format
    #[arg(long)]
    json: bool,

    /// Show only version number
    #[arg(long)]
    short: bool,
  },

  /// Generate shell completion scripts
  Completions {
    /// Target shell for completions
    #[arg(value_enum)]
    shell: Shell,
  },
}

/// Authentication subcommands
/// Normalize a URL by adding https:// if no scheme is present
fn normalize_url(url: &str) -> Result<String, String> {
  let trimmed = url.trim();

  // Try to parse the URL as-is
  let parsed = match Url::parse(trimmed) {
    Ok(parsed) => parsed,
    Err(_) => {
      // Failed to parse, likely missing scheme
      // Try prepending https://
      let with_https = format!("https://{trimmed}");
      Url::parse(&with_https).map_err(|e| format!("Invalid URL: {e}"))?
    }
  };

  // Convert to string and remove trailing slash if present
  let mut url_str = parsed.to_string();
  if url_str.ends_with('/') && url_str.len() > 1 {
    url_str.pop();
  }

  Ok(url_str)
}

/// Authentication options
#[derive(Debug, Parser)]
pub struct AuthOptions {
  /// Confluence base URL
  #[arg(long, env = "CONFLUENCE_URL", value_name = "URL", value_parser = normalize_url)]
  pub url: Option<String>,

  /// Confluence user email
  #[arg(long, env = "CONFLUENCE_USER", value_name = "EMAIL")]
  pub user: Option<String>,

  /// Confluence API token
  #[arg(long, env = "CONFLUENCE_TOKEN", value_name = "TOKEN")]
  pub token: Option<String>,
}

/// Output options
#[derive(Debug, Parser)]
pub struct OutputOptions {
  /// Output directory
  #[arg(short, long, default_value = "./confluence-export", value_name = "DIR")]
  pub output: String,

  /// Overwrite existing files
  #[arg(long)]
  pub overwrite: bool,

  /// Save raw Confluence storage format alongside Markdown
  #[arg(long)]
  pub save_raw: bool,

  /// Render Markdown tables without padding columns for alignment
  #[arg(long)]
  pub compact_tables: bool,
}

/// Behavior options
#[derive(Debug, Parser)]
pub struct BehaviorOptions {
  /// Show what would be downloaded without actually downloading
  #[arg(long)]
  pub dry_run: bool,

  /// Increase verbosity (-v info, -vv debug, -vvv trace)
  #[arg(short, long, action = clap::ArgAction::Count)]
  pub verbose: u8,

  /// Suppress all output except errors
  #[arg(short, long, conflicts_with = "verbose")]
  pub quiet: bool,

  /// Colorize output
  #[arg(long, value_enum, default_value = "auto", value_name = "WHEN")]
  pub color: ColorOption,
}

/// Color output options
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ColorOption {
  Auto,
  Always,
  Never,
}

/// Page-specific options
#[derive(Debug, Parser)]
pub struct PageOptions {
  /// Download child pages recursively
  #[arg(short = 'r', long, alias = "recursive")]
  pub children: bool,

  /// Maximum depth when downloading children
  #[arg(long, value_name = "N", requires = "children")]
  pub max_depth: Option<usize>,

  /// Download page attachments
  #[arg(long)]
  pub attachments: bool,
}

/// Image and link options
#[derive(Debug, Parser)]
pub struct ImagesLinksOptions {
  /// Download embedded images
  #[arg(
    long,
    default_value_t = true,
    default_missing_value = "true",
    action = clap::ArgAction::Set,
    num_args = 0..=1
  )]
  pub download_images: bool,

  /// Directory for images (relative to output)
  #[arg(long, default_value = "images", value_name = "DIR")]
  pub images_dir: String,

  /// Keep Confluence anchor IDs
  #[arg(long)]
  pub preserve_anchors: bool,
}

/// Performance options
#[derive(Debug, Parser)]
pub struct PerformanceOptions {
  /// Number of parallel downloads (`-1` uses available cores)
  #[arg(long, default_value = "4", value_name = "N", allow_negative_numbers = true)]
  pub parallel: isize,

  /// Max requests per second
  #[arg(long, default_value = "10", value_name = "N")]
  pub rate_limit: usize,

  /// Request timeout in seconds
  #[arg(long, default_value = "30", value_name = "SECONDS")]
  pub timeout: u64,
}

impl PerformanceOptions {
  /// Resolve the parallel download limit into a concrete positive value.
  pub fn resolved_parallel(&self) -> usize {
    match self.parallel {
      value if value > 0 => value as usize,
      -1 => std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
      _ => 1, // Validation should prevent other values.
    }
  }

  /// Human-readable label describing the parallel setting.
  pub fn parallel_label(&self) -> String {
    match self.parallel {
      -1 => {
        let resolved = self.resolved_parallel();
        format!("auto ({resolved})")
      }
      value if value > 0 => value.to_string(),
      _ => "1".to_string(),
    }
  }
}

impl Cli {
  /// Parse CLI arguments from the environment
  pub fn parse_args() -> Self {
    let mut cli = Self::parse();

    // Normalize URL: add https:// if no scheme is present
    if let Some(url) = &cli.auth.url
      && !url.contains("://")
    {
      cli.auth.url = Some(format!("https://{url}"));
    }

    cli
  }

  /// Validate CLI arguments
  ///
  /// Returns an error if the CLI configuration is invalid.
  pub fn validate(&self) -> Result<(), String> {
    // Check if we have a page input or a command
    if self.page_input.is_none() && self.command.is_none() {
      return Err("Either provide a page URL/ID or use a subcommand".to_string());
    }

    // If page_input is provided, check if we need a base URL
    if let Some(ref input) = self.page_input {
      // If it's a numeric ID (not a URL), we need a base URL
      if !input.contains("://") && self.auth.url.is_none() {
        return Err("--url is required when using a numeric page ID".to_string());
      }
    }

    // Check for conflicting options
    if self.page.max_depth.is_some() && !self.page.children {
      return Err("--max-depth requires --children".to_string());
    }

    if self.performance.parallel == 0 || self.performance.parallel < -1 {
      return Err("--parallel must be at least 1 or -1 to use available cores".to_string());
    }

    if self.performance.rate_limit == 0 {
      return Err("--rate-limit must be at least 1 request per second".to_string());
    }

    Ok(())
  }
}

/// Parse CLI arguments, initialize shared services, and dispatch to the chosen
/// command.
pub async fn run() {
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
      Command::Ls { target, max_depth } => {
        handle_ls_command(target, *max_depth, &cli, &colors).await;
      }
      Command::Auth { subcommand } => {
        handle_auth_command(subcommand, &cli, &colors).await;
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
    handle_page_download(page_input, &cli, &colors).await;
  }
}

fn init_tracing(behavior: &BehaviorOptions) {
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

/// Get custom styles for clap help output
fn get_clap_styles() -> clap::builder::Styles {
  use clap::builder::styling::{AnsiColor, Effects};

  clap::builder::Styles::styled()
    .header(AnsiColor::BrightYellow.on_default() | Effects::BOLD)
    .usage(AnsiColor::BrightYellow.on_default() | Effects::BOLD)
    .literal(AnsiColor::BrightGreen.on_default())
    .placeholder(AnsiColor::BrightCyan.on_default())
    .error(AnsiColor::BrightRed.on_default() | Effects::BOLD)
    .valid(AnsiColor::BrightGreen.on_default())
    .invalid(AnsiColor::BrightRed.on_default())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_cli_validation_requires_page_or_command() {
    let cli = Cli {
      page_input: None,
      command: None,
      auth: AuthOptions {
        url: None,
        user: None,
        token: None,
      },
      output: OutputOptions {
        output: "./output".to_string(),
        overwrite: false,
        save_raw: false,
        compact_tables: false,
      },
      behavior: BehaviorOptions {
        dry_run: false,
        verbose: 0,
        quiet: false,
        color: ColorOption::Auto,
      },
      page: PageOptions {
        children: false,
        max_depth: None,
        attachments: false,
      },
      images_links: ImagesLinksOptions {
        download_images: true,
        images_dir: "images".to_string(),
        preserve_anchors: false,
      },
      performance: PerformanceOptions {
        parallel: 4,
        rate_limit: 10,
        timeout: 30,
      },
    };

    let result = cli.validate();
    assert!(result.is_err());
    assert!(
      result
        .unwrap_err()
        .contains("provide a page URL/ID or use a subcommand")
    );
  }

  #[test]
  fn test_cli_validation_numeric_id_requires_url() {
    let cli = Cli {
      page_input: Some("123456".to_string()),
      command: None,
      auth: AuthOptions {
        url: None,
        user: None,
        token: None,
      },
      output: OutputOptions {
        output: "./output".to_string(),
        overwrite: false,
        save_raw: false,
        compact_tables: false,
      },
      behavior: BehaviorOptions {
        dry_run: false,
        verbose: 0,
        quiet: false,
        color: ColorOption::Auto,
      },
      page: PageOptions {
        children: false,
        max_depth: None,
        attachments: false,
      },
      images_links: ImagesLinksOptions {
        download_images: true,
        images_dir: "images".to_string(),
        preserve_anchors: false,
      },
      performance: PerformanceOptions {
        parallel: 4,
        rate_limit: 10,
        timeout: 30,
      },
    };

    let result = cli.validate();
    assert!(result.is_err());
    assert!(
      result
        .unwrap_err()
        .contains("--url is required when using a numeric page ID")
    );
  }

  #[test]
  fn test_cli_validation_max_depth_requires_children() {
    let cli = Cli {
      page_input: Some("https://example.com/page/123".to_string()),
      command: None,
      auth: AuthOptions {
        url: Some("https://example.com".to_string()),
        user: None,
        token: None,
      },
      output: OutputOptions {
        output: "./output".to_string(),
        overwrite: false,
        save_raw: false,
        compact_tables: false,
      },
      behavior: BehaviorOptions {
        dry_run: false,
        verbose: 0,
        quiet: false,
        color: ColorOption::Auto,
      },
      page: PageOptions {
        children: false,
        max_depth: Some(3),
        attachments: false,
      },
      images_links: ImagesLinksOptions {
        download_images: true,
        images_dir: "images".to_string(),
        preserve_anchors: false,
      },
      performance: PerformanceOptions {
        parallel: 4,
        rate_limit: 10,
        timeout: 30,
      },
    };

    let result = cli.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("--max-depth requires --children"));
  }

  #[test]
  fn test_cli_validation_parallel_must_be_positive_or_auto() {
    let cli = Cli {
      page_input: Some("https://example.com/page/123".to_string()),
      command: None,
      auth: AuthOptions {
        url: None,
        user: None,
        token: None,
      },
      output: OutputOptions {
        output: "./output".to_string(),
        overwrite: false,
        save_raw: false,
        compact_tables: false,
      },
      behavior: BehaviorOptions {
        dry_run: false,
        verbose: 0,
        quiet: false,
        color: ColorOption::Auto,
      },
      page: PageOptions {
        children: false,
        max_depth: None,
        attachments: false,
      },
      images_links: ImagesLinksOptions {
        download_images: true,
        images_dir: "images".to_string(),
        preserve_anchors: false,
      },
      performance: PerformanceOptions {
        parallel: 0,
        rate_limit: 10,
        timeout: 30,
      },
    };

    let result = cli.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("--parallel must be at least 1 or -1"));
  }

  #[test]
  fn test_cli_validation_parallel_auto_allowed() {
    let cli = Cli {
      page_input: Some("https://example.com/page/123".to_string()),
      command: None,
      auth: AuthOptions {
        url: None,
        user: None,
        token: None,
      },
      output: OutputOptions {
        output: "./output".to_string(),
        overwrite: false,
        save_raw: false,
        compact_tables: false,
      },
      behavior: BehaviorOptions {
        dry_run: false,
        verbose: 0,
        quiet: false,
        color: ColorOption::Auto,
      },
      page: PageOptions {
        children: false,
        max_depth: None,
        attachments: false,
      },
      images_links: ImagesLinksOptions {
        download_images: true,
        images_dir: "images".to_string(),
        preserve_anchors: false,
      },
      performance: PerformanceOptions {
        parallel: -1,
        rate_limit: 10,
        timeout: 30,
      },
    };

    assert!(cli.validate().is_ok());
  }

  #[test]
  fn test_cli_validation_parallel_negative_invalid() {
    let cli = Cli {
      page_input: Some("https://example.com/page/123".to_string()),
      command: None,
      auth: AuthOptions {
        url: None,
        user: None,
        token: None,
      },
      output: OutputOptions {
        output: "./output".to_string(),
        overwrite: false,
        save_raw: false,
        compact_tables: false,
      },
      behavior: BehaviorOptions {
        dry_run: false,
        verbose: 0,
        quiet: false,
        color: ColorOption::Auto,
      },
      page: PageOptions {
        children: false,
        max_depth: None,
        attachments: false,
      },
      images_links: ImagesLinksOptions {
        download_images: true,
        images_dir: "images".to_string(),
        preserve_anchors: false,
      },
      performance: PerformanceOptions {
        parallel: -2,
        rate_limit: 10,
        timeout: 30,
      },
    };

    let result = cli.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("--parallel must be at least 1 or -1"));
  }

  #[test]
  fn test_cli_parallel_auto_parse() {
    use clap::Parser;

    let cli = Cli::try_parse_from([
      "confluence-dl",
      "--parallel",
      "-1",
      "https://example.com/wiki/pages/123",
    ])
    .unwrap();

    assert_eq!(cli.performance.parallel, -1);
  }

  #[test]
  fn test_cli_validation_url_input_succeeds() {
    let cli = Cli {
      page_input: Some("https://example.com/wiki/pages/123".to_string()),
      command: None,
      auth: AuthOptions {
        url: None,
        user: None,
        token: None,
      },
      output: OutputOptions {
        output: "./output".to_string(),
        overwrite: false,
        save_raw: false,
        compact_tables: false,
      },
      behavior: BehaviorOptions {
        dry_run: false,
        verbose: 0,
        quiet: false,
        color: ColorOption::Auto,
      },
      page: PageOptions {
        children: false,
        max_depth: None,
        attachments: false,
      },
      images_links: ImagesLinksOptions {
        download_images: true,
        images_dir: "images".to_string(),
        preserve_anchors: false,
      },
      performance: PerformanceOptions {
        parallel: 4,
        rate_limit: 10,
        timeout: 30,
      },
    };

    let result = cli.validate();
    assert!(result.is_ok());
  }

  #[test]
  fn test_cli_validation_command_succeeds() {
    let cli = Cli {
      page_input: None,
      command: Some(Command::Version {
        json: false,
        short: false,
      }),
      auth: AuthOptions {
        url: None,
        user: None,
        token: None,
      },
      output: OutputOptions {
        output: "./output".to_string(),
        overwrite: false,
        save_raw: false,
        compact_tables: false,
      },
      behavior: BehaviorOptions {
        dry_run: false,
        verbose: 0,
        quiet: false,
        color: ColorOption::Auto,
      },
      page: PageOptions {
        children: false,
        max_depth: None,
        attachments: false,
      },
      images_links: ImagesLinksOptions {
        download_images: true,
        images_dir: "images".to_string(),
        preserve_anchors: false,
      },
      performance: PerformanceOptions {
        parallel: 4,
        rate_limit: 10,
        timeout: 30,
      },
    };

    let result = cli.validate();
    assert!(result.is_ok());
  }

  #[test]
  fn test_cli_validation_numeric_id_with_url_succeeds() {
    let cli = Cli {
      page_input: Some("123456".to_string()),
      command: None,
      auth: AuthOptions {
        url: Some("https://example.com".to_string()),
        user: None,
        token: None,
      },
      output: OutputOptions {
        output: "./output".to_string(),
        overwrite: false,
        save_raw: false,
        compact_tables: false,
      },
      behavior: BehaviorOptions {
        dry_run: false,
        verbose: 0,
        quiet: false,
        color: ColorOption::Auto,
      },
      page: PageOptions {
        children: false,
        max_depth: None,
        attachments: false,
      },
      images_links: ImagesLinksOptions {
        download_images: true,
        images_dir: "images".to_string(),
        preserve_anchors: false,
      },
      performance: PerformanceOptions {
        parallel: 4,
        rate_limit: 10,
        timeout: 30,
      },
    };

    let result = cli.validate();
    assert!(result.is_ok());
  }

  #[test]
  fn test_cli_validation_children_with_max_depth_succeeds() {
    let cli = Cli {
      page_input: Some("https://example.com/page/123".to_string()),
      command: None,
      auth: AuthOptions {
        url: None,
        user: None,
        token: None,
      },
      output: OutputOptions {
        output: "./output".to_string(),
        overwrite: false,
        save_raw: false,
        compact_tables: false,
      },
      behavior: BehaviorOptions {
        dry_run: false,
        verbose: 0,
        quiet: false,
        color: ColorOption::Auto,
      },
      page: PageOptions {
        children: true,
        max_depth: Some(3),
        attachments: false,
      },
      images_links: ImagesLinksOptions {
        download_images: true,
        images_dir: "images".to_string(),
        preserve_anchors: false,
      },
      performance: PerformanceOptions {
        parallel: 4,
        rate_limit: 10,
        timeout: 30,
      },
    };

    let result = cli.validate();
    assert!(result.is_ok());
  }

  #[test]
  fn test_url_normalization_adds_https_when_missing() {
    // Create a CLI with a URL without a scheme
    use clap::Parser;

    let cli = Cli::try_parse_from(["confluence-dl", "--url", "example.atlassian.net", "auth", "test"]).unwrap();

    // URL should have https:// prepended
    assert_eq!(cli.auth.url, Some("https://example.atlassian.net".to_string()));
  }

  #[test]
  fn test_url_normalization_preserves_https_scheme() {
    use clap::Parser;

    let cli = Cli::try_parse_from([
      "confluence-dl",
      "--url",
      "https://example.atlassian.net",
      "auth",
      "test",
    ])
    .unwrap();

    // URL should remain unchanged
    assert_eq!(cli.auth.url, Some("https://example.atlassian.net".to_string()));
  }

  #[test]
  fn test_url_normalization_preserves_http_scheme() {
    use clap::Parser;

    let cli = Cli::try_parse_from(["confluence-dl", "--url", "http://localhost:8080", "auth", "test"]).unwrap();

    // URL should remain unchanged (http:// preserved for localhost testing)
    assert_eq!(cli.auth.url, Some("http://localhost:8080".to_string()));
  }

  #[test]
  fn test_url_normalization_from_env_var() {
    use std::env;

    use clap::Parser;

    // Set the environment variable
    unsafe {
      env::set_var("CONFLUENCE_URL", "mycompany.atlassian.net");
    }

    let cli = Cli::try_parse_from(["confluence-dl", "auth", "test"]).unwrap();

    // URL from environment should have https:// prepended
    assert_eq!(cli.auth.url, Some("https://mycompany.atlassian.net".to_string()));

    // Clean up
    unsafe {
      env::remove_var("CONFLUENCE_URL");
    }
  }
}
