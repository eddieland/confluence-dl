# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## ⚠️ CRITICAL: Test Runner Requirements

**DO NOT use `cargo test`. This project ONLY supports `cargo nextest run`.**

The standard `cargo test` command is explicitly unsupported and should never be used. Always use `make test` or `cargo nextest run` instead.

Install nextest if needed: `cargo install cargo-nextest --locked`

## Build & Test Commands

```bash
make all                # Format, lint, and test (run before committing)
make build              # Build debug version
make test               # Run tests with nextest (⛔ NEVER use cargo test)
make fmt                # Format code and auto-fix clippy issues
make lint               # Run clippy with -D warnings
make release            # Build optimized binary
```

### Snapshot Testing (insta)

```bash
make update-snapshots   # Update snapshots: INSTA_UPDATE=1 cargo nextest run
make insta-review       # Review snapshot changes interactively
```

### Running Locally

```bash
cargo run -- <args>     # Pass CLI args after --
cargo run -- --help     # Test CLI help
```

## Architecture

### Module Layout

- **`src/main.rs`** - Entry point, tracing setup, subcommand dispatch
- **`src/cli.rs`** - Clap-based CLI definition with derive macros
- **`src/commands/`** - Command handlers: `auth`, `completions`, `ls`, `page`, `version`
- **`src/confluence/`** - Confluence API integration:
  - `api.rs` - `ConfluenceApi` trait (enables test mocking via `FakeConfluenceClient`)
  - `client.rs` - HTTP client implementation
  - `models.rs` - API response types
  - `tree.rs` - Page hierarchy traversal
  - `url.rs` - Confluence URL parsing
- **`src/markdown/`** - Confluence storage format → Markdown conversion:
  - `mod.rs` - Main entry point (`storage_to_markdown_with_options`)
  - `elements.rs` - HTML element converters
  - `tables.rs` - Table rendering
  - `macros/` - Confluence macro handlers (panels, code blocks, admonitions, etc.)
- **`src/credentials/`** - Auth providers: CLI flags, env vars, `.netrc`
- **`src/color.rs`** - Semantic terminal coloring (`ColorScheme`)
- **`src/images.rs`, `src/attachments.rs`** - Asset downloading

### Testing Pattern

E2E tests use a stub-based approach with trait-based dependency injection:

- `ConfluenceApi` trait in `src/confluence/api.rs` defines the interface
- `FakeConfluenceClient` in `tests/common/fake_confluence.rs` provides test doubles
- Pre-built fixtures in `tests/common/fixtures.rs` simulate API responses

## Code Style

- **Line width**: 120 characters
- **Indentation**: 2 spaces
- **Edition**: 2024
- **MSRV**: 1.90.0
- **Toolchain**: Nightly (for rustfmt unstable features)

Linting thresholds:

- Cognitive complexity: max 25 per function
- Function length: max 100 lines
- Function arguments: max 7

## Development Notes

- Run `make fmt` after any code changes to apply rustfmt and clippy auto-fixes
- Build metadata (git hash, timestamp, target arch) is embedded via `build.rs`
- Use semantic color methods from `ColorScheme` instead of raw ANSI codes
