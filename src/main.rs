//! confluence-dl - Export Confluence pages to Markdown
//!
//! This is the main entry point for the CLI application.

use confluence_dl::cli;

#[tokio::main]
async fn main() {
  cli::run().await;
}
