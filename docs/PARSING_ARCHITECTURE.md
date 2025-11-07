# Parsing Architecture

This document explains how `confluence-dl` turns Confluence storage format into Markdown. The codebase prioritizes predictable, best effort output rather than exhaustive coverage of every Confluence feature. When a structure is not formally supported we still aim to produce stable Markdown that preserves the core reading experience.

## High-Level Flow

1. The CLI resolves credentials, fetches a page body via the `ConfluenceApi`, and stores the storage-format payload.
2. `html_entities::preprocess_html_entities` replaces unsupported named entities with numeric references so the XML parser can understand the input.
3. `utils::wrap_with_namespaces` injects the Confluence namespace declarations that many responses omit, producing a well-formed document for `roxmltree`.
4. `roxmltree::Document::parse` builds the DOM once per page body. Parse failures capture tracing diagnostics before bubbling an `anyhow::Error`.
5. `markdown::convert_node_to_markdown` walks the DOM depth-first. Each tag is handled by focused helpers in `elements`, `tables`, `macros`, and `emoji`.
6. The resulting Markdown string is cleaned via `utils::clean_markdown`, which dedents blank lines, collapses stray whitespace, and normalizes list spacing.

This sequence keeps parsing pure and side-effect free. The download pipeline writes Markdown only after this conversion completes.

## Module Responsibilities

- `src/markdown/html_entities.rs` performs deterministic replacements so HTML entities survive XML parsing, covering both Confluence-specific names and standard entities.
- `src/markdown/utils.rs` wraps XML, finds rich-text bodies, and exposes helpers for trimming whitespace and building link targets.
- `src/markdown/elements.rs` handles the common HTML subset such as headings, paragraphs, lists, inline text styles, and anchors. Each function converts one node type, which keeps the recursion small and composable.
- `src/markdown/tables.rs` maps `<table>` nodes to GitHub-flavored Markdown, including column width detection and optional compact rendering controlled by `MarkdownOptions::compact_tables`.
- `src/markdown/macros/mod.rs` focuses on structured macros such as panels, notes, statuses, and excerpts. Each macro implementation produces either fenced blocks, blockquotes, or inline adornments depending on the original intent.
- `src/markdown/emoji.rs` converts Confluence colon codes into Unicode emoji while leaving unknown codes untouched so readers can still infer intent.

`MarkdownOptions` (see `src/markdown/mod.rs`) threads through every helper. New flags, such as anchor preservation, only require extending this struct and the leaf functions that care about the behavior.

## Core Parsing Principles

- Canonicalize before interpreting. We normalize entities and namespaces prior to traversing nodes so every downstream helper receives a consistent DOM regardless of how Confluence serialized the original payload.
- Traverse like an AST. The recursion never inspects string slices directly; it always reads tag names, attributes, and text nodes from `roxmltree`, which keeps conversions predictable and side-effect free.
- Prefer text retention over styling fidelity. When a construct is unknown we keep the textual content and drop the chrome instead of inventing Markdown that might mislead the reader.
- Fail loudly on structural issues, fall back gently on content gaps. Invalid XML returns an error so users can fix the source page, while unsupported macros are rendered as readable blocks that clearly call out their origin.
- Keep knobs centralized. Every conditional behavior originates from `MarkdownOptions`, which allows future CLI toggles without scattering `cfg`-style checks throughout the tree walker.

## Consistency Before Coverage

Confluence exposes hundreds of macros and HTML features. Supporting every combination would require duplicating Confluence renderer logic, which conflicts with the goal of providing a reliable offline copy. Instead we focus on:

- Deterministic normalization so running the exporter twice on the same page yields identical Markdown.
- Graceful degradation that keeps readable text even when advanced layout hints are dropped.
- Hooks for teams to layer custom transforms on top of the cleaned Markdown if they need tighter fidelity.

When the parser meets a structure it cannot understand, it either falls back to the raw text or to a neutral blockquote with a short prefix, making the missing context obvious without breaking the document.

## Limitations

- Complex layout macros such as multi-column sections flatten into sequential blocks, which means carefully arranged dashboards lose their grid structure.
- Third-party or marketplace macros are passed through as plain text. The exporter cannot execute their custom renderers, so diagrams or charts provided by add-ons need manual follow-up.
- Dynamically generated content (Jira issue lists, recently updated lists) is captured as-is at export time and will not auto-refresh.
- CSS-based styling, inline colors, and font choices are dropped because Markdown deliberately limits formatting.
- HTML that is already invalid when received from Confluence may fail to parse even after preprocessing. The converter surfaces the error instead of guessing.
