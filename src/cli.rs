//! Command-line interface definitions for confluence-dl.
//!
//! This module defines the CLI structure using clap derives, organizing
//! commands and arguments according to the design in CLI_DESIGN.md.

use clap::{Parser, Subcommand, ValueEnum};

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
#[derive(Debug, Subcommand)]
pub enum AuthCommand {
  /// Test authentication credentials against Confluence API
  Test,

  /// Display current authentication configuration (without sensitive data)
  Show,
}

/// Shell types for completion generation
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Shell {
  Bash,
  Zsh,
  Fish,
  Powershell,
  Elvish,
}

/// Authentication options
#[derive(Debug, Parser)]
pub struct AuthOptions {
  /// Confluence base URL
  #[arg(long, env = "CONFLUENCE_URL", value_name = "URL")]
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

  /// Output format
  #[arg(long, value_enum, default_value = "markdown", value_name = "FORMAT")]
  pub format: OutputFormat,

  /// Overwrite existing files
  #[arg(long)]
  pub overwrite: bool,
}

/// Output format options
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
  Markdown,
  Json,
  Html,
}

/// Behavior options
#[derive(Debug, Parser)]
pub struct BehaviorOptions {
  /// Show what would be downloaded without actually downloading
  #[arg(long)]
  pub dry_run: bool,

  /// Increase verbosity (-v, -vv, -vvv)
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

  /// Include comments in export
  #[arg(long)]
  pub comments: bool,
}

/// Image and link options
#[derive(Debug, Parser)]
pub struct ImagesLinksOptions {
  /// Download embedded images
  #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
  pub download_images: bool,

  /// Directory for images (relative to output)
  #[arg(long, default_value = "images", value_name = "DIR")]
  pub images_dir: String,

  /// Convert Confluence links to markdown
  #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
  pub convert_links: bool,

  /// Keep Confluence anchor IDs
  #[arg(long)]
  pub preserve_anchors: bool,
}

/// Performance options
#[derive(Debug, Parser)]
pub struct PerformanceOptions {
  /// Number of parallel downloads
  #[arg(long, default_value = "4", value_name = "N")]
  pub parallel: usize,

  /// Max requests per second
  #[arg(long, default_value = "10", value_name = "N")]
  pub rate_limit: usize,

  /// Request timeout in seconds
  #[arg(long, default_value = "30", value_name = "SECONDS")]
  pub timeout: u64,
}

impl Cli {
  /// Parse CLI arguments from the environment
  pub fn parse_args() -> Self {
    Self::parse()
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

    Ok(())
  }
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
        format: OutputFormat::Markdown,
        overwrite: false,
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
        comments: false,
      },
      images_links: ImagesLinksOptions {
        download_images: true,
        images_dir: "images".to_string(),
        convert_links: true,
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
        format: OutputFormat::Markdown,
        overwrite: false,
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
        comments: false,
      },
      images_links: ImagesLinksOptions {
        download_images: true,
        images_dir: "images".to_string(),
        convert_links: true,
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
        format: OutputFormat::Markdown,
        overwrite: false,
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
        comments: false,
      },
      images_links: ImagesLinksOptions {
        download_images: true,
        images_dir: "images".to_string(),
        convert_links: true,
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
        format: OutputFormat::Markdown,
        overwrite: false,
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
        comments: false,
      },
      images_links: ImagesLinksOptions {
        download_images: true,
        images_dir: "images".to_string(),
        convert_links: true,
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
        format: OutputFormat::Markdown,
        overwrite: false,
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
        comments: false,
      },
      images_links: ImagesLinksOptions {
        download_images: true,
        images_dir: "images".to_string(),
        convert_links: true,
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
        format: OutputFormat::Markdown,
        overwrite: false,
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
        comments: false,
      },
      images_links: ImagesLinksOptions {
        download_images: true,
        images_dir: "images".to_string(),
        convert_links: true,
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
        format: OutputFormat::Markdown,
        overwrite: false,
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
        comments: false,
      },
      images_links: ImagesLinksOptions {
        download_images: true,
        images_dir: "images".to_string(),
        convert_links: true,
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
}
