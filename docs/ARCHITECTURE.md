# Architecture

The repository begins with two intentionally narrow crates.

`mg-vault-core` owns deterministic filesystem authority. `XdgPaths` derives application locations, `VaultRegistry` persists named canonical vault roots, `Vault` validates every relative user path at use time, and note operations perform durable writes and optimistic-concurrency checks. Vault-local trash is an explicit privileged operation rather than an escape hatch through generic note paths.

`mg-vault-cli` translates command-line requests into core calls and formats either concise human output or a stable JSON envelope (`version: 1`). It owns no filesystem policy.

Future parser, index, app, editor, TUI, and plugin crates are deferred until their own accepted slices. In particular, no database can become source authority.
