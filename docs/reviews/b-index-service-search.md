# Scorecard: Index Service and Search Foundation

**Feature ID:** b-index-service-search
**Spec file:** docs/specs/b-index-service-search.md
**Reviewer agent:** blind verification agent
**Date:** 2026-08-30
**Spec iteration reviewed:** 1

---

## Verdict: PASS

**Summary:** The spec's strongest quality is that it treats freshness truthfulness as
the load-bearing contract rather than a nicety — §3.2 (search step 3), §3.2
(overflow step 5), §3.6 `index_stale`, and §4.3 `SearchPage` collectively make it
structurally impossible to label a behind-index `current`, and §4.7 states the
100k-note/1m-block budgets with an explicit editing-isolation clause. The most
critical gap is factual, not architectural: §4.1 and §7.1 assert that
`mg-vault-index`, the SQLite schema, freshness generations, and the `index`/`search`
CLI commands are absent, and all four now exist in the tree. That staleness leaves
two real design holes unaddressed — the shipped CLI opens the index database directly
(which §7.5 forbids but §7.2 never schedules for removal), and the shipped
`empty|current|stale|degraded` freshness vocabulary and `version: 1` JSON envelope
have no migration path to the spec's six-state model.

---

## Lens 1: Data Ownership and Format Compatibility (weight: 30%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 1A Authority | 3 | §1.2 "SQLite and extracted structure are derived data only; no index observation, migration, repair, or query may modify a vault source or `.obsidian`". §4.3 "Search never returns source bytes as authoritative; exact content/open operations re-read the file through foundation APIs and compare the indexed fingerprint". §4.1 states the index crate "has no source-mutation API". §3.2 (search step 8) forbids emulating backlinks/properties/tasks from stale SQLite even in fallback. §5.2 asserts byte-for-byte source non-mutation across a delete-and-rebuild cycle. | — |
| 1B Preservation | 3 | §4.2 "Duplicate properties, invalid YAML, malformed links, and malformed tasks produce diagnostics/projections where safe rather than rewriting input". §4.5 binds the indexer to the token-preserving A4/F parser and forbids "a second incompatible dialect". §5.2 builds a compatibility vault with "typed/unknown YAML, duplicate/malformed properties" compared to approved fixtures. | — |
| 1C Identity | 3 | §4.2 "Unique keys are internal only; they never become note identity or hidden YAML UUIDs"; paths stored as "vault-relative identity plus a safe display representation". §7.5 non-goal explicitly bars "injecting identifiers into Markdown". §4.6 ties case sensitivity and Unicode normalization to "actual path identity". | Minor: §3.4 says "paths remain UTF-8"; on Linux paths are arbitrary bytes (the shipped store already uses `path BLOB PRIMARY KEY`). State byte identity explicitly. |
| 1D Coexistence | 3 | §1.2 bars any modification of `.obsidian`. §4.1 "No index or socket is stored inside a vault"; databases live under XDG cache/data. §7.4 (A1/A2) requires portable `.mg-vault` exclusion/config access. §7.5 non-goal covers `.obsidian` and `.mg-vault` source. | §3.2 (watch step 2) says only "internal application/runtime paths" are skipped; name `.obsidian` and `.mg-vault` as index exclusions explicitly. |
| 1E Transactions | 3 | §3.2 (incremental step 3) one transaction per observed source version, discarded on fingerprint change. §3.2 (rebuild steps 2–6): never truncate the readable DB in place, integrity/schema/referential checks before publication, atomic publish, "A crash before publication leaves the old generation and an incomplete candidate that is safe to delete". §3.6 `source_changed`, `migration_failed`, `resource_exhausted` ("no false success"). §5.1 `generation_publishes_only_after_complete_manifest`, `migration_failure_keeps_prior_database`. | — |

**Lens average:** 3.00
**Lens pass:** Yes — avg ≥ 2.0, zero 1s, zero 0s

---

## Lens 2: Editor and TUI Excellence (weight: 25%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 2A Keyboard completeness | 3 | §3.4 "Keyboard and standard CLI arguments are complete", with an enumerated control set (`--vault`, `--json`, `--no-input`, `--no-color`, `--limit`, `--cursor`, `--sort`, `--deadline`, `--allow-stale`, `--explain`), `Ctrl-C` cancellation semantics, and explicit `--query-file`/`--stdin` rather than inferred piping. §3.1 lists a reachable command for every view. §4.3 defines the query grammar's operator set. | — |
| 2B Editing durability | 2 | In-scope half is covered: §4.7 "Editing isolation" guarantees no source write blocks on SQLite/service availability and caps p99 added latency at 20 ms during rebuild; §3.6 `source_changed` and §5.1 `source_change_rolls_back_projection` cover external-edit detection by fingerprint. Autosave, persistent undo, and external-edit merge are neither designed nor explicitly deferred — §7.5's first non-goal only says this feature does not edit, and §7.4 never names the owning feature. | Add an explicit deferral in §7.4 or §7.5 naming which feature owns autosave/undo/merge, and state the index's obligation toward it (e.g. that a mid-edit external change must not be published as a generation). |
| 2C Workspace | 2 | §3.1 gives a defensible deferral: "This feature adds no graphical screen and does not require the future TUI. It defines line-oriented CLI views and the data contracts later TUI/Quickshell clients consume", plus "A future TUI may render the same contracts as a full-screen search panel and health pane, but that UI is outside this milestone". §3.7 (last bullet) binds the future TUI to the same labels, cancellation, filters, and result metadata. The forward contract stops at that sentence: no pane/session-restore obligations are enumerated for the consumer. | Enumerate in §3.1 what the TUI consumer is guaranteed (stable cursor across a session, generation-bound result identity for pane restore) so E can build against it. |
| 2D Text correctness | 3 | §3.7 "Terminal control characters from source, extractor output, paths, and snippets are escaped. Unicode display width affects layout only; paths and cursors preserve exact bytes/UTF-8 identity contracts". §3.2 (search step 4) requires Unicode-aware tokenization; §4.5 requires fixture-backed segmentation/case-fold libraries. §4.2 gives every projection an exact `byte_range` tied to the indexed fingerprint, verified by §5.1 `structural_ranges_match_source_fingerprint`. §5.1 `ranking_has_total_order` uses equal-score Unicode titles/paths; §5.3 covers combining characters and RTL. | — |
| 2E Degraded experience | 3 | §3.1 "Degraded notice" view row. §3.6's fifteen-row table marks every state with an explicit user-visible presentation and "None; source remains usable". §4.6 "Service autostart is optional and never required for direct-file operation" and "Database migrations never gate source access". §5.3 degraded E2E kills the service mid-paging and asserts raw note read/write still succeeds while "no client opens the DB as fallback". | — |

**Lens average:** 2.60
**Lens pass:** Yes — avg ≥ 2.0, zero 1s, zero 0s

---

## Lens 3: Knowledge Retrieval and Structure (weight: 20%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 3A Determinism | 3 | §1.3 makes rebuild-equivalence the milestone's success signal. §3.2 (search step 6) orders results "by requested sort or by score then normalized path and structural position"; cursors bind vault, normalized query, generation, sort, and last key. §4.2 "Every derived row references one source and observed fingerprint". §5.1 `ranking_has_total_order` and `cursor_binds_generation_and_query`; §5.2 asserts equivalent normalized result sets after DB deletion and rebuild. | The score function itself is never defined (§3.1 only promises `--explain` reports "ranking strategy"). Specify the ranking inputs and field weights so two implementations agree. |
| 3B Ambiguity | 3 | §3.2 (search step 5) "this milestone stores exact parsed targets and exposes unresolved/resolution status only when a versioned resolver has supplied it". §4.2 `LinkProjection` keeps `raw_target`, `raw_alias`, `resolved_target: Option`, `resolution: ResolutionState`, `resolver_version: Option`. §6.4 3B and §7.4 (G1–G7) hand user-facing resolution to G; §7.5 disowns the backlink chooser. | — |
| 3C Query depth | 2 | §4.3 enumerates every required dimension — phrases, `text:`/`title:`/`alias:`, bounded fuzzy, bounded regex, `tag:`, `path:`, `property.KEY`, `task.state`/`task.text`, date and typed ranges, `link.to`/`link.from`/`link.unresolved`/`embed.to`, booleans and comparisons. Two gaps: (a) §3.2 (step 5) and §4.3 both say precedence is "documented" but no precedence table or grammar production is given anywhere in the spec; (b) §4.3 advertises `link.to` and `link.unresolved` while §3.2 (step 5) and §6.4 3B say resolution status exists only once G1's resolver supplies it — so the semantics of those two predicates inside B's own boundary are undefined. | Add the operator precedence table to §4.3, and define what `link.to` / `link.unresolved` return in B before a resolver exists (exact-target match only, or a typed `resolution_unavailable` error). |
| 3D Derived authority | 3 | §6.4 3D "SQLite, FTS, properties, extracted text, and later graph/Bases consumers are disposable views". §3.3 "Health and search clients derive state solely from the IPC response, never by inspecting database files". §7.5 non-goals bar direct SQLite access by CLI, TUI, Quickshell, plugins, AI, or `mg-calr`, and bar making FTS rows, watcher order, or timestamps authoritative. | — |
| 3E Scale | 3 | §4.7 names the required corpus literally: "at least 100,000 Markdown notes, 1,000,000 blocks, 2,000,000 links/tags/property/task projections combined". Editing isolation is budgeted separately and numerically: watcher enqueue p95 ≤ 10 ms, p99 added latency to direct note read/write ≤ 20 ms under a full rebuild, "no write is blocked on SQLite/service availability". Search p95 ≤ 100 ms warm / ≤ 750 ms cold; freshness p95 ≤ 500 ms; rebuild ≤ 10 min; RSS ≤ 256 MiB target / 512 MiB ceiling; storage ≤ 2.5× indexed Markdown bytes; ±15% regression gate; §5.4/§4.7 require reporting direct-edit latency during rebuild. | — |

**Lens average:** 2.80
**Lens pass:** Yes
**Auto-fail triggered:** No — see the walk-through below

### Auto-fail walk-through (criteria.md line 13)

| Rule | Result | Controlling spec text |
|---|---|---|
| Source-content loss | Pass | §1.2; §7.5 first non-goal; §5.2 byte-for-byte non-mutation assertion |
| Partial multi-file mutation | Pass | No source mutation path exists (§4.1 "no source-mutation API") |
| Index state overriding source | Pass | §4.3 "Search never returns source bytes as authoritative"; §7.5 "Exact source reads from snippets or projections; clients reopen and fingerprint source"; §3.2 search step 8 |
| Stale index presented as current | Pass | §3.2 search step 3 (default requires `current`, "stale opt-in is never implicit", every page labeled); §3.2 overflow step 5 ("advances no current-generation claim"); §3.2 reconciliation step 6 (current only after the complete scan succeeds); §3.2 rebuild step 3 ("sustained churn yields `stale` rather than false convergence"); §3.3 empty-result copy explicitly refuses to imply no source match; §3.6 `index_stale` |
| Silent conflict winner | Pass | §3.6 closing paragraph "Recovery never writes source, chooses a conflict winner"; §7.5 |
| Unknown syntax loss | Pass | §4.2 (diagnostics rather than rewriting); §4.5 shared parser dialect |
| Unconfirmed overwrite/import | Pass | §3.2 rebuild step 2 (never truncates the readable DB in place); §3.4 `--no-input` forbids implicit rebuild/stale consent |
| Unsafe traversal or symlink escape | Pass | §3.2 watch step 2 ("never follows a symlink outside the canonical vault"); §3.6 `unsafe_path`; §4.2 "database file paths are never accepted over IPC"; §4.3 "ignores caller-provided roots"; §5.1 `safe_discovery_rejects_escape_and_special_files` |
| Capability/data-exfiltration bypass | Pass | §4.3 peer-UID validation, `0700`/`0600`, no TCP listener or cross-user API; §6.1 "Plugins/AI have no ambient IPC capability" |
| Active raw HTML/script by default | Pass | No renderer; §3.3 and §3.7 escape control sequences; §5.1 `terminal_text_escapes_controls` |
| Non-atomic save claiming success | Pass | §3.2 rebuild step 5 atomic publication; §6.4 "no false rebuild success"; §3.6 `resource_exhausted` "no false success" |
| Recovery overwriting newer source | Pass | §3.6 closing paragraph; recovery writes only the external DB |
| Graph/Canvas information lacking textual equivalent | Pass | §3.7 "A textual list is the canonical representation for this milestone; no graph-only information is introduced"; §6.4 |

---

## Lens 4: Security, Reliability, and Extensibility (weight: 15%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 4A Confinement | 3 | §3.2 (watch step 2) validates vault containment and never follows a symlink outside the canonical vault. §3.6 `unsafe_path` covers traversal, protected paths, disallowed file kinds, and symlink escape. §4.2 forbids accepting database file paths over IPC; §4.3 "ignores caller-provided roots, resolves vault IDs through its registry". §5.1 `safe_discovery_rejects_escape_and_special_files` covers symlinks, FIFO, socket, traversal. §6.1 routes paths through foundation confinement. | — |
| 4B Concurrency | 2 | Source-side concurrency is strong: §3.2 (incremental step 3) discards the transaction on fingerprint change; §5.1 `source_change_rolls_back_projection`; §5.2 asserts "no SQLite busy error blocks editing". §4.4 gives each vault actor one write connection and a generation-bound read pool, and §3.2 (startup step 1) takes a single-instance lock. The gap is a second writer the spec does not know about: `crates/mg-vault-cli/src/main.rs` (`run_index`, `run_search`) opens the index database in-process today via `PersistentIndexStore::open`, so a running daemon and a CLI invocation would contend for the same SQLite file. §7.5 forbids direct SQLite access by the CLI but §7.2 lists no task to remove it, because §7.1 believes the code does not exist. | Add a §7.2 delta item: route the existing `run_index`/`run_search` direct-DB path through the service client, and define the transition behavior while no daemon is running. Add a §3.2 rule for what happens when the single-instance lock is held by a process that is not the service. |
| 4C Least privilege | 2 | §6.1 "Plugins/AI have no ambient IPC capability; future access requires their own explicit capability gate and scoped vault/query contract", and §7.5 bars direct SQLite access by plugins/AI/`mg-calr`. §4.3 applies per-client concurrency/cost limits and validates peer credentials. But the boundary is UID-only: §4.3's `Hello(client_versions[], capabilities[])` never says whether `capabilities` is authorization-bearing or merely feature negotiation, and no per-caller attribution or scoping mechanism is designed, so "scoped, attributable, previewed" is deferred rather than enabled. | In §4.3, state whether `Hello` capabilities are authorization tokens, and reserve a per-connection caller identity field so a future gated plugin caller can be scoped and attributed without a protocol break. |
| 4D Recovery | 3 | §3.2 (rebuild) builds side-by-side at a unique temp path, runs integrity/schema/referential/fixture checks before publication, publishes atomically, and leaves "an incomplete candidate that is safe to delete" after a crash. §3.1 `index doctor` gives "ordered checks and non-mutating evidence; repair requires an explicit command". §3.6 `index_corrupt` quarantines and offers rebuild; `migration_failed` keeps the prior readable DB stale-labeled. §5.1 `migration_failure_keeps_prior_database` faults every migration/publication phase. §8 Q4 leaves retention length open while requiring manual deletion to stay safe. | — |
| 4E Contracts | 2 | Forward design is strong: §4.3 requires `protocol_version`, `request_id`, echoed responses, capability negotiation that fails closed on an unsupported required capability; §3.3 puts protocol/schema version, freshness, generation, completeness, cursor, and warnings in every response; §3.6 defines fifteen stable error codes; §4.6 guarantees rolling upgrade across current and previous client versions; §5.3 uses golden JSON fixtures independent of volatile timestamps. Backward reconciliation is missing: the shipped CLI already emits a `version: 1` envelope with `status`, `freshness`, `persistence`, `degraded`, and `fallback_reason` keys, and `mg-vault-index` already publishes an `empty \| current \| stale \| degraded` freshness vocabulary. §3.2 (startup step 4) defines a different seven-state set with no `empty` and no `degraded` and no mapping, and §7.2 schedules no migration. | Add to §3.2/§4.3 an explicit mapping from the shipped `empty`/`degraded` states into the new state set (in particular: which state a complete scan with per-file read failures reports), and add a §7.2 item covering the `version: 1` CLI envelope's compatibility or deprecation. |
| 4F Privacy | 3 | §3.6 closing paragraph enumerates what logs may and may not contain, excluding note bodies, snippets, secret property values, attachment text, query literals by default, environment values, and unrelated absolute paths. §6.1 requires owner-only permissions, IPC owner verification, exclusions applied before ordinary tables/FTS/extraction, fully local processing, and deletion of derived caches. §5.2 tests that excluded properties/paths never reach tables, FTS, snippets, logs, diagnostics, or extraction jobs and that changing the exclusion config forces a rebuild. §4.1 keeps the index out of the vault, so derived private text is not swept into the user's sync tree. | — |

**Lens average:** 2.50
**Lens pass:** Yes — avg ≥ 2.0, zero 1s, zero 0s

---

## Lens 5: Performance and Accessibility (weight: 10%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 5A Offline/local-first | 3 | §4.4 "Offline behavior is the normal mode: all core indexing/search is local and makes no network request". §4.7 "Network: zero bytes for service, search, and Markdown indexing". §4.3 "There is no TCP listener, network discovery, anonymous mode, or cross-user administration API". §4.5 "There is no server, cloud database, CDN, telemetry endpoint, or proprietary sync dependency". | — |
| 5B Responsiveness | 3 | §4.7 states latency (search p95 ≤ 100 ms warm / p99 ≤ 250 ms, cold p95 ≤ 750 ms; freshness p95 ≤ 500 ms / p99 ≤ 2 s), memory (256 MiB target, 512 MiB hard ceiling), startup (IPC health p95 ≤ 150 ms, never claiming current before verification), and cancellation (2 s default deadline on expensive queries, `--deadline`, `Ctrl-C`) with a ±15% regression gate; the closing paragraph requires reporting cancellation time as a benchmark metric. | — |
| 5C Accessible equivalents | 3 | §3.7 "Every state is written as text (`current`, `stale`, `rebuilding`, `unavailable`); icons and color are redundant decoration" and "A textual list is the canonical representation for this milestone; no graph-only information is introduced". §3.7 requires matches to identify field and location "in words, not highlighting alone". §6.4 makes textual representation of every health/search relationship an explicit gate. §5.3 asserts a screen-reader linear transcript; §5.4 requires screen-reader passes over health, paging, malformed-query recovery, and rebuild cancellation. | — |
| 5D Terminal resilience | 3 | §3.3 "At terminal widths below 60 columns, each field occupies its own labeled line". §3.7 "Narrow layouts (40/60 columns) wrap values under labels without truncating paths, generations, warnings, or recovery commands" and "No operation has a timed prompt. Progress is queryable as a static snapshot". §3.5 caps interactive progress at four static stderr updates per second and emits none under non-TTY/JSON/reduced-motion/no-color. §5.3 runs the matrix at 40/60/120 columns with RTL, combining characters, and control characters. | — |
| 5E Automation | 3 | §3.4 defines `--no-input` semantics precisely ("forbids service-start, rebuild, stale-consent, or ambiguity prompts; missing consent/selection yields a typed error and required flags"), requires explicit `--query-file`/`--stdin` instead of inferred piping, and honors `--no-color` and `NO_COLOR`. §3.3 "JSON responses use stable objects independent of terminal width". §3.5 reserves stdout for the final result. §3.2 (rebuild step 1) "`--json --no-input` is fully deterministic". §5.3 validates golden JSON for version, freshness, generation, completeness, errors, sort, and cursor. | — |

**Lens average:** 3.00
**Lens pass:** Yes

---

## Feasibility Check

Verified directly against `crates/mg-vault-core/src/{lib,index,vault,xdg,registry}.rs`,
`crates/mg-vault-index/src/lib.rs`, `crates/mg-vault-index/tests/persistent_store.rs`,
`crates/mg-vault-core/tests/index.rs`, `crates/mg-vault-cli/src/main.rs`, `README.md`,
and every `Cargo.toml`.

| Check | Status | Notes |
|---|---|---|
| Types/models exist or are clearly specified | ✓ | §4.2's structs are concrete and self-consistent. `SourceFingerprint` exists (`mg-vault-core/src/vault.rs:24`). `VaultId` does not exist anywhere in the tree — the shipped index derives identity from `SourceFingerprint::of(path bytes)` in `database_path`. `BlockKind`, `ResolvedTarget`, `ResolutionState`, `PropertyType`, `TypedValue`, `TaskState`, `CivilDate`, `ExtractionStatus`, `StaleReason` are named but undefined; acceptable for a spec that labels them "Illustrative Rust contracts". |
| API/interface changes are feasible with current architecture | ✓ | Adding `mg-vault-service` and `mg-vault-client` to `Cargo.toml` `[workspace] members` is trivial and matches the existing three-crate shape. Caveat: `crates/mg-vault-cli/Cargo.toml` already declares `mg-vault-index = { path = "../mg-vault-index" }` and `main.rs` calls `PersistentIndexStore::open` directly, which §7.5 forbids and §7.2 never schedules for removal. |
| Views/screens fit current navigation pattern | ✓ | `index rebuild`, `index status`, and `search QUERY` already exist as clap subcommands (`main.rs`, `enum IndexCommand`), and `--vault`, `--json`, `--no-input`, `--no-color` already exist as global flags. §3.1 adds `index doctor`, `--wait`, and the search flag set on top of a matching pattern. |
| Dependencies are available and version-compatible | ✓ | `rusqlite = { version = "0.40.2", features = ["bundled"] }` is already a workspace dependency; FTS5 is confirmed available — `libsqlite3-sys-0.38.2/build.rs:159` passes `-DSQLITE_ENABLE_FTS5`. `serde`, `serde_json`, `thiserror`, `sha2`, `rustix` are present. `notify` and `tokio` are absent, but §4.5 correctly presents them as unpinned candidates and §8 Q1 blocks pinning on review. Note the workspace sets `unsafe_code = "forbid"` and clippy `pedantic = "deny"`, which the future IPC/watcher crates must satisfy. |
| Platform/renderer requirements are realistic | ✓ | Linux-first with `openat2`/`RESOLVE_BENEATH`/`NO_SYMLINKS` is already the shipped idiom (`mg-vault-index/src/lib.rs` `confined_database_parent`; `mg-vault-core/src/index.rs` `read_source_bytes`). Caveat: the non-Linux `read_source_bytes` currently returns an error unconditionally, so §4.6's macOS portability target is genuinely unstarted — the spec does not claim otherwise. |
| Test strategy is executable with current infrastructure | ✓ | `cargo test --workspace --all-targets --all-features` with `tempfile` dev-deps already runs 30+ index tests, several of which are near-exact analogues of §5.1 entries. Caveat: unmount/remount and kernel watcher-overflow injection (§5.2) and the 100k-note corpus generator (§4.7) need harness infrastructure that does not exist, and §8 Q3 leaves the acceptance hardware profile open. |
| Performance budget is realistic for target hardware | ✓ | 100k notes / 1m blocks in ≤ 10 min is ≈ 167 notes/sec of parse-plus-insert, comfortably achievable with batched SQLite writes. Storage ≤ 2.5× Markdown bytes is tight but consistent with §4.2's choice of contentless/external-content FTS5 rather than duplicated text. RSS ≤ 256 MiB requires streaming rather than the shipped whole-corpus-in-memory `MarkdownIndex::rebuild`, which §4.4's bounded-worker design replaces. |
| No undeclared dependency on unbuilt features | ✓ | Every external prerequisite is declared in §7.4: A1/A2 registry and XDG, A3 confinement and fingerprints, A4/F1/F2 parser contract, A7/C4 error and envelope conventions, G1–G7 resolution, I1/J4 property and task semantics, L extraction, R packaging. The A4/F parser genuinely does not exist (core has only `wikilinks()`, `title_for()`, `scan_frontmatter`, `locate_frontmatter_scalar`), but §4.5 and §7.4 both name it as a blocking dependency. |

**Feasibility verdict:** Feasible with caveats

**Caveats — stale "absent" claims (spec authored 2026-08-23; verified 2026-08-30):**

| Spec claim | Reality in tree | Where |
|---|---|---|
| §4.1 "The current workspace contains only `mg-vault-core` and `mg-vault-cli`; index/service/search code is absent." | False. `Cargo.toml` lists three members including `crates/mg-vault-index`. | `Cargo.toml`; `crates/` |
| §7.1 Absent: "index/service/client crates" | Partly false. `mg-vault-index` exists (1,245 lines). Service and client crates are correctly absent. | `crates/mg-vault-index/src/lib.rs` |
| §7.1 Absent: "SQLite schema/migrations" | False. `SCHEMA` defines `metadata`/`notes`/`diagnostics` STRICT tables at `SCHEMA_VERSION = 2`, with a `LEGACY_SCHEMA_VERSION = 1` transactional reset path and `validate_legacy_schema`. | `mg-vault-index/src/lib.rs:16–47, 780+` |
| §7.1 Absent: "freshness generations" | False. `Freshness { Empty, Current, Stale, Degraded }`, monotonic `generation`, `rebuild_with_guards` atomic publication, `verify_freshness` observational comparison, `record_freshness` with a generation CAS. | `mg-vault-index/src/lib.rs` |
| §7.1 Absent: "service/index CLI commands" | Partly false. `index rebuild`, `index status`, and `search QUERY` ship with direct-file fallback and JSON envelope. Service commands are correctly absent. | `mg-vault-cli/src/main.rs` `run_index`, `run_search` |
| §7.1 Absent: "all tests in this spec" | Partly false. Close analogues exist: `failed_publication_keeps_previous_generation_and_rows_visible` ≈ §5.1 `generation_publishes_only_after_complete_manifest`; `rebuild_refuses_source_drift_between_candidate_and_publication` ≈ `source_change_rolls_back_projection`; `recovery_failure_after_candidate_sync_preserves_corrupt_target` ≈ `migration_failure_keeps_prior_database`; `database_file_symlink_fails_closed_without_touching_its_target` and core's `symlinked_markdown_is_not_read_outside_the_vault` ≈ `safe_discovery_rejects_escape_and_special_files`. | `mg-vault-index/tests/persistent_store.rs`; `mg-vault-core/tests/index.rs` |
| §7.1 "`README.md` explicitly states that SQLite index, watcher/service, link engine, and Markdown parser are not implemented." | Half false. The current README's "Implemented foundation and index slices" lists the SQLite projection, generations, `index rebuild`/`index status`, search, and freshness as shipped; only the watcher/service, IPC, structural projections, FTS, and query grammar remain in "Milestone 2 still in progress". | `README.md` |
| §7.2 delta "Add `mg-vault-index`" | Already done; the delta list is missing the harder item of reconciling with what shipped. | `crates/mg-vault-index/` |

Nothing in the tree contradicts a *behavioral* claim of the spec — every stale
statement is confined to §4.1 and §7, and the shipped design is a strict subset of
what the spec specifies. Feasibility is therefore not compromised, but the two
design consequences are scored under 4B and 4E above.

---

## Composite Score

| Lens | Average | Weight | Weighted |
|---|---|---|---|
| Data Ownership and Format Compatibility | 3.00 | 30% | 0.900 |
| Editor and TUI Excellence | 2.60 | 25% | 0.650 |
| Knowledge Retrieval and Structure | 2.80 | 20% | 0.560 |
| Security, Reliability, and Extensibility | 2.50 | 15% | 0.375 |
| Performance and Accessibility | 3.00 | 10% | 0.300 |
| **Composite** | | | **2.79** |

**Pass conditions (from criteria.md):**
- [x] Composite ≥ 2.0 — 2.79
- [x] All lens averages ≥ 2.0 — 3.00 / 2.60 / 2.80 / 2.50 / 3.00
- [x] No criterion scores 0 — lowest score is 2 (2B, 2C, 3C, 4B, 4C, 4E)
- [x] No more than two criteria at 1 per lens — zero criteria at 1
- [x] All auto-fail rules pass — all thirteen walked individually in the Lens 3 table
- [x] Feasibility ≠ Infeasible — Feasible with caveats

**All conditions met:** Yes → PASS

---

## Remediation Brief (non-blocking — spec passed)

### Priority 1 — Must fix to pass
None. No condition in `criteria.md` is violated.

### Priority 2 — Should fix for quality

1. **§4.1 and §7.1 — correct the stale state assessment.** Rewrite §4.1's opening
   sentence: the workspace contains `mg-vault-core`, `mg-vault-index`, and
   `mg-vault-cli`. Move from §7.1's "Absent" list to an "Implemented (narrower than
   this spec)" entry: the `mg-vault-index` crate, the `metadata`/`notes`/`diagnostics`
   STRICT schema at `SCHEMA_VERSION = 2` with a legacy-v1 reset path, generation
   publication and `verify_freshness`, the `index rebuild`/`index status`/`search`
   CLI commands with direct-file fallback, and the existing test suites. Replace the
   README sentence in §7.1 with the current README's actual division of implemented
   versus in-progress work.
2. **§4.3 / §3.2 — reconcile the freshness vocabulary.** The shipped store publishes
   `empty | current | stale | degraded`; §3.2 (startup step 4) defines
   `discovering | current | catching_up | rebuilding | stale | blocked | unavailable`.
   Add an explicit mapping table, and answer the case the shipped `degraded` state
   exists for and the spec does not cover: a scan that completes but could not read
   some individual files. State whether that publishes a `current` generation with
   `complete: false`, or refuses to publish at all.
3. **§7.2 — add the CLI-to-service migration item.** `mg-vault-cli/src/main.rs`
   (`run_index`, `run_search`) opens the index database in-process, which §7.5's
   "Direct SQLite access by CLI" non-goal forbids. Add a delta item to route those
   commands through the typed client, and add a §3.2 rule for the interim period when
   a CLI process and a daemon could both hold write intent on one database.
4. **§4.3 — publish the query grammar's precedence.** §3.2 (step 5) and §4.3 both
   promise a "documented precedence" that the spec never documents. Add the operator
   precedence table (or an EBNF production set) so `a OR b AND NOT c` has one
   defined parse.
5. **§4.3 / §3.2 — define `link.to` and `link.unresolved` inside B's boundary.**
   These predicates are advertised in §4.3 while §3.2 (step 5) says resolution status
   is exposed only once G1's versioned resolver supplies it. Specify the pre-resolver
   behavior: exact-target string match only, or a typed `resolution_unavailable` error.

### Priority 3 — Consider for excellence

6. **§3.1 — enumerate the TUI consumer contract.** §3.7's closing bullet binds a future
   TUI to the same labels and cancellation; extend it with what E can rely on for
   session restore (cursor stability, generation-bound result identity).
7. **§3.4 / §4.2 — state byte-level path identity.** "paths remain UTF-8" understates
   Linux path semantics; the shipped store already uses `path BLOB PRIMARY KEY`.
   Say that path identity is the exact byte sequence and that UTF-8 is a display and
   tokenization concern only.
8. **§3.2 (watch step 2) — name the excluded control directories.** "internal
   application/runtime paths" should say `.obsidian` and `.mg-vault` explicitly, as
   `mg-vault-core/src/index.rs` `collect_markdown_paths` already does.
9. **§3.1 / §4.3 — specify the ranking function's inputs.** `--explain` is promised to
   report "ranking strategy"; define the actual score inputs and field weights so
   ranking is reproducible across implementations, not merely totally ordered.
10. **§4.3 — clarify `Hello` capability semantics.** State whether the negotiated
    `capabilities[]` are authorization-bearing, and reserve a per-connection caller
    identity field now so a future capability-gated plugin caller can be scoped and
    attributed without a protocol break.
11. **§7.4 / §7.5 — name the owner of editing durability.** Add an explicit deferral
    for autosave, persistent undo, and external-edit merge, and state the index's
    obligation toward them (a mid-edit external change must never be published as a
    complete generation).
