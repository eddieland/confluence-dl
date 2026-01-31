//! confluence-dl - Export Confluence pages to Markdown
//!
//! This is the main entry point for the CLI application.

use clap::CommandFactory;
use clap_complete::CompleteEnv;
use confluence_dl::cli::{self, Cli};

#[tokio::main]
async fn main() {
  CompleteEnv::with_factory(Cli::command).complete();
  cli::run().await;
}
