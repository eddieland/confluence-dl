# AI Agent Guide for confluence-dl

## Project Overview

`confluence-dl` is a Rust CLI tool for exporting Confluence spaces and pages to Markdown. The project is in early development with core infrastructure in place but minimal implementation.

## Architecture

### Project Structure
```
confluence-dl/
├── src/
│   └── main.rs          # Entry point (currently minimal)
├── Cargo.toml           # Dependencies: clap, clap_complete
├── build.rs             # Embeds git hash, timestamp, target arch
├── Makefile             # Development workflow automation
└── *.toml               # Rust tooling configuration
```

### Build System
- [`build.rs`](build.rs:1) runs at compile time to embed metadata:
  - Git commit hash via `git rev-parse --short HEAD`
  - Build timestamp as Unix epoch seconds
  - Target architecture from `TARGET` env var
- Access these at runtime via `env!("GIT_HASH")`, `env!("BUILD_TIMESTAMP")`, `env!("TARGET")`

## Development Workflow

### Essential Commands (via Makefile)

```bash
make build              # Build debug version
make test               # Run tests with nextest
make fmt                # Format with rustfmt + clippy --fix
make lint               # Clippy with -D warnings (fails on warnings)
make all                # fmt + lint + test
make release            # Build optimized binary
```

### Testing Strategy

**⚠️ CRITICAL: This project ONLY supports `cargo nextest` for running tests. Standard `cargo test` is NOT supported and should NOT be used.**

- **Test Runner**: [`cargo-nextest`](Makefile:27) is the ONLY supported test runner
  - `cargo nextest` is faster, more reliable, and better integrated with our tooling
  - Do NOT use `cargo test` - it is explicitly not supported in this project
- **Test Location**: Tests live alongside implementation (unit tests) or in `tests/` (integration tests)
- **Snapshot Testing**: Uses [`insta`](Makefile:51-64) for snapshot testing
  - Update snapshots: `make update-snapshots` or `INSTA_UPDATE=1 cargo nextest run`
  - Review snapshots: `make insta-review` or `cargo insta review`
- **E2E Testing**: Uses stub-based approach with [`FakeConfluenceClient`](tests/common/fake_confluence.rs:17)
  - See [`tests/README.md`](tests/README.md:1) for comprehensive testing guide
  - Trait-based design via [`ConfluenceApi`](src/confluence.rs:14) enables dependency injection
  - Pre-built fixtures in [`tests/common/fixtures.rs`](tests/common/fixtures.rs:1) provide realistic API responses
  - Fast, simple, and maintainable - no HTTP mocking required

**Installation**: If you don't have nextest installed:
```bash
cargo install cargo-nextest --locked
```

### Code Quality Tools
- **Formatter**: [`rustfmt`](.rustfmt.toml:1) with nightly features enabled
- **Linter**: [`clippy`](.clippy.toml:1) configured to deny warnings
- **Coverage**: [`cargo-llvm-cov`](Makefile:76-90) for code coverage reports

## Rust-Specific Conventions

### Formatting Rules ([.rustfmt.toml](.rustfmt.toml:1))
- **Max line width**: 120 characters
- **Indentation**: 2 spaces (not 4)
- **Imports**: Grouped by std/external crates/local, merged by module
- **Edition**: 2024 (latest Rust edition)
- **Line endings**: Unix (LF)

### Linting Configuration ([.clippy.toml](.clippy.toml:1))
- **MSRV**: 1.90.0 (enforce this minimum version)
- **Cognitive complexity**: Max 25 per function
- **Function arguments**: Max 7 parameters
- **Function length**: Max 100 lines
- **Doc identifiers**: "GitHub" and "Confluence" don't need backticks

### Toolchain ([rust-toolchain.toml](rust-toolchain.toml:1))
- **Channel**: `nightly` (required for rustfmt unstable features)
- **Components**: clippy, rustfmt, rust-src, llvm-tools-preview

## Code Patterns

### Error Handling
When implementing error handling:
- Use `anyhow` or `thiserror` for error types
- Return `Result<T, E>` from fallible functions
- Use `?` operator for error propagation

### CLI Structure
Uses [`clap`](Cargo.toml:10) v4 with derive macros for argument parsing and [`clap_complete`](Cargo.toml:11) for shell completion. See [`src/cli.rs`](src/cli.rs:1) for the complete CLI implementation.

## Common Tasks

### Adding Dependencies
```bash
cargo add <crate>              # Add to Cargo.toml
cargo add --dev <crate>        # Add dev dependency
```

### Running Locally
```bash
cargo run -- <args>            # Pass args after --
cargo run -- --help            # Test CLI help
```

### Debugging Builds
Build metadata is embedded at compile time:
- Check [`build.rs`](build.rs:1) for what's captured
- Access via `env!()` macro in Rust code
- Rerun triggers: build.rs changes, .git/HEAD changes, TARGET env changes

## Project Goals

Based on [README.md](README.md:1):
1. Export individual Confluence pages to Markdown
2. Export entire Confluence spaces
3. Preserve document hierarchy and structure
4. Download and reference embedded images
5. Convert internal Confluence links to Markdown-compatible links
6. Support authentication via env vars or CLI flags

## Current State

- **Implemented**: Build infrastructure, tooling configuration, project scaffolding
- **Not Implemented**: Core functionality (Confluence API client, Markdown conversion, file I/O)
- **Next Steps**: Implement CLI argument parsing, Confluence API client, page fetching

## Color and Visual Design

The application uses a comprehensive color system to improve user experience and "feel". Colors are used semantically to convey meaning and guide user attention.

### Color Module ([`src/color.rs`](src/color.rs:1))

The [`ColorScheme`](src/color.rs:13) struct provides semantic color methods that respect user preferences:
- Automatically detects terminal capabilities
- Respects `--color` flag (auto/always/never)
- Falls back gracefully when colors are disabled

### Semantic Color Methods

Use these methods instead of raw ANSI codes:

| Method | Color | Use For | Example |
|--------|-------|---------|---------|
| [`success()`](src/color.rs:41) | Green | Successful operations, confirmations | "✓ Downloaded 5 pages" |
| [`error()`](src/color.rs:50) | Bright Red (bold) | Error messages, failures | "✗ Failed to connect" |
| [`warning()`](src/color.rs:59) | Yellow | Warnings, cautionary messages | "⚠ File already exists" |
| [`info()`](src/color.rs:68) | Cyan | Informational messages | "Fetching page metadata..." |
| [`debug()`](src/color.rs:77) | Bright Black (gray) | Debug/verbose output | "API response: 200 OK" |
| [`emphasis()`](src/color.rs:86) | Bright White (bold) | Important text, headers | "Authentication:" |
| [`link()`](src/color.rs:95) | Blue (underlined) | URLs, clickable items | "https://confluence.example.com" |
| [`path()`](src/color.rs:104) | Magenta | File paths, directories | "./output/page.md" |
| [`number()`](src/color.rs:113) | Bright Blue | Numbers, metrics, counts | "42 pages" |
| [`code()`](src/color.rs:122) | Bright Green | Code snippets, commands | "`confluence-dl --help`" |
| [`dimmed()`](src/color.rs:131) | Gray (dimmed) | Secondary/less important text | "(optional)" |
| [`progress()`](src/color.rs:140) | Bright Cyan | Progress indicators, ongoing tasks | "Downloading..." |

### Best Practices for Color Usage

1. **Always Use Semantic Methods**
   ```rust
   // ✓ GOOD - Semantic and meaningful
   println!("{} {}", colors.success("✓"), colors.info("Download complete"));

   // ✗ BAD - Raw colors without meaning
   println!("\x1b[32m✓\x1b[0m Download complete");
   ```

2. **Respect User Preferences**
   ```rust
   // ✓ GOOD - ColorScheme respects --color flag
   let colors = ColorScheme::new(cli.behavior.color);
   println!("{}", colors.error("Connection failed"));

   // ✗ BAD - Forces colors regardless of user preference
   println!("\x1b[31mConnection failed\x1b[0m");
   ```

3. **Never Rely Solely on Color**
   ```rust
   // ✓ GOOD - Icon + color conveys meaning
   println!("{} {}", colors.success("✓"), colors.info("Success"));

   // ✗ BAD - Only color, no visual indicator
   println!("{}", colors.success("Success"));
   ```

4. **Consistent Color Meanings**
   - Green = Success, positive outcomes
   - Red = Errors, failures, stop
   - Yellow = Warnings, caution
   - Blue/Cyan = Information, links
   - Magenta = Files/paths
   - Gray = Diminished importance

5. **Accessibility Considerations**
   - Always include icons or text indicators (✓, ✗, ⚠, →)
   - Ensure good contrast for readability
   - Test with both light and dark terminal backgrounds
   - Color should enhance, not replace, textual information

### Clap Color Styling

The CLI help output uses custom colors defined in [`get_clap_styles()`](src/cli.rs:266):
- **Headers/Usage**: Bright Yellow + Bold
- **Literals** (commands, flags): Bright Green
- **Placeholders** (<args>): Bright Cyan
- **Errors**: Bright Red + Bold
- **Valid values**: Bright Green
- **Invalid values**: Bright Red

These colors create a consistent, professional appearance for `--help` output.

### Example: Complete Feature with Colors

```rust
fn download_page(url: &str, cli: &Cli, colors: &ColorScheme) {
  // Progress indicator
  println!("{} {}", colors.progress("→"), colors.info("Downloading page..."));
  println!("  {}: {}", colors.emphasis("URL"), colors.link(url));

  match fetch_page(url) {
    Ok(page) => {
      // Success with metrics
      println!("{} {}", colors.success("✓"), colors.info("Download complete"));
      println!("  {}: {}", colors.emphasis("Size"), colors.number(page.size));
      println!("  {}: {}", colors.emphasis("Output"), colors.path(&cli.output.output));
    }
    Err(e) => {
      // Error with details
      eprintln!("{} {}", colors.error("✗"), colors.error("Download failed"));
      eprintln!("  {}: {}", colors.emphasis("Reason"), e);
      eprintln!("  {}", colors.dimmed("Hint: Check your network connection"));
    }
  }
}
```

### Testing Colors

When adding new output:
1. Test with `--color=always` to verify colors appear correct
2. Test with `--color=never` to ensure output is still readable
3. Test with both light and dark terminal backgrounds
4. Verify icons render correctly in your terminal font

## Tips for AI Agents

1. **Always use `cargo nextest run`**, never `cargo test` - see Testing Strategy section
2. **Run `make all`** (fmt + lint + test) before suggesting changes are complete
3. **Write descriptive doc comments** - explain the "why" not just the "what"
4. **Add tests alongside new features** - use `#[cfg(test)]` modules
5. **When adding features**, update the appropriate section of this file

Note: Formatting and linting rules are enforced automatically by rustfmt and clippy (see Rust-Specific Conventions section).
