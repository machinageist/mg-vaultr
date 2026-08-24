# Spec: CLI and Note Operations

**Feature ID:** c-cli-note-operations
**Parent feature:** root
**Spec author agent:** Hermes Agent (CLI/note-operations subagent)
**Date:** 2026-08-23
**Iteration:** 2 — remediation against binding criteria review

---

## 1. Purpose

### 1.1 One-sentence job

Provide one safe, composable `mg-vault` command surface for selecting a vault; creating, reading, editing, appending, relocating, trashing, restoring, and explicitly purging ordinary Markdown notes; and consuming B-owned retrieval through truthful, textual, versioned CLI contracts, with equivalent guided and automation paths.

### 1.2 Why it matters

`mg-vault` is local-first and ordinary files are authoritative. The CLI is therefore not a thin convenience over an index: it is a public mutation boundary used by people, shell pipelines, the future TUI, and Quickshell. It must preserve path identity, reject ambiguous or stale decisions, expose degraded health honestly, and make destructive work previewable and recoverable. It must remain useful when the index service is absent without presenting stale derived data as current.

### 1.3 Success signal

On synthetic vaults at small and 100,000-note/1,000,000-block scales, the same command matrix passes in guided and `--no-input` modes, and fault-injection proves that ambiguity, collision, stale fingerprints, unsafe paths, interrupted transactions, and unavailable/stale index state never mutate source or produce a false success. Milestone 3 is accepted only when every command has deterministic human and versioned JSON behavior, `create/read/append` round-trip byte-exactly through pipes, every dry-run artifact can be committed without changing its identified inputs, and every graph/relationship/view result available from this CLI has a complete line-oriented textual representation.

---

## 2. User Stories

> As a writer, I want guided note creation and editing, so that I can work without memorizing every flag while retaining ordinary Markdown files.

> As an automation author, I want every guided choice to have a non-interactive flag and stable JSON result, so that scripts never hang on a prompt or parse decoration.

> As a shell user, I want note content to flow through stdin and stdout without status text, so that standard Unix tools compose safely.

> As a user with several vaults, I want explicit registration, selection, and status, so that I always know which isolated namespace a mutation targets.

> As a concurrent editor, I want stale edits, appends, moves, and renames to fail with the observed fingerprint, so that an external edit is never silently overwritten.

> As a recovering user, I want trash and restore to be reversible and purge to require explicit confirmation, so that ordinary deletion mistakes do not become permanent loss.

> As a screen-reader or no-color terminal user, I want line-oriented, color-independent status and errors with stable ordering, so that every CLI workflow remains understandable and keyboard complete.

> As a knowledge worker or automation author, I want text/title/regex/tag/property/path/relationship/task/date queries and backlink/graph traversal through one stable CLI, so that B's disposable index remains useful without ever impersonating current source.

---

## 3. UX Specification

### 3.1 Screen / view inventory

This feature adds no graphical screens, panels, mouse gestures, animation, or haptics. It introduces or completes these line-oriented CLI views:

| View | Entry point | Mode and purpose |
|---|---|---|
| Global help | `mg-vault --help`, command `--help` | Static command/flag grammar, examples, exit behavior |
| Vault inventory | `vault list`, `vault status`, `status` | Registered and selected vaults; source/index/transaction health |
| Registration flow | `vault register`, `vault remove`, `vault select` | Guided prompts only when allowed; complete flag equivalents |
| Creation flow | `note create` | Guided path/title/body choices or deterministic operands |
| Content stream | `note read` | Raw source bytes by default, or structured JSON |
| Editor handoff | `note edit` | `$VISUAL`/`$EDITOR` guided handoff or supplied replacement bytes |
| Selection handoff | `note locate`, `note open` | Resolve/display one exact path; optionally hand it to a configured editor without owning editor state |
| Append flow | `note append` | Literal byte append with explicit body source |
| Relocation preview/result | `note move`, `note rename` | Planned source/destination and link-impact limitation, then result |
| Trash inventory/action | `trash list`, `note trash`, `note restore`, `note purge` | Recovery IDs, original paths, fingerprints, timestamps, state |
| Retrieval results | `search`, `backlinks`, `links`, `graph`, `query` | B-owned derived results with provenance/freshness and complete line-oriented text/JSON |
| Diagnostics | `doctor` | Ordered checks, severity, evidence, repair guidance; never source content |

Human output goes to stdout on success and stderr on warning/error. Raw `note read` content is the only unadorned content stream. In `--json`, successful results are exactly one UTF-8 JSON object plus newline on stdout; failures are exactly one JSON error object plus newline on stderr and leave stdout empty. Prompts are written to `/dev/tty` when available, never into a redirected data stream.

### 3.2 Interaction flows

#### Vault registration, selection, removal, and status

1. `vault register [NAME] [ROOT]` obtains missing fields from prompts only when stdin and a controlling terminal permit input and `--no-input` is absent. `--name` and `--path` are equivalent explicit forms.
2. Registration canonicalizes an existing directory, validates a unique ASCII-safe registry name, checks whether the same canonical root is already registered under another name, and previews the record. A duplicate name or duplicate root is a collision; neither is silently replaced or aliased.
3. `vault select NAME` atomically persists the selected record. `--vault NAME` overrides selection for only the current command and never changes persisted selection.
4. `vault list` is sorted by name and marks selection with the literal field `selected: yes/no`, not color alone.
5. `vault remove NAME` unregisters only the registry record. It never deletes the directory, `.mg-vault`, notes, index, trash, or state. Guided mode confirms the exact name/root; `--no-input` requires `--yes`. Removing the selected record clears selection atomically. A non-empty recovery/transaction state produces a warning but does not imply deletion.
6. `status` is shorthand for `vault status` on the resolved vault. It reports canonical root, source reachability, read/write capability, pending transaction/recovery state, trash count, and index service state/freshness. It does not enumerate or hash all note content.

#### Create

1. Resolve the vault from `--vault NAME` or the persisted selection; display the name in guided mode before mutation.
2. Obtain an exact vault-relative `.md` path directly, or obtain a title and target folder then derive a preview slug. Slug rules are versioned and documented; the preview always shows the exact resulting path.
3. Obtain bytes from exactly one of `--body TEXT`, `--body-file FILE`, `--stdin`, or an allowed prompt/editor. A piped stdin is consumed only with `--stdin`; the CLI never guesses that unrelated stdin is note content.
4. Validate the path and parent chain, protected control directories, collisions, current transaction health, and the complete consumed input. `--dry-run` runs the identical planner used by commit and returns the canonical plan described in §4.3, including planned path, byte length, input digest, all preconditions, and warnings. It does not reserve the path or imply that commit cannot later conflict.
5. On collision, guided mode offers only: open/read existing note, enter another folder, enter an explicit filename, or cancel. It never overwrites. Non-interactive mode returns `collision` with the candidate path and alternatives; `--force` is not supported for create.
6. Create durably and report the path and source fingerprint only after the transaction commits.

#### Read

1. Resolve an exact vault-relative path and revalidate confinement.
2. Raw mode writes exact source bytes to stdout with no inserted or removed newline. `--output FILE` uses collision-refusing creation unless `--overwrite-output` is explicitly supplied; this affects an export destination, never the source note.
3. `--json` returns exact UTF-8 content plus path, byte length, and fingerprint. Invalid UTF-8 returns a typed error in this Markdown-only milestone; no lossy decoding is permitted.
4. Read never consults the index for source bytes.

#### Selection, editor, and workspace handoff

1. `note locate PATH` revalidates one exact source path and prints that vault-relative path (or a versioned JSON record). It never performs fuzzy resolution. `note open PATH` uses the same exact-path contract and launches the configured argv editor adapter; `--print-path` performs no launch. Both fail before launch on ambiguity, unsafe paths, or missing notes.
2. The child receives the note through one path argument after `--`; paths and content are never shell-evaluated. The handoff record contains `vault`, `path`, source fingerprint, and an opaque invocation ID. On return, the CLI re-reads and reports `unchanged`, `changed`, `missing`, or `conflict/unknown`; it never claims the editor saved durably.
3. This slice owns neither cursor/selection state, modal editing, panes, tabs, splits, preview rendering, keystroke undo, nor workspace/session persistence. D owns Unicode/grapheme-safe editing and persistent undo; E owns panes/tabs/splits/source-preview/session restore. C supplies stable, terminal-independent `read_note`, `plan_replace`, and commit-result contracts plus the exact-path handoff above; D/E may call those contracts without scraping CLI text.
4. The D/E conformance harness must prove: a selected exact path and captured fingerprint round-trip through the seam; stale editor commits retain both versions; D can persist and replay its undo journal across restart without C rewriting the buffer; and E can restore a two-tab split/source-preview session by paths while missing/moved paths become explicit unresolved entries. These tests are owned by D/E and block integrated editor/TUI claims, but their absence does not let C claim those features.
5. C's executable acceptance tests use fake editors to verify argv boundaries, exact path/fingerprint handoff, exit/crash states, and external-change recovery. This is the complete C-owned workspace scope; help and capability JSON label richer workspace features `provided_by: "editor"` or `provided_by: "tui"`, never `available: true` based on this CLI alone.

#### Retrieval, relationships, graph, and typed queries

1. C provides read-only front ends to B's versioned IPC: `search`, `backlinks PATH`, `links PATH`, `graph [PATH]`, and `query`. C does not parse the vault into an index, mutate query results, or treat B as source authority. Every result page/envelope and every human record block carries B's `index_generation`, observed `source_generation`, `freshness` (`current|stale|rebuilding|unknown`), and provenance `derived_from: "ordinary_files"`.
2. `search` supports explicit modes `--text`, `--title`, `--regex`, `--tag`, `--property KEY[=VALUE]`, and `--path`. `query` supports conjunctions over those predicates plus `--links-to`, `--linked-from`, `--task-status`, `--due-before/--due-after`, and `--created-before/--created-after` / `--modified-before/--modified-after`. Dates are ISO-8601 with an explicit timezone; regex dialect and property comparison/coercion are versioned by B and echoed in output. Unsupported operators return `unsupported_query`, never a silently broadened match.
3. `backlinks`, `links`, and `graph` preserve unresolved and ambiguous links as typed records. Ambiguous targets contain ordered candidate paths and no chosen target. `graph` emits an adjacency-list textual mode (`node PATH`, then indented `edge KIND TARGET STATE`) and equivalent JSON nodes/edges; `--from`, `--to`, `--depth`, `--limit`, and opaque `--after` pagination are deterministic. Canvas/Bases/formula results, when B exposes them, use `query --view` and MUST include a complete ordered text/table representation of every card, field, formula value/error, node, and edge; the CLI never offers a visual-only result.
4. Freshness is fail-closed. Default retrieval requires `current`; stale/rebuilding/unknown/unavailable returns exit 6 and no result rows. `--allow-stale` is an explicit read-only opt-in that returns rows labeled `freshness: stale` on every human block and envelope, includes both generations, and emits a warning; it is forbidden for mutation selection. `--refresh` requests B refresh and waits only to the configured deadline, then returns truthful state. No index response may supply note bytes or authorize a write.
5. A retrieval result may be passed to a guided exact-path chooser only while interactive. Before read or mutation, C displays the selected exact path and revalidates it directly; automation must pass an exact path and may not use rank/first-result selectors. Any generation change invalidates the chooser and requires rerun. Thus retrieval can assist selection without becoming identity or a silent conflict winner.
6. Default ordering is contract data, not locale/UI behavior: ranked search uses B's versioned integer rank descending then vault-relative path raw UTF-8 byte order and match byte offset; unranked query uses path then block byte offset; links/backlinks use source path, source offset, link kind, then target/candidate path; graph nodes use path and edges use `(from, kind, to/state)`. `--sort` accepts only a documented stable key plus explicit direction and the same path/offset tie-breakers. Floating-point score, filesystem enumeration, locale collation, and arrival order never determine output.

#### Edit

1. Read the note bytes and fingerprint.
2. Guided mode writes a private, permission-restricted temporary working copy under XDG state, launches `$VISUAL`, then `$EDITOR`, and fails with `editor_unavailable` if neither exists. The command waits and validates the editor exit status.
3. Non-interactive mode uses exactly one of `--body`, `--body-file`, or `--stdin`. `--no-input` with no source fails before creating a temporary file.
4. Show a summary (path, before/after fingerprint, byte counts) and, when requested, a unified `--diff`. `--dry-run` consumes the complete proposed bytes and emits the same canonical plan accepted by commit; it never replaces source or persists private input implicitly.
5. Commit only if the source still matches `--expected FINGERPRINT` or the fingerprint captured at editor launch. A mismatch retains the proposed bytes in recovery state and returns `conflict`; it never chooses either version. `--expected` is required for non-interactive replacement unless `--read-current` is explicitly supplied to make the command's immediate read the concurrency baseline.
6. An unchanged editor buffer exits successfully with `changed: false` and performs no write.

#### Append

1. Obtain append bytes from exactly one body source. No separator or newline is added implicitly; `--separator TEXT` is the only way to add one.
2. Validate the exact path and expected fingerprint. Non-interactive mode requires `--expected` or explicit `--read-current`.
3. Build `current || separator || append` in bounded memory or a same-directory transaction file and atomically replace only if the fingerprint remains current at commit.
4. `--dry-run` consumes the complete append input and returns the canonical plan with old/new lengths and digests without echoing private content. Empty input is a successful no-op with `changed: false`.

#### Move and rename

1. `note move SOURCE DESTINATION` changes the full vault-relative path. `note rename SOURCE NEW_FILENAME` changes only the final component in the same directory; `NEW_FILENAME` must be a single `.md` component.
2. Both commands require an exact source path and exact destination; no fuzzy/title selector is accepted for mutation. Guided title lookup, if enabled by a future resolver, may only produce a chooser and must resolve to one displayed exact path before preview.
3. Both always compute a canonical plan first. `--dry-run` returns exactly the plan schema commit accepts: source, destination, source fingerprint, parent/destination preconditions, collision state, and `links_updated: false`; commit cannot substitute either path.
4. Milestone 3 relocation does **not** rewrite inbound links. Human and JSON output state this limitation. Atomic link-aware refactoring belongs to H1; there is no claim that links remain resolved.
5. Destination collisions fail closed. There is no overwrite flag. Case-only rename uses a transaction-safe intermediate path on case-insensitive filesystems and rolls back on failure.
6. Commit requires the source fingerprint to remain unchanged and both paths to remain confined. Success is emitted only after the rename and required directory synchronization/journal commit complete.

#### Trash, restore, and purge

1. `note trash PATH` validates an exact source and mandatory concurrency baseline: non-interactive use requires `--expected FINGERPRINT` or explicit `--read-current`; guided use captures the fingerprint before preview. Dry-run emits a canonical plan containing that baseline, original path, and a deterministic proposed recovery handle; commit revalidates the source and moves note plus metadata as one recoverable transaction. Trash does not rewrite links.
2. `trash list` returns deterministic newest-first entries with ID, original path, trashed timestamp, byte length, fingerprint, and health state; it never emits content.
3. `note restore ID` resolves one exact opaque trash ID and captures the entry metadata digest/payload fingerprint as mandatory preconditions. Default destination is the recorded original path; `--to PATH` is an explicit alternate. Destination collision fails closed. Guided mode may ask for another destination or cancel; non-interactive mode returns `collision`. Dry-run and commit use the same canonical plan and exact destination.
4. `note purge ID` permanently deletes only one healthy trash entry. Its plan binds ID, metadata digest, payload fingerprint, health, and the literal confirmation requirement. Guided mode displays ID, original path, and `PERMANENT` confirmation. Under `--no-input`, `--yes` is mandatory. `--dry-run` is supported but never counts as confirmation for a later commit. Purge never accepts a note path, wildcard, prefix, or `all` in Milestone 3.
5. Interrupted trash/restore/purge is reconciled from a durable transaction journal on next startup or by `doctor --repair`. Recovery is idempotent and may restore the last known pre-operation state, but never overwrites a newer source. Orphaned payload/metadata is quarantined and reported, not discarded.

### 3.3 Layout descriptions

All views use a stable top-to-bottom hierarchy:

1. command outcome or health summary;
2. exact vault name/root and affected path(s);
3. operation-specific facts (fingerprint, recovery ID, index freshness, check code);
4. warnings and one actionable recovery command.

Tables switch to one-record-per-block at terminal widths below 60 columns. They never truncate IDs, paths, error codes, or recovery commands; long values wrap with indentation. Human status ordering is `vault`, `source`, `transactions`, `trash`, `index`. Empty states use explicit text such as `No registered vaults. Run: mg-vault vault register NAME PATH` and `Trash is empty.` JSON never changes shape based on terminal width.

Data sources are the XDG vault registry, direct filesystem validation, transaction/trash metadata, and the index service health contract. Status labels include `healthy`, `degraded`, `blocked`, and `unknown`; freshness labels include `current`, `stale`, `rebuilding`, and `unknown`. Color may decorate but never replace a literal label. Retrieval records repeat freshness/provenance after pagination and at least once per human result block, so detached lines cannot be mistaken for current source.

### 3.4 Input & gestures

- All workflows are keyboard-only and line-oriented. Mouse, touch, stylus, camera, voice, and game controller input are N/A.
- Global flags: `--vault NAME`, `--json`, `--jsonl`, `--no-input`, `--no-color`, `--quiet`, and `--verbose` (mutually exclusive where appropriate). `--jsonl` is accepted only by paginated read-only retrieval and is mutually exclusive with `--json`.
- Mutation flags shared where applicable: `--dry-run`, `--plan-out FILE`, `--from-plan FILE`, `--expected FINGERPRINT`, and explicit body-source flags. `--plan-out -` is permitted only when no raw/body stream uses stdout. `--from-plan` is mutually exclusive with target-changing operands; required body input is supplied again and digest-checked.
- Retrieval flags shared where applicable: `--allow-stale`, `--refresh`, `--limit`, `--after`, and `--deadline-ms`. Pagination tokens are opaque and generation-bound.
- `--no-input` guarantees no prompt, chooser, editor launch, confirmation, or `/dev/tty` read. Missing choices return a typed error.
- `--json` implies machine-stable formatting but does not by itself imply `--no-input`; automation SHOULD combine them. If stdout is not a TTY and a prompt would be needed, the command fails rather than contaminating a pipe.
- `--no-color` and a present `NO_COLOR` environment variable disable ANSI styling. `NO_COLOR` wins over forced auto-color; JSON and raw content never contain styling.
- `note read` can pipe bytes. `create`, `edit`, and `append` consume stdin only with `--stdin`; conflicting body sources are parse errors.
- `SIGINT` before commit cancels with no mutation. During a commit, the transaction completes or remains journaled for deterministic recovery; it never reports success merely because the signal arrived late.

### 3.5 Transitions & animation

N/A — this line-oriented CLI has no animation, progress spinner, sound, or haptic feedback. Long checks may emit static progress lines to stderr only in interactive human mode. `--quiet`, `--json`, non-TTY output, and reduced-motion environments use no dynamic updates.

### 3.6 Error states

| Code | Trigger | Presentation and recovery | Data-loss risk |
|---|---|---|---|
| `no_vault_selected` / `unknown_vault` | No resolvable vault | Name the selection/override command | No |
| `vault_unreachable` | Registered root missing/inaccessible | Status is `blocked`; repair path or unregister; no mutation | No |
| `unsafe_path` | Absolute/parent path, protected directory, non-`.md`, symlink escape | Print rejected operand and rule, not escaped canonical secrets | No |
| `ambiguous` | A guided resolver has multiple exact candidates | Ordered candidates; choose one or supply exact path | No; mutation forbidden |
| `collision` | Registry name/root or destination exists | Show exact collision and alternate actions; never overwrite | No |
| `conflict` | Expected fingerprint differs at commit | Show expected/actual digests and retained proposal location/ID | No source loss; proposal recoverable |
| `input_required` / `confirmation_required` | Required field/choice absent under no-input | List exact required flags | No |
| `invalid_input` | Conflicting body sources, invalid fingerprint/filename/ID | Identify the invalid field without echoing body content | No |
| `editor_unavailable` / `editor_failed` | No editor or nonzero exit | Preserve working copy when edits may exist; give recovery command | No |
| `transaction_incomplete` | Journal indicates interrupted operation | Block overlapping mutation; run `doctor`/automatic safe recovery | No if protocol holds |
| `restore_collision` | Original or alternate path occupied | Keep trash entry intact; choose another exact destination | No |
| `index_unavailable` / `index_stale` | Service absent, rebuilding, or behind source | Direct-file commands continue; indexed fields marked unavailable/stale | No |
| `unsupported_query` / `generation_changed` | B cannot execute the exact operator or a page/chooser token no longer matches generation | Echo safe operator/version or require rerun; no partial broadening | No |
| `plan_mismatch` / `plan_expired` | Commit operands/input digest/target/preconditions differ from serialized dry-run plan | Name mismatching non-secret field; regenerate plan | No mutation |
| `io` / `permission_denied` / `out_of_space` | Filesystem failure | State uncommitted/unknown transaction status and next doctor command | No false success; journal decides recovery |
| `broken_pipe` | Downstream closes raw/JSON output | Read exits according to platform convention; no mutation command interprets output failure as filesystem rollback | No source loss |

Messages never claim rollback, durability, freshness, or success unless the corresponding synchronization/journal state proves it. Error JSON includes `version`, `ok: false`, `error.code`, `error.message`, `error.details`, `retryable`, and, when safe, `recovery`; fields containing note body, environment secrets, editor command contents, or unrelated paths are prohibited.

### 3.7 Accessibility

- Every prompt has a visible text label, accepted value format, default, and cancel key. No timed response is required.
- Every state and selection marker has a literal text equivalent; red/green and glyphs are optional decoration only.
- Output remains understandable with ANSI stripped, under `NO_COLOR`, and in a screen reader reading linearly.
- Tables have deterministic reading order and a narrow-terminal record layout. No essential information is right-aligned or spatial-only.
- Unicode paths and content pass through without normalization or lossy conversion. Width calculation affects presentation only, never identity.
- Grapheme segmentation is used only for wrapping metadata; raw content is never segmented or rewritten. Invalid display/control bytes in metadata are escaped as `\\u{...}` with the underlying identity available in JSON, preventing bidi/control spoofing without changing paths.
- Focus order is the shell's natural prompt order. `--no-input` exposes a complete alternative to every prompt.
- Dynamic type is N/A in a terminal; wrapping is tested at 40, 60, and 120 columns. Pager use is opt-in and never occurs in JSON/raw/redirected output.
- Graph adjacency, Canvas/Bases cards, formulas, media/attachment references, and all health/status states have ordered text and JSON equivalents. Missing alt text/caption is printed literally as `description: unavailable`; filenames are not silently promoted to descriptions.

---

## 4. Implementation Specification

### 4.1 Architecture placement

- `crates/mg-vault-cli/src/main.rs` becomes a thin executable bootstrap. Parsing is split into `cli.rs`; dispatch into `commands/{vault,note,trash,status,doctor}.rs`; rendering into `output/{human,json}.rs`; prompt/editor/stdin adapters into `interaction.rs`.
- `crates/mg-vault-core` retains all filesystem authority. Extend `registry.rs` for duplicate-root detection/removal/status, `vault.rs` for byte reads and single-note use cases, and `error.rs` for stable typed causes.
- Add core transaction primitives (prefer `transaction.rs`) for append/edit replacement, move/rename, and trash/restore/purge. CLI code must not call `std::fs` for managed vault mutation.
- A future `mg-vault-app` may own shared orchestration once TUI/Quickshell arrive; until then, reusable use cases must remain terminal-independent in core rather than embedded in prompt handlers.
- The B index service is consulted only through its versioned health/IPC client. Direct-file operations never require SQLite and never read index rows as source.

### 4.2 Data model

Illustrative Rust contracts (exact module names may follow repository conventions):

```rust
/// Public path identity: normalized only lexically, never Unicode-normalized.
pub struct NotePath(PathBuf);

/// Exact bytes plus the optimistic-concurrency token observed at read time.
pub struct SourceSnapshot {
    pub path: NotePath,
    pub bytes: Vec<u8>,
    pub fingerprint: SourceFingerprint,
}

/// One fully identified proposed mutation. It is not authorization and commit revalidates it.
pub struct NoteMutationPlan {
    pub schema: u16,                  // 1
    pub operation: OperationKind,
    pub vault_name: String,
    pub vault_root_identity: VaultRootFingerprint,
    pub source: Option<NotePath>,
    pub destination: Option<NotePath>,
    pub expected: Option<SourceFingerprint>,
    pub input: Option<InputDescriptor>, // SHA-256, exact byte length, semantic role
    pub options: CanonicalOperationOptions, // separator, slug/version, destination policy, etc.
    pub trash_entry: Option<TrashEntryPrecondition>,
    pub parent_preconditions: Vec<DirectoryPrecondition>,
    pub collision: CollisionState,
    pub links_updated: bool,
    pub confirmation: ConfirmationRequirement,
    pub expires_at: SystemTime,
    pub plan_hash: PlanHash,          // domain-separated canonical fields above
}

/// Durable state used to finish or roll back an interrupted filesystem change.
pub struct TransactionRecord {
    pub version: u16,
    pub id: TransactionId,
    pub operation: OperationKind,
    pub phase: TransactionPhase,
    pub paths: Vec<NotePath>,
    pub preconditions: Vec<SourceFingerprint>,
}

/// Recoverable deletion metadata; payload bytes remain ordinary vault-local data.
pub struct TrashEntry {
    pub version: u16,
    pub id: TrashId,
    pub original_path: NotePath,
    pub trashed_at: SystemTime,
    pub byte_len: u64,
    pub fingerprint: SourceFingerprint,
    pub health: TrashHealth,
}

pub enum IndexHealth {
    Current { indexed_generation: String, source_generation: String },
    Rebuilding { started_at: SystemTime },
    Stale { indexed_generation: String, source_generation: String },
    Unavailable { reason: String },
}
```

Registry and transaction/trash schemas are explicitly versioned. Unknown newer schema versions fail closed with a doctor finding; they are never rewritten by an older binary. There is no note database migration. Paths remain public identity and no UUID/property is injected into note content. Transaction IDs and trash IDs are operational handles, not note identities.

`SourceFingerprint`, `InputDescriptor`, `TrashEntryPrecondition`, and directory preconditions are computed by deterministic core code from opened handles; callers cannot label arbitrary bytes as validated state. `CanonicalOperationOptions` is a closed tagged enum that includes every behavior-changing option, including append separator bytes/digest, slug algorithm version, restore destination policy, and relocation/link policy; unknown variants fail closed. A plan's canonical encoding has domain `mg-vault.note-plan`, schema `1`, fixed field order, explicit nulls, and no timestamps except `expires_at`. `plan_hash` binds every execution-affecting field, including vault root identity, operation, exact paths, options, expected source/trash state, complete input digest/length/role, collision observation, link-update policy, and confirmation requirement. Presentation text, warnings, transaction IDs, and clocks do not alter execution and are excluded. A plan is safe to log only after path redaction; note content is never embedded.

The version-1 JSON envelope is:

```json
{"version":1,"ok":true,"command":"note.create","data":{},"warnings":[]}
```

Errors use the contract described in §3.6. Version 1 retains the foundation stream rule as a settled compatibility decision: success envelope on stdout; error envelope on stderr; the other stream empty. JSON Lines is not used except explicit `--jsonl` retrieval. That mode is one versioned stream on stdout: one header, ordered record lines, and exactly one terminal line with either `ok:true` plus cursor/count or `ok:false` plus typed error; stderr is empty and exit is nonzero for terminal error. This explicit streaming subprotocol prevents already-consumed rows from being mistaken for a complete success and does not change the single-envelope `--json` rule. Paths are vault-relative UTF-8 strings; a path not representable as UTF-8 fails explicitly rather than becoming lossy. Raw note bytes are not wrapped unless `--json` is requested.

Operation-specific `data` objects are normative and fixture-locked as follows (`?` means present with null when inapplicable, never omitted). Output emits only documented fields in schema order. Unknown additive fields from a compatible B response are ignored after validation; unknown plan fields, required fields, enum variants, or semantic versions fail closed rather than being proxied unpredictably:

| `command` | Required `data` fields in schema order |
|---|---|
| `vault.list/status` | `vaults` or `vault`; `source_health`, `transaction_health`, `trash_count`, `index_health`, `freshness`, `source_generation?`, `index_generation?` |
| `note.create/edit/append/move/rename/trash/restore/purge` dry-run | `dry_run:true`, `plan` (complete canonical plan), `plan_hash`, `changed`; no receipt/transaction success claim |
| same mutation commands, commit | `dry_run:false`, `plan_hash`, `receipt_id`, `transaction_id`, `path?`, `source?`, `destination?`, `before_fingerprint?`, `after_fingerprint?`, `trash_id?`, `changed`, `durability` |
| `note.read` | `path`, `content`, `byte_len`, `fingerprint`; `content` is an exact JSON UTF-8 string and is present only because read was explicit |
| `note.locate/open` | `vault`, `path`, `fingerprint`, `invocation_id?`, `handoff_state`, `provided_by` |
| `trash.list` | `entries[{id,original_path,trashed_at,byte_len,fingerprint,health,metadata_digest}]`, `next?` |
| `search/query` | `query_version`, `freshness`, `source_generation`, `index_generation`, `derived_from`, `records`, `next?`, `ordering` |
| `links/backlinks/graph` | same provenance fields plus `nodes[{path,state}]`, `edges[{from,to?,kind,state,candidates}]`, `next?` |
| `doctor` | `overall`, `checks[{code,severity,status,evidence,recovery?}]`, `repair_performed` |

All arrays have documented stable ordering and always appear, including when empty. `warnings` is always an array. Digests are lowercase tagged strings (`sha256:<64 hex>`), timestamps are UTC RFC 3339 with nanoseconds, byte counts are unsigned decimal JSON integers within the documented 64-bit range, and opaque pagination/operation IDs are never paths. Full golden examples for every command, no-op, warning, and error code are public compatibility fixtures; a field removal/type change/stream move requires envelope version 2.

### 4.3 API contracts

Core interfaces are side-effect-free at planning time and transactional at commit time:

```rust
fn resolve_vault(requested: Option<&str>) -> Result<ResolvedVault>;
fn plan_create(vault: &Vault, request: CreateRequest) -> Result<NoteMutationPlan>;
fn commit_create(vault: &Vault, plan: &NoteMutationPlan, input: VerifiedInput)
    -> Result<MutationReceipt>;
fn read_note(vault: &Vault, path: &NotePath) -> Result<SourceSnapshot>;
fn plan_replace(vault: &Vault, request: ReplaceRequest) -> Result<NoteMutationPlan>;
fn commit_replace(vault: &Vault, plan: &NoteMutationPlan, input: VerifiedInput)
    -> Result<MutationReceipt>;
fn plan_relocate(vault: &Vault, request: RelocateRequest) -> Result<NoteMutationPlan>;
fn commit_relocate(vault: &Vault, plan: &NoteMutationPlan) -> Result<MutationReceipt>;
fn plan_trash(vault: &Vault, request: TrashRequest) -> Result<NoteMutationPlan>;
fn commit_trash(vault: &Vault, plan: &NoteMutationPlan) -> Result<TrashEntry>;
fn plan_restore(vault: &Vault, request: RestoreRequest) -> Result<NoteMutationPlan>;
fn commit_restore(vault: &Vault, plan: &NoteMutationPlan) -> Result<MutationReceipt>;
fn plan_purge(vault: &Vault, request: PurgeRequest) -> Result<NoteMutationPlan>;
fn commit_purge(vault: &Vault, plan: &NoteMutationPlan,
                confirmation: ConfirmedPermanentDelete) -> Result<PurgeReceipt>;
fn query_index(client: &IndexClient, request: VersionedQuery)
    -> Result<DerivedPage>;
fn recover(vault: &Vault, mode: RepairMode) -> Result<RecoveryReport>;
```

A single `plan_*` path serves guided preview, `--dry-run`, and immediate commit. There is no second planner with weaker checks. `--dry-run --plan-out FILE` serializes that exact canonical plan; `--from-plan FILE` is the only deferred-commit form. Deferred commit rejects an unknown schema/domain, invalid hash, expired plan, different selected/canonical vault root, changed path operand, changed operation/options, stale source/destination/parent/trash precondition, or input whose streamed byte length/digest differs. It does not re-plan or silently update the artifact. The CLI may spool an immediate command's input once to an owner-only unlinked/private file, compute `VerifiedInput`, and pass the same opened handle to commit; deferred input is read again into a fresh private spool and verified before the first destructive step. A digest mismatch deletes/quarantines the spool and returns `plan_mismatch` with no vault mutation.

Commit revalidates every plan precondition from opened handles immediately before mutation; plans and plan hashes are not authorization tokens. Confirmation is a separate deterministic satisfaction type constructed only after the current invocation receives the literal guided confirmation or `--yes`; a prior dry-run cannot manufacture it. All note mutations require vault write permission, per-vault coordination, current confinement, and the operation-specific capability. No plugin, AI, index response, or CLI JSON supplied by an untrusted caller can construct `VerifiedInput`, `ConfirmedPermanentDelete`, or a transaction receipt.

Plan/commit equivalence means the planned semantic target and bytes are identical, not that commit is guaranteed to succeed later: changed external state produces a typed conflict and preserves all versions. The receipt repeats `plan_hash`; tests recompute the canonical plan from the dry-run fixture and assert it equals the commit-consumed artifact byte-for-byte. Direct immediate commits internally serialize/deserialize the same schema to prevent a privileged shortcut.

The B IPC client is read-only and version-negotiated. It validates message size, schema, enums, ordering, generation-bound pagination tokens, and provenance before rendering. No network authentication or rate limiting applies; local IPC uses peer ownership checks and a vault-scoped endpoint. Trash listing and retrieval use deterministic `--limit/--after`, but pagination cannot hide unhealthy entries from `doctor`.

Exit codes are stable categories: `0` success/no-op, `2` CLI usage/input, `3` not found/selection, `4` collision/ambiguity/conflict, `5` unsafe or denied, `6` degraded dependency (only when the requested capability cannot run), `7` I/O/transaction failure, and `130` user interrupt where supported. Exact codes and JSON error codes are tested together.

### 4.4 State management

- Authoritative state: note/attachment files in the selected ordinary vault.
- Portable application state: `.mg-vault/`, limited here to trash and versioned transaction/recovery metadata. Generic note commands may not mutate `.mg-vault` or `.obsidian`; privileged core APIs alone may access their specific internal namespaces.
- Per-user state: XDG registry, editor recovery copies, runtime lock records, and disposable index/service data. Registry selection is convenience state, not vault identity.
- Derived state: SQLite/index health, relationships, queries, graph edges, views, and counts. If unavailable or stale, status says so and direct-file create/read/edit/append/move/rename/trash/restore/purge remain available unless transaction safety itself is compromised. Stale data is returned only under explicit `--allow-stale` and can never become a mutation precondition.
- Mutation serialization is per vault. A lock coordinates cooperating `mg-vault` processes, but fingerprints and commit-time path validation remain mandatory because external programs need not honor the lock.
- Transaction records are persisted and synchronized before the first destructive step, phase-updated durably, and removed only after source and relevant parent directories are synchronized. Each record binds the plan hash, opened-source fingerprint, trash metadata digest/payload fingerprint where applicable, and every affected parent. Recovery is idempotent. If the tool cannot prove whether a step committed, it reports `unknown`, blocks overlapping writes, and preserves all available versions rather than guessing.
- Recovery writes never target a path whose current opened-handle fingerprint differs from the journal's precondition. Such bytes are retained under an owner-only, collision-free recovery ID and surfaced by `doctor`; recovery requires a new explicit restore destination. Purge tombstones and payload deletion phases are journaled so payload is deleted only after the exact healthy entry is proven and confirmation is satisfied; any uncertainty quarantines rather than deletes.
- C provides durable transaction receipts and retained conflict proposals, not editor keystroke undo. Receipts are append-only audit/recovery metadata with bounded retention and owner-only access; they never contain note bodies. D's persistent undo journal must store its own proposed bytes/deltas and use C fingerprints on replay. C never advertises a receipt as undoable unless a reversible operation-specific recovery action currently exists.

### 4.5 Dependencies

Required predecessor: **A Foundation and vault authority**, especially XDG/registry, path confinement, fingerprints, atomic same-file replacement, and recoverable trash. Milestone 3 must harden or replace any foundation primitive that cannot satisfy commit-time validation and journal guarantees.

Optional/degraded predecessor for source work, required provider for retrieval: **B Index service** versioned health/query IPC. `status`, `doctor`, `search`, `query`, links/backlinks, graph, and typed views consume its truthful freshness/provenance when available; all source operations remain direct-file. C acceptance uses a contract fake even before B ships and must return `index_unavailable`, never placeholder results.

Dependent specs: **D Editor engine** and **E TUI** consume the same use cases; **H Safe note refactoring** adds atomic inbound-link updates to move/rename; **L Capture**, **M Git/sync/recovery**, **P Import/export/publishing**, and **Q Quickshell** consume stable CLI/JSON contracts.

Existing Clap, Serde/JSON, SHA-256, `thiserror`, and filesystem support may be retained. A reviewed temporary-file/editor adapter and descriptor-relative confinement library may be added. No database, network client, Markdown parser, terminal UI framework, or shell-evaluation library is required for this milestone. Editor commands are spawned as an argv program contract; note paths/content must never be interpolated into `sh -c`.

### 4.6 Platform-specific considerations

- Arch Linux/Hyprland is first support; core file behavior remains portable to Linux/macOS.
- The accepted core abstraction is a private `VaultDir` capability backed by `rustix`/libc descriptor-relative calls; all managed path components are walked from an already-opened canonical vault-root directory descriptor. Linux uses `openat2` with `RESOLVE_BENEATH|RESOLVE_NO_MAGICLINKS|RESOLVE_NO_SYMLINKS`, `O_NOFOLLOW`, opened-handle metadata/fingerprints, and descriptor-relative rename/unlink. macOS uses an `openat` component walk with `O_NOFOLLOW`, rejects links at every component, pins `(dev, ino)` for root/parents, rechecks them immediately before descriptor-relative rename/unlink, and never resolves an absolute reconstructed path. Hard-linked regular source files are rejected for mutation when link count exceeds one. This backend decision is closed, not an implementation open question.
- A platform/backend is enabled for mutation only after the traversal, in-root symlink, external symlink, ancestor swap, final-entry swap, mount crossing, and case-fold collision suite passes. Unsupported primitives yield `confinement_unavailable`/exit 5 before planning and status says `blocked`; there is no lexical/canonicalize fallback and no reduced-hardening mutation mode. Read may run only if its opened-handle confinement suite passes and is labeled separately.
- Rename is atomic only within one filesystem. Vault-relative source/destination should normally share a filesystem; mount-point crossings return `cross_device` and do not fall back to unsafe copy-delete in Milestone 3.
- Directory synchronization support differs by platform. A platform is Milestone-3 mutation-supported only if the backend proves file and affected-directory synchronization in crash tests. Otherwise mutations fail before planning with `durability_unavailable`, while read/retrieval may continue; there is no successful reduced-durability commit.
- Case sensitivity and Unicode normalization vary. Identity uses exact filesystem paths; collision checks include platform-equivalent destination names. Case-only rename uses a journaled intermediate.
- `$VISUAL`/`$EDITOR` parsing is settled as POSIX shell lexical tokenization via a reviewed `shell-words`-equivalent parser: quoting and backslash grouping only; no variable, tilde, glob, command, or process substitution and no shell process. Empty/malformed values fail; token 0 is resolved by the normal executable lookup; the literal note path is appended after `--`. `--editor PROGRAM --editor-arg ARG...` is the lossless explicit alternative.
- Feature flags may isolate index IPC and Linux hardening, but JSON behavior and direct-file safety cannot vary silently by build.

### 4.7 Performance budget

Measured on a warm local SSD reference system and reported with hardware/OS metadata:

- `--help` and argument errors: p95 ≤ 50 ms; no registry/index scan.
- Resolve registry and start a direct-file command: p95 ≤ 100 ms with 1,000 registered vaults.
- `status` direct checks: p95 ≤ 200 ms; index health IPC has a 100 ms budget then reports unavailable, never blocks source work.
- Create/read/append/edit commit for notes ≤ 1 MiB: p95 ≤ 100 ms excluding user/editor time and unavoidable `fsync` variance; durability is not weakened to meet latency.
- Move/rename/trash/restore metadata path: p95 ≤ 150 ms for same-filesystem operations.
- Streaming read/write uses ≤ 8 MiB working memory plus bounded buffers. Replacement may use a same-directory temporary file and must not require two in-memory copies of large notes.
- Startup memory target ≤ 20 MiB RSS excluding the external editor/index service. No network payload exists.
- Registry/transaction metadata is O(number of records/active operations); each trash entry adds bounded metadata plus the original bytes. Doctor surfaces trash/state storage size without reading note bodies.
- Against B's required 100,000-note/1,000,000-block fixture, first-page (`limit=50`) text/title/tag/property/path/task/date search p95 ≤ 250 ms, regex p95 ≤ 500 ms, backlinks/links p95 ≤ 250 ms, and depth-2 graph p95 ≤ 500 ms after IPC connection on the reference SSD. CLI overhead beyond measured B service time is p95 ≤ 25 ms. A 100 ms IPC connect/health deadline and caller-selected query deadline are enforced; deadline expiry returns no falsely complete page.
- Retrieval is streamed/page-bounded: CLI working memory ≤ 16 MiB plus one bounded IPC frame, default page 50, maximum page 1,000, maximum graph nodes/edges 10,000 per invocation, and JSONL uses backpressure. Oversized messages/results fail `resource_limit`; ordinary JSON refuses a result that exceeds the bound rather than partially truncating it. Stable ordering and opaque generation-bound cursors make page concatenation deterministic.
- All potentially long reads, spooling, queries, graph traversals, doctor scans, and refresh waits are cancellable and check cancellation at least every 50 ms or 1 MiB. Cancellation before commit mutates nothing; cancellation of read-only retrieval closes IPC and emits no success terminal page. This milestone does not scan 100,000 notes for ordinary exact-path commands; the scale fixture asserts zero B/index calls for each direct-file operation.

---

## 5. Test Specification

### 5.1 Unit tests

| Test | Setup and assertion |
|---|---|
| `cli_body_sources_are_exclusive` | Generate flag combinations; exactly one allowed source parses and conflicting/missing non-interactive sources fail before I/O. |
| `no_input_has_no_prompt_paths` | Inject a prompt/editor spy; every command under `--no-input` makes zero interaction calls. |
| `note_path_rejects_escape_and_controls` | Property-test absolute, `..`, empty, prefix, symlink, `.obsidian`, `.mg-vault`, non-`.md`; all fail before mutation. |
| `registry_rejects_duplicate_name_and_root` | Canonical aliases and symlinks collide deterministically; registry bytes remain unchanged. |
| `slug_preview_is_versioned_and_deterministic` | Unicode/title fixtures yield documented paths or explicit invalid input; no locale dependence. |
| `raw_read_is_byte_exact` | Include no final newline, CRLF, emoji, NUL-valid UTF-8; stdout equals source bytes. |
| `append_adds_no_implicit_separator` | Empty/nonempty files and separators produce exact concatenation. |
| `plan_has_no_side_effects` | Snapshot tree/metadata before every dry-run; byte-for-byte state remains unchanged. |
| `plan_hash_binds_executable_semantics` | Mutate each vault/operation/path/option/fingerprint/input length+digest/trash/parent/collision/link/confirmation/expiry field and assert hash changes; presentation/warning text does not. |
| `deferred_plan_rejects_substitution` | For every mutation, alter operation, vault, target, input byte, option, schema, hash, expiry, or precondition; `--from-plan` returns typed mismatch/conflict before mutation. |
| `json_envelopes_match_fixtures` | Success/error schemas, null/empty fields, warning order, and codes match version-1 goldens. |
| `json_stream_contract_is_total` | Every command/no-op/error checks exact required fields, stdout/stderr placement, one newline, no ANSI, JSONL header/terminal record, stable order, and additive-field handling. |
| `human_output_contains_literal_states` | Strip ANSI and assert selection/health/error meaning remains complete. |
| `editor_argv_never_uses_shell` | Metacharacter paths and environment values reach the intended argv without execution/interpolation. |
| `purge_requires_exact_id_and_confirmation` | Reject path, glob, prefix, `all`, and missing `--yes` under no-input. |
| `status_never_promotes_stale_index` | Every stale/rebuild/unavailable state remains explicit and no derived count is labeled current. |
| `query_operator_matrix_is_exact` | Fixture every text/title/regex/tag/property/path/link/task/date predicate, ordering/coercion version, unsupported operator, ambiguous/unresolved relationship, and generation-bound cursor. |
| `textual_equivalents_are_complete` | Linearize graph, Canvas/Bases/formula/media fixtures and assert every JSON semantic node/edge/card/field/value/error/reference/status appears in ordered plain text without color/spatial dependence. |
| `metadata_controls_are_escaped` | Combining, wide, RTL/bidi, ESC/C0/C1 and long path fixtures cannot inject terminal state; raw read remains byte-exact. |

### 5.2 Integration tests

- `guided_and_scripted_create_parity`: run the same request through injected guided answers and explicit flags; assert identical source bytes, path, and receipt fields except transaction ID/time.
- `dry_run_commit_equivalence_matrix`: for create/edit/append/move/rename/trash/restore/purge, save dry-run artifact, commit only with `--from-plan`, and assert canonical artifact/receipt plan hash and semantic target match exactly; then change each input/precondition and prove zero mutation. Immediate commit must consume the same serialize/deserialize path.
- `pipeline_round_trip`: `producer | note create --stdin`, `note read | consumer`, and `producer | note append --stdin`; assert exact bytes and clean stdout/stderr separation.
- `create_collision_never_overwrites`: precreate content, run guided cancel and non-interactive create/dry-run, and assert original bytes/fingerprint unchanged.
- `edit_external_change_preserves_versions`: modify source after editor snapshot; commit returns conflict, external source survives, proposed bytes are recoverable.
- `append_external_change_fails`: race expected fingerprint before commit; no append bytes are applied.
- `move_and_rename_collision_rollback`: inject destination collision and sync failure at each phase; assert either original path or one recoverable journal state, never two claimed authorities or missing bytes.
- `case_only_rename_recovery`: simulate interruption around intermediate path and recover idempotently.
- `trash_restore_round_trip`: preserve exact bytes/path; restore collision keeps trash payload and metadata intact.
- `trash_restore_purge_stale_preconditions`: alter source after trash preview and entry payload/metadata/health after restore/purge preview; every commit conflicts, preserves all versions, and never substitutes an ID/destination.
- `purge_fault_matrix`: inject failure before/after metadata and payload steps; doctor never loses the only known payload silently and repeated recovery converges.
- `registry_atomicity`: kill around register/select/remove persistence; old or new complete registry loads, never partial JSON.
- `degraded_without_index`: stop/omit service; exact-path note commands pass, status explicitly reports unavailable, indexed requests return degraded exit code rather than stale data.
- `retrieval_freshness_matrix`: current results render; stale/rebuilding/unknown/unavailable return no rows by default; explicit `--allow-stale` labels every block/envelope and remains unusable as mutation selection; generation changes invalidate chooser/page tokens.
- `retrieval_query_and_text_equivalence`: exercise every §3.2 predicate plus links/backlinks/depth graph and Canvas/Bases/formula/media fixtures; compare human semantics to JSON and preserve ambiguous/unresolved records without a chosen target.
- `retrieval_scale_budget`: run 100,000-note/1,000,000-block fixture, measure p50/p95/max time and RSS, verify bounded pages/backpressure/cancellation/deadline semantics, and assert exact-path commands make zero index calls.
- `unsafe_symlink_swap`: race directory entries during every mutation on supported Linux; descriptor-relative confinement prevents escape.
- `platform_confinement_gate`: on Linux and macOS backends, run traversal, in-root/external links, hard links, ancestor/final swaps, mount crossing, case-fold collision, and crash durability suites; backend cannot advertise mutation capability unless every gate passes.
- `two_process_conflict`: concurrent append/edit/move processes with one expected fingerprint yield one commit and one conflict, never silent last-writer-wins.
- `no_color_contract`: test `--no-color`, `NO_COLOR=`, `NO_COLOR=1`, TTY/non-TTY, raw and JSON; no ANSI appears.
- `broken_pipe_behavior`: close downstream read output and verify conventional exit without diagnostics contaminating stdout.

Tests use synthetic temporary vaults and fake XDG homes. Private user paths/content and network access are forbidden.

### 5.3 UI / E2E tests

There is no graphical UI. CLI E2E tests use pseudo-terminals for:

1. guided register/select/create/edit/trash/restore/purge, including cancel at each prompt;
2. collision chooser focus/order and terminal resize during a prompt;
3. Ctrl-C before preview, after preview, and during an injected commit boundary;
4. 40-column and screen-reader-linear status/doctor output;
5. external editor success, unchanged buffer, nonzero exit, crash, and source conflict;
6. non-TTY invocation proving no accidental prompt or pager;
7. locate/open fake-editor handoff plus machine-readable capability output proving editor/TUI-owned workspace features are not claimed;
8. search/query/backlinks/graph current, degraded, stale-opt-in, pagination, cancellation, and 40-column textual equivalents;
9. all commands in human, JSON, JSONL where supported, raw, `--no-input`, `--no-color`, and combined automation modes.

### 5.4 Visual / manual verification

- Run all help and status views at 40, 60, 80, and 120 columns with empty and populated registries/trash.
- Verify ANSI-on light/dark themes only as decoration, then strip ANSI and compare semantic output.
- Read output with a terminal screen reader or linearized transcript; confirm paths, states, IDs, and recovery actions are ordered and complete.
- Test Unicode paths/content, combining marks, emoji, RTL content (without allowing terminal control injection), CRLF, and no-final-newline files.
- Test `NO_COLOR`, redirected stdin/stdout/stderr, a missing index service, read-only vault, full disk/fault shim, missing editor, stale journal, and recovery.
- Run `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace --all-targets --all-features`.

---

## 6. Compliance & Safety Gate

### 6.1 Sensitive data classification

- [ ] No sensitive data involvement
- [x] Handles sensitive data — note bodies, paths, titles, vault roots, editor buffers, and trash may be private. Source stays local. Content is emitted only by an explicit read/JSON request, written only to an explicit body destination, and never included in ordinary logs, status, doctor, dry-run, error details, shell command strings, or telemetry. Temporary/editor and recovery files use owner-only permissions and are removed after verified commit; retained proposals are identified for explicit recovery. No network access occurs.
- [ ] Uses synthetic/test data only until compliance gate clears

### 6.2 Asset provenance

- [x] No third-party assets
- [ ] Uses third-party assets

Rust packages are implementation dependencies, not bundled creative/data assets; their licenses must still pass repository dependency policy.

### 6.3 Language / claims audit

- [x] No claim in this spec treats planned behavior as currently implemented; §7 separates current foundation from target state.
- [x] Human output may claim `created`, `edited`, `moved`, `trashed`, `restored`, `purged`, `durable`, or `current` only after its specific transaction/freshness proof succeeds.
- [x] “Atomic” applies to the scoped same-filesystem transaction protocol and is fault-tested; link rewrites are explicitly excluded from Milestone 3 relocation.
- [x] No regulated-domain claim is made.

### 6.4 Regulatory alignment

The template names Lens 3; all binding criteria are addressed because the CLI crosses every safety lens:

- **3A determinism:** exact paths and source reads are deterministic; retrieval ordering, pagination, provenance, and generation-bound freshness are versioned and fixture-tested.
- **3B ambiguity:** mutation requires one displayed exact path; multiple resolver candidates cannot mutate.
- **3C query depth:** C exposes B-owned text, title, regex, tag, property, path, relationship, task, date, backlink, graph, and typed-view contracts; unsupported predicates fail rather than broaden.
- **3D derived authority:** index/query/graph/view results never override source files, and stale results require explicit read-only opt-in.
- **3E scale:** exact-path operations do not scan the vault; retrieval is bounded, cancellable, paginated, and tested at 100,000 notes/1,000,000 blocks.

Binding acceptance traceability (a dependent feature is named rather than falsely claimed where C cannot own the behavior):

| Criterion | C-owned acceptance or stable seam |
|---|---|
| 1A Authority | Direct opened-file reads/mutations; B/SQLite disposable and never supplies source bytes. |
| 1B Preservation | Byte-exact read/replace/append/relocation; no Markdown/YAML parsing or reserialization. |
| 1C Identity | Exact vault-relative path is public identity; no note UUID injection. |
| 1D Coexistence | Generic commands reject `.obsidian`/`.mg-vault`; privileged state is isolated and versioned. |
| 1E Transactions | One canonical plan/commit artifact, input digest equivalence, mandatory preconditions, collision/conflict refusal, journal recovery. |
| 2A Keyboard completeness | Every CLI prompt has operands/flags and `--no-input`; no mouse-only action. |
| 2B Editing durability | C provides atomic fingerprint commits and retained proposals; D-owned persistent undo is explicitly tested across the seam. |
| 2C Workspace | C owns exact-path locate/open and capability contracts; D/E own editor panes/tabs/splits/preview/session restore and must pass named conformance tests before claims. |
| 2D Text correctness | C preserves Unicode bytes/identity and grapheme-safe display; D owns grapheme-safe cursor/structural editing through byte/fingerprint APIs. |
| 2E Degraded experience | Missing editor/TUI/index capabilities are literal and direct source read/edit-by-bytes remains available. |
| 4A Confinement | `VaultDir` descriptor-relative platform backends pass adversarial capability gates or block mutation. |
| 4B Concurrency | Every mutation, including trash/restore/purge, binds and revalidates opened-handle fingerprints/digests. |
| 4C Least privilege | Private core satisfaction types authorize commit/confirmation; index/plugins/AI cannot mint them. |
| 4D Recovery | Dry-run, durable journals, trash, retained conflict versions, quarantine, and no-newer-source-overwrite rules are fault-tested. |
| 4E Contracts | Exact v1 envelope/data/JSONL/exit/stream schemas and golden fixtures; incompatible change requires v2. |
| 4F Privacy | Owner-only state, no network/telemetry/query history, content/redaction rules, and private-index opt-in. |
| 5A Offline/local-first | All source and IPC workflows are local; missing B degrades only retrieval. |
| 5B Responsiveness | Startup/latency/RSS/frame/page/cancellation/deadline budgets cover direct and million-block retrieval paths. |
| 5C Accessible equivalents | Graph, Canvas/Bases/formula/media/status semantics have complete ordered text and JSON. |
| 5D Terminal resilience | 40-column wrapping, literal states, ANSI-free modes, control escaping, grapheme width, and no dynamic progress in reduced modes. |
| 5E Automation | Human, exact raw, JSON, JSONL, stdin/stdout/stderr, `--no-input`, `--no-color`, and `NO_COLOR` contracts are fixture-tested. |

Automatic-failure review:

- No source-content loss or partial mutation may be accepted; fault matrices verify every phase.
- No index value can overwrite source or be presented as current when stale; stale retrieval is visibly labeled per record/block and cannot select a mutation target.
- No ambiguity, collision, or conflict picks a winner.
- Note operations are byte-level and do not parse/reserialize unknown Markdown/YAML.
- No create/restore/move/rename overwrite flag exists; purge is explicit and confined to one trash ID.
- Commit-time path checks and descriptor-relative hardening reject traversal/symlink escape.
- Success follows durable commit only; recovery never overwrites newer source.
- Plugin/AI and active HTML are not executed. Graph/Canvas/Bases data is only rendered as inert text/JSON derived from B, with a complete textual equivalent and no mutation authority.

### 6.5 Security controls

- Treat paths, note bytes, titles, editor buffers, recovery copies, and diagnostics as untrusted/private input. Escape terminal control characters in human metadata output while preserving raw note output byte-for-byte.
- Use least-privilege, owner-only temporary/recovery files; reject symlink/hard-link substitution where it could cross authority; revalidate directory descriptors and fingerprints at commit.
- Spawn editors without a shell, inherit only the documented environment, and never place note content in argv, process titles, logs, or error strings.
- Generic commands cannot address `.mg-vault` or `.obsidian`; narrowly scoped privileged core methods validate the exact transaction/trash namespace and schema.
- Doctor and verbose modes redact note content, unrelated absolute paths, secrets, remotes, and environment values. No telemetry or network is introduced.
- Resource limits bound input size, JSON/error detail, transaction metadata, and repair work; malformed registry/trash/journal data fails closed rather than triggering unbounded allocation or destructive repair.
- Index IPC is accepted only from the owner-matching vault-scoped local endpoint, with schema negotiation, frame limits, deadlines, and path/generation validation. A forged index daemon can at worst propose derived read-only rows: C never accepts bytes, capabilities, filesystem paths outside the vault, or mutation authorization from it.
- Query text and returned snippets are private content. They are emitted only to the explicitly requested stdout mode, never logs/diagnostics/telemetry; no query history is persisted by C. B's index root is owner-only and excluded from exports. Fields marked secret/private by the B contract are omitted from default indexes/results and require an explicit local `--include-private` capability plus warning; this flag is unavailable to plugins/AI in this slice.

---

## 7. Gap Analysis vs. Current State

### 7.1 What exists today

Implemented in commit `bb2b723` and the current files:

- `crates/mg-vault-core/src/{xdg,registry,vault,atomic,error}.rs`: XDG paths, canonical registry and selection, relative `.md` path validation, protected-directory rejection, exact UTF-8 read, create-new collision behavior, fingerprint-checked atomic write, and vault-local trash/restore.
- `crates/mg-vault-cli/src/main.rs`: `vault register/list/select`; `note create/read/write/trash/restore`; global `--vault`, `--json`, `--no-input`, and `--no-color`; version-1 success/error JSON.
- `crates/mg-vault-core/tests/foundation.rs` and `crates/mg-vault-cli/tests/cli.rs`: foundation and initial CLI coverage.

This is a useful implemented foundation, not Milestone 3 completion. Current commands are non-interactive; body input is flag-only; no status/doctor/remove/edit/append/move/rename/purge/dry-run exists. JSON schemas are minimal, exit failures collapse to code 1, `--no-color` has no rendered-state matrix, trash lacks listing/purge, and no index health integration exists. Current canonical path checks acknowledge unresolved hostile directory-entry races. Current trash/restore rollback is narrower than the target journaled fault model.

### 7.2 Delta to spec

- Refactor CLI parsing, interaction, dispatch, and output into testable modules without moving filesystem policy into CLI.
- Add vault duplicate-root detection, remove, status, source capability checks, and truthful index-service health adapter.
- Add guided prompts and editor handoff with full `--no-input`/flag parity.
- Add explicit stdin/body-file sources and exact raw stdout behavior.
- Add plan/dry-run and typed receipts for create/edit/append/move/rename/trash/restore/purge.
- Add commit-time fingerprint checks to every mutation and recovery for rejected editor proposals.
- Add journaled single-note transactions, per-vault coordination, case-only rename, cross-device rejection, trash listing, safe restore alternate, and explicit one-ID purge.
- Add descriptor-relative confinement/race hardening or document and gate unsupported guarantees by platform.
- Expand versioned JSON/error/details/warnings schemas and stable exit categories; retain compatibility or deliberately version any break from foundation v1.
- Add doctor checks/repair reports, degraded index semantics, privacy-safe diagnostics, performance instrumentation, fixtures, PTY E2E, property tests, and fault injection.
- Add B IPC consumer commands for full query predicate coverage, links/backlinks/graph/textual typed views, generation-bound pagination, current-by-default freshness, stale opt-in labeling, and 100,000-note/1,000,000-block contract benchmarks; do not add an indexer to C.
- No note-content, SQLite, or index schema migration is allowed. Existing registry/trash metadata migrations must be versioned, atomic, backward-tested, and non-destructive.

### 7.3 Estimated scope

**L.** The command count is moderate, but trustworthy guided/scripted parity, stable public JSON, byte-exact pipes, editor conflict recovery, descriptor-relative safety, and fault-tested transaction/trash/purge recovery cross core and CLI boundaries. It should be implemented as several reviewable slices rather than one broad patch.

### 7.4 Blocking dependencies

- **A Foundation and vault authority:** implemented in part, but transaction journaling and descriptor-relative race hardening needed by this target remain incomplete.
- **B Index service:** not required for direct-file commands. Its versioned health/query contract and conformance fake are required for C retrieval acceptance; a live B implementation is required before retrieval can report `current`. Until then, commands exist but the only truthful runtime state is `unavailable/not installed` and exit 6.
- **H Safe note refactoring:** explicitly not a blocker for path-only move/rename; it is required before offering inbound-link rewrite flags or claiming link-preserving relocation.
- **P Import/export/publishing:** C6 import/export dry-run/reporting from the broad feature tree is assigned to P in the dependency roadmap and is not part of Milestone 3 note operations. This CLI establishes reusable plan/report/envelope conventions for it.

### 7.5 Non-goals

- No TUI, built-in editor engine, rendered Markdown preview, resolver heuristic, or index implementation. C does expose B-owned search/query/links/backlinks/graph/view data as inert text/JSON and exact-path selection assistance under the boundaries above.
- No Markdown/YAML parsing, formatting, title/frontmatter mutation, templates, or unknown-syntax rewrite.
- No inbound-link updates during move/rename and no structural extract/split/merge/refactor.
- No attachment move, cross-vault move, cross-filesystem copy-delete fallback, batch wildcard mutation, bulk purge, or empty-trash command.
- No import/export/publishing implementation in Milestone 3; only shared CLI contract design.
- No Git/Syncthing operation, backup, auto-checkpoint, push, network, plugin, AI, Quickshell, or `mg-calr` coupling.
- No hidden note UUID, database-authoritative content, overwrite-by-force, silent conflict winner, or automatic repair that discards the only version.
- No guarantee against malicious kernel/root processes; supported confinement is against caller input and concurrent filesystem entry manipulation within documented OS capabilities.

---

## 8. Resolved Decisions and Remaining External Dependencies

No design question in this specification blocks executable acceptance. The binding decisions are:

1. Version-1 success JSON remains on stdout and error JSON on stderr with the other stream empty (§4.2); changing streams requires version 2.
2. `VaultDir` descriptor-relative backends and fail-closed platform capability gates are fixed by §4.6; no canonical-path fallback may mutate.
3. Editor environment values use lexical shell-word tokenization without a shell, with explicit argv flags as the lossless alternative (§4.6).
4. `vault remove` remains in Milestone 3 and unregisters only the record (§3.2).
5. Freshness agreement uses B's version-1 `SourceGeneration`: a vault-scoped monotonic token advanced by B's durable watcher/change journal for every observed source mutation and paired with the generation atomically committed by the index. `current` is legal only when B proves watcher coverage has been continuous since the indexed generation, the journal has no overflow/gap, and `indexed_generation == source_generation` in one IPC snapshot. Overflow, missed coverage, startup before catch-up, unknown external filesystem semantics, or inability to read the journal yields `rebuilding` or `unknown`, never `current`. C validates equality and state but does not independently scan or mint generations.

Live B and D/E implementations remain external dependencies for, respectively, current retrieval and integrated editor/workspace claims. Their absence exercises the specified degraded/capability states rather than weakening or blocking C's direct-file safety contract.
