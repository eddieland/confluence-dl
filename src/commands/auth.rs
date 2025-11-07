//! Authentication subcommand handlers.
//!
//! Covers both `confluence-dl auth test`, which performs a live API call, and
//! `confluence-dl auth show`, which prints the currently detected credential
//! sources.

use std::process;

use crate::cli::{AuthCommand, Cli};
use crate::color::ColorScheme;
use crate::confluence::{self, ConfluenceApi};
use crate::credentials::{CredentialsProvider, NetrcProvider};

/// Dispatch the authentication subcommands defined under `confluence-dl auth`.
///
/// `auth test` validates that the provided credentials work against the
/// Confluence API, while `auth show` prints a human-readable summary of the
/// resolved credential sources.
///
/// # Arguments
/// * `subcommand` - Auth-specific variant to execute.
/// * `cli` - Parsed CLI settings containing authentication, output, and
///   telemetry options.
/// * `colors` - Shared color scheme used to render output consistently.
pub(crate) async fn handle_auth_command(subcommand: &AuthCommand, cli: &Cli, colors: &ColorScheme) {
  match subcommand {
    AuthCommand::Test => {
      // Verify we have a base URL
      let base_url = match &cli.auth.url {
        Some(url) => url,
        None => {
          eprintln!("{} {}", colors.error("✗"), colors.error("Base URL not provided"));
          eprintln!("\n{}", colors.info("Please provide the Confluence URL:"));
          eprintln!("  confluence-dl auth test --url https://your-instance.atlassian.net");
          eprintln!("  Or set CONFLUENCE_URL environment variable");
          process::exit(1);
        }
      };

      println!("{} {}", colors.info("→"), colors.info("Testing authentication"));
      println!("  {}: {}", colors.emphasis("URL"), colors.link(base_url));

      // Load credentials
      let (username, token) = match load_credentials(base_url, cli) {
        Ok(creds) => creds,
        Err(e) => {
          eprintln!("\n{} {}", colors.error("✗"), colors.error("Failed to load credentials"));
          eprintln!("  {e}");
          eprintln!("\n{}", colors.info("Setup instructions:"));
          eprintln!(
            "  1. Create an API token at: {}",
            colors.link("https://id.atlassian.com/manage-profile/security/api-tokens")
          );
          eprintln!("  2. Provide credentials via:");
          eprintln!("     • CLI flags: --user and --token");
          eprintln!("     • Environment variables: CONFLUENCE_USER and CONFLUENCE_TOKEN");
          eprintln!("     • ~/.netrc file");
          process::exit(2);
        }
      };

      println!("  {}: {}", colors.emphasis("Username"), username);

      // Create client
      let client = match confluence::ConfluenceClient::new(
        base_url,
        &username,
        &token,
        cli.performance.timeout,
        cli.performance.rate_limit,
      ) {
        Ok(c) => c,
        Err(e) => {
          eprintln!(
            "\n{} {}",
            colors.error("✗"),
            colors.error("Failed to create API client")
          );
          eprintln!("  {e}");
          process::exit(1);
        }
      };

      // Test authentication
      println!("\n{} {}", colors.info("→"), colors.info("Calling Confluence API..."));
      match client.test_auth().await {
        Ok(user_info) => {
          println!(
            "\n{} {}",
            colors.success("✓"),
            colors.success("Authentication successful!")
          );
          println!("\n{}", colors.emphasis("User Information:"));
          println!("  {}: {}", colors.emphasis("Display Name"), user_info.display_name);
          println!(
            "  {}: {}",
            colors.emphasis("Account ID"),
            colors.dimmed(&user_info.account_id)
          );
          if let Some(email) = user_info.email {
            println!("  {}: {}", colors.emphasis("Email"), email);
          }
          if let Some(public_name) = user_info.public_name {
            println!("  {}: {}", colors.emphasis("Public Name"), public_name);
          }
          println!("\n{} Your credentials are working correctly.", colors.info("ℹ"));
        }
        Err(e) => {
          eprintln!("\n{} {}", colors.error("✗"), colors.error("Authentication failed"));
          eprintln!("  {e}");
          eprintln!("\n{}", colors.info("Common issues:"));
          eprintln!(
            "  1. Invalid API token - verify at {}",
            colors.link("https://id.atlassian.com/manage-profile/security/api-tokens")
          );
          eprintln!("  2. Incorrect username - should be your email address");
          eprintln!("  3. Wrong base URL - should be https://your-instance.atlassian.net");
          eprintln!("  4. Network connectivity issues");
          eprintln!(
            "\n{}",
            colors.dimmed("Run 'confluence-dl auth show' to see your current configuration")
          );
          process::exit(2);
        }
      }
    }
    AuthCommand::Show => {
      show_auth_config(cli, colors);
    }
  }
}

/// Display the currently configured authentication sources and values.
///
/// The output highlights whether values came from CLI flags, environment
/// variables, or a `.netrc` file so that users can quickly diagnose conflicts.
///
/// # Arguments
/// * `cli` - Parsed CLI options containing the user-facing configuration.
/// * `colors` - Color palette used for consistent, accessible output.
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

/// Resolve Confluence credentials from CLI flags, environment variables, or
/// `.netrc`.
///
/// The lookup order honors explicit CLI input first, then falls back to the
/// host-specific entry in `.netrc`. The helper returns both username and API
/// token so callers can immediately construct an API client.
///
/// # Arguments
/// * `base_url` - Base Confluence URL whose host is used for `.netrc` lookups.
/// * `cli` - Parsed CLI struct carrying the `--user`/`--token` overrides.
///
/// # Returns
/// A tuple of `(username, token)` suitable for authenticating with Confluence.
///
/// # Errors
/// Returns an error when the base URL is invalid, when `.netrc` parsing fails,
/// or when no credential source provides both username and token.
pub(crate) fn load_credentials(base_url: &str, cli: &Cli) -> anyhow::Result<(String, String)> {
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

/// Extract the hostname component from a Confluence base URL string.
///
/// This lightweight helper avoids pulling in an additional URL parser for the
/// subset of logic needed by credential discovery.
///
/// # Arguments
/// * `url` - Fully qualified or scheme-less host string.
///
/// # Returns
/// The hostname portion of the URL, if one can be derived.
fn extract_host(url: &str) -> Option<String> {
  // Simple URL parsing to extract the host
  if let Some(start) = url.find("://") {
    let after_scheme = &url[start + 3..];
    if let Some(end) = after_scheme.find('/') {
      Some(after_scheme[..end].to_string())
    } else {
      Some(after_scheme.to_string())
    }
  } else if let Some(end) = url.find('/') {
    Some(url[..end].to_string())
  } else {
    Some(url.to_string())
  }
}
