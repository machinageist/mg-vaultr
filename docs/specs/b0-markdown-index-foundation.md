# Spec: Rebuildable Markdown Projection Foundation

Feature ID: b0-markdown-index-foundation
Parent: B — Index service and search
Status: implementation slice

## Purpose

Provide a deterministic, disposable in-memory projection over authoritative Markdown files. This slice is the safe foundation for the later persistent index service; it does not claim to be that service.

## In scope

- Recursive discovery of ordinary `.md` files under one opened vault.
- Exclusion of `.obsidian` and `.mg-vault` internals.
- Vault-rooted no-symlink reads on Linux.
- Explicit degraded diagnostics for root, directory-entry, unreadable, invalid-UTF-8, and changing-source failures.
- Full rebuild replacement semantics: deleted sources disappear and changed sources are reflected.
- Source fingerprint, title fallback, raw text, and basic wikilink extraction.
- Deterministic path ordering and case-insensitive substring search.

## Explicitly out of scope

- SQLite persistence or migrations.
- Watcher lifecycle, overflow recovery, IPC, service authorization, or multi-vault daemon state.
- FTS, fuzzy/regex/structured queries, pagination, cursors, cancellation, and deadlines.
- Headings, blocks, tags, properties, tasks, embeds, attachment extraction, or scale benchmarks.
- CLI search commands and TUI integration.

## Acceptance criteria

1. A rebuild replaces the projection from a fresh source walk.
2. Any enumeration/read/decoding/change failure prevents `Current` and is represented as `Degraded`.
3. Linux symlinked Markdown cannot cause reads outside the vault.
4. A source changed during indexing is not published as a successful note snapshot.
5. Search ordering is deterministic and source files are never rewritten.
6. Empty vaults are `Current` after a successful complete rebuild.
7. `cargo fmt --check`, workspace tests, strict Clippy, and `git diff --check` pass.

## Current implementation evidence

- `crates/mg-vault-core/src/index.rs`
- `crates/mg-vault-core/tests/index.rs`
- `crates/mg-vault-core/src/lib.rs`

## Next gates

The full `b-index-service-search` contract remains open until SQLite projection, watcher/reconciliation, service/IPC boundaries, richer indexing, and scale evidence are implemented.
