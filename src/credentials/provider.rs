//! Credential provider abstractions.
//!
//! Defines the [`CredentialsProvider`] trait so different credential backends
//! (environment variables, `.netrc`, custom stores) can plug into the rest of
//! the application without changing call sites.

use super::{Credential, CredentialError};

/// A provider for retrieving credentials.
///
/// This trait allows different credential sources to be used interchangeably.
pub trait CredentialsProvider {
  /// Retrieves credentials for the specified host.
  ///
  /// # Arguments
  /// * `host` - Hostname whose credentials should be resolved (e.g., `example.atlassian.net`).
  ///
  /// # Returns
  /// * `Ok(Some(Credential))` when the provider contains credentials for the host.
  /// * `Ok(None)` when the provider has no entry for the host, allowing fallback providers to run.
  ///
  /// # Errors
  /// Returns `Err(CredentialError)` when an unexpected failure occurs (for
  /// example, unreadable configuration files).
  fn get_credentials(&self, host: &str) -> Result<Option<Credential>, CredentialError>;
}
