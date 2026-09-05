# Spec: Periodic Notes, Templates, and Tasks

**Feature ID:** j-periodic-templates-tasks
**Parent feature:** root
**Spec author agent:** Hermes Agent (periodic/templates/tasks subagent)
**Date:** 2026-08-29
**Iteration:** 1

---

## 1. Purpose

### 1.1 One-sentence job

Give the vault a calendar spine and a work surface — deterministic daily/weekly/monthly/quarterly/yearly notes that open idempotently under an explicit local-day boundary, a deliberately non-programmable template language that fills them in without ever becoming a code-execution surface, first-class Markdown checkbox tasks that toggle without disturbing a single neighbouring byte, and an explicit, one-directional handoff that promotes one named task into `mg-calr` through its published CLI contract.

### 1.2 Why it matters

Periodic notes are the highest-frequency write path in a knowledge system: a daily note is opened more often than any other file, usually by reflex, often twice in one minute, sometimes from a script and a keybind at the same instant. That makes it the place where a tool most easily destroys work — by creating a second file for the same day, by re-applying a template over yesterday's writing, or by disagreeing with itself about when "today" starts on a laptop that crossed a timezone.

Templates are where notes apps quietly turn into interpreters. Obsidian's Templater evaluates arbitrary JavaScript inside the vault; a template shared in a forum post is then indistinguishable from a script with the user's full filesystem and network privileges. `mg-vault` refuses that trade: §3.2 and §4.3 specify a template language with no expressions, no recursion, no inclusion, no environment, no filesystem and no process — a closed variable allowlist evaluated in one bounded pass. Anything that genuinely needs computation is a plugin (branch **N**) and must go through **N**'s WASM sandbox and explicit capability grant. This is criterion **4C** (least privilege, deny-by-default) applied to the feature most likely to violate it, and "capability or data-exfiltration bypass" is an auto-fail rule.

Tasks matter because a checkbox is the single most commonly *rewritten* line in a Markdown vault, and the usual implementation — parse the file to an AST, mutate, print it back — reformats tables, renumbers lists, normalizes quoting, and eats plugin syntax. Criterion **1B** requires unknown syntax and properties to survive outside explicitly edited spans, so toggling a task must replace exactly the marker byte and nothing else.

And promotion matters because `mg-calr` is the sibling product with PostgreSQL authority over todos. The tempting integration — a daemon that keeps a Markdown checkbox and a Postgres row in sync — is two authorities for one fact, with a silent conflict winner and a partial cross-product mutation on every crash. `mg-vault` instead does one explicit, user-named, previewed handoff per task, through `mg-calr`'s documented `--json` CLI, and records the link as a plain visible property the user can read, grep, and delete.

### 1.3 Success signal

On a fixture vault exercised across a spring-forward day, a fall-back day, an ISO week-year boundary (2026-01-01 falls in `2025-W53`), a fiscal-quarter offset, and a laptop that changes host timezone mid-suite: `periodic open daily` invoked 100 times concurrently for the same instant produces exactly one file, exactly one create receipt, 99 `created: false` open receipts, and zero byte changes to that file after the first; no template application ever overwrites an existing note without an explicit typed confirmation; the template conformance corpus — which includes Templater `<% tp.system.prompt() %>`, Handlebars `{{#each}}`, shell `$(id)`, `${HOME}`, and a 4 MiB expansion bomb — produces byte-verbatim passthrough or a typed refusal in every case, and never a process, a socket, an environment read, or a file read; toggling every task in a 5,000-task fixture leaves each file byte-identical except the marker bytes, proven by a diff whose only hunks are single-character; and `task promote` with `mg-calr` absent, present-but-unprovisioned, and present-and-healthy yields respectively zero note bytes changed, zero note bytes changed, and exactly one appended visible inline field, with no PostgreSQL client, socket, or `DATABASE_URL` read reachable from the promotion module's dependency graph.

---

## 2. User Stories

> As a daily journaller, I want one keybind or one command to open today's note, creating it from my template only if it does not exist, so that reflexively pressing it twice never costs me writing.

> As a traveller, I want "today" to mean a day in an explicitly recorded timezone with an explicitly recorded day-start, and I want the tool to tell me when a DST transition changed that boundary, so that a flight does not silently split or merge my journal.

> As a template author, I want a template language that provably cannot run code, read my filesystem, read my environment, or reach the network, so that pasting a stranger's template into my vault is a formatting decision and not a security decision.

> As a user of an existing Obsidian vault, I want my Templater and Dataview syntax to survive unchanged in any file `mg-vault` touches, so that migrating in one direction never forecloses migrating back.

> As a planner, I want to toggle a checkbox from the CLI or the TUI and have the file be byte-identical except that one marker, so that my carefully aligned table two lines below is still aligned afterwards.

> As a screen-reader or no-color terminal user, I want task state, promotion state, and template preview outcomes spelled out as words in a stable order, so that no state is conveyed by a glyph or a colour alone.

> As a suite user, I want to name one task and push it into `mg-calr` with a preview and a confirmation, and I want the link recorded as visible text in my note, so that the two products stay honest without a background daemon deciding which of them is right.

> As an automation author, I want `periodic path`, `task list --json`, and `task promote --dry-run --json` to be stable, versioned, non-mutating, and safe under `--no-input`, so that a script can compute a path or plan a promotion without ever writing by accident.

---

## 3. UX Specification

### 3.1 Screen / view inventory

`mg-vault` is a CLI and TUI product. This feature introduces no graphical screen, no mouse requirement, no sound, and no haptics. Its views are line-oriented CLI output plus, once **E** lands, terminal panes that render the identical contracts.

| View | Entry point | New / modification | Layout pattern |
|---|---|---|---|
| Periodic open receipt | `mg-vault periodic open KIND [--date D]` | New | Receipt block: resolved period, path, `created` boolean, template provenance |
| Periodic path (read-only) | `mg-vault periodic path KIND [--date D]` | New | One vault-relative path on stdout, or a JSON record; never creates |
| Periodic explain | `mg-vault periodic explain KIND [--date D]` | New | Ordered derivation: instant, zone, day-start, DST rule applied, civil period, format, folder, final path |
| Periodic inventory | `mg-vault periodic list KIND --from D --to D` | New | One row per period in calendar order with `exists` / `missing` |
| Periodic config | `mg-vault periodic config show \| set \| import-obsidian` | New | Labelled settings block; `import-obsidian` is a previewed read-only adoption |
| Template inventory | `mg-vault template list` | New | Vault-relative template paths with placeholder counts and validity |
| Template render preview | `mg-vault template render PATH --dry-run` | New | Rendered body plus a placeholder resolution table |
| Template apply plan/result | `mg-vault template apply PATH --to TARGET --mode …` | New | Canonical plan (C's `--dry-run` schema), unified diff, then commit receipt |
| Task list | `mg-vault task list [FILTERS]` | New; front end over **G**'s query grammar | Freshness banner, one record per task, footer banner |
| Task show | `mg-vault task show PATH:LINE` | New | Grouped detail: location, marker, state, text, metadata by dialect, promotion |
| Task toggle plan/result | `mg-vault task toggle PATH:LINE [--to STATE]` | New | Before/after marker, byte range, fingerprints, `changed` boolean |
| Promotion plan/result | `mg-vault task promote PATH:LINE …` | New | Exact `mg-calr` argv, exact note delta, confirmation, then receipt |
| Promotion status (pull) | `mg-vault task calr-status PATH:LINE` | New; read-only | `mg-calr`'s reported state, labelled as pulled-on-demand, writes nothing |
| Doctor checks | `mg-vault doctor --check periodic\|templates\|tasks\|calr-links` | Modification of **C**'s `doctor` | Ordered checks with severity, evidence, repair command |
| TUI daily pane (E) | `<leader>j` in the TUI | New pane, owned by **E**, fed by J contracts | Opens or creates the current period note in the focused pane |
| TUI task pane (E) | `<leader>t` | New pane | List region over **G** results; `<Space>` toggles; `<leader>tp` promotes |
| TUI template picker (E) | `:template`, `<leader>i` | New chooser | Fuzzy list of templates with a live preview region; insert-at-cursor only |

The TUI panes are consumers, not owners: **E** supplies pane geometry, focus, and session restore; **D** supplies the buffer and undo; J supplies the model, the ordering, the state words, and every mutation plan. Every pane has a CLI equivalent producing the same records, so the whole feature is usable with no TUI at all.

### 3.2 Interaction flows

#### Resolving a period (the deterministic core)

Given a period kind and either an explicit `--date`/`--datetime` or the current instant:

1. **Obtain the instant.** `now()` once per command, or the operand. An operand that is a civil date (`2026-08-29`) skips steps 2–4 entirely: it *is* the civil period, and no timezone is consulted. This is why `--date` is the reproducible form for scripts.
2. **Obtain the zone.** `periodic.timezone` from `.mg-vault/settings.json` — an explicit IANA identifier, recorded at first configuration, **not** re-read from the host on every run. `--timezone` overrides for one command. If the setting is absent, the command refuses with `timezone_unconfigured`, prints the host zone as a *suggestion*, and creates nothing; a vault that travels between machines must not silently change what "today" means. `TZ` in the environment is deliberately ignored for period derivation and its value is never read.
3. **Apply the day-start offset.** `periodic.day_start` (default `00:00`, range `00:00`–`11:59`) shifts the boundary for night owls. The local civil date is `(instant → zone → local wall clock) − day_start`, then truncated to a date.
4. **Resolve DST explicitly.** Converting an instant to a wall clock in an IANA zone is total, so a `day_start` of `00:00` is never ambiguous and a daily note exists exactly once on every calendar day, including 23-hour and 25-hour days. A non-zero `day_start` names a *wall-clock* boundary that a transition can delete or duplicate:
   - **Gap** (spring forward; the boundary wall time does not exist): the boundary resolves **forward** to the first instant that does exist. The day is one hour shorter; there is still exactly one note.
   - **Fold** (fall back; the boundary wall time occurs twice): the boundary resolves to the **earlier** (first) occurrence. The day is one hour longer; there is still exactly one note.
   - Both rules are `compatible`-style, fixed, documented, and identical to the gap/fold policy `mg-calr`'s C8 uses, so the two products never disagree about a boundary. `periodic explain` prints `dst_rule: gap_forward` / `dst_rule: fold_earlier` / `dst_rule: none` and the pre- and post-resolution wall times. Nothing silently falls back to host-local interpretation.
5. **Truncate to the period.** Calendar arithmetic only — never seconds-per-day arithmetic:
   - `daily` — the civil date.
   - `weekly` — the containing week under `periodic.week.start` (`mon` default, `sun` allowed) and `periodic.week.numbering` (`iso` default, `us` allowed). ISO numbering pairs the ISO week-year with the ISO week; `us` numbering pairs the calendar year with a week-of-year count.
   - `monthly` — the civil year and month.
   - `quarterly` — a three-month block starting at `periodic.quarter.fiscal_year_start_month` (default `1`), so `Q1` is Jan–Mar by default and Apr–Jun under a fiscal-April vault. The quarter's *label year* is the fiscal year and is printed in `explain`.
   - `yearly` — the civil year.
6. **Render the path.** `folder` (a slash-joined template) and `format` (a filename template) are rendered from the closed date-token allowlist of §4.3, then `.md` is appended if absent. Month, weekday, and quarter names are emitted in a fixed English table; the process locale, `LC_TIME`, and the host calendar never affect a filename.
7. **Validate the path as untrusted input.** The result is split into components and each is required to be non-empty, not `.`, not `..`, free of `/` (the folder template's own separators having already been consumed), free of NUL and control bytes, and **not beginning with `.`** — which structurally forbids any format string from ever targeting `.obsidian` or `.mg-vault`. The whole path then goes through **A**'s `validate_note_path` and confined resolution, which reject traversal and symlink escape. A format that cannot produce a valid path is a configuration error reported at `periodic config set` time, not at 06:00 on a Monday.

#### Opening a period note (idempotency is the contract)

1. Resolve the path as above.
2. `stat` the path through **A**'s confined resolution.
3. **If it exists:** open it. `created: false`. **No template is applied, considered, or previewed. No byte is written. No frontmatter is touched. No property is stamped.** This is absolute: `periodic open` has exactly one write path and it is the creation of a file that did not exist.
4. **If it is absent:** render the configured template for that kind (or an empty body if none), then create with **A**'s `create_note`, which uses `create_new`/`O_EXCL` semantics.
5. **If the create loses a race** (`collision`), the command does not retry-with-overwrite and does not error out: it re-reads the now-existing note and returns the step-3 result with `created: false` and `race_detected: true`. Open-or-create converges. Two concurrent invocations produce one file, one create receipt, and one open receipt.
6. Report the exact path, the `created` boolean, the source fingerprint, and — when a template was applied — the template's vault-relative path and content digest, so the receipt says where the bytes came from.
7. `--print-path` and `periodic path` never reach step 4 at all; they are pure functions of configuration and the instant.
8. `periodic ensure --from D --to D` bulk-creates missing period notes. It is bounded (`--limit`, default 90, hard max 1,000), previews the full list with `--dry-run`, requires `--yes` under `--no-input`, creates each note as its own independent transaction, and on partial failure reports exactly which paths were created and which were not. It never overwrites an existing note, so a partial run is safely re-runnable.
9. **Format changes never orphan silently.** `.mg-vault/settings.json` keeps a `format_history` list per kind. If the current format yields a missing path but a historical format yields an existing note for the same period, `periodic open` **refuses to create a duplicate**: it reports `periodic_format_drift`, names both paths, opens the existing note, and prints the `mg-vault note move` command (owned by **C**/**H**) that would migrate it. Migration is never automatic.

#### Rendering a template (the safety spine)

A template is an ordinary Markdown note inside the vault — readable, diffable, syncable, and editable in Obsidian with no tool involvement. Rendering has four stages and the engine is reached only at stage 3.

1. **Read.** The caller resolves the template path through **A**'s confined read: vault-relative, `.md`, no traversal, no symlink escape, not under `.obsidian` or `.mg-vault`. The template engine itself never opens a file; it receives bytes.
2. **Scan.** A single left-to-right pass finds candidate `{{ … }}` spans. A span whose interior does **not** match the closed placeholder grammar of §4.3 is **not a placeholder**: its exact bytes are copied to the output verbatim and it is never evaluated. This is how `{{#each items}}`, `{{ tp.date.now() }}`, `{{{triple}}}`, and every other foreign dialect survive intact. Byte sequences that are not `{{ … }}` at all — `<% tp.system.prompt() %>`, `${HOME}`, `$(id)`, `<?php ?>`, Dataview blocks, callouts, YAML — are outside the scanner's alphabet entirely and are copied verbatim by construction.
3. **Evaluate.** Each real placeholder names one variable from a closed allowlist and zero to four formatters from a closed allowlist. Evaluation is a pure total function from `(TemplateContext, placeholder)` to a `String`. **The output is never re-scanned**: a value that happens to contain `{{` is emitted literally. There is no inclusion, no partials, no macros, no recursion, no branching, no iteration, no assignment, no comparison, and no arithmetic beyond the bounded `offset` formatter. Termination is structural rather than budgeted — one pass over a finite token list, each step producing a bounded string — and the budgets in §4.7 are a second, independent belt.
4. **Preview and commit.** `--dry-run` prints the rendered body, a resolution table naming each placeholder's variable, formatter chain, resolved value, and provenance (`builtin` or `plugin:<id>`), and a unified diff against the target. Commit uses **A**'s atomic transaction.

**A placeholder that lexes correctly but names an unknown variable or formatter is a hard error** (`template_unknown_variable` / `template_unknown_formatter`), reported with the byte offset inside the template, and **nothing is written**. It is never silently expanded to the empty string, because a silently empty variable is how a template quietly loses the content the author intended. `--literal-unknown` is an explicit opt-in that copies the unknown placeholder verbatim instead.

**What the engine cannot do, structurally.** The engine lives in its own crate (`mg-vault-template`, §4.1) whose dependency graph contains no filesystem, process, environment, network, time, or randomness capability. It cannot read a file inside the vault, let alone outside it; it cannot spawn `sh`; it cannot read `$HOME`; it cannot open a socket; it cannot generate a UUID. Its whole input is a `TemplateContext` of already-computed strings assembled by the caller. §5.1 asserts this mechanically over the crate's resolved dependency graph rather than trusting review.

**When a template genuinely needs computation** — "insert the sum of this month's expenses", "call my time tracker", "generate a table from a CSV" — that is not a template feature. It is a **plugin (branch N)** and must go through **N**'s versioned manifest, WASM sandbox, explicit per-capability grant, attribution, and preview. A plugin contributes variables under the reserved `plugin.<plugin-id>.<name>` namespace; those resolve **only** when the plugin is installed *and* has been granted the `template-variable` capability for this vault; otherwise they fail closed with `template_capability_denied` and nothing is written. A granted plugin variable's value is attributed in the resolution table and the diff, so the user always sees which bytes came from third-party code. There is no escape hatch, no `--allow-shell`, no `exec` formatter, and no configuration key that enables one.

#### Applying a template to a target

| Mode | Target state | Behaviour |
|---|---|---|
| `create` (default) | must not exist | Render, then `create_note` (`O_EXCL`). A collision is a typed error; there is no `--force`. |
| `append` | must exist | Replace exactly the zero-length span at end-of-file with the rendered bytes; every prior byte is preserved. |
| `prepend` | must exist | Replace exactly the zero-length span at offset 0, or immediately after a closing frontmatter delimiter when `--after-frontmatter` is given. |
| `insert-at-marker` | must exist | Replace exactly the span between a matched `<!-- mg-vault:template:NAME -->` … `<!-- /mg-vault:template:NAME -->` pair. Missing, malformed, nested, or unbalanced markers are `template_marker_invalid` and write nothing. |
| `replace` | must exist | **Guarded overwrite**, below. |

**Unconfirmed overwrite is an auto-fail rule, and `replace` is the only mode that can overwrite.** It requires *all* of: an explicit `--mode replace`; a shown unified diff; a confirmation in which the user types the exact vault-relative target path (interactive) or passes `--yes` **and** `--expected FINGERPRINT` (non-interactive — `--yes` alone is refused with `confirmation_required`); a successful **A** trash transaction capturing the prior bytes *before* the replace, so the previous version is recoverable by ID; and an atomic replace whose fingerprint precondition still holds at commit. Any of these failing aborts before the write. `periodic open` can never reach this mode.

Every non-`create` mode is a **span-local** edit through `Vault::edit_note_span` with an expected fingerprint: the output is provably `prefix ‖ rendered ‖ suffix` where `prefix` and `suffix` are the exact original bytes. Frontmatter is not reserialized, list numbering is not renormalized, line endings are not converted, trailing whitespace is not stripped, and unknown syntax anywhere in the file is untouched. This is criterion **1B** and it is asserted byte-for-byte in §5.1.

#### Tasks: reading, toggling, and querying

**Syntax surface.** A task is a GFM task-list item: an ordinary list item whose content begins with `[`, one marker character, `]`, then a space. `[ ]` is `todo`; `[x]` and `[X]` are `done`; **any other single character is preserved exactly and mapped through `tasks.markers` in settings**, whose defaults follow the widely used Obsidian conventions — `/` in-progress, `-` cancelled, `>` forwarded, `?` question — and whose fallback for an unmapped character is the state `other`, never `todo` and never `done`. `mg-vault` never rewrites a marker character it did not understand, never normalizes `[X]` to `[x]`, and never converts one convention to another.

**Metadata conventions**, all read as ordinary visible text, in three dialects that coexist in one vault:

- Dataview inline fields — `[due:: 2026-09-01]`, `(priority:: high)`.
- Tasks-plugin emoji shorthand — `📅` due, `⏳` scheduled, `🛫` start, `➕` created, `✅` done, `🔁` recurrence, `⏫`/`🔼`/`🔽` priority.
- Ordinary Markdown — `#tags`, `[[wikilinks]]`, and a trailing Obsidian block ID `^abc123`.

Each parsed datum records its byte range, its value, **and the dialect it came from**. When J writes a datum it uses the dialect already present on that line, falling back to `tasks.write_dialect` (default `dataview`) only for a line that has none. J never converts a dialect, never reorders existing metadata, and never "tidies" a task line.

**Toggling.** `task toggle PATH:LINE [--to STATE|--cycle]`:

1. Read the note and its fingerprint through **A**; locate the task; verify `task_hash` (SHA-256 of the task's text run) still matches the value the caller was shown, if one was supplied. A mismatch is `task_moved` — the line is identified, nothing is written, and re-running resolves it.
2. Compute the *marker span*: the one byte between `[` and `]`. The default toggle replaces exactly that one byte. If `--to` names a state whose configured marker is a different length (all configured markers are one character, so this is only reachable via a malformed config), the span is the three bytes `[`…`]` and the check that the result is still a valid task is made before writing.
3. Optional completion stamping (`tasks.stamp_completion`, **off by default**) adds a second span: the done-date datum appended at the end of the task's text run, in the line's existing dialect.
4. Both spans are applied to an in-memory copy and committed as **one** atomic replace with the fingerprint precondition — never two writes, never a partially stamped file. `--dry-run` shows both byte ranges and the resulting line.
5. A `🔁`/`repeat::` recurring task does **not** spawn its next occurrence automatically. `--recur` is explicit, previewed, inserts one new line adjacent to the completed one, and is part of the same single atomic write.
6. `task toggle` on a note with no such task, an ambiguous `PATH:LINE`, or a non-task line is a typed error with zero writes.

**Identity without hidden identifiers.** A task is addressed by `path:line[:column]` plus the disposable `task_hash` staleness check. There is no injected UUID, no hidden marker, no zero-width character, and no comment (criterion **1C**). A user who wants a durable handle adds a visible Obsidian block ID `^abc123`; J will *offer* that as an explicit, previewed, span-local mutation when a durable reference is needed, and never mint one on its own.

**Querying leans on G.** `task list` is a friendly front end over **G**'s `task.*` predicates (`task.state`, `task.marker`, `task.text`, `task.due`, `task.scheduled`, `task.done`, `task.count`) with `--state`, `--due-before`, `--due-after`, `--tag`, `--path-under`, `--project`, `--promoted`/`--unpromoted`, and `--period` (which resolves a period and scopes to that note). It inherits **G**'s contract wholesale: fail-closed freshness with exit 6 when the index is not `current`, `--allow-stale` labelling every row and both banners, deterministic ordering (path, then byte offset), opaque generation-bound cursors, and cancellation. When the index service is unavailable, `task list --source direct` performs a **clearly labelled** bounded live scan (`mode: direct-scan`, `freshness: unindexed-live`) restricted to an explicit `--path-under` subtree with a node cap; predicates needing projections are refused with `requires_index`. No task view ever presents an index row as note content: opening a result re-reads the file through **A** and compares fingerprints.

#### Promoting a task to `mg-calr`

Promotion is **explicit, per-task, previewed, confirmed, and one-directional**. `mg-vault task promote PATH:LINE [--project P] [--due D] [--priority …] [--tag …] [--dry-run] [--yes]`:

1. **Resolve the task** and refuse early if the line already carries a promotion field: `already_promoted`, printing the existing `mg-calr` reference. `--again` is the explicit override that permits a second todo.
2. **Locate `mg-calr`.** `integrations.calr.command` from settings, else `--calr-bin PATH`, else a `PATH` lookup of `mg-calr`. Not found is `calr_unavailable`: it names what was looked for and where, changes **zero bytes**, and exits before any plan is built. `mg-vault doctor` reports `mg-calr: not detected`, and `--help` / the capability JSON label promotion `provided_by: "mg-calr", available: false` rather than promising it.
3. **Check the contract.** Run `mg-calr --version` (cached per process, 5 s deadline). An unrecognized major version is `calr_contract_incompatible`, naming the version J speaks (`mg-calr todo add` per its C1 flag set, envelope `version: 1`) and the version found. J never screen-scrapes `mg-calr`'s human output and never guesses at a newer schema.
4. **Build the argv** — a vector, never a shell string, never `sh -c`, with no interpolation into a command line. Task text becomes `--title`; parsed or supplied metadata become `--due`, `--timezone`, `--priority`, `--project`, repeated `--tag`, `--notes`; `--no-reminder` is passed unless `--remind-before` is given, matching `mg-calr` C1's explicit-reminder rule; `--json` and `--no-input` are always passed so the call is non-interactive and machine-readable.
5. **Preview.** Print the exact argv (each element on its own line, control characters escaped), the exact note delta as a unified diff, and the target vault path. `--dry-run` stops here, having executed nothing and written nothing.
6. **Confirm.** Interactive mode requires `y`; `--no-input` requires `--yes`. `confirmation_required` otherwise.
7. **Call `mg-calr` first, write the note second.** The subprocess runs with a deadline (`integrations.calr.timeout`, default 15 s), inherited stdin closed, stdout/stderr captured, and a sanitized environment. Its `version: 1` envelope is parsed and validated; `ok:false` is surfaced verbatim as `calr_error` with `mg-calr`'s own code, message, and exit status, and **the note is not touched**.
8. **Record a plain visible link.** On `ok:true`, exactly one inline field is appended to the task line, in that line's dialect: `[calr:: 7f3a2c]` — `mg-calr`'s human-scale short ID. `--record-uuid` additionally writes `[calr-id:: <uuid>]`. Both are ordinary visible text: greppable, editable, deletable, portable, and rendered by Obsidian. Neither is `mg-vault` identity — `mg-vault` resolves notes only by path — and deleting either breaks nothing in `mg-vault`. `--no-backlink` records nothing at all. There is no hidden marker, no HTML comment, no zero-width character, and no frontmatter injection (**1C**).
9. **Partial cross-product failure is reported, never faked.** If step 8's note write fails or conflicts after step 7 succeeded, J reports `promotion_recorded_but_not_linked`, prints the created todo's short ID and UUID, prints the exact `mg-vault task link-calr PATH:LINE --id …` command that records the link later, and **exits nonzero**. It never claims success, and it never deletes the `mg-calr` todo — silently rolling back another product's authoritative record is worse than an unlinked one. A disposable receipt line is appended to `.mg-vault/state/calr-receipts.jsonl` before the note write, and `mg-vault doctor --check calr-links` lists receipts whose note link is missing.

**The reverse direction is pull-only and never automatic.** `task calr-status PATH:LINE` shells out to `mg-calr todo show ID --json --no-input` and *displays* the state; it writes nothing, ever. `task sync-back PATH:LINE` is a separate, explicit, single-task, previewed, confirmed command that sets the Markdown marker to match `mg-calr` — one task per invocation, `--yes` required under `--no-input`, refused when the note's fingerprint moved. **There is no background sync daemon, no watcher, no polling loop, no systemd timer, and no IPC channel between `mg-vault` and `mg-calr`.** A user who wants periodic reconciliation writes their own cron entry calling these commands, or installs a plugin under branch **N**'s capability grant.

**Database access is prohibited, not merely discouraged.** J never opens `mg-calr`'s PostgreSQL database, never reads `DATABASE_URL` or `mg-calr`'s TOML config, never connects to `/run/postgresql`, and never reads or writes `mg-calr`'s `todo-projection.json`. §5.1 asserts that no PostgreSQL client, TLS stack, or socket-capable crate is reachable from the promotion module's dependency graph, and §5.2 asserts under a sandboxed run that the only file descriptors J's promotion path opens are the vault note, the receipt log, and the subprocess pipes.

#### Degraded and unavailable

Any view whose data depends on the index shows the literal freshness word first, then what is still possible (period path derivation, period open, template render and apply, task toggle by explicit `PATH:LINE` — none of which need the index at all), then what is not (task queries, `--promoted` filters, cross-note task counts), then one recovery command. `periodic` and `template` are fully functional with the index service stopped, the database deleted, and `mg-calr` uninstalled; only `task list` and promotion degrade, and each says so in words.

### 3.3 Layout descriptions

**Periodic open receipt (human).** Reading order: resolved period, path, action, provenance.

```
period   daily 2026-08-29  (America/Los_Angeles, day starts 00:00)
path     journal/2026/2026-08-29.md
action   created
template Templates/Daily.md  sha256:1c9f…  12 placeholders resolved
source   sha256:9f2c…
```

An existing note replaces two lines and adds nothing else — the absence of a `template` line is itself the evidence that no template ran:

```
action   opened  (already existed; no template applied, no bytes written)
```

**Periodic explain.** A fixed ordered derivation, one `label  value` pair per line: `instant`, `zone`, `zone_source` (`settings` / `--timezone`), `wall_clock`, `day_start`, `dst_rule`, `civil_date`, `period_kind`, `period_start`, `period_end`, `week_numbering` (weekly only), `fiscal_year_start_month` (quarterly only), `folder_template`, `format_template`, `rendered_path`, `path_validation`, `exists`.

**Template resolution table.** One row per placeholder, in template byte order, so the table is a readable audit of every value that will be written:

```
offset  placeholder                       variable        value                provenance
   0:14 {{date}}                          date            2026-08-29           builtin
  61:96 {{date | offset:"-1d" | date:"YYYY-MM-DD"}}
                                          date            2026-08-28           builtin
 210:232 {{period.prev.path}}             period.prev.path journal/2026/2026-08-28.md  builtin
 301:340 {{plugin.expenses.month_total}}  plugin variable  DENIED               plugin:expenses (capability not granted)
verbatim 402:428 {{#each items}}          —               copied unchanged     not a placeholder
```

**Task list rows.** Fixed semantic order, every state a word: `path:line`, marker in brackets, state word, text, then labelled metadata, then promotion state.

```
Index: current  generation 41  observed 2026-08-29T18:04:11Z

journal/2026/2026-08-29.md:12  [ ]  todo     Draft the Q3 retro  due 2026-09-01 (dataview)  #work
journal/2026/2026-08-29.md:13  [/]  in-progress  Refactor the ladder  promoted calr:7f3a2c
projects/alpha.md:44           [x]  done     Ship the CLI  done 2026-08-27 (emoji)
Index: current  generation 41  3 tasks  order path,offset
```

**Task toggle receipt.** `path`, `line`, `before` marker, `after` marker, `byte_range`, `fingerprint_before`, `fingerprint_after`, `changed`, and, when stamping is on, the second span's range. A no-op toggle (already in the requested state) reports `changed: false` and performs no write.

**Promotion preview.** Three labelled blocks in fixed order — `will run` (argv, one element per line), `will change` (unified diff of the single task line), `will not change` (the literal sentence `mg-calr's database is never accessed by mg-vault; this runs mg-calr's CLI`) — then the confirmation prompt.

**Empty states.** `No templates found. A template is an ordinary .md note under Templates/ (configure with: mg-vault periodic config set templates.folder PATH).` — `No tasks match in indexed generation 41.` — `No periodic notes exist for daily between 2026-08-01 and 2026-08-29. Create them with: mg-vault periodic ensure daily --from 2026-08-01 --to 2026-08-29 --dry-run` — `This note has no tasks.`

**Data sources.** Period resolution is driven by `.mg-vault/settings.json` plus the process clock; template rendering by the template file's bytes plus the computed `TemplateContext`; task detail by **A**'s direct confined read of the note; task *lists* by **B**/**G**'s IPC responses with **A** revalidation on open; promotion state by the visible inline field in the note, and by `mg-calr`'s CLI when explicitly pulled. No view reads SQLite directly and no view reads `mg-calr`'s database at all.

### 3.4 Input & gestures

- Everything is keyboard-reachable. Mouse, touch, stylus, voice, camera, and controller input are N/A for this feature; if **E** later adds mouse click-to-focus in the task pane it must remain strictly redundant.
- CLI commands: `periodic open|path|explain|list|ensure|config`, `template list|render|apply|validate`, `task list|show|toggle|promote|link-calr|calr-status|sync-back`, and `doctor --check periodic|templates|tasks|calr-links`.
- Shared flags inherited from **C**: `--vault`, `--json`, `--jsonl`, `--dry-run`, `--yes`, `--expected`, `--no-input`, `--no-color`, `--ascii`. Retrieval flags inherited from **G** for `task list`: `--limit`, `--after`, `--sort`, `--deadline-ms`, `--allow-stale`, `--refresh`.
- Period flags: `--date`, `--datetime`, `--timezone`, `--offset` (`-1`, `+2` in period units), `--kind`, `--print-path`, `--no-template`, `--template PATH`, `--from`, `--to`, `--limit`.
- Template flags: `--to`, `--mode`, `--var NAME=VALUE` (repeatable; the **only** source of `prompt.*` values besides an interactive prompt), `--literal-unknown`, `--after-frontmatter`, `--marker NAME`.
- Task flags: `--to STATE`, `--cycle`, `--recur`, `--stamp`/`--no-stamp`, `--task-hash`, plus the filters listed in §3.2.
- Promotion flags: `--project`, `--due`, `--timezone`, `--priority`, `--tag`, `--notes`, `--remind-before`, `--record-uuid`, `--no-backlink`, `--again`, `--calr-bin`, `--timeout-ms`.
- `--no-input` forbids every prompt: an unresolved `prompt.*` variable, an unconfirmed `replace`, an unconfirmed promotion, or an unconfirmed `sync-back` becomes a typed error naming the exact flags that would satisfy it. Prompts, when allowed, are written to `/dev/tty`, never into a redirected data stream.
- Piped stdin is never assumed to be a template body or a task list; `--body-file`/`--stdin` per **C** are the explicit forms.
- TUI keymap (documented in **E**'s grammar, composable, Vim-consistent): `<leader>j` open the current period note, `<leader>J` a period chooser, `[j`/`]j` previous/next period, `<leader>t` task pane, `<Space>` toggle the task under the cursor (normal mode, in a buffer or the pane), `<leader>tc` cycle marker states, `<leader>tp` promote with the standard preview modal, `<leader>i` template picker with insert-at-cursor. Every binding appears in `:help mg-vault-periodic`, `:help mg-vault-tasks`, the command palette with its availability reason, and the corresponding `--help`.
- Responsive behaviour is verified at 40, 60, 80, and 120 columns. Tables become one-record-per-block below 60 columns. Paths, byte ranges, template placeholders, `mg-calr` IDs, error codes, and recovery commands are never truncated; they wrap with indentation.
- `NO_COLOR`, `--no-color`, a non-TTY stdout, and `--json` all disable styling, and JSON never contains escape sequences.

### 3.5 Transitions & animation

No animation exists anywhere in this feature and none is required. Opening a period note is a single atomic state change; the template preview is a static diff; a task toggle is a single redraw of one line. The only long-running operations are `periodic ensure` over a range and `task list` over a large corpus; both emit rate-limited static progress to stderr, at most four updates per second, in interactive human mode only, with stdout reserved for the result. Under `--json`, `--jsonl`, a non-TTY stdout, `--quiet`, `NO_COLOR`, or a reduced-motion environment setting, no dynamic cursor updates are emitted at all — progress becomes a queryable static snapshot so a screen reader is not repeatedly interrupted. Pane transitions in the TUI are instantaneous state swaps owned by **E**; the task pane never animates a checkbox and never plays a sound on completion.

### 3.6 Error states

Every row below writes zero note bytes unless the "Data-loss risk" column says otherwise. Presentation is inline for a per-record condition and a banner for a whole-command condition; a modal exists only in the TUI and only for the two confirmations (`replace`, `promote`), because those are the two irreversible-feeling actions and a modal is the only presentation that reliably stops a reflex keypress.

| Code | Trigger | Presentation and recovery | Data-loss risk |
|---|---|---|---|
| `timezone_unconfigured` | No `periodic.timezone` and no `--timezone` | Banner naming the host zone as a suggestion and the `config set` command; nothing created | No |
| `invalid_period_format` | Format token unknown, `YYYY` paired with `WW`, or a rendered component that is empty, `.`, `..`, or dot-leading | Reported at `config set` time with the offending token's offset; the setting is not persisted | No |
| `periodic_format_drift` | Current format misses, a historical format hits | Names both paths, opens the existing note, prints the `note move` command; never creates a duplicate | No |
| `periodic_range_too_large` | `ensure` range exceeds `--limit` or the 1,000 hard cap | Names the count and the bounding flags | No |
| `template_not_found` / `unsafe_path` | Template path missing, traversal, symlink escape, control directory, non-`.md` | Reuses **C**'s foundation error contract verbatim; prints the rejected operand and the rule | No |
| `template_unknown_variable` / `template_unknown_formatter` | Placeholder lexes but names something outside the allowlist | Byte offset in the template, the allowlist version, `--literal-unknown` as the alternative; nothing written | No |
| `template_capability_denied` | `plugin.*` variable without an **N** grant | Names the plugin, the capability, and the grant command; fails closed, writes nothing | No |
| `template_budget_exceeded` | Template size, placeholder count, chain length, or output size cap hit | Names the exact limit and the observed value | No |
| `template_marker_invalid` | Insert markers missing, malformed, nested, or unbalanced | Names the marker and every occurrence found; writes nothing | No |
| `collision` | `create` mode and the target exists | Shows the exact path and offers the other modes; there is no `--force` for create | No |
| `confirmation_required` | `replace`, `promote`, `sync-back`, or `ensure` under `--no-input` without `--yes` (and, for `replace`, `--expected`) | Names the exact required flags | No |
| `conflict` | Expected fingerprint differs at commit | Shows expected and actual digests and the retained proposal's recovery ID; never picks a winner | No source loss; proposal recoverable |
| `task_not_found` / `task_moved` | No task at `PATH:LINE`, or `task_hash` mismatch | Prints the nearest tasks with their line numbers; re-running resolves | No |
| `task_marker_unknown` | `--to STATE` names a state with no configured marker | Lists the configured marker map; never guesses a character | No |
| `already_promoted` | Task line already carries a `calr::` field | Prints the existing reference and `--again` | No |
| `calr_unavailable` | `mg-calr` binary not found | Names the lookup order and the config key; nothing runs, nothing is written | No |
| `calr_contract_incompatible` | `mg-calr --version` major is unrecognized | Names both versions; refuses rather than guessing a schema | No |
| `calr_error` | `mg-calr` returned `ok:false` | Surfaces `mg-calr`'s own code, message, and exit status verbatim under a labelled block | No |
| `calr_timeout` | Subprocess exceeded the deadline | Kills the child, reports that the todo's creation state is **unknown**, prints the `mg-calr todo list` command to check | No note change; unknown remote state stated plainly |
| `promotion_recorded_but_not_linked` | `mg-calr` succeeded, note write failed | Prints the created short ID and UUID and the `task link-calr` command; exits nonzero; receipt retained; the todo is not deleted | No; cross-product state is reported, never silently reconciled |
| `index_stale` / `index_unavailable` / `requires_index` | `task list` when freshness is not `current`, or a projection-dependent filter with the service down | Zero rows, exit 6, the state word, generations, refresh command; `--allow-stale` labels every row and both banners | No |
| `cancelled` / `deadline_exceeded` | `Ctrl-C`, `Esc`, or `--deadline-ms` | `complete: false`; partial pages are labelled and cannot be paginated further; a mutation cancelled before commit writes nothing | No |
| `io` / `permission_denied` / `out_of_space` | Filesystem failure | States uncommitted or unknown transaction status and the next `doctor` command; never a false success | No false success; **A**'s journal decides recovery |

Error JSON reuses **C**'s envelope: `version`, `ok:false`, `command`, `error.code`, `error.message`, `error.details`, `retryable`, and `recovery` when safe. Details may carry vault-relative paths, line and byte ranges, template offsets, allowlist versions, state words, `mg-calr` codes, and short IDs. They never carry note bodies, template bodies, `prompt.*` values, environment values, absolute paths outside the vault, or `mg-calr` connection strings.

### 3.7 Accessibility

- **Every interactive element has a text label, hint, and role.** In the CLI these are the literal field labels of §3.3. In the TUI, each task row announces role and state as words — "row 3 of 12, task, todo, Draft the Q3 retro, due 2026-09-01, not promoted, journal/2026/2026-08-29.md line 12" — and the pane announces "task pane, 12 tasks, filter state:todo, index current". The template picker announces "template Daily.md, 12 placeholders, all resolvable".
- **Custom actions for complex interactions.** The task pane exposes named actions rather than gestures: `toggle state`, `cycle marker`, `set state to…`, `open source`, `promote to mg-calr`, `copy path and line`, `filter by state`. Each is a keybinding, appears in the pane's `?` help, and has an exact CLI equivalent.
- **Color-independent state.** `todo`, `done`, `cancelled`, `in-progress`, `forwarded`, `other`, `promoted`, `not promoted`, `created`, `opened`, `stale`, `denied`, and `verbatim` are always spelled out as words next to any glyph. Checkbox rendering falls back from `☐`/`☑` to `[ ]`/`[x]` under `--ascii` or a terminal without the relevant Unicode support, and the state word is present in both tiers. A completed task is never conveyed by strikethrough or colour alone. `--no-color` and `NO_COLOR` remove styling without removing meaning.
- **The task list is the canonical representation.** There is no picture-only calendar view, no heat map that is the sole carrier of a datum, and no visual-only progress ring; a period's completion count is a printed fraction. Whatever a future TUI calendar grid draws must be a rendering of the same `periodic list` records, and set-equality between the two is asserted the way **G** asserts it for the graph pane.
- **Text scaling / dynamic type is N/A in a terminal.** The equivalent obligation is wrapping correctness at 40/60/80/120 columns and correct display width for wide, combining, and RTL characters in task text — which routinely contains emoji metadata markers. Display width affects presentation only; byte ranges and identity stay exact. Control characters, bidi overrides, and escape sequences in task text, template values, paths, and `mg-calr` output are escaped as `\u{…}` before display, so a task line can never inject a terminal escape.
- **Focus order** is the documented reading order of §3.3: banner, records in deterministic order, footer. In the TUI, focus enters the task list; the template picker's focus enters the list, not the preview region.
- **No timed prompts.** No confirmation expires, no toast disappears, and no promotion proceeds because a prompt timed out. `Ctrl-C` at any prompt exits 130 before any transaction opens.

---

## 4. Implementation Specification

### 4.1 Architecture placement

- **`crates/mg-vault-template/` — new crate, deliberately capability-free.** Contains the placeholder lexer, the closed grammar parser, the variable/formatter allowlists, and the one-pass evaluator. Its `Cargo.toml` depends on nothing that can touch the outside world: no `std::fs`, `std::env`, `std::process`, `std::net`, or `std::time` use anywhere in the crate, enforced by `#![deny(clippy::disallowed_types, clippy::disallowed_methods)]` with a `clippy.toml` disallow list, and by the dependency-graph test of §5.1. The workspace already sets `unsafe_code = "forbid"`. Its entire input is a `TemplateContext` of pre-computed strings. Making this a separate crate is the point: "the template engine cannot read your files" becomes a fact about a dependency graph rather than a promise in prose.
- **`crates/mg-vault-core/src/periodic/`** — `calendar.rs` (pure civil-date and period arithmetic, week/quarter rules), `resolve.rs` (instant → zone → day-start → civil period → path, and the path-component validation of §3.2 step 7), `settings.rs` (the `.mg-vault/settings.json` schema for this feature, versioned, unknown-newer-version fails closed). No template knowledge, no CLI knowledge.
- **`crates/mg-vault-core/src/tasks.rs`** — the task span model: locate task items, marker spans, text runs, and metadata data with dialects and byte ranges; compute `task_hash`; build the toggle/stamp span set. Built over **F**'s token stream once **F** lands; until then, over a bounded line scanner that recognizes exactly the surface of §3.2 and classifies everything else as opaque. No mutation lives here — it emits spans that **A** applies.
- **`crates/mg-vault-cli/src/commands/{periodic,template,task}.rs`** — argument parsing, prompts, plan rendering, and translation into core calls, following **C**'s planner/commit split and envelope.
- **`crates/mg-vault-cli/src/integrations/calr.rs`** — the **only** module in the workspace permitted to spawn a subprocess, and the only one that knows `mg-calr` exists. It builds an argv vector, spawns with `std::process::Command`, never through a shell, parses `mg-calr`'s `version: 1` envelope, and returns a typed result. It has no database client, no async runtime, and no socket.
- **TUI panes** live in **E**'s crate and consume J through core and the CLI's contract types; they link neither `rusqlite` nor the integration module's process code.
- **Hard rule mirroring B/G:** `mg-vault-template` has no write API of any kind, and `tasks.rs` has no write API — every byte that reaches disk goes through **A**'s `create_note` / `edit_note_span` / `write_note` with a fingerprint precondition and an atomic replace.

### 4.2 Data model

```rust
/// One of the five period kinds. Closed enum; unknown values fail closed.
pub enum PeriodKind { Daily, Weekly, Monthly, Quarterly, Yearly }

/// A resolved calendar period. Civil, zone-independent, and comparable.
/// `start` is inclusive, `end` exclusive, both civil dates.
pub struct Period {
    pub kind: PeriodKind,
    pub start: CivilDate,
    pub end: CivilDate,
    /// ISO week-year, calendar year, or fiscal year, per `kind` and settings.
    pub label_year: i16,
    /// Week number, month number, or quarter number; `None` for daily/yearly.
    pub ordinal: Option<u8>,
}

/// How an instant became a civil date. Recorded so `explain` can prove it.
pub struct PeriodDerivation {
    pub instant: Timestamp,
    pub zone: TimeZoneId,          // explicit IANA id; never read from $TZ
    pub zone_source: ZoneSource,   // Settings | Flag
    pub wall_clock: CivilDateTime,
    pub day_start: CivilTime,
    pub dst_rule: DstRule,         // None | GapForward { from, to } | FoldEarlier { at }
    pub period: Period,
}

/// The complete, deterministic outcome of resolving a period to a file.
pub struct PeriodTarget {
    pub derivation: PeriodDerivation,
    pub folder_template: String,
    pub format_template: String,
    pub path: NotePath,            // vault-relative, validated component by component
    pub template: Option<NotePath>,
}

/// Versioned, portable, vault-scoped settings; lives in `.mg-vault/settings.json`.
/// An unknown newer `version` fails closed with a doctor finding and is never rewritten.
pub struct PeriodicSettings {
    pub version: u8,
    pub timezone: Option<TimeZoneId>,
    pub day_start: CivilTime,                       // default 00:00
    pub week: WeekSettings,                         // start = Mon, numbering = Iso
    pub quarter: QuarterSettings,                   // fiscal_year_start_month = 1
    pub kinds: BTreeMap<PeriodKind, KindSettings>,  // enabled, folder, format, template
    /// Previously used (folder, format) pairs per kind, so a format change is
    /// detected as drift instead of silently creating a duplicate note.
    pub format_history: BTreeMap<PeriodKind, Vec<FormatRecord>>,
}

/// Everything the template engine may see. Assembled by the caller from already
/// computed strings. There is no handle, path, socket, or environment in here,
/// and no field is lazily evaluated.
pub struct TemplateContext {
    pub vars: BTreeMap<VariableName, String>,
    /// Values supplied by `--var NAME=VALUE` or an interactive prompt. Never
    /// sourced from the environment, a file, a command, or the clipboard.
    pub prompts: BTreeMap<String, String>,
    /// Values contributed by an N-sandboxed plugin that holds a granted
    /// `template-variable` capability for this vault. Attributed in every preview.
    pub plugin_vars: BTreeMap<QualifiedName, PluginValue>,
    pub allowlist_version: &'static str,   // "template-vars-v1"
}

/// The closed placeholder grammar's parse result. There is no expression node,
/// no call node, no block node, and no include node — by construction.
pub struct Placeholder {
    pub byte_range: Range<usize>,
    pub variable: VariableRef,             // Builtin(VariableName) | Plugin(QualifiedName)
    pub formatters: ArrayVec<Formatter, 4>,
}

/// Every formatter is a pure, total, bounded String-or-Date to String function.
pub enum Formatter {
    Date { tokens: Vec<DateToken> },       // closed token allowlist
    Offset { days: i32 },                  // |days| <= 3660, parsed from ±NdNwNmNy
    Upper, Lower, Slug,
    Pad { width: u8 },                     // width <= 16
    Default { text: String },              // text <= 256 bytes
}

/// Rendering never mutates anything; it returns bytes and an audit trail.
pub struct RenderedTemplate {
    pub bytes: Vec<u8>,
    pub resolutions: Vec<PlaceholderResolution>,  // variable, value, provenance
    pub verbatim_spans: Vec<Range<usize>>,        // `{{…}}` runs copied unevaluated
    pub template_digest: [u8; 32],
}

/// One Markdown task, addressed by location. There is no injected identifier.
pub struct Task {
    pub path: NotePath,
    pub line: u32,
    pub column: u32,
    /// The exact one-byte marker span, the only bytes a toggle replaces.
    pub marker_span: Range<usize>,
    pub marker: char,                 // preserved verbatim, including unknown chars
    pub state: TaskState,             // Todo | Done | Cancelled | InProgress | Forwarded | Other
    pub text_span: Range<usize>,
    pub metadata: Vec<TaskDatum>,
    /// SHA-256 of the normalized text run. A staleness check, never identity.
    pub task_hash: [u8; 32],
    /// A visible `^blockid`, if the user wrote one. Never minted by mg-vault.
    pub block_id: Option<String>,
    /// A visible `calr::` inline field, if the task was promoted.
    pub calr_ref: Option<CalrRef>,
}

/// One metadata datum, with the dialect it was written in, so a write-back
/// never converts one convention into another.
pub struct TaskDatum {
    pub key: TaskField,               // Due | Scheduled | Start | Created | Done | Priority | Repeat | Tag | Custom
    pub value: String,
    pub byte_range: Range<usize>,
    pub dialect: MetadataDialect,     // Dataview | Emoji | Markdown
}

/// A visible, deletable foreign reference to an mg-calr record. Never mg-vault
/// identity: mg-vault resolves notes only by path, and deleting this breaks nothing.
pub struct CalrRef {
    pub short_id: String,
    pub uuid: Option<Uuid>,           // only when `--record-uuid` was passed
    pub byte_range: Range<usize>,
    pub dialect: MetadataDialect,
}
```

**Migrations.** There is no note database migration and no note content migration. `.mg-vault/settings.json` gains this feature's keys at schema `version: 1`; an older binary meeting a newer version fails closed with a doctor finding rather than rewriting it. `.mg-vault/state/calr-receipts.jsonl` is new, disposable, append-only, size-capped, and rotated; deleting it loses no note content and no identity — only the `doctor --check calr-links` cross-reference. **B**'s projection gains task rows (`tasks(source_id, line, marker, state, text_span, task_hash)` and `task_metadata(task_id, key, value, dialect, byte_range)`) under **B**'s existing rules: disposable, rebuildable from unchanged source, published atomically per generation, never authority.

### 4.3 API contracts

**CLI signatures.** Each returns **C**'s `version: 1` envelope under `--json`, honours `--no-input`/`--no-color`/`NO_COLOR`, and uses **C**'s planner/commit split so that `--dry-run` emits exactly the plan schema that commit accepts.

```text
mg-vault periodic open KIND [--date D|--datetime T|--offset ±N] [--timezone Z]
                            [--template PATH|--no-template] [--print-path]     -> PeriodOpenReceipt
mg-vault periodic path KIND [--date D] [--offset ±N]                           -> PeriodTarget (no write)
mg-vault periodic explain KIND [--date D|--datetime T]                         -> PeriodDerivation
mg-vault periodic list KIND --from D --to D                                    -> PeriodInventory
mg-vault periodic ensure KIND --from D --to D [--limit N] [--dry-run] [--yes]  -> EnsureReport
mg-vault periodic config show | set KEY VALUE | import-obsidian [--dry-run]    -> PeriodicSettings
mg-vault template list | validate PATH                                         -> TemplateInventory
mg-vault template render PATH [--var K=V]… [--period KIND --date D]            -> RenderedTemplate
mg-vault template apply PATH --to TARGET --mode create|append|prepend|
                            insert-at-marker|replace [--marker NAME]
                            [--expected FP] [--dry-run] [--yes]                -> ApplyReceipt
mg-vault task list [--state S] [--due-before D] [--tag T] [--path-under P]
                   [--period KIND] [--promoted|--unpromoted] [--source direct] -> TaskPage
mg-vault task show PATH:LINE                                                   -> Task
mg-vault task toggle PATH:LINE [--to STATE|--cycle] [--stamp] [--recur]
                     [--task-hash H] [--expected FP] [--dry-run]               -> ToggleReceipt
mg-vault task promote PATH:LINE [mg-calr passthrough flags] [--record-uuid]
                      [--no-backlink] [--again] [--dry-run] [--yes]            -> PromotionReceipt
mg-vault task link-calr PATH:LINE --id SHORT_ID [--uuid U] [--expected FP]     -> ToggleReceipt
mg-vault task calr-status PATH:LINE                                            -> CalrStatus (read-only)
mg-vault task sync-back PATH:LINE [--dry-run] [--yes]                          -> ToggleReceipt
```

**Exit codes** follow **C**: `0` success; `1` generic failure; `2` usage; `5` confinement unavailable; `6` index not `current` (task queries only); `64` `input_required`/`confirmation_required`; `65` invalid value or refused plan; `66` not found or ambiguous; `70` `calr_error`/`calr_contract_incompatible`; `75` `calr_timeout` (retryable, remote state unknown); `130` interrupt.

**Date-token allowlist (normative, versioned `date-tokens-v1`).** Closed set, fixed English name table, no locale input:

| Token | Meaning | Token | Meaning |
|---|---|---|---|
| `YYYY` / `YY` | calendar year | `GGGG` / `GG` | ISO week-year |
| `MM` / `M` | month number | `MMM` / `MMMM` | English month name |
| `DD` / `D` | day of month | `DDD` | day of year |
| `WW` / `W` | week number | `Q` | quarter number |
| `ddd` / `dddd` | English weekday name | `HH` / `mm` / `ss` | wall clock (datetime contexts only) |
| `[literal]` | bracketed literal text | | |

`GGGG` must pair with `WW`/`W`, and `YYYY` must not — the pairing is validated at `config set` time because `2026-01-01` falls in `2025-W53`, and a vault that gets this wrong produces one wrong filename per year and notices in December. No token can emit `/`, `\`, NUL, or a control byte, and the rendered result is still validated component by component per §3.2 step 7.

**Placeholder grammar (normative, versioned `template-vars-v1`).**

```text
placeholder := "{{" ws? var ( ws? "|" ws? formatter )* ws? "}}"
var         := ident ( "." ident )*                 # must match the allowlist exactly
formatter   := ident ( ":" quoted_arg )?            # must match the allowlist exactly
ident       := [A-Za-z_] [A-Za-z0-9_]*
quoted_arg  := '"' ( [^"\\] | "\\" ["\\] )* '"'     # a literal string, never an expression
ws          := ( " " | "\t" )*
```

There is no rule for a call, an operator, an index, a block, a conditional, a loop, a comment, an include, an assignment, or a nested placeholder. A `{{ … }}` run that does not match this grammar is copied verbatim and never evaluated. The grammar is closed by construction, which is what makes the language non-Turing-complete: the evaluator is a finite fold over a finite token list with no back-edge.

**Variable allowlist (closed).** `date`, `time`, `datetime`, `period.kind`, `period.start`, `period.end`, `period.index`, `period.label_year`, `period.prev.date`, `period.next.date`, `period.prev.path`, `period.next.path`, `note.path`, `note.folder`, `note.basename`, `note.title`, `vault.name`, `prompt.<NAME>`, `plugin.<id>.<name>`.

**Explicitly absent, permanently.** Environment variables of any kind; `$HOME`, username, hostname, machine ID; the contents of any file, including other notes; directory listings; the clipboard; the system's random source; a UUID generator (also forbidden by **1C**); process, network, or shell access; a "run this command" or "evaluate this expression" escape. There is no configuration key, feature flag, or build option that adds any of them. A future need for one is a plugin under branch **N**, sandboxed and granted.

**Formatter allowlist (closed).** `date:"FORMAT"` (date-token allowlist above), `offset:"±NdNwNmNy"` (bounded to ±3660 days, calendar-correct month/year arithmetic), `upper`, `lower`, `slug` (**C**'s versioned slug algorithm), `pad:"N"` (N ≤ 16), `default:"TEXT"` (TEXT ≤ 256 bytes). Chains are left-to-right, at most 4 long, and every composition is type-checked at parse time — `upper | offset:"+1d"` is `template_type_mismatch` before anything runs.

**`mg-calr` contract (consumed, not defined).** J speaks exactly the surface `mg-calr`'s `c-todo-core` spec publishes and nothing else:

```text
mg-calr --version                                    -> version string; major must be known
mg-calr todo add --title T [--due D] [--timezone Z] [--priority P] [--project X]
                 [--tag T]… [--notes N] [--remind-before D | --no-reminder]
                 --json --no-input                   -> {"version":1,"ok":true,"data":{...}}
mg-calr todo show SELECTOR --json --no-input         -> read-only status pull
```

J reads `data.short_id`, `data.id`, `data.title`, `data.due`, `data.completed`, and `data.blocked`, ignores unknown additive fields after validation, and fails closed on a missing required field, an unknown enum variant, or an unknown envelope version. It never invokes `mg-calr interop import-todo`, `mg-calr database …`, or any other subcommand; it never writes `mg-calr`'s projection file; and it opens no PostgreSQL connection, socket, or config of `mg-calr`'s. Auth is the OS process boundary: `mg-calr` runs as the same unprivileged user with its own peer-authenticated database, exactly as it would from a shell. There is no pagination in this contract and no rate limiting beyond the per-invocation deadline, because every promotion is one explicit user action.

### 4.4 State management

- **Vault-scoped, portable, durable:** `.mg-vault/settings.json` owns periodic configuration, marker maps, template folder, write dialect, and `format_history`. It travels with the vault, is isolated from `.obsidian` (criterion **1D**), and is written through **A**'s atomic replace with a version precondition.
- **Vault-scoped, disposable:** `.mg-vault/state/calr-receipts.jsonl`. Append-only, size-capped at 8 MiB with rotation to one `.1` file, owner-only mode. Deleting it loses no note content, no identity, and no user data — only the `doctor --check calr-links` cross-reference.
- **Per-user, outside the vault:** nothing new. Template rendering keeps no cache; a rendered template is recomputed from the template file and the context every time, which is cheap and removes a whole class of staleness bug.
- **In-process only:** the resolved `TemplateContext`, the parsed template, and the `mg-calr --version` probe result. `prompt.*` values are held in memory for the duration of one command, are never written to disk, never logged, never echoed into an error, and are zeroed on drop.
- **Owned by other branches:** task *rows* are **B**'s disposable projection (never authority, never presented as note content); TUI pane and cursor state is **E**'s; buffer state and undo are **D**'s; the atomic transaction, journal, and trash are **A**'s.
- **Local vs. server-synced:** everything here is local. There is no server, no account, and no sync. `mg-calr`'s database is a *different local application's* state, reached only through its CLI, and is never mirrored, cached, or reconciled by `mg-vault`.
- **Offline / draft persistence:** a template render aborted before commit leaves nothing behind. A `template apply` whose commit fails on a fingerprint conflict retains the proposed bytes in **C**'s conflict spool with a recovery ID and reports both digests; it never picks a winner. A promotion interrupted between the `mg-calr` call and the note write leaves a receipt line, which is exactly the durable evidence `doctor --check calr-links` needs.

### 4.5 Dependencies

- **`jiff`** (MIT OR Unlicense) for IANA timezone handling, civil date/time types, calendar arithmetic, and explicit DST gap/fold disambiguation. It is chosen over `chrono` + `chrono-tz` because its `Zoned`/`civil::Date` split and its named ambiguity strategies express §3.2's gap/fold policy directly rather than by convention. It reads the system tzdb at `/usr/share/zoneinfo` by default, with a `bundled-tzdb` cargo feature for static builds. **It is a dependency of `mg-vault-core`, not of `mg-vault-template`** — the template engine receives formatted strings and never touches a clock.
- **`unicode-segmentation`** and **`unicode-width`** (MIT/Apache-2.0) for grapheme-safe task text handling and terminal layout. Both are already required by **F**/**G**.
- **No new dependency for the template engine.** It uses `core`/`alloc` plus `arrayvec`-style bounded containers; adding any I/O-capable crate to that manifest is a review-blocking change and is caught by the §5.1 dependency test.
- **No PostgreSQL client, no async runtime, no TLS stack, no HTTP client** anywhere in this feature. Promotion uses `std::process::Command` only.
- **No new assets**, fonts, images, models, or data files. The English month/weekday name table is a compiled-in constant, not a locale database.
- **Infrastructure:** none. No database, no CDN, no third-party service, no network access of any kind. `mg-calr` is an optional *runtime* peer discovered on `PATH`, never a build dependency and never a hard requirement; every other command in this feature works with it absent.
- All additions are subject to the same license, SBOM, and version-pinning review **A**/**B**'s dependencies are.

### 4.6 Platform-specific considerations

- **tzdb availability.** Arch and Ubuntu ship `/usr/share/zoneinfo`; musl or minimal container images may not. The `bundled-tzdb` feature is off by default and on for release artifacts that target such environments. A missing tzdb is a startup-time `timezone_database_unavailable` error naming the feature flag — never a silent fall back to UTC, which would silently shift every daily note.
- **Case-insensitive and Unicode-normalizing filesystems.** A period path is generated, not typed, so case collisions are rare — but a vault synced from macOS may hold `2026-08-29.md` under an NFD-normalized parent. Path identity stays exact bytes per **A**/**G**; `periodic open` that misses while a normalization near-miss exists reports `periodic_format_drift` with the near-miss named rather than creating a second file.
- **Clock changes and suspend.** `now()` is sampled once per command; a clock step mid-command cannot make one invocation resolve two different periods. A machine resuming from suspend across midnight resolves the new day on the next invocation, with no cached "today".
- **Terminal capability** affects checkbox and progress rendering only, never which records exist: no Unicode support, `--ascii`, or a width below 60 columns drops to `[ ]`/`[x]` and one-record-per-block.
- **`mg-calr` version skew** is handled by the major-version probe of §3.2; the two products ship independently and neither blocks the other's release.
- **Feature flags.** `periodic-ensure`, `task-promotion`, and `template-plugin-variables` may ship gated for gradual rollout. The idempotency of `periodic open`, the closed template grammar, the capability-free engine, span-local toggling, and the confirmation requirements are **never** feature-gated and have no override.
- **Renderer or framework migration:** none. This feature adds no rendering engine; the TUI panes are plain terminal drawing inside **E**'s existing pane system.

### 4.7 Performance budget

These are acceptance budgets on recorded hardware, not measured claims. The corpus is **B**'s: 100,000 notes, 1,000,000 blocks, and a task fixture of 250,000 tasks.

- **`periodic open` is index-free and must stay that way.** Cold p95 ≤ 30 ms, warm p95 ≤ 12 ms, for the whole path: settings read, tzdb lookup, calendar arithmetic, one `stat`, and — on a miss — one template read plus one `O_EXCL` create with parent `fsync`. It performs no index query, so it works identically with the service stopped and its latency does not grow with vault size.
- **`periodic path` / `explain`** perform no filesystem write and at most one `stat`: p95 ≤ 5 ms.
- **`periodic ensure`** over 90 periods: p95 ≤ 1.5 s, one independent transaction each, cancellable between transactions, bounded memory.
- **Template rendering** of a 64 KiB template with 500 placeholders: p95 ≤ 5 ms, allocation bounded by the output cap. Hard caps, each a typed refusal rather than a slow path: template ≤ 1 MiB, placeholders ≤ 4,096, formatter chain ≤ 4, `offset` ≤ ±3,660 days, `pad` ≤ 16, `default` ≤ 256 bytes, rendered output ≤ 4 MiB. Because the evaluator is one pass with no re-scan and no inclusion, output size is bounded by `template_size + Σ placeholder_outputs` — an expansion bomb is arithmetically impossible, and the caps are a second, independent guard.
- **Task parse** of one 1 MiB note: p95 ≤ 20 ms. **Task toggle** end to end (read, locate, one-or-two span edit, atomic replace, `fsync`) on a note ≤ 1 MiB: p95 ≤ 15 ms, p99 ≤ 40 ms. Toggle cost is O(note size), never O(vault).
- **Task queries** inherit **G**'s budgets: warm first page p95 ≤ 100 ms, cold p95 ≤ 750 ms, deterministic ordering, cancellation within p95 ≤ 100 ms of the signal. The `--source direct` fallback is bounded by an explicit subtree and a node cap and reports its estimated cost before running.
- **Promotion** overhead attributable to `mg-vault` — argv construction, preview, envelope parse, span edit — is p95 ≤ 20 ms; total wall time is dominated by the `mg-calr` subprocess and bounded by the 15 s default deadline. Exactly one subprocess per promotion; no batch mode in this slice, so no fan-out of processes.
- **Memory.** Template engine peak ≤ 8 MiB above baseline for a maximal template. Task projection adds ≤ 64 bytes per task and ≤ 48 bytes per metadata datum to **B**'s index, inside **B**'s ≤ 2.5× total index budget. No structure in this feature is loaded eagerly at process start.
- **Storage.** `.mg-vault/settings.json` ≤ 16 KiB. The receipt log is capped at 8 MiB with one rotation. Task rows target ≤ 0.15× the existing projection size.
- **Startup.** Nothing here runs unless a `periodic`, `template`, or `task` command is invoked; the tzdb is loaded lazily on first period resolution (system tzdb ≤ 5 ms, bundled ≤ 15 ms). **C**'s process-start budget is unaffected.
- **Network:** zero bytes. Every workflow in this feature — including promotion, which is a local subprocess — is fully offline.

---

## 5. Test Specification

### 5.1 Unit tests

| Test | Setup and assertion | Edge covered |
|---|---|---|
| `daily_is_idempotent_under_repetition` | Resolve and open 100 times for one instant; assert one file, one `created:true`, 99 `created:false`, byte-identical after the first | Idempotency; no double create |
| `open_existing_never_applies_template` | Existing daily note with distinctive bytes and a configured template; open; assert byte-identical and `template: null` in the receipt | Unconfirmed overwrite (auto-fail) |
| `spring_forward_yields_one_note` | Zone `America/Los_Angeles`, 2026-03-08, `day_start` 00:00 and 04:00; assert exactly one period, `dst_rule` recorded, 23-hour day | DST gap |
| `fall_back_boundary_takes_earlier` | 2026-11-01 fold with `day_start` 01:30; assert `FoldEarlier`, one note, 25-hour day | DST fold |
| `day_start_gap_resolves_forward` | `day_start` inside a nonexistent wall-clock hour; assert `GapForward` with both wall times reported | Gap on the boundary itself |
| `iso_week_year_boundary` | 2026-01-01 weekly; assert `2025-W53`, and assert `YYYY`+`WW` is refused at config time | Week-year pairing |
| `fiscal_quarter_offset` | `fiscal_year_start_month = 4`; assert 2026-05-02 is `Q1` with the fiscal label year | Quarter arithmetic |
| `format_cannot_escape_the_vault` | Formats yielding `../x`, `/abs`, `.obsidian/x`, `.mg-vault/x`, empty and `.`-leading components | 4A confinement; control directories |
| `locale_does_not_affect_filenames` | Render `MMMM`/`dddd` under three `LC_*` settings; assert byte-identical | Determinism |
| `template_grammar_rejects_everything_foreign` | `{{#each}}`, `{{ tp.date.now() }}`, `{{{x}}}`, `<% … %>`, `${HOME}`, `$(id)`, `{{a+b}}`, `{{a==b}}` | Verbatim passthrough; no evaluation |
| `template_engine_has_no_io_capability` | Resolve `mg-vault-template`'s dependency graph and scan its source; assert zero references to `std::fs`, `std::env`, `std::process`, `std::net`, and no I/O-capable transitive dependency | **4C** capability bypass (auto-fail) |
| `template_output_is_never_rescanned` | A `prompt` value containing `{{date}}`; assert it is emitted literally | No recursion; no injection |
| `unknown_variable_fails_closed` | `{{nope}}`; assert typed error, zero bytes written, and that `--literal-unknown` copies it verbatim | No silent empty expansion |
| `plugin_variable_denied_without_grant` | `{{plugin.x.y}}` with no **N** grant; assert `template_capability_denied` and no write | Deny-by-default |
| `formatter_chain_is_type_checked` | `upper \| offset:"+1d"`; assert parse-time refusal | No runtime coercion |
| `template_budgets_are_enforced` | Oversized template, 5,000 placeholders, chain of 5, `offset` of 9,999d, 5 MiB output | Bounded evaluation |
| `apply_modes_preserve_all_other_bytes` | Append, prepend, and marker-insert into a note with frontmatter, a table, Templater syntax, CRLF, and trailing whitespace; assert output is exactly `prefix ‖ inserted ‖ suffix` | **1B**; source-content loss (auto-fail) |
| `replace_requires_confirmation_and_trash` | `--mode replace` with: nothing, `--yes` alone, wrong `--expected`, correct both | Unconfirmed overwrite (auto-fail); recoverability |
| `toggle_replaces_exactly_one_byte` | Toggle in a note with an aligned table two lines below; assert the diff has exactly one single-character hunk | **1B** span-local |
| `unknown_markers_are_preserved` | `[/]`, `[-]`, `[>]`, `[?]`, `[€]`; assert character preserved, state mapped or `other`, `[X]` never normalized | Unknown syntax loss (auto-fail) |
| `metadata_dialect_is_never_converted` | Toggle-with-stamp on emoji, Dataview, and mixed lines; assert each write matches the line's existing dialect | Dialect preservation |
| `toggle_and_stamp_are_one_write` | Instrument the atomic layer; assert exactly one `replace_atomic` call | Non-atomic save (auto-fail) |
| `task_hash_detects_movement` | Edit the note behind a captured `task_hash`; assert `task_moved` and zero writes | Stale-write refusal |
| `no_identifier_is_ever_injected` | Toggle, stamp, recur, and promote across a fixture; assert no UUID, no zero-width character, no HTML comment, no frontmatter key was added | **1C** |
| `promotion_argv_is_a_vector` | Task text containing `; rm -rf ~`, `$(id)`, backticks, newlines, and a NUL; assert argv elements are exact and no shell is invoked | Command injection |
| `promotion_module_has_no_database_client` | Dependency-graph assertion over `integrations::calr`: no PostgreSQL, TLS, async-runtime, or socket crate reachable | Cross-product boundary |
| `already_promoted_refuses` | Promote twice; assert `already_promoted` and that `--again` is required | Duplicate todos |

### 5.2 Integration tests

- **Concurrent open race:** 100 processes invoke `periodic open daily` for the same instant against one vault; assert exactly one file, one create receipt, 99 open receipts (some with `race_detected: true`), and zero post-create byte changes.
- **Timezone travel:** run the suite with the host zone changed between invocations while `periodic.timezone` stays fixed; assert every resolution is unchanged, and assert that removing the setting yields `timezone_unconfigured` rather than a host-derived guess.
- **DST matrix:** for `America/Los_Angeles`, `Europe/Berlin`, `Australia/Lord_Howe` (30-minute offset), and `Asia/Kolkata`, across gap and fold days at `day_start` values `00:00`, `01:30`, `04:00`: assert one note per calendar day, the correct `dst_rule`, and correct period start/end for weekly and monthly containers.
- **Template safety corpus (capability-bypass gate):** render a corpus containing Templater, Handlebars, Jinja, ERB, shell substitution, a path-traversal template path, a symlinked template, a 4 MiB expansion attempt, and a `{{plugin.*}}` without a grant, inside a sandbox that fails the test if the process opens any file outside the vault, spawns any child, reads any environment variable, or opens any socket. Assert verbatim passthrough or a typed refusal for every case and zero sandbox violations.
- **Obsidian coexistence:** a vault with `.obsidian/daily-notes.json`, `.obsidian/templates.json`, Templater templates, Tasks-plugin emoji, and Dataview inline fields. Assert `.obsidian` is never written; `periodic config import-obsidian` reads it, previews the adoption, and writes only to `.mg-vault/settings.json`; every touched note round-trips byte-identically outside edited spans; and Obsidian still opens the vault afterwards.
- **Rebuild equivalence:** index the task fixture, snapshot every source digest, delete the database, rebuild; assert byte-for-byte source non-mutation and identical task rows, states, ordering, and counts (**1A** + **3A**).
- **Freshness fail-closed:** edit a note behind the index; assert `task list` returns zero rows and exit 6 with the state named, that `--allow-stale` labels every row and both banners, and that `task toggle PATH:LINE` still succeeds because it reads the file directly.
- **Promotion, three worlds:** (a) `mg-calr` absent — assert `calr_unavailable`, zero bytes changed, `doctor` reports not detected; (b) `mg-calr` present, PostgreSQL unprovisioned — assert `mg-calr`'s own typed error is surfaced verbatim and zero bytes changed; (c) `mg-calr` healthy against a disposable `mg_calr_test` database — assert one todo created, exactly one visible inline field appended, the rest of the file byte-identical, and a re-run refused as `already_promoted`.
- **Promotion partial failure:** make the note read-only after the `mg-calr` call succeeds; assert `promotion_recorded_but_not_linked`, a nonzero exit, the printed short ID and UUID, a retained receipt, that the `mg-calr` todo still exists, and that `task link-calr` then completes the link.
- **No daemon, no reverse writes:** run the full suite under a process and filesystem monitor; assert `mg-vault` starts no long-lived process, opens no socket, registers no timer, and never writes any path under `mg-calr`'s XDG data directory. Complete a todo in `mg-calr` and assert the Markdown file is unchanged until `sync-back` is explicitly and interactively run.
- **Crash and recovery:** kill the process between the `mg-calr` call and the note write, between the trash capture and the `replace` write, and mid-`ensure`; assert **A**'s journal recovery is idempotent, that recovery never writes to a path whose bytes differ from the recorded precondition, and that a re-run of `ensure` creates only the still-missing notes.
- **Scale:** 250,000 tasks across 100,000 notes; assert every §4.7 budget with p50/p95/p99, peak RSS, index growth, cancellation latency, and unaffected direct-write latency during a rebuild.

### 5.3 UI / E2E tests

CLI E2E for every command in human, `--json`, and `--jsonl` modes with golden fixtures covering: happy path; existing-note open; `timezone_unconfigured`; `periodic_format_drift`; every template refusal code; each apply mode; `replace` refused three ways and then accepted; toggle in every marker state including unknown characters; `task_moved`; empty task list; stale index with and without `--allow-stale`; each of the three promotion worlds; `calr_timeout`; and `promotion_recorded_but_not_linked`. Goldens assert envelope version, `command` name, deterministic ordering, and stable error codes without depending on volatile timestamps. A dedicated golden asserts that `--dry-run` output for `template apply` and `task promote` is byte-identical to the plan the subsequent commit accepts.

Navigation and mutation E2E: open today's note, type into it, run `periodic open` again, assert the typed bytes survive; apply a template in `append` mode and assert the prior content is a contiguous prefix; promote a task, cancel at the confirmation, assert nothing ran and nothing changed; promote again and accept.

TUI E2E (once **E** lands, gating the pane claim, not this spec): `<leader>j` opens today's note in the focused pane; `<leader>t` lists tasks; `<Space>` toggles and the buffer's undo (**D**) reverses it; `<leader>tp` shows the promotion modal, `Esc` cancels with no subprocess spawned; the template picker inserts at the cursor and nowhere else; every step is keyboard-only and every state is announced in words.

### 5.4 Visual / manual verification

- Terminal themes: default, light, dark, high-contrast, and fully ANSI-stripped; confirm every state word (`todo`, `done`, `created`, `opened`, `promoted`, `denied`, `verbatim`, `stale`) survives.
- `NO_COLOR`, `--no-color`, `--ascii`, and a non-TTY pipe; confirm identical record content in all four and that checkboxes degrade from `☐`/`☑` to `[ ]`/`[x]` with the state word still present.
- Widths 40, 60, 80, 120 with long Unicode, RTL, and emoji-metadata task text; confirm no path, byte range, template offset, `mg-calr` ID, error code, or recovery command is truncated, and that the resolution table becomes one-record-per-block below 60 columns.
- Empty vs. populated: a vault with no templates, a note with no tasks, a period range with no notes, a note with 500 tasks, a template with zero placeholders, and a template with 500.
- Freshness states for `task list`: current, stale, rebuilding, degraded, unavailable — each verified to show its banner above and below the rows, and each verified not to affect `periodic open` or `task toggle` at all.
- Screen-reader pass over a periodic receipt, a template resolution table, the task list, and the promotion confirmation.
- Terminal font-size zoom as the dynamic-type analogue; confirm reflow correctness.

---

## 6. Compliance & Safety Gate

### 6.1 Sensitive data classification

- [ ] No sensitive data involvement
- [x] **Handles sensitive data** — journal entries are among the most private content a person writes, and task text, due dates, projects, tags, and `prompt.*` values may all be personal. Protections: everything is local and offline; `prompt.*` values live only in process memory for one command, are never persisted, logged, or echoed into an error, and are zeroed on drop; the template engine cannot read a file, an environment variable, or a socket, so a malicious template cannot exfiltrate anything even if executed against a private vault; error details carry paths, ranges, codes, and versions but never note bodies or template bodies; the argv passed to `mg-calr` is displayed to the user before it is used and contains only fields the user named; the receipt log stores a path, a line, and a short ID, never task text; `mg-vault` never reads `mg-calr`'s database, config, or connection string.
- [x] **Uses synthetic/test data only until compliance gate clears** — every fixture vault, the DST matrix, the template safety corpus, the 250,000-task corpus, and the `mg_calr_test` database are generated; no personal vault or production `mg-calr` database is required for acceptance.

### 6.2 Asset provenance

- [x] **No third-party assets** — this feature adds no images, fonts, models, or data files. The English month and weekday names are a compiled-in constant table authored in-repo. `jiff`, `unicode-segmentation`, and `unicode-width` are code dependencies under the same license/SBOM review as **A**/**B**'s, not bundled content; `jiff`'s optional `bundled-tzdb` feature vendors the IANA time zone database, which is public domain, and is disabled by default in favour of the system `/usr/share/zoneinfo`.
- [ ] Uses third-party assets

### 6.3 Language / claims audit

- [x] Make claims not supported by evidence? **No.** Every latency, memory, and scale figure in §4.7 is labelled an acceptance budget, not a measurement. The capability-free claim about `mg-vault-template` is stated as a property enforced by a specific test over a specific dependency graph, not as an assurance.
- [x] Promise capabilities not yet built? **No.** This entire branch is absent today and §7 says so in the required state words. TUI panes are labelled **E**-gated; task queries are labelled **B**/**G**-gated; promotion is labelled `provided_by: "mg-calr", available: false` in help and capability JSON whenever the binary is not detected, rather than `available: true`.
- [x] Use language restricted by domain regulations? **No.** This is not a regulated domain. The word "sync" is deliberately avoided for the `mg-calr` relationship; the spec uses "promotion", "pull", and "one-shot sync-back", because calling a one-directional explicit handoff "sync" would imply a bidirectional guarantee that does not exist.

### 6.4 Regulatory alignment

**Lens 3 — Knowledge Retrieval and Structure** (the template's named gate):

- **3A Determinism.** A period target is a pure function of `(kind, instant-or-date, zone, day_start, week/quarter settings, folder, format, token-table version)`; the English name table removes locale as an input; DST resolution is a fixed named rule recorded in `explain`; task parsing yields byte ranges reproducible from source alone, so deleting the index and rebuilding reproduces every task row and every ordering (§5.2 rebuild equivalence).
- **3B Ambiguity.** Ambiguity is never resolved silently anywhere: `periodic_format_drift` names both candidate paths and creates nothing; a `PATH:LINE` that names no task or a moved task is a typed error with the nearest candidates listed; an ambiguous `mg-calr` selector is `mg-calr`'s own `selector_ambiguous`, surfaced verbatim with no note change; a `{{ … }}` run that is ambiguous between "placeholder" and "foreign syntax" is resolved by a closed grammar, deterministically, in favour of verbatim passthrough.
- **3C Query depth.** Task querying is expressed entirely in **G**'s grammar — `task.state`, `task.marker`, `task.text`, `task.due`, `task.scheduled`, `task.done`, `task.count` — composed with text, title, tag, property, path, relationship, and date predicates. J adds the projection those predicates read (marker, state, text span, each metadata datum with its dialect and byte range) plus `--period` and `--promoted` conveniences that desugar to documented expressions, and refuses unsupported operators rather than broadening a match.
- **3D Derived authority.** Templates are ordinary notes. Periodic notes are ordinary notes at ordinary paths. Settings are one small JSON file inside `.mg-vault`, not a database. Task rows are **B**'s disposable projection and are never authority; opening a task result re-reads the file through **A** and compares fingerprints. `mg-calr`'s database is authoritative for todos and `mg-vault` never reads or writes it; the Markdown checkbox remains authoritative for the *note*, and the visible `calr::` field is a reference between two authorities rather than a third one.
- **3E Scale.** §4.7 budgets task parse, toggle, and query at 100,000 notes and 250,000 tasks, and keeps the highest-frequency operation — `periodic open` — entirely index-free so its latency is independent of vault size and unaffected by a rebuild in progress.

**Lens 4 — Security, Reliability, and Extensibility** (this spec's security spine):

- **4A Confinement.** Template paths, period paths, and task paths all go through **A**'s validation; a generated period path is additionally validated component by component with a dot-leading component rejected, so no format string can address `.obsidian` or `.mg-vault`; symlinked templates and traversal formats are refused (§5.1).
- **4B Concurrency.** Every mutation carries an expected fingerprint and commits through **A**'s atomic replace; `periodic open` uses `O_EXCL` and converges on a lost race instead of overwriting; `task_hash` catches a task that moved behind the caller; a conflict retains both versions and names neither the winner.
- **4C Least privilege.** The template engine is deny-by-default in the strongest available sense — a separate crate with no I/O capability in its dependency graph, whose entire input is pre-computed strings, whose grammar contains no call or expression node, and whose evaluation is a single non-recursive pass. Plugin-contributed variables require an explicit **N** capability grant, are namespaced and attributed in every preview, and fail closed when ungranted. `mg-calr` is reached only by spawning a binary with an argv vector after showing the user that exact argv, never through a shell and never through its database.
- **4D Recovery.** Every destructive path is previewable (`--dry-run` emits the plan commit accepts), atomic (one `replace_atomic` per command), and trash-backed (`replace` mode captures prior bytes before writing). No command claims success before the write and its directory sync complete; `calr_timeout` reports the remote state as **unknown** rather than guessing.
- **4E Contracts.** CLI signatures, exit codes, the `version: 1` envelope, the date-token allowlist (`date-tokens-v1`), the variable allowlist (`template-vars-v1`), and the settings schema version are all explicit and versioned; unknown newer versions fail closed; the `mg-calr` contract is version-probed and refused on an unknown major rather than guessed at.
- **4F Privacy.** `prompt.*` values, note bodies, and template bodies never enter logs, errors, receipts, or the index. The receipt log holds a path, line, and short ID only. Nothing in this feature emits a network byte.

**Lenses 1, 2, and 5.** **1A** — every artifact is an ordinary file; deleting the index or the receipt log loses nothing. **1B** — every write is a span-local edit whose output is provably `prefix ‖ new ‖ suffix`, asserted against a fixture containing Templater, Dataview, CRLF, and unknown markers. **1C** — no UUID, zero-width character, or hidden marker is ever injected; `calr::` is visible, optional, deletable, and never `mg-vault` identity. **1D** — `.obsidian` is read for an explicit previewed import and never written; all app settings live under `.mg-vault`. **1E** — collisions, drift, conflicts, and partial cross-product state all fail closed and recoverably. **2A** — every workflow has a documented keybinding and an exact CLI equivalent. **2B** — atomic writes with fingerprint preconditions, **D**-owned undo over toggles, and **A**-owned journal recovery. **2E** — with the index down, `mg-calr` uninstalled, and the TUI absent, period notes, templates, and toggling all still work, and each unavailable capability says so by name. **5A** — zero network bytes. **5B** — §4.7. **5C** — the task list is the canonical representation; no datum is picture-only. **5D** — 40/60/80/120 columns, ASCII checkbox fallback, colour-independent state words, no animation. **5E** — stable versioned JSON, `--jsonl`, `--no-input`, `--no-color`, and `NO_COLOR` throughout.

**Named auto-fail rules:**

- **Unconfirmed overwrite/import** — `periodic open` applies a template only on a path that did not exist, and applies none at all on an existing note; `create` mode has no `--force`; `replace` is the sole overwrite path and requires a shown diff, a typed path or `--yes` **plus** `--expected`, a prior-bytes trash capture, and a fingerprint precondition that still holds at commit; `periodic config import-obsidian` previews and writes only to `.mg-vault`; `sync-back` is one task, previewed and confirmed, never batched or automatic.
- **Source-content loss** — every non-create write is a span-local edit through `edit_note_span`; §5.1 asserts the output is exactly `prefix ‖ inserted ‖ suffix` on a fixture carrying frontmatter, tables, CRLF, trailing whitespace, and foreign syntax; unknown task markers keep their exact character; `[X]` is never normalized; metadata dialects are never converted; toggle-plus-stamp is one atomic write, never two.
- **Capability or data-exfiltration bypass** — the template engine is a crate with no filesystem, process, environment, network, clock, or randomness capability in its dependency graph, asserted mechanically in §5.1 and behaviourally under a syscall-monitoring sandbox in §5.2; the grammar has no call or expression node and evaluation never re-scans its own output; there is no flag, config key, or build option that adds an escape; plugin variables require an explicit **N** grant and fail closed; `mg-calr` is spawned as an argv vector, never a shell string, and its database, config, connection string, and projection file are never opened.
- Additionally: **no partial multi-file mutation** — `ensure` commits one independent transaction per note and reports exactly which succeeded; promotion's cross-product partial state is reported as `promotion_recorded_but_not_linked` with a repair command and is never silently reconciled or rolled back. **No index state overriding source, and no stale index presented as current** — task rows are **B**'s disposable projection under **G**'s fail-closed freshness contract, and every opened result is re-read from disk. **No silent conflict winner** — fingerprint mismatches report both digests and retain the proposal. **No unknown-syntax loss** — foreign template dialects and unknown task markers are preserved verbatim. **No unsafe traversal or symlink escape** — §5.1's format and template path suites. **No non-atomic save claiming success** — one `replace_atomic` per command, success reported only after the sync. **No recovery overwriting newer source** — **A**'s journal quarantines rather than writes when bytes differ from the recorded precondition.

---

## 7. Gap Analysis vs. Current State

### 7.1 What exists today

**Implemented.** Only the substrate this branch would sit on. `crates/mg-vault-core/src/vault.rs` provides `create_note` (`create_new`/`O_EXCL`, collision-refusing), `read_note`, `write_note` and `edit_note_span` (both fingerprint-gated), `trash_note`/`restore_note`, and `validate_note_path`, which rejects absolute paths, `..`, non-`.md` extensions, and any mutation whose first component is `.obsidian` or `.mg-vault`. `atomic.rs` provides same-directory temp-write, `fsync`, rename, and parent sync. `frontmatter.rs`/`frontmatter_scalar.rs` provide byte-preserving frontmatter scalar location. `registry.rs`/`xdg.rs` provide vault resolution. `crates/mg-vault-cli/src/main.rs` exposes `vault`, `note create|read|write|trash|restore`, `index rebuild|status`, `search`, and `interop export` with the `version: 1` envelope, `--json`, `--no-input`, `--no-color`, and `NO_COLOR`. `crates/mg-vault-index/src/lib.rs` persists a disposable SQLite projection with atomic generations and truthful `empty|current|stale|degraded` freshness.

**Prototyped.** Nothing in this branch. The nearest adjacent prototype is `mg_vault_core::index`'s wikilink extraction, which is unrelated to tasks.

**Planned (specified, not built).** **A** specifies the journal, `.mg-vault/settings.json`, and quarantine-based recovery that this feature's settings and multi-step writes depend on. **B** specifies the watcher/service, IPC, and structural projections — including task projections — that `task list` reads. **F** specifies the token-preserving parser whose `TaskMarker` token kind and `TokenSpan` contract `tasks.rs` is meant to consume. **G** specifies the `task.*` query predicates, the fail-closed freshness contract, and the result envelope that `task list` inherits wholesale. **C** specifies the planner/commit split, `--dry-run` plan schema, exit-code categories, and `doctor`. **N** specifies the WASM sandbox and capability grant that plugin template variables require. None of these are built.

**Gated.** TUI panes are gated on **E**. Task queries are gated on **B** and **G**; until they land, `task list --source direct` (bounded, explicitly labelled) is the only listing available and must report `provided_by: "index", available: false` for projection-dependent filters. Plugin template variables are gated on **N** and must fail closed until it ships. Promotion is gated at runtime on `mg-calr`'s presence and its C1 `todo add` flag set, which is itself specified-not-built on that side — `mg-calr`'s README documents `todo add` as retained legacy CRUD during its `mg-todo` extraction, so J must probe the version and refuse on an unknown major rather than assume.

**Absent.** Everything this spec describes: the `periodic`, `template`, and `task` command families; period kinds, calendar arithmetic, timezone and day-start handling, DST policy, and `format_history`; the template crate, its grammar, both allowlists, the evaluator, and every apply mode; task parsing, marker spans, metadata dialects, `task_hash`, toggling, stamping, and recurrence; the `mg-calr` integration module, the receipt log, `link-calr`, `calr-status`, and `sync-back`; the `.mg-vault/settings.json` keys for all of the above; and every test in §5. No file in the repository mentions a period, a template, a task, or `mg-calr`.

### 7.2 Delta to spec

- **New crate:** `crates/mg-vault-template/` — `lexer.rs`, `grammar.rs`, `allowlist.rs`, `eval.rs`, `clippy.toml` with the disallowed-type list, and the dependency-graph test. Added to the workspace members and to the workspace lint inheritance.
- **New modules in `crates/mg-vault-core/src/`:** `periodic/{calendar.rs,resolve.rs,settings.rs}` and `tasks.rs`; re-exports added to `lib.rs`.
- **New modules in `crates/mg-vault-cli/src/`:** `commands/{periodic,template,task}.rs` and `integrations/calr.rs`; the `Command` enum, help text, and completion scripts extended. `main.rs`'s inline command handling continues its **C**-planned split into modules.
- **Modified:** `crates/mg-vault-core/src/vault.rs` gains a multi-span variant of `edit_note_span` so toggle-plus-stamp is one write (or `tasks.rs` composes the spans and calls the existing single-span API on a pre-assembled buffer via `write_note` with a fingerprint precondition — either is acceptable, the invariant is one atomic replace). `crates/mg-vault-index` gains the task projection tables. `README.md` and `docs/PRODUCT.md` gain a truthful description of what shipped.
- **Schema and settings:** `.mg-vault/settings.json` at `version: 1` with the periodic, template, task-marker, and integration keys; `.mg-vault/state/calr-receipts.jsonl` (new, disposable, rotated). **B**'s `SCHEMA_VERSION` and `PARSER_VERSION` advance for the task projection, using **B**'s side-by-side rebuild, never an in-place mutation of a published generation.
- **New dependencies:** `jiff` (with `bundled-tzdb` as an off-by-default feature); `unicode-segmentation` and `unicode-width` if **F**/**G** have not already introduced them. Explicitly no PostgreSQL client, async runtime, TLS stack, or HTTP client.
- **New fixtures and harnesses:** the DST matrix vault; the ISO week-year and fiscal-quarter fixtures; the Obsidian coexistence vault carrying Templater, Tasks-plugin, and Dataview syntax; the template safety corpus plus the syscall-monitoring sandbox runner; the 250,000-task corpus generator; a fake `mg-calr` binary with scriptable envelopes for the three promotion worlds; golden JSON for every command and every error code.
- **Documentation:** the period definitions and timezone/day-start/DST policy; the date-token and variable/formatter allowlists with the explicit non-goals; the template safety rationale and the "computation belongs to plugins" boundary; the task syntax surface and dialect-preservation rule; the promotion contract, its one-directionality, and the explicit statement that no daemon exists.

### 7.3 Estimated scope

**L.** Four separable subsystems, none individually enormous but each carrying a hard correctness or safety invariant: a calendar/timezone resolver (small code, large fixture surface), a template engine (small code, whose value is entirely in what it refuses), a task span model (moderate, and cheap once **F** lands, expensive before), and a cross-product integration (small code, large failure-mode surface). It is smaller than **B**, **G**, or **H** because it introduces no service, no query planner, no index, and no refactoring engine — but the DST matrix, the template safety corpus, and the three-worlds promotion suite are substantial test investments that should not be traded away.

Deliver as gated increments, each individually reviewable and shippable behind the CLI: **J1** period resolution, `path`/`explain`/`open` with no templates, settings, and the DST matrix; **J2** the template crate, `render`/`validate`, and `create`-mode apply; **J3** the remaining apply modes with their confirmation and trash guarantees; **J4** task parsing, `show`, and span-local toggling; **J5** `task list` over **G**, plus stamping and recurrence; **J6** `mg-calr` promotion, `link-calr`, `calr-status`, and `sync-back`; **J7** the TUI panes. J1 and J2 are independently useful on the day they land.

### 7.4 Blocking dependencies

- **A (foundation)** — confined path validation, `O_EXCL` create, fingerprint-gated span edits, atomic replace, trash, and the planned journal/`settings.json`/quarantine recovery. Implemented for the parts J1–J4 need; the journal is planned and blocks J3's `replace` mode guarantee.
- **C (CLI operations)** — the planner/commit split, `--dry-run` plan schema, exit-code categories, prompt policy, slug algorithm, and `doctor`. J's commands must be consistent with it rather than inventing a second convention. Planned.
- **F (Markdown parser)** — the token-preserving model defining list items, task markers, code fences, and inline spans. J4 can ship against a bounded line scanner, but the scanner must be replaced by **F**'s tokens before J claims correct behaviour inside code fences, nested quotes, or unusual list nesting. Planned.
- **B (index service)** and **G (search/query)** — the task projection, freshness contract, ordering, pagination, and cancellation that J5 requires. J1–J4 and J6 do not need them at all. Planned.
- **E (TUI)** — panes, focus, keymaps, and session restore for J7. Absent **E**, every capability remains fully available from the CLI. Planned.
- **N (plugins)** — the WASM sandbox, manifest, and capability grant that plugin template variables require. Until **N** ships, `plugin.*` must fail closed with `template_capability_denied`; nothing in J1–J6 depends on it. Planned.
- **`mg-calr`** — an external product gate, not a build dependency. J6's acceptance requires `mg-calr`'s C1 `todo add` flag set and `version: 1` envelope to be stable and its `--version` output to be parseable. Every other J command must pass its acceptance suite with `mg-calr` uninstalled.

### 7.5 Non-goals

- Any template feature that computes, branches, loops, includes, or reaches outside the `TemplateContext` — permanently a plugin concern under **N**'s sandbox.
- Executing, translating, or "upgrading" Templater, Handlebars, Jinja, or any other template dialect found in a vault; they are preserved verbatim and otherwise ignored.
- A background sync daemon, watcher, timer, IPC channel, or polling loop between `mg-vault` and `mg-calr`, in either direction.
- Reading or writing `mg-calr`'s PostgreSQL database, its configuration, its connection string, or its `todo-projection.json`, under any flag.
- Bidirectional task synchronization, conflict resolution between a checkbox and a Postgres row, or automatic completion propagation in either direction.
- Minting block IDs, injecting UUIDs, or adding any hidden identifier to make a task addressable.
- Rewriting, normalizing, reordering, or converting task metadata between dialects; renumbering lists; reformatting tables; reserializing frontmatter.
- A calendar grid, heat map, or any other picture that carries a datum the textual list does not.
- Task dependency graphs, subtask trees, recurrence templates, reminder scheduling, and blocked-state semantics — those are `mg-calr`'s C4–C8, deliberately not duplicated here; `mg-vault` reads and preserves the syntax and hands the semantics to the product that owns them.
- Notifications, reminders, or any scheduled execution originating from `mg-vault`.

---

## 8. Open Questions

- **Q1:** Should `periodic.timezone` be required (this spec's choice — refuse with `timezone_unconfigured` rather than guess) or should first use silently adopt the host zone and record it? Requiring it costs one setup step and buys a vault whose meaning does not change when it travels. — **blocks:** §3.2 step 2 and the `timezone_unconfigured` error.
- **Q2:** Should `--date 2026-08-29` bypass the timezone entirely (this spec's choice, making it the reproducible form for scripts) or be interpreted as midnight in the configured zone and then re-derived? — **blocks:** §3.2 step 1 and the automation story in §2.
- **Q3:** Should `prompt.*` variables exist at all in the first release? They are the only variable family whose value is not derivable from the period and the target path, and they are the only reason `template apply` ever needs a prompt. Dropping them makes rendering totally non-interactive. — **blocks:** the variable allowlist in §4.3 and the `--var` flag in §3.4.
- **Q4:** What is the default for `tasks.stamp_completion`? This spec chooses **off**, on the grounds that a toggle should change one byte unless the user asked for more; Obsidian's Tasks plugin defaults it on and users may expect that. — **blocks:** the default in §3.2, not the mechanism.
- **Q5:** Should the promotion link default to the short ID alone (this spec's choice, human-scale and greppable) or always record the UUID too, since `mg-calr` short IDs are a display convenience and could in principle be re-issued? — **blocks:** the default in §3.2 step 8 and the `--record-uuid` flag.
- **Q6:** Should `task sync-back` ship at all in the first release? It is the only command in this feature that lets `mg-calr` state change a note, and even one explicit, confirmed, single-task command establishes a direction that a future user will ask to automate. — **blocks:** J6's contents in §7.3; the one-directional default stands either way.
- **Q7:** Should the template folder default to `Templates/` (Obsidian's common convention, discoverable) or require explicit configuration, given that a folder of templates is otherwise indistinguishable from ordinary notes to every other branch's queries? — **blocks:** the default in §3.3's empty-state copy and `periodic config`.
