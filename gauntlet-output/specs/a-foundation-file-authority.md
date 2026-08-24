# Spec: Foundation and File Authority

**Feature ID:** a-foundation-file-authority
**Parent feature:** root
**Date:** 2026-08-23
**Iteration:** 1

## 1. Purpose

Provide a trustworthy local filesystem boundary so users can register ordinary vault directories and create, read, replace, trash, and restore Markdown without silent overwrite or escape. Success means fault-safe tests demonstrate that every accepted mutation remains confined and durable or reports failure.

## 2. User stories

- As a user, I register and select independent vault directories so commands target an explicit namespace.
- As a writer, I create and read Markdown while retaining ownership through ordinary files.
- As an automation author, I use versioned JSON and no-input behavior without parsing decoration.
- As a concurrent editor, I receive a conflict instead of overwriting bytes changed since my read.
- As a recovering user, I trash and restore a note without permanent deletion or collision overwrite.
- As a safety-conscious user, I cannot make generic note commands mutate `.obsidian`, `.mg-vault`, parent paths, or symlink escapes.

## 3. UX specification

This slice introduces CLI commands only: `vault register/list/select` and `note create/read/write/trash/restore`. Human output is concise; `--json` returns a version-1 envelope. `--vault` overrides selection. Missing required input is an error and `--no-input` guarantees no prompt. `--no-color` and `NO_COLOR` suppress styling; this slice emits no color in either mode. Errors identify collision, conflict, invalid path, missing selection, or I/O without claiming mutation. There is no animation/UI. Keyboard and screen-reader access are inherited from line-oriented terminal output.

## 4. Implementation specification

`mg-vault-core` owns `XdgPaths`, the JSON `VaultRegistry`, canonical `Vault` path authority, `SourceFingerprint`, atomic write, and trash primitives. `mg-vault-cli` owns parsing and output only. Registry roots are canonical directories. User paths are relative and reject parent/root/prefix components; each use revalidates canonical containment. Mutation rejects first components `.obsidian` and `.mg-vault`. Existing-source replacement compares a fingerprint immediately before replacement. Atomic output uses a same-directory unique file, `sync_all`, rename, and directory `sync_all`. Note creation uses collision-refusing `create_new`. Trash uses a vault-local internal namespace with explicit restore and no purge.

Dependencies are limited to Clap, Serde/JSON, SHA-256, and typed errors. Storage is small JSON metadata plus source bytes. No network, database, daemon, Markdown parser, or platform UI is involved. Core behavior is portable to Linux/macOS; directory synchronization follows supported Rust filesystem semantics.

## 5. Test specification

Unit/integration tests cover XDG overrides and HOME fallback; registry canonicalization/selection/round-trip; absolute/parent traversal; symlink escape; protected directory mutation; create collision; byte-exact read; fingerprint change conflict; atomic replacement; trash/restore round-trip and restore collision. CLI tests cover versioned JSON and selected-vault create/read. Tests use synthetic temporary directories. Manual checks run help and both output modes.

## 6. Compliance and safety gate

Vault notes may be sensitive, but this slice transmits nothing and logs no content. JSON read output includes content only because the user explicitly requests the note. No third-party assets are included. User-visible claims are limited to implemented behavior. The auto-fail boundaries for source loss, overwrite, traversal/symlink escape, false atomic success, and recovery overwrite are directly tested.

## 7. Gap analysis

The repository was absent before this slice. This spec creates the workspace, two crates, docs, gauntlet binding files, and tests. Scope is medium because durable filesystem boundaries have adversarial edge cases. Later token-preserving Markdown, SQLite/index, editor/TUI, multi-file transactions, descriptor-relative race hardening, and permanent purge are separate dependencies/deferred work.

## 8. Open questions

- License remains MIT versus Apache-2.0 and blocks a LICENSE file, not implementation.
- Descriptor-relative Linux hardening is deferred because portable canonical checks satisfy this foundation's accepted scope but do not eliminate all hostile concurrent directory-entry races.
