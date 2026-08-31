# Scorecard: Editor Engine

**Feature ID:** d-editor-engine
**Spec file:** gauntlet-output/specs/d-editor-engine.md
**Reviewer agent:** blind verification agent (Spec Gauntlet, mg-vault)
**Date:** 2026-08-30
**Spec iteration reviewed:** 2

---

## Verdict: PASS

**Summary:** The spec's strongest quality is that it makes durability a structural
invariant rather than a promise: §4.2's fixed Milestone 4 policy stages every mutation
invisibly until an `fdatasync` on an already-linked journal file returns, rolls back
text/cursor/registers/undo/revision on barrier failure, and forbids any timed batching
mode — and §5.1 `mutation_ack_requires_synced_journal` fault-injects that at every byte
boundary. Combined with §3.2's "Recovery never writes automatically" and §4.3's
reread-and-fingerprint-check before every save/import/merge acceptance, none of the
four auto-fail conditions this feature is most exposed to are reachable. The most
critical gap is not architectural but contractual: §3.4's closed `:` command list omits
any keyboard path to `accept_external`/`accept_merge` — the two actions §3.2 makes
mandatory for every changed external result — and its freshness vocabulary and
`--output json` flag do not reconcile with the `Freshness{Empty,Current,Stale,Degraded}`
enum and `--json` flag already shipped in `mg-vault-index` and `mg-vault-cli`.

---

## Lens 1: Data Ownership and Format Compatibility (weight: 30%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 1A Authority | 3 | §4.4 Source boundaries: "Authority: current ordinary file bytes and fingerprint read through `TextStore`/core; no cache, parse tree, index, watcher event, temp file, plugin, AI result, or client-provided hash can substitute". §4.3 "Only `SourceRead` from `TextStore` can establish or advance `BaseSnapshot`" and "Watch notifications cannot authorize a save or reload". §4.4 demotes persistent undo to "convenience history admitted only through the exact content-hash gate". §5.2 authority-inversion matrix forges watcher metadata, index results, Tree-sitter spans, protocol hashes and asserts none advances `BaseSnapshot`. Verified compatible: `Vault::read_note`/`write_note` in `crates/mg-vault-core/src/vault.rs` already provide exactly this fingerprint-gated read/replace pair. | — |
| 1B Preservation | 3 | §4.5 token-preserving dependency contract: structural edits are unavailable until `mg-vault-markdown` returns `TokenSpan { revision, byte_range, token_kind, surrounding_hash }`, the editor validates revision/hash/UTF-8-grapheme edges/kind/non-overlap and "never prints or reserializes an AST/document/frontmatter block"; "Unknown tags, properties, comments, whitespace, quoting, line endings, Obsidian wikilinks/embeds/callouts, and unsupported YAML must remain byte-identical outside accepted ranges". §4.2 "line endings are retained as loaded ... without normalizing untouched lines". §5.1 `structural_spans_are_token_preserving`, §5.2 Obsidian/source preservation. This matches the shipped `docs/spikes/token-preserving-markdown-yaml.md` decision and `Vault::edit_note_span`. | — |
| 1C Identity | 3 | §4.1 "Public identity is canonical vault identity plus public vault-relative path; the editor must not inject UUIDs, frontmatter, sidecars, or metadata into a note". §4.2 persistent undo is "keyed by canonical vault identity plus public relative path—not injected into notes". §7.2 "no metadata injection into notes are permitted"; §7.5 non-goal bars "hidden UUID injection". | Minor: §4.2 uses `<vault-key>/<path-key>` for XDG state directories without defining the path→key encoding; §4.2 only handles the failure mode ("path-identity ambiguity rejects/quarantines that history"). Specify the encoding. |
| 1D Coexistence | 3 | §4.1 "`.obsidian/**` is a protected coexistence directory: opening it as an editable buffer, external handoff targeting it, and any editor-originated create/replace/remove operation fail `ProtectedControlPath`. Its existing bytes are otherwise ignored and preserved"; portable settings confined to `.mg-vault/**`, also protected; "recovery/undo/temp state remains outside the vault in XDG locations". §5.2 control-directory coexistence exercises open/save/recovery/external handoff/purge plus "hostile traversal/symlink aliases" and asserts both trees "remain byte-for-byte unchanged". Consistent with the shipped guard in `validate_note_path` (`crates/mg-vault-core/src/vault.rs`). | — |
| 1E Transactions | 3 | §4.3 "A mutating command either commits one complete `EditTransaction` or leaves buffer/register/undo/journal state unchanged. Register changes caused by an operator commit atomically with its text change". §4.2 barrier failure "rolls back text, cursor, registers, undo, revision, and derived invalidations and returns `JournalDurabilityFailed`; it must not emit an acknowledgment"; compaction "interruption leaves at least one valid generation"; cap pressure "never deletes the sole recoverable generation" and instead pauses editing with `RecoveryStorageFull`. §3.6 error table gives every row an explicit data-loss column. §5.2 crash matrix asserts "never false save success or overwrite of newer source". | — |

**Lens average:** 3.00
**Lens pass:** Yes — avg ≥ 2.0, zero 1s, zero 0s

---

## Lens 2: Editor and TUI Excellence (weight: 25%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 2A Keyboard completeness | 2 | §3.4 enumerates a genuinely finite grammar — modes, nine operators with doubled linewise forms, ~20 motions, text objects including Tree-sitter `ih`/`ah`/`ic`/`ac`/`il`/`al`, counts with a checked 1,000,000 default cap, the full register set with an explicit persistence policy, macros with four named limits, marks/jumps, search, and `.` repeat with an explicit exclusion list — and §6.3 forbids marketing it as Vim-compatible. **Defect:** §3.4's command line says "only `/`, `?`, and the following `:` commands are in scope" and then lists 22 commands that do **not** include accept/reject of an external import or a merge, while "Unsupported Vim/Ex commands return `UnsupportedCommand`". §3.2 steps 5–6 make `accept_external(candidate_id)` and `accept_merge` mandatory for every changed external result, so the closed grammar has no keyboard path to the feature's own load-bearing action. §3.7's blanket "Every operation has a stable command ID" contradicts the closed list rather than resolving it. Second defect: `q!` and `e!` are listed but their discard semantics are never defined anywhere in the spec. | Add `:accept`, `:reject`, `:accept-merge` (or equivalents, with candidate/hunk arguments) to §3.4's `:` list, and define `q!`/`e!` discard semantics explicitly — including that they leave the recovery journal generation intact per §4.2 retention. |
| 2B Editing durability | 3 | All four required elements are specified and fault-tested. **Atomic autosave:** §3.2 step 5 + §4.4 (suspension during incomplete command/macro/handoff/merge/recovery, coalescing, one save per buffer) + §4.3 "Save success is emitted only after it returns the new fingerprint" + §5.2 atomic-autosave-conflict asserting "B remains byte-exact, and merge contains A/B/C". **Recovery:** §4.2's fixed sync-before-acknowledgment barrier, read-only open with `DurableJournalUnavailable` if the platform cannot guarantee it, and the explicit note that grouped insert fragments "recover as one undo node, so undo ergonomics never create an acknowledged-loss window"; §5.1 `mutation_ack_requires_synced_journal` and §5.2 crash matrix inject at record write, commit marker, fsync, compaction rename, file rename, and directory fsync. **Persistent undo:** §4.2 loads "only when the current source content hash exactly matches a recorded durable save node", quarantines on mismatch, "never modifies source merely to make history fit"; §4.4 branching undo preserves alternate futures; §5.1 `persistent_undo_requires_exact_hash`. **External-edit merge:** §3.2 external-change flow steps 4–6 with base/local/external retained and a second fingerprint gate at save; §5.2 `fixtures/conflicts/` covering Git-checkout and Syncthing-style cases. | — |
| 2C Workspace | 2 | Defensible deferral with a real forward contract: §3.1 defines five named view models (editor buffer, command line, merge session, recovery offer, diagnostics/outline) and binds the consumer — "E may lay them out but may not weaken their status labels, ordering, source revision, freshness, or available actions" — while explicitly excluding "graph, preview, tab/split layout, theme, command palette, or workspace restoration UI"; §4.4 "E owns multiple editor instances and focus; app owns watcher subscriptions and path lifecycle"; §7.5 names E as owner. **Gap:** session restore is the one listed sub-criterion with no obligation at all. §4.2's persistent-state list covers recovery, undo, opted-in registers, and temp artifacts but not cursor, marks, jump list, or undo head, and §4.4 never says which engine state is serializable across sessions — so E cannot build session restore against this contract. | State in §3.1 or §4.4 which engine state is session-serializable (cursor/marks/jump list/undo head/search history) and what identity a restored buffer must present, so E's session restore has a defined seam. |
| 2D Text correctness | 3 | This is a grapheme-cluster model, not an assertion. §4.2 types it: `GraphemePos { byte, grapheme, line }` documented as "A valid position between extended grapheme clusters"; `TextEdit.range` is "UTF-8 byte boundaries and grapheme-safe edges"; "Internal edits use UTF-8 byte ranges only after checking scalar boundaries and required grapheme-edge invariants". §4.5 names `unicode-segmentation` for extended grapheme clusters and confines `unicode-width` to "client display projections" — the correct separation. §3.4 makes `h`/`l` grapheme motions, `r{grapheme}` a grapheme replace, marks "revision-aware grapheme anchors ... they never point into a grapheme", and block operations "defined over logical grapheme columns, not terminal cell halves". §4.5's structural contract re-validates "UTF-8/grapheme edges" before applying a `TokenSpan`, so structural ops inherit the invariant. §3.2 rejects invalid UTF-8 as `UnsupportedEncoding` rather than lossy-decoding. §5.1 `grapheme_cursor_never_splits_cluster` (combining marks, ZWJ, flags, skin tones, variation selectors, RTL, CJK) and `byte_grapheme_line_maps_round_trip_after_random_edits` against a reference model. | Polish only: no Unicode normalization policy (NFC/NFD) is stated for search comparison or IME/paste input, and `w`/`e`/`b` word boundaries for scriptless-space languages (CJK, Thai) are deferred to `docs/EDITOR.md` without naming the segmentation rule. |
| 2E Degraded experience | 3 | §4.7 "A budget miss degrades derived structure/diagnostics first, never text correctness, journaling, conflict detection, or ability to save/export source". §4.6 "Plain editing, internal registers, journal, save, and merge are mandatory core behavior; no Milestone 4 feature flag enables network diagnostics". §3.6 parse/grammar/diagnostic-failure row: "Degraded status; structural commands disabled ... Continue plain-text editing/retry parser", data-loss "None". §3.4 structural text objects "fail visibly rather than guessing"; retrieval absence "returns `unavailable`/`stale` without blocking local dispatch, open, save, recovery, or export". §5.2 Tree-sitter degradation (missing grammar, parser panic, huge fence) asserts "source editor/search/save remain functional"; §5.3 script 7. | — |

**Lens average:** 2.60
**Lens pass:** Yes — avg ≥ 2.0, zero 1s, zero 0s

---

## Lens 3: Knowledge Retrieval and Structure (weight: 20%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 3A Determinism | 3 | §4.3 "Diagnostics and syntax results are accepted only if `result.revision == buffer.revision`; stale work is discarded", and every protocol response carries buffer `revision` plus an index watermark (§3.4). §6.4 Lens 3A: "structural ranges are derived from exact buffer revisions ... Search behavior, grammar, protocol ordering, source revision, and index watermark are fixture-defined". §3.4 search is "UTF-8 and grapheme-aligned", regex is a linear-time engine with size/complexity limits, empty pattern repeats prior. §5.1 `syntax_and_diagnostics_drop_stale_revisions` completes async jobs out of order and asserts only the current revision projects. Motion inclusive/exclusive semantics are deferred to `docs/EDITOR.md`, but to a named artifact that §7.2 schedules and §5.1 `grammar_table_accepts_only_documented_sequences` table-drives against. | — |
| 3B Ambiguity | 3 | §3.4 "Ambiguous references return all candidates and require an explicit path choice". §4.5 missing/stale/overlapping/ambiguous spans "return `StructuralTargetUnavailable` with zero mutation". §3.4 structural objects "fail visibly rather than guessing" when syntax is unavailable. §6.4 Lens 3B: "external conflicts require explicit resolution rather than a heuristic winner". §3.2 external-change step 5: overlaps "require explicit per-hunk or whole-file decisions"; step 6 blocks save until resolved. §5.1 `knowledge_results_are_read_only_and_freshness_typed` fakes ambiguity; §5.1 `structural_spans_are_token_preserving` generates ambiguous spans. | — |
| 3C Query depth | 2 | The nine kinds are named, not dodged: §3.4 "Its versioned query kinds cover text, titles, regex, tags, properties, paths, relationships, tasks, and dates and return stable path/block references plus an index watermark and freshness"; §6.4 Lens 3C restates them and scopes D to contract-testing every kind; §5.1 "fake every query kind"; within-buffer text/literal/regex is owned outright (§3.4 search). **Gap:** the shape is never specified. §4.3 declares only `fn query(&self, request: KnowledgeQuery, cancel: CancelToken) -> KnowledgeTask` with `KnowledgeQuery` opaque, and §4.2 defines no variant, result reference type, or watermark field. A B/G implementer cannot build to this seam from the spec, and "versioned" has no version field anywhere. | Write the `KnowledgeQuery` enum (one variant per named kind), the result reference type (path + optional block/byte range), and the watermark/freshness fields into §4.2, with an explicit version discriminant. |
| 3D Derived authority | 3 | §4.1 "Tree-sitter, indexes, plugins, AI, and diagnostics are derived clients of immutable snapshots and never obtain a `TextStore`, journal handle, mutable `Editor`, or source-write capability". §4.4 lists "Tree-sitter tree, folds, outline, diagnostics, spellcheck, search caches, display coordinates" as derived/non-authoritative. §3.4 "Links, backlinks, graph, Bases, and formulas are derived source views and can only navigate/select ranges; they cannot issue text edits or authorize saves". §6.4 Lens 3D. §5.2 authority-inversion matrix asserts no derived artifact "authorizes save/import, or overrides a fresh `TextStore` reread". Matches the shipped invariant in `docs/ARCHITECTURE.md` ("no database can become source authority"). | — |
| 3E Scale | 3 | §3.4 states the number and the isolation guarantee: "Provider absence or load at the 100,000-note/1,000,000-block target returns `unavailable`/`stale` without blocking local dispatch, open, save, recovery, or export". §6.4 Lens 3E assigns actual indexing to B/G while keeping the non-blocking gate in D. §5.2 "Retrieval isolation at target scale" runs a deterministic fake provider "delayed, saturated, stale, cancelled, and unavailable" and asserts "local p95 dispatch/open/save budgets remain within the editor limits" — an executable gate, not a claim. §4.7 supplies the per-note budgets those assertions measure against. | — |

**Lens average:** 2.80
**Lens pass:** Yes — avg ≥ 2.0, zero 1s, zero 0s
**Auto-fail triggered:** No

---

## Lens 4: Security, Reliability, and Extensibility (weight: 15%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 4A Confinement | 3 | §4.1 the editor "does not perform unconstrained filesystem traversal. `mg-vault-core` remains the sole path and durable-write authority"; `.obsidian`/`.mg-vault` mutation fails `ProtectedControlPath`. §4.2 recovery/undo under `$XDG_STATE_HOME`, "mode `0700` directories and `0600` files". §4.6 "If private runtime storage cannot be established, external handoff fails closed or uses a private state directory with a warning; it never uses a predictable world-readable temp path". §5.2 exercises "hostile traversal/symlink aliases" and "journal/undo/temp path symlink substitution". §7.4 is honest about the residual limit — "hostile directory-entry race hardening remains a known platform limitation and must not be overstated" — which matches the real caveat in `docs/SECURITY.md`. | — |
| 4B Concurrency | 3 | §4.3 "Every save, external acceptance, recovery acceptance, and merge acceptance rereads through `TextStore` and checks the expected fingerprint immediately before mutation/write". §3.2 external-change step 6: "The final save still requires the current external fingerprint, so another change restarts merge rather than overwriting". §3.2 external-editor step 6: a pre-acceptance change refuses acceptance and opens a merge — "No side silently wins". Both versions survive: §3.2 step 5 "Base, local, and external remain exportable". §5.2 atomic-autosave-conflict asserts "merge contains A/B/C"; §5.1 `autosave_revision_race_stays_dirty`. Feasible against the shipped `Error::Conflict { expected, actual }` in `crates/mg-vault-core/src/error.rs`. | — |
| 4C Least privilege | 3 | §4.3 "Plugin and AI adapters are likewise excluded from execution in D. Future adapters must receive an explicit capability object scoped to named vault-relative paths, byte ranges, operation IDs, expiry, and read-only/read-proposal rights; default is no content and no network ... They can never hold raw `TextStore`, clipboard, process, query, or network capabilities transitively". Proposals are "immutable textual diffs requiring explicit user acceptance and a fresh fingerprint, carry provider attribution, and pass through the same journal/save gates". §4.3 confines spellcheck to "immutable scoped snapshots" with "no filesystem or network capability". §5.1 `capabilities_are_deny_by_default_and_scoped` tests undeclared paths/ranges, network, clipboard, process, store, logs, and expired operation IDs. | — |
| 4D Recovery | 3 | §3.2 crash-recovery step 4 "Recovery never writes automatically"; step 5 replays only "through the last valid committed transaction" and retains original journal bytes for export. §3.2 external step 4 makes every changed result an "immutable `ExternalImportCandidate`" — "Merely returning from the editor never mutates the buffer". §6.3 "no message may claim saved/recovered/copied/cleaned up before read-back or adapter confirmation"; §3.2 step 7 "failed cleanup is reported without claiming deletion". §4.2 retention keeps three valid dirty generations and removes them "only after a durable save plus verified clean close". §3.6 marks source-deleted as "no implicit recreation". | — |
| 4E Contracts | 2 | Strong in the abstract: §3.4 protocol v1 is newline-delimited JSON with `protocol_version`, `request_id`, stable command ID, `allow_input`, and responses carrying `ok`, `result`/`error`, `accepted_base_hash`, `revision`, freshness; "Unknown versions or fields required for semantics fail closed; additive response fields are allowed"; stdout purity with stderr diagnostics; the truthfulness rule that `accepted_base_hash` "must never be presented as a claim about current disk bytes unless that response performed a fresh `TextStore` read"; §5.1 `protocol_v1_jsonl_is_stable_and_noninteractive_safe` with golden schemas. **Two concrete compatibility holes against shipped code.** (1) §3.4's freshness set is `current`/`stale`/`unavailable`/`not_applicable`; `mg-vault-index` already ships `Freshness { Empty, Current, Stale, Degraded }` (`crates/mg-vault-index/src/lib.rs:53`) and the CLI emits `"status": "degraded"` — the spec's set has no `degraded` and no mapping rule, so a truthful degraded index cannot be represented. (2) §3.4 requires the CLI adapter to offer `--output json`; the shipped CLI is `--json` with a `version: 1` envelope (`crates/mg-vault-cli/src/main.rs`, `struct Cli { json: bool }`, and `README.md`). Neither divergence is acknowledged or scheduled in §7.2. | In §3.4, either adopt the shipped `empty/current/stale/degraded` vocabulary or state the total mapping from it to the four editor values; and either use `--json` or add an explicit compatibility note in §7.2 reconciling `--output json` with the shipped `--json` + `version: 1` envelope. |
| 4F Privacy | 3 | §6.1 classifies note text, search terms, registers, undo, journals, clipboard payloads, dictionaries, diagnostics, temp files, and conflict versions as sensitive, then lists measures: "no content in routine logs, panic reports, metrics, filenames derived from content, or error decoration ... Crash reports contain identifiers/hashes only when needed and never source excerpts by default". §3.4 "Human diagnostics go to stderr, never corrupt JSON stdout, and never include note text unless the explicit command is an export/read operation". §4.3 OSC52 is "write-only, disabled by default ... and never attempts terminal-response scraping". §4.2 "no private clipboard content is persisted implicitly". §5.2 diagnostic-privacy asserts "only the exact selected snapshot reaches local diagnostics, ambient note/vault content is absent, logs are redacted"; clipboard matrix asserts "sensitive text is absent from logs". | — |

**Lens average:** 2.83
**Lens pass:** Yes — avg ≥ 2.0, one 1-or-below? none; zero 0s

---

## Lens 5: Performance and Accessibility (weight: 10%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 5A Offline/local-first | 3 | §4.7 "Milestone 4 performs no network access". §4.3 "Local spellcheck is the only Milestone 4 diagnostic backend and remains available offline; it receives immutable scoped snapshots and has no filesystem or network capability", and "the remote LanguageTool adapter is excluded until a later capability review". §4.6 "no Milestone 4 feature flag enables network diagnostics". §4.5 "There is no database, CDN, cloud service, or asset required". §5.2 diagnostic privacy asserts "no network capability exists in Milestone 4" as a test, not a policy statement. | — |
| 5B Responsiveness | 3 | §4.7 gives numbers with a stated measurement basis ("optimized builds on the project reference Arch machine and reported with corpus/hardware metadata"): p95 ≤ 8 ms / p99 ≤ 16 ms dispatch on a 1 MiB/50,000-line note with "no full-buffer clone on each keystroke"; 10 MiB open to editable state ≤ 500 ms with async syntax; incremental syntax ≤ 16 ms p95, cancellable; literal search ≤ 100 ms in 10 MiB; memory ≤ 4× source + 64 MiB after idle compaction; durable-ack p95 ≤ 50 ms / p99 ≤ 150 ms, where "a miss causes a visible slow-storage state rather than batching or weakening durability". §4.7 requires cancellation and memory/input caps across macro, parser, syntax, diagnostics, merge, retrieval, and external work. | — |
| 5C Accessible equivalents | 3 | §3.7 enumerates the complete textual surface — "mode, path, logical line, grapheme column, selection shape/extent, dirty/save state, pending command, active register, macro status, search position/count, diagnostics, folds, and merge/recovery status in deterministic focus order" — and fixes that order explicitly ("path → mode → position/selection → dirty/save state → pending input → search/diagnostics → merge/recovery actions"), with "no hidden hover, color, pane geometry, or timing conveys unique information". §3.1 makes the projections "the single accessibility and automation seam" that E "may not weaken". §3.3 requires `clean`/`dirty`/`saving`/`saved`/`save failed`/`external change`/`merge required`/`recovery available` to be distinguishable without color. Critically, §3.7 accepts accessibility **in D**, not in E: a "screen-reader transcript adapter and 40×10-equivalent line-budget consumer that exercises every command, recovery, merge, and external-acceptance action", asserted by §5.1 `accessible_projection_has_complete_linear_equivalent`. | — |
| 5D Terminal resilience | 3 | §3.5 "Reduced-motion behavior is therefore identical to default behavior" — the correct answer for a headless engine, not a dodge. §3.7 "No text-size assumptions exist in the engine ... block selections use logical columns so cell-width rendering differences do not corrupt edits", which is the substantive protection against terminals with divergent width tables. §4.5 confines `unicode-width` to display projections. §5.4 checks "default and `NO_COLOR`/monochrome rendering", 40×10 and 80×24 "without hiding mode, dirty/conflict, or recovery status", "combining/RTL/CJK/emoji cursor and selection rendering in terminals with differing width tables", and "terminal disconnect during OSC52/save/external editor and subsequent recovery". The 40×10 consumer is also a D-level gate per §3.7, so the E deferral does not empty this. | — |
| 5E Automation | 3 | §3.4 names all five requirements explicitly: "human output by default, `--output json`, JSONL stdin/stdout batch operation, `--no-input`, and `--no-color`; `NO_COLOR` has the same effect as `--no-color`", plus fail-closed non-interactivity — "any operation requiring external-import acceptance, conflict resolution, recovery choice, clipboard consent, or remote capability returns `InteractionRequired` without mutation". §5.1 `protocol_v1_jsonl_is_stable_and_noninteractive_safe` asserts human/JSON parity, stdout purity, stderr redaction, and each flag. §3.4 binds the engine protocol as "implemented and contract-tested in D" rather than deferring it to E. (The `--output json` vs shipped `--json` spelling is scored against 4E, not repeated here.) | — |

**Lens average:** 3.00
**Lens pass:** Yes — avg ≥ 2.0, zero 1s, zero 0s

---

## Auto-fail walk

| Rule | Reachable? | Basis |
|---|---|---|
| Source-content loss | No | §4.2 barrier: mutation "remains staged and invisible to observers until that barrier succeeds"; failure rolls back fully and "must not emit an acknowledgment". §4.5 carries untouched bytes through exactly. |
| Partial multi-file mutation | No | §6.4 "Multi-file refactors are out of scope and must use H transactions rather than editor-local repeated saves"; §7.5 non-goal. |
| Index state overriding source | No | §4.4 Source boundaries; §4.1 derived clients never obtain a `TextStore`; §5.2 authority-inversion matrix. |
| Stale index presented as current | No | §3.4 "Stale results are labeled and cannot be represented as current"; `accepted_base_hash` truthfulness rule; §5.1 `knowledge_results_are_read_only_and_freshness_typed`. (The vocabulary mismatch scored at 4E is a compatibility defect, not a truthfulness one.) |
| Silent conflict winner | No | §3.2 external step 6 "No side silently wins; even a conflict-free merge remains a pending preview requiring explicit `accept_merge`"; §5.1 `three_way_merge_classifies_hunks` asserts no silent winner. |
| Unknown syntax loss | No | §4.5 "never prints or reserializes an AST/document/frontmatter block"; unknown tags/comments/quoting/line endings/Obsidian constructs "must remain byte-identical outside accepted ranges"; structural edits blocked until A4's corpus passes. §3.4 confines `=` to "a deterministic whitespace/indent action only". |
| Unconfirmed overwrite/import | No | §3.2 steps 4–6; §6.4; `--no-input` returns `InteractionRequired`. |
| Unsafe traversal or symlink escape | No | §4.1 core is sole path authority; §5.2 hostile alias and symlink-substitution suites; §4.6 no predictable temp path. |
| Capability/data-exfiltration bypass | No | §4.3 capability object with expiry and no transitive escalation; §5.1 `capabilities_are_deny_by_default_and_scoped`. |
| Active raw HTML/script by default | N/A | Headless engine renders nothing; §7.5 defers rich content to F. |
| Non-atomic save claiming success | No | §4.3 "Save success is emitted only after it returns the new fingerprint"; §3.6 "never false success"; §1.3 "atomic autosave never reports success unless the authoritative ordinary file durably contains the expected bytes". |
| Recovery overwriting newer source | No | §3.2 crash-recovery steps 3–4: recovery to a dirty buffer only when source matches the recorded base or last durable save hash, otherwise three-way merge; "Recovery never writes automatically". §5.2 crash matrix asserts never "overwrite of newer source". |
| Graph/Canvas lacking textual equivalent | N/A / satisfied | Graph and Canvas are out of scope (§7.5, G/K); every engine view has an ordered text + JSON equivalent (§3.7). |

**Auto-fail triggered:** No

---

## Feasibility Check

Source read: `vault/Cargo.toml`, `crates/*/Cargo.toml`, `mg-vault-core/src/{lib,vault,atomic,xdg,error,frontmatter,frontmatter_scalar,index}.rs`, `mg-vault-core/tests/*`, `mg-vault-index/src/lib.rs`, `mg-vault-cli/src/main.rs`, `docs/{ARCHITECTURE,PRODUCT,SECURITY}.md`, `docs/spikes/token-preserving-markdown-yaml.md`, `README.md`, and `git log` (9 commits, 2026-08-23 → 2026-08-24).

| Check | Status | Notes |
|---|---|---|
| Types/models exist or are clearly specified | ✓ | `SourceFingerprint`, `Note`, `Vault`, `XdgPaths`, `Error::Conflict` exist in `mg-vault-core` and map cleanly onto the spec's `SourceRead`/`BaseSnapshot`/`TextStore`. All editor types in §4.2 are new and written out with doc comments. `KnowledgeQuery` is the one named type left opaque (scored at 3C). |
| API/interface changes are feasible with current architecture | ✓ | `Vault::write_note(relative, bytes, expected)` is already the fingerprint-gated atomic replace the spec's `TextStore::replace_atomic` requires; `atomic.rs` already does temp-write → `sync_all` → rename → parent-dir `sync_all`. Two undeclared core extensions: `Vault::read_note` rejects any non-`.md` path and returns `Error::InvalidUtf8` **without bytes**, so §3.2's "byte-preserving external open" and §7.5's byte-preservation promise need a new core byte-read that §7.2 does not name; and core has no `ProtectedControlPath` variant (it returns `Error::UnsafePath`). |
| Views/screens fit current navigation pattern | ✓ | Headless by design (§3.1); no TUI exists to conflict with. `docs/ARCHITECTURE.md` still defers "app, editor, TUI, and plugin crates", so the placement is uncontested. |
| Dependencies are available and version-compatible | ✓ with caveats | **None of the named editor dependencies is a current workspace dependency.** `vault/Cargo.toml` `[workspace.dependencies]` holds only `clap`, `serde`, `serde_json`, `rustix`, `rusqlite`, `sha2`, `thiserror`, `yaml-edit`. So `ropey`/`crop`, `unicode-segmentation`, `unicode-width`, `regex`/`regex-automata`, `tree-sitter` + grammars, a diff/merge crate, and spellcheck crates are all additions. This is legitimate for a forward-looking spec because §4.5 declares each one and §7.2 schedules "add only justified dependencies to the workspace" — but §4.5 pins no versions at all ("pinned only after evidence spikes"), so the manifest delta is not yet reviewable. All named crates are real, maintained, and Rust-2024/1.85 compatible. |
| Platform/renderer requirements are realistic | ✓ with caveats | Arch/Hyprland first-class matches the environment. `[workspace.lints.rust] unsafe_code = "forbid"` and `[workspace.lints.clippy] all/pedantic = "deny"` apply to every workspace member: `tree-sitter`'s own unsafe lives in the dependency crate and is fine, but any FFI glue, and any direct `fdatasync` handling in `mg-vault-editor`, must be written unsafe-free (reachable via `rustix`, already a workspace dep). §4.6 correctly requires pinning the Tree-sitter ABI. |
| Test strategy is executable with current infrastructure | ✓ with caveats | Existing tests are plain `#[test]` + `tempfile` (`mg-vault-core/tests/foundation.rs`, `mg-vault-index/tests/persistent_store.rs` — the latter already does guard-injected crash/interruption testing, so the pattern exists). The spec's demands go well beyond it: property/generative Unicode corpora, byte-boundary fault injection across journal writes, process-kill crash matrices, fake editor processes, a JSONL golden-schema harness, screen-reader transcripts, and benchmarks. All buildable, none present; §7.2 schedules them as a named deliverable. |
| Performance budget is realistic for target hardware | ✓ | 8 ms p95 keystroke on 1 MiB with a rope is comfortable. The tight one is the ≤ 50 ms p95 durable acknowledgment, since §4.2 mandates a real `fdatasync` per mutating call: achievable on local NVMe/ext4, marginal on network or heavily-loaded storage — and the spec handles the miss honestly ("a visible slow-storage state rather than batching or weakening durability"). |
| No undeclared dependency on unbuilt features | ✓ with caveats | `mg-vault-markdown` (A4), Spike 2, the app watcher boundary, and E are all declared in §7.4. Undeclared: the byte-preserving core read noted above, and the reconciliation with the already-shipped `mg-vault-index`/CLI contracts noted below. |

### Stale claims in §7.1 (spec dated 2026-08-23; verified against HEAD `dfe33cf`, 2026-08-24)

| §7.1 claim | Actual | Effect |
|---|---|---|
| "`Cargo.toml` contains Rust 2024 workspace members `mg-vault-core` and `mg-vault-cli`" | Three members: `mg-vault-core`, `mg-vault-index`, `mg-vault-cli`. `mg-vault-index` landed in commits `99ab4f8`/`632ef17` (2026-08-23) and `dfe33cf` (2026-08-24), all after the spec's own commit `3ffad99`. Edition 2024 / rust-version 1.85 is correct. | Factual error. Scored against 4E, where the unacknowledged index crate is the source of the freshness-vocabulary divergence. |
| "`docs/ARCHITECTURE.md` explicitly defer[s] editor/TUI/parser/index work" | `ARCHITECTURE.md` now describes `mg-vault-index` as shipped ("owns a disposable SQLite projection ... Complete rebuilds publish a generation atomically") and defers only "watcher/service, IPC client, structural parser/query foundation, app, editor, TUI, and plugin". | Factual error on the index half. Same scoring locus. |
| "no editor crate exists" | Correct — no `mg-vault-editor` anywhere. | ✓ |
| "`vault.rs`, `atomic.rs`, `xdg.rs` ... provide confined file authority, fingerprints, XDG paths, and atomic replacement primitives suitable as dependencies" | Correct and verified in detail. | ✓ |
| "`gauntlet-output/specs/a-foundation-file-authority.md` defines the current accepted file-authority slice" | Correct; file present. | ✓ |
| **Absent list** (rope buffer, grapheme cursor, modal grammar, registers/macros, marks/jumps, search/command line, undo tree, recovery WAL, autosave, watcher, merge, external handoff, Tree-sitter, spellcheck, editor docs/fixtures/benchmarks) | All confirmed absent **as editor components**. Note that vault-level `search` and a freshness-typed query surface now ship in `mg-vault-index` + the CLI; §3.4 designs the `KnowledgeQueryProvider` seam as if only a hypothetical B/G provider exists. | Not a false "absent" claim (editor search genuinely does not exist), but the omission is why the seam and the shipped store share no vocabulary. |

**Feasibility verdict:** Feasible with caveats
**Caveats:** (1) `mg-vault-index` exists and §7.1 says it does not, leaving the retrieval seam and freshness vocabulary unreconciled with shipped code; (2) every editor dependency is new to the workspace and none is version-pinned; (3) core needs an undeclared byte-preserving read for the invalid-UTF-8 path; (4) the entire fault-injection/property/benchmark harness is new build-out, correctly scheduled but large; (5) `unsafe_code = "forbid"` constrains how the journal fsync and any Tree-sitter glue may be written.

---

## Composite Score

| Lens | Average | Weight | Weighted |
|---|---|---|---|
| Data Ownership and Format Compatibility | 3.00 | 30% | 0.900 |
| Editor and TUI Excellence | 2.60 | 25% | 0.650 |
| Knowledge Retrieval and Structure | 2.80 | 20% | 0.560 |
| Security, Reliability, and Extensibility | 2.83 | 15% | 0.425 |
| Performance and Accessibility | 3.00 | 10% | 0.300 |
| **Composite** | | | **2.84** |

**Pass conditions (from criteria.md):**
- [x] Composite ≥ 2.0 — 2.84
- [x] All lens averages ≥ 2.0 — minimum 2.60 (Lens 2)
- [x] No criterion scores 0
- [x] No more than two criteria at 1 per lens — zero 1s in any lens
- [x] All auto-fail rules pass — all thirteen walked above
- [x] Feasibility ≠ Infeasible — Feasible with caveats

**All conditions met:** Yes → PASS

---

## Remediation Brief (non-blocking — verdict is PASS)

No Priority 1 items: nothing here blocks acceptance. The following would raise
scores and remove real defects before implementation begins.

### Priority 2 — Should fix for quality

1. **§3.4, command line.** The closed `:` list omits accept/reject of an external
   import and acceptance of a merge, which §3.2 steps 5–6 make mandatory and
   §3.7 claims are keyboard reachable. Add `:accept {candidate}`, `:reject {candidate}`,
   and `:accept-merge` (or equivalents) to the enumerated list, and reconcile §3.7's
   blanket "Every operation has a stable command ID" with §3.4's "only ... in scope"
   framing so the two sections cannot be read as contradicting each other. (2A)
2. **§3.4, command line.** Define the discard semantics of `q!` and `e!` — the only
   two enumerated commands whose purpose is to throw work away — and state explicitly
   that both leave the recovery journal generation intact under §4.2 retention. (2A)
3. **§3.4 and §7.2, freshness and flags.** `mg-vault-index` ships
   `Freshness { Empty, Current, Stale, Degraded }` and the CLI emits `"status": "degraded"`;
   the spec's `current`/`stale`/`unavailable`/`not_applicable` set cannot express it.
   Either adopt the shipped vocabulary or give the total mapping. Likewise reconcile
   `--output json` with the shipped `--json` + `version: 1` envelope, or record the
   divergence and its migration in §7.2. (4E)
4. **§4.2/§4.3, `KnowledgeQuery`.** Write out the enum (one variant per named query
   kind), the result reference type (path plus optional block/byte range), and the
   watermark/freshness fields, with an explicit version discriminant. §3.4 calls the
   kinds "versioned" but no version field appears anywhere. (3C)
5. **§7.1.** Correct the two stale claims: the workspace has three members including
   `mg-vault-index`, and `docs/ARCHITECTURE.md` no longer defers index work. Add a
   sentence acknowledging that a shipped vault search/freshness surface exists and
   that §3.4's provider seam must eventually be backed by it rather than only by fakes.
6. **§7.2.** Add the two undeclared core extensions: a byte-preserving read for
   invalid-UTF-8 sources (today `Vault::read_note` returns `Error::InvalidUtf8` and no
   bytes, so §3.2's byte-preserving external open cannot be built), and a
   `ProtectedControlPath` error variant distinct from the current `Error::UnsafePath`.

### Priority 3 — Consider for excellence

7. **§3.1 or §4.4, session restore.** State which engine state is session-serializable
   (cursor, marks, jump list, undo head, search history) and what identity a restored
   buffer presents, so E can build session restore against a defined seam rather than
   inventing one. This is the single sub-criterion of 2C with no obligation at all. (2C)
8. **§4.2 or §3.4, Unicode normalization.** State the normalization policy explicitly:
   that buffer bytes are never normalized, and whether search and word motions compare
   canonically-equivalent sequences. Also name the word-boundary rule for languages
   without spaces (CJK, Thai) rather than leaving `w`/`e`/`b` entirely to
   `docs/EDITOR.md`. (2D)
9. **§4.2, key encoding.** Define the `<vault-key>`/`<path-key>` encoding used for XDG
   recovery and undo directories — collision behavior, case handling, and non-UTF-8
   path bytes — since Linux paths are arbitrary bytes and the shipped index already
   stores `path BLOB PRIMARY KEY` for that reason. (1C)
10. **§4.5.** Give each candidate dependency a target version range now, even if the
    final pin waits on Spike 2, so the manifest delta is reviewable before the spike
    rather than after.
