# Scorecard: CLI and Note Operations

**Feature ID:** c-cli-note-operations
**Spec file:** gauntlet-output/specs/c-cli-note-operations.md
**Reviewer agent:** Blind verification agent (Spec Gauntlet, mg-vault)
**Date:** 2026-08-30
**Spec iteration reviewed:** 2

---

## Verdict: PASS

**Summary:** The spec's strongest quality is its mutation-safety spine: a single
`plan_*`/`commit_*` planner (§4.3) whose canonical plan hash binds every
execution-affecting field (§4.2), mandatory opened-handle fingerprint
revalidation at commit (§4.3, §4.4), and a fail-closed descriptor-relative
confinement backend that refuses to mutate at all when the platform gate does not
pass (§4.6) — no auto-fail condition is reachable from any flow in §3.2. The most
critical gap is factual staleness in §7.1: the spec pins current state to commit
`bb2b723` and asserts "no index health integration exists", but the working tree
is eight commits ahead and already ships `mg-vault-index` (SQLite, `Freshness`
= empty/current/stale/degraded) plus `mg-vault index rebuild|status`, `search`,
and `interop export` — none of which §4.2's normative command table, §3.2's
freshness vocabulary, or §7.2's delta reconcile.

---

## Lens 1: Data Ownership and Format Compatibility (weight: 30%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 1A Authority | 3 | §3.2 Read 4 ("Read never consults the index for source bytes"), §3.2 Retrieval 4 ("No index response may supply note bytes or authorize a write"), §4.1 ("Direct-file operations never require SQLite and never read index rows as source"), §4.4 derived-state bullet. Index is disposable and cannot become a mutation precondition. | — |
| 1B Preservation | 3 | §7.5 forbids Markdown/YAML parsing, formatting, or frontmatter mutation; §3.2 Append 1 adds no implicit separator/newline; §5.1 `raw_read_is_byte_exact` (no final newline, CRLF, emoji) and `append_adds_no_implicit_separator`; §6.4 row 1B commits to byte-exact read/replace/append/relocation with no reserialization. | — |
| 1C Identity | 2 | §4.2 states paths are public identity, "never Unicode-normalized", no UUID injection, and trash/transaction IDs are handles not identities — excellent. But §4.2 also rules "a path not representable as UTF-8 fails explicitly", which silently regresses shipped behavior: `crates/mg-vault-cli/src/main.rs::path_json` already emits non-UTF-8 paths losslessly as `{"encoding":"unix_bytes_hex",...}` (test `search_and_status_encode_non_utf8_paths_and_escape_filename_controls`). The spec neither cites nor justifies removing an identity representation that exists. | In §4.2, either adopt the shipped `unix_bytes_hex` path encoding for the v1 envelope or state explicitly that non-UTF-8 paths become unaddressable and why that is acceptable for a filesystem-authority product. |
| 1D Coexistence | 3 | §4.4 scopes `.mg-vault/` to portable trash + versioned transaction metadata, per-user state to XDG, and forbids generic note commands from mutating `.mg-vault` or `.obsidian`; §3.6 `unsafe_path` covers protected directories; §5.1 `note_path_rejects_escape_and_controls` fixtures both directories; §6.5 restricts internal namespaces to narrowly scoped privileged core methods. | — |
| 1E Transactions | 3 | §3.2 Create 5 (no `--force`), Move 5 ("no overwrite flag", fail-closed collisions), Edit 5 (mismatch retains proposal, "never chooses either version"), Trash 5 (journal reconciliation, quarantine not discard); §4.4 journal-before-first-destructive-step with `unknown` reporting; §5.2 `move_and_rename_collision_rollback`, `purge_fault_matrix`, `registry_atomicity`. | — |

**Lens average:** 2.80
**Lens pass:** Yes — avg ≥ 2.0, zero 1s, no 0s

---

## Lens 2: Editor and TUI Excellence (weight: 25%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 2A Keyboard completeness | 3 | §3.4 ("`--no-input` guarantees no prompt, chooser, editor launch, confirmation, or `/dev/tty` read"; every guided choice has an operand/flag), §3.1 help-grammar view, §3.7 focus order, §5.1 `no_input_has_no_prompt_paths` (prompt/editor spy asserts zero interaction calls), §5.2 `guided_and_scripted_create_parity`. | — |
| 2B Editing durability | 3 | §3.2 Edit 2 (private permission-restricted working copy, `$VISUAL`→`$EDITOR`, validated exit), Edit 5 (conflict retains proposed bytes in recovery state), Edit 6 (unchanged buffer = `changed:false`, no write); §3.6 `editor_unavailable`/`editor_failed` preserve the working copy with a recovery command; §4.4 receipts + retained proposals, D's persistent undo journal explicitly deferred with a named seam; §5.2 `edit_external_change_preserves_versions`. Deferral of keystroke undo is justified and testable, not a dodge. | — |
| 2C Workspace | 2 | §3.2 handoff 3 legitimately disclaims panes/tabs/splits/preview/session persistence and assigns them to E, and handoff 5 forbids C from advertising them (`provided_by: "editor"`/`"tui"`); §3.2 handoff 4 names an E-owned conformance test for two-tab split/source-preview restore "by paths". But C defines no session-record or workspace-state contract that E would consume — only `note locate/open` (§4.2 `note.locate/open` data fields). The seam for restore is asserted as a test obligation, not specified as data. | Add to §4.2/§4.3 the minimal C-owned contract E needs for session restore (path list + fingerprint + unresolved-entry representation), or state that E owns that schema entirely and C contributes only `note.locate`. |
| 2D Text correctness | 2 | §3.7 guarantees Unicode passthrough with no normalization/lossy conversion, width affecting presentation only, grapheme segmentation confined to metadata wrapping; §5.1 `metadata_controls_are_escaped`; §6.4 row 2D defers grapheme-safe cursor/structural editing to D over byte/fingerprint APIs. However C exposes no offset/span mutation contract at all — only whole-file replace (§3.2 Edit) and tail append (§3.2 Append) — so D's only structural path is full-buffer replacement. The already-shipped `Vault::edit_note_span` (char-boundary validated, `crates/mg-vault-core/src/vault.rs`) is neither used nor mentioned. | In §4.3 either expose a boundary-validated span-replace contract (the core primitive exists) or state explicitly that D must always commit whole-file bytes and why. |
| 2E Degraded experience | 3 | §3.6 `index_unavailable`/`index_stale` ("Direct-file commands continue"), §3.2 Retrieval 4 fail-closed default with truthful state, §4.4 derived-state bullet keeping create/read/edit/append/move/rename/trash/restore/purge available, §7.4 (B absent ⇒ only truthful runtime state is `unavailable`, exit 6, never placeholder results), §5.2 `degraded_without_index`. | — |

**Lens average:** 2.60
**Lens pass:** Yes — avg ≥ 2.0, zero 1s, no 0s

---

## Lens 3: Knowledge Retrieval and Structure (weight: 20%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 3A Determinism | 2 | §3.2 Retrieval 6 is an excellent ordering contract (rank desc → raw UTF-8 path byte order → match byte offset; link/graph tuple ordering; "Floating-point score, filesystem enumeration, locale collation, and arrival order never determine output"), and §5.1 `query_operator_matrix_is_exact` fixtures it. It is undercut by staleness: §7.1's "no index health integration exists" is factually wrong, and the spec's freshness vocabulary (`current\|stale\|rebuilding\|unknown`, §3.2 Retrieval 1) does not match the shipped `Freshness{Empty,Current,Stale,Degraded}` plus evidence strings (`source_observed_snapshot`, `direct_rebuild_snapshot`) in `crates/mg-vault-index/src/lib.rs` and `mg-vault-cli/src/main.rs`. No mapping or migration is specified. | Correct §7.1 and add to §3.2 Retrieval 1 an explicit mapping from the shipped `Freshness`/evidence vocabulary and the current direct-scan fallback to the spec's four states. |
| 3B Ambiguity | 3 | §3.2 Retrieval 3 (ambiguous targets carry ordered candidates and "no chosen target"), Retrieval 5 (chooser is interactive-only; automation must pass an exact path and "may not use rank/first-result selectors"; generation change invalidates the chooser); §3.6 `ambiguous` row is marked "No; mutation forbidden"; §3.2 Move 2 forbids fuzzy selectors for mutation. | — |
| 3C Query depth | 3 | §3.2 Retrieval 2 covers every element the criterion names: `--text/--title/--regex/--tag/--property KEY[=VALUE]/--path`, plus `--links-to`, `--linked-from`, `--task-status`, `--due-before/after`, `--created/modified-before/after`, ISO-8601 with explicit timezone, versioned regex dialect and property coercion, and `unsupported_query` instead of silent broadening. | — |
| 3D Derived authority | 3 | §3.2 Retrieval 1 (`derived_from: "ordinary_files"` on every envelope and human block), Retrieval 4 (stale can never be a mutation precondition), §4.4 derived-state bullet, §6.4 3D. `query --view` renders Canvas/Bases/formula results as inert derived data with no mutation authority. | — |
| 3E Scale | 3 | §4.7 sets 100,000-note/1,000,000-block budgets per query class (text/title/tag/property/path/task/date p95 ≤ 250 ms, regex ≤ 500 ms, depth-2 graph ≤ 500 ms), CLI overhead ≤ 25 ms, page bounds (default 50 / max 1,000 / 10,000 graph elements), ≤ 16 MiB retrieval working memory, cancellation every 50 ms or 1 MiB, and "does not scan 100,000 notes for ordinary exact-path commands"; §5.2 `retrieval_scale_budget` asserts zero index calls for direct-file operations. | — |

**Lens average:** 2.80
**Lens pass:** Yes — avg ≥ 2.0, zero 1s, no 0s
**Auto-fail triggered:** No

---

## Lens 4: Security, Reliability, and Extensibility (weight: 15%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 4A Confinement | 3 | §4.6 fixes a `VaultDir` descriptor-relative backend: Linux `openat2` with `RESOLVE_BENEATH\|RESOLVE_NO_MAGICLINKS\|RESOLVE_NO_SYMLINKS`, macOS `openat` component walk with `O_NOFOLLOW` and `(dev, ino)` pinning rechecked before rename/unlink, hard-link rejection, and "no lexical/canonicalize fallback and no reduced-hardening mutation mode" — unsupported platforms return `confinement_unavailable`/exit 5 before planning. §5.2 `unsafe_symlink_swap` and `platform_confinement_gate` gate the capability. This directly closes the TOCTOU the current `fs::canonicalize` code acknowledges. | — |
| 4B Concurrency | 3 | §3.2 Edit 5, Append 2, Move 6, Trash 1, Restore 3 all require `--expected` or an explicit `--read-current` baseline; §4.3 "Commit revalidates every plan precondition from opened handles immediately before mutation; plans and plan hashes are not authorization tokens"; §5.2 `two_process_conflict` requires one commit and one conflict, "never silent last-writer-wins", and `trash_restore_purge_stale_preconditions` extends this to trash metadata/payload digests. | — |
| 4C Least privilege | 3 | §4.3 "No plugin, AI, index response, or CLI JSON supplied by an untrusted caller can construct `VerifiedInput`, `ConfirmedPermanentDelete`, or a transaction receipt"; confirmation is a separate satisfaction type that a prior dry-run "cannot manufacture". §6.5 bounds a forged index daemon to proposing read-only rows and gates private fields behind `--include-private`, unavailable to plugins/AI. §4.4 restricts `.mg-vault` to narrowly scoped privileged core methods. | — |
| 4D Recovery | 3 | §3.2 Trash 5 (idempotent journal recovery, quarantine orphans rather than discard), §4.4 (journal written and synced before the first destructive step; "If the tool cannot prove whether a step committed, it reports `unknown`"; recovery never targets a path whose fingerprint differs from the journal precondition), §6.3 claims audit ties every success verb to its proof; §5.2 `purge_fault_matrix`, `case_only_rename_recovery`. | — |
| 4E Contracts | 2 | §4.2 is a strong contract: normative per-command `data` field table in schema order, always-present arrays, `sha256:` digests, RFC 3339 nanosecond timestamps, golden fixtures, and "a field removal/type change/stream move requires envelope version 2". The defect is reconciliation: the shipped binary already emits v1 envelopes for `search` (`{query, results, status, generation, note_count, degraded, diagnostics, freshness, persistence, schema_version, parser_version, …}`), `index rebuild\|status`, and `interop export`. §4.2's table lists none of those commands and redefines `search` data to `{query_version, freshness, source_generation, index_generation, derived_from, records, next?, ordering}` — a field removal and a `freshness` vocabulary change — while keeping `version: 1`, violating the spec's own version-2 rule. §4.3's exit categories 2–7 also break the shipped uniform exit 1 with only a one-line hedge in §7.2. | In §4.2, add rows for the already-shipped `index.rebuild`, `index.status`, `search`, and `interop.export` commands and state explicitly whether the redefinition is a v2 break, a rename, or a retirement; in §4.3 state the compatibility decision for moving off exit 1. |
| 4F Privacy | 3 | §6.1 (content emitted only on explicit read/JSON request, never in logs, status, doctor, dry-run, error details, shell strings, or telemetry; owner-only temp/recovery files); §4.2 ("A plan is safe to log only after path redaction; note content is never embedded"); §3.6 (`invalid_input` identifies the field "without echoing body content"; error JSON prohibits body/env/editor-command fields); §4.4 receipts "never contain note bodies"; §6.5 (no query history persisted; editor argv never carries content). | — |

**Lens average:** 2.83
**Lens pass:** Yes — avg ≥ 2.0, zero 1s, no 0s

---

## Lens 5: Performance and Accessibility (weight: 10%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 5A Offline/local-first | 3 | §4.3 ("No network authentication or rate limiting applies; local IPC uses peer ownership checks and a vault-scoped endpoint"), §4.7 ("No network payload exists"), §6.1 ("No network access occurs"), §4.5 (no network client required), §5.2 forbids network access in tests. | — |
| 5B Responsiveness | 3 | §4.7 gives explicit, measurable budgets: `--help` p95 ≤ 50 ms with no registry/index scan, registry resolve ≤ 100 ms at 1,000 vaults, `status` ≤ 200 ms with a 100 ms index-health deadline, ≤ 1 MiB note commit p95 ≤ 100 ms, relocation ≤ 150 ms, ≤ 8 MiB streaming working memory, ≤ 20 MiB startup RSS, and cancellation checks every 50 ms or 1 MiB with "durability is not weakened to meet latency". | — |
| 5C Accessible equivalents | 3 | §3.2 Retrieval 3 defines the adjacency text mode (`node PATH`, indented `edge KIND TARGET STATE`) and requires Canvas/Bases/formula views to carry "a complete ordered text/table representation of every card, field, formula value/error, node, and edge"; §3.7 requires ordered text and JSON for graph, cards, media, and every health state, printing `description: unavailable` rather than promoting filenames; §5.1 `textual_equivalents_are_complete` asserts every JSON semantic element appears in ordered plain text. | — |
| 5D Terminal resilience | 3 | §3.3 (one-record-per-block below 60 columns; IDs, paths, error codes, and recovery commands never truncated; wrapping with indentation; JSON shape independent of width), §3.5 (no animation; static stderr progress only in interactive human mode), §3.7 (ANSI-stripped equivalence, 40/60/120-column wrapping tests, `\u{...}` escaping of bidi/control bytes with identity retained in JSON), §5.4. | — |
| 5E Automation | 3 | §3.1 stream discipline (one JSON object plus newline; success on stdout, error on stderr, other stream empty), §3.4 (`--json`, `--jsonl`, `--no-input`, `--no-color`, `NO_COLOR` precedence, stdin only with `--stdin`, broken-pipe rule), §4.2 JSONL subprotocol with a terminal record so consumed rows cannot read as complete success; §5.2 `pipeline_round_trip`, `no_color_contract`, `broken_pipe_behavior`; §5.3 item 9. | — |

**Lens average:** 3.00
**Lens pass:** Yes

---

## Feasibility Check

Source verified against the working tree at `/home/mgeist/geist/vault` (HEAD `dfe33cf`, 2026-08-24, with uncommitted modifications to `mg-vault-cli/src/main.rs`, `mg-vault-index/src/lib.rs`, and docs). The spec is dated 2026-08-23 and pins itself to `bb2b723`, which is eight commits behind.

| Check | Status | Notes |
|---|---|---|
| Types/models exist or are clearly specified | ✓ | `SourceFingerprint`, `Note`, `TrashReceipt`, `Vault`, `VaultRegistry`, `XdgPaths` exist in `mg-vault-core`. §4.2's `NoteMutationPlan`, `TransactionRecord`, `TrashEntry`, `IndexHealth` are new but fully specified with versioned schemas and a closed `CanonicalOperationOptions` enum. Current `TrashMetadata` (version 1, `original_path`, `fingerprint`) is a strict subset of §4.2's `TrashEntry`, so migration is additive. |
| API/interface changes are feasible with current architecture | ✓ | §4.1's split (`cli.rs`, `commands/`, `output/`, `interaction.rs`) fits the 658-line monolithic `mg-vault-cli/src/main.rs`; `docs/ARCHITECTURE.md` already states the CLI "owns no filesystem policy", matching §4.1's rule that CLI code never calls `std::fs` for managed mutation. `plan_*`/`commit_*` are new but sit naturally beside `create_note`/`write_note`/`trash_note`/`restore_note`. |
| Views/screens fit current navigation pattern | ✓ | All views in §3.1 are clap subcommands. `vault register/list/select` and `note create/read/write/trash/restore` exist; `note write` is not mentioned in §3.1 and appears to be superseded by `note edit` without a stated rename/retirement. |
| Dependencies are available and version-compatible | ✓ | Workspace `Cargo.toml` already has clap 4.5, serde 1, serde_json 1, sha2 0.10, thiserror 2, and **rustix 1 (features `fs`, `process`)** — the exact backend §4.6 requires for `openat2`/`RESOLVE_BENEATH` and descriptor-relative rename/unlink. `tempfile 3` is dev-only, so the runtime temp/editor adapter of §3.2 Edit 2 needs promotion. §4.6's `shell-words`-equivalent parser and §5.3's PTY harness are unnamed and absent. Workspace lints `unsafe_code = "forbid"` permit rustix but forbid the raw-`libc` half of §4.6's "`rustix`/libc" phrasing. |
| Platform/renderer requirements are realistic | ✓ | Linux-first with a macOS `openat` walk is consistent with `mg-vault-index`'s existing `#[cfg(target_os = "linux")]` confined-parent code. Fail-closed `confinement_unavailable`/`durability_unavailable` gates make the unproven platforms safe by construction. |
| Test strategy is executable with current infrastructure | ✓ (caveats) | `mg-vault-cli/tests/cli.rs` already drives `CARGO_BIN_EXE_mg-vault` with a fake `HOME` and XDG dirs, exactly the pattern §5.2 assumes. `foundation.rs`, `index.rs`, `persistent_store.rs` show fault/degradation testing is established. Not yet available: PTY harness (§5.3), property-testing crate (§5.1 "Property-test"), fault-injection shim (§5.2), and the 100,000-note/1,000,000-block fixture, which §7.4 correctly makes B-dependent. |
| Performance budget is realistic for target hardware | ✓ | Direct-file budgets (≤ 50/100/150/200 ms, ≤ 20 MiB RSS) are plausible for a Rust CLI on warm SSD. Caveat: `mg-vault-cli/Cargo.toml` currently links `mg-vault-index` and therefore bundled SQLite into the same binary, which the spec's §4.1 ("Direct-file operations never require SQLite") assumes is out of process. |
| No undeclared dependency on unbuilt features | ✗ | §3.2 Retrieval, §4.2's `search/query` and `links/backlinks/graph` rows, and §4.7's retrieval budgets all assume a **versioned IPC B service that does not exist**; `docs/ARCHITECTURE.md` confirms "the watcher/service, IPC client … remain deferred". §7.4 declares B as a dependency and mandates a contract fake, which is the correct treatment — but the spec never accounts for the **in-process** retrieval that already ships (`mg-vault-index` linked into the CLI; `search`, `index rebuild`, `index status` subcommands with a live direct-scan fallback), so the transition from shipped in-process search to IPC-only retrieval is undeclared work. |

**Stale or incorrect claims found in §7.1 (recorded per the review protocol):**

1. "no index health integration exists" — **false**. `mg-vault index status` and `search` already report `status`, `generation`, `note_count`, `degraded`, `diagnostics`, `freshness`, `persistence`, `schema_version`, `parser_version`, and `derived_from`, and the test `status_detects_external_source_change_without_publishing_it` proves the stale-refusal behavior.
2. The file inventory omits `mg-vault-core/src/{frontmatter,frontmatter_scalar,index,interop}.rs` and the entire `crates/mg-vault-index` crate (SQLite, schema v2, `PersistentIndexStore`, `recover_and_rebuild`).
3. The CLI command inventory omits `index rebuild`, `index status`, `search QUERY`, and `interop export`.
4. The implicit "no … edit … exists" is imprecise: `Vault::edit_note_span` (byte-preserving, char-boundary-validated span replacement) exists in core; only the CLI `note edit` subcommand is absent.
5. Test inventory omits `mg-vault-core/tests/{index,frontmatter_scalar}.rs` and `mg-vault-index/tests/persistent_store.rs` (813 lines of fault/durability coverage).
6. §7.5's non-goal "No import/export/publishing implementation in Milestone 3" conflicts with the already-shipped `mg-vault interop export` (`interop.rs`, `mg.interop/1`), which the spec neither claims nor retires.

Confirmed-accurate §7.1 claims: XDG/registry/canonicalization, `.md` path validation with protected-directory rejection, create-new collision via `renameat` `RENAME_NOREPLACE`, fingerprint-checked atomic replace, vault-local trash/restore, non-interactive flag-only body input, exit failures collapsing to `ExitCode::from(1)`, minimal v1 JSON envelope `{version, ok, data}`, `--no-color` accepted but unrendered, no trash listing/purge, and unresolved hostile directory-entry races in the `fs::canonicalize` path checks.

**Feasibility verdict:** Feasible with caveats
**Caveats:** (a) §7.1/§7.2 must be refreshed to the current tree before implementation, or an implementing agent will duplicate or clobber the shipped index/search/interop surface; (b) the retrieval half of this spec cannot be accepted until B's IPC exists — §7.4 says so, but the shipped in-process search path needs an explicit retire/migrate decision; (c) runtime `tempfile`, a shell-word parser, a PTY harness, and a property-test crate are all required and unnamed.

---

## Composite Score

| Lens | Average | Weight | Weighted |
|---|---|---|---|
| Data ownership and format compatibility | 2.80 | 30% | 0.840 |
| Editor and TUI excellence | 2.60 | 25% | 0.650 |
| Knowledge retrieval and structure | 2.80 | 20% | 0.560 |
| Security, reliability, extensibility | 2.83 | 15% | 0.425 |
| Performance and accessibility | 3.00 | 10% | 0.300 |
| **Composite** | | | **2.78** |

**Pass conditions (from criteria.md):**
- [x] Composite ≥ 2.0 — 2.78
- [x] All lens averages ≥ 2.0 — 2.80 / 2.60 / 2.80 / 2.83 / 3.00
- [x] No criterion scores 0
- [x] No more than two criteria at 1 per lens — zero criteria at 1
- [x] All auto-fail rules pass — see below
- [x] Feasibility ≠ Infeasible — Feasible with caveats

**Auto-fail walk (each rule checked against §3.2 flows):**

| Rule | Result | Controlling section |
|---|---|---|
| Source-content loss | Pass | §3.2 Create 5 (no `--force`), Edit 5, Move 5, Trash 5, Restore 3, Purge 4 |
| Partial multi-file mutation | Pass | §3.2 Trash 1 ("note plus metadata as one recoverable transaction"), §4.4 journal phases + quarantine, §5.2 `purge_fault_matrix` |
| Index state overriding source | Pass | §3.2 Retrieval 4 ("No index response may supply note bytes or authorize a write") |
| Stale index presented as current | Pass | §3.2 Retrieval 4 fail-closed default + per-block `freshness: stale` labeling; §8 item 5 defines when `current` is legal |
| Silent conflict winner | Pass | §3.2 Edit 5 ("never chooses either version"), §5.2 `two_process_conflict` |
| Unknown syntax loss | Pass | §7.5 no Markdown/YAML parsing; §5.1 `raw_read_is_byte_exact` |
| Unconfirmed overwrite/import | Pass | No overwrite flag for create/move/restore; `--overwrite-output` applies only to an export file, never source (§3.2 Read 2); purge requires literal confirmation or `--yes` (§3.2 Purge 4). `--read-current` is an explicit opt-in baseline still revalidated at commit (§4.3), not an unconfirmed overwrite |
| Unsafe traversal or symlink escape | Pass | §4.6 `RESOLVE_BENEATH`/`O_NOFOLLOW`/`(dev,ino)` pinning, "no lexical/canonicalize fallback", §5.2 `unsafe_symlink_swap` |
| Capability/data-exfiltration bypass | Pass | §4.3 private satisfaction types, §6.5 forged-daemon bound |
| Active raw HTML/script by default | Pass | §6.4 — graph/Canvas/Bases rendered as inert text/JSON only |
| Non-atomic save claiming success | Pass | §3.2 Create 6 / Move 6, §4.6 `durability_unavailable`, §6.3 claims audit |
| Recovery overwriting newer source | Pass | §4.4 ("Recovery writes never target a path whose current opened-handle fingerprint differs from the journal's precondition") |
| Graph/Canvas lacking textual equivalent | Pass | §3.2 Retrieval 3, §3.7, §5.1 `textual_equivalents_are_complete` |

**All conditions met:** Yes → PASS

---

## Remediation Brief

Verdict is PASS, so no Priority 1 blockers are recorded. The items below are required for quality and should be applied before implementation begins, because a stale gap analysis will misdirect an implementing agent.

### Priority 2 — Should fix for quality

1. **§7.1 — rewrite the current-state inventory against the tree, not `bb2b723`.** Delete "no index health integration exists". Add: `mg-vault-index` (SQLite, schema v2, `PersistentIndexStore`, `Freshness{Empty,Current,Stale,Degraded}`, `recover_and_rebuild`); `mg-vault-core/src/{frontmatter,frontmatter_scalar,index,interop}.rs`; `Vault::edit_note_span`; CLI `index rebuild`, `index status`, `search QUERY`, `interop export`; tests `mg-vault-core/tests/{index,frontmatter_scalar}.rs` and `mg-vault-index/tests/persistent_store.rs`. Pin to HEAD `dfe33cf`.
2. **§4.2 — reconcile the normative command table with the shipped v1 surface.** Add rows for `index.rebuild`, `index.status`, `search`, and `interop.export` as they exist today, then state explicitly for each whether this spec renames, retires, or version-2-breaks it. As written, the `search/query` row removes and retypes fields of a live v1 command while keeping `version: 1`, contradicting the spec's own rule that a field removal or type change requires envelope version 2.
3. **§3.2 Retrieval 1 / §7.2 — state the in-process-to-IPC transition.** §7.2 says "do not add an indexer to C", but the `mg-vault` binary already links `mg-vault-index` and performs live rebuilds (`MarkdownIndex::rebuild`, `PersistentIndexStore::rebuild`) with a direct-scan fallback when the persistent store is not current. Say whether that fallback survives under the fail-closed `--allow-stale` model, and map the shipped `Freshness` values and evidence strings (`source_observed_snapshot`, `direct_rebuild_snapshot`) onto `current|stale|rebuilding|unknown`.
4. **§4.2 — resolve the non-UTF-8 path contradiction (1C).** The shipped CLI already encodes non-UTF-8 paths losslessly as `{"encoding":"unix_bytes_hex","value":…,"display":…}`; the spec hard-fails them. Adopt the existing encoding or justify the narrowing.
5. **§3.1 / §7.2 — account for `note write`.** The existing `note write --body --expected` subcommand does not appear in the view inventory or the delta; state whether `note edit` replaces it and whether the old name remains as an alias.

### Priority 3 — Consider for excellence

6. **§4.3 — expose a boundary-validated span-replace contract** (or explicitly rule it out) so D is not forced into whole-file replacement for every keystroke commit; `Vault::edit_note_span` already implements the char-boundary and fingerprint checks.
7. **§4.5 / §5.3 — name the missing crates**: a runtime temporary-file adapter (`tempfile` is currently dev-only), the shell-word tokenizer, the PTY harness for §5.3, and the property-testing crate for §5.1's "Property-test" cases; note that `unsafe_code = "forbid"` in the workspace lints rules out the raw-`libc` alternative mentioned in §4.6.
8. **§4.1 / §4.7 — address SQLite linkage.** `mg-vault-cli/Cargo.toml` depends on `mg-vault-index` today, so the direct-file binary links bundled SQLite; either state that C's binary drops that dependency once IPC lands, or adjust the §4.7 startup-RSS reasoning.
9. **§2C seam** — publish the minimal path/fingerprint/unresolved-entry record E needs for session restore, so the conformance test named in §3.2 handoff 4 has a C-side contract to bind to.

---

**End of scorecard.**
