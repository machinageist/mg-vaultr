# mg-vault

`mg-vault` is a local-first knowledge system whose authoritative data is ordinary files in user-owned vault directories.

## Implemented foundation

This first slice provides:

- XDG config/data/state/cache path resolution;
- a per-user JSON registry for registering, listing, and selecting vaults;
- canonical vault confinement with traversal, symlink-escape, and protected `.obsidian` / `.mg-vault` mutation rejection;
- collision-safe Markdown creation and reading;
- same-directory atomic replacement with file and directory synchronization;
- SHA-256 source fingerprints for optimistic concurrency;
- vault-local trash and collision-safe restore;
- human and versioned JSON CLI output, plus `--no-input`, `--no-color`, and `NO_COLOR` handling.

## Not implemented

There is no TUI, editor, Markdown parser, SQLite index, watcher/service, link engine, renderer, plugin host, sync adapter, import/export, or AI integration yet. Unknown Markdown is preserved because this slice reads and writes note bytes without parsing them.

## Quick start

```sh
cargo run -p mg-vault -- vault register notes /path/to/vault
cargo run -p mg-vault -- vault select notes
cargo run -p mg-vault -- note create ideas/first.md --body '# First'
cargo run -p mg-vault -- --json note read ideas/first.md
```

All note commands use `--vault NAME` when supplied, otherwise the selected vault. Commands are non-interactive in this slice; `--no-input` explicitly locks that behavior for automation.

## Development

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

No license file is present: the project license choice (MIT versus Apache-2.0) remains unresolved.
