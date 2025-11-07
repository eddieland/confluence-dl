#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::process;

use crate::cli::{AuthCommand, Cli};
use crate::color::ColorScheme;
use crate::confluence::{self, ConfluenceApi};
use crate::credentials::{CredentialsProvider, NetrcProvider};

/// Handle authentication subcommands
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

      warn_if_insecure_netrc(colors);

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

/// Load credentials from CLI args, env vars, or .netrc
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
  } else if let Some(end) = url.find('/') {
    Some(url[..end].to_string())
  } else {
    Some(url.to_string())
  }
}

#[cfg(unix)]
fn warn_if_insecure_netrc(colors: &ColorScheme) {
  if let Ok(home) = std::env::var("HOME") {
    let netrc_path = std::path::Path::new(&home).join(".netrc");
    if let Ok(metadata) = std::fs::metadata(&netrc_path) {
      let mode = metadata.permissions().mode() & 0o777;
      if mode & 0o077 != 0 {
        println!(
          "\n{} {}",
          colors.warning("⚠"),
          colors.warning(".netrc permissions are too permissive")
        );
        println!("  {}: {}", colors.emphasis("File"), colors.path(netrc_path.display()));
        println!(
          "  {}: {}",
          colors.emphasis("Current mode"),
          colors.number(format!("{mode:03o}"))
        );
        println!(
          "  {} {} {}",
          colors.dimmed("Hint:"),
          colors.dimmed("restrict access using"),
          colors.code("chmod 600 ~/.netrc")
        );
      }
    }
  }
}

#[cfg(not(unix))]
fn warn_if_insecure_netrc(_: &ColorScheme) {}
