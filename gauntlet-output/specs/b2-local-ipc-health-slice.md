# Slice Spec: Local IPC Health Boundary

**Parent:** `b-index-service-search` (B2 subset)  
**Milestone:** 2  
**Scope:** versioned, owner-only local service health transport

## Contract

The per-user `mg-vault-indexd` process exposes a Unix-domain socket outside every vault. A reusable client crate owns a length-prefixed JSON protocol with a hard frame limit, explicit hello negotiation, request IDs, protocol/service versions, and typed errors. The server validates Linux peer credentials before reading a request and accepts only its effective UID. The socket directory and socket are owner-only.

`mg-vault service status` performs the real hello/status exchange and reports service protocol/version and registered-vault count. It never opens SQLite. The service reads the authoritative registry through `mg-vault-core`; this slice adds no watcher, index mutation, search, daemon autostart, or database ownership transfer. Existing index/search behavior remains unchanged until a later service-operation slice can migrate it without reducing operator functionality.

## Acceptance tests

1. A real server and client negotiate protocol v1 and return status with matching request IDs.
2. No version overlap returns `protocol_incompatible` and no status request runs.
3. Frames larger than 1 MiB are rejected before allocation/deserialization.
4. The runtime directory is mode `0700`, the socket is mode `0600`, and the endpoint is outside the vault.
5. UID authorization rejects a mismatched peer identity.
6. `mg-vault service status` emits the existing versioned CLI JSON envelope over the IPC path.

## Non-goals

- Watchers, dirty queues, reconciliation, rebuild scheduling, cancellation, subscriptions
- Routing `index` or `search` through the service
- Service autostart/systemd packaging, stale socket recovery, cross-platform IPC
- Any source-file mutation or direct client SQLite access

## Required evidence

- Behavior-first unit/integration/CLI tests
- `cargo fmt --all --check`
- `cargo test --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo audit` when available
- `git diff --check` and scoped status/diff review
