//! Strongly typed credentials and related errors.
//!
//! These types are shared between different credential providers and the
//! higher-level CLI logic so that callers can reason about usernames, tokens,
//! and failure modes consistently.

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
  #[allow(dead_code)]
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
