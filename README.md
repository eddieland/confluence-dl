# confluence-dl

A fast, intuitive CLI tool for exporting Confluence pages to Markdown.

## Overview

`confluence-dl` is a Rust-powered command-line utility that downloads Confluence pages and converts them to clean Markdown. Whether you need to backup documentation, migrate to a static site generator, or work offline, confluence-dl makes it simple.

**For AI Code Assistants**: Local Confluence docs enable GitHub Copilot, Cursor, and Cline to reference your documentation directly in your workspace without a complicated RAG pipeline.

**Current Status**: v0.1 - Core functionality for page-based exports

## Quick Start

```bash
# Export a single page
confluence-dl https://your-domain.atlassian.net/wiki/pages/123456/My+Page

# Export a page with all children (recursive)
confluence-dl https://your-domain.atlassian.net/wiki/pages/123456 --children

# Test your authentication first
confluence-dl auth test
```

## What Do You Want to Do?

### 📄 "I want to export a single page"

The simplest case - just provide the page URL:

```bash
confluence-dl https://your-domain.atlassian.net/wiki/spaces/DOCS/pages/123456/Getting+Started
```

Or use the page ID with a base URL:

```bash
confluence-dl 123456 --url https://your-domain.atlassian.net
```

**Output**: Creates `./confluence-export/Getting-Started.md` with embedded images downloaded to `./confluence-export/images/`

### 📚 "I want to export a page and all its children"

Add the `--children` flag to download the entire page tree:

```bash
confluence-dl https://your-domain.atlassian.net/wiki/pages/123456 --children
```

Limit how deep you go:

```bash
confluence-dl 123456 --url https://your-domain.atlassian.net --children --max-depth 2
```

**Output**: Creates a directory structure matching your page hierarchy, with all child pages as individual Markdown files.

### 💾 "I want a complete backup with attachments"

Include everything - child pages, attachments, and comments:

```bash
confluence-dl 123456 \
  --url https://your-domain.atlassian.net \
  --children \
  --attachments \
  --comments \
  -o ./backup
```

**Output**: Full backup in `./backup/` with all files, attachments, and metadata preserved.

### 🔍 "I want to preview what will be downloaded"

Use `--dry-run` to see what would happen without actually downloading:

```bash
confluence-dl 123456 --url https://your-domain.atlassian.net --children --dry-run -v
```

**Output**: Shows the page tree and what files would be created, without downloading anything.

### ⚙️ "I want to customize the output"

Control where files go and how they're formatted:

```bash
confluence-dl 123456 \
  --url https://your-domain.atlassian.net \
  --children \
  -o ./my-docs \
  --images-dir assets \
  --overwrite
```

**Options**:

- `-o, --output <DIR>`: Output directory (default: `./confluence-export`)
- `--images-dir <DIR>`: Where to save images (default: `images`)
- `--overwrite`: Replace existing files instead of skipping
- `--format <FORMAT>`: Output format - markdown (default), json, or html

### 🐚 "I want shell completions"

Generate completion scripts for your shell:

```bash
# Bash (user-level, no sudo required)
mkdir -p ~/.local/share/bash-completion/completions
confluence-dl completions bash > ~/.local/share/bash-completion/completions/confluence-dl

# Zsh
confluence-dl completions zsh > ~/.zsh/completions/_confluence-dl

# Fish
confluence-dl completions fish > ~/.config/fish/completions/confluence-dl.fish
```

Supported shells: bash, zsh, fish, powershell, elvish

## Authentication

`confluence-dl` supports multiple authentication methods. Choose the one that best fits your security requirements and workflow.

**Getting an API token**: Create one at [https://id.atlassian.com/manage-profile/security/api-tokens](https://id.atlassian.com/manage-profile/security/api-tokens)

### Command-line Flags

Explicit credentials for testing or one-off exports:

```bash
confluence-dl --url https://your-domain.atlassian.net \
              --user your-email@example.com \
              --token your-api-token \
              123456
```

**Use when**: Testing authentication, one-time exports, or scripts where credentials are managed externally.

### Environment Variables

Common in CI/CD pipelines and containerized environments:

```bash
export CONFLUENCE_URL=https://your-domain.atlassian.net
export CONFLUENCE_USER=your-email@example.com
export CONFLUENCE_TOKEN=your-api-token

confluence-dl 123456
```

**Use when**: Running in automated environments, containers, or CI/CD systems.

### `.netrc` File

Standard credential storage supported by many tools:

Add to `~/.netrc` (Unix/macOS) or `~/_netrc` (Windows):

```netrc
machine your-domain.atlassian.net
login your-email@example.com
password your-api-token
```

**Use when**: You want a persistent, tool-agnostic credential store. Remember to set appropriate permissions (`chmod 600 ~/.netrc` on Unix/macOS).

### Security Best Practices

- **Follow your organization's guidelines**: Consult your IT/security team for approved credential management practices
- **Use the most secure option available**: If your environment supports credential managers or secret vaults, prefer those
- **Protect your credentials**: Never commit API tokens to version control or share them publicly
- **Rotate tokens regularly**: Set expiration dates and rotate API tokens according to your security policy
- **Limit token scope**: Use the minimum required permissions for your API tokens

### Credential Precedence

When multiple methods are configured, `confluence-dl` checks them in this order:

1. Command-line flags (`--user`, `--token`, `--url`)
2. Environment variables (`CONFLUENCE_USER`, `CONFLUENCE_TOKEN`, `CONFLUENCE_URL`)
3. `.netrc` file

This allows you to override stored credentials for specific operations.

### Future Authentication Methods

We welcome contributions for additional authentication methods! Areas of interest:

- **OS Keychain integration**: macOS Keychain, Windows Credential Manager, GNOME Keyring
- **Secret manager support**: HashiCorp Vault, AWS Secrets Manager, Azure Key Vault
- **SSO/OAuth flows**: Interactive authentication for organizations using SSO

See our [contribution guidelines](CONTRIBUTING.md) if you'd like to help implement these features.

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

## Common Options

### Page-Specific

- `--children`: Download child pages recursively
- `--max-depth <N>`: Limit recursion depth
- `--attachments`: Download page attachments
- `--comments`: Include comments in export

### Output Control

- `-o, --output <DIR>`: Output directory (default: `./confluence-export`)
- `--format <FORMAT>`: Output format: markdown, json, html (default: markdown)
- `--overwrite`: Overwrite existing files

### Behavior

- `--dry-run`: Preview without downloading
- `--verbose, -v`: Increase verbosity (-v, -vv, -vvv)
- `--quiet, -q`: Suppress all output except errors
- `--color <WHEN>`: Colorize output (auto, always, never)

### Images & Links

- `--download-images`: Download embedded images (default: true)
- `--images-dir <DIR>`: Directory for images (default: images)
- `--convert-links`: Convert Confluence links to markdown (default: true)

### Performance

- `--parallel <N>`: Number of parallel downloads (default: 4)
- `--rate-limit <N>`: Max requests per second (default: 10)
- `--timeout <SECONDS>`: Request timeout (default: 30)

For complete option details, run:

```bash
confluence-dl --help
```

## Development

### Prerequisites

- Rust 1.90 or later
- **cargo-nextest** (required for running tests)

**⚠️ IMPORTANT**: This project uses `cargo-nextest` as the ONLY supported test runner. Standard `cargo test` is not supported.

Install nextest:

```bash
cargo install cargo-nextest --locked
```

### Quick Commands

```bash
make build              # Build debug version
make test               # Run tests with nextest
make fmt                # Format code
make lint               # Run clippy
make all                # Format, lint, and test
make release            # Build optimized binary
```

### Running Tests

**⚠️ This project ONLY supports `cargo nextest` - do NOT use `cargo test`.**

```bash
make test
# OR
cargo nextest run
```

### Code Quality

- Formatting: [`rustfmt`](.rustfmt.toml:1) with nightly features
- Linting: [`clippy`](.clippy.toml:1) configured to deny warnings
- Testing: Comprehensive E2E tests using stub-based approach

See [`AGENTS.md`](AGENTS.md:1) for detailed development guidelines.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE:1) file for details.
