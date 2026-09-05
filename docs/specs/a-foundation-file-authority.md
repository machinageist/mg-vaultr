# Spec: Foundation and Vault Authority

**Feature ID:** a-foundation-file-authority
**Parent feature:** root
**Spec author agent:** Foundation/vault-authority spec agent
**Date:** 2026-08-29
**Iteration:** 2 — full-template rebuild of the iteration-1 draft; every iteration-1 claim and deferral is preserved

---

## 1. Purpose

### 1.1 One-sentence job

Give the user a trustworthy local filesystem boundary — XDG paths and a vault registry, a `.mg-vault` control directory, path-based note identity with source fingerprints, byte-preserving edits, atomic single-file transactions, recoverable trash, and one stable error/JSON contract — so that registering an ordinary directory and creating, reading, replacing, span-editing, trashing, or restoring Markdown inside it can never silently overwrite, escape, corrupt, or half-apply anything.

### 1.2 Why it matters

Every other feature in the tree (B index, C CLI, D editor, E TUI, H refactoring, M sync, N plugins, O AI) eventually mutates a user's files. If the mutation boundary is not correct, no amount of correctness above it helps: a stale write silently destroys an external edit, a `..` component escapes the vault, an interrupted write leaves a truncated note, or a "recovery" restores an old copy over newer work. `mg-vault` also promises that a vault stays usable without this application and coexists with Obsidian — which means the foundation must never inject hidden identifiers into notes, never reserialize unknown syntax, never treat a database as authority, and never write outside `.mg-vault` for its own bookkeeping. This feature exists so that authority, identity, preservation, coexistence, and transactionality are decided once, in one small crate, and are enforced for every caller rather than re-litigated per command.

### 1.3 Success signal

On synthetic vaults, an adversarial fault-and-race suite demonstrates that **every accepted mutation is either fully committed and durable, or reported as failed with the source byte-identical to its pre-operation state** — with zero cases of source-content loss, partial write, unconfirmed overwrite, traversal or symlink escape, false atomic success, or a recovery that replaced newer source. Concretely: `cargo test --workspace --all-targets --all-features` passes with the confinement, collision, conflict, durability, and trash/restore matrices green; and for every injected failure point, a post-condition checker finds the note bytes equal to either the exact pre-image or the exact committed image, never anything else.

---

## 2. User Stories

> As a user with several separate note directories, I want to register and select vault roots by name, so that every command targets one explicit, canonical namespace instead of guessing from my working directory.

> As a writer, I want to create and read Markdown notes as ordinary files at ordinary paths, so that I keep full ownership and can open the same vault with Obsidian, `grep`, or a plain text editor tomorrow.

> As a careful editor, I want a replacement to be rejected when the file changed since I read it, so that an external edit is never silently overwritten and I am told the expected and observed fingerprints instead of losing bytes.

> As an Obsidian user, I want `mg-vault` to leave `.obsidian` untouched and keep its own state under `.mg-vault`, so that switching between tools never corrupts either app's settings.

> As a person editing a note with unusual syntax — Dataview blocks, callouts, plugin fences, CRLF endings, no trailing newline — I want everything outside the exact span I asked to change to survive byte-for-byte, so that the tool never quietly reformats work it does not understand.

> As a user who deleted the wrong note, I want trash and restore that refuse to overwrite whatever is at the destination now, so that recovering an old file cannot destroy a newer one.

> As a safety-conscious user, I want generic note commands to be structurally incapable of writing to `..`, an absolute path, a symlink that escapes the vault, `.obsidian`, or `.mg-vault`, so that a bad path in a script cannot reach outside the vault.

> As an automation author, I want one versioned JSON envelope, stable error codes, stable exit categories, and a `--no-input` guarantee, so that my scripts never hang on a prompt and never parse human decoration.

> As a screen-reader or `NO_COLOR` terminal user, I want every state, marker, and error to be a literal word on its own line, so that nothing important is conveyed only by color, glyph, or column position.

---

## 3. UX Specification

`mg-vault` is a command-line program. There are no screens, modals, sheets, popovers, drawers, panels, mouse gestures, sound, or haptics anywhere in this feature; §3 is read as terminal UX throughout.

### 3.1 Command / view inventory

Every "view" below is line-oriented output from one invocation. "New" is relative to an empty repository; A owns all of them.

| View | Invocation | New / modified | Output shape |
|---|---|---|---|
| Global help and grammar | `mg-vault --help`, `mg-vault <group> --help` | New (clap-generated) | Static usage block; no registry or vault access |
| Version | `mg-vault --version` | New | One line |
| Vault registration result | `mg-vault vault register NAME PATH` | New | One receipt line, or JSON record |
| Vault inventory | `mg-vault vault list` | New | One row per vault, sorted by name; explicit empty state |
| Vault selection result | `mg-vault vault select NAME` | New | One receipt line |
| Vault root resolution | `mg-vault vault path [--vault NAME]` | New (target) | One canonical path line; script-friendly |
| Note creation receipt | `mg-vault note create PATH --body …` | New | Path plus resulting fingerprint |
| Note content stream | `mg-vault note read PATH` | New | Exact source bytes on stdout, nothing else |
| Note replacement receipt | `mg-vault note write PATH --expected FP --body …` | New | Path plus before/after fingerprint |
| Byte-span edit receipt | `mg-vault note edit-span PATH --start N --end N --expected FP --replacement …` | New (target) | Path, span, before/after fingerprint, byte deltas |
| Trash receipt | `mg-vault note trash PATH` | New | Original path plus recovery ID |
| Trash inventory | `mg-vault trash list` | New (target) | One row per entry: ID, original path, fingerprint, health |
| Restore receipt | `mg-vault note restore ID` | New | ID plus restored path |

Not owned here and never emitted by these commands: `index`, `search`, `interop`, `doctor`, `note edit` with editor handoff, `note append/move/rename/purge`. Those belong to B (index/search), C (full CLI surface), and P (export/publishing). A supplies the core APIs they call.

Stream discipline (binding, version 1):

- Success human output goes to **stdout**; warnings and errors go to **stderr**.
- With `--json`, success is exactly one UTF-8 JSON object plus one `\n` on stdout and stderr is empty; failure is exactly one JSON error object plus one `\n` on stderr and stdout is empty.
- `note read` without `--json` is the only unadorned content stream: stdout receives the source bytes and nothing is inserted, removed, or appended.

### 3.2 Interaction flows

#### Registering, listing, and selecting a vault

1. `vault register NAME PATH` validates `NAME` against the registry-name rule (non-empty; ASCII alphanumeric, `-`, `_` only), canonicalizes `PATH`, and requires the result to be an existing directory.
2. It refuses a duplicate registry name (`collision`). **Target:** it also refuses a canonical root already registered under another name (`duplicate_root`), naming the existing record, because two names for one root make "which vault did I mutate" ambiguous.
3. The record `{name, canonical path}` is inserted and the registry is persisted atomically (temp file → `fsync` → rename → parent `fsync`). A crash yields the complete old registry or the complete new one, never partial JSON.
4. If nothing was selected before, the newly registered vault becomes the selection. This is stated in the receipt line so the user is never surprised about what a later bare command targets.
5. `vault list` prints rows sorted by name with an explicit literal `selected: yes|no` field (target) in addition to the current `*` marker, so selection is never conveyed by a glyph alone. Empty registry prints `No registered vaults. Run: mg-vault vault register NAME PATH`.
6. `vault select NAME` fails with `unknown_vault` for an unregistered name and otherwise persists the selection atomically.
7. `--vault NAME` overrides selection for exactly one invocation and never mutates persisted selection.
8. **Target:** `vault unregister NAME` removes only the registry record. It never deletes the directory, notes, `.mg-vault`, trash, or index data, and the receipt says so literally. Removing the selected record clears selection in the same atomic write.

#### Creating a note

1. Resolve the vault (`--vault NAME`, else selection, else `no_vault_selected`) and open it, canonicalizing the root.
2. Validate the caller path: it must be relative, non-empty, composed only of normal components (no `.`, `..`, root, or prefix), end in `.md`, and — because this is a mutation — not have `.obsidian` or `.mg-vault` as its first component.
3. Walk the parent chain from the canonical root one component at a time. An existing component that is a symlink or not a directory is rejected; a missing component is created and its parent `fsync`ed; after each step the component is canonicalized and re-checked to be strictly beneath the root.
4. Write the body to a unique same-directory temporary file (`.<name>.mg-vault-<pid>-<serial>.tmp`), `write_all`, `sync_all`.
5. Link it into place with a **non-replacing** rename (`renameat2(RENAME_NOREPLACE)` on supported platforms; `hard_link` + `unlink` fallback elsewhere). An existing destination yields `collision`; **there is no `--force`, no overwrite flag, and no interactive overwrite prompt for create.**
6. `fsync` the parent directory, then report the path and the fingerprint of the bytes written. Success is printed only after that directory sync returns.
7. Any failure removes the temporary file and reports the failure; no partially written file is ever left at a note path.

#### Reading a note

1. Validate the path (read mode: control-directory first components are permitted for reading, never for writing).
2. Canonicalize the target, require it to be strictly beneath the root, and require it to be a regular file.
3. Read the complete bytes, compute the fingerprint, and require valid UTF-8. Invalid UTF-8 is `invalid_utf8` — an explicit refusal, never a lossy or replacement-character decode.
4. Human mode writes the exact bytes to stdout. `--json` returns `path`, `content`, `byte_len`, and `fingerprint`. Read never consults any index and never claims freshness from derived state.

#### Replacing a note (`note write`)

1. Validate the path in mutation mode.
2. Resolve the existing target with `symlink_metadata`: a symlink or non-regular file is `unsafe_path`. Canonicalize and re-check containment.
3. Read the current bytes and compare their fingerprint against `--expected`. A mismatch is `conflict`, reports both digests, and performs **no** write.
4. On match, write the replacement through the same-directory temp/`fsync`/rename/parent-`fsync` sequence and report the new fingerprint.
5. `--expected` is mandatory. There is no "just overwrite" path, so a stale write cannot be requested by accident.

#### Editing one byte span (`note edit-span`, target CLI surface over an implemented core primitive)

1. Validate path, resolve existing file, and check `--expected` exactly as for `note write`.
2. Require the current bytes to be valid UTF-8, the replacement to be valid UTF-8, `start <= end <= len`, and both offsets to sit on character boundaries. Any violation is `invalid_edit_span` or `invalid_utf8` with no mutation.
3. Build `prefix || replacement || suffix` by copying the untouched byte ranges verbatim — no parser, no serializer, no line-ending normalization, no whitespace or quoting rewrite.
4. Commit through the same atomic replacement and report before/after fingerprints and byte-length delta.
5. Span discovery helpers are separate and fail closed: `scan_frontmatter` refuses a UTF-8 BOM and an unclosed envelope, preserves LF/CRLF/mixed endings and reports which was observed; `locate_frontmatter_scalar` refuses missing frontmatter, a missing key, a duplicate top-level key, a non-scalar value, and invalid YAML. Nothing is guessed into an editable region.
6. **Multi-span and multi-file edits are not offered by this feature.** One invocation edits at most one span of one file. A future batch operation must be one plan, one fingerprint check, overlap rejection, and one atomic replacement — repeated single-span commits are explicitly not a substitute (see §7.5).

#### Trashing and restoring

1. `note trash PATH` validates the path in mutation mode, resolves the existing regular file, reads its bytes to record a fingerprint, and mints a recovery ID.
2. It creates `.mg-vault/trash/files/` and `.mg-vault/trash/entries/`, renames the note to `files/<id>`, then `fsync`s the old and new parent directories. If either sync fails, the rename is reversed and the error is returned.
3. It writes `entries/<id>.json` (`version`, `original_path`, `fingerprint`) with the non-replacing atomic create. If that fails, the payload is renamed back and the error is returned.
4. **Target:** steps 2–4 are bracketed by a durable journal record under `.mg-vault/journal/` written and `fsync`ed *before* the first destructive step, phase-updated, and removed only after the final sync. A process killed mid-operation leaves a record that the next `Vault::open`/recovery pass reconciles idempotently, and an orphaned payload is quarantined and reported rather than deleted (§4.4).
5. `trash list` (target) prints entries newest-first with ID, original path, byte length, fingerprint, and health (`ok`, `orphan_payload`, `orphan_metadata`, `unreadable_metadata`). It never prints note content.
6. `note restore ID` validates the ID shape, reads and version-checks the metadata, revalidates the recorded original path through the same public path authority, prepares the parent chain, and links the payload into place with a **collision-refusing** operation. An occupied destination is `collision`: the trash entry and its payload are left completely intact, and the restore is refused. This is the mechanism by which **recovery can never overwrite newer source** — restore has no force flag and no "newer/older" heuristic.
7. Only after the destination is durably linked are the payload and metadata removed and their directories synced.
8. Permanent purge is **not implemented and not offered** in this feature. Nothing here deletes user bytes irreversibly.

#### Failure branching common to all mutations

Every mutation ends in exactly one of three outcomes, and the output states which:

- **Committed** — the durable sequence completed; a receipt with the resulting fingerprint is printed.
- **Refused** — a precondition failed (unsafe path, collision, conflict, invalid span, missing selection); the source is byte-identical to its pre-image.
- **Failed** — an I/O or durability step failed mid-sequence; the message says which step and never uses the words `created`, `wrote`, `trashed`, or `restored`. **Target:** it also names the journal record to consult.

There is no fourth "probably worked" outcome, and no message claims durability before the corresponding `sync_all` returns.

### 3.3 Layout descriptions

Every human view uses the same top-to-bottom order, one fact per line, leading label then value:

1. outcome verb (`registered`, `selected`, `created`, `wrote`, `edited`, `trashed`, `restored`) or `error:`;
2. the vault name and, where relevant, its canonical root;
3. the affected vault-relative path(s);
4. operation facts — fingerprint before/after, byte length, span, recovery ID;
5. warnings, then exactly one suggested recovery command.

Component/data sources: `vault list` is driven by the XDG registry file; note receipts are driven by core return values (`SourceFingerprint`, `TrashReceipt`) and never by any index; trash inventory is driven by `.mg-vault/trash/entries/`.

Tabular views (`vault list`, `trash list`) are column-aligned at ≥ 60 columns and switch to one-record-per-block (`key: value` lines separated by a blank line) below 60 columns. **Nothing is ever truncated**: IDs, paths, fingerprints, error codes, and recovery commands wrap with a two-space continuation indent rather than being elided. JSON output never varies with terminal width.

Empty states are explicit sentences, not blank output:

- `No registered vaults. Run: mg-vault vault register NAME PATH`
- `Trash is empty.`
- `note read` of a zero-byte note emits zero bytes and exits 0; the JSON form reports `byte_len: 0`.

Path rendering is a single contract shared by all views (target: applied uniformly; today applied only on the index/search path): a UTF-8 path prints with control and non-printable characters escaped (`\xNN` / `\u{…}`) so that a filename can never inject terminal escape sequences; a non-UTF-8 path prints an escaped display form and, in JSON, an object `{"encoding":"unix_bytes_hex","value":"<hex>","display":"<escaped>"}` so identity survives losslessly. Escaping is presentation only — it never alters the bytes used to open, create, or rename a file, and `note read` output is never escaped.

### 3.4 Input & gestures

- **Modality:** keyboard/argv only. Touch, mouse, stylus, camera, voice, and game-controller input are N/A for a CLI; there are no in-app keyboard shortcuts because there is no persistent UI (D and E own modal keybindings).
- **Global flags:** `--vault NAME`, `--json`, `--no-input`, `--no-color`. Target additions: `--quiet` and `--verbose` (mutually exclusive).
- **Positional grammar:** `mg-vault <group> <verb> [OPERANDS] [FLAGS]` with groups `vault`, `note`, and (target) `trash`. Operand order is fixed and documented in `--help`.
- **`--no-input`** guarantees no prompt, no confirmation, no `/dev/tty` read, and no editor launch, for every command in this feature. This is currently trivially true — every command here is non-interactive — and the flag exists so that automation can lock the behavior against future additions. Missing required input is a typed `input_required`/usage error, never a prompt.
- **stdin/stdout:** `note read` writes bytes to stdout suitable for piping. **Target:** `note create` and `note write` accept `--body-file FILE` and `--stdin` as alternatives to `--body TEXT`; stdin is consumed only when `--stdin` is given, so an unrelated inherited pipe is never mistaken for note content, and exactly one body source may be supplied.
- **`--no-color` / `NO_COLOR`:** styling is disabled when `--no-color` is passed or the `NO_COLOR` environment variable is present with any value. Today no ANSI is emitted by any command in this feature, so the guarantee holds; **target** wires the flag into the argument parser's color choice as well, so that generated help and usage errors also honor it, and adds an automatic non-TTY default of no color. `--json` and `note read` output never contain styling under any setting.
- **Terminal width:** read from the terminal when stdout is a TTY, otherwise assumed 80. Width affects wrapping only.
- **Signals:** `SIGINT` before the first destructive step cancels with no mutation. During a commit the atomic sequence either completes or leaves a recoverable state; the tool never reports success merely because a signal arrived late.

### 3.5 Transitions & animation

N/A — this feature emits no animation, spinner, progress bar, cursor addressing, alternate screen, sound, or haptic. All output is append-only lines, which is inherently reduced-motion-safe and screen-reader-safe. Long operations do not exist in this slice (every command is bounded by one file or one small registry), so no progress affordance is required; if one is ever added it must be stderr-only, interactive-human-only, and absent under `--quiet`, `--json`, non-TTY, and reduced-motion environments.

### 3.6 Error states

| Code | Trigger | Presentation and recovery | Data-loss risk |
|---|---|---|---|
| `unsafe_path` | Absolute path, `..`/`.`/root/prefix component, non-`.md` extension, symlink at target or in the parent chain, escape from canonical root, `.obsidian`/`.mg-vault` first component in a mutation | stderr line naming the rejected operand and the rule violated; never prints the resolved outside-vault path | No — refused before any write |
| `collision` | `create` destination exists; `restore` destination exists; duplicate registry name | Names the occupied path; suggests another path or reading the existing note. No force flag exists | No — original bytes and trash payload untouched |
| `duplicate_root` (target) | Canonical root already registered under another name | Names the existing record; suggests `vault select` | No |
| `conflict` | `--expected` fingerprint differs from the observed source | Prints expected and actual digests and the path; suggests re-reading. **Neither version is chosen** | No — external edit preserved, proposed bytes not applied |
| `invalid_edit_span` | Reversed, out-of-bounds, or non-character-boundary span | Prints `start`, `end`, and current byte length | No |
| `invalid_utf8` | Note or replacement bytes are not valid UTF-8 | Names the path; explicitly refuses rather than decoding lossily | No |
| `no_vault_selected` / `unknown_vault` / `invalid_vault_name` | No resolvable vault, or a name that is not registered/valid | Names the resolution rule and the `vault register`/`vault select` command | No |
| `invalid_trash_entry` | Malformed ID, unreadable metadata, or unsupported metadata version | Names the ID; leaves payload and metadata untouched for manual recovery | No — never deletes an entry it cannot understand |
| `registry_corrupt` (target, split from `json`) | Registry file is unparsable or has an unknown newer `version` | Refuses to load; never rewrites the file with an older schema | No — old registry preserved verbatim |
| `home_unset` (target, split from `unsafe_path`) | `HOME` is unset and no XDG override supplies a path | Names the required environment variables | No |
| `invalid_fingerprint` (target, split from `unsafe_path`) | `--expected` is not `sha256:` + 64 hex | Names the expected format | No |
| `durability_unavailable` (target) | Directory `fsync` unsupported/failing on the target filesystem | Refuses the mutation *before* the first destructive step; status says `blocked` | No — no reduced-durability commit is offered |
| `cross_device` (target) | A future relocation would cross a filesystem boundary | Refuses; no copy-then-delete fallback | No |
| `transaction_incomplete` (target) | Journal shows an interrupted trash/restore | Blocks overlapping mutation on the same paths and names the recovery command | No if the protocol holds; orphans are quarantined, never discarded |
| `io` | Any filesystem failure (permission, ENOSPC, EIO) | States which step failed and whether the change committed, is uncommitted, or is `unknown`; never claims success | No false success; temp files are removed |

Presentation choice: every one of these is a single stderr block, because a CLI has no banner, toast, or modal, and inline-in-stdout errors would contaminate pipes and JSON consumers. In `--json` mode the same information is the error envelope of §4.3 on stderr with stdout left empty, so a consumer that only reads stdout cannot mistake a failure for an empty success.

### 3.7 Accessibility

- **Screen reader / linear reading:** all output is plain lines with a leading literal label. There are no interactive elements needing roles, traits, or custom actions; the "labels" are the field names (`path:`, `fingerprint:`, `selected:`, `error:`). Reading the output top to bottom conveys 100% of the information.
- **Color independence:** no state is conveyed by color. The selected vault is marked by the literal field `selected: yes` (target) in addition to the legacy `*` glyph; success/failure is conveyed by the outcome verb and the exit code, not by red/green. Stripping all ANSI from any output must leave it semantically complete — this is an executable test (§5.1).
- **Glyph independence:** no state depends on a Unicode symbol, box-drawing character, or emoji. Tables use spaces; no borders are required to parse a row.
- **Text scaling / dynamic type:** N/A in a terminal — the terminal owns font size. The equivalent obligation is width tolerance, met by the ≥ 60 / < 60 column layouts in §3.3 and tested at 40, 60, 80, and 120 columns.
- **Focus order and keyboard navigability:** the shell owns focus; there is no in-process focus model. Every capability is reachable from argv alone with no prompt, which is the CLI form of "keyboard complete". `--no-input` proves it.
- **Unicode safety:** paths and content pass through without normalization, case folding, or lossy conversion. Terminal control characters in *metadata* (paths, IDs) are escaped before display so a hostile filename cannot move the cursor, change colors, or spoof a prompt; `note read` content is byte-exact and never escaped, which is the correct trade because content goes to stdout as data.
- **No timing dependence:** nothing requires a timed response, and no output is transient.
- **Textual equivalents:** this feature produces no graph, canvas, image, chart, or spatial view, so there is nothing that lacks a textual equivalent. Every value it emits is already text.

---

## 4. Implementation Specification

### 4.1 Architecture placement

- `crates/mg-vault-core/` owns **all** filesystem authority for this feature:
  - `src/xdg.rs` — `XdgPaths` (config/data/state/cache + `registry_file()`).
  - `src/registry.rs` — `VaultRegistry`, `VaultRecord`, atomic persistence, name validation, `resolve()`.
  - `src/vault.rs` — `Vault` (canonical root), path validation, `create_note`, `read_note`, `write_note`, `edit_note_span`, `trash_note`, `restore_note`, `SourceFingerprint`, `Note`, `TrashReceipt`, and the private `prepare_destination` / `existing_mutation_path` / `ensure_beneath` guards.
  - `src/atomic.rs` — `create_atomic` (non-replacing), `replace_atomic`, `sync_parent`, temp-path minting.
  - `src/error.rs` — the `Error` enum that defines the public error taxonomy.
  - `src/frontmatter.rs`, `src/frontmatter_scalar.rs` — span *locators* for token-preserving edits. They validate and return byte ranges; they never serialize.
  - `src/lib.rs` — the public re-export surface.
- `crates/mg-vault-cli/src/main.rs` owns parsing and rendering only. It contains **no** filesystem policy: it never calls `std::fs` against a vault path, never constructs a path to write, and never decides confinement.
- `crates/mg-vault-index/` and `mg-vault-core/src/{index,interop}.rs` are **not** part of this feature. They are B-owned disposable projections that consume A's `Vault` and `SourceFingerprint`. A must not depend on them, and no A code path reads an index row. (Their present location inside `mg-vault-core` is a packaging accident recorded as a delta in §7.2, not an authority relationship.)
- Directional rule that keeps this true: dependencies point *into* core. `mg-vault-cli → mg-vault-core`; `mg-vault-index → mg-vault-core`; nothing points back out of `mg-vault-core`.

Filesystem layout owned here:

```
$XDG_CONFIG_HOME/mg-vault/vaults.json      # per-user registry + selection
$XDG_STATE_HOME/mg-vault/                  # per-user durable state (target: recovery copies)
$XDG_CACHE_HOME/mg-vault/                  # disposable per-user data (B's index lives here)
<vault>/                                   # ordinary user files — the authority
<vault>/.obsidian/                         # never written by mg-vault
<vault>/.mg-vault/                         # this app's portable, vault-scoped state
<vault>/.mg-vault/trash/files/<id>         # trashed payload bytes
<vault>/.mg-vault/trash/entries/<id>.json  # trash metadata (versioned)
<vault>/.mg-vault/journal/<txn>.json       # target: durable transaction records
<vault>/.mg-vault/recovered/<id>           # target: quarantined orphans
<vault>/.mg-vault/settings.json            # target: portable app settings
```

Two invariants make coexistence work: **portable, vault-scoped app state lives under `.mg-vault` and nowhere else in the vault**, and **disposable derived data lives in XDG cache outside the vault**, so copying a vault directory carries settings and trash but never a stale database.

### 4.2 Data model

Types as implemented/targeted in Rust (Jeff's conventions: 4-space indent, `// Verb + noun` above each function, section dividers above their block, constants in `ALL_CAPS_SNAKE_CASE`):

```rust
/// Application locations derived from the XDG Base Directory spec.
pub struct XdgPaths { config: PathBuf, data: PathBuf, state: PathBuf, cache: PathBuf }

/// A named, canonical vault root persisted in the per-user registry.
pub struct VaultRecord { pub name: String, pub path: PathBuf }

/// XDG configuration registry for independent vault roots.
pub struct VaultRegistry {
    path: PathBuf,                            // not serialized
    vaults: BTreeMap<String, VaultRecord>,    // ordered => deterministic bytes
    selected: Option<String>,
    // target: version: u16 = 1, fail closed on an unknown newer value
}

/// Canonical authority boundary around one ordinary vault directory.
pub struct Vault { root: PathBuf }

/// Stable digest of the exact source bytes observed by a caller.
pub struct SourceFingerprint(String);        // "sha256:" + 64 lowercase hex

/// Exact note bytes plus the fingerprint that authorizes a later replacement.
pub struct Note { pub bytes: Vec<u8>, pub content: String, pub fingerprint: SourceFingerprint }

/// Opaque handle returned by a successful trash operation.
pub struct TrashReceipt { pub id: String, pub original_path: PathBuf }

/// Persisted trash metadata. Versioned; an unknown version fails closed.
struct TrashMetadata { version: u8, original_path: PathBuf, fingerprint: SourceFingerprint }

/// Exact source spans for a YAML frontmatter envelope and Markdown body.
pub struct FrontmatterEnvelope { yaml: Range<usize>, body: Range<usize>, line_endings: LineEndings }
pub enum LineEndings { Lf, CrLf, Mixed }

// --- target additions ---

/// One trash entry as reported to a caller, including detected health.
pub struct TrashEntry {
    pub id: String, pub original_path: PathBuf, pub byte_len: u64,
    pub fingerprint: SourceFingerprint, pub health: TrashHealth,
}
pub enum TrashHealth { Ok, OrphanPayload, OrphanMetadata, UnreadableMetadata }

/// Durable record used to finish or reverse an interrupted multi-step change.
pub struct TransactionRecord {
    pub version: u16, pub id: TransactionId, pub operation: OperationKind,
    pub phase: TransactionPhase, pub paths: Vec<PathBuf>,
    pub preconditions: Vec<SourceFingerprint>,
}
pub enum TransactionPhase { Prepared, PayloadMoved, MetadataWritten, Committed }
```

**Persisted schemas.** `vaults.json` is `{"vaults": {"<name>": {"name","path"}}, "selected": "<name>|null"}` (target adds `"version": 1`). Trash metadata is `{"version":1,"original_path":"…","fingerprint":"sha256:…"}`. Both are versioned; an older binary that meets an unknown newer version **refuses to load and refuses to rewrite**, so downgrading cannot silently destroy state. There is **no note-content migration, no note database, and no schema that owns note bytes** — the only migrations that can ever exist here are for registry, trash, and journal metadata, and each must be atomic, versioned, reversible-by-preservation, and covered by a backward-compatibility fixture.

**Identity.** A note's identity is its vault-relative path. Nothing in this feature writes an ID, UUID, checksum, or property into note content. Trash IDs (`<128-bit-ns-hex>-<counter-hex>`, 49 chars) and transaction IDs are operational handles for `.mg-vault` bookkeeping only; they are never note identity and never appear in a note. Fingerprints are computed from content, so they are recomputable by any tool with `sha256sum` and are not an injected identifier.

### 4.3 API contracts

Public core functions (all return `Result<T, Error>`):

```rust
// Resolve application locations from the process environment.
fn XdgPaths::from_env() -> Result<XdgPaths>;
fn XdgPaths::from_values(home, config, data, state, cache) -> XdgPaths;
fn XdgPaths::registry_file(&self) -> PathBuf;

// Manage the per-user registry of canonical vault roots.
fn VaultRegistry::load(path: &Path) -> Result<VaultRegistry>;
fn VaultRegistry::register(&mut self, name: &str, path: &Path) -> Result<VaultRecord>;
fn VaultRegistry::select(&mut self, name: &str) -> Result<()>;
fn VaultRegistry::list(&self) -> Vec<VaultRecord>;
fn VaultRegistry::selected_name(&self) -> Option<&str>;
fn VaultRegistry::resolve(&self, requested: Option<&str>) -> Result<PathBuf>;
fn VaultRegistry::unregister(&mut self, name: &str) -> Result<VaultRecord>;   // target

// Operate on exactly one confined note inside one canonical vault.
fn Vault::open(path: &Path) -> Result<Vault>;
fn Vault::root(&self) -> &Path;
fn Vault::create_note(&self, relative, bytes: &[u8]) -> Result<SourceFingerprint>;
fn Vault::read_note(&self, relative) -> Result<Note>;
fn Vault::write_note(&self, relative, bytes, expected: &SourceFingerprint)
    -> Result<SourceFingerprint>;
fn Vault::edit_note_span(&self, relative, span: Range<usize>, replacement,
    expected: &SourceFingerprint) -> Result<SourceFingerprint>;
fn Vault::trash_note(&self, relative) -> Result<TrashReceipt>;
fn Vault::restore_note(&self, id: &str) -> Result<TrashReceipt>;
fn Vault::list_trash(&self) -> Result<Vec<TrashEntry>>;                      // target
fn Vault::recover(&self, mode: RepairMode) -> Result<RecoveryReport>;        // target

// Locate exact byte spans without normalizing or reserializing source.
fn scan_frontmatter(source: &str) -> Result<Option<FrontmatterEnvelope>, FrontmatterError>;
fn locate_frontmatter_scalar(source: &str, key: &str)
    -> Result<Range<usize>, FrontmatterScalarError>;
```

**Error cases** are the `Error` variants in §3.6. **Auth/permissions:** there is no network, no account, no role, and no rate limiting; authority is filesystem permissions plus this crate's confinement rules. The privilege boundary that matters is internal: `.mg-vault` is reachable *only* through the specific private helpers (`internal_trash_dir` and, target, the journal/settings accessors) and is unreachable through any caller-supplied path, because `validate_note_path(_, mutation = true)` rejects it. No caller — including a future plugin or AI adapter — can construct a `Vault` mutation that targets the control directory or an out-of-root path, because those types are only produced by core code from an already-canonicalized root. **Pagination:** N/A for single-note operations; `list_trash` returns a fully ordered vector and pagination is deferred to C if entry counts ever justify it.

**JSON envelope (version 1, binding).**

```json
{"version":1,"ok":true,"command":"note.create","data":{ … },"warnings":[]}
{"version":1,"ok":false,"error":{"code":"conflict","message":"…","details":{ … }}}
```

`version`, `ok`, and (target) `command` and `warnings` are always present; `warnings` is always an array, possibly empty. Success goes to stdout, error to stderr, and the other stream stays empty. Digests are `sha256:` + 64 lowercase hex. Byte counts are unsigned JSON integers. Paths are vault-relative UTF-8 strings, or the `unix_bytes_hex` object of §3.3 when not representable. Removing a field, changing a type, or moving a stream requires envelope version 2; adding a documented optional field does not. Every command, no-op, warning, and error code gets a golden fixture, and those fixtures are the compatibility contract.

**Exit codes (target categories; aligned with C so one taxonomy serves the whole CLI):** `0` success/no-op, `2` usage/invalid input, `3` not found or no selection, `4` collision/conflict/ambiguity, `5` unsafe or denied, `6` degraded dependency, `7` I/O or transaction failure, `130` interrupt. Exit code and JSON error code are asserted together in tests.

### 4.4 State management

- **Authoritative state:** the ordinary files in the vault directory. Nothing else. There is no server, no sync, and no cloud copy, so there is no local/server boundary to reconcile in this feature.
- **Owner:** `Vault` owns per-operation state only; it holds a canonical root and nothing cached. Every operation revalidates the path at use time rather than trusting a previously validated handle, so a directory swapped between two commands cannot be exploited across invocations. There is no long-lived store, controller, or view model — deliberately, because a cached tree would become a second authority.
- **Portable vault-scoped state:** `.mg-vault/` (trash, target journal/settings). It travels with the vault, is preserved by `.obsidian`-agnostic tools, and is unreachable via generic note paths.
- **Per-user state:** the XDG registry (which vaults exist, which is selected). Selection is *convenience*, not identity: losing `vaults.json` loses no note data and is fully repaired by re-registering directories.
- **Derived/disposable state:** none is owned here; B's SQLite projection lives under XDG cache keyed by a fingerprint of the canonical root, and deleting it must be harmless. No A code path reads it.
- **Concurrency model:** optimistic. Every replacement carries the caller's `SourceFingerprint`; a mismatch is a `conflict` and both versions survive (the on-disk one untouched, the proposed one still in the caller's hands). **Target:** conflict proposals from long-running callers are spooled to an owner-only file under `$XDG_STATE_HOME/mg-vault/` and named in the error, so a rejected write is recoverable rather than lost. There is no lock that makes a fingerprint check optional — external programs need not honor any lock, so the check stays mandatory.
- **Transaction/recovery protocol (target):** write and `fsync` a journal record before the first destructive step; update the phase durably; delete the record only after every affected file and parent directory is synced. Recovery is idempotent and re-runnable. **Recovery never writes to a path whose current bytes differ from the journal's recorded precondition** — such content is quarantined under `.mg-vault/recovered/<id>` with an owner-only mode and reported, and the operator must choose an explicit destination. Where the tool cannot prove whether a step committed, it reports `unknown`, blocks overlapping writes on those paths, and preserves every version it holds.
- **Offline/draft persistence:** everything here is offline by construction (§6.1, §5A). The only "draft" concept is the retained conflict proposal above.

### 4.5 Dependencies

- **Crates (all present in `Cargo.toml`, all offline):** `clap` 4.5 (parsing), `serde` + `serde_json` 1 (registry/trash/envelope), `sha2` 0.10 (fingerprints), `thiserror` 2 (error taxonomy), `rustix` 1 (`renameat2(RENAME_NOREPLACE)`; target: `openat2`-based confinement), `yaml-edit` 0.2 (frontmatter scalar *location* only — never serialization). Workspace lints already set `unsafe_code = "forbid"` and deny `clippy::all` + `clippy::pedantic`.
- **Not depended on by this feature:** `rusqlite` (B), any network, HTTP, TLS, or telemetry crate, any Markdown renderer, any terminal UI framework, any async runtime.
- **New for the target:** a reviewed descriptor-relative confinement layer (may be `rustix` alone) and, if adopted, a `shell-words`-equivalent only if this feature ever spawns a process — which it currently does not and should not.
- **Assets:** none. No fonts, images, models, icons, or bundled data files.
- **Infrastructure:** none. No database, daemon, service, CDN, or third-party endpoint. CI runs `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace --all-targets --all-features` on a clean machine.

### 4.6 Platform-specific considerations

- **Primary target:** Arch Linux. Core behavior is portable to Linux and macOS; Windows is out of scope for this milestone (path semantics, `RENAME_NOREPLACE`, and directory `fsync` all differ enough to need their own gate).
- **Non-replacing rename:** implemented via `rustix::fs::renameat_with(RenameFlags::NOREPLACE)` on Linux/Android/Apple/Redox, with a `hard_link` + `unlink` fallback elsewhere. Both are collision-refusing; the fallback is rejected for filesystems without hard-link support, which fails closed rather than degrading to a replacing rename.
- **Directory durability:** `sync_parent` opens the parent directory and calls `sync_all`. Filesystems where this is unsupported must be detected at open time; **target:** such a filesystem returns `durability_unavailable` before the first destructive step rather than committing with weaker durability and claiming success.
- **Confinement backend:** the current, accepted portable approach canonicalizes and re-checks containment at each use boundary and rejects symlinks with `symlink_metadata` on the final component and each parent component. This defeats traversal, symlink escape, and stale-path reuse across invocations. It does **not** defeat a hostile process that swaps a directory entry between the check and the operation. The accepted target is a descriptor-relative `VaultDir` capability: `openat2` with `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS | RESOLVE_NO_MAGICLINKS` on Linux, and an `openat` component walk with `O_NOFOLLOW` plus `(dev, ino)` pinning on macOS. A precedent already exists in-tree: the index projection's confined reader uses exactly the `openat2` flags above, so the mutation path can adopt the same primitive. Until it does, this limitation is stated in `docs/SECURITY.md`, in §7, and in §8 — it is a named deferral, not an unstated assumption.
- **Case sensitivity and Unicode normalization** vary by filesystem. Identity is the exact byte path; nothing is case-folded or NFC/NFD-normalized. A case-insensitive filesystem may therefore report `collision` where a case-sensitive one would not — this is correct fail-closed behavior and is documented.
- **Version compatibility:** Rust edition 2024, `rust-version = "1.85"`. No OS-version-gated APIs beyond the `renameat2`/`openat2` availability handled by the `cfg` split above.
- **Feature flags / rollout:** confinement backend selection may be a compile-time `cfg`, but **JSON shape, error codes, and safety behavior must never vary silently by build**; a build without the hardened backend must say so in `--version`/status rather than quietly offering weaker guarantees.

### 4.7 Performance budget

Reference: warm local SSD, single invocation, reported with hardware/OS metadata.

- **Startup:** `--help` and argument errors p95 ≤ 50 ms with zero filesystem access to any vault or registry.
- **Registry:** load + resolve p95 ≤ 100 ms with 1,000 registered vaults; the registry is a single small JSON file read once per invocation and is O(number of records).
- **Note operations** on notes ≤ 1 MiB: `read` p95 ≤ 50 ms; `create`/`write`/`edit-span` commit p95 ≤ 100 ms excluding unavoidable `fsync` variance. **Durability is never weakened to meet a latency number** — if the budget and `fsync` conflict, the budget loses.
- **Trash/restore:** metadata-path p95 ≤ 150 ms for same-filesystem operations.
- **Memory:** ≤ 20 MiB RSS for any command in this feature. `edit_note_span` allocates one output buffer sized by checked arithmetic (`len - span + replacement`), so peak is bounded by roughly 2× note size; a streaming replacement for very large notes is a target improvement, and the current implementation must not require more than two in-memory copies.
- **Storage:** registry is a few hundred bytes per vault; each trash entry costs the original note bytes plus ~200 bytes of metadata; journal records (target) are ≤ 4 KiB each and are deleted on commit. Nothing here grows without an explicit user action.
- **Network payloads:** none — there is no network code path (§5A).
- **Scale posture:** every operation in this feature is O(1) in vault size — no command here walks the vault. This is the property that lets a 100,000-note vault stay responsive for editing while B's index rebuilds; the 100,000-note/1,000,000-block budgets themselves belong to B and are asserted there, and a regression test here asserts that no A command performs a recursive directory walk.

---

## 5. Test Specification

### 5.1 Unit tests

Implemented today in `crates/mg-vault-core/tests/foundation.rs` and `tests/frontmatter_scalar.rs`; target additions marked.

| Test | Setup → assertion (edge case covered) |
|---|---|
| `xdg_paths_honor_overrides` | Set each `XDG_*` var and clear them → each resolved directory matches the override, and the `HOME` fallback yields `~/.config/mg-vault` etc. (environment precedence) |
| `xdg_missing_home_is_typed` *(target)* | Unset `HOME` and all overrides → typed `home_unset`, not a panic or a relative path (missing-environment edge) |
| `registry_round_trips_and_selects_canonical_vault` | Register through a symlinked path → stored root is canonical; reload from disk returns the same records and selection (persistence round-trip) |
| `registry_rejects_duplicate_name` | Register the same name twice → `collision`, registry bytes unchanged (idempotence under error) |
| `registry_rejects_duplicate_root` *(target)* | Register one canonical root under two names, including via a symlink alias → `duplicate_root` (aliasing ambiguity) |
| `registry_rejects_unknown_schema_version` *(target)* | Write `{"version":2,…}` → refuses to load and refuses to rewrite (downgrade safety) |
| `registry_name_validation` | Empty, space, `/`, `..`, non-ASCII names → `invalid_vault_name` (injection through a name) |
| `rejects_traversal_absolute_and_protected_mutations` | `../x.md`, `/etc/x.md`, `.obsidian/app.md`, `.mg-vault/trash/x.md`, `note.txt` → `unsafe_path` for every mutation entry point (traversal + control-directory + extension) |
| `rejects_symlink_escape` | Symlink inside the vault pointing outside; symlinked parent directory → `unsafe_path` on read and on every mutation (symlink escape) |
| `create_refuses_collision_and_read_is_byte_exact` | Pre-create a file, then `create_note` → `collision` and original bytes intact; read of CRLF/emoji/no-trailing-newline content → bytes identical (unconfirmed overwrite; byte fidelity) |
| `optimistic_write_rejects_changed_source` | Read, mutate externally, then `write_note` with the old fingerprint → `conflict` and the external bytes survive (stale write) |
| `atomic_write_changes_fingerprint_and_content` | Successful replacement → returned fingerprint equals SHA-256 of the new bytes and the file matches (receipt truthfulness) |
| `narrow_edit_preserves_all_bytes_outside_the_selected_span` | Splice into a note containing frontmatter, callouts, code fences, CRLF → prefix and suffix are byte-identical (unknown-syntax preservation) |
| `narrow_edit_rejects_invalid_utf8_boundaries_without_mutation` | Span mid-codepoint → `invalid_edit_span`, file unchanged |
| `narrow_edit_rejects_reversed_and_out_of_bounds_spans_without_mutation` | `start > end`, `end > len` → typed error, file unchanged |
| `narrow_edit_rejects_invalid_existing_utf8_without_mutation` | Non-UTF-8 file → `invalid_utf8`, file unchanged |
| `narrow_edit_rejects_invalid_utf8_replacement_without_mutation` | Non-UTF-8 replacement → `invalid_utf8`, file unchanged |
| `narrow_edit_rejects_a_stale_fingerprint_without_mutation` | Changed source → `conflict`, file unchanged |
| `trash_restore_round_trip_and_collision_refusal` | Trash then restore → exact bytes and original path; recreate the path first, then restore → `collision` with payload and metadata intact (recovery never overwrites) |
| `trash_id_validation` *(target)* | Wrong length, non-hex, path-shaped, and `../` IDs → `invalid_trash_entry` before any filesystem access (ID as an injection vector) |
| `trash_metadata_unknown_version_is_refused` *(target)* | `version: 2` metadata → refuses and leaves the entry untouched |
| `locates_only_the_existing_scalar_token` / `translates_yaml_offsets_across_crlf_frontmatter` / `ignores_nested_keys_when_locating_a_top_level_property` | Frontmatter fixtures → exact byte range of the scalar; CRLF offsets correct; nested key not matched (span accuracy) |
| `rejects_duplicate_target_keys` / `rejects_missing_frontmatter_and_missing_keys` / `rejects_non_scalar_targets_and_malformed_yaml` | Ambiguous or absent targets → fail closed with a typed error, no range returned (ambiguity never guesses) |
| `frontmatter_bom_and_unclosed_fail_closed` | BOM prefix, `---` with no closer → `UnsupportedBom` / `Unclosed` (never guessed into an editable region) |
| `fingerprint_parse_is_strict` *(target)* | Wrong prefix, 63/65 hex, uppercase, non-hex → `invalid_fingerprint`; uppercase input normalizes to lowercase on the accepted form |
| `no_command_walks_the_vault` *(target)* | Instrument `read_dir` → zero recursive walks for every A command (scale posture) |

### 5.2 Integration tests

Run against synthetic temporary vaults and fake XDG homes; no user data and no network.

- `registry_persistence_is_atomic` — kill the process at each write phase of register/select/unregister → the file always parses and equals either the complete old or the complete new state; never partial JSON.
- `create_is_never_partially_visible` — inject failure after temp write, after `sync_all`, and after rename → the destination either does not exist or contains the complete bytes; no temp file is left at a note path and no `.tmp` file survives a failed create.
- `write_fault_matrix` — inject failure at each of temp-write, temp-`fsync`, rename, parent-`fsync` → the note contains exactly the old or exactly the new bytes; a failure at the parent-`fsync` step is reported as `unknown durability`, never as success.
- `two_process_conflict` — two processes read the same note and both write with the same expected fingerprint → exactly one commits and one gets `conflict`; there is no last-writer-wins outcome.
- `trash_interruption_leaves_recoverable_state` — kill between rename and metadata write → the payload is discoverable, the note is not silently gone, and (target) the journal reconciles it idempotently on the next run; repeated recovery converges.
- `restore_never_overwrites_newer_source` — trash a note, write a *newer* note at the same path, then restore → `collision`, the newer file is byte-identical to what was written, and the trash entry is fully intact.
- `orphan_trash_is_quarantined_not_deleted` *(target)* — delete the metadata, then the payload, in separate runs → `list_trash` reports the health state and nothing is discarded.
- `control_directories_are_untouched` — populate `.obsidian/` with app settings and run the full command matrix → every `.obsidian` file has an unchanged mtime and byte content, and every attempted mutation into it is `unsafe_path`.
- `mg_vault_state_stays_inside_the_control_dir` — after the full matrix, the only new vault paths are under `.mg-vault/` (plus intended note paths); assert the set difference explicitly.
- `no_identifier_is_injected` — create, write, edit-span, trash, and restore a note, then compare the final bytes to the expected exact bytes → no UUID, ID, timestamp, checksum, or property was added anywhere in the file.
- `disposable_index_does_not_affect_source` — delete B's database entirely, run the A matrix → every A command behaves identically; A never reads or requires the index.
- `cli_json_envelope_is_versioned` — every command in human and `--json` mode → success on stdout with empty stderr, error on stderr with empty stdout, one trailing newline, matching golden fixture.
- `exit_codes_match_error_codes` *(target)* — one case per error code → asserted exit category and JSON code together.
- `no_color_contract` — `--no-color`, `NO_COLOR=`, `NO_COLOR=1`, TTY and non-TTY, human/`--json`/raw read → zero ANSI bytes in any stream, including generated help and usage errors *(target: help/usage)*.
- `no_input_never_prompts` — every command under `--no-input` with a prompt spy installed → zero interaction calls and zero `/dev/tty` opens.
- `unsafe_symlink_swap` *(target, Linux)* — race a directory entry swap against each mutation → descriptor-relative confinement prevents escape, or the operation fails closed.
- `platform_confinement_gate` *(target)* — traversal, in-root symlink, external symlink, hard-link substitution, ancestor swap, final-entry swap, mount crossing, case-fold collision, crash durability → the mutation backend may not advertise support unless every gate passes.

### 5.3 UI / E2E tests

There is no graphical UI; the E2E surface is the process boundary. Tests drive the built binary, some through a pseudo-terminal:

1. Full happy path — `vault register` → `vault select` → `note create` → `note read` → `note write` → `note trash` → `note restore` — asserting stdout/stderr placement, exit codes, and byte-exact round-trip of the note.
2. Error recovery path — every §3.6 error triggered end-to-end, asserting the message names the rule, the recovery suggestion is a runnable command, and the vault is unchanged.
3. Navigation equivalent — `--vault NAME` vs. persisted selection produce identical results and the override does not persist.
4. Non-TTY invocation with stdin, stdout, and stderr all redirected → no prompt, no pager, no color, clean stream separation.
5. PTY invocation at 40, 60, 80, and 120 columns → no truncation of IDs, paths, fingerprints, or codes; block layout below 60.
6. Screen-reader-linear check — strip ANSI, read every output top to bottom, assert the semantic content is complete without color or column position.
7. Hostile-filename check — filenames containing ESC, C0/C1 bytes, RTL overrides, combining marks, and non-UTF-8 bytes → metadata is escaped or hex-encoded, `note read` content is byte-exact, and no terminal state is altered.
8. Piping check — `mg-vault note read a.md | sha256sum` equals `sha256sum` of the file; downstream closing the pipe exits conventionally without contaminating stdout.

### 5.4 Visual / manual verification

- **Theme variants:** N/A in the usual sense — no color is emitted. The equivalent check is running every view on a light and a dark terminal profile and confirming legibility depends only on the terminal's own colors, plus confirming `--no-color`/`NO_COLOR` produce byte-identical output to the default.
- **Text size extremes:** N/A (terminal-owned). Substituted by the width matrix below.
- **Screen size extremes:** 40, 60, 80, 120, and 200 columns; 10-row and 60-row terminals; confirm no truncation and no dependence on scrollback.
- **Empty vs. populated:** empty registry, one vault, 50 vaults; empty vault, one note, deep nested directories; empty trash, one entry, 50 entries; empty note, 1 MiB note.
- **Content extremes:** LF, CRLF, mixed endings, no trailing newline, BOM, emoji, combining marks, RTL text, and a note that is only frontmatter.
- **Coexistence:** open the same vault in Obsidian before and after the full command matrix; confirm Obsidian's settings and workspace load unchanged and that Obsidian sees the notes as ordinary files.
- **Portability:** copy the vault directory to another machine and confirm trash and settings travel with it while nothing stale is carried in.
- **Gates:** `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-targets --all-features`.

---

## 6. Compliance & Safety Gate

### 6.1 Sensitive data classification

- [ ] No sensitive data involvement
- [x] **Handles sensitive data** — note bodies, note titles, vault-relative paths, and vault root locations may be private (journals, client notes, credentials pasted into a note). Protections: everything stays on the local filesystem; **there is no network code path in any dependency of this feature**; no telemetry, crash reporting, or analytics exists; note content is emitted only when the user explicitly runs `note read` (or `--json` on it) and is never included in receipts, warnings, error messages, error `details`, or any log; error messages name paths and rules but never bytes; temporary files are same-directory, uniquely named, and removed on every failure path; target recovery/conflict spools are created owner-only under `$XDG_STATE_HOME`. The `.mg-vault` control directory contains user content only in the form of trashed payloads, which are the user's own files awaiting restore.
- [ ] Uses synthetic/test data only until compliance gate clears

### 6.2 Asset provenance

- [x] **No third-party assets** — no fonts, images, icons, models, sounds, or bundled datasets. Rust crate dependencies (§4.5) are implementation libraries, not creative or data assets; each must still pass repository dependency-license policy, which is tracked with the unresolved project-license question in §8.
- [ ] Uses third-party assets

### 6.3 Language / claims audit

- [x] **No unsupported claims.** §7 separates implemented behavior from target behavior with explicit state words, and every "target" marker in §3–§5 is a commitment, not a description of today.
- [x] **No promised-but-unbuilt capability is presented as available.** Permanent purge, journaled transactions, descriptor-relative hardening, `trash list`, `note edit-span` as a CLI verb, duplicate-root detection, and stream/body flags are all labeled target.
- [x] **"Atomic" is used precisely** — it means the scoped same-filesystem temp/`fsync`/rename/parent-`fsync` protocol for one file, and it is fault-tested. It is never used to describe a multi-file operation, because this feature offers none.
- [x] **Output verbs are earned.** `created`, `wrote`, `edited`, `trashed`, `restored` are printed only after the corresponding durability step returns; failures never use them.
- [x] **No regulated-domain language.** This is a personal knowledge tool; no medical, financial, legal, or safety claim is made.

### 6.4 Regulatory alignment

The template points at criteria.md Lens 3. Lens 3 is largely out of this slice, so each Lens 3 criterion is addressed explicitly as either an A-owned enabling property or a named deferral to **B (index service)** and **G (links, search, and graph)** — never as fake coverage:

- **3A Determinism** — *A-owned enabling property.* `SourceFingerprint` gives every derived layer a deterministic, recomputable token for "which bytes did I derive from", and path identity gives a stable key. A itself derives nothing. Full determinism of links, backlinks, search, graph, and views is **deferred to B and G**.
- **3B Ambiguity** — *A-owned analogue, plus deferral.* A's ambiguity rule is that ambiguity fails closed and never mutates: a duplicate frontmatter key, an unclosed envelope, or a BOM refuses to yield an editable span rather than picking one. **Link-target ambiguity is deferred to G**, which must likewise never silently choose or mutate.
- **3C Query depth** — **N/A here; deferred to B and G.** A exposes no query surface at all: no text, title, regex, tag, property, path, relationship, task, or date query. This is deliberate — A must stay O(1) in vault size.
- **3D Derived authority** — *A-owned enabling property.* A is the reason derived layers can never become authority: nothing in A reads an index, a database, or a cache to answer a question about source, and B's projection lives outside the vault under XDG cache so deleting it is harmless. Schemas, Bases, formulas, and graphs remain views over ordinary files by construction. **Their own contracts are deferred to B, G, and I.**
- **3E Scale** — *A-owned enabling property.* No A command walks the vault (§4.7), so editing stays responsive independent of vault size; a regression test asserts this. **The 100,000-note / 1,000,000-block budgets and their evidence are deferred to B.**

Architecture note showing this foundation does not preclude Lens 3: fingerprints, path identity, confined reads, and byte-exact source access are exactly the inputs a deterministic index needs, and B already consumes them (`MarkdownIndex`, `PersistentIndexStore::verify_freshness`). Nothing in A caches, normalizes, or reserializes source, so no future retrieval layer inherits a lossy input.

Lens 2 is likewise out of this slice and is addressed by name below rather than left blank.

**Binding acceptance traceability (all five lenses).**

| Criterion | A-owned acceptance, or explicit deferral |
|---|---|
| **1A Authority** | Ordinary `.md` files are the only source read or written. No database, cache, or index is consulted by any A operation; B's projection is disposable, lives outside the vault, and deleting it changes no A behavior (`disposable_index_does_not_affect_source`). |
| **1B Preservation** | Reads and writes are `Vec<u8>`; `edit_note_span` copies prefix and suffix verbatim; frontmatter tooling *locates* spans and never serializes; whole-document reserialization is prohibited by the accepted spike; LF/CRLF/mixed endings are reported and preserved, never normalized; BOM, unclosed envelopes, and duplicate keys fail closed. |
| **1C Identity** | Vault-relative path is the public identity. No UUID, ID, checksum, or property is ever written into a note (`no_identifier_is_injected`). Trash and transaction IDs are `.mg-vault` bookkeeping handles; interop `global_id` values are *derived* from path and namespace at export time and are never persisted into source. Fingerprints are content digests any tool can recompute. |
| **1D Coexistence** | `.obsidian` is never written — mutation rejects it as a first component and the index walk skips it entirely (`control_directories_are_untouched`). mg-vault's own portable state is isolated under `.mg-vault/`; disposable derived data lives in XDG cache outside the vault (`mg_vault_state_stays_inside_the_control_dir`). |
| **1E Transactions** | Create refuses collisions with a non-replacing rename; replacement requires a matching fingerprint; restore refuses an occupied destination; span edits reject invalid boundaries; writes are temp → `fsync` → rename → parent `fsync`; every failure path removes temp files and leaves the pre-image. Target journaling makes the two-step trash/restore recoverable and idempotent. No multi-file mutation is offered, so none can be partial. |
| **2A Keyboard completeness** | Every capability is reachable from argv with no prompt; `--no-input` locks it. Modal editing grammar is **deferred to D**. |
| **2B Editing durability** | A supplies the durable primitive: fingerprint-checked atomic commit, conflict refusal that preserves both versions, and trash-backed recovery. Autosave, persistent undo, and external-edit merge are **deferred to D**, which must build them on these APIs. |
| **2C Workspace** | **N/A — deferred to E.** A has no panes, tabs, splits, preview, or session state and does not preclude them: `Vault` is terminal-independent, holds no UI state, and can be driven by many concurrent front ends. |
| **2D Text correctness** | A preserves Unicode bytes exactly, enforces character-boundary spans, and never normalizes. Grapheme-safe cursor and structural operations are **deferred to D**, which receives byte-accurate spans and fingerprints from A. |
| **2E Degraded experience** | With no editor, TUI, or index present, direct source read/create/write/span-edit/trash/restore all work, and nothing claims a capability it lacks (target: `--version`/status names the confinement backend). |
| **3A–3E** | See the itemized paragraphs above: enabling properties owned here, query/retrieval semantics **deferred to B and G**. |
| **4A Confinement** | Relative-only, normal-components-only, `.md`-only paths; symlink rejection at the final component and every parent; canonicalize-and-recheck at each use boundary; `.obsidian`/`.mg-vault` mutation rejected; control directory reachable only through private core helpers. Descriptor-relative hardening is a named target (§4.6, §8-Q2). |
| **4B Concurrency** | Mandatory `SourceFingerprint` on every replacement; mismatch is `conflict`, the on-disk version is untouched, and the proposed version stays with the caller (target: spooled and named). No lock substitutes for the check. |
| **4C Least privilege** | There is no plugin host, no AI adapter, and no network in this feature — deny by default because the capability does not exist. The internal boundary is enforced structurally: no caller-supplied path can reach `.mg-vault`, and mutation types are constructible only by core from a canonicalized root. Any future plugin/AI must go through these same APIs and inherits every check. |
| **4D Recovery** | Trash is reversible and refuses to overwrite; nothing is permanently deleted by this feature; failures never claim success; target journaling makes interrupted operations idempotently recoverable and quarantines orphans instead of discarding them. |
| **4E Contracts** | One versioned JSON envelope, stable error codes, target exit categories, versioned registry and trash schemas that fail closed on unknown newer versions, golden fixtures as the compatibility contract, and a lossless encoding for non-UTF-8 paths. |
| **4F Privacy** | No network, telemetry, or logging of content; content leaves only through an explicit `note read`; errors name paths and rules, never bytes; temp and recovery files are owner-only and cleaned up. |
| **5A Offline/local-first** | Every workflow is filesystem-only. No dependency in this feature opens a socket; the tool works with networking disabled, and this is asserted by dependency review rather than assumed. |
| **5B Responsiveness** | Explicit startup, latency, memory, and storage budgets in §4.7, plus the O(1)-in-vault-size rule and a regression test for it. Cancellation is trivial (single short operations) and `SIGINT` semantics are specified in §3.4. |
| **5C Accessible equivalents** | A emits no graph, canvas, media, or card view — every value it produces is already text with a stable field label. |
| **5D Terminal resilience** | 40/60/80/120-column layouts with no truncation, literal state words with no color dependence, no glyph-only meaning, terminal-control escaping for hostile filenames, lossless hex encoding for non-UTF-8 paths, and no animation to reduce. |
| **5E Automation** | Human and versioned JSON output on separate, disciplined streams; byte-exact `note read` for piping; target `--stdin`/`--body-file` input; `--no-input`, `--no-color`, and `NO_COLOR` all honored and fixture-tested. |

**Auto-fail review, by name.**

- **Source-content loss** — every failure path leaves the exact pre-image; temp files are removed; no operation truncates in place; fault matrices (§5.2) assert the post-condition byte-for-byte.
- **Partial multi-file mutation** — this feature offers **no** multi-file operation. Trash and restore touch a payload and a metadata file, and that pair is the one two-step sequence; it is bracketed by rollback today and by a durable journal in the target, and an unreconciled state is reported rather than assumed complete.
- **Unconfirmed overwrite** — `create` uses a non-replacing rename, `restore` refuses an occupied destination, and `write`/`edit-span` require a matching fingerprint. There is no `--force` and no overwrite prompt anywhere in this feature.
- **Unsafe traversal or symlink escape** — rejected at path validation, at parent-chain walking, and again at canonical containment re-check; symlinks are rejected on the final component and on every parent; tested by `rejects_traversal_absolute_and_protected_mutations` and `rejects_symlink_escape`. The residual concurrent-swap window is named in §4.6, §7, and §8-Q2 and is closed by the target backend.
- **Non-atomic save claiming success** — success is printed only after `sync_all` on the file and on the parent directory returns; a failure at any step reports the step and, where it cannot prove the outcome, says `unknown`. No message uses a success verb otherwise.
- **Recovery overwriting newer source** — `restore` has no force flag and fails closed on collision, keeping both the newer file and the trash entry (`restore_never_overwrites_newer_source`). Target journal recovery additionally refuses to write any path whose current bytes differ from the recorded precondition, quarantining instead.
- **Silent conflict winner** — a fingerprint mismatch is a typed `conflict` that reports both digests and applies nothing. Two concurrent writers produce exactly one commit and one conflict (`two_process_conflict`); there is no last-writer-wins path.
- **Index state overriding source / stale index presented as current** — no A operation reads an index, so neither is reachable from this feature; B owns freshness truthfulness.
- **Unknown syntax loss** — no parser serializes; edits are byte splices outside declared spans; ambiguous or unparseable structures refuse to produce a span.
- **Capability/data-exfiltration bypass, active raw HTML/script, graph without textual equivalent** — N/A: no plugin host, no renderer, no network, and no graphical output exist in this feature.

### 6.5 Security controls

- Treat every caller-supplied path, note byte, vault name, and trash ID as untrusted input; validate shape before touching the filesystem (a malformed trash ID must be rejected before it is joined onto a path).
- Escape terminal control characters in all displayed metadata; never escape `note read` content, which is data on stdout.
- Keep `unsafe_code = "forbid"` and `clippy::pedantic` denied workspace-wide; both are already configured.
- Bound resource use: reject absurd spans through checked arithmetic (already implemented in `edit_note_span`), and fail closed on malformed registry/trash JSON rather than allocating unbounded structures or attempting a destructive repair.
- Never spawn a process, never build a shell command string, and never place note content in argv, environment, process titles, or diagnostics. This feature spawns nothing today and must not start.
- Restrict `.mg-vault` access to the specific private helpers; adding a new accessor requires the same validation path as note operations.
- Where the tool cannot prove a filesystem outcome, prefer refusing and quarantining over guessing — no automatic repair may discard the only known copy of any bytes.

---

## 7. Gap Analysis vs. Current State

### 7.1 What exists today

State words are exact: implemented / prototyped / planned / gated / absent.

**Implemented** (commits `bb2b723` → `dfe33cf`; files as listed):

- `crates/mg-vault-core/src/xdg.rs` — XDG config/data/state/cache resolution with per-variable overrides, `HOME` fallback, and `registry_file()`. **Implemented.**
- `crates/mg-vault-core/src/registry.rs` — JSON registry, ASCII-safe name validation, canonicalizing `register`, duplicate-*name* refusal, `select`, `list`, `selected_name`, `resolve`, first-registration auto-select, atomic persistence via `create_atomic`/`replace_atomic`. **Implemented.**
- `crates/mg-vault-core/src/vault.rs` — canonical `Vault::open`; `validate_note_path` (relative, normal components only, `.md` extension, `.obsidian`/`.mg-vault` rejected for mutation); `prepare_destination` (component-by-component parent walk, symlink and non-directory rejection, directory creation with parent `fsync`, canonical containment re-check); `existing_mutation_path` (`symlink_metadata` + canonicalize + containment); `create_note`, `read_note`, `write_note`, `edit_note_span`, `trash_note`, `restore_note`; `SourceFingerprint` with strict parsing. **Implemented.**
- `crates/mg-vault-core/src/atomic.rs` — same-directory unique temp file, `write_all`, `sync_all`, `renameat2(RENAME_NOREPLACE)` on supported platforms with a `hard_link`+`unlink` fallback, replacing rename for updates, `sync_parent`, temp cleanup on every failure. **Implemented.**
- `crates/mg-vault-core/src/error.rs` — the typed `Error` taxonomy (`UnsafePath`, `Collision`, `Conflict`, `NoVaultSelected`, `UnknownVault`, `InvalidVaultName`, `InvalidUtf8`, `InvalidEditSpan`, `InvalidTrashEntry`, `Io`, `Json`). **Implemented.**
- `crates/mg-vault-core/src/frontmatter.rs`, `frontmatter_scalar.rs` — exact envelope/body span scanning with LF/CRLF/mixed detection, BOM and unclosed-envelope refusal; top-level scalar span location via `yaml-edit` as a locator only, with duplicate-key, missing-key, non-scalar, and invalid-YAML refusals. **Implemented.**
- `crates/mg-vault-cli/src/main.rs` — `vault register/list/select`; `note create/read/write/trash/restore`; global `--vault`, `--json`, `--no-input`, `--no-color`; version-1 success envelope `{version, ok, data}` on stdout and error envelope `{version, ok, error{code,message}}` on stderr; per-variant error codes; escaped/hex path rendering helpers (`escape_path`, `path_json`). **Implemented.**
- `crates/mg-vault-core/tests/foundation.rs` (13 tests) and `tests/frontmatter_scalar.rs` (6 tests), plus `crates/mg-vault-cli/tests/cli.rs` — XDG overrides, registry round-trip, traversal/absolute/protected-mutation rejection, symlink escape, create collision, byte-exact read, stale-fingerprint conflict, atomic replacement, six span-edit refusal cases, trash/restore round-trip and restore-collision refusal, and versioned JSON flow. **Implemented.**
- `docs/PRODUCT.md`, `docs/ARCHITECTURE.md`, `docs/SECURITY.md`, `docs/spikes/token-preserving-markdown-yaml.md`. **Implemented.**

**Prototyped:**

- Descriptor-relative confined reads exist, but only on B's projection path: `mg-vault-core/src/index.rs::read_source_bytes` uses `openat2` with `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS` on Linux and refuses to open at all on other platforms. The mutation path does not yet use it. **Prototyped** (proves the primitive; not yet the authority backend).
- `note edit-span` exists as the core API `Vault::edit_note_span` with full test coverage but has **no CLI verb**. **Prototyped** at the library level.

**Gated:**

- `--no-color` and `--no-input` are accepted and their guarantees currently hold, but only because no command prompts and no A command emits ANSI. `NO_COLOR` is documented in the flag help and is **not read** by the program, and the flag is not wired into the argument parser's own color choice, so generated help and usage errors can still be colored by the parser's default. **Gated** on the color-policy wiring in §7.2.

**Absent:**

- Duplicate-root detection, `unregister`, a `version` field in `vaults.json`, `trash list`, `Vault::list_trash`, transaction journaling, orphan quarantine, `Vault::recover`, permanent purge, stable exit-code categories (every failure is exit `1`), `command`/`warnings` in the JSON envelope, `--stdin`/`--body-file`, `--quiet`/`--verbose`, `vault path`, streaming replacement for very large notes, and the confinement/durability capability gate. **Absent.**
- Distinct error codes for `home_unset`, `invalid_fingerprint`, `duplicate_root`, `registry_corrupt`, `durability_unavailable`, `transaction_incomplete`. Today `XdgPaths::from_env` and `SourceFingerprint::from_str` both report `UnsafePath`, which is a mislabel. **Absent.**
- A repository `LICENSE` file — the MIT vs. Apache-2.0 choice is unresolved (§8-Q1). **Absent.**

**Note on packaging accuracy:** `mg-vault-core` currently also contains `index.rs` and `interop.rs`, which are B/P concerns, not foundation authority. Their presence in the core crate does not make derived data authoritative — no A code path reads them — but the coupling should be broken (§7.2) so the authority crate stays minimal and auditable.

### 7.2 Delta to spec

**New files / modules**

- `crates/mg-vault-core/src/journal.rs` — `TransactionRecord`, phase-durable write/update/clear, idempotent `recover()`, orphan quarantine into `.mg-vault/recovered/`.
- `crates/mg-vault-core/src/trash.rs` — split trash/restore/list out of `vault.rs`; add `TrashEntry`, `TrashHealth`, `list_trash`.
- `crates/mg-vault-core/src/confine.rs` — the `VaultDir` descriptor-relative capability (`openat2` on Linux; `openat` + `O_NOFOLLOW` + `(dev, ino)` pinning on macOS) plus the capability gate that blocks mutation when the backend cannot pass its suite.
- `crates/mg-vault-cli/src/{cli,output/human,output/json}.rs` — split parsing from rendering; centralize the path-rendering and color-policy contract.
- `crates/mg-vault-core/tests/{transactions,confinement,registry_schema}.rs` and `crates/mg-vault-cli/tests/{contracts,pty}.rs` — the fault, race, schema, and terminal matrices of §5.
- `tests/fixtures/json/*.json` — golden envelope fixtures per command and error code.

**Modified files**

- `src/registry.rs` — add `version: 1`, unknown-newer refusal, canonical duplicate-root detection, `unregister`, and a distinct duplicate-name error that names the *name* rather than the registry path.
- `src/vault.rs` — route every mutation through `VaultDir`; bracket trash/restore in journal records; return `TrashEntry` data; keep the current canonical checks as a defense-in-depth second layer rather than removing them.
- `src/atomic.rs` — detect and report unsupported directory `fsync` as `durability_unavailable` before the first destructive step; add a streaming replacement path for large notes.
- `src/error.rs` — add `HomeUnset`, `InvalidFingerprint`, `DuplicateRoot`, `RegistryCorrupt`, `TrashMetadataCorrupt`, `DurabilityUnavailable`, `ConfinementUnavailable`, `TransactionIncomplete`; stop overloading `UnsafePath` for environment and parsing failures; split `Json` by source.
- `src/xdg.rs` — return `HomeUnset` instead of `UnsafePath`.
- `src/lib.rs` — export the new types; **remove `index` and `interop` from this crate** and move them to `mg-vault-index` / a P-owned crate so the authority crate has no derived-data code.
- `mg-vault-cli/src/main.rs` — add `note edit-span`, `trash list`, `vault unregister`, `vault path`; add `--stdin`/`--body-file`/`--quiet`/`--verbose`; add `command` and `warnings` to the envelope; map errors to exit categories; wire `--no-color` and `NO_COLOR` into the parser's color choice; route **all** path output through the escaping/hex contract (today `note create`/`write`/`trash` use `Path::display()` and serde's UTF-8-only `PathBuf` serialization, which is inconsistent with the index path's `path_json` and fails on non-UTF-8 paths).
- `docs/SECURITY.md`, `docs/ARCHITECTURE.md`, `README.md` — record the confinement backend, capability gate, journal protocol, and exit categories once they land.

**Migrations / schema changes**

- `vaults.json`: add `"version": 1` on next write; a file without `version` is read as version 1 (backward compatible); a file with an unknown newer version fails closed and is never rewritten.
- Trash metadata stays at `version: 1`; the journal is new at `version: 1`. **No note-content migration exists or may ever exist.**

**New dependencies**

- None strictly required beyond the existing `rustix` (which already provides `openat2`). No network, database, or async dependency may be added to this feature.

### 7.3 Estimated scope

**M–L.** The API surface is small and roughly half the target is already implemented and tested, which caps the size. What pushes it above S is that the remaining work is precisely the adversarial part: descriptor-relative confinement with a real capability gate, a durable journal with idempotent recovery and quarantine, a durability-unavailable refusal path, and a fault/race test harness that can inject failure at each `fsync`/rename boundary and race directory-entry swaps. Those cannot be validated by ordinary unit tests and cannot be rushed, because every one of them is an auto-fail boundary. Recommended slicing: (1) error taxonomy + exit categories + envelope fields + path-rendering unification; (2) registry version/duplicate-root/unregister; (3) journal + trash/restore/list + recovery + quarantine; (4) `VaultDir` confinement backend + capability gate; (5) durability gate + streaming replacement; (6) crate decoupling of `index`/`interop`. Each slice is independently reviewable and shippable.

### 7.4 Blocking dependencies

- **None external.** This is the root of the dependency tree: it depends on no other feature, no service, no network, and no unreleased crate. Everything in §7.2 can be built against the current workspace.
- **Blocks:** B (index service) needs `Vault`, `SourceFingerprint`, and confined reads; C (CLI and note operations) needs the plan/commit-grade primitives, journal, and error/exit/JSON contracts and explicitly names A's journaling and race hardening as incomplete; D (editor engine) needs atomic fingerprint commits and conflict retention for autosave/undo; E (TUI) needs terminal-independent core APIs; H (refactoring) needs a multi-file transaction built on the journal; M (Git/Syncthing) needs trash/history semantics; N and O need the confinement boundary to enforce least privilege.
- **Sequencing note:** H, and any future multi-file operation, must not begin until slice (3) — the journal — lands, because a multi-file mutation without a durable journal is exactly the "partial multi-file mutation" auto-fail.
- **Non-blocking external gate:** the project license decision (§8-Q1) blocks publishing a `LICENSE` file and a dependency-license policy sign-off, not implementation.

### 7.5 Non-goals

- No index, database, search, query, link resolution, backlinks, or graph — B and G.
- No editor, buffer, undo journal, keymap, pane, tab, split, preview, or session restore — D and E.
- No Markdown or YAML *serialization*. Span location is in scope; whole-document or whole-frontmatter rewriting is prohibited by the accepted spike and by 1B.
- No multi-span, multi-file, or batch mutation; no move, rename, append, extract, split, or merge — C and H.
- No permanent purge, no empty-trash, no wildcard or prefix deletion.
- No network, sync, Git, Syncthing, backup, plugin host, AI adapter, publishing, or Quickshell coupling.
- No hidden note identifier, no content-owning database, no force overwrite, no last-writer-wins, and no automatic repair that discards the only copy of any bytes.
- No claim of protection against a malicious root or kernel; the supported threat model is hostile *caller input* and concurrent filesystem-entry manipulation within documented OS capabilities.

---

## 8. Open Questions

- **Q1:** Project license — MIT or Apache-2.0 (or dual)? — **blocks:** §6.2 dependency-license policy sign-off and the repository `LICENSE` file. Does **not** block implementation. Carried unchanged from iteration 1.
- **Q2:** Confirm the accepted staging of descriptor-relative hardening: the current portable canonicalize-and-recheck boundary is accepted for this foundation's scope, but it does not eliminate hostile concurrent directory-entry races; the `VaultDir` backend in §4.6/§7.2 closes it. Should mutation be **gated off entirely** on platforms where the hardened backend is unavailable (the fail-closed position specified in §4.6), or should those platforms keep today's behavior behind an explicit, loudly labeled opt-in? — **blocks:** §4.6 platform gate and §7.2 slice (4). Carried unchanged from iteration 1.
- **Q3:** Should adding `command` and `warnings` to the version-1 JSON envelope be treated as an additive v1 change (consumers must ignore unknown fields) or as envelope version 2? The spec currently assumes additive-v1 with golden fixtures. — **blocks:** §4.3 envelope contract and the fixture set.
- **Q4:** Where should a retained conflict proposal live — `$XDG_STATE_HOME/mg-vault/` (per-user, does not travel with the vault) or `.mg-vault/recovered/` (portable, travels with the vault but is visible to Obsidian and sync)? §4.4 currently specifies XDG state for conflict spools and `.mg-vault/recovered/` for journal quarantine, which is deliberate but worth confirming. — **blocks:** §4.4 and §7.2 slice (3).
- **Q5:** Should `index.rs` and `interop.rs` move out of `mg-vault-core` in this feature's slice (6), or is that deferred to B's own restructuring? The authority argument favors moving them; the churn argument favors deferring. — **blocks:** §7.2 slice (6) sequencing only.
- **Q6:** For a case-insensitive filesystem, should `create` treat a case-variant of an existing filename as a `collision` (fail closed, current behavior falls out of the non-replacing rename) or as a distinct path? §4.6 specifies fail-closed; confirm this is the desired user experience. — **blocks:** §4.6 and the `platform_confinement_gate` case-fold case.
