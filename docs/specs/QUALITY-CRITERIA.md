# mg-vault — Binding Quality Criteria

**Generated:** 2026-08-23
**Basis:** accepted product plan and prior user interview
**Criteria version:** 1

## Scoring

Each criterion is graded 0–3: missing, inadequate, acceptable, excellent. Passing requires no zero, every lens average at least 2.0, no more than two scores of 1 per lens, and no auto-fail.

## Auto-fail rules

A spec automatically fails for source-content loss; partial multi-file mutation; index state overriding source; stale index presented as current; silent conflict winner; unknown syntax loss; unconfirmed overwrite/import; unsafe traversal or symlink escape; capability/data-exfiltration bypass; active raw HTML/script by default; non-atomic save claiming success; recovery overwriting newer source; or graph/Canvas information lacking a textual equivalent.

## Lens 1: Data Ownership and Format Compatibility (30%)

**Standards:** CommonMark, GFM, documented Obsidian behavior, YAML, JSON Canvas 1.0, versioned Bases fixtures.

- **1A Authority:** ordinary files remain sufficient, editable source of truth; indexes are disposable.
- **1B Preservation:** unknown syntax/properties survive outside explicitly edited spans.
- **1C Identity:** file paths remain public identity; no hidden UUID injection.
- **1D Coexistence:** `.obsidian` is preserved and portable app settings are isolated under `.mg-vault`.
- **1E Transactions:** collisions, ambiguity, conflicts, and partial writes fail closed and recoverably.

## Lens 2: Editor and TUI Excellence (25%)

**Benchmarks:** Neovim modal precision and composability; Obsidian integrated workflows.

- **2A Keyboard completeness:** every workflow is keyboard reachable with coherent documented grammar.
- **2B Editing durability:** atomic autosave, recovery, persistent undo, and external-edit merge preserve work.
- **2C Workspace:** panes, tabs, splits, source/preview, navigation, and session restore are coherent.
- **2D Text correctness:** Unicode/grapheme-safe cursor and structural operations.
- **2E Degraded experience:** unavailable features are explicit and source editing remains possible.

## Lens 3: Knowledge Retrieval and Structure (20%)

**Benchmarks:** Obsidian links/graph, Logseq relationships/queries, Notion typed views.

- **3A Determinism:** links, backlinks, search, graph, and views derive consistently from source.
- **3B Ambiguity:** link ambiguity never silently chooses or mutates.
- **3C Query depth:** text, titles, regex, tags, properties, paths, relationships, tasks, and dates are addressed.
- **3D Derived authority:** schemas, Bases, graphs, and formulas remain views over ordinary files.
- **3E Scale:** budgets address 100,000 notes and 1,000,000 blocks without blocking editing.

## Lens 4: Security, Reliability, and Extensibility (15%)

- **4A Confinement:** traversal, symlink escape, and protected-control-directory mutations fail closed.
- **4B Concurrency:** fingerprints/versions detect stale writes and preserve both concurrent versions.
- **4C Least privilege:** plugins and AI are deny-by-default, scoped, attributable, and previewed.
- **4D Recovery:** destructive work is previewable, atomic, trash/history backed, and never claims false success.
- **4E Contracts:** CLI/IPC/plugin JSON contracts are stable, versioned, and truthful.
- **4F Privacy:** secrets and private content are excluded from unintended logs, indexes, exports, and payloads.

## Lens 5: Performance and Accessibility (10%)

- **5A Offline/local-first:** core workflows require no network.
- **5B Responsiveness:** explicit latency, memory, cancellation, and startup budgets exist.
- **5C Accessible equivalents:** graph, Canvas, media, cards, and status have complete textual modes.
- **5D Terminal resilience:** narrow terminals, Unicode, color-independent states, and reduced motion are handled.
- **5E Automation:** human and stable JSON output, stdin/stdout, `--no-input`, `--no-color`, and `NO_COLOR` are supported.

## Scoring summary

| Lens | Criteria | Weight | Auto-fail focus |
|---|---:|---:|---|
| Data ownership and compatibility | 5 | 30% | source loss / authority inversion |
| Editor and TUI excellence | 5 | 25% | recovery overwrites newer source |
| Retrieval and structure | 5 | 20% | stale results / inaccessible-only views |
| Security, reliability, extensibility | 6 | 15% | traversal / capability bypass / partial writes |
| Performance and accessibility | 5 | 10% | inaccessible representations |
