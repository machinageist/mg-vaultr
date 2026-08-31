# Spec: Properties, Schemas, and Bases

**Feature ID:** i-properties-schemas-bases
**Parent feature:** root
**Spec author agent:** Spec Gauntlet agent I (properties/schemas/Bases)
**Date:** 2026-08-29
**Iteration:** 1

---

## 1. Purpose

### 1.1 One-sentence job

Let a user give the YAML frontmatter in ordinary Markdown files a declared, checkable shape — and query, relate, aggregate, and tabulate those files through Obsidian-compatible `.base` views — without any schema, Base, formula, relation, or rollup ever becoming authority over, or silently rewriting, the bytes in a note.

### 1.2 Why it matters

Properties are where file-authoritative tools usually break their own promise. The ordinary way to change one YAML key is to deserialize the frontmatter, mutate a map, and serialize it back — which reorders keys, drops every comment, renormalizes quoting, collapses block scalars, expands anchors, discards tags and document markers, and rewrites CRLF as LF. The user asked to change `status`, and the tool silently rewrote a file it did not understand. That is precisely the "unknown syntax loss" and "source-content loss" this product's criteria treat as automatic failure, and `docs/spikes/token-preserving-markdown-yaml.md` already forbids it: *no component may serialize a whole document back to storage.*

The second failure mode is authority inversion. Notion-style typed views, Dataview-style computed fields, and relation/rollup systems normally live in a database that becomes the real product; the files degrade into an export format. `mg-vault` inverts this: a schema is an ordinary YAML file in the vault, a Base is an ordinary `.base` file in the vault, and every formula, relation, inverse relation, and rollup is a *computation over files performed at view time* and thrown away. Deleting every index and cache and recomputing from unchanged source must produce the identical table.

The third failure mode is the quiet wrong number. Relations form a graph, users create cycles by accident, and a rollup that follows a cycle either hangs or — far worse — returns a plausible partial sum. This feature makes a cycle a named, typed, non-numeric value that propagates, never a zero and never a blank cell.

### 1.3 Success signal

On a versioned Obsidian-Bases fixture corpus plus an adversarial YAML corpus (comments in every position, all five scalar styles, anchors, aliases, merge keys, explicit tags, `%YAML` directives, `...` end markers, CRLF and mixed endings, duplicate keys, Unicode and RTL keys): every fixture Base evaluates to the recorded expected result set, ordering, and per-cell types; every property edit changes only the declared value span and leaves every other byte of the file identical, proven by prefix/suffix/gap equality plus a CST re-index comparison; a relation cycle produces a `cycle` cell naming the ordered path and never a number; and a full `schema migrate` over 20,000 affected notes either commits every file or leaves every file byte-identical to its pre-image under fault injection at every journal phase, with a receipt that rolls back all 20,000 byte-for-byte.

---

## 2. User Stories

> As a writer, I want to change `status` in a note's frontmatter from the CLI, so that my carefully formatted YAML — its comments, key order, quoting, and CRLF endings — comes back byte-identical apart from the four characters I asked to change.

> As a writer, I want to declare that `due` is a date and `rating` is a number between 0 and 5, so that a typo is reported as a diagnostic instead of quietly sorting my table wrong.

> As an Obsidian user, I want my existing `.base` files to open and evaluate in the terminal with the same rows in the same order, so that I can work in either app on the same vault without converting anything.

> As an Obsidian user whose Base uses a function `mg-vault` has not implemented, I want the file left exactly as written and the view to say plainly which feature it cannot evaluate, so that I never lose work and never get rows filtered by a rule the tool only half understood.

> As a maintainer of a 40,000-note vault, I want to change `priority` from text to number across the whole vault as a dry run first, see every note that will change and every value that cannot be converted, and roll the whole thing back from one receipt — so that a schema decision is not a one-way door.

> As a project tracker, I want a `parent` relation and a rollup of my children's open task count, so that a Base table shows real aggregates — and if I accidentally make A the parent of B and B the parent of A, I want the cell to say `cycle A → B → A`, not `0`.

> As a screen-reader and 60-column-terminal user, I want every Base table available as a linear labeled record list and as stable JSON with identical content, so that a column that does not fit is never information I cannot reach.

> As an automation author, I want `--json`, `--jsonl`, `--dry-run`, `--no-input` that refuses rather than assumes yes, and stable exit codes for every property, schema, and Base command, so that scripted schema work is auditable and cannot silently escalate into a vault-wide rewrite.

---

## 3. UX Specification

This is a CLI and TUI product. There is no GUI, no mouse requirement, no sound, and no haptic anywhere in this feature. "Views" below are terminal output and terminal panes.

### 3.1 Screen / view inventory

| View | Entry point | New / modified | Layout pattern |
|---|---|---|---|
| Property listing | `mg-vault property list PATH` | New | Linear key/type/value/state records, one block per property |
| Property get | `mg-vault property get PATH KEY` | New | Single record: raw lexeme, style, declared type, parsed value, byte range |
| Property edit preview | any mutating `property` subcommand, always before write; also `--dry-run` | New | Header, single-file unified diff of the frontmatter region only, precondition block |
| Schema report | `mg-vault schema show` | New | Sections: scopes, property definitions, relations, rollups, source file and digest |
| Schema conformance report | `mg-vault schema check [--path P]` | New | Per-rule sections, then per-note diagnostics sorted by path, then a counts footer |
| Schema inference proposal | `mg-vault schema infer` | New | A proposed schema document printed to stdout plus an evidence table; writes nothing without `--write` |
| Migration plan preview | `mg-vault schema migrate KEY --to TYPE` | New | H-style five-block plan: header, complete file manifest, per-note diffs, classification lists, precondition block |
| Migration receipt / history | `mg-vault schema migrate history\|rollback` | New | Newest-first receipt table with a `rollbackable` column and reason |
| Schema diagnostics | `mg-vault schema doctor --check …` | New | Ordered checks: conformance, duplicates, inherited, relations, cycles |
| Base inventory | `mg-vault base list` | New | One record per discovered `.base` file or embedded ```` ```base ```` block |
| Base table view | `mg-vault base run PATH [--view NAME]` | New; **tabular** | Freshness banner, unsupported-feature line, column header, aligned rows, group headers, footer |
| Base record view | `mg-vault base run … --view records` | New; **canonical textual equivalent** | One labeled `key: value` block per row, every column present |
| Base explanation | `mg-vault base run … --explain` | New | Parsed filter tree, definition evaluation order, index structures used, limits, versions |
| Base validation | `mg-vault base check PATH` | New | Parse result, supported/unsupported feature inventory with byte ranges |
| Materialize preview | `mg-vault base materialize …` | New | Same plan/preview/confirm machinery as a migration |
| Property form pane (TUI) | E command palette → `properties`, or `<leader>p` | New pane, owned by E, driven by I contracts | Two-column key/value form above a diagnostics strip |
| Base pane (TUI) | E command palette → `base`, or `<leader>t` | New pane | Table region with a frozen first column, plus a detail strip for the focused cell |
| Schema diagnostics pane (TUI) | `<leader>D` | New pane | Same records as `schema check`, navigable |

TUI panes are consumers: **E** owns pane geometry, focus, keymap registration, and session restore; **I** owns the model, ordering, column widths, state words, and every string. Every pane has a CLI equivalent producing the same records, so the feature is fully usable with no TUI at all.

### 3.2 Interaction flows

#### Property edit — the token-preserving core

Every property mutation follows exactly this sequence. There is no second, weaker path.

1. **Read and fingerprint.** `Vault::read_note` returns bytes and a `SourceFingerprint`. All later steps carry that fingerprint.
2. **Locate the envelope.** `scan_frontmatter` (implemented) returns exact YAML and body byte ranges and the observed `LineEndings`. `UnsupportedBom` and `Unclosed` fail closed with `frontmatter_ambiguous`; the file is never guessed into an editable region and never rewritten to "fix" it.
3. **Build the CST index.** Parse the YAML slice with `yaml-edit` into a lossless CST and walk it into a `YamlIndex`: for every node, the absolute byte range of its key token, its value token, its style, its anchor, its alias status, its explicit tag, its indentation column, its ordinal among siblings, and the byte ranges of every comment token with an ownership classification (§below). Absolute offsets are the CST offset plus `envelope.yaml_range().start`.
4. **Resolve the target path.** `KEY`, `a.b.c` for nested maps, `list[3]` for sequence items, and `KEY#2` for the second occurrence of a duplicated key. A duplicate key without an explicit occurrence is `duplicate_key`, listing every occurrence with line, column, and byte range — never a silent first-wins.
5. **Plan the edit** as one or more **disjoint byte spans** with replacement bytes (§4.3 `PropertyEdit`). Overlapping spans are rejected at plan time.
6. **Style decision.** The replacement is emitted in the *existing* style of the value token whenever that style can carry the new value. A style change is required only when the value would change meaning (a plain scalar that would now parse as bool/number/null/date, contains `: `, ` #`, a leading indicator character, a newline, or trailing whitespace). When required, the preview prints `restyled: plain → single-quoted (value would parse as a boolean)`. A style change is never silent and never cosmetic.
7. **Preview.** A unified diff of the frontmatter region only, plus a precondition block naming the expected fingerprint, the exact spans, and any `restyled` / `comment moved` notices. `--dry-run` stops here with exit 0 and `changed: false`.
8. **Verify the candidate before writing** (§4.3 `verify_edit`). The candidate bytes are re-scanned and re-parsed; the resulting `YamlIndex` must match the pre-edit index on every key path, sibling ordinal, style, anchor, tag, and comment token except the ones the plan declared changed; the prefix before the first span, the suffix after the last, and every inter-span gap must be byte-identical; and the document must still be valid YAML with the intended value. Any mismatch aborts with `edit_verification_failed` and writes nothing.
9. **Commit** through `Vault::edit_note_spans` — one fingerprint check, one buffer assembly, one `replace_atomic`. A changed file returns `Error::Conflict` with both digests and writes nothing.

**Comment ownership rule** (concrete, because removal depends on it):

- A comment on the same line *after* a value is **owned by that entry**; a value edit preserves it, and an entry removal removes it with the entry.
- An unbroken run of full-line comments immediately preceding an entry, with no blank line between the run and the entry, is **owned by that entry**.
- A full-line comment run separated from the following entry by at least one blank line, or appearing before the first entry, or after the last entry, is **floating** — owned by the document. Removing an entry never removes a floating comment.
- A comment run that both follows an entry's value line and precedes the next entry with blank lines on **both** sides is `comment_ownership_ambiguous`; removal refuses. This is the spike's existing binding policy, made testable.

**Anchors, aliases, merge keys, tags.**

- Editing a node that *carries* `&anchor` rewrites only the value token; the anchor survives. The preview states `this anchor is referenced by N alias sites; their resolved value changes with it` and lists them.
- Editing a node that *is* an alias (`*name`) is refused with `yaml_alias_target`: replacing it with a literal would silently sever the anchor relationship. Recovery offered, never performed: edit the anchor, or convert the alias explicitly with `--break-alias`.
- Keys arriving through a merge key (`<<: *base`) are visible to queries and Base columns as `inherited` values with the anchor's location. A write targeting an inherited key is refused with `property_inherited`; `--materialize-inherited` explicitly adds a real shadowing key (an additive edit that removes nothing) after a preview that shows the shadowing.
- An explicit tag (`!!str`, `!custom`) is preserved on the node. A scalar edit under a tag keeps the tag and validates the new lexeme against it. A tag `mg-vault` does not model gives the node type `unknown_tagged`: queryable as its raw lexeme, never coerced, never a migration target without `--allow-tagged`.
- `%YAML` / `%TAG` directives and `...` end markers inside the envelope are preserved verbatim; because `scan_frontmatter` closes on the first exact `---` line, a second YAML document cannot be inside frontmatter, and `---` inside a value is only a delimiter when it is the entire line — already tested in `frontmatter.rs`. For whole-file YAML (`.base`), a multi-document stream is parsed with **document 1 interpreted** and **every document's bytes preserved** under any edit.

**Non-scalar and unmodeled shapes.** Nested maps, block and flow sequences, literal (`|`) and folded (`>`) block scalars, and multi-line plain scalars are all *addressable and editable* by span. Any shape the model cannot represent as a span edit — the current `NotScalar` case generalized — is surfaced **read-only** with the literal state word `raw`, is fully visible in listings and Base columns, and directs the user to source editing. Nothing is ever reshaped to fit the form.

#### Schema check and migration

1. `schema check` loads the schema file (§4.2), validates it, then evaluates every rule over indexed properties, gated by B's freshness contract. Result is diagnostics only: **`schema check` never writes a note.**
2. `schema migrate KEY --to TYPE` plans a multi-file transaction. It classifies every occurrence of `KEY` in the vault into exactly one bucket, and prints the counts before any diff:
   - `already_conformant` — no write planned.
   - `coercible` — a single deterministic lexeme rewrite exists (`"3"` → `3`, `2026-01-02` → date, `yes`/`true` → checkbox per the documented table). Shown as a per-note diff.
   - `restyle_required` — coercible but needs a quoting change; shown with the reason.
   - `ambiguous` — more than one plausible reading (`01/02/2026`, `1e3`, a bare `NO` under YAML 1.1 vs 1.2, a list-of-one vs a scalar). **Never guessed.**
   - `uncoercible` — no representation in the target type.
   - `inherited` / `alias_site` / `unknown_tagged` — structurally excluded from automatic rewrite.
3. **Values are never silently dropped.** The default for any `ambiguous` or `uncoercible` occurrence is `--on-uncoercible=refuse`: the entire migration refuses, exits 4, lists every offending path, line, and raw lexeme, and writes nothing. Two explicit alternatives exist and are printed in the refusal message: `skip` leaves those notes byte-identical and records each one in the receipt and in the `schema doctor` backlog; `quarantine --quarantine-key SUFFIX` performs an **additive** edit that writes the coerced value into `KEY` *and* the original raw lexeme into `KEY.SUFFIX`, so nothing is lost. There is no option, flag, or force that deletes or overwrites an original value without preserving it.
4. **The schema file is part of the same transaction.** The edit that changes `type:` in the schema and the edits to every note commit or abort together, so the vault is never in a state where the schema claims a type the notes do not have.
5. Commit uses **H's transaction engine** unchanged: staged post-images, pre-image undo store, journal with a commit barrier, canonical step order, receipt, `--from-plan` for automation, and `schema migrate rollback RECEIPT` which restores every touched file byte-for-byte and refuses all-or-nothing if any file drifted since the commit. Confirmation escalates exactly as H specifies; `--no-input` requires `--yes` and, above the escalation threshold, `--from-plan`.
6. A migration revalidates the completeness evidence (index generation, or the digest of the sorted `(path, fingerprint)` manifest in `--rescan` mode) at the commit barrier. A note created or edited during the preview yields `generation_changed` and forces a replan; it is never silently missed.

#### Base evaluation

1. **Discover.** `base list` walks the vault for `*.base` files and for ```` ```base ```` fenced blocks inside notes, skipping `.obsidian` and `.mg-vault`, through A's confined enumeration.
2. **Parse.** The `.base` YAML is parsed with the same lossless CST used for frontmatter. Every top-level key, view key, formula, and function name is classified against the versioned **Bases profile** (§4.6) as `supported`, `display_only_unknown`, or `row_affecting_unknown`.
3. **Unsupported handling — the compatibility contract.** Unknown bytes are **always preserved verbatim**; nothing is rewritten, normalized, or dropped, ever, including on an edit to an unrelated key in the same file. Behavior then splits:
   - `display_only_unknown` (a key on the versioned display-only allowlist, e.g. an unrecognized `displayName` sibling) — rows are evaluated and shown; the key is listed in the header line `ignored for display: cardSize`.
   - `row_affecting_unknown` (anything that could change which rows or which values appear: an unknown filter function, operator, view type, filter key, or formula function) — **the affected view fails closed**: zero rows, exit 6, and the literal line `unsupported: file.hasCoverImage() at base line 7 col 12 — this view cannot be evaluated`. Returning rows from a filter the tool only partly understands is a silent wrong answer and is not permitted.
   - Unknown keys default to `row_affecting_unknown`. Promotion to display-only requires a profile-version bump plus a fixture.
4. **Freshness gate.** Base rows come from B's projections and inherit the fail-closed contract: any state other than `current` returns zero rows and exit 6 with the state named; `--allow-stale` labels the banner, every group header, every record block, and the JSON envelope, and is refused for `base materialize`.
5. **Definition load and cycle check** (§3.2 cycle safety, below) — before any row is evaluated.
6. **Row set.** Filters are compiled to bound-parameter predicates over B's typed property, tag, path, link, and task projections. A filter comparing incompatible types is `type_mismatch`, naming the observed type and one example path; it never coerces and never falls back to text.
7. **Column evaluation** in definition-topological order with a per-invocation memo cache keyed by `(index_generation, base_digest, schema_digest, note, definition)`.
8. **Sort, group, paginate**, then render (§3.3).

#### Relations, rollups, and cycle safety

**Model.** A *relation* is an ordinary property whose values are links, declared in the schema with `type: relation`, a `target`, and an optional `inverse` name. The inverse is **derived** from backlinks over that property and is **never written into the target note** — no tool in this feature adds a `children:` key to a file to make a parent link bidirectional. A *rollup* is `over` a relation (optionally a relation path with a `depth`), `of` an expression evaluated in the related note's scope, reduced by an `aggregate`.

**Two graphs, two cycle policies.**

- **Definition graph** (schema rollup → formula → schema rollup, Base formula → Base formula, `this`-referencing filters). Cycles here are configuration bugs. Detected at **load time**, before any row is evaluated, by Tarjan SCC over the definition graph. A non-trivial SCC is a hard error: `definition_cycle`, exit 4, with the full ordered path rendered as `rollup:children_cost → formula:total → rollup:children_cost` and the file plus byte range of each definition. The whole Base or schema fails closed. No view renders and no partial column is emitted.
- **Data graph** (note A relates to note B relates to note A). This is a *normal accident* in a real vault and must not take the vault down. Evaluation is depth-first with an explicit visit stack keyed by `(note_path, definition_id)`. Re-entering a key already on the stack makes that cell the typed value `Cycle { path }` — not a number, not zero, not blank. The cycle path is canonicalized by rotating it to start at its lexicographically smallest member path, so the same cycle prints identically no matter which note the traversal entered from.

**Error propagation is total.** `Cycle`, `DepthExceeded`, `BudgetExceeded`, `TypeMismatch`, and `SecretRedacted` are cell *values*, and any aggregate whose input set contains one of them is itself that value, carrying the original path or reason. There is no "sum the rest and ignore the bad ones" mode, because that emits a plausible wrong number. A user who wants the partial sum must say so per-rollup with `on_error: exclude`, which is recorded in the schema file, shown in the column header as `partial(excluding N)`, and reported in JSON as `excluded_cells`.

**Limits.** Per rollup: `max_depth` default 8, hard ceiling 32; `max_visited` default 10,000 notes per cell, ceiling 200,000; per view `max_eval_ms` default 5,000, and a global evaluation step budget. Exceeding any limit yields the corresponding typed error cell plus a header line `12 cells truncated at depth 8 — raise with --max-depth`. Limits are enforced by counters checked on every visit, never by wall-clock alone, so results stay deterministic.

**Diagnostics.** `schema doctor --check cycles` runs one whole-vault SCC pass per relation and prints every cycle as an ordered named path with the file and byte range of each offending property value, plus a *suggested but never applied* edit. Cycle detection reads only; it never breaks a cycle by editing a note.

#### Materialization — the only path from computed to written

A formula or rollup value never enters a note as a side effect of viewing. `mg-vault base materialize PATH --column NAME --into KEY` is the single explicit path: it produces an H-style plan with a complete file manifest and per-note frontmatter diffs, requires confirmation, commits atomically with per-file fingerprint preconditions, and yields a receipt that rolls back byte-for-byte. The value written is an **ordinary property with ordinary YAML** — no provenance keys, no hidden markers, and no injected identifier are added to the note, because that would be exactly the hidden metadata this product refuses. Provenance lives in the receipt (`base_digest`, `formula_digest`, `index_generation`, per-path old and new value). Drift is detected on demand by `base materialize --check`, which recomputes and reports differences without writing. Materialization is refused entirely when freshness is not `current`, when any cell is `Cycle`/`DepthExceeded`/`BudgetExceeded`/`SecretRedacted`, or when the target key is `inherited` or an alias site.

### 3.3 Layout descriptions

**Property listing** (`property list PATH`) — reading order: file path and fingerprint; then one block per property in **source order** (never alphabetized, because source order is user information); then a footer of counts.

```
notes/projects/alpha.md  sha256:9f2c…
  status      enum      active            plain          line 3
  due         date      2026-09-14        plain          line 4
  tags        list      [rust, vault]     flow-sequence  line 5
  summary     text      (block scalar, 4 lines)  literal  lines 6-10   raw
  legacy_id   raw       *base_id          alias          line 11       not editable in form
5 properties, 1 not editable in the form; schema: mg-vault.schema.yaml@a91c…
```

Columns: key, declared type (or `untyped` / `raw`), a bounded value preview, YAML style, and location. `raw` and `not editable in form` are literal words, never color or a glyph.

**Base table view.** Reading order: freshness banner; base and view identity with digests; unsupported/ignored line if any; column header; group headers and rows; footer with counts, truncation notices, and a repeat of the freshness state.

```
Index: current  generation 41  observed 2026-08-29T18:04:11Z
Base: projects.base  view "Active"  profile bases/2026.07  schema a91c…
ignored for display: cardSize
 name              status   due          children  open   parent
 alpha             active   2026-09-14         3     11   moc/projects.md
 beta              active   2026-10-01         0      0   —
 gamma             blocked  —                  2  cycle   projects/beta.md
3 rows · 1 cell cycle (gamma.open: beta → gamma → beta) · sorted by due ASC, path ASC
```

**Column model and width algorithm** (deterministic, must produce identical output for identical input and width):

1. Column order = the view's `order:` if present, else schema declaration order, else key byte order. The first column is always the row identity (`file.name` unless the Base overrides it) and is never dropped.
2. Alignment is driven by declared type: text left, number right and decimal-aligned, date left in ISO-8601, checkbox as `[x]` / `[ ]`, link as display text plus a state marker, list as comma-joined with a bounded preview.
3. Natural width = max grapheme display width (`unicode-width`, grapheme clusters, East Asian wide and combining marks accounted) of the header and of every cell **on the current page**, capped at `--max-col-width` (default 40).
4. If the sum plus separators fits the terminal, use natural widths.
5. Otherwise run a deterministic waterfall shrink: every column has a floor of `min(natural, 6)`; remove one display cell at a time from the currently widest column (ties broken by rightmost column order) until it fits or all columns are at their floor.
6. If floors still do not fit, drop columns from the **right** of the order, never the first, and print the literal line `columns hidden: rating, owner (width 52 < required 78) — use --view records or --json`.
7. Truncated cells end with `…` (`...` under `--ascii`). The full value is always reachable in `--view records`, `--json`, and by focusing the cell in the TUI.
8. Below 60 columns, or with `--view records`, or when `--view cards` is requested (a terminal has no image grid), the table is replaced by one labeled record block per row containing **every** column including any that a table would have hidden.

**Group headers** print the literal group value and count; a missing value is the literal group `(no value)` placed last. Sorting is total: the view's `sort` list, then vault-relative path byte order as the final tie-break. Missing values sort last within a direction, and the footer says so whenever any sorted column has missing values. Error cells sort after all real values, ordered `cycle` < `depth_exceeded` < `budget_exceeded` < `type_mismatch`.

**Migration plan preview** reuses H's five ordered blocks verbatim: header (operation, key, old and new type, evidence string), complete file manifest (every path, never elided), per-note frontmatter diffs (`--diff-limit` controls how many *bodies* print, never how many paths are listed), classification lists (`coercible` / `restyle_required` / `ambiguous` / `uncoercible` / `skipped`), and the precondition block.

**Data sources.** Property views read source bytes through `Vault::read_note` and the CST index — never the index. Base rows come from B's typed projections through the IPC client, with source re-read on open. Schema definitions come from the schema file in the vault. Nothing in this feature reads note bytes out of SQLite.

**Empty states.** `property list` on a note without frontmatter prints `No frontmatter in PATH. Add one with: mg-vault property set PATH key value`. `schema show` with no schema file prints `No schema at mg-vault.schema.yaml. Propose one with: mg-vault schema infer`. `base list` with none prints `No .base files or base blocks in this vault.` A Base whose filters match nothing prints `0 rows in indexed generation 41.` — and, when freshness is not current, explicitly does **not** imply that no source rows exist.

### 3.4 Input & gestures

There is no touch, stylus, controller, voice, or camera input in this feature; it is keyboard and stdin only.

**CLI grammar.** `property {list,get,set,add,remove,rename}`; `schema {show,check,infer,migrate,doctor,import,export}`; `base {list,run,check,materialize,new}`. Value grammar for `property set`: the argument is parsed as a literal of the property's declared type using that type's documented parser; `--type T` forces a type for this edit; `--raw` passes an exact YAML lexeme through, validated by reparse and refused if it changes the node's shape. List operations use `--append VALUE`, `--remove VALUE`, `--at N`, and `--clear`. Duplicate keys require `--occurrence N`. Shared flags on every command: `--vault NAME`, `--json`, `--jsonl`, `--dry-run`, `--yes`, `--from-plan FILE`, `--plan-out FILE`, `--no-input`, `--no-color`, `--ascii`, `--width N`, `--allow-stale`, `--deadline-ms N`, `--quiet`. `NO_COLOR` in the environment equals `--no-color` and beats auto-detection. Content can be piped: `property set --value-from-stdin` reads a value from stdin; `base run --json` is pipeable and never emits ANSI to a non-TTY.

**TUI property form** (registered as E actions, bindings rebindable there): `j`/`k` move row, `g g`/`G` first/last, `Enter` edit the focused value inline, `r` rename key, `a` add key, `d d` remove key (with confirmation escalated for a key that owns comments), `t` change declared type for this note's value only, `u` undo through D's persistent undo, `e` open the raw frontmatter in the editor buffer, `y` yank the raw lexeme, `?` help. A `raw` row refuses `Enter` with the spoken line `not editable in the form — press e to edit source`.

**TUI Base pane:** `h`/`l` move column, `j`/`k` move row, `s` toggle sort direction on the focused column, `S` add the focused column as a secondary sort, `g` group by focused column, `/` filter, `f` open the filter editor, `Enter` open the focused row's note, `x` expand the focused cell into the detail strip (full value, type, state, cycle path), `J` dump the current view as JSON to stdout, `R` re-evaluate. Every action is keyboard-reachable and appears in E's command palette with the same name; there is no mouse-only affordance.

**Responsive behavior** is by terminal width, not screen class: ≥ 100 columns full table; 60–99 columns table with the waterfall shrink and possible right-side column hiding; < 60 columns automatic record view. `--width N` forces a width for reproducible output and testing; piping to a non-TTY uses `--width` or an unbounded record view rather than guessing 80.

### 3.5 Transitions & animation

There is no animation, easing, spinner, sound, or haptic in this feature, and none may be added — a schema migration and a Base render are static output. Two motion-adjacent behaviors exist and are bounded:

- A long `schema check`, `schema migrate --rescan`, or whole-vault `schema doctor --check cycles` may emit static, rate-limited (≤ 4/s) progress lines to **stderr**, in interactive human mode only: `scanned 41,200 / 100,000 notes`. Suppressed under `--quiet`, `--json`, `--jsonl`, `--no-input`, non-TTY stderr, `NO_COLOR`, and any reduced-motion environment variable; in those modes progress is emitted once per 10% as a plain appended line or not at all. No dynamic cursor addressing, no redraw-in-place.
- TUI pane transitions are E's: an instantaneous repaint with no easing. Scrolling a Base table is a redraw, not a scroll animation. Because there is no motion, reduced-motion configuration changes nothing here, which is itself the reduced-motion alternative.

### 3.6 Error states

| Trigger | Code | Presentation | Recovery | Data loss risk |
|---|---|---|---|---|
| BOM or unclosed frontmatter | `frontmatter_ambiguous` | Inline error; structural edits refused, the file still reads and renders | Edit source directly; the file is never rewritten | No |
| Invalid YAML in frontmatter | `yaml_invalid` | Inline error with the parser message and byte offset | Fix in the editor; property commands refuse until valid | No |
| Duplicate top-level key | `duplicate_key` | Inline; every occurrence listed with line/column/byte range | Re-run with `--occurrence N` | No |
| Target is an alias node | `yaml_alias_target` | Inline; names the anchor and its location | Edit the anchor, or `--break-alias` after a preview | No |
| Target arrives via merge key | `property_inherited` | Inline; names the anchor's file and offset | `--materialize-inherited` adds a shadowing key additively | No |
| Comment ownership ambiguous on removal | `comment_ownership_ambiguous` | Inline; shows the comment run and both candidate owners | Remove the comment manually, or use `--keep-comments` | No |
| Post-edit verification mismatch | `edit_verification_failed` | Inline; names which invariant failed (gap bytes, style, comment token, key order) | Nothing written; report as a bug with the fixture | No |
| Fingerprint changed since read | `conflict` (existing `Error::Conflict`) | Inline; prints expected and actual digests | Re-read and replan; both versions intact on disk | No |
| Value fails schema type | `schema_violation` | Diagnostic record in `schema check`, inline on a set | Fix the value, or widen the type; never auto-corrected | No |
| Migration has uncoercible/ambiguous values | `migration_blocked` | Complete list of paths, lines, and raw lexemes; exit 4 | `--on-uncoercible=skip` or `quarantine`; never a destructive force | No — nothing written |
| Vault changed during a migration preview | `generation_changed` | Inline; names the generation or manifest digest that moved | Replan | No |
| Migration interrupted | `transaction_incomplete` | Transaction ID, durable step count, literal `state: incomplete`, recovery command | `refactor recover` semantics; converges to all-or-nothing | No if the protocol holds |
| Rollback target drifted | `rollback_blocked` | Every drifted path listed; nothing written | Materialize pre-images into `recovered/` and reconcile | No |
| Base file unparseable | `base_invalid` | Byte span and expected form; file untouched | Fix in the editor | No |
| Base uses a row-affecting unknown feature | `unsupported_feature` | Header line plus zero rows, exit 6; feature named with byte range | Use Obsidian for that view, or wait for profile support | No — file preserved verbatim |
| Definition cycle in schema or Base | `definition_cycle` | Full ordered definition path with file and byte ranges; whole view fails closed, exit 4 | Break the definition cycle | No |
| Data cycle in a relation | cell state `cycle` | The literal word `cycle` in the cell; canonical ordered path in the footer, detail strip, and JSON | `schema doctor --check cycles`; edit a note to break it | No |
| Depth or visit budget exceeded | cell state `depth_exceeded` / `budget_exceeded` | Typed error cell plus a footer count | Raise `--max-depth` / `--max-visited`, or narrow the rollup | No |
| Type mismatch in a filter or sort | `type_mismatch` | Names observed type and one example path; no rows returned | Fix the filter or the schema | No |
| Freshness not current | `stale_index` | Zero rows, exit 6, state word, generation, refresh command | `index rebuild`, or explicit `--allow-stale` | No |
| Confirmation required under `--no-input` | `confirmation_required` | Exit 2 with the exact flags needed | Add `--yes` (and `--from-plan` above the escalation threshold) | No |
| Secret property requested | `secret_redacted` | Cell renders the literal word `redacted`; JSON state `secret_redacted`, no value | Remove `secret: true` if intended | No |

No message uses a success verb (`set`, `migrated`, `materialized`, `rolled back`, `durable`) unless the corresponding durable record exists. Error JSON carries `version`, `ok:false`, `error.code`, `error.message`, `error.details`, `retryable`, and `recovery`, and never contains note body bytes, secret values, environment values, or absolute paths outside the vault.

### 3.7 Accessibility

- **Screen reader.** All output is linear terminal text; the screen reader reads what the terminal holds. Every property row, table row, group header, cell state, and diagnostic is a self-describing line: `gamma  open  cycle (beta → gamma → beta)` rather than a symbol. Table cells that a sighted user reads by column position are re-labeled in `--view records`, which is the announced equivalent and is auto-selected below 60 columns.
- **Labels, hints, traits for interactive elements.** Every TUI pane element has a spoken label registered with E's accessibility layer: the property form announces `row 3 of 5, key due, type date, value 2026-09-14, editable`; a `raw` row announces `not editable in the form, press e to edit source`; the Base pane announces `column 4 of 6, open, type number, cell state cycle` plus the cycle path on focus. Focus changes announce the new position and its state word.
- **Custom actions.** Complex interactions expose named actions rather than requiring gesture chords: on a Base cell — `open note`, `expand cell`, `copy value`, `explain cycle`; on a property row — `edit`, `rename`, `remove`, `edit source`, `show diagnostics`. Each appears in E's command palette by name and can be invoked without knowing a binding.
- **Text scaling / dynamic type.** The terminal analogue is font size and terminal width; everything reflows through the §3.3 width algorithm, and no layout assumes a fixed 80 columns. Nothing is positioned by absolute cursor coordinates, so a resize is a clean re-render.
- **Color-independent state.** Every state is a literal word — `active`, `raw`, `cycle`, `redacted`, `stale`, `unsupported`, `inherited`, `restyled`, `skipped`, `truncated`. Color and glyphs are decoration only; stripping ANSI removes no information, which is asserted by a test that diffs colored and `NO_COLOR` output after ANSI removal. Checkboxes render `[x]` / `[ ]`, never a colored dot. Diff lines are distinguished by leading `+` / `-`.
- **Focus order and keyboard navigability.** Focus order in the property form is source order of the properties, then the diagnostics strip. In the Base pane it is row-major: row, then column, then the detail strip. Every command in §3.4 is reachable by keyboard, and no capability exists that only a mouse can reach. Tab order is stable across a re-render; after a re-evaluation focus returns to the same `(row identity, column key)`, or to the nearest surviving row when it disappeared, and announces the change.
- **Unicode and RTL.** Keys, values, paths, and group labels may be any Unicode; widths use grapheme clusters and `unicode-width`, and bidirectional control characters in any cell are escaped before display so a table cannot be visually reordered by content.

---

## 4. Implementation Specification

### 4.1 Architecture placement

- **New crate `crates/mg-vault-props`** — the whole of this feature's logic, with no filesystem policy of its own. Modules: `yaml/{model.rs,index.rs,edit.rs,style.rs,comments.rs,verify.rs}`; `schema/{model.rs,load.rs,check.rs,infer.rs,migrate.rs}`; `base/{parse.rs,profile.rs,filter.rs,formula.rs,eval.rs,view.rs,unsupported.rs}`; `relation/{graph.rs,rollup.rs,cycle.rs}`; `render/{table.rs,records.rs,json.rs}`. It takes bytes plus a confined reader capability; it never opens an absolute path, spawns a process, or opens a socket.
- **One YAML adapter in the workspace, not two.** `crates/mg-vault-core/src/frontmatter_scalar.rs` is generalized behind that adapter. **F** (`specs/f-markdown-rich-content.md` §4.1, §7.2) already claims the *read-side* YAML span adapter inside `mg-vault-markdown`; **I** owns the *write-side* model (`yaml/edit.rs`, `yaml/style.rs`, `yaml/comments.rs`, `yaml/verify.rs`) and the CST index that F's adapter also consumes. Whichever lands first exports the type; the other depends on it. Two YAML dialects in this workspace is a defect, not a design choice.
- **`crates/mg-vault-core`** gains exactly one new primitive: `Vault::edit_note_spans` — a multi-span, disjoint, single-fingerprint, single-`replace_atomic` splice, which the spike already mandates ("one plan, one fingerprint check, overlap rejection, and one atomic replacement"). Core remains the sole filesystem and durability authority.
- **`crates/mg-vault-cli`** gains `commands/{property,schema,base}.rs` and `output/{property,schema,base_table,base_records}.rs`. It owns no policy.
- **`crates/mg-vault-index` (B)** consumes I's type resolution to fill `PropertyProjection.value_type` / `canonical_value` and to store relation edges. The index is a disposable view: it holds no schema authority, and deleting it and rebuilding from unchanged source must reproduce every table byte-identically.
- **`crates/mg-vault-transaction` / `mg-vault-core::transaction` (H)** is reused unchanged for migration and materialization. I defines no second transaction engine.
- **E** hosts the three TUI panes and consumes I's render models through B's IPC client; it links neither `rusqlite` nor `mg-vault-props`' evaluation internals.

### 4.2 Data model

Schema file — an **ordinary YAML file at the vault root**, `mg-vault.schema.yaml`, versioned, diffable, syncable, and readable in any editor. It is content, not app settings, so it does **not** live under `.mg-vault`; only its optional path override does.

```yaml
version: 1
scopes:
  - match: "projects/**"
    require: [status, due]
properties:
  status:  { type: enum, values: [inbox, active, blocked, done], default_missing: untyped }
  due:     { type: date, format: civil }
  rating:  { type: number, min: 0, max: 5 }
  tags:    { type: list, of: text }
  owner:   { type: link }
  api_key: { type: text, secret: true }
relations:
  parent:  { type: relation, target: link, inverse: children }
rollups:
  open_children_tasks:
    over: children
    depth: 3
    of: 'file.tasks.where(state == "todo").length'
    aggregate: sum
    max_depth: 8
    on_error: propagate      # propagate | exclude
```

```rust
/// A declared property type. Obsidian's five types plus mg-vault refinements
/// that map down to them for interop; never used to coerce source bytes.
pub enum PropertyType {
    Text, List, Number, Checkbox, Date, DateTime,          // Obsidian-compatible core
    Enum { values: Vec<String> }, Link, ListOf(Box<PropertyType>), Duration,
    Relation { target: Box<PropertyType>, inverse: Option<String> },
    Untyped,                                               // no schema entry: kept as written
    UnknownTagged { tag: String },                         // explicit YAML tag we do not model
    Raw,                                                   // a shape the form cannot edit
}

/// One node in the lossless frontmatter/`.base` CST index. Byte ranges are
/// absolute file offsets, not YAML-slice offsets.
pub struct YamlNode {
    pub path: YamlPath,                 // key path with occurrence ordinals
    pub key_range: Option<Range<usize>>,
    pub value_range: Range<usize>,
    pub style: ScalarStyle,             // Plain|Single|Double|Literal|Folded|FlowSeq|FlowMap|BlockSeq|BlockMap
    pub anchor: Option<String>,
    pub is_alias: bool,
    pub tag: Option<String>,
    pub indent_col: u16,
    pub sibling_ordinal: u32,
    pub inherited_from: Option<String>, // merge-key anchor name
}

/// A comment token and who owns it. Ownership decides removal behavior.
pub struct CommentToken { pub range: Range<usize>, pub owner: CommentOwner }
pub enum CommentOwner { TrailingOn(YamlPath), LeadingOf(YamlPath), Floating, Ambiguous }

/// One planned mutation. Always expressed as disjoint byte spans over the
/// original file; there is no serializer anywhere in this crate.
pub struct PropertyEdit {
    pub note: NotePath,
    pub expected: SourceFingerprint,
    pub spans: Vec<(Range<usize>, Vec<u8>)>,   // validated disjoint, ascending
    pub restyled: Vec<RestyleNotice>,
    pub comments_moved: Vec<Range<usize>>,
}

/// A parsed `.base` file or embedded ```base block. Unknown bytes are retained.
pub struct BaseDocument {
    pub source: BaseSource,             // File(NotePath) | Embedded { note, block_ordinal }
    pub digest: [u8; 32],
    pub filters: Option<FilterNode>,
    pub formulas: Vec<(String, Expr)>,
    pub properties: Vec<ColumnConfig>,
    pub views: Vec<BaseView>,
    pub unsupported: Vec<UnsupportedFeature>,   // byte range + class + name
    pub extra_documents: Vec<Range<usize>>,     // preserved verbatim, never interpreted
}
pub enum UnsupportedClass { DisplayOnly, RowAffecting }

/// One evaluated cell. `state` is never inferred from an empty value.
pub struct Cell { pub value: Option<TypedValue>, pub declared: PropertyType, pub state: CellState }
pub enum CellState {
    Ok, Missing, Inherited, TypeMismatch { expected: PropertyType, found: PropertyType },
    Cycle { path: Vec<NotePath> },              // canonical rotation, smallest path first
    DepthExceeded { limit: u32, via: Vec<NotePath> },
    BudgetExceeded { budget: &'static str },
    Unsupported { feature: String }, SecretRedacted, Raw,
}

/// Stamped on every schema, Base, and property response.
pub struct DerivationProvenance {
    pub freshness: Freshness, pub index_generation: Option<u64>,
    pub schema_digest: Option<[u8; 32]>, pub base_digest: Option<[u8; 32]>,
    pub bases_profile_version: &'static str, pub yaml_model_version: &'static str,
    pub complete: bool, pub derived_from: &'static str,   // always "ordinary_files"
}
```

**Database migrations.** I adds no durable store of its own. B's SQLite gains typed property columns, a `relations(from_source_id, property, ordinal, raw_target)` table, and a `bases(source_id, digest, profile_version)` discovery pointer — all disposable, all rebuildable, all requiring a `SCHEMA_VERSION` bump and B's side-by-side rebuild (never in-place mutation of a published generation). No table ever stores an identifier that appears in a file; no UUID is injected into any note (criterion 1C). Rollup and formula results are **never** persisted as authority: the memo cache is per-invocation and keyed by generation and digests.

### 4.3 API contracts

```rust
// mg-vault-core (new): the one multi-span atomic primitive.
pub fn edit_note_spans(&self, relative: impl AsRef<Path>, spans: &[(Range<usize>, Vec<u8>)],
                       expected: &SourceFingerprint) -> Result<SourceFingerprint>;
// Errors: UnsafePath, Conflict, InvalidUtf8, InvalidEditSpan, OverlappingSpans, Io.

// mg-vault-props: YAML model and edit planning.
pub fn index_yaml(source: &str, region: Range<usize>) -> Result<YamlIndex, YamlError>;
pub fn plan_property_edit(source: &str, op: PropertyOp, schema: Option<&Schema>)
    -> Result<PropertyEdit, PropertyError>;
pub fn verify_edit(before: &str, after: &str, plan: &PropertyEdit) -> Result<(), VerifyError>;

// Schema.
pub fn load_schema(vault: &Vault) -> Result<Option<Schema>, SchemaError>;
pub fn check_schema(schema: &Schema, rows: &PropertyRows) -> Vec<Diagnostic>;
pub fn plan_migration(vault: &Vault, key: &str, to: PropertyType, policy: UncoerciblePolicy)
    -> Result<MigrationPlan, MigrationError>;

// Bases.
pub fn parse_base(source: &str, profile: &BasesProfile) -> Result<BaseDocument, BaseError>;
pub fn evaluate(base: &BaseDocument, view: &str, ctx: &EvalContext) -> Result<ResultTable, EvalError>;
```

**CLI signatures** (each returns C's `version: 1` envelope under `--json` and accepts the §3.4 shared flags):

```text
mg-vault property list PATH                                        -> PropertyRecords
mg-vault property get PATH KEY [--occurrence N]                    -> PropertyRecord
mg-vault property set PATH KEY VALUE [--type T|--raw] [--occurrence N] [--dry-run]
mg-vault property add PATH KEY VALUE [--after KEY|--at-end]
mg-vault property remove PATH KEY [--occurrence N] [--keep-comments]
mg-vault property rename PATH OLD NEW [--occurrence N]
mg-vault schema show | check [--path P] | infer [--sample N] [--write]
mg-vault schema migrate KEY --to TYPE [--on-uncoercible refuse|skip|quarantine] [--rescan]
mg-vault schema migrate history | rollback RECEIPT
mg-vault schema doctor --check conformance|duplicates|inherited|relations|cycles
mg-vault schema import --from-obsidian | export --to-obsidian --out FILE
mg-vault base list | check PATH | run PATH [--view NAME] [--view-kind table|records]
mg-vault base run PATH --explain | --json | --jsonl | --format csv
mg-vault base materialize PATH --column NAME --into KEY [--check]
```

**Normative JSON `data` objects.** `base run` emits `{provenance, base:{path,digest,profile_version,unsupported:[{name,class,byte_range}]}, view:{name,kind,requested_columns,effective_columns,hidden_columns,sort,group_by}, columns:[{key,display_name,declared_type,source:"property"|"formula"|"rollup"|"file",alignment}], groups:[{value,count}], rows:[{path,cells:{KEY:{value,type,state,cycle_path?,reason?}}}], truncated:{cells,reason}, complete}`. `--jsonl` streams one row object per line after a header object. **Conformance requirement (5C):** for every fixture and every terminal width, the set of `(row, column, value, state)` tuples in `--json` is a strict superset of everything any rendering displays, and the record view is exactly equal to it — asserted by test, so a hidden column is never information loss.

**Errors, auth, limits.** Exit codes follow C's stable categories exactly and introduce no new category: `0` success or no-op, `2` usage/confirmation, `3` not found, `4` collision/ambiguity/conflict/`definition_cycle`/`migration_blocked`, `5` unsafe or denied, `6` degraded dependency (stale index, `unsupported_feature`), `7` I/O or transaction failure, `130` interrupt. Auth is B's: local per-user socket, peer-UID check, no TCP. Pagination on `base run` uses B's opaque generation-bound cursor (page 50, max 500); a generation change returns `cursor_expired` rather than mixing pages. Plugins and AI reach schema or Base evaluation only through **N/O** explicit, scoped, previewed, attributable capability grants, and may propose but never execute a mutation.

### 4.4 State management

- **Authoritative state is the files**: note frontmatter, `mg-vault.schema.yaml`, and `.base` files. Every one is an ordinary file a user can edit, sync, diff, and delete with no tool involvement, and the vault stays fully usable if `mg-vault` is uninstalled.
- **Derived state** — typed property projections, relation edges, inverse relations, formula and rollup results, Base row sets — is owned by B's per-vault index actor and I's per-invocation evaluator. All of it is disposable. Inverse relations and rollups exist **only** in memory and in the response; nothing writes them back.
- **The memo cache** is per invocation (or per TUI pane refresh), bounded at 250,000 cells / 64 MiB with LRU eviction, and keyed by `(index_generation, base_digest, schema_digest)` so it cannot outlive the inputs that produced it. Dropping it costs only recomputation.
- **View state** — focused row and column, sort, group, filter, scroll — is owned by the CLI invocation or by E's pane, is session-restorable by `(base path, view name, row identity, column key)`, and reports an explicit unresolved entry rather than guessing when a restored row no longer exists.
- **Local only.** There is no server, no sync, and no remote component in this feature.
- **Draft/offline persistence.** The TUI property form keeps an in-progress cell edit in memory and, via D's persistent-undo store under XDG state, across a crash; it is never written into the vault, never into `.obsidian`, and is excluded from indexes and exports. A recovered draft is presented as a proposal against the *current* file bytes and is refused if the fingerprint moved, so recovery can never overwrite newer source. The only bytes this feature writes into a vault are property edits, schema-file edits, and explicit materializations, each through core's fingerprint-checked atomic transaction.

### 4.5 Dependencies

- **`yaml-edit 0.2.3`, `default-features = false`, is already a workspace dependency** in `vault/Cargo.toml` and is used exactly as the spike prescribes: a lossless CST that yields byte ranges, behind a project-owned adapter, **never** as a serializer. It is in the workspace today precisely because `frontmatter_scalar.rs` needs YAML validation and typing without rewriting the document, and this feature is the reason that constraint was accepted early.
- `serde_yaml` is **rejected** for any source path (loses comments, quoting, order, whitespace; unmaintained). Any whole-document YAML or Markdown serializer is prohibited by the spike and by an API-surface golden test (§5.1).
- New: `unicode-width` and `unicode-segmentation` (terminal column math and grapheme-safe truncation; MIT/Apache-2.0) — shared with F/E, added once. `jiff` or `time` for civil dates and instants with an explicit timezone policy (MIT/Apache-2.0). No regex engine of I's own; bounded regex predicates come from G.
- Dev-only: `yaml-rust2` as a **differential oracle** (semantic value agreement after every edit) and `proptest` for the prefix/suffix/gap invariants. Neither is in the mutation path.
- Rejected: any expression-language crate that can evaluate arbitrary code, any embedded scripting runtime, and any dependency that performs I/O during evaluation. The formula interpreter is project-owned, pure, total, with no host functions, no file writes, no process spawn, no network, and a step and allocation budget.
- New assets: a versioned Bases fixture corpus and an adversarial YAML corpus under `crates/mg-vault-props/fixtures/` (§6.2). Infrastructure changes: none — no database of I's own, no CDN, no third-party service, no network egress of any kind.

### 4.6 Platform-specific considerations

- **Bases profile versioning.** Compatibility is a **fixture-tested contract**, not a claim. `crates/mg-vault-props/fixtures/bases/<profile-version>/` holds `.base` inputs, a fixture vault, and expected result JSON, each pinned to a documented Obsidian release. `bases_profile_version` is reported in every response and in `base check`. Adding a function or view type requires a profile bump plus fixtures; behavior may not change silently under a user. A `.base` written by a newer Obsidian than the pinned profile is not an error — its unknown features are classified per §3.2 and its bytes are preserved.
- **`.obsidian` coexistence (1D).** Obsidian stores property type assignments in `.obsidian/types.json`. `mg-vault` **reads** it as an optional compatibility input for `schema infer` and **never writes it** — the existing core rule that rejects any mutation whose first path component is `.obsidian` or `.mg-vault` stands unchanged. `schema export --to-obsidian --out FILE` writes a candidate document outside `.obsidian` (or to stdout) for the user to install manually. Portable app settings (schema path override, default view kind, width preferences) live under `.mg-vault`; the schema itself is vault content.
- **Line endings and encoding.** `LineEndings::{Lf, CrLf, Mixed}` (implemented in `frontmatter.rs`) is observed and preserved, never normalized, including for inserted keys, which adopt the file's observed ending and the mapping's observed indent unit. A UTF-8 BOM before frontmatter remains a fail-closed refusal for structural edits.
- **Case-insensitive and normalization-varying filesystems.** Property keys are compared byte-exactly; `Status` and `status` are two different keys and a note carrying both gets a `duplicate_key`-adjacent diagnostic (`key_case_collision`) rather than a merge. Path-valued properties inherit G's byte-exact identity rule with `near_miss` reporting.
- **YAML version.** The model targets YAML 1.2 core schema; YAML 1.1 booleans (`yes`, `no`, `on`, `off`) are **not** silently coerced — they are `ambiguous` in migration and are reported by `schema doctor`, because Obsidian and various YAML libraries disagree here and guessing would produce a silent wrong value. The chosen resolution is documented and fixture-pinned.
- **Feature flags.** `relations`, `rollups`, `base-materialize`, and `base-cards` may ship gated. Preservation semantics, cycle fail-closed behavior, freshness fail-closed behavior, and the "never silently drop a value" migration rule are **never** feature-gated.
- **Version compatibility.** `yaml_model_version` and `bases_profile_version` participate in B's protocol negotiation; a client and service that disagree get `protocol_incompatible` rather than mixed semantics.

### 4.7 Performance budget

Baseline corpus is B's: 100,000 notes, 1,000,000 blocks, ~2,000,000 property/tag/task/link projections, on recorded acceptance hardware. These are acceptance budgets, not measured claims.

- **Editing is never blocked.** All schema, Base, formula, relation, and rollup work is read-only and off the save path. During a whole-vault `schema check` or `schema doctor --check cycles`, p99 added latency to direct `note read`/`note write` stays within B's ≤ 20 ms allowance. No save waits on evaluation.
- **Single-note property work** is O(frontmatter size), independent of vault size: parse + index + plan + verify + atomic write p95 ≤ 8 ms, p99 ≤ 25 ms for frontmatter under 8 KiB; a 1 MiB pathological frontmatter stays under 250 ms and is bounded, not unbounded.
- **Base evaluation, warm:** a filter over indexed typed properties producing a 50-row first page p95 ≤ 150 ms, p99 ≤ 400 ms; scalar formula evaluation ≤ 20 µs/cell p95; a rollup at depth ≤ 3 over ≤ 200 related notes per row, for a 50-row page, p95 ≤ 400 ms, p99 ≤ 1.2 s. **Cold** (service just started): first page p95 ≤ 1 s, reported in `--explain` so a slow first render is explicable.
- **Whole-vault passes:** `schema check` p95 ≤ 90 s and cancellable; `schema doctor --check cycles` (one SCC pass per relation) p95 ≤ 60 s; migration preview over 20,000 affected notes ≤ 60 s; migration commit ≤ 180 s with progress on stderr.
- **Cancellation:** `Ctrl-C`, TUI `Esc`, or an expired `--deadline-ms` returns `cancelled` / `deadline_exceeded` within p95 ≤ 100 ms of the signal, with `complete: false`, no partial page treated as final, and — for a migration — nothing written before the commit barrier.
- **Memory:** I adds ≤ 96 MiB steady-state above B's ≤ 256 MiB target: memo cache ≤ 64 MiB (bounded, LRU), CST indexes held only for open notes and released on close, relation adjacency streamed from B rather than materialized whole. Peak during a 20,000-note migration ≤ 192 MiB, because post-images are staged to disk, not held in memory.
- **Storage:** typed property columns, relation edges, and Base discovery pointers target ≤ 0.25× the existing projection size, inside B's ≤ 2.5× total index budget. Migration staging and undo pre-images are bounded by the affected file set and are garbage-collected by H's retention policy.
- **Network:** zero bytes. Every workflow here is fully offline.
- **Startup:** no I structure is loaded eagerly; the schema file is read on first schema/Base use and cached by digest, so B's ≤ 150 ms IPC-health budget is unaffected.

---

## 5. Test Specification

### 5.1 Unit tests

| Test | Setup and assertion | Edge covered |
|---|---|---|
| `scalar_edit_preserves_everything_else` | Property test over the adversarial corpus: for every fixture and every locatable scalar, splice a replacement; assert prefix, suffix, and every inter-span gap are byte-identical | 1B core invariant |
| `comments_key_order_and_quoting_survive` | Frontmatter with leading, trailing, and floating comments, all five scalar styles, and a specific key order; edit one unrelated key; assert every comment token, every sibling ordinal, and every style is unchanged | 1B, the decisive criterion |
| `crlf_and_mixed_endings_are_preserved` | Edit a value in CRLF and in mixed-ending frontmatter, and insert a new key into each; assert the inserted line adopts the observed ending and no other line changes | `LineEndings` preservation |
| `anchors_aliases_and_merge_keys` | Edit an anchored node (anchor survives, alias sites listed); edit an alias node (`yaml_alias_target`, nothing written); write an inherited key (`property_inherited`); `--materialize-inherited` adds without removing | Anchor/alias/merge semantics |
| `explicit_tags_and_directives_survive` | `!!str`, `!custom`, `%YAML 1.2`, `%TAG`, and a `...` end marker; edit an unrelated key; assert byte equality outside the span and `unknown_tagged` typing | Unknown syntax loss (auto-fail) |
| `block_and_flow_shapes_are_addressable_or_raw` | Literal, folded, multi-line plain, flow map, flow seq, block seq; assert each is either span-editable or reported `raw`, never reshaped | Generalizes today's `NotScalar` |
| `duplicate_key_requires_occurrence` | Two `title:` entries; assert `DuplicateKey` with every occurrence's line/column/byte range, and that `--occurrence 2` edits only the second | Existing behavior, extended |
| `comment_ownership_matrix` | Trailing, leading-attached, blank-separated, first-entry, last-entry, and both-sides-blank comment runs; assert removal keeps floating comments and refuses the ambiguous case | Comment ownership rule |
| `restyle_only_when_meaning_changes` | New values `true`, `2026-01-02`, `a: b`, `x #y`, ` lead`, `line\nline`; assert restyle happens exactly when required and is always reported | No silent restyling |
| `verify_edit_catches_injected_corruption` | Mutate the candidate bytes outside the plan before verification; assert `edit_verification_failed` and no write | Defence in depth |
| `edit_note_spans_rejects_overlap_and_stale` | Overlapping spans, non-boundary spans, wrong fingerprint | 1E fail-closed |
| `migration_classification_matrix` | `"3"`, `3`, `01/02/2026`, `1e3`, `yes`, `[a]`, `*alias`, `!custom v`, missing | Each lands in exactly one bucket |
| `migration_never_drops_a_value` | Property test: for every policy (`refuse`/`skip`/`quarantine`), assert the multiset of original lexemes is recoverable from the post-state plus the receipt | Source-content loss (auto-fail) |
| `base_parse_preserves_unknown_bytes` | `.base` with unknown keys, unknown functions, unknown view types, and a second YAML document; edit a known key; assert byte equality elsewhere | Bases compatibility contract |
| `unsupported_is_classified_and_fails_closed` | Row-affecting unknown → zero rows + `unsupported_feature`; display-only unknown → rows + ignored line | Never half-understood filtering |
| `definition_cycle_fails_at_load` | Rollup → formula → rollup; assert `definition_cycle` with the ordered path and byte ranges, and zero rows rendered | Cycle safety |
| `data_cycle_yields_typed_cell_not_a_number` | A ↔ B parents; assert the cell is `Cycle`, the path is canonically rotated, no numeric value is emitted, and 100 runs from different entry points print identically | Cycle safety, determinism |
| `error_cells_propagate_through_aggregates` | A `Cycle` leaf under `sum`; assert the parent is `Cycle`, not a partial sum; then `on_error: exclude` labels `partial(excluding N)` | No silent partial value |
| `depth_and_visit_budgets_are_deterministic` | A 40-deep chain and a 300,000-node fan-out; assert `DepthExceeded`/`BudgetExceeded` at exactly the documented counts, not by wall clock | Determinism under limits |
| `column_width_algorithm_is_deterministic` | Widths 40/60/80/120 with CJK, emoji, combining marks, RTL; assert identical output for identical input and that column 1 is never dropped | 5D |
| `no_serializer_exists` | API-surface golden test: no `Display`, `to_string`, `serialize`, or `write_yaml` on any CST or document type | Prohibited by the spike |
| `terminal_text_is_escaped` | ESC, bidi override, and newline inside a key, value, group label, and cycle path | Terminal injection |
| `secret_properties_never_appear` | `secret: true` key; assert absent from cells, JSON, `--explain`, logs, diagnostics, and materialization | 4F |

### 5.2 Integration tests

- **Round-trip corpus gate (the 1B acceptance gate).** For every fixture in the adversarial YAML corpus, for every addressable node: apply an edit, then assert (a) prefix/suffix/gap byte equality, (b) `YamlIndex` equality on key paths, ordinals, styles, anchors, tags, and comment tokens except the declared change, and (c) `yaml-rust2` semantic agreement. Any disagreement is a hard failure requiring an explicit documented divergence entry.
- **Versioned Bases fixture conformance.** For each pinned profile version, evaluate every fixture `.base` against the fixture vault and assert equality with the recorded expected result set, ordering, grouping, per-cell values, and per-cell states. Assert every fixture file is byte-identical after a full read-evaluate-edit-unrelated-key cycle.
- **Rebuild equivalence (1A + 3A).** Snapshot every source digest, delete the SQLite database, rebuild, re-run every Base and `schema check`; assert byte-for-byte source non-mutation and identical tables, orderings, cycle paths, and diagnostics.
- **Index-authority inversion.** Tamper with a stored typed property row directly in SQLite; assert freshness verification reports `degraded` and that no tampered value is ever rendered as current, exactly as the existing store test does for title/text/links.
- **Migration fault injection.** Kill the process at every journal phase and every per-step boundary during a 20,000-note migration; assert convergence on the next invocation to either the complete pre-state or the complete post-state, never a mixture, with every file byte-identical to one of two known digests and the schema file on the same side as the notes.
- **Migration rollback.** Roll back a committed receipt; assert every touched file is restored byte-for-byte; then drift one file and assert `rollback_blocked` with nothing written and pre-images offered under `recovered/`.
- **Concurrency (4B).** External edit between preview and commit; assert `conflict`/`generation_changed`, both versions intact, and no partial write. Two concurrent migrations; assert the second refuses on the active journal.
- **Confinement (4A).** A `.base` whose filter names `../../etc`, a schema path override pointing outside the vault, a symlinked `.base`, and a schema file inside `.obsidian`; assert each fails closed and that `.obsidian` is never written.
- **Freshness fail-closed.** Edit a note behind the index; assert `base run` and `schema check` return zero rows and exit 6 with the state named, and that `--allow-stale` labels the banner, every group header, every record block, and the envelope, while `base materialize` refuses outright.
- **Textual-equivalence conformance (auto-fail gate, 5C).** For every fixture Base and every width in {40, 60, 80, 120}, assert the `(row, column, value, state)` set of the record view and of `--json` equals the full model, and is a superset of what the table displays.
- **Obsidian coexistence.** A vault containing `.obsidian/types.json` and Obsidian-authored `.base` files; assert `.obsidian` is read-only, never written, never indexed as notes, and that its type assignments influence only `schema infer` proposals.
- **Scale.** On the 100k corpus, assert every §4.7 budget with p50/p95/p99, peak RSS, cancellation latency, and concurrent direct-write latency during a whole-vault check.

### 5.3 UI / E2E tests

CLI E2E for every command in human, `--json`, and `--jsonl` modes with golden fixtures covering: happy path; empty states; `duplicate_key`; `yaml_alias_target`; `property_inherited`; `comment_ownership_ambiguous`; `frontmatter_ambiguous`; `migration_blocked` with each policy; `generation_changed`; `transaction_incomplete` and recovery; `rollback_blocked`; `base_invalid`; `unsupported_feature`; `definition_cycle`; a `cycle` cell; `depth_exceeded`; `type_mismatch`; `stale_index`; `confirmation_required` under `--no-input`; and `cancelled`. Goldens assert envelope version, provenance fields, deterministic ordering, stable error codes, and stable exit codes without depending on volatile timestamps.

TUI E2E (once **E** lands; this gates the pane claim, not the rest of the spec): open the property form, edit a value, observe the diff preview, confirm, and assert the file changed only in the declared span; attempt to edit a `raw` row and assert the refusal plus the `e` affordance; add and remove a key that owns comments and assert the escalated confirmation; open a Base pane, change sort, group by a column, focus a `cycle` cell and read its path in the detail strip, dump `J` to JSON and diff it against `base run --json`; restart and assert session restore returns to the same row and column, announcing the change if the row disappeared. Assert every step is keyboard-only and every state is announced in words.

### 5.4 Visual / manual verification

- Terminal themes: default, light, dark, high-contrast, and fully ANSI-stripped; confirm every state word (`raw`, `cycle`, `redacted`, `stale`, `unsupported`, `inherited`, `restyled`, `truncated`) survives stripping.
- `NO_COLOR`, `--no-color`, `--ascii`, and a non-TTY pipe: confirm identical record content in all four and that `…` becomes `...`.
- Widths 40, 60, 80, 120, 200 with long Unicode, CJK, emoji, combining marks, and RTL keys and values; confirm the automatic switch to the record view below 60, that no path, error code, cycle path, or recovery command is ever truncated, and that column 1 is never dropped.
- Empty vs populated: a note with no frontmatter; a note with one property; a note with 200 properties; a Base with 0, 1, 50, and 5,000 rows; a Base with every cell in an error state.
- Freshness states: current, stale, rebuilding, degraded, unavailable — each verified to print its banner above **and** below the rows.
- Screen-reader pass over the property form, a migration preview, a Base table, a Base record view, and a cycle explanation.
- Terminal font-size zoom as the dynamic-type analogue; confirm reflow correctness and that focus survives the resize.

---

## 6. Compliance & Safety Gate

### 6.1 Sensitive data classification

- [ ] No sensitive data involvement
- [x] **Handles sensitive data** — frontmatter properties commonly hold API keys, client names, health notes, and financial figures, and Base tables aggregate exactly that. Protections: everything is local and offline with zero network egress; `secret: true` properties are excluded before ordinary index tables, FTS, snippets, Base cells (rendered as the literal word `redacted`), `--explain`, JSON, logs, diagnostics, exports, and materialization, and a configuration change to the secret set forces a generation rebuild; the existing interop export already excludes `private`, `.private`, `secrets`, `.secrets` and this feature honors the same exclusions; error strings carry codes, vault-relative paths, byte ranges, and versions but never note bodies, property values, environment values, or absolute paths outside the vault; the memo cache is in-memory, generation-keyed, and never persisted; there is no telemetry and no query history in this feature.
- [x] **Uses synthetic/test data only until compliance gate clears** — every fixture, adversarial, Bases, and 100k-scale corpus is generated; no personal vault is required for acceptance.

### 6.2 Asset provenance

- [x] **No third-party assets** — no images, fonts, models, or bundled data. The Bases fixture corpus and adversarial YAML corpus are authored originally for this project; where a fixture is intended to mirror a documented Obsidian behavior, the behavior is cited by Obsidian release version in the fixture header and the file itself is newly written, not copied.
- [ ] Uses third-party assets

Code dependencies (`yaml-edit` MIT/Apache-2.0, already vetted and in the workspace; `unicode-width`, `unicode-segmentation`, a date crate, and dev-only `yaml-rust2` and `proptest`) are subject to the same license/SBOM/pinning review as B's and F's.

### 6.3 Language / claims audit

- [x] Make claims not supported by evidence? **No.** Every latency, memory, and scale figure in §4.7 is labeled an acceptance budget. "Obsidian-compatible" is defined narrowly and testably as *equality with a versioned fixture corpus at a named profile version*, and `base check` reports the profile so a user is never told a Base is supported when its features are not.
- [x] Promise capabilities not yet built? **No.** This is target state throughout; §7 records exactly what exists, what is prototyped, what is planned, what is gated, and what is absent. TUI panes are labeled E-gated and must report `provided_by: "tui"` rather than `available: true` until E ships.
- [x] Use language restricted by domain regulations? **No.** This is not a regulated domain; no medical, financial, or legal claim is made about property values the user stores.

### 6.4 Regulatory alignment

Lens 3 is the criteria file's structural gate for this feature, and Lens 1 is its weight:

- **1A Authority.** Schema, Bases, formulas, relations, and rollups are all ordinary files or pure computations over them. Deleting every index, cache, and memo and rebuilding from unchanged source reproduces every table byte-identically (§5.2). No derived artifact can be edited into authority; `mg-vault-props` has no persistence.
- **1B Preservation.** The only mutation primitive is a disjoint span splice through `edit_note_spans`; no serializer exists in the workspace and an API-surface test enforces that; comments, key order, quoting style, anchors, aliases, tags, directives, document markers, indentation, and line endings survive an edit to an unrelated key, proven by the corpus round-trip gate plus CST index comparison plus an independent oracle.
- **1C Identity.** No UUID, no `id:` key, no hidden marker is ever injected into a note — not by property edit, not by migration, not by materialization, not by relation inversion. Paths remain public identity; property keys are compared byte-exactly.
- **1D Coexistence.** `.obsidian` is read-only and never written (the existing core rule stands); `.obsidian/types.json` informs `schema infer` only; the schema is vault content at the vault root so it syncs and diffs with the notes; portable app settings live under `.mg-vault`; `.base` files stay in Obsidian's own format.
- **1E Transactions.** Fingerprint preconditions on every write; disjoint-span validation; candidate verification before commit; H's journal, commit barrier, receipt, and all-or-nothing rollback for migration and materialization; every failure path writes nothing and reports both digests.
- **3A Determinism.** Definition-topological evaluation order, canonical cycle-path rotation, total sort with path byte-order tie-break, deterministic width algorithm, counter-based (not clock-based) limits.
- **3B Ambiguity.** Duplicate keys, ambiguous coercions, ambiguous comment ownership, YAML 1.1 booleans, and type mismatches are all terminal states with full candidate listings; none is resolved by a tie-break and none mutates a file to disambiguate.
- **3C Query depth.** Base filters cover text, titles, tags, typed properties, paths, links, tasks, and dates through B/G's grammar, plus formulas, relations, inverse relations, and rollups added here.
- **3D Derived authority.** Explicitly specified in §3.2 and §4.4: a Base is a file, not a table; an inverse relation is computed and never written; a rollup enters a note only through `base materialize`, which is opt-in, previewed, atomic, receipted, and rollbackable.
- **3E Scale.** §4.7 gives warm, cold, whole-vault, memory, storage, cancellation, and editing-isolation budgets at 100,000 notes and 1,000,000 blocks; §5.2 makes the corpus an acceptance gate.

**Named auto-fail rules.** *Source-content loss:* impossible by construction for edits (span splice + verification + property tests) and forbidden by policy for migration (no option deletes a value; `quarantine` is additive). *Partial multi-file mutation:* migration and materialization run through H's journal with a commit barrier and converge to all-or-nothing under fault injection. *Index state overriding source:* no note bytes ever come from SQLite; opening a row re-reads the file and compares fingerprints. *Stale index presented as current:* `DerivationProvenance` is mandatory on every response, banner, group header, record block, and pane; default is fail-closed with exit 6. *Silent conflict winner:* fingerprint mismatch reports both digests and writes nothing. *Unknown syntax loss:* the central invariant of §3.2 and §5.1/§5.2 — unknown YAML, unknown Base keys, unknown functions, and unknown view types are all preserved verbatim and reported. *Unconfirmed overwrite/import:* every mutation previews and confirms; `--no-input` refuses rather than assumes yes; escalated changes require `--from-plan`. *Unsafe traversal or symlink escape:* all paths go through A's confined authority; `.obsidian`/`.mg-vault` mutation is rejected. *Capability/data-exfiltration bypass:* the formula interpreter is pure with no host functions and no I/O; plugins and AI need N/O grants. *Active raw HTML/script by default:* no HTML is produced or activated here; cell text is escaped. *Non-atomic save claiming success:* no success verb is printed without the durable record. *Recovery overwriting newer source:* a recovered draft is a proposal checked against the current fingerprint and refused if it moved. *Graph/Canvas information lacking a textual equivalent:* the record view and `--json` carry a superset of every table datum, asserted by conformance test at every width.

---

## 7. Gap Analysis vs. Current State

### 7.1 What exists today

**Implemented.**
- `crates/mg-vault-core/src/frontmatter.rs` — `scan_frontmatter` returns exact YAML and body byte ranges plus `LineEndings::{Lf, CrLf, Mixed}` without normalizing anything. It requires an exact `---` first line and an exact `---` closing line, treats a closing delimiter at EOF as valid, ignores delimiter-like content inside values, and fails closed on `UnsupportedBom` and `Unclosed`. Seven unit tests cover LF, CRLF, mixed, delimiter-like values, EOF close, leading whitespace, and both rejection cases.
- `crates/mg-vault-core/src/frontmatter_scalar.rs` — `locate_frontmatter_scalar(source, key)` returns the absolute byte range of a **top-level scalar value**. It parses the YAML slice with `yaml_edit::Document` for validation and for `as_mapping()` / `as_scalar()` type checking, then **re-locates the value with a hand-rolled line scanner over the original bytes**, so CRLF, comments, quoting, and spacing are untouched. It counts matches across the slice and returns `NoFrontmatter`, `MissingKey`, `DuplicateKey`, `NotScalar`, or `InvalidYaml`. `strip_comment` removes a trailing `#` comment while respecting single and double quotes. Six integration tests in `crates/mg-vault-core/tests/frontmatter_scalar.rs` cover a quoted value with leading and trailing comments, CRLF, a nested key shadowed by a top-level key, duplicate keys, missing frontmatter and missing keys, and non-scalar plus malformed YAML.
- `Vault::edit_note_span` — one fingerprint-checked, UTF-8-boundary-checked, checked-arithmetic, exact prefix/suffix-copying, durably atomic single-span splice.
- `yaml-edit 0.2.3` with `default-features = false` is a workspace dependency in `vault/Cargo.toml`, present specifically to supply lossless YAML validation and byte ranges without a serializer.
- The core rule rejecting any mutation whose first path component is `.obsidian` or `.mg-vault`, and the interop export's exclusion of `private`, `.private`, `secrets`, `.secrets`.

**Prototyped.** `mg_vault_core::index::title_for` reads a frontmatter `title` as a special-cased property for the search projection. That is the only property-aware behavior in the codebase and it is not a typed model.

**Planned (specified, not built).** `docs/spikes/token-preserving-markdown-yaml.md` fixes the architecture this spec builds on: targeted source spans as the only mutation model, `yaml-edit` as a span locator behind a project adapter, mandatory reparse of candidate YAML before commit, one plan and one atomic replacement for multi-edits, refusal on ambiguous comment ownership, explicit occurrence selection for duplicate keys, and a prohibition on whole-document serializers. It also lists the required fixture gates (all scalar styles, anchors, aliases, tags, directives, complex keys, Unicode, malformed YAML). Spec **B** plans `PropertyProjection { key, yaml_path, value_type, canonical_value, source_byte_range }` and typed property comparisons that reject coercion; Spec **F** plans the YAML span adapter that finally uses `yaml-edit` byte ranges; Spec **G** plans the query grammar's `property.KEY` predicates; Spec **H** plans the multi-file transaction engine this feature's migration reuses.

**Gated.** All TUI panes are gated on **E**. Migration and materialization are gated on **H**'s transaction engine. Base row sets are gated on **B**'s typed property projections and freshness contract; until B lands, `base run` can only report `unavailable` and exit 6.

**Absent.** Everything else: the CST index, the write-side YAML edit model, style and comment-ownership policy, anchor/alias/merge/tag handling, `edit_note_spans`, the schema file format, `schema` and `property` and `base` commands in any form, the Bases parser and profile, the formula interpreter, relations, inverse relations, rollups, cycle detection at either level, the table renderer and its width algorithm, the record view, the JSON contract, the migration planner, the fixture corpora, and every test in §5.

### 7.2 Delta to spec

- **New crate:** `crates/mg-vault-props` with the module tree in §4.1, plus `fixtures/bases/<profile>/` and `fixtures/yaml/` corpora.
- **New in `mg-vault-core`:** `Vault::edit_note_spans` (multi-span, disjoint, single fingerprint, single atomic replace) and an `Error::OverlappingSpans` variant.
- **Modified `crates/mg-vault-core/src/frontmatter_scalar.rs`:** replace the hand-rolled line scanner with `yaml-edit` byte ranges behind the shared adapter, keeping the existing public signature and all six integration tests green, and generalizing beyond top-level single-line plain scalars to nested paths, sequences, block and flow styles, occurrence selection, and anchors/aliases/tags.
- **Modified `crates/mg-vault-cli/src/main.rs`:** add `property`, `schema`, and `base` command trees plus their output modules; extend the error-code mapping with this feature's codes; keep the `version: 1` envelope and exit-code categories unchanged.
- **Modified `crates/mg-vault-index`:** typed property columns, a `relations` table, a `bases` discovery pointer, a `SCHEMA_VERSION` bump, and B's side-by-side rebuild for the migration.
- **Migrations / schema changes:** the index schema bump above (disposable, rebuildable); the vault-level `mg-vault.schema.yaml` format at `version: 1`; the `bases_profile_version` and `yaml_model_version` constants, both echoed in every response and negotiated over IPC.
- **New dependencies:** `unicode-width`, `unicode-segmentation`, a date crate, and dev-only `yaml-rust2` and `proptest`. `yaml-edit` is already present.
- **Documentation:** `docs/yaml-edit-model.md` (span model, style policy, comment ownership, anchor/alias/tag rules), `docs/bases-profile.md` (the supported function/view inventory per profile version and the unknown-feature classification), `docs/schema.md` (schema file format and migration policy), plus README and `docs/PRODUCT.md` updates when each slice ships.

### 7.3 Estimated scope

**XL.** Four substantial subsystems sit here: a write-side lossless YAML model with a provable preservation invariant (the hardest and most consequential part, since Lens 1B at 30% weight turns on it); a schema system with a fault-tolerant, rollbackable, never-lossy whole-vault migration; a Bases parser, expression interpreter, and evaluator that must match a versioned external product's behavior by fixture; and a relation/rollup evaluator with two distinct cycle policies and a deterministic terminal table renderer. It also depends on B, F, and H, two of which are themselves XL and largely unbuilt.

Deliver as gated increments behind one preservation contract and one freshness contract: **I1** CST index + span edit model + style/comment policy + `edit_note_spans` + the YAML corpus round-trip gate (this is the preservation-critical slice and must pass its full fixture gate before anything else begins); **I2** `property` commands and typed values; **I3** schema file, `schema show/check/infer/doctor`; **I4** `schema migrate` on H's transaction engine, with rollback and fault injection; **I5** Bases parser, profile, unknown-feature classification, `base list/check`; **I6** filters, formulas, table and record renderers, JSON contract; **I7** relations, inverse relations, rollups, both cycle policies; **I8** TUI panes and `base materialize`. I1 through I6 are individually reviewable and shippable from the CLI alone.

### 7.4 Blocking dependencies

- **A (foundation) — implemented.** Confined path validation, `scan_frontmatter`, fingerprints, atomic replacement, and the `.obsidian`/`.mg-vault` protection rule. I1 needs only `edit_note_spans` added here.
- **F (Markdown/YAML parser) — planned.** The shared YAML span adapter and the token cover. I and F must ship exactly one adapter; whichever lands first owns it. I1 can proceed independently of F's Markdown work.
- **B (index service) — planned.** Typed property projections, relation edges, freshness generations, IPC, cursors, and cancellation. `base run` cannot report anything but `unavailable` until B1–B5 land; `property` and `schema check --path` on a single note work without B.
- **H (refactoring) — planned.** The multi-file transaction engine, journal, receipts, and rollback. I4 and I8 block on it entirely; I1–I3 and I5–I7 do not.
- **G (links/search) — planned.** Link resolution for `link`-typed properties and relation targets, and the `property.KEY` query predicates that Base filters compile into. I7 needs G's resolution to build relation edges; ambiguous relation targets inherit G's fail-closed ambiguity contract.
- **C (CLI contracts) — partially implemented.** The `version: 1` envelope, error codes, exit-code categories, `--no-input`, `--no-color`, and `NO_COLOR` exist; the confirmation and `--from-plan` machinery is planned.
- **E (TUI) — absent.** Gates I8 only. Without E, every capability here remains fully available from the CLI.
- **External gate:** a decision on which Obsidian release the first `bases_profile_version` pins to, since the fixture corpus and the compatibility claim are defined against it (see Q3).

---

## 8. Open Questions

- **Q1:** Should the vault schema live at `mg-vault.schema.yaml` in the vault root (chosen here: content that syncs and diffs with the notes, visible to Obsidian as an inert file) or under `.mg-vault/` (tidier root, but then a schema does not travel with a synced vault and violates the spirit of file authority)? — blocks: §4.2, §4.6, §7.2.
- **Q2:** Should `schema export --to-obsidian` ever be allowed to write `.obsidian/types.json` behind an explicit, previewed, single-purpose exception to the protected-directory rule, or must it always stay a candidate file the user installs by hand (chosen here)? — blocks: §4.6, criterion 1D.
- **Q3:** Which Obsidian release does the first `bases_profile_version` pin to, and what is the update cadence when Obsidian ships new Bases functions — automatic profile bump with fixtures, or user-visible opt-in? — blocks: §4.6, §5.2, and the entire compatibility claim.
- **Q4:** For YAML 1.1 booleans (`yes`, `no`, `on`, `off`), this spec chooses "never coerce, report as ambiguous". Obsidian's own behavior here should be fixture-pinned before I2 ships. Is refusing to coerce acceptable friction, or should a documented, previewed, opt-in coercion table exist? — blocks: §4.6, §3.2 migration classification.
- **Q5:** Should the default `on_error` for rollups be `propagate` (chosen here: a cycle poisons the aggregate) or `exclude` with a loud label? `propagate` is the honest default but will surface as an error cell in real vaults on day one. — blocks: §3.2 cycle safety, §4.2.
- **Q6:** Should `base materialize` be permitted at all in v1, given that a materialized value is a snapshot that immediately begins to drift with no in-file provenance, and the only drift detector is an explicit `--check`? — blocks: §3.2, §4.3.
- **Q7:** Are property keys case-sensitive (chosen here: byte-exact, with `key_case_collision` as a diagnostic), or should `Status` and `status` be merged for query purposes with a documented comparison key the way B treats tags? — blocks: §4.6, §4.2.
- **Q8:** How aggressive should `schema infer` be — propose a type only at 100% agreement across observed values, or at a configurable threshold with the minority reported as prospective violations? — blocks: §3.1, §4.3.
