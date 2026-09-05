# Spec: Quickshell and Suite Integration

**Feature ID:** q-quickshell-suite-integration **Parent feature:** root **Spec author agent:** Spec agent (branch Q) **Date:** 2026-08-30 **Iteration:** 1

> **Status: absent today, and scoped honestly.** Nothing in branch Q exists in the working
> tree at commit `dfe33cf` (§7.1). This spec describes the **target** state. Where it says
> "the widget does X", read it as "when Q is built, the widget will do X". The one piece of Q
> that should land *early* — before any QML is written — is the versioned interface manifest
> and its contract tests, because that is what makes every other branch unable to break the
> desktop boundary silently.
>
> **The load-bearing rule of this branch, in one sentence:** the Quickshell widget never opens
> the SQLite index, never reads a vault file, and never reads `mg-calr`'s PostgreSQL database.
> It spawns short-lived `mg-vault` child processes listed in a frozen manifest, and nothing
> else. This mirrors `mg-calr`'s own client-interface contract in
> `mg-calr/docs/specs/i-deferred-branches.md` §4.3, deliberately, so the two
> products' desktop integrations are one architecture rather than two.
>
> **Naming.** Branch letters (Q) and criteria letters collide with lens numbering. Criteria are
> always written **Lens-*n* *X*** (e.g. Lens-3 3A); features are always written **branch *X***.

---

## 1. Purpose

### 1.1 One-sentence job

Put the vault's live state — how fresh the index is, how many notes and open tasks there are, whether today's periodic note exists — on the Hyprland bar and one keystroke away, and let any application hand `mg-vault` a link that navigates straight to a note, all of it through a versioned, read-only, fail-closed command contract that no client may reach around.

### 1.2 Why it matters

`mg-vault` is a terminal product on a workstation whose whole desktop is a Quickshell shell (`~/dotfiles/config/quickshell/mgeist/`). A CLI answers "what is in my vault?" only when the user types; a bar pill answers it continuously. That convenience is also exactly where this product's two hardest invariants go to die. First, **authority**: a widget that wants a note count has an obvious shortcut — open the SQLite file, or `find` the vault — and a shell that reads the index directly becomes a second reader with its own idea of what is current, which is authority inversion by the back door (Lens-1 1A). Second, **freshness**: a pill's job is to hold a number on screen between polls, which is the precise mechanism by which a stale index gets presented as current — an auto-fail. Writing the desktop surface down *before* it is built is what keeps `contracts/` a real boundary instead of a comment.

The suite dimension matters for the same structural reason. `mg-calr` already owns todos in PostgreSQL; `mg-vault` owns Markdown checkboxes. Branch J settled that boundary: one explicit, previewed, user-named promotion through `mg-calr todo add --json`, never a sync daemon. Branch Q is where that boundary is most tempting to breach, because a single card showing "3 notes, 2 events" looks like it wants one shared database behind it. It must not have one, and the two products already share the only interchange they need: the implemented `mg.interop/1` snapshot schema that both `vault/crates/mg-vault-core/src/interop.rs` and `calendar/src/interop.rs` produce and validate today.

### 1.3 Success signal

Two outcomes, one available before any widget exists and one after:

- **Available first, and the reason to write this now:** `contracts/shell-interface-v1.json`, `contracts/shell-interface-v1.lock`, and `crates/mg-vault-cli/tests/shell_interface_contract.rs` are checked in and green, and the workspace test suite fails if any branch A–P change removes, renames, or changes the meaning of a `stable` entry, or if any file under `integrations/quickshell/` names SQLite, a vault path, or an argv the manifest does not list.
- **After the widget ships:** on the reference workstation, across an eight-hour session, the pill's rendered state matches `mg-vault shell snapshot --text` at every observation; editing a note behind a stopped index service flips the pill from `current` to `stale` with a visible age within one poll interval and the pill shows **no number** while stale; a byte-level diff of the vault tree and of the index database before and after every non-mutating client path is empty; and 10,000 fuzzed `mgvault://` URIs produce zero vault byte changes and zero accepted paths outside the canonical root.

---

## 2. User Stories

> As a writer at a Hyprland desktop, I want a bar pill showing whether my vault index is current
> and how many tasks are open, so that I know the state of my notes without switching workspace
> or opening a terminal.

> As a writer whose laptop slept for two days with the index service stopped, I want the pill to
> say `stale 2h` instead of holding yesterday's count, so that I never act on a number that is no
> longer true.

> As a writer reading a link in my browser or a chat client, I want to click `mgvault://open?…`
> and land in the right note, so that the vault is addressable from the rest of the desktop.

> As a security-minded operator, I want a deep link that names a path outside the vault, a
> `.obsidian` file, or a `..` escape to be refused with a named rule and change nothing, so that a
> URL handed to me by any application cannot become a filesystem primitive.

> As someone who screen-shares and records, I want the pill to show counts rather than note
> titles by default, a one-key toggle that suppresses content entirely, and a guarantee that
> notes I marked private never reach the widget at all, so that a glanceable surface is not a
> disclosure channel.

> As a screen-reader user and as an automation author, I want everything the pill and card show
> to be available as labeled text from `mg-vault shell snapshot --text` and `--json`, so that the
> graphical surface is a convenience and never the only way to reach a fact.

> As a suite user, I want `mg-vault` and `mg-calr` installed together on Arch with one documented
> keybind set, and I want a version mismatch between them to be *reported* by `doctor` rather than
> silently producing a wrong card, so that upgrading one product cannot quietly break the other.

---

## 3. UX Specification

Branch Q has two distinct surfaces: a **desktop shell component** (pill and card, rendered by the user's Quickshell process, not by `mg-vault`) and a **CLI/keybind surface** (`mg-vault` subcommands, Hyprland binds, and the `mgvault://` URI handler). Both are specified in full below. The widget is emphatically not an application window: it has no title bar, no menu, no window rules, and it is drawn on the shell's layer surfaces.

### 3.1 Screen / view inventory

**Quickshell surfaces** — all new, all QML, all rendered by the existing shell at `~/dotfiles/config/quickshell/mgeist/`, installed by the package as documentation-grade example integration under `/usr/share/mg-vault/quickshell/` and **never auto-enabled**.

| Surface | Reached by | New / modified | Layout pattern |
|---|---|---|---|
| Vault pill (`Bar/modules/VaultPill.qml`) | always present in the bar | New | single bar module, `Widgets/Pill` + `BarText`, max 260 px |
| Vault card (`Panels/VaultPanel.qml`) | click the pill, or `SUPER + CTRL + M` | New | layer-shell popover anchored to the pill, 460×560 px max, internally scrolled |
| Vault service (`Services/Vault.qml`) | singleton, no surface | New | `Quickshell.Io.Process` + `SplitParser`; owns poll timer, handshake, last frame |
| Action confirm strip | inside the card, only when `shell.actions = true` | New | inline two-step confirm row, never a floating dialog |
| Result toast | after a card action | New | the shell's existing `Services/Osd` surface, ≤2 lines, 4 s |

The pill has **six mutually exclusive states**, each carrying a distinct glyph **and** a distinct text token, never distinguished by color alone: `current`, `stale`, `degraded`, `unavailable`, `unconfigured`, `hidden`.

**CLI and keybind surfaces** — all new `mg-vault` subcommands unless noted.

| Surface | Invocation | New / modified | Notes |
|---|---|---|---|
| Interface manifest | `mg-vault contract describe [--json]` | New | prints the compiled-in manifest verbatim; the widget's first call |
| Shell snapshot | `mg-vault shell snapshot [--json\|--text] [--window N] [--allow-stale] [--require-current]` | New | the pill/card payload and its complete textual equivalent (Lens-5 5C) |
| Privacy control | `mg-vault shell privacy [show\|set MODE]` | New | writes XDG config only; never touches a vault |
| Deep-link handler | `mg-vault open-uri URI [--print-plan] [--json]` | New | the `x-scheme-handler/mgvault` target |
| Suite health | `mg-vault doctor --check suite [--json]` | Modification of **C**'s `doctor` | peer detection, contract versions, skew verdict |
| Desktop entry | `/usr/share/applications/mg-vault.desktop` | New | `Exec=mg-vault open-uri %u`; association is an opt-in user step |
| Hyprland binds | `~/dotfiles/config/hypr/keybindings.lua` | Modification (user's config; shipped as a documented snippet) | §3.4 |

### 3.2 Interaction flows

**Primary flow — handshake, glance, expand.**

1. At shell start, `Services/Vault.qml` runs `mg-vault contract describe --json` **once**. It compares `data.interface_version.major` against the major the QML was written against. A mismatch puts the pill in `unconfigured` with the text `vault interface v<n> unsupported`, and **no further mg-vault process is ever spawned** for the session. A missing binary is `unavailable` with `mg-vault not installed`.
2. On its poll timer (default 60 s; §4.7) the service spawns `mg-vault shell snapshot --json --no-input --no-color`, reads exactly one JSON object from stdout, and reads the exit code. Only one child may be in flight; a tick that finds one running is skipped, not queued.
3. Exit 0 → validate the envelope, read `data.freshness`, and choose the pill state by the total function in §4.2. `presentable_as_current == true` → `current`, and index-derived numbers may be rendered. Anything else → the corresponding state word plus the age, and **no index-derived number is rendered on the pill at all**.
4. Nonzero exit → `unavailable` or `degraded` per the typed error code. The previous frame is retained **only** so the card can render a labeled stale block; it is never promoted back into a `current` pill.
5. Clicking the pill (or `SUPER + CTRL + M`) opens the card, which immediately requests one fresh snapshot at the card interval (15 s) and, if freshness is not current, additionally requests `mg-vault shell snapshot --json --allow-stale` to populate the explicitly labeled "last known" block.
6. Closing the card returns the service to the pill interval. Hiding the bar, locking the session, or a compositor idle signal suspends polling entirely and kills nothing that is already running.

**Branch — index behind the source.** The service does not decide this; `mg-vault` does. The snapshot command performs the same confined double-observation `PersistentIndexStore::verify_freshness` already performs, and reports `stale` with `source_drift_paths`. The pill switches to `! stale 2h` within one interval. The card shows a full-width banner, and every row in the last-known block is prefixed with the literal token `stale`.

**Branch — index service unavailable.** Freshness state `unavailable`; `unavailable_capabilities` lists `search`, `tasks`, `backlinks`. Index-derived counts are `null` with `withheld_reason: "index_unavailable"`. **Direct-read facts survive**: vault name, canonical display root, trash entry count, and whether today's periodic note exists are computed by confined direct reads (branch A) and are still rendered, each labeled `provenance: direct-read`. The pill therefore degrades to a smaller true statement rather than going blank or lying.

**Branch — deep link.** An application invokes `mg-vault open-uri "mgvault://open?…"`. `open-uri` validates the grammar (§4.3), resolves the vault by *registered name only*, confines the path through branch A, then launches the configured open target. Read/navigate actions never prompt. `mgvault://capture` opens a prefilled composer and requires an explicit in-app confirmation; under `--no-input` it refuses with `confirmation_required` and writes nothing. Any validation failure exits nonzero, writes nothing, and emits a desktop notification naming the rule that was violated — never the resolved outside-vault path.

**Branch — card action (opt-in only).** With `shell.actions = true`, a task row exposes `toggle` and `promote to mg-calr`. Both are two-step: the first activation reveals an inline confirm strip naming the exact file, line, and fingerprint; the second runs one short-lived child carrying `--expected <fingerprint>` from the snapshot. A fingerprint mismatch returns branch A's `conflict`, the toast says the note changed, and the card refreshes. With the default `shell.actions = false` the strip does not exist and no mutating argv is reachable from the QML.

**Branch — version skew with `mg-calr`.** The card's suite row is populated from `mg-vault doctor --check suite --json`, which probes `mg-calr --version` on `PATH`. Absent → `mg-calr: not detected` and promotion is labeled `available: false`. Unknown major → `calr_contract_incompatible` (branch J's code) with both versions named; the row is red **and** carries the word `incompatible`, and promotion is refused rather than attempted.

### 3.3 Layout descriptions

**Pill anatomy**, leading → trailing, one row, max 260 px, title (when shown) is the only element allowed to shrink:

```
[glyph] [state token when not current] [counts]
 
current:       ﴜ 4213 · 37↺            (notes · open tasks)
current+today: ﴜ 4213 · 37↺ · ●        (● = today's periodic note exists)
stale:         ! stale 2h              (no number, ever)
degraded:      ! degraded · 3 paths    (drift count is source-observed, not indexed)
unavailable:   × index unavailable
unconfigured:  ? vault interface v2 unsupported
hidden:        ◌ vault hidden
```

**Card anatomy**, top → bottom:

```
┌ mg-vault · notes ─────────────────────────── ~/notes ┐
│ freshness: current · generation 41 · observed 3s ago │   ← always first, never omitted
├──────────────────────────────────────────────────────┤
│ counts                                               │
│   indexed notes   4213      index · gen 41           │
│   open tasks        37      index · gen 41           │
│   inbox unfiled      3      index · gen 41           │
│   trash entries      2      direct-read              │
├──────────────────────────────────────────────────────┤
│ today  journal/2026/2026-08-30.md   exists  direct   │
├──────────────────────────────────────────────────────┤
│ open tasks (5 of 37)                                 │
│  [ ] Draft the Q3 retro      projects/alpha.md:12    │
│  [/] Refactor the ladder     journal/…/08-29.md:13   │
├──────────────────────────────────────────────────────┤
│ suite   mg-calr 0.1.0 · contract 1 · compatible      │
├──────────────────────────────────────────────────────┤
│ [ Open in terminal ]  [ Refresh ]  [ Privacy: bal. ] │
└──────────────────────────────────────────────────────┘
```

- **Data sources.** Every field is read from exactly one `shell snapshot` envelope. The card performs no arithmetic on the numbers, no merging across frames, and no derivation of its own. `provenance` and `generation` are rendered next to every count precisely so that two numbers from different sources can never look like one consistent view.
- **Freshness line is mandatory and first.** It is never dropped by responsive layout, never collapsed into an icon, and never replaced by a colored dot.
- **Empty states.** No registered vault: `No vault registered. Run mg-vault vault register.` No vault selected: names `mg-vault vault select`. Empty vault: counts render `0` with freshness `empty` — which is a true state produced by a completed empty reconciliation, not an error. No open tasks: `No open tasks in the indexed generation.` — never `No tasks`, which would imply a claim about source that a generation cannot support.
- **Stale block.** When freshness is not current, the counts section is replaced by a banner `Showing last known values from 07:12 (2h old) — index is stale (3 source paths changed)`, and every value line is prefixed with the literal token `stale`. Reduced emphasis is applied *in addition*, never instead.
- **Theming.** The pill and card take every color from the shell's `Theme` singleton (`Theme.fg`, `Theme.accent`, `Theme.red`, `Theme.yellow`, `Theme.iconFontFamily`) so they track all 38 palettes and the wallpaper-derived `auto` palette without modification. **No literal color is permitted in the QML** — §5.2 scans for `#RRGGBB` and fails the build on a hit. Geometry, type, and motion come from `Theme` for the same reason.

### 3.4 Input & gestures

**Hyprland binds.** Shipped as a documented snippet for `~/dotfiles/config/hypr/keybindings.lua`, following that file's existing namespacing (`SUPER` window management, `SUPER + SHIFT` launch, `SUPER + CTRL` shell panels, `SUPER + ALT` variants) and its `panel()` helper, which dispatches `qs -c mgeist ipc call <target> <action>`. Every bind carries a `description` so it appears in the shell's keybindings panel.

| Bind | Action | Namespace rule |
|---|---|---|
| `SUPER + CTRL + M` | toggle the vault card | panel |
| `SUPER + CTRL + C` | toggle the `mg-calr` card | panel; the suite's other half |
| `SUPER + ALT + M` | cycle `shell.privacy` `balanced` ↔ `counts` | variant of the bare action |
| `SUPER + SHIFT + M` | launch the terminal running `mg-vault ui` | launch applications |
| `SUPER + CTRL + SHIFT + M` | open today's periodic note in a terminal (`mgvault://periodic?kind=daily`) | panel-adjacent |

None of these chords is currently bound in `keybindings.lua` (verified at read time); the snippet is additive and the package installs nothing into the user's Hyprland config.

**Card keyboard operation.** The card is fully keyboard operable and takes focus only on explicit activation, never when it refreshes on a timer. `Tab`/`Shift-Tab` move between sections and the action row; `j`/`k` and `↑`/`↓` move the row selection; `Enter` opens the selected row's deep link; `r` refreshes; `p` cycles privacy; `Esc` closes and returns focus to the compositor's prior window. Every action reachable by pointer has a key; there is no pointer-only path.

**Mouse.** Left click toggles the card. Right click on the pill cycles privacy. Scroll over the pill does nothing (a bar module that changes state on stray scroll is a misfeature). No drag, no gesture, no long-press semantics.

**Specialized input.** N/A — no stylus, controller, camera, or voice input. Assistive input reaches both surfaces through the compositor's and Qt's accessibility stacks, which is why every action has a keybinding and an accessible name rather than a gesture.

**Responsive behavior.** The pill never exceeds 260 px and never reflows the bar; under pressure it drops, in order: the periodic-note dot, the task count, the note count — never the state token. The card is capped at 460×560 logical px and scrolls internally; where 560 would exceed 60% of the output height (4K at scale 2, or a 200% font scale) it becomes a full-height side panel. At a font scale where a row cannot fit, rows **wrap**; the path and the freshness line are never the elements that get truncated.

**Deep-link grammar — what opens where.** A deep link is a URL any application may hand to `mg-vault`. It can navigate and open. It can never mutate or delete.

| URI | Opens | Mutates |
|---|---|---|
| `mgvault://open?vault=notes&path=projects%2Falpha.md` | that note in the open target | no |
| `mgvault://open?path=projects%2Falpha.md&heading=Design&pane=vsplit` | that note, scrolled to the heading, in a vertical split | no |
| `mgvault://open?path=notes%2Fa.md&line=12` | that note at line 12 | no |
| `mgvault://search?q=ripgrep&mode=text` | the terminal running `mg-vault ui` with the search overlay primed | no |
| `mgvault://periodic?kind=daily&date=2026-08-30` | today's periodic note **if it exists**; otherwise a confirmation offering to create it | no, until confirmed |
| `mgvault://graph?focus=projects%2Falpha.md` | branch G's graph view focused on that note | no |
| `mgvault://capture?title=…&body=…` | a prefilled composer requiring explicit confirmation (gated on branch L) | no, until confirmed |

`pane` accepts `focused` (default), `split`, `vsplit`, `tab`, `preview` and maps onto branch E's window grammar. The open target is `shell.open_target`: `tui` (default — spawn `shell.terminal` running `mg-vault ui --open …`), or `editor` (branch C's `$VISUAL`/`$EDITOR` handoff). **v1 always opens a new session**; it does not attach to a running TUI, because branch E specifies no control socket and inventing one here would be a second, unspecified IPC surface. Two concurrent sessions on one vault are safe by construction: branch A's fingerprint-checked optimistic writes make the second session's save fail closed rather than clobber (§8 Q4 revisits attach).

### 3.5 Transitions & animation

- **The pill never animates.** No pulse, no blink, no marquee, no attention motion when a count changes, and specifically no motion when freshness degrades — the state token and glyph carry that, and motion on a persistently visible surface is both a distraction and an accessibility hazard. Counts change without transition.
- **The card** uses the shell's existing panel transition, capped at 120 ms, and is reduced to an instant show/hide when the compositor or the shell reports a reduced-motion preference or when `shell.animate = false`. Reduced-motion behavior loses no information because no state is carried by motion in the first place.
- **The CLI surfaces have no animation at all.** `shell snapshot` prints one result; there is no spinner and no progress bar. Long operations do not exist here — every command in the manifest is bounded by a deadline (§4.7).

### 3.6 Error states

Every row below changes **zero** vault bytes. Presentation choice: the pill gets a state token because it is a single-row surface with no room for a sentence; the card gets a full-width banner because the condition applies to the whole view; the CLI gets a stderr block in branch A's error envelope because a CLI has no banner and inline-in-stdout errors contaminate pipes.

| Trigger | Presentation | Recovery | Data loss |
|---|---|---|---|
| `mg-vault` not on `PATH` | pill `unavailable` + `mg-vault not installed`; card banner names the expected command | install the package | none |
| `contract describe` major unknown | pill `unconfigured`; **all further calls suppressed for the session** | update the widget or `mg-vault` | none — no call is made |
| index service unavailable | freshness `unavailable`; index counts `null` with `withheld_reason`; direct-read facts still shown | start the service; `mg-vault index rebuild` | none |
| index stale (source drift) | pill `stale` + age, **no number**; card stale banner + per-row `stale` token | `mg-vault index rebuild` | none |
| index degraded (incomplete scan) | pill `degraded` + drift path count; card lists the diagnostic codes | inspect `mg-vault index status`; rebuild | none |
| malformed or over-size JSON on stdout | treated as an error frame, discarded whole, **never partially parsed**; cap 2 MiB | report defect | none |
| child exceeds its deadline | `SIGTERM` then `SIGKILL`; toast `snapshot timed out`; **not retried faster than the interval** | check in a terminal | none — read path |
| `deep_link_invalid` | grammar violation: unknown action, unknown or duplicate key, over-length, double-encoded, control character | notification names the rule; exit 2 | none |
| `unsafe_path` | deep link naming an absolute path, `..`, a symlink escape, a non-`.md` target, or a `.obsidian`/`.mg-vault` first component | notification names the rule violated; the resolved outside-vault path is **never printed or logged in clear**; exit 5 | none — refused before any open |
| `unknown_vault` / `no_vault_selected` | deep link names an unregistered vault, or none is selected | names `mg-vault vault register` / `select`; exit 3 | none |
| `confirmation_required` | `mgvault://capture` or a card action under `--no-input` without confirmation | names the exact flag that would satisfy it; exit 2 | none |
| `conflict` on a card action | fingerprint from the snapshot no longer matches the file | toast `note changed since it was read`; card refreshes; **no overwrite is offered** | none — branch A refuses the write |
| `calr_contract_incompatible` | `mg-calr --version` major unknown | suite row `incompatible` with both versions; promotion refused, not attempted | none |
| `promotion_recorded_but_not_linked` | branch J's cross-product partial: `mg-calr` created the todo, the note write failed | toast names the short ID and the `task link-calr` command; exits nonzero | none — reported, never faked |
| shell privacy `hidden` | pill renders `◌ vault hidden`; **no child process is spawned at all** | cycle privacy | none |

No error message, log line, notification, or toast may contain note body text, a snippet, a property value marked secret, a query literal, an absolute path outside the vault, an environment value, or `mg-calr`'s connection string.

### 3.7 Accessibility

- **Accessible names, roles, hints (widget).** The pill is a button named `Vault notes: <state>, <counts or state detail>`; it exposes its state as text, not as a color. Each card section is a labeled group; each row is a list item whose accessible name is the same semantic sequence the textual mode emits — `task, todo, Draft the Q3 retro, projects/alpha.md line 12, index generation 41`. Each action is a button with an explicit verb name (`Toggle task "Draft the Q3 retro"`, never `✓`). The freshness line is announced on card open and again whenever the state word changes.
- **Complete textual equivalent (Lens-5 5C, and an auto-fail gate).** `mg-vault shell snapshot --text` prints every datum the card renders, as `label: value` lines in a fixed order, with provenance and generation on every count. §5.2 asserts **set equality** between the card's rendered model and the `--text` output for 20 fixture states: the card may add no datum and omit none. There is no fact reachable only through the graphical surface.
- **Color independence (Lens-5 5D).** All six pill states carry a distinct glyph *and* a distinct word. `stale`, `degraded`, `unavailable`, `unconfigured`, `hidden`, `current`, `empty`, `compatible`, `incompatible`, `direct-read`, and `index` are always spelled out. Stripping all color from any rendering leaves it semantically complete; the CLI equivalent honors `--no-color` and `NO_COLOR` (presence wins over config), and the text mode is verified at 40, 60, 80, and 120 columns with paths, generations, error codes, and recovery commands wrapped rather than truncated.
- **Text scaling.** The card honors the shell's font scale; at large scales rows wrap and the card becomes a side panel (§3.4). The freshness line, the path, and the state token are never the elements dropped. In the terminal, scaling is the user's font size and the obligation is correct display width for wide, combining, and RTL characters in note titles and paths.
- **Control-character safety.** Note titles and paths are attacker-influenced text. Every string is escaped before rendering: in the terminal as `\u{…}` for C0/C1, ESC, CSI, OSC, and bidi overrides; in QML as plain text with rich-text interpretation **disabled** (`textFormat: Text.PlainText`), so a title containing markup cannot become markup. Truncation is grapheme-cluster safe (Lens-2 2D), never mid-codepoint.
- **Focus.** Focus order equals visual order; focus is never trapped; the card never steals focus on a timer refresh; `Esc` always returns focus to the previously focused window.
- **Privacy is accessibility-adjacent here.** A screen reader announcing note titles in a shared room is the same disclosure as rendering them. `shell.privacy` therefore governs the announced text identically to the drawn text — the widget cannot announce a title it was never sent (§4.3).

---

## 4. Implementation Specification

### 4.1 Architecture placement

Target placement against the existing three-crate workspace (`crates/mg-vault-core`, `crates/mg-vault-index`, `crates/mg-vault-cli`):

- `crates/mg-vault-cli/src/shell/mod.rs` — `shell snapshot` and `shell privacy`. Composes existing application calls; owns no filesystem policy and no SQL.
- `crates/mg-vault-cli/src/shell/snapshot.rs` — assembles `ShellSnapshot` from branch A direct reads and branch B/G/J index responses, applies privacy filtering **before serialization**.
- `crates/mg-vault-cli/src/shell/text.rs` — the `--text` renderer; a pure function of `&ShellSnapshot`, which is what makes textual-equivalence testable.
- `crates/mg-vault-cli/src/contract.rs` — `contract describe`, printing `include_str!` of the manifest verbatim.
- `crates/mg-vault-cli/src/uri.rs` — the `mgvault://` parser, validator, and launch planner. Pure parse/validate functions plus one impure launch step, so the whole grammar is unit-testable without spawning anything.
- `contracts/shell-interface-v1.json` + `contracts/shell-interface-v1.lock` — the public manifest and the frozen digest over its `stable` subset.
- `contracts/shell-snapshot-v1.schema.json` — the payload schema referenced by every read entry.
- `integrations/quickshell/{VaultPill.qml,VaultPanel.qml,Vault.qml,README.md}` — first-party QML, **vendored in this repository** precisely so `mg-vault`'s own test suite can assert what it does and does not do. Installed to `/usr/share/mg-vault/quickshell/`, never into a user's shell.
- `integrations/hypr/mg-vault-keybindings.lua` — the documented bind snippet of §3.4.
- `packaging/arch/{mg-vault,mg-suite}/PKGBUILD` — branch R owns `mg-vault`; Q adds the `mg-suite` meta-package and the packaging contract assertions of §4.6.
- `crates/mg-vault-cli/tests/{shell_interface_contract,deep_link,quickshell_boundary}.rs`.

**Layering rules, binding on branches A–P.** `shell/` and `uri.rs` depend on `mg-vault-core` and on the index/query façades only; they may not open a `rusqlite::Connection`, may not contain a SQL string literal, and may not construct a path from URI text without going through branch A's `VaultDir` confinement. `integrations/quickshell/` is a separate process tree and depends on nothing but an `mg-vault` executable on `PATH`. No component of Q may reference `mg-calr`'s database, config, or projection file; the only permitted `mg-calr` contact is branch J's `crates/mg-vault-cli/src/integrations/calr.rs`, which is already specified as the sole subprocess-spawning module for that peer.

### 4.2 Data model

**No database migration. No schema change. No new table, column, or index.** Branch Q adds no persistent state to the vault and none to the SQLite projection. Its wire types:

```rust
// Author: Jeff
// Date: 2026-08-30
// Description: Desktop shell snapshot contract types
// Notes: Serialized shape is the public contract; field removal needs schema 2

// ── Freshness provenance — mandatory on every payload ─────────────
/// Truthful evidence about the derived index behind this snapshot
///
/// `presentable_as_current` is the single boolean a client may use to decide
/// whether an index-derived number may be shown without a stale label
#[derive(Debug, Clone, Serialize)]
pub struct FreshnessProvenance {
    /// One of empty, current, stale, degraded, rebuilding, unavailable
    pub state: FreshnessState,
    /// Published generation, or None when nothing is published
    pub indexed_generation: Option<u64>,
    /// When the confined source observation was taken, RFC 3339 UTC
    pub observed_at: String,
    /// Age of that observation at serialization time
    pub observation_age_ms: u64,
    /// Source paths observed to differ from the published generation
    pub source_drift_paths: u32,
    /// False whenever a client must not render a number as live
    pub presentable_as_current: bool,
    /// Capability names that cannot be answered right now
    pub unavailable_capabilities: Vec<String>,
    /// One sentence naming the evidence, for the card and the text mode
    pub evidence: String,
}

// ── Counted facts carry their own provenance ──────────────────────
/// One number plus where it came from
///
/// A count is never a bare integer in the payload: `provenance` distinguishes
/// an index generation from a confined direct read, so two numbers from
/// different sources can never be rendered as one consistent view
#[derive(Debug, Clone, Serialize)]
pub struct ProvenancedCount {
    /// None when withheld because the index is not current
    pub value: Option<u64>,
    /// index or direct-read
    pub provenance: Provenance,
    /// Present only for index provenance
    pub generation: Option<u64>,
    /// Present only when value is None
    pub withheld_reason: Option<String>,
    /// True only under --allow-stale
    pub stale: bool,
}

// ── Privacy is applied before serialization, never in the client ──
/// How much note content may leave the process
///
/// Full permits titles everywhere, Balanced is the default and keeps titles
/// off the persistently visible pill, Counts suppresses every title, Hidden
/// causes the widget to spawn no process at all
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PrivacyMode {
    Full,
    Balanced,
    Counts,
    Hidden,
}

// ── Deep links are parsed into a closed enum, never a free path ───
/// A validated navigation intent from an untrusted URI
///
/// Construction is only possible after grammar validation and branch A path
/// confinement, so a value of this type cannot name a location outside the
/// vault and cannot express a mutation
#[derive(Debug, Clone)]
pub enum DeepLink {
    Open { vault: VaultName, path: VaultRelativePath, anchor: Option<Anchor>, pane: Pane },
    Search { vault: VaultName, query: BoundedQuery, mode: SearchMode },
    Periodic { vault: VaultName, kind: PeriodKind, date: Option<CivilDate> },
    Graph { vault: VaultName, focus: Option<VaultRelativePath> },
    Capture { vault: VaultName, draft: BoundedDraft },
}
```

The pill state is a **total function** with no fallthrough, which is what makes §5.1's exhaustive test possible:

```rust
// ── Pill state selection ──────────────────────────────────────────
/// Choose the single pill state from client-observable facts
///
/// Nonzero exit can never produce Current, and a snapshot whose freshness is
/// not presentable as current can never produce Current either
pub fn pill_state(exit: ExitStatus, frame: Option<&ShellSnapshot>, privacy: PrivacyMode)
    -> PillState
{
    if matches!(privacy, PrivacyMode::Hidden) { return PillState::Hidden; }
    match (exit.code(), frame) {
        (Some(0), Some(snapshot)) if snapshot.freshness.presentable_as_current => PillState::Current,
        (Some(0), Some(snapshot)) => PillState::from(snapshot.freshness.state),
        _ => PillState::Unavailable,
    }
}
```

**Client-side persistence.** The widget persists exactly two things in the shell's own per-user storage: the last frame (one, capped) and the privacy mode. Storing the last frame is permitted **only** because §3.3 requires it to be rendered as a labeled stale block with an age; presenting it as current is forbidden and is asserted by a widget test. `shell.privacy = hidden` clears the cached frame. Nothing is ever written inside a vault, inside `.obsidian`, or inside `.mg-vault` by branch Q.

**Configuration.** A `[shell]` table in the XDG config (`$XDG_CONFIG_HOME/mg-vault/config.toml`), not in `.mg-vault`, because these are per-workstation desktop preferences rather than portable vault settings (Lens-1 1D): `privacy` (default `balanced`), `actions` (default `false`), `poll_seconds` (60), `card_poll_seconds` (15), `open_target` (`tui`), `terminal`, `exclude` (glob list), `animate`, `deadline_ms`.

### 4.3 API contracts

This subsection is the load-bearing part of the branch: it is where "the widget never reads the index or the vault" stops being a claim and becomes an artifact plus a test.

**Manifest (`contracts/shell-interface-v1.json`).** One JSON document compiled in with `include_str!` and printed verbatim by `mg-vault contract describe --json`. Entry shape:

```json
{
  "id": "shell.snapshot",
  "argv": ["shell", "snapshot", "--json", "--no-input", "--no-color"],
  "stability": "stable",
  "mutates_vault": false,
  "mutates_config": false,
  "network": false,
  "schema_ref": "contracts/shell-snapshot-v1.schema.json",
  "exit_codes": [0, 2, 3, 5, 7],
  "since": "0.2.0",
  "deprecated_since": null
}
```

The v1 surface is exactly:

| id | argv shape | mutates vault | Purpose |
|---|---|---|---|
| `contract.describe` | `contract describe --json` | no | interface version + entry list; the widget's first call |
| `shell.snapshot` | `shell snapshot --json --no-input --no-color [--window N]` | no | pill and card payload |
| `shell.snapshot.stale` | `shell snapshot --json --allow-stale --no-input` | no | the card's explicitly labeled last-known block only |
| `shell.text` | `shell snapshot --text --no-color` | no | the Lens-5 5C textual equivalent |
| `search.read` | `search QUERY --json --limit N --no-input --no-color` | no | branch G search from the card |
| `task.list` | `task list --json --state todo --limit N --no-input` | no | branch J task rows |
| `periodic.path` | `periodic path --kind daily --json --no-input` | no | index-free today's-note resolution |
| `doctor.suite` | `doctor --check suite --json --no-input` | no | peer detection and skew verdict |
| `shell.privacy.set` | `shell privacy set MODE --json --no-input` | no (config only) | the privacy keybind |
| `task.toggle` | `task toggle PATH:LINE --expected FP --json --no-input --yes` | **yes** | opt-in card action, `shell.actions = true` only |
| `task.promote` | `task promote PATH:LINE --json --no-input --yes` | **yes** | opt-in card action; branch J's contract, unchanged |

Every entry uses branch A's version-1 envelope: `{"version":1,"ok":true,"command":…,"data":{…}, "warnings":[]}` on stdout, `{"version":1,"ok":false,"error":{"code":…,"message":…,"details":{…}}}` on stderr, with the other stream empty, and branch C's stable exit categories (`0` success, `2` usage/input, `3` not found/selection, `4` collision/ambiguity/conflict, `5` unsafe or denied, `6` degraded dependency, `7` I/O, `130` interrupt). Every entry passes `--no-input`, so no client can ever be blocked on a prompt or served a chooser. Clients always send vault-relative paths taken verbatim from a payload; a client that constructs a path itself is a client defect.

**Stability guarantee.** Within `shell_schema: "mg.shell/1"`, an entry marked `stable` guarantees: the argv shape keeps accepting the same arguments; JSON field names, types, enum values, and the null-vs-absent policy do not change; and each listed exit code keeps its meaning. **Additions are permitted** — new optional fields, new flags whose defaults preserve behavior, new entries. Removing an entry or field, narrowing an enum, changing a type or meaning, or repointing an error code requires `mg.shell/2` **and** a deprecation window of at least one minor release during which the old entry keeps working with `deprecated_since` set and `mg-vault doctor --check clients` emits a warning row. An entry marked `provisional` carries no guarantee and must be advertised as such; a client must treat it as optional. Golden fixtures per entry per state are the compatibility contract, exactly as branch A's envelope fixtures are.

**Snapshot payload (`mg.shell/1`), abbreviated:**

```json
{"version":1,"ok":true,"command":"shell.snapshot","warnings":[],"data":{
  "shell_schema":"mg.shell/1",
  "interface_version":{"major":1,"minor":0},
  "producer":{"app":"mg-vault","app_version":"0.2.0"},
  "vault":{"name":"notes","namespace":"sha256:…","display_root":"~/notes"},
  "generated_at":"2026-08-30T09:14:02Z",
  "privacy":{"mode":"balanced","titles_included":false,"exclude_rules":3},
  "freshness":{"state":"current","indexed_generation":41,"observed_at":"2026-08-30T09:14:02Z",
    "observation_age_ms":120,"source_drift_paths":0,"presentable_as_current":true,
    "unavailable_capabilities":[],"evidence":"two matching confined observations vs generation 41"},
  "counts":{"indexed_notes":{"value":4213,"provenance":"index","generation":41,"stale":false},
    "trash_entries":{"value":2,"provenance":"direct-read","generation":null,"stale":false}},
  "periodic":{"kind":"daily","date":"2026-08-30","path":"journal/2026/2026-08-30.md",
    "exists":true,"provenance":"direct-read"},
  "items":[{"path":"projects/alpha.md","line":12,"kind":"task","state":"todo",
    "title":null,"title_suppressed":true,"fingerprint":"sha256:…",
    "provenance":"index","generation":41,
    "deep_link":"mgvault://open?vault=notes&path=projects%2Falpha.md&line=12"}],
  "suite":{"mg_calr":{"detected":true,"version":"0.1.0","contract_major":1,"skew":"compatible"}},
  "degraded":false,"diagnostics":[]}}
```

Binding payload rules:

1. `freshness` is **mandatory and non-nullable** on every response, including errors that still produce a snapshot.
2. When `presentable_as_current` is `false`, every `counts.*` and every `items[]` entry whose provenance is `index` is withheld — `value: null` with `withheld_reason`, `items: []` — **unless** `--allow-stale` is passed, in which case each carries `"stale": true` plus `as_of` and `age_seconds`, and the envelope carries a `warnings` entry. Direct-read facts are never withheld, because they were observed now.
3. `title_suppressed` makes the privacy decision visible in the payload. A suppressed title is `null`, not an empty string, so a client cannot render a blank as a real title.
4. **Filtering happens in `mg-vault`, before serialization.** The widget never receives an excluded or private title; there is no client-side redaction to get wrong.
5. Note body text, snippets, property values marked secret, and query literals never appear in any payload in v1. There is no snippet field to opt into.
6. `shell snapshot` returns exit **0** even when degraded, because it is a status command: a pill that receives exit 6 whenever the index is behind cannot distinguish "index stale" from "mg-vault broken", and the truthful envelope is strictly more informative. `--require-current` is the fail-closed form for scripts, returning exit 6 and no rows, matching branch G's query rule.

**Deep-link grammar and validation.** The URI is fully untrusted input from an arbitrary application.

```abnf
uri     = "mgvault://" action [ "?" params ]
action  = "open" / "search" / "periodic" / "graph" / "capture"
params  = param *( "&" param )
param   = key "=" value
key     = 1*32( ALPHA / "-" )     ; closed allowlist per action
value   = *2048( unreserved / pct-encoded )
```

- Exactly **one** percent-decoding pass. A decoded value is never decoded again; a value that decodes to a control character (U+0000–U+001F, U+007F), a NUL, `\n`, `\r`, or a bidi override is rejected.
- The scheme token is matched case-insensitively; everything after it is case-sensitive.
- **No userinfo, no host, no port, no fragment, no path segment after the action.** Duplicate keys, unknown keys, and unknown actions are **rejected**, not ignored — unknown input fails closed.
- Total URI ≤ 4096 bytes; `path` ≤ 1024; `q` ≤ 512; `body` ≤ 16 KiB.
- `vault` is a **registered vault name**, validated by branch A's name rules; it is never a filesystem path. Absent → the selected vault; none selected → `no_vault_selected`, exit 3.
- `path` must be vault-relative, must have no absolute prefix and no `.`, `..`, empty, or root component, must end in `.md`, and is resolved through branch A's descriptor-relative `VaultDir` confinement (`openat2` with `RESOLVE_BENEATH|RESOLVE_NO_SYMLINKS|RESOLVE_NO_MAGICLINKS`). A first component of `.obsidian` or `.mg-vault` is rejected. Any escape, symlink escape, or ancestor swap is `unsafe_path`, exit 5, refused before anything is opened, and the outside-vault resolved path is never printed or logged in clear — only a digest.
- **Argv injection is structurally impossible:** the desktop entry is `Exec=mg-vault open-uri %u`, which expands to exactly one argument, and the handler rejects an operand beginning with `-` before parsing. The handler spawns launch targets as an argv vector, never through a shell, with a sanitized environment.
- **No deep link mutates.** `open`, `search`, `periodic`, and `graph` are navigate-only. `periodic` on a missing note **does not create it**; it opens a confirmation offering branch J's idempotent `periodic open`. `capture` opens a prefilled composer requiring an explicit in-app confirmation. Under `--no-input`, both refuse with `confirmation_required` and write nothing. There is no delete action, no write action, no `--yes` reachable from a URI, and no way to express one.
- **Audit.** Every handled URI appends one bounded line to `$XDG_STATE_HOME/mg-vault/deeplink.log`: timestamp, action, vault name, decision (`accepted` / `rejected:<code>`), and — for rejections — a digest rather than the operand. Rotated at 1 MiB. Never note content, never a clear outside-vault path.
- `--print-plan --json` prints the resolved launch plan and exits without launching, which is both the automation form and what makes §5.2's non-mutation assertion cheap.

**Peer contract consumed from `mg-calr` (consumed, not defined).** Exactly branch J's surface and nothing more: `mg-calr --version`; `mg-calr todo add --title … --json --no-input`; `mg-calr todo show SELECTOR --json --no-input`. Branch Q adds one optional probe, `mg-calr contract describe --json`, used only if that command exists, for the skew verdict; its absence degrades to the `--version` probe rather than failing. Branch Q never invokes `mg-calr interop import-todo`, never opens PostgreSQL, never reads `mg-calr`'s TOML or its `todo-projection.json`. The shared `mg.interop/1` snapshot schema — already implemented on both sides — remains a **manual, read-only export/import vocabulary**, not a sync channel; branch Q only reads its version string for the skew matrix.

**Auth and limits.** There is no token and no session. Authorization is the OS process boundary: every child runs as the invoking user with that user's own registry and permissions, exactly as it would from a shell. No client is handed a database path, a socket path, or a connection string. Rate limiting is the widget's own interval floor plus the one-child-in-flight rule (§4.7); pagination is `--limit` on the two list entries, and the pill's payload is bounded by construction.

### 4.4 State management

- **Authority is unchanged and unchallenged.** Markdown files under the vault root are the sole source of truth; the SQLite projection remains disposable; branch Q adds no store, no cache of note content, no daemon, and no second reader. Its complete durable footprint is: a `[shell]` table in XDG config, a bounded deep-link audit log in XDG state, and one frame plus one privacy mode in the shell's per-user storage.
- **Ownership.** `mg-vault` owns every fact; the widget owns one immutable frame and replaces it wholesale. `ShellSnapshot` is constructed per invocation and never merged across invocations — there is no incremental update path and therefore no way for two generations to appear in one view.
- **Local vs. synced boundary.** Everything is local. There is no server, no account, and no sync. `mg-calr`'s database is a different local application's state, reached only through its CLI, and is never mirrored, cached, or reconciled here.
- **Offline / draft persistence: none, deliberately.** The widget queues nothing. A failed action is reported and dropped, never retried in the background — a queued offline mutation would be a second authority with its own conflict policy. A `capture` composer abandoned before confirmation leaves nothing behind.
- **Freshness is state, not decoration.** Every rendered surface carries an age and a provenance. The transition from `current` to `stale` is driven by `mg-vault`'s own confined double-observation, not by a client-side timer heuristic; the client's `stale_after` only governs how long it will keep showing a *labeled* last-known block before dropping it entirely.

### 4.5 Dependencies

- **New Rust dependencies: none.** `shell`, `contract`, and `open-uri` are built from `clap`, `serde`/`serde_json`, `sha2`, and `thiserror`, all already in `Cargo.toml`. URI parsing is hand-written against the closed grammar of §4.3 rather than pulling a general URL crate, because the grammar is deliberately narrower than RFC 3986 and a permissive parser would accept forms this spec rejects. `unsafe_code = "forbid"` and `clippy::pedantic = "deny"` stay; a dependency requiring either to be relaxed is rejected.
- **QML side: no dependency added by this repository.** `Quickshell`, `Qt`, and the user's shell are runtime prerequisites of the *desktop*, listed at most as `optdepends` on the package. The integration uses only `QtQuick`, `Quickshell`, and `Quickshell.Io` (`Process`, `SplitParser`) — the same primitives the existing `Services/*.qml` singletons already use.
- **Assets: none.** No font, icon, image, or sound file is added. The pill uses the shell's existing icon font via `Theme.iconFontFamily` and its existing `Widgets/Pill` and `Widgets/BarText` components.
- **Infrastructure: none.** No network service, no CDN, no daemon, no systemd unit, no D-Bus name. Branch B's index service remains branch B's and remains optional for everything Q does with direct-read provenance.

### 4.6 Platform-specific considerations

- **Compositor.** Wayland under Hyprland with Quickshell as the shell. Positioning uses `wlr-layer-shell` through Quickshell's own abstractions. The integration reads only the reduced-motion, idle, and lock signals the shell already exposes; it does not talk to Hyprland IPC directly. **X11 is untested and unclaimed.** A different Wayland shell (waybar, ags, eww) can consume the identical JSON — that is the point of specifying a command contract rather than a widget API — but only the Quickshell integration is first-party.
- **Lock screen.** The pill and card must be excluded from lock-screen and screenshot layers by the shell's layer rules, and the service must render nothing and spawn nothing while the session is locked. This is asserted by a widget E2E test, not left to configuration.
- **Version compatibility.** Rust edition 2024, `rust-version = "1.85"`. Quickshell's API moves fast, so `integrations/quickshell/README.md` records the Quickshell and Qt versions the QML was developed against, and the runtime handshake (`contract describe`) means an `mg-vault`/widget skew degrades to `unconfigured` rather than misrendering.
- **Feature flags and rollout.** `shell.actions` defaults to `false`, so the shipped widget is strictly read-only until a user opts in. The `mgvault://` scheme association is **not** activated by the package; the `.desktop` file is installed and `xdg-mime default mg-vault.desktop x-scheme-handler/mgvault` is a documented user step, because no maintainer script may run commands on the user's behalf.
- **Package suite on Arch.**

| Package | Owner | Contents | Versioning |
|---|---|---|---|
| `mg-vault` | branch R | binary, completions, man pages, `contracts/`, `.desktop`, `/usr/share/mg-vault/quickshell/`, `/usr/share/mg-vault/hypr/` | its own semver |
| `mg-calr` | `mg-calr`'s branch H | binary, completions, man pages | its own semver |
| `mg-suite` | **branch Q** | no executables; `SUITE.md`, the combined Hyprland snippet, the combined Quickshell README | semver of the *contract set*, bumped when either `mg.shell` or `mg.interop` major changes |

  `mg-suite` declares `depends=('mg-vault>=X' 'mg-calr>=Y')` where X and Y are **contract floors**, not lockstep pins: the two products version independently and are coupled only by the majors they speak. Cross-cutting packaging rules, enforced by a contract test (§5.2): the same file layout branch R fixes, nothing under `/etc`, no maintainer script that runs `sudo`, `systemctl`, `xdg-mime`, `pacman`, `psql`, or `mg-vault index rebuild`, and no unit enabled by any package.

- **Version skew — detected and reported, never silent.**

| Situation | Detection | Behavior |
|---|---|---|
| widget major ≠ `mg.shell` major | `contract describe` handshake at shell start | pill `unconfigured`; all further calls suppressed; text names both versions |
| widget minor < payload minor | same handshake | render normally; ignore unknown additive fields (that is what additive means) |
| `mg-calr` absent | `PATH` lookup in `doctor --check suite` | suite row `not detected`; promotion labeled `available: false`; every `mg-vault` feature still works |
| `mg-calr` major unknown | `mg-calr --version` probe | branch J's `calr_contract_incompatible`; both versions named; promotion refused, never attempted |
| `mg.interop` schema mismatch | string compare of `interop_schema` (implemented on both sides today) | export/import refuse whole; never a partial translation |
| one product upgraded alone | `mg-suite` version floors plus `doctor --check suite` | doctor prints the skew row and the exact upgrade command; nothing degrades silently |

### 4.7 Performance budget

Measured on the reference workstation against branch B's 100,000-note / 1,000,000-block corpus.

- **Poll cadence.** Pill 60 s default, card-open 15 s, **floor 15 s** (a shorter interval is rejected by config validation). Polling is fully suspended when the bar is hidden, the session is locked, or the compositor reports idle. Idle CPU with the bar hidden is 0%.
- **Process discipline.** One `mg-vault` child per tick; a tick that finds one in flight is skipped, never queued. At most one *mutating* child ever, and its button is disabled while it runs so a double-click cannot double-submit. Every child carries a deadline (`shell.deadline_ms`, default 3000 for reads, 15000 for a promotion per branch J) after which it gets `SIGTERM` then `SIGKILL`.
- **Latency.** `shell snapshot` with the index `current`: p95 ≤ 80 ms, p99 ≤ 200 ms — it is a metadata and counting query against a published generation plus a handful of confined stats, never a vault walk. With the index unavailable, the direct-read subset is p95 ≤ 25 ms because it stats a bounded set of known paths and never recurses. `open-uri` validate-and-launch p95 ≤ 30 ms excluding the launched program.
- **Payload.** ≤ 64 KiB for the default window; the widget rejects > 2 MiB outright without parsing. `--window N` is capped at 50 items.
- **Memory.** Snapshot command resident ≤ 24 MiB. Widget footprint inside the shell process ≤ 12 MiB including the single cached frame, which is capped at one frame.
- **Storage.** Config ≤ 4 KiB; deep-link audit log rotated at 1 MiB; nothing else. **Zero bytes written inside any vault.**
- **Startup.** Branch Q adds nothing to `mg-vault`'s startup path: `contract describe` prints a compiled-in string and exits, p95 ≤ 10 ms. The shell's own startup cost is one extra child process.
- **Network payload: zero.** No entry may open a socket.

---

## 5. Test Specification

### 5.1 Unit tests

| Name | Setup → assertion | Edge covered |
|---|---|---|
| `pill_state_is_total_and_never_lies` | table over (exit code × freshness state × privacy) → exactly one state per row; a nonzero exit **cannot** yield `current`; a snapshot with `presentable_as_current:false` **cannot** yield `current` | exhaustive state selection |
| `stale_snapshot_withholds_every_index_number` | freshness `stale` without `--allow-stale` → every index-provenance `value` is `null` with a `withheld_reason`, `items` is `[]`, direct-read values survive | the stale-as-current auto-fail |
| `allow_stale_labels_every_layer` | same input with `--allow-stale` → each value has `stale:true`, `as_of`, `age_seconds`; envelope carries a warning; text mode prefixes every line with `stale` | a copied fragment cannot lose the qualifier |
| `freshness_is_never_absent` | every constructible snapshot → `freshness` serializes non-null with all fields | mandatory provenance |
| `privacy_suppresses_titles_before_serialization` | `balanced` and `counts` over a fixture with titles → no title byte appears in stdout; `title_suppressed:true`; `hidden` → command refuses to run at all | 4F |
| `excluded_notes_never_enter_a_payload` | fixture with `shell.exclude` globs and branch B secret-marked properties → excluded paths and titles appear nowhere in stdout bytes, diagnostics, or the audit log | 4F |
| `deep_link_grammar_rejects_the_hostile_corpus` | `..`, `%2e%2e`, double-encoded `%252e`, absolute `/etc/passwd`, `file://`, NUL, CR/LF, bidi override, 8 KiB path, duplicate `path=`, unknown key, unknown action, missing scheme, `mgvault://open/extra` → typed `deep_link_invalid` or `unsafe_path`, always before any filesystem access | grammar fail-closed |
| `deep_link_path_confinement` | symlink inside the vault pointing out, symlinked parent, `.obsidian/app.md`, `.mg-vault/trash/x.md`, `note.txt` → `unsafe_path` exit 5, and the rejection message and log line contain no clear outside-vault path | 4A + auto-fail |
| `deep_link_cannot_express_a_mutation` | property test over 10,000 generated URIs → the parsed `DeepLink` is always a navigate variant; `Capture` never yields a write plan without a confirmation token | mutation is unrepresentable |
| `deep_link_rejects_flag_shaped_operands` | operand beginning with `-` or `--` → refused before clap parsing | argv injection |
| `text_mode_is_a_pure_function_of_the_snapshot` | same snapshot rendered twice → byte-identical; no clock or environment read inside the renderer | determinism |
| `titles_truncate_on_grapheme_boundaries` | emoji ZWJ sequences, combining marks, RTL text → truncation never splits a cluster and display width is computed in cells | 2D |
| `control_sequences_are_escaped` | titles containing ESC, CSI, OSC 8, NUL → text mode emits `\u{…}`, no raw control byte, column count unchanged | terminal injection |
| `count_provenance_is_never_mixed` | a snapshot mixing index and direct-read counts → each carries its own provenance and generation; no aggregate field sums across provenances | 3A |
| `skew_verdict_is_total` | (`mg-calr` absent / older major / same major / newer major / newer minor) → exactly one verdict each, and `incompatible` never yields an attempted promotion | suite skew |
| `poll_interval_floor_is_enforced` | config `poll_seconds = 2` → rejected at load with a named error | 4.7 budget |

### 5.2 Integration tests

- `shell_manifest_entries_all_exist` — parse every `argv` template in `contracts/shell-interface-v1.json` against the real clap command tree. **A manifest entry naming a nonexistent command or flag fails the build.** This is the test that makes branches A–P unable to break the boundary silently.
- `shell_manifest_stable_entries_are_frozen` — a checked-in `contracts/shell-interface-v1.lock` digest over the `stable` subset. Any change fails until the lock is regenerated with a note; a *removal* within `mg.shell/1` fails unconditionally.
- `shell_payloads_validate_against_schema` — run every non-mutating entry against fixture vaults in each freshness state (empty, current, stale, degraded, unavailable) and validate stdout against `contracts/shell-snapshot-v1.schema.json`, including that `freshness` is present in all five.
- `shell_non_mutating_entries_change_nothing` — snapshot every vault file's bytes and mtime and the index database's bytes before and after every `mutates_vault:false` entry and every `open-uri --print-plan` over the hostile URI corpus; assert byte equality. **This is the executable form of "a deep link never mutates".**
- `quickshell_sources_never_reach_the_index_or_the_vault` — scan `integrations/quickshell/**` for `rusqlite`, `sqlite`, `.db`, `LocalStorage`, `QSqlDatabase`, `Quickshell.Io.FileView`, `readFile`, `XMLHttpRequest`, `.mg-vault`, `.obsidian`, and any absolute path literal. Any hit fails. **This is the executable form of "Quickshell never reads the index or the vault files directly."**
- `quickshell_sources_only_invoke_manifest_commands` — extract every `mg-vault` and `mg-calr` argv literal from the QML/JS and assert each matches a `stable` manifest entry; assert every `mutates_vault:true` invocation is lexically guarded by the `shell.actions` setting. An ad-hoc flag fails the suite.
- `quickshell_sources_use_no_literal_colors` — scan for `#RRGGBB` / `#RRGGBBAA` literals and for `textFormat: Text.RichText`; assert none, so the 38-palette theming and the plain-text rendering guarantees hold by construction.
- `shell_snapshot_matches_individual_commands` — field-by-field parity between `shell snapshot` and the union of `index status`, `task list`, `periodic path`, and the trash count, on the same vault in the same generation. The aggregate may not disagree with its parts.
- `text_mode_and_card_model_are_set_equal` — for 20 fixture states, assert set equality between the card's rendered model (dumped by the widget test harness) and `shell snapshot --text`. The card may add no datum and omit none. **This is the Lens-5 5C auto-fail gate.**
- `shell_entries_make_no_network` — run every manifest entry inside `unshare -rn`; all must succeed. Any future entry with `"network": true` would need an explicit allowlist entry, not an omission.
- `no_component_reaches_mg_calr_storage` — assert no PostgreSQL client, TLS stack, or socket-capable crate is reachable from the dependency graph of `shell/`, `uri.rs`, or `integrations/calr.rs`, and that under a sandboxed run the only descriptors the promotion path opens are the vault note, the receipt log, and the subprocess pipes (extends branch J's assertion to the widget path).
- `packaging_scriptlets_are_unprivileged` — scan `packaging/arch/**` for `sudo`, `systemctl`, `xdg-mime`, `psql`, `pacman`, and `index rebuild`; assert nothing installs under `/etc`, no unit is enabled, and `mg-suite` ships no executable.
- `deep_link_audit_log_is_bounded_and_clean` — 10,000 links including rejections → the log rotates at 1 MiB, contains no note content and no clear outside-vault path, and every rejection line carries a digest and a code.

### 5.3 UI / E2E tests

- **Widget E2E** (headless Quickshell in a nested compositor, `mg-vault` replaced by a scripted stub returning canned envelopes and exit codes): assert each of the six pill states renders with its glyph **and** its word; assert a `stale` frame renders **no index-derived number on the pill**; assert an error frame never replaces `current` content with an unlabeled last-known block; assert the card is reachable and fully operable by keyboard; assert unknown `interface_version.major` suppresses every subsequent call; assert a 2 MiB+ stdout is discarded whole rather than partly parsed; assert a child exceeding its deadline is killed and toasted and not retried early.
- **Freshness-transition E2E** (real binary, real fixture vault): start with the index current → pill shows counts; edit a note behind the index → within one interval the pill shows `stale` with an age and no number, the card shows the banner and per-row `stale` tokens; run `index rebuild` → the pill returns to `current`. Assert no intermediate frame ever showed a number without a label.
- **Degraded E2E:** stop the index service mid-session → pill `unavailable`, index counts withheld, direct-read facts (vault name, trash count, today's note existence) still rendered and labeled; assert the widget opened no database file (verified with `strace`-style descriptor auditing on the shell process).
- **Deep-link E2E:** invoke the handler through `xdg-open` for one accepted link per action and for the full hostile corpus; assert the accepted ones launch exactly the planned argv and the rejected ones produce a notification naming the rule, exit nonzero, and leave the vault byte-identical.
- **Privacy E2E:** cycle `full → balanced → counts → hidden`; assert no title text appears in the rendered scene graph at `counts`, that `hidden` spawns **no** `mg-vault` process at all, and that the widget renders nothing while the session is locked.
- **Suite E2E:** with `mg-calr` absent, present-and-compatible, and present-with-an-unknown-major, assert the card's suite row and `doctor --check suite --json` agree, that promotion is offered only in the compatible case, and that the incompatible case names both versions.
- **Textual-mode PTY E2E:** `shell snapshot --text` at 40, 60, 80, and 120 columns, with `NO_COLOR`, `--no-color`, non-TTY redirection, and a screen-reader linear transcript; assert no state depends on color and that no path, generation, error code, or recovery command is truncated.

### 5.4 Visual / manual verification

- **Theme variants:** pill and card against the lightest and darkest of the 38 palettes plus the wallpaper-derived `auto` palette; confirm the stale and error banners meet contrast in every case and that removing color removes no meaning. Terminal text mode on light and dark profiles at truecolor, 256-color, 16-color, and `NO_COLOR`.
- **Text size extremes:** shell font scale 100% and 200%; confirm rows wrap rather than dropping the path or the freshness line, and that the card converts to a side panel where specified. Terminal font at the smallest and largest practical sizes.
- **Screen size extremes:** 1080p single output and a 4K output at scale 2; confirm the pill never reflows the bar at any state, including the longest state string (`vault interface v2 unsupported`).
- **Empty vs. populated:** unregistered, registered-but-empty, 5 notes, 4,213 notes, and the 100,000 note corpus; each of `current`, `stale`, `degraded`, `unavailable`, `unconfigured`, `hidden`; each reachable by deliberate manual setup (stop the service, edit behind the index, rename the binary, bump the interface version, set privacy).
- **Suite view:** `mg-calr` installed, absent, and skewed; confirm the suite row reads correctly and that no card element implies a shared database.

---

## 6. Compliance & Safety Gate

### 6.1 Sensitive data classification

- [ ] No sensitive data involvement
- [x] **Handles sensitive data — and puts it on a persistently visible surface, which is the distinguishing risk of this branch.** In scope: note titles, vault-relative paths, task text, counts, vault names, and the canonical root's display form. Protections: privacy filtering applied **inside `mg-vault` before serialization**, so the widget never receives what it must not show; `shell.privacy` defaulting to `balanced`, which keeps titles off the always-visible pill; a one-key cycle to `counts` and a `hidden` mode that spawns no process; mandatory exclusion from lock-screen and screenshot layers and no rendering on a locked session; branch B's secret/private exclusions plus a `shell.exclude` glob list honored in every payload; **no note body text, snippet, or property value in any v1 payload**; all rendered strings escaped and forced to plain text; a bounded audit log that records digests rather than rejected operands; and no client ever handed a database path, socket path, or connection string.
- [x] **Uses synthetic/test data only until the compliance gate clears** — every fixture, golden, widget stub, and hostile-URI corpus uses synthetic notes and `example.invalid` references.

### 6.2 Asset provenance

- [x] **No third-party assets.** No font, icon, image, sound, model, or data file is added by branch Q. The pill uses the shell's already-installed icon font by reference through `Theme.iconFontFamily` and the shell's existing `Widgets/Pill` and `Widgets/BarText` components. All QML under `integrations/quickshell/` is first-party.
- [ ] Uses third-party assets

Third-party *code*: none added. Branch Q introduces zero new Rust dependencies (§4.5). Quickshell and Qt are runtime prerequisites of the user's desktop, not dependencies of this repository, and appear at most as `optdepends`.

### 6.3 Language / claims audit

- [x] **Makes claims not supported by evidence? No.** §7.1 states the true state — branch Q is **absent** — with the required vocabulary, against a named commit. The banner at the top says the same. Nothing here claims a working pill, card, deep link, or package.
- [x] **Promises capabilities not yet built? No — and it is enforced in shipped text.** `mg-vault --help` must not mention Quickshell, a pill, a card, or a deep link until the corresponding entry exists; `mg-vault contract describe` must list only entries that actually exist and must mark anything unproven `provisional`; the card labels a feature provided by another branch with `provided_by` and `available:false` rather than rendering a dead control, matching branch J's rule. A test asserts the help text contains none of the forbidden terms while the feature is absent. The card never uses the word "synced" about `mg-calr`, because nothing is synced.
- [x] **Uses language restricted by domain regulations? No.** `mg-vault` is a personal knowledge tool; no health, financial, or legal claim is made. The word `current` in any surface refers strictly to a published index generation verified by a confined source observation, and is never used loosely.

### 6.4 Regulatory alignment

**Lens 3 — Knowledge Retrieval and Structure, criterion by criterion.**

- **3A Determinism.** Every number and row the widget shows is bound to one published generation and is stamped with `provenance` and `generation`. The snapshot is assembled from a single freshness observation, never merged across frames or across provenances, and `shell_snapshot_matches_individual_commands` asserts the aggregate cannot disagree with the commands it composes. The widget performs no derivation of its own: it renders fields. Links shown in the card come from branch G's resolver, not from a client-side heuristic.
- **3B Ambiguity.** The widget never resolves anything. A deep link whose `path` does not resolve uniquely, or whose target is ambiguous under branch G's rules, returns the typed ambiguity error and **the widget shows `ambiguous — resolve in the terminal` rather than choosing**. `open-uri` never picks a candidate, never mutates to disambiguate, and never falls back to a basename match that branch G's resolver refused.
- **3C Query depth.** Branch Q defines no query grammar; it consumes branch G's `search` and branch J's `task list` through the manifest, with their filters, limits, deterministic ordering, and generation-bound cursors intact. The card exposes a deliberately small subset — open tasks and one text search — and every richer predicate is reachable through the CLI equivalent named in the card's own footer, so the widget narrows the surface without narrowing the product.
- **3D Derived authority.** The pill and card are views over views: they render a projection of a projection of ordinary files, and neither can write. `contracts/` documents that the payload is derived, `provenance` says so on every count, and the boundary tests make it impossible for a client to reach past the derivation into the store. No widget state ever feeds back into the index or the vault.
- **3E Scale.** The snapshot is a bounded metadata query against a published generation plus a fixed set of confined stats; it never walks the vault. Its budget (§4.7) is stated against branch B's 100,000-note / 1,000,000-block corpus, `--window` is capped at 50 items, payload is capped at 64 KiB with a 2 MiB client-side hard reject, and the poll floor plus one-child-in-flight rule bound the process cost regardless of vault size. Nothing in branch Q blocks editing: every command is read-only and short-lived.

**Other lenses, briefly.** **1A** the widget is never an authority and holds no store; **1B** branch Q writes no note bytes at all, so nothing can be lost outside an explicitly confirmed branch J action; **1C** deep links address notes by vault-relative path and mint no identifier — `mgvault://` carries a path, never a UUID; **1D** `.obsidian` is never read or written and desktop preferences live in XDG config, not in the vault; **1E** the one mutating path reuses branch A's fingerprint-checked atomic write and branch J's cross-product partial-failure reporting, which reports rather than fakes. **2A** every widget action has a keybind and an exact CLI equivalent; **2B** branch Q owns no editing and cannot overwrite newer source — a card action carries `--expected` and fails closed; **2C** deep links target branch E's documented pane grammar; **2D** grapheme-safe truncation and escaping; **2E** all six degraded states are explicit words and direct source editing is never blocked. **4A** deep-link confinement through branch A's `VaultDir` with named-rule refusals; **4B** every mutating entry carries an expected fingerprint by construction; **4C** the widget is deny-by-default (`shell.actions = false`), holds no capability, and is given no credential; **4D** no destructive action is reachable from the widget or a URI, and nothing claims false success; **4E** a versioned, locked, contract-tested manifest with golden fixtures per entry per state; **4F** privacy filtering before serialization, no bodies or snippets in payloads, and a clean bounded audit log. **5A** every entry runs inside `unshare -rn`; **5B** explicit poll, latency, memory, payload, and deadline budgets; **5C** the `--text` set-equality gate; **5D** color-independent state words at 40–120 columns with `NO_COLOR`; **5E** stable `--json`, `--no-input`, `--no-color`, stdout/stderr discipline, and `--print-plan` for scripted use.

**Auto-fail rules, addressed by name.** *Stale index presented as current* — `freshness` is mandatory, index numbers are withheld unless `presentable_as_current`, the pill shows **no number** when not current, `--allow-stale` labels every layer including individual rows, and three tests (`stale_snapshot_withholds_every_index_number`, `allow_stale_labels_every_layer`, the freshness-transition E2E) enforce it. *Unsafe traversal or symlink escape* — the deep-link grammar and branch A confinement, with a hostile corpus and a property test. *Index state overriding source* — the widget writes nothing and the payload is explicitly derived. *Unconfirmed overwrite* — no URI can express a write; card actions are two-step and fingerprint-checked. *Partial multi-file mutation* — branch Q performs no multi-file operation; branch J's cross-product partial is reported with a recovery command and a nonzero exit. *Capability/data-exfiltration bypass* — actions are off by default, the widget is handed no credential or path, and payload filtering happens before serialization. *Graph or card information lacking a textual equivalent* — the `--text` set-equality gate. *Active raw HTML or script by default* — QML rich text is disabled and asserted.

---

## 7. Gap Analysis vs. Current State

### 7.1 What exists today

Verified against the working tree at commit `dfe33cf` on 2026-08-30.

- **The entire branch — absent.** There is no `integrations/` directory, no `contracts/` directory, no `packaging/` directory, no QML, no `.desktop` file, and no URI handler anywhere in the workspace. `crates/mg-vault-cli/src/main.rs`'s `Command` enum is exactly `Vault`, `Note`, `Index`, `Search`, `Interop` — there is no `shell`, `contract`, `open-uri`, `doctor`, `task`, or `periodic` subcommand. A repository-wide grep for `mg-calr`, `Quickshell`, and `mgvault://` matches nothing outside one forward-looking sentence in `docs/PRODUCT.md`.
- **`mg-calr` integration — absent from this repository**, and branch J (which owns promotion) is itself specified but unimplemented. `mg-calr` exists as a separate product at version 0.1.0, with `src/tui.rs` prototyped and its own Quickshell branch deferred; its `contracts/client-interface-v1.json` does not exist yet either.
- **Available and relied upon — implemented.** The version-1 JSON envelope and per-variant error codes in `crates/mg-vault-cli/src/main.rs`; canonical vault confinement, collision-safe creation, fingerprinted optimistic writes, and vault-local trash in `crates/mg-vault-core/src/vault.rs` and `atomic.rs`; XDG resolution in `xdg.rs`; the named-vault registry in `registry.rs`; the disposable SQLite projection with explicit schema/parser versions, atomic generations, and the four-state `empty` / `current` / `stale` / `degraded` freshness model with confined double-observation in `crates/mg-vault-index/src/lib.rs`; and the deterministic `mg.interop/1` snapshot exporter in `crates/mg-vault-core/src/interop.rs` — whose `PRODUCER_APP` / `vault_namespace` / `global_id` conventions are exactly what the suite's identity story is built on, and whose schema string `mg.interop/1` is validated by `mg-calr`'s `src/interop.rs` today. Sixty-two workspace tests pass.
- **Gated.** `shell snapshot`'s index-derived counts are gated on branch B's service and query foundation; the card's task rows on branch J; the card's search on branch G; the `pane` parameter's targets on branch E; `mgvault://capture` on branch L; the packaging layout on branch R.
- **Planned.** Everything in §3 and §4 of this document.

### 7.2 Delta to spec

**New files.** `crates/mg-vault-cli/src/shell/{mod,snapshot,text}.rs`; `crates/mg-vault-cli/src/contract.rs`; `crates/mg-vault-cli/src/uri.rs`; `contracts/{shell-interface-v1.json,shell-interface-v1.lock,shell-snapshot-v1.schema.json}`; `integrations/quickshell/{VaultPill.qml,VaultPanel.qml,Vault.qml,README.md}`; `integrations/hypr/mg-vault-keybindings.lua`; `packaging/arch/mg-suite/PKGBUILD`; `packaging/desktop/mg-vault.desktop`; `docs/SUITE.md`; `crates/mg-vault-cli/tests/{shell_interface_contract,deep_link,quickshell_boundary,packaging_contract}.rs`; `crates/mg-vault-cli/tests/fixtures/{shell/*.json,uri/hostile.txt}`.

**Modified files.** `crates/mg-vault-cli/src/main.rs` — add the `Shell`, `Contract`, and `OpenUri` subcommands and route them; extend `doctor` with `--check suite` when branch C's `doctor` lands. `crates/mg-vault-cli/src/main.rs` error mapping — add `deep_link_invalid`, `shell_unconfigured`, and `interface_incompatible` to the typed set with their exit categories. `README.md` and `docs/ARCHITECTURE.md` — document the client-interface boundary and the "no client opens the index" rule. `docs/SECURITY.md` — add the deep-link threat model and the URI validation rules. `packaging/arch/mg-vault/PKGBUILD` (branch R's file) — install `contracts/`, the `.desktop` file, and `/usr/share/mg-vault/{quickshell,hypr}/`.

**Migrations / schema changes.** **None.** Branch Q adds no table, column, index, or migration to the SQLite projection and no schema to any vault file. The only new persisted artifacts are a `[shell]` table in XDG config and a bounded audit log in XDG state.

**New dependencies.** **None** (§4.5).

### 7.3 Estimated scope

**L overall**, decomposing very unevenly, which is the argument for slicing it rather than scheduling it as one unit:

- **Manifest + contract tests + `contract describe` — S.** Small, self-contained, needs nothing else, and is the piece that should land **first and early**, before any other branch hardens, because its whole value is constraining branches A–P.
- **`shell snapshot` (`--json` and `--text`) with freshness provenance — M.** The command is thin, but the freshness plumbing, the withholding rules, the privacy filtering, the provenance-per-count model, and the five-state fixture matrix are where the care goes. Direct-read-only mode can ship before branch B, which makes it independently useful.
- **`open-uri` grammar, validation, confinement, and the hostile corpus — M.** Little code, high adversarial test load: this is the branch's real attack surface and the property test plus the corpus are most of the work.
- **The QML pill, card, and service — M.** Three files against a stable contract, but the evidence requirements (headless Quickshell in a nested compositor, source-scanning boundary tests, the set-equality gate against `--text`) are where the effort sits.
- **`mg-suite` packaging, `.desktop`, keybind snippet, skew reporting — S**, gated on branch R.

### 7.4 Blocking dependencies

- **Hard blocker for the widget's index-derived content: branch B (`b-index-service-search`) being *implemented*, not merely specified** — the service, IPC, and freshness/generation contract are the substrate for every index-provenance count. Until B lands, `shell snapshot` can still ship with the direct-read subset and freshness state `unavailable`, which is a genuinely useful and completely truthful pill; that is the recommended first slice.
- **Branch A's confinement capability gate** (`VaultDir` / `openat2` with `RESOLVE_BENEATH`) must land before `open-uri` accepts any path, because lexical validation alone is not a confinement claim. Until it does, `open-uri` must refuse with `confinement_unavailable` rather than fall back.
- **Branch C** for the stable exit categories, the `command`/`warnings` envelope fields, and `doctor` (which `--check suite` extends). **Branch E** for the `pane` targets and `mg-vault ui --open`. **Branch G** for `search` and for the ambiguity contract the deep-link resolver defers to. **Branch J** for `task list`, `task toggle`, `task promote`, and `periodic path` — and note that J is where the `mg-calr` boundary is actually defined; branch Q consumes it and adds only the skew probe. **Branch L** for `mgvault://capture`. **Branch R** for the package layout `mg-suite` depends on.
- **External:** an `mg-calr` release that publishes its own `contract describe` would let the skew verdict be exact rather than version-string-based; its absence degrades gracefully and does not block. Quickshell and Qt are the user's, not this project's.
- **Nothing in branch Q blocks any earlier branch.** The single obligation Q places on branches A–P is the manifest and its contract test, so that "stable" is enforced rather than assumed.

---

## 8. Open Questions

- **Q1:** Should `integrations/quickshell/` be vendored in this repository (assumed throughout, because it is what makes `quickshell_sources_never_reach_the_index_or_the_vault` an executable test in `mg-vault`'s own suite), or should the QML live in `~/dotfiles/config/quickshell/mgeist/` with only the JSON contract owned here? — **blocks:** §4.1 placement, §5.2's three source-scanning tests, and whether the package installs anything under `/usr/share/mg-vault/quickshell/`. The same question is open on `mg-calr`'s side (its Q2); the two should be answered the same way.
- **Q2:** Is a 60-second poll of `shell snapshot` sufficient forever, or should `mg-vault` grow a `provisional` `shell watch --json-lines` streaming entry once branch B's service exists? Polling is simpler, holds no long-lived connection, and cannot leak a process; a stream reduces latency and process churn. — **blocks:** one optional manifest entry only; the pill works either way.
- **Q3:** Should the default `shell.privacy` be `balanced` (specified: pill shows counts, card shows titles) or `counts` (no titles anywhere until explicitly enabled)? `balanced` is the more useful default and keeps content off the always-visible surface; `counts` is the more conservative one. — **blocks:** §3.3's card rendering and §4.2's default only.
- **Q4:** Should a deep link be able to attach to an already-running `mg-vault ui` session instead of spawning a new one? Attaching needs a TUI control socket that branch E does not specify, which is a new IPC surface with its own authentication and injection questions; spawning is safe today because branch A's fingerprinted writes make concurrent sessions fail closed rather than clobber. — **blocks:** §3.4's open target and a possible branch E addition.
- **Q5:** Should `mg-suite` exist as an Arch meta-package at all, or is documentation plus independent packages enough? A meta-package makes the contract floors machine-checkable by `pacman`; it also creates a third versioned artifact to maintain for two products that deliberately version independently. — **blocks:** §4.6's packaging table and §5.2's packaging contract test.
- **Q6:** Should the card be able to show a one-line search snippet, which is by far the most useful thing it could add and is also note *body* text on a persistently visible surface? The spec's answer is no snippet field in v1, deliberately. — **blocks:** §4.3's payload and §6.1's classification if reversed.

Resolved by this spec and **not** open: that the widget consumes public commands and never opens the SQLite index, a vault file, or `mg-calr`'s database; that the boundary is a versioned manifest with a lock and contract tests in `mg-vault`'s own suite; that every payload carries mandatory freshness provenance and the pill shows no index-derived number unless the index is verifiably current; that a deep link can navigate and open but can never express a mutation or escape the canonical vault root; that privacy filtering happens inside `mg-vault` before serialization and `hidden` spawns no process; that each product owns its own store and cross-product action goes through branch J's explicit, previewed, one-directional CLI promotion; and that version skew between the two products is detected at a handshake and reported, never absorbed.
