# mg-vault

`mg-vault` is a local-first knowledge system whose authoritative data is ordinary files in user-owned vault directories.

## Implemented foundation and index slices

The current implementation provides:

- XDG config/data/state/cache path resolution;
- a per-user JSON registry for registering, listing, and selecting vaults;
- canonical vault confinement with traversal, symlink-escape, and protected `.obsidian` / `.mg-vault` mutation rejection;
- collision-safe Markdown creation and reading;
- same-directory atomic replacement with file and directory synchronization;
- SHA-256 source fingerprints for optimistic concurrency;
- vault-local trash and collision-safe restore;
- byte-preserving frontmatter scalar edits and deterministic interop snapshot export;
- a rebuildable Markdown projection with path, title, text, fingerprint, and basic wikilink extraction;
- a disposable per-vault SQLite projection with explicit schema/parser versions and atomic generations;
- `index rebuild`, source-observing `index status`, and deterministic search with direct-file fallback;
- truthful `empty`, `current`, `stale`, and `degraded` freshness, including path-specific diagnostics for source drift or incomplete scans;
- human and versioned JSON CLI output, plus `--no-input`, `--no-color`, and `NO_COLOR` handling.

Markdown remains the sole authority. SQLite rows are disposable: deleting the database and rebuilding from unchanged source produces equivalent ordered results. Status and search perform a fresh confined source observation before treating a persisted generation as current; observed drift never silently publishes candidate rows or advances the generation.

## Milestone 2 still in progress

There is not yet a long-running watcher/service, bounded dirty queue, overflow recovery, reconciliation scheduler, local IPC protocol/client, or daemon lifecycle. The index does not yet project headings, blocks, embeds, tags, properties, tasks, or FTS, and it has no structured/fuzzy/regex query grammar, pagination, cancellation, or scale evidence. The basic wikilink projection is not a deterministic ambiguity-safe link/backlink engine.

There is also no TUI, editor, renderer, plugin host, sync adapter, broad import pipeline, publishing workflow, or AI adapter. AI output has no execution or mutation authority; any future AI integration must remain proposal-only until an explicit user-approved deterministic operation is implemented.

## Quick start

```sh
cargo run -p mg-vault -- vault register notes /path/to/vault
cargo run -p mg-vault -- vault select notes
cargo run -p mg-vault -- note create ideas/first.md --body '# First'
cargo run -p mg-vault -- --json note read ideas/first.md
cargo run -p mg-vault -- index rebuild
cargo run -p mg-vault -- --json index status
cargo run -p mg-vault -- search First
```

All note commands use `--vault NAME` when supplied, otherwise the selected vault. Commands are non-interactive in this slice; `--no-input` explicitly locks that behavior for automation.

## Development

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

## License

MIT. See `LICENSE`.
