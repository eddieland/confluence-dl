//! Credentials management for Confluence authentication.
//!
//! This module provides a trait-based interface for retrieving credentials
//! from various sources. The default implementation uses `.netrc` files.
//!
//! # Atlassian API Tokens
//!
//! Atlassian Cloud requires **API tokens** for authentication, not traditional
//! passwords. You must create an API token at: <https://id.atlassian.com/manage-profile/security/api-tokens>
//!
//! Store your credentials in `~/.netrc`:
//! ```text
//! machine your-instance.atlassian.net
//!   login your.email@example.com
//!   password your-api-token-here
//! ```
//!
//! **Important**: Use your email address as the login and your API token as the
//! password.

use std::fmt;

/// Represents a set of credentials for authenticating with a host.
///
/// For Atlassian Cloud/Confluence:
/// - `username` should be your email address
/// - `password` should be your API token (created at <https://id.atlassian.com/manage-profile/security/api-tokens>)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credential {
  /// The username for authentication (email address for Atlassian Cloud)
  pub username: String,
  /// The password or API token for authentication
  pub password: String,
}

/// Errors that can occur during credential operations.
#[derive(Debug)]
pub enum CredentialError {
  /// The .netrc file could not be found or read
  NetrcNotFound,
  /// The .netrc file is malformed or could not be parsed
  NetrcParseError(String),
  /// An I/O error occurred while reading credentials
  IoError(std::io::Error),
}

impl fmt::Display for CredentialError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::NetrcNotFound => write!(f, ".netrc file not found"),
      Self::NetrcParseError(msg) => write!(f, "failed to parse .netrc: {msg}"),
      Self::IoError(err) => write!(f, "I/O error: {err}"),
    }
  }
}

impl std::error::Error for CredentialError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      Self::IoError(err) => Some(err),
      _ => None,
    }
  }
}

impl From<std::io::Error> for CredentialError {
  fn from(err: std::io::Error) -> Self {
    Self::IoError(err)
  }
}

/// A provider for retrieving credentials.
///
/// This trait allows different credential sources to be used interchangeably.
pub trait CredentialsProvider {
  /// Retrieves credentials for the specified host.
  ///
  /// # Arguments
  ///
  /// * `host` - The hostname to retrieve credentials for
  ///
  /// # Returns
  ///
  /// * `Ok(Some(Credential))` if credentials were found
  /// * `Ok(None)` if no credentials exist for this host
  /// * `Err(CredentialError)` if an error occurred
  fn get_credentials(&self, host: &str) -> Result<Option<Credential>, CredentialError>;
}

/// A credentials provider that reads from `.netrc` files.
///
/// This provider searches for a `.netrc` file in the user's home directory
/// and parses it to retrieve credentials for specified hosts.
///
/// # Example `.netrc` entry for Atlassian Cloud
///
/// ```text
/// machine your-instance.atlassian.net
///   login your.email@example.com
///   password your-api-token-here
/// ```
///
/// Create an API token at: <https://id.atlassian.com/manage-profile/security/api-tokens>
#[derive(Debug, Default)]
pub struct NetrcProvider;

impl NetrcProvider {
  /// Creates a new `.netrc` credentials provider.
  pub fn new() -> Self {
    Self
  }
}

impl CredentialsProvider for NetrcProvider {
  fn get_credentials(&self, host: &str) -> Result<Option<Credential>, CredentialError> {
    // Get the home directory
    let home = std::env::var("HOME").map_err(|_| CredentialError::NetrcNotFound)?;
    let netrc_path = std::path::Path::new(&home).join(".netrc");

    // Check if .netrc exists
    if !netrc_path.exists() {
      return Ok(None);
    }

    // Read the .netrc file
    let content = std::fs::read_to_string(&netrc_path)?;

    // Parse the .netrc file manually (simple parser)
    parse_netrc(&content, host)
  }
}

/// Parses a .netrc file and extracts credentials for a specific host.
///
/// The .netrc format is:
/// ```text
/// machine <hostname>
///   login <username>
///   password <password>
/// ```
fn parse_netrc(content: &str, target_host: &str) -> Result<Option<Credential>, CredentialError> {
  let lines = content.lines();
  let mut current_machine: Option<String> = None;
  let mut username: Option<String> = None;
  let mut password: Option<String> = None;

  for line in lines {
    let line = line.trim();

    // Skip empty lines and comments
    if line.is_empty() || line.starts_with('#') {
      continue;
    }

    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
      continue;
    }

    match parts[0] {
      "machine" => {
        // If we found credentials for the target machine, return them
        if let (Some(machine), Some(user), Some(pass)) = (&current_machine, &username, &password)
          && machine == target_host
        {
          return Ok(Some(Credential {
            username: user.clone(),
            password: pass.clone(),
          }));
        }

        // Start a new machine entry
        current_machine = parts.get(1).map(|s| s.to_string());
        username = None;
        password = None;
      }
      "login" => {
        username = parts.get(1).map(|s| s.to_string());
      }
      "password" => {
        password = parts.get(1).map(|s| s.to_string());
      }
      "default" => {
        // Handle default entry (matches any machine)
        current_machine = Some("default".to_string());
      }
      _ => {
        // Skip unknown tokens
      }
    }
  }

  // Check if the last entry matches
  if let (Some(machine), Some(user), Some(pass)) = (current_machine, username, password)
    && (machine == target_host || machine == "default")
  {
    return Ok(Some(Credential {
      username: user,
      password: pass,
    }));
  }

  Ok(None)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_netrc_simple() {
    let content = r#"
machine example.com
  login user1
  password pass1
"#;

    let result = parse_netrc(content, "example.com").unwrap();
    assert!(result.is_some());

    let cred = result.unwrap();
    assert_eq!(cred.username, "user1");
    assert_eq!(cred.password, "pass1");
  }

  #[test]
  fn test_parse_netrc_multiple_machines() {
    let content = r#"
machine example.com
  login user1
  password pass1

machine other.com
  login user2
  password pass2
"#;

    let result = parse_netrc(content, "other.com").unwrap();
    assert!(result.is_some());

    let cred = result.unwrap();
    assert_eq!(cred.username, "user2");
    assert_eq!(cred.password, "pass2");
  }

  #[test]
  fn test_parse_netrc_not_found() {
    let content = r#"
machine example.com
  login user1
  password pass1
"#;

    let result = parse_netrc(content, "notfound.com").unwrap();
    assert!(result.is_none());
  }

  #[test]
  fn test_parse_netrc_with_comments() {
    let content = r#"
# This is a comment
machine example.com
  login user1  # inline comment is not supported, but treated as part of value
  password pass1
"#;

    let result = parse_netrc(content, "example.com").unwrap();
    assert!(result.is_some());
  }

  #[test]
  fn test_parse_netrc_default() {
    let content = r#"
default
  login defaultuser
  password defaultpass
"#;

    let result = parse_netrc(content, "any-host.com").unwrap();
    assert!(result.is_some());

    let cred = result.unwrap();
    assert_eq!(cred.username, "defaultuser");
    assert_eq!(cred.password, "defaultpass");
  }

  #[test]
  fn test_credential_equality() {
    let cred1 = Credential {
      username: "user".to_string(),
      password: "pass".to_string(),
    };
    let cred2 = Credential {
      username: "user".to_string(),
      password: "pass".to_string(),
    };
    let cred3 = Credential {
      username: "other".to_string(),
      password: "pass".to_string(),
    };

    assert_eq!(cred1, cred2);
    assert_ne!(cred1, cred3);
  }
}
