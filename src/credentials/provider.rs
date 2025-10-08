use super::{Credential, CredentialError};

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
