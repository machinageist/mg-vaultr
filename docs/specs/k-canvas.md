# Spec: Canvas

**Feature ID:** k-canvas
**Parent feature:** root
**Spec author agent:** Canvas spec agent (K)
**Date:** 2026-08-29
**Iteration:** 1

---

## 1. Purpose

### 1.1 One-sentence job

Let a keyboard-only user read, navigate, and edit a JSON Canvas 1.0 `.canvas` file in a terminal — both as a spatial picture and as a complete, information-equivalent textual outline — while every byte the tool does not explicitly edit, including every property and key order it does not model, survives the save unchanged.

### 1.2 Why it matters

A `.canvas` file is the one artifact in this product that is *authored by other applications* and is *inherently spatial*. Both facts are traps.

The preservation trap: Obsidian, Kinopio, Flowchart Fun, Trellis and future tools all write `.canvas`, and JSON Canvas 1.0 explicitly permits extra properties. The obvious implementation — `serde_json::from_str::<Canvas>()`, mutate, `to_string_pretty()` — silently destroys every field mg-vault does not model, reorders keys, reformats numbers, and rewrites the whole document on a one-pixel nudge. That is unknown-syntax loss, an auto-fail, and it is the default outcome of the default approach. K exists to refuse it structurally.

The accessibility trap: a canvas is normally a picture. In a terminal there is no picture worth trusting — box drawing is not available everywhere, 40-column terminals exist, screen-reader users exist, and any information that lives only in pixel positions is information a large fraction of this product's users cannot reach. "Graph or Canvas information lacking a textual equivalent" is an explicit auto-fail rule for this product. So K inverts the usual hierarchy: the **textual outline is the canonical representation and the source of truth for the UI**, and the spatial view is a derived, strictly additive, always-suppressible rendering of it. Nothing is reachable in the drawing that is not reachable in the list.

The identity trap: JSON Canvas nodes have `id` strings. It would be convenient to treat those as a note-identity system. K does not. Node ids are file-internal handles; file paths remain public note identity, and no id is ever written into a Markdown note.

### 1.3 Success signal

On a fixture corpus of `.canvas` files produced by Obsidian and by hand — including unknown top-level keys, unknown node types, unknown node and edge properties, non-ASCII labels, negative and non-integer coordinates, CRLF, no trailing newline, 4-space and tab indentation, and `\u`-escaped strings — two properties hold and are enforced by CI:

1. **Byte-faithfulness:** for every fixture and every single-property edit K supports, the saved file differs from the original in exactly the byte spans the preview listed, and `diff` over the remaining bytes is empty. A no-op edit produces a byte-identical file. Round-tripping a fixture through open → save-with-no-change → open is the identity function.
2. **Textual completeness:** a coverage test walks every leaf position in each fixture's JSON span tree and asserts that each one is reachable in `mg-vault canvas outline --full --unknown --json` output; the assertion is set equality, not containment, so an unmodelled property fails the build rather than disappearing quietly. Separately, the set of (node, edge, label, group-membership, spatial-relation) facts the spatial pane can display is asserted equal to the set the textual pane displays.

---

## 2. User Stories

> As a writer who built a canvas in Obsidian, I want to open it in mg-vault, move a node, and reopen it in Obsidian with everything else intact, so that using this tool is never a one-way door.

> As a keyboard-only user, I want to add nodes, connect them with labelled edges, retype card text, and reposition things without ever touching a mouse, so that a canvas is as editable here as a note is.

> As a screen-reader user, I want every node, edge, label, group membership, and spatial relationship available as ordered lines I can navigate and edit, so that the drawing is an optional aid rather than the only channel.

> As a user on a 40-column terminal over SSH, I want the canvas to remain fully usable with the drawing turned off, so that a narrow terminal costs me rendering, not capability.

> As a user who renamed a note, I want the canvas file nodes that pointed at it updated in the same transaction as my other links, so that reorganizing does not silently break my canvases.

> As a user with two notes named `alpha.md`, I want a file node that says `alpha.md` reported as ambiguous and left exactly as written, so that the tool never silently repoints a card at the wrong note.

> As a cautious user, I want to see the exact JSON byte spans a command will change before it changes them, so that I can refuse an edit that would touch more than I expected.

> As an automation author, I want stable versioned JSON for nodes, edges, resolution states, and unknown properties, so that I can script canvas reporting without scraping a drawing.

> As a user opening a canvas some other tool wrote badly, I want mg-vault to tell me what is nonconforming and refuse to "helpfully" repair it, so that my file is never rewritten behind my back.

---

## 3. UX Specification

### 3.1 Screen / view inventory

This is a CLI and TUI product. K introduces no GUI, no mouse requirement, no sound, and no haptics.

**CLI views (all line-oriented):**

| View | Entry point | New/modified | Layout pattern |
|---|---|---|---|
| Canvas outline (**canonical**) | `mg-vault canvas outline PATH` | New | Header, ordered node blocks with indented edge/membership lines, footer summary |
| Node inventory | `canvas nodes PATH [--sort …]` | New | One record block per node |
| Edge inventory | `canvas edges PATH` | New | One record block per edge |
| Single node detail | `canvas node show --id ID` | New | Labelled `field: value` lines including every unknown property |
| Spatial render (static) | `canvas render PATH [--width --height --zoom --center]` | New | Deterministic box-drawn frame emitted to stdout, followed by an off-screen manifest |
| File-node references | `canvas links PATH` | New | One record per file node with resolution state and rule |
| Canvas diagnostics | `canvas doctor [PATH \| --all]` | New | Ordered findings: code, severity, JSON pointer, byte range, evidence |
| Edit preview | any mutating `canvas` subcommand, always before commit; also `--dry-run` | New | Header, complete span manifest, per-span before/after, preconditions |
| Explicit repair | `canvas fix PATH --finding CODE …` | New | Same preview, per-finding opt-in, never bulk |
| URL open | `canvas open-url --id ID` | New | Echoes the exact URL and scheme decision before launching |

**TUI (hosted by E; E owns geometry and focus, K owns the model):**

| Pane / overlay | How to reach | New/modified | Backing contract |
|---|---|---|---|
| Canvas pane, **textual mode (default)** | Opening a `.canvas` file in any pane | New (E's `PaneKind::Canvas { mode: Textual }`) | K node/edge/relation records |
| Canvas pane, spatial mode | `T` in the canvas pane, `:canvas spatial` | New (`CanvasMode::Spatial`) | Same records, rendered |
| Node detail overlay | `K` on a selected row | New | K node record with unknown properties |
| Edge picker overlay | `]e` / `[e` when a node has >1 edge | New | Ordered edge list, no pre-selection |
| Canvas edit confirmation | Any mutating action | New | Reuses E's confirmation overlay with K's span manifest |
| Canvas diagnostics rows | `<leader>d` while a canvas is focused | Modification of E's diagnostics pane | `canvas doctor` findings |

Every TUI capability has a CLI equivalent producing the same records, so K is fully usable with no TUI at all.

### 3.2 Interaction flows

#### Open and parse

1. Resolve the vault and validate the path through A's authority, extended to accept the `.canvas` extension as a first-class mutable file kind (§4.1). Traversal, symlink escape, and `.obsidian`/`.mg-vault` first components are rejected exactly as for notes.
2. Read the exact bytes and compute the `SourceFingerprint`. The fingerprint is the precondition for every later write.
3. Build a **span tree** (§4.2): a lossless index of the JSON text recording, for every value, its byte range, and for every object, its member key ranges in document order. Nothing is deserialized into an owned model. Number lexemes, string escape forms, whitespace, indentation, line endings, BOM presence, and trailing newline are all recoverable because the original bytes are retained and never rewritten wholesale.
4. Project a **read-only view** over the span tree: nodes, edges, and the JSON Canvas 1.0 fields K understands. Everything else is retained as `unknown` entries carrying key, byte range, and raw lexeme.
5. Classify the document: `conforming`, `nonconforming` (valid JSON, violates JSON Canvas 1.0 — e.g. a node missing `x`, a duplicate node `id`, an edge whose `fromNode` names no node), or `invalid_json`. `invalid_json` opens **read-only**: the outline reports the parse error's byte offset and line/column, no edit is accepted, and the file is still openable as plain text through D/E. Nonconforming opens editable, with every violation listed by `canvas doctor` and edits to the *affected* member refused (§3.6).

#### The JSON Canvas 1.0 surface, and everything outside it

K models exactly this surface and treats it as versioned by `canvas_spec_version: "1.0"`:

- **Top level:** `nodes` (array), `edges` (array). Any other top-level key is **unknown**: preserved byte-for-byte, listed by `canvas outline --unknown`, never removed, never reordered.
- **All nodes:** `id` (string), `type` (string), `x`, `y`, `width`, `height` (numbers), `color` (optional). Node array order is z-order (later is on top) and is preserved; it is only changed by explicit `canvas node raise|lower`.
- **`type: "text"`:** `text` (string, Markdown).
- **`type: "file"`:** `file` (string, vault-relative path), `subpath` (optional string beginning `#`).
- **`type: "link"`:** `url` (string).
- **`type: "group"`:** `label`, `background`, `backgroundStyle` (`cover` | `ratio` | `repeat`), all optional.
- **Edges:** `id`, `fromNode`, `toNode` (required strings); `fromSide`, `toSide` (`top`|`right`|`bottom`|`left`); `fromEnd`, `toEnd` (`none`|`arrow`, defaults `none`/`arrow`); `color`; `label`.
- **`canvasColor`:** either `"#RRGGBB"`/`"#RGB"` or a preset digit string `"1".."6"` (red, orange, yellow, green, cyan, purple).
- **Coordinates:** canvas units, y increases downward, origin arbitrary, values may be negative. Rectangles are axis-aligned. JSON Canvas 1.0 specifies integers; K reads any JSON number, preserves the exact lexeme, and only ever *writes* a canonical shortest integer.

Everything outside that surface is handled by one rule — **preserve, report, refuse to interpret**:

| Out-of-surface thing | K's behavior |
|---|---|
| Unknown top-level key | Preserved; listed in `outline --unknown`; never a parse error |
| Unknown node/edge property | Preserved; shown in `node show` / `edges --unknown`; editable only via `canvas node set --raw KEY=JSON`, which is explicit and previewed |
| Unknown `type` value | Node is `type: "foo" (unknown)`. Rendered as a generic rectangle labelled with its literal type. Geometry edits allowed when `x/y/width/height` exist and are numbers; payload edits refused with `unknown_node_type` |
| Unknown `fromSide`/`toSide`/`backgroundStyle`/`fromEnd`/`toEnd` value | Preserved and echoed literally; layout falls back to the documented default anchor and says so in the record |
| Unknown color token | Preserved and echoed literally; display uses the theme's neutral role; the literal token always appears as text |
| Missing required property | Document is `nonconforming`; `canvas doctor` reports it with a JSON pointer; edits to that node's geometry/payload are refused until an explicit `canvas fix` supplies it |
| Duplicate key inside one object | Preserved. The view reads the **last** occurrence (documented, matching JSON parser convention); `doctor` reports `duplicate_key`; any edit targeting that key is **refused** because the resulting semantics would be ambiguous |
| Duplicate node `id` | Preserved; `doctor` reports `duplicate_node_id` listing every byte range; edges referencing that id are `ambiguous_endpoint`; K never renumbers an authored id |
| Non-integer / high-precision coordinate | Lexeme preserved; layout uses `f64`; an arithmetic edit that would not round-trip returns `coordinate_precision` and writes nothing |

K has **no whole-document serializer**. There is no code path that can emit a `.canvas` file from a model; the only writer emits fragments for insertion into an existing byte buffer (§4.2). This is the structural guarantee behind 1B, and it mirrors the accepted precedent in A, where `yaml-edit` is used to *locate* frontmatter scalars and never to serialize.

#### Textual mode — the canonical representation

Textual mode is the default in both CLI and TUI. It is not a fallback; the spatial view is generated from the same records and can display nothing the outline omits.

The outline is a deterministic sequence of node blocks. Each block carries, in fixed order: ordinal and id; type; geometry; color; type-specific payload; **derived group membership**; **derived spatial relations**; degree; unknown properties; and every incident edge as an indented line. Sort order is `--sort document` (array order, the default), `reading` (y, then x, then array index), `id`, `type`, or `label`; every order has array index as the final tie-break, so output is reproducible.

**Group membership is derived, and labelled as derived.** JSON Canvas 1.0 has no parent pointer: containment is geometric. K computes it with a documented, versioned rule (`membership-rule-v1`): node *N* is a member of group *G* iff *G*`.type == "group"`, *N* ≠ *G*, and *N*'s rectangle lies entirely within *G*'s rectangle, edges inclusive. Every containing group is listed, innermost first (smallest area; ties broken by later array index). Partial overlap is **not** membership; it is reported separately as `overlaps`. Every membership line is stamped `derived: geometric containment (membership-rule-v1)` so it can never be mistaken for a stored property, and **K never writes a parent property into the file** to record it.

**Spatial relationships are stated in words, not implied by position.** Each node block carries a `spatial:` group giving: rectangle in canvas units; center; the nearest node in each of the four directions with its gap in canvas units; alignment sets it participates in (`left-aligned with #3, #7`); overlaps; and, when a group contains it, its offset from that group's top-left. These are exactly the facts a sighted user reads off the picture, written down. They are computed deterministically and are also present in `--json`.

Textual mode is fully **editable by keyboard**, not merely readable. Every mutation available in spatial mode is available here by an explicit command with the same preview and the same atomic commit: `o` add node, `O` add edge, `e` edit payload, `r` rename label, `c` set color, `m` set geometry (absolute or `--dx/--dy`), `[`/`]` lower/raise in z-order, `dd` remove (confirmed, listing the edges that would become dangling and refusing until they are also removed or reassigned), `gp` jump to containing group, `gm` cycle members, `]e`/`[e` cycle incident edges, `Enter` follow a file node or an edge to its other endpoint.

#### Spatial mode — an honest terminal viewport

Spatial mode is a deterministic projection of canvas coordinate space onto character cells. There is no force-directed layout, no animation, and no randomness: the same document, viewport, and zoom always produce the identical frame, which is what makes `canvas render` testable as a golden file.

- **Projection.** A viewport is `(center_x, center_y, units_per_col)`. Because a terminal cell is roughly twice as tall as wide, `units_per_row = units_per_col × cell_aspect`, with `cell_aspect` defaulting to `2.0` and configurable via `:set canvasaspect`. `units_per_col` is quantized to a documented ladder (`… 5, 10, 20, 40, 80, 160 …`), so zoom is reproducible and never a float the user cannot name.
- **Rasterization.** A node rectangle maps to a cell rectangle by `floor` on the top-left and `ceil` on the bottom-right, then is clamped to a minimum of 3×3 cells so a border and one label character always fit. A node whose projected size is below that minimum renders as a single marker cell with its ordinal, and the status line reports `N nodes below render size`.
- **Nothing is ever silently dropped.** Nodes outside the viewport are counted per edge of the screen (`◀ 3 off-screen  ▶ 11 off-screen`), and every off-screen node remains in the list. A frame that cannot show everything says so; it never implies the canvas is smaller than it is.
- **Boxes.** Unicode box drawing (`┌─┐│└┘`) with a full ASCII fallback (`+-+|+ +`). Inside the border: a header line `#{ordinal} {type-sigil} {label}` where the sigil is a letter (`T` text, `F` file, `L` link, `G` group, `?` unknown) — **type is never communicated by color alone** — then wrapped payload text. Groups are drawn with a doubled border and their label in the top edge.
- **Edges.** Orthogonal (Manhattan) routing between anchor points derived from `fromSide`/`toSide`; when a side is absent, the anchor is chosen by the documented default rule (the side facing the other node's center, ties resolved `right, bottom, left, top`). Lines use box-drawing characters, crossings use `┼`, arrowheads follow `fromEnd`/`toEnd` (`▶◀▲▼`, ASCII `> < ^ v`). Labels are drawn at the route midpoint when they fit; otherwise a `⟨7⟩` ordinal marker is drawn and the full label appears in the list. When routing is impossible (congestion, off-screen endpoint), the edge degrades to **stubs** — a short labelled stub at each visible endpoint naming the other endpoint's ordinal — so the edge is still visible as a fact. An edge is never omitted from the frame's accounting.
- **Navigation.** `h/j/k/l` pan one cell, `H/J/K/L` pan a page, `+`/`-` step zoom, `zf` zoom-to-fit-all (computes the tightest ladder step whose projected bounding box fits, then centers), `zs` fit selection, `zz` center selection, `0` reset to the 1:1 ladder step at origin. Panning and zooming are viewport state only and never write the file.
- **Selection is shared.** Spatial and textual mode share one selection cursor. `Tab`/`S-Tab` move selection in canonical order; `T` toggles modes and lands on the same node in both. Any edit made in spatial mode goes through the identical preview and commit path as the textual command.
- **Suppression.** The drawing region is omitted entirely — with no loss of capability, because the list is canonical — when the terminal is narrower than 60 columns or shorter than 20 rows, when box-drawing capability is not detected, under `--no-graphics`, or in E's screen-reader mode. In those cases the pane *is* the outline.

#### File nodes: resolution, missing, ambiguous

A `file` node references another vault file. Resolution reuses **G's resolver, byte-for-byte, with the same versioned ladder and the same refusals** — K adds no second resolution policy:

1. `exact_path` — the `file` value read as a vault-relative path, byte-exact including extension.
2. `vault_relative` — root-anchored, then with `.md` appended, after lexical `./` removal.
3. `canvas_relative` — the same two forms resolved against the directory of the `.canvas` file itself; `..` permitted only while the result stays inside the root.
4. `unique_basename` — only when the value contains no `/`.

The first tier yielding **exactly one** candidate wins and the `resolution_rule` is recorded. A tier yielding more than one candidate ends the ladder as `ambiguous` — it never falls through and never applies a tie-break. Exhausting the ladder is `unresolved`. Comparison is byte-exact: no case folding, no Unicode normalization.

- **Missing target:** state `unresolved`, with `near_miss` candidates naming the exact byte difference. The node renders normally with a literal `[missing]` marker in its header and a `target: unresolved` line. **The `file` value is not changed, not cleared, and not repointed.** `Enter` offers to create the note at that exact path through C's collision-safe create, always confirmed with the full path — never automatic.
- **Ambiguous target:** state `ambiguous`, with the ordered candidate list. `Enter` opens E's ambiguity chooser with no pre-selection. Under `--no-input`, the command returns `ambiguous_file_node` with candidates and the exact-path form that would settle it. **Never silently repointed** — this is criterion 3B and it is the single most likely place for this feature to violate it.
- **Unsafe target:** absolute path, a `..` escape, a URI scheme, or a symlink escape → `unsafe_file_node`. Recorded, reported by `doctor`, never followed, never opened.
- **`subpath`:** resolved against the target's heading/block projection using G's fragment rules. A resolved file with an unresolved subpath is `resolved_file_unresolved_fragment`; navigation opens the file at its start and says so in words.
- **Rendering an embed in a terminal:** a resolved Markdown target renders through F into the node box, truncated to fit with a `… +N lines` marker and the full text reachable by `Enter`. A resolved non-Markdown target renders as `file: PATH (image/png, 412 KiB)` — a filename is never promoted to a description, and if no alt text exists the line reads `description: unavailable`. Nothing is fetched over the network, ever.
- **Degraded resolution:** tiers 1–3 need only a confined `stat` and work with no index. Tier 4 needs the vault's path set; with B unavailable it returns `requires_index` naming the capability, or performs a bounded confined rescan under explicit `--rescan`. K never opens the SQLite file directly and never treats an index row as file content.

#### Editing and saving

Every mutation follows one path: **plan → preview → precondition check → span splice → atomic write → receipt.**

1. The command computes a `CanvasEditPlan`: an ordered, non-overlapping list of byte-span splices against the document as read, plus the `expected` fingerprint.
2. The preview prints the **complete** span manifest — every span's byte range, JSON pointer, before-lexeme and after-lexeme — with no elision and no "and N more". For a nudge this is one line; for adding a node it is one insertion. A user can always see exactly how much of the file is being touched. `--dry-run` prints the plan and exits, having written nothing.
3. Commit re-reads the file, recomputes the fingerprint, and refuses with `conflict` if it changed — showing both digests and retaining the proposed plan for recovery. It never picks a winner.
4. Splices are applied right-to-left into one buffer and written with **A's existing atomic path** — same-directory temp file, `write_all`, `sync_all`, `rename`, parent-directory `fsync` — the identical `replace_atomic` used for notes. Success, and the word "saved", are printed only after the parent directory sync returns. A partially written or unsynced file never reports success.
5. Inserted fragments are emitted by a minimal fragment writer that matches the document's observed indentation unit, line ending, and array-element separator style. It touches nothing outside the splice.

Autosave in the TUI follows E/D's existing autosave and journal discipline: the canvas buffer is journaled the same way a note buffer is, an external change while dirty raises `EXTERNAL CHANGE` and blocks the save until resolved, and recovery never overwrites a newer source.

#### Rename propagation (branch H)

Canvas file-node references are a link kind and participate in H's link-aware refactors:

1. H's link discovery includes `.canvas` files: it parses each canvas's span tree and yields, for each `file` node, the byte range of the **string literal only** and the resolution state.
2. The rewrite is a span splice on that literal, re-encoded using the *same escape style* observed in the original (a value written with `/` stays escaped; a plain value stays plain). No other byte of the canvas moves — not key order, not geometry, not unknown properties.
3. Canvas splices are ordinary steps in H's write-ahead journal, so a rename touching 12 notes and 3 canvases is **one** all-or-nothing transaction. There is no path where the notes are updated and the canvases are not.
4. Ambiguous and unsafe references are **not rewritten**. They appear in H's `not updated` classification list with their reason, and the refactor still commits with a truthful count. Silence is never used to imply an update happened.
5. `refactor rollback` restores canvases byte-for-byte from H's pre-image store, like any other file.

#### Diagnostics and explicit repair

`canvas doctor` runs ordered checks and reports findings with code, severity, JSON pointer, byte range, and evidence: `invalid_json`, `duplicate_key`, `duplicate_node_id`, `missing_required_property`, `unknown_node_type`, `dangling_edge` (endpoint names no node), `self_edge`, `unresolved_file_node`, `ambiguous_file_node`, `unsafe_file_node`, `unresolved_subpath`, `coordinate_precision`, `zero_or_negative_size`, `unknown_color_token`, `unknown_property` (informational only).

`canvas doctor` **never writes**. Repair is `canvas fix PATH --finding CODE --id ID`, one finding at a time, with the same span preview and confirmation as any edit. There is no bulk fix, no `--all`, and no repair on open.

#### Degraded and unavailable

Any canvas view whose resolution evidence is `stale | rebuilding | degraded | unknown | unavailable` prints the literal state word first, then what still works (opening, outline, geometry editing, exact-path resolution) and what does not (unique-basename resolution, cross-vault reference reports), then one recovery command. Node and edge data never depend on the index — they come from the file — so a canvas is always readable and editable with the index service completely absent.

### 3.3 Layout descriptions

**Canvas outline (canonical, human).** Reading order: header → node blocks → edge summary → footer.

```
canvas  projects/roadmap.canvas   nodes 14  edges 19  groups 2   spec 1.0  conforming
bounds  x -420..1180  y -60..940   (canvas units)

#1  id=1a2b3c4d5e6f7a8b  type=text  [T]
    geometry  x=-420 y=-60 w=260 h=120        color="4" (green)
    text      "Q3 rollout — cold start path"  (1 line, 29 bytes)
    group     member of #12 "Planning" (derived: geometric containment, membership-rule-v1)
    spatial   center=(-290,0)  right→#2 gap 60  below→#5 gap 140  left-aligned with #5
    degree    out 2  in 0
    edge out  #e1 → #2  side right→left  end arrow  label "then"
    edge out  #e2 → #7  side bottom→top  end arrow  label -            color="1" (red)

#2  id=90ab12cd34ef5678  type=file  [F]  [missing]
    geometry  x=-100 y=-60 w=300 h=160        color=-
    file      "notes/cold-start.md"  subpath="#Retry budget"
    target    unresolved  rule=- near_miss="notes/cold_start.md" (byte 11: '-' vs '_')
    group     member of #12 "Planning"
    spatial   center=(50,20)  left→#1 gap 60  right→#3 gap 40
    degree    out 0  in 1
    unknown   "obsidianVersion": "1.5.3"   (bytes 412..441, preserved)

edges: 19 total   resolved 17  dangling 1  ambiguous 1
unknown top-level keys: "metadata" (bytes 12..96, preserved)
canvas: read 8.1 KiB  fingerprint sha256:9f2c…  resolution: index current generation 41
```

Under `--no-graphics`/`--ascii` the sigils and arrows degrade to `[T]`, `->`; the content is unchanged.

**Canvas pane, spatial mode (TUI).** Two stacked regions in one E pane. Top: the deterministic frame, bounded to the visible cells, with off-screen counters on each edge and a viewport line `zoom 40 u/col  center (120,300)  aspect 2.0`. Bottom: the outline rows for the visible-and-off-screen nodes, in canonical order. **Keyboard focus lives in the list**, exactly as G's graph pane specifies; the frame highlights whatever the list has selected. Below 60 columns or without box-drawing support, the frame region is omitted and the pane is the list.

**Edit preview.**

```
canvas edit  projects/roadmap.canvas   operation=node.move  id=1a2b3c4d5e6f7a8b
expected     sha256:9f2c…
spans        2 (no other byte changes)
  bytes 244..247   /nodes/0/x   -420  ->  -380
  bytes 259..261   /nodes/0/y     -60  ->   -20
members      none (node is not a group)
confirm      --yes, or run without --dry-run in interactive mode
```

**Empty states.** `Canvas has no nodes. Add one with: mg-vault canvas node add --type text --text "…"` · `Node #4 has no edges.` · `No canvas files in this vault.` · `canvas doctor: no findings.` A group with no members prints `group #12 "Planning": no members (derived containment found none)` — it states the evidence rather than implying emptiness is certain.

**Data sources.** Node/edge/geometry/unknown data comes only from the file's span tree. Resolution states come from G over B, or from a confined direct `stat` in degraded mode, always labelled with which. Rendering geometry comes from the viewport state, which is session-local and never written to the file.

### 3.4 Input & gestures

- Everything is keyboard-reachable. Mouse, touch, stylus, voice, camera, and controller are **N/A**; if E later adds optional mouse click-to-select in the spatial region it must remain strictly redundant with a key binding.
- **CLI commands:** `canvas outline|nodes|edges|links|render|doctor|fix|open-url`, `canvas node show|add|set|move|raise|lower|remove`, `canvas edge add|set|remove`.
- **Shared read flags:** `--vault`, `--json`, `--jsonl`, `--sort`, `--full`, `--unknown`, `--limit`, `--after`, `--ascii`, `--no-graphics`, `--no-color`, `--no-input`, `--deadline-ms`, `--rescan`.
- **Shared mutation flags:** `--expected FINGERPRINT`, `--dry-run`, `--plan-out FILE`, `--from-plan FILE`, `--yes`. `--expected` is required under `--no-input` unless `--read-current` is passed to make the command's own read the baseline. There is no `--force` and no overwrite flag anywhere in K.
- **Render flags:** `--width`, `--height`, `--zoom`, `--center X,Y`, `--fit`, `--aspect`.
- **Textual-mode keys (canvas pane):** `j`/`k` row, `gg`/`G` first/last, `Enter` follow, `K` detail overlay, `]e`/`[e` cycle edges, `gp` containing group, `gm` cycle members, `/` filter, `s` cycle sort, `o` add node, `O` add edge, `e` edit payload, `r` label, `c` color, `m` geometry, `[`/`]` z-order, `dd` remove (confirmed), `T` toggle spatial, `?` help.
- **Spatial-mode keys:** `h/j/k/l` pan, `H/J/K/L` page pan, `+`/`-` zoom, `zf` fit all, `zs` fit selection, `zz` center, `0` reset, `Tab`/`S-Tab` select, `Enter` follow, `]e`/`[e` edges, `gv` return to list on the same selection. Every binding appears in `:help mg-vault-canvas` and in `canvas --help`, and none of them is the only way to do anything.
- `--no-input` forbids every prompt, chooser, and confirmation; ambiguity and confirmation then become typed errors naming the flag that would settle them.
- **Responsive behavior:** verified at 40, 60, 80, 120, and 300 columns. Below 60 columns the spatial region disappears and node blocks become one `label: value` line each. Ids, paths, JSON pointers, byte ranges, error codes, and recovery commands are **never** truncated — they wrap with indentation.
- `NO_COLOR`, `--no-color`, non-TTY stdout, and `--json` all disable styling; JSON never contains escape sequences.

### 3.5 Transitions & animation

No animation exists anywhere in K. Pan and zoom are single atomic redraws of a deterministic layout — there is no easing, no smooth scroll, and no relaxation step, which is precisely what makes `canvas render` golden-file testable. Mode toggling is an instantaneous state swap owned by E. Long operations (a `--rescan`, `canvas doctor --all` over many canvases) emit rate-limited static progress to stderr in interactive human mode only, at most four updates per second, and emit none at all under `--json`, `--jsonl`, non-TTY output, `--quiet`, `NO_COLOR`, E's screen-reader mode, or a reduced-motion environment setting — where progress becomes a queryable static snapshot instead, so a screen reader is not repeatedly interrupted. There are no sounds and no haptics.

### 3.6 Error states

| Code | Trigger | Presentation and recovery | Data-loss risk |
|---|---|---|---|
| `canvas_invalid_json` | File is not valid JSON | Banner with byte offset, line/column, and the failing token; document opens **read-only**; offer to open as plain text | No — no write path exists in this state |
| `canvas_nonconforming` | Valid JSON, violates JSON Canvas 1.0 | Inline per-node/per-edge marker plus full `doctor` findings; unaffected edits still work | No |
| `duplicate_key` | Two members with the same key in one object | Refuse edits to that key, naming both byte ranges; other edits unaffected | No — refused before any write |
| `duplicate_node_id` | Two nodes share `id` | `doctor` finding listing every occurrence; edges to it are `ambiguous_endpoint`; K never renumbers | No |
| `unknown_node_type` | Payload edit on a node whose `type` is outside the 1.0 set | Name the literal type and the fields K will not interpret; geometry edits remain available | No |
| `missing_required_property` | Node/edge lacks a required 1.0 key | Refuse edits to that node; point at `canvas fix --finding missing_required_property` | No |
| `unresolved_file_node` | Ladder exhausted | Node marked `[missing]`; near-miss list; offer explicit create at the exact path | No — the `file` value is untouched |
| `ambiguous_file_node` | Tier matched >1 candidate | Ordered candidates, no pre-selection, and the exact-path form that resolves it; **never repointed** | No |
| `unsafe_file_node` | Absolute path, `..` escape, URI scheme, symlink escape | Named as rejected with the rule; never followed or opened | No |
| `dangling_edge` | `fromNode`/`toNode` names no node | Edge listed under its own heading with the missing id; still rendered as a stub | No |
| `coordinate_precision` | Arithmetic on a coordinate that would not round-trip | State the original lexeme and refuse; suggest an absolute `--x`/`--y` | No — refused before any write |
| `conflict` | Fingerprint changed between read and commit | Both digests; the plan is retained for recovery; no winner is chosen | No — both versions survive |
| `collision` | `canvas node add --id` names an existing id | Refuse; suggest omitting `--id` to generate one | No |
| `canvas_too_large` | File or node count over the §4.7 caps | Refuse the spatial render and the in-memory span tree; offer streaming `outline --jsonl` | No |
| `render_unavailable` | Terminal too small / no box drawing / `--no-graphics` | Literal message plus the full outline; **capability is unaffected** | No |
| `requires_index` | Unique-basename resolution with B unavailable | Name the unavailable capability and offer `--rescan`; all other views work | No |
| `url_scheme_denied` | `canvas open-url` on a non-allowlisted scheme | Print the exact URL and the denied scheme; nothing is launched | No |
| `transaction_incomplete` | An H journal is active | Transaction id, literal `state: incomplete`, recovery command; blocks canvas mutation | No if the protocol holds |
| `unsafe_path` / `io` / `permission_denied` / `out_of_space` | A path or filesystem failure | Name the rejected operand or the failing phase; never a success verb | No false success |

No error is a timed toast. Every persistent condition is a persistent status flag with a `:messages` entry. No message says `saved`, `moved`, or `durable` unless the parent-directory sync returned successfully. Error JSON carries `version`, `ok:false`, `error.code`, `error.message`, `error.details`, `retryable`, `recovery`, and never contains node text, absolute paths outside the vault, or environment values.

### 3.7 Accessibility

**The textual equivalence contract (Lens 5C, and the Canvas auto-fail rule).** The relationship between the two modes is not "the list is also available"; it is an enforced superset:

- The canvas pane opens in textual mode by default, in every context, for every user.
- `canvas outline` is always available and requires no terminal capability beyond stdout.
- The spatial region is *derived from* the same `CanvasView` records that the outline prints. It has no private data source. A `#[non_exhaustive]` `CanvasFact` enum enumerates every displayable fact (node identity, type, geometry, color, payload, membership, spatial relation, edge endpoints, sides, ends, label, color, resolution state, unknown property); the spatial renderer accepts `CanvasFact` values and cannot construct one the outline printer does not handle, because both consume the same total `match`.
- §5.2's `spatial_is_a_subset_of_textual` test asserts set equality over `CanvasFact` for every fixture at multiple viewports. A renderer that invented a fact would fail the build.
- Every node's and edge's **position, size, direction, containment, and adjacency** are stated as words and numbers in the outline (§3.2 "Textual mode"), because those are exactly the facts a picture encodes spatially. This is why suppressing the drawing costs nothing.

**Screen-reader mode** (E's `--screen-reader` / `MG_VAULT_SCREEN_READER=1`): the spatial region is never drawn; the canvas pane is the outline; each selection change emits one linear announcement in the fixed order path → node ordinal → id → type → label → geometry → membership → degree → resolution state; overlays become numbered linear prompts; no progress counter is emitted. E's existing transcript adapter is reused so the same assertions run headlessly.

**Color-independent state.** Every state-bearing role declares a text token, and the theme loader rejects a theme missing one: node type sigils `T/F/L/G/?`; resolution `[ok]`, `[missing]`, `[ambiguous]`, `[unsafe]`; edge direction words `out`/`in`; selection `[selected]`; dirty `+`; nonconforming `!`. Canvas `color` values are *content*, not state: they are always printed as their literal token (`"4" (green)`, `"#FF00AA"`) and never used as the only signal for anything. The 16-color and monochrome capability tests assert the text plane alone distinguishes every state.

**Focus and keyboard navigability.** Focus is always defined and always lives in the list, in both modes. Overlay focus is trapped and `Esc` returns to the prior focus. Every interactive element is reachable by the documented grammar and appears in E's command palette with its binding and availability reason.

**Text scaling** is N/A in a terminal; the user scales by changing terminal font size, which K observes only as a different cell count. Correct behavior at any cell count is the requirement, tested at 300×100, 120×30, 80×24, 60×20, 40×10, and 24×8.

**Unicode safety.** Labels and node text are wrapped and truncated only at grapheme-cluster boundaries; wide glyphs are never bisected by a box border (the boundary cell is blanked and a `>` continuation marker drawn). Bidi controls and other invisible characters in labels are rendered as visible escapes (`\u{202e}`) so a label cannot spoof a path or reorder a line, while the underlying bytes are untouched. Escaping affects display only and never identity or the saved file.

---

## 4. Implementation Specification

### 4.1 Architecture placement

- **`crates/mg-vault-canvas`** (new workspace member) — the whole feature except terminal drawing and CLI parsing. It has no terminal, index, or CLI dependency and can be driven headlessly.

```text
crates/mg-vault-canvas/src/
├── lib.rs
├── jsonspan.rs      # lossless RFC 8259 lexer/parser -> span tree; NO serializer
├── view.rs          # CanvasView, NodeView, EdgeView, UnknownProperty over spans
├── model.rs         # JSON Canvas 1.0 surface types, canvasColor, sides/ends
├── derive.rs        # membership-rule-v1, spatial relations, z-order, degree
├── resolve.rs       # file-node resolution via the G resolver contract
├── edit.rs          # CanvasEditPlan, SpanEdit, fragment writer (fragments only)
├── layout.rs        # viewport projection, rasterization, orthogonal edge routing
├── outline.rs       # canonical textual projection + CanvasFact enumeration
└── doctor.rs        # ordered conformance checks
```

- **`crates/mg-vault-core`** gains, and remains the only filesystem authority:
  - `vault.rs` — `validate_note_path` generalizes to `validate_vault_path(path, kind)` over a `VaultFileKind` allowlist (`Markdown` = `.md`, `Canvas` = `.canvas`, plus the attachment kinds F needs). All existing rules — relative, normal components only, no `.obsidian`/`.mg-vault` first component on mutation, symlink rejection, canonical containment — are unchanged and shared. Today's `.md`-only check is the single blocker to `.canvas` being a first-class file, and it is a widening of an allowlist, not a weakening of a check.
  - `atomic.rs` — unchanged. K uses `replace_atomic` exactly as notes do.
  - A new `Vault::apply_span_edits(relative, edits: &[SpanEdit], expected) -> Result<SourceFingerprint>` generalizing `edit_note_span` from one span to N non-overlapping spans in one file, one buffer, one atomic replace. It is byte-oriented and imposes no UTF-8 span-boundary rule beyond the existing one.
- **`crates/mg-vault-cli`** gains a `canvas` command module. It owns no filesystem policy and no canvas semantics.
- **`crates/mg-vault-tui`** gains `panes/canvas.rs`, consuming `mg-vault-canvas` records. E owns pane geometry, focus, and session restore; K owns the model, ordering, labels, and rendering rules.
- **`crates/mg-vault-index`** may project canvas file-node references as link rows so G/H can find them without opening every canvas. Those rows are disposable and never authoritative: H re-parses the canvas from source before placing any edit.

Authority is unchanged: `mg-vault-core` is the only writer, the index is disposable, and `mg-vault-canvas` holds no write path of its own — it produces plans that core executes.

### 4.2 Data model

```rust
/// A canvas file as read: the exact bytes, plus a lossless index into them.
/// There is deliberately no `to_string`/`serialize` for this type.
pub struct CanvasDocument {
    pub path: VaultRelativePath,
    pub bytes: Vec<u8>,
    pub fingerprint: SourceFingerprint,
    pub spans: SpanTree,
    pub style: DocumentStyle,     // indent unit, line ending, BOM, trailing newline
    pub conformance: Conformance, // Conforming | Nonconforming(Vec<Finding>) | InvalidJson(ParseError)
}

/// Lossless JSON structure. Every variant stores byte ranges into `CanvasDocument::bytes`;
/// no scalar is decoded until a caller asks, and the raw lexeme is always recoverable.
pub enum SpanNode {
    Object { span: Range<usize>, members: Vec<Member> },   // document order preserved
    Array  { span: Range<usize>, items: Vec<SpanNode> },   // z-order for `nodes`
    String { span: Range<usize> },                          // includes quotes and escapes
    Number { span: Range<usize> },                          // exact lexeme, never re-formatted
    Bool   { span: Range<usize> },
    Null   { span: Range<usize> },
}

pub struct Member { pub key_span: Range<usize>, pub key: String, pub value: SpanNode }

/// Read-only projection. Every field points back at a span, so any edit is a splice.
pub struct NodeView<'d> {
    pub ordinal: u32,               // 1-based array position; also the z-order
    pub id: Cow<'d, str>,
    pub kind: NodeKind<'d>,         // Text | File | Link | Group | Unknown(&str)
    pub rect: Option<Rect>,         // None when a required coordinate is missing
    pub rect_spans: RectSpans,
    pub color: Option<CanvasColor<'d>>,
    pub unknown: Vec<UnknownProperty<'d>>,   // key, raw lexeme, byte range
    pub span: Range<usize>,
}

/// Derived, never stored in the file. Carries its rule id so it cannot be mistaken
/// for an authored property.
pub struct Membership { pub group_ordinal: u32, pub rule: &'static str /* "membership-rule-v1" */ }

/// Every fact any mode may display. The outline printer and the spatial renderer
/// both consume this enum exhaustively; the renderer cannot invent a variant.
#[non_exhaustive]
pub enum CanvasFact { NodeIdentity{..}, NodeType{..}, Geometry{..}, Color{..}, Payload{..},
                      Membership{..}, SpatialRelation{..}, EdgeEndpoints{..}, EdgeSides{..},
                      EdgeEnds{..}, EdgeLabel{..}, Resolution{..}, UnknownProperty{..} }

/// One byte-span replacement. Spans are non-overlapping and sorted; applied right-to-left.
pub struct SpanEdit { pub span: Range<usize>, pub replacement: Vec<u8>, pub pointer: String }

/// A complete, previewable, hashable proposal. Not authorization: commit revalidates it.
pub struct CanvasEditPlan {
    pub schema: u16,                 // 1
    pub operation: CanvasOperation,
    pub path: VaultRelativePath,
    pub expected: SourceFingerprint,
    pub edits: Vec<SpanEdit>,
    pub affected_ids: Vec<String>,   // e.g. every member of a moved group
    pub plan_hash: PlanHash,
}

/// Session-local. Never written to the .canvas file.
pub struct Viewport { pub center_x: f64, pub center_y: f64, pub units_per_col: u32, pub cell_aspect: f64 }
```

**No database, no migration, no schema.** A canvas is a file. The index's canvas link rows are disposable projections with the existing `schema_version`/`parser_version` discipline; deleting the database and rebuilding reproduces them exactly. **No identifier is injected into any Markdown note**, and node ids are never used as note identity.

### 4.3 API contracts

Library (side-effect-free until commit):

```rust
fn open(vault: &Vault, path: &VaultRelativePath) -> Result<CanvasDocument, CanvasError>;
fn view<'d>(doc: &'d CanvasDocument) -> CanvasView<'d>;
fn outline(view: &CanvasView, opts: OutlineOptions) -> OutlineReport;      // canonical text + facts
fn facts(view: &CanvasView) -> Vec<CanvasFact>;                            // total enumeration
fn render(view: &CanvasView, vp: Viewport, caps: TerminalCaps) -> Frame;   // pure, deterministic
fn resolve(view: &CanvasView, r: &dyn LinkResolver) -> Vec<FileNodeResolution>;
fn doctor(doc: &CanvasDocument, r: Option<&dyn LinkResolver>) -> Vec<Finding>;
fn plan(doc: &CanvasDocument, op: CanvasOperation) -> Result<CanvasEditPlan, CanvasError>;
// commit lives in core, because core is the only writer:
fn Vault::apply_span_edits(&self, rel, edits: &[SpanEdit], expected: &SourceFingerprint)
    -> Result<SourceFingerprint>;
```

`plan` never touches the filesystem. `render` is a pure function of `(view, viewport, caps)` — this is what makes golden-frame tests possible. `LinkResolver` is G's trait; K depends on the contract, not on an implementation, so it can be tested with a fake resolver before G ships.

**CLI signatures** (all accept the global `--vault/--json/--no-input/--no-color`):

| Command | Key params | Errors | Notes |
|---|---|---|---|
| `canvas outline PATH` | `--sort --full --unknown --json --jsonl --limit --after` | `canvas_invalid_json`, `canvas_too_large` | Canonical view; read-only |
| `canvas nodes\|edges PATH` | as above | same | Subsets of outline |
| `canvas node show --id ID` | `--json` | `not_found` | Includes unknown properties |
| `canvas render PATH` | `--width --height --zoom --center --fit --ascii` | `render_unavailable`, `canvas_too_large` | Deterministic frame + off-screen manifest |
| `canvas links PATH` | `--rescan --allow-stale` | `requires_index` | Resolution states, never a rewrite |
| `canvas doctor [PATH\|--all]` | `--check CODE` | — | Never writes |
| `canvas node add` | `--type --x --y --width --height [--text\|--file\|--url\|--label] [--id] --expected --dry-run --yes` | `collision`, `conflict`, `unsafe_path` | Appends one array element |
| `canvas node set --id ID` | `--x --y --width --height --text --file --url --label --color --raw KEY=JSON` | `unknown_node_type`, `duplicate_key`, `coordinate_precision`, `conflict` | One splice per changed key |
| `canvas node move --id ID` | `--dx --dy [--with-members]` | `coordinate_precision`, `conflict` | Group move lists every affected node in the preview |
| `canvas node raise\|lower --id ID` | — | `conflict` | Array-element move splice |
| `canvas node remove --id ID` | `--yes` | `dangling_edge_would_result` | Refuses until incident edges are handled |
| `canvas edge add` | `--from --to [--from-side --to-side --label --color --from-end --to-end]` | `not_found`, `collision`, `conflict` | |
| `canvas edge set\|remove --id ID` | as applicable | `conflict` | |
| `canvas fix PATH --finding CODE` | `--id --yes` | — | One finding at a time; no bulk |
| `canvas open-url --id ID` | `--yes` | `url_scheme_denied` | Allowlist `http`, `https`, `mailto`; `--no-input` requires `--yes` |

**Auth/permissions:** vault write permission plus A's confinement. No network, no rate limiting, no remote endpoint. **Pagination:** `--limit`/`--after` with opaque cursors bound to `(path, fingerprint, sort)`; a fingerprint change returns `cursor_expired` rather than mixing two documents.

**JSON envelope** reuses the existing `{"version":1,"ok":true,"command":"canvas.outline","data":{…},"warnings":[]}` contract on stdout, errors on stderr, other stream empty. `data` for read commands always carries `path`, `fingerprint`, `canvas_spec_version`, `conformance`, `resolution_freshness`, `derived_from: "canvas_file"`, then `nodes`/`edges`/`unknown_top_level`. `unknown` arrays are always present, even when empty. **Exit codes** follow C's categories: `0` success, `2` usage, `3` not found, `4` collision/ambiguity/conflict, `5` unsafe/denied, `6` degraded dependency, `7` I/O or transaction failure, `130` interrupt.

### 4.4 State management

- **Authoritative:** the `.canvas` file. Every read re-reads it; nothing is cached across commands.
- **Owned by `CanvasDocument`:** the bytes and span tree for the life of one command or one open TUI buffer. The TUI buffer holds one `CanvasDocument` plus a dirty edit list; it is journaled by D's existing autosave/recovery journal exactly like a note buffer, so a crash mid-edit is recoverable and recovery **never overwrites a newer source** (D's `source changed since journal base — merge required` state applies unchanged).
- **Session-local, never persisted to the file:** `Viewport`, selection, sort order, filter, and mode. These go into E's `WorkspaceSnapshot` (content-free, `.mg-vault/workspaces/`), so reopening a workspace restores the view without ever having written to the user's canvas. A canvas file is **never** modified by looking at it.
- **Derived, non-authoritative:** membership, spatial relations, degree, rendered frames, and resolution states. Every one is labelled with the rule version or index generation it came from.
- **Local vs. server-synced:** N/A — there is no server. All state is local files plus process memory.
- **Concurrency:** every write carries the read-time fingerprint; a mismatch is `conflict` and both versions survive. Multi-file work (a rename touching notes and canvases) is owned by H's journal, and K contributes steps rather than running its own transaction.

### 4.5 Dependencies

- **New packages: none required.** The span-preserving JSON parser is written in-repo (`jsonspan.rs`, ~600 lines, RFC 8259) rather than added as a dependency, because no widely-used JSON crate preserves whitespace, key order, number lexemes, escape forms, and duplicate keys simultaneously — and this is the exact property the feature exists to guarantee. This mirrors A's accepted precedent of using `yaml-edit` only to *locate* spans. `unicode-segmentation`/`unicode-width` (already needed by D/E for grapheme-safe rendering) are reused for label wrapping.
- **Existing crates reused:** `serde`/`serde_json` for the CLI envelope only (never for the canvas file), `sha2` for fingerprints, `thiserror`, `clap`, `rustix`.
- **New assets:** none. No fonts, no images, no icons; box-drawing characters come from the terminal font.
- **Infrastructure:** none. No database, no network, no third-party service. Fixture `.canvas` files are added to the repo test corpus with their provenance recorded (§6.2).

### 4.6 Platform-specific considerations

- **Terminal capability** is the main variable. Box drawing, Unicode width, and 256-color support are detected by E's existing capability probe; K consumes the report and never probes independently. Missing box drawing degrades to ASCII; a terminal below 60×20 suppresses the spatial region. `:capabilities` shows the detection and its evidence.
- **Cell aspect ratio** cannot be queried portably. It is a documented setting (`canvasaspect`, default `2.0`) rather than a guess, and it affects only the projection — never the file.
- **Line endings and BOM** are preserved as read. A CRLF canvas stays CRLF; K's fragment writer emits the document's observed ending. There is no normalization anywhere.
- **Case-insensitive filesystems** affect file-node resolution: comparison stays byte-exact (a case-differing target is `unresolved` with a `near_miss`, matching G's deliberate, documented divergence from Obsidian), while collision checks for any file K creates also use case-folded comparison.
- **Confinement backend** is A's: today's canonicalize-and-recheck, with the `VaultDir`/`openat2` descriptor-relative backend as the accepted target. K inherits whatever gate A enforces and adds no path handling of its own; where A blocks mutation, K blocks canvas mutation.
- **Version compatibility:** Rust edition 2024, `rust-version = "1.85"`, `unsafe_code = "forbid"`, `clippy::all` + `clippy::pedantic` denied — the workspace lints apply unchanged.
- **Feature flags:** the canvas pane lives behind E's existing `feature = "canvas"` flag. When the feature is not built, the palette lists canvas commands as `unavailable: feature not built` — the same honest degradation mechanism as any other, with no hidden capability. The CLI `canvas` commands are not flag-gated.

### 4.7 Performance budget

Reference: warm local SSD, reported with hardware/OS metadata. A typical canvas is far smaller than a vault, so budgets are tight.

- **Parse + span tree:** p95 ≤ 30 ms for a 2 MiB file with 1,000 nodes and 2,000 edges; ≤ 5 ms for a typical 50-node file. Parsing is one pass, O(bytes).
- **Outline render:** p95 ≤ 50 ms for 1,000 nodes including derived membership and spatial relations. Membership is O(nodes × groups); with a cap of 200 groups this is bounded, and above it K switches to a sweep-line pass to stay O(n log n).
- **Spatial frame:** p95 ≤ 16 ms at 300×100 cells, so an interactive pan stays at one frame per keypress with headroom. Rendering is O(visible cells + edges), never O(vault).
- **Edit plan + commit:** p95 ≤ 100 ms for files ≤ 2 MiB, excluding unavoidable `fsync` variance. Durability is never weakened to meet a latency number.
- **Memory:** the span tree is roughly 2–3× the file size in indices; total working set ≤ 4× file size. Process budget ≤ 20 MiB RSS for CLI commands beyond the document itself.
- **Hard caps (fail closed, never silently truncate):** file > 20 MiB, nodes > 50,000, or edges > 100,000 returns `canvas_too_large`; streaming `outline --jsonl` remains available so the content is never unreachable.
- **Cancellation:** parse, doctor, rescan, and outline check for cancellation at least every 50 ms or 1 MiB. Cancelling emits `complete: false` and never a partial result presented as whole.
- **Startup impact:** zero. Nothing in K runs unless a `.canvas` path is opened. K adds no work to the note save path and no work to vault startup.
- **Storage:** K writes nothing outside the user's own canvas file, except E's content-free workspace snapshot.

---

## 5. Test Specification

### 5.1 Unit tests

| Test | Setup and assertion |
|---|---|
| `span_tree_covers_every_byte` | Property test over generated JSON: the union of all span ranges plus inter-token whitespace equals `0..len`; no byte is unaccounted for. |
| `no_change_save_is_byte_identical` | Open each corpus fixture, apply an empty edit list, save; assert output bytes equal input bytes exactly, including BOM and trailing newline. |
| `edit_touches_only_listed_spans` | For every supported single-property edit on every fixture: assert the diff against the original is exactly the plan's span set. |
| `unknown_top_level_key_survives` | Fixture with `"metadata": {...}`; move a node; assert the key, its position, and its bytes are unchanged. |
| `unknown_node_and_edge_properties_survive` | Fixture with vendor properties on nodes and edges; edit neighbors; assert byte equality of the unknown members. |
| `key_order_is_preserved` | Fixture with `type` before `id` and `height` before `width`; edit `x`; assert member order is unchanged. |
| `number_lexemes_are_not_reformatted` | Fixture with `1.0`, `-0`, `1e2`, `0.30000000000000004`; edit an unrelated key; assert every untouched lexeme is byte-identical. |
| `string_escape_style_is_preserved` | Fixture with `"/path"`; rewrite it via rename; assert the escape style is retained. |
| `duplicate_key_edit_is_refused` | Object with two `"x"` members; `node set --x` returns `duplicate_key` and writes nothing. |
| `unknown_node_type_geometry_only` | `type: "kanban"` node: geometry edit succeeds, payload edit returns `unknown_node_type`, all properties survive. |
| `invalid_json_is_read_only` | Truncated fixture: open reports offset/line/column, every mutating API returns an error, no temp file is created. |
| `membership_rule_v1_is_exact` | Table of containment cases — inside, edge-touching, partial overlap, nested groups, equal rects — asserting membership, ordering, and that overlap is never membership. |
| `spatial_relations_are_deterministic` | Shuffle array order; assert nearest-neighbor, alignment, and overlap sets are identical for identical geometry. |
| `zorder_is_array_order` | `raise`/`lower` move exactly one array element and change no other byte. |
| `resolution_ladder_matches_g` | Shared fixture table with G: same targets produce the same tier, rule, state, and candidate list. |
| `ambiguous_file_node_is_never_rewritten` | Two `alpha.md`: assert `ambiguous`, candidates listed, and the file bytes unchanged after every read and navigation attempt. |
| `unsafe_file_node_is_refused` | `/etc/passwd`, `../outside.md`, `file:///x`, symlink-to-outside: all `unsafe_file_node`, never opened. |
| `render_is_pure_and_deterministic` | Same `(view, viewport, caps)` renders byte-identical frames across 1,000 runs and across array shuffles that preserve geometry. |
| `zoom_to_fit_is_reproducible` | For each fixture, `zf` selects the same ladder step and center every time, and the projected bounding box fits. |
| `no_node_or_edge_is_silently_dropped` | Random viewports: on-screen + off-screen + below-render-size counts always equal the document totals. |
| `ascii_fallback_is_information_equal` | Unicode and ASCII frames enumerate identical `CanvasFact` sets. |
| `node_ids_never_leave_the_file` | Grep-style assertion over every write path: no node id is ever written into a `.md` file or a link target. |
| `plan_hash_binds_semantics` | Mutate each of path, operation, span set, replacement bytes, expected fingerprint; hash changes. Change only preview text; hash does not. |

### 5.2 Integration tests

- `obsidian_round_trip`: open every Obsidian-authored fixture, perform each supported edit, and assert (a) the file still parses in a strict JSON Canvas 1.0 validator, (b) every unmodelled byte is unchanged, (c) a checked-in Obsidian re-read fixture is byte-comparable.
- `textual_covers_every_json_leaf`: walk each fixture's span tree, enumerate every leaf position, and assert set equality with the positions reachable in `canvas outline --full --unknown --json`. **An unmodelled property fails the build.**
- `spatial_is_a_subset_of_textual`: for each fixture × {5 viewports} × {unicode, ascii} × {80×24, 40×10, 300×100}, assert `facts(rendered) ⊆ facts(outline)` and, for the fit viewport, set equality.
- `atomic_save_fault_matrix`: inject failure at temp-create, `write_all`, `sync_all`, `rename`, and parent `fsync`; assert the canvas is byte-identical to either the pre-image or the committed image, never anything else, and that success is printed only after the final sync returns.
- `conflict_preserves_both`: modify the file externally between read and commit; assert `conflict`, both digests reported, the plan retained, and zero bytes written.
- `rename_updates_canvas_transactionally`: H renames a note referenced by 3 canvases and 40 notes; assert one journal, all 43 files updated or none, every canvas byte-identical outside the `file` string span, and `refactor rollback` restores all 43 byte-for-byte.
- `rename_skips_ambiguous_canvas_reference`: an ambiguous file node is listed under `not updated` with its reason, the refactor still commits, and the canvas bytes are unchanged.
- `crash_during_refactor_with_canvas`: kill at every journal phase; recovery converges to the complete pre- or post-state including canvases.
- `degraded_without_index`: stop the index service; outline, render, geometry edits, and tiers 1–3 resolution all work; tier 4 returns `requires_index`; `--rescan` produces the same answers as the indexed run.
- `nonconforming_is_never_repaired`: run every read command and every unrelated edit against nonconforming fixtures; assert `doctor` reports the findings and no byte changes without an explicit `canvas fix`.
- `large_canvas_budgets`: 1,000-node / 2,000-edge fixture meets the §4.7 parse, outline, and frame budgets; the 60,000-node fixture returns `canvas_too_large` and remains readable through `outline --jsonl`.
- `json_contract_fixtures`: every command's success, empty, warning, and error envelope matches a golden fixture, including always-present empty `unknown` arrays.
- `url_node_is_never_auto_opened`: assert no code path launches a URL without explicit invocation, and that denied schemes are refused with the exact URL echoed.

### 5.3 UI / E2E tests

Driven through E's headless backend and transcript adapter, so every scenario is assertable without a real terminal:

1. Open a `.canvas` file → the pane opens in **textual mode**, and the transcript's first announcement names the path, node count, and conformance.
2. Navigate every node and edge with `j/k`, `]e/[e`, `gp`, `gm`; assert every node, edge, label, membership, and spatial relation in the fixture was announced.
3. Toggle to spatial, pan to each corner, `zf`, `zs`, `zz`, `0`; return with `gv` and assert selection is preserved and identical in both modes.
4. Add a node, add a labelled edge, move a group with members, recolor, delete a node with incident edges (refused), then delete after removing edges — each with its preview and confirmation.
5. Follow a resolved file node into an editor pane; attempt to follow an ambiguous one and assert the chooser has no pre-selection and `Esc` changes nothing; attempt an unresolved one and assert the create offer names the exact path.
6. Run at 24×8, 40×10, 60×20, 80×24, 300×100: assert the spatial region disappears at the two smallest sizes and that no information becomes unreachable.
7. Screen-reader mode end-to-end: assert no box-drawing byte is ever emitted and that the fact set announced equals the fact set of the full outline.
8. External modification of the open canvas → `EXTERNAL CHANGE`, save blocked, merge path offered, no silent overwrite.
9. `--no-input` over every mutating command: no prompt, typed errors naming the missing flag, zero writes.
10. Every command in human, `--json`, `--jsonl`, `--ascii`, `--no-graphics`, `--no-color`, and `NO_COLOR` modes.

### 5.4 Visual / manual verification

- Light and dark terminal themes, 256-color, 16-color, and monochrome: strip the attribute plane and confirm every state is still distinguishable from text alone.
- Terminal font sizes from very small to very large (the terminal's text-scaling equivalent), verifying only cell-count behavior matters.
- Widths 40, 60, 80, 120, 300 and heights 8, 10, 20, 24, 100, with empty, single-node, typical (50-node), and dense (1,000-node) canvases.
- Kitty, foot, Alacritty, xterm, tmux, and `TERM=dumb`; confirm the ASCII fallback and the suppression path both look deliberate rather than broken.
- Canvases with CJK and RTL labels, combining marks, emoji, and bidi controls: confirm no bisected glyphs, no border corruption, and visible escapes for controls.
- Open a canvas mg-vault edited in Obsidian and confirm by eye that layout, colors, groups, and all cards are unchanged.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-targets --all-features`.

---

## 6. Compliance & Safety Gate

### 6.1 Sensitive data classification

- [ ] No sensitive data involvement
- [x] **Handles sensitive data** — canvas node text, labels, file paths, and URLs are private user content. Protections: everything stays local with no network access of any kind; node text and labels are emitted only in explicitly requested output and never in logs, `doctor` findings, error details, plan artifacts, or telemetry (there is none); error messages carry JSON pointers and byte ranges, never content; plan artifacts written by `--plan-out` contain replacement bytes and are therefore created owner-only and documented as private; `canvas render` output contains content by definition and is treated exactly like `note read`.
- [ ] Uses synthetic/test data only until compliance gate clears

### 6.2 Asset provenance

- [ ] No third-party assets
- [x] **Uses third-party assets** — the test corpus contains `.canvas` fixtures. Each fixture is either (a) authored by this project for testing, or (b) a minimal structural sample generated by Obsidian on a synthetic vault with no user content. Every fixture is recorded in `crates/mg-vault-canvas/tests/fixtures/PROVENANCE.md` with its origin, generating tool and version, and license status; no fixture contains third-party creative content, and none is copied from a public vault without a recorded license. **JSON Canvas 1.0 itself is an open specification (MIT) implemented, not vendored.** Box-drawing characters are Unicode code points supplied by the user's terminal font, not bundled assets.

### 6.3 Language / claims audit

- [x] **No claim unsupported by evidence.** Every preservation claim in this spec is tied to a named test in §5. "Byte-faithful" means the §5.1 diff assertions pass, not a design intention.
- [x] **No promise of unbuilt capability.** §7.1 states that this entire branch is **absent** today. Under a build without G, `canvas links` returns `requires_index` rather than implying resolution occurred; without E, the CLI works and the pane is reported `feature not built`.
- [x] **No restricted-domain language.** K makes no medical, financial, legal, or safety claim.
- [x] **"Atomic" is scoped and tested.** It refers to A's single-file same-directory temp + `rename` + parent `fsync` protocol, verified by `atomic_save_fault_matrix`; multi-file atomicity is H's journal and is named as such.
- [x] **"Canvas" is not overclaimed.** The spec says a terminal viewport with quantized zoom and orthogonal edge routing — not a GUI canvas, not free-form drawing, not curved edges.

### 6.4 Regulatory alignment

The template points at Lens 3, but K touches every lens, so each binding criterion is addressed explicitly:

| Criterion | K-owned acceptance |
|---|---|
| **1A Authority** | The `.canvas` file is the only source. No index row supplies node or edge data; deleting the database changes nothing about what a canvas shows or how it saves. |
| **1B Preservation** | No whole-document serializer exists. Edits are byte-span splices over a lossless span tree; key order, whitespace, number lexemes, escape forms, unknown top-level keys, unknown node/edge properties, BOM, and line endings all survive, asserted by `no_change_save_is_byte_identical`, `edit_touches_only_listed_spans`, and the unknown-property tests. |
| **1C Identity** | Node ids are file-internal handles, explicitly documented as *not* note identity. File paths remain public identity; no id is ever written into a note; authored ids are never renumbered; `node_ids_never_leave_the_file` enforces it. |
| **1D Coexistence** | `.obsidian` is never read or written by K; view state (viewport, selection, sort) lives in `.mg-vault/workspaces/`, never in the canvas file. An Obsidian-authored canvas round-trips. |
| **1E Transactions** | Fingerprint precondition on every write; `collision` on duplicate id; `conflict` preserves both versions; A's atomic replace; canvas steps join H's journal for multi-file work. |
| **2A Keyboard completeness** | Every capability, including all spatial ones, has a documented key binding and a CLI equivalent; no mouse-only action exists. |
| **2B Editing durability** | Canvas buffers use D's autosave and recovery journal; recovery refuses to overwrite a newer source; external change while dirty blocks the save. |
| **2C Workspace** | The canvas pane is a first-class E pane with textual/spatial modes, session restore of view state, and unresolved-path placeholders. |
| **2D Text correctness** | Labels and node text wrap and truncate only at grapheme boundaries; wide glyphs are never bisected; escapes are display-only and never change bytes. |
| **2E Degraded experience** | No box drawing, a narrow terminal, `--no-graphics`, or screen-reader mode costs the drawing and nothing else; invalid JSON still opens as text; no index still permits editing. |
| **3A Determinism** | Resolution, membership, spatial relations, ordering, and rendering are pure functions of source bytes plus versioned rules; array shuffles that preserve geometry produce identical derived output. |
| **3B Ambiguity** | An ambiguous file node is reported with ordered candidates and **never chosen, never repointed** — not on open, not on navigation, not during a rename. |
| **3C Query depth** | `canvas links`, `canvas nodes --sort`, and `--filter` cover paths, types, labels, colors, resolution states, and group membership; K feeds canvas edges into G's graph so canvases are reachable from the ordinary query surface. |
| **3D Derived authority** | Membership, relations, frames, and index link rows are all views; each is stamped with its rule version or generation and none can write to the file. |
| **3E Scale** | Canvas work is O(document), never O(vault); K adds zero work to the note save path; hard caps fail closed with a streaming alternative. |
| **4A Confinement** | A's path authority, extended by allowlist to `.canvas`; unsafe file-node targets are refused, never followed; group `background` paths are confined identically. |
| **4B Concurrency** | Read-time fingerprints on every write; conflicts preserve both versions; cursors are fingerprint-bound so pages never mix documents. |
| **4C Least privilege** | K has no plugin or AI surface; `open-url` is explicit with a scheme allowlist; no `file:`/`javascript:`/`data:` is ever launched; nothing in a canvas can cause execution. |
| **4D Recovery** | Every mutation is previewable with a complete span manifest, `--dry-run` writes nothing, commits are atomic, deletions route through trash where a whole file is removed, and no message claims success before the final sync. |
| **4E Contracts** | Versioned envelope v1, stable `data` shapes, always-present `unknown` arrays, golden fixtures, and C's exit-code categories. |
| **4F Privacy** | No network. Node text and labels never appear in logs, diagnostics, or error details; plan artifacts are owner-only; no query or view history is persisted. |
| **5A Offline/local-first** | Every K workflow is local file I/O. Nothing is fetched — not embedded images, not `url` nodes, not group backgrounds. |
| **5B Responsiveness** | §4.7 gives parse, outline, frame, commit, memory, cancellation, and hard-cap budgets; zero startup cost. |
| **5C Accessible equivalents** | The textual outline is canonical, is the default, states every geometric and relational fact in words, and is proven a superset of the drawing by `spatial_is_a_subset_of_textual` plus the JSON-leaf coverage test. |
| **5D Terminal resilience** | ASCII fallback, 40-column and 24×8 layouts, box-drawing suppression, color-independent tokens, control-character escaping, and no animation at all. |
| **5E Automation** | Human, `--json`, `--jsonl`, stdin/stdout discipline, `--no-input`, `--no-color`, and `NO_COLOR` across every command. |

**Auto-fail review.** *Source-content loss:* prevented structurally — no serializer exists, edits are span splices, and byte-equality is a CI gate. *Partial multi-file mutation:* canvas edits are single-file and atomic; multi-file work runs inside H's journal. *Index state overriding source / stale index presented as current:* node and edge data never come from the index; resolution states carry freshness and fail closed. *Silent conflict winner:* fingerprint mismatch is a terminal `conflict` retaining both. *Unknown syntax loss:* the central guarantee of this spec — unknown top-level keys, node properties, edge properties, types, color tokens, key order, and number lexemes all survive, with a leaf-coverage test that fails the build if K stops modelling something. *Unconfirmed overwrite/import:* no `--force`, no overwrite flag, no bulk fix, no repair on open; `--no-input` requires `--yes`. *Unsafe traversal or symlink escape:* A's confinement plus refusal of absolute, `..`, scheme, and symlink-escaping file-node targets. *Capability/data-exfiltration bypass:* no network, no process spawn except the explicit allowlisted `open-url`, no plugin surface. *Active raw HTML/script by default:* node text renders through F's deny-by-default sanitizer as passive text; nothing in a canvas is executable. *Non-atomic save claiming success:* success is printed only after parent-directory `fsync` returns, proven by the fault matrix. *Recovery overwriting newer source:* D's journal-base comparison applies to canvas buffers unchanged. *Graph or Canvas information lacking a textual equivalent:* the textual outline is canonical, default, and proven a superset of the drawing.

---

## 7. Gap Analysis vs. Current State

### 7.1 What exists today

- The entire K branch is **absent**. There is no `mg-vault-canvas` crate, no JSON span parser, no canvas model, no viewport or renderer, no outline, no canvas CLI command, and no canvas pane. Nothing in the repository mentions Canvas outside `docs/PRODUCT.md`'s target-state list and the feature tree, both of which correctly describe it as a target rather than a claim.
- **Actively blocking:** `crates/mg-vault-core/src/vault.rs::validate_note_path` requires `path.extension() == Some("md")` for both reads and mutations, so a `.canvas` file cannot currently be read or written through vault authority at all. `crates/mg-vault-core/src/interop.rs::collect_paths` likewise filters to `.md`, so canvases are invisible to the snapshot export.
- **Implemented and directly reusable:** `crates/mg-vault-core/src/atomic.rs` (`replace_atomic`: same-directory temp, `write_all`, `sync_all`, `rename`, parent `fsync`) — this is exactly the save path K requires, unchanged. `Vault::read_note`, `SourceFingerprint`, `existing_mutation_path`, `prepare_destination`, and `ensure_beneath` are the confinement and concurrency primitives K inherits.
- **Prototyped:** `Vault::edit_note_span` is the single-span ancestor of K's multi-span splice — the byte-preserving edit model K needs already exists in the codebase at library level, with tests, and has no CLI verb.
- **Prototyped (precedent):** `frontmatter_scalar.rs` uses `yaml-edit` to *locate* a scalar's span and never to serialize. K's "parse for spans, never serialize" rule is a direct generalization of an approach already accepted and shipped in this repo.
- **Absent and depended upon:** the TUI (E), the editor engine (D), Markdown rendering (F), and the link resolver (G) are all absent, so the canvas pane, embedded Markdown rendering, and tier-4 resolution have no backing implementation yet.

### 7.2 Delta to spec

**New files / modules**
- `crates/mg-vault-canvas/` — new workspace member with the nine modules listed in §4.1, plus `tests/fixtures/` and `PROVENANCE.md`.
- `crates/mg-vault-cli/src/commands/canvas.rs` — the `canvas` command tree.
- `crates/mg-vault-tui/src/panes/canvas.rs` — the canvas pane (lands with E).

**Modified files**
- `crates/mg-vault-core/src/vault.rs` — generalize `validate_note_path` to `validate_vault_path(path, VaultFileKind)` over an extension allowlist; add `Vault::apply_span_edits`. All existing rules are preserved and shared, not relaxed.
- `crates/mg-vault-core/src/lib.rs` — export `VaultFileKind`, `SpanEdit`, `apply_span_edits`.
- `crates/mg-vault-core/src/error.rs` + `mg-vault-cli/src/main.rs::error_code` — add the K error variants and their stable codes.
- `crates/mg-vault-core/src/interop.rs` — include `.canvas` files in `collect_paths` so they appear in the snapshot with their file-node references as links.
- `crates/mg-vault-index/src/lib.rs` — project canvas file-node references as disposable link rows (`parser_version` bump).
- `Cargo.toml` — add the new workspace member.
- `README.md`, `docs/PRODUCT.md`, `docs/ARCHITECTURE.md`, `docs/SECURITY.md` — record the canvas file kind and the span-edit model once they land, not before.

**Migrations / schema changes**
- No note-content migration and no canvas-file migration of any kind; K never rewrites a file it was not explicitly asked to edit. The index `parser_version` bump forces a rebuild of a disposable projection, which is the existing, accepted mechanism.

**New dependencies**
- None. The span-preserving parser is written in-repo for the reason in §4.5.

### 7.3 Estimated scope

**L.** The command surface is moderate and the data model is small, but three parts are genuinely hard and cannot be rushed because each sits on an auto-fail boundary: (1) the lossless JSON span parser and the splice engine, where the whole 1B guarantee lives; (2) the textual-completeness machinery — the `CanvasFact` enumeration and the leaf-coverage and subset tests that make 5C structurally true rather than aspirational; (3) the deterministic terminal projection, including edge routing, degradation, and the golden-frame tests. Recommended slicing, each independently reviewable and shippable: (a) `VaultFileKind` + `apply_span_edits` in core, with the fault matrix; (b) `jsonspan` + round-trip corpus gate; (c) view, derive, outline, `CanvasFact`, and the CLI read commands; (d) mutations with plan/preview/commit; (e) resolution and `doctor` against a fake `LinkResolver`; (f) layout, renderer, and golden frames; (g) the E pane; (h) H integration for rename propagation.

### 7.4 Blocking dependencies

- **A (foundation) — hard blocker, small.** `VaultFileKind` and `apply_span_edits` must land first. Everything else in A that K needs is already implemented.
- **G (links, search, graph) — soft blocker.** K depends on the `LinkResolver` *contract*, not its implementation, and ships against a fake resolver. Without G, tiers 1–3 work by direct `stat` and tier 4 returns `requires_index`. K must not implement a second resolver.
- **H (safe note refactoring) — required for rename propagation only.** Until H lands, a rename does not update canvases, and C's existing `links_updated: false` disclosure covers it truthfully. K must never claim canvas references are maintained before H's journal can carry the step.
- **E (TUI) and D (editor) — required for the pane only.** The CLI half of K is complete without them; the pane is feature-gated and reported `feature not built`.
- **F (Markdown rendering) — required for rendering embedded note content inside a file node.** Without F, a file node shows its resolved path, size, and type instead of rendered content, labelled as such rather than shown blank.
- **B (index) — optional.** Needed only for tier-4 resolution and for finding canvases without walking the vault.

### 7.5 Non-goals

- No GUI, no mouse-driven canvas, no free-form drawing, no curved or bezier edges, no automatic layout, no force-directed arrangement, and no animation.
- No JSON Canvas extension of our own: K does not invent properties, does not add a `parent` key to record group membership, and does not write a `mg-vault` block into a canvas.
- No canvas creation wizard, no template canvases, no conversion of a Markdown note into a canvas or back.
- No image decoding, no thumbnailing, no sixel or kitty-graphics rendering of embedded images in this slice.
- No network fetch of any kind, including `url` node previews and remote group backgrounds.
- No automatic repair of nonconforming files, no bulk fix, and no formatter or pretty-printer for `.canvas` files.
- No canvas-to-canvas linking semantics beyond ordinary file nodes, and no cross-vault canvas references.
- No plugin or AI authority over canvas contents; any future AI proposal must arrive as a previewed `CanvasEditPlan` and cannot construct a commit.

---

## 8. Open Questions

- **Q1:** When a group node is moved, should contained nodes move with it by default? Obsidian's GUI moves them, so `--with-members` defaulting to true matches user expectation and Obsidian round-trip behavior; but it makes one keystroke edit N nodes' coordinates, which is the largest span set any K command produces. The spec currently defaults to moving members **with the complete affected-node list shown in the preview**. Confirm the default. — **blocks:** §3.2 (group move), §4.3 (`canvas node move`).
- **Q2:** `cell_aspect` cannot be queried portably and is currently a documented setting defaulting to `2.0`. Should K instead offer a one-time interactive calibration (draw a square, ask the user to adjust until it looks square) stored in XDG config? It improves fidelity but adds a stateful setup step to a feature that otherwise has none. — **blocks:** §3.2 (projection), §4.6.
- **Q3:** Should `canvas node add` generate ids in Obsidian's observed form (16 lowercase hex characters) for maximum interoperability, at the cost of implying a compatibility guarantee JSON Canvas 1.0 does not actually make, or use an obviously distinct prefixed form that is self-identifying but visibly foreign in a shared vault? The spec assumes the former with uniqueness checked within the file. — **blocks:** §4.2, §5.1 `node_ids_never_leave_the_file`.
- **Q4:** For a canvas whose file node targets an attachment F cannot render (PDF, video), should the node box show file metadata (type, size, modified time) or only the path plus `description: unavailable`? Metadata is more useful; it is also the only place in K where a `stat` result appears as content. — **blocks:** §3.2 (embed rendering), §6.1.
- **Q5:** Should `.canvas` files participate in the disposable index's link projection by default, or only under an explicit opt-in? Projecting them makes canvases visible to `backlinks` and the graph, which is clearly desirable; it also means canvas node text enters the index, which the privacy exclusions in §6.4 must then cover as carefully as note text. — **blocks:** §4.1 (index projection), §7.2, and G's graph node set.
- **Q6:** External dependency, not a design question: H must land before any user-visible statement that canvas references are maintained across renames. Until then, is the correct behavior to (a) stay silent, or (b) have `canvas links` warn that a rename will not update these references? The spec assumes (b), on the grounds that silence about a known gap is the failure mode this product is trying to avoid. — **blocks:** §3.2 (rename propagation), §6.3.
