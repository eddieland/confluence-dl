//! Build script for the confluence-dl project
//!
//! Embeds version and build metadata for runtime access

use std::env;
use std::process::Command;

/// Entry point for the build script.
fn main() {
  embed_build_info();
  set_rerun_conditions();
}

/// Embeds build-time information as environment variables accessible at
/// runtime.
///
/// Captures and stores metadata about the build environment:
/// - Git commit hash for version tracking and debugging
/// - Build timestamp for release identification
/// - Target architecture for platform-specific behavior
fn embed_build_info() {
  // Capture the current Git commit hash for version identification
  // Falls back gracefully if Git is unavailable or not in a repository
  if let Ok(output) = Command::new("git").args(["rev-parse", "--short", "HEAD"]).output() {
    let git_hash = String::from_utf8(output.stdout).unwrap_or_default().trim().to_string();
    println!("cargo:rustc-env=GIT_HASH={git_hash}");
  }

  // Record the exact build time as a Unix timestamp
  // Used for build identification and debugging purposes
  println!(
    "cargo:rustc-env=BUILD_TIMESTAMP={}",
    std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_secs()
  );

  // Store the target architecture
  println!("cargo:rustc-env=TARGET={}", env::var("TARGET").unwrap_or_default());
}

/// Configures conditions that trigger build script re-execution.
///
/// Monitored conditions:
/// - Changes to this build script itself
/// - Git HEAD changes (for commit hash updates)
/// - TARGET environment variable changes (for cross-compilation)
fn set_rerun_conditions() {
  // Re-run when this build script is modified
  println!("cargo:rerun-if-changed=build.rs");

  // Re-run when Git HEAD changes to update commit hash
  println!("cargo:rerun-if-changed=.git/HEAD");

  // Re-run when target architecture changes during cross-compilation
  println!("cargo:rerun-if-env-changed=TARGET");
}
