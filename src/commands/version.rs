use crate::color::ColorScheme;

/// Handle version command
pub(crate) fn handle_version_command(json: bool, short: bool, colors: &ColorScheme) {
  let version = env!("CARGO_PKG_VERSION");

  if short {
    println!("{version}");
    return;
  }

  if json {
    // Output JSON format (no colors in JSON)
    let git_hash = env!("GIT_HASH");
    let build_timestamp = env!("BUILD_TIMESTAMP");
    let target = env!("TARGET");

    println!("{{");
    println!("  \"version\": \"{version}\",");
    println!("  \"git_commit\": \"{git_hash}\",");
    println!("  \"build_timestamp\": \"{}\",", format_timestamp(build_timestamp));
    println!("  \"target\": \"{target}\",");
    println!("  \"rust_version\": \"{}\"", rustc_version());
    println!("}}");
  } else {
    // Output human-readable format with colors
    let git_hash = env!("GIT_HASH");
    let build_timestamp = env!("BUILD_TIMESTAMP");
    let target = env!("TARGET");

    println!("{} {}", colors.emphasis("confluence-dl"), colors.number(version));
    println!("{}: {}", colors.emphasis("Git commit"), colors.code(git_hash));
    println!(
      "{}: {}",
      colors.emphasis("Built"),
      colors.dimmed(format_timestamp(build_timestamp))
    );
    println!("{}: {}", colors.emphasis("Target"), target);
    println!("{}: {}", colors.emphasis("Rust version"), rustc_version());
  }
}

/// Format Unix timestamp as ISO 8601 UTC string
fn format_timestamp(timestamp: &str) -> String {
  timestamp
    .parse::<i64>()
    .ok()
    .and_then(|ts| {
      use std::time::{Duration, UNIX_EPOCH};
      UNIX_EPOCH.checked_add(Duration::from_secs(ts as u64))
    })
    .map(|time| {
      let datetime: chrono::DateTime<chrono::Utc> = time.into();
      datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    })
    .unwrap_or_else(|| timestamp.to_string())
}

/// Get Rust compiler version
///
/// Returns the actual rustc version used to build this binary,
/// captured at build time by build.rs.
fn rustc_version() -> String {
  env!("RUSTC_VERSION").to_string()
}
