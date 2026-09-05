# Spec: CLI Markdown Index Integration

Feature ID: b1-cli-index-search
Parent: C — CLI and vault operations
Status: implementation slice

## Purpose

Expose the safe Markdown projection through explicit CLI commands without introducing a persistent index or TUI dependency.

## In scope

- `mg-vault index rebuild` performs a fresh source scan and reports its snapshot.
- `mg-vault index status` performs the same explicit rebuild-backed inspection; it does not claim to read persistent state.
- `mg-vault search QUERY` performs a rebuild-backed deterministic search.
- Selected-vault and `--vault NAME` resolution.
- Stable human and versioned JSON output with status, generation, note count, degraded diagnostics, query, result paths, titles, fingerprints, freshness, persistence, and source authority.
- Structured no-vault errors.

## Explicitly out of scope

- Persistent SQLite index state or cross-process generations.
- Watcher/service/IPC lifecycle.
- TUI integration.
- Full structured retrieval, FTS, fuzzy, regex, pagination, or cursor contracts.

## Acceptance criteria

1. Every command clearly identifies its result as a rebuild snapshot derived from authoritative vault files.
2. `index status` does not imply persistent state; it reports `persistence: "none"` and `freshness: "rebuild_snapshot"`.
3. Human degraded output places each diagnostic on a separate line.
4. JSON output is deterministic and includes the authority/freshness fields.
5. Selected-vault, named-vault, degraded, deterministic search, and no-vault behavior are tested.
6. Formatting, workspace tests, strict Clippy, and diff checks pass.

## Current implementation evidence

- `crates/mg-vault-cli/src/main.rs`
- `crates/mg-vault-cli/tests/cli.rs`

## Next gate

A later persistent-index slice must implement SQLite/service generations and the broader retrieval contract before this integration can be upgraded to full index-service status/search.
