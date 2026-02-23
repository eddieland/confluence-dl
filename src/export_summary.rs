//! Export summary statistics and dry-run output preview.
//!
//! This module provides [`ExportStats`] for tracking page/image/attachment
//! counts during a download, and helpers for rendering a planned output
//! directory tree when running in `--dry-run` mode.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use crate::color::ColorScheme;
use crate::confluence::PageTree;
use crate::format::OutputFormat;
use crate::processed_page::sanitize_filename;

/// Tracks cumulative statistics during a page export operation.
///
/// All counters use atomics so they can be safely updated from concurrent
/// download tasks that share an `Arc<ExportStats>`.
pub struct ExportStats {
  pages: AtomicUsize,
  images: AtomicUsize,
  attachments: AtomicUsize,
  started_at: Instant,
}

impl ExportStats {
  pub fn new() -> Self {
    Self {
      pages: AtomicUsize::new(0),
      images: AtomicUsize::new(0),
      attachments: AtomicUsize::new(0),
      started_at: Instant::now(),
    }
  }

  /// Record that a page was successfully exported.
  pub fn record_page(&self) {
    self.pages.fetch_add(1, Ordering::Relaxed);
  }

  /// Record downloaded images for a single page.
  pub fn record_images(&self, count: usize) {
    self.images.fetch_add(count, Ordering::Relaxed);
  }

  /// Record downloaded attachments for a single page.
  pub fn record_attachments(&self, count: usize) {
    self.attachments.fetch_add(count, Ordering::Relaxed);
  }

  pub fn pages(&self) -> usize {
    self.pages.load(Ordering::Relaxed)
  }

  pub fn images(&self) -> usize {
    self.images.load(Ordering::Relaxed)
  }

  pub fn attachments(&self) -> usize {
    self.attachments.load(Ordering::Relaxed)
  }

  pub fn elapsed(&self) -> std::time::Duration {
    self.started_at.elapsed()
  }

  /// Print a short summary block after downloads finish.
  pub fn print_summary(&self, colors: &ColorScheme) {
    let elapsed_str = format_duration(self.elapsed());
    let pages = self.pages();
    let images = self.images();
    let attachments = self.attachments();

    println!();
    println!(
      "{} {}",
      colors.success("✓"),
      colors.success("Export summary")
    );
    println!(
      "  {}: {} exported",
      colors.emphasis("Pages"),
      colors.number(pages)
    );

    if images > 0 {
      println!(
        "  {}: {} downloaded",
        colors.emphasis("Images"),
        colors.number(images)
      );
    }

    if attachments > 0 {
      println!(
        "  {}: {} downloaded",
        colors.emphasis("Attachments"),
        colors.number(attachments)
      );
    }

    println!(
      "  {}: {}",
      colors.emphasis("Duration"),
      colors.dimmed(&elapsed_str)
    );
  }
}

/// Format a [`std::time::Duration`] for human display.
fn format_duration(d: std::time::Duration) -> String {
  let secs = d.as_secs_f64();
  if secs < 1.0 {
    format!("{:.0}ms", d.as_millis())
  } else if secs < 60.0 {
    format!("{secs:.1}s")
  } else {
    let mins = secs as u64 / 60;
    let remaining = secs as u64 % 60;
    format!("{mins}m {remaining}s")
  }
}

// ---------------------------------------------------------------------------
// Dry-run output tree preview
// ---------------------------------------------------------------------------

/// An entry in the planned output directory structure.
enum OutputEntry {
  File(String),
  Dir(String, Vec<OutputEntry>),
}

/// Build the planned output tree for a recursive (children) export.
///
/// Returns formatted lines ready to print, with tree-drawing characters
/// matching the style used by `ls`.
pub fn format_output_tree(
  tree: &PageTree,
  output_dir: &Path,
  format: OutputFormat,
  colors: &ColorScheme,
) -> Vec<String> {
  let ext = format.file_extension();
  let entries = page_tree_to_entries(tree, ext);

  let mut lines = Vec::new();
  let header = format!("{}/", output_dir.display());
  lines.push(format!("{}", colors.path(header)));
  render_entries(&entries, "  ", colors, &mut lines);
  lines
}

/// Build the planned output path for a single-page export.
pub fn format_single_page_output(title: &str, output_dir: &Path, format: OutputFormat, colors: &ColorScheme) -> String {
  let filename = sanitize_filename(title);
  let ext = format.file_extension();
  let path = output_dir.join(format!("{filename}.{ext}"));
  format!("{}", colors.path(path.display()))
}

/// Convert a [`PageTree`] into output directory entries.
fn page_tree_to_entries(tree: &PageTree, ext: &str) -> Vec<OutputEntry> {
  let filename = sanitize_filename(&tree.page.title);
  let mut entries = vec![OutputEntry::File(format!("{filename}.{ext}"))];

  if !tree.children.is_empty() {
    let mut dir_children = Vec::new();
    for child in &tree.children {
      dir_children.extend(page_tree_to_entries(child, ext));
    }
    entries.push(OutputEntry::Dir(format!("{filename}/"), dir_children));
  }

  entries
}

/// Render output entries as tree-drawing lines.
fn render_entries(entries: &[OutputEntry], prefix: &str, colors: &ColorScheme, lines: &mut Vec<String>) {
  for (idx, entry) in entries.iter().enumerate() {
    let is_last = idx + 1 == entries.len();
    let connector = if is_last { "└── " } else { "├── " };
    let next_prefix = if is_last {
      format!("{prefix}    ")
    } else {
      format!("{prefix}│   ")
    };

    match entry {
      OutputEntry::File(name) => {
        lines.push(format!("{prefix}{connector}{}", colors.path(name)));
      }
      OutputEntry::Dir(name, children) => {
        lines.push(format!("{prefix}{connector}{}", colors.path(name)));
        render_entries(children, &next_prefix, colors, lines);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::cli::ColorOption;
  use crate::confluence::{Page, PageTree};

  fn make_page(id: &str, title: &str) -> Page {
    Page {
      id: id.to_string(),
      title: title.to_string(),
      page_type: "page".to_string(),
      status: "current".to_string(),
      body: None,
      space: None,
      links: None,
    }
  }

  #[test]
  fn format_duration_millis() {
    let d = std::time::Duration::from_millis(450);
    assert_eq!(format_duration(d), "450ms");
  }

  #[test]
  fn format_duration_seconds() {
    let d = std::time::Duration::from_secs_f64(3.7);
    assert_eq!(format_duration(d), "3.7s");
  }

  #[test]
  fn format_duration_minutes() {
    let d = std::time::Duration::from_secs(125);
    assert_eq!(format_duration(d), "2m 5s");
  }

  #[test]
  fn export_stats_tracks_counters() {
    let stats = ExportStats::new();
    assert_eq!(stats.pages(), 0);
    assert_eq!(stats.images(), 0);
    assert_eq!(stats.attachments(), 0);

    stats.record_page();
    stats.record_page();
    stats.record_images(3);
    stats.record_attachments(1);

    assert_eq!(stats.pages(), 2);
    assert_eq!(stats.images(), 3);
    assert_eq!(stats.attachments(), 1);
  }

  #[test]
  fn single_page_output_format() {
    let colors = ColorScheme::new(ColorOption::Never);
    let result = format_single_page_output("My Page", Path::new("./export"), OutputFormat::Markdown, &colors);
    assert!(result.contains("My Page.md"));
    assert!(result.contains("export"));
  }

  #[test]
  fn single_page_output_asciidoc() {
    let colors = ColorScheme::new(ColorOption::Never);
    let result = format_single_page_output("My Page", Path::new("./export"), OutputFormat::AsciiDoc, &colors);
    assert!(result.contains("My Page.adoc"));
  }

  #[test]
  fn output_tree_single_root_no_children() {
    let colors = ColorScheme::new(ColorOption::Never);
    let tree = PageTree {
      page: make_page("1", "Root"),
      depth: 0,
      children: vec![],
    };

    let lines = format_output_tree(&tree, Path::new("./out"), OutputFormat::Markdown, &colors);
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("out/"));
    assert!(lines[1].contains("Root.md"));
  }

  #[test]
  fn output_tree_with_children() {
    let colors = ColorScheme::new(ColorOption::Never);
    let tree = PageTree {
      page: make_page("1", "Root"),
      depth: 0,
      children: vec![
        PageTree {
          page: make_page("2", "Child A"),
          depth: 1,
          children: vec![PageTree {
            page: make_page("3", "Grandchild"),
            depth: 2,
            children: vec![],
          }],
        },
        PageTree {
          page: make_page("4", "Child B"),
          depth: 1,
          children: vec![],
        },
      ],
    };

    let lines = format_output_tree(&tree, Path::new("./out"), OutputFormat::Markdown, &colors);

    // Should contain: header, Root.md, Root/ dir, Child A.md, Child A/ dir, Grandchild.md, Child B.md
    assert!(lines.len() >= 7, "expected at least 7 lines, got {}: {lines:?}", lines.len());

    // Verify key entries exist
    let joined = lines.join("\n");
    assert!(joined.contains("Root.md"), "missing Root.md");
    assert!(joined.contains("Root/"), "missing Root/ dir");
    assert!(joined.contains("Child A.md"), "missing Child A.md");
    assert!(joined.contains("Child A/"), "missing Child A/ dir");
    assert!(joined.contains("Grandchild.md"), "missing Grandchild.md");
    assert!(joined.contains("Child B.md"), "missing Child B.md");
  }

  #[test]
  fn output_tree_uses_tree_connectors() {
    let colors = ColorScheme::new(ColorOption::Never);
    let tree = PageTree {
      page: make_page("1", "Root"),
      depth: 0,
      children: vec![
        PageTree {
          page: make_page("2", "First"),
          depth: 1,
          children: vec![],
        },
        PageTree {
          page: make_page("3", "Last"),
          depth: 1,
          children: vec![],
        },
      ],
    };

    let lines = format_output_tree(&tree, Path::new("./out"), OutputFormat::Markdown, &colors);
    let joined = lines.join("\n");

    // First file uses ├──, last dir entry uses └──
    assert!(joined.contains("├──"), "should contain ├── connector");
    assert!(joined.contains("└──"), "should contain └── connector");
  }
}
