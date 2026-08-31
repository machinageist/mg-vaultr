# Scorecard: Foundation and Vault Authority

**Feature ID:** a-foundation-file-authority
**Spec file:** gauntlet-output/specs/a-foundation-file-authority.md
**Reviewer agent:** Spec Gauntlet verification agent (blind review)
**Date:** 2026-08-30
**Spec iteration reviewed:** 2

**Graded against commit:** `dfe33cf` (2026-08-24), **plus uncommitted working-tree
modifications** to `Cargo.toml`, `crates/mg-vault-cli/{src/main.rs,tests/cli.rs}`,
`crates/mg-vault-index/*`, `README.md`, `docs/{ARCHITECTURE,PRODUCT}.md`.
The uncommitted work is entirely B-side (index/search/persistent store). Every §7.1
claim below was verified against the **working tree**, and no A-owned claim was
invalidated by it. `cargo test --workspace --all-targets --all-features` passes
(8 CLI + 15 core-unit + 14 foundation + 6 frontmatter_scalar + 7 core-index + 31
persistent_store = 81 tests, 0 failures).

---

## Verdict: PASS

**Summary:** This is a rigorous, unusually honest foundation spec — §7.1's
current-state report is verifiable almost line for line against the real source,
§4.3's JSON/versioning contract and §4.7's budgets are genuinely binding, and §6.4
walks every criterion and every auto-fail rule by name rather than leaving Lens 2/3
blank. It passes every gate. The single most important defect is that §3.2's
replacement protocol (read bytes → compare fingerprint → `replace_atomic`) has an
unspecified gap between the check and the commit, so the spec's own
`two_process_conflict` invariant ("exactly one commits and one gets `conflict`;
there is no last-writer-wins outcome", §5.2) cannot be delivered by the design as
written — verified in `vault.rs:155-173`. That, plus `--body TEXT` placing note
content in argv against §6.5's own rule, are the required pre-implementation fixes.

---

## Lens 1: Data Ownership and Format Compatibility (weight: 30%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 1A Authority | 3 | §4.4 "Authoritative state: the ordinary files in the vault directory. Nothing else."; §4.1 makes `mg-vault-index` and `core/src/{index,interop}.rs` explicitly non-A with a directional dependency rule; §4.4 puts B's projection in XDG cache keyed by root fingerprint and requires deleting it to be harmless; §5.2 `disposable_index_does_not_affect_source`; §6.4 1A row. **Verified:** no `Vault` method reads an index — `read_note` (`vault.rs:122`) opens the file directly; `lib.rs` exports `MarkdownIndex` but nothing in `vault.rs`/`registry.rs`/`atomic.rs` references it. §7.1's "packaging accuracy" note flags `index.rs`/`interop.rs` living in core as a packaging accident, not an authority relationship, with §7.2 remediation and Q5. | Fix one imprecise claim: §4.5 says "Not depended on by this feature: `rusqlite` (B)", but `crates/mg-vault-cli/Cargo.toml` — which §4.1 places inside this feature — depends on `mg-vault-index` → `rusqlite 0.40.2`. Reword to "no A **code path** links or calls rusqlite; the shared binary hosts B's `index`/`search` verbs." |
| 1B Preservation | 3 | §3.2 edit-span step 3: "copying the untouched byte ranges verbatim — no parser, no serializer, no line-ending normalization"; step 5 fail-closed locators; §4.2 `Note.bytes: Vec<u8>`; §4.5 "yaml-edit 0.2 (frontmatter scalar *location* only — never serialization)"; §7.5 prohibits serialization; §5.1 `narrow_edit_preserves_all_bytes_outside_the_selected_span`; §5.4 content extremes. **Verified:** `vault.rs:223-226` splices `current[..start] ‖ replacement ‖ current[end..]`; `frontmatter.rs` reports `LineEndings::{Lf,CrLf,Mixed}` without normalizing, and its inline tests assert CRLF and mixed spans survive. | — |
| 1C Identity | 3 | §4.2 "Identity" paragraph: vault-relative path is identity; trash IDs (`<128-bit-ns-hex>-<counter-hex>`, 49 chars) and transaction IDs are `.mg-vault` bookkeeping handles only; fingerprints are recomputable content digests, not injected identifiers; §5.2 `no_identifier_is_injected`; §6.4 1C row correctly scopes interop `global_id` as derived at export. **Verified:** `trash_id()` emits exactly 49 chars (`vault.rs:392`); no write path touches note content with an ID; `interop.rs:280 note_global_id` is computed, never persisted into source. | — |
| 1D Coexistence | 3 | §4.1 filesystem-layout block plus the two invariants ("portable, vault-scoped app state lives under `.mg-vault` and nowhere else"; "disposable derived data lives in XDG cache outside the vault"); §3.2 create step 2 rejects `.obsidian`/`.mg-vault` as mutation first component; §5.2 `control_directories_are_untouched` and `mg_vault_state_stays_inside_the_control_dir` (asserts the set difference of new paths); §5.4 Coexistence and Portability manual checks. **Verified:** `validate_note_path(_, true)` rejects both names (`vault.rs:359-367`); `internal_trash_dir()` is private and returns `root/.mg-vault/trash`. | Consider stating whether the protection is deliberately first-component-only (it is, in code) and whether `.git`/`.stfolder` need the same treatment before M (Git/Syncthing) lands; §5.2's control-dir matrix currently tests `.obsidian` only. |
| 1E Transactions | 2 | Strong: §3.2 per-flow fail-closed rules, §3.6's error table with an explicit data-loss column, §4.4's target journal protocol, §5.2's four fault/race matrices, §6.4's named auto-fail walk. **Two real defects.** (a) **Check-to-commit is not atomic.** §3.2 "Replacing a note" steps 3–4 specify read → fingerprint compare → temp/rename, with nothing binding the checked bytes to the committed bytes. Verified at `vault.rs:163-171` (and `edit_note_span`, `vault.rs:190-227`): two processes that both read the same pre-image both pass the check and both `fs::rename`, so the second silently destroys the first — which is precisely the "stale write silently destroys an external edit" failure §1.2 exists to prevent, and directly contradicts §5.2's `two_process_conflict` ("no last-writer-wins outcome") and §6.4's "Silent conflict winner" clearance. (b) **Trash rollback uses a replacing rename.** §3.2 trash step 3: "If that fails, the payload is renamed back" — verified as `fs::rename(&payload, &source)` at `vault.rs:261`, which clobbers anything created at the original path in the window. §6.4 nevertheless claims "every failure path leaves the exact pre-image". | Priority 1 items 1 and 2 below. |

**Lens average:** 2.80
**Lens pass:** Yes — avg ≥ 2.0, zero 1s, no 0s

---

## Lens 2: Editor and TUI Excellence (weight: 25%)

Lens 2 is legitimately deferred to branches D and E for a foundation slice. §6.4
addresses 2A–2E individually with architecture notes rather than leaving them
blank, so justified N/As are scored "acceptable" (2), not penalized.

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 2A Keyboard completeness | 3 | §3.1's full command/invocation table; §3.4 "Positional grammar: `mg-vault <group> <verb> [OPERANDS] [FLAGS]` with groups `vault`, `note`, and (target) `trash`. Operand order is fixed and documented in `--help`"; §3.4's `--no-input` guarantee ("no prompt, no confirmation, no `/dev/tty` read, and no editor launch") with §5.2 `no_input_never_prompts` as the proof; §3.7 "Every capability is reachable from argv alone with no prompt, which is the CLI form of 'keyboard complete'"; §6.4 2A defers modal grammar to D. | Minor grammar incoherence to resolve: inventory is `note trash` / `note restore ID` but `trash list` (§3.1, §3.4). Pick one home for trash verbs. |
| 2B Editing durability | 2 | §6.4 2B: A supplies the durable primitive (fingerprint-checked atomic commit, conflict refusal preserving both versions, trash-backed recovery); autosave, persistent undo, and external-edit merge are "**deferred to D**, which must build them on these APIs" — a real architecture note, not a blank. §4.4 adds a target conflict spool under `$XDG_STATE_HOME` so a rejected write is recoverable. Held at 2 because the primitive D would build on inherits the 1E(a) check-to-commit gap, and the "preserve both versions" half is target-only. | Close 1E(a) first; D's autosave is the highest-frequency consumer of the racing write path. |
| 2C Workspace | 2 | §6.4 2C: "**N/A — deferred to E.** A has no panes, tabs, splits, preview, or session state and does not preclude them: `Vault` is terminal-independent, holds no UI state, and can be driven by many concurrent front ends." **Verified:** `Vault` is `{root: PathBuf}`, `Clone`, caches nothing (`vault.rs:70-74`), and §4.4 states the no-cached-tree rule as deliberate. Correctly justified N/A. | The "many concurrent front ends" claim is the strongest form of the 1E(a) race; either close it or qualify the claim. |
| 2D Text correctness | 3 | §3.2 edit-span step 2 requires "`start <= end <= len`, and both offsets to sit on character boundaries"; §3.7 Unicode safety ("paths and content pass through without normalization, case folding, or lossy conversion"); §4.6 case/normalization paragraph; §5.1 `narrow_edit_rejects_invalid_utf8_boundaries_without_mutation`; §5.4 content extremes (combining marks, RTL, emoji); §6.4 2D correctly scopes grapheme-safe cursor work to D while supplying byte-accurate spans. **Verified:** `is_char_boundary` on both ends at `vault.rs:204-205`; test exists and passes. | — |
| 2E Degraded experience | 2 | §6.4 2E: "With no editor, TUI, or index present, direct source read/create/write/span-edit/trash/restore all work, and nothing claims a capability it lacks"; §4.6's rule that "a build without the hardened backend must say so in `--version`/status rather than quietly offering weaker guarantees"; §3.6's `durability_unavailable`/`cross_device` refuse rather than silently degrade. Held at 2: §6.4's 2E row lists span-edit among what works without an editor, but §7.1 correctly reports `note edit-span` as **Prototyped — no CLI verb** (verified: `NoteCommand` has only Create/Read/Write/Trash/Restore, `main.rs:77-96`). Internal inconsistency between §6.4 and §7.1. | Qualify §6.4's 2E row: span-edit is a **library-level** capability today; the CLI verb is target (§7.2). |

**Lens average:** 2.40
**Lens pass:** Yes — avg ≥ 2.0, zero 1s, no 0s

---

## Lens 3: Knowledge Retrieval and Structure (weight: 20%)

Lens 3 is largely deferred to B and G. §6.4's "Regulatory alignment" section
addresses each of 3A–3E by name as either an A-owned enabling property or an
explicit deferral, and closes with an architecture note showing the foundation
does not preclude retrieval. That is the correct handling and is not penalized.

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 3A Determinism | 3 | §6.4 3A: A-owned enabling property — `SourceFingerprint` gives every derived layer "a deterministic, recomputable token for 'which bytes did I derive from'", path identity gives a stable key, and "A itself derives nothing"; full determinism deferred to B and G. §4.4 forbids a cached tree ("a cached tree would become a second authority"). The closing architecture note names the actual consumers, `MarkdownIndex` and `PersistentIndexStore::verify_freshness`. **Verified:** both exist and are exercised by 31 passing `persistent_store.rs` tests. More than a deferral — a specified, tested substrate. | — |
| 3B Ambiguity | 3 | §6.4 3B states A's analogue precisely: "ambiguity fails closed and never mutates: a duplicate frontmatter key, an unclosed envelope, or a BOM refuses to yield an editable span rather than picking one"; link-target ambiguity deferred to G with the same obligation. §3.2 edit-span step 5; §5.1 `rejects_duplicate_target_keys` / `frontmatter_bom_and_unclosed_fail_closed`. **Verified:** `FrontmatterScalarError::{DuplicateKey,NotScalar,InvalidYaml,MissingKey,NoFrontmatter}` and `FrontmatterError::{UnsupportedBom,Unclosed}` all exist with passing tests. | — |
| 3C Query depth | 2 | §6.4 3C: "**N/A here; deferred to B and G.** A exposes no query surface at all: no text, title, regex, tag, property, path, relationship, task, or date query. This is deliberate — A must stay O(1) in vault size." Enumerates the criterion's own sub-items and gives a design reason for the omission rather than a bare N/A. | — |
| 3D Derived authority | 3 | §6.4 3D: "A is the reason derived layers can never become authority: nothing in A reads an index, a database, or a cache to answer a question about source, and B's projection lives outside the vault under XDG cache so deleting it is harmless." Reinforced by §4.1's directional rule and §5.2's `disposable_index_does_not_affect_source`. **Verified** against `vault.rs` — no index reference on any read or mutation path. | — |
| 3E Scale | 3 | §4.7 "Scale posture: every operation in this feature is O(1) in vault size — no command here walks the vault… a regression test here asserts that no A command performs a recursive directory walk" (§5.1 `no_command_walks_the_vault`), with the 100,000-note/1,000,000-block budgets explicitly assigned to B. Directly answers the criterion's "without blocking editing" half and defers the rest with a named owner. **Verified:** no `read_dir` in `vault.rs`/`registry.rs`/`atomic.rs`. | — |

**Lens average:** 2.80
**Lens pass:** Yes — avg ≥ 2.0, zero 1s, no 0s
**Auto-fail triggered:** No

---

## Lens 4: Security, Reliability, and Extensibility (weight: 15%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 4A Confinement | 2 | Strong core: §3.2 create steps 2–3 (relative, normal-components-only, `.md`-only, per-component parent walk with symlink/non-dir rejection and canonical re-check after each step); §3.6's `unsafe_path` row; §4.6 characterizes the current portable backend honestly — "It does **not** defeat a hostile process that swaps a directory entry between the check and the operation" — and specifies the `openat2 RESOLVE_BENEATH\|NO_SYMLINKS\|NO_MAGICLINKS` / macOS `openat`+`O_NOFOLLOW`+`(dev,ino)` target plus a fail-closed capability gate; §5.2 `unsafe_symlink_swap` and `platform_confinement_gate`; §8-Q2. **Verified:** `prepare_destination` and `ensure_beneath` behave as described (`vault.rs:312-338`, `378-384`), and the `openat2` precedent really does exist at `index.rs:207-226`. **Docked for two misreported claims:** §6.4's 4A row asserts "symlink rejection at the final component and every parent", but the spec's own §3.2 read flow does no symlink check (verified: `read_note` canonicalizes and checks containment only) and `existing_mutation_path` checks the final component only (`vault.rs:340-350`); and §5.1 says `rejects_symlink_escape` covers "read and on every mutation" when the real test exercises `create_note` alone (`foundation.rs:64-75`). | Priority 1 item 3. |
| 4B Concurrency | 2 | §3.2 write step 5 ("`--expected` is mandatory. There is no 'just overwrite' path"); §4.4's optimistic model with "There is no lock that makes a fingerprint check optional — external programs need not honor any lock, so the check stays mandatory"; §3.6's `conflict` row reports both digests and applies nothing; §5.1 `optimistic_write_rejects_changed_source` (verified, passing). The detection half is correct and implemented. Docked because the specified mechanism does not deliver §5.2's `two_process_conflict` invariant (see 1E(a)), and the "preserve both concurrent versions" half is target-only (the §4.4 conflict spool). | Priority 1 item 1. |
| 4C Least privilege | 2 | §4.3's Auth/permissions paragraph is a genuine structural argument: "`.mg-vault` is reachable *only* through the specific private helpers… No caller — including a future plugin or AI adapter — can construct a `Vault` mutation that targets the control directory or an out-of-root path, because those types are only produced by core code from an already-canonicalized root." §6.4 4C: deny-by-default "because the capability does not exist"; §7.5 non-goals. **Verified:** `Vault.root` and `internal_trash_dir` are both private. Held at 2 because the criterion's "attributable, and previewed" half has no A-side primitive at all — no operation log, no dry-run/plan mode — even though §7.4 says C needs "plan/commit-grade primitives" from A and N/O must enforce through these same APIs. | Add one line to §4.3 or §6.4 4C naming where attribution and preview will attach (a `plan`-returning variant of each mutation, or an operation record in `.mg-vault/journal/`), so N/O are not left to invent it. |
| 4D Recovery | 2 | Excellent on three of four sub-clauses: §3.2's "Failure branching common to all mutations" (Committed / Refused / Failed, with "the message says which step and never uses the words `created`, `wrote`, `trashed`, or `restored`" and "There is no fourth 'probably worked' outcome"); §3.2 restore step 6 ("no force flag and no 'newer/older' heuristic"); §3.2 trash step 8 ("Permanent purge is **not implemented and not offered**"); §4.4's target journal with "Recovery never writes to a path whose current bytes differ from the journal's recorded precondition"; §5.2 `restore_never_overwrites_newer_source`; §6.3's "Output verbs are earned". **Verified:** restore refuses collision via `hard_link` AlreadyExists (`vault.rs:290-296`), covered by a passing test. Docked because **"previewable" is silently dropped** — §6.4's 4D row omits the word and no section anywhere offers a dry-run, plan, or preview affordance for `trash`/`write`/`edit-span`; and because of 1E(b)'s replacing-rename rollback. | Priority 1 item 2; plus add a `--dry-run` (or explicit "preview is deferred to C with this rationale") to §3.4/§6.4 4D. |
| 4E Contracts | 3 | §3.1's binding stream discipline ("With `--json`, success is exactly one UTF-8 JSON object plus one `\n` on stdout and stderr is empty; failure is exactly one JSON error object plus one `\n` on stderr and stdout is empty"); §4.3's envelope with a real evolution rule ("Removing a field, changing a type, or moving a stream requires envelope version 2; adding a documented optional field does not") and golden fixtures as the compatibility contract; the exit-code taxonomy aligned with C; §4.2's versioned registry/trash schemas that "refuse to load and refuse to rewrite" on unknown newer versions; §3.3's lossless `unix_bytes_hex` object for non-UTF-8 paths; §8-Q3 raises the additive-vs-v2 question honestly instead of assuming. **Verified:** the live envelopes are exactly `{version,ok,data}` / `{version,ok,error{code,message}}` (`main.rs:604-638`), and all eleven `error_code` strings match §3.6's names character for character. §7.1 correctly reports exit codes as Absent ("every failure is exit `1`" — verified `ExitCode::from(1)`). | §7.1's Absent list should also name the §3.3 human-layout items that do not exist: the multi-line receipt order, "exactly one suggested recovery command", and the empty-state sentences (live output is `no registered vaults`, not `No registered vaults. Run: mg-vault vault register NAME PATH`). §7.2 already names the path-rendering gap precisely and correctly. |
| 4F Privacy | 2 | §6.1 is thorough and specific: "there is no network code path in any dependency of this feature"; no telemetry or crash reporting; "note content is emitted only when the user explicitly runs `note read`… and is never included in receipts, warnings, error messages, error `details`, or any log"; owner-only spools; temp files removed on every failure path. §6.4 4F; §6.5. **Verified:** `Error::Conflict` carries digests, not bytes; no logging crate in any manifest. **Docked for a direct self-contradiction:** §6.5 says "never place note content in argv, environment, process titles, or diagnostics", yet §3.1/§3.4 make `--body TEXT` the primary (and per §7.1 the *only*) body source — verified at `main.rs:83` and `:91` — which exposes note content in `/proc/<pid>/cmdline` to any local user. `--stdin`/`--body-file` are target-only. | Priority 1 item 4. |

**Lens average:** 2.17
**Lens pass:** Yes — avg ≥ 2.0, zero 1s, no 0s

---

## Lens 5: Performance and Accessibility (weight: 10%)

5A, 5D, and 5E apply fully to this slice and were graded without any deferral credit.

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 5A Offline/local-first | 3 | §4.5 "Not depended on by this feature: `rusqlite` (B), any network, HTTP, TLS, or telemetry crate…"; §4.7 "Network payloads: none — there is no network code path (§5A)"; §6.1's identical assertion; §6.4 5A "the tool works with networking disabled, and this is asserted by dependency review rather than assumed"; §7.5 excludes network/sync/Git/publishing; §4.5 CI runs "on a clean machine". **Verified:** `mg-vault-core` deps are exactly serde, serde_json, rustix, sha2, thiserror, yaml-edit — none opens a socket. | Optional: promote "asserted by dependency review" into an executable gate (`cargo deny`/offline build) so the claim regresses loudly. |
| 5B Responsiveness | 3 | §4.7 covers every sub-clause with numbers and reference hardware: startup (`--help` p95 ≤ 50 ms with zero filesystem access — verified, clap exits before `run()` touches XDG), registry (≤ 100 ms at 1,000 vaults), per-op latency (read ≤ 50 ms, commit ≤ 100 ms), memory (≤ 20 MiB RSS, "peak is bounded by roughly 2× note size" — verified, `edit_note_span` allocates one checked-arithmetic buffer at `vault.rs:214-223`), storage, and the O(1) scale rule. Includes the discipline clause "**Durability is never weakened to meet a latency number** — if the budget and `fsync` conflict, the budget loses." Cancellation is specified in §3.4's Signals paragraph with pre- and mid-commit semantics. | — |
| 5C Accessible equivalents | 2 | §3.7 "this feature produces no graph, canvas, image, chart, or spatial view, so there is nothing that lacks a textual equivalent. Every value it emits is already text"; §3.5 confirms no animation/cursor addressing/alternate screen; §6.4 5C. Justified N/A for graph/Canvas/media/cards. The criterion's applicable "status" clause is handled — §3.3 makes trash health a literal word (`ok`, `orphan_payload`, `orphan_metadata`, `unreadable_metadata`) and §3.2 step 5 makes selection a literal `selected: yes\|no` field — but that field is marked target, so status is not yet fully textual today. | — |
| 5D Terminal resilience | 2 | Substantial: §3.3 (column-aligned at ≥ 60, one-record-per-block below 60, "**Nothing is ever truncated**" with a two-space continuation wrap, JSON invariant to width); §3.4 Terminal width; §3.5 (no animation — "inherently reduced-motion-safe"); §3.7 (color independence with the ANSI-strip test, glyph independence, Unicode safety, control-character escaping so "a filename can never inject terminal escape sequences"); §5.3 items 5–7 and §5.4's 40/60/80/120/200-column matrix. **Docked because §7.1 does not report these as absent:** the live `vault list` emits one tab-joined line per record with a bare `*` marker and `Path::display()`, with no width logic, no wrapping, and no block layout (verified `main.rs:513-536`). §7.2's "Modified files" names the path-escaping and color-policy gaps precisely but not the width/layout/empty-state gaps, so a reader of §7.1 would under-estimate the work. | Priority 2 item 1. |
| 5E Automation | 3 | §3.1's stream discipline plus "`note read` without `--json` is the only unadorned content stream: stdout receives the source bytes and nothing is inserted, removed, or appended" (**verified** — `raw_human` path uses `print!` on exact content, `main.rs:618-622`); §3.4's `--no-input` guarantee, the `--no-color`/`NO_COLOR`-with-any-value rule, and an unusually careful stdin clause ("stdin is consumed only when `--stdin` is given, so an unrelated inherited pipe is never mistaken for note content, and exactly one body source may be supplied"); §4.3's versioned envelope; §5.2 `no_color_contract` and `no_input_never_prompts`; §5.3 item 8's `sha256sum` pipe equality check. **§7.1 is honest about the gaps** — its "Gated" bullet states that `--no-color`/`--no-input` hold only because nothing prompts or emits ANSI and that "`NO_COLOR` is documented in the flag help and is **not read** by the program", which I verified exactly (both flags are declared at `main.rs:24-29` and referenced nowhere else; `NO_COLOR` appears only in a doc comment). | — |

**Lens average:** 2.60
**Lens pass:** Yes — avg ≥ 2.0, zero 1s, no 0s

---

## Auto-Fail Review (walked individually)

| Rule | Status | Basis |
|---|---|---|
| Source-content loss | **Pass (with finding)** | §1.3, §3.2's failure branching, §5.2's fault matrices, and §6.4 all require the exact pre-image on every failure. The one hole is 1E(b)'s replacing-rename rollback in trash — a narrow error-path race, not a designed loss. Not an auto-fail; recorded as Priority 1 item 2. |
| Partial multi-file mutation | **Pass** | §3.2 edit-span step 6 and §7.5 forbid multi-file/batch mutation outright; §6.4 names trash/restore as the single two-step pair, bracketed by rollback today and a durable journal in target; §7.4's sequencing note blocks H until the journal lands. |
| Unconfirmed overwrite / import | **Pass** | §3.2 create step 5: "**there is no `--force`, no overwrite flag, and no interactive overwrite prompt for create**"; write requires `--expected`; restore refuses an occupied destination. Verified: `RENAME_NOREPLACE` (`atomic.rs`), `hard_link` AlreadyExists → `Collision` (`vault.rs:290`). |
| Unsafe traversal or symlink escape | **Pass** | Layered validation + canonical containment re-check, verified in `vault.rs`. The residual concurrent-swap window is disclosed by name in §4.6, §7.1, §6.4, and §8-Q2 with a specified fail-closed target gate — a named deferral, not an unstated assumption. |
| Non-atomic save claiming success | **Pass** | §3.2 create step 6 ("Success is printed only after that directory sync returns"), §3.6's `io` row, §6.3's "Output verbs are earned", §6.4. Verified: `create_atomic` returns only after `sync_parent`, and the CLI prints from the `Ok` arm only. |
| Recovery overwriting newer source | **Pass** | §3.2 restore step 6 — collision-refusing, "no force flag and no 'newer/older' heuristic"; §5.2 `restore_never_overwrites_newer_source`; target journal recovery "refuses to write any path whose current bytes differ from the recorded precondition". Verified and covered by a passing test. |
| Silent conflict winner | **Pass — narrowly; see note** | The spec **prohibits** last-writer-wins in three places (§4.4, §5.2, §6.4) and never presents a silent winner as acceptable. But the mechanism in §3.2 cannot enforce it (1E(a)). I judged this an under-specified mechanism rather than a permitted outcome, so it is scored hard (1E and 4B both docked to 2) and raised as Priority 1 item 1 rather than auto-failed. **Flagging explicitly so the orchestrator can override:** if the gauntlet reads "silent conflict winner" as covering a spec whose stated invariant its own design cannot deliver, this becomes an auto-fail. |
| Index state overriding source / stale index as current | **Pass** | No A operation reads an index (verified); §6.4 assigns freshness truthfulness to B. |
| Unknown syntax loss | **Pass** | §3.2 edit-span step 3 (verbatim byte copy, no serializer), §7.5's serialization prohibition, §5.1's preservation test. |
| Capability / data-exfiltration bypass | **Pass** | No plugin host, AI adapter, or network exists (§4.3, §6.1, §7.5); the internal `.mg-vault` boundary is structurally enforced and verified. |
| Active raw HTML/script by default | **N/A** | No renderer in this feature (§6.4). |
| Graph/Canvas without textual equivalent | **N/A** | No graphical output (§3.7, §6.4 5C). |

**Auto-fail triggered:** No

---

## Feasibility Check

Every §7.1 claim was checked item by item against the working tree.

| Check | Status | Notes |
|---|---|---|
| Types/models exist or are clearly specified | ✓ | `XdgPaths`, `VaultRecord`, `VaultRegistry`, `Vault`, `SourceFingerprint`, `Note`, `TrashReceipt`, `TrashMetadata`, `FrontmatterEnvelope`, `LineEndings` all match §4.2 field for field, including `VaultRegistry`'s `#[serde(skip)] path` + `BTreeMap` + `selected: Option<String>` and the absence of a `version` field (correctly marked target). Target types (`TrashEntry`, `TrashHealth`, `TransactionRecord`, `TransactionPhase`) are specified in Rust with doc comments and are clearly constructible. |
| API/interface changes feasible with current architecture | ✓ | All eleven implemented signatures in §4.3 exist as written; `list_trash`/`recover`/`unregister` are marked target and are additive. `Error` has exactly the eleven variants §7.1 lists. |
| Views/screens fit current navigation pattern | ✓ | §3.1's inventory matches `main.rs`'s clap tree; the four target verbs (`note edit-span`, `trash list`, `vault unregister`, `vault path`) are ordinary subcommand additions. |
| Dependencies available and version-compatible | ✓ (one imprecision) | Verified in `Cargo.toml`: clap 4.5, serde 1, serde_json 1, sha2 0.10, thiserror 2, rustix 1 (features `fs`,`process` — `renameat_with` and `openat2` both available), yaml-edit 0.2.3. §4.5's "no new dependencies required" holds. **Imprecision:** §4.5 says rusqlite is "not depended on by this feature", but `mg-vault-cli` — which §4.1 scopes into this feature — pulls `mg-vault-index` → `rusqlite 0.40.2`. No A code path uses it, so the substance holds; the wording does not. Docked at 1A remediation. |
| Platform/renderer requirements realistic | ✓ | The `renameat2(RENAME_NOREPLACE)` cfg split and `hard_link`+`unlink` fallback exist exactly as §4.6 describes. The `openat2` precedent §4.6 cites is real: `index.rs:207-226` uses `ResolveFlags::BENEATH \| NO_SYMLINKS` on Linux and refuses on other platforms — so the target `VaultDir` backend has a working in-tree template. `unsafe_code = "forbid"` and denied `clippy::{all,pedantic}` are confirmed workspace lints. |
| Test strategy executable with current infrastructure | ✗ (two misreported claims) | The §1.3 success command runs clean today (81 tests, 0 failures). **But §5.1's preamble says "Implemented today in `crates/mg-vault-core/tests/foundation.rs` and `tests/frontmatter_scalar.rs`; target additions marked", and three unmarked rows misstate reality:** (1) `registry_rejects_duplicate_name` and `registry_name_validation` **do not exist** — `foundation.rs` contains no duplicate-name or `InvalidVaultName` assertion at all (grep count 0), and §7.1's own prose list omits them, so §5.1 and §7.1 contradict each other. (2) `rejects_traversal_absolute_and_protected_mutations` is claimed to cover `note.txt` and "every mutation entry point"; the real test (`foundation.rs:47-62`) exercises `create_note` only and never tests a non-`.md` extension. (3) `rejects_symlink_escape` is claimed to cover "read and every mutation"; the real test covers `create_note` with a symlinked parent only. Also `frontmatter_bom_and_unclosed_fail_closed` lives as `rejects_bom_and_unclosed_envelopes` in `src/frontmatter.rs`'s inline `#[cfg(test)]` module, not in either file the preamble names. Minor: §7.1 says `foundation.rs` has 13 tests; it has 14. The §5.2/§5.3 fault, race, and PTY matrices are all legitimately future work and are correctly presented as such. |
| Performance budget realistic for target hardware | ✓ | All §4.7 numbers are loose for single-file operations on a warm SSD; the memory bound matches the verified single-buffer allocation; `--help`'s zero-filesystem-access claim is correct because clap exits inside `Cli::parse()` before `run()` reads XDG. |
| No undeclared dependency on unbuilt features | ✓ | §7.4: "**None external.** This is the root of the dependency tree." Verified — `mg-vault-core` depends on no sibling crate. Every target item is buildable against the current workspace. |

**Feasibility verdict:** Feasible with caveats
**Caveats:** §5.1 over-reports existing test coverage in three unmarked rows (docked at 4A and recorded above); §4.5's rusqlite scoping sentence is imprecise at the binary level (docked at 1A). Both are current-state reporting errors, not design infeasibility. The spec was written after `dfe33cf` and the only newer changes in the tree are B-side, so no A claim is stale.

---

## Composite Score

| Lens | Average | Weight | Weighted |
|---|---|---|---|
| Data ownership and compatibility | 2.80 | 30% | 0.840 |
| Editor and TUI excellence | 2.40 | 25% | 0.600 |
| Retrieval and structure | 2.80 | 20% | 0.560 |
| Security, reliability, extensibility | 2.17 | 15% | 0.325 |
| Performance and accessibility | 2.60 | 10% | 0.260 |
| **Composite** | | | **2.59** |

**Pass conditions (from criteria.md):**
- [x] Composite ≥ 2.0 — 2.59
- [x] All lens averages ≥ 2.0 — 2.80 / 2.40 / 2.80 / 2.17 / 2.60
- [x] No criterion scores 0
- [x] No more than two criteria at 1 per lens — zero 1s in any lens
- [x] All auto-fail rules pass
- [x] Feasibility ≠ Infeasible — Feasible with caveats

**All conditions met:** Yes → **PASS**

---

## Remediation Brief (advisory — PASS, but Priority 1 must be resolved before implementation)

The spec passes every gate. These are not scoring blockers; items 1–4 are defects
that, if carried into code as written, would create the exact failures the spec
exists to prevent. Each is written so a different agent can act without
clarification.

### Priority 1 — Fix before any implementation slice ships

1. **Make the fingerprint check atomic with the commit (§3.2 "Replacing a note" steps 3–4; §3.2 edit-span step 4; §4.4 concurrency model; §5.2 `two_process_conflict`).**
   As written, `write_note`/`edit_note_span` read the source, compare the digest, then
   commit via a *replacing* rename with nothing binding the two — so two callers that
   read the same pre-image both pass the check and both rename, and the second silently
   destroys the first. This contradicts §4.4 ("There is no lock that makes a fingerprint
   check optional"), §5.2 ("there is no last-writer-wins outcome"), and §6.4's "Silent
   conflict winner" clearance, and it is the same failure §1.2 opens with.
   Add a numbered sub-step to §3.2 specifying the enforcement mechanism. Acceptable
   forms, any one of which resolves it — pick one and write it into the flow:
   (a) hold an advisory-plus-`O_EXCL` commit lock file under `.mg-vault/locks/<path-hash>`
   for the read→commit window, and re-read + re-compare the source under that lock
   immediately before the rename; or
   (b) commit with `renameat2(RENAME_EXCHANGE)` against a witness copy and verify the
   swapped-out bytes equal the expected pre-image, rolling back if not; or
   (c) re-`stat` the destination for `(dev, ino, mtime, size)` immediately before the
   rename and abort with `conflict` on any change, and state plainly in §4.6 that this
   narrows but does not close the window.
   Whichever is chosen, update §5.2's `two_process_conflict` to describe how the test
   forces the interleaving deterministically (e.g. a fault-injection hook between the
   fingerprint compare and the rename), and update §4.4's concurrency paragraph so the
   guarantee it states is the guarantee the mechanism delivers.

2. **Stop the trash rollback from using a replacing rename (§3.2 "Trashing and restoring" step 3; §6.4 "Source-content loss").**
   Step 3 currently says "the payload is renamed back", which is a replacing
   `rename(2)`: if anything was created at the original path between the payload move
   and the failed metadata write, the rollback destroys it — while §6.4 claims "every
   failure path leaves the exact pre-image".
   Rewrite step 3 to require the rollback to use the **same non-replacing primitive as
   `create`** (`RENAME_NOREPLACE` / `hard_link`+`unlink`), and specify the branch where
   the destination is now occupied: leave the payload in `.mg-vault/trash/files/<id>`,
   emit `transaction_incomplete` naming the payload id and the occupied path, and add
   the case to §3.6's error table with data-loss risk "No". Add an integration test to
   §5.2 — `trash_rollback_refuses_an_occupied_original_path` — that creates a file at
   the original path between the two steps and asserts both files survive.

3. **Correct the confinement claims that over-report current coverage (§6.4 4A row; §5.1 `rejects_symlink_escape` and `rejects_traversal_absolute_and_protected_mutations`).**
   §6.4's 4A row says "symlink rejection at the final component and every parent", but
   §3.2's own read flow performs no symlink check and the existing-file mutation path
   checks the final component only; the per-parent walk exists solely in
   `prepare_destination` (create/restore). Rewrite the 4A row to state the three
   distinct guarantees per operation class — (i) create/restore: per-component parent
   walk with symlink rejection, (ii) write/edit-span/trash: `symlink_metadata` on the
   final component plus canonical containment, (iii) read: canonical containment plus
   regular-file check, no symlink rejection — and say which class each of §5.1's tests
   actually covers today. Then either strengthen the two §5.1 rows to match reality
   (they currently claim "on read and on every mutation" and "for every mutation entry
   point" plus a `note.txt` case that the real test does not exercise) or mark the
   uncovered halves *(target)*.

4. **Resolve the argv/privacy contradiction (§6.5 bullet 5 vs. §3.1/§3.4 `--body TEXT`).**
   §6.5 states "never place note content in argv, environment, process titles, or
   diagnostics", yet `--body TEXT` is the primary and currently only body source, and
   note content in argv is world-readable via `/proc/<pid>/cmdline`. Either:
   (a) promote `--stdin`/`--body-file` out of *target* into the first implementation
   slice, make `--body` explicitly documented as "convenience only; exposes content to
   other local users via the process table", and add that warning to the §3.1
   invocation rows; or (b) narrow §6.5's rule to "never place note content in argv
   **that mg-vault itself constructs**" and add an explicit accepted-risk paragraph in
   §6.1 covering caller-supplied `--body`.
   Whichever, add a §5.2 test `body_sources_are_mutually_exclusive_and_stdin_is_opt_in`.

### Priority 2 — Should fix for quality

1. **Complete §7.1's Absent list for human rendering (affects 4E and 5D).** §7.1 and
   §7.2 name the path-escaping, color-policy, and exit-code gaps precisely, but not:
   the §3.3 multi-line receipt order, "exactly one suggested recovery command", the
   ≥60/<60 column layouts and the no-truncation wrap rule, or the empty-state copy
   (live output is `no registered vaults`, not §3.3's
   `No registered vaults. Run: mg-vault vault register NAME PATH`). Add them as
   **Absent** so slice sizing is honest.
2. **Fix the two test-inventory errors (§5.1, §7.1).** `registry_rejects_duplicate_name`
   and `registry_name_validation` are listed as implemented but do not exist — mark them
   *(target)*. `foundation.rs` has 14 tests, not 13. Note that
   `frontmatter_bom_and_unclosed_fail_closed` is really
   `rejects_bom_and_unclosed_envelopes` in `src/frontmatter.rs`'s inline test module,
   which §5.1's preamble does not list as a source of implemented tests.
3. **Reword §4.5's rusqlite sentence** to distinguish the A crate (`mg-vault-core`, which
   genuinely does not depend on rusqlite) from the shared `mg-vault` binary (which does,
   via `mg-vault-index`, because it also hosts B's `index`/`search` verbs).
4. **Reconcile §6.4's 2E row with §7.1** — span-edit works at the library level only;
   there is no CLI verb today.
5. **Name where attribution and preview attach (4C/4D).** Add one sentence to §4.3 or
   §6.4 pointing N/O and C at a concrete hook — a `plan`-returning variant of each
   mutation, or an operation record in `.mg-vault/journal/` — so "attributable and
   previewed" is not left entirely to downstream branches.

### Priority 3 — Consider for excellence

1. Settle the trash verb grammar: `note trash` / `note restore ID` / `trash list`
   splits one concept across two groups (§3.1, §3.4).
2. Make §6.4 5A's "asserted by dependency review" executable — a `cargo deny` /
   offline-build CI gate — so a future networked transitive dependency fails loudly.
3. Consider whether `.git` and Syncthing's `.stfolder` need the same first-component
   mutation rejection as `.obsidian`/`.mg-vault` before M lands, and extend §5.2's
   `control_directories_are_untouched` matrix accordingly.
4. State explicitly in §1.3 or §5.2 how the fault matrices inject failure at each
   `fsync`/rename boundary (LD_PRELOAD shim, a `cfg(test)` seam in `atomic.rs`, or a
   filesystem fault-injection layer) — §7.3 correctly identifies this harness as the
   part that "cannot be rushed", so naming the mechanism de-risks the estimate.
