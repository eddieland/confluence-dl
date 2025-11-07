# CLI Interface Design for confluence-dl

## Overview

This document specifies the command-line interface for `confluence-dl`, a tool for exporting Confluence pages to Markdown. The CLI is designed to be intuitive and expressive, with the primary function at the root level and debugging/introspection subcommands.

## Design Principles

1. **Simple default case**: Most common operations should require minimal typing
2. **Page-centric approach**: Always start from a root page (space-level exports are future enhancement)
3. **Explicit options**: Clear flags for controlling behavior
4. **Consistent authentication**: Same auth options work across all commands
5. **Helpful debugging**: Built-in commands for troubleshooting and inspection

## Command Structure

The CLI has a simple structure with the main download functionality at the root level:

- **Root command**: Download a page (and optionally children)
- **`auth`**: Authentication testing and inspection
- **`version`**: Version and build information
- **`completions`**: Generate shell completion scripts

## Root Command - Download Page

The primary function downloads a Confluence page and optionally its children.

### Syntax

```bash
confluence-dl [OPTIONS] <PAGE_URL_OR_ID>
```

### Arguments

- `<PAGE_URL_OR_ID>`: Full page URL or numeric page ID. Required unless a subcommand is used.

### Behavior

- Downloads the specified page to Markdown
- Optionally includes child pages recursively
- Downloads and references embedded images
- Converts Confluence links to Markdown format

### Examples

```bash
# Download by full URL
confluence-dl https://example.atlassian.net/wiki/spaces/MYSPACE/pages/123456/My+Page

# Download by page ID (requires --url flag for base URL)
confluence-dl 123456 --url https://example.atlassian.net

# Download with children (recursive)
confluence-dl 123456 --url https://example.atlassian.net --children

# Download with children using the -r shorthand
confluence-dl 123456 --url https://example.atlassian.net -r

# Limit recursion depth (requires --children or -r)
confluence-dl https://example.atlassian.net/wiki/pages/123456 --children --max-depth 2

# Include attachments
confluence-dl https://example.atlassian.net/wiki/pages/123456 --attachments
```

### Page-Specific Options

```
  -r, --children                Download child pages recursively (alias: --recursive)

      --max-depth <N>           Maximum depth when downloading children
                                [requires: --children]

      --attachments             Download page attachments
```

## Debugging & Introspection Commands

### `auth` - Authentication Testing

Test and display authentication configuration.

```bash
confluence-dl auth [SUBCOMMAND]
```

**Subcommands:**

#### `auth test`

Test authentication credentials against Confluence API.

```bash
confluence-dl auth test [OPTIONS]
```

**Examples:**

```bash
# Test with .netrc credentials
confluence-dl auth test

# Test with explicit credentials
confluence-dl auth test --url https://example.atlassian.net --user user@example.com --token mytoken
```

**Output:**

```
✓ Successfully authenticated to https://example.atlassian.net
  User: user@example.com
  Account ID: 557058:abc123...
  Display Name: John Doe
```

#### `auth show`

Display current authentication configuration (without sensitive data).

```bash
confluence-dl auth show
```

**Output:**

```
Authentication Configuration:
  Source: .netrc file
  URL: https://example.atlassian.net
  User: user@example.com
  Token: ******** (8 chars, from .netrc)
```

### `version` - Version Information

Display detailed version information including build metadata.

```bash
confluence-dl version [OPTIONS]
```

**Options:**

- `--json`: Output in JSON format
- `--short`: Show only version number

**Examples:**

```bash
# Full version info
confluence-dl version

# JSON format for scripts
confluence-dl version --json

# Just the version number
confluence-dl version --short
```

**Output (full):**

```
confluence-dl 0.1.0
Git commit: a1b2c3d
Built: 2025-10-07 21:30:42 UTC
Target: x86_64-unknown-linux-gnu
Rust version: 1.90.0
```

**Output (JSON):**

```json
{
  "version": "0.1.0",
  "git_commit": "a1b2c3d",
  "build_timestamp": "2025-10-07T21:30:42Z",
  "target": "x86_64-unknown-linux-gnu",
  "rust_version": "1.90.0"
}
```

### `completions` - Shell Completion Generation

Generate shell completion scripts for various shells.

```bash
confluence-dl completions <SHELL>
```

**Arguments:**

- `<SHELL>`: Target shell (bash, zsh, fish, powershell, elvish)

**Examples:**

```bash
# Bash (user-level, no sudo required - recommended)
mkdir -p ~/.local/share/bash-completion/completions
confluence-dl completions bash > ~/.local/share/bash-completion/completions/confluence-dl

# Bash (system-level, requires sudo)
confluence-dl completions bash | sudo tee /usr/share/bash-completion/completions/confluence-dl

# Zsh
mkdir -p ~/.zsh/completions
confluence-dl completions zsh > ~/.zsh/completions/_confluence-dl

# Fish
mkdir -p ~/.config/fish/completions
confluence-dl completions fish > ~/.config/fish/completions/confluence-dl.fish
```

## Global Options

These options are available for all commands:

### Authentication Options

```
  --url <URL>              Confluence base URL
                           [env: CONFLUENCE_URL]

  --user <EMAIL>           Confluence user email
                           [env: CONFLUENCE_USER]

  --token <TOKEN>          Confluence API token
                           [env: CONFLUENCE_TOKEN]
```

**Priority Order:**

1. CLI flags (highest priority)
2. Environment variables
3. `.netrc` file (lowest priority)

### Output Options

```
  -o, --output <DIR>       Output directory
                           [default: ./confluence-export]

      --overwrite          Overwrite existing files
                           [default: skip existing]

      --save-raw           Save raw Confluence storage format alongside Markdown

      --compact-tables     Render Markdown tables without padding columns for alignment
```

_Note: The CLI currently exports Markdown only. Additional formats will be reconsidered once a concrete data model exists._

### Behavior Options

```
      --dry-run            Show what would be downloaded without actually downloading

  -v, --verbose            Increase verbosity (-v info, -vv debug, -vvv trace)

  -q, --quiet              Suppress all output except errors
                           [conflicts with: --verbose]

      --color <WHEN>       Colorize output
                           [possible: auto, always, never]
                           [default: auto]
```

### Image & Link Options

```
      --download-images[=<BOOL>]
                           Download embedded images (`--download-images=false` disables)
                           [default: true]

      --images-dir <DIR>   Directory for images (relative to output)
                           [default: images]

      --preserve-anchors   Keep Confluence anchor IDs
                           [default: false]
```

### Performance Options

```
      --parallel <N>       Number of parallel downloads (-1 = available cores)
                           [default: 4]

      --rate-limit <N>     Max requests per second
                           [default: 10]

      --timeout <SECONDS>  Request timeout in seconds
                           [default: 30]

_Validation:_ `--parallel` must be `-1` (auto) or at least `1`, and `--rate-limit` must be at least `1` request/second.
```

## Help System

### Short Help

```bash
confluence-dl --help
confluence-dl auth --help
```

### Long Help

```bash
confluence-dl --help-all       # Show all options and subcommands
```

## Usage Examples

### Common Workflows

#### Quick Single Page Export

```bash
confluence-dl https://example.atlassian.net/wiki/spaces/DOCS/pages/123456/Getting+Started
```

#### Download Page Tree (with children)

```bash
confluence-dl https://example.atlassian.net/wiki/pages/123456 --children --max-depth 3
```

#### Full Documentation Backup

```bash
confluence-dl 123456 \
  --url https://example.atlassian.net \
  --children \
  --attachments \
  --download-images \
  -o ./backup
```

#### Export with Explicit Credentials

```bash
confluence-dl 123456 \
  --url https://example.atlassian.net \
  --user user@example.com \
  --token mytoken \
  --children \
  -o ./export
```

#### Test Authentication Before Export

```bash
# First test
confluence-dl auth test

# Then export
confluence-dl https://example.atlassian.net/wiki/pages/123456
```

#### Dry Run to Preview

```bash
confluence-dl 123456 --url https://example.atlassian.net --children --dry-run -v
```

## Error Handling

### Exit Codes

- `0`: Success
- `1`: General error
- `2`: Authentication error
- `3`: Network error
- `4`: Invalid arguments
- `5`: Permission error

### Error Messages

All errors should be clear and actionable:

```
Error: Failed to authenticate to Confluence
  URL: https://example.atlassian.net
  User: user@example.com

Suggestion:
  1. Check that your API token is valid (create one at https://id.atlassian.com/manage-profile/security/api-tokens)
  2. Verify credentials with: confluence-dl auth test
  3. Update your .netrc file or use --token flag
```

## Implementation Notes

### Page ID vs URL Handling

- URLs are accepted with or without a scheme; values without `http(s)://` are normalized to `https://`.
- Numeric IDs resolve to the configured `--url` base host.
- Validation ensures ambiguous inputs (non-numeric strings without a scheme) produce helpful errors.

### Input Validation

- Either `<PAGE_URL_OR_ID>` or a subcommand must be provided.
- Numeric page IDs require `--url` to supply the base Confluence host.
- `--max-depth` can only be used together with `--children`/`-r`.
- URL inputs are normalized to include `https://` if no scheme is provided.
- `--parallel` values below `-1` or equal to `0` are rejected.
- `--rate-limit` must be at least `1` request per second.

### Using clap_complete

- Generate completions at build time or runtime
- Support all major shells
- Include dynamic completions for flags and options

### Progress Indication

- Use progress bars for downloads (e.g., `indicatif` crate)
- Show current page/total when downloading children
- ETA calculations for large page trees

### Logging

- Use `--verbose` for detailed API calls
- `-vv` for debug-level output
- `-vvv` for trace-level output (includes full request/response)

## Future Enhancements

Potential features for later versions:

1. **`space` subcommand**: Download entire spaces

   ```bash
   confluence-dl space MYSPACE -o ./backup
   ```

2. **Space browsing**: List all pages in a space
3. **Label filtering**: Filter by page labels
4. **Archived page handling**: Options for archived content
5. **Watch mode**: Continuous sync for a page tree
6. **Diff mode**: Show what changed since last export
7. **Search**: Search across downloaded content
8. **Config file**: Support `.confluence-dl.toml` for persistent settings
9. **Templates**: Custom markdown templates for different page types
10. **Hooks**: Pre/post-processing scripts
11. **Batch processing**: Read page URLs from stdin
