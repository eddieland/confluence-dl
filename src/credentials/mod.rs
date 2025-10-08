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

mod netrc;
mod provider;
mod types;

pub use netrc::NetrcProvider;
pub use provider::CredentialsProvider;
pub use types::{Credential, CredentialError};
