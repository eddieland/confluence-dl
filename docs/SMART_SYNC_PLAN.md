# Smart Syncing Plan

## Problem Statement

The current export flow always downloads every requested Confluence page, its attachments, and images regardless of whether we
already have an identical copy on disk. As exports get larger (tens of thousands of pages and binary assets) this wastes API
quota, slows down incremental backups, and causes noisy diffs in downstream tooling. We need a "smart sync" mode that only
fetches content that actually changed since the last successful run.

## Guiding Principles

1. **API efficient** – minimize the number of REST calls and payload size while staying within Atlassian rate limits.
2. **Correct** – never skip a resource whose content or metadata changed. It is acceptable to re-download when unsure.
3. **Deterministic outputs** – preserve existing file layout and formatting semantics so downstream consumers do not need to
   change anything when smart sync is enabled.
4. **Recoverable** – corrupted or partial manifests should be detected, triggering a safe fallback full sync.
5. **Observable** – surface metrics in the CLI output (pages scanned, pages changed, downloads saved) to make the benefit clear
   to users.

## High-level Strategy

Smart syncing requires maintaining a persistent manifest that records the state of every exported resource. Each subsequent run
uses lightweight metadata calls to detect changes before performing expensive downloads.

1. **Manifest Store**
   - File: `.confluence-dl/sync-manifest.json` relative to the export root (configurable via CLI).
   - Contents per resource:
     - `resource_id`: page ID, attachment ID, or image canonical URL hash.
     - `resource_type`: `page`, `attachment`, `image` (open for future types like `comment`).
     - `version`: Confluence version number for pages/attachments, SHA256 hash for derived markdown.
     - `etag` and/or `last_modified`: if provided by API responses.
     - `output_path`: relative path of the exported artifact.
     - `fetched_at`: UNIX timestamp of last successful download.
   - Atomic updates: write to `sync-manifest.json.tmp`, then rename. Keep backup of previous manifest for recovery.

2. **Change Detection Workflow**
   - **Pages**: issue a metadata-only request (Confluence REST `/content/{id}?expand=version` or GraphQL equivalent). Compare the
     server version number against `manifest.version`. Re-download only when the version increments or the manifest lacks an
     entry.
   - **Attachments**: list attachments with `expand=version`. Use version increment detection similar to pages.
   - **Images referenced within pages**: while rendering markdown we already resolve image download URLs. Persist a stable hash
     of the download URL + `atl_token`. Store the checksum of the on-disk file (SHA256). Re-download if the manifest is missing
     the entry or if a checksum mismatch occurs during verification.
   - **Deletions**: after traversing the Confluence tree, compute the set difference between manifest entries and resources we
     saw this run. For missing resources, either delete local files (if `--prune` flag is set) or mark them stale for reporting.

3. **Execution Phases**
   1. Load manifest (or create empty if missing).
   2. Traverse target pages to build the list of resources to consider (existing behavior).
   3. For each resource, run the change detector to decide one of: `Unchanged`, `Changed`, `Unknown` (download to confirm).
   4. Perform downloads/rendering only for `Changed` and `Unknown` resources.
   5. Verify post-download (e.g., hash the written file) and update manifest entries.
   6. Handle deletions/pruning and write the final manifest atomically.
   7. Emit summary statistics.

## Detailed Components

### Manifest Data Model

```rust
struct Manifest {
  format_version: u8,
  generated_by: String, // "confluence-dl <version>"
  exported_at: DateTime<Utc>,
  entries: HashMap<ResourceKey, ManifestEntry>,
}

enum ResourceKey {
  Page { id: String },
  Attachment { page_id: String, attachment_id: String },
  Image { url_hash: String },
}

struct ManifestEntry {
  version: Option<u64>,
  etag: Option<String>,
  last_modified: Option<DateTime<Utc>>,
  checksum: Option<String>,
  output_path: PathBuf,
  fetched_at: DateTime<Utc>,
}
```

- `format_version` allows future schema migrations. On mismatch, fallback to full sync.
- `checksum` uses SHA256 for local files; for markdown we can hash the rendered content to detect transformations even if the
  server version is identical but we changed formatting logic.
- Use `serde_json` for serialization; rely on `fs_err` to provide better error messages.

### Traversal Changes

- Extend the existing download planner to emit `ResourceDescriptor` records (page, attachment, image) with enough metadata for
  change detection. This is independent from the actual download stage so we can parallelize detection in the future.
- Incorporate user options:
  - `--force` bypasses the manifest and forces re-downloads (existing behavior).
  - `--no-smart-sync` disables the feature for debugging.
  - `--prune` deletes local files that disappeared on the server.

### API Usage

- **Pages**: `GET /wiki/rest/api/content/{id}?expand=version,body.storage,body.view` (already used). For smart sync, first fetch
  only `expand=version`. If unchanged, skip the heavy `body.*` call. If changed, perform the existing full fetch.
- **Attachments**: `GET /wiki/rest/api/content/{id}/child/attachment?expand=version` for metadata-only pass. Download specific
  attachment only on change.
- **Rate limiting**: use concurrency limits (Tokio semaphore) to avoid spikes. Combine metadata calls by batching multiple IDs
  with `POST /wiki/rest/api/content/batch` where supported to reduce HTTP round trips.

### Output Synchronization

- During rendering, compute SHA256 of the generated markdown before writing. Compare with manifest checksum to skip disk write
  when unchanged (avoids touching mtime and creating diffs).
- Keep attachments/images in deterministic directories (current behavior) to avoid path churn.

### Error Handling & Recovery

- If manifest parsing fails, log a warning, move the corrupt file to `.bak`, and perform a full sync.
- If an API call returns 403/404, treat the resource as changed (forces re-download) but bubble up errors for complete
  failures.
- On partial failures during a run, do not write the updated manifest. Instead, leave the previous manifest intact and exit with
  an error code so the user can retry.

### Telemetry & UX

- CLI summary block example:
  ```
  Smart sync summary:
    Pages scanned:      120
    Pages updated:       15
    Attachments updated:  4 (saved 98 requests)
    Images updated:       2 (checksum mismatch)
    Local deletions:      3 (--prune)
  ```
- Provide `--json` output flag to print the same summary as structured JSON for scripting.

## Milestones

1. **Manifest infrastructure** (load/save, schema versioning, CLI flags).
2. **Page metadata detection** (skip markdown rendering when unchanged).
3. **Attachment detection** (skip binary downloads when version matches).
4. **Image checksum verification** (skip downloads when the local cache matches).
5. **Deletion handling & pruning**.
6. **Telemetry & documentation**.

Each milestone should land behind a feature flag until the end-to-end flow is stable.

## Open Questions

- Should we support multiple parallel manifests (e.g., per-space) or always a single file per export root?
- How do we handle users who manually edit exported Markdown? Proposed approach: log a warning when checksum mismatch occurs but
  allow overwriting unless `--preserve-local-edits` is set.
- Can we leverage Confluence's `If-Modified-Since` / `If-None-Match` headers to avoid downloads entirely for attachments? Need to
  confirm Atlassian Cloud support.
- Should we expose manifest inspection commands (e.g., `confluence-dl sync status`)?

Answering these during implementation will refine the UX, but the plan above provides the foundational architecture needed to
ship smart syncing.
