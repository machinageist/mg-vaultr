# Spec: Import, Export, and Publishing

**Feature ID:** p-import-export-publishing
**Parent feature:** root
**Spec author agent:** Spec agent P (import/export/publishing)
**Date:** 2026-08-30
**Iteration:** 1

---

## 1. Purpose

### 1.1 One-sentence job

Let a person move an existing knowledge base *into* a vault without losing a byte or overwriting anything they did not approve, turn any subset of that vault *out* into Markdown, HTML, or PDF artifacts that never touch the source, and publish a deliberately chosen, reviewed, allowlisted subset to a destination they configured — with the network opened only by the command that says it will.

### 1.2 Why it matters

P is the product's only two-way door. Every other branch operates inside a vault the user already has; P is how a vault begins and how its contents leave.

Three distinct failure modes converge here, and each maps to an auto-fail rule:

1. **Import destroys what it touches.** Every importer users have been burned by does one of: overwriting a same-named file without asking, dropping a construct it could not convert and never mentioning it, or half-applying a 40,000-file migration and leaving no record of where it stopped. Those are, in order, *unconfirmed overwrite/import*, *source-content loss*, and *partial multi-file mutation*.
2. **Export is where inert source becomes a live document.** A note is bytes on disk; an exported HTML page is a thing a browser executes. Any `<script>`, `on*` handler, `javascript:` URL, or remote subresource that survives the export boundary is *active raw HTML/script by default* plus a privacy leak, because a remote `<img>` in a published page beacons every reader's IP address to a third party.
3. **Publishing is the exfiltration surface.** It is the one place in the entire product where a user's private notes are deliberately copied to a machine they do not own. A vault contains client names, credentials, health notes, and half-formed opinions. "Publish my notes folder" with a plausible-looking filter is how a person discovers, on a search engine, that `finance/2026-budget.md` was in the glob.

P answers all three with the same posture the rest of the product uses: **plan before you write, refuse before you guess, and never let a default do something the user did not say.** Import is a dry run until the user types otherwise; export is structurally incapable of writing to the vault; and nothing is published that is not on a list the user wrote in a file they own, staged as exact bytes, and reviewed.

### 1.3 Success signal

Over a versioned interop corpus — an Obsidian vault with plugin syntax and a populated `.obsidian`, a plain Markdown tree, a project-authored Notion export, a Logseq graph, the hostile-HTML corpus shared with **F**, and a secret-bearing publish fixture — all of the following hold simultaneously:

- **(a)** A whole-vault SHA-256 manifest (path set plus per-file digest) is **byte-identical before and after** every `import --dry-run`, every `export` of every format, and every `publish stage` and `publish review` in the corpus — zero exceptions, asserted per command, not per suite.
- **(b)** Set equality between the constructs the source-parser identifies and the union of `{carried, retained-verbatim, ledger-reported}` for every fixture: **an unmodelled construct fails the build rather than disappearing.**
- **(c)** Every note produced by import carries `source_tool`, `source_path`, `source_hash`, and `imported_at` as ordinary visible YAML properties, and the byte range of every other property in that file is unchanged, proven by prefix/suffix/gap equality.
- **(d)** Zero exported or staged HTML byte across the hostile corpus contains a `<script>`, `<iframe>`, `<object>`, `<style>`, an `on*` attribute, a `javascript:`/`data:`/`vbscript:`/`file:` destination, or any subresource reference outside the artifact — asserted against the file on disk, not the render.
- **(e)** A `connect(2)`-denying harness proves that **no** P command other than `publish push` attempts a connection, including every import, every export, `publish stage`, `publish review`, `publish status`, and the TUI panes.
- **(f)** With a `deny`-class secret planted in an allowlisted note and a second secret in an allowlisted-but-M-excluded note, `publish stage` refuses, and the secret bytes appear in no staging file, no manifest, no receipt, no log line, and no byte delivered to the fake destination.
- **(g)** A crash injected at every phase of a 40,000-file `import --apply` converges, on the next invocation, to a state where every batch is complete-pre or complete-post, no file is half-written, and `import status` names the exact resume point.

---

## 2. User Stories

> As someone leaving another tool, I want `mg-vault import notion ~/Downloads/Export` to show me every file it would create, overwrite, skip, or rename — with counts and a list of exactly what it cannot represent — and write nothing until I say so, so that a migration is a decision rather than a discovery.

> As an Obsidian user who is not leaving Obsidian, I want to point `mg-vault` at my existing vault and have it change nothing at all — my `.obsidian` untouched, my files byte-identical — so that trying this tool costs me nothing and I can stop at any time.

> As a careful migrator, I want every imported note to say in plain YAML which tool it came from, which file, when, and with what content hash, so that six months later I can tell a verbatim copy from a lossy conversion without trusting my memory.

> As a writer, I want `mg-vault export html --dir essays --out ~/site` to produce clean, offline-readable pages with my alt text intact and my diagrams still legible as text, and to be certain it did not modify a single note, so that publishing is not a risk to my source.

> As a security-minded user, I want publishing to be impossible for any note I have not explicitly listed in a file I control, to be blocked outright for anything my exclusion rules or secret scanner flag, and to require me to look at the exact bytes first, so that "I published my vault by accident" is not a state this tool can reach.

> As an automation author, I want `import --dry-run --json`, `export --out -`, and `publish stage --json` to emit stable versioned envelopes, and `--no-input` to refuse rather than assume a collision policy, so that a script cannot silently overwrite my notes.

> As a screen-reader and no-color terminal user, I want the import report, the per-collision prompts, the staging review, and progress output to be plain ordered lines with literal state words and no reliance on color, spinner, or table alignment, so that the highest-consequence confirmations in the product are fully readable to me.

> As someone on a machine without a PDF renderer, I want `export pdf` to say which tool is missing and how to install it and write nothing, rather than emitting a zero-page file, so that a missing capability is visible.

---

## 3. UX Specification

### 3.1 Screen / view inventory

P is a CLI-and-TUI feature. It introduces no graphical screens, windows, modals, or pointer gestures. It adds these line-oriented views and three TUI panes hosted by **E**:

| View | Entry point | New / modified | Layout pattern |
|---|---|---|---|
| Import plan report | `mg-vault import <SOURCE> <PATH>` (dry run is the default) | New | Full-width ordered record block on stdout; per-class sections; complete refusal list |
| Import per-collision prompt | Interactive `import --apply` when the plan holds an unresolved collision | New | One record per collision on stderr; prompt on `/dev/tty` |
| Import loss ledger | `mg-vault import … --ledger` and `_import/<id>/IMPORT.md` | New | Grouped construct table plus per-note detail; also written as vault content |
| Import status / resume | `mg-vault import status`, `import resume <id>` | New | Ordered batch list with phase and digest per batch |
| Source capability table | `mg-vault import sources` | New | One row per source kind: detection rule, fidelity, what is retained |
| Export receipt | `mg-vault export {markdown,html,pdf,snapshot}` | New (`snapshot` re-homes today's `interop export`) | Destination, artifact count, bytes, per-artifact manifest, warnings |
| Publish allowlist inspector | `mg-vault publish allow {list,check PATH,init}` | New | Rule table naming the deciding layer and rule per path |
| Publish staging review | `mg-vault publish review <stage-id>` | New | Class counts, complete path list, gate verdicts, closing negative statement |
| Staged artifact viewer | `mg-vault publish review <id> --show PATH` | New | Exact staged bytes to stdout, unmodified |
| Publish push confirmation | `mg-vault publish push --stage <id>` | New | Redacted destination, `will contact network: yes`, escalated typed confirmation on `/dev/tty` |
| Publish status | `mg-vault publish status` | New | Destination, last publish, per-class counts, drift since last stage |
| Import review pane (TUI) | E command palette → `import` | New pane kind, hosted by E, semantics owned by P | Header, per-class row list, decision column, detail pane |
| Publish staging pane (TUI) | E command palette → `publish` | New pane kind, hosted by E | Header, artifact list, gate verdict rows, byte-preview pane |
| Export pane (TUI) | E command palette → `export` | New pane kind, hosted by E | Selector summary, artifact list, warning list |

Stream discipline follows **C**'s version-1 envelope unchanged: human success on stdout, warnings and errors on stderr; under `--json` exactly one envelope object plus `\n` on the corresponding stream with the other stream empty. Prompts go to `/dev/tty` when one exists and never into a redirected stream. Exported content sent to `--out -` is data and goes to stdout alone.

### 3.2 Interaction flows

#### The three rules that shape every flow

1. **Import is a dry run until the user says otherwise.** `mg-vault import <SOURCE> <PATH>` plans and reports; it writes nothing and exits `0` with `changed: false`. Writing requires `--apply`. There is no `--force`, and no flag combination makes overwrite the default.
2. **Export cannot write to the vault.** `mg-vault-export` receives a read-only `VaultReader` capability and has no path to `create_note`, `write_note`, `edit_note_span(s)`, `trash_note`, or `mg_vault_core::transaction` (§4.1). An `--out` path inside the vault root is refused unless `--allow-in-vault` is given, and even then it must be under `_export/`.
3. **Publishing is staged, allowlisted, and network-silent until `push`.** Nothing is publishable that is not matched by the user's allowlist file; nothing reaches a socket except from `publish push`; and a stage is evidence, never authorization.

#### `import` — plan

1. Resolve the target vault through **A**. Stat `.mg-vault/refactor/ACTIVE` and `.mg-vault/import/ACTIVE`; refuse with `transaction_incomplete` and name the recovery command if either exists.
2. **Detect and confirm the source kind.** The explicit subcommand (`obsidian`, `markdown-dir`, `notion`, `logseq`) is authoritative. `import detect PATH` reports what the heuristics see (`.obsidian/` present, a Notion `*_all.csv` plus 32-hex filename suffixes, a Logseq `logseq/config.edn` with `journals/` and `pages/`) but never silently switches kinds. A mismatch between the named kind and the detected kind is a **warning printed before the report**, not an override.
3. **Enumerate** the source through a confined walk. The source directory is opened as its own `VaultDir`-equivalent confinement root: `..`, absolute paths, and symlinks that leave the source root are refused, and a symlink *inside* the source is reported as `not-imported (symlink)` with its target — never silently skipped, never followed.
4. **Classify every source file** into exactly one class: `create`, `overwrite`, `rename`, `skip`, `refuse`, or `not-imported`. Every source byte is accounted for in exactly one class; the classifier's totals are asserted against the enumeration count.
5. **Convert and hash.** For each importable file, produce the target bytes, the target vault-relative path, the source SHA-256, and the derived SHA-256. Verbatim sources (`obsidian`, `markdown-dir`) produce derived bytes equal to source bytes plus, when provenance mode is `frontmatter`, a span-local frontmatter insertion computed through **I**'s lossless CST — never a reserialization.
6. **Build the loss ledger** (§3.3). Every construct the source adapter recognized but could not carry lands here with its count, its disposition, and the path of the retained original.
7. **Report and stop.** Print the plan report; write nothing; exit `0`. `--plan-out FILE` serializes the canonical plan (**C**'s plan model, domain `mg-vault.import-plan`).

**Branch — collisions.** A target path that already exists is resolved by `--on-collision`:

| Policy | Behavior | Confirmation class |
|---|---|---|
| `skip` | Existing file untouched; source counted as `skip` | Ordinary |
| `rename` | Import to `<stem>--<source-tool>-<n>.md`, create-new semantics; never replaces | Ordinary |
| `overwrite` | Pre-image goes to **A**'s trash first; then fingerprint-checked replacement | **Escalated**, and additionally requires `--overwrite-existing` |
| `fail` | The whole plan refuses if any collision exists | Ordinary |
| `ask` | Per-collision decision at apply time (interactive default) | Per decision |

There is **no default under `--no-input`.** If a plan contains at least one collision and no `--on-collision` was given, the command refuses with `collision_policy_required`, exit `4`, lists every colliding path with both digests, and writes nothing. This is the direct answer to the *unconfirmed overwrite/import* auto-fail rule: the absence of a decision is never resolved by the tool.

A collision where the existing bytes are **identical** to the incoming derived bytes is classified `skip` regardless of policy, is reported as `skip (identical bytes)`, and requires no decision — that is a no-op, not an overwrite.

**Branch — provenance key conflict.** If a source note's frontmatter already defines one of P's provenance keys, P does not overwrite it. The note is classified `refuse` with `provenance_key_conflict`, naming the key and both values. `--provenance-prefix mgv_` re-plans with prefixed keys; `--provenance sidecar` re-plans with no frontmatter write at all.

**Branch — path is not representable.** A source path that fails `validate_note_path` after slugging (non-`.md` extension, a component that is `.` / `..`, a first component of `.obsidian` or `.mg-vault`, a non-UTF-8 byte sequence, a name that de-suffixes onto another name) is `refuse`, not silently renamed. Notion's 32-hex filename suffixes are stripped to produce readable paths; when stripping makes two source files collide, **both** are refused with both original names shown. P never picks a winner (3B).

#### `import` — apply

1. `--apply` re-runs the identical planner (there is no second, weaker path), then revalidates every precondition from freshly opened handles. A `--from-plan FILE` apply additionally verifies the plan hash, domain, schema, expiry, vault root identity, and every source file's digest; drift returns `plan_mismatch` and writes nothing.
2. **Confirm.** Escalated confirmation — the user types the literal word `import` — whenever the plan writes anything. If the plan contains any `overwrite` step, the prompt states the overwrite count first and requires the literal word `overwrite`. Under `--no-input`, `--yes` is honored only when every class present is covered by an explicit policy flag; any `refuse` class refuses regardless of `--yes`.
3. **Apply in batches through H.** Each batch of at most `--batch-size` (default 2,000, `--single-transaction` forces one batch and is refused above 5,000 files) is exactly one `mg_vault_core::transaction` plan with `operation: Import`, inheriting **H**'s staging store, write-ahead journal, single commit-point barrier, canonical step order, digest-driven idempotent replay, and quarantine-on-drift. P implements no second transaction engine.
4. **The batching contract is disclosed before confirmation, not after.** The report prints `batches: 21 (2000 files each); each batch is all-or-nothing; an interrupted import leaves a whole number of completed batches`. P's own journal at `.mg-vault/import/journal/<import-id>.json` records the ordered batch list with each batch's plan hash and phase, so `import status` reports `applied 13 of 21` and `import resume <id>` continues from batch 14 after re-verifying every remaining source digest. Because import only creates files, or overwrites with the pre-image already in trash, a partially applied import is a well-defined prefix of a disclosed sequence — never a half-written file and never an undisclosed partial state.
5. **Write the ledger as vault content.** `_import/<import-id>/IMPORT.md` (human) and `_import/<import-id>/manifest.json` (machine) land in the final batch. Retained originals are written under `_import/<import-id>/originals/<source relative path>` in earlier batches, **before** any derived note that references them.
6. **Report.** Files created / overwritten / renamed / skipped / refused, batches applied, bytes written, trash receipts for every overwrite, `durability: synced`, and the line `index: stale until reconciled (mg-vault index status)`.

#### The three-tier no-loss rule

Every construct in a source file lands in exactly one tier, and the tier is reported:

- **Tier 1 — carried.** The bytes become part of the derived note.
- **Tier 2 — retained.** The exact source bytes are copied verbatim to `_import/<id>/originals/…`, and the derived note's `source_original:` property names that path.
- **Tier 3 — reported.** A construct that can be neither carried nor meaningfully converted is preserved as literal text in the body where the source is textual (**F**'s `Unrecognized` token guarantees it renders as literal text rather than being interpreted), and is listed in the loss ledger with path, construct name, occurrence count, and the retained original that still holds it.

**Binding invariant:** any source whose derived note is not byte-identical to its source file **forces `--keep-originals` on**. `--no-keep-originals` is accepted only for `obsidian` and `markdown-dir` in verbatim mode; requesting it for `notion` or `logseq` is a usage error (exit `2`), not a silent downgrade. You cannot perform a lossy import and keep no original.

#### Per-source mapping

**`obsidian`** — the coexistence case. Two modes:

- `--in-place` (recommended, and what `import detect` suggests for a vault containing `.obsidian/`): **zero writes.** P registers the directory through **A**, audits it, and prints what a conversion *would* change — which is nothing, because Obsidian's Markdown is mg-vault's Markdown. `.obsidian` is read (for `app.json`'s `attachmentFolderPath`, `newLinkFormat`, `useMarkdownLinks`, and `types.json` to seed **I**'s `schema infer` proposals) and **never written, never copied, never mutated** (1D).
- Copy mode: `.md`, `.canvas`, `.base`, and attachments are copied **byte-identically**; relative directory structure is preserved exactly so every wikilink keeps resolving; frontmatter is untouched except for the provenance insertion.
- *Not imported, listed by name:* `.obsidian/workspace.json`, hotkeys, themes, snippets, and community-plugin data — machine-local app settings that are not vault content. `--include-obsidian-config` copies them to `_import/<id>/originals/.obsidian/` for reference; they are **never** written into the target's `.obsidian`.
- *Cannot be represented at render time:* syntax produced by community plugins (Dataview queries, custom `:::admonition:::` blocks, templater expressions). These are Tier 1 — the bytes are carried verbatim — and are listed in the ledger under `unsupported-at-render`, which is explicitly **not** loss.

**`markdown-dir`** — every `.md` and every attachment with an allowlisted extension is copied byte-identically. Any other file type is `not-imported (unsupported type)` with its full path listed, never a bare count. `--include-all` copies them as attachments instead.

**`notion`** — the structured case (HTML export or Markdown+CSV export, unzipped).

| Notion construct | Disposition |
|---|---|
| Page title, headings, paragraphs, lists, quotes, code blocks, tables, callouts, dividers | **Carried** → CommonMark/GFM plus Obsidian callout syntax |
| Page properties table | **Carried** → typed YAML frontmatter through **I**'s property model |
| Internal page links | **Carried** → wikilinks, after the whole plan's path map is known; an unresolvable target stays literal text and is reported (never guessed) |
| Files and images | **Carried** as attachments; the reference is rewritten to the new relative path |
| Database (`*_all.csv` + per-row pages) | **Carried** → one note per row with typed frontmatter. A `.base` view file is emitted **only** under `--emit-base`, and is proposed, never assumed |
| 32-hex filename/URL id suffixes | Stripped for readability; **original recorded in `source_path`**; a de-suffix collision refuses both files |
| Synced blocks | **Retained + reported** — the block's text is carried at its literal location; the sync relationship is not representable |
| Database views, filters, sorts, groupings | **Retained + reported** — not representable; the original export file holds them |
| Comments and discussions | **Retained + reported** — not representable in a note body |
| User mentions | **Carried** as the display name in text; the user identity is **reported** as not representable |
| Page icon / cover | Emoji icons **carried** as an `icon:` property; asset covers copied as attachments with a `cover:` property; page layout **reported** as not representable |
| Permissions, sharing, version history | **Reported** — not present in a Notion export at all; P states this explicitly rather than implying it preserved them |

**`logseq`** — the second structured case.

| Logseq construct | Disposition |
|---|---|
| `pages/*.md` | **Carried** byte-near-identically; outline bullets preserved as Markdown lists |
| `journals/YYYY_MM_DD.md` | **Carried**, renamed into **J**'s configured periodic path with the mapping printed per file |
| `key:: value` block properties | **Carried** → frontmatter when on the first block; otherwise **carried as literal text** and reported, because a block-level property has no frontmatter equivalent |
| `[[refs]]`, `#tags`, `((block-refs))` | Page refs and tags **carried**; `((uuid))` block refs are **carried as literal text** and reported — resolving them would require injecting block ids, which 1C forbids |
| `id:: <uuid>` properties | **Carried verbatim as ordinary text/properties.** P neither honors them as identity nor strips them; path remains identity |
| `:LOGBOOK:` / `CLOCK:` drawers | **Carried verbatim** as literal text and reported as not modelled |
| `logseq/config.edn`, `.recycle`, `.bak` | `not-imported (app settings)`; listed |

#### `export`

1. **Select.** `--note PATH` (repeatable), `--dir DIR`, `--from-file LIST`, `--allowlist` (the publish allowlist), or `--query '<G/I query>'`. A query selector requires **B**'s freshness to be `current`; `stale`/`degraded` refuses unless `--allow-stale`, and then every artifact and the receipt carry the literal word `stale` and the observation time. An index is never permitted to *add* a note the user did not select nor to supply note bytes — bytes always come from a fresh confined read of source.
2. **Render.** Markdown export copies source bytes (`--fidelity verbatim`, default) or emits a portable form (`--fidelity portable`: wikilinks → relative Markdown links, embeds → links, callouts → blockquotes) with every transformation counted in the receipt. HTML and PDF export go through **F**'s `RenderDocument` and `render/sanitize.rs`.
3. **Write to the destination** with create-new semantics per artifact; an existing artifact at the destination requires `--overwrite-artifacts` and is reported per file. Export writes only under `--out` and never inside the vault (§3.2 rule 2).
4. **Report.** Artifact count, total bytes, per-artifact manifest (`artifact path`, `bytes`, `sha256`, `source note`, `source fingerprint`), warning list, and the literal line `source notes modified: 0`.

**HTML export specifics.** `--html-mode sanitized` is the **default for export** (this resolves **F**'s open question Q4). Rationale: in an artifact the render *is* the product, and `literal` would turn a user's deliberate `<sub>` into visible tag text, which is a fidelity loss in a document meant to be read. Sanitized is not permissive: it is **F**'s deny-by-default allowlist, byte-for-byte the same policy, imported as `mg_vault_markdown::render::sanitize` — P defines no HTML policy of its own and its output constructors accept only `SafeInline`/`SafeBlock`, so an unsanitized export path fails to compile. `--html-mode literal` remains available for the maximally conservative case. Every exported page additionally carries:

- `<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src 'self' data:; style-src 'sha256-…'; script-src 'none'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'">` — defense in depth behind the sanitizer, not instead of it.
- `rel="noopener noreferrer nofollow ugc"` and no `target` on every anchor.
- **No off-artifact subresource.** Images resolve to `self` or an inlined `data:` URI; a remote image becomes **F**'s blocked media card (destination as inert text, alt text preserved). No web font, no analytics, no CDN. The page renders completely from `file://` with the network down (5A) and does not report the reader's IP to a third party (4F).
- `lang` on `<html>`, a `prefers-color-scheme` stylesheet with both palettes meeting WCAG AA contrast, and a system font stack.

**Link rewriting across the export set.** A link to a note **inside** the set becomes a relative link. A link to a note **outside** the set is handled by `--outside-links`: `show-path` (default for local export — renders the target path as inert text, which is useful on your own machine) or `flatten` (**default and non-overridable for publish** — renders the link text only, with the target elided, because the path of an unpublished note is itself private information). An **ambiguous** link (**G**'s `Resolution::Ambiguous`) is never resolved: it flattens with the literal marker `[ambiguous]` and is listed with its ordered candidates. An unresolved link flattens with `[unresolved]`. Counts for all four cases appear in the receipt.

**PDF export.** Feature-gated at compile time (`--features pdf`) and dependent at runtime on an external renderer (`typst`, or `weasyprint` under `--pdf-engine weasyprint`), invoked as a separate process with a clean environment, a temporary root containing only P's generated input, and no network. When the feature is off or the tool is absent, `mg-vault capabilities` reports `pdf_export: unavailable — reason: typst not found on PATH; install: pacman -S typst`, and `export pdf` exits `6` writing nothing. It never emits a zero-page or truncated PDF, and never silently downgrades to HTML (2E, 4E).

#### `publish` — allowlist, stage, review, push

**The three files, all owned by the user:**

| File | Location | Purpose |
|---|---|---|
| `mg-vault.publish-allow.txt` | **Vault root** — ordinary content, syncs and diffs with the notes | The allowlist. One vault-relative path or glob per line; `#` comments; `!pattern` negations. **Deny by default: an unmatched note is never published.** Negations always beat allows |
| `mg-vault.publish.yaml` | **Vault root** | Destination, profile, property projection list. **Contains no credential, ever** |
| `.mg-vault/publish/state.json` | Control dir, `0600` | Disposable digest map of what was last published. Deleting it costs only the diff |

**`publish stage`** — renders exactly what would be published and writes it to `.mg-vault/publish/staging/<stage-id>/` (or `--out DIR`). Gates, applied in order, every one a refusal rather than a warning:

- **G1 Allowlist.** Unmatched → not staged. Reported as a count, with `--list-unmatched` naming them.
- **G2 Exclusion beats allowlist, always.** Every candidate is evaluated against **M**'s `ExclusionSet` — the one evaluator that already serves staging, backup, and export. A path excluded by the built-in layer, the user layer, or `.gitignore` **can never be published even if allowlisted**, and the built-in deny layer is not overridable by an allowlist entry any more than it is by a `.gitignore` negation. Reported as `refused: excluded (layer=builtin, rule=…)`.
- **G3 Secret scan.** **M**'s `scan_bytes` runs over both the source note and the staged artifact. Any `deny` finding refuses **the entire publish**, not just that file — a partial publish after a secret hit invites a retry that ships the rest unexamined. `warn` findings refuse under `--no-input` and require `--accept-warnings` interactively.
- **G4 Embed closure.** An allowlisted note that transcludes a **non**-allowlisted note does not expand it. The artifact carries `[embedded content not published]` and the count is reported. This is the subtlest exfiltration path in the product: publishing a private body through a public note's back door.
- **G5 Property projection.** Only properties on an explicit positive list (`title`, `tags`, `date`, `description`, `aliases`, plus whatever the user adds in `mg-vault.publish.yaml`) reach the artifact. Every other property is withheld from the artifact — never from the note. The review reports `31 properties withheld across 22 notes`; `--list-withheld` names the **keys**, never the values.
- **G6 Attachment closure.** An attachment is published only if it is referenced by a published note **and** itself allowlisted. Otherwise the reference flattens to its alt text and is reported.
- **G7 Provenance stripping.** Import provenance keys that can carry a local filesystem path (`source_path`, `source_original`) are stripped from published artifacts by default and the count is reported.
- **G8 Path-leak audit.** The staged bytes are scanned for the vault root string, `$HOME`, the OS username, and the path of any note not in the publish set. Any hit refuses with the artifact, offset, and a redacted excerpt.

Staging writes `MANIFEST.json` (per artifact: path, bytes, sha256, source note path, source fingerprint), `REVIEW.md`, and the gate verdicts. It **never opens a socket**, and its receipt says so.

**`publish review <stage-id>`** prints the review in fixed order (§3.3), including the complete refused list with no elision, and closes with the literal lines `will contact network: no` and `nothing has been published.` `--show PATH` writes one staged artifact's exact bytes to stdout so the user can read what would ship.

**`publish push --stage <stage-id>`** — the only P command that opens a socket.

1. Re-verify every staged artifact's digest against `MANIFEST.json`; any mismatch refuses.
2. **Re-run G1–G8 against live source.** A stage is evidence, not authorization: if a note changed, was removed from the allowlist, or newly trips the scanner since staging, push refuses and names the drift.
3. Print the destination with userinfo stripped, the class counts, and the literal line `will contact network: yes`.
4. Escalated confirmation: the user types `publish`. Under `--no-input`, push requires both `--yes` **and** `--stage-digest <sha256>` equal to the manifest digest, so an automated publish must have observed the exact staged content.
5. Resolve the credential (§4.5), upload changed and new artifacts only (unchanged digests are skipped, `changed: false`), and write a receipt.
6. **Removals are never automatic.** An artifact present at the destination and no longer in the publish set is reported as `orphaned at destination`; deleting it requires `--prune` and its own confirmation.
7. **Interruption is truthful.** `Ctrl-C` stops after the current artifact; the receipt records exactly which artifacts reached the destination and P reports `partial: N of M delivered` — never a success verb for an incomplete publish.

### 3.3 Layout descriptions

Every human view uses one fact per line, a leading literal label, then the value. Tables are space-aligned at ≥ 60 columns and become one-record-per-block below 60. **Nothing is truncated** — paths, digests, detector names, and rules wrap with a two-space continuation indent. JSON never varies with terminal width. Paths render through **A**'s escaping contract, so a hostile source filename cannot emit terminal control sequences.

**Import plan report.** Reading order: header → target → policy → class counts → loss ledger → refusals → closing negative statement → the exact command to apply.

```
import plan  20260830T141244Z-3f2a
source: notion    /home/j/Downloads/Export-8f3a2b1c
target: notes     /home/j/notes   (into: imported/notion)
provenance: frontmatter    originals: kept (conversion is lossy)
collision policy: not set

create        412 files   8.1 MiB
overwrite       0 files
rename          7 files   (would need: --on-collision rename)
skip           19 files   (identical bytes)
refuse          3 files
not-imported   61 files   (unsupported type; listed with --list-skipped)

loss ledger: 5 construct kinds across 88 notes; all originals retained
  synced-block        41 notes   text carried at its location; sync relation not representable
  database-view       12 notes   not representable; original retained
  page-comment        22 notes   not representable; original retained
  user-mention        11 notes   display name carried; user identity not representable
  page-cover           2 notes   asset carried; page layout not representable

refused (3) — each needs a decision; nothing is assumed:
  imported/notion/Meeting notes.md
    reason: collision, existing bytes differ
    existing sha256:9f2c1ab4…  incoming sha256:41ab77de…
    choose: --on-collision {skip|rename|overwrite} or --resolve 'imported/notion/Meeting notes.md=rename'
  imported/notion/Roadmap.md
    reason: de-suffix collision with imported/notion/Roadmap.md
    sources: 'Roadmap 1a2b….md', 'Roadmap 9f8e….md'
    choose: --keep-suffixes or --resolve per path
  imported/notion/Ledger.md
    reason: provenance_key_conflict on 'source_path'
    choose: --provenance-prefix mgv_ | --provenance sidecar

nothing has been written.
apply with:  mg-vault import notion /home/j/Downloads/Export-8f3a2b1c --apply --on-collision rename
```

**Publish staging review.**

```
publish review  stage-20260830T151002Z-77c1
destination: rsync-ssh  ssh://deploy@static.example.com/srv/notes
credential:  external command (pass show hosting/deploy); not stored by mg-vault
allowlist:   mg-vault.publish-allow.txt   14 rules, 2 negations
profile:     html/1   sanitizer mg-vault.html-allowlist/1   properties mg-vault.publish-props/1

new          12 artifacts   842 KiB
changed       3 artifacts    61 KiB
unchanged    41 artifacts          (not re-uploaded)
removed       1 artifact           (orphaned at destination; --prune required)
refused       2 notes
withheld     31 properties across 22 notes   (--list-withheld for keys)
flattened    18 links to notes outside the publish set
              4 embeds of unpublished notes; bodies NOT included

refused (2):
  finance/2026-budget.md
    gate: G2 exclusion   layer=builtin   rule=**/secrets/**
    allowlisted: yes — exclusion wins over the allowlist, always
  ops/deploy-keys.md
    gate: G3 secret      detector=pem-private-key   line=14   redacted=----…----

secret scan: 1 deny, 0 warn  →  publish refused until resolved
path-leak audit: 0 findings
will contact network: no
nothing has been published.
```

**Data sources.** Import views are driven by the import planner over a fresh confined source walk plus **A**'s path authority; export views by **F**'s `RenderDocument` over fresh source reads plus **G**'s resolver; publish views by the allowlist evaluator, **M**'s `ExclusionSet` and `scan_bytes`, and the staging manifest. **No P view is driven by B's index** except the optional `--query` selector, and that path is labeled with **B**'s freshness word and never supplies note bytes.

**Empty states.** `import sources` with no argument lists the four kinds and their detection rules. `publish stage` with no allowlist file prints: `No allowlist at mg-vault.publish-allow.txt. Nothing is publishable until you create one. Start with: mg-vault publish allow init` — deny-by-default made visible rather than an error. `publish status` before any publish prints `Never published from this vault.` An export selection matching zero notes prints `0 notes selected; no artifacts written.` and, when index freshness is not `current`, explicitly does not imply that zero source notes match.

### 3.4 Input & gestures

- **CLI grammar.** `mg-vault import {detect|sources|status|resume} …` and `mg-vault import {obsidian|markdown-dir|notion|logseq} PATH [--into DIR] [--apply] [--on-collision P] [--overwrite-existing] [--provenance {frontmatter|sidecar|both}] [--provenance-prefix S] [--keep-originals|--no-keep-originals] [--batch-size N] [--single-transaction] [--resolve 'PATH=CHOICE']…`. `mg-vault export {markdown|html|pdf|snapshot} <selector> --out DEST [--fidelity …] [--html-mode …] [--outside-links …] [--single-file] [--assets {copy|inline|omit}] [--pdf-engine …]`. `mg-vault publish {allow {init|list|check PATH}|stage|review|status|push|prune}`.
- **Global flags** unchanged from **C**: `--vault`, `--json`, `--no-input`, `--no-color`, plus `NO_COLOR`, `--dry-run` (a no-op on import, which is already dry by default, and accepted for symmetry), `--plan-out`, `--from-plan`.
- **TUI keys** (E-hosted, all also reachable from the command palette so no binding is required — 2A). Import review pane: `j`/`k` move; `Enter` opens the per-file detail; `s`/`r`/`f` set skip/rename/refuse on the selected row; `S`/`R`/`F` set the whole class; **there is no bulk-overwrite key** — overwrite requires the CLI flag plus its typed confirmation; `a` applies through the escalated confirm overlay. Publish pane: `Enter` previews the exact staged bytes, `d` diffs against the last published digest, `g` jumps to the gate verdicts, `p` pushes through the escalated overlay. Export pane: `Enter` previews an artifact, `w` writes.
- **Specialized input:** none. No stylus, controller, voice, or camera path exists or is planned.
- **Responsive behavior:** the width ladder above; `--width N` overrides detection; `COLUMNS` is honored; JSON is width-invariant.

### 3.5 Transitions & animation

There is no animation. All views are instantaneous full-state redraws, consistent with **E**. The only time-dependent visual is a long-operation progress counter — a static line updated at most 2 Hz (`hashing 4102/12841`, `staging 812/1204`, `uploading 9/15`) — which is suppressed entirely under `--reduce-motion`, `MG_VAULT_REDUCE_MOTION`, screen-reader mode, `--json`, and any non-TTY stdout. The final summary line always prints regardless, so no information exists only in the transient counter. Reduced-motion behavior is therefore identical to default behavior minus the counter; no information is conveyed by timing, and no P view uses a spinner or a progress bar.

### 3.6 Error states

| Error | Trigger | Presentation | Recovery | Data loss risk |
|---|---|---|---|---|
| `collision_policy_required` | Plan has collisions, no `--on-collision`, `--no-input` | stderr block listing every colliding path with both digests | Re-run with a policy or `--resolve` per path | **No** — nothing written |
| `overwrite_unconfirmed` | Overwrite steps present without `--overwrite-existing` + typed confirmation | stderr block naming the count and every path | Supply both signals | **No** |
| `provenance_key_conflict` | Source frontmatter already defines a P key | Per-note record with key and both values | `--provenance-prefix` or `--provenance sidecar` | **No** |
| `unsafe_path` | Source path escapes, or slugs to `.obsidian`/`.mg-vault`/non-`.md` | Names the rejected operand and the rule; never prints a resolved outside-vault path | Rename at the source or `--resolve` | **No** |
| `desuffix_collision` | Two Notion files de-suffix to one path | Both original names, both digests | `--keep-suffixes` or per-path `--resolve` | **No** |
| `transaction_incomplete` | An `ACTIVE` marker exists | Names the recovery command | `refactor recover` then `import resume` | **No** |
| `import_partial` | Crash mid-apply | `import status` names applied batches and the resume point | `import resume <id>` | **No** — every batch is complete-pre or complete-post |
| `drift_quarantined` | H recovery found a file matching neither pre- nor post-image | Path plus three digests; staged bytes quarantined | Operator decides | **No** — both versions retained |
| `export_destination_inside_vault` | `--out` resolves under the vault root | Names the root and the resolved destination | `--out` elsewhere, or `--allow-in-vault` under `_export/` | **No** |
| `artifact_exists` | Destination artifact present without `--overwrite-artifacts` | Per-artifact list | Re-run with the flag or a clean destination | **No** — outside the vault, but never silent |
| `index_stale` | `--query` selector with non-`current` freshness | Literal freshness word, observation time, what still works, one recovery command | `index rebuild`, or `--allow-stale` with labeled artifacts | **No** |
| `pdf_engine_unavailable` | `pdf` feature off or renderer absent | Names the tool, the reason, and the install command | Install, or use `export html` | **No** — nothing written |
| `allowlist_missing` / `allowlist_empty` | No allowlist, or it matches nothing | Literal deny-by-default statement plus `publish allow init` | Write the allowlist | **No** |
| `excluded_but_allowlisted` | G2 | Names the deciding layer and rule, and states exclusion wins | Remove the exclusion deliberately, or the allowlist entry | **No** |
| `secret_detected` | G3 | Path, line, detector, redacted excerpt (first 4 / last 4) | Rotate first, then remove; remediation printed as text | **No** |
| `path_leak_detected` | G8 | Artifact, offset, redacted excerpt | Edit the note or adjust projection | **No** |
| `stage_drift` | Live source changed since staging | Names each drifted path with both digests | Re-stage and re-review | **No** |
| `credential_unavailable` | Secret command missing, failing, or empty | Names the command; **never echoes output** | Fix the command | **No** — never falls back to unauthenticated |
| `publish_partial` | Interrupt or transport failure mid-upload | `partial: N of M delivered`, per-artifact state | Re-run push; unchanged digests skip | **No** locally; destination is partially updated and says so |
| `network_unavailable` | Transport cannot connect | Redacted destination and the transport error | Retry | **No** |

Presentation choice: every error is a single stderr block. A CLI has no banner, toast, or modal, and an error inside stdout would contaminate pipes and JSON consumers. Under `--json`, the same information is **C**'s error envelope on stderr with stdout empty, so a consumer reading only stdout cannot mistake a failure for an empty success. No P error message ever contains note bytes, a full secret, a credential, or a credentialed URL.

### 3.7 Accessibility

- **Screen reader.** P adds no graphical element. In **E**'s screen-reader mode (`--screen-reader`, `MG_VAULT_SCREEN_READER=1`), each P pane is its linear record list; selection changes emit one announcement in fixed order — pane → row ordinal → class word → path → decision → gate verdict. Overlays (the import confirm, the publish confirm) become numbered linear prompts. Progress counters are suppressed. The same transcript adapter **D** and **E** already specify is reused, so these assertions run headlessly.
- **Traits and labels.** Every interactive row announces its class (`create`, `overwrite`, `refuse`, `excluded`, `secret`) as a literal word, its current decision, and whether a decision is still required. The confirm overlay announces the exact literal word it requires.
- **Custom actions.** The per-class bulk decisions (`S`/`R`/`F`) and per-row decisions are exposed as named palette commands (`import.decide.skip`, `import.decide.rename`, …), so no capability depends on a chord.
- **Text scaling.** Terminal-native: P emits no fixed-width assumption beyond the ≥ 60 / < 60 ladder, and every field wraps rather than truncating.
- **Color independence.** Every state is a literal word — `create`, `overwrite`, `skip`, `refuse`, `excluded`, `secret`, `stale`, `partial`. Color is decoration only; `--no-color` and `NO_COLOR` produce semantically complete output, asserted by an ANSI-stripping test.
- **Focus order and keyboard navigability.** Fixed and documented: header → class summary → row list → gate verdicts → action bar. `Tab`/`S-Tab` traverse in that order; every action has a palette command.
- **Accessible exported artifacts (5C).** This is P's distinctive obligation, since P produces artifacts consumed outside the terminal:
  - **Alt text is carried through** every format. Markdown export preserves it verbatim; HTML emits `alt`; PDF emits `image(..., alt: …)` where the engine supports it. When a source image has no alt text, the artifact emits `alt=""` plus a visible caption reading literally `description: unavailable`. **A filename is never promoted to a description**, matching **F**.
  - **Diagrams have no raster path, and that is a specified behavior, not a gap.** A mermaid fence exports as a `<figure role="img" aria-label="diagram: …">` containing (a) the diagram source in a `<pre>` and (b) **F**'s ordered node list and edge list as a real `<ul>`. There is no image. Under `--diagram-image` with an external renderer, an SVG may be added, and it is `aria-hidden="true"` and strictly additive — the outline is **never** removed.
  - **Canvases export as K's canonical outline**: an ordered list of nodes with id, type, geometry, color, payload, group membership, spatial relations, degree, and incident edges, plus unknown properties. The optional SVG under `--canvas-image` is again `aria-hidden` and additive. This is what keeps P clear of the *graph/Canvas information lacking a textual equivalent* auto-fail rule.
  - **Graph views** are not exportable as an image in v1 at all; `export html --graph` emits **G**'s textual adjacency listing.
  - **PDF** always ships a companion `<name>.txt` carrying the complete textual equivalent — alt text, diagram outlines, canvas outlines, table linearizations. The receipt states plainly whether the engine carried alt text and structure tags; P does **not** claim the PDF is a tagged, accessible PDF (6.3).
  - **HTML structural rules**, asserted in CI: `lang` present; heading levels never skip; every `img` has an `alt` attribute; every `figure` has a `figcaption` or `aria-label`; every table has a `<caption>` and `<th scope>`; task checkboxes are `disabled` inputs accompanied by a literal `[x]`/`[ ]` so state is not glyph-only; both `prefers-color-scheme` palettes meet WCAG AA contrast; the page is usable at 320 CSS px and at 200% text zoom.

---

## 4. Implementation Specification

### 4.1 Architecture placement

Three new crates, deliberately split so the network cannot reach the export path:

```
crates/mg-vault-import/     # planner, adapters, provenance, ledger. No network. Writes only via H.
  src/{plan,collision,provenance,ledger,slug,walk}.rs
  src/adapters/{obsidian,markdown_dir,notion,logseq}.rs
crates/mg-vault-export/     # pure, read-only, no network, no socket-capable dependency
  src/{select,markdown,html,pdf,assets,a11y,manifest}.rs
crates/mg-vault-publish/    # allowlist, projection, staging, gates, transport, credentials
  src/{allow,project,stage,gate,receipt}.rs
  src/transport/{directory,rsync_ssh,git_branch,https}.rs
```

Binding structural rules, each enforced by a test rather than a convention:

- **`mg-vault-export` has no write path.** It receives a `VaultReader` (a read-only capability wrapping **A**'s confined `VaultDir`) and does not depend on `mg_vault_core::transaction` or on any mutating `Vault` method. An API-surface golden test plus a `cargo metadata` dependency-direction test assert this, so "export never mutates source" is a compile-time property, not a promise.
- **`mg-vault-export` may not depend on `mg-vault-publish` or on any crate capable of opening a socket.** The dependency edge runs one way: `publish → export`.
- **P defines no HTML policy.** It imports `mg_vault_markdown::render::sanitize` and `HTML_ALLOWLIST_VERSION` from **F**; there is exactly one allowlist in the workspace, and P's writers accept only `SafeInline`/`SafeBlock`.
- **P defines no exclusion or secret policy.** It imports `ExclusionSet`, `exclusion_set()`, and `scan_bytes()` from **M** — the same evaluator M already documents as serving "staging, backup, and export".
- **P defines no transaction engine.** Import writes are `mg_vault_core::transaction` plans with `operation: Import`.
- **`mg-vault-cli`** gains `import`, `export`, and `publish` subtrees that own no policy. Today's `interop export` becomes `export snapshot`; `interop export` remains as a deprecated alias for two minor releases, emitting a stderr deprecation line while keeping its JSON byte-identical.
- **`mg-vault-core/src/interop.rs`** is moved behind the export crate and reworked per §7.2; core keeps only what is genuinely filesystem authority.

### 4.2 Data model

```rust
/// Version of the import/provenance contract. Bumping it is a breaking change.
pub const IMPORT_SCHEMA: &str = "mg-vault.import/1";
/// Version of the published-artifact profile (projection + link policy + a11y rules).
pub const PUBLISH_PROFILE: &str = "mg-vault.publish-props/1";

/// Where an imported note came from. Every field is written as an ordinary,
/// visible YAML property. Nothing here is hidden, and nothing here is identity:
/// `import_id` names a BATCH, is shared by every note in that run, and is never
/// a per-note identifier. File paths remain the only public note identity (1C).
pub struct ImportProvenance {
    pub import_schema: String,          // IMPORT_SCHEMA
    pub source_tool: SourceTool,        // Obsidian | MarkdownDir | Notion | Logseq
    pub source_tool_version: Option<String>, // only when the export declares one
    pub source_path: String,            // path RELATIVE to the source root, never absolute
    pub source_hash: String,            // sha256 of the EXACT source bytes
    pub source_bytes: u64,
    pub imported_at: Timestamp,         // RFC 3339 WITH offset, never naive
    pub imported_by: String,            // "mg-vault import 0.1.0"
    pub import_id: BatchId,             // batch handle; NOT note identity
    pub conversion: String,             // "verbatim-copy" | "notion-html/1" | "logseq-md/1"
    pub conversion_fidelity: Fidelity,  // shares L's enum; Verbatim is unconstructible on lossy paths
    pub source_original: Option<NotePath>, // retained original; REQUIRED when fidelity is Lossy
}

/// One classified source file. Every enumerated file gets exactly one of these.
pub struct PlannedFile {
    pub source: SourceRelPath,
    pub class: ImportClass,             // Create | Overwrite | Rename | Skip | Refuse | NotImported
    pub target: Option<NotePath>,
    pub source_hash: String,
    pub derived_hash: Option<String>,
    pub existing_hash: Option<String>,  // present for Overwrite/Skip/Refuse-on-collision
    pub decision_required: bool,        // true => apply refuses without an explicit choice
    pub reason: Option<RefusalReason>,
    pub provenance: Option<ImportProvenance>,
}

/// A construct the adapter recognized but could not carry. Never a silent drop.
pub struct LedgerEntry {
    pub construct: &'static str,        // "synced-block", "database-view", …
    pub disposition: Disposition,       // Carried | CarriedAsLiteral | Retained | NotRepresentable
    pub occurrences: u64,
    pub notes: Vec<NotePath>,
    pub retained_original: Option<NotePath>, // REQUIRED unless disposition is Carried
}

/// The complete, canonically serializable description of one import.
pub struct ImportPlan {
    pub schema: u16,                    // 1
    pub domain: &'static str,           // "mg-vault.import-plan"
    pub import_id: BatchId,
    pub source_kind: SourceTool,
    pub source_root_identity: RootFingerprint,
    pub vault_root_identity: VaultRootFingerprint,
    pub files: Vec<PlannedFile>,        // canonical order: source path, byte-ascending
    pub ledger: Vec<LedgerEntry>,
    pub batches: Vec<BatchSpec>,        // ordered; each becomes exactly one H transaction
    pub collision_policy: Option<CollisionPolicy>, // None => apply refuses
    pub confirmation: ConfirmationClass, // Ordinary | Escalated (any Overwrite => Escalated)
    pub expires_at: SystemTime,
    pub plan_hash: PlanHash,
}

/// P's thin phase record. File writes live inside H's transaction journal.
pub struct ImportJournal {
    pub version: u16, pub import_id: BatchId, pub plan_hash: PlanHash,
    pub batches: Vec<(BatchId, Option<TransactionId>, BatchPhase)>,
    pub applied: u32, pub total: u32,
}

// --- export ---

pub struct ExportRequest {
    pub selector: Selector,             // Notes | Dir | FromFile | Query | PublishAllowlist
    pub format: ExportFormat,           // Markdown | Html | Pdf | Snapshot
    pub out: Destination,
    pub html_mode: HtmlMode,            // default Sanitized for export (resolves F's Q4)
    pub outside_links: OutsideLinkMode, // ShowPath (local default) | Flatten (publish, forced)
    pub assets: AssetMode,              // Copy | Inline | Omit
    pub allow_stale: bool,
}

pub struct ArtifactRecord {
    pub artifact: PathBuf, pub bytes: u64, pub sha256: String,
    pub source_note: NotePath, pub source_fingerprint: SourceFingerprint,
    pub text_equivalent_bytes: u64,     // > 0 for every artifact carrying a diagram or canvas
    pub warnings: Vec<ExportWarning>,   // flattened links, blocked media, missing alt, …
}

// --- publish ---

/// Deny-by-default allowlist decision. Negations beat allows; M's exclusion beats both.
pub struct AllowDecision { pub allowed: bool, pub rule: Option<String>, pub line: Option<u32> }

pub struct StagedPublish {
    pub stage_id: StageId, pub profile: String, pub sanitizer: String,
    pub artifacts: Vec<ArtifactRecord>,
    pub classes: PublishClasses,        // new / changed / unchanged / removed
    pub refused: Vec<PublishRefusal>,   // gate id, path, layer, rule, redacted excerpt
    pub withheld_properties: Vec<(NotePath, Vec<String>)>, // KEYS only, never values
    pub flattened_links: u64, pub unexpanded_embeds: u64,
    pub manifest_digest: String,
}

/// Private satisfaction type. Constructible ONLY inside the `publish push` handler,
/// from an in-invocation typed confirmation or an explicit `--yes --stage-digest` pair.
/// No plugin, AI adapter, TUI background task, index response, timer, config value,
/// or deserialized JSON can mint one, and `transport::send` accepts nothing else.
pub struct PublishGrant { /* private fields */ }

/// A credential in memory. No Debug, no Clone, no Serialize; zeroized on drop.
pub struct Credential(Zeroizing<String>);
```

**On-disk layout.** Vault content the user owns: `mg-vault.publish-allow.txt`, `mg-vault.publish.yaml`, `_import/<id>/{IMPORT.md,manifest.json,originals/**}`. Control-directory state, `0700`/`0600`, all disposable: `.mg-vault/import/{ACTIVE,journal/<id>.json,receipts/<id>.json}` and `.mg-vault/publish/{state.json,staging/<id>/**,receipts/<id>.json}`.

**Migrations.** No note-content migration exists or may ever exist. New versioned on-disk schemas: import plan v1, import journal v1, import receipt v1, publish state v1, staging manifest v1, publish receipt v1. Each fails closed on an unknown newer version and is never rewritten by an older binary. `_import/**` and `_export/**` are added to **M**'s built-in exclusion set so retained originals never travel to a remote or into a publish by accident.

### 4.3 API contracts

```rust
// mg-vault-import — no network, writes only through H
pub fn plan_import(vault: &Vault, req: ImportRequest) -> Result<ImportPlan, ImportError>;
pub fn apply_import(vault: &Vault, plan: &ImportPlan, grant: ConfirmedImport)
    -> Result<ImportReceipt, ImportError>;
pub fn resume_import(vault: &Vault, id: BatchId) -> Result<ImportReceipt, ImportError>;
pub fn import_status(vault: &Vault) -> Result<Vec<ImportJournal>, ImportError>;

// mg-vault-export — read-only capability; no `Vault` write method is reachable
pub fn export(reader: &VaultReader, req: ExportRequest) -> Result<ExportReceipt, ExportError>;
pub fn export_snapshot(reader: &VaultReader) -> Result<Value, ExportError>; // today's interop, reworked

// mg-vault-publish
pub fn allow_decision(vault: &Vault, path: &NotePath) -> Result<AllowDecision, PublishError>;
pub fn stage(vault: &Vault, cfg: &PublishConfig) -> Result<StagedPublish, PublishError>;
pub fn review(vault: &Vault, id: StageId) -> Result<StagedPublish, PublishError>;
pub fn push(vault: &Vault, id: StageId, grant: PublishGrant)
    -> Result<PublishReceipt, PublishError>;
```

Binding contract rules:

- **One planner.** The dry-run report, `--plan-out`, the TUI review pane, and `--apply` all call `plan_import`. There is no second, weaker path. The same holds for `stage`/`review`/`push`.
- **A plan is not authorization, and a stage is not authorization.** `apply_import` revalidates every source digest, every target's existence and fingerprint, every parent chain, and confinement from freshly opened handles immediately before the first write. `push` re-runs every gate against live source. Drift returns `plan_mismatch` / `stage_drift` and writes or sends nothing.
- **`ConfirmedImport` and `PublishGrant` are private satisfaction types** constructible only from an in-invocation confirmation. A prior dry run cannot manufacture one.
- **Exit codes** reuse **C**'s stable categories with no additions: `0` success or no-op, `2` usage/input, `3` not found/selection, `4` collision/ambiguity/conflict (including `collision_policy_required`), `5` unsafe or denied (including `secret_detected`, `path_leak_detected`, `excluded_but_allowlisted`), `6` degraded dependency (`pdf_engine_unavailable`, `index_stale` without opt-in, `network_unavailable`), `7` I/O or transaction failure, `130` interrupt. Exit code and JSON error code are asserted together.
- **JSON `data` objects** (envelope `{version:1, ok, data, warnings}` from **C**): `import.plan` → `{schema, import_id, dry_run:true, changed:false, counts, files:[…], ledger:[…], batches, decisions_required:[…]}`; `import.apply` → `{dry_run:false, plan_hash, batches_applied, batches_total, created, overwritten, renamed, skipped, trash_receipts:[…], durability:"synced", changed:true}`; `export.*` → `{format, artifacts:[…], bytes, source_notes_modified:0, warnings:[…]}`; `publish.stage` → the `StagedPublish` fields; `publish.push` → `{destination, redacted_url, delivered, skipped_unchanged, orphaned, partial:false, receipt_id}`. Arrays always appear even when empty, with documented stable ordering.
- **Pagination.** `import --list-skipped` and `publish review --list-unmatched` accept `--limit`/`--after` with an explicit `N more` line. **The refused/decision-required sections are never paginated and never elided** — a user must see every item that needs a decision.
- **Auth and rate limiting.** N/A for import and export, which are local. For `publish push`, authentication is entirely the transport's (SSH agent, git credential helper, or the external secret command) and mg-vault stores no credential; the only rate limit is that P issues requests for one explicit `push` invocation and never retries automatically after a `4xx`-class rejection.

### 4.4 State management

- **Owner.** `mg-vault-import` owns the plan and its journal; `mg-vault-export` owns **no durable state at all**; `mg-vault-publish` owns the staging area, the digest map, and receipts. The CLI owns presentation; the TUI panes own selection and pending decisions only.
- **Authoritative state is the files.** Notes, attachments, `_import/**` originals, `mg-vault.publish-allow.txt`, and `mg-vault.publish.yaml` are ordinary files a user can edit, sync, diff, grep, and delete with no tool involvement. The allowlist and config deliberately live at the **vault root**, not under `.mg-vault`, so they travel with a synced vault and are visible in `git diff` — the same reasoning **I** applies to `mg-vault.schema.yaml`.
- **Everything under `.mg-vault/{import,publish}/` is disposable.** Deleting `.mg-vault/publish/state.json` costs only the ability to say what changed since the last publish; the next stage simply reports everything as `new`. Deleting a staging directory costs only that stage. No P state is ever authority over a note.
- **Local vs. remote boundary.** The vault is local and authoritative. The publish destination is a **projection** — write-only, never read back as content. P never pulls from a destination, never reconciles it into the vault, and never lets a destination's state change a note. `publish status` reads only the local digest map, and drift at the destination is reported, never repaired by writing to the vault.
- **Offline / draft persistence.** An import plan can be spooled (`--plan-out`) and applied later, subject to re-verification. A stage persists until pruned (`--keep N`, default 5) and can be reviewed indefinitely offline. Interrupted work is resumable: `import resume` continues at a batch boundary, and `publish push` re-run skips already-delivered artifacts by digest.
- **Interaction with H and M.** P holds no lock of its own: it obeys **H**'s `ACTIVE` marker and refuses while a transaction is unresolved, and runs **H**'s recovery first, exactly as **M** does.

### 4.5 Dependencies

- **New Rust crates.** `globset` (allowlist matching — already introduced by **M**), `zip` (Notion export archives, so `--from-zip` need not shell out), `html5ever`/`markup5ever` (Notion HTML parsing; **F** already needs an HTML tokenizer for its sanitizer and the two share it), `zeroize` (credential handling), and one HTTPS client **only if** the `https` transport ships (see Q4 — the proposed v1 has zero in-process sockets). `sha2`, `serde`, `serde_json`, `thiserror`, `clap`, and `rustix` are already present. No async runtime is introduced for import or export.
- **New workspace dependencies P consumes rather than adds:** `mg-vault-markdown` (**F**), `mg-vault-sync` (**M**), `mg-vault-core::transaction` (**H**), and optionally `mg-vault-index` (**B**, read-only, for `--query` selectors).
- **External tools, invoked as separate processes and never linked:** `typst` (Apache-2.0) or `weasyprint` (BSD-3-Clause) for PDF, both behind the `pdf` feature and both probed at runtime; `rsync` and `ssh` for the `rsync-ssh` transport; `git` for the `git-branch` transport, which delegates to **M**'s existing allowlisted wrapper and push path so there is exactly one push implementation in the product. Each is probed, version-recorded, and reported by `mg-vault capabilities`.
- **New assets.** Fixture corpora under `crates/mg-vault-import/fixtures/` and `crates/mg-vault-export/fixtures/`: a project-authored Obsidian vault with plugin syntax and a populated `.obsidian`; a plain Markdown tree; a **project-authored** Notion export (no real Notion export is vendored); a project-authored Logseq graph; expected-artifact goldens; and the secret-bearing publish fixture. The exported HTML stylesheet is project-authored and ships **no web font** — a system font stack, which is both a licensing and a privacy decision.
- **Infrastructure changes.** None. P adds no database, no service, no CDN, and no third-party account. The publish destination is whatever the user already owns.

### 4.6 Platform-specific considerations

- **Arch Linux is first support.** Import, export, and staging are platform-independent pure Rust; only PDF, `rsync-ssh`, and `git-branch` depend on external executables, and each degrades explicitly (2E).
- **Confinement.** Both the vault side and the **source** side use **A**'s descriptor-relative `VaultDir` backend (`openat2` with `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS | RESOLVE_NO_MAGICLINKS` on Linux; `openat` + `O_NOFOLLOW` + `(dev, ino)` pinning on macOS). Where the hardened backend is unavailable, import falls back to **A**'s canonicalize-and-recheck boundary and says so in the report; it does not silently claim the stronger guarantee.
- **Case-insensitive filesystems.** Target-path checks use case-folded comparison in addition to exact comparison; two source files whose targets differ only by case refuse rather than collapsing onto one.
- **Unicode normalization.** Identity is the exact byte path; nothing is NFC/NFD-normalized. A source path that normalizes onto an existing target on macOS refuses, and the report names both byte sequences.
- **Line endings.** Import never translates CRLF↔LF. Preserving bytes is the point; a translated file would break fingerprint equality and 1B preservation.
- **Feature flags / rollout.** `pdf` (compile-time, off by default), `https-transport` (compile-time, off in v1 per Q4). Safety behavior, gate order, JSON shape, and error codes never vary by build; only availability does, and `mg-vault capabilities` reports it truthfully.
- **Version compatibility.** `IMPORT_SCHEMA` and `PUBLISH_PROFILE` are echoed in every response. Changing an adapter's mapping, the property projection list, the link policy, or the a11y rules bumps the profile; it **never** rewrites an existing note or an already-published artifact.

### 4.7 Performance budget

Reference: warm local SSD, the 100,000-note / 1,000,000-block fixture from **B**, reported with hardware/OS metadata.

- **Memory.** ≤ 300 MiB RSS for any P command. The import plan **streams**: `PlannedFile` records spool to an owner-only temporary file rather than living in RAM, and the report pages from it. Export streams per note. Staging streams per artifact. No P command loads a whole vault into memory. *(Today's `export_snapshot` does exactly that — see §7.1.)*
- **Import plan** over 100,000 source files: p95 ≤ 45 s, dominated by SHA-256 (≥ 400 MiB/s streaming, single-threaded; `--jobs N` parallelizes hashing only, never writes). A 5,000-file plan: p95 ≤ 3 s.
- **Import apply:** ≥ 25 MiB/s including staging, journaling, and `fsync`; a 2,000-file batch p95 ≤ 6 s. Startup impact is one additional `stat` of `.mg-vault/import/ACTIVE`, target ≤ 1 ms, alongside **H**'s.
- **Export:** Markdown ≥ 40 MiB/s; HTML ≥ 5 MiB/s including parse and sanitization; 1,000 notes to HTML p95 ≤ 20 s. PDF is bounded by the external engine and is reported separately so a slow renderer is never attributed to mg-vault.
- **Publish stage:** export throughput plus **M**'s scan (≥ 40 MiB/s); 500 artifacts p95 ≤ 12 s. `publish status` p95 ≤ 300 ms (digest map plus a confined stat walk).
- **Network payload:** exactly the changed and new artifacts. Unchanged digests are not re-uploaded. P adds no telemetry, no beacon, and no background request; total bytes are printed before the confirmation.
- **Storage:** `_import/<id>/originals/` is bounded by the source size and is deletable by the user at any time with no effect on the notes; staging is bounded by `--keep N` (default 5). Nothing grows without an explicit user action.
- **Cancellation:** `Ctrl-C` during any plan, export, stage, or review is safe — nothing has been written to the vault. During `import apply` it aborts at the next batch boundary and leaves a recoverable journal. During `push` it stops after the current artifact and reports `partial`.

---

## 5. Test Specification

### 5.1 Unit tests

| Name | Setup → assertion | Edge case covered |
|---|---|---|
| `collision_policy_matrix` | Each of `{skip,rename,overwrite,fail,ask}` × `{identical,different}` existing bytes → exact class, exact write count | Identical bytes are `skip`, never `overwrite` |
| `no_input_refuses_without_policy` | Collision present, `--no-input`, no `--on-collision` → `collision_policy_required`, exit 4, zero writes | **The auto-fail rule: no assumed default** |
| `overwrite_requires_two_signals` | `--on-collision overwrite` without `--overwrite-existing`, and with it but without confirmation → refused both times | Unconfirmed overwrite |
| `overwrite_trashes_preimage_first` | Apply an overwrite → trash receipt exists and restores the pre-image byte-for-byte | 4D recoverability |
| `ledger_is_set_equal_to_source_constructs` | For every fixture: adapter-recognized construct set == `carried ∪ retained ∪ reported` | **Source-content loss; an unmodelled construct fails the build** |
| `lossy_import_forces_originals` | `notion` + `--no-keep-originals` → usage error, exit 2 | Cannot import lossily with no original |
| `provenance_roundtrip` | `ImportProvenance` → YAML → parse → equal; `imported_at` always carries an offset | Naive timestamp |
| `provenance_insert_preserves_all_other_bytes` | Insert provenance into 200 adversarial-YAML fixtures → prefix/suffix/gap equality outside the inserted span | 1B preservation |
| `provenance_key_conflict_refuses` | Source already defines `source_path` → `refuse`, both values shown, zero writes | No silent clobber |
| `import_id_is_not_note_identity` | Two notes from one batch share `import_id`; no per-note UUID exists in any output | **1C: no hidden identity injection** |
| `notion_desuffix_collision_refuses_both` | Two pages de-suffixing to one path → both refused with both originals | 3B: never picks a winner |
| `slug_never_escapes` | Property test, 100,000 fuzzed source titles/paths → every target passes `validate_note_path` and canonicalizes beneath the root | Traversal / symlink escape |
| `symlinks_are_reported_not_followed` | Symlink inside the source (file and dir, in-root and escaping) → `not-imported (symlink)` with target, never traversed | Silent skip is itself a defect |
| `export_html_allowlist_is_deny_by_default` | Every element/attribute/scheme in **F**'s hostile corpus → output subset of the allowlist; zero active constructs | **Active raw HTML/script by default** |
| `export_has_no_off_artifact_subresource` | Every exported page → all `img`/`link` sources are `self` or `data:`; zero remote subresources | 4F reader-privacy |
| `export_csp_and_rel_present` | Every page → CSP meta exact; every anchor has `rel`, no `target` | Defense in depth |
| `outside_links_flatten_under_publish` | Publish-profile export with links to unpublished notes → link text only, no path anywhere in the bytes | Path leak |
| `ambiguous_link_never_resolved` | **G** `Ambiguous` → `[ambiguous]` marker plus ordered candidates | 3B |
| `alt_text_carried_and_never_synthesized` | Images with and without alt → carried verbatim; missing → literal `description: unavailable`, never the filename | 5C |
| `diagram_and_canvas_always_have_outline` | Every mermaid and `.canvas` fixture, with and without the image feature → node/edge outline present in the artifact bytes | **Graph/Canvas textual-equivalent auto-fail** |
| `pdf_unavailable_is_explicit` | No engine on PATH → exit 6, named tool, install hint, zero bytes written | 2E, 4E |
| `allowlist_negation_and_precedence` | `!private/**` beats an allow; **M** built-in exclusion beats an allow **and** a negation | **G2: exclusion wins, always** |
| `unlisted_note_is_never_staged` | 1,000-note fixture, 12-line allowlist → exactly the matched set staged | Deny by default |
| `embed_of_unpublished_note_not_expanded` | Allowlisted note embeds a private note → body bytes absent from the artifact | **G4 exfiltration path** |
| `withheld_properties_never_leak_values` | Serialize `StagedPublish` → withheld values appear nowhere; only keys | 4F |
| `credential_type_has_no_debug_or_serialize` | `trybuild` compile-fail on `{:?}` and `serde::to_string` of `Credential` | Credential in a log |
| `credential_command_failure_refuses` | Secret command missing / non-zero / empty → `credential_unavailable`, zero requests | Never falls back to unauthenticated |
| `redacted_url_strips_userinfo` | `https://user:token@host/p`, `ssh://git@host/p` → rendered without credentials in review, receipt, error, and log | Credential leak |
| `publish_grant_is_unminted_elsewhere` | `trybuild` compile-fail constructing `PublishGrant` outside the push handler | 4C / capability bypass |
| `plan_hash_binds_executable_fields_only` | Reorder warnings and change timestamps → hash stable; change one target path or policy → hash changes | Plan integrity |
| `ansi_stripping_preserves_meaning` | Strip ANSI from every P view → semantically complete | 5D color independence |
| `width_matrix_never_truncates` | 40/60/80/120 columns → every path, digest, rule, and command present in full | 5D |

### 5.2 Integration tests

- **`export_never_mutates_source`** — the load-bearing one. Take a whole-vault manifest (path set plus per-file SHA-256 plus `.mg-vault` entry set); run every export format over every selector on the full corpus; re-take the manifest; assert **equality**. Repeat with the destination inside the vault (must refuse) and with a read-only vault mount (must still export).
- **`import_dry_run_writes_nothing`** — same manifest technique across every source kind, every collision policy, and every fixture; assert equality, and additionally assert `.mg-vault` gained no journal, no staging object, and no `ACTIVE` marker.
- **`import_crash_matrix`** — kill the process at every phase of every batch of a 40,000-file apply; assert the next invocation converges every batch to complete-pre or complete-post, that no file is half-written, that `import status` names the resume point, and that `import resume` completes without duplicating a write.
- **`obsidian_in_place_is_zero_writes`** — run `import obsidian --in-place` on a populated Obsidian vault; assert the manifest is unchanged, `.obsidian` is byte-identical, and the audit report is non-empty. Then open the vault in Obsidian and confirm nothing moved (§5.4).
- **`notion_lossy_keeps_originals_and_ledger`** — every derived note that differs from its source names a retained original that exists and hashes to the source; every ledger entry with a non-`Carried` disposition names a retained original.
- **`round_trip_markdown_is_byte_identical`** — `import markdown-dir` with `--provenance sidecar`, then `export markdown --fidelity verbatim`; assert every exported file is byte-identical to its source file.
- **`publish_excluded_never_leaves`** — plant a `deny` secret in an allowlisted note and a second in an allowlisted-but-excluded note; run `stage`, `review`, and an attempted `push` against a recording fake destination; assert the publish refuses and that neither secret's bytes appear in any staging file, manifest, receipt, log line, error message, or byte delivered.
- **`stage_is_not_authorization`** — stage, then modify a note / remove it from the allowlist / plant a secret; assert `push` refuses with `stage_drift` and sends nothing.
- **`push_is_the_only_socket`** — run the entire P suite under a `connect(2)`-denying harness; assert every command except `publish push` completes normally, and that `push` itself fails cleanly with `network_unavailable` rather than hanging or partially claiming success.
- **`publish_idempotence_and_partial_truth`** — push twice with no changes (`changed:false`, zero requests); then interrupt a push mid-way and assert the receipt reports `partial: N of M` and that a re-push delivers exactly the remainder.
- **`degraded_without_index`** — with **B** absent, assert path and directory selectors work, `--query` returns exit 6 with the freshness word, and no artifact is produced from stale data unless `--allow-stale` labels it.

### 5.3 UI / E2E tests

- **CLI transcript goldens** for the import plan report, per-collision prompt, export receipt, staging review, and push confirmation, captured at 40, 60, 80, and 120 columns, with and without color, and compared byte-for-byte.
- **`--json` envelope goldens** for every P command, success and failure, with exit code and JSON error code asserted together.
- **Navigation and happy path (TUI):** open the import review pane from the palette, set a per-row decision, set a class decision, apply through the escalated overlay, and assert the writes match the plan exactly.
- **Error recovery (TUI):** trigger a collision with no policy, a secret finding, and a stage drift; assert each renders as a blocking record with a stated recovery, and that no pane state allows proceeding past a refusal.
- **Screen-reader run:** replay every scenario in **E**'s screen-reader mode through **D**'s transcript adapter; assert announcement order matches §3.7 and that each state change emits exactly one announcement.
- **Automation run:** every command under `--no-input --json --no-color` with a non-TTY stdout; assert no prompt, no progress counter, no `/dev/tty` read, and a typed error wherever a decision is required.

### 5.4 Visual / manual verification

- **Exported HTML in a browser:** light and dark via `prefers-color-scheme`; 320 px viewport and 2560 px; 200% text zoom; a page containing an image with alt text, an image without, a mermaid diagram, a canvas outline, a wide table, and a task list. Confirm the diagram and canvas are readable as text with the image feature off, and that the SVG adds nothing when it is on.
- **Offline check:** load an exported page from `file://` with the network interface down; confirm complete rendering and, in the browser devtools network panel, **zero requests**.
- **Screen reader:** Orca + Firefox over an exported page; confirm the figure labels, table caption, heading order, and checkbox states announce correctly.
- **PDF:** open in three viewers; confirm the outline/bookmarks from headings, the `/Lang`, and the presence and completeness of the companion `.txt`.
- **Terminal:** import report and staging review at 40, 60, 80, and 120 columns; `NO_COLOR=1`; a 24-row terminal; a hostile source filename containing ESC and RTL override bytes (confirm escaping, no cursor movement).
- **Coexistence:** after an `--in-place` Obsidian import and after a copy-mode import, open both vaults in Obsidian; confirm links resolve, frontmatter renders as ordinary properties, `.obsidian` is untouched, and nothing in a note body is active.
- **Empty vs. populated:** every view with zero items, one item, and 10,000 items.

---

## 6. Compliance & Safety Gate

### 6.1 Sensitive data classification

- [x] **Handles sensitive data — describe protection measures.**

P touches the most sensitive material in the product and is the only branch that deliberately copies it off the machine. In scope: complete note bodies (in artifacts, staging, and uploads), frontmatter that commonly holds client names, health details, and identifiers; local filesystem paths and the OS username; destination URLs that may embed credentials; and a hosting credential itself.

Protections, each with a test in §5: everything under `.mg-vault/{import,publish}/` is `0700`/`0600`; **publishing is deny-by-default** and requires a user-authored allowlist file; **M**'s exclusion evaluator overrides the allowlist and its built-in layer is not overridable; **M**'s secret scanner runs over both source and artifact and a `deny` finding refuses the whole publish; a positive-list property projection means an unanticipated frontmatter key is withheld by default rather than shipped; embeds of unpublished notes are never expanded; links to unpublished notes are flattened so their paths do not appear; import provenance that can carry an absolute path is stripped from artifacts; a path-leak audit scans staged bytes for the vault root, `$HOME`, the username, and unpublished paths; the credential is resolved from an external command into a `Zeroizing` type with no `Debug`, `Clone`, or `Serialize`, held only for the request, and never written to config, log, receipt, manifest, or error; destination URLs are redacted (userinfo stripped) everywhere; and a socket-denial harness proves that no command other than `publish push` connects. Erasure is user-controlled: `_import/**` originals, staging directories, and `.mg-vault/publish/state.json` are all deletable with no effect on notes.

- [ ] No sensitive data involvement
- [x] **Uses synthetic/test data only until compliance gate clears** — every fixture (Obsidian, Markdown, Notion, Logseq, hostile HTML, secret corpus, 100k-scale corpus) is generated or project-authored; no personal vault and no real exported account data is required for acceptance.

### 6.2 Asset provenance

- [x] **Uses third-party assets — listed with source, license, and rights status.**

| Asset | Source | License / rights |
|---|---|---|
| Obsidian, Markdown, Notion, and Logseq fixture corpora | **Project-authored.** No real export from any account is vendored; each fixture is written from the documented format and cites the product version it mirrors in its header | Project license (blocked on the MIT-vs-Apache-2.0 decision before distribution) |
| Hostile-HTML corpus | Shared with **F** and **L**; project-authored or public security test vectors with each vector's origin and license recorded in `fixtures/PROVENANCE.md` | Recorded per vector |
| `typst` (Apache-2.0), `weasyprint` (BSD-3-Clause), `rsync`, `ssh`, `git` | Invoked as **separate processes, never linked**, so no license obligation propagates to `mg-vault`; the reasoning is recorded in `docs/` beside the dependency table | Compatible |
| Exported HTML stylesheet | Project-authored; **no web font**, system font stack only | Project license |
| New Rust crates (§4.5) | crates.io | All MIT/Apache-2.0; verified before merge |

### 6.3 Language / claims audit

- [ ] Makes claims not supported by evidence — **no.** "Sanitized" is scoped to **F**'s versioned allowlist and proven by the hostile corpus; it never implies safety against a malicious kernel or root process. "Verbatim" is regulated by the data model: `Fidelity::Verbatim` is unconstructible on the Notion and Logseq paths, so the product cannot label a lossy conversion verbatim even by mistake. "Published" is never claimed before the transport confirms delivery; an interrupted push reports `partial: N of M`.
- [ ] Promises capabilities not yet built — **no.** §7 states plainly that this entire branch is absent, that `pdf` and `https-transport` are compile-time gated and off by default, and that PDF requires an external tool that may not exist. **P does not claim to produce a tagged, accessible PDF**; it claims a companion textual equivalent and reports truthfully whether the engine carried alt text.
- [ ] Uses language restricted by domain regulations — **no.** P makes no medical, financial, legal, or safety claim. "Secure" is not used as a bare adjective anywhere in user-visible text; each guarantee is stated as the specific mechanism that enforces it.

### 6.4 Regulatory alignment

**Lens 3 — Knowledge Retrieval and Structure**, addressed criterion by criterion as the template requires:

- **3A Determinism.** Every P output derives from source bytes plus a versioned profile. The slug algorithm, the collision classifier, the adapter mappings, the property projection, the link policy, and the sanitizer allowlist are all versioned and fixture-pinned; the same input produces the same plan, the same artifacts, and the same `plan_hash`. Enumeration and report ordering are canonical (source path, byte-ascending). Deleting every cache and re-running reproduces byte-identical artifacts.
- **3B Ambiguity.** P never silently chooses. A collision with differing bytes and no policy refuses; a Notion de-suffix collision refuses both files; two targets differing only in case refuse; an **G**-ambiguous link is flattened with `[ambiguous]` and its ordered candidates listed rather than resolved; a source-kind detection mismatch warns and never overrides. No P path mutates a note to disambiguate it.
- **3C Query depth.** P's own contribution is to keep imported material *queryable by ordinary means*: `source_tool`, `source_path`, `source_hash`, `imported_at`, and `conversion_fidelity` are ordinary properties that **B** indexes and **C**/**I** query with existing predicates, so "show me everything I imported from Notion that was converted lossily" is an existing query, not a new subsystem. P adds no private query path. Selection reuses **G**/**I** query syntax rather than inventing one.
- **3D Derived authority.** Every artifact P produces is **downstream and disposable**. An exported page, a staged artifact, a published destination, and `.mg-vault/publish/state.json` are all views over ordinary files; none can be edited into authority, and P never reads a destination back into the vault. The one place a derived store could gain authority — using **B**'s index as the selection set — is constrained: an index may narrow a selection but never supplies note bytes, and a non-`current` index refuses rather than silently exporting stale content.
- **3E Scale.** Budgets in §4.7 address the 100,000-note fixture explicitly: the import plan streams to a spool rather than into RAM, export and staging stream per note, memory is capped at 300 MiB for any command, and long operations report a static counter and are cancellable at a safe boundary. Nothing in P blocks editing: import holds no lock beyond **H**'s existing `ACTIVE` marker, and export and staging are read-only.

**Other lenses, in brief.** *1A* — artifacts and destinations are disposable; ordinary files stay the source of truth. *1B* — provenance is inserted through **I**'s span-local CST edit; no import reserializes a file, and unknown syntax is carried verbatim. *1C* — `import_id` is a batch handle shared across notes, never a per-note identifier; paths remain identity. *1D* — `--in-place` Obsidian import writes nothing at all, `.obsidian` is read but never written or copied into a target, and P's own settings live under `.mg-vault` while user-owned policy files live as vault content. *1E* — every collision, ambiguity, and drift fails closed; every write is an **H** transaction; recovery is digest-driven. *2A/2E* — every P workflow is reachable from the palette without a chord, and every unavailable capability is explicit. *4A* — both the vault and the source root are confined; symlinks are refused and reported. *4C* — `ConfirmedImport` and `PublishGrant` are private satisfaction types. *4D* — every overwrite is trash-backed and previewed. *4E* — `mg-vault.import/1`, `mg-vault.publish-props/1`, and the JSON envelopes are versioned and truthful. *4F* — §6.1. *5A* — everything but `push` is offline, and exported pages render offline with zero requests. *5D/5E* — width ladder, literal state words, `--no-input`, `--no-color`, `NO_COLOR`, stable JSON.

**Auto-fail review.**

| Rule | Why P is clear |
|---|---|
| Source-content loss | The three-tier rule with a set-equality ledger test; lossy conversions force retained originals; unknown syntax is carried verbatim as **F**'s `Unrecognized` |
| Partial multi-file mutation | Every write is an **H** transaction; batching is disclosed **before** confirmation, each batch is atomic, and the journal resolves to complete-pre or complete-post per batch |
| Index state overriding source | An index may narrow a selection; it never supplies bytes and never adds a note |
| Stale index presented as current | `--query` refuses on non-`current` freshness; `--allow-stale` labels every artifact and the receipt |
| Silent conflict winner | No collision, de-suffix collision, case collision, or ambiguous link is ever resolved by the tool |
| Unknown syntax loss | Carried verbatim; **F**'s total token cover guarantees literal rendering; ledger-reported as `unsupported-at-render`, which is explicitly not loss |
| **Unconfirmed overwrite/import** | Dry run is the default; `--no-input` refuses without an explicit policy; overwrite needs `--overwrite-existing` **and** a typed confirmation **and** trashes the pre-image first |
| Unsafe traversal or symlink escape | **A**'s `VaultDir` confinement on both roots; symlinks refused and reported; slug property test over 100,000 fuzzed inputs |
| **Capability / data-exfiltration bypass** | `PublishGrant` is unmintable outside the push handler; deny-by-default allowlist; **M**'s exclusion overrides it; secret scan; embed closure; property projection; path-leak audit; socket-denial harness |
| **Active raw HTML/script by default** | **F**'s deny-by-default allowlist is the only policy, enforced by `SafeInline`/`SafeBlock` constructors; plus CSP, forced `rel`, and no off-artifact subresource |
| Non-atomic save claiming success | Import prints a success verb only after **H**'s terminal record is durable; push reports `partial` rather than success on interruption |
| Recovery overwriting newer source | Import recovery is **H**'s: it refuses any file matching neither pre- nor post-image and quarantines instead of writing |
| **Graph/Canvas information lacking a textual equivalent** | Canvas exports as **K**'s canonical outline and mermaid as **F**'s node/edge outline; any image is `aria-hidden` and additive; a coverage test asserts the outline is present in every artifact |

---

## 7. Gap Analysis vs. Current State

### 7.1 What exists today

- **`interop::export_snapshot` — implemented** (`crates/mg-vault-core/src/interop.rs`, 557 lines, 8 tests) and surfaced as `mg-vault interop export` in `crates/mg-vault-cli/src/main.rs`. It produces a deterministic read-only JSON snapshot (`mg.interop/1`) with vault-namespaced global ids, per-note records carrying path/title/text/wikilinks/fingerprint, wikilink edges with stable occurrence-bound ids, a `degraded` flag, and sorted diagnostics. It never writes the vault. This is the only piece of P's target surface that exists.
- **Foundation P will build on — implemented:** `Vault` confinement and `validate_note_path` (rejects absolute paths, `..`, non-`.md`, and `.obsidian`/`.mg-vault` first components on mutation); `create_note` with `renameat2(RENAME_NOREPLACE)` collision refusal; `write_note`/`edit_note_span` with SHA-256 fingerprint preconditions; `replace_atomic`/`create_atomic` with temp-file + `fsync` + parent-`fsync` discipline; vault-local trash and collision-refusing restore; `scan_frontmatter` and `locate_frontmatter_scalar`; the version-1 JSON envelope with `--json`, `--no-input`, `--no-color`, and `NO_COLOR`.
- **Accurate limitations of today's `export_snapshot`, all of which P must fix rather than inherit:**
  - It applies a **hardcoded, undocumented, non-configurable, non-reported** directory skip list — `.mg-vault`, `.obsidian`, `private`, `.private`, `secrets`, `.secrets` (`collect_paths`). Notes under those directories are omitted from the snapshot **with no diagnostic and no count**, which is precisely the silent omission P forbids. It is also *not* **M**'s exclusion evaluator, so the product would otherwise have two divergent exclusion policies.
  - It **neither follows nor reports symlinks**: `fs::DirEntry::file_type` does not follow links, so `is_dir()`/`is_file()` are both false for a symlink and the entry falls through the loop silently. There is no escape (good), but there is also no diagnostic (not good).
  - It walks with `fs::read_dir` rather than the descriptor-relative confined reader that already exists on the index path (`index.rs::read_source_bytes`), so the export path is the weaker of the two backends in-tree.
  - `created_at` and `observed_at` are hardcoded to the Unix epoch, with an honest in-code comment explaining that no trustworthy creation time exists — truthful, but P's receipts need real timestamps with offsets.
  - It builds `records` holding **every note's full text in memory** before serializing, so it is O(vault bytes) RSS and does not meet §4.7's 300 MiB cap at the 100,000-note scale.
- **Absent — everything else in this spec.** There is no `import`, `export`, or `publish` command in any form; no `mg-vault-import`, `mg-vault-export`, or `mg-vault-publish` crate; no source adapter for Obsidian, Markdown directories, Notion, or Logseq; no plan, collision classifier, decision prompt, loss ledger, retained-originals mechanism, or `_import/` layout; no `ImportProvenance` type and no provenance property anywhere; no batching, import journal, `import status`, or `import resume`; no selector grammar; no Markdown, HTML, or PDF renderer or writer; no artifact manifest; no allowlist file, evaluator, or `publish allow` command; no staging area, gate chain, review view, receipt, transport adapter, credential handling, or `PublishGrant`; no TUI pane of any kind; and no fixture corpus. `Cargo.toml` contains **no network dependency, no HTML parser, no archive crate, and no glob crate**, which is the honest starting point. `README.md` states plainly that there is "no broad import pipeline" and "no publishing workflow", and `docs/PRODUCT.md` lists import/export as target state.
- **Absent upstream that P hard-depends on:** the **F** markdown/sanitizer crate, **G**'s resolver, **H**'s transaction engine, **I**'s CST property model, **K**'s canvas outline, and **M**'s `ExclusionSet`/`scan_bytes` are all specified and all currently absent.

### 7.2 Delta to spec

**New crates and modules**

- `crates/mg-vault-import/` — `plan.rs`, `walk.rs` (confined source enumeration), `collision.rs`, `slug.rs`, `provenance.rs`, `ledger.rs`, `journal.rs`, `resume.rs`, `adapters/{obsidian,markdown_dir,notion,logseq}.rs`.
- `crates/mg-vault-export/` — `select.rs`, `markdown.rs`, `html.rs` (consumes **F**'s sanitizer only), `pdf.rs` (feature-gated), `assets.rs`, `a11y.rs` (alt/diagram/canvas equivalents), `manifest.rs`, plus the reworked `snapshot.rs`.
- `crates/mg-vault-publish/` — `allow.rs`, `project.rs`, `stage.rs`, `gate.rs` (G1–G8), `credential.rs`, `receipt.rs`, `transport/{directory,rsync_ssh,git_branch,https}.rs`.
- Tests: `crates/mg-vault-import/tests/{plan,collision,provenance,ledger,adapters,crash_matrix,confinement}.rs`; `crates/mg-vault-export/tests/{no_mutation,sanitizer,links,a11y,pdf_gate}.rs`; `crates/mg-vault-publish/tests/{allowlist,gates,secrets,credential,offline,transport}.rs`.
- Fixtures: `tests/fixtures/interop/{obsidian,markdown,notion,logseq,publish-secrets}/` plus expected-artifact goldens.

**Modified files**

- `crates/mg-vault-core/src/interop.rs` — **move** to `mg-vault-export`, replace the hardcoded skip list with **M**'s `ExclusionSet` (reporting every excluded path with its deciding layer and rule), add a symlink diagnostic, switch to the descriptor-relative confined reader, stream records instead of buffering them, and emit real RFC 3339 timestamps with offsets. Keep the output schema `mg.interop/1` byte-compatible for existing consumers, adding only new fields.
- `crates/mg-vault-core/src/lib.rs` — stop re-exporting `export_snapshot`; add the `VaultReader` read-only capability.
- `crates/mg-vault-core/src/vault.rs` — add the attachment write path (today `validate_note_path` requires `.md`, so **no non-Markdown file can be imported at all**), gated to an extension allowlist and routed through the same confinement.
- `crates/mg-vault-cli/src/main.rs` — register the `import`, `export`, and `publish` subtrees; deprecate `interop export` to `export snapshot`; add the `.mg-vault/import/ACTIVE` stat beside **H**'s.
- `crates/mg-vault-core/src/error.rs` — add the P error variants in §3.6 and map them to **C**'s exit categories.
- `Cargo.toml` — three new workspace members and the §4.5 dependencies, with `pdf` and `https-transport` as default-off features.
- `README.md`, `docs/PRODUCT.md`, `docs/ARCHITECTURE.md`, `docs/SECURITY.md` — record the dry-run-by-default import rule, the export no-mutation guarantee, the publish gate chain, the explicit-network rule, and the credential policy — **only once each lands**.

**Migrations / schema changes** — six new versioned on-disk schemas (§4.2), all failing closed on an unknown newer version. `_import/**` and `_export/**` added to **M**'s built-in exclusion set. No note-content migration, ever.

### 7.3 Estimated scope

**XL.** The command surface is wide, but the correctness surface is wider and three separate auto-fail rules run straight through it. Four source adapters each need a documented, fixture-pinned mapping and a provable no-loss ledger; the import path needs a collision model with no assumed default, a disclosed batching contract layered on **H**, and a crash matrix crossed with **H**'s own; the export path needs a structurally enforced no-write guarantee, a security-critical HTML boundary that must be **F**'s and only **F**'s, and a complete accessibility contract for constructs that have no raster path; and publishing needs a deny-by-default allowlist that loses to **M**'s exclusion set, an eight-gate chain, credential handling with no on-disk footprint, and a capability type no other subsystem can mint. None of it can be validated by ordinary unit tests, and P also depends on six upstream branches that are themselves largely unbuilt.

Recommended gated increments, each behind the same contracts: **P1** `export markdown` + selectors + the whole-vault no-mutation manifest gate + the reworked snapshot (independently valuable, zero new risk, and it fixes today's silent skip list); **P2** the import planner + `obsidian`/`markdown-dir` adapters + provenance + the dry-run report, **plan only, no apply** (this is the security-critical slice and can be fuzzed before it can write); **P3** `import --apply` on **H** + batching + journal + resume + the crash matrix; **P4** `export html` on **F**'s sanitizer + link policy + the a11y contract + the hostile corpus gate; **P5** the `notion` and `logseq` adapters + the loss ledger's full construct set; **P6** the allowlist + `publish stage`/`review` with the `directory` transport only — **still zero sockets**; **P7** `publish push` with `rsync-ssh` and `git-branch`; **P8** `export pdf` behind its feature. P2 must pass the traversal property test and the ledger set-equality gate before P3 begins, and P6 must pass the full gate chain and secret corpus before P7 opens anything.

### 7.4 Blocking dependencies

- **H Safe note refactoring — required, absent.** Every import write is an **H** transaction. P3 and everything after it are blocked on H1 (the engine, journal, and recovery). P1, P2, P4, and P6 are not blocked, because they write no vault byte.
- **F Markdown and rich content — required, absent.** P4 (HTML export) is blocked on F3 (render model + sanitizer). P must not implement an HTML policy; the allowlist version is shared. P5's ledger also relies on F's total token cover to guarantee that unrecognized source syntax renders literally rather than being interpreted.
- **M Git, sync, and recovery — required, absent.** P6/P7 are blocked on M2 (the exclusion evaluator and secret scanner). The `git-branch` transport is blocked on M5. P must not fork either policy.
- **I Properties, schemas, and Bases — required, absent.** Frontmatter provenance insertion needs I1's lossless CST and `edit_note_spans`; until then, `--provenance sidecar` is the only available mode and P2 must say so.
- **G Links, search, and graph — required for P4.** Link rewriting, ambiguity flattening, and the `[unresolved]` marker consume **G**'s `Resolution`.
- **K Canvas — required for canvas export in P4.** The canonical outline is K's; P renders it, it does not define it.
- **A Foundation — partially present.** P needs the `VaultDir` descriptor-relative backend (A slice 4) for both roots, and the attachment write path, which does not exist today.
- **B Index service — optional.** Only the `--query` selector depends on it, and it degrades explicitly.
- **External gate:** the MIT-vs-Apache-2.0 license decision blocks *distributing* the fixture corpora, not writing them.

---

## 8. Open Questions

- **Q1:** Confirm `--html-mode sanitized` as the export default (proposed here, resolving **F**'s Q4). The trade: `literal` escapes a user's deliberate `<sub>`/`<kbd>` into visible tag text, which is a fidelity loss in a document meant to be read; `sanitized` is still deny-by-default and provably inert, but it is a different default from **F**'s render path, which could surprise someone who checked with `render`. Should `publish` be forced to `literal` regardless, since a published page has the widest blast radius? — **blocks:** §3.2 export boundary defaults and §5.1 `export_html_allowlist_is_deny_by_default`.
- **Q2:** PDF engine and whether PDF ships in v1 at all. Proposed: `typst` primary (fast, single static binary, carries image alt text and document outline), `weasyprint` alternate (better CSS fidelity, heavier Python dependency), neither vendored, feature off by default. The alternative is to drop `export pdf` from v1 entirely and document "print the exported HTML from a browser," which costs nothing in capability and removes an external dependency. — **blocks:** §4.5, §4.6, and the P8 increment.
- **Q3:** Should the publish allowlist additionally honor a `publish: true` frontmatter property? Convenient, and it keeps the decision next to the note — but it creates a second source of truth, and one careless template or one `property set` over a directory would make a large set public at once. Proposed: **file only**, with `publish allow add PATH` as the ergonomic path. — **blocks:** §3.2 G1 and §4.2 `AllowDecision`.
- **Q4:** Which transports ship in v1? Proposed: `directory`, `rsync-ssh`, and `git-branch` only — **all three delegate the network to an external process, so v1 contains zero in-process sockets and the `https`/`s3-compatible` adapter, its TLS stack, and its credential-in-memory path can be deferred entirely.** Confirm, or name the hosting target that forces HTTPS in v1. — **blocks:** §4.5 dependencies and the P7 increment.
- **Q5:** Should `_import/<id>/originals/` live inside the vault (proposed: yes — it syncs, diffs, greps, and is deletable by the user, which is what makes the retained-original guarantee real) or outside it (smaller vault, but the guarantee then depends on a path the user may not back up)? If inside, confirm the default exclusion from publish, backup, and index. — **blocks:** §3.2 tier 2, §4.2 on-disk layout, §7.2 exclusion additions.
- **Q6:** Default `--batch-size` for `import --apply` and the `--single-transaction` ceiling. Proposed 2,000 and 5,000, chosen so a single batch's staging store stays well under a gigabyte. A larger batch means stronger atomicity and a bigger staging footprint; a smaller one means faster recovery and more journal writes. — **blocks:** §3.2 apply flow and §4.7.
- **Q7:** Should removal from the allowlist ever prune the destination automatically? Proposed: **never** — report `orphaned at destination` and require `--prune` with its own confirmation, because deleting remote content on the strength of local state is a way to lose something a reader still depends on. Confirm, or define an opt-in mirror mode. — **blocks:** §3.2 push step 6.
- **Q8:** Does `import obsidian --in-place` register the *existing* directory as the vault (proposed — true coexistence, zero writes, and the honest answer to 1D) or is in-place mode purely an audit that still requires a separate `vault register`? — **blocks:** §3.2 obsidian flow and the `obsidian_in_place_is_zero_writes` test.
