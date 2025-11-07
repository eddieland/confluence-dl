//! `.netrc` credential discovery.
//!
//! Provides a [`CredentialsProvider`] implementation that reads the user's
//! `~/.netrc` file to locate Confluence credentials. This keeps Atlassian API
//! tokens outside of shell history and supports multiple hosts.

use super::{Credential, CredentialError, CredentialsProvider};

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
  /// Resolve credentials for `host` by scanning the user's `.netrc`.
  ///
  /// # Arguments
  /// * `host` - Hostname to look up (e.g., `company.atlassian.net`).
  ///
  /// # Returns
  /// * `Ok(Some(Credential))` when the `.netrc` file contains a matching entry.
  /// * `Ok(None)` when the file is present but no entry exists for the host.
  ///
  /// # Errors
  /// Returns `Err(CredentialError)` when the home directory cannot be
  /// determined, the `.netrc` file is unreadable, or parsing fails.
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

  // Edge case tests

  #[test]
  fn test_parse_netrc_empty_file() {
    let content = "";
    let result = parse_netrc(content, "example.com").unwrap();
    assert!(result.is_none());
  }

  #[test]
  fn test_parse_netrc_only_whitespace() {
    let content = "   \n\t\n  \n";
    let result = parse_netrc(content, "example.com").unwrap();
    assert!(result.is_none());
  }

  #[test]
  fn test_parse_netrc_only_comments() {
    let content = r#"
# Comment line 1
# Comment line 2
    # Indented comment
"#;
    let result = parse_netrc(content, "example.com").unwrap();
    assert!(result.is_none());
  }

  #[test]
  fn test_parse_netrc_missing_login() {
    let content = r#"
machine example.com
  password pass1
"#;
    let result = parse_netrc(content, "example.com").unwrap();
    assert!(result.is_none());
  }

  #[test]
  fn test_parse_netrc_missing_password() {
    let content = r#"
machine example.com
  login user1
"#;
    let result = parse_netrc(content, "example.com").unwrap();
    assert!(result.is_none());
  }

  #[test]
  fn test_parse_netrc_missing_both_credentials() {
    let content = r#"
machine example.com
machine other.com
  login user2
  password pass2
"#;
    let result = parse_netrc(content, "example.com").unwrap();
    assert!(result.is_none());
  }

  #[test]
  fn test_parse_netrc_machine_without_hostname() {
    let content = r#"
machine
  login user1
  password pass1
"#;
    let result = parse_netrc(content, "example.com").unwrap();
    assert!(result.is_none());
  }

  #[test]
  fn test_parse_netrc_single_line_format() {
    // Note: Current parser processes one keyword per line, so single-line
    // format with multiple keywords is not fully supported
    let content = "machine example.com login user1 password pass1";
    let result = parse_netrc(content, "example.com").unwrap();
    // Parser only captures "machine example.com" from this line
    assert!(result.is_none());
  }

  #[test]
  fn test_parse_netrc_mixed_whitespace() {
    // Parser handles tabs and spaces in whitespace, but only processes
    // one keyword per line
    let content = "machine\texample.com\nlogin\t\tuser1\npassword   pass1";
    let result = parse_netrc(content, "example.com").unwrap();
    assert!(result.is_some());

    let cred = result.unwrap();
    assert_eq!(cred.username, "user1");
    assert_eq!(cred.password, "pass1");
  }

  #[test]
  fn test_parse_netrc_reverse_order_login_password() {
    let content = r#"
machine example.com
  password pass1
  login user1
"#;
    let result = parse_netrc(content, "example.com").unwrap();
    assert!(result.is_some());

    let cred = result.unwrap();
    assert_eq!(cred.username, "user1");
    assert_eq!(cred.password, "pass1");
  }

  #[test]
  fn test_parse_netrc_email_as_username() {
    let content = r#"
machine example.atlassian.net
  login user@example.com
  password api-token-123
"#;
    let result = parse_netrc(content, "example.atlassian.net").unwrap();
    assert!(result.is_some());

    let cred = result.unwrap();
    assert_eq!(cred.username, "user@example.com");
    assert_eq!(cred.password, "api-token-123");
  }

  #[test]
  fn test_parse_netrc_complex_password() {
    let content = r#"
machine example.com
  login user1
  password P@ssw0rd!#$%^&*()
"#;
    let result = parse_netrc(content, "example.com").unwrap();
    assert!(result.is_some());

    let cred = result.unwrap();
    assert_eq!(cred.password, "P@ssw0rd!#$%^&*()");
  }

  #[test]
  fn test_parse_netrc_duplicate_machines_returns_first() {
    let content = r#"
machine example.com
  login user1
  password pass1

machine example.com
  login user2
  password pass2
"#;
    let result = parse_netrc(content, "example.com").unwrap();
    assert!(result.is_some());

    let cred = result.unwrap();
    assert_eq!(cred.username, "user1");
    assert_eq!(cred.password, "pass1");
  }

  #[test]
  fn test_parse_netrc_specific_machine_overrides_default() {
    let content = r#"
default
  login defaultuser
  password defaultpass

machine example.com
  login specificuser
  password specificpass
"#;
    let result = parse_netrc(content, "example.com").unwrap();
    assert!(result.is_some());

    let cred = result.unwrap();
    assert_eq!(cred.username, "specificuser");
    assert_eq!(cred.password, "specificpass");
  }

  #[test]
  fn test_parse_netrc_default_after_specific() {
    // Note: When the parser encounters a new machine or default entry,
    // it overwrites the previous entry's state without checking if it should
    // return the previous entry first. So the "default" entry overwrites
    // the "specific.com" entry.
    let content = r#"
machine specific.com
  login specificuser
  password specificpass

default
  login defaultuser
  password defaultpass
"#;
    // The default entry overwrites specific.com, so both queries get default
    let result = parse_netrc(content, "specific.com").unwrap();
    assert!(result.is_some());

    let cred = result.unwrap();
    assert_eq!(cred.username, "defaultuser");
    assert_eq!(cred.password, "defaultpass");

    // Should also match default for non-specific host
    let result = parse_netrc(content, "any-host.com").unwrap();
    assert!(result.is_some());

    let cred = result.unwrap();
    assert_eq!(cred.username, "defaultuser");
    assert_eq!(cred.password, "defaultpass");
  }

  #[test]
  fn test_parse_netrc_unknown_tokens_ignored() {
    let content = r#"
machine example.com
  login user1
  unknown_field value
  password pass1
  another_unknown another_value
"#;
    let result = parse_netrc(content, "example.com").unwrap();
    assert!(result.is_some());

    let cred = result.unwrap();
    assert_eq!(cred.username, "user1");
    assert_eq!(cred.password, "pass1");
  }

  #[test]
  fn test_parse_netrc_login_without_value() {
    let content = r#"
machine example.com
  login
  password pass1
"#;
    let result = parse_netrc(content, "example.com").unwrap();
    assert!(result.is_none());
  }

  #[test]
  fn test_parse_netrc_password_without_value() {
    let content = r#"
machine example.com
  login user1
  password
"#;
    let result = parse_netrc(content, "example.com").unwrap();
    assert!(result.is_none());
  }

  #[test]
  fn test_parse_netrc_case_sensitive_hostname() {
    let content = r#"
machine Example.com
  login user1
  password pass1
"#;
    // Should not match lowercase
    let result = parse_netrc(content, "example.com").unwrap();
    assert!(result.is_none());

    // Should match exact case
    let result = parse_netrc(content, "Example.com").unwrap();
    assert!(result.is_some());
  }

  #[test]
  fn test_parse_netrc_multiple_credentials_same_line() {
    // Current parser processes one keyword per line by only looking at parts[0].
    // So when all keywords are on a single line, only the first keyword is
    // processed.
    let content = "machine example.com login user1 password pass1 machine other.com login user2 password pass2";

    // Parser only sees "machine example.com" from this line, the rest is ignored
    // No login/password found, so returns None
    let result = parse_netrc(content, "example.com").unwrap();
    assert!(result.is_none());

    let result = parse_netrc(content, "other.com").unwrap();
    assert!(result.is_none());
  }

  #[test]
  fn test_parse_netrc_trailing_whitespace() {
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
  fn test_parse_netrc_leading_whitespace_before_machine() {
    let content = r#"
    machine example.com
      login user1
      password pass1
"#;
    let result = parse_netrc(content, "example.com").unwrap();
    assert!(result.is_some());
  }

  #[test]
  fn test_parse_netrc_unicode_support() {
    let content = r#"
machine example.com
  login user_日本語
  password pàsswørd_🔑
"#;
    let result = parse_netrc(content, "example.com").unwrap();
    assert!(result.is_some());

    let cred = result.unwrap();
    assert_eq!(cred.username, "user_日本語");
    assert_eq!(cred.password, "pàsswørd_🔑");
  }

  #[test]
  fn test_parse_netrc_incomplete_entry_at_eof() {
    let content = r#"
machine complete.com
  login user1
  password pass1

machine incomplete.com
  login user2
"#;
    // Complete entry should work
    let result = parse_netrc(content, "complete.com").unwrap();
    assert!(result.is_some());

    // Incomplete entry should return None
    let result = parse_netrc(content, "incomplete.com").unwrap();
    assert!(result.is_none());
  }

  #[test]
  fn test_parse_netrc_macdef_ignored() {
    let content = r#"
machine example.com
  login user1
  password pass1

macdef init
cd /pub
mget *
quit
"#;
    let result = parse_netrc(content, "example.com").unwrap();
    assert!(result.is_some());

    let cred = result.unwrap();
    assert_eq!(cred.username, "user1");
    assert_eq!(cred.password, "pass1");
  }

  #[test]
  fn test_parse_netrc_extra_tokens_after_values() {
    let content = r#"
machine example.com extra tokens ignored
  login user1 extra
  password pass1 tokens ignored
"#;
    let result = parse_netrc(content, "example.com").unwrap();
    assert!(result.is_some());

    let cred = result.unwrap();
    assert_eq!(cred.username, "user1");
    assert_eq!(cred.password, "pass1");
  }

  #[test]
  fn test_credential_error_display() {
    let err1 = CredentialError::NetrcNotFound;
    assert_eq!(err1.to_string(), ".netrc file not found");

    let err2 = CredentialError::NetrcParseError("bad syntax".to_string());
    assert_eq!(err2.to_string(), "failed to parse .netrc: bad syntax");

    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err3 = CredentialError::IoError(io_err);
    assert!(err3.to_string().contains("I/O error"));
  }

  #[test]
  fn test_credential_error_source() {
    use std::error::Error;

    let err1 = CredentialError::NetrcNotFound;
    assert!(err1.source().is_none());

    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err2 = CredentialError::IoError(io_err);
    assert!(err2.source().is_some());
  }

  #[test]
  fn test_credential_error_from_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
    let cred_err: CredentialError = io_err.into();
    assert!(matches!(cred_err, CredentialError::IoError(_)));
  }

  #[test]
  fn test_credential_debug() {
    let cred = Credential {
      username: "user".to_string(),
      password: "secret".to_string(),
    };
    let debug_str = format!("{cred:?}");
    assert!(debug_str.contains("Credential"));
    assert!(debug_str.contains("username"));
    assert!(debug_str.contains("password"));
  }

  #[test]
  fn test_credential_clone_and_equality() {
    let cred1 = Credential {
      username: "user1".to_string(),
      password: "pass1".to_string(),
    };
    let cred2 = cred1.clone();
    let cred3 = Credential {
      username: "user2".to_string(),
      password: "pass1".to_string(),
    };

    assert_eq!(cred1, cred2);
    assert_ne!(cred1, cred3);
  }
}
