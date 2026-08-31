# Spec: TUI Workspace

**Feature ID:** e-tui-workspace
**Parent feature:** root
**Spec author agent:** Hermes TUI-workspace spec subagent
**Date:** 2026-08-29
**Iteration:** 1

---

## 1. Purpose

### 1.1 One-sentence job

Give a keyboard-only user a full-screen terminal workspace — panes, splits, tabs, named workspaces, source and preview views, pins, a command palette, and a truthful status line — so that many notes can be read, navigated, and edited in one session without a mouse and without ever mistaking a preview, an index result, or a recovered buffer for the authoritative file.

### 1.2 Why it matters

`mg-vault` already has, or has specified, the parts that make a knowledge system: file authority (A), a disposable index and search (B), a non-interactive CLI (C), and a Unicode-correct modal editing engine (D). None of them is a place to *work*. Spec D is deliberately headless: it produces buffer, command-line, merge, recovery, and diagnostic projections and explicitly defers "terminal rendering, panes/tabs/splits, source preview, themes, terminal key decoding, or workspace/session UI" to E. Spec C likewise disclaims cursor state, panes, preview, and session persistence and hands E a stable exact-path/fingerprint seam.

E is therefore the only component that turns those contracts into an integrated workflow comparable to Obsidian's multi-pane workspace, while keeping Neovim's modal precision. It is also the only component that renders anything, which makes it the single place where four failure classes can appear for the first time: a terminal that cannot express what the design assumes (color depth, Unicode, key encoding); a screen-only affordance with no textual equivalent; a status line that says "current" when the index is behind; and a session restore that resurrects stale or recovered content over newer source. This spec exists to make each of those impossible by construction rather than by care.

Scope: panes/splits/tabs; the focus model; source, preview, and linked source-preview views; named workspaces and session restore; navigation, pickers, and pins; the command palette; the keymap grammar and its composition with D's Vim grammar; the status line; themes; terminal capability detection and degradation; accessibility.

### 1.3 Success signal

The Milestone 5 acceptance suite passes on a headless terminal backend across a full capability matrix (truecolor/256/16/none × UTF-8/ASCII × kitty/legacy keys × 300×100 / 80×24 / 40×10 / 24×8), and in every cell of that matrix: every command in the palette is reachable by keyboard, the status line still shows mode, dirty/conflict state, and the literal index-freshness word; every pane type produces a byte-identical textual projection under `--text` and under screen-reader mode; and a scripted session that crashes mid-edit, is restarted, and restores a two-tab split workspace never applies a recovery journal without an explicit acceptance and never writes over source that changed since the journal's base.

---

## 2. User Stories

> As a keyboard-first writer, I want to split a pane, open a second note beside the first, and move focus between them without lifting my hands, so that comparing and transcluding notes is as fast as editing one.

> As a Vim user, I want E's window and workspace keys to layer on top of D's modal grammar without shadowing any buffer command, so that `Ctrl-w` in Insert mode still deletes a word and `Space` still reaches the buffer whenever a command is pending.

> As a returning user, I want `mg-vault ui` to restore the tabs, splits, view modes, pins, and cursor anchors I left, so that a session is a place I come back to rather than something I rebuild.

> As a user whose machine lost power mid-sentence, I want the recovery offer shown *before* my workspace is restored, with the words "source changed since this journal" when it applies, so that recovery never silently overwrites an edit I made elsewhere.

> As a user on a locked-down remote host with `TERM=xterm`, no truecolor, no kitty keyboard protocol, and a Latin-1 locale, I want a named list of exactly what is unavailable and a working alternative binding for each, so that source editing and every workflow still function.

> As a screen-reader user, I want every pane, the split geometry, the status line, the graph, and the Canvas to have a complete linear textual mode, so that nothing meaningful is encoded only in position, color, or box drawing.

> As a user searching a 100,000-note vault, I want the picker to stay responsive and to say "partial" or "stale" rather than silently returning fewer results, so that an empty result never reads as "no such note".

> As an automation author, I want the same workspace and layout state available as versioned JSON through `mg-vault ui --dump-session` and `:layout --json`, so that I can script and assert workspace behavior without scraping a rendered frame.

---

## 3. UX Specification

### 3.1 Screen / view inventory

The TUI occupies one terminal in the alternate screen. There are no OS windows. The inventory below is panes, overlays, and bars.

**Chrome (always present, not focusable panes):**

| Element | Entry point | New/modified | Layout |
|---|---|---|---|
| Tab bar | Top row; hidden when one unnamed tab exists and `showtabline=auto` | New | Single row, horizontally scrolling, ordinal + name + dirty marker word |
| Pin bar | Row under tab bar; `:set pinbar=on` (default `auto`: shown when ≥1 pin) | New | Single row, ordinal-prefixed pinned note names |
| Status line | Bottom row (or two rows when narrow) | New | Fixed segment order, §3.3 |
| Message line | Bottom-most row, shared with D's command line | New | Last message; full history in `:messages` |

**Panes (focusable, live in a tab's split tree):**

| Pane | How to open | New/modified | Backing contract |
|---|---|---|---|
| Editor (source) | `:e PATH`, picker, link follow, session restore | New (renders D) | D buffer view model |
| Preview | `:preview`, `<leader>v` toggle, `:vsplit +preview` | New | F render model over the same D buffer revision; read-only |
| Linked source+preview | `<leader>V` | New | A split pair with a scroll/anchor link |
| Outline | `<leader>o` | New | D's ordered outline projection (Tree-sitter derived) |
| Backlinks / links | `<leader>b` | New | B/G read-only query results |
| Search results | `<leader>/`, `:search` | New | B query results with freshness metadata |
| Diagnostics | `<leader>d` | New | D `VersionedDiagnostics` |
| Merge | Opened automatically when D reports `merge required` | New | D `MergeSession` hunks |
| Recovery | Startup, or `:recover` | New | D `RecoveryOffer` projection |
| Graph (textual-first) | `<leader>g` | New | G adjacency records; §3.7 |
| Canvas (textual-first) | Opening a `.canvas` file | New | K node/edge records; §3.7 |
| Unresolved placeholder | Session restore of a missing/moved path | New | Recorded path + actions; never auto-creates a file |

**Overlays (modal, centered, dismissed with `Esc`; exactly one at a time):**

| Overlay | Key | Purpose |
|---|---|---|
| Command palette | `<leader><leader>`, `:palette`, `Ctrl-Space` (enhanced keyboards only) | Fuzzy list of every command with title, ID, binding, availability, and reason if unavailable |
| Quick open (path picker) | `<leader>f` | Fuzzy path/title open; ambiguity resolved explicitly |
| Buffer/pane switcher | `<leader>e` | Open buffers with dirty/conflict words |
| Workspace manager | `<leader>w` | List/save/load/delete named workspaces |
| Pin list | `<leader>p` | Ordered pins; reorder, jump, unpin |
| Full status | `<leader>s` | Every status segment as labeled lines; the narrow-terminal fallback |
| Keymap help | `F1`, `:help keys` | Complete keymap grammar, per-namespace, with availability |
| Capability report | `:capabilities` | Detected terminal capabilities, source of each detection, and what is degraded |
| Confirmation | Raised by destructive actions | Explicit text prompt; never timed |
| Link ambiguity chooser | Raised by following an ambiguous link | Ordered candidates; no default selection |

`Ctrl-o`/`Ctrl-i` (D's jump list) work inside a pane; E adds no competing overlay for them.

### 3.2 Interaction flows

#### Startup

1. `mg-vault ui [--vault NAME] [--workspace NAME]` resolves the vault through the existing registry (`VaultRegistry`, `XdgPaths`) exactly as the CLI does. No vault selected and none supplied → a text error naming `mg-vault vault select`, exit code 2, no alternate screen entered.
2. Capability detection runs before any drawing (§3.4). `TERM=dumb`, a non-TTY stdout, or a terminal smaller than 20×6 refuses the TUI and prints the equivalent CLI invocation. Between 20×6 and 40×10 the TUI starts in **constrained mode**: one pane, no splits, two-row status, and a message saying so.
3. The recovery scan runs **before** workspace restore. If D reports any recovery candidate for this vault, the recovery pane opens as the first and only pane. Nothing is applied. Each row shows path, journal timestamp, recoverable edit count, and one of three literal states: `source unchanged since journal base`, `source changed since journal base — merge required`, or `journal corrupt — partial prefix recoverable`. Actions per row: inspect, recover to dirty buffer (only offered in the first state), open merge (offered in the second), export copy, discard journal (requires confirmation naming the path). `Continue without recovering` leaves every journal intact.
4. Workspace restore then runs (§3.2 "Workspace save and restore").
5. First interactive frame is drawn after the focused pane's buffer is open. Other panes render `loading…` and load on demand or in the background.

#### Split, focus, resize, close

1. `Ctrl-w s` splits the focused pane horizontally, `Ctrl-w v` vertically. The new pane shows the same buffer at the same cursor position and takes focus (configurable with `splitfocus`).
2. `Ctrl-w h/j/k/l` moves focus in a direction using the geometric rule in §3.3. `Ctrl-w w`/`W` cycles forward/backward in deterministic tree order. `Ctrl-w p` returns to the previously focused pane. `Ctrl-w t`/`b` goes to first/last.
3. `Ctrl-w H/J/K/L` moves the focused pane to the far edge; `Ctrl-w r`/`R` rotates siblings; `Ctrl-w x` exchanges with the sibling.
4. `Ctrl-w +`/`-`/`<`/`>` resize by one row/column (`{count}` multiplies), `Ctrl-w =` equalizes, `Ctrl-w _`/`|` maximizes one axis, `Ctrl-w o` closes all other panes in the tab (confirmation if any closed pane holds a dirty buffer), `Ctrl-w z` toggles zoom (a reversible full-tab view that preserves the tree).
5. `Ctrl-w c` closes the focused pane. Closing the last pane holding a dirty buffer does **not** close it: the pane stays, the status shows `dirty — save or explicitly discard`, and a confirmation lists the exact actions. Closing a pane never saves implicitly and never discards a recovery journal.
6. If a resize or a terminal shrink makes the requested geometry impossible, panes are not silently dropped. Panes below minimum size collapse into a **stack**: the focused pane renders, the others become one labeled row each listing name and dirty word, and the status line adds `layout: stacked (N hidden)`. `Ctrl-w z` and focus movement still reach every pane. Growing the terminal restores the exact prior geometry.

#### Tabs

`Ctrl-w T` moves the focused pane into a new tab; `:tabnew [PATH]`, `gt`/`{count}gt`, `gT`, `:tabmove {n}`, `:tabclose`. Each tab owns its own split tree, focus, and previous-pane pointer. Tab names default to the focused pane's note title, truncated at the tab bar but never in `:tabs`. `:tabclose` on a tab containing dirty buffers raises the same explicit confirmation as pane close.

#### Source, preview, and the linked pair

1. `<leader>v` toggles the focused editor pane between `source` and `preview` for the *same buffer revision*. Preview is read-only: any editing key in preview mode produces `preview is read-only — press <leader>v for source` and no mutation. This is a deliberate, documented refusal, not a silent no-op.
2. `<leader>V` creates a linked pair: source left, preview right (stacked source-above-preview below 80 columns). The pair shares one buffer; the preview re-renders from D's snapshot after each committed transaction, debounced to at most 10 Hz, and is labeled `preview: revision N` — never `revision N` when it is showing N−1.
3. Scroll linking anchors on the nearest enclosing block (heading, paragraph, list item, fence) rather than on line numbers, so wrapped and folded content stays aligned. `:set previewsync=off` unlinks.
4. If F (Markdown rendering) is unavailable, degraded, or fails on a document, preview does not go blank. It falls back in this order: (a) F's partial render with a `preview: partial — N blocks unrendered` label; (b) D's outline projection; (c) plain source text with `preview unavailable — showing source`. Source editing is never blocked by a preview failure.
5. Preview renders **passive text only**. Raw HTML, scripts, and any control sequence embedded in content are inert (§4.6, §6.4).

#### Navigation

1. `gd` or `Enter` on a link/embed under the cursor follows it. Resolution comes from G through B. A unique resolution opens the target per `linkopen` (default: reuse focused pane, `Ctrl-w gd` opens in a split, `<leader>gd` in a new tab).
2. An **ambiguous** link opens the ambiguity chooser with every candidate path in deterministic order and no pre-selection; `Esc` cancels and nothing changes. An **unresolved** link offers `create note at PATH` — a create action that goes through C's collision-safe create and is always confirmed with the exact path.
3. If the index is not `current`, link following still works but the chooser and the target header carry the literal freshness word, and `create` is refused with `index not current — resolve with an exact path or refresh` because creating from a stale resolution can produce a duplicate.
4. `<leader>f` (quick open) queries B for path/title candidates, streaming as the user types, with a per-keystroke deadline (§4.7). Results are labeled `current`, `stale`, `partial`, or `direct scan` and never mixed silently. When B is unavailable, quick open falls back to a bounded confined directory scan, explicitly labeled `direct scan — index unavailable`.
5. `Ctrl-o`/`Ctrl-i` traverse D's per-buffer jump list. E adds a *pane* history (`<leader><`/`<leader>>`) recording which buffer each pane showed, so closing and reopening a note is reversible.

#### Pins

`<leader>P` toggles a pin on the focused buffer. Pins are an ordered, workspace-scoped list of vault-relative paths. `<leader>1`–`<leader>9` jump to pins 1–9; `<leader>p` opens the pin list where pins can be reordered (`J`/`K`), jumped (`Enter`), opened in a split (`s`/`v`), or removed (`x`). A pinned path that no longer resolves is shown as `pin 3: notes/old.md — missing` and is never removed automatically. Pins are content-free: only paths are stored.

#### Command palette

1. `<leader><leader>` opens the palette. It is populated from a single command registry that merges D's command IDs, E's TUI command IDs, and the C/B adapter command IDs.
2. Each row is `title · command-id · binding · availability`. Availability is `available`, or `unavailable: <reason>` with reasons such as `requires index (current: stale)`, `requires terminal capability: enhanced-keyboard`, `requires feature: markdown-render (F)`, `requires a merge session`. Unavailable commands are shown, not hidden, so the workspace is self-documenting under degradation.
3. Filtering is fuzzy over title and command ID with a deterministic score, ties broken by command ID byte order. `Enter` executes in the focused pane's context; `Ctrl-y` copies the ID; `Ctrl-k` starts an interactive rebind.
4. Any command that mutates goes through its owning contract (D's dispatch, C's plan/commit). The palette never has a private write path.

#### Workspace save and restore

1. A **workspace** is a named, portable snapshot: tab order, per-tab split tree with normalized weights, per-pane kind, vault-relative path, view mode, fold state, cursor anchor and scroll anchor, focused pane, pin list, and a `schema_version`.
2. `:workspace save NAME` writes `.mg-vault/workspaces/NAME.json` through a dedicated privileged writer (the same pattern as vault-local trash) using A's atomic same-directory replacement. Ordinary note-edit APIs still reject `.mg-vault` paths.
3. Machine-local, non-portable state — last terminal size, per-`TERM` width probe results, message history, recently-used ordering, last-active workspace — is written to `$XDG_STATE_HOME/mg-vault/session/<vault-key>/` and never into the vault. `.obsidian` is never read for layout and never written.
4. `:workspace load NAME` and startup restore both follow the same rules: every path is revalidated through A's path authority and re-read through D's `TextStore`; a path that is missing, moved, now a directory, outside the vault, or inside a protected control directory becomes an **unresolved placeholder pane** naming the recorded path with actions `locate`, `remove from workspace`, `create`. Restore never creates a file, never resurrects buffer content from the session file (the session file holds no content, only anchors), and never applies a recovery journal.
5. Cursor and scroll anchors are stored as `(line, grapheme_column, surrounding_context_hash)`. On restore, if the context hash does not match, the cursor is placed at the nearest valid grapheme boundary of the recorded line and the pane reports `cursor position approximate — file changed since last session`. It is never placed inside a grapheme.
6. Autosave of the current workspace (`autoworkspace`, default on) rewrites the last-used workspace file atomically on layout change, debounced 2 s. A write failure produces a persistent `workspace not saved` status flag, never a claim of success.

#### Quit

`:q` closes a pane, `:qa` quits. `:qa` with dirty buffers is refused and opens a list of every dirty/conflicted/merge-blocked buffer with per-row actions. `:qa!` discards in-memory buffers but explicitly **does not** delete recovery journals; the message says so. On any exit path — including panic — the terminal is restored: alternate screen left, keyboard protocol flags popped, bracketed paste and mouse disabled, cursor shown. The panic hook restores the terminal first, then prints a report containing no buffer content.

### 3.3 Layout descriptions

#### Split tree and geometry solver

Each tab holds a tree: `Leaf(PaneId)` or `Split { axis: Horizontal | Vertical, children: Vec<(Node, weight: u32)> }`. Geometry is solved top-down over integer terminal cells:

1. The tab's content rectangle is total rows minus tab bar, pin bar, status rows, and message row.
2. A split distributes its extent across children proportionally to weight, using the **largest-remainder method** with ties broken by child index. The result is deterministic and the child extents always sum exactly to the parent extent — no rounding gaps, no overlap.
3. Separators consume one cell between siblings (a box-drawing line, or `|`/`-` in ASCII mode).
4. Minimum pane content size is 20 columns × 3 rows. A subtree that cannot meet the minimum for all children collapses to the stack described in §3.2.
5. `Ctrl-w =` resets all weights to equal. Resizes adjust the focused pane's weight and the adjacent sibling's, preserving the parent's total.

Directional focus (`Ctrl-w h/j/k/l`): among panes whose rectangles overlap the focused pane's perpendicular span and lie in the requested direction, choose the nearest edge; tie-break by the larger overlap, then by tree order. If no such pane exists, focus does not move and the message line says `no pane to the left` (etc.). The rule is stated in `docs/TUI.md` and property-tested, so focus movement is reproducible rather than heuristic.

#### Editor pane

Top → bottom: optional one-row pane header (path, view mode word, dirty word) shown when `paneheader=auto` and more than one pane exists; then the text viewport; then, in the focused pane only, D's command line when active. Leading → trailing inside the viewport: optional sign column (diagnostic severity letter, merge-hunk marker, mark letter), optional line-number column (absolute or relative), a one-cell gutter, then text. Data source is D's buffer view model, exclusively.

Empty states: a pane with no buffer shows `No note open. <leader>f to open, <leader><leader> for commands.` An empty note shows `(empty note)` on the first line. An unresolved placeholder shows `Unresolved: notes/moved.md — recorded in workspace "daily"` plus the three actions.

#### Status line

Fixed segment order, each with a literal word — color is decoration only:

`1 mode` · `2 vault` · `3 path + view mode` · `4 buffer state` · `5 position` · `6 selection` · `7 pending input` · `8 search/diagnostics` · `9 index freshness` · `10 capability badge` · `11 workspace name`

- Segment 4 is one of `clean`, `dirty`, `pending durability`, `saving`, `saved`, `SAVE FAILED`, `EXTERNAL CHANGE`, `MERGE REQUIRED (n unresolved)`, `RECOVERY AVAILABLE`, `READ-ONLY`, taken directly from D. It is never inferred.
- Segment 9 is the literal index freshness from B/`mg-vault-index`: `idx:current g<generation>`, `idx:stale`, `idx:degraded`, `idx:empty`, `idx:unavailable`, or `idx:rebuilding`. E never renders a bare dot, badge, or color for this; a stale or degraded index is never rendered with the same text as a current one, and E has no code path that substitutes `current` for an unknown value.
- Segment 10 appears only when something is degraded: `caps:ascii,16color,legacy-keys` etc. Selecting it (`<leader>s` → capability section, or `:capabilities`) explains each.

Width policy, by terminal columns:

| Width | Status rendering |
|---|---|
| ≥ 100 | All segments, full path |
| 60–99 | Path abbreviated to leaf plus one parent; segments 6, 11 dropped first |
| 40–59 | Two rows: row A = 1, 4, 9, 10; row B = 3, 5, 7, 8 |
| 20–39 | Two rows, non-droppable set only: mode, buffer state, index freshness, capability badge, then `… <leader>s` |

Segments 1, 4, 9, and 10 are **never** dropped and never abbreviated below a word that a reader can distinguish (`stale` never becomes `st`). If they cannot fit, the terminal is below the operating floor and constrained mode applies.

#### Overlays

Overlays are centered, at most 80% of each dimension, with a one-row title, a filter input, a scrollable list, and a one-row key hint. Below 50 columns they occupy the full width. The list is always keyboard-scrollable (`Ctrl-n`/`Ctrl-p`, `Ctrl-d`/`Ctrl-u`, `Home`/`End`) and always announces `showing X of Y` so truncation is visible.

### 3.4 Input & gestures

#### Keymap grammar and composition with D

The grammar is `[{count}] {namespace-prefix}* {key}`, with four namespaces and a strict routing precedence evaluated per key event:

1. **Overlay namespace** — when an overlay is open, it consumes every key except a documented set (`Esc`, `Ctrl-c`). No key reaches D.
2. **Global namespace** — `F1` (help) and `Ctrl-c` (cancel current operation) in any mode. Deliberately tiny.
3. **Window namespace** — prefix `Ctrl-w`, claimed **only in Normal and Visual modes**. In Insert and Replace modes `Ctrl-w` is D's delete-word-backwards and E never sees it. This is the concrete rule that keeps E from shadowing the buffer.
4. **Leader namespace** — prefix `<Space>`, claimed **only in Normal mode and only when D reports an empty pending state** (no count, operator, register prefix, pending `f`/`t`, pending text object, or active macro recording). E queries D's `PendingInput` projection before claiming the key; if anything is pending, `Space` is forwarded to D unchanged. Rebinding `<Space>` in Normal mode away from D's default `l` is a documented deliberate difference from Vim, listed in `docs/TUI.md`.
5. **Buffer namespace** — everything else goes to D's `dispatch` verbatim, including `:`, `/`, `?`, all operators, motions, text objects, counts, registers, and macros.

E registers its own commands with D's command line, so `:split`, `:vsplit`, `:tabnew`, `:tabclose`, `:preview`, `:workspace {save|load|list|delete}`, `:pin`, `:palette`, `:layout`, `:tabs`, `:status`, `:capabilities`, `:theme`, `:messages`, and `:set {tui-option}` become *known* commands rather than D's `UnsupportedCommand`. Unknown commands still fail closed with no mutation.

Keymaps are user-configurable in `$XDG_CONFIG_HOME/mg-vault/keymap.toml` (machine) and `.mg-vault/keymap.toml` (portable, vault-scoped; machine wins on conflict, and the override is reported by `:help keys`). The loader validates that every mapping resolves to a known command ID, rejects a mapping that would shadow a buffer-namespace key in Insert/Replace mode unless `allow_insert_override = true` is set on that entry, and reports **every** conflict as an error with both entries' file/line rather than picking a winner. A keymap file that fails to load leaves the defaults active and raises a persistent status flag.

Every default binding, its command ID, and its namespace are in one generated table in `docs/TUI.md`, and `:help keys` renders that same table. There is no undocumented binding.

Counts compose: `{count}gt` selects tab N, `{count}Ctrl-w +` grows by N, `{count}<leader>1` is rejected (leader-digit bindings take no count) with a named error rather than a surprising action.

#### Mouse and other input

Mouse is **off by default**. `:set mouse=on` enables click-to-focus, click-to-position-cursor, wheel scroll, separator drag-resize, tab-bar click, and overlay row click. Every one of these has a keyboard equivalent listed in the same `docs/TUI.md` row, and the acceptance suite asserts the set of mouse-reachable commands is a strict subset of the keyboard-reachable set — **zero mouse-only paths**. Touch, stylus, voice, camera, and game-controller input are N/A for a terminal application. There is no haptic or audio output; the terminal bell is never emitted.

#### Terminal capability detection

Detection runs once at startup and again on `SIGWINCH`-adjacent reconfiguration or `:capabilities refresh`. Every result records its evidence source so `:capabilities` can explain it.

**Color depth**, first match wins:

1. `NO_COLOR` present with any value (including empty) → `none`.
2. `--color=never|always|auto` or `MG_VAULT_COLOR` → explicit.
3. `COLORTERM` ∈ {`truecolor`, `24bit`} → truecolor.
4. terminfo `RGB` or `Tc` capability, or `TERM` ending in `-direct` → truecolor.
5. terminfo `colors` ≥ 256, or `TERM` containing `256color` → 256.
6. terminfo `colors` ≥ 8 → 16.
7. Otherwise → `none` (attributes only).
8. `TERM=dumb` or absent `TERM` → refuse the TUI, print the CLI equivalent.

Degradation: truecolor theme values downsample to 256 by nearest-cube quantization; to 16 by the theme's **declared** ANSI fallback per role (never automatic quantization, which destroys contrast); to `none` by the theme's declared attribute fallback (bold / reverse / underline / dim). Because every state-bearing role also declares a text token (§3.7), `none` loses no information.

**Unicode:** the charset from `LC_ALL`/`LC_CTYPE`/`LANG` must be UTF-8; otherwise ASCII mode. ASCII mode replaces box drawing (`│─┌┐└┘├┤┬┴┼` → `|-+`), status markers, list bullets, and separators with ASCII, and escapes any non-ASCII content byte for display as `\u{...}` while leaving the buffer untouched. Display width uses `unicode-width`; East-Asian ambiguous width is `:set ambiwidth=single|double` (default `single`) with a `:probe width` command that draws a calibration string and asks the user, storing the answer per-`TERM` in machine-local state.

**Bracketed paste:** enabled via DECSET 2004. If the terminal ignores it, pasted text arrives as ordinary keys and would be interpreted as commands. Mitigation: E detects an input burst (≥ 32 events within 20 ms with no intervening idle) and, in Normal mode, suspends command interpretation, buffers the burst, and raises `possible paste detected — Enter to insert literally, Esc to discard, or enable bracketed paste`. In Insert mode the burst is inserted literally, which is the safe interpretation. `:set pasteguard=off` disables the heuristic for users who script keystrokes.

**Keyboard protocol:** on startup E sends the kitty keyboard-protocol query (`CSI ? u`) and waits at most 150 ms. On a positive reply it pushes flags for disambiguated escape codes, event types, and alternate keys, enabling `Ctrl-Space`, `Ctrl-;`, `Shift-Enter`, unambiguous `Esc`, and distinct key-release. Without it, legacy encoding applies and:

- `Alt-x` is indistinguishable from `Esc` then `x`. E resolves this with a bounded `escdelay` (default 25 ms) **only in Normal and Visual modes**. In Insert and Replace modes `Esc` is dispatched immediately with no wait, because a delayed mode exit is unacceptable; consequently no default binding uses `Alt` in Insert mode.
- `Ctrl-Space`, `Ctrl-;`, `Shift-Enter`, `Ctrl-Enter`, and key-release are unavailable. Each is listed in the palette as `unavailable: requires enhanced-keyboard`, and **each has a documented non-modifier alternative** in the leader, window, or ex namespace. The acceptance suite asserts that with enhanced-keyboard disabled, the set of reachable commands is unchanged.

**Other capabilities:** alternate screen (required; absent → refuse with explanation); synchronized output DEC 2026 (absent → ordinary redraw, no functional loss, possible tearing on large repaints); focus in/out events (absent → autosave idle timer runs on its own clock instead of pausing on blur); OSC 11 background query for light/dark auto-detection (absent → `MG_VAULT_THEME`, then default dark); OSC 52 clipboard, which D owns and which stays opt-in — E only reports whether the terminal advertises it and never enables it silently.

`:capabilities` prints every capability, the detected value, the evidence (`env COLORTERM=truecolor`, `terminfo colors=256`, `query reply CSI?1u`, `default`), and, for each degraded one, the exact list of affected commands and their alternatives.

#### Responsive behavior

| Columns × rows | Behavior |
|---|---|
| ≥ 120 × 30 | Full: multi-split, tab bar, pin bar, single-row status, linked source+preview side by side |
| 80 × 24 | Full, linked pair still side by side; pin bar `auto` |
| 60–79 | Linked pair stacks vertically; tab bar scrolls; path abbreviated |
| 40–59 | Two-row status; overlays full width; splits allowed only while every pane meets the 20×3 minimum |
| 20–39 or < 10 rows | Constrained mode: one pane, no splits, non-droppable status only, overlays full screen |
| < 20 × 6 | Refuse; print the CLI equivalent |

### 3.5 Transitions & animation

There is no animation. Pane, tab, overlay, and mode changes are instantaneous full-state redraws of the affected regions. There are no fades, slides, spinners, or progress bars.

The only time-dependent visuals are: (a) the message line, which holds the last message until replaced or dismissed and is never auto-cleared on a timer — the full history is in `:messages`, so no message can be missed; (b) long-operation progress, which is a static counter updated at most 2 Hz (`indexing 1200/100000`) and is suppressed entirely under screen-reader mode, `--reduce-motion`, or `MG_VAULT_REDUCE_MOTION`; (c) cursor blink, which E never controls — it is the terminal's setting.

Reduced-motion behavior is therefore identical to default behavior except for the progress counter, and E declares reduced motion as its baseline rather than an alternate path. No information is conveyed by timing.

### 3.6 Error states

| Trigger | Presentation (and why) | Recovery path | Data-loss risk |
|---|---|---|---|
| Terminal too small / `TERM=dumb` / not a TTY | Plain stderr text before alternate screen; a TUI cannot present its own failure to start | Resize, set `TERM`, or use the CLI command printed | No |
| Terminal capability missing | Persistent status capability badge + `:capabilities` detail; persistent because it affects every later keystroke | Use the listed alternative binding; enable the capability | No |
| Keymap file invalid or conflicting | Persistent status flag + `:messages` entry with both conflicting entries and file/line; not a modal, because defaults still work | Fix the file, `:keymap reload` | No |
| Theme file invalid or low-contrast | Status flag + `:theme check` report; fall back to `mg-dark` | Fix or choose another theme | No |
| Pane cannot meet minimum size | Inline `layout: stacked (N hidden)` in status; inline because it is a continuous condition | Resize, close a pane, `Ctrl-w z` | No |
| Buffer open failed (permission, not UTF-8, is a directory) | Inline in the pane, with D's exact error name; the pane is the natural place | Open externally, choose another path | No |
| Save failed (D reports it) | Persistent `SAVE FAILED` status segment **and** an inline pane banner; a transient toast would be unacceptable for a durability failure | Retry, save copy, inspect journal (all D actions) | No false success; buffer stays dirty and journaled |
| External change while dirty | `EXTERNAL CHANGE` status, merge pane opens on the next save attempt | Resolve hunks in the merge pane | No; both versions preserved |
| Merge unresolved | `MERGE REQUIRED (n unresolved)`; save is blocked by D | Resolve or abort | No |
| Recovery candidate present | Recovery pane at startup, before restore; modal because it must precede any workspace state | Inspect / recover / merge / export / discard | No; never auto-applied |
| Recovery journal base older than current source | Row labeled `source changed since journal base — merge required`; the recover action is **not offered** | Open merge | No; recovery cannot overwrite newer source |
| Index unavailable / stale / degraded / rebuilding | Literal word in status segment 9 and on every result set; index-dependent commands marked unavailable in the palette | Continue with direct-scan fallback, or `mg-vault index rebuild` | No |
| Index result set truncated by deadline | Result header `partial — deadline reached at N results` | Raise `--deadline-ms`, refine query | No |
| Ambiguous link | Chooser overlay with no pre-selection; modal because a wrong silent choice is unrecoverable | Choose or cancel | No |
| Preview render failure | Pane-level `preview unavailable — showing source` with the failing block range | Continue in source | No |
| Workspace file unreadable / wrong schema version | Startup message + empty workspace; original file untouched and never rewritten | Fix or `:workspace load` another | No |
| Workspace autosave write failed | Persistent `workspace not saved` status flag | Retry, `:workspace save` elsewhere | No note content at risk (workspaces hold no content) |
| Content contains terminal control sequences | Rendered as visible escapes (`^[`, `\u{...}`) in source; stripped in preview; never emitted raw | None needed | No |
| Panic | Terminal restored first, then a report with no buffer content; recovery journals intact | Restart; recovery pane offers the journal | No |

No error is presented as a timed toast. Every error that persists as a condition is represented as a persistent status flag, and every one has a `:messages` entry.

### 3.7 Accessibility

**Textual equivalents (Lens 5C).** Every visual affordance has a complete, ordered textual mode, and the acceptance suite asserts that no pane can present information that its textual mode omits:

- **Split geometry** → `:layout` renders an indented tree: axis, weight, cell extent, pane kind, path, view mode, and the literal word `[focused]`. `:layout --json` is the versioned machine form.
- **Tab bar / pin bar** → `:tabs` and `:pins` list every entry with ordinal, full name, path, and dirty/missing words. The bars are never the only place a tab or pin appears.
- **Status line** → `<leader>s` / `:status` renders every segment as `label: value` lines in the same fixed order, with nothing dropped regardless of width.
- **Graph pane** → opens in **textual adjacency mode by default**: `node PATH (in: N, out: M)` followed by indented `edge KIND TARGET STATE` lines in the deterministic order C specifies (source path, offset, kind, target). The spatial/plot rendering is a secondary, optional mode; `:graph --text` is always available and is what screen-reader mode uses. There is no graph information that exists only in the plot.
- **Canvas pane** → ordered node list (id, type, position as text, size, title, content reference) and ordered edge list (from/to node, side, label). Spatial editing is K's; E always exposes the list form, so no Canvas information is spatial-only.
- **Bases / typed views** → ordered records with every column labeled, including formula values and formula errors as text.
- **Preview** → tables become ordered `row N: column = value` records in screen-reader mode; images and attachments render as `image: PATH — alt: TEXT` or `alt: unavailable` (a filename is never promoted to a description); code fences announce their language; callouts announce their type word; math announces the source TeX; diagram fences (mermaid and similar) always expose their source text and are never image-only.
- **Diagnostics, merge hunks, search results, recovery candidates** → E renders D's and B's already-ordered projections without adding spatial-only structure.

**Screen-reader mode** (`--screen-reader`, `:set screenreader=on`, or `MG_VAULT_SCREEN_READER=1`): suppresses the alternate screen and full-frame repaint in favor of appended linear output; disables box drawing and decorative separators; emits a one-line announcement for each state change in D's fixed order (path → mode → position/selection → buffer state → pending input → search/diagnostics → merge/recovery → index freshness); turns every overlay into a linear prompt with numbered choices; disables the progress counter. The transcript adapter D already specifies is reused so the same assertions run headlessly.

**Focus order and keyboard navigability.** Focus is a single, always-defined pane or overlay; there is no state in which no element is focused. Overlay focus is trapped and `Esc` always returns to the prior focus. Tab order within an overlay is filter input → list → key hints. Every interactive element is reachable by the documented grammar and appears in the palette with its binding.

**Color-independent state.** Every state-bearing theme role must declare a `text_token` (e.g. diagnostic error `E`, warning `W`; merge conflict `!`; resolved link `[[ ]]`, unresolved `[[?]]`, ambiguous `[[*]]`; dirty `+`; pinned `#`; focused `[focused]`; index states as full words). The theme loader **rejects** a theme that omits a token for any state-bearing role. The 16-color and monochrome capability tests assert that the frame's text plane alone — with the attribute plane discarded — still distinguishes every state.

**Text scaling.** Dynamic type is N/A in a terminal; the user scales by changing terminal font size, which E observes only as a different cell count. Correct behavior at any cell count is therefore the scaling requirement, and it is covered by the responsive table in §3.4 and tested at 300×100, 120×30, 80×24, 60×20, 40×10, and 24×8.

**Unicode and safety in display.** Wide glyphs are never bisected by a pane edge: the boundary cell is left blank and a `>` continuation marker is drawn. Wrapping and horizontal scrolling split only at grapheme boundaries. Bidi control characters and other invisible controls in content are rendered as visible escapes in source mode (preventing bidi/homoglyph spoofing of paths and links) while the underlying bytes are untouched.

---

## 4. Implementation Specification

### 4.1 Architecture placement

Two new workspace members, plus a subcommand on the existing binary:

- **`crates/mg-vault-app`** — terminal-independent orchestration: buffer/pane lifecycle over D's `Editor`, path resolution through `mg-vault-core`, the watcher boundary, the B index client, the merged command registry, and workspace persistence. It has no terminal dependency, so the whole workspace can be driven headlessly in tests.
- **`crates/mg-vault-tui`** — everything terminal: capability detection, backends, key decoding and routing, layout solving, pane rendering, overlays, status line, themes, sanitization, and the accessibility projections.
- `crates/mg-vault-cli` gains a `ui` subcommand that constructs the app and hands it to the TUI; a thin `mg-vault-ui` binary is also produced for launcher/Quickshell integration (Q).

```text
crates/mg-vault-tui/src/
├── lib.rs
├── caps.rs             # capability probe, evidence, CapabilityReport
├── backend.rs          # TerminalBackend trait; crossterm impl; headless test backend
├── input/{decode,router,keymap}.rs
├── layout/{tree,geometry,focus,tabs}.rs
├── panes/{editor,preview,outline,links,search,diagnostics,merge,recovery,graph,canvas,unresolved}.rs
├── overlay/{palette,picker,workspaces,pins,status_full,help,confirm,ambiguity}.rs
├── status.rs           # segment model + width policy
├── theme/{model,load,downsample,contrast}.rs
├── render/{frame,sanitize,width}.rs
├── a11y.rs             # linear projection + screen-reader mode
└── session.rs          # workspace snapshot model, save/restore

crates/mg-vault-app/src/
├── lib.rs
├── workspace.rs        # tabs, panes, pins, session snapshot
├── buffers.rs          # BufferId ↔ path ↔ D Editor instances
├── commands.rs         # merged command registry and availability
├── index_client.rs     # B IPC client with deadlines and freshness passthrough
└── watch.rs            # watcher subscriptions; events are hints only
```

Authority is unchanged and E adds none. `mg-vault-core` remains the only filesystem authority; D remains the only mutator of buffer text and the only component that saves; B remains disposable derived data. E holds no `TextStore`, performs no direct note write, and cannot advance a `BaseSnapshot`. The single exception is the workspace/session writer, which touches only `.mg-vault/workspaces/` and XDG state through the same privileged-writer pattern A uses for trash, and never a note path.

### 4.2 Data model

```rust
/// One tab page: an independent split tree with its own focus.
pub struct Tab {
    pub id: TabId,
    pub name: Option<String>,      // None means derive from the focused pane
    pub root: LayoutNode,
    pub focused: PaneId,
    pub previous: Option<PaneId>,
}

/// Split tree. Weights are relative; cell extents are solved, never stored.
pub enum LayoutNode {
    Leaf(PaneId),
    Split { axis: Axis, children: Vec<(LayoutNode, u32)> },
}

pub enum Axis { Horizontal, Vertical }

/// A focusable region. Every variant has a textual projection.
pub enum PaneKind {
    Editor { buffer: BufferId, view: ViewMode },
    Preview { buffer: BufferId, linked_to: Option<PaneId> },
    Outline { buffer: BufferId },
    Links { buffer: BufferId, direction: LinkDirection },
    SearchResults { query: QueryId },
    Diagnostics { buffer: BufferId },
    Merge { buffer: BufferId },
    Recovery,
    Graph { root: Option<VaultRelativePath>, mode: GraphMode },
    Canvas { buffer: BufferId, mode: CanvasMode },
    Unresolved { recorded: VaultRelativePath, reason: UnresolvedReason },
}

/// Preview is never editable; `Linked` is a source/preview pair.
pub enum ViewMode { Source, Preview, Linked }

/// Textual is the default for both; spatial is opt-in and additive.
pub enum GraphMode { Textual, Spatial }
pub enum CanvasMode { Textual, Spatial }

/// Portable, content-free workspace snapshot. Serialized to
/// `.mg-vault/workspaces/<name>.json`.
pub struct WorkspaceSnapshot {
    pub schema_version: u32,       // refuse unknown major versions; never rewrite
    pub name: String,
    pub tabs: Vec<TabSnapshot>,
    pub active_tab: usize,
    pub pins: Vec<VaultRelativePath>,
}

pub struct PaneSnapshot {
    pub kind: PaneKindTag,
    pub path: Option<VaultRelativePath>,
    pub view: ViewMode,
    /// Grapheme-safe anchor; `context_hash` gates exact restoration.
    pub cursor: Option<CursorAnchor>,
    pub scroll: Option<BlockAnchor>,
    pub folds: Vec<FoldAnchor>,
}

pub struct CursorAnchor {
    pub line: usize,
    pub grapheme_column: usize,
    pub context_hash: ContentHash,
}

/// Detected once at startup; every field records how it was determined.
pub struct CapabilityReport {
    pub color: ColorDepth,             // Truecolor | Ansi256 | Ansi16 | None
    pub unicode: UnicodeSupport,       // Utf8 | AsciiOnly
    pub ambiwidth: AmbiWidth,          // Single | Double
    pub keyboard: KeyboardProtocol,    // Enhanced | Legacy
    pub bracketed_paste: bool,
    pub synchronized_output: bool,
    pub focus_events: bool,
    pub osc52_advertised: bool,
    pub mouse_enabled: bool,           // user setting, default false
    pub evidence: Vec<CapabilityEvidence>,
}

/// A theme role. `text_token` is mandatory for every state-bearing role;
/// the loader rejects a theme that omits one.
pub struct ThemeRole {
    pub id: RoleId,
    pub truecolor: Option<Rgb>,
    pub ansi256: Option<u8>,
    pub ansi16: Option<AnsiColor>,     // declared, not quantized
    pub attributes: Attributes,        // fallback for ColorDepth::None
    pub text_token: Option<&'static str>,
}
```

No database and no migration. `WorkspaceSnapshot` is versioned JSON; an unknown major `schema_version` is refused with a message and the file is left byte-identical rather than rewritten. Machine-local session state is a separate versioned JSON file under `$XDG_STATE_HOME/mg-vault/session/<vault-key>/` with mode `0600`. Nothing E writes ever enters a note, and no UUID is injected anywhere — panes reference notes by public vault-relative path only.

### 4.3 API contracts

E consumes contracts; it publishes only the app-level and automation ones below.

```rust
/// The only source of terminal input and output. The headless backend used in
/// tests implements the same trait, so every assertion runs without a TTY.
pub trait TerminalBackend {
    fn capabilities(&self) -> &CapabilityReport;
    fn size(&self) -> (u16, u16);
    fn poll(&mut self, timeout: Duration) -> Result<Option<TerminalEvent>, BackendError>;
    fn present(&mut self, frame: &Frame) -> Result<(), BackendError>;
    fn restore(&mut self) -> Result<(), BackendError>;   // idempotent; runs in the panic hook
}

impl Workspace {
    pub fn open(app: &mut App, name: Option<&str>) -> Result<Self, WorkspaceError>;
    pub fn save(&self, name: &str) -> Result<(), WorkspaceError>;   // atomic, via A
    pub fn snapshot(&self) -> WorkspaceSnapshot;                    // content-free
    pub fn restore(&mut self, snap: WorkspaceSnapshot) -> RestoreReport;
    pub fn execute(&mut self, command: CommandId, ctx: PaneContext) -> Result<Vec<AppEvent>, CommandError>;
    pub fn projection(&self) -> LinearProjection;                   // §3.7 textual mode
}

/// Every command, from every namespace, with a truthful availability reason.
pub struct CommandEntry {
    pub id: CommandId,
    pub title: &'static str,
    pub namespace: Namespace,           // Buffer(D) | Window | Leader | Ex | Adapter
    pub binding: Option<KeySequence>,
    pub availability: Availability,     // Available | Unavailable { reason: Reason }
}
```

Contract rules:

- `Workspace::restore` returns a `RestoreReport` listing every pane that became `Unresolved` and why. It never creates, moves, or writes a note, and it never applies a recovery journal.
- Every mutating command dispatches to D or to C's plan/commit seam. `CommandError::NotAuthorized` is returned if any TUI code path attempts a direct write; this is enforced by the crate boundary (E does not depend on a `TextStore`).
- Index results carry B's `freshness`, `index_generation`, and `observed_generation` unchanged. E has no code that maps an unknown or missing freshness value to `current`; the type has no default and the renderer is exhaustive over the enum.
- `present` receives an already-sanitized `Frame`. Sanitization happens when content enters the frame, not at the byte stream, so no path can bypass it.
- `restore` is idempotent and is called from the panic hook, from every exit path, and on `SIGTERM`/`SIGHUP`.

**Automation surface.** `mg-vault ui --dump-session [--workspace NAME]` prints the versioned `WorkspaceSnapshot` JSON and exits without entering the terminal. `mg-vault ui --check-capabilities --json` prints `CapabilityReport`. `:layout --json`, `:tabs --json`, `:pins --json`, `:status --json`, and `:capabilities --json` produce the same shapes in-session. `--no-input` refuses to start an interactive TUI and directs the caller to these commands; `--no-color` and `NO_COLOR` force `ColorDepth::None`. No prompt, no pager, and no ANSI ever appears in a JSON stream.

### 4.4 State management

- **`App`** (in `mg-vault-app`) owns: the vault handle, the `BufferId → Editor` map, the command registry, the index client, watcher subscriptions, and the message log. One `Editor` per *buffer*, not per pane, so two panes on one note share cursor-independent views of one authoritative buffer state.
- **`Workspace`** owns: tabs, split trees, focus, pins, and the workspace name. It is the unit that is saved and restored.
- **`Tui`** (in `mg-vault-tui`) owns: the backend, capability report, theme, keymap tables, frame buffers, overlay stack, and the width policy. It holds no note state.

Boundaries:

- **Authoritative:** ordinary files, read only through D via A. E never caches note bytes for correctness — only for rendering the current revision, and every render is tagged with that revision.
- **Unsaved user work:** D's buffer plus D's recovery journal. E's session file deliberately holds **no content**, so a corrupted or deleted session file can never lose text.
- **Derived, non-authoritative:** preview render, outline, diagnostics, links/backlinks, search results, graph, Canvas view, palette index. All are labeled with the revision or index generation they came from.
- **Local, per-machine:** capability report, width probe results, terminal size, message history, recently-used ordering.
- **Portable, per-vault:** workspaces, keymap overrides, themes — all under `.mg-vault/`, never `.obsidian`, which is read-never and written-never.

Offline/draft persistence is D's autosave and journal; E's contribution is to (a) supply the idle timer and pause it when a modal overlay holds a pending decision, (b) never treat pane close, tab close, focus change, or quit as an implicit save, and (c) never delete a recovery journal outside an explicit, confirmed `discard journal` action. Watcher events reach E only as hints that cause D to re-read through `TextStore`; E cannot reload or save on a watcher event alone.

### 4.5 Dependencies

**Stack: `ratatui` + `crossterm`,** with `termini` for terminfo, `unicode-width` for display width, `unicode-segmentation` (already D's) for boundary-safe wrapping, `toml` for themes/keymaps, `serde`/`serde_json` (already in the workspace) for session and JSON contracts, and `nucleo-matcher` for deterministic fuzzy ranking in the palette and pickers.

Justification against the repo's posture:

- **`unsafe_code = "forbid"`** is a workspace lint on first-party crates; it does not and cannot govern dependencies, and the workspace already accepts dependency-internal `unsafe` through `rustix` and `rusqlite` (bundled C SQLite). Within that reality, ratatui is the strongest choice: its own source contains no `unsafe`, and it is a pure immediate-mode widget/diff layer with a swappable backend — which is exactly what lets `TerminalBackend` have a headless implementation for the acceptance suite. crossterm confines the necessary `libc`/Windows-console `unsafe` to one well-reviewed layer and already exposes what this spec needs: alternate screen, bracketed paste, mouse capture, focus events, resize events, and kitty keyboard-protocol push/pop.
- **`clippy::all` + `pedantic` denied** argues against callback-heavy or `Rc<RefCell<_>>`-shaped frameworks. ratatui's render-a-frame-from-state model produces straight-line code that passes pedantic without `#[allow]` sprawl. The one place pedantic will bite is cell arithmetic (`cast_possible_truncation`); the geometry solver therefore works in `u16` with saturating/checked arithmetic throughout and confines any `#[expect(...)]` to a single documented conversion helper in `layout/geometry.rs`.
- **Edition 2024 / MSRV 1.85** — all listed crates support both; versions are pinned and their licenses (MIT for ratatui/crossterm/termini/nucleo, Unicode-3.0/MIT for the unicode crates) are recorded in `docs/DEPENDENCIES.md` before packaging (R).

Rejected alternatives: **cursive** (callback/ownership model fights a modal editor and pulls a backend zoo); **termwiz** (excellent terminfo-driven capability model — whose approach this spec borrows — but a much heavier dependency for a thin widget layer); **tui-rs** (superseded by ratatui); **hand-rolled ANSI** (would reimplement terminfo, width tables, and damage-based diffing with no safety benefit).

New assets: built-in themes `mg-dark`, `mg-light`, `mg-mono`, `mg-high-contrast` as first-party TOML in-repo. **No Nerd Font or icon font is required**; the default glyph set is ASCII plus basic box drawing, and an opt-in `icons = nerdfont` setting never removes a text label. No infrastructure, network service, CDN, or database is added.

### 4.6 Platform-specific considerations

- **Primary target:** Linux/Arch with Hyprland-hosted terminals (kitty, foot, alacritty, wezterm). macOS terminals (Terminal.app at 256 colors, iTerm2, WezTerm) are supported and exercised in the capability matrix. Windows is out of scope for Milestone 5; crossterm keeps the door open but nothing is claimed.
- **Terminal multiplexers** are a first-class degradation case: tmux and screen frequently do not forward the kitty protocol, may report a narrower color depth, and can rewrite `TERM`. Detection therefore relies on the *inner* environment plus reply probes, never on assumption, and `:capabilities` shows the multiplexer's effect. Under a multiplexer without passthrough, enhanced-keyboard bindings are unavailable and their alternatives apply — no workflow is lost.
- **SSH and slow links:** damage-based rendering keeps per-frame output proportional to changed cells; synchronized output is used when advertised. `:set lowbandwidth=on` disables the pin bar, sign column, and separators to cut bytes further.
- **Signals:** `SIGWINCH` recomputes layout; `SIGTERM`/`SIGHUP` restore the terminal and exit without saving buffers implicitly (journals persist); `SIGCONT` after `Ctrl-z` re-enters the alternate screen, re-pushes keyboard flags, and forces a full repaint.
- **Feature flags:** the graph and Canvas panes are behind `feature = "graph"` / `"canvas"` pending G and K. When a feature is off the palette lists the command as `unavailable: feature not built`, which is the same mechanism as any other degradation — there is no hidden capability. Source editing, splits, tabs, workspaces, status line, and accessibility modes are mandatory and not flag-gated.
- **Version compatibility:** `WorkspaceSnapshot.schema_version` and the JSON contract version are independent of the CLI's `version: 1` envelope and are declared in `docs/TUI.md`. A newer session file opened by an older binary is refused, never downgraded in place.

### 4.7 Performance budget

Measured in optimized builds on the project reference machine, reported with terminal size and corpus metadata:

- **Input to presented frame:** p95 ≤ 16 ms, p99 ≤ 33 ms at 80×24 with 2 panes; p95 ≤ 24 ms at 300×100 with 4 panes. This excludes D's journal sync barrier, which D budgets separately (p95 ≤ 50 ms) and which E surfaces as the `pending durability` status word rather than by blocking the frame.
- **Render only:** p95 ≤ 8 ms at 80×24, ≤ 16 ms at 300×100. Rendering is damage-based; a full repaint occurs only on resize, theme change, capability change, or `SIGCONT`.
- **Startup:** capability detection ≤ 200 ms worst case (bounded by the 150 ms keyboard-protocol probe); cold start to first interactive frame ≤ 250 ms for a workspace of ≤ 8 panes, because only the focused pane's buffer is loaded before the first frame. Startup time is independent of vault size — no vault-wide scan runs before the first frame.
- **Scale (Lens 3E):** the picker and palette never materialize the vault. Queries stream from B with a bounded candidate window of ≤ 10,000 held candidates and a per-keystroke deadline of 150 ms, after which results render labeled `partial`. At 100,000 notes / 1,000,000 blocks the picker's first frame budget is unchanged, and index latency or unavailability degrades the picker only — it never blocks a keystroke in an editor pane, a save, a merge, or recovery.
- **Preview:** viewport-only rendering; ≤ 32 ms p95 for a 1 MiB note. Off-screen blocks are never rendered. Preview re-render is debounced to ≤ 10 Hz and is cancellable.
- **Memory:** TUI-owned resident memory (frames, theme, capability report, layout, keymap tables, palette index, message log) ≤ 32 MiB, independent of vault size and excluding D's buffers, which D budgets. The message log is capped at 1,000 entries and the palette index at the candidate window above.
- **Cancellation:** `Ctrl-c` and `Esc` cancel any in-flight picker query, search, graph expansion, or preview render. Cancellation is cooperative and always leaves the prior projection labeled stale rather than blank.
- **Network:** none. E performs no network I/O in any code path (Lens 5A).

A budget miss degrades derived views first — preview, graph, picker results, diagnostics — and never text correctness, key routing, status truthfulness, save, merge, or recovery.

---

## 5. Test Specification

### 5.1 Unit tests

- `geometry_solver_is_exact_and_deterministic` — property test over random trees and extents: child extents sum exactly to the parent, no rectangle overlaps, no negative extents, and two identical inputs produce identical output.
- `geometry_minimum_size_collapses_to_stack` — shrink below 20×3 per pane; assert stacking, that no pane is dropped, and that regrowth restores the exact prior geometry.
- `directional_focus_follows_documented_rule` — table over overlapping and non-overlapping arrangements; assert the nearest-edge/overlap/tree-order tie-break and the `no pane to the left` message.
- `router_never_shadows_buffer_keys` — for every mode and every key in D's documented grammar, assert routing: `Ctrl-w` reaches D in Insert/Replace, `Space` reaches D whenever D reports pending state, and no other key is claimed by E.
- `keymap_conflicts_are_reported_not_resolved` — conflicting entries across machine and vault keymaps; assert both are named with file/line, defaults stay active, and no winner is chosen.
- `every_command_is_keyboard_reachable` — walk the merged command registry; assert each entry has either a default binding or a palette/ex path, and that the mouse-reachable set is a strict subset of the keyboard-reachable set.
- `capability_detection_precedence` — table over `NO_COLOR`, `--color`, `COLORTERM`, terminfo `RGB`/`Tc`/`colors`, `TERM` variants, and `TERM=dumb`; assert the exact depth and the recorded evidence string.
- `enhanced_keyboard_absence_loses_no_command` — build the registry with `KeyboardProtocol::Legacy`; assert the reachable command set equals the `Enhanced` set and every affected entry carries a reason and an alternative.
- `esc_is_immediate_in_insert_mode` — legacy encoding, `Esc` followed by a byte after 1 ms; assert mode exits without waiting `escdelay`, and that Alt-chords are recognized only in Normal/Visual.
- `paste_burst_guard` — 32 events in 20 ms in Normal and Insert modes; assert the guard prompt in Normal, literal insertion in Insert, and no command execution in either.
- `status_never_drops_the_non_droppable_set` — every width from 20 to 200; assert mode, buffer state, index freshness word, and capability badge are present and unabbreviated.
- `status_index_freshness_is_exhaustive_and_literal` — every `Freshness` variant plus `unavailable`/`rebuilding`; assert a distinct literal word for each, assert the renderer is exhaustive over the enum, and assert no code path produces `current` for an unknown value.
- `theme_without_state_tokens_is_rejected` — a theme missing `text_token` on a state-bearing role fails to load with a named role.
- `theme_downsamples_by_declaration_not_quantization` — assert 16-color output uses declared fallbacks and monochrome uses declared attributes.
- `frame_text_plane_distinguishes_every_state` — render every state at `ColorDepth::None`, discard the attribute plane, assert all states remain distinguishable in text alone.
- `sanitizer_neutralizes_control_sequences` — content containing CSI, OSC 8, OSC 52, DCS, BEL, bidi controls, and NUL; assert visible escapes in source, stripping in preview, and that no raw sequence reaches the backend.
- `wide_glyph_never_bisected` — CJK, emoji ZWJ sequences, flags, and combining marks at every pane boundary column; assert a blank boundary cell plus continuation marker and that display width never feeds back into an edit range.
- `cursor_cell_mapping_is_grapheme_derived` — map D's `GraphemePos` to cells across the same corpus; assert the cell column is derived from D's grapheme column via `unicode-width` and never computed by E from bytes.
- `session_snapshot_round_trips_and_holds_no_content` — property test over workspaces; assert byte-stable serialization, exact round-trip, and that no note text appears anywhere in the serialized form.
- `unknown_session_schema_is_refused_not_rewritten` — assert refusal, an explanatory message, and a byte-identical file afterwards.
- `restore_marks_missing_paths_unresolved` — missing, moved, directory, outside-vault, and protected-control-directory paths; assert an `Unresolved` pane for each, zero creates, and zero writes.
- `cursor_anchor_mismatch_is_approximate_not_wrong` — changed context hash; assert the cursor lands on a valid grapheme boundary and the pane reports `approximate`.

### 5.2 Integration tests

- **Capability matrix** — the full cross product of {truecolor, 256, 16, none} × {UTF-8, ASCII} × {enhanced, legacy} × {300×100, 120×30, 80×24, 40×10, 24×8} on the headless backend. In every cell: every command reachable, status non-droppable set present, every pane's textual projection identical to its non-degraded projection modulo glyphs, and source editing functional.
- **Crash and restore** — edit in a two-tab split workspace, kill the process at randomized points, restart. Assert: the recovery pane appears before restore; nothing is applied; a journal whose base predates a newer source shows `source changed since journal base` and offers **no** recover action, only merge; and the restored layout matches the pre-crash snapshot with unresolved panes for any path changed in the interim.
- **Recovery cannot overwrite newer source** — dedicated fixture: journal base = A, source externally advanced to B, recovery attempted through every route (startup pane, `:recover`, palette, session restore, `--no-input`). Assert B is byte-identical afterward in all routes and that only an explicitly accepted merge can change it.
- **Non-atomic save never claims success** — fault-inject D's save (fsync failure, rename failure, ENOSPC). Assert the status shows `SAVE FAILED`, the buffer stays dirty, the journal is intact, and no frame at any point rendered `saved`.
- **Stale index is never presented as current** — drive B through `empty`, `current`, `stale`, `degraded`, `rebuilding`, and unavailable. Assert the status word, every result header, and the palette availability reason match the actual state; assert `create note from unresolved link` is refused when not `current`; assert direct-scan fallback is labeled.
- **External change and merge** — external write while a pane is dirty; assert `EXTERNAL CHANGE`, that the merge pane opens on save attempt, that `MERGE REQUIRED (n)` blocks save, and that narrow terminals stack the three versions without hiding any hunk.
- **Persistent undo across sessions** — edit, save, quit, restart, restore workspace, `u`. Assert D's history is restored on an exact hash match and that a source changed out-of-band yields `undo history unavailable — source changed since last save` in the buffer-info overlay rather than a replay.
- **Protected directories and confinement** — attempt to open, split into, preview, pin, and restore panes for `.obsidian/**` and `.mg-vault/**`, plus symlinks and `..` paths pointing outside the vault. Assert `ProtectedControlPath`/`unsafe_path` rejection at every entry point, byte-identical control directories, and that the only E write inside `.mg-vault` is the workspace file through its dedicated writer.
- **Watcher events are hints** — deliver forged, duplicated, and out-of-order watcher events. Assert no reload or save happens without a fresh `TextStore` read and that no event advances a base.
- **Scale isolation** — a fake B advertising 100,000 notes / 1,000,000 blocks, delayed, saturated, stale, cancelled, and absent. Assert picker labels, the bounded candidate window, and that editor-pane keystroke, save, merge, and recovery latencies stay inside §4.7 budgets.
- **Terminal lifecycle** — panic, `SIGTERM`, `SIGHUP`, `Ctrl-z`/`SIGCONT`, terminal disconnect mid-frame. Assert the terminal is restored in every case, keyboard flags are popped, no buffer content appears in the panic report, and journals survive.
- **Multiplexer degradation** — simulate tmux without kitty passthrough and with a rewritten `TERM`. Assert detection reports it, affected bindings are marked unavailable with alternatives, and no workflow is lost.

### 5.3 UI / E2E tests

Scripted key sequences against the headless backend, asserting frame text planes, the linear projection, and app state after each step:

1. **Split and navigate:** open note, `Ctrl-w v`, `<leader>f` open a second note, `Ctrl-w h/l` both directions, `Ctrl-w =`, `Ctrl-w z` toggle, `Ctrl-w c`. Assert geometry, focus, and that closing a dirty pane is refused.
2. **Tabs and pins:** `Ctrl-w T`, `gt`/`2gt`/`gT`, `<leader>P` pin two notes, `<leader>1`/`<leader>2`, `<leader>p` reorder and unpin. Assert `:tabs` and `:pins` match the bars exactly.
3. **Source/preview:** `<leader>v` round trip, `<leader>V` linked pair, edit in source and assert the preview revision label advances and is never ahead; type in preview and assert the read-only refusal with zero mutation.
4. **Palette:** open, filter, execute a command in a non-focused-pane context, and assert an unavailable command shows its reason and cannot execute.
5. **Navigation and ambiguity:** follow a unique link, follow an ambiguous link (assert the chooser has no pre-selection and `Esc` changes nothing), follow an unresolved link (assert the create confirmation names the exact path and is refused when the index is not current).
6. **Workspace lifecycle:** build a two-tab split with pins, `:workspace save daily`, quit, restart, restore. Assert an exact match, then move a file externally and assert the unresolved placeholder with its three actions.
7. **Recovery flow:** crash mid-edit, restart, assert the recovery pane precedes restore, exercise inspect/export/discard, and assert `Continue without recovering` leaves every journal intact.
8. **Merge flow:** external edit during a dirty buffer, `]c`/`[c` hunk navigation, per-hunk resolution, save; repeat at 40 columns and assert the stacked layout exposes every hunk.
9. **Degraded run:** the whole of scripts 1–8 replayed with `NO_COLOR=1`, ASCII locale, legacy keyboard, and 40×10. Assert identical app-state outcomes.
10. **Screen-reader run:** scripts 1–8 replayed in screen-reader mode through D's transcript adapter. Assert announcement order matches §3.7 and that every state change produces exactly one announcement.

### 5.4 Visual / manual verification

- **Themes:** `mg-dark`, `mg-light`, `mg-mono`, `mg-high-contrast` at truecolor, 256, 16, and none; plus `:theme check` contrast report (warn below 4.5:1) and OSC 11 light/dark auto-detection with and without a reply.
- **Text size extremes:** terminal font sizes producing 300×100, 120×30, 80×24, 60×20, 40×10, and 24×8; plus the below-floor 16×5 refusal message.
- **Screen/terminal extremes:** the same sizes with the tab bar, pin bar, two-row status, stacked layout, and full-width overlays; a 200-character path; 50 tabs; 9 splits.
- **Empty vs populated:** empty vault, empty note, no pins, no tabs, no diagnostics, no search results, no backlinks — each with its explicit copy; then a 100,000-note vault with a saturated picker.
- **Unicode:** combining marks, ZWJ emoji, flags, skin tones, CJK, Thai, RTL Arabic/Hebrew, and embedded bidi controls in kitty, foot, alacritty, wezterm, xterm, tmux, and Terminal.app, checking wide-glyph boundaries and the `:probe width` calibration.
- **Terminals and multiplexers:** each of the above with and without kitty protocol, bracketed paste, and synchronized output; plus a slow SSH link with `lowbandwidth=on`.
- **Adversarial content:** notes containing ANSI/OSC/DCS sequences, an OSC 8 hyperlink, a 10 MiB single line, and 10,000 diagnostics — assert nothing escapes the sanitizer and the frame budget degrades derived views only.

Milestone 5 quality gates: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-targets --all-features`, property tests, the capability matrix, the accessibility projection suite, and frame-latency benchmark regression reports.

---

## 6. Compliance & Safety Gate

### 6.1 Sensitive data classification

- [ ] No sensitive data involvement
- [x] **Handles sensitive data** — note text is rendered on screen; workspace files record vault-relative paths (which can themselves be sensitive); picker queries, palette history, and message logs can contain note titles and search terms.
- [ ] Uses synthetic/test data only until compliance gate clears

Protections: E is local-only and performs no network I/O. Workspace snapshots are **content-free** by construction (paths and anchors only) and live under `.mg-vault/workspaces/` with the vault; machine-local session state lives under `$XDG_STATE_HOME/mg-vault/session/<vault-key>/` with `0700` directories and `0600` files. No note content, no picker query, and no path enters routine logs, and the panic report contains none. The message log is in-memory only and capped. Nothing is written to `.obsidian`. Terminal content is scrubbed of OSC 52 before rendering, so a note cannot use the display path to write the user's clipboard; D's opt-in OSC 52 write remains the only clipboard egress. Any future AI or plugin pane inherits D's deny-by-default capability contract and gets no `TextStore`, clipboard, or process handle from E.

### 6.2 Asset provenance

- [ ] No third-party assets
- [x] **Uses third-party assets** — `ratatui`, `crossterm`, `termini`, `nucleo-matcher` (MIT), `unicode-width` and `unicode-segmentation` (Unicode-3.0 / MIT-or-Apache-2.0 dual), and the system terminfo database (read only, not redistributed). Versions, licenses, checksums, and upstream URLs are recorded in `docs/DEPENDENCIES.md` before packaging (R). Built-in themes are first-party and carry the repository license. **No font is shipped or required**: the default glyph set is ASCII plus basic box drawing, and Nerd Font icons are strictly opt-in with a text label always present.

### 6.3 Language / claims audit

- [x] **No unsupported claims.** "Vim-style" refers only to D's enumerated grammar plus E's documented window/leader namespaces; nothing here may be described as Vim-compatible. "Preview" is a read-only rendering of what F implements — never WYSIWYG editing.
- [x] **No promised-but-unbuilt capability is presented as available.** The palette shows unbuilt or unavailable commands with an explicit reason (`feature not built`, `requires index`, `requires terminal capability`), never as available.
- [x] **No restricted-domain language.** This is a general-purpose knowledge tool; no medical, financial, or legal claims appear.
- [x] **Status words are literal and truthful.** `current`, `stale`, `degraded`, `empty`, `rebuilding`, `unavailable`, `partial`, `saved`, `SAVE FAILED`, `approximate` mean exactly what the underlying contract reports; E has no code path that upgrades an unknown state to a better one.

### 6.4 Regulatory alignment

Confirming each Lens 3 criterion, plus the auto-fail rules and the Lens 2/5 criteria this feature anchors:

- **3A Determinism** — every view derives from a specific buffer revision (D) or index generation (B), and each is labeled with it. Layout, focus movement, fuzzy ranking, and result ordering are deterministic and property-tested; no locale collation, hash iteration order, or arrival order affects output.
- **3B Ambiguity** — an ambiguous link opens a chooser with no pre-selection and no default; cancelling changes nothing. E never picks a candidate, and creating a note from an unresolved link requires an exact confirmed path and a `current` index.
- **3C Query depth** — the picker and search pane expose B's full grammar (text, titles, regex, tags, properties, paths, relationships, tasks, dates) as first-class filters with a visible parsed query, and pass through B's typed errors rather than broadening a failed query.
- **3D Derived authority** — preview, outline, graph, Canvas, Bases, diagnostics, links, and search panes are strictly read-only views. E depends on no `TextStore`, holds no write path to a note, and every mutation is routed to D or C.
- **3E Scale** — the picker's bounded candidate window, per-keystroke deadlines, lazy pane loading, and vault-size-independent startup keep a 100,000-note vault from blocking any keystroke, save, merge, or recovery.

Auto-fail rules addressed by name:

- **Recovery overwriting newer source** — the recovery pane runs before workspace restore, applies nothing automatically, and *withholds the recover action entirely* when the journal base predates current source, offering only merge. Tested directly in §5.2.
- **Non-atomic save claiming success** — E never saves; it renders D's `SaveOutcome`. `saved` is rendered only on D's post-`replace_atomic` success event, and fault injection asserts no frame ever showed `saved` on a failed save.
- **Stale index presented as current** — the status line carries a literal freshness word from B, the renderer is exhaustive over the freshness enum with no default arm, every result set repeats its freshness, and direct-scan fallback is labeled as such.
- **Index state overriding source / source-content loss** — E holds no note bytes as authority, its session files contain no content, and every pane re-reads through D on restore.
- **Unsafe traversal or symlink escape** — every path E touches goes through A's `Vault` validation; `.obsidian` and `.mg-vault` are rejected by note-path APIs, with the single dedicated workspace writer as the audited exception.
- **Active raw HTML/script by default** — the preview renders passive text only; raw HTML, scripts, and embedded control sequences are inert, and the sanitizer runs at frame-construction time so no render path can bypass it.
- **Graph or Canvas information lacking a textual equivalent** — both panes default to textual mode, `--text` is always available, spatial rendering is strictly additive, and the acceptance suite asserts the spatial mode surfaces no information absent from the textual mode.
- **Silent conflict winner / unconfirmed overwrite** — merge resolution is D's and is always explicit; E blocks save while hunks are unresolved and refuses implicit saves on pane close, tab close, focus change, or quit.

Lens 2 and Lens 5 anchors: **2A** — the routing precedence, the four documented namespaces, the generated binding table, the palette as universal fallback, and the mouse-subset assertion. **2B** — E surfaces D's atomic autosave, crash recovery, persistent undo, and external merge, and adds no path that bypasses them. **2C** — panes, splits, tabs, source/preview, navigation, pins, and session restore in one coherent model. **2D** — all cursor and structural work is D's grapheme buffer; E maps `GraphemePos` to cells for display only. **2E** — every degradation is named in `:capabilities` with an alternative, and source editing works in every matrix cell. **5C/5D** — §3.7 in full.

---

## 7. Gap Analysis vs. Current State

### 7.1 What exists today

- **Absent** — the entire TUI. There is no `mg-vault-tui` or `mg-vault-app` crate, no `ui` subcommand, no terminal dependency of any kind in `Cargo.toml` (workspace dependencies are `clap`, `serde`, `serde_json`, `rustix`, `rusqlite`, `sha2`, `thiserror`, `yaml-edit`), and no pane, split, tab, workspace, theme, keymap, palette, status-line, capability-detection, or accessibility code anywhere in `crates/`.
- **Absent** — the editor engine E renders. Spec D is written but `crates/mg-vault-editor` does not exist; there is no buffer, grapheme cursor, modal grammar, undo tree, recovery journal, autosave, or merge implementation to drive a pane.
- **Absent** — markdown rendering (F), link/graph semantics (G), and Canvas (K), so the preview, graph, and Canvas panes have no backing contract implemented yet.
- **Planned** — B's index service, IPC, and query grammar. `README.md` states there is no long-running watcher, IPC protocol, or daemon lifecycle, so the picker and search panes have no service to call yet.
- **Implemented** — the foundation E depends on. `crates/mg-vault-core/src/{xdg,registry,vault,atomic,error}.rs` provide XDG paths, the vault registry and selection, confined relative-path validation with traversal/symlink/protected-directory rejection, SHA-256 fingerprints, and atomic same-directory replacement — the exact primitives the workspace writer and every pane path check need.
- **Implemented** — `crates/mg-vault-index/src/lib.rs` provides the disposable SQLite projection with the literal `Freshness::{Empty, Current, Stale, Degraded}` enum, `IndexMetadata` with `generation`/`diagnostics`, and observation-guarded `verify_freshness`. This is the concrete type the status line's segment 9 renders.
- **Implemented** — `crates/mg-vault-cli/src/main.rs` provides the `version: 1` JSON envelope and the global `--json`, `--no-input`, `--no-color`, `--vault` flags with `NO_COLOR` handling. E's `ui` subcommand and JSON dumps extend this surface rather than inventing a second convention. Its doc comment records that all current commands are non-interactive, which is accurate today.
- No current file, test, or crate should be read as evidence that any E behavior exists.

### 7.2 Delta to spec

**New crates and modules:** `crates/mg-vault-app` (workspace, buffers, commands, index_client, watch) and `crates/mg-vault-tui` (the full module tree in §4.1), plus a `mg-vault-ui` binary.

**Modified files:** `Cargo.toml` — add both members and the pinned `ratatui`, `crossterm`, `termini`, `unicode-width`, `unicode-segmentation`, `toml`, `nucleo-matcher` workspace dependencies. `crates/mg-vault-cli/src/main.rs` — add the `ui` subcommand and the `--dump-session` / `--check-capabilities` JSON paths. `crates/mg-vault-core/src/vault.rs` — add the privileged `.mg-vault/workspaces/` writer alongside the existing trash writer, reusing `atomic.rs`; no change to note-path policy. `README.md`, `docs/ARCHITECTURE.md`, `docs/PRODUCT.md` — record the TUI slice honestly once it lands.

**New docs and assets:** `docs/TUI.md` with the generated binding table, namespace routing rules, deliberate Vim differences, layout and focus rules, status width policy, capability matrix, and status vocabulary; `docs/DEPENDENCIES.md`; four built-in theme TOMLs; default `keymap.toml`.

**New fixtures and harnesses:** headless `TerminalBackend` with frame text/attribute plane capture; capability-matrix harness; layout property-test generators; Unicode width corpora; adversarial-content corpus (ANSI/OSC/DCS/bidi); fake B index client covering every freshness state and the 100,000-note scale case; fake D editor for pane tests plus real-D integration; session-file goldens; screen-reader transcript goldens; frame-latency benchmarks.

**Migrations / schema:** none in the database sense. New versioned JSON schemas for `WorkspaceSnapshot` and machine-local session state, each with a documented unknown-version refusal policy. No note metadata is added and no UUID is injected.

### 7.3 Estimated scope

**XL.** E is a full terminal application: a geometry solver, a key-routing layer that must interlock precisely with another spec's grammar without shadowing it, eleven pane types, ten overlays, a theming and downsampling system, a capability-detection matrix with real degradation paths for each capability, a portable session format, and a parallel textual/accessibility rendering of everything visual. Roughly comparable in effort to D, and it cannot be accepted until its accessibility matrix and capability matrix pass, both of which are cross-cutting.

Recommended dependency-ordered checkpoints:

1. `TerminalBackend` (crossterm + headless), capability detection, `:capabilities`, panic-safe terminal restore.
2. Layout tree, geometry solver, focus model, splits, tabs, and their textual projections (`:layout`, `:tabs`).
3. Key decoding, namespace routing against a fake D, keymap loading and conflict reporting, `:help keys`.
4. Editor pane over real D, status line with the full width policy and literal freshness segment, message line and `:messages`.
5. Themes, downsampling, contrast check, mandatory state text tokens, `NO_COLOR`/monochrome acceptance.
6. Command registry, palette, pickers, pins, and the availability-reason mechanism.
7. Workspace save/restore, unresolved placeholders, autoworkspace, `--dump-session`.
8. Recovery pane, merge pane, save-failure surfacing — the auto-fail suite.
9. Preview and linked pair over F; outline, links, diagnostics, search panes over B/G; graph and Canvas textual modes.
10. Screen-reader mode, the full capability × size matrix, benchmarks, and `docs/TUI.md` generation.

### 7.4 Blocking dependencies

- **D (editor engine) — hard blocker.** E cannot render an editor pane without D's buffer view model, `PendingInput` projection (which the leader-key routing rule depends on), command registry, `MergeSession`, recovery offer, save outcomes, and grapheme coordinates. Checkpoints 1–3 can proceed against a fake D; checkpoint 4 onward cannot.
- **A (foundation) — partially implemented, sufficient to start.** Path authority, fingerprints, XDG paths, and atomic replacement exist. E additionally needs the privileged `.mg-vault/workspaces/` writer. A's known limitation (no descriptor-relative hardening against directory-entry races) is inherited and must not be overstated by E.
- **B (index service) — soft blocker.** Quick open, search, backlinks, and link resolution degrade to labeled direct scans without it. E must ship its unavailable/stale paths regardless, since they are also the degraded paths.
- **F (Markdown rendering) — soft blocker for preview.** Until F exists, `<leader>v` falls back to outline, then source, with the labeled message. Splits, tabs, workspaces, and source editing are unaffected.
- **G (links and graph) and K (Canvas) — soft blockers** for the link/backlink and graph/Canvas panes, which are feature-gated and reported as `feature not built` until then.
- **C (CLI seam)** — E reuses C's exact-path/fingerprint handoff and its create/move plan-commit contracts for note creation from unresolved links. C's D/E conformance harness (its §3.2 item 4: restoring a two-tab split source-preview session by path, with missing paths becoming explicit unresolved entries) is owned by E and blocks integrated TUI claims.
- **R (packaging)** — dependency license review and `docs/DEPENDENCIES.md` gate release, not development.

### 7.5 Explicit non-goals

- A GUI, a web view, an embedded browser, or image-protocol rendering (sixel/kitty graphics). Images are always reported as `image: PATH — alt: …` text.
- Reimplementing D's grammar, motions, text objects, registers, macros, undo, autosave, journal, or merge. E routes keys and renders projections; it owns no text mutation.
- Implementing Markdown rendering (F), link resolution or graph computation (G), Canvas semantics (K), Bases evaluation (I), Git or sync workflows (M), plugin hosting (N), or AI panes (O). E provides pane and overlay surfaces for these with textual-first modes, nothing more.
- Vim/Neovim window-command compatibility beyond the documented `Ctrl-w` subset; no Vimscript, no autocommands, no modelines, no plugin API.
- Windows terminal support, remote/collaborative shared workspaces, session sharing between machines beyond the portable workspace file, and terminal image protocols.
- Mouse-first workflows, timed toasts, animation, and sound.

---

## 8. Open Questions

- **Q1:** Is `<Space>` the right leader, given it rebinds D's Normal-mode `l`? The alternative is `,` (no Vim conflict, worse ergonomics) or a configurable default with `<Space>` shipped. — blocks: §3.4 routing rule 4, §3.1 overlay bindings.
- **Q2:** Should workspaces live in the vault at `.mg-vault/workspaces/` (portable across machines and syncable, but adds a vault write path) or XDG-state-only (no vault writes at all, no portability)? This spec chose in-vault for portability with machine-local state split out. — blocks: §3.2 workspace flow, §4.2, §6.1.
- **Q3:** Does the graph pane belong to E or to G? This spec assumes E owns the pane and overlay chrome while G owns adjacency data and resolution. — blocks: §3.1 inventory, §4.6 feature flags, §7.4.
- **Q4:** Default `ambiwidth` — `single` (correct for most modern Linux terminals) or `double` (correct for CJK-configured terminals)? The `:probe width` calibration mitigates either choice but the default still matters for first-run correctness. — blocks: §3.4 Unicode detection.
- **Q5:** Is 20×6 the right absolute floor for refusing to start, and 40×10 the right threshold for constrained mode? Lower floors increase the matrix; higher floors exclude some embedded terminals. — blocks: §3.2 startup, §3.4 responsive table.
- **Q6:** Should mouse support default to `on` (discoverability, matches Obsidian expectations) rather than `off` (matches Neovim, and this spec's choice)? The zero-mouse-only-paths guarantee holds either way. — blocks: §3.4 mouse.
- **Q7:** Should E ship an interactive "first run" capability walkthrough, or is `:capabilities` on demand sufficient? — blocks: §3.2 startup.
- **Q8:** Which fuzzy matcher is acceptable after license and determinism review — `nucleo-matcher` (fast, MPL-2.0 adjacent licensing needs checking) or a first-party bounded matcher? — blocks: §4.5, §6.2.
