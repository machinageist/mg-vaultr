# Spec: Index Service and Search Foundation

**Feature ID:** b-index-service-search
**Parent feature:** root
**Spec author agent:** Hermes Agent (index-service/search subagent)
**Date:** 2026-08-23
**Iteration:** 1

---

## 1. Purpose

### 1.1 One-sentence job

Maintain a disposable, per-user projection of each registered vault so local clients can perform fast, truthful full-text, fuzzy, and structured retrieval without making the index an authority over user-owned files.

### 1.2 Why it matters

At the target scale, rescanning every Markdown file for every backlink, task, property, or search would interrupt writing. A persistent local service can watch and index registered vaults once, but it introduces correctness risks: dropped watcher events, stale rows, corrupted databases, and daemon/client version skew. This feature defines the boundary that makes those failures visible and recoverable while preserving direct-file operation. SQLite and extracted structure are derived data only; no index observation, migration, repair, or query may modify a vault source or `.obsidian`.

### 1.3 Success signal

Milestone 2 passes when deleting every index database and rebuilding from a synthetic 100,000-note/1,000,000-block vault produces the same normalized query results and source generations without changing any source-file fingerprint, overflow and crash fixtures converge through a rebuild, unavailable or behind indexes are never labeled current, and benchmark clients can continue direct source edits while indexing runs.

---

## 2. User Stories

> As a writer, I want search results to reflect known index freshness, so that I never mistake stale derived data for current source.

> As a keyboard user, I want full-text, fuzzy-title, and structured searches through stable CLI/IPC contracts, so that I can retrieve notes without scanning a large vault manually.

> As a user editing with another program or sync tool, I want watcher overflow and burst changes to trigger reconciliation, so that dropped events cannot silently leave permanent index drift.

> As an automation author, I want deterministic pagination, typed query errors, cancellation, and versioned JSON, so that scripts remain reliable across service upgrades.

> As a privacy-conscious user, I want a per-user, local-only service with vault isolation and explicit extraction provenance, so that another local account, plugin, or vault cannot query my content by accident.

> As a screen-reader or no-color terminal user, I want linear result and health views that state freshness in words, so that meaning is not encoded only by color, animation, or layout.

> As an operator recovering from corruption, I want to delete or rebuild the database without touching notes, so that the knowledge base remains usable throughout repair.

---

## 3. UX Specification

### 3.1 Screen / view inventory

This feature adds no graphical screen and does not require the future TUI. It defines line-oriented CLI views and the data contracts later TUI/Quickshell clients consume:

| View | Navigation / entry point | Status and layout |
|---|---|---|
| Index health | `mg-vault index status [--vault NAME]` and `status` index subsection | New; one record per vault with service, watcher, generation, queue, and rebuild state |
| Rebuild control | `mg-vault index rebuild [--vault NAME] [--wait]` | New; plan/result lines, static progress snapshots in interactive mode |
| Index diagnostics | `mg-vault index doctor [--vault NAME]` | New; ordered checks and non-mutating evidence; repair requires an explicit command |
| Search results | `mg-vault search QUERY [filters]` | New; ranked list or stable JSON page with freshness metadata on every response |
| Search explanation | `mg-vault search ... --explain` | New; parsed query, normalized filters, index requirement, ranking strategy, and freshness; never SQLite internals that expose unrelated data |
| Degraded notice | Any client requesting indexed functionality while unavailable/stale/rebuilding | Modification to shared status/error presentation; direct-file actions remain available |

A future TUI may render the same contracts as a full-screen search panel and health pane, but that UI is outside this milestone. No client opens SQLite directly.

### 3.2 Interaction flows

#### Service startup and registration

1. The per-user service starts on demand through the CLI or the platform user-service manager and takes a single-instance lock under XDG runtime/state paths.
2. It loads the versioned vault registry through the foundation API, canonicalizes each registered root, and assigns an internal vault key derived from the registry record rather than accepting caller-supplied database paths.
3. It creates a user-only local IPC endpoint, negotiates protocol version/capabilities, and rejects peers that are not the owning user.
4. Each vault enters `discovering`, `current`, `catching_up`, `rebuilding`, `stale`, `blocked`, or `unavailable`. Clients receive the literal state and generation evidence, not an inferred green/red indicator.
5. Registry additions/removals update watched namespaces and index catalog records. Unregistering a vault closes watchers and may garbage-collect its external index after an explicit retention policy; it never deletes vault files.

#### Watch, incremental update, overflow, and reconciliation

1. A watcher event marks affected paths dirty; it is a hint, not proof of completeness. Events are debounced/coalesced by vault and path without discarding the need to re-read source.
2. The worker validates vault containment, ignores configured index exclusions and internal application/runtime paths, fingerprints the current file, and parses only regular supported source files. It never follows a symlink outside the canonical vault.
3. One SQLite transaction replaces all projections for one observed source version: note metadata, headings, blocks, links/embeds, tags, properties, tasks, FTS rows, and extraction references. If the source fingerprint changes during parse, the transaction is discarded and the path is requeued.
4. Deletes remove derived rows only after a reconciliation scan confirms absence. Rename pairing is an optimization; delete-plus-create yields the same correct result.
5. Watcher overflow, queue loss, unmount/remount, event sequence discontinuity, service crash with an unclean queue, or an unsupported event is a completeness failure. The service immediately marks the vault `stale` or `rebuilding`, advances no current-generation claim, and schedules a bounded full manifest reconciliation.
6. Reconciliation compares a fresh confined filesystem manifest (relative path, file kind, size, modification metadata, and content fingerprint when needed) with indexed source records. It reindexes changed/missing rows and commits a new current generation only after the complete scan succeeds.
7. Repeated overflow uses exponential backoff and reduced watcher assumptions while retaining truthful state. Users can cancel an optional rebuild request, but cancellation leaves the last completed generation labeled `stale`; it does not label partial rows current.

#### Explicit rebuild and corruption recovery

1. `index rebuild` checks the target vault and reports that source is read-only to the operation. `--json --no-input` is fully deterministic.
2. The service builds a new database at a unique XDG cache/data temporary path. It never truncates the currently readable database in place.
3. It scans and parses a stable series of source observations. Files changing during the scan are retried and included in a final catch-up pass; sustained churn yields `stale` rather than false convergence.
4. Integrity checks, schema/version checks, referential checks, and fixture invariants run before publication.
5. The service atomically publishes the new database and generation metadata, then retains or removes the old database according to a bounded recovery policy. A crash before publication leaves the old generation and an incomplete candidate that is safe to delete.
6. Schema migration uses the same side-by-side rebuild for incompatible projection changes. SQLite corruption, migration failure, or parser-version mismatch marks indexed capabilities unavailable/stale and offers rebuild; direct source reads and writes continue.

#### Search

1. The client resolves one explicit/selected vault; cross-vault search requires an explicit list and keeps result namespaces distinct.
2. The parser accepts free text plus explicit structured filters. It returns a normalized abstract syntax tree or a typed error with source span; it never guesses malformed field names or ambiguous date/property types.
3. Before execution, the service snapshots the published index generation and reports `freshness`, `indexed_generation`, observation time, rebuild state, and known lag. Default search requires `current`. A caller may explicitly request `--allow-stale`, in which case every page and result set is labeled stale and includes the limitation; stale opt-in is never implicit.
4. Full-text search uses Unicode-aware tokenization over note body/headings/blocks and supports phrases, prefix terms where documented, field scope, and deterministic tie-breaking. Fuzzy search applies to title/aliases/path leaf with a bounded edit/trigram candidate strategy. Regex is supported only through a bounded/cancellable scan of candidates and is never represented as FTS.
5. Structured filters support tag, typed property, path, incoming/outgoing/unresolved link, embed, Markdown task state/text, and date/range predicates. Boolean `AND`, `OR`, `NOT`, parentheses, and field comparison have a documented precedence. G link resolution semantics remain a dependency: this milestone stores exact parsed targets and exposes unresolved/resolution status only when a versioned resolver has supplied it.
6. Results are deterministically ordered by requested sort or by score then normalized path and structural position. Cursor pagination binds to vault, normalized query, generation, sort, and last key; generation change returns `cursor_expired` rather than mixing pages.
7. Cancellation interrupts SQLite work and parser/extraction workers. Deadlines return `cancelled`/`deadline_exceeded` with no claim of complete results.
8. If the service is unavailable, the CLI may offer an explicit bounded direct-file fallback for exact path/read and optionally a clearly labeled live text scan. It must not emulate backlinks/properties/tasks from stale SQLite, and it must state which indexed capabilities are unavailable.

### 3.3 Layout descriptions

Index health uses a fixed reading order: vault name and canonical display path; service/IPC state; freshness state; indexed and observed generation evidence; watcher state; queue/rebuild progress; last successful reconciliation; then one recovery action. At terminal widths below 60 columns, each field occupies its own labeled line. Empty registry copy is `No registered vaults. Register one before indexing.` A vault with no Markdown files is `current` only after a completed empty reconciliation, with `notes: 0`.

Human search output begins with a literal banner such as `Index: current (generation …)` or `Index: STALE — explicitly allowed; results may omit recent source changes`. Each result is a linear record: rank, vault, path, heading/block location, match excerpt, matched fields, and source fingerprint observed by the index. Snippets are escaped/sanitized terminal text and never execute control sequences. Empty results say `No matches in indexed generation …`; they do not imply no source match when freshness is not current.

JSON responses use stable objects independent of terminal width. Search response metadata precedes `results` and includes protocol/schema version, query normalization, vault, freshness, generation, completeness, page cursor, elapsed time, and warnings. Health and search clients derive state solely from the IPC response, never by inspecting database files.

### 3.4 Input & gestures

- Keyboard and standard CLI arguments are complete. There are no touch, mouse, stylus, camera, voice, game-controller, sound, or haptic requirements.
- Query text may be an argument or explicitly read with `--query-file`/`--stdin`; clients do not infer use of piped stdin.
- Common controls include `--vault`, `--json`, `--no-input`, `--no-color`, `--limit`, `--cursor`, `--sort`, `--deadline`, `--allow-stale`, and `--explain`.
- `--no-input` forbids service-start, rebuild, stale-consent, or ambiguity prompts; missing consent/selection yields a typed error and required flags.
- `Ctrl-C` sends cancellation. Rebuild cancellation preserves the currently published database; search cancellation ends only that request.
- Unicode queries and paths remain UTF-8 and are not identity-normalized. Search normalization/tokenization is reported separately from exact path identity.
- JSON and redirected output never use a pager, spinner, cursor movement, or ANSI escapes. `--no-color` and `NO_COLOR` remove optional styling from human output.

### 3.5 Transitions & animation

There are no required animations. Interactive rebuild may print rate-limited static progress updates to stderr, no more than four per second, while stdout remains reserved for the final result. Non-TTY, JSON, reduced-motion, and no-color modes emit no dynamic cursor updates. State transitions are represented by literal words and timestamps/generations.

### 3.6 Error states

| Code / state | Trigger | User-visible presentation and recovery | Data-loss risk |
|---|---|---|---|
| `service_unavailable` | Socket absent, startup failed, or service exited | State indexed capabilities are unavailable; offer service status/start or truthful direct-file fallback | None; source remains usable |
| `protocol_incompatible` | No supported IPC version overlap | Show client/service versions and upgrade action; do not open DB directly | None |
| `permission_denied` | Wrong peer UID, endpoint permissions, vault unreadable | Reject before query; identify scope without exposing note content | None |
| `vault_blocked` | Root missing, unmounted, inaccessible, or fails confinement | Mark blocked; retry after mount/permission repair | None |
| `watch_overflow` | Kernel/backend reports dropped events or queue continuity is unknown | Persistent stale/rebuilding state and reconciliation progress | None; completeness unavailable |
| `index_stale` | Source observations are newer than published complete generation | Default query fails; explicit stale opt-in labels every response | None; stale data cannot appear current |
| `index_rebuilding` | Side-by-side rebuild active with no acceptable current generation | Show progress; permit only explicitly stale old generation if intact | None |
| `index_corrupt` | SQLite integrity/read failure | Quarantine external DB, mark unavailable, offer rebuild | None; never repair from DB into source |
| `migration_failed` | Candidate schema migration/rebuild fails | Keep prior DB if readable and stale-labeled; report candidate log ID | None |
| `source_changed` | Fingerprint changes during parse/extraction | Roll back that projection transaction and retry | None |
| `unsafe_path` | Traversal, protected path, disallowed file kind, or symlink escape | Reject/skip with diagnostic and vault health warning | None |
| `invalid_query` | Syntax/type/regex/date/property error | Show byte span, expected grammar, no execution | None |
| `query_too_complex` | Depth, term, regex, wildcard, or cost budget exceeded | State exact limit and simplification action | None |
| `cursor_expired` | Query, vault, sort, or generation differs from cursor | Restart from first page; never mix generations | None |
| `deadline_exceeded` / `cancelled` | Time budget or user cancellation | Mark `complete: false`; retry/refine | None |
| `extraction_failed` | Attachment tool absent, timeout, parse error, or unsafe output | Keep attachment metadata; provenance status reports failure; text absent | None; attachment unchanged |
| `resource_exhausted` | Disk full, memory/queue cap, SQLite busy beyond deadline | Pause indexing, mark stale, free space/rebuild guidance | None; no false success |

Logs and errors may contain vault IDs, relative paths where necessary, event/check codes, sizes, timings, and digests. They exclude note bodies, snippets, property values marked secret, attachment text, query literals by default, environment values, and unrelated absolute paths. Recovery never writes source, chooses a conflict winner, or advances freshness without a complete committed generation.

### 3.7 Accessibility

- Every state is written as text (`current`, `stale`, `rebuilding`, `unavailable`); icons and color are redundant decoration.
- Health fields and result records have deterministic linear focus/reading order. A textual list is the canonical representation for this milestone; no graph-only information is introduced.
- Search matches identify the field and location in words, not highlighting alone. ANSI highlighting is optional and absent under `NO_COLOR`.
- Terminal control characters from source, extractor output, paths, and snippets are escaped. Unicode display width affects layout only; paths and cursors preserve exact bytes/UTF-8 identity contracts.
- Narrow layouts (40/60 columns) wrap values under labels without truncating paths, generations, warnings, or recovery commands. Large result sets paginate explicitly and never rely on infinite scrolling.
- No operation has a timed prompt. Progress is queryable as a static snapshot, allowing screen readers to avoid repeated announcements.
- The future TUI must expose these same labels, cancellation, filters, and result metadata through keyboard focus; mouse support cannot be required.

---

## 4. Implementation Specification

### 4.1 Architecture placement

The current workspace contains only `mg-vault-core` and `mg-vault-cli`; index/service/search code is absent. Target placement:

- Add `crates/mg-vault-index` for source discovery, parsing projections, SQLite schema/repository, query parsing/planning, ranking, generations, reconciliation, extraction provenance, and benchmark fixtures. It depends on foundation types but has no source-mutation API.
- Add `crates/mg-vault-service` as a library plus a per-user `mg-vault-indexd` binary for lifecycle, watcher adapters, bounded work queues, scheduling, cancellation, and local IPC server.
- Add `crates/mg-vault-client` (or a clearly isolated module reusable by CLI/TUI) for protocol negotiation and typed requests/responses. Clients must not link to the SQLite repository module.
- Extend `mg-vault-core` only with read-only, confined vault enumeration/snapshot contracts and portable configuration access from A. Index crates cannot call foundation mutation transactions.
- Extend `mg-vault-cli` with thin `index` and `search` command adapters plus human/JSON rendering. Direct note operations remain independent of the service.
- Store runtime socket/lock under `$XDG_RUNTIME_DIR/mg-vault/` when available with a private state fallback; service state under XDG state; disposable databases and extraction cache under XDG cache/data according to documented retention. No index or socket is stored inside a vault.

The service is per OS user, not per terminal and not system-wide. One process may manage multiple registered vaults, but each vault has isolated database transactions, generations, queues, quotas, and query authorization.

### 4.2 Data model

Illustrative Rust contracts:

```rust
/// A complete, published observation boundary for one vault projection.
pub struct IndexGeneration {
    pub vault_id: VaultId,
    pub generation: u64,
    pub manifest_digest: [u8; 32],
    pub parser_version: String,
    pub schema_version: u32,
    pub completed_at: SystemTime,
}

pub enum Freshness {
    Current,
    CatchingUp { dirty_paths: u64 },
    Stale { reason: StaleReason, since: SystemTime },
    Rebuilding { scanned: u64, discovered: Option<u64> },
    Blocked { reason: String },
    Unavailable { reason: String },
}

/// One immutable source observation; source bytes never originate from SQLite.
pub struct IndexedSource {
    pub source_id: i64,
    pub vault_relative_path: PathBuf,
    pub source_fingerprint: SourceFingerprint,
    pub byte_len: u64,
    pub observed_mtime: Option<SystemTime>,
    pub indexed_generation: u64,
}

pub struct HeadingProjection {
    pub source_id: i64,
    pub ordinal: u32,
    pub level: u8,
    pub text: String,
    pub slug: String,
    pub byte_range: Range<u64>,
}

pub struct BlockProjection {
    pub source_id: i64,
    pub ordinal: u32,
    pub kind: BlockKind,
    pub explicit_block_id: Option<String>,
    pub heading_ordinal: Option<u32>,
    pub byte_range: Range<u64>,
    pub searchable_text: String,
}

pub struct LinkProjection {
    pub source_id: i64,
    pub kind: LinkKind, // Markdown, Wiki, Embed, Heading, Block, External
    pub raw_target: String,
    pub raw_alias: Option<String>,
    pub source_byte_range: Range<u64>,
    pub resolved_target: Option<ResolvedTarget>,
    pub resolution: ResolutionState,
    pub resolver_version: Option<String>,
}

pub struct PropertyProjection {
    pub source_id: i64,
    pub key: String,
    pub yaml_path: String,
    pub value_type: PropertyType,
    pub canonical_value: TypedValue,
    pub source_byte_range: Range<u64>,
}

pub struct TaskProjection {
    pub source_id: i64,
    pub block_ordinal: u32,
    pub marker: String,
    pub state: TaskState,
    pub text: String,
    pub due: Option<CivilDate>,
    pub source_byte_range: Range<u64>,
}

pub struct ExtractionProvenance {
    pub attachment_path: PathBuf,
    pub attachment_fingerprint: SourceFingerprint,
    pub extractor_id: String,
    pub extractor_version: String,
    pub config_digest: [u8; 32],
    pub extracted_at: SystemTime,
    pub status: ExtractionStatus,
    pub text_digest: Option<[u8; 32]>,
}
```

SQLite uses foreign keys, WAL mode where safe, bounded busy timeouts, prepared statements, and explicit transactions. Logical tables include `meta`, `vault_generation`, `sources`, `headings`, `blocks`, `links`, `tags`, `source_tags`, `properties`, typed property value tables/columns, `tasks`, `attachments`, `extractions`, and contentless/external-content FTS5 tables tied to projections. Every derived row references one source and observed fingerprint. Unique keys are internal only; they never become note identity or hidden YAML UUIDs.

Paths are stored as vault-relative identity plus a safe display representation; database file paths are never accepted over IPC. Byte ranges refer to the exact indexed source fingerprint. Tags preserve exact spelling and store a separate documented comparison key. YAML values preserve a typed query projection while the source/token model remains authoritative. Duplicate properties, invalid YAML, malformed links, and malformed tasks produce diagnostics/projections where safe rather than rewriting input.

Schema version and parser/resolver/extractor versions are explicit. Forward-compatible additive migrations may transact in place only if failure leaves the prior schema readable; any projection-semantic or destructive migration uses side-by-side rebuild. Index backups are conveniences, not recovery authority.

### 4.3 API contracts

IPC is a framed, length-limited, versioned local protocol (for example length-delimited JSON initially, with the framing/version fixed before implementation). Every request contains `protocol_version`, `request_id`, and an explicit vault selector. Every response echoes the request ID and includes `ok`, service version, protocol version, freshness, and generation when applicable. Unknown fields are handled according to the negotiated version; unsupported required capability fails closed.

Required operations:

```text
Hello(client_versions[], capabilities[]) -> HelloResponse
ServiceStatus() -> ServiceStatusResponse
VaultStatus(vault) -> VaultIndexStatus
RequestRebuild(vault, reason, wait) -> RebuildAccepted | RebuildResult
Cancel(request_id) -> CancelResult
Search(vault, QueryAst|query_text, SearchOptions) -> SearchPage
ExplainQuery(vault, query_text) -> ParsedQueryPlan
SubscribeStatus(vaults[]) -> stream StatusEvent
```

`SearchOptions` explicitly sets page size (default 50, maximum 500), cursor, sort, deadline, stale policy (`RequireCurrent` default or `AllowStale` explicit), snippet policy, and requested fields. `SearchPage` contains normalized query, `freshness`, immutable generation, `complete`, deterministic results, warnings, and an opaque authenticated cursor. Search never returns source bytes as authoritative; exact content/open operations re-read the file through foundation APIs and compare the indexed fingerprint.

Query grammar supports:

- unqualified terms and quoted phrases; `text:`, `title:`, and `alias:` fields;
- fuzzy title/alias terms with an explicit operator and bounded distance;
- bounded regex with explicit field scope;
- `tag:`, `path:`, `property.KEY`, `task.state`, `task.text`, `date`/typed property ranges;
- `link.to`, `link.from`, `link.unresolved`, and `embed.to` predicates;
- `AND`, `OR`, unary `NOT`, parentheses, comparisons, existence, sorting, and limits.

Typed property comparisons reject incompatible types rather than coercing silently. Dates use a documented ISO-8601/civil-date interpretation and explicit timezone for instants. Regex, fuzzy distance, nesting, boolean clauses, and result/snippet sizes have hard limits. The planner uses bound parameters only; query text cannot become SQL syntax.

The socket directory and endpoint are mode `0700`/`0600` where supported. The server validates peer credentials against the service UID, ignores caller-provided roots, resolves vault IDs through its registry, and applies per-client concurrency/cost limits. There is no TCP listener, network discovery, anonymous mode, or cross-user administration API.

### 4.4 State management

The service supervisor owns process state, protocol compatibility, registry refresh, and global resource limits. Each vault actor owns watcher health, dirty-set/queue, manifest reconciliation, one write connection/transaction scheduler, generation publication, and rebuild state. A read pool serves generation-bound queries. Request contexts own cancellation/deadline and cannot mutate source.

Watcher state is ephemeral and untrusted. Durable derived state consists of published SQLite databases and minimal external generation/rebuild metadata, all disposable. A clean shutdown checkpoints work as an optimization; startup always verifies database integrity/version and reconciles filesystem state before claiming `current`. Queue persistence cannot substitute for reconciliation after an unclean shutdown.

Offline behavior is the normal mode: all core indexing/search is local and makes no network request. Attachment extraction is disabled unless configured and uses local tools only in this milestone. Extraction cache keys include attachment fingerprint, extractor/version, and configuration. Failed or stale extraction is visibly excluded or provenance-labeled; original attachments are opened read-only and never modified.

### 4.5 Dependencies

Proposed Rust dependencies, subject to license/security review and version pinning:

- SQLite binding with bundled/system policy documented, FTS5 enabled, migration/integrity support, and cancellation hooks (`rusqlite` is a candidate).
- Cross-platform filesystem notification adapter (`notify` is a candidate), treated as hints with mandatory reconciliation.
- Local async/runtime and IPC primitives only if needed (`tokio` is a candidate); avoid network-facing features.
- Existing Serde/JSON, SHA-256, typed error, and foundation crates.
- Unicode segmentation/case-fold/tokenization and bounded fuzzy matching libraries with fixture-backed behavior.
- Markdown/YAML structural parser from the token-preserving A4/F compatibility work; the indexer must not invent a second incompatible dialect.
- Optional local extractor adapters are external executables with explicit allowlists, timeouts, output caps, environment scrubbing, and provenance; no extractor is required for core Markdown acceptance.

Infrastructure changes are a user-level service definition for the first Arch/systemd environment and XDG runtime/cache/state directories. There is no server, cloud database, CDN, telemetry endpoint, or proprietary sync dependency.

### 4.6 Platform-specific considerations

Linux/Arch with systemd user services and native filesystem notifications is first supported. Linux peer credentials and endpoint permissions enforce the local identity boundary. macOS remains a core portability target using its local socket/watcher equivalents; backend-specific event semantics must pass the same overflow/reconciliation suite before support is claimed.

Network filesystems, removable drives, Syncthing trees, and Git checkouts may have coarse timestamps, event bursts, atomic rename patterns, or unreliable watchers; correctness therefore depends on fingerprints and reconciliation, not timestamps or rename cookies. Case sensitivity and Unicode normalization behavior follow actual path identity and are fixture-tested per platform.

Service autostart is optional and never required for direct-file operation. Protocol rolling upgrade supports at least current and immediately previous compatible client versions; incompatible upgrades fail with `protocol_incompatible`. Database migrations never gate source access. Feature flags may gate attachment extraction and experimental tokenizer/fuzzy backends, but freshness semantics and source non-mutation are not feature-gated.

### 4.7 Performance budget

Acceptance hardware and corpus generators must be recorded with benchmark results; claims are invalid without corpus shape, warm/cold state, and percentile evidence. Required baseline corpus is at least 100,000 Markdown notes, 1,000,000 blocks, 2,000,000 links/tags/property/task projections combined, and representative Unicode/path/frontmatter sizes.

- **Editing isolation:** source mutation commands do not wait for indexing. Watcher enqueue acknowledgement target is p95 ≤ 10 ms; indexing runs in bounded workers with low-priority/background scheduling. Under a full rebuild, p99 added latency to direct `note read/write` foundation benchmarks is ≤ 20 ms and no write is blocked on SQLite/service availability.
- **Freshness:** after a quiescent single-note change, current generation target is p95 ≤ 500 ms and p99 ≤ 2 s on acceptance hardware. Bursts may exceed this but must expose queue/lag and converge.
- **Search latency:** warm-index p95 ≤ 100 ms for common full-text/fuzzy/structured first pages and p99 ≤ 250 ms; cold p95 ≤ 750 ms. Expensive regex/compound queries have a default 2 s deadline and are cancellable.
- **Service startup:** IPC health available p95 ≤ 150 ms when DB needs no recovery; startup never claims current before verification. Initial discovery/reconciliation proceeds in background.
- **Rebuild:** complete 100k-note/1m-block rebuild target ≤ 10 minutes on acceptance hardware, with static progress, cancellation, bounded retries, and no source mutation. Performance regression gate is ±15% from an accepted baseline unless justified.
- **Memory:** steady-state service RSS target ≤ 256 MiB for the baseline corpus; hard configurable ceiling 512 MiB. Work queues, parser buffers, snippets, regex candidates, and extraction output are bounded; backpressure marks measurable lag rather than growing without limit.
- **Storage:** index plus FTS/extraction metadata target ≤ 2.5× indexed Markdown bytes excluding optional extracted attachment text; optional extraction reports its separate bytes. Temporary rebuild may require old + new DB + 20% headroom and fails before start if unavailable.
- **CPU/I/O:** worker count and I/O rate are configurable and default to leaving one logical core available. SQLite writes are batched without holding query locks for an entire vault scan. WAL/checkpoints have size/time bounds.
- **Network:** zero bytes for service, search, and Markdown indexing. Any later network extractor is a separate capability-gated feature, not Milestone 2.

Benchmarks report throughput, p50/p95/p99 latency, peak RSS, CPU, bytes read/written, DB size, queue lag, cancellation time, and direct-edit latency during rebuild.

---

## 5. Test Specification

### 5.1 Unit tests

| Test | Setup and assertion | Edge covered |
|---|---|---|
| `generation_publishes_only_after_complete_manifest` | Inject failure before final transaction; prior generation remains published and freshness is stale | Partial rebuild cannot appear current |
| `source_change_rolls_back_projection` | Change fingerprint between parse and commit; no mixed rows commit and path requeues | Concurrent external edit |
| `watch_events_coalesce_without_losing_dirty_state` | Generate create/write/rename/delete sequences | Event storms and rename optimization |
| `overflow_forces_stale_and_reconciliation` | Inject overflow marker | Dropped events never silently converge |
| `query_parser_precedence_and_spans` | Fixture Boolean/field syntax and malformed input | Deterministic grammar/actionable errors |
| `typed_property_comparisons_reject_coercion` | Compare dates/numbers/bools/strings with wrong operator/type | No silent query guess |
| `ranking_has_total_order` | Equal-score Unicode titles/paths/positions | Stable pagination |
| `cursor_binds_generation_and_query` | Change generation/query/sort | No cross-generation mixed page |
| `fts_fuzzy_regex_are_bounded_and_cancellable` | Adversarial long terms/patterns | Resource exhaustion/ReDoS |
| `structural_ranges_match_source_fingerprint` | Parse heading/block/link/tag/property/task fixtures | Exact provenance |
| `extraction_cache_key_includes_all_provenance` | Change attachment/tool/config independently | No stale extracted text claim |
| `terminal_text_escapes_controls` | Source/snippet/path with ESC, bidi, newlines | Terminal injection |
| `safe_discovery_rejects_escape_and_special_files` | Symlinks, FIFO, socket, traversal | Confinement and hangs |
| `migration_failure_keeps_prior_database` | Fault every migration/publication phase | Recoverability |

### 5.2 Integration tests

- Start one service for one UID, negotiate supported/unsupported protocol versions, verify socket modes and reject a peer-credential mismatch fixture.
- Register two vaults with identical note names; prove searches, rows, generations, queues, and cursors never cross namespaces.
- Build a compatibility vault containing CommonMark/GFM and documented Obsidian headings, block IDs, links/aliases, embeds, tags, typed/unknown YAML, duplicate/malformed properties, and task markers; compare projections and byte ranges to approved fixtures.
- Index, snapshot all source fingerprints/bytes, delete the SQLite/index directory, rebuild, then assert byte-for-byte source non-mutation and equivalent normalized result sets/generations.
- Inject watcher overflow, service kill, queue-file truncation, unmount/remount, rapid rename/delete/create, and changes during rebuild; assert stale state appears before any query and reconciliation converges after quiescence.
- Corrupt pages, WAL, schema metadata, and candidate migrations; assert direct-file CLI remains usable, old readable generation is truthfully labeled, and side-by-side rebuild recovers.
- Run simultaneous queries, cancellation, deadlines, and incremental writes; assert transaction isolation, cursor expiry, bounded cancellation, and no SQLite busy error blocks editing.
- Test optional extraction with a fake local tool for success, timeout, oversized output, crash, malicious control text, fingerprint change, and missing executable; assert provenance and original attachment digest.
- Test secret-index exclusion configuration: excluded properties/paths never enter ordinary tables, FTS, snippets, logs, diagnostics, or extraction jobs; configuration change forces a generation rebuild.
- Verify unregistering/re-registering and service restart do not mutate/delete vault content or accidentally reuse another vault's index.

### 5.3 UI / E2E tests

CLI E2E scenarios cover human and JSON output for status, rebuild, search, stale opt-in, invalid query, cancellation, and unavailable service. Golden JSON fixtures validate version, freshness/generation/completeness fields, stable errors, deterministic sort, and cursor behavior without depending on volatile timestamps.

Run at 40-, 60-, and 120-column terminal widths with current/stale/rebuilding/unavailable, empty and populated results, long Unicode paths, combining characters, RTL text, control characters, `NO_COLOR`, `--no-color`, non-TTY redirection, and a screen-reader linear transcript. Assert no essential state depends on color or repeated animated updates and no path/generation/recovery command is silently truncated.

A degraded-mode E2E kills the service between pages and edits source directly: subsequent indexed requests fail or are explicitly stale, raw note read/write succeeds, and no client opens the DB as fallback. A recovery E2E restarts/rebuilds and verifies the health view reaches current only after reconciliation.

### 5.4 Visual / manual verification

- Inspect status/search transcripts in default, no-color, high-contrast terminal themes, and ANSI-stripped form.
- Verify empty, one-result, 500-result, stale, rebuilding, corrupt, blocked, and protocol-incompatible presentations.
- Resize live terminals from 120 to 40 columns and confirm labeled order/wrapping; test text zoom through terminal font settings.
- Use a screen reader on health, result paging, malformed-query recovery, and rebuild cancellation.
- Observe system service start/stop/restart and ensure direct note operations remain available.
- Monitor process/socket/database permissions, logs, CPU, memory, queue, WAL growth, and source file timestamps/digests during a rebuild.

---

## 6. Compliance & Safety Gate

### 6.1 Sensitive data classification

- [ ] No sensitive data involvement
- [x] Handles sensitive data — note text, paths, properties, tasks, links, snippets, and locally extracted attachment text may be private. Databases/endpoints use owner-only permissions; IPC verifies the owner; logs/diagnostics omit content and secret fields; exclusions are applied before ordinary tables/FTS/extraction; all processing is local/offline; deletion of derived caches is supported.
- [x] Uses synthetic/test data only until compliance gate clears — performance, corruption, extractor, and adversarial security suites use generated fixtures; no personal vault is required for acceptance.

Threat boundaries include an untrusted local IPC caller, malformed vault files, symlinks/special files, hostile query text, malicious extractor output, corrupted SQLite state, and resource exhaustion. SQL is parameterized, frames/query complexity/output are bounded, paths are resolved through foundation confinement, and no index component exposes a source-write method. Plugins/AI have no ambient IPC capability; future access requires their own explicit capability gate and scoped vault/query contract.

### 6.2 Asset provenance

- [x] No third-party assets
- [ ] Uses third-party assets — N/A; libraries and optional executables are dependencies, not bundled content assets, and require normal license/SBOM review before selection.

### 6.3 Language / claims audit

- [x] Make claims not supported by evidence? No; latency/scale values are acceptance budgets and must not be marketed as achieved until benchmark artifacts pass.
- [x] Promise capabilities not yet built? No; all behavior is explicitly target-state, and current absence is recorded in Section 7.
- [x] Use language restricted by domain regulations? No.

### 6.4 Regulatory alignment

This is not a regulated-domain feature; the template's named Lens 3 is treated as the binding retrieval gate:

- **3A Determinism:** all projections bind to source fingerprints and complete generations; search pagination and ordering bind to one generation; backlinks/graph later consume these same relationships.
- **3B Ambiguity:** this milestone stores raw targets and resolution status; it never silently chooses an ambiguous link. G1 owns user-facing link resolution semantics.
- **3C Query depth:** full text, fuzzy titles/aliases, bounded regex, tags, typed properties, paths, incoming/outgoing/unresolved links, embeds, tasks, and date ranges have explicit grammar/contracts.
- **3D Derived authority:** SQLite, FTS, properties, extracted text, and later graph/Bases consumers are disposable views. Exact content and mutations always return to ordinary source through foundation transactions.
- **3E Scale:** Section 4.7 and the synthetic benchmark suite cover 100,000 notes/1,000,000 blocks, latency, memory, storage, startup, cancellation, and editing isolation.

Auto-fail gates are explicit: no source mutation/loss or index authority inversion; no stale result presented as current; no silent ambiguity; no traversal/symlink escape; no secret/capability bypass; no false rebuild success; and every health/search relationship has a textual representation.

---

## 7. Gap Analysis vs. Current State

### 7.1 What exists today

**Implemented:** the repository has a Rust 2024 workspace with `mg-vault-core` and `mg-vault-cli`; foundation code provides XDG paths, registry/vault confinement, source fingerprints, atomic replacement, trash/restore, and basic human/versioned JSON output. `README.md` explicitly states that SQLite index, watcher/service, link engine, and Markdown parser are not implemented.

**Planned/specification only:** token-preserving A4 parsing and broader transaction/portable-configuration contracts are described by the foundation/product plan but are not complete dependencies for this feature.

**Absent:** index/service/client crates, daemon binary, local IPC protocol, watcher/reconciliation, SQLite schema/migrations, structural projections, extraction provenance, query grammar/planner/ranking, freshness generations, service/index CLI commands, benchmark harness, and all tests in this spec.

### 7.2 Delta to spec

- Add `mg-vault-index`, `mg-vault-service`, and reusable typed IPC client modules/crates plus the `mg-vault-indexd` binary.
- Extend the workspace manifest and CLI command/output modules without coupling direct file commands to the service.
- Add confined read-only discovery/snapshot integration with foundation and portable index/exclusion configuration.
- Define and version IPC frames, handshake, status, rebuild, query, cursor, cancellation, error, and JSON fixture schemas.
- Implement watcher adapters, bounded dirty queues, overflow detection, manifest reconciliation, side-by-side rebuild/publication, and startup integrity checks.
- Create SQLite schema/migration policy and projection parser for sources, headings, blocks, links/embeds, tags, typed properties, tasks, diagnostics, FTS, attachments, and extraction provenance.
- Implement bounded full-text, fuzzy-title, regex, and structured query parser/planner/ranking with generation-bound pagination.
- Add systemd user-service packaging hooks, XDG permission tests, fixture corpora, fault injection, security tests, and 100k-note/1m-block benchmark generation/reporting.
- Document freshness/degraded behavior, query grammar, limits, storage/privacy controls, rebuild operations, and platform backend support.

No production file is modified by this specification work.

### 7.3 Estimated scope

**XL.** The feature combines a long-running multi-vault process, secure/versioned IPC, unreliable filesystem event handling, transactional SQLite/FTS projection, a structural Markdown/YAML model, a typed query language, deterministic ranking/pagination, corruption/rebuild recovery, privacy controls, platform watcher differences, and a large performance/fault-injection matrix. It should be delivered as independently gated B1–B8 increments behind the same freshness contract rather than as one unreviewable patch.

### 7.4 Blocking dependencies

- **A1/A2:** versioned registry, selected vault, XDG locations, and portable `.mg-vault` exclusion/config access.
- **A3:** canonical vault confinement, path identity, safe file kinds/symlinks, and source fingerprints.
- **A4/F1/F2 parser contract:** token/byte-range-preserving CommonMark/GFM/Obsidian/YAML semantics shared by indexing and later rendering; the index must not define incompatible syntax.
- **A7/C4 contracts:** stable error codes and human/JSON envelope conventions.
- **G1–G7:** final link resolution, unlinked mentions, saved-query persistence, user-facing search integration, and graph semantics consume this foundation; B stores/query-projects relationships but does not silently resolve ambiguity.
- **I1/J4:** canonical typed-property/schema and Markdown-task semantics refine the initial projections without making them authoritative.
- **L extraction:** production PDF/OCR extractor selection and sandbox/capability policy; B6 only defines provenance/cache/failure contracts and a fake local adapter.
- **R packaging:** systemd user unit, clean-machine tests, release migration/SBOM policy.

### 7.5 Non-goals

- Editing, formatting, repairing, renaming, deleting, or injecting identifiers into Markdown, YAML, attachments, `.obsidian`, or `.mg-vault` source.
- Making SQLite, FTS rows, watcher order, timestamps, or extractor text authoritative.
- Direct SQLite access by CLI, TUI, Quickshell, plugins, AI, or `mg-calr`.
- Cross-user/system-wide service, TCP/HTTP listener, remote/cloud search, synchronization, telemetry, or proprietary server.
- Real-time multi-user collaboration or conflict winner selection.
- Final backlink chooser/UI, unlinked-mention semantics, graph rendering, saved-query files, Bases/formulas, task mutation/toggling, note refactors, or `mg-calr` promotion.
- Shipping a PDF/OCR engine or network extractor in Milestone 2; only provenance and safe adapter boundaries are defined.
- Exact source reads from snippets or projections; clients reopen and fingerprint source.
- Claiming full Obsidian compatibility without approved, versioned fixtures.

---

## 8. Open Questions

- **Q1:** Which SQLite and watcher crates satisfy the final license, FTS5, cancellation, peer-platform, and maintenance review? — blocks dependency pinning in Sections 4.5–4.6, not the behavioral contract.
- **Q2:** Should disposable database bytes live under XDG cache or XDG data with cache-like deletion semantics, while generation/rebuild metadata remains under state? — blocks exact paths in Section 4.1; no path may be inside a vault.
- **Q3:** What acceptance hardware profile and generated corpus distribution become the published performance baseline? — blocks numeric pass/fail evidence for Section 4.7, not the minimum 100k-note/1m-block scale.
- **Q4:** How long may the prior published DB be retained after side-by-side rebuild, within privacy and disk budgets? — blocks retention defaults in the rebuild flow; manual deletion must remain safe.
