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

**⚠️ IMPORTANT: This project ONLY supports `cargo nextest` for running tests. Standard `cargo test` is NOT supported and should NOT be used.**

- **Test Runner**: [`cargo-nextest`](Makefile:27) is the ONLY supported test runner
  - `cargo nextest` is faster, more reliable, and better integrated with our tooling
  - Do NOT use `cargo test` - it is explicitly not supported in this project
- **Snapshot Testing**: Uses [`insta`](Makefile:51-64) for snapshot testing
  - Update snapshots: `make update-snapshots` or `INSTA_UPDATE=1 cargo nextest run`
  - Review snapshots: `make insta-review`

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
- **MSRV**: 1.88.0 (enforce this minimum version)
- **Cognitive complexity**: Max 25 per function
- **Function arguments**: Max 7 parameters
- **Function length**: Max 100 lines
- **Doc identifiers**: "GitHub" and "Confluence" don't need backticks

### Toolchain ([rust-toolchain.toml](rust-toolchain.toml:1))
- **Channel**: `nightly` (required for rustfmt unstable features)
- **Components**: clippy, rustfmt, rust-src, llvm-tools-preview

## Code Patterns

### Error Handling
No error handling patterns established yet. When implementing:
- Use `anyhow` or `thiserror` for error types (add to Cargo.toml)
- Return `Result<T, E>` from fallible functions
- Use `?` operator for error propagation

### CLI Structure (Planned)
Current dependencies indicate CLI will use:
- [`clap`](Cargo.toml:10) v4 with derive macros for argument parsing
- [`clap_complete`](Cargo.toml:11) for shell completion generation

Example CLI structure (not yet implemented):
```rust
use clap::Parser;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    // Define commands here
}
```

### Testing Conventions

**⚠️ CRITICAL: Always use `cargo nextest run`, NEVER use `cargo test`**

- Tests live alongside implementation (unit tests) or in `tests/` (integration tests)
- Run tests via `make test` or `cargo nextest run` - **DO NOT use `cargo test`**
- Snapshot tests use [`insta`](https://insta.rs) - review with `cargo insta review`
- All test commands in this project assume nextest is available

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

### Before Committing
```bash
make all                       # Runs fmt, lint, test
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

## Tips for AI Agents

1. **NEVER use `cargo test`** - This project ONLY supports `cargo nextest run`. Always use `make test` or `cargo nextest run`
2. **Always run `make all`** before suggesting changes are complete
3. **Respect the 120-char line limit** - rustfmt will enforce this
4. **Use 2-space indentation** - project uses 2 spaces, not Rust's typical 4
5. **Write descriptive doc comments** - explain the "why" not just the "what"
6. **Keep functions under 100 lines** - clippy enforces this
7. **Group imports properly** - std, external crates, then local (rustfmt handles this)
8. **Add tests alongside new features** - use `#[cfg(test)]` modules
9. **When adding features**, update the appropriate section of this file
