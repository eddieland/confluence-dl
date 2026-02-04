# Library Refactoring Plan

This document outlines a plan for refactoring `confluence-dl` to provide better library support, enabling:
1. Other Rust crates to use the core functionality
2. Future FFI bindings (Python, Node.js, etc.)

## Current State Assessment

### Strengths

| Aspect | Status | Notes |
|--------|--------|-------|
| Dual targets | ✅ Already configured | `lib.rs` + `main.rs` both exist |
| Trait-based API | ✅ Excellent | `ConfluenceApi` trait enables mocking and abstraction |
| No global state | ✅ Clean | No `lazy_static`, `once_cell`, or thread-locals |
| Module separation | ✅ Mostly clean | `confluence/`, `markdown/`, `asciidoc/` have zero CLI coupling |
| Error handling | ✅ Library-friendly | Uses `anyhow::Result<T>` throughout |

### Current Coupling Issues

| Issue | Location | Impact |
|-------|----------|--------|
| `ColorScheme` threading | `commands/*`, `cli.rs` | Presentation mixed with logic |
| `process::exit()` calls | `commands/*` | Library code must never call this |
| Credential orchestration | `commands/auth.rs` | CLI flags/env vars mixed with netrc loading |
| File I/O + output | `commands/page.rs` | Download logic coupled with disk writes |
| Direct `println!`/`eprintln!` | ~144 instances | Scattered throughout commands |

## Module Classification

### Fully Library-Ready (No Changes Needed)

These modules have **zero CLI dependencies** and can be exposed as-is:

```
confluence/
├── api.rs        # ConfluenceApi trait
├── client.rs     # HTTP client implementation
├── models.rs     # Page, Attachment, UserInfo DTOs
├── tree.rs       # PageTree, get_page_tree()
└── url.rs        # parse_confluence_url()

markdown/
├── mod.rs        # storage_to_markdown_with_options()
├── elements.rs   # HTML→Markdown conversion
├── tables.rs     # Table rendering
├── emoji.rs      # Confluence emoji→Unicode
├── html_entities.rs
├── utils.rs
└── macros/       # Confluence macro handlers

asciidoc/
├── mod.rs        # storage_to_asciidoc_with_options()
├── elements.rs   # HTML→AsciiDoc conversion
└── utils.rs

format.rs         # OutputFormat enum
```

### Needs Minor Refactoring

```
credentials/
├── provider.rs   # CredentialsProvider trait (✅ ready)
├── types.rs      # Credential, CredentialError (✅ ready)
└── netrc.rs      # NetrcProvider (✅ ready)
                  # Missing: credential resolution orchestration

images.rs         # 95% ready - consider splitting async download from extraction
attachments.rs    # 95% ready - same consideration
```

### CLI-Only (Stay in Binary)

```
cli.rs            # Clap parsing, tracing init, command dispatch
color.rs          # ColorScheme, ANSI terminal formatting
commands/         # All command handlers (auth, ls, page, version)
main.rs           # Entry point
```

---

## Refactoring Phases

### Phase 1: Clean Public API Surface

**Goal:** Define a clear, documented public API in `lib.rs`

**Current `lib.rs`:**
```rust
pub mod asciidoc;
pub mod attachments;
pub mod cli;           // ← Should NOT be public for library users
pub mod color;         // ← Should NOT be public for library users
pub mod commands;      // ← Should NOT be public for library users
pub mod confluence;
pub mod credentials;
pub mod format;
pub mod images;
pub mod markdown;
```

**Proposed `lib.rs`:**
```rust
//! # confluence-dl
//!
//! A library for interacting with Confluence and converting content to Markdown/AsciiDoc.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use confluence_dl::{ConfluenceClient, MarkdownOptions, storage_to_markdown};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = ConfluenceClient::new(
//!         "https://wiki.example.com",
//!         "user@example.com",
//!         "api-token",
//!     )?;
//!
//!     let page = client.get_page("12345").await?;
//!     let markdown = storage_to_markdown(&page.body.storage.value)?;
//!     println!("{}", markdown);
//!     Ok(())
//! }
//! ```

// ============================================================
// PUBLIC LIBRARY MODULES
// ============================================================

/// Confluence API client and data models
pub mod confluence;

/// Confluence storage format → Markdown conversion
pub mod markdown;

/// Confluence storage format → AsciiDoc conversion
pub mod asciidoc;

/// Credential providers for authentication
pub mod credentials;

/// Image extraction and download utilities
pub mod images;

/// Attachment handling utilities
pub mod attachments;

/// Output format definitions
pub mod format;

// ============================================================
// CONVENIENCE RE-EXPORTS
// ============================================================

// Core API
pub use confluence::{
    ConfluenceApi, ConfluenceClient,
    Page, PageTree, Attachment, UserInfo,
    get_page_tree, parse_confluence_url, UrlInfo,
};

// Markdown conversion
pub use markdown::{MarkdownOptions, storage_to_markdown_with_options};

// AsciiDoc conversion
pub use asciidoc::{AsciiDocOptions, storage_to_asciidoc_with_options};

// Credentials
pub use credentials::{Credential, CredentialError, CredentialsProvider, NetrcProvider};

// Format
pub use format::OutputFormat;

// Convenience aliases
pub fn storage_to_markdown(content: &str) -> anyhow::Result<String> {
    storage_to_markdown_with_options(content, &MarkdownOptions::default())
}

pub fn storage_to_asciidoc(content: &str) -> anyhow::Result<String> {
    storage_to_asciidoc_with_options(content, &AsciiDocOptions::default())
}

// ============================================================
// CLI-ONLY MODULES (not part of public API)
// ============================================================

#[doc(hidden)]
pub mod cli;

#[doc(hidden)]
pub mod color;

#[doc(hidden)]
pub mod commands;
```

**Tasks:**
- [ ] Update `lib.rs` with clean public API
- [ ] Add module-level documentation with examples
- [ ] Add `#[doc(hidden)]` to CLI-only modules
- [ ] Add convenience functions (`storage_to_markdown`, `storage_to_asciidoc`)

---

### Phase 2: Extract Credential Resolution

**Problem:** `commands/auth.rs` contains `load_credentials()` which orchestrates:
- CLI flag credentials (`--user`, `--token`)
- Environment variables (`CONFLUENCE_USER`, `CONFLUENCE_TOKEN`)
- `.netrc` file lookup

This logic is useful for library users but is currently CLI-specific.

**Solution:** Create a `CredentialResolver` in the library:

```rust
// src/credentials/resolver.rs (NEW FILE)

use crate::credentials::{Credential, CredentialError, CredentialsProvider, NetrcProvider};

/// Sources for credential resolution, in priority order
pub struct CredentialResolver {
    explicit: Option<Credential>,
    env_prefix: Option<String>,
    use_netrc: bool,
}

impl CredentialResolver {
    pub fn new() -> Self {
        Self {
            explicit: None,
            env_prefix: None,
            use_netrc: true,
        }
    }

    /// Set explicit credentials (highest priority)
    pub fn with_credentials(mut self, username: String, password: String) -> Self {
        self.explicit = Some(Credential { username, password });
        self
    }

    /// Enable environment variable lookup with given prefix
    /// E.g., prefix "CONFLUENCE" checks CONFLUENCE_USER and CONFLUENCE_TOKEN
    pub fn with_env_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.env_prefix = Some(prefix.into());
        self
    }

    /// Enable/disable .netrc lookup (enabled by default)
    pub fn with_netrc(mut self, enabled: bool) -> Self {
        self.use_netrc = enabled;
        self
    }

    /// Resolve credentials for the given host
    pub fn resolve(&self, host: &str) -> Result<Option<Credential>, CredentialError> {
        // 1. Explicit credentials
        if let Some(ref cred) = self.explicit {
            return Ok(Some(cred.clone()));
        }

        // 2. Environment variables
        if let Some(ref prefix) = self.env_prefix {
            let user_var = format!("{}_USER", prefix);
            let token_var = format!("{}_TOKEN", prefix);

            if let (Ok(user), Ok(token)) = (std::env::var(&user_var), std::env::var(&token_var)) {
                if !user.is_empty() && !token.is_empty() {
                    return Ok(Some(Credential {
                        username: user,
                        password: token,
                    }));
                }
            }
        }

        // 3. .netrc file
        if self.use_netrc {
            let provider = NetrcProvider::new()?;
            return provider.get_credentials(host);
        }

        Ok(None)
    }
}
```

**Tasks:**
- [ ] Create `src/credentials/resolver.rs`
- [ ] Add `CredentialResolver` to public API
- [ ] Refactor `commands/auth.rs` to use `CredentialResolver`
- [ ] Update CLI to construct resolver from flags/options

---

### Phase 3: Separate Download Logic from I/O

**Problem:** `commands/page.rs` (922 lines) tightly couples:
- API calls (fetching pages, attachments)
- Content conversion (Markdown/AsciiDoc)
- File system writes
- Progress output (ColorScheme)

**Solution:** Create a `PageDownloader` struct in the library that handles the pure logic:

```rust
// src/download.rs (NEW FILE)

use crate::confluence::{ConfluenceApi, Page, PageTree};
use crate::format::OutputFormat;
use crate::markdown::MarkdownOptions;
use crate::asciidoc::AsciiDocOptions;

/// Result of processing a single page
pub struct ProcessedPage {
    pub page: Page,
    pub converted_content: String,
    pub images: Vec<ImageToDownload>,
    pub attachments: Vec<AttachmentToDownload>,
}

pub struct ImageToDownload {
    pub url: String,
    pub filename: String,
}

pub struct AttachmentToDownload {
    pub url: String,
    pub filename: String,
}

/// Options for page processing
pub struct ProcessOptions {
    pub format: OutputFormat,
    pub markdown_options: MarkdownOptions,
    pub asciidoc_options: AsciiDocOptions,
    pub include_images: bool,
    pub include_attachments: bool,
    pub recursive: bool,
    pub max_depth: Option<usize>,
}

/// Process pages without performing I/O
/// Returns structured data that the caller can write to disk
pub async fn process_page(
    client: &dyn ConfluenceApi,
    page_id: &str,
    options: &ProcessOptions,
) -> anyhow::Result<ProcessedPage> {
    // Fetch page
    // Convert content
    // Extract image/attachment references
    // Return structured result
}

/// Process a page tree recursively
pub async fn process_page_tree(
    client: &dyn ConfluenceApi,
    page_id: &str,
    options: &ProcessOptions,
) -> anyhow::Result<Vec<ProcessedPage>> {
    // ...
}
```

**Tasks:**
- [ ] Create `src/download.rs` with pure processing logic
- [ ] Define `ProcessedPage`, `ProcessOptions` structs
- [ ] Implement `process_page()` and `process_page_tree()`
- [ ] Refactor `commands/page.rs` to use library functions + handle I/O
- [ ] Add to public API

---

### Phase 4: Cargo Features for Optional Dependencies

**Goal:** Allow library users to include only what they need, reducing compile times and binary size.

**Proposed features in `Cargo.toml`:**
```toml
[features]
default = ["markdown", "asciidoc", "netrc"]

# Content conversion formats
markdown = []
asciidoc = []

# Credential providers
netrc = []

# CLI-only dependencies (not needed for library use)
cli = ["clap", "owo-colors", "tracing-subscriber"]

# FFI support (future)
ffi = ["cbindgen"]  # or pyo3, napi, etc.
```

**Conditional compilation:**
```rust
// In lib.rs
#[cfg(feature = "markdown")]
pub mod markdown;

#[cfg(feature = "asciidoc")]
pub mod asciidoc;

#[cfg(feature = "netrc")]
pub use credentials::NetrcProvider;
```

**Tasks:**
- [ ] Add feature flags to `Cargo.toml`
- [ ] Add `#[cfg(feature = "...")]` guards to modules
- [ ] Move CLI dependencies behind `cli` feature
- [ ] Update documentation with feature descriptions

---

### Phase 5: Error Types for Library Use

**Current:** Uses `anyhow::Result<T>` everywhere, which is convenient but loses type information.

**For library users:** Consider adding typed errors for better error handling:

```rust
// src/error.rs (NEW FILE)

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfluenceError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Page not found: {page_id}")]
    PageNotFound { page_id: String },

    #[error("Rate limited, retry after {retry_after_secs} seconds")]
    RateLimited { retry_after_secs: u64 },

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("HTTP error: {status} - {message}")]
    HttpError { status: u16, message: String },

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
}

#[derive(Error, Debug)]
pub enum ConversionError {
    #[error("Invalid XML: {0}")]
    InvalidXml(String),

    #[error("Unsupported element: {element}")]
    UnsupportedElement { element: String },
}

/// Unified library error type
#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Confluence(#[from] ConfluenceError),

    #[error(transparent)]
    Conversion(#[from] ConversionError),

    #[error(transparent)]
    Credentials(#[from] crate::credentials::CredentialError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
```

**Tasks:**
- [ ] Create `src/error.rs` with typed errors
- [ ] Gradually migrate from `anyhow::Result` to typed `Result`
- [ ] Keep `anyhow` for internal use, expose typed errors in public API
- [ ] Add `From` implementations for error conversion

---

### Phase 6: FFI Preparation (Future)

**Goal:** Prepare for Python/Node.js bindings without implementing them yet.

**Considerations:**

1. **C-compatible types:** Ensure core structs can be represented in C
   ```rust
   #[repr(C)]
   pub struct PageInfo {
       pub id: *const c_char,
       pub title: *const c_char,
       // ...
   }
   ```

2. **Async to sync bridge:** FFI typically needs synchronous functions
   ```rust
   // Blocking wrapper for FFI
   pub fn get_page_blocking(
       client: &ConfluenceClient,
       page_id: &str,
   ) -> Result<Page> {
       tokio::runtime::Runtime::new()?
           .block_on(client.get_page(page_id))
   }
   ```

3. **Memory management:** Clear ownership rules for FFI
   - Functions that return owned data should have corresponding `free` functions
   - Consider using `Box::into_raw()` / `Box::from_raw()`

4. **Potential FFI crates:**
   - **Python:** `pyo3` + `maturin`
   - **Node.js:** `napi-rs`
   - **C/C++:** `cbindgen` for header generation

**Tasks (future):**
- [ ] Add `ffi` feature flag
- [ ] Create `src/ffi.rs` module
- [ ] Add blocking wrappers for async functions
- [ ] Generate C headers with cbindgen
- [ ] Create separate `confluence-dl-python` / `confluence-dl-node` crates

---

## Implementation Roadmap

### Milestone 1: Clean Library API (Low Risk)
- Phase 1: Public API surface
- Estimated changes: ~50 lines in `lib.rs`
- No breaking changes to CLI

### Milestone 2: Credential Refactoring (Low Risk)
- Phase 2: Extract credential resolution
- Estimated changes: ~150 new lines, ~50 lines modified
- No breaking changes to CLI

### Milestone 3: Download Logic Separation (Medium Risk)
- Phase 3: Separate download logic
- Estimated changes: ~300 new lines, ~200 lines refactored
- Internal refactoring, CLI behavior unchanged

### Milestone 4: Feature Flags (Medium Risk)
- Phase 4: Cargo features
- Requires careful testing of all feature combinations
- May affect CI/CD pipeline

### Milestone 5: Error Types (Medium Risk)
- Phase 5: Typed errors
- Gradual migration, can be done incrementally
- Improves library ergonomics

### Milestone 6: FFI (Future)
- Phase 6: FFI preparation
- Separate milestone, depends on actual need
- Consider creating separate crates for bindings

---

## Testing Strategy

### Library API Tests
```rust
// tests/library_api_tests.rs

#[test]
fn test_public_api_accessible() {
    // Verify all public types are accessible
    use confluence_dl::{
        ConfluenceClient, ConfluenceApi,
        Page, PageTree, Attachment,
        MarkdownOptions, storage_to_markdown,
        AsciiDocOptions, storage_to_asciidoc,
        CredentialResolver, NetrcProvider,
        OutputFormat,
    };
}

#[test]
fn test_markdown_conversion_standalone() {
    let storage = r#"<p>Hello <strong>world</strong></p>"#;
    let md = confluence_dl::storage_to_markdown(storage).unwrap();
    assert_eq!(md.trim(), "Hello **world**");
}
```

### Feature Flag Tests
```bash
# Test each feature combination
cargo nextest run --no-default-features
cargo nextest run --no-default-features --features markdown
cargo nextest run --no-default-features --features asciidoc
cargo nextest run --all-features
```

### Documentation Tests
```bash
cargo test --doc
```

---

## Documentation Plan

### Crate-level docs (`lib.rs`)
- Overview of library capabilities
- Quick start example
- Feature flag documentation
- Links to module docs

### Module-level docs
- Each public module should have a doc comment explaining its purpose
- Include usage examples

### README updates
- Add "Library Usage" section
- Cargo.toml dependency example
- Feature flag explanations

### Examples directory
```
examples/
├── basic_download.rs      # Simple page download
├── custom_conversion.rs   # Custom MarkdownOptions
├── recursive_export.rs    # Export entire space
└── credential_providers.rs # Custom credential provider
```

---

## Summary

The codebase is **well-positioned** for library extraction:

1. **Already has `lib.rs`** with module exports
2. **Trait-based `ConfluenceApi`** enables clean abstraction
3. **Zero global state** makes it safe for concurrent use
4. **Core modules** (`confluence/`, `markdown/`, `asciidoc/`) have **no CLI coupling**

Main work involves:
1. Defining a clean public API surface
2. Extracting credential resolution logic
3. Separating pure processing from I/O in download logic
4. Adding feature flags for modularity
5. Improving error types for library users

The refactoring can be done **incrementally** without breaking the existing CLI.
