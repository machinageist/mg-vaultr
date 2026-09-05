# Scorecard: Links, Search, and Graph

**Feature ID:** g-links-search-graph
**Spec file:** docs/specs/g-links-search-graph.md
**Reviewer agent:** Verification agent (blind review)
**Date:** 2026-08-30
**Spec iteration reviewed:** 1
**Graded against commit:** `dfe33cf`

---

## Verdict: PASS

**Summary:** This spec's anchor lens is its strongest work: the resolution ladder is total, ordered,
and explicitly forbidden from tie-breaking (§3.2), ambiguity is a terminal state with no target
(§3.2 step 4, §4.2 `Resolution::Ambiguous`), and the graph's textual mode is declared canonical with
a set-equality conformance test gating the auto-fail rule (§3.7, §5.2). No auto-fail rule is
triggered; all four aimed at this branch are addressed with named mechanisms and named tests. The
most critical gap is contract drift against sibling spec B: G's freshness vocabulary
(`stale | rebuilding | degraded | unknown | unavailable`, §3.2/§3.6) does not match B's `Freshness`
enum (`Current | CatchingUp | Stale | Rebuilding | Blocked | Unavailable`, B §4.2), and G adds a
`Query`/`ExplainQuery` IPC pair alongside B's existing `Search`/`ExplainQuery` without reconciling
them (§4.3 vs B §4.3).

---

## Lens 1: Data Ownership and Format Compatibility (weight: 30%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 1A Authority | 3 | §4.4 splits "Derived state" (resolutions, candidates, basename map, mention cache, graph adjacency, saved-query rows — "disposable, and rebuildable from unchanged source") from "Authoritative state ... the files". §4.1: the index crate "keeps B's hard rule: **no source-mutation API anywhere in it.**" §3.2 search step 8: G "never opens the SQLite file from the CLI or TUI"; §3.3 Data sources: "No view reads SQLite directly, and no view treats an index row as note content." §5.2 rebuild-equivalence and "Saved queries survive index loss" make it testable. Saved queries are ordinary notes (§3.2 Saved queries 1), and the `saved_queries` table is "a discovery pointer only" (§4.2) — verified against the 3D/1A instruction. | — |
| 1B Preservation | 3 | G writes almost nothing. The single write path (§3.2 Saved queries 4) "touches **only** the bytes between the markers (criterion 1B)" through A's atomic transaction with an expected-fingerprint precondition; §5.2 "Materialized snapshot transaction" asserts a successful refresh changes only inter-marker bytes and a failed one leaves the note byte-identical. §3.2 Saved queries 5: malformed query text "is never rewritten, never auto-corrected, and never removed." §4.1 forbids a second Markdown dialect; §3.2 step 1 treats an undefined reference-link label as literal text, "produces no edge, and is not an error." §5.2 Obsidian coexistence asserts unknown syntax "passes through untouched." Feasible today: `Vault::edit_note_span` (crates/mg-vault-core/src/vault.rs:181) already implements exactly this single-span, fingerprint-guarded, atomic replace. | — |
| 1C Identity | 3 | §3.2 "Byte-exact everywhere ... Path identity is bytes (criterion 1C, foundation A)" — no case folding, no Unicode normalization, no accent stripping; a case/NFC-NFD difference is `unresolved` with a `near_miss` naming the exact byte difference. §4.2: "**No table stores a note identifier that appears in a file**; no UUID is ever injected (criterion 1C)." §7.5 non-goals: "injecting UUIDs or any hidden identifier." §4.6 fixture-tests case-insensitive filesystems and NFD paths per platform. §3.7: "path identity stays exact bytes." | — |
| 1D Coexistence | 2 | `.obsidian` half is solid: §5.2 "Obsidian coexistence" asserts `.obsidian` is "never read as authority, never written, never indexed as notes"; §4.4 places query drafts/history "under XDG state (never inside a vault, never inside `.obsidian`)"; §3.2 Saved queries 1 keeps both saved-query forms "visible in any editor and in Obsidian"; §3.2 aliases read "(list or scalar, per Obsidian)". The `.mg-vault` half is not addressed: G never states where its own per-vault portable settings live. The two settings it plausibly needs are deferred to open questions — Q1 (per-vault tier-3 disable) and Q4 (vault timezone, which explicitly lists "a `.mg-vault` config key" as one unresolved option) — so §4.6/§4.4 make no positive `.mg-vault` isolation commitment. | State in §4.4 that any G-owned per-vault setting (tier-3 ladder policy, comparison timezone, mention defaults) lives in `.mg-vault` portable config and never in `.obsidian` or a note, and that G reads but never writes `.obsidian`. |
| 1E Transactions | 3 | §3.6's 17-row error table records "Data-loss risk: No" for every row with a concrete recovery. Ambiguity fails closed with no target (§3.2 step 4). Freshness fails closed with zero rows and exit 6 (§3.2 Search 3), and `--allow-stale` "is refused for any flow that feeds a mutation". §3.2 Backlinks 1 requires "an exact, confined vault-relative path (no fuzzy selection for anything that later feeds a mutation)". Snapshot markers that are "missing, malformed, nested, or unbalanced" report `saved_query_invalid` and write nothing (§3.2 Saved queries 4). §5.2 "Materialized snapshot transaction" injects a concurrent external edit and asserts precondition failure with no partial marker write. `cursor_expired` prevents mixing generations (§3.2 Search 6). | — |

**Lens average:** 2.80
**Lens pass:** Yes — avg ≥ 2.0, zero 1s, no 0s

---

## Lens 2: Editor and TUI Excellence (weight: 25%)

Lens 2 substantively belongs to branches D/E. G does not claim a blanket N/A; instead §3.1 states the
ownership boundary ("The TUI panes are consumers, not owners: E supplies pane geometry, focus, and
session restore; G supplies the model, ordering, labels, and freshness") and §7.4 gates pane work on
E while asserting "Absent E, every G capability remains fully available from the CLI." That is a
justified delegation, graded on the slice G retains rather than penalized as a dodge.

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 2A Keyboard completeness | 3 | §3.4 enumerates the full CLI surface (`search`, `query`, `links`, `backlinks`, `mentions`, `graph`, `query list\|run\|save\|refresh`, `links doctor`), the shared retrieval flag set, the graph flag set, and a Vim-consistent TUI keymap (`<leader>f/b/g/q`, `gd`, `gD`, `[[`/`]]`, `<C-o>`/`<C-i>`, `gr`, `gm`, plus in-pane `j/k/l/h/<CR>/f/u///s/?`), with the discoverability rule "Every binding appears in `:help mg-vault-graph` and in `mg-vault graph --help`." §3.7 makes mouse strictly redundant if E ever adds it. §3.1 guarantees "Every pane has a CLI equivalent that produces the same records." §5.3 asserts every TUI E2E step is keyboard-only. | — |
| 2B Editing durability | 2 | G's only write is covered well: atomic, fingerprint-preconditioned, marker-bounded, and §4.4 guarantees "a failed refresh leaves the prior snapshot byte-identical." External-edit detection on the read path is specified (§3.2 Backlinks 4 re-reads through A and compares fingerprints; §3.6 `source_changed`). Autosave, persistent undo, and merge are correctly delegated to C/D/E (§7.4). Gap: a *successful* `query refresh` overwrites the prior snapshot with no stated trash entry, history record, or undo — the spec covers the failure path but not regret on the success path, for the only bytes G ever writes into a vault. | In §3.2 Saved queries 4 / §4.4, state whether a successful materialized-snapshot refresh is recoverable (trash-backed via A, or a retained prior-snapshot marker), or state explicitly that it is not and why that is acceptable for derived bytes. |
| 2C Workspace | 3 | §3.1's view inventory gives every pane an entry point, new/modified status, and layout pattern, including the two-region graph pane. §4.4 assigns view state (focus node, ring depth, filters, sort, cursor, selected row, navigation history) to the CLI invocation or E's pane and specifies restore semantics: "a restored session with a missing or moved path shows an explicit unresolved entry rather than a guess." Split handling is present (`gD` follow-in-split, `open in split` named action in §3.7); §5.3 exercises session restore "after restart with one target note moved." Source/preview and tabs are D/E-owned and correctly out of scope per §3.1's ownership sentence. | — |
| 2D Text correctness | 3 | §3.2 Unlinked mentions 3 requires "grapheme-cluster word boundaries on both sides" and a three-grapheme minimum; §4.5 pins `unicode-segmentation` for grapheme boundaries and `unicode-width` "for terminal layout only". §3.7: "Display width affects presentation only; path identity stays exact bytes, and control characters, bidi overrides, and escape sequences in paths, titles, aliases, snippets, and candidate lists are escaped as `\u{…}` before display." §3.2 heading normalization is specified precisely (markup stripped, ATX closer removed, whitespace trimmed and collapsed, *then* byte-exact). §5.1 `mention_boundaries_and_exclusions` covers RTL and combining sequences; `terminal_text_is_escaped` covers ESC/bidi/newline in every user-visible field. | — |
| 2E Degraded experience | 3 | §3.2 "Degraded and unavailable" mandates the literal state word first, then what remains possible ("direct file reads, exact-path navigation, live text scan") and what does not ("relationships, tags, properties, tasks, graph"), then one recovery command; "No view renders rows without its freshness banner, and no banner is color-only." §3.2 Search 8 defines a labeled bounded live fallback for text/title/path with `requires_index` refusal "naming each unavailable capability", and forbids emulating a relationship from stale rows. §4.6 pins the invariants that may never be feature-gated: "Freshness semantics, fail-closed defaults, ambiguity behavior, and the textual-equivalence invariant." §5.2 "Degraded fallback" tests it. | — |

**Lens average:** 2.80
**Lens pass:** Yes — avg ≥ 2.0, one 1-or-below count of zero, no 0s

---

## Lens 3: Knowledge Retrieval and Structure (weight: 20%) — ANCHOR LENS

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 3A Determinism | 3 | §3.2 step 6 states the invariant outright: "Resolution is a pure function of (indexed path set, target string, linking note path, resolver version); the same inputs always produce the same output, and a rebuilt-from-scratch index reproduces it exactly." The ladder table (Tiers 1, 2a, 2b, 3) is declared "ordered and total; sub-steps inside a tier are also ordered." Result ordering is fully pinned in §3.2 Search 5 — "ranked search by integer rank descending, then vault-relative path in raw byte order, then match byte offset" — with the nondeterminism sources named and forbidden: "Locale collation, float score, and filesystem enumeration order never determine output." Graph traversal is BFS with "node ordering within each ring by raw byte path order" (§3.2 Graph 3) and components "labeled by their lexicographically smallest member path" (Graph 4). Tested by §5.1 `graph_traversal_is_deterministic` (100 runs, byte-identical `--text`) and §5.2 rebuild equivalence, which asserts identical resolutions, candidate lists, ordering, graph output, and counts after a full database deletion. Residual: the derivation of the "integer rank" is not defined here, but §7.1 places FTS/ranking storage in B and B §7.2 owns "deterministic ranking/pagination", so the delegation is consistent rather than missing. | — |
| 3B Ambiguity | 3 | The strongest section in the spec. §3.2 step 4: "A tier that yields **more than one** candidate ends the ladder as `ambiguous` — it never falls through to a later tier and never applies a tie-break." §4.2 encodes this in the type system: `Resolution::Ambiguous { tier, candidates }` carries no `target` field at all, so a downstream consumer cannot read a chosen target. Heading fragments: two equal normalized headings in scope produce `ambiguous_heading` with ordinals and line numbers and "the tool never takes the first one." Block fragments: duplicate IDs produce `ambiguous_block`, and "**G never mints a block ID to satisfy a reference**". §3.2 Backlinks 3: an ambiguous link "contributes **no** resolved edge to any candidate", is excluded from degree counts and the default graph, and under `--include-ambiguous` "the state travels with the edge everywhere, so no downstream consumer can lose it." Never mutates: §3.2 "Nothing about resolution mutates a file. Not the linking note, not the target, not frontmatter, not a block ID. There is no 'fix links' side effect anywhere in G." §3.4 closes the non-interactive hole: `--no-input` "forbids every prompt and chooser, including the ambiguity chooser; unresolved ambiguity then returns a typed error listing candidates and the exact-path flag that would settle it." Tested by §5.1 `tier_three_collision_is_ambiguous_not_chosen` (asserts **no** `target`), `ambiguity_never_mutates_source` (digest snapshot of every file before/after), `resolver_never_mints_block_ids`, and `duplicate_heading_and_block_ids_are_ambiguous`. The case/NFC divergence from Obsidian is the correct call and is justified in-line: "silently matching a different byte string is exactly the 'silently chooses a target' failure." Tier ordering is a *documented* precedence, not a tie-break, and §3.2 backs it with `links doctor --check shadowing` so concealment "is auditable rather than invisible." | — |
| 3C Query depth | 2 | Coverage against the criterion's nine required domains is complete and concrete — §4.3's domain table gives fields, operator sets, and a worked example for **Text** (`text`, `body`, bare terms, quoted phrases, `term*`), **Titles** (`title`, `alias`, `name`, with bounded `~n` fuzzy, n ≤ 3), **Regex** (RE2-style `=~ /pat/flags` scoped to `text`/`title`/`path`/`tag`/`task.text`/`property.KEY`), **Tags** (`tag` with descendants, `tag.exact`, `tag.count`), **Properties** (`property.KEY`, `.exists`, `.type`, list membership via `in`, full comparison set), **Paths** (`path`, `path.under`, `path.name`, `path.ext`, `path.depth`), **Relationships** (`link.to/from`, `embed.to/from`, `link.unresolved/ambiguous/external`, `link.rule`, `mention.of`, `graph.distance(PATH)`, `orphan`, `leaf`), **Tasks** (`task.state/marker/text/due/scheduled/done/count`), and **Dates** (`created`, `modified` "filesystem-observed and labeled as such", date-typed properties, ISO-8601 civil dates, RFC 3339 instants, `today`, `now`, `today-7d` offsets) — plus a tenth Structure domain, precedence rules, non-coercing type rules, and closed operator/field pairs where "an unlisted pair is `unsupported_operator`, never a fallback to text." **Deduction:** the grammar block §4.3 declares itself "(normative)" but cannot derive two constructs its own table advertises. `field := ident ("." ident)*` and `predicate := field op value` admit no call form, so `graph.distance("index.md") <= 2` is unparseable; and `section:"H"` is described as scoping "the enclosed predicate to that section" while the grammar renders it an ordinary predicate, leaving its binding genuinely undefined — in the example `section:"Decisions" AND text:rollback` it is unspecified whether the scope covers the adjacent conjunct, the enclosing `and_expr`, or the whole `clause_list`. That is a semantic gap, not cosmetic: it changes result sets. Terminals (`list`, `word`, `date`, `duration`, `flags`, `direction`) are also left undefined. | In §4.3, either (a) add grammar productions for a scoped group — e.g. `primary := ... \| scope_op "(" or_expr ")"` — and for function-form predicates, and state the binding scope of `section:` precisely; or (b) drop `section:` and `graph.distance(...)` from the domain table until the grammar admits them. Define the `list`, `date`, `duration`, and `flags` terminals. |
| 3D Derived authority | 3 | §4.4 is explicit: "The graph, every view, and every materialized snapshot are derived views with no authority of their own (criterion 3D)." Saved queries live as ordinary files in two editor-visible forms (§3.2 Saved queries 1: an `mg-vault:` frontmatter mapping, or a fenced `mg-query` block), and the discovery row is disposable — "the index row is a disposable pointer, and deleting the whole database loses nothing but discovery speed. ... Any text editor can create, edit, rename, or delete a saved query with no tool involvement." §3.2 Saved queries 3: "A saved query stores **no results** by default; it is a question, not a cache, so there is nothing to invalidate." Snapshots are opt-in, marker-bounded, and never silently refreshed, with a four-way invalidation rule (query hash, generation, resolver/parser version, `refresh_after` age). §4.1 keeps the index crate mutation-free and routes every G-adjacent write through core's transaction API. §5.2 "Index-authority inversion" tampers a stored resolution row directly in SQLite and asserts it is reported `degraded` and never presented as current; §5.2 "Saved queries survive index loss" asserts rediscovery after database deletion with byte-identical notes. §7.5 non-goals bar making "resolutions, the graph, saved-query snapshots, or any index row authoritative over files." | — |
| 3E Scale | 3 | §4.7 is concrete numbers throughout, on B's stated 100,000-note / 1,000,000-block / ~2,000,000-projection corpus, and is honestly framed ("Numbers are acceptance budgets, not measured claims"). Editing isolation is first and explicit: "**Editing is never blocked.** All G work is read-only and off the write path. During a full rebuild or a whole-vault `links doctor`, p99 added latency to direct `note read`/`note write` stays within B's ≤ 20 ms allowance, and no save waits on resolution, mention scanning, or graph work." Resolution during rebuild ≤ 90 s of the ≤ 10 min budget at ≤ 200 MiB peak; incremental reprojection p95 ≤ 300 ms / p99 ≤ 1 s; warm first page p95 ≤ 100 ms / p99 ≤ 250 ms, `backlinks` p95 ≤ 30 ms, 2-hop ≤ 2,000 nodes p95 ≤ 150 ms, whole-graph `--stats`/`--clusters` p95 ≤ 3 s; cold 750 ms / 250 ms / 1 s; mentions p95 ≤ 1.5 s; cancellation p95 ≤ 100 ms of signal; typeahead debounce 120 ms; memory ≤ 64 MiB steady-state addition with graph caps of 5,000 nodes / 20,000 edges and ≤ 128 MiB streaming peak; storage ≤ 0.4× existing projection size; network zero; startup unaffected. The scaling *design* is stated, not just the budget: a create/rename that changes basename uniqueness "re-evaluates only the links registered against that basename via a reverse index — cost is O(affected links), never O(vault)." §5.2 "Scale" makes every §4.7 budget an acceptance gate with p50/p95/p99, peak RSS, DB growth, cancellation latency, and concurrent direct-write latency during rebuild. | — |

**Lens average:** 2.80
**Lens pass:** Yes — avg ≥ 2.0, zero 1s, no 0s
**Auto-fail triggered:** No — see the individual walk below.

### Auto-fail walk (rules aimed at this branch)

| Rule | Status | Basis |
|---|---|---|
| **Index state overriding source** | PASS | §3.2 Backlinks 4: opening a result "re-reads the source file through A's confined read and compares that fingerprint; a mismatch says `source changed since indexing` and offers refresh — the index is never the source of the bytes shown in the editor." §3.3 Data sources: "No view reads SQLite directly, and no view treats an index row as note content." §4.1: no source-mutation API in the index crate. §5.2 "Index-authority inversion" tampers a row and requires `degraded`. The cited precedent is real: `persisted_derived_field_tampering_cannot_be_certified_as_current` exists at crates/mg-vault-index/tests/persistent_store.rs:586 and asserts `Freshness::Degraded` plus `StoreError::NotCurrent`. |
| **Stale index presented as current** | PASS | §4.2 `ResultProvenance` is "stamped on every result set, page, graph render, and pane" and carries freshness, generation, observation time, resolver/parser/schema versions, query hash, `complete`, and `derived_from`. §3.2 Search 3 makes the default fail-closed: anything but `current` returns "zero rows, exit 6, and a sentence naming the state, the indexed generation, the last complete observation time, and the refresh command." `--allow-stale` labels "the envelope, the banner, every page footer, and every human record block", and §3.3 explains why the per-record label exists: "so a copied fragment cannot lose the qualifier." Empty results are qualified too — "`No matches in indexed generation 41.` and never imply that no source match exists when freshness is not current." Degradation is visible, never silent: §3.2 Degraded section and the labeled `mode: direct-scan` / `freshness: unindexed-live` fallback. §5.1 `provenance_present_on_every_response` is a property test over all response types; §5.2 "Freshness fail-closed" asserts exit 6 across all six commands. Exit 6 matches C §7.4 and B §3.6, so the code is real, not invented. |
| **Graph or Canvas information lacking a textual equivalent** | PASS | §3.1 marks `graph --text` "New; **canonical** representation" and the pane's "focus lives in the list". §3.7 states the invariant as a hard rule: "the set of nodes, edges, edge kinds, edge states, candidate lists, counts, components, and filter effects rendered by the pane equals the set emitted by `graph --text` / `--json` for the same request. The drawing may add no datum of its own and may omit none." §3.3 gives the full textual format with a worked example carrying every attribute the drawing encodes plus the ones it cannot (rule, byte range, candidates), and specifies total suppression of the drawing "Below 60 columns, or when the terminal reports no Unicode box-drawing support, or under `--no-graphics` ... no information is lost, which is precisely why the list is canonical." §3.2 Graph 6 prevents divergence at the filter level: filters "apply identically to the drawing, the list, and every export, so no filter can make the picture and the text disagree." §5.2 names it an auto-fail gate and tests set equality across 20 fixture focuses plus 40-column rendering with the drawing suppressed. This is a complete navigable textual mode, not a decorative summary. |
| Source-content loss | PASS | G writes only opt-in snapshot bytes between explicit markers (§3.2 Saved queries 4); §7.5 bars editing, repairing, renaming, or reformatting any note. |
| Partial multi-file mutation | PASS | §6.4: "G performs no multi-file mutation at all." Mention-linking is per-occurrence and routed through C/H (§3.2 Unlinked mentions 5). |
| Silent conflict winner | PASS | §3.6 `source_changed`; §6.4 "fingerprint mismatch reports both states"; §5.2 asserts a failed refresh leaves the note byte-identical with the prior snapshot retained. |
| Unknown syntax loss | PASS | §4.1 forbids a second dialect and consumes F/B's token-preserving parser; §3.2 step 1 keeps undefined reference labels as literal text; §5.2 Obsidian coexistence asserts unknown syntax passes through untouched. |
| Unconfirmed overwrite/import | PASS | `--materialize` is opt-in with an explicit previewable refresh and a fingerprint precondition (§3.2 Saved queries 4). |
| Unsafe traversal or symlink escape | PASS | §3.2 step 3 classifies absolute paths, `file:` URIs, and vault-escaping normalized forms as `unsafe_target`, "recorded, never resolved ... and never followed"; tier 2b permits `..` "only while the result stays inside the vault root"; §5.1 `note_relative_cannot_escape_vault`. Foundation support is real: crates/mg-vault-core/src/index.rs opens through `openat2` with `ResolveFlags::BENEATH \| ResolveFlags::NO_SYMLINKS`. |
| Capability / data-exfiltration bypass | PASS | §4.3: "no ambient capability for plugins or AI — a plugin or AI adapter that wants query access needs N/O's explicit, scoped, previewed, attributable capability grant, and may propose but never execute a mutation." §7.5 bars direct SQLite access from CLI, TUI, Quickshell, plugins, AI, and `mg-calr`. |
| Active raw HTML/script by default | N/A | G renders no HTML; rendering is F's domain. |
| Non-atomic save claiming success | PASS | The sole write goes through A's atomic transaction (§3.2 Saved queries 4); §3.2 Search 7 forbids treating a partial page as final. |
| Recovery overwriting newer source | PASS | §3.2 Backlinks 4 and §3.6 `source_changed` re-read from disk; index bytes "are never substituted". |

---

## Lens 4: Security, Reliability, and Extensibility (weight: 15%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 4A Confinement | 3 | §3.2 step 3 rejects and records `unsafe_target` before any filesystem touch, and externalizes scheme-matching destinations without resolving them. Tier 2b bounds `..` to the vault root. §3.2 Graph 7: "Navigation from any graph or result row uses the exact path, revalidated directly against the filesystem before the editor opens it." §3.6 rows `unsafe_link_target` and `path_not_found`/`unsafe_path` (which "Reuse C's foundation error contract verbatim"). §4.4 keeps all volatile state outside every vault. §5.1 `note_relative_cannot_escape_vault` covers `[[../../etc/passwd]]`, `](/etc/passwd)`, a `file:` URI, and a symlinked target. Control-directory protection is inherited: the walker in crates/mg-vault-core/src/index.rs skips `.obsidian` and `.mg-vault`, so a saved query cannot be discovered inside either. | — |
| 4B Concurrency | 3 | §4.2 stamps `source_fingerprint` on every `Edge` and `MaterializedSnapshot` carries `query_hash`, `index_generation`, `resolver_version`, `parser_version`, and `run_at`. §3.2 Backlinks 4 requires a fingerprint comparison at every result-open. §3.2 Search 6 binds cursors to "vault, normalized query, sort, generation, and last key", returning `cursor_expired` "rather than mixing pages". §4.6 puts resolver version into protocol negotiation so a mismatch yields `protocol_incompatible` "rather than mixed semantics"; §4.2 makes a resolver-version mismatch mark a generation unusable. §5.2 "Materialized snapshot transaction" is the concurrency test. The losing side of a snapshot conflict is derived and regenerable, so the "preserve both versions" obligation is satisfied by preserving the on-disk note byte-identically. | — |
| 4C Least privilege | 2 | §4.3 states all four required properties — deny-by-default ("no ambient capability for plugins or AI"), scoped, previewed, attributable — and §7.5 bars direct SQLite access from every non-first-party consumer. But G defines no scope vocabulary for its own surface, and this surface is precisely where an exfiltration-shaped grant would be issued: nothing says whether a grant can be limited to a path prefix, to metadata-only results without snippets, to specific query domains (e.g. relationships but not `text:`), or to a single vault. Everything is deferred wholesale to N/O, which §7.4 correctly marks "Not required for G itself." | In §4.3, enumerate the grant dimensions G's IPC must be able to honor — at minimum vault, path prefix, snippet/no-snippet, and permitted predicate domains — so N/O has a concrete surface to scope against. |
| 4D Recovery | 2 | Proposed mutations are handled well: §3.2 Unlinked mentions 5 requires "an explicit per-occurrence C/H mutation with dry-run preview, exact byte range, expected fingerprint, and atomic commit. Declining changes nothing." Nothing claims false success: §3.2 Search 7 and §3.6 `cancelled`/`deadline_exceeded` mark `complete: false` with no further pagination. Snapshot refresh is "an explicit, previewable command" and fails closed. Gap: the criterion requires destructive work to be "trash/history backed", and a successful snapshot refresh — which discards the previous snapshot's bytes — has no stated trash entry, history record, or undo, even though A already provides `trash_note`/`restore_note`. Same root gap as 2B. | Same fix as 2B: state in §3.2 Saved queries 4 whether a successful refresh is trash- or history-backed via A, or justify why derived snapshot bytes need no recovery path. |
| 4E Contracts | 2 | Strong core: §4.3 returns C's `version: 1` envelope (confirmed real — crates/mg-vault-cli/src/main.rs:612 emits `version: 1`); §4.2 `ResultProvenance` carries `resolver_version`, `parser_version`, `schema_version`, `index_generation`, `query_hash`, `complete`, and `derived_from`; §5.3 goldens assert "envelope version, provenance fields, deterministic ordering, and stable error codes without depending on volatile timestamps"; §6.3 requires panes to report `provided_by: "tui"` rather than `available: true` until E ships, matching C §3.1's stated convention. Page-size defaults (50 default / 500 max) match B §4.3 exactly. **Two unreconciled contradictions with B:** (1) *Freshness vocabulary.* §3.2/§3.6 enumerate `stale \| rebuilding \| degraded \| unknown \| unavailable` and §4.2 says the type "extends today's Empty\|Current\|Stale\|Degraded", but the enum G actually consumes over IPC is B §4.2's `Current \| CatchingUp { dirty_paths } \| Stale { reason, since } \| Rebuilding { scanned, discovered } \| Blocked { reason } \| Unavailable { reason }`. G never mentions `CatchingUp` or `Blocked`; B defines no `degraded` or `unknown` Freshness variant (it has `index_corrupt`/`vault_blocked` as *error codes*). Behavior is still safe because G's rule is "anything other than `current`" fails closed, but the state list users and goldens are told to expect is wrong against the contract. (2) *Operation naming.* §4.3 adds `Query(vault, QueryAst\|text, options, page)` and `ExplainQuery(vault, text)` to a protocol where B §4.3 already requires `Search(vault, QueryAst\|query_text, SearchOptions)` and `ExplainQuery(vault, query_text)` — a duplicate `ExplainQuery` and an unexplained `Query`/`Search` split in one versioned op set. | In §3.2/§3.6/§4.2, replace G's ad-hoc freshness word list with B §4.2's six variants and give the display mapping for `CatchingUp` and `Blocked`; drop or justify `unknown`. In §4.3, either fold `Query` into B's `Search` and delete the duplicate `ExplainQuery`, or state explicitly why both exist and how a client chooses. |
| 4F Privacy | 2 | The prose commitments are comprehensive and identify the non-obvious leak vector. §6.1 lists the sensitive surface (note text, titles, aliases, paths, tags, properties, tasks, snippets, mention context, query literals) and the protections: "secret-marked properties and excluded paths never enter query results, snippets, graph labels, mention context, saved-query snapshots, or logs; query literals are not logged by default and `--no-history` disables the local history file entirely; ... materialized snapshots are opt-in and inherit the same exclusions, so publishing a saved-query note cannot leak an excluded property." §3.6 bars bodies, secret values, unflagged query literals, environment values, and outside-vault absolute paths from error details. §4.4 keeps query drafts and history under XDG state, "excluded from indexes and exports", with one documented clearing command. Gap: none of this is test-gated. §5.1's only adjacent row is `terminal_text_is_escaped`, and §5.2 has no privacy or exclusion test — unusual in a spec where nearly every other invariant carries a named test. | Add a §5.2 integration test asserting that a secret-marked property and an excluded path are absent from query results, snippets, graph labels, mention context, `--explain` output, a materialized snapshot, error details, and the history file. |

**Lens average:** 2.33
**Lens pass:** Yes — avg ≥ 2.0, zero 1s, no 0s

---

## Lens 5: Performance and Accessibility (weight: 10%)

| Criterion | Score (0–3) | Evidence from spec | Remediation needed |
|---|---|---|---|
| 5A Offline/local-first | 3 | §4.4: "**Local-only, never server-synced.** There is no remote component in this feature at all." §4.5: "No network dependency of any kind." §4.7: "**Network:** zero bytes. Every workflow in this feature is fully offline." §7.5 excludes remote/cloud search and network extractors. §6.1 requires only synthetic corpora for acceptance. | — |
| 5B Responsiveness | 3 | §4.7 covers all four required budget classes with numbers, not adjectives: latency (warm/cold/incremental/mention/graph, at p95 and p99), memory (≤ 64 MiB steady addition, 5,000-node / 20,000-edge caps, ≤ 128 MiB streaming peak), cancellation (`Ctrl-C`/`Esc`/`--deadline-ms` returning within p95 ≤ 100 ms of signal), and startup ("no G structure is loaded eagerly ... so B's ≤ 150 ms IPC-health budget is unaffected"). Storage (≤ 0.4× projection size) and the TUI typeahead debounce (120 ms, cancel-in-flight) are specified too. Every figure is labeled an acceptance budget rather than a measured claim. | — |
| 5C Accessible equivalents | 3 | Beyond the graph invariant scored under the auto-fail walk: §3.7 gives spoken-form row announcements with role and state ("row 3 of 12, inbound link, journal/2026-08-19.md, resolved by unique basename, line 4 column 3"; "ambiguous, 2 candidates, not counted as a backlink"; "graph pane, focus projects/alpha.md, depth 2, 14 nodes, 27 edges"), replaces gestures with seven named custom actions each having a keybinding and a CLI equivalent, and spells out `resolved`, `unresolved`, `ambiguous`, `external`, `stale`, and `partial` as words with "Glyphs and color are redundant decoration." §3.7 fixes focus order to §3.3's reading order and requires focus to enter the canonical list, never the drawing: "Tab order never jumps into a decorative region." §3.3 specifies every empty state's literal copy. §5.4 includes a screen-reader pass over paging, backlink sections, an ambiguity report, and the graph list. | — |
| 5D Terminal resilience | 3 | §3.4: layouts verified at 40, 60, 80, and 120 columns; tables become one-record-per-block below 60; "Paths, generations, candidate lists, error codes, and recovery commands are never truncated; they wrap with indentation." `NO_COLOR`, `--no-color`, non-TTY stdout, and `--json` all disable styling, "and JSON never contains escape sequences." §3.5 handles reduced motion by removing dynamic output entirely under `--json`, `--jsonl`, non-TTY, `--quiet`, `NO_COLOR`, or a reduced-motion environment setting — "progress becomes a queryable static snapshot instead, so a screen reader is not re-interrupted" — and caps interactive progress at four updates per second on stderr with stdout reserved for results. §3.3 gives a full ASCII fallback glyph set (`->`, `<-`, `=>`, `~?`, `..`). §5.4 checks five theme/ANSI configurations plus RTL and combining-character paths. Minor: the reduced-motion environment variable is referenced but not named. | Name the reduced-motion signal in §3.5 (e.g. a specific env var) so it is testable. |
| 5E Automation | 3 | §3.4's shared flag set includes `--json`, `--jsonl`, `--limit`, `--after`, `--sort`, `--deadline-ms`, `--no-input`, `--no-color`, and `--ascii`, with `NO_COLOR` honored. §2's automation story states the contract: "stable versioned JSON, deterministic ordering, generation-bound pagination, and typed errors for every retrieval command, so that scripts do not silently broaden a match or mix two generations." stdin handling is explicit and safe: "Query text is an argument, or explicit `--query-file FILE` / `--stdin`; unrelated piped stdin is never assumed to be a query." `--no-input` "forbids every prompt and chooser, including the ambiguity chooser", returning a typed error with candidates and the settling flag. §5.3 runs E2E for every command in human, `--json`, and `--jsonl` modes with goldens over 13 named scenarios. | — |

**Lens average:** 3.00
**Lens pass:** Yes

---

## Feasibility Check

Graded against `dfe33cf`. Sources read: `crates/mg-vault-index/src/lib.rs` (1401 lines),
`crates/mg-vault-index/tests/persistent_store.rs` (948 lines), `crates/mg-vault-core/src/index.rs`,
`vault.rs`, `interop.rs`, `crates/mg-vault-cli/src/main.rs`, `Cargo.toml`, `Cargo.lock`,
`libsqlite3-sys-0.38.2/build.rs`, and `docs/specs/b-index-service-search.md`.

| Check | Status | Notes |
|---|---|---|
| Types/models exist or are clearly specified | ✓ | §4.2's types are new but concrete and hang off types that exist: `SourceFingerprint` and `Vault` (mg-vault-core), `MarkdownIndex`/`IndexedNote` (core/index.rs), `Freshness`/`IndexMetadata`/`StoredNote` (index/lib.rs). `NotePath`, `SortSpec`, `ColumnSpec`, `ByteDifference`, and `UnsafeTargetReason` are used without declaration — normal at spec altitude, noted for completeness. |
| API/interface changes are feasible with current architecture | ✓ | The CLI is a clap `Subcommand` enum (main.rs:38) that extends trivially; the named helpers `run_search` (226), `direct_search` (233), and `search_output` (330) exist exactly as §4.1 describes and are movable. The envelope is real: `version: 1` at main.rs:612–615. Crucially, §3.2 Saved queries 4's marker-bounded atomic write is already implemented — `Vault::edit_note_span` (vault.rs:181) validates the path, checks an expected `SourceFingerprint`, rejects non-char-boundary and out-of-bounds spans, and calls `replace_atomic`. |
| Views/screens fit current navigation pattern | ✓ | CLI-first with TUI panes as consumers matches today's clap subcommand tree; every pane has a CLI equivalent (§3.1), so nothing depends on E existing. |
| Dependencies are available and version-compatible | ✓ | §4.5's rusqlite claim is **verified exactly**: `Cargo.toml` pins `rusqlite = { version = "0.40.2", features = ["bundled"] }`, resolving to `libsqlite3-sys 0.38.2` (Cargo.lock:302–304), whose `build.rs` passes `-DSQLITE_ENABLE_FTS5` unconditionally at line 159 of the bundled `cfg.file(...)` chain. So FTS5 **is** reachable from SQL today, and §4.5/§7.1's nuance — that the rusqlite `fts5` cargo feature is the custom-tokenizer API and is separately required for a Unicode-aware tokenizer — is correct. No FTS table exists yet (`SCHEMA` at lib.rs:21 defines only `metadata`, `notes`, `diagnostics`), matching the "Gated" classification. `regex`, `aho-corasick`, `unicode-segmentation`, and `unicode-width` are all edition-2024 compatible and need no `unsafe` (the workspace sets `unsafe_code = "forbid"`). |
| Platform/renderer requirements are realistic | ✓ (caveat) | §4.6's per-platform case-insensitivity fixtures presume a macOS build, but `read_source_bytes` is `#[cfg(target_os = "linux")]` only and the fallback returns "safe source opening is unsupported on this platform" (core/index.rs). This is inherited from A/B, not introduced by G, and B §4.6 carries the same macOS-portability commitment. |
| Test strategy is executable with current infrastructure | ✓ (caveat) | §5.1's resolution/query/graph unit tests are executable immediately. §5.2's precedents are real: `persistent_store.rs` already contains `persisted_derived_field_tampering_cannot_be_certified_as_current` (line 586) and `verified_search_rejects_database_tampering_after_prior_freshness_check` (line 839), so the "Index-authority inversion" test is a direct extension of shipped work, and the spec cites the existing test by its true name. The 100k/1M corpus generator, the pane-vs-text conformance harness, and the benchmark harness do not exist and are correctly listed as new work in §7.2 and gated in §7.4. |
| Performance budget is realistic for target hardware | ✓ (caveat) | Every borrowed figure matches B §4.7 exactly: ≤ 20 ms added edit latency, ≤ 10 min rebuild, 500 ms / 2 s freshness, ≤ 150 ms IPC health, ≤ 2.5× storage, 100 ms / 250 ms / 750 ms search. One unreconciled item: G's "≤ 64 MiB steady-state above B's ≤ 256 MiB target" pushes the same service past B's stated steady-state target to ~320 MiB (still inside B's 512 MiB hard ceiling). The spec acknowledges the addition but does not reconcile the two budget lines. |
| No undeclared dependency on unbuilt features | ✓ | §7.4 declares A (implemented), B (B1–B5 gate everything past G1), F, C, E, I, J, H, and N/O, each with the specific artifact needed. §7.3 sequences G1–G7 with G1–G5 "individually reviewable and individually shippable behind the CLI". §6.3 forbids advertising E-gated panes as `available: true`. |

**Feasibility verdict:** Feasible with caveats

**Caveats:**
1. **§7.1 line counts are stale** (low severity per active concurrent development): the spec says
   `mg-vault-index/src/lib.rs` is "one 960-line `lib.rs`" (§4.1 and §7.1) — it is **1401** lines at
   `dfe33cf` — and that `tests/persistent_store.rs` is "712 lines" — it is **948**. Every *behavioral*
   claim in §7.1 checked out (see audit below); only the counts drifted.
2. **§4.2's "`links` gains ..." phrasing** describes a table that does not exist at HEAD; there is no
   `links` table today, only `notes.outgoing_links_json`. The sentence is true relative to B's planned
   schema and §7.1's Absent list disambiguates it, but §4.2 read alone implies more exists than does.
3. macOS support is aspirational at the foundation layer (see the platform row above).
4. The memory budget tension with B §4.7 (see the performance row above).

### §7.1 accuracy audit (item by item, against `dfe33cf`)

**"Implemented" — every claim verified true:** lexical-order `.md` walk (`entries.sort_by_key(file_name)`
plus `paths.sort()`); skips `.obsidian` and `.mg-vault`; `openat2` with `ResolveFlags::BENEATH |
ResolveFlags::NO_SYMLINKS` on Linux; double-read concurrent-change detection ("source changed during
indexing; retry rebuild"); title precedence frontmatter `title` → first `# ` heading → file stem;
`IndexedNote { path, title, text, fingerprint, outgoing_links }` field-for-field; `MarkdownIndex::search`
is `to_lowercase()` substring over path/title/text/links sorted by path; SQLite `metadata`/`notes`/
`diagnostics` at `SCHEMA_VERSION = 2` and `PARSER_VERSION = "markdown-index-v1"`; atomic generation
publication; freshness verified by two matching complete observations against the published manifest
(`verify_freshness_with_guards` in fact takes three rebuilds — projection, confirmation, certified);
`empty | current | stale | degraded` (matches the SQL `CHECK` constraint verbatim); `StoreError::NotCurrent`
refusal to search a non-current generation; symlinked database path and parent rejection
(`reject_sqlite_links`, `reject_sqlite_links_at`, `sqlite_family_names`); transactional legacy-v1 reset
(`reset_legacy_cache`); CLI `index rebuild`, `index status`, `search QUERY` with labeled direct-scan
fallback plus `--json`, `--no-input`, `--no-color`, `NO_COLOR`.

**"Prototyped" — verified true:** `wikilinks()` splits the alias at `|`, trims, and stores raw targets
in `outgoing_links_json`, with no positions, kinds, Markdown links, embeds, fragments, or resolution.

**"Gated" — verified true:** FTS5 is compiled in (build.rs:159, unconditional) but no FTS table,
tokenizer, or rusqlite `fts5` feature is configured.

**"Planned" — verified true against B:** B §4.2 defines `LinkProjection { ..., resolved_target,
resolution, resolver_version }` exactly as G says it will fill in; B §4.3 defines the IPC ops, cursors,
and cancellation; B §7.4 states "**G1–G7:** final link resolution, unlinked mentions, saved-query
persistence, user-facing search integration, and graph semantics consume this foundation; B stores/
query-projects relationships but does not silently resolve ambiguity"; B §7.5 non-goals list "Final
backlink chooser/UI, unlinked-mention semantics, graph rendering, saved-query files". G's characterization
of B's deferral is accurate.

**"Absent" — verified true:** nothing in the tree implements links, backlinks, mentions, query grammar,
graph, pagination, or cancellation.

### G↔B contradictions found

1. **Freshness enum mismatch** (scored under 4E). G: `stale | rebuilding | degraded | unknown |
   unavailable`; B §4.2: `Current | CatchingUp | Stale | Rebuilding | Blocked | Unavailable`. Behavior
   is safe (G fails closed on anything ≠ `current`), but two B states are unhandled by name and two
   G states do not exist in B.
2. **IPC operation duplication** (scored under 4E). G adds `Query` and `ExplainQuery`; B already
   requires `Search` and `ExplainQuery`.
3. **Memory budget** (noted in Feasibility). G's +64 MiB exceeds B's ≤ 256 MiB steady-state target
   while staying inside B's 512 MiB ceiling; unreconciled.

No contradiction rises to an internal-correctness failure or an auto-fail.

---

## Composite Score

| Lens | Average | Weight | Weighted |
|---|---|---|---|
| Data Ownership and Format Compatibility | 2.80 | 30% | 0.840 |
| Editor and TUI Excellence | 2.80 | 25% | 0.700 |
| Knowledge Retrieval and Structure | 2.80 | 20% | 0.560 |
| Security, Reliability, and Extensibility | 2.33 | 15% | 0.350 |
| Performance and Accessibility | 3.00 | 10% | 0.300 |
| **Composite** | | | **2.75** |

**Pass conditions (from criteria.md):**
- [x] Composite ≥ 2.0 — 2.75
- [x] All lens averages ≥ 2.0 — minimum 2.33 (Lens 4)
- [x] No criterion scores 0
- [x] No more than two criteria at 1 per lens — zero 1s across all 26 criteria
- [x] All auto-fail rules pass — walked individually above
- [x] Feasibility ≠ Infeasible — Feasible with caveats

**All conditions met:** Yes → **PASS**

---

## Remediation Brief

Not required (PASS). Recorded for iteration 2.

### Priority 2 — Should fix for quality

1. **§4.3 grammar/table mismatch (3C).** The "(normative)" grammar cannot derive
   `graph.distance("index.md") <= 2` (`field := ident ("." ident)*` admits no call form) or express
   `section:"H"` scoping (it parses as an ordinary predicate). Add a scoped-group production and a
   function-predicate production, and state the binding scope of `section:` precisely — whether it
   scopes the adjacent conjunct, the enclosing `and_expr`, or the whole `clause_list` — or remove both
   constructs from the domain table. Also define the `list`, `date`, `duration`, `flags`, and
   `direction` terminals.
2. **§3.2 / §3.6 / §4.2 freshness vocabulary (4E).** Replace G's `stale | rebuilding | degraded |
   unknown | unavailable` list with B §4.2's six `Freshness` variants, give display copy and recovery
   text for `CatchingUp { dirty_paths }` and `Blocked { reason }`, and either map or drop `unknown`.
   The fail-closed default already covers these states behaviorally; the enumerated list and the
   §5.3 goldens must match the contract.
3. **§4.3 IPC operation set (4E).** Fold `Query` into B's `Search`, delete the duplicate
   `ExplainQuery`, or state explicitly why both exist in one versioned op set and how a client chooses.
4. **§5.2 privacy test (4F).** §6.1 makes strong, specific exclusion claims that no test gates. Add an
   integration test asserting a secret-marked property and an excluded path are absent from query
   results, snippets, graph labels, mention context, `--explain` output, a materialized snapshot,
   error details, and the history file.
5. **§3.2 Saved queries 4 / §4.4 snapshot recovery (2B, 4D).** State whether a *successful*
   `query refresh` is trash- or history-backed through A, or justify why overwriting derived snapshot
   bytes needs no recovery path. The failure path is already correct.
6. **§4.4 `.mg-vault` isolation (1D).** Commit positively to `.mg-vault` as the home for any G-owned
   per-vault setting (Q1's ladder policy, Q4's comparison timezone, mention defaults), and restate
   that G reads but never writes `.obsidian`.

### Priority 3 — Consider for excellence

7. **§4.1 / §7.1 line counts.** `mg-vault-index/src/lib.rs` is 1401 lines, not 960;
   `tests/persistent_store.rs` is 948 lines, not 712. Prefer describing the file's shape over its
   length so the spec does not re-stale on the next commit.
8. **§4.2 table phrasing.** "`links` gains `resolution_rule`, ..." reads as an edit to an existing
   table; no `links` table exists at HEAD. Say "B's planned `links` table gains ..." to match §7.1's
   own Planned/Absent split.
9. **§4.7 memory reconciliation.** Reconcile G's +64 MiB against B's ≤ 256 MiB steady-state target
   explicitly (raise B's target, or budget G inside it), rather than stating the overage and moving on.
10. **§4.3 rank derivation.** Name the ranking function or explicitly cite B as its owner, so the
    "integer rank descending" ordering rule in §3.2 Search 5 is traceable to something reproducible.
11. **§3.5 reduced motion.** Name the environment signal that triggers reduced-motion behavior so it
    is testable.
12. **§3.2 Backlinks 3 / §3.3 graph.** Under `--include-ambiguous`, state whether an ambiguous link
    contributes one edge per candidate or a single edge to the raw target. §3.3's example implies the
    latter (`edge out link "alpha" ambiguous candidates=2`) while §3.2 implies per-candidate counting;
    Q2 flags the default but not this shape question.
