# Architecture

The repository currently has three intentionally narrow crates.

`mg-vault-core` owns deterministic filesystem authority. `XdgPaths` derives application locations, `VaultRegistry` persists named canonical vault roots, `Vault` validates every relative user path at use time, and note operations perform durable writes and optimistic-concurrency checks. Vault-local trash is an explicit privileged operation rather than an escape hatch through generic note paths.

`mg-vault-cli` translates command-line requests into core calls and formats either concise human output or a stable JSON envelope (`version: 1`). It owns no filesystem policy.

`mg-vault-index` owns a disposable SQLite projection. Complete rebuilds publish a generation atomically. Confined source observations compare exact path/fingerprint manifests with the published generation and report `stale` or `degraded` without publishing candidate rows. The CLI uses this evidence before persisted search and falls back to a clearly labeled live scan when current indexed results are unavailable.

The watcher/service, IPC client, structural parser/query foundation, app, editor, TUI, and plugin crates remain deferred to their own accepted slices. In particular, no database can become source authority.
