//! CLI subcommand handlers.
//!
//! This module groups the implementations for each `confluence-dl` subcommand,
//! keeping the top-level `main.rs` lightweight while still allowing the
//! handlers to share utilities and types.

pub mod auth;
pub mod completions;
pub mod page;
pub mod version;
