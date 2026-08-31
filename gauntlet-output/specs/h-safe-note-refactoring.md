# Spec: Safe Note Refactoring

**Feature ID:** h-safe-note-refactoring
**Parent feature:** root
**Spec author agent:** refactoring spec agent (H)
**Date:** 2026-08-29
**Iteration:** 1

---

## 1. Purpose

### 1.1 One-sentence job

Let a user restructure a vault — rename, move, extract, split, merge, and relocate sections, with every inbound link kept correct — as one previewable, all-or-nothing, rollbackable multi-file transaction that never leaves the vault half-changed and never claims a success it did not durably achieve.

### 1.2 Why it matters

Restructuring is where a file-authoritative knowledge system is most dangerous. A rename of one note can require edits in four hundred other files; an extract removes bytes from one file and must place them in another; a merge trashes sources and redirects every link that pointed at them. Every one of these is a multi-file mutation, and the failure modes are exactly the ones this product forbids: half the links rewritten and half not, source bytes vanishing between a delete and a create, an interrupted apply that reports "renamed", and a stale index that quietly misses inbound links so the vault silently loses connectivity.

Feature C deliberately ships path-only `note move`/`note rename` with `links_updated: false` because link-aware relocation cannot be done safely with single-file primitives. H is the feature that earns the right to say `links_updated: true`. It exists so a user can reorganize aggressively — the thing knowledge vaults most need and most rarely get safely — while the ordinary Markdown files stay the only authority, unknown syntax outside edited spans stays byte-identical, and path stays public identity with no hidden UUID injected to make renaming cheap.

### 1.3 Success signal

On a synthetic 100,000-note vault, renaming a note with 400 inbound links produces a preview that lists all 401 affected files before any write; committing it changes exactly those files and no others; and a fault-injection harness that kills the process at every journal phase and every per-step boundary always converges, on the next invocation, to either the complete pre-state or the complete post-state — never a mixture — with the vault's total set of resolved links equal in both outcomes and every source file byte-identical to one of the two known digests. `refactor rollback` on the committed receipt restores every one of the 401 files byte-for-byte, and refuses without mutating anything if any of them changed after the commit.

---

## 2. User Stories

> As a writer reorganizing my vault, I want to rename a note and have every note that links to it updated in the same operation, so that reorganizing does not silently break my graph.

> As a cautious user, I want to see the complete list of affected files and the exact edits before anything is written, so that I can cancel a refactor I did not intend.

> As a user with a large vault, I want a 400-file change set summarized without any file being hidden from me, so that a long preview never becomes a reason to skip reading it.

> As a user whose index is stale or whose index service is not running, I want the refactor to refuse or to re-scan the source, so that a link is never missed because derived data was behind.

> As a user with two notes named `Meeting.md` in different folders, I want ambiguous links reported and left alone, so that the tool never guesses which note I meant and rewrites the wrong one.

> As a note author, I want to extract a section into its own note without the rest of the file being reformatted, so that my unusual syntax, spacing, and frontmatter survive untouched.

> As a user recovering from a power loss during a merge, I want the next command to either finish the merge or undo it completely and tell me which happened, so that I never have to diff my vault by hand to find out what state it is in.

> As an automation author, I want `--dry-run`, a serialized plan, `--no-input` that refuses rather than assumes yes, stable JSON, and stable exit codes, so that scripted refactors are auditable and cannot silently escalate.

> As a screen-reader or no-color terminal user, I want the diff, the file manifest, and the confirmation to be readable as linear text with literal status words, so that I can review a destructive change without depending on color or column alignment.

---

## 3. UX Specification

### 3.1 Screen / view inventory

This is a CLI and TUI product. H introduces no graphical screens, modals, drawers, or popovers. It introduces these line-oriented CLI views, and one TUI panel that E hosts but does not own the semantics of.

| View | Entry point | New / modified | Layout pattern |
|---|---|---|---|
| Refactor help and grammar | `mg-vault refactor --help`, per-subcommand `--help` | New | Static text |
| Plan preview | any `refactor` subcommand, always shown before commit; also `--dry-run` | New | Header, complete file manifest, per-file unified diffs, classification lists, precondition block |
| Confirmation prompt | interactive terminal after preview | New | Single-line prompt on `/dev/tty` with escalation for destructive classes |
| Commit result / receipt | after apply | New | Outcome line, receipt ID, per-file result lines, durability statement |
| Partial-failure report | apply failure or interrupt | New | Transaction ID, durable-step count, literal `state: incomplete`, one recovery command |
| Refactor history | `refactor history [--path P] [--limit N]` | New | Newest-first receipt table with `rollbackable` column and reason |
| Rollback preview / result | `refactor rollback RECEIPT` | New | Same preview and confirmation machinery as a forward refactor |
| Recovery report | `refactor recover`, `doctor` refactor checks | New | Ordered checks, phase, decision (`rolled_forward` / `rolled_back` / `quarantined`) |
| Refactor panel (TUI) | E command palette → `refactor` | New view, owned by E, driven by H contracts | Preview pane plus confirm affordance; renders the same plan JSON |

Human output goes to stdout on success and stderr on warning/error. Under `--json` a success is exactly one version-1 envelope on stdout and a failure is exactly one error envelope on stderr, matching C §4.2. Prompts are written to `/dev/tty` when one exists and never into a redirected stream. Diff bodies are content and go to stdout only.

### 3.2 Interaction flows

#### Command grammar

```
mg-vault refactor rename  SOURCE NEW_FILENAME        [link opts] [txn opts]
mg-vault refactor move    SOURCE DESTINATION         [link opts] [txn opts]
mg-vault refactor extract SOURCE#SECTION --to NEW    [--leave link|embed|none] [...]
mg-vault refactor split   SOURCE --at-level N        [--into DIR] [--leave links|embeds|none]
mg-vault refactor merge   SOURCE... --into TARGET    [--separator TEXT] [--heading-shift N]
                                                     [--frontmatter first|drop|keep-all]
mg-vault refactor section move    SOURCE#SECTION --to TARGET[#AFTER_SECTION]
mg-vault refactor section promote SOURCE#SECTION [--levels N]
mg-vault refactor section demote  SOURCE#SECTION [--levels N]
mg-vault refactor section reorder SOURCE#SECTION --before|--after SOURCE#OTHER
mg-vault refactor rollback RECEIPT_ID
mg-vault refactor history  [--path PATH] [--limit N] [--after CURSOR]
mg-vault refactor recover  [--transaction ID]
mg-vault refactor gc       [--older-than DURATION] [--keep N]
```

Link options: `--links update|skip` (default `update`), `--link-style preserve|path`, `--rescan`, `--resolve 'FILE:OFFSET=TARGET'` (repeatable), `--leave-ambiguous`, `--accept-new-ambiguity`, `--drop-anchors`.
Transaction options: `--dry-run`, `--plan-out FILE`, `--from-plan FILE`, `--expected FINGERPRINT` (repeatable, `PATH=FINGERPRINT`), `--yes`, `--diff-all`, `--diff PATH`, `--diff-limit N`, `--context N`, plus the global `--vault`, `--json`, `--no-input`, `--no-color`, `--quiet`.

`SECTION` is one of `#Heading text`, `#Heading text[2]` (1-based occurrence when a heading repeats), `#^block-id`, or `@START-END` byte range. There is no fuzzy or title-based selector for any operand: every operand is an exact vault-relative `.md` path or an exact section selector, and ambiguity is an error, never a guess.

#### Universal flow (every mutating subcommand)

1. **Resolve and confine.** Resolve the vault from `--vault` or persisted selection. Validate every operand path through A's path authority: relative, no `..`, `.md`, no `.obsidian`/`.mg-vault` first component, confined beneath the canonical root at the moment of use.
2. **Check for an active journal.** If `.mg-vault/refactor/ACTIVE` exists, refuse with `transaction_incomplete` and name the recovery command; recovery runs first (§3.2 recovery flow). No refactor plans on top of an unresolved one.
3. **Read pre-images.** Read every directly named source file, capture exact bytes and `SourceFingerprint`.
4. **Resolve the link impact set** (§3.2 link discovery), unless `--links skip`.
5. **Compute span-local edits.** For every affected file, produce a set of non-overlapping byte spans with exact replacement bytes. Untouched bytes are copied verbatim; no reflow, no reserialization, no line-ending normalization, no whitespace trimming, no YAML rewriting.
6. **Build the canonical plan** (§4.2): an ordered list of typed steps, each with pre-image and post-image digests, plus preconditions and classification lists.
7. **Present the preview** (§3.2 preview) — always, including immediately before an interactive commit. Preview writes nothing to the vault and nothing to `.mg-vault`.
8. **Confirm** (§3.2 confirmation). `--dry-run` stops here with exit 0 and `changed: false`.
9. **Stage, journal, apply** (§4.4 transaction model).
10. **Report.** On full success, print the receipt. On failure, print the partial-failure report and exit nonzero; never print a success verb.

#### Link discovery, staleness, and ambiguity

1. H asks G's resolver, over B's versioned read-only IPC, for every inbound reference to each source path: wikilinks, embeds, Markdown links, and property values that I's schema declares to be link-typed.
2. **Freshness is fail-closed and there is no stale opt-in.** H requires `freshness: current` with `indexed_generation == source_generation`. Any of `stale`, `rebuilding`, `unknown`, or `unavailable` refuses with exit 6 and offers exactly two remedies in the message: wait for or trigger an index refresh (`--refresh`, bounded by `--deadline-ms`), or `--rescan`. `--allow-stale` is rejected as an invalid flag for every `refactor` subcommand; a stale index may never drive a mutation.
3. `--rescan` makes H perform its own bounded, confined, cancellable walk of every `.md` file under the vault root, skipping `.obsidian` and `.mg-vault`, parsing links directly from source. It produces a `(path, fingerprint)` manifest of every file scanned. This mode does not consult the index at all and does not write to it.
4. **Index rows are hints, never authority.** In both modes, every candidate file is re-read from source at plan time and its links re-parsed from the real bytes; edit spans always come from the fresh parse. A candidate the index reported but whose current bytes contain no matching link is dropped with an informational note. Index-derived byte offsets are never used to place an edit.
5. **Missed-file protection.** The plan records the completeness evidence it relied on: in index mode, `(indexed_generation, source_generation)`; in rescan mode, the digest of the sorted `(path, fingerprint)` manifest. At commit, H revalidates it. A changed generation, or a rescan manifest that no longer matches, returns `generation_changed`/`vault_changed` and requires a replan. This is what prevents a note created or edited during the preview from being silently missed.
6. **Ambiguity is never resolved silently.** A link whose target resolves to more than one note is classified `ambiguous`, listed with every ordered candidate path, and left byte-identical. The refactor proceeds only with explicit `--leave-ambiguous` (leave them all alone) or per-link `--resolve 'notes/a.md:1042=projects/Meeting.md'`. Under `--no-input` with ambiguous links present and neither flag supplied, the command refuses with exit 4.
7. **Introduced ambiguity is detected.** If the destination basename would make previously unique short links ambiguous, each affected link is listed under `ambiguity_introduced` and the command refuses unless `--accept-new-ambiguity` (leave them, accepting future ambiguity) or `--link-style path` (rewrite affected links to full vault-relative paths) is given.
8. **Anchors.** `[[note#Heading]]` and `[[note#^block]]` keep their anchors verbatim through rename and move. Under merge or section move, an anchor whose heading no longer exists uniquely at the destination is classified `anchor_unresolvable`, left unmodified, and requires explicit `--drop-anchors` or per-link `--resolve` to proceed.
9. **Never-rewritten regions.** Text inside fenced or indented code blocks, inline code spans, HTML comments, and math blocks is not a link; matches there are classified `skipped_in_code` and listed, not edited. Frontmatter values are edited only when I declares the property link-typed; otherwise they are listed under `skipped_untyped_property`.
10. **Link style.** `--link-style preserve` (default) changes the minimum bytes needed: a bare-basename link survives a folder-only move untouched (reported as `unchanged_by_design` with the count), and a rename replaces only the basename token, preserving alias, anchor, angle brackets, and percent-encoding exactly. `--link-style path` rewrites affected links to vault-relative paths and is the documented remedy for introduced ambiguity.

#### Preview and diff presentation

The preview has five ordered blocks and is identical in `--dry-run` and pre-commit form.

```
refactor move
vault: notes (/home/user/notes)
links: update (link-style: preserve)
link evidence: index generation 8412 == source generation 8412 (current)
affected files: 401   create: 0   edit: 400   relocate: 1   trash: 0
bytes: +0  -0  net 0        hunks: 412

FILES (complete, 401 of 401)
 relocate  ideas/old name.md -> archive/2024/old name.md   (0 hunks)
 edit      daily/2026-01-04.md                             (1 hunk, +12 -12)
 edit      daily/2026-01-09.md                             (2 hunks, +24 -24)
 ...
 edit      zettel/z-0991.md                                (1 hunk, +12 -12)

DIFFS (20 of 400 files shown; use --diff-all or --diff PATH for the rest)
--- daily/2026-01-04.md
+++ daily/2026-01-04.md
@@ line 18 @@
-see [[old name|the old note]] for context
+see [[old name 2|the old note]] for context

NOT UPDATED
 ambiguous            2 links   (2 candidate targets each; use --resolve or --leave-ambiguous)
   projects/q1.md:1180  [[Meeting]]  candidates: team/Meeting.md, client/Meeting.md
   inbox/notes.md:96    [[Meeting]]  candidates: team/Meeting.md, client/Meeting.md
 skipped_in_code      1 match    docs/syntax.md:412 (fenced code block)
 unchanged_by_design  0 links

PRECONDITIONS
 401 source fingerprints, 1 destination absent, 3 parent directories
 confirmation: required (yes)
 plan hash: sha256:9f2c...  expires: 2026-08-29T18:44:10.000000000Z
```

Presentation rules that are binding, not cosmetic:

- **The FILES manifest is always complete.** Every affected path is printed, with no elision, no "and N more", no width-based truncation, in vault-relative raw-UTF-8 byte order. Only diff *bodies* are paginated.
- **Diff bodies** default to unified format with 3 lines of context (`--context N`), `---`/`+++`/`@@` headers, and `+`/`-`/space prefixes so meaning survives ANSI stripping. Diff count defaults to `--diff-limit 20`; the elision line always states both numbers and both escape hatches. `--diff-all` shows all; `--diff PATH` shows one. `--json` and `--plan-out` always carry every hunk, paginated by generation-bound `--after` cursor, never truncated.
- **When 400 of 401 hunks are the same edit** — the common rename case — the preview additionally prints one `pattern:` line showing the single before/after replacement and the count, above the diff bodies. This is a summary in addition to, never in place of, the complete manifest.
- **A binary or invalid-UTF-8 candidate file** is never diffed or edited; it is listed under `skipped_non_utf8`.
- Control characters in paths and diff context are escaped as `\u{...}` in human output (C §3.7); the underlying bytes remain exact in JSON and in the applied edit.
- `--json` returns the complete plan object; the human preview is a rendering of it and adds no information the JSON lacks.

#### Confirmation flow

1. **Interactive, ordinary class** (no trash step, ≤ 50 affected files): prompt `Apply this refactor to 12 files? [y/N]` on `/dev/tty`. Default is No. Any answer but `y`/`yes` cancels with exit 0 and `changed: false`.
2. **Interactive, escalated class** (any `TrashFile` step, any `--leave none`, any `--frontmatter drop`, or > 50 affected files): prompt `Type "apply" to refactor 401 files (1 relocate, 400 edits):`. Only the literal word `apply` proceeds. This escalation is documented and deterministic, not a heuristic.
3. **`--no-input` refuses rather than assumes yes.** With `--no-input`, no prompt, chooser, pager, editor, or `/dev/tty` read occurs. A mutating refactor requires `--yes`; without it the command exits 2 with `confirmation_required` and prints the exact flags needed. For the escalated class, `--yes` alone is insufficient: `--from-plan FILE` (a plan artifact produced by a prior `--dry-run`) is additionally required, so automation cannot commit a large or destructive change set it never observed.
4. A prior `--dry-run` never manufactures confirmation. Confirmation is a value constructed only within the invocation that receives it (C §4.3).
5. `SIGINT` before the commit point cancels with zero vault mutation and exit 130. After the commit point, the signal is deferred until the transaction reaches a terminal record; the process never exits reporting success because a signal arrived mid-apply.

#### Partial failure and recovery flow

1. Any failure during apply produces exit 7, error code `transaction_incomplete`, and a report naming: the transaction ID, the phase reached, how many of N steps are durable, the literal line `state: incomplete — no operation is claimed complete`, and the command `mg-vault refactor recover --transaction ID`. No verb such as `moved`, `renamed`, `merged`, or `extracted` appears.
2. `.mg-vault/refactor/ACTIVE` remains present, so every subsequent mutating command in that vault — including C's `note` commands — refuses with `transaction_incomplete` until recovery resolves it.
3. `refactor recover` (also run automatically at the start of the next `refactor` invocation, and reported by `doctor`) reads the journal and decides by phase (§4.4): before the commit point it rolls back, after it rolls forward. Both are idempotent and digest-driven.
4. If roll-forward encounters a file whose current digest matches neither the recorded pre-image nor the recorded post-image, it does not write. It copies the staged post-image into `.mg-vault/refactor/quarantine/<txn>/` , reports `drift_quarantined` with the exact path and all three digests, leaves `ACTIVE` in place, and requires an explicit operator decision. Recovery never overwrites content newer than the transaction's own knowledge.
5. `refactor rollback RECEIPT_ID` runs the inverse plan through the same transaction machinery, with the original post-image digests as mandatory preconditions. It is all-or-nothing: if any file drifted since the commit, it refuses with `rollback_blocked`, lists every drifted path, writes nothing, and offers to materialize the pre-images into `.mg-vault/refactor/recovered/<receipt>/` for manual reconciliation. There is no partial rollback and no `--force`.

### 3.3 Layout descriptions

Every H view uses the fixed reading order: outcome or operation → vault name and canonical root → evidence (link provenance, freshness, generations) → counts → complete affected-file manifest → detail bodies → not-updated classifications → preconditions and confirmation requirement → warnings → one actionable next command.

Data sources per block: the vault registry and A path authority (vault/root), the G/B IPC response or H's own rescan manifest (link evidence), the in-memory plan (counts, manifest, hunks, classifications), the journal and receipt ledger (recovery and history views).

At widths below 60 columns, tables degrade to one labeled field per line; IDs, paths, digests, error codes, and recovery commands are never truncated, and long values wrap with two-space continuation indent. Every diff hunk keeps its `+`/`-`/space column at position 0 so linear reading is unambiguous.

Empty states: `refactor history` with no receipts prints `No refactors recorded for this vault.`; `refactor recover` with no journal prints `No incomplete refactor transaction.`; a rename whose link set is empty prints `affected files: 1   (no inbound links found; evidence: index generation 8412, current)` — it states the evidence rather than implying certainty from silence. `refactor gc` with nothing to prune prints `Nothing to prune. Retained: 12 receipts, 4.1 MiB.`

The TUI panel (E-hosted) renders the same plan JSON into a scrollable preview pane with the manifest above the diff bodies, a status line carrying the same evidence string, and a confirm affordance bound to the same escalation rules. E owns pane geometry; it may not weaken or skip a confirmation class.

### 3.4 Input & gestures

- All workflows are keyboard-only and line-oriented. Touch, stylus, mouse, voice, camera, and controller input are N/A for the CLI; E's panel is navigable entirely by keyboard.
- Every guided prompt has an exact flag equivalent; there is no CLI capability reachable only interactively and none reachable only non-interactively.
- Suggested TUI keybindings, registered in E's keymap and rebindable there: `g r` rename, `g m` move, `g x` extract section, `g s` split, `g M` merge, `[` / `]` previous/next affected file in the preview, `d` toggle diff body for the focused file, `a` apply (subject to the same escalation prompt), `q` cancel. H defines the actions; E owns binding defaults and conflict resolution.
- Responsive behavior is terminal width only. Tested at 40, 60, 80, and 120 columns.
- Pager use is opt-in (`--pager`), never automatic, and never engaged under `--json`, `--no-input`, or a non-TTY stdout.
- `--no-color` and a present `NO_COLOR` variable disable all styling; JSON and diff payload bytes never contain styling in any mode.

### 3.5 Transitions & animation

N/A for the CLI — there is no animation, spinner, sound, or haptic. A long `--rescan` or a large apply may emit static, rate-limited (≤ 4/s) progress lines to **stderr** in interactive human mode only: `scanned 41,200 / 100,000 files`. These are suppressed under `--quiet`, `--json`, `--no-input`, non-TTY stderr, and when `NO_COLOR` or a reduced-motion environment variable is set; in those modes progress is either absent or emitted once per 10% as a plain appended line. No dynamic cursor addressing is used. In E's panel, the preview is a static render; scrolling is instantaneous with no easing, and reduced-motion configuration changes nothing because there is no motion to reduce.

### 3.6 Error states

| Code | Trigger | Presentation and recovery | Data-loss risk |
|---|---|---|---|
| `unsafe_path` | Operand escapes the vault, hits `.obsidian`/`.mg-vault`, is non-`.md`, or crosses a symlink | Error before planning; print the rejected operand and the rule | No |
| `not_found` | Source note or section selector matches nothing | Error before planning; suggest `note locate` | No |
| `ambiguous_section` | Heading text repeats in the file and no occurrence index or byte range was given | List every occurrence with line and byte offset; require an exact selector | No |
| `ambiguous` | Inbound links with multiple candidate targets and no `--resolve`/`--leave-ambiguous` | Ordered candidates per link; refuse; exit 4 | No; nothing written |
| `ambiguity_introduced` | Rename/move would make existing short links ambiguous | List each affected link; require `--accept-new-ambiguity` or `--link-style path` | No |
| `anchor_unresolvable` | A heading/block anchor cannot survive the operation | List each link; require `--drop-anchors` or `--resolve` | No |
| `collision` | Destination path exists, or a split/extract target name is taken | Print exact colliding path; no overwrite flag exists | No |
| `conflict` | A pre-image fingerprint changed between plan and commit | Print path, expected and actual digests; replan | No; nothing written |
| `generation_changed` / `vault_changed` | Index generation moved, or the rescan manifest no longer matches, since planning | Explain that new links may exist; require replan | No |
| `index_stale` / `index_unavailable` | Link evidence is not `current` | Refuse with exit 6; offer `--refresh` or `--rescan`; never proceed on stale rows | No |
| `confirmation_required` | Mutating refactor under `--no-input` without `--yes`, or escalated class without `--from-plan` | Print the exact required flags; exit 2 | No |
| `plan_mismatch` / `plan_expired` | `--from-plan` artifact does not re-derive, hash-match, or is past `expires_at` | Name the mismatching field; regenerate the plan | No |
| `cross_device` | Source and destination are on different filesystems | Refuse; no copy-delete fallback | No |
| `durability_unavailable` / `confinement_unavailable` | Platform cannot prove directory sync or descriptor-relative confinement | Refuse before planning; exit 5; status reports `blocked` | No |
| `transaction_incomplete` | Apply interrupted, or a prior journal is unresolved | Transaction ID, durable-step count, literal `state: incomplete`, recovery command; blocks other mutations | No if the protocol holds; recovery converges |
| `drift_quarantined` | Recovery found a file matching neither pre- nor post-image | Path plus three digests; staged bytes preserved in quarantine; operator decides | No; both versions retained |
| `rollback_blocked` | A file changed after the commit being rolled back | List drifted paths; write nothing; offer `--materialize` into `recovered/` | No |
| `undo_pruned` | Rollback requested for a receipt whose pre-images were garbage-collected | State the retention policy and the receipt date | No; nothing written |
| `resource_limit` | Change set exceeds configured file/byte/hunk caps | Print the limit and the observed value; suggest narrower operands | No |
| `io` / `permission_denied` / `out_of_space` | Filesystem failure at any phase | State uncommitted or unknown status and the recovery command; never a success verb | No false success; the journal decides |

No message claims `renamed`, `moved`, `merged`, `extracted`, `split`, `rolled back`, or `durable` unless the corresponding terminal journal record is durable. Error JSON carries `version`, `ok:false`, `error.code`, `error.message`, `error.details`, `retryable`, and `recovery`; it never contains note body bytes, diff hunks, environment values, or absolute paths outside the vault.

### 3.7 Accessibility

- Every interactive element is a labeled text prompt with its accepted values, its default, and its cancel key stated inline. No prompt is timed.
- Every status is a literal word (`create`, `edit`, `relocate`, `trash`, `ambiguous`, `skipped_in_code`, `incomplete`, `rollbackable: no`). Color and glyphs are optional decoration; stripping ANSI removes no meaning. Added and removed diff lines are distinguished by the leading `+`/`-` character, never by color alone.
- Screen-reader linearity: each diff body is preceded by `file 3 of 401: daily/2026-01-09.md, 2 hunks` so position is spoken rather than inferred from layout; each hunk header states `at line 18`. Nothing essential is right-aligned, column-aligned, or spatial-only.
- Dynamic type is N/A in a terminal. Wrapping is verified at 40, 60, 80, and 120 columns with long Unicode paths; identity bytes are never altered by width handling.
- Unicode paths, combining marks, RTL text, and emoji pass through unchanged; grapheme segmentation is used only to compute display width for wrapping. Bidi and C0/C1 control characters in human output are escaped so a hostile note cannot rewrite the terminal during a confirmation prompt; the exact bytes remain available in JSON and in the edit itself.
- Focus order is the shell's natural prompt order; in E's panel, focus moves manifest → diff bodies → confirm, with a documented reverse traversal.
- The complete plan, including every hunk, every classification, and every precondition, is available as ordered text and as JSON. No refactor information exists only as a rendered visual.

---

## 4. Implementation Specification

### 4.1 Architecture placement

- **`crates/mg-vault-core/src/transaction/`** (new module tree) owns the multi-file transaction engine: `plan.rs` (canonical plan type and hashing), `journal.rs` (write-ahead journal format, phases, fsync discipline), `staging.rs` (content-addressed staging store), `apply.rs` (ordered step execution), `recover.rs` (replay, roll-forward, rollback, quarantine), `undo.rs` (pre-image store, receipts, retention/GC). This is core filesystem authority; it must not depend on CLI, index, or TUI crates.
- **`crates/mg-vault-core/src/atomic.rs`** is extended, not replaced. Its `create_atomic`, `replace_atomic`, and `sync_parent` become the primitives the apply engine calls, promoted from `pub(crate)` to a `pub(crate)` transaction-facing surface plus a new `relocate_no_replace` built on the existing `rename_without_replace`.
- **`crates/mg-vault-core/src/vault.rs`** gains `edit_note_spans` (multi-span generalization of the existing single-span `edit_note_span`) and a `stage_note_bytes` entry that materializes a post-image without publishing it. Existing single-note APIs keep their signatures.
- **`crates/mg-vault-refactor/`** (new crate) owns refactor semantics with no filesystem authority of its own: the Markdown structural model needed for headings, sections, code-fence masking, and link token spans; the operation planners (`rename`, `move`, `extract`, `split`, `merge`, `section`); the link-rewrite rules; and the diff renderer. It depends on `mg-vault-core` and on F's token model once F lands, and on G's resolver contract through a trait so it can be exercised against a contract fake.
- **`crates/mg-vault-cli`** gains `commands/refactor.rs` and `output/diff.rs`. It owns parsing, prompting, and rendering only; it never calls `std::fs` for a managed mutation.
- **`crates/mg-vault-index`** and the future service are consulted only through G's read-only resolver contract. No refactor code writes to the index; the index is invalidated by the ordinary source changes H makes and re-derives itself.

### 4.2 Data model

```rust
/// One primitive, individually recoverable filesystem step.
/// The closed set is deliberate: recovery must be able to decide the state of
/// every step from on-disk digests alone.
pub enum RefactorStep {
    /// Create a file that must not exist. Content comes from staging.
    CreateFile { path: NotePath, post: SourceFingerprint },
    /// Fingerprint-checked whole-file replacement from staging.
    ReplaceFile { path: NotePath, pre: SourceFingerprint, post: SourceFingerprint },
    /// Same-filesystem, no-replace rename. Content is unchanged by this step.
    RelocateFile { from: NotePath, to: NotePath, digest: SourceFingerprint },
    /// Move a file into vault-local trash with recoverable metadata.
    TrashFile { path: NotePath, pre: SourceFingerprint, trash_id: TrashId },
}

/// The complete, canonically serializable description of one refactor.
pub struct RefactorPlan {
    pub schema: u16,                       // 1
    pub domain: &'static str,              // "mg-vault.refactor-plan"
    pub operation: RefactorOperation,      // declarative request, replayable
    pub vault_root_identity: VaultRootFingerprint,
    pub steps: Vec<RefactorStep>,          // canonical order, see §4.4
    pub edits: Vec<FileEdits>,             // span-local hunks per edited file
    pub link_evidence: LinkEvidence,
    pub classifications: Classifications,  // ambiguous, anchors, code, non-utf8
    pub preconditions: Preconditions,      // fingerprints, absent paths, parents
    pub confirmation: ConfirmationClass,   // Ordinary | Escalated
    pub expires_at: SystemTime,
    pub plan_hash: PlanHash,
}

/// Exactly what H relied on to believe it found every inbound link.
pub enum LinkEvidence {
    Index { indexed_generation: u64, source_generation: u64, resolver_version: u16 },
    Rescan { manifest_digest: Digest, files_scanned: u64, scanned_at: SystemTime },
    Skipped,                               // --links skip, recorded explicitly
}

/// Non-overlapping byte spans and their exact replacements, ascending by start.
pub struct FileEdits {
    pub path: NotePath,
    pub pre: SourceFingerprint,
    pub post: SourceFingerprint,
    pub hunks: Vec<SpanEdit>,              // { span: Range<usize>, replacement: Vec<u8>, reason }
}

/// Durable write-ahead record. Written before the first vault mutation.
pub struct JournalRecord {
    pub version: u16,                      // 1
    pub transaction_id: TransactionId,
    pub plan_hash: PlanHash,
    pub phase: Phase,                      // Preparing | Committing | Applied | RolledBack | Quarantined
    pub steps: Vec<RefactorStep>,          // identical to the plan's, in commit order
    pub staged: Vec<StagedObject>,         // { digest, relative staging path, byte_len }
    pub undo: Vec<UndoObject>,             // pre-image digest -> undo store path
    pub touched_dirs: Vec<PathBuf>,
    pub committed_at: Option<SystemTime>,
}

/// Retained proof of a completed refactor and the basis for rollback.
pub struct RefactorReceipt {
    pub version: u16,
    pub receipt_id: ReceiptId,
    pub transaction_id: TransactionId,
    pub operation: RefactorOperation,
    pub completed_at: SystemTime,
    pub steps: Vec<RefactorStep>,          // pre/post digests for every file
    pub undo_available: bool,
    pub undo_pruned_reason: Option<String>,
}
```

On-disk layout, all under the vault-local, portable, owner-only `.mg-vault/refactor/`:

```
.mg-vault/refactor/
  ACTIVE                       -> transaction id of the one in-flight transaction (absent when none)
  journal/<txn-id>.json        -> JournalRecord, fsynced at each phase change
  staging/<txn-id>/<digest>    -> post-image bytes, content-addressed
  undo/<digest>                -> pre-image bytes, content-addressed, shared across receipts
  history/<receipt-id>.json    -> RefactorReceipt
  quarantine/<txn-id>/...      -> staged bytes recovery refused to apply, plus a report
  recovered/<receipt-id>/...   -> pre-images materialized after a blocked rollback
```

Directories are `0700` and files `0600`. Every schema is explicitly versioned; an unknown newer version fails closed with a `doctor` finding and is never rewritten by an older binary. No note content is parsed into a database and no identifier is ever written into a note: `TransactionId`, `ReceiptId`, and `TrashId` are operational handles, and the plan carries no note identity other than vault-relative paths.

`plan_hash` is a domain-separated SHA-256 over the canonical encoding of `schema`, `domain`, `operation`, `vault_root_identity`, `steps`, `edits` (spans and replacement bytes), `link_evidence`, `classifications`, `preconditions`, `confirmation`, and `expires_at`, with fixed field order and explicit nulls. Presentation text, warnings, progress counts, transaction IDs, and clocks are excluded. Changing any executable field changes the hash.

The version-1 JSON envelope from C is reused unchanged. `refactor.*` commands add these normative `data` objects: dry-run emits `{dry_run:true, plan, plan_hash, changed:false}`; commit emits `{dry_run:false, plan_hash, receipt_id, transaction_id, steps_applied, files_changed, links_updated, durability:"synced", changed:true}`; failure emits the error envelope plus `error.details.transaction_id`, `steps_durable`, and `phase`. `links_updated` is a truthful integer, and `refactor` is the only feature permitted to report it as nonzero.

### 4.3 API contracts

```rust
// Planning is side-effect free. It reads source and index; it writes nothing.
fn plan_rename(vault: &Vault, r: &dyn LinkResolver, req: RenameRequest)   -> Result<RefactorPlan>;
fn plan_move(vault: &Vault, r: &dyn LinkResolver, req: MoveRequest)       -> Result<RefactorPlan>;
fn plan_extract(vault: &Vault, req: ExtractRequest)                       -> Result<RefactorPlan>;
fn plan_split(vault: &Vault, req: SplitRequest)                           -> Result<RefactorPlan>;
fn plan_merge(vault: &Vault, r: &dyn LinkResolver, req: MergeRequest)     -> Result<RefactorPlan>;
fn plan_section(vault: &Vault, req: SectionRequest)                       -> Result<RefactorPlan>;
fn plan_rollback(vault: &Vault, receipt: &ReceiptId)                      -> Result<RefactorPlan>;

// Commit is transactional and requires an in-invocation confirmation value.
fn commit(vault: &Vault, plan: &RefactorPlan, ok: ConfirmedRefactor) -> Result<RefactorReceipt>;

// Recovery is idempotent and may be called by any invocation.
fn recover(vault: &Vault, mode: RecoverMode) -> Result<RecoveryReport>;

// Read-only ledger.
fn history(vault: &Vault, filter: HistoryFilter) -> Result<Vec<RefactorReceipt>>;
fn gc(vault: &Vault, policy: RetentionPolicy)    -> Result<GcReport>;

// The only seam to G/B. Read-only; it can propose candidates, never authorize.
trait LinkResolver {
    fn evidence(&self) -> Result<LinkEvidence>;
    fn inbound(&self, target: &NotePath) -> Result<Vec<LinkCandidate>>;
    fn resolve(&self, from: &NotePath, raw: &RawLink) -> Result<Resolution>; // Unique | Ambiguous | Unresolved
}
```

Binding contract rules:

- **One planner.** Guided preview, `--dry-run`, and immediate commit all run the same `plan_*` function. There is no second, weaker path.
- **`--from-plan` re-derives rather than trusts.** The artifact carries the declarative `operation` and the derived `steps`/`edits`. H re-runs `plan_*` from `operation` against current source and requires the freshly derived plan to hash-equal the artifact. A tampered hunk list, an injected extra file, or a swapped destination therefore cannot be applied even by a caller who can write the artifact. This is what makes an AI-proposed or plugin-proposed refactor safe: a proposal is a *request*, never an edit list.
- **Commit revalidates everything from opened handles** immediately before the first destructive step: every pre-image fingerprint, every absent destination, every parent directory, confinement of every path, and the `LinkEvidence` completeness token. A plan and its hash are not authorization.
- **`ConfirmedRefactor` is a private core type** constructible only from an in-invocation prompt answer or `--yes` (with `--from-plan` for the escalated class). No plugin, AI adapter, index response, or deserialized JSON can mint it (4C).
- Exit codes follow C's stable categories exactly: `0` success or cancel-without-change, `2` usage/confirmation, `3` not found, `4` collision/ambiguity/conflict, `5` unsafe or denied, `6` degraded dependency (stale/unavailable link evidence), `7` I/O or transaction failure, `130` interrupt. No new category is introduced.
- Pagination applies only to `refactor history` and to the hunk list in `--json`; cursors are opaque and bound to the plan hash or ledger generation. Rate limiting and auth are N/A — the process is local and runs as the owning user; the IPC client verifies peer ownership per B.

### 4.4 State management — the multi-file transaction model

This is the core of the feature. A refactor is a write-ahead-logged, staged, ordered, digest-recoverable transaction over N files.

**Ownership.** `mg-vault-core::transaction` owns all transaction state. The CLI and TUI own only presentation and confirmation. Authoritative state is always the ordinary vault files; everything under `.mg-vault/refactor/` is recovery metadata, and losing it can only cost the ability to roll back — it can never make the vault's Markdown wrong.

**Canonical step order.** Steps are always ordered `CreateFile` (path byte order) → `ReplaceFile` (path byte order) → `RelocateFile` (source path byte order) → `TrashFile` (path byte order). Additive work happens first so an interruption leaves at worst an unreferenced new file; destructive work happens last. A case-only rename on a case-insensitive filesystem is expanded at plan time into two `RelocateFile` steps through a reserved journaled intermediate name.

**Phases.**

1. **Plan.** Read-only. Nothing is written anywhere.
2. **Prepare.** Allocate `<txn-id>`. For every step, write the complete post-image into `staging/<txn-id>/<digest>`, `write_all`, `sync_all`. For every `ReplaceFile`/`TrashFile`/`RelocateFile` pre-image, materialize the pre-image into `undo/<digest>` (hard link from the live file when the filesystem allows and the link count check passes; otherwise a copy), then `sync_all`. `fsync` the staging and undo directories. Write `journal/<txn-id>.json` with `phase: Preparing`, `fsync` it, `fsync` the journal directory. Nothing in the vault's ordinary files has been touched.
3. **Commit point.** Rewrite the journal record with `phase: Committing` and `committed_at`, `fsync` the file, `fsync` the journal directory, then create `ACTIVE` and `fsync` its directory. **This barrier is the atomicity boundary:** a crash before it means the transaction never happened; a crash after it means the transaction will be completed.
4. **Apply.** Execute steps in canonical order. `CreateFile` copies the staged object to a same-directory temp, `sync_all`, then `RENAME_NOREPLACE` (`atomic::create_atomic` semantics). `ReplaceFile` re-reads the target through a descriptor-relative open, verifies the pre-image digest, writes a same-directory temp from staging, `sync_all`, then `rename` (`atomic::replace_atomic` semantics). `RelocateFile` verifies the source digest and destination absence, then `RENAME_NOREPLACE`. `TrashFile` verifies the digest, renames the payload into `.mg-vault/trash/files/<id>`, and writes the trash metadata. A per-step progress marker is appended to the journal record, but progress markers are **hints only** — recovery is digest-driven and correct even if every marker is lost.
5. **Sync.** `fsync` every directory in `touched_dirs`, including the trash directories.
6. **Terminal record.** Rewrite the journal with `phase: Applied`, `fsync`, write `history/<receipt-id>.json`, `fsync`, then remove `ACTIVE` and `fsync` its directory. Only now may the CLI print a success verb, `durability: "synced"`, and the receipt ID.
7. **Cleanup.** Remove `staging/<txn-id>/` and the journal record. Undo objects are retained per §4.4 retention. Cleanup failure is a warning, never a failure of the refactor.

**Crash recovery / journal replay.** Every `mg-vault` invocation that resolves a vault stats `.mg-vault/refactor/ACTIVE` — one `stat`, no measurable startup cost. If present, mutating commands refuse and `refactor recover` (automatic for `refactor` invocations, explicit elsewhere, also surfaced by `doctor`) replays:

- `phase: Preparing` → **roll back**: no ordinary file was touched; delete `staging/<txn-id>/`, mark the journal `RolledBack`, drop `ACTIVE`. Report `rolled_back`.
- `phase: Committing` → **roll forward**, step by step, idempotently. For each step, compare the live target's current digest:
  - matches `post` (or, for `RelocateFile`, destination present with `digest` and source absent; for `TrashFile`, payload present and source absent) → already applied, skip;
  - matches `pre` (or, for `RelocateFile`, source present and destination absent) → apply now from staging;
  - matches neither → **do not write**. Copy the staged bytes to `quarantine/<txn-id>/`, record `drift_quarantined` with the pre, post, and observed digests, keep `ACTIVE`, and stop. The operator decides; recovery never overwrites content it cannot account for.
  After all steps apply, run phases 5–7 normally.
- `phase: Applied` but `ACTIVE` still present → finish phases 6–7 (idempotent).
- Missing or unparseable staging object needed by an unapplied step → the transaction cannot roll forward. Because pre-images are in `undo/` and no destructive step has run past that point in canonical order, recovery rolls **back** every already-applied step from the undo store using the same digest guard, then reports `rolled_back`. Rollback of an already-applied step also refuses on drift and quarantines.

Recovery is idempotent: running it N times produces the same state as running it once. It is also crash-safe: it is itself replayable, because every decision is derived from durable digests rather than from in-memory position.

**Concurrency (4B).** An advisory per-vault lock at `.mg-vault/locks/refactor.lock` is held only from the commit point through the terminal record — never across a human preview, so previewing does not block editing. Between plan and commit, safety comes entirely from the mandatory commit-time revalidation of every fingerprint, destination, and evidence token, because external editors do not honor the lock. Two concurrent refactors therefore produce exactly one commit and one `conflict` or `transaction_incomplete` refusal; last-writer-wins is impossible.

**Rollback retention.** Undo objects are content-addressed and shared, so a file unchanged across many refactors is stored once. Default retention is the tightest of: 14 days, 100 receipts, or 256 MiB, with the most recent 10 receipts always retained regardless. `refactor gc` applies the policy; `refactor history` shows `rollbackable: yes|no` with `undo_pruned` as the reason when applicable. `refactor forget RECEIPT` removes one receipt's undo objects immediately — the privacy escape hatch for content a user does not want lingering. A receipt is invoked with `mg-vault refactor rollback RECEIPT_ID`.

**Identity across a move (1C).** A rename changes public identity by design. Three mechanisms track it without touching note content: inbound links are rewritten *in the source files themselves*, so the graph is correct from source alone with no ledger; the trash metadata already records the original path for a trashed source; and `history/<receipt-id>.json` records `from → to` with digests, powering `refactor history --path P` as a convenience view. That ledger is app state under `.mg-vault`, is prunable, and its deletion changes nothing about vault correctness. No UUID, `id:` property, alias table, redirect map, or hidden frontmatter key is ever injected. `refactor rename --leave-stub` is an explicit, non-default opt-in that creates an ordinary Markdown note at the old path containing a visible link — ordinary user-owned content, shown in the preview like any other created file, never a hidden identity mechanism.

**Span-locality (1B).** Every `FileEdits` is a set of non-overlapping spans applied to the exact pre-image bytes; all other bytes are copied verbatim. H never reserializes YAML, never re-emits Markdown from an AST, never normalizes line endings, never trims trailing whitespace, and never touches a byte outside a recorded span. Extracting a section from the middle of a file leaves the surrounding text — including unknown syntax, Obsidian-specific forms, and irregular spacing — byte-identical, which the tests assert by digesting the untouched prefix and suffix.

### 4.5 Dependencies

- **A Foundation** (implemented in part): `Vault` path authority, `SourceFingerprint`, `atomic::{create_atomic, replace_atomic, sync_parent}`, `rename_without_replace` via `rustix` `RenameFlags::NOREPLACE`, trash/restore, and the protected-directory rule.
- **C CLI and note operations**: plan/commit conventions, `--dry-run`/`--plan-out`/`--from-plan`, envelope v1, exit categories, `--no-input` semantics, `VaultDir` descriptor-relative confinement backend, and the transaction journal C already requires for single-note trash/restore/purge. H generalizes that journal from one file to N.
- **G Links, search, and graph** (required for `--links update` without `--rescan`): the `LinkResolver` contract, ambiguity semantics, and anchor resolution. H ships against a contract fake so it is testable before G lands.
- **B Index service** (required transitively by G): freshness/generation evidence. H never queries B directly for source bytes and never accepts a mutation authorization from it.
- **F Markdown model** (desirable, not blocking): the token/structural model for headings, code-fence masking, and link spans. Until F lands, `mg-vault-refactor` carries a minimal, conservative scanner whose documented rule is to *skip* anything it cannot classify with certainty rather than to guess — a skipped link is reported, never silently rewritten.
- **I Properties/schemas** (optional): declares which frontmatter properties are link-typed. Absent I, all property values are `skipped_untyped_property`.
- New Rust crates: a diff library or a small internal Myers implementation for hunk rendering (presentation only — the applied edit is always the recorded spans, never a re-derived diff), and `rustix` features already present. No database, network client, or async runtime is required by H itself.
- Infrastructure: none. No CDN, no third-party service, no network.

### 4.6 Platform-specific considerations

- Arch Linux is first support; Linux and macOS are the supported targets. `RENAME_NOREPLACE` is available on Linux and via `renameatx_np`/`RENAME_EXCL` on macOS; the existing hard-link-plus-unlink fallback in `atomic.rs` is acceptable only for `CreateFile` and is explicitly rejected for `RelocateFile`, which requires a true atomic no-replace rename.
- A platform may execute refactor mutations only after passing the same capability gate C defines: descriptor-relative confinement (`openat2` with `RESOLVE_BENEATH|RESOLVE_NO_SYMLINKS` on Linux, an `openat` component walk with `(dev, ino)` pinning on macOS) and proven file plus directory synchronization under crash test. Failure yields `confinement_unavailable` or `durability_unavailable` and exit 5 *before* planning. There is no reduced-durability or lexical-path fallback mode.
- All steps of one transaction must be on one filesystem. A plan spanning a mount point fails with `cross_device`; there is no copy-delete fallback, because a copy-delete pair cannot be made atomic with the rest of the set.
- Case-insensitive filesystems: destination collision checks use case-folded comparison in addition to exact comparison, and case-only renames use the journaled intermediate described in §4.4.
- Hard-linked source files (link count > 1) are refused for mutation, matching C, because replacement-by-rename would break the other name silently.
- Feature flags may gate the G resolver client and the F token model, so `--rescan` and conservative scanning remain available in a build without them. Transaction semantics, confirmation classes, and durability are never feature-gated.
- Rollout: H ships behind no runtime flag, but `refactor` subcommands report `links_updated` truthfully; in a build without G, `--links update` without `--rescan` returns `index_unavailable` rather than pretending.

### 4.7 Performance budget

Measured on a warm local SSD reference machine and reported with hardware metadata. `N` is the number of affected files.

- Plan for a rename with link evidence from a current index: p95 ≤ 250 ms for candidate lookup (inherits C/B's backlinks budget) plus ≤ 3 ms per candidate file for re-read and parse; a 400-link rename plans in p95 ≤ 1.5 s.
- `--rescan` over the 100,000-note / 1,000,000-block fixture: p95 ≤ 45 s, streamed, bounded, cancellable at least every 50 ms or 1 MiB, with static progress to stderr. This is deliberately slow and deliberately the safe fallback rather than the default.
- Preview rendering: p95 ≤ 200 ms for a 400-file manifest with 20 diff bodies; rendering is O(printed hunks), never O(vault).
- Apply: p95 ≤ 8 ms per file plus fsync cost; a 401-file commit targets p95 ≤ 4 s dominated by durability. Latency is never traded for durability — there is no batching mode that skips a required fsync.
- fsync accounting per transaction: 1 per staged object, 1 per undo object, 2 journal barriers plus 1 terminal, 1 per temp file during apply, and 1 per touched directory. For a 401-file rename that is roughly 1,200 syncs; the budget above assumes them.
- Memory: working set ≤ 64 MiB independent of `N`. Post-images are streamed to staging one file at a time and never all held in memory; the plan holds hunks, not file contents, and a file larger than 8 MiB is streamed rather than buffered whole.
- Storage: staging plus undo transiently costs (sum of post-image bytes) + (sum of changed pre-image bytes). Undo is content-addressed and deduplicated across receipts; retention caps it at 256 MiB by default. `doctor` and `refactor history` report the current size without reading content.
- Startup impact: one `stat` of `.mg-vault/refactor/ACTIVE` per vault-resolving invocation, target ≤ 1 ms; H adds no daemon, no background thread, and no index work.
- Hard limits, configurable and reported by `resource_limit`: 20,000 affected files, 200,000 hunks, 512 MiB total staged bytes per transaction. Exceeding a limit refuses before any write rather than degrading.
- Network payload: N/A — H performs no network I/O.

---

## 5. Test Specification

### 5.1 Unit tests

| Test | Setup and assertion |
|---|---|
| `section_span_is_deterministic` | ATX and setext headings, nested levels, headings inside fenced and indented code, headings in frontmatter; assert the extracted span begins at the heading's first byte and ends before the next same-or-shallower heading, and that code-fenced and frontmatter headings never terminate a section. |
| `duplicate_heading_requires_occurrence` | Two `## Notes` in one file; bare `#Notes` returns `ambiguous_section` listing both offsets; `#Notes[2]` selects the second exactly. |
| `edits_are_span_local` | For every operation, digest the pre-image prefix before the first hunk and suffix after the last; assert both are byte-identical post-commit, with CRLF, no trailing newline, tabs, emoji, and unknown Obsidian syntax present. |
| `link_rewrite_preserves_form` | Wikilink, aliased wikilink, anchored wikilink, block-ref, embed, Markdown link, angle-bracket link, percent-encoded link; assert only the target token bytes change and alias/anchor/encoding survive verbatim. |
| `code_and_comment_regions_are_masked` | `[[target]]` inside fenced, indented, and inline code, HTML comments, and math; assert zero hunks and a `skipped_in_code` classification for each. |
| `ambiguous_link_is_never_rewritten` | Two notes with the same basename; assert refusal with ordered candidates, zero hunks, and that `--leave-ambiguous` produces a plan whose hunk set excludes them. |
| `introduced_ambiguity_is_detected` | Rename that duplicates an existing basename; assert `ambiguity_introduced` lists every newly ambiguous link and that `--link-style path` resolves it. |
| `basename_move_needs_no_edits` | Folder-only move under `--link-style preserve` with unique basename; assert `affected files: 1` and `unchanged_by_design` count matches the inbound-link count. |
| `plan_hash_binds_executable_semantics` | Mutate each of operation, vault identity, step list, span, replacement byte, evidence, precondition, confirmation class, expiry; assert the hash changes. Mutate warning text and progress counts; assert it does not. |
| `from_plan_rederives_and_rejects_tampering` | Edit a serialized plan's hunk bytes, add a step, swap a destination, change the evidence; assert `plan_mismatch` before any write in every case. |
| `step_order_is_canonical` | Randomized operation sets; assert create → replace → relocate → trash, each in path byte order, deterministically. |
| `journal_roundtrips_and_rejects_future_schema` | Serialize/deserialize every phase; a `version: 2` record fails closed with a doctor finding and is not rewritten. |
| `no_input_never_prompts` | Prompt/tty spy across every subcommand under `--no-input`; assert zero interaction calls and `confirmation_required` without `--yes`. |
| `escalation_class_is_deterministic` | Fixtures at 50 and 51 files, with and without trash steps, `--leave none`, `--frontmatter drop`; assert the class matches the documented rule exactly. |
| `merge_frontmatter_defaults_fail_closed` | Two sources with frontmatter under default `--frontmatter first`; assert refusal, and that `drop` and `keep-all` each produce the documented, previewed bytes. |
| `manifest_is_never_elided` | 5,000-affected-file plan at 40 columns; assert every path appears once, no truncation, and the diff elision line states both counts. |
| `diff_meaning_survives_ansi_strip` | Render with color, strip ANSI, assert add/remove/context classification is still recoverable from the leading character. |
| `retention_policy_is_exact` | Age, count, size, and always-keep-10 policies individually and in combination; assert which receipts remain `rollbackable`. |

### 5.2 Integration tests

- `rename_updates_all_inbound_links`: 400 inbound links across 400 files; assert exactly 401 files change, every link resolves post-commit, all other files are byte-identical, and `links_updated: 400`.
- `dry_run_commit_equivalence`: for every subcommand, `--dry-run --plan-out`, then commit with `--from-plan`; assert the receipt's `plan_hash` equals the artifact's and the resulting tree equals the tree predicted by the plan's post-image digests, file for file.
- `stale_index_refuses`: index generation behind source; assert exit 6, zero writes, no rows used, and that `--allow-stale` is rejected as an unknown flag. Then `--rescan` succeeds and finds the link the stale index lacked.
- `note_created_during_preview_is_not_missed`: plan, then create a new note containing an inbound link, then commit; assert `generation_changed` (index mode) or `vault_changed` (rescan mode) and zero mutation.
- `crash_matrix_converges`: kill the process at each phase boundary and after each of the first 20 apply steps, for rename, move, extract, split, merge, and section move; after recovery assert every file matches either its pre-image or its post-image digest **as a set** — never a mixture — and that recovery is idempotent when run three times.
- `commit_point_is_the_boundary`: kill immediately before and immediately after the `Committing` fsync barrier; assert the former always rolls back to the complete pre-state and the latter always rolls forward to the complete post-state.
- `drift_during_recovery_quarantines`: after the commit point, externally overwrite one target with third-party bytes; assert recovery writes nothing to it, quarantines the staged bytes, keeps `ACTIVE`, and reports all three digests.
- `rollback_restores_or_refuses`: roll back a 401-file rename and assert byte-for-byte restoration of all 401; then modify one file and assert `rollback_blocked` with zero writes and a complete drifted-path list.
- `rollback_is_itself_transactional`: kill during a rollback; assert the same converge-or-quarantine guarantee applies to the inverse plan.
- `extract_preserves_remainder`: extract a middle section with `--leave link`, `--leave embed`, and `--leave none`; assert the new note holds the exact extracted bytes and the source's untouched regions are byte-identical in all three.
- `merge_trashes_sources_recoverably`: assert sources are in trash with correct original paths and fingerprints, inbound links point at the merged note, anchors are preserved or reported, and `note restore` of a source after the merge refuses on collision rather than overwriting.
- `active_journal_blocks_other_mutations`: with a live journal, assert `note create/edit/append/move/trash` and every `refactor` subcommand refuse with `transaction_incomplete` until recovery runs.
- `two_process_refactor_conflict`: two concurrent refactors touching overlapping files; assert exactly one commits and the other returns `conflict` or `transaction_incomplete` with zero partial application.
- `symlink_and_traversal_during_apply`: swap a directory entry for a symlink between plan and apply; assert descriptor-relative confinement refuses and the transaction rolls back.
- `cross_device_refused`: bind-mount a subdirectory; assert `cross_device` before any write.
- `resource_limits_refuse_early`: a plan over the file/hunk/byte caps refuses before staging anything.
- `scale_rename_100k`: on the 100,000-note fixture, measure plan, preview, and apply p50/p95/max and peak RSS against §4.7; assert memory is independent of `N` and no unrelated file is read during index-mode planning.
- `no_network`: run the full suite with network syscalls denied.

### 5.3 UI / E2E tests

There is no graphical UI. PTY-driven CLI end-to-end scenarios:

1. Guided rename: preview → `n` cancels with zero writes; preview → `y` commits and prints the receipt.
2. Escalated confirmation: 401-file rename requires the literal word `apply`; `y`, empty input, and `APPLY ` (trailing space) all cancel.
3. `--no-input` without `--yes` refuses with exit 2 and prints the exact flags; `--no-input --yes` on the escalated class still refuses without `--from-plan`.
4. Ctrl-C at the preview, at the prompt, and (via a fault shim) mid-apply: the first two exit 130 with zero writes; the third never prints a success verb and the next invocation reports and completes recovery.
5. Ambiguity: interactive chooser lists candidates and cancel is always available; non-interactive returns exit 4 with the same ordered candidates.
6. 40-column and screen-reader-linear rendering of a 401-file preview, a partial-failure report, and `refactor history`.
7. `--json` and `--json --no-input` for every subcommand: exactly one envelope on the correct stream, no ANSI, complete hunk list, stable ordering.
8. Non-TTY invocation: no prompt, no pager, no progress lines on stdout.
9. E's refactor panel driven by the same plan JSON: assert the panel cannot commit an escalated class without the escalated affordance and renders the complete manifest.

### 5.4 Visual / manual verification

- Render preview, receipt, partial-failure, history, and recovery views at 40, 60, 80, and 120 columns, with empty, 1-file, 12-file, 401-file, and 5,000-file change sets.
- Verify light and dark terminal themes affect decoration only; strip ANSI and compare semantics byte-for-byte. Verify `NO_COLOR`, `NO_COLOR=1`, `--no-color`, and TTY/non-TTY combinations produce no escape sequences.
- Read a full preview through a terminal screen reader or linearized transcript; confirm file position counters, hunk line numbers, classifications, and the recovery command are all spoken in order.
- Exercise Unicode paths, combining marks, RTL headings, emoji filenames, CRLF files, and files without a trailing newline; confirm identity bytes and diff content are exact and that no control sequence from note content reaches the terminal unescaped.
- Manually verify degraded states: index service stopped, index stale, `.mg-vault` read-only, disk full during staging (fault shim), and a hand-crafted stale journal.
- Run `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace --all-targets --all-features`.

---

## 6. Compliance & Safety Gate

### 6.1 Sensitive data classification

- [ ] No sensitive data involvement
- [x] **Handles sensitive data** — note bodies, headings, link text, paths, and diff hunks may be private, and the staging and undo stores hold complete copies of note content before and after every refactor. Protections: everything under `.mg-vault/refactor/` is created `0700`/`0600` and owner-only; content never leaves the machine; no network call, telemetry, or query history exists; diff bodies are written only to the explicitly requested stdout and never to logs, `doctor` output, error details, or progress lines; `--plan-out` writes hunks only where the user directed it; `refactor forget RECEIPT` and `refactor gc` provide explicit deletion of retained pre-images; `.mg-vault` is excluded from index projection and from exports, and the packaging docs recommend excluding it from Git (M owns sync policy). Error envelopes are audited by test to contain no body bytes.
- [ ] Uses synthetic/test data only until compliance gate clears

### 6.2 Asset provenance

- [x] **No third-party assets.** H bundles no models, images, fonts, corpora, or data files. Rust crate dependencies (a diff renderer, `rustix`, `sha2`, `serde`, `clap`) are implementation dependencies, not creative assets, and must still pass repository dependency-license policy. Test fixtures are synthetically generated at build time and contain no real user content.

### 6.3 Language / claims audit

- [x] **No claim not supported by evidence.** Every user-visible verb is gated: `renamed`/`moved`/`merged`/`extracted`/`split`/`rolled back` and `durability: "synced"` are emitted only after the terminal journal record is durable. Interruption reports say `state: incomplete` and name no completed operation.
- [x] **No promise of unbuilt capability.** §7 separates today's implemented single-file primitives from H's target state. `links_updated` is a truthful count and is `0` under `--links skip`; a build without G returns `index_unavailable` rather than implying link updates occurred. "Atomic" refers precisely to the scoped, same-filesystem, journal-bounded protocol in §4.4 and is fault-tested, not asserted.
- [x] **No restricted-domain language.** H makes no medical, financial, legal, security-certification, or regulatory claim. The word "safe" in the feature name is defined operationally by §4.4 and by the test matrix in §5, not used as marketing.

### 6.4 Regulatory alignment

Lens 3 criteria, addressed explicitly:

- **3A Determinism.** Link discovery, edit spans, step order, manifest order, and diff order all derive deterministically from source bytes plus a versioned resolver. Index rows are re-verified against source before any edit is placed; no result depends on filesystem enumeration order, locale collation, or arrival order.
- **3B Ambiguity.** An ambiguous link is never rewritten, never chosen, and never dropped: it is classified, listed with every candidate, and requires explicit `--resolve` or `--leave-ambiguous`. Introduced ambiguity is detected and requires explicit acceptance. Ambiguous section selectors are errors, not guesses.
- **3C Query depth.** H consumes G/B's inbound-link, embed, anchor, block-reference, and link-typed-property predicates for its impact set, and its own structural model addresses headings, sections, and code regions. It exposes no query surface of its own and adds no predicate G lacks.
- **3D Derived authority.** The index can only *propose* candidates. Every edit comes from a fresh read of the ordinary file; a stale, rebuilding, unknown, or unavailable index refuses the refactor rather than driving it; and `--rescan` bypasses derived data entirely. Deleting the whole index changes no refactor outcome except forcing `--rescan`.
- **3E Scale.** Budgets in §4.7 cover the 100,000-note / 1,000,000-block fixture: index-mode planning is O(affected files), memory is independent of `N`, `--rescan` is bounded and cancellable, and hard caps refuse rather than degrade. Editing is never blocked by H, which runs no daemon and holds its lock only across the apply window.

Auto-fail review, criterion by criterion:

| Auto-fail rule | How H forecloses it |
|---|---|
| Source-content loss | Every destructive step has a content-addressed pre-image in `undo/` written and fsynced before the commit point; merge trashes rather than deletes; extraction with `--leave none` retains the pre-image; rollback restores byte-for-byte. |
| Partial multi-file mutation | The whole point of §4.4: staged post-images, a fsynced write-ahead journal, a single commit-point barrier, canonical step order, digest-driven idempotent replay, and quarantine on drift. Recovery converges to the complete pre-state or the complete post-state, proven by the `crash_matrix_converges` and `commit_point_is_the_boundary` tests. |
| Index state overriding source | Index rows are hints only; edit spans always come from a fresh source read; commit revalidates every fingerprint from opened handles. |
| Stale index presented as current | H requires `current` evidence, records the generations in the plan, revalidates at commit, and rejects `--allow-stale` outright. Rescan mode records and revalidates a full manifest digest. |
| Silent conflict winner | Every fingerprint mismatch is a `conflict` refusal; every recovery drift is a quarantine; there is no `--force` on any refactor subcommand. |
| Unknown syntax loss | Edits are non-overlapping spans on exact pre-image bytes; no AST re-emission, no normalization, asserted by `edits_are_span_local`. |
| Unconfirmed overwrite/import | No overwrite flag exists; destinations must be absent; `--no-input` refuses without `--yes`, and the escalated class additionally requires an observed plan. |
| Unsafe traversal or symlink escape | A's path authority plus C's descriptor-relative `VaultDir` backend at plan and at apply; `confinement_unavailable` blocks mutation entirely on unsupported platforms. |
| Capability/data-exfiltration bypass | `ConfirmedRefactor` is a private, in-invocation type; `--from-plan` re-derives from the declarative request so a tampered or AI-authored hunk list cannot execute; H makes no network call. |
| Active raw HTML/script by default | H renders inert text only, escapes control characters in human output, and executes nothing from note content. |
| Non-atomic save claiming success | Success verbs and `durability: "synced"` are emitted only after the terminal record is fsynced; interruption reports `state: incomplete`. |
| Recovery overwriting newer source | Roll-forward, rollback, and `--from-plan` all refuse any file whose current digest matches neither the recorded pre-image nor the recorded post-image, and quarantine instead of writing. |
| Graph/Canvas information lacking a textual equivalent | Every plan, manifest, hunk, classification, and receipt is complete ordered text and complete JSON; nothing exists only as a rendered view. |

---

## 7. Gap Analysis vs. Current State

### 7.1 What exists today

Verified against the working tree at commit `dfe33cf`.

**Implemented, and directly reusable:**
- `crates/mg-vault-core/src/atomic.rs` — `write_temp` (unique same-directory temp, `write_all`, `sync_all`), `create_atomic` (temp + `rename_without_replace` + `sync_parent`), `replace_atomic` (temp + replacing `fs::rename` + `sync_parent`), `sync_parent` (opens the parent and `sync_all`), and `rename_without_replace` using `rustix` `RenameFlags::NOREPLACE` on Linux/macOS with a hard-link fallback elsewhere. These are the exact primitives §4.4's apply engine needs — but they are `pub(crate)` and single-file only.
- `crates/mg-vault-core/src/vault.rs` — `Vault::open` canonicalization, `validate_note_path` (relative, all-`Normal` components, `.md` extension, `.obsidian`/`.mg-vault` mutation rejection), `ensure_beneath`, `prepare_destination` (per-component symlink rejection and directory creation with `sync_parent`), `existing_mutation_path`, `create_note`, `read_note`, `write_note` (fingerprint-checked replace), `edit_note_span` (**single**-span, byte-preserving, UTF-8-boundary-checked), `trash_note`, `restore_note`, and `SourceFingerprint` (`sha256:` tagged).
- `crates/mg-vault-core/src/registry.rs`, `xdg.rs`, `error.rs` — registry, XDG paths, and the typed `Error` enum (`UnsafePath`, `Collision`, `Conflict`, `InvalidEditSpan`, `InvalidTrashEntry`, `Io`, …).
- `crates/mg-vault-core/tests/foundation.rs` — 13 tests including `narrow_edit_preserves_all_bytes_outside_the_selected_span`, symlink-escape rejection, protected-directory rejection, stale-fingerprint refusal, and trash/restore collision refusal. This is real evidence for 1B and 1E at single-file scale.
- `crates/mg-vault-cli/src/main.rs` — `vault register/list/select`, `note create/read/write/trash/restore`, `index rebuild/status`, `search`, `interop export`, global `--vault`/`--json`/`--no-input`/`--no-color`, and the version-1 envelope.

**Prototyped:** `crates/mg-vault-core/src/index.rs` `MarkdownIndex` — a rebuildable in-memory projection whose `wikilinks()` is a naive `[[`…`]]` scanner that returns target strings only. It records **no byte offsets**, is not code-fence aware, does not parse Markdown links, embeds, anchors, block references, or properties, and performs no resolution or ambiguity detection. It is nowhere near sufficient as H's link engine. `crates/mg-vault-index` provides a disposable SQLite generation with observational freshness — the right shape for evidence, but with no link table.

**Absent:** every part of H. There is no write-ahead journal, no staging store, no undo/pre-image store, no receipt ledger, no `ACTIVE` marker, no multi-file transaction of any kind, no rollback, no recovery command, no multi-span edit API, no heading/section model, no code-fence masking, no link-span rewriting, no diff renderer, no `refactor` command tree, no confirmation escalation, no `--dry-run`/`--from-plan` machinery, and no descriptor-relative `VaultDir` backend. `trash_note` and `restore_note` today do best-effort inline rollback with ignored `let _ = fs::rename(...)` results and no journal, so an interrupted trash can leave a payload and metadata out of step with no durable record of intent.

**Gated:** nothing in H is behind a runtime flag today because nothing in H exists. C's `note move`/`note rename` are specified with `links_updated: false` precisely so that no shipped command can claim link-preserving relocation before H lands.

### 7.2 Delta to spec

New modules and crates:
- `crates/mg-vault-core/src/transaction/{plan,journal,staging,apply,recover,undo}.rs`.
- `crates/mg-vault-refactor/` — structural Markdown model (headings, sections, code-fence masking, link token spans), the six operation planners, link-rewrite rules, and the diff renderer.
- `crates/mg-vault-cli/src/commands/refactor.rs` and `src/output/diff.rs`.

Modified files:
- `crates/mg-vault-core/src/atomic.rs` — expose the primitives to the transaction module; add `relocate_no_replace`; refuse the hard-link fallback for relocations.
- `crates/mg-vault-core/src/vault.rs` — add `edit_note_spans` (multi-span), staging entry points, and hard-link-count refusal; route mutations through the descriptor-relative `VaultDir` backend C introduces.
- `crates/mg-vault-core/src/error.rs` — add `TransactionIncomplete`, `DriftQuarantined`, `RollbackBlocked`, `GenerationChanged`, `AmbiguousLink`, `AmbiguousSection`, `AnchorUnresolvable`, `CrossDevice`, `ResourceLimit`, `UndoPruned`.
- `crates/mg-vault-core/src/lib.rs` — export the new public transaction and receipt types.
- `crates/mg-vault-cli/src/main.rs` — register the `refactor` subcommand tree; add the `ACTIVE` startup check to vault resolution; extend exit-code mapping to C's stable categories.
- `docs/ARCHITECTURE.md`, `docs/SECURITY.md`, `README.md` — document the transaction model, the `.mg-vault/refactor/` layout, retention, and the honest capability state.

Schema and layout changes (no note-content migration of any kind):
- New versioned on-disk schemas: journal record v1, receipt v1, plan artifact v1 (domain `mg-vault.refactor-plan`).
- New `.mg-vault/refactor/` tree with `0700`/`0600` permissions and a documented GC policy.
- Envelope v1 gains `refactor.*` command `data` objects additively; no existing field changes type or stream.

New dependencies: a diff library (or a small internal Myers implementation) for presentation only; G's `LinkResolver` contract plus a test fake. No database, network, or async runtime.

### 7.3 Estimated scope

**XL.** The command surface is moderate but the correctness surface is not. H combines a durable multi-file transaction engine with crash-recovery semantics, a structural Markdown model precise enough to rewrite link tokens without touching a neighbouring byte, ambiguity semantics that must never guess, a complete-manifest diff presentation, a confirmation escalation ladder, a rollback system with retention and privacy controls, and a fault-injection matrix that must be run at every phase and step boundary. It should ship as gated increments — H1 transaction engine and recovery with a rename-only planner, H2 move plus link rewriting, H3 extract/split/section, H4 merge and anchors, H5 rollback/history/GC — each behind the same journal contract, rather than as one unreviewable patch. No increment may claim `links_updated` until its own link evidence path is fault-tested.

### 7.4 Blocking dependencies

- **A Foundation** — implemented in part. H additionally requires the descriptor-relative `VaultDir` backend and the platform durability gate; without them H must refuse with `confinement_unavailable`/`durability_unavailable` rather than mutate.
- **C CLI and note operations** — required. H reuses C's plan/commit conventions, `--dry-run`/`--plan-out`/`--from-plan`, envelope v1, exit categories, `--no-input` rules, and the single-note journal C introduces. H generalizes that journal; the two must not diverge into competing transaction formats.
- **G Links, search, and graph** — required for `--links update` in index mode. Until G ships, H is usable only with `--rescan` or `--links skip`, and index mode returns `index_unavailable`. G also owns ambiguity and anchor resolution semantics that H consumes rather than defines.
- **B Index service** — required transitively by G for freshness generations. Not required for `--rescan`.
- **F Markdown and rich content** — desirable, not blocking. Until F's token model lands, H's conservative scanner skips and reports anything it cannot classify with certainty.
- **I Properties, schemas, and Bases** — optional; without it, link-typed frontmatter properties are reported as skipped rather than rewritten.
- **E TUI workspace** — dependent, not blocking. E's refactor panel consumes H's plan JSON and confirmation classes; H ships fully usable from the CLI without E.
- External gate: the repository license decision (MIT vs Apache-2.0) remains open from spec A and blocks a LICENSE file, not implementation.

---

## 8. Open Questions

- **Q1:** Default undo retention is proposed as 14 days / 100 receipts / 256 MiB with the last 10 always kept. Are those the right defaults for a vault that may hold private content the user expects deleted content to leave? — blocks the shipped defaults in §4.4 and §6.1, not the design.
- **Q2:** Should `refactor split` derive destination filenames from heading text via C's versioned slug algorithm, or require an explicit `--name-template`? Slugging is convenient but makes filenames — public identity under 1C — a function of prose. — blocks §3.2 split naming.
- **Q3:** `--link-style preserve` leaves a bare-basename link untouched during a folder-only move. Is silently correct-but-unchanged the behavior the user wants, or should a folder move offer to rewrite short links to full paths to make the graph explicit? — blocks the default for §3.2 rule 10.
- **Q4:** Should `--rescan` be permitted under `--no-input` for automation given its 45 s cost on a 100k-note vault, or should batch automation be required to ensure a current index first? — blocks the automation guidance in §3.2 and §4.7.
- **Q5:** When a merge's `--frontmatter keep-all` appends dropped frontmatter as fenced YAML blocks, that is the one place H adds structure rather than moving bytes. Should this option exist at all, or should `first` and `drop` be the only choices, with multi-frontmatter merges simply refused? — blocks §3.2 merge options.
- **Q6:** `refactor rename --leave-stub` creates an ordinary note at the old path. Is that in scope for H, or does it belong to a later navigation/redirect feature so H stays purely structural? — blocks §4.4 identity, not the transaction model.
