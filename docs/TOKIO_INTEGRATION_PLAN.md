# Tokio Integration Plan

## Objective

Adopt Tokio as the asynchronous runtime across `confluence-dl` so the CLI can fetch large Confluence datasets efficiently, unlock
concurrency for future features (bulk space exports, synchronization jobs), and eliminate the current blocking HTTP stack.

## Current Pain Points

- **Blocking HTTP client**: `reqwest::blocking::Client` forces synchronous calls, making long-running exports slow and preventing
  concurrent page or attachment downloads.
- **Single-threaded page traversal**: Recursive traversal of page trees currently waits for each network call to complete
  sequentially, limiting throughput and increasing perceived latency.
- **Tight coupling in API trait**: The `ConfluenceApi` trait is synchronous, which blocks us from experimenting with async job
  scheduling, rate limiting, or streaming results to downstream processors.

## Target Architecture

1. **Async runtime at the top level**
   - Use `#[tokio::main]` in `src/main.rs` and convert command handlers to `async fn`s so network I/O happens within Tokio's runtime.
   - Move any CPU-heavy Markdown conversion into `tokio::task::spawn_blocking` to keep the async executor responsive.

2. **Async Confluence client**
   - Replace `reqwest::blocking::Client` with the async `reqwest::Client` and expose async methods for each API call.
   - Use structured response types unchanged, but return `async fn` results to allow concurrent requests and graceful cancellation.

3. **Concurrency controls**
   - Introduce a `tokio::sync::Semaphore` (configurable via CLI) that limits the number of in-flight API requests to respect
     Confluence rate limits while still parallelizing bulk fetches.
   - Use `FuturesUnordered` to pipeline page tree traversal so children can be fetched concurrently with attachment downloads.

4. **Streaming downloads & large payload handling**
   - Switch attachment downloads to `reqwest::Response::bytes_stream()` and write to disk using `tokio::fs` to avoid buffering large
     files entirely in memory.
   - Provide progress instrumentation hooks so future features can surface download metrics while streams are consumed.

5. **Async-friendly trait surface**
   - Update `ConfluenceApi` to use the `async_trait` crate (already compatible with nightly) so fake clients in tests can stay
     simple.
   - Ensure shared structs remain `Send + Sync` by using `Arc` where the client is cloned across tasks.

6. **Observability and error handling**
   - Standardize request contexts with `tracing` (Tokio-native) to capture spans for each API call.
   - Propagate cancellation and timeout errors with `anyhow::Context` so multi-request workflows surface actionable failures.

## Implementation Steps

1. **Runtime bootstrap**
   - Add necessary Tokio features (e.g., `rt-multi-thread`, `macros`, `fs`, `signal`, `sync`) in `Cargo.toml` and remove the
     `reqwest::blocking` feature flag.
   - Convert `main()` to an async entrypoint and adjust CLI command dispatch to await async handlers.

2. **Refactor Confluence client**
   - Introduce a new async `ConfluenceClient` builder that owns a `reqwest::Client` configured with connection pooling and timeout
     defaults suited for large exports.
   - Update each API method (`get_page`, `get_child_pages`, `get_attachments`, `download_attachment`, `test_auth`) to async variants
     returning `Result<T>` and handling streaming bodies.
   - Ensure authentication headers and base URL normalization remain synchronous helpers to keep behavior unchanged.

3. **Parallel page tree traversal**
   - Rewrite `get_page_tree` and associated helpers to issue child page fetches concurrently using a semaphore-guarded executor.
   - Process child page tasks with `FuturesUnordered`, aggregating results while respecting depth limits and preserving ordering
     semantics in the final tree.

4. **Async image & attachment pipeline**
   - Migrate the attachment download workflow in `src/images.rs` to use async file operations (`tokio::fs::File`) and chunked writes
     from `StreamExt`.
   - Replace blocking filesystem utilities with async equivalents where practical, falling back to `spawn_blocking` for operations
     that require std-only crates.

5. **Testing & tooling updates**
   - Update the fake Confluence client in `tests/common` to implement the async trait and provide deterministic async responses.
   - Adjust integration tests to run inside Tokio's runtime (e.g., using `#[tokio::test(flavor = "multi_thread")]`).
   - Add targeted concurrency tests to ensure semaphore limits and cancellation behave as expected under load.

6. **Cleanup & documentation**
   - Remove unused synchronous helpers and confirm no modules import `reqwest::blocking`.
   - Document the async architecture in `docs/` (including configuration knobs for concurrency) so future contributors understand the
     runtime model.

## Risks & Mitigations

- **Increased complexity**: Async introduces new mental models. Mitigate with clear abstractions (`ConfluenceApi` trait) and
  documentation for each async boundary.
- **Test flakiness**: Async tests can be timing-sensitive. Use deterministic fake clients and Tokio's `time::pause` utilities to
  control scheduling in unit tests.
- **Backwards compatibility**: Removing blocking implementations may affect downstream users who depended on synchronous APIs. Since
  this is an internal CLI, we accept the breaking change and communicate it via release notes.

## Recommendation

Proceed with the plan now. The codebase already depends on Tokio and reqwest, so adopting the async runtime will unlock immediate
performance gains for large exports without incurring migration debt later. No prerequisites are blocking the switch.
