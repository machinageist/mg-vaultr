# Slice Spec: Persistent SQLite Index Generations

**Parent:** `b-index-service-search` (B3, B4, B7 subset)  
**Milestone:** 2  
**Scope:** one disposable per-vault SQLite projection store

## Contract

Source Markdown remains the sole authority. A dedicated index crate may read the existing safe in-memory `MarkdownIndex` projection and persist only derived rows. The database has an explicit schema and parser version. A complete rebuild replaces every projected note in one transaction and publishes its generation only at that transaction's commit point.

A failed source scan or injected pre-publication failure must not expose any candidate rows or advance the published generation. It records degraded freshness and diagnostics while retaining the last complete generation, if one exists. Deleting the database loses no authority: rebuilding from unchanged sources recreates equivalent ordered query results and a deterministic first generation.

Public metadata states schema version, parser version, published generation, freshness (`empty`, `current`, or `degraded`), note count, completion time, and diagnostics. `current` means the store contains the last complete explicit rebuild snapshot; this slice does not claim watcher-backed continuous currency.

The CLI's explicit rebuild/status/search adapters may use this store, but a database/open/rebuild failure and a source-degraded rebuild must preserve the existing direct in-memory search path and label it as non-persistent/degraded. No client treats SQLite content as source content for mutation.

## Acceptance tests

1. Build an index, delete its database, rebuild, and compare normalized ordered results; source bytes are unchanged and both fresh databases publish generation 1.
2. Inject failure after candidate rows are staged but before publication; reopening the store shows the previous generation and rows, with degraded freshness and no candidate rows.
3. Change one Markdown source and delete another, rebuild, and prove old text/deleted paths are absent while replacement text is present.
4. Equal-query matches are ordered by exact vault-relative path independent of insertion/discovery order and remain identical after reopen/rebuild.
5. Malformed UTF-8 yields a path-specific diagnostic, publishes no partial generation, and leaves an earlier complete generation visible but degraded.
6. `PRAGMA user_version` and public metadata equal the crate schema version; parser version is non-empty and explicit.

## Non-goals

- Filesystem watcher, dirty queue, incremental event processing, overflow recovery
- Daemon lifecycle, IPC, protocol negotiation, TUI, or Quickshell
- FTS5, fuzzy/regex/structured query grammar, pagination, backlinks, blocks, properties, tasks, or attachments
- In-place schema migration beyond rejecting an unsupported version
- Writing, repairing, deleting, or otherwise mutating vault source
- Claiming that a rebuild snapshot remains current after unobserved external edits

## Required evidence

- Behavior-first integration tests for every acceptance item
- `cargo fmt --all --check`
- `cargo test --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `git diff --check` and scoped diff/status review
