# confluence-dl

A Rust CLI tool for exporting Confluence spaces and pages to Markdown.

## Overview

`confluence-dl` is a command-line utility that allows you to download and convert Confluence documentation to Markdown format. It's designed to help teams migrate their documentation, create local backups, or integrate Confluence content into static site generators.

## Features

- 🚀 Fast and efficient Rust implementation
- 📄 Export individual Confluence pages to Markdown
- 📚 Export entire Confluence spaces
- 🔄 Preserve document structure and hierarchy
- 🖼️ Download and reference embedded images
- 🔗 Convert internal links to work with Markdown
- ⚙️ Configurable output formatting

## Installation

### From Crates.io (Coming Soon)

```bash
cargo install confluence-dl
```

### From Source

```bash
git clone https://github.com/eddieland/confluence-dl
cd confluence-dl
cargo build --release
```

The binary will be available at `target/release/confluence-dl`.

## Usage

### Export a Single Page

```bash
confluence-dl page <PAGE_URL> -o output/
```

### Export an Entire Space

```bash
confluence-dl space <SPACE_KEY> -o output/
```

### Authentication

Configure authentication using environment variables:

```bash
export CONFLUENCE_URL=https://your-domain.atlassian.net
export CONFLUENCE_USER=your-email@example.com
export CONFLUENCE_TOKEN=your-api-token
```

Or provide credentials via command-line options:

```bash
confluence-dl --url https://your-domain.atlassian.net \
              --user your-email@example.com \
              --token your-api-token \
              space MYSPACE
```

## Development

### Prerequisites

- Rust 1.90 or later
- Cargo
- **cargo-nextest** (required for running tests)

**⚠️ IMPORTANT**: This project uses `cargo-nextest` as the ONLY supported test runner. Standard `cargo test` is not supported.

Install nextest if you don't have it:
```bash
cargo install cargo-nextest --locked
```

### Building

```bash
make build
```

### Running Tests

**⚠️ This project ONLY supports `cargo nextest` for running tests. Do NOT use `cargo test`.**

```bash
make test                    # Runs cargo nextest run
# OR
cargo nextest run            # Run tests directly with nextest
```

If you don't have nextest installed:
```bash
cargo install cargo-nextest --locked
```

### Linting

```bash
make lint
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
