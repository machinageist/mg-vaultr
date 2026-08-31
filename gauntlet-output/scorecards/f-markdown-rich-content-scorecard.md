# Scorecard: Markdown and Rich Content

**Feature ID:** f-markdown-rich-content
**Spec file:** gauntlet-output/specs/f-markdown-rich-content.md
**Reviewer agent:** Spec Gauntlet verification agent (blind review)
**Date:** 2026-08-30
**Spec iteration reviewed:** 1
**Graded against commit:** `dfe33cf` (`dfe33cf617870ea7529283eb6ab8a7c222aa9f0f`, 2026-08-24, "feat: add mg-vault interop snapshot export")

---

## Verdict: PASS

**Summary:** The preservation model is the real thing, not a promise: §4.2 defines a total gapless token cover with an explicit concatenation invariant, `TokenKind::Unrecognized` as a first-class kind, and §4.3's gap filler that makes `parse` total over a parser (`pulldown-cmark::OffsetIter`) that is not itself total — this is span description over immutable bytes, not parse-to-AST-and-reprint, and the only mutation primitive is a fingerprint-bound disjoint splice committed by `mg-vault-core`. The most critical gap is that the spec silently drops one binding rule from the repo's own spike (`docs/spikes/token-preserving-markdown-yaml.md` §"Staged architecture" item 3: *"Candidate YAML must be reparsed before commit"*) — grep for `reparse`/`re-parse`/`revalidat` returns zero hits on candidate bytes anywhere in §3.2, §4.3, or §5, so a plan that passes hash/range/kind validation but whose replacement bytes break YAML or the frontmatter envelope commits unopposed. That is a fail-open in an otherwise fail-closed model, and it costs 1E, but it does not touch any auto-fail rule.

---

## Lens 1: Data Ownership and Format Compatibility (weight: 30%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 1A Authority | 3 | §4.4 "Authoritative state: the note bytes and the attachment files. F reads them and never owns them"; §4.2 the render cache is "disposable and never consulted for source bytes"; §3.3 explicitly forbids presenting an index answer as source (labels `indexed`/`direct`/`unknown`); §6.4 1A. `SourceDocument` holds `Arc<str>` + a description of it, nothing else. | — |
| 1B Preservation | 3 | §4.2 `tokens` carries the stated INVARIANT that concatenating `source[t.byte_range]` reproduces `source` byte-for-byte; `Unrecognized` is a documented first-class kind, not a failure. §4.3 names the *mechanism*: the `OffsetIter` driver plus a gap filler that emits every uncovered byte run as `Whitespace`/`LineEnding`/`Unrecognized`, which is why `parse` has no error case. §4.2 bans `Display`/`to_string`/`serialize`/`write_source`/`render_markdown` on `SourceDocument`/`Token`/`Node`, and §5.1 `no_document_serializer_in_public_api` is an API-surface golden that enforces it rather than promising it. Mutation is `edit_note_spans` splices only (§4.3); §4.3 also states media is never base64'd into source and §4.3 HTML "preserved byte-identically and never rewritten in the file"; §3.2 "Branch — unsupported construct" renders literal text and "never escaped into the file". Gates: §5.1 `unknown_syntax_becomes_unrecognized`, `targeted_edit_preserves_prefix_and_suffix`; §5.2 `corpus_round_trip` (empty plan → byte-identical SHA-256) and `structural_edit_preserves_everything_else`. No whole-document serializer path exists in any of render (stdout), export (separate artifact), cache (XDG, keyed by fingerprint), or transclusion ("render-only: no target bytes are ever written into the host file"). | — (see P2-1: the golden test is scoped to "the crate's public items", which excludes the `frontmatter_scalar.rs` YAML adapter §7.2 migrates inside `mg-vault-core`; §5.2's on-disk assertions do cover it behaviorally) |
| 1C Identity | 3 | §6.4 1C "never injects a UUID, a `^blockid`, or any identifier into a note"; §3.2 step 1 and §4.3 CLI contracts require an exact vault-relative path with no fuzzy resolution; block IDs and heading anchors are read-only (§4.3 Transclusion anchors). | — |
| 1D Coexistence | 3 | §4.2 `TokenKind` has `WikiLink`, `Embed`, `CalloutMarker`, `Tag`, `BlockId`, `ObsidianComment`, `Highlight` as first-class kinds with sub-ranges; §4.3 "`.obsidian` is read, never written, by any code path in this feature", read narrowed to `app.json` through a separate capability; §5.2 `obsidian_directory_never_written` asserts mtimes and bytes unchanged and that generic writes are refused by `validate_note_path` (verified: `crates/mg-vault-core/src/vault.rs` rejects `.obsidian`/`.mg-vault` first components when `mutation` is true); §5.1 `attachment_policy_precedence` re-asserts `.obsidian` bytes unchanged in every case; portable settings live in `.mg-vault` (§6.4 1D). §4.6 preserves `LineEndings::{Lf,CrLf,Mixed}` as observed. | — (minor: §3.4 lists `--show-comments` but §3.3 never states the default rendering of `%%comment%%`) |
| 1E Transactions | 2 | Strong on ranges and staleness: §3.2 step 3 validates revision equality, sorted non-overlapping ranges, UTF-8 boundaries, `lexeme_hash`, `surrounding_hash`, and expected token kind; §4.3 `edit_note_spans` rejects "overlapping, unsorted, out-of-bounds, or non-boundary spans … before I/O"; §3.6 rows `conflict`, `structural_target_unavailable`; §5.2 `multi_span_commit_is_atomic` and `stale_revision_rejects_plan`; ambiguity is always a terminal state that never auto-selects (§3.6 `media_ambiguous`, `embed_ambiguous`). **Gap:** nothing validates the *replacement* bytes. The spike's binding rule "Candidate YAML must be reparsed before commit" appears nowhere (zero hits for reparse/re-parse/revalidat on candidate bytes), and no rule requires the post-splice document to still classify the edited range as the same token kind. A `frontmatter_value` replacement containing `\n---\n`, or a `link_target` replacement containing `]]`, passes every listed check and commits. | Add to §3.2 step 3 / §4.3 a mandatory post-splice validation: re-scan the candidate bytes and reject if the frontmatter envelope, the edited token's kind, or the token cover boundaries change outside the replaced span; add a §5.1 row (`candidate_bytes_are_revalidated`) covering envelope-breaking, delimiter-injecting, and kind-changing replacements. |

**Lens average:** 2.80
**Lens pass:** Yes — avg ≥ 2.0, zero 1s, no 0s

---

## Lens 2: Editor and TUI Excellence (weight: 25%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 2A Keyboard completeness | 3 | §3.4 defines a full preview-action grammar (`p`, `zc`/`zo`, `gd`, `gr`, `yc`, `]b`/`[b`, `]d`/`[d`, `gi`, `gh`) with F owning actions and E owning binding, and states every action is reachable as `:render <action>` with "no pointer-only or graphics-only action"; §3.7 adds custom actions for tables/diagrams/transclusions; §3.4 correctly marks mouse/touch/stylus/voice N/A for a terminal product. | — |
| 2B Editing durability | 2 | Correctly delegated, not dodged: §4.4 "F persists no drafts … D's recovery journal owns the proposed bytes and F contributes nothing that could be replayed over a newer source"; §6.4 2B ties durability to D's journal and core's atomic replacement. **Gap in F's own seam:** the spec never defines what a `SourceDocument`/`RenderDocument` does when the underlying file changes on disk while a preview holds it — no refresh, invalidation, or must-revalidate-before-display contract, and §4.4's staleness discipline is scoped only to index-sourced resolution answers. | State in §4.4 that a `RenderDocument` is invalid once its `revision` no longer matches the file, and name which side (E's watcher, per d-editor-engine §4.1's `mg-vault-app` file watching) must revalidate before repaint. |
| 2C Workspace | 2 | Addresses only F's seam, with justification: §3.1 lists the "Preview pane view model" row (F supplies model, E owns placement) and marks the source pane "Unmodified"; §4.4 "E owns the preview pane store and holds a `RenderDocument` per visible buffer"; §6.4 2C assigns panes/tabs/splits/session restore to E. Panes, splits, and session restore are genuinely out of scope for F, so this is a justified partial rather than an evasion, but F contributes no session-restore contract (e.g. whether fold state survives a restore) despite owning fold state in §3.5. | Say whether the per-block fold boolean in §3.5 is session-persistable and who owns it. |
| 2D Text correctness | 3 | §3.7 "Unicode correctness": grapheme clusters and a pinned width table for wrapping/alignment/cursor mapping, combining marks, ZWJ, CJK, RTL; §4.6 "width never affects byte offsets, identity, or `--json`"; §4.3 UTF-8 boundary enforcement on every span; §5.1 `grapheme_layout_is_correct`; §3.7 escapes bidi/C0/C1 in *metadata* while keeping note-body bytes byte-exact — the correct split. | — |
| 2E Degraded experience | 3 | §4.6: `images`/`diagrams` feature flags degrade to the textual card/outline "which is already the mandatory path, so semantics do not vary by build"; confinement unavailable ⇒ attachment reads disabled and `Blocked{reason}` rather than a lexical fallback, note rendering continues; §3.6 `resolver_unavailable`; §5.2 `degraded_without_index`; §3.4 tier ladder falls to `unicode` on detection failure, never to a richer tier. | — |

**Lens average:** 2.60
**Lens pass:** Yes — avg ≥ 2.0, zero 1s, no 0s

---

## Lens 3: Knowledge Retrieval and Structure (weight: 20%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 3A Determinism | 3 | §4.1 one Markdown authority for the workspace; B's `PARSER_VERSION` becomes derived from `SYNTAX_PROFILE` and "B stops carrying its own `wikilinks()` scanner"; §5.2 `index_uses_the_shared_parser` asserts no independent wikilink scanner survives and that a profile change invalidates the published generation; ordering is source byte order everywhere (§4.3). Verified feasible: `crates/mg-vault-index/src/lib.rs:17` already has `PARSER_VERSION` written into the metadata row and compared on read. | — |
| 3B Ambiguity | 3 | §4.3 resolution step 3: "Two or more → `Ambiguous { candidates }`, ordered by vault-relative byte order, **never auto-selected**"; §3.6 `media_ambiguous`/`embed_ambiguous` rows require ordered candidates and no chosen target; §4.3 transclusion duplicate headings/block IDs are `Ambiguous`; §5.1 `media_resolution_matrix` "Ambiguity never auto-picks". | — |
| 3C Query depth | 2 | §6.4 3C claims F "emits byte-ranged structural projections — headings, blocks, block IDs, tags, properties, tasks, links, embeds, code, math, diagrams, media" as B's substrate, and the `TokenKind` enum in §4.2 does carry `Tag`, `TaskMarker`, `BlockId`, `WikiLink`, `Embed`, `HeadingText`, `FenceOpen{info}`, `MathInline/Display`. **Gap:** no projection API is actually specified. §4.3's public surface is `parse`, `cover_is_total`, `find_span`, `validate_plan`, `expand` plus CLI commands — there is no typed accessor (`headings()`, `tags()`, `links()`, `tasks()`) and no projection type in §4.2, so the crate three branches block on does not define how B consumes it. Dates and typed properties correctly belong to I. | Add a §4.3 projection contract: the named accessor(s) B calls, their return type, ordering guarantee, and whether they are computed lazily over `tokens` or materialized. |
| 3D Derived authority | 3 | §4.2 "No database migration is introduced — F stores nothing durable except an optional owner-only render cache … disposable and never consulted for source bytes"; §6.4 3D "the render model, outline, and cache are views over ordinary files; none can be edited to change a note"; §4.4 marks owned durable state as none. | — |
| 3E Scale | 3 | §4.7 ties F's budgets to B's corpus explicitly: ≥20 MiB/s parse, ≤40 µs/KiB amortized "which keeps **B**'s 100,000-note / 1,000,000-block corpus inside its own budget", p50/p95/p99 plus peak RSS published against it; covers dropped when a note closes; 64 MiB/document ceiling with large-document mode instead of OOM; every operation cancellable within 4 ms or 64 KiB; §5.2 `large_document_mode` exercises a 200 MiB note. | — |

**Lens average:** 2.80
**Lens pass:** Yes — avg ≥ 2.0, zero 1s, no 0s
**Auto-fail triggered:** No

---

## Lens 4: Security, Reliability, and Extensibility (weight: 15%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 4A Confinement | 3 | §4.3 `read_attachment` uses "the `openat2` + `RESOLVE_BENEATH \| RESOLVE_NO_SYMLINKS` path already used by `index::read_source_bytes`" (verified present in `crates/mg-vault-core/src/index.rs`, Linux-only with an explicit refusal on other platforms) and rejects `.obsidian`/`.mg-vault` first components for generic reads; §4.6 disables attachment reading entirely where descriptor-relative confinement is unavailable rather than falling back to a lexical check; §4.3 resolution step 4 blocks absolute paths, `file:`, `..`, and symlink escapes; §3.6 `unsafe_path` names the rule, not the resolved path; §5.2 `render_never_reads_outside_the_vault`. | — |
| 4B Concurrency | 3 | §4.2 `TokenSpan` carries `revision`, `lexeme_hash` (SHA-256 of the lexeme), and `surrounding_hash` (±64 bytes); §4.3 the commit is fingerprint-checked in core immediately before the atomic replacement (matches the existing `edit_note_span` in `vault.rs`, which re-reads and compares before `replace_atomic`); §3.6 `conflict` uses the existing `Error::Conflict` with expected/actual digests and marks both versions intact; §5.2 `stale_revision_rejects_plan`. | — |
| 4C Least privilege | 3 | §4.1 "Nothing in `mg-vault-markdown` opens a file by absolute path, spawns a process, or opens a socket"; §4.5 mermaid is project-owned Rust with "no JavaScript runtime, no headless browser, no `eval`, no network"; §4.3 the writer accepts only `SafeInline`/`SafeBlock` with fields private to `render/sanitize.rs`, so an unsanitized write "fails to compile"; §5.1 `sanitizer_is_the_only_constructor` (trybuild compile-fail); §6.4 4C "Plugins and AI get render output, never a mutation authority". | — |
| 4D Recovery | 3 | §3.6's data-loss column is `No` for all 19 rows with the reason given ("F is a read-and-describe feature, and the only write path is the caller-driven, fingerprint-checked splice"); §3.2 step 4 "Validation failure aborts with zero mutation"; §5.2 `multi_span_commit_is_atomic` injects failure between splice construction and rename and asserts wholly-old-or-wholly-new; §6.4 4D "F never claims a save". Trash/history correctly stays with A/core. | — (the "previewable" claim in §6.4 4D rests on the caller holding `EditPlan`; no dry-run API is named, which is acceptable since `SpanReplacement` carries the literal bytes) |
| 4E Contracts | 2 | Versioning is right: §4.2 pins `SYNTAX_PROFILE`, `HTML_ALLOWLIST_VERSION`, `RENDER_SCHEMA`, and §4.6 requires a snapshot gate and index rebuild on any bump. **Three truthfulness gaps.** (i) §4.3's envelope `{"version":1,"ok":true,"command":"render","data":{…},"warnings":[]}` adds `command` and `warnings` keys that the real envelope does not have — `crates/mg-vault-cli/src/main.rs` defines `struct Envelope<T> { version: u8, ok: bool, data: T }` — yet §3.1 calls this "the version-1 envelope convention established in `crates/mg-vault-cli/src/main.rs`". (ii) The `mg.render/1` block schema is never enumerated: §3.2's only block example `{"kind":"unrecognized","raw":"…","byte_range":[…]}` carries a `raw` field that `RenderBlock` (§4.2) does not have, and `state`/`text_equivalent`/`children` never appear in a JSON sample. (iii) §4.3 defers exit codes to `specs/c-cli-note-operations.md` §4.3 (which does define them: 0/2/3/4/5/6/7/130) but **C is absent from §7.4's blocking dependencies**, even though F adds four subcommands to the CLI surface C owns. | Reconcile §4.3's envelope with `Envelope<T>` (either add `command`/`warnings` as an explicit envelope change in §7.2 or drop them); enumerate the `mg.render/1` block object field-by-field and make §3.2's example conform to `RenderBlock`; add C to §7.4. |
| 4F Privacy | 3 | §6.1: zero network I/O so an untrusted clipped note cannot beacon, owner-only XDG cache keyed by fingerprint, diagnostics "name rules and vault-relative paths but never note bodies, absolute host paths, or environment values", and the existing `private`/`.private`/`secrets`/`.secrets` exclusions honored at the export seam (verified present in `crates/mg-vault-core/src/interop.rs`); §4.7 "Network payload: zero bytes" asserted by §5.2 `render_performs_no_writes_and_no_network`. | — |

**Lens average:** 2.83
**Lens pass:** Yes — avg ≥ 2.0, zero 1s, no 0s

---

## Lens 5: Performance and Accessibility (weight: 10%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 5A Offline/local-first | 3 | §4.3 "`http:`/`https:` destinations are never fetched by the terminal renderer"; §4.5 "no network egress of any kind"; §4.7 zero network payload; §5.2 socket-failing shim test; §6.4 5A remote URLs render as inert link cards. | — |
| 5B Responsiveness | 3 | §4.7 gives parse throughput, per-note p50/p95/p99, memory multiplier and hard ceiling, 16 ms viewport render, 4 ms incremental, decode/diagram/transclusion budgets, cancellation granularity, 64 MiB LRU cache bound, and an explicit zero startup cost. | — |
| 5C Accessible equivalents | 3 | §4.2 makes `text_equivalent: String` non-optional so "a block cannot exist without one, which is how the accessibility guarantee is enforced by type"; §3.7 requires alt text or the literal `description: unavailable` (never a filename promoted to a description), node+edge lists for diagrams, LaTeX source plus a Unicode approximation for math, and record-form linearization for tables; §3.2 step 8 — in the `graphics` tier image cells are emitted "**after** the textual card line, never instead of it"; §3.4's tier table shows no construct reachable only via a graphics protocol; §5.1 `render_block_has_text_equivalent` and `mermaid_fallback_always_has_text`; §5.2 `render_tier_semantic_equivalence` asserts the `(kind, state, text_equivalent)` inventory is identical across all four tiers and equal to `--json`. No pixels-only construct exists. | — |
| 5D Terminal resilience | 3 | §3.4 four-tier ladder with explicit detection rules and record-form thresholds at 60 and 40 columns, "cells are never truncated"; §3.7 every state is a literal word and color is decoration only, `[x]` not green; `NO_COLOR` wins over auto-detection; §3.5 `MG_VAULT_REDUCED_MOTION=1` and non-TTY disable progressive painting; §5.4 diffs ANSI-stripped output between themes. | — |
| 5E Automation | 3 | §3.1 stdout/stderr discipline with the other stream empty; §3.4 `--json`, `--plain`, `--ascii`, `--no-color`, `--no-input`, `--from-stdin` with `--base-path`; §4.3 arrays always present with documented stable ordering; §5.3 test 7 pipes `--json` and `--from-stdin` through `jq` and asserts clean stream separation. | — (schema under-specification already penalized under 4E) |

**Lens average:** 3.00
**Lens pass:** Yes

---

## Auto-fail Walk (each rule individually)

| Rule | Result | Basis |
|---|---|---|
| Source-content loss | Pass | F has no write path (§4.3 "F performs no writes. It emits plans"); the only mutation is a disjoint splice in core; §5.2 `corpus_round_trip` asserts whole-file SHA-256 equality after an empty plan. |
| Partial multi-file mutation | Pass | Single-file only; §4.3 `edit_note_spans` is "one fingerprint-checked atomic write"; §5.2 `multi_span_commit_is_atomic` forbids an observable partial subset. Matches the spike's rule that repeated single-span commits are not a substitute. |
| Index state overriding source | Pass | §3.3 "it never presents an index answer as source"; §4.4 index answers labeled `indexed` with B's generation, direct probes `direct`, absence `unknown`. |
| Stale index presented as current | Pass | §4.4 staleness discipline; §5.2 `degraded_without_index` asserts nothing claims `resolved` without evidence or claims index freshness. |
| Silent conflict winner | Pass | §4.3 resolution never auto-selects; §3.6 every ambiguity row is a terminal state with ordered candidates. |
| **Unknown syntax loss** | **Pass** | Total gapless cover invariant (§4.2) + `TokenKind::Unrecognized` as a first-class kind + gap filler making `parse` total (§4.3) + no serializer, enforced by §5.1 `no_document_serializer_in_public_api` rather than promised + §5.1 `unknown_syntax_becomes_unrecognized` + §5.2 `structural_edit_preserves_everything_else`. §3.2's unsupported branch renders literal text and never rewrites the file. Model is span-preserving, not parse-and-reprint. |
| Unconfirmed overwrite/import | Pass | F imports nothing and overwrites nothing; every plan carries a revision core revalidates. |
| Unsafe traversal or symlink escape | Pass | §4.3/§4.6 `openat2` + `RESOLVE_BENEATH \| RESOLVE_NO_SYMLINKS`, disabled rather than downgraded where unavailable; §5.2 confinement shim test. |
| Capability/data-exfiltration bypass | Pass | No network, no process spawn, no `eval`, sanitizer-only output constructors (§4.1, §4.3, §4.5), asserted by §5.2. |
| **Active raw HTML/script by default** | **Pass** | §4.3: `--html-mode literal` is the default "everywhere, including export unless P overrides it explicitly"; `sanitized` mode is an explicit element/attribute allowlist with everything else — including `on*`, `style`, `class`, comments, CDATA, PIs, namespaced names, and parse-differential constructs — dropped and reported; URL schemes allowlisted to `http`/`https`/`mailto`/vault-relative with percent/unicode/whitespace obfuscation normalized before the check and unparseable schemes denied; anchors forced to `rel="noopener noreferrer nofollow"` with `target` removed; SVG never inlined or executed. Enforcement is structural, not procedural: the writer accepts only `SafeInline`/`SafeBlock` whose fields are private to `render/sanitize.rs`, so an unsanitized render or export path fails to compile (§5.1 `sanitizer_is_the_only_constructor`). Source HTML is preserved byte-identically and never rewritten. Q4 correctly flags P's export default as open without weakening F's default. |
| Non-atomic save claiming success | Pass | §6.4 4D "F never claims a save"; core's `replace_atomic` (temp + fsync + rename + parent fsync) is the commit path. |
| Recovery overwriting newer source | Pass | §4.4 F persists no drafts and holds no recovery state; D's journal owns proposed bytes and every plan is revision-checked. |
| Graph/Canvas info lacking a textual equivalent | Pass | Non-`Option` `text_equivalent` (§4.2), diagram node/edge outline mandatory even on parse failure or budget exhaustion (§3.3, §5.1 `mermaid_fallback_always_has_text`), tier-equivalence test (§5.2). |

**Any auto-fail triggered:** No

---

## Feasibility Check

Verified against real source at `dfe33cf`.

| Check | Status | Notes |
|---|---|---|
| Types/models exist or are clearly specified | ✓ | §7.1 is accurate item by item. `scan_frontmatter` in `crates/mg-vault-core/src/frontmatter.rs` does locate exact YAML/body ranges, requires an exact `---` open and close, reports `LineEndings::{Lf,CrLf,Mixed}` without normalizing, and fails closed on BOM/unclosed — and it has exactly seven unit tests covering the listed cases. The claim that the top-level scanner is **hand-rolled** is correct twice over: `scan_frontmatter` is a byte-level `next_line` loop with no YAML library, and `locate_frontmatter_scalar` in `frontmatter_scalar.rs` uses `yaml_edit::Document` only for validation/scalar type-checking, then re-locates the value with its own `split_inclusive('\n')` line scan and a hand-written `strip_comment`. §7.1's "absent" note that `Scalar::byte_range` is unused and that block scalars, flow collections, multi-line values, nested keys, and duplicate-occurrence selection are unsupported is exactly right. `TokenSpan`'s shape matches what `specs/d-editor-engine.md` line 399/581 actually demands (`{revision, byte_range, token_kind, surrounding_hash}`); F adds `lexeme_hash` as a compatible superset. |
| API/interface changes are feasible with current architecture | ✓ | `edit_note_spans` is a mechanical generalization of `Vault::edit_note_span` (`vault.rs`), which already does fingerprint compare → UTF-8 boundary check → checked-arithmetic capacity → prefix/replacement/suffix copy → `replace_atomic`. `read_attachment` mirrors the existing private `index::read_source_bytes` `openat2` path via `rustix` (already a workspace dep), which matters under the workspace's `unsafe_code = "forbid"` lint. |
| Views/screens fit current navigation pattern | ✓ | `render`/`outline`/`media`/`transclude` slot alongside the existing clap `Command` enum (`Vault`, `Note`, `Index`, `Search`, `Interop`) in `crates/mg-vault-cli/src/main.rs`. |
| Dependencies are available and version-compatible | ✗ | `yaml-edit 0.2.3` with `default-features = false` is confirmed already in `vault/Cargo.toml` `[workspace.dependencies]`, exactly as §4.5/§7.1 claim. Every other named crate is genuinely new: `pulldown-cmark`, `unicode-segmentation`, `unicode-width`, `image`, `markdown-rs`, `yaml-rust2` — all declared in §4.5. **Two are not:** `trybuild` is required by §5.1 `sanitizer_is_the_only_constructor` and appears in §7.2's Cargo.toml list but is missing from §4.5 *and* from the §6.2 provenance table; and the "terminal-graphics encoder for Kitty/iTerm2/Sixel" is a category, not a named crate, so it has no version, no license row in §6.2, and no rights status. `pulldown-cmark` itself is never version-pinned even though §4.6 makes a bump a gated event and §4.7 pins `unicode-width`. |
| Platform/renderer requirements are realistic | ✓ | Linux-first with `openat2`; §4.6's "disable attachment reads where confinement is unavailable" matches the existing `#[cfg(not(target_os = "linux"))]` refusal in `index.rs`. The 150 ms bounded capability query and TTY-only escape emission are standard. |
| Test strategy is executable with current infrastructure | ✗ | The only dev-dependency in the workspace today is `tempfile` (both crate manifests). §5.1–§5.3 require, undeclared: a property/fuzz engine (proptest/arbitrary or cargo-fuzz) for `cover_is_total_and_ordered` and `parse_is_total_and_panic_free`; `trybuild`; a PTY driver for the four §5.3 terminal scenarios; a write-failing filesystem shim and a socket-failing syscall shim (§5.2 `render_performs_no_writes_and_no_network`); a confinement shim; and a fault-injection point between splice construction and rename inside `atomic.rs::replace_atomic` for `multi_span_commit_is_atomic` — §7.2's modified-files list does not include `atomic.rs`. The repo does have precedent for an injectable guard (`mg-vault-index` exposes a "testable guard immediately before generation publication"), so this is a declaration gap, not an impossibility. |
| Performance budget is realistic for target hardware | ✓ (caveat) | ≥20 MiB/s and ≤40 µs/KiB are comfortable for `pulldown-cmark` plus a gap filler and a second restricted scan. The "≤ 6× source bytes" cover+arena ceiling is the tight one: `TokenKind` carries up to three `Range<usize>` payloads (`WikiLink`), so `Token` lands near 80–88 bytes; marker-dense fixtures averaging one token per ~10 bytes would exceed 6× before the node arena is counted. Fixable with `u32` offsets and side tables, but the budget should say so. |
| No undeclared dependency on unbuilt features | ✗ | §7.4 lists A, B, G, D, E, P but omits **C**, whose spec §4.3 supplies the exit-code categories F adopts (`specs/c-cli-note-operations.md` line 350: 0/2/3/4/5/6/7/130) and which owns the CLI surface F extends with four subcommands. |

**Feasibility verdict:** Feasible with caveats
**Caveats:** undeclared test infrastructure and `trybuild`/graphics-encoder dependencies; unpinned `pulldown-cmark`; C missing from §7.4; the 6× memory ceiling is optimistic for the specified `TokenKind` layout.

---

## Composite Score

| Lens | Average | Weight | Weighted |
|---|---|---|---|
| Data Ownership and Format Compatibility | 2.80 | 30% | 0.840 |
| Editor and TUI Excellence | 2.60 | 25% | 0.650 |
| Knowledge Retrieval and Structure | 2.80 | 20% | 0.560 |
| Security, Reliability, and Extensibility | 2.83 | 15% | 0.425 |
| Performance and Accessibility | 3.00 | 10% | 0.300 |
| **Composite** | | | **2.78** |

**Pass conditions (from criteria.md):**
- [x] Composite ≥ 2.0 — 2.78
- [x] All lens averages ≥ 2.0 — 2.60 lowest (Lens 2)
- [x] No criterion scores 0
- [x] No more than two criteria at 1 per lens — zero 1s anywhere
- [x] All auto-fail rules pass — all 13 walked individually above
- [x] Feasibility ≠ Infeasible — Feasible with caveats

**All conditions met:** Yes → PASS

---

## Remediation Brief (non-blocking — spec passes)

### Priority 2 — Should fix for quality

1. **Restore the spike's candidate-revalidation rule (§3.2 step 3, §4.3, §5.1).** `docs/spikes/token-preserving-markdown-yaml.md` binds "Candidate YAML must be reparsed before commit"; the spec's validation list stops at revision/ranges/UTF-8/hashes/kind. Require a post-splice re-scan that rejects a plan when the frontmatter envelope, the edited token's kind, or cover boundaries outside the replaced span change, and add the matching §5.1 test. This is the only place the spec silently diverges from the repo's own accepted architecture.
2. **Specify the `mg.render/1` block schema and reconcile the envelope (§4.3, §3.2, §7.2).** Enumerate every field of a block object; make §3.2's `{"kind":"unrecognized","raw":…}` example match `RenderBlock` (which has no `raw`); and either drop the `command`/`warnings` keys or declare them as an explicit change to `Envelope<T>` in `crates/mg-vault-cli/src/main.rs`, which today carries only `version`/`ok`/`data`.
3. **Declare the test and build dependencies the test spec assumes (§4.5, §6.2, §7.2).** Add `trybuild`, a property/fuzz engine, a PTY driver, and a named terminal-graphics encoder crate (with its license row in §6.2); list `crates/mg-vault-core/src/atomic.rs` among modified files if `multi_span_commit_is_atomic` needs a fault-injection seam there.
4. **Add C to §7.4 blocking dependencies** and note that F's exit codes and CLI-surface conventions come from `specs/c-cli-note-operations.md` §4.3.
5. **Specify a projection contract for B (§4.2/§4.3).** Name the accessor(s) and return type by which `mg-vault-index` obtains headings, tags, tasks, links, embeds, and property spans, so §6.4 3C's claim is backed by an API rather than by the token enum alone.
6. **Define preview staleness (§4.4).** State that a `RenderDocument` whose `revision` no longer matches the file is invalid for display and which component revalidates.

### Priority 3 — Consider for excellence

7. Widen §5.1 `no_document_serializer_in_public_api` beyond "the crate's public items" to cover the `mg-vault-core` YAML adapter that §7.2 migrates, so the no-serializer rule follows the adapter across crate boundaries.
8. Revisit the "≤ 6× source bytes" ceiling in §4.7 against the actual `TokenKind` layout, or specify `u32` offsets / out-of-line payloads for the multi-range variants.
9. Pin `pulldown-cmark` to an exact version in §4.5, matching the treatment `unicode-width` already gets in §4.7.
10. State the default for `--show-comments` (§3.3/§3.4) so `%%comment%%` rendering is unambiguous.
