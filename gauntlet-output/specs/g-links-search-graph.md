# Spec: Links, Search, and Graph

**Feature ID:** g-links-search-graph
**Parent feature:** root
**Spec author agent:** Hermes Agent (links/search/graph subagent)
**Date:** 2026-08-29
**Iteration:** 1

---

## 1. Purpose

### 1.1 One-sentence job

Turn the ordinary Markdown files in a vault into a navigable knowledge surface — deterministically resolved links and backlinks, unlinked mentions, a typed query language over text/titles/regex/tags/properties/paths/relationships/tasks/dates, saved queries that are themselves ordinary notes, and a graph whose terminal picture and textual listing are the same model — without any derived artifact ever becoming authority over the files.

### 1.2 Why it matters

Retrieval is where a knowledge system usually starts lying. Obsidian silently picks a target when two notes share a basename; many tools inject identifiers into files to make links "reliable"; most graph views are pictures that a screen-reader user cannot read at all; almost every one presents index rows as if they were the current state of disk. `mg-vault` is local-first and file-authoritative, so G must earn the same navigation value while refusing all four shortcuts: it reports ambiguity instead of guessing, never mutates a note to make a link resolvable, gives every graph datum a complete textual form, and stamps freshness provenance on every result set so a stale answer is visibly stale. It must also stay out of the writer's way: at 100,000 notes nothing in this feature may block a save.

### 1.3 Success signal

On a synthetic 100,000-note / 1,000,000-block / ~2,000,000-link corpus containing deliberate basename collisions, duplicate headings, duplicate block IDs, case-and-normalization near-misses, and Unicode/RTL paths: every link's resolution is reproducible from source alone across a full index deletion and rebuild (identical `resolution_rule`, target, and ambiguity candidate lists); no ambiguous link ever yields a chosen target or a source-byte change; every query, backlink pane, saved-query result, and graph render carries freshness provenance and returns zero rows by default when freshness is not `current`; `graph --text` emits a superset of every node, edge, kind, state, and count the TUI graph pane can display, proven by set equality in a conformance test; and warm query p95 stays within §4.7 budgets while concurrent direct note writes stay within C's unmodified save budget.

---

## 2. User Stories

> As a writer, I want `[[Meeting Notes]]` to resolve to exactly the note I meant by a rule I can predict, so that navigation is trustworthy rather than lucky.

> As a writer with two notes named `alpha.md` in different folders, I want the tool to tell me the link is ambiguous and list both candidates, so that it neither guesses a target nor edits my file to disambiguate.

> As a researcher, I want one query language covering text, titles, regex, tags, properties, paths, links, tasks, and dates, so that I can ask a precise question instead of grepping and filtering by hand.

> As a returning user, I want saved queries to be ordinary notes in my vault, so that they sync, diff, version, survive index deletion, and remain readable in Obsidian or a plain text editor.

> As a screen-reader user, I want the graph to be a complete, navigable list of nodes and typed edges with the same information the drawing carries, so that the picture is an aid and never the only channel.

> As an automation author, I want stable versioned JSON, deterministic ordering, generation-bound pagination, and typed errors for every retrieval command, so that scripts do not silently broaden a match or mix two generations.

> As a user whose index service is down or behind, I want retrieval to fail closed with a plain-language reason and an explicitly labeled live fallback, so that I never mistake a stale answer for the current vault.

> As a privacy-conscious user, I want query history, snippets, and secret-marked properties kept out of logs, graph labels, and materialized query snapshots, so that retrieval does not leak what indexing was careful about.

---

## 3. UX Specification

### 3.1 Screen / view inventory

This is a CLI and TUI product. No GUI screen, mouse requirement, sound, or haptic exists anywhere in this feature. Views are line-oriented CLI output plus, once E lands, terminal panes that render the identical contracts.

| View | Entry point | New / modified | Layout pattern |
|---|---|---|---|
| Search results | `mg-vault search QUERY …` | Modification of the existing substring `search` command | Freshness banner, ranked linear records, footer banner |
| Structured query results | `mg-vault query 'EXPR' [--view table\|list\|paths]` | New | Same envelope; optional ordered column table |
| Query explanation | `mg-vault query … --explain` | New | Parsed AST, normalized predicates, chosen plan, candidate counts, versions |
| Outgoing links | `mg-vault links PATH` | New | One record per link occurrence with byte range, rule, state |
| Backlinks | `mg-vault backlinks PATH` | New | Sections: resolved inbound, embeds, ambiguous inbound (not counted), unresolved-to-this-name |
| Unlinked mentions | `mg-vault mentions PATH` | New | Per-occurrence records with context excerpt and an explicit "link this" command line |
| Link diagnostics | `mg-vault links doctor [--check …]` | New | Ordered checks: broken, ambiguous, shadowed, unsafe target, duplicate heading/block ID |
| Graph, textual mode | `mg-vault graph --from PATH [--depth N] [--text]` | New; **canonical** representation | Deterministic adjacency listing, `node` blocks with indented `edge` lines |
| Graph, export | `mg-vault graph … --format json\|jsonl\|dot\|mermaid` | New | Machine formats; text/JSON are normative, DOT/Mermaid are conveniences |
| Saved query inventory | `mg-vault query list` / `query run PATH` / `query refresh PATH` | New | Vault-file backed list; run/refresh use the standard result envelope |
| TUI search pane (E) | `<leader>f` in the TUI | New pane, owned by E's workspace, fed by G contracts | Split: query bar, result list, preview |
| TUI backlink pane (E) | `<leader>b`, or toggled beside the editor | New pane | Sidebar list, same sections as the CLI command |
| TUI graph pane (E) | `<leader>g` | New pane | Two-region pane: deterministic ring drawing on top, canonical edge list below; focus lives in the list |

The TUI panes are consumers, not owners: E supplies pane geometry, focus, and session restore; G supplies the model, ordering, labels, and freshness. Every pane has a CLI equivalent that produces the same records, so the whole feature is usable with no TUI at all.

### 3.2 Interaction flows

#### Link resolution (the deterministic core)

Resolution runs over indexed source and never writes. For one link occurrence:

1. **Classify the raw target.** Wikilink forms `[[t]]`, `[[t|alias]]`, `[[t#Heading]]`, `[[t#H1#H2]]`, `[[t#^blockid]]`, `[[#Heading]]`, `[[#^blockid]]`, and the embed forms `![[…]]`. Markdown forms `[text](dest)`, `[text](dest "title")`, `![alt](dest)`, `<autolink>`, and reference links `[text][label]` resolved through the note's own link reference definitions (CommonMark case-folded label matching; an undefined label is literal text, produces no edge, and is not an error).
2. **Split the fragment.** The first unescaped `#` separates the note part from the fragment. `#^id` is a block reference; anything else is a heading path. An empty note part means "this note".
3. **Reject or externalize.** A destination matching `^[A-Za-z][A-Za-z0-9+.-]*:` is `External` and is recorded, not resolved (no filesystem touch). An absolute path, a `file:` URI, or any target whose normalized form escapes the vault root is `unsafe_target`: recorded, never resolved, reported by `links doctor`, and never followed. Markdown destinations are percent-decoded exactly once; wikilink targets are not percent-decoded. Both retain their exact raw bytes in the projection.
4. **Run the resolution ladder** (below). The first tier that yields **exactly one** candidate resolves the link and records `resolution_rule`. A tier that yields **more than one** candidate ends the ladder as `ambiguous` — it never falls through to a later tier and never applies a tie-break. A tier that yields zero candidates advances to the next tier. Exhausting the ladder is `unresolved`.
5. **Resolve the fragment** against the resolved note's heading/block projections (below). A note that resolves but whose fragment does not is `resolved_note_unresolved_fragment` — navigation opens the note at its start and says so in words.
6. **Record** target, rule, state, candidates, source byte range, and the fingerprint of the source observation the resolution was computed from. Resolution is a pure function of (indexed path set, target string, linking note path, resolver version); the same inputs always produce the same output, and a rebuilt-from-scratch index reproduces it exactly.

**The ladder.** Tiers are ordered and total; sub-steps inside a tier are also ordered.

| Tier | Rule ID | What it tries | Ambiguity possible? |
|---|---|---|---|
| 1 | `exact_path` | The target read as a vault-relative path, byte-for-byte as written, including extension. `notes/alpha.md` matches only that file. | No — a path names at most one file |
| 2a | `vault_relative` | Root-anchored path with extension inference: try `target`, then `target + ".md"`, after lexical normalization (`./` removal; no `..` permitted in this sub-step). | No |
| 2b | `vault_relative` (note-relative) | The same two forms resolved against the directory of the **linking** note; `..` segments are permitted only while the result stays inside the vault root. | No |
| 3 | `unique_basename` | Only when the target contains no `/`. Match `target` and `target + ".md"` against the basename of every indexed file, byte-exact. | **Yes** — two or more files sharing the basename |

Notes on the ladder:

- **Byte-exact everywhere.** No case folding, no Unicode normalization, no whitespace tolerance, no accent stripping. Path identity is bytes (criterion 1C, foundation A). A target that differs from a real file only by case or by NFC/NFD form is `unresolved` with a `near_miss` candidate list — the report names the exact byte difference. This is a deliberate, documented divergence from Obsidian's platform-dependent case-insensitivity, because silently matching a different byte string is exactly the "silently chooses a target" failure.
- **Shadowing is reported, not hidden.** Because tier order is total, a tier-1 hit can conceal a different file that tier 3 would have matched. Per-query resolution does not pay for detecting that; `links doctor --check shadowing` performs a whole-vault pass and reports `link_shadowed` advisories with both files named, so the precedence is auditable rather than invisible.
- **Nothing about resolution mutates a file.** Not the linking note, not the target, not frontmatter, not a block ID. There is no "fix links" side effect anywhere in G.

**Heading fragments.** The heading path `#A#B` resolves segment by segment: segment 1 against top-level-or-any headings of the note, each later segment only within the subtree of the previously matched heading. Comparison uses the heading's rendered inline text from the F/B parser (markup stripped, ATX closing `#` sequence removed), with leading/trailing whitespace trimmed and internal whitespace runs collapsed to one space; comparison is then byte-exact. Two headings whose normalized text is equal within the searched scope produce `ambiguous_heading` with the ordinals and line numbers of every occurrence; the tool never takes the first one. Recovery is offered, never performed: an interactive chooser for navigation, or a printed `mg-vault note …` command (owned by C/H) if the user wants to disambiguate by editing.

**Block fragments.** `#^id` matches an explicit trailing block identifier (`^[A-Za-z0-9-]+` at the end of a block) byte-exactly against the target note's block projection. Zero matches is `unresolved_block`. Two or more identical IDs in one note is `ambiguous_block`, listing every occurrence. **G never mints a block ID to satisfy a reference**; creating one is an explicit user-invoked C/H mutation with the usual preview, fingerprint precondition, and atomic write.

**Embeds** use the identical target grammar, identical ladder, and identical fragment rules, and are recorded with `kind: embed`. Rendering transclusion is F's job; G contributes only the edge and its state.

#### Backlinks and relationships

1. `backlinks PATH` requires an exact, confined vault-relative path (no fuzzy selection for anything that later feeds a mutation) and revalidates it against the filesystem before answering.
2. Results are grouped in a fixed order with literal section headers: `resolved inbound links`, `inbound embeds`, `inbound heading/block references`, `ambiguous inbound (not counted as backlinks)`, `unresolved links naming this file`, and, only when `--mentions` is passed, `unlinked mentions`.
3. An ambiguous link contributes **no** resolved edge to any candidate. It is listed under each candidate's "ambiguous inbound" section, excluded from every degree count and from the default graph, and marked with the literal word `ambiguous` plus its candidate list. `--include-ambiguous` adds it to counts and graph output as an edge whose `state` is `ambiguous`; the state travels with the edge everywhere, so no downstream consumer can lose it.
4. Every record carries the linking note's path, the link's byte range and line/column in that note, the raw target as written, the alias if any, the resolution rule, and the source fingerprint the record was derived from. Opening a result re-reads the source file through A's confined read and compares that fingerprint; a mismatch says `source changed since indexing` and offers refresh — the index is never the source of the bytes shown in the editor.

#### Unlinked mentions

1. Mentions are a suggestion surface, computed on demand — never on the save path and never implicitly during typing.
2. The candidate strings for a target note are its title, its frontmatter `aliases` (list or scalar, per Obsidian), and its basename without extension. Aliases are ordinary properties; they are never identity and never rewrite a file.
3. Scanning uses a multi-pattern automaton over indexed block text, requires grapheme-cluster word boundaries on both sides, requires at least three graphemes, is case-insensitive by default (`--case-sensitive` available), and **excludes** frontmatter, code spans, fenced/indented code, math spans, URLs and autolinks, existing link spans of any kind, and the target note itself.
4. Each hit reports path, line/column, byte range, and a bounded context excerpt with control characters escaped. `--limit` and `--after` paginate; the scan is cancellable and reports `complete: false` if a deadline cut it short.
5. "Link this mention" is never automatic. The command prints, and the TUI offers, an explicit per-occurrence C/H mutation with dry-run preview, exact byte range, expected fingerprint, and atomic commit. Declining changes nothing.

#### Search and structured query

1. Resolve one vault (`--vault` or selection). Cross-vault query requires an explicit vault list and keeps namespaces and cursors separate.
2. Parse the query into an AST (grammar in §4.3). A syntax, type, operator, or regex error returns `invalid_query` / `type_mismatch` / `unsupported_operator` with the exact byte span, the expected form, and no execution. Malformed input is never widened into a text search.
3. Snapshot freshness **before** execution. Default policy is fail-closed: anything other than `current` returns zero rows, exit 6, and a sentence naming the state, the indexed generation, the last complete observation time, and the refresh command. `--allow-stale` is an explicit read-only opt-in that labels the envelope, the banner, every page footer, and every human record block with `stale`, and is refused for any flow that feeds a mutation.
4. Plan and execute against B's projections. `--explain` prints the AST, normalized predicates, the index structures chosen, estimated and actual candidate counts, whether a bounded regex or live scan was needed, and `parser_version` / `resolver_version` / `schema_version` / `index_generation`.
5. Order deterministically: ranked search by integer rank descending, then vault-relative path in raw byte order, then match byte offset; unranked query by path then block byte offset. `--sort` accepts only documented keys with an explicit direction and keeps the same tie-breakers. Locale collation, float score, and filesystem enumeration order never determine output.
6. Paginate with an opaque cursor bound to vault, normalized query, sort, generation, and last key. A generation change returns `cursor_expired` rather than mixing pages.
7. Cancel on `Ctrl-C`, TUI `Esc`, or `--deadline-ms`. The response says `complete: false`, no partial page is treated as final, and no further page may be requested from a cancelled scan.
8. If the index service is unavailable, G offers a **clearly labeled** bounded live fallback for `text`, `title`, and `path` predicates only, with `mode: direct-scan`, `freshness: unindexed-live`, and a per-run cost estimate. Predicates that need projections (tags, properties, tasks, relationships, graph, mentions) are **refused** with `requires_index`, naming each unavailable capability. G never emulates a relationship from stale rows and never opens the SQLite file from the CLI or TUI.

#### Saved queries

1. A saved query is an **ordinary note**. Two forms, both visible in any editor and in Obsidian:
   - a note whose frontmatter contains an `mg-vault:` mapping with `query:` plus optional `sort:`, `limit:`, `columns:`, `view:`, `refresh_after:`;
   - a fenced code block with info string `mg-query` inside any note, whose body is one query expression optionally followed by the same trailing clause lines.
2. `query list` discovers them by scanning the index projection; the index row is a disposable pointer, and deleting the whole database loses nothing but discovery speed. `query run PATH[#block-N]` executes one. Any text editor can create, edit, rename, or delete a saved query with no tool involvement.
3. A saved query stores **no results** by default; it is a question, not a cache, so there is nothing to invalidate.
4. `query save --materialize` is opt-in and writes a results snapshot into the note between explicit HTML-comment markers carrying `id`, `query-hash`, `index_generation`, `resolver_version`, `parser_version`, and `run` timestamp. The write goes through A's atomic transaction with an expected-fingerprint precondition and touches **only** the bytes between the markers (criterion 1B). A snapshot is `stale` — banner-labeled, never silently refreshed, never presented as current — when the query text hash, index generation, resolver/parser version, or `refresh_after` age no longer matches. Refresh is an explicit, previewable command; if the markers are missing, malformed, nested, or unbalanced, the command reports `saved_query_invalid` and writes nothing.
5. Malformed query text in a saved note is reported at `query list` / `query run` time with the byte span inside the note. It is never rewritten, never auto-corrected, and never removed.

#### Graph

1. `graph` requires a focus (`--from PATH`) or an explicit `--all`. `--all` without limits on a large vault returns `graph_too_large` with the node/edge estimate and the exact flags needed (`--limit`, `--depth`, `--format jsonl`, `--export`), rather than attempting a 100,000-node render.
2. The model is one typed multigraph: nodes are indexed notes, plus optional pseudo-nodes for unresolved targets (`--include-unresolved`), tags (`--include-tags`), and attachments (`--include-attachments`); edges are `link`, `embed`, `heading_ref`, `block_ref`, `tag`, and `mention` (opt-in only), each with `state` in `resolved | unresolved | ambiguous | external`.
3. Traversal is breadth-first from the focus to `--depth N` (default 1, max 6), with `--direction out|in|both` (default `both`), and node ordering within each ring by raw byte path order. Cycles and revisits are marked, never expanded twice.
4. Textual mode is canonical: `node` blocks with indented `edge` lines, plus `--stats` (counts, components, degree ranking with documented tie-break), `--clusters` (connected components labeled by their lexicographically smallest member path), `--orphans`, and `--hubs`.
5. The TUI pane draws a **deterministic ring layout**, not a force-directed cloud: focus row, then one column per depth ring, members sorted by byte path order, so the same vault and focus render identically every time. Below the drawing sits the canonical edge list, and **keyboard focus lives in the list**. Every glyph has an ASCII fallback and a spelled-out equivalent in the list row.
6. Filters (`--kind`, `--state`, `--tag`, `--path-under`, any query expression via `--where`) apply identically to the drawing, the list, and every export, so no filter can make the picture and the text disagree.
7. Navigation from any graph or result row uses the exact path, revalidated directly against the filesystem before the editor opens it.

#### Degraded and unavailable

Any retrieval view in any of `stale | rebuilding | degraded | unknown | unavailable` states shows the literal state word first, then what is still possible (direct file reads, exact-path navigation, live text scan) and what is not (relationships, tags, properties, tasks, graph), then one recovery command. No view renders rows without its freshness banner, and no banner is color-only.

### 3.3 Layout descriptions

**Search / query results (human).** Reading order: (1) freshness banner; (2) query echo and normalization; (3) result records; (4) pagination line; (5) repeated freshness footer. One record is a block:

```
1  projects/alpha.md
   title    Alpha rollout
   match    text  line 42  "…rollback the cold start path…"
   fields   text, title
   source   sha256:9f2c… (indexed)
Index: current  generation 41  observed 2026-08-29T18:04:11Z  resolver link-resolver-v1
```

Under `--allow-stale`, every record block gains a first line `STALE — indexed generation 41, source changed since 2026-08-29T17:12:03Z`, so a copied fragment cannot lose the qualifier. Empty results say `No matches in indexed generation 41.` and never imply that no source match exists when freshness is not current.

**Backlinks pane / command.** Fixed section order with literal headers and counts, then records sorted by source path, then source byte offset:

```
resolved inbound links (3)
  journal/2026-08-19.md  4:3-4:24   [[alpha]]        rule=unique_basename
inbound embeds (1)
  moc/rust.md            17:1-17:22 ![[projects/alpha]]  rule=vault_relative
ambiguous inbound (not counted as backlinks) (1)
  inbox/capture-12.md    9:8-9:17   [[alpha]]        candidates: archive/alpha.md, projects/alpha.md
unresolved links naming this file (0)
```

**Graph, textual mode (canonical).**

```
node projects/alpha.md  depth=0 in=3 out=4 unresolved=1 ambiguous=1
  edge out link   moc/rust.md             resolved   rule=vault_relative  at=12:5-12:20
  edge out embed  assets/plan.png         resolved   rule=exact_path      at=20:1-20:24
  edge in  link   journal/2026-08-19.md   resolved   rule=unique_basename at=4:3-4:24
  edge out link   "alpha"                 ambiguous  candidates=2
    candidate archive/alpha.md
    candidate projects/alpha.md
  edge out link   "gone"                  unresolved near_miss=Gone.md (case differs at byte 0)
node moc/rust.md  depth=1 in=1 out=6
  …
graph: nodes=14 edges=27 components=1 focus=projects/alpha.md depth=2 direction=both
Index: current  generation 41  observed 2026-08-29T18:04:11Z  resolver link-resolver-v1
```

**Graph pane (TUI).** Two stacked regions in one pane. Top: the deterministic ring drawing, bounded to the visible area, with `▶`/`◀` for direction, `═` for embeds, `~` for ambiguous, `·` for unresolved, and an ASCII fallback set (`->`, `<-`, `=>`, `~?`, `..`). Bottom: the canonical edge list, one row per edge, carrying every attribute the drawing encodes plus the ones it cannot (rule, byte range, candidates). Focus, selection, and all keybindings operate on the list; the drawing highlights whatever the list has selected. Below 60 columns, or when the terminal reports no Unicode box-drawing support, or under `--no-graphics`, the drawing region is omitted entirely and the pane is the list — no information is lost, which is precisely why the list is canonical.

**Empty states.** `No links in this note.` / `No backlinks. This note is currently a leaf in generation 41.` / `No saved queries found. A saved query is an ordinary note with an mg-vault.query property.` / `Graph focus has no edges at depth 1.`

**Data sources.** Every view is driven by B's IPC responses plus A's direct filesystem revalidation for exact paths. No view reads SQLite directly, and no view treats an index row as note content.

### 3.4 Input & gestures

- Everything is keyboard-reachable; mouse, touch, stylus, voice, camera, and controller input are N/A for this feature. If E later adds optional mouse click-to-focus in the graph pane, it must remain strictly redundant.
- CLI commands: `search`, `query`, `links`, `backlinks`, `mentions`, `graph`, `query list|run|save|refresh`, `links doctor`.
- Shared retrieval flags: `--vault`, `--json`, `--jsonl`, `--limit`, `--after`, `--sort`, `--deadline-ms`, `--allow-stale`, `--refresh`, `--explain`, `--no-input`, `--no-color`, `--ascii`, `--no-graphics`.
- Graph flags: `--from`, `--all`, `--depth`, `--direction`, `--kind`, `--state`, `--where`, `--include-unresolved`, `--include-tags`, `--include-attachments`, `--include-ambiguous`, `--format`, `--stats`, `--clusters`, `--orphans`, `--hubs`.
- Query text is an argument, or explicit `--query-file FILE` / `--stdin`; unrelated piped stdin is never assumed to be a query.
- `--no-input` forbids every prompt and chooser, including the ambiguity chooser; unresolved ambiguity then returns a typed error listing candidates and the exact-path flag that would settle it.
- TUI keymap (documented, composable, Vim-consistent per E/D): `<leader>f` search pane, `<leader>b` backlinks, `<leader>g` graph, `<leader>q` saved queries, `gd` follow link under cursor, `gD` follow-in-split, `[[` / `]]` previous/next link in the buffer, `<C-o>` / `<C-i>` navigation history, `gr` backlinks for the current note, `gm` mentions for the current note. In the graph pane: `j`/`k` move in the list, `l`/`h` expand/collapse a ring, `<CR>` open, `f` focus the selected node, `u` undo focus, `/` filter, `s` cycle sort, `?` help. Every binding appears in `:help mg-vault-graph` and in `mg-vault graph --help`.
- Responsive behavior: layouts are verified at 40, 60, 80, and 120 columns. Tables become one-record-per-block below 60 columns. Paths, generations, candidate lists, error codes, and recovery commands are never truncated; they wrap with indentation.
- `NO_COLOR`, `--no-color`, non-TTY stdout, and `--json` all disable styling, and JSON never contains escape sequences.

### 3.5 Transitions & animation

No animation is required anywhere. The graph pane never animates layout, never relaxes a force simulation, and never re-flows on a timer; a focus change is a single atomic redraw of a deterministic layout. Long scans (mentions, regex, `--all` graph, whole-vault `links doctor`) emit rate-limited static progress to stderr in interactive human mode only, at most four updates per second, with stdout reserved for the result. Under `--json`, `--jsonl`, non-TTY output, `--quiet`, `NO_COLOR`, or a reduced-motion environment setting, no dynamic cursor updates are emitted at all — progress becomes a queryable static snapshot instead, so a screen reader is not re-interrupted. Pane transitions in the TUI are instantaneous state swaps owned by E.

### 3.6 Error states

| Code | Trigger | Presentation and recovery | Data-loss risk |
|---|---|---|---|
| `ambiguous_link` | Tier 3 matched two or more files | Inline record: `ambiguous`, ordered candidates, and the exact-path form that resolves it. Never a chosen target, never an edit | No |
| `ambiguous_heading` / `ambiguous_block` | Duplicate normalized heading text or duplicate block ID in the target | List every occurrence with ordinal and line; offer an interactive chooser for navigation only | No |
| `unresolved_link` | Ladder exhausted | Record `unresolved` plus `near_miss` candidates naming the exact byte difference; suggest the create/refactor command without running it | No |
| `unsafe_link_target` | Absolute path, `..` escape, `file:` URI, or symlink escape | Banner + `links doctor` finding naming the rejected operand and the rule; never followed | No |
| `invalid_query` | Syntax error | Byte span, caret, expected grammar; nothing executes | No |
| `type_mismatch` | Property/date compared against an incompatible literal | Names the observed type, the literal type, and one example fixture path; no coercion | No |
| `unsupported_operator` | Operator not implemented for that field | Echo the field's supported operator set and the grammar version | No |
| `regex_rejected` | Backreference, lookaround, or size/step limit exceeded | State the dialect (RE2-style) and the exceeded limit | No |
| `full_scan_required` | Regex or mention scan with no narrowing predicate | Print the estimated cost and require explicit `--full-scan`; refuse under `--no-input` without it | No |
| `query_too_complex` | Depth, clause, term, or cost budget exceeded | Name the exact limit and the simplification | No |
| `requires_index` | Relationship/tag/property/task/graph predicate while the service is unavailable | List each unavailable capability by name; offer live fallback for text/title/path only | No |
| `index_stale` / `index_rebuilding` / `index_degraded` | Freshness is not `current` | Zero rows, exit 6, literal state, generations, observation time, refresh command; `--allow-stale` labels every row | No |
| `graph_too_large` | Node/edge estimate over the cap | Print the estimate and the exact flags to bound or stream it | No |
| `cursor_expired` | Generation, query, sort, or vault changed | Restart from page one; pages are never mixed across generations | No |
| `cancelled` / `deadline_exceeded` | `Ctrl-C`, `Esc`, or `--deadline-ms` | `complete: false`; partial rows are labeled `partial: true` and cannot be paginated further | No |
| `saved_query_invalid` | Malformed query text, malformed/unbalanced snapshot markers, unknown clause | Byte span inside the note; the note is never rewritten or auto-corrected | No |
| `snapshot_stale` | Materialized snapshot's hash/generation/version/age no longer matches | Banner above the snapshot; explicit `query refresh` required | No |
| `source_changed` | Fingerprint mismatch when opening a result | `source changed since indexing`; re-read from disk and offer refresh; index bytes are never substituted | No |
| `path_not_found` / `unsafe_path` | Exact-path operand rejected by A | Reuse C's foundation error contract verbatim | No |

Error JSON reuses C's envelope: `version`, `ok:false`, `error.code`, `error.message`, `error.details`, `retryable`, and `recovery` when safe. Details may carry vault-relative paths, byte ranges, rules, candidate paths, counts, and versions; they never carry note bodies, secret-marked property values, query literals (unless `--verbose` is explicit), environment values, or absolute paths outside the vault.

### 3.7 Accessibility

- **Every interactive element has a text label, hint, and role.** In the CLI those are the literal field labels above. In the TUI, each list row announces role and state as words: "row 3 of 12, inbound link, journal/2026-08-19.md, resolved by unique basename, line 4 column 3"; ambiguous rows announce "ambiguous, 2 candidates, not counted as a backlink"; the pane announces "graph pane, focus projects/alpha.md, depth 2, 14 nodes, 27 edges".
- **Custom actions for complex interactions.** The graph pane exposes named actions rather than gestures: `focus node`, `expand ring`, `collapse ring`, `open in split`, `copy path`, `filter by kind`, `show candidates`. Each is a keybinding, appears in the pane's `?` help, and has a CLI equivalent.
- **The graph's textual mode is complete, not a summary.** This is a hard invariant with a conformance test (§5.2): the set of nodes, edges, edge kinds, edge states, candidate lists, counts, components, and filter effects rendered by the pane equals the set emitted by `graph --text` / `--json` for the same request. The drawing may add no datum of its own and may omit none. Below 60 columns or without Unicode support the drawing disappears and nothing is lost.
- **Color-independent state.** `resolved`, `unresolved`, `ambiguous`, `external`, `stale`, `partial` are always spelled out. Glyphs and color are redundant decoration; `--no-color` and `NO_COLOR` remove styling without removing meaning.
- **Text scaling / dynamic type is N/A in a terminal**; the equivalent obligation is wrapping correctness at 40/60/80/120 columns and correct display width for wide, combining, and RTL characters. Display width affects presentation only; path identity stays exact bytes, and control characters, bidi overrides, and escape sequences in paths, titles, aliases, snippets, and candidate lists are escaped as `\u{…}` before display.
- **Focus order** is the documented reading order of §3.3: banner, records in deterministic order, pagination, footer. In the TUI, focus enters the canonical list, never the drawing. Tab order never jumps into a decorative region.
- **No timed prompts** anywhere, including the ambiguity chooser. Progress is a static queryable snapshot.

---

## 4. Implementation Specification

### 4.1 Architecture placement

- `crates/mg-vault-index/src/` (exists today as one 960-line `lib.rs`) is split into modules and extended: `link/{grammar.rs,ladder.rs,resolve.rs}` for target parsing and the ladder, `link/mentions.rs`, `query/{lexer.rs,parser.rs,ast.rs,plan.rs,exec.rs}`, `graph.rs` for traversal/components/degrees, `saved.rs` for saved-query discovery and snapshot marker parsing, and the existing store as `store.rs`. The crate keeps B's hard rule: **no source-mutation API anywhere in it.**
- `crates/mg-vault-core` gains nothing but read-side helpers already needed by A/B (confined enumeration, fingerprints). Every G-adjacent mutation — link a mention, refresh a materialized snapshot, add a block ID — is a proposal rendered by the CLI and executed through core's transaction API (C for byte edits, H for refactors), never through the index crate.
- `crates/mg-vault-cli` gains `commands/{search,query,links,backlinks,mentions,graph}.rs` and `output/{results,graph,backlinks}.rs`. `main.rs`'s current inline `run_search` / `direct_search` / `search_output` helpers move into those modules.
- Markdown/YAML structure comes from the F/B token-preserving parser. G must not define a second dialect; it consumes headings, blocks, links, tags, properties, and tasks as B projects them and adds only resolution, mentions, graph, and the user-facing query surface on top.
- The TUI panes live in E's crate and consume G through B's IPC client. They link neither `rusqlite` nor the index repository module.

### 4.2 Data model

```rust
/// A link target exactly as written, before any resolution is attempted.
pub struct RawTarget {
    pub note_part: String,      // bytes before the first unescaped '#'
    pub fragment: Option<Fragment>,
    pub alias: Option<String>,
    pub syntax: LinkSyntax,     // Wiki | WikiEmbed | Markdown | MarkdownImage | Reference | Autolink
}

/// Heading path or explicit block identifier, never both.
pub enum Fragment { Heading(Vec<String>), Block(String) }

/// The ladder tier that produced a resolution. Recorded on every resolved link.
pub enum ResolutionRule { ExactPath, VaultRelative, NoteRelative, UniqueBasename }

/// The complete, deterministic outcome for one link occurrence.
pub enum Resolution {
    Resolved { target: NotePath, rule: ResolutionRule, fragment: FragmentResolution },
    Ambiguous { tier: ResolutionRule, candidates: Vec<NotePath> },   // ordered by raw byte path
    Unresolved { near_misses: Vec<NearMiss> },
    External { scheme: String },
    Unsafe { reason: UnsafeTargetReason },
}

/// A file that differs from the target only by case or Unicode normalization.
/// Recorded for the report; never resolved to.
pub struct NearMiss { pub path: NotePath, pub difference: ByteDifference }

pub enum FragmentResolution {
    None,
    Heading { ordinal: u32, byte_range: Range<u64> },
    Block { ordinal: u32, byte_range: Range<u64> },
    AmbiguousHeading { occurrences: Vec<u32> },
    AmbiguousBlock { occurrences: Vec<u32> },
    MissingHeading, MissingBlock,
}

/// One typed edge in the derived graph. Disposable; rebuildable from source alone.
pub struct Edge {
    pub from: NotePath,
    pub to: EdgeEndpoint,          // Note | UnresolvedTarget | Tag | Attachment | External
    pub kind: EdgeKind,            // Link | Embed | HeadingRef | BlockRef | Tag | Mention
    pub state: EdgeState,          // Resolved | Unresolved | Ambiguous | External
    pub rule: Option<ResolutionRule>,
    pub source_byte_range: Range<u64>,
    pub source_fingerprint: SourceFingerprint,
}

/// A saved query as read from an ordinary note. The note is the source of truth.
pub struct SavedQuery {
    pub note: NotePath,
    pub location: SavedQueryLocation,   // Frontmatter | FencedBlock { ordinal }
    pub query_text: String,
    pub sort: Option<SortSpec>,
    pub limit: Option<u32>,
    pub columns: Vec<ColumnSpec>,
    pub view: ViewKind,                  // List | Table | Paths | Graph
    pub refresh_after: Option<Duration>,
    pub snapshot: Option<MaterializedSnapshot>,
}

/// Provenance for an opt-in results snapshot written between explicit markers.
pub struct MaterializedSnapshot {
    pub marker_range: Range<u64>,        // the only bytes a refresh may replace
    pub query_hash: [u8; 32],
    pub index_generation: u64,
    pub resolver_version: String,
    pub parser_version: String,
    pub run_at: SystemTime,
    pub state: SnapshotState,            // Fresh | Stale { reason } | Invalid { reason }
}

/// Provenance stamped on every result set, page, graph render, and pane.
pub struct ResultProvenance {
    pub freshness: Freshness,            // extends today's Empty|Current|Stale|Degraded
    pub index_generation: Option<u64>,
    pub observed_at: SystemTime,
    pub resolver_version: &'static str,
    pub parser_version: &'static str,
    pub schema_version: u32,
    pub query_hash: [u8; 32],
    pub complete: bool,
    pub derived_from: &'static str,      // always "ordinary_files"
}
```

**Schema changes.** `SCHEMA_VERSION` 2 → 3; `PARSER_VERSION` `markdown-index-v1` → `markdown-index-v2`; new `RESOLVER_VERSION = "link-resolver-v1"` stored in `metadata` and echoed in every envelope. Because this is a projection-semantic change, migration uses B's side-by-side rebuild, never an in-place mutation of a published generation; a resolver-version mismatch marks the generation unusable and offers rebuild exactly as a schema mismatch does today.

New/changed tables (all disposable, all rebuildable from unchanged source): `links` gains `resolution_rule`, `resolved_source_id`, `fragment_kind`, `fragment_ordinal`, `state`, `candidate_count`; `link_candidates(link_id, ordinal, path)` holds ambiguity candidate lists so a report never recomputes them; `basenames(basename, source_id)` is the tier-3 index with a covering count; `near_misses(link_id, path, difference)`; `mentions(target_source_id, source_id, byte_range, context_digest)` is a bounded on-demand cache keyed by both fingerprints; `saved_queries(source_id, location, query_hash, view, snapshot_state)` is a discovery pointer only. Every derived row references one source row and the fingerprint it was derived from. **No table stores a note identifier that appears in a file**; no UUID is ever injected (criterion 1C).

Today's `notes.outgoing_links_json` — a list of raw wikilink targets with no position, no kind, and no resolution — is replaced by the `links` table and removed.

### 4.3 API contracts

**Query grammar (normative).**

```
query        := clause_list
clause_list  := or_expr ( trailing_clause )*
or_expr      := and_expr ( ("OR" | "|") and_expr )*
and_expr     := unary ( ("AND" | ",")? unary )*        # juxtaposition means AND
unary        := ("NOT" | "-")? primary
primary      := "(" or_expr ")" | predicate | bare_term
bare_term    := word | quoted_string | word "*"        # implicit text: scope, "*" = prefix
predicate    := field op value
field        := ident ("." ident)*
op           := ":" | "=" | "!=" | "<" | "<=" | ">" | ">=" | "~" | "=~" | "in"
value        := quoted_string | word | number | bool | date | duration | regex | list
regex        := "/" pattern "/" flags?                 # RE2-style: no backrefs, no lookaround
trailing_clause := "sort:" key direction? | "limit:" number | "columns:" list | "view:" ident
```

Precedence: `NOT` > `AND` > `OR`; parentheses override; juxtaposition is `AND`. Every operator/field pair is closed — an unlisted pair is `unsupported_operator`, never a fallback to text.

| Domain | Fields | Operators | Example |
|---|---|---|---|
| **Text** | `text`, `body` (alias), bare terms, `"quoted phrase"`, `term*` prefix | `:` `~` `=~` | `text:"cold start" AND NOT boilerplate` |
| **Titles** | `title`, `alias`, `name` (title OR alias OR basename) | `:` `=` `~n` (bounded fuzzy, n ≤ 3) `=~` | `title~2:"weekly reveiw"` |
| **Regex** | `text`, `title`, `path`, `tag`, `task.text`, `property.KEY` | `=~ /pat/flags` | `path =~ /^journal\/2026-0[1-6]\//` |
| **Tags** | `tag` (matches the tag and its descendants), `tag.exact`, `tag.count` | `:` `=` `>=` `<=` | `tag:project/alpha AND NOT tag.exact:project` |
| **Properties** | `property.KEY`, `property.KEY.exists`, `property.KEY.type`, list membership | `=` `!=` `<` `<=` `>` `>=` `:` `~` `=~` `in` | `property.status = "in-progress" AND property.rating >= 4` |
| **Paths** | `path` (substring), `path.under` (directory prefix), `path.name`, `path.ext`, `path.depth` | `:` `=` `=~` `<` `>` | `path.under:"projects/" AND path.ext:md` |
| **Relationships** | `link.to`, `link.from`, `embed.to`, `embed.from`, `link.unresolved`, `link.ambiguous`, `link.external`, `link.rule`, `mention.of`, `graph.distance(PATH)`, `orphan`, `leaf` | `:` `=` `<=` | `link.to:"moc/rust.md" AND graph.distance("index.md") <= 2` |
| **Tasks** | `task.state` (`todo\|done\|cancelled\|in-progress\|any`), `task.marker` (raw char), `task.text`, `task.due`, `task.scheduled`, `task.done`, `task.count` | `:` `=` `=~` `<` `>` `<=` `>=` | `task.state:todo AND task.due < 2026-09-01` |
| **Dates** | `created`, `modified` (filesystem-observed and labeled as such), any date-typed `property.KEY`, `task.*` dates; literals are ISO-8601 civil dates or RFC 3339 instants, plus `today`, `now`, and `today-7d` style offsets | `<` `<=` `>` `>=` `=` `in` | `modified >= today-7d OR property.due in 2026-09` |
| **Structure** | `heading`, `heading.level`, `block.id`, `section:"H"` (scopes the enclosed predicate to that section), `has.frontmatter`, `has.attachment` | `:` `=` | `section:"Decisions" AND text:rollback` |

Type rules: comparisons never coerce. A string property compared with `<` against a date literal returns `type_mismatch` naming the observed type and one example path. Civil dates compared to instants use the vault-configured timezone, which `--explain` prints. Regex runs only over a candidate set narrowed by another predicate, or with explicit `--full-scan`; it is bounded in pattern size and steps, and is cancellable. All SQL is parameterized; query text can never become SQL syntax.

**CLI signatures** (each returns C's `version: 1` envelope under `--json`; each accepts the shared retrieval flags of §3.4):

```text
mg-vault search QUERY [--text|--title|--regex|--tag|--property K[=V]|--path]  -> ResultPage
mg-vault query 'EXPR' [--view list|table|paths] [--columns …]                 -> ResultPage
mg-vault query --explain 'EXPR'                                               -> QueryPlan
mg-vault links PATH [--kind …] [--state …]                                    -> LinkRecords
mg-vault backlinks PATH [--include-ambiguous] [--mentions]                    -> BacklinkSections
mg-vault mentions PATH [--case-sensitive] [--full-scan]                       -> MentionRecords
mg-vault graph (--from PATH | --all) [--depth N] [--direction out|in|both] …  -> GraphModel
mg-vault query list | run PATH[#block-N] | save PATH [--materialize] | refresh PATH
mg-vault links doctor [--check broken|ambiguous|shadowing|unsafe|duplicate-ids]
```

**IPC operations** added to B's set, same framing, same versioning, same peer-UID check, same per-client cost limits:

```text
ResolveLink(vault, from_path, raw_target)   -> Resolution
Links(vault, path, filters, page)           -> LinkPage
Backlinks(vault, path, filters, page)       -> BacklinkPage
Mentions(vault, path, options, page)        -> MentionPage
Query(vault, QueryAst|text, options, page)  -> ResultPage
ExplainQuery(vault, text)                   -> QueryPlan
Graph(vault, GraphRequest)                  -> GraphPage
SavedQueries(vault)                         -> SavedQueryList
```

Every response embeds `ResultProvenance`. Pagination is B's opaque generation-bound cursor; `SearchOptions`-style page size defaults to 50, maximum 500; graph pages default to 500 nodes, maximum 5,000. Auth is B's: local per-user socket, peer credential check, no TCP, no cross-user API, and no ambient capability for plugins or AI — a plugin or AI adapter that wants query access needs N/O's explicit, scoped, previewed, attributable capability grant, and may propose but never execute a mutation.

### 4.4 State management

- **Derived state** (resolutions, candidates, basename map, mention cache, graph adjacency, saved-query discovery rows) is owned by the per-vault index actor in `mg-vault-index`, is disposable, and is rebuildable from unchanged source. Deleting the database and rebuilding must reproduce identical resolutions, ordering, and counts.
- **Authoritative state** is the files: link syntax, aliases, tags, properties, tasks, and saved queries all live in Markdown/YAML. The graph, every view, and every materialized snapshot are derived views with no authority of their own (criterion 3D).
- **View state** (focus node, ring depth, active filters, sort, cursor, selected row, navigation history) is owned by the CLI invocation or by E's pane and is session-restorable by path — a restored session with a missing or moved path shows an explicit unresolved entry rather than a guess.
- **Local-only, never server-synced.** There is no remote component in this feature at all.
- **Draft/offline persistence:** the query bar's in-progress text and a capped query history live under XDG state (never inside a vault, never inside `.obsidian`), are excluded from indexes and exports, honor `--no-history`, and are clearable with one documented command. Materialized snapshots are the only G-adjacent bytes that ever enter a vault, are opt-in, and go through A's atomic transaction with a fingerprint precondition; a failed refresh leaves the prior snapshot byte-identical.

### 4.5 Dependencies

- `rusqlite` **is already a workspace dependency** (`0.40.2`, `features = ["bundled"]`) and the bundled build compiles SQLite with `SQLITE_ENABLE_FTS5`, so FTS5 is reachable from SQL today. No FTS table, tokenizer configuration, or rusqlite `fts5` feature (the custom-tokenizer API) is configured yet; enabling a Unicode-aware tokenizer requires adding that feature explicitly and is part of B's dependency gate.
- `regex` (RE2-style, no backtracking) for the bounded regex predicate; `aho-corasick` for multi-pattern mention scanning; `unicode-segmentation` for grapheme-boundary mention matching and safe wrapping; `unicode-width` for terminal layout only.
- A graph library (`petgraph`) was considered and rejected: the traversals needed here are bounded BFS, connected components, and degree ranking over an adjacency map, and a hand-rolled model keeps ordering guarantees explicit and dependency surface small.
- No new assets, fonts, images, or data files. No infrastructure changes beyond B's existing per-user service and XDG directories. No network dependency of any kind.
- All additions are subject to the same license/SBOM/pinning review B's dependencies are.

### 4.6 Platform-specific considerations

- **Case-insensitive filesystems** (macOS APFS default, some network mounts) are the main portability hazard: the basename map stores exact bytes and resolution stays byte-exact everywhere, so a link differing only in case is `unresolved` with a `near_miss` on every platform. This is fixture-tested per platform so behavior cannot silently diverge.
- **Unicode normalization** differs across platforms and sync tools (NFD on some macOS paths). Paths are never normalized for identity; a normalization difference is a `near_miss`, reported with the exact byte difference.
- **Terminal capability** detection drives the graph drawing only: no Unicode box-drawing, a width below 60 columns, `--ascii`, or `--no-graphics` drops to the canonical list. Terminal capability never changes which records exist.
- **Renderer/framework migration:** none. This feature adds no rendering engine; the TUI pane is plain terminal drawing inside E's existing pane system.
- **Feature flags:** `mentions`, `graph-pane`, and `materialized-queries` may ship gated for gradual rollout. Freshness semantics, fail-closed defaults, ambiguity behavior, and the textual-equivalence invariant are **never** feature-gated.
- **Version compatibility:** the resolver version participates in B's protocol negotiation; a client and service disagreeing on resolver version get `protocol_incompatible` rather than mixed semantics.

### 4.7 Performance budget

Baseline corpus is B's: 100,000 notes, 1,000,000 blocks, ~2,000,000 link/tag/property/task projections, on recorded acceptance hardware. Numbers are acceptance budgets, not measured claims.

- **Editing is never blocked.** All G work is read-only and off the write path. During a full rebuild or a whole-vault `links doctor`, p99 added latency to direct `note read`/`note write` stays within B's ≤ 20 ms allowance, and no save waits on resolution, mention scanning, or graph work.
- **Resolution during rebuild:** the ladder is a hash-join over a path map and a basename map, target ≤ 90 s of the ≤ 10 min full-rebuild budget for ~2,000,000 links, with resolution structures ≤ 200 MiB peak.
- **Incremental update:** saving one note reprojects its outgoing links and updates affected backlink sets, p95 ≤ 300 ms, p99 ≤ 1 s, inside B's ≤ 500 ms / ≤ 2 s freshness budget. A create/rename that changes a basename's uniqueness re-evaluates only the links registered against that basename via a reverse index — cost is O(affected links), never O(vault).
- **Warm query** (index resident): text/title/tag/property/path first page p95 ≤ 100 ms, p99 ≤ 250 ms; `backlinks` for one note p95 ≤ 30 ms; 2-hop neighborhood of ≤ 2,000 nodes p95 ≤ 150 ms; `--stats`/`--clusters` over the whole graph p95 ≤ 3 s and cancellable.
- **Cold query** (service just started, page cache cold): first page p95 ≤ 750 ms; `backlinks` p95 ≤ 250 ms; graph neighborhood p95 ≤ 1 s. Cold state is reported in `--explain` so a slow first query is explicable rather than mysterious.
- **Unlinked mentions:** one note against the corpus p95 ≤ 1.5 s warm, hard-bounded, cancellable, never implicit, never on the save path or on keystrokes.
- **Cancellation:** `Ctrl-C`, `Esc`, or an expired `--deadline-ms` stops SQLite and scan work and returns `cancelled`/`deadline_exceeded` within p95 ≤ 100 ms of the signal, with `complete: false`, no further pages, and no partial page treated as final.
- **Typeahead in the TUI** debounces 120 ms, requests only the first page, cancels the in-flight request on the next keystroke, and never re-parses source.
- **Memory:** G's additions ≤ 64 MiB steady-state above B's ≤ 256 MiB target. Graph neighborhood caps at 5,000 nodes / 20,000 edges by default; `--all` requires explicit limits or streaming (`--format jsonl`), whose peak stays ≤ 128 MiB above baseline.
- **Storage:** resolution columns, candidate lists, basename map, and mention cache target ≤ 0.4× the existing projection size, inside B's ≤ 2.5× total index budget. The mention cache is bounded and evictable; dropping it costs only recomputation.
- **Network:** zero bytes. Every workflow in this feature is fully offline.
- **Startup:** no G structure is loaded eagerly; the basename map is built during rebuild and memory-mapped/queried from SQLite, so B's ≤ 150 ms IPC-health budget is unaffected.

---

## 5. Test Specification

### 5.1 Unit tests

| Test | Setup and assertion | Edge covered |
|---|---|---|
| `ladder_tier_order_is_total` | Fixture where the same target matches at tiers 1, 2a, 2b, and 3; assert the tier-1 file wins and `resolution_rule` says so | Deterministic precedence |
| `tier_three_collision_is_ambiguous_not_chosen` | Two `alpha.md` files; assert `Ambiguous` with both candidates in byte order and **no** `target` | 3B: never silently chooses |
| `ambiguity_never_mutates_source` | Resolve an ambiguous link, snapshot every file digest before/after | 3B: never mutates to disambiguate |
| `case_and_normalization_are_near_misses` | `[[Alpha]]` vs `alpha.md`; NFC vs NFD names | 1C identity; documented Obsidian divergence |
| `note_relative_cannot_escape_vault` | `[[../../etc/passwd]]`, `](/etc/passwd)`, `file:` URI, symlinked target | 4A confinement, `unsafe_link_target` |
| `markdown_and_wikilink_target_decoding` | Percent-encoded Markdown destination vs literal wikilink; reference links, undefined labels, autolinks | Syntax coverage without a second dialect |
| `duplicate_heading_and_block_ids_are_ambiguous` | Two `## Notes`, two `^abc`; assert every occurrence listed, none chosen | 3B at fragment level |
| `resolver_never_mints_block_ids` | Reference an absent `^id`; assert unresolved and byte-identical source | No mutate-to-disambiguate |
| `query_parser_precedence_and_spans` | `a OR b AND NOT c`, unbalanced parens, unknown field | Grammar determinism, actionable errors |
| `typed_comparisons_reject_coercion` | String property vs date literal, bool vs number, list vs scalar | `type_mismatch`, no guessing |
| `regex_dialect_and_limits` | Backreference, lookaround, catastrophic pattern, oversized pattern | `regex_rejected`, ReDoS resistance |
| `date_literals_and_offsets` | Civil date vs instant, `today-7d`, timezone echo | Date semantics |
| `mention_boundaries_and_exclusions` | Substring inside a word, inside code fence, inside frontmatter, inside an existing link, RTL and combining sequences | Mention precision |
| `graph_traversal_is_deterministic` | Same vault, same focus, 100 runs; assert byte-identical `--text` output | 3A determinism |
| `graph_excludes_ambiguous_edges_by_default` | Assert degree counts and edges with and without `--include-ambiguous` | Honest counts |
| `saved_query_parsing_both_forms` | Frontmatter form, fenced form, malformed clause, unknown key | 3D, no rewriting |
| `snapshot_invalidation_matrix` | Change query hash / generation / resolver version / age independently | `snapshot_stale` in every case |
| `provenance_present_on_every_response` | Property test over all response types | Stale-as-current auto-fail |
| `terminal_text_is_escaped` | ESC, bidi override, newline in path/title/alias/candidate/snippet | Terminal injection |

### 5.2 Integration tests

- **Rebuild equivalence:** index the fixture vault, snapshot every source digest, delete the database, rebuild, assert byte-for-byte source non-mutation and identical resolutions, candidate lists, ordering, graph output, and counts (criterion 1A + 3A).
- **Textual-equivalence conformance (auto-fail gate):** for each of 20 fixture focuses, assert set equality between the TUI graph pane's model (nodes, edges, kinds, states, candidates, counts, components, filter results) and `graph --text` / `graph --json` for the same request; assert the pane adds no datum and omits none; assert the pane still renders every datum at 40 columns with the drawing suppressed.
- **Freshness fail-closed:** edit a note behind the index; assert `search`, `query`, `links`, `backlinks`, `mentions`, and `graph` all return zero rows and exit 6 with the state named; assert `--allow-stale` labels the envelope, banner, every record block, and every page footer.
- **Degraded fallback:** stop the service; assert text/title/path fall back with `mode: direct-scan` and `freshness: unindexed-live`, that relationship/tag/property/task/graph predicates return `requires_index` naming each capability, and that no client opens the SQLite file.
- **Index-authority inversion:** tamper with a stored resolution row directly in SQLite; assert freshness verification reports `degraded` (as `persisted_derived_field_tampering_cannot_be_certified_as_current` does today for title/text/links) and that no tampered row is ever presented as current.
- **Saved queries survive index loss:** create both forms, delete the database, rebuild, assert both are rediscovered with identical parse results and that the notes are byte-identical.
- **Materialized snapshot transaction:** refresh with a concurrent external edit; assert fingerprint precondition failure, byte-identical note, retained prior snapshot, and no partial marker write; assert a successful refresh changes only the bytes between markers.
- **Obsidian coexistence:** a vault with `.obsidian/graph.json` and community-plugin link syntax; assert `.obsidian` is never read as authority, never written, never indexed as notes, and that unknown syntax passes through untouched.
- **Scale:** on the 100k/1M corpus, assert every §4.7 budget with p50/p95/p99, peak RSS, DB growth, cancellation latency, and concurrent direct-write latency during a rebuild.
- **Cross-vault isolation:** two vaults with identical note names; assert no resolution, candidate list, cursor, or graph edge crosses namespaces.

### 5.3 UI / E2E tests

CLI E2E for every command in human, `--json`, and `--jsonl` modes, with golden fixtures covering: happy path, empty results, ambiguous link, unresolved link with near-miss, duplicate heading, invalid query with byte span, `type_mismatch`, `regex_rejected`, `graph_too_large`, `cursor_expired`, `cancelled`, stale opt-in, and service unavailable. Goldens assert envelope version, provenance fields, deterministic ordering, and stable error codes without depending on volatile timestamps.

TUI E2E (once E lands, gating the pane claim, not this spec): open the search pane, type a query, cancel mid-flight, page forward and back, open a result in a split, jump to backlinks, open the graph pane, expand a ring, filter by kind, select an ambiguous edge and read its candidates, restore the session after restart with one target note moved. Assert every step is keyboard-only and every state is announced in words.

A navigation E2E follows `gd` into a resolved link, then into an ambiguous one (asserting a chooser appears and nothing is written when cancelled), then into an unresolved one (asserting an offer to create, and no file created unless accepted).

### 5.4 Visual / manual verification

- Terminal themes: default, light, dark, high-contrast, and fully ANSI-stripped; confirm every state word survives.
- `NO_COLOR`, `--no-color`, `--ascii`, `--no-graphics`, and a non-TTY pipe; confirm identical record content in all five.
- Widths 40, 60, 80, 120 with long Unicode, RTL, and combining-character paths; confirm no path, candidate, generation, error code, or recovery command is truncated, and that the graph drawing disappears below 60 columns without losing rows.
- Empty vs populated: zero results, one result, 500 results, a 5,000-node capped graph, an orphan note, a node with 400 backlinks.
- Freshness states: current, stale, rebuilding, degraded, unavailable — each verified to show its banner above and below the rows.
- Screen-reader pass over search paging, backlink sections, an ambiguity report, and the graph pane's list.
- Terminal font-size zoom as the dynamic-type analogue; confirm reflow correctness.

---

## 6. Compliance & Safety Gate

### 6.1 Sensitive data classification

- [ ] No sensitive data involvement
- [x] **Handles sensitive data** — note text, titles, aliases, paths, tags, properties, tasks, snippets, mention context, and query literals may all be private. Protections: everything is local and offline; the index database and IPC endpoint keep B's owner-only permissions and peer-UID check; secret-marked properties and excluded paths never enter query results, snippets, graph labels, mention context, saved-query snapshots, or logs; query literals are not logged by default and `--no-history` disables the local history file entirely; error details carry paths, ranges, codes, and versions but never bodies; materialized snapshots are opt-in and inherit the same exclusions, so publishing a saved-query note cannot leak an excluded property.
- [x] **Uses synthetic/test data only until compliance gate clears** — all fixture, adversarial, and 100k-scale corpora are generated; no personal vault is required for acceptance.

### 6.2 Asset provenance

- [x] **No third-party assets** — this feature adds no images, fonts, models, or data files. `regex`, `aho-corasick`, `unicode-segmentation`, and `unicode-width` are code dependencies subject to the same license/SBOM review as B's, not bundled content.
- [ ] Uses third-party assets

### 6.3 Language / claims audit

- [x] Make claims not supported by evidence? **No.** Every latency, memory, and scale figure in §4.7 is an acceptance budget and is labeled as such; none may be marketed as achieved before benchmark artifacts pass.
- [x] Promise capabilities not yet built? **No.** Everything here is explicitly target state; §7 records exactly what exists, what B plans, and what is absent. The TUI panes are labeled as E-gated and must report `provided_by: "tui"` rather than `available: true` until E ships.
- [x] Use language restricted by domain regulations? **No.** This is not a regulated domain.

### 6.4 Regulatory alignment

Lens 3 of `criteria.md` is this spec's binding gate:

- **3A Determinism.** Resolution is a pure function of the indexed path set, the target string, the linking note's path, and the resolver version; the ladder is total and ordered; ordering, ranking, tie-breaks, traversal, and component labeling are all specified byte-deterministically; §5.2's rebuild-equivalence test proves a from-scratch rebuild reproduces every resolution, backlink, search result, graph render, and view.
- **3B Ambiguity.** A tier that yields more than one candidate ends the ladder as `ambiguous` with an ordered candidate list and **no** target; no tie-break exists anywhere; duplicate headings and duplicate block IDs behave identically; nothing in G writes to a file, mints a block ID, or edits frontmatter to make a link resolve; recovery is always a printed or offered explicit user action.
- **3C Query depth.** §4.3 enumerates grammar for text, titles, regex, tags, properties, paths, relationships, tasks, dates, and structure, with per-field operator sets, precedence, type rules, and refusal (`unsupported_operator`) instead of silent broadening.
- **3D Derived authority.** Links, backlinks, mentions, search results, saved-query results, snapshots, and the graph are all derived views over ordinary files. Saved queries are ordinary notes, not hidden database rows. `mg-vault-index` has no source-mutation API; every G-adjacent write is an explicit, previewable, atomic core transaction. Deleting the database and rebuilding loses nothing but time.
- **3E Scale.** §4.7 gives cold, warm, and incremental budgets plus memory, storage, cancellation, and editing-isolation limits at 100,000 notes and 1,000,000 blocks, with the reverse-index design that keeps rename/create cost proportional to affected links rather than vault size, and §5.2 makes the corpus an acceptance gate.

Named auto-fail rules:

- **"Index state overriding source"** — no index row is ever presented as note content; opening any result re-reads the file through A and compares fingerprints; tampered derived rows are caught by freshness verification and reported `degraded`; the index crate exposes no write path to source.
- **"Stale index presented as current"** — `ResultProvenance` is mandatory on every response, page, pane, banner, footer, and human record block; default policy is fail-closed with exit 6 and zero rows; `--allow-stale` labels every layer including individual records; degraded modes announce themselves and name each unavailable capability; the live fallback is labeled `unindexed-live` and never masquerades as indexed.
- **"Graph or Canvas information lacking a textual equivalent"** — the textual adjacency listing is the canonical representation, the drawing is derived from it, keyboard focus lives in the list, the drawing is suppressed entirely on narrow or non-Unicode terminals with no loss, and §5.2's conformance test asserts set equality of nodes, edges, kinds, states, candidates, counts, components, and filter effects between pane and text.
- Additionally: no source-content loss (G writes nothing except opt-in snapshot bytes between explicit markers, atomically, with a fingerprint precondition); no partial multi-file mutation (G performs no multi-file mutation at all); no silent conflict winner (fingerprint mismatch reports both states); no unsafe traversal or symlink escape (`unsafe_link_target`, A's confined reads); no capability bypass (plugins/AI need N/O's explicit scoped grant).

---

## 7. Gap Analysis vs. Current State

### 7.1 What exists today

**Implemented.** `mg_vault_core::index` (`crates/mg-vault-core/src/index.rs`) walks `.md` files in lexical order, skips `.obsidian` and `.mg-vault`, reads through `openat2` with `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS` on Linux, double-reads to detect concurrent change, derives a title from frontmatter `title` → first `# ` heading → file stem, and produces `IndexedNote { path, title, text, fingerprint, outgoing_links }`. `MarkdownIndex::search` is a case-lowered substring match over path/title/text/links, ordered by path. `mg-vault-index` (`crates/mg-vault-index/src/lib.rs`) persists this to SQLite `metadata`/`notes`/`diagnostics` at `SCHEMA_VERSION = 2` and `PARSER_VERSION = "markdown-index-v1"`, publishes generations atomically, verifies freshness by two matching complete source observations against the published manifest, exposes `empty | current | stale | degraded`, refuses to search a non-`current` generation (`StoreError::NotCurrent`), rejects symlinked database paths and parents, and transactionally resets a legacy v1 cache. `crates/mg-vault-cli/src/main.rs` exposes `index rebuild`, `index status`, and `search QUERY` with a labeled direct-scan fallback plus `--json`, `--no-input`, `--no-color`, and `NO_COLOR`. `crates/mg-vault-index/tests/persistent_store.rs` (712 lines) covers rebuild determinism, publication aborts, drift, tampering, and symlink refusal.

**Prototyped.** `index::wikilinks()` extracts raw `[[target]]` strings, splitting the alias at `|`, and stores them as `outgoing_links_json`. There are no positions, no kinds, no Markdown links, no embeds, no heading or block fragments, and **no resolution of any kind** — the stored string is the target as typed. `search_current`'s freshness gate is the working seed of the fail-closed retrieval contract this spec generalizes.

**Planned (specified, not built).** Spec B (`gauntlet-output/specs/b-index-service-search.md`) plans the watcher/service, IPC protocol, incremental reconciliation, heading/block/link/tag/property/task projections, FTS, the query grammar's storage layer, cursors, cancellation, and the `LinkProjection { resolved_target, resolution, resolver_version }` fields this feature fills in. B explicitly defers final link resolution, unlinked mentions, saved-query persistence, and graph semantics to G (its §7.4 and §7.5).

**Gated.** FTS5 is compiled into the bundled SQLite the workspace already pins (`rusqlite 0.40.2`, `features = ["bundled"]`), so it is reachable from SQL, but no FTS table, tokenizer choice, or rusqlite `fts5` feature is configured. The TUI panes are gated on E; refactor-style mutations offered by ambiguity and mention reports are gated on C/H.

**Absent.** Link resolution, backlinks, unlinked mentions, the query grammar and planner, saved queries in any form, the graph model and both of its renderings, `links`/`backlinks`/`mentions`/`graph`/`query` commands, resolution-aware SQLite tables, the basename map, near-miss detection, shadowing lint, pagination, cancellation, and every test in §5.

### 7.2 Delta to spec

- **New modules** in `crates/mg-vault-index/src/`: `link/{grammar,ladder,resolve,mentions}.rs`, `query/{lexer,parser,ast,plan,exec}.rs`, `graph.rs`, `saved.rs`; split the existing `lib.rs` into `store.rs` plus a re-export root.
- **New modules** in `crates/mg-vault-cli/src/`: `commands/{search,query,links,backlinks,mentions,graph}.rs`, `output/{results,graph,backlinks}.rs`; move `run_search`, `direct_search`, `search_output`, and the status formatters out of `main.rs`.
- **Modified:** `mg_vault_core::index` to emit positioned, kinded link records instead of bare target strings (or to defer entirely to the F/B parser once it lands); `mg-vault-cli` command enum and help; `README.md` and `docs/PRODUCT.md` retrieval descriptions.
- **Schema/migration:** `SCHEMA_VERSION` 2 → 3, `PARSER_VERSION` → `markdown-index-v2`, new `RESOLVER_VERSION`; add `link_candidates`, `basenames`, `near_misses`, `mentions`, `saved_queries`; extend `links`; drop `notes.outgoing_links_json`. Side-by-side rebuild only, per B; the existing legacy-reset path is the precedent.
- **New dependencies:** `regex`, `aho-corasick`, `unicode-segmentation`, `unicode-width`; enable a documented FTS5 tokenizer configuration under B.
- **New fixtures and harnesses:** an ambiguity/duplicate-heading/duplicate-block/near-miss/Unicode-RTL compatibility vault; the 100k-note corpus generator; the pane-versus-text conformance harness; golden JSON for every command and error.
- **Documentation:** the resolution ladder (including the deliberate case-sensitivity divergence from Obsidian), the query grammar reference, saved-query file format and snapshot markers, graph textual format, and the freshness/degraded contract.

### 7.3 Estimated scope

**XL.** Four substantial subsystems (a resolution engine with an auditable ladder, a typed query language with planner and ranking, a bounded mention scanner, and a graph model with two conformant renderings) sit on top of B, which is itself XL and largely unbuilt. Deliver as gated increments behind one freshness and one ambiguity contract: **G1** link grammar + ladder + candidates/near-misses; **G2** backlinks and `links doctor`; **G3** query grammar, planner, ranking, pagination, cancellation; **G4** saved queries as files (no snapshots); **G5** graph model + canonical textual mode + exports; **G6** unlinked mentions; **G7** TUI panes and opt-in materialized snapshots. G1–G5 are individually reviewable and individually shippable behind the CLI.

### 7.4 Blocking dependencies

- **A (foundation):** confined path validation, byte-exact path identity, source fingerprints, atomic transactions for the opt-in snapshot write, and the trash/recovery contract. Implemented.
- **B (index service):** structural projections (headings, blocks, links, tags, properties, tasks), FTS, generations and freshness, IPC framing/versioning/peer auth, cursors, cancellation, and the 100k benchmark harness. G cannot ship past G1 without B1–B5.
- **F (Markdown parser):** the token-preserving CommonMark/GFM/Obsidian model that defines heading inline text, block boundaries, block IDs, code/math spans, and link syntax. G must consume it rather than define a second dialect.
- **C (CLI):** the `version: 1` envelope, error contract, exit codes, `--json`/`--jsonl` conventions, exact-path chooser rules, and editor handoff.
- **E (TUI):** panes, focus, keymaps, and session restore for G7. Absent E, every G capability remains fully available from the CLI.
- **I (properties/schemas)** and **J (tasks/dates):** canonical typed-property and task semantics that refine the property/task/date predicates without making the projection authoritative.
- **H (refactoring):** the atomic previewable rename/move/extract operations that ambiguity and mention reports *offer*; G never performs them.
- **N/O (plugins/AI):** the capability grant any non-first-party consumer of the query IPC must obtain. Not required for G itself.

### 7.5 Non-goals

- Editing, repairing, renaming, or reformatting any note to make a link resolve; minting block IDs; auto-linking mentions; injecting UUIDs or any hidden identifier.
- Making resolutions, the graph, saved-query snapshots, or any index row authoritative over files.
- Case-insensitive or Unicode-normalizing link resolution (documented divergence from Obsidian; near-misses are reported instead).
- Link-aware rename/refactor (H), Bases views/formulas/rollups (I), Canvas graph semantics (K), or task mutation (J).
- Force-directed, animated, or otherwise non-deterministic graph layout; image-only graph export.
- Cross-vault graph traversal in one model; remote/cloud search; multi-user collaboration; a network extractor.
- Direct SQLite access from the CLI, TUI, Quickshell, plugins, AI, or `mg-calr`.

---

## 8. Open Questions

- **Q1:** Should the tier-3 basename ladder be configurable per vault (for example, disabled entirely so that only explicit paths resolve, matching a stricter workflow)? — blocks a config key in §4.3; the default and the never-tie-break rule stand either way.
- **Q2:** What is the default for `--include-ambiguous` in the graph and in backlink counts? This spec defaults to excluded-but-listed; a user who links loosely may prefer them counted with the state carried. — blocks defaults in §3.2 and §3.3, not the data model.
- **Q3:** Should unlinked mentions default to case-insensitive (this spec's choice, as a suggestion surface) or byte-exact (consistent with resolution)? — blocks the default in §3.2; both modes exist regardless.
- **Q4:** Where does the vault timezone used for civil-date/instant comparison come from — a `.mg-vault` config key, the system zone, or a required explicit setting? — blocks the date semantics note in §4.3 and the `--explain` output.
- **Q5:** Should materialized query snapshots ship at all in the first release, given that they are the only G feature that writes into a vault? — blocks G7 sequencing in §7.3; the marker format and invalidation rules are specified either way.
- **Q6:** Should tag and attachment pseudo-nodes be included in the default graph model or remain opt-in flags, given the node-count budget at 100k notes? — blocks defaults in §3.2, not the traversal budget.
