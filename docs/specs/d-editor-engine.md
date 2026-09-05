# Spec: Editor Engine

**Feature ID:** d-editor-engine
**Parent feature:** root
**Spec author agent:** Hermes editor-engine spec subagent
**Date:** 2026-08-23
**Iteration:** 2 — remediation after binding-criteria review

---

## 1. Purpose

### 1.1 One-sentence job

Provide a Unicode-correct, crash-recoverable editing engine with a bounded Vim-style command language so a user can edit authoritative Markdown efficiently without silent loss when the process crashes, another program changes the file, or an external editor is used.

### 1.2 Why it matters

`mg-vault` is a keyboard-driven knowledge system whose ordinary files remain authoritative. The editor therefore has to combine modal precision with stronger file-concurrency and recovery guarantees than a memory-only text area. It must keep cursor and selection operations on Unicode grapheme boundaries, preserve bytes outside intentional edits, expose deterministic commands to the future TUI, and treat Tree-sitter, spellcheck, clipboards, and external tools as fallible helpers rather than authorities.

This specification covers D1–D11 and Milestone 4: buffer/graphemes; modes; operators, motions, text objects, counts, and repeat; registers, clipboard/OSC52, macros, marks, and jumps; search and a bounded command line; undo/recovery journals; autosave; external-editor handoff; external-change merge; Tree-sitter structure; and diagnostics.

### 1.3 Success signal

The Milestone 4 acceptance suite passes command-level grammar fixtures, randomized grapheme/edit invariants, crash-recovery fault injection, explicit external-import acceptance and merge fixtures, source-authority and token-preservation fixtures, headless automation contract tests, accessibility/security tests, and performance gates. An edit is acknowledged only after its recovery record is durably synced; persistent undo refuses a source-hash mismatch without changing source; and atomic autosave never reports success unless the authoritative ordinary file durably contains the expected bytes.

---

## 2. User Stories

> As a keyboard-first writer, I want composable modes, operators, motions, text objects, counts, registers, and repeat, so that editing is fast and predictable without claiming complete Vim compatibility.

> As a multilingual writer, I want cursor movement, deletion, selection, search results, and undo to respect extended grapheme clusters, so that combining marks, emoji sequences, and non-Latin text are not split accidentally.

> As a writer recovering from a crash, I want a journaled dirty buffer offered for recovery without overwriting a newer disk file, so that neither my unsaved work nor an external edit disappears.

> As a user who also edits in Neovim, I want a note, selection, or fenced code block to round-trip through `$EDITOR`, so that changed output is previewed and requires my explicit acceptance before it can alter the buffer, an accepted result is one undo transaction, and concurrent changes enter merge rather than being overwritten.

> As a screen-reader user, I want every engine state and conflict represented as ordered text with explicit mode, cursor, selection, diagnostic, and merge status, so that no operation depends on color, pointer input, or spatial presentation.

> As a user on a remote terminal, I want copy to use an explicitly enabled OSC52 fallback with size limits and visible failure, so that clipboard support does not emit unbounded or surprising terminal control sequences.

> As a user with a very large or temporarily malformed note, I want plain source editing to remain available when parsing or diagnostics are slow or fail, so that derived structure never blocks access to authoritative text.

---

## 3. UX Specification

### 3.1 Screen / view inventory

This is a headless engine slice and introduces no independently rendered screen. It exposes view models and actions consumed by **E. TUI workspace**:

- **Editor buffer view model** — new; text viewport input, cursor/selection, mode, dirty state, save/recovery state, pending key sequence, register/macro status, search state, fold ranges, and diagnostics.
- **Command-line view model** — new; a single-line prompt for `/`, `?`, and `:` with history and completion candidates. E owns placement and rendering.
- **Merge session view model** — new; ordered base/local/external hunks, resolution state, and a textual three-way representation. E owns pane layout.
- **Recovery offer view model** — new; source path, journal timestamp, base/current hashes, recoverable edit count, and safe actions: inspect, recover to dirty buffer, export copy, discard journal.
- **Diagnostics/outline projection** — new; ordered ranges and messages derived from a specific buffer revision.

No graph, preview, tab/split layout, theme, command palette, or workspace restoration UI is defined here; those belong to E and later feature branches.

This engine does define renderer-independent, versioned textual and JSON projections for all of the views above. They are the single accessibility and automation seam; E may lay them out but may not weaken their status labels, ordering, source revision, freshness, or available actions.

### 3.2 Interaction flows

#### Primary built-in editing flow

1. The app requests a buffer from an authoritative path through core path authority and receives source bytes plus `SourceFingerprint`.
2. The engine validates UTF-8. Valid text opens with a grapheme-safe cursor at the first grapheme. Invalid UTF-8 fails as `UnsupportedEncoding` and offers byte-preserving external open; it is never lossy-decoded and saved.
3. Normal mode accepts a documented command sequence. The parser displays pending prefixes/count/operator/register state to its client.
4. Insert and visual edits produce typed transactions. A completed insert session, operator command, paste, accepted external-editor import, merge resolution, or diagnostic quick-fix is one undo change unless explicitly split by a checkpoint. Undo grouping is independent of recovery durability: every successful mutating `dispatch`/`execute` call durably appends the resulting journal fragment before returning an acknowledgment event.
5. After the configured idle delay, autosave compares the original/latest accepted fingerprint and uses the foundation atomic replacement API. Success updates the buffer base hash and save point.
6. If persistence fails, the buffer remains dirty, the recovery journal remains available, and the engine reports a non-success state with retry/export actions.

#### Crash recovery flow

1. On open, the engine checks the XDG-state recovery registry for a journal keyed by canonical vault identity and relative path.
2. A journal whose checksums and sequence are valid is replayed into an isolated candidate buffer from its recorded base snapshot/hash.
3. If current source equals the recorded base or last durable save hash, the user may recover the candidate as a dirty buffer.
4. If current source differs, the engine creates a three-way merge using recorded base, recovered local, and current source. Recovery never writes automatically.
5. Corrupt/truncated records are replayed only through the last valid committed transaction; corruption is reported and original journal bytes are retained for export/diagnosis.

#### External editor round-trip

1. The user invokes external edit for a note, selection, or fenced code body.
2. The engine writes a private temporary file under XDG runtime state (or XDG state if runtime storage is unavailable), records source/buffer revision and selected-span anchors, and invokes the configured editor as an argument vector without a shell.
3. Nonzero exit, signal, launch failure, timeout/cancel, missing temp file, invalid UTF-8, or over-limit output leaves the buffer unchanged and keeps the temp artifact available for explicit recovery.
4. A zero exit with byte-identical output is reported as `ExternalEditUnchanged` and may close without confirmation because it performs no import or mutation. Any changed output becomes an immutable `ExternalImportCandidate` containing scope, base/current fingerprints, a textual diff, affected byte/grapheme ranges, and the retained temp path. Merely returning from the editor never mutates the buffer.
5. The user or an authorized headless caller must invoke `accept_external(candidate_id)` explicitly. If the buffer and authoritative source still match the candidate bases, acceptance imports the changed bytes as one durably journaled undo transaction. Selection/code-body handoff replaces only its anchored span; delimiters remain owned by the original buffer. Reject/cancel leaves the buffer byte-identical.
6. If the buffer or source changed before acceptance, acceptance is refused and the engine creates a three-way merge candidate. No side silently wins; even a conflict-free merge remains a pending preview requiring explicit `accept_merge` before buffer mutation.
7. The temp file is securely removed only after accepted import or explicit discard; failed cleanup is reported without claiming deletion.

#### External filesystem change flow

1. Watcher events are hints. Before reload/save, the engine reads and fingerprints the source through core authority.
2. If source bytes equal the known hash, duplicate/coalesced events are ignored.
3. A clean buffer auto-reloads and creates a non-undoable base transition plus a jump/history notice; cursor is restored by grapheme-safe context anchoring where possible.
4. A dirty buffer opens a merge session from last common accepted source (`base`), current buffer (`local`), and newly read file (`external`).
5. Non-overlapping changes may auto-merge but are previewed as a pending dirty result. Overlaps require explicit per-hunk or whole-file decisions. Base, local, and external remain exportable.
6. Save is blocked until all conflict hunks are resolved. The final save still requires the current external fingerprint, so another change restarts merge rather than overwriting.

There are no haptic or sound cues. Engine state changes are event-driven, not animated; E may render them statically.

### 3.3 Layout descriptions

N/A — this slice has no renderer. It supplies ordered, serializable projections so E can build source, command-line, recovery, and merge views. Empty projections are explicit: no selection, no pending command, no search matches, no diagnostics, no folds, or no conflicts. Status copy must distinguish `clean`, `dirty`, `saving`, `saved`, `save failed`, `external change`, `merge required`, and `recovery available` without color.

### 3.4 Input & gestures

The engine accepts normalized key events and text/IME commits; terminal escape decoding belongs to E. Mouse, touch, voice, and stylus are optional client translations to the same actions.

#### Bounded Vim-style grammar

The grammar is intentionally documented and finite. Unsupported Vim/Ex commands return `UnsupportedCommand` without mutation. Default commands are:

- **Modes:** Normal; Insert (`i`, `a`, `I`, `A`, `o`, `O`, `R`); Visual character (`v`), line (`V`), and block (`Ctrl-v`); Escape returns to Normal and commits the current insertion transaction.
- **Operators:** delete `d`, change `c`, yank `y`, indent `>`, outdent `<`, format `=`, lowercase `gu`, uppercase `gU`, and swap case `g~`. Doubled operators act linewise (`dd`, `cc`, `yy`, `>>`, `<<`, `==`, `guu`, `gUU`, `g~~`). Format is a deterministic whitespace/indent action only; Markdown reflow is a later explicit command.
- **Motions:** grapheme left/right `h`/`l`; display-independent logical line up/down `j`/`k`; word/WORD start/end/back `w`, `W`, `e`, `E`, `b`, `B`; line positions `0`, `^`, `$`; document `gg`, `G`; find/till `f`, `F`, `t`, `T` with `;`/`,` repeat; matching delimiter `%`; paragraph `{`/`}`; first nonblank line motion `_`; search-result motions `n`/`N`. Motions define inclusive/exclusive and characterwise/linewise behavior in `docs/EDITOR.md` fixtures before implementation acceptance.
- **Text objects:** `iw`/`aw`, `iW`/`aW`, `ip`/`ap`, and inside/around paired delimiters or quotes: `()`, `[]`, `{}`, `<>`, `'`, `"`, and backtick. Tree-sitter adds Markdown heading section `ih`/`ah`, fenced code body `ic`/`ac`, and link label/target `il`/`al`; when syntax is unavailable these structural objects fail visibly rather than guessing.
- **Counts:** decimal positive counts prefix commands, operators, and motions. Operator and motion counts multiply with checked arithmetic. Zero is a motion when no count is pending. The configurable hard maximum defaults to 1,000,000; overflow or excess returns `CountLimit` before mutation.
- **Edits/paste:** `x`, `X`, `s`, `r{grapheme}`, `J`, `p`, `P`, `~`, `u`, `Ctrl-r`, and `.`. Paste type follows register shape (character/line/block). Block operations are defined over logical grapheme columns, not terminal cell halves; short lines are padded only by an explicit block insertion.
- **Registers:** optional prefix `"{name}`. Supported registers are unnamed `"`, yank `0`, delete/change history `1`–`9`, small-delete `-`, named `a`–`z`, append `A`–`Z`, black-hole `_`, last-insert `.`, filename `%`, alternate-buffer `#` when supplied by E, expression register omitted, and clipboard `+`/`*`. Persisted user registers are only named registers explicitly opted into persistence; numbered/unnamed/transient registers are session state.
- **Macros:** `q{a-z}` starts recording normalized editor actions and `q` stops; `@{a-z}` executes, `@@` repeats. A macro stores commands/text, never arbitrary code. Defaults cap one recording at 10,000 actions, execution at 100,000 actions, recursion depth at 16, and wall-clock work per dispatch at 250 ms before cooperative cancellation and transaction rollback.
- **Marks/jumps:** `m{a-z}` buffer-local marks; `m{A-Z}` vault/path marks managed through an injected path resolver; `` `{mark}`` exact position and `'{mark}` first nonblank line; `` `` ``/`''` prior jump; `Ctrl-o`/`Ctrl-i` traverse a bounded jump list. Positions are revision-aware grapheme anchors and are invalidated or relocated explicitly after edits; they never point into a grapheme.
- **Search:** `/pattern` forward, `?pattern` backward, `n`/`N`, `*`/`#` literal whole-word under cursor. Search is UTF-8 and grapheme-aligned. Literal mode is always available; regex uses a linear-time engine with size/complexity limits. Empty pattern repeats the prior pattern. Invalid patterns do not mutate position. Wrap is configurable and announced.
- **Repeat:** `.` repeats the last complete mutating command with inserted text and register identity captured. It does not repeat undo/redo, macro recording, save, external process launch, merge acceptance, or commands with now-invalid structural targets. A new count overrides the captured count. Failure rolls back the repeat transaction.
- **Command line:** only `/`, `?`, and the following `:` commands are in scope: `w`, `write`, `q`, `quit`, `q!`, `wq`, `x`, `e[dit]`, `e!`, `undo`, `redo`, `earlier {count}`, `later {count}`, `set {supported-option}[?]`, `nohlsearch`, `marks`, `registers`, `jumps`, `recover`, `merge`, `edit-external`, `edit-selection-external`, and `edit-code-external`. Paths are resolved by injected core/app authority. `:!`, shell pipelines, arbitrary Ex ranges, substitutions, scripting, modelines, autocommands, plugins, and unlisted commands are out of scope.

Keymap configuration and conflict detection are owned by E6. The engine exposes command IDs and default bindings, and validates that remappings resolve to known commands; it does not parse user config itself.

Responsive behavior is client-owned. All commands remain available as named actions for narrow-terminal or screen-reader command interfaces.

#### Headless retrieval and automation seam

The crate exposes protocol DTOs independent of terminal layout. Protocol v1 is newline-delimited UTF-8 JSON on stdin/stdout for the editor harness and future CLI/IPC adapters. Every request has `protocol_version`, `request_id`, a stable command/query ID, explicit path/scope, and `allow_input`; every response repeats those fields and contains `ok`, structured `result` or `error`, `accepted_base_hash`, buffer `revision`, and a freshness value of `current`, `stale`, `unavailable`, or `not_applicable`. `accepted_base_hash` names the buffer's last authority read and must never be presented as a claim about current disk bytes unless that response performed a fresh `TextStore` read. Unknown versions or fields required for semantics fail closed; additive response fields are allowed. Human diagnostics go to stderr, never corrupt JSON stdout, and never include note text unless the explicit command is an export/read operation.

The future `mg-vault-cli editor` adapter is required to provide human output by default, `--output json`, JSONL stdin/stdout batch operation, `--no-input`, and `--no-color`; `NO_COLOR` has the same effect as `--no-color`. With `--no-input`, any operation requiring external-import acceptance, conflict resolution, recovery choice, clipboard consent, or remote capability returns `InteractionRequired` without mutation. This binding engine protocol is implemented and contract-tested in D; CLI argument parsing and TUI rendering remain E/app integration.

Within-buffer text/literal/regex search is editor-owned. Vault retrieval is injected through a read-only `KnowledgeQueryProvider` owned by B/G. Its versioned query kinds cover text, titles, regex, tags, properties, paths, relationships, tasks, and dates and return stable path/block references plus an index watermark and freshness. Links, backlinks, graph, Bases, and formulas are derived source views and can only navigate/select ranges; they cannot issue text edits or authorize saves. Ambiguous references return all candidates and require an explicit path choice. Stale results are labeled and cannot be represented as current. Provider absence or load at the 100,000-note/1,000,000-block target returns `unavailable`/`stale` without blocking local dispatch, open, save, recovery, or export. D supplies fake-provider contract and non-blocking isolation tests; B/G supplies indexing semantics and scale implementation.

### 3.5 Transitions & animation

No animation is produced. Mode, pending-command, save, recovery, and merge transitions are immediate state events. Reduced-motion behavior is therefore identical to default behavior.

### 3.6 Error states

| Trigger | Presentation contract | Recovery | Data-loss risk |
|---|---|---|---|
| Invalid UTF-8 source/import | Blocking error with path/source and byte offset, no replacement characters | Open externally or export bytes | No mutation |
| Incomplete/unsupported key sequence | Pending status until timeout/cancel, then named error | Escape/retry/use command help | None |
| Count/macro/regex/resource limit | Explicit bounded-operation error | Reduce operation or cancel | None; active transaction rolls back |
| Clipboard unavailable/denied/too large | Status error naming attempted backend | Use internal register, enable backend, or export | None |
| Parse/grammar/diagnostic failure | Degraded status; structural commands disabled | Continue plain-text editing/retry parser | None |
| Atomic save or fsync failure | Persistent `save failed`; buffer stays dirty | Retry, save copy, inspect journal | Possible only if underlying filesystem violates reported semantics; never false success |
| Fingerprint mismatch during save | `external change—merge required` | Three-way merge/export versions | None; source unchanged |
| Recovery journal corrupt/truncated | Recovery warning with last valid sequence | Recover valid prefix/export original/discard explicitly | Partial unsaved tail may be unavailable; source unchanged |
| Persistent undo hash mismatch | Undo history quarantined/disabled, not replayed | Start new history or inspect/export old history | None |
| External editor failure/invalid output | Blocking handoff result; buffer unchanged | Retry or inspect temp file | None |
| Changed external-editor output | Immutable textual diff and explicit accept/reject actions; buffer unchanged while pending | Accept, reject, export, or inspect temp file | None before acceptance; acceptance is journaled and fingerprint-gated |
| Merge conflict | Save-blocking conflict state with unresolved count | Resolve hunks, choose/export whole versions, or abort | None until explicit resolution |
| Source deleted/moved externally | Missing-source conflict; autosave disabled | Locate, save as, restore externally, or discard | None; no implicit recreation |

### 3.7 Accessibility

- Every operation has a stable command ID and textual label; no action requires mouse input, color, animation, syntax highlighting, or spatial conflict controls.
- The projection exposes mode, path, logical line, grapheme column, selection shape/extent, dirty/save state, pending command, active register, macro status, search position/count, diagnostics, folds, and merge/recovery status in deterministic focus order.
- Clients can request logical text by line/range, unfolded context, ordered outline, ordered diagnostics, and ordered merge hunks. Folded text is announced and can always be expanded by command.
- Status differences include words/symbol-independent labels. Diagnostics include severity text, message, source, and range.
- Cursor and selection are represented in grapheme and byte coordinates; line/column announcements never expose a half-grapheme location.
- No text-size assumptions exist in the engine. E owns terminal sizing and reflow; block selections use logical columns so cell-width rendering differences do not corrupt edits.
- Each projection is available as ordered plain text and protocol-v1 JSON with the same stable labels/actions. Projection tests use a linear focus/read order of path → mode → position/selection → dirty/save state → pending input → search/diagnostics → merge/recovery actions; no hidden hover, color, pane geometry, or timing conveys unique information.
- Accessibility is accepted headlessly in D using a screen-reader transcript adapter and 40×10-equivalent line-budget consumer that exercises every command, recovery, merge, and external-acceptance action. E must later verify terminal rendering, but that later work does not defer the completeness of engine labels, ordered text, keyboard reachability, or automation actions.

---

## 4. Implementation Specification

### 4.1 Architecture placement

Create `crates/mg-vault-editor/` as a Rust 2024 workspace member after the editor evidence spike. Proposed modules:

```text
crates/mg-vault-editor/src/
├── lib.rs
├── buffer.rs          # rope abstraction, revisions, grapheme/byte mapping
├── cursor.rs          # cursor, selections, anchors, marks, jump list
├── command.rs         # stable command IDs and typed commands
├── grammar.rs         # bounded modal parser/state machine
├── transaction.rs     # atomic in-memory edits and rollback
├── register.rs        # register shapes, persistence policy, clipboard adapters
├── macro_engine.rs    # action recording, limits, cancellation
├── search.rs          # literal/regex search and history
├── undo.rs            # branching undo and hash gate
├── journal.rs         # recovery WAL/snapshot/checksum/replay
├── persistence.rs     # autosave coordinator over core authority
├── external.rs        # safe process handoff/temp artifacts
├── merge.rs           # three-way merge model
├── syntax.rs          # Tree-sitter snapshots/folds/outline/structural objects
└── diagnostics.rs     # spellcheck and versioned diagnostic adapters
```

`mg-vault-editor` owns text, command, undo, recovery, and merge state. It depends on narrow interfaces for foundation file reads/fingerprints/atomic replacement and XDG paths; it does not perform unconstrained filesystem traversal. `mg-vault-core` remains the sole path and durable-write authority. `mg-vault-markdown` (A4/F prerequisite) supplies revision-bound, token-preserving Markdown/YAML spans where needed. `mg-vault-app` injects vault/path resolution, read-only knowledge retrieval, and coordinates file watching. `mg-vault-tui` translates terminal events and renders projections. Tree-sitter, indexes, plugins, AI, and diagnostics are derived clients of immutable snapshots and never obtain a `TextStore`, journal handle, mutable `Editor`, or source-write capability.

Ordinary vault files are sufficient and authoritative. Public identity is canonical vault identity plus public vault-relative path; the editor must not inject UUIDs, frontmatter, sidecars, or metadata into a note. `.obsidian/**` is a protected coexistence directory: opening it as an editable buffer, external handoff targeting it, and any editor-originated create/replace/remove operation fail `ProtectedControlPath`. Its existing bytes are otherwise ignored and preserved. Portable mg-vault settings belong only under `.mg-vault/**`, which is also protected from note-edit APIs; recovery/undo/temp state remains outside the vault in XDG locations. The editor cannot migrate, rewrite, merge, clean, or interpret either control directory.

### 4.2 Data model

Representative contracts (exact storage implementation is selected by Spike 2, but these semantics are binding):

```rust
/// Monotonic identity of an in-memory buffer state.
pub struct RevisionId(pub u64);

/// A valid position between extended grapheme clusters.
pub struct GraphemePos {
    pub byte: usize,
    pub grapheme: usize,
    pub line: usize,
}

/// Authoritative bytes accepted at the last open or durable save.
pub struct BaseSnapshot {
    pub bytes: std::sync::Arc<[u8]>,
    pub fingerprint: SourceFingerprint,
    pub hash: ContentHash,
}

/// Editor-owned state for one UTF-8 source buffer.
pub struct Buffer {
    pub id: BufferId,
    pub path: VaultRelativePath,
    pub revision: RevisionId,
    pub base: BaseSnapshot,
    pub mode: Mode,
    pub dirty: bool,
    // Rope, cursors, undo, parser, diagnostics, and journal are private invariants.
}

pub enum Mode { Normal, Insert, Replace, Visual(SelectionShape) }
pub enum SelectionShape { Character, Line, Block }
pub enum RegisterShape { Character, Line, Block }

/// One all-or-nothing in-memory change, journaled before acknowledgment.
pub struct EditTransaction {
    pub id: TransactionId,
    pub before: RevisionId,
    pub edits: Vec<TextEdit>,
    pub command: CommandId,
    pub timestamp_millis: i64,
}

pub struct TextEdit {
    pub range: std::ops::Range<usize>, // UTF-8 byte boundaries and grapheme-safe edges
    pub replacement: String,
}

pub struct UndoNode {
    pub id: UndoNodeId,
    pub parent: Option<UndoNodeId>,
    pub source_hash_before: ContentHash,
    pub source_hash_after: ContentHash,
    pub transaction: EditTransaction,
}

pub struct RecoveryHeader {
    pub format_version: u32,
    pub vault_key: VaultKey,
    pub path: VaultRelativePath,
    pub base_hash: ContentHash,
    pub base_fingerprint: SourceFingerprint,
    pub created_millis: i64,
}

pub struct MergeSession {
    pub base: std::sync::Arc<str>,
    pub local: std::sync::Arc<str>,
    pub external: std::sync::Arc<str>,
    pub external_fingerprint: SourceFingerprint,
    pub hunks: Vec<MergeHunk>,
    pub state: MergeState,
}

pub struct VersionedDiagnostics {
    pub revision: RevisionId,
    pub source: DiagnosticSource,
    pub items: Vec<Diagnostic>,
}
```

All public offsets state their coordinate system. Internal edits use UTF-8 byte ranges only after checking scalar boundaries and required grapheme-edge invariants. Conversion caches are invalidated locally by edits. Buffer text is always valid UTF-8; line endings are retained as loaded, and edits use the buffer's detected/default line-ending policy without normalizing untouched lines.

#### Persistent state

- Recovery: `$XDG_STATE_HOME/mg-vault/recovery/<vault-key>/<path-key>/`, outside the vault, mode `0700` directories and `0600` files where supported.
- Persistent undo: `$XDG_STATE_HOME/mg-vault/undo/<vault-key>/<path-key>/`, versioned, checksummed, size/age bounded, and keyed by canonical vault identity plus public relative path—not injected into notes.
- Explicit persisted registers/dictionaries: XDG state/config according to portability policy; no private clipboard content is persisted implicitly.
- Runtime external-edit artifacts: `$XDG_RUNTIME_DIR/mg-vault/` when available, otherwise private XDG state with explicit cleanup records.

Default retention is binding: persistent undo is capped per note at the lesser of 10,000 transactions, 256 MiB, or 30 days since last access; recovery keeps the newest three valid generations while dirty and removes them only after a durable save plus verified clean close, retaining failed/corrupt generations for 30 days unless explicitly discarded; opted-in named registers are capped at 1 MiB total and persist until user purge; failed external artifacts are capped at 1 GiB total and seven days. Cap pressure never deletes the sole recoverable generation or an active candidate: editing pauses with `RecoveryStorageFull` and export/purge actions. Purge is explicit, path-scoped, and truthful about failures.

A journal contains a versioned header, base snapshot or content-addressed recoverable base, ordered transaction records, commit markers, and per-record checksum. An acknowledged mutation must have a committed and synced recoverable record under the fixed Milestone 4 policy below. Journal compaction writes a new snapshot atomically, fsyncs it and its directory, then retires the old generation; interruption leaves at least one valid generation.

The Milestone 4 durability policy is fixed, not configurable. Before the first mutation is enabled, the engine creates a private journal generation containing its header and recoverable base, syncs the file, atomically installs it, and fsyncs the parent directory; if the platform/backend cannot provide those guarantees, the buffer opens read-only with `DurableJournalUnavailable`. Before any later mutating API returns `Ok` or emits any event/status that means accepted, applied, or complete, it writes the full recovery payload and commit marker, calls `fdatasync`/equivalent on the already durably linked journal file, and confirms success. The mutation remains staged and invisible to observers until that barrier succeeds. Barrier failure rolls back text, cursor, registers, undo, revision, and derived invalidations and returns `JournalDurabilityFailed`; it must not emit an acknowledgment. Insert-session undo grouping may span several separately synced journal fragments. Recovery reassembles those fragments into one undo node, so undo ergonomics never create an acknowledged-loss window. There is no timed durability batching mode in Milestone 4.

Persistent undo loads only when the current source content hash exactly matches a recorded durable save node selected as the history head. A mismatch, missing base, invalid checksum, unsupported version, or path-identity ambiguity rejects/quarantines that history. It never replays edits speculatively and never modifies source merely to make history fit.

### 4.3 API contracts

```rust
pub trait TextStore: Send + Sync {
    fn read(&self, path: &VaultRelativePath) -> Result<SourceRead, EditorIoError>;
    fn replace_atomic(
        &self,
        path: &VaultRelativePath,
        expected: &SourceFingerprint,
        bytes: &[u8],
    ) -> Result<SourceFingerprint, EditorIoError>;
}

pub trait Clipboard: Send + Sync {
    fn read(&self, kind: ClipboardKind, max_bytes: usize) -> Result<String, ClipboardError>;
    fn write(&self, kind: ClipboardKind, text: &str) -> Result<(), ClipboardError>;
}

pub trait ProcessRunner: Send + Sync {
    fn run_editor(&self, request: ExternalEditRequest) -> Result<ExitStatus, ExternalEditError>;
}

pub trait SyntaxProvider: Send + Sync {
    fn update(&self, snapshot: BufferSnapshot, edits: &[InputEdit]) -> SyntaxTask;
}

pub trait DiagnosticProvider: Send + Sync {
    fn diagnose(&self, snapshot: BufferSnapshot, cancel: CancelToken) -> DiagnosticTask;
}

/// Read-only B/G seam; results can navigate but never mutate or authorize a save.
pub trait KnowledgeQueryProvider: Send + Sync {
    fn query(&self, request: KnowledgeQuery, cancel: CancelToken) -> KnowledgeTask;
}

impl Editor {
    pub fn open(store: &dyn TextStore, path: VaultRelativePath) -> Result<Self, EditorError>;
    pub fn dispatch(&mut self, input: EditorInput) -> Result<Vec<EditorEvent>, EditorError>;
    pub fn execute(&mut self, command: Command) -> Result<Vec<EditorEvent>, EditorError>;
    pub fn autosave(&mut self, store: &dyn TextStore) -> Result<SaveOutcome, EditorError>;
    pub fn observe_external(&mut self, read: SourceRead) -> Result<ExternalOutcome, EditorError>;
    pub fn recover(candidate: RecoveryCandidate, current: SourceRead) -> Result<RecoveryOutcome, EditorError>;
    pub fn preview_external(&mut self, request: ExternalEditRequest) -> Result<ExternalImportCandidate, EditorError>;
    pub fn accept_external(&mut self, store: &dyn TextStore, candidate: CandidateId) -> Result<Vec<EditorEvent>, EditorError>;
    pub fn reject_external(&mut self, candidate: CandidateId) -> Result<Vec<EditorEvent>, EditorError>;
}
```

Important contract rules:

- `dispatch` parses only normalized keys/text; malformed terminal sequences never enter this crate.
- A mutating command either commits one complete `EditTransaction` or leaves buffer/register/undo/journal state unchanged. Register changes caused by an operator commit atomically with its text change.
- `Ok` from a mutating call and `MutationAccepted`/equivalent events are acknowledgment boundaries and occur only after the fixed journal sync barrier. Clients may render staged input, but must label it `pending durability` and must not report it as accepted.
- `replace_atomic` is the A-foundation fingerprint-gated durable operation. Save success is emitted only after it returns the new fingerprint.
- Watch notifications cannot authorize a save or reload; the app must supply a fresh authoritative read.
- Only `SourceRead` from `TextStore` can establish or advance `BaseSnapshot`; watcher metadata, Tree-sitter trees, query/index/plugin/AI results, external temp files, and protocol claims are never source authority. Every save, external acceptance, recovery acceptance, and merge acceptance rereads through `TextStore` and checks the expected fingerprint immediately before mutation/write.
- Clipboard adapters are explicit capabilities. OSC52 is write-only, disabled by default unless the terminal client opts in, base64-encodes after a strict byte cap, and never attempts terminal-response scraping.
- External command configuration is parsed once into executable plus arguments. Note/temp paths are appended as single arguments. No shell interpolation, modeline execution, or source-derived command is allowed.
- Diagnostics and syntax results are accepted only if `result.revision == buffer.revision`; stale work is discarded.
- Local spellcheck is the only Milestone 4 diagnostic backend and remains available offline; it receives immutable scoped snapshots and has no filesystem or network capability.
- Milestone 4 ships local diagnostics only; the remote LanguageTool adapter is excluded until a later capability review. Plugin and AI adapters are likewise excluded from execution in D. Future adapters must receive an explicit capability object scoped to named vault-relative paths, byte ranges, operation IDs, expiry, and read-only/read-proposal rights; default is no content and no network. Proposed edits are immutable textual diffs requiring explicit user acceptance and a fresh fingerprint, carry provider attribution, and pass through the same journal/save gates. They can never hold raw `TextStore`, clipboard, process, query, or network capabilities transitively.

### 4.4 State management

One `Editor` owns mutable state for a buffer: rope, cursors/selections, mode/parser, registers, macro recorder, marks/jumps, search/command history, undo tree, journal handle, base/save point, merge state, and versioned derived projections. E owns multiple editor instances and focus; app owns watcher subscriptions and path lifecycle.

Source boundaries:

- **Authority:** current ordinary file bytes and fingerprint read through `TextStore`/core; no cache, parse tree, index, watcher event, temp file, plugin, AI result, or client-provided hash can substitute.
- **Unsaved user work:** buffer plus recovery journal.
- **Last common merge base:** immutable `BaseSnapshot` retained until a newer durable save/reload is accepted.
- **Derived/non-authoritative:** Tree-sitter tree, folds, outline, diagnostics, spellcheck, search caches, display coordinates.
- **Persistent undo:** convenience history admitted only through the exact content-hash gate.
- **Knowledge retrieval:** read-only B/G projection with explicit watermark/freshness; it may request navigation to a public path/range but cannot mutate a buffer or advance its base.

Undo is branching: editing after undo creates a branch rather than deleting the former future. A save marks a node as durable but does not flatten history. Recovery journal and undo storage are separate: the journal protects unsaved work after failure, while persistent undo navigates accepted edit history. Undo/redo after external reload starts from a clearly marked base transition and cannot resurrect stale source across a hash mismatch without an explicit merge/import action.

Autosave defaults to an idle delay configured by app/E, is suspended during an incomplete command, macro transaction, external handoff, unresolved merge, or recovery preview, and coalesces requests. Only one save per buffer runs at a time. Edits arriving during I/O remain dirty against the newly saved revision and schedule another save; they are never incorrectly marked saved.

### 4.5 Dependencies

Candidate dependencies, pinned only after evidence spikes and license/security review:

- Rope/piece table: evaluate `ropey`, `crop`, and purpose-built alternatives for grapheme mapping, edit cost, memory, and persistent undo integration.
- Unicode: `unicode-segmentation` for extended grapheme clusters and `unicode-width` only for client display projections.
- Search: Rust `regex` or `regex-automata` configured for bounded linear-time matching.
- Syntax: `tree-sitter`, Markdown grammar, and allowlisted fenced-language grammars. Grammar versions and fixture corpus are pinned. Tree-sitter supplies navigation/highlighting candidates, not serialization or source authority.
- Merge/diff: evaluate maintained bounded diff/merge crates; the merge model and conflict guarantees remain internal if no candidate is safe.
- Spellcheck: local maintained dictionary/tokenizer crates with user dictionaries. A remote LanguageTool adapter is a post-Milestone candidate only and is not compiled or feature-enabled in D.
- Checksums/hashes and serialization reuse reviewed workspace dependencies where possible.

There is no database, CDN, cloud service, or asset required. Dictionaries and Tree-sitter grammars are third-party data/code and require recorded licenses and release provenance before packaging.

Token-preserving dependency contract: plain commands may edit only their explicit grapheme-safe byte ranges and must carry untouched bytes through exactly. Markdown/YAML structural mutations (`ih`/`ah`, `ic`/`ac`, `il`/`al`, diagnostic quick-fixes, or future property edits) are unavailable until `mg-vault-markdown` returns `TokenSpan { revision, byte_range, token_kind, surrounding_hash }` from the exact buffer snapshot. The editor validates revision, surrounding hash, UTF-8/grapheme edges, expected token kind, and non-overlap, then applies only the replacement range; it never prints or reserializes an AST/document/frontmatter block. Unknown tags, properties, comments, whitespace, quoting, line endings, Obsidian wikilinks/embeds/callouts, and unsupported YAML must remain byte-identical outside accepted ranges. Missing, stale, overlapping, or ambiguous spans return `StructuralTargetUnavailable` with zero mutation. Milestone 4 cannot claim structural-edit acceptance until A4’s shared preservation corpus passes against this contract; plain source editing remains independently acceptable.

### 4.6 Platform-specific considerations

- Arch/Hyprland terminals are first-class. Core/editor behavior remains portable to Linux and macOS.
- Clipboard backend preference is client-configurable: native platform command/API when available, terminal clipboard, then explicitly enabled OSC52 write fallback. `+` is the system clipboard and `*` is the primary selection only where a backend truthfully supports both; otherwise `*` returns `UnsupportedClipboardKind` and is never silently aliased to `+` unless the user explicitly configures that alias. Backend absence never breaks internal registers.
- File mode and XDG runtime protections are enforced where supported. If private runtime storage cannot be established, external handoff fails closed or uses a private state directory with a warning; it never uses a predictable world-readable temp path.
- `$EDITOR`/`$VISUAL` parsing must handle executable-plus-argument configuration without invoking `/bin/sh`. Platform-specific launch adapters are tested separately.
- Tree-sitter native build/toolchain and grammar ABI versions are pinned and checked. Unsupported fenced languages remain plain code text.
- Feature flags may isolate Tree-sitter language packs and platform clipboards. Plain editing, internal registers, journal, save, and merge are mandatory core behavior; no Milestone 4 feature flag enables network diagnostics.

### 4.7 Performance budget

Budgets are measured in optimized builds on the project reference Arch machine and reported with corpus/hardware metadata:

- For a 1 MiB/50,000-line Markdown note, p95 dispatch for single-grapheme insert/delete and ordinary cursor motions is **≤ 8 ms**, p99 **≤ 16 ms**, excluding durable journal fsync; no full-buffer clone on each keystroke.
- Opening a 10 MiB UTF-8 note into editable plain-text state is **≤ 500 ms**; Tree-sitter and diagnostics may complete asynchronously and cannot delay first editing beyond that budget.
- Incremental syntax update for a local edit is targeted at **≤ 16 ms p95**; work exceeding one frame is cancellable/backgrounded and leaves the prior projection marked stale.
- Literal search first result in 10 MiB is **≤ 100 ms**; regex and all-match enumeration are cancellable with configured match/pattern/input limits.
- Resident editor-owned memory for one 10 MiB note, excluding optional grammar code/dictionaries, is targeted at **≤ 4× source bytes + 64 MiB** after idle compaction. Undo/journal hard limits follow the binding retention policy above, with atomic compaction.
- Journal append records are bounded by the transaction payload plus framing. Every mutating dispatch pays the fixed sync barrier before acknowledgment; benchmarks report both pre-barrier engine latency and end-to-end durable acknowledgment latency. The latter has a p95 target of **≤ 50 ms** and p99 **≤ 150 ms** on the reference local filesystem, but a miss causes a visible slow-storage state rather than batching or weakening durability. Save completion independently uses durable foundation semantics.
- Macro, parser, syntax, diagnostics, merge, retrieval, and external-tool work has cancellation and memory/input caps. Milestone 4 performs no network access.

A budget miss degrades derived structure/diagnostics first, never text correctness, journaling, conflict detection, or ability to save/export source.

---

## 5. Test Specification

### 5.1 Unit tests

- `grapheme_cursor_never_splits_cluster`: generated text containing combining marks, ZWJ emoji, flags, skin tones, variation selectors, RTL, CJK, and newline variants; apply every motion/edit; assert all cursor/selection/edit edges are valid grapheme and UTF-8 boundaries.
- `byte_grapheme_line_maps_round_trip_after_random_edits`: property-generated edits and coordinate conversions; assert text equality and mapping invariants against a simple reference model.
- `grammar_table_accepts_only_documented_sequences`: table-drive every grammar production and incomplete prefix; assert command IDs, counts, shapes, and no mutation for unsupported sequences.
- `operator_motion_count_composition`: cover `2d3w`, doubled operators, overflow, zero ambiguity, line endings, empty/final lines, and Unicode words; assert deterministic range and checked maximum.
- `text_objects_preserve_delimiters_and_graphemes`: nested/unmatched delimiters, quotes, paragraphs, headings, links, and fences; assert inside/around ranges and visible failure when syntax is unavailable.
- `register_rotation_append_black_hole_and_shapes`: assert unnamed/0/1–9/small-delete/named append/black-hole behavior and atomicity with failed edits.
- `macro_limits_recursion_and_rollback`: nested/self macros, cancellation, invalid target, action/time/count limits; assert no partial transaction.
- `marks_and_jumps_relocate_or_invalidate`: edits before/through anchors and path changes; assert explicit valid state and grapheme boundaries.
- `search_is_bounded_and_grapheme_aligned`: literal/regex forward/back/wrap/invalid/empty patterns, catastrophic-looking regex, Unicode; assert cancellation and aligned matches.
- `dot_repeat_replays_one_change_atomically`: insert/operator/paste/count override plus excluded commands; assert register capture and rollback on invalid structural target.
- `undo_branching_keeps_alternate_future`: edit/undo/edit; assert both branches and exact content hashes.
- `persistent_undo_requires_exact_hash`: valid history, different source bytes, checksum/version/path mismatch; assert rejection and zero source mutation.
- `journal_replays_committed_prefix_only`: truncate/corrupt at every byte boundary; assert replay stops at last committed checksummed transaction and retains evidence.
- `mutation_ack_requires_synced_journal`: fault journal-generation creation/install/directory-fsync and every byte boundary before/within/after payload write, commit marker, and file sync for every mutating API; assert an undurable generation opens read-only, no success/event/revision/text/register/undo visibility occurs before sync, failures roll back totally, and every acknowledged return recovers exactly. Grouped insert fragments recover as one undo node without an acknowledged-loss window.
- `autosave_revision_race_stays_dirty`: edit while save is in flight; assert saved revision advances but later edit remains dirty.
- `three_way_merge_classifies_hunks`: generated non-overlap, overlap, deletion, line-ending, Unicode, and empty-file cases; assert no silent winner and all versions remain exportable.
- `syntax_and_diagnostics_drop_stale_revisions`: complete asynchronous jobs out of order; assert only current revision projects.
- `osc52_caps_and_escapes_payload`: boundary sizes and control characters; assert fixed framing, bounded base64, opt-in, and no response read.
- `structural_spans_are_token_preserving`: generate stale, wrong-kind, overlapping, ambiguous, and valid `TokenSpan` values across CommonMark/GFM/YAML/Obsidian fixtures; assert only the validated byte range changes and all surrounding bytes remain exact.
- `knowledge_results_are_read_only_and_freshness_typed`: fake every query kind, ambiguity, stale watermark, cancellation, and provider absence; assert deterministic ordered references, no mutation capability, and no stale-as-current projection.
- `protocol_v1_jsonl_is_stable_and_noninteractive_safe`: golden-schema requests/responses, unknown versions, malformed JSON, human/JSON parity, stdout purity, stderr redaction, `--no-input`, `--no-color`, and `NO_COLOR`; assert every interactive/destructive action fails closed without explicit acceptance.
- `accessible_projection_has_complete_linear_equivalent`: assert ordered plain-text/JSON labels and actions for every mode, selection, fold, diagnostic, save, recovery, merge, external candidate, and degraded state; replay with the transcript and 40×10-equivalent consumers without color/spatial-only information.
- `capabilities_are_deny_by_default_and_scoped`: attempt plugin/AI/query/diagnostic access to undeclared paths/ranges, network, clipboard, process, store, logs, and expired operation IDs; assert denial, attribution, redacted errors, and explicit preview/fingerprint gates for proposals.

### 5.2 Integration tests

- **Milestone crash matrix:** launch an editor harness, edit, and kill/fault-inject before/after record write, commit marker, fsync, compaction rename, file rename, and directory fsync. Reopen and assert either durable source or recoverable committed edits, never false save success or overwrite of newer source.
- **Atomic autosave conflict:** open at fingerprint A, externally write B, attempt local save C; assert foundation rejects the write, B remains byte-exact, and merge contains A/B/C.
- **External clean reload:** externally change a clean file; assert verified reload, grapheme-safe cursor relocation, and new base hash.
- **External dirty merge fixtures:** use `fixtures/conflicts/` for independent and overlapping edits, deletion/move, Git checkout, Syncthing-style conflict, and repeated edits during merge; assert both versions preserved and second fingerprint gate enforced.
- **External editor note/selection/fence round-trip:** use deterministic fake editor processes for success, no change, nonzero exit, signal, invalid UTF-8, oversized output, deleted temp, concurrent buffer edit, and concurrent disk edit; assert one undo transaction or unchanged buffer/merge as specified.
- **External import confirmation:** for clean note, selection, and fence output, assert the changed result remains only an immutable diff candidate until explicit accept; reject/cancel/`--no-input` are byte-identical; accept rereads authority, journals durably, and creates exactly one undo node; concurrent changes invalidate acceptance and require a separately accepted merge.
- **Private temp and no-shell launch:** hostile filenames and editor arguments containing spaces, quotes, `$()`, semicolons, newlines, and leading dashes; assert single-argument path passing, no command execution, and private permissions.
- **Tree-sitter degradation:** missing grammar, parser panic/error boundary, malformed Markdown, huge fence, and cancellation; assert source editor/search/save remain functional and projection says degraded.
- **Clipboard matrix:** fake native, unavailable, denied, oversized, and OSC52 adapters; assert internal register always works and sensitive text is absent from logs.
- **Diagnostic privacy:** exercise local spellcheck and a non-executable fake future-remote capability proposal; assert no network capability exists in Milestone 4, only the exact selected snapshot reaches local diagnostics, ambient note/vault content is absent, logs are redacted, and stale responses are rejected.
- **Obsidian/source preservation:** edit narrow spans in versioned Markdown/YAML/Obsidian fixtures and assert all bytes outside explicit transaction ranges remain byte-identical.
- **Control-directory coexistence:** populate `.obsidian` with versioned fixtures and `.mg-vault` with portable settings; exercise open, save, recovery, external handoff, purge, and hostile traversal/symlink aliases; assert editor mutation APIs return `ProtectedControlPath` and both trees remain byte-for-byte unchanged.
- **Authority inversion matrix:** forge watcher metadata, stale index/query results, Tree-sitter spans, protocol hashes, plugin/AI proposals, and temp-file contents; assert none advances `BaseSnapshot`, authorizes save/import, or overrides a fresh `TextStore` reread.
- **Retrieval isolation at target scale:** use a deterministic fake provider advertising 100,000 notes/1,000,000 blocks while delayed, saturated, stale, cancelled, and unavailable; assert all query kinds have truthful freshness and local p95 dispatch/open/save budgets remain within the editor limits.
- **Security/adversarial suite:** permission failures, journal/undo/temp path symlink substitution, control-directory aliases, malicious provider payloads, log/panic capture, capability expiry, and resource exhaustion; assert fail-closed behavior, no content leak, no authority escape, and preserved recoverable state.

### 5.3 UI / E2E tests

The engine harness—not terminal snapshots alone—must execute normalized key sequences and assert text, cursor, mode, pending grammar, registers, undo nodes, events, and save state after every command. Required scripts cover:

1. Normal → insert → escape → counted operator/text object → yank/paste → dot repeat → undo/redo.
2. Visual character/line/block operations over mixed-width graphemes and short lines.
3. Search/marks/jump-list/command-line flow with invalid and cancelled commands.
4. Macro record/replay with a bounded failure and full rollback.
5. Recovery offer against unchanged source and merge offer against newer source.
6. External editor handoff followed by a concurrent external edit and explicit hunk resolution.
7. Parser/diagnostic degradation while ordinary editing and atomic save continue.

E integration later adds terminal key decoding, focus, and visual layout. D acceptance already requires complete textual/JSON status announcements, keyboard/action reachability, transcript-screen-reader behavior, and narrow-consumer behavior; E snapshots may supplement but cannot replace those engine-semantic assertions.

### 5.4 Visual / manual verification

Using the future E harness, verify:

- default and `NO_COLOR`/monochrome rendering;
- narrow terminals (40×10 and 80×24) without hiding mode, dirty/conflict, or recovery status;
- long paths, long diagnostics, empty file, one-line file, 10 MiB file, and many conflicts;
- combining/RTL/CJK/emoji cursor and selection rendering in terminals with differing width tables;
- keyboard-only recovery, merge, register, macro, search, and external-editor flows;
- textual/screen-reader mode announces ordered hunks, fold state, and diagnostics without relying on highlighting;
- terminal disconnect during OSC52/save/external editor and subsequent recovery.

Milestone 4 quality gates include `cargo fmt --check`, strict workspace Clippy, all-target/all-feature tests, property tests, fault injection, and benchmark regression reports.

---

## 6. Compliance & Safety Gate

### 6.1 Sensitive data classification

- [ ] No sensitive data involvement
- [x] Handles sensitive data — note text, search terms, registers, undo, recovery journals, clipboard payloads, dictionaries, diagnostics, external-editor temp files, and conflict versions may all be private.
- [ ] Uses synthetic/test data only until compliance gate clears

Protection measures: local-first operation with no Milestone 4 network backend; private XDG state/runtime permissions; no content in routine logs, panic reports, metrics, filenames derived from content, or error decoration; explicit clipboard/OSC52 capability; no implicit register persistence; bounded retention and explicit purge for journals/undo/temp artifacts; shell-free external launch; deny-by-default future plugin/AI/remote-diagnostic capability proposals; synthetic fixtures only in CI. Crash reports contain identifiers/hashes only when needed and never source excerpts by default.

### 6.2 Asset provenance

- [ ] No third-party assets
- [x] Uses third-party assets — Unicode segmentation tables/crates, Tree-sitter runtimes/grammars, and spellcheck dictionaries require pinned source, version, license, checksum, and redistribution review. No private dictionary or corpus may enter repository fixtures.

### 6.3 Language / claims audit

- [x] No unsupported user-visible claims are authorized. “Vim-style” means only the grammar enumerated in this spec and versioned `docs/EDITOR.md`; it must not be marketed as Vim-compatible.
- [x] Target behavior is labeled specification, not current implementation.
- [x] Errors distinguish journaled, saved, merged, degraded, and failed states; no message may claim saved/recovered/copied/cleaned up before read-back or adapter confirmation.

### 6.4 Regulatory alignment

The template references Lens 3, and all binding criteria are addressed as follows:

- **Lens 3A Determinism:** structural ranges are derived from exact buffer revisions; stale parser/query/index results are rejected or explicitly labeled. Search behavior, grammar, protocol ordering, source revision, and index watermark are fixture-defined.
- **Lens 3B Ambiguity:** structural text objects fail when the parse target is absent/ambiguous; retrieval returns all ambiguous public-path candidates; external conflicts require explicit resolution rather than a heuristic winner.
- **Lens 3C Query depth:** this slice owns text/literal/regex within-buffer search and a versioned read-only seam for B/G text, titles, regex, tags, properties, paths, relationships, tasks, and dates. D contract-tests every kind without claiming to implement the vault index.
- **Lens 3D Derived authority:** Tree-sitter trees, folds, outlines, diagnostics, queries, links/backlinks, graphs, Bases, formulas, plugins, and AI cannot write or override source; only a fresh `TextStore` read establishes authority.
- **Lens 3E Scale:** parsing/diagnostics/retrieval are cancellable and revisioned, with explicit per-note budgets and a 100,000-note/1,000,000-block fake-provider isolation gate. B/G owns actual indexing at that scale, but its load or absence cannot block local editing.

Auto-fail gates directly covered here: no acknowledged source-content loss (every mutating acknowledgment follows a journal sync barrier), unconfirmed import (all changed external output remains a candidate until explicit acceptance), silent conflict winner, unknown-syntax reserialization, non-atomic autosave success, recovery overwrite of newer source, traversal/symlink bypass or protected-control-directory mutation, stale/derived authority inversion, or unintended content exfiltration. Multi-file refactors are out of scope and must use H transactions rather than editor-local repeated saves.

---

## 7. Gap Analysis vs. Current State

### 7.1 What exists today

**Implemented foundation only:**

- `Cargo.toml` contains Rust 2024 workspace members `mg-vault-core` and `mg-vault-cli`; no editor crate exists.
- `crates/mg-vault-core/src/vault.rs`, `atomic.rs`, `xdg.rs`, and related tests provide confined file authority, fingerprints, XDG paths, and atomic replacement primitives suitable as dependencies.
- `docs/PRODUCT.md` and `docs/ARCHITECTURE.md` explicitly defer editor/TUI/parser/index work.
- `docs/specs/a-foundation-file-authority.md` defines the current accepted file-authority slice.

**Absent:** rope/piece-table buffer, grapheme cursor model, modal grammar, registers/macros, marks/jumps, search/command line, undo tree, recovery WAL, autosave coordinator, watcher integration, merge engine, external-editor handoff, Tree-sitter, spellcheck/diagnostics, editor documentation, fixtures, and editor benchmarks.

No current source file, test name, or crate should be treated as proof that any D behavior is implemented.

### 7.2 Delta to spec

- Run and record Spike 2 to select the text structure, persistent-undo representation, incremental parse strategy, and bounded merge implementation. Journal representation may be selected by evidence, but its per-mutation sync-before-acknowledgment policy is already binding.
- Add `crates/mg-vault-editor` and its modules/tests; add only justified dependencies to the workspace.
- Extend core/app contracts for fresh reads, fingerprint-gated atomic replacement, canonical vault/path keys, and watcher-delivered rereads without moving filesystem authority into editor.
- Add `docs/EDITOR.md` with the exact grammar table, command semantics, options, error/status language, persistence locations/retention, and deliberate Vim differences.
- Add generated Unicode/property corpora, command fixtures, crash/fault harnesses, `fixtures/conflicts/`, Markdown/Obsidian/control-directory preservation fixtures, fake clipboard/process/diagnostic/query/capability adapters, JSONL protocol goldens, accessibility transcripts, security adversarial tests, and performance benchmarks.
- Add XDG recovery/undo schema versions and migration policy. No database migration and no metadata injection into notes are permitted.
- Add E/app integration later for terminal decoding/rendering, file-watch orchestration, multi-buffer path lifecycle, and user-facing recovery/merge panes.

### 7.3 Estimated scope

**XL.** D1–D11 combine a Unicode text data structure, a composable command language, branching/persistent history, crash consistency, optimistic filesystem concurrency, process and clipboard security boundaries, incremental parsing, diagnostics, and adversarial fault testing. The slice should land internally in dependency-ordered sub-slices, but Milestone 4 is accepted only when the cross-cutting recovery and external-edit fixtures pass.

Recommended implementation checkpoints:

1. Buffer/revision/grapheme invariants and transaction model.
2. Modes plus bounded operators/motions/text objects/counts/repeat with command fixtures.
3. Registers/clipboard/macros/marks/jumps/search/command line and resource limits.
4. Branching undo plus sync-before-acknowledgment journal replay/compaction, retention, and hash-gated persistence.
5. Autosave over A authority with revision races and failure injection.
6. External-change three-way merge and recovery flow.
7. Secure external editor note/selection/fence handoff.
8. Tree-sitter/folds/outline/structural objects and diagnostic degradation.
9. Accessibility/text/JSON projections, retrieval and automation contracts, documentation, security review, and performance gates.

### 7.4 Blocking dependencies

- **A3/A5/A6/A7:** confined path authority, source fingerprints, durable atomic write semantics, XDG state, and truthful errors. The current foundation supplies a narrow start; hostile directory-entry race hardening remains a known platform limitation and must not be overstated.
- **A4 / mg-vault-markdown:** the revision/range/kind/surrounding-hash `TokenSpan` contract and shared Markdown/YAML/Obsidian compatibility fixtures are required before structural Markdown edits can be enabled or claim preservation. Plain range editing is not blocked.
- **Spike 2:** must select/validate rope or piece table, undo/journal, Tree-sitter incremental integration, and merge behavior before production implementation.
- **B2/app watcher boundary:** required for prompt event delivery, though editor always verifies events by reread and can detect conflicts at save without the service.
- **E:** renders engine projections and maps terminal input; not required for headless command/recovery acceptance.
- **F/G/H/M:** rendering, vault-wide search/link semantics, multi-file refactors, and Git/Syncthing workflows are dependent consumers and must not be folded into D.

### 7.5 Explicit non-goals

- Bug-for-bug Vim/Neovim or full Ex/Vimscript compatibility, arbitrary shell commands, modelines, autocommands, and editor plugins.
- Terminal rendering, panes/tabs/splits, source preview, themes, terminal key decoding, or workspace/session UI (E).
- Whole-vault index implementation, link-resolution semantics, backlink/graph computation, Bases/formula evaluation, and storage at scale (B/G). D still owns and tests their read-only, freshness-typed, non-blocking protocol seam.
- Lossy Markdown/YAML reserialization, rich-content rendering, or general formatter/reflow behavior (A4/F).
- Multi-file rename/refactor transactions (H), Git/Syncthing orchestration (M), proprietary sync, or real-time multi-user collaboration.
- Database-authoritative content, hidden UUID injection, implicit cloud diagnostics, ambient AI access, or persistence of transient/private registers by default.
- Binary-file or non-UTF-8 editing in the built-in engine; such files remain byte-preserved and may be opened by an explicit external tool.

---

## 8. Binding Decisions and Evidence Questions

The policy questions raised by review are resolved for Milestone 4:

- **D1 journal durability:** every mutating API syncs its recovery record before acknowledgment; no timed/bounded-loss batching mode ships.
- **D2 external import:** unchanged output is a no-op; every changed note/selection/fence result is an immutable diff candidate and requires explicit accept after a fresh authority read. `--no-input` never accepts.
- **D3 retention:** undo is bounded by 10,000 transactions/256 MiB/30 days; recovery keeps three valid dirty generations and never removes the sole recoverable one under pressure; opted-in registers are capped at 1 MiB; failed temp artifacts at 1 GiB/seven days.
- **D4 clipboard identity:** `+` and `*` remain distinct where supported; unsupported `*` fails unless the user explicitly configures aliasing.
- **D5 diagnostics:** Milestone 4 is local-only; remote LanguageTool, plugins, and AI execution are deferred behind the specified least-privilege proposal contract.
- **D6 source/structure:** ordinary files via `TextStore` are sole authority; `.obsidian` and `.mg-vault` are protected; structural mutations require the token-preserving A4 span contract.
- **D7 retrieval/automation:** D ships the versioned read-only query seam, protocol-v1 JSONL DTOs/harness, noninteractive fail-closed semantics, and scale-isolation tests; B/G implements indexing and E/app implements CLI/TUI adapters without changing these contracts.

Only implementation-evidence choices remain open, and none may weaken the binding semantics above:

- **E1:** Which rope/piece-table and diff/merge implementations pass Spike 2’s Unicode, memory, incremental-parse, sync-before-acknowledgment, and recovery tests?
- **E2:** Which Markdown and fenced-language Tree-sitter grammars/versions meet navigation, ABI, license, malformed-input, and packaging requirements? Tree-sitter never authorizes serialization; plain editing remains available if no grammar qualifies.

Implementers may not silently substitute bug-for-bug Vim behavior, lossy source normalization, shell execution, heuristic conflict winners, unconfirmed import, stale retrieval represented as current, capability escalation, or recovery overwrite.
