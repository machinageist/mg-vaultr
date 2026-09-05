# Spec: Git, Filesystem Sync, and Recovery

**Feature ID:** m-git-sync-recovery
**Parent feature:** root
**Spec author agent:** sync/recovery spec agent (M)
**Date:** 2026-08-29
**Iteration:** 1

---

## 1. Purpose

### 1.1 One-sentence job

Let a user carry one vault across machines and back in time — local checkpoints, readable history, explicit push and pull, reconciliation of Syncthing conflict artifacts, a merge that refuses to guess, verified backups, and deny-by-default secret exclusion — without git, a remote, a sync daemon, or a restore ever becoming the authority over the ordinary files in the working tree.

### 1.2 Why it matters

A local-first knowledge system is only local-first if it survives the machine. Users already solve this with git and Syncthing, and both fail in the exact ways this product forbids. `git merge` writes `<<<<<<<` markers into Markdown, corrupting YAML frontmatter and fenced blocks and then feeding that corruption to the index and the renderer. `git checkout` and `git reset --hard` silently replace hours of work with an older tree. Syncthing writes `note.sync-conflict-20260829-140311-K5J3ABC.md` beside the file and then never mentions it again, so one side of every conflict rots unread in the vault. A backup nobody verified is not a backup. And a vault synced to a remote is the single easiest way to publish an API key by accident, because notes are exactly where people paste credentials.

Every one of those is a hard failure of this product's criteria: silent conflict winner, recovery overwriting newer source, source-content loss, and secrets reaching an unintended payload. M exists so that sync and recovery are the *most* conservative subsystem in the product rather than the least: git is used as a content-addressed history and transport mechanism over ordinary files, the working tree stays the sole authority, both sides of every conflict are always preserved, and the network is touched only when the user types the command that touches it.

### 1.3 Success signal

On a synthetic two-machine fixture, a full round trip — checkpoint, push, edit on both sides, pull with overlapping and frontmatter-level changes, Syncthing artifacts present, secrets planted in three files — produces: zero bytes written to any working-tree file without an explicit confirmed decision; zero conflict markers anywhere in the vault; both sides of all 12 conflicts recoverable byte-for-byte after resolution; three refused commits naming the exact detector and redacted match; zero secret bytes in any log, backup, or pushed object; and a crash-injection matrix that kills the process at every phase of every M command converging, on the next invocation, to a working tree byte-identical to a known pre- or post-state — never a mixture, never a file the user had not approved. Measurably: `cargo test -p mg-vault-sync --all-features` green, and a socket-denial harness proving every non-`sync` command in the whole product completes with `connect(2)` disabled.

---

## 2. User Stories

> As a writer with a laptop and a desktop, I want to push and pull my vault on demand, so that both machines converge without a background process ever contacting a server while I am writing.

> As a cautious user, I want a pull to show me the exact per-file plan and make me confirm before a single byte is written, so that a remote can never rearrange my vault behind a progress bar.

> As a user whose two machines both edited the same note's frontmatter, I want both versions kept and presented side by side with a key-level comparison, so that I choose the winner instead of discovering a line-merged YAML block that no longer parses.

> As a Syncthing user, I want `*.sync-conflict-*` files found, explained, and reconciled through a decision I make, so that they are never silently deleted, silently merged, or left to accumulate unread for months.

> As a user who pastes an API key into a scratch note, I want the commit refused with the file, line, and detector named, so that the credential never reaches a remote — and if one already did, I want rotation-first remediation steps rather than a tool that quietly rewrites shared history.

> As a user restoring from a backup after a disk failure, I want restore to default to a fresh directory, preview every difference, and refuse to overwrite a file that is newer than the backup unless I say so per path, so that recovery cannot destroy the work it was supposed to protect.

> As a user who lost power during a pull that landed on top of a refactor, I want the next command to tell me exactly which of the two is incomplete and how to finish it, so that I never have to diff my vault by hand.

> As an automation author, I want `--dry-run`, `--json`, stable exit codes, and a `--no-input` mode that refuses conflicts rather than resolving them, so that a scheduled script can report drift without ever being able to pick a winner.

> As a screen-reader or no-color terminal user, I want status, history, diffs, and conflicts as linear labeled text with literal status words, so that reviewing a destructive sync never depends on color, columns, or a redrawing progress bar.

---

## 3. UX Specification

`mg-vault` is a CLI and TUI product. M introduces no graphical screens, modals, drawers, or popovers. §3 is terminal UX throughout: line-oriented command output plus one TUI panel that E hosts and M supplies the semantics for.

### 3.1 Command / view inventory

| View | Invocation | New / modified | Layout pattern |
|---|---|---|---|
| Sync help and grammar | `mg-vault sync --help`, per-subcommand `--help` | New | Static text; no repo, network, or vault access |
| Sync status | `mg-vault sync status [--refresh] [--fast]` | New | Labeled block, fixed reading order, evidence-timestamped |
| Repository setup receipt | `mg-vault sync init`, `sync remote add/list/remove/test` | New | Receipt lines; `remote list` is a table |
| Checkpoint receipt | `mg-vault sync checkpoint [-m MSG]` | New | Outcome line, commit id, file count, durability statement |
| History | `mg-vault sync history [--path P] [--limit N] [--checkpoints-only]` | New | Newest-first table, explicit empty state |
| Commit / range diff | `mg-vault sync show REV`, `sync diff [--rev A..B]` | New | File blocks with literal status words, then hunks |
| Push preview + confirmation | `mg-vault sync push` | New | Preview block, network declaration, `/dev/tty` prompt |
| Pull preview + confirmation | `mg-vault sync pull` | New | Fetch summary, complete materialization manifest, escalated prompt |
| Conflict inbox | `mg-vault sync conflicts [--kind git\|syncthing\|all]` | New | One record per conflict, both sides always named |
| Conflict detail | `mg-vault sync resolve show ID [--diff]` | New | Side-by-side digests, semantic frontmatter comparison, hunks |
| Resolution receipt | `mg-vault sync resolve take ID …` | New | What was written, what was retained, where the losing side lives |
| Restore preview / receipt | `mg-vault sync restore --from REV --path P…` | New | Same preview + confirm machinery as pull |
| Backup create / verify / restore | `mg-vault sync backup {create,verify,restore}` | New | Manifest summary, digest table, verification verdict |
| Exclusion inspector | `mg-vault sync exclude {list,check PATH,write-gitignore}` | New | Rule table with the deciding layer named per path |
| Secret scan report | `mg-vault sync scan [--range A..B] [--staged] [--worktree]` | New | Finding list: path, line, detector, redacted excerpt |
| Recovery report | `mg-vault sync recover [--transaction ID]` | New | Ordered checks, phase, decision, one recovery command |
| Sync panel (TUI) | E command palette → `sync` | New view, hosted by E, semantics owned by M | Status header, conflict list, detail pane, confirm affordance |

Stream discipline follows C's version-1 envelope unchanged: human success on stdout, warnings and errors on stderr; under `--json` exactly one envelope object plus `\n` on the corresponding stream and the other stream empty. Prompts go to `/dev/tty` when one exists and never into a redirected stream. Diff bodies are content and go to stdout only.

### 3.2 Interaction flows

#### The rule that shapes every flow

**mg-vault never runs a git command that writes to the working tree.** The git subcommand set is partitioned and enforced by an allowlist in one invocation wrapper:

- **Read-only:** `status`, `log`, `show`, `cat-file`, `rev-parse`, `rev-list`, `diff`, `diff-tree`, `ls-files`, `ls-tree`, `check-ignore`, `for-each-ref`, `merge-base`, `var`, `bundle verify`.
- **Repository-only writes** (objects, refs, index — never the working tree): `init`, `add -- <exact pathspecs>`, `update-index --refresh`, `write-tree`, `commit-tree`, `hash-object -w`, `update-ref`, `merge-tree --write-tree`, `bundle create`, `fetch`, `push`, `ls-remote`, `remote`.
- **Forbidden entirely, under any flag:** `checkout`, `switch`, `restore`, `merge`, `pull`, `rebase`, `reset`, `stash`, `clean`, `cherry-pick`, `revert`, `apply`, `am`, `mv`, `rm`, `filter-branch`, `filter-repo`, `gc --prune=now`, and any `--force`/`--force-with-lease` push.

Every working-tree byte M writes goes through A's atomic replacement with a fingerprint precondition, inside H's multi-file transaction. That single rule is what makes "git is a mechanism, the files are the authority" true rather than aspirational, and it is what makes "recovery overwriting newer source" structurally unreachable.

#### `sync status` (never touches the network unless `--refresh` is typed)

1. Resolve the vault through A. Stat `.mg-vault/refactor/ACTIVE` and `.mg-vault/sync/ACTIVE`; report either as `transaction: <kind> incomplete` with the recovery command, and continue reporting read-only facts.
2. If `.git` is absent, report `repository: absent` and continue — Syncthing conflict detection, backup, and exclusion inspection all work with no git at all.
3. If the repository is mid-rebase, mid-merge, mid-cherry-pick, bisecting, or on a detached HEAD created by the user's own git usage, report `repository: <state> (unsupported by mg-vault)` and refuse every mutating M command with `git_state_unsupported`. M never cleans up a state it did not create.
4. Run `git status --porcelain=v2 -z --untracked-files=all` (`--fast` downgrades to `normal`) and summarize modified / added / deleted / untracked counts.
5. Report ahead/behind **with the timestamp of the last fetch**, never as a current fact: `behind: 3   as-of-fetch: 2026-08-29T13:41:07Z (48m ago)`. Remote-tracking refs are stale derived data and are labeled exactly like B labels a stale index. `--refresh` runs `git ls-remote` — this is the one flag on `status` that opens a socket, it is never implied, and the output says `network: contacted origin`.
6. Scan for Syncthing artifacts (below) and count unresolved entries in the conflict store.
7. Report the last checkpoint, the excluded-path count, and, informationally only, B's index freshness with the literal note that sync never reads the index.

#### Syncthing conflict detection and reconciliation

1. Detection matches basenames against the exact pattern `^(?P<stem>.+)\.sync-conflict-(?P<date>\d{8})-(?P<time>\d{6})-(?P<device>[A-Z0-9]{7})(?P<ext>\.[^./]*)?$` during a confined walk that skips `.mg-vault` and does **not** descend into `.obsidian` for mutation purposes — artifacts found under `.obsidian` are reported with the literal note `not managed by mg-vault` and are never touched, because A forbids mutating that directory.
2. Each artifact becomes a conflict record: live path, sidecar path, both fingerprints, both byte lengths, decoded device id, the sidecar's embedded timestamp (reported verbatim with the literal caveat `device local time, no zone recorded`), and a relation classification: `identical`, `sidecar-only` (the live file is missing), or `divergent` with added/removed line counts.
3. **Artifacts are never auto-deleted, auto-renamed, auto-merged, or auto-committed.** They are excluded from every commit by the managed exclusion set, so a conflict artifact can never propagate to a remote as if it were a note.
4. Unreconciled artifacts **block** `sync pull` and `sync resolve` on the same path — reconciling a filesystem conflict and a git merge on one file at the same time is exactly the ambiguity this product refuses — and **warn** on `sync push`, which is not blocked because the artifacts are excluded from the pushed content anyway.
5. `sync resolve take ID --side live|incoming|both|--merged-file F` is the only way an artifact is retired:
   - `--side live` keeps the working-tree file byte-identical (zero writes) and moves the sidecar's bytes into `.mg-vault/sync/conflicts/<id>/incoming` before unlinking the sidecar.
   - `--side incoming` writes the sidecar's bytes to the live path through A's atomic replacement with the live file's current fingerprint as precondition, after moving the live bytes into `.mg-vault/sync/conflicts/<id>/live`.
   - `--side both --as PATH` requires the user to supply the new path. There is no auto-derived filename, because filenames are public identity (1C).
   - `--merged-file F` writes bytes the user produced; both original sides are retained.
   - `relation: identical` is the one case where the sidecar may be removed with no retained copy, because the live file *is* the byte-identical copy. It still requires one confirmation.
6. In every branch the losing bytes are retained under `.mg-vault/sync/conflicts/<id>/` and listed by `sync conflicts --resolved` until retention prunes them. The sidecar file is unlinked only after the retained copy is durably `fsync`ed.

#### `sync checkpoint`

1. A checkpoint **is an ordinary git commit** — not a separate mechanism. Justification: any parallel snapshot store would be a second history with its own corruption modes, would not be readable by `git log`, and would not travel with a push. A commit is inspectable by the user's own git, costs one commit object when nothing changed, and keeps git in its proper role.
2. Convention: subject `checkpoint: <ISO-8601 UTC> (<n> files)` — or the user's `-m` text — plus trailers:

   ```
   mg-vault-checkpoint: v1
   mg-vault-reason: manual|pre-refactor|pre-pull|pre-restore|pre-import|recovered
   mg-vault-txn: <transaction-id>        # present when tied to an H/M transaction
   mg-vault-receipt: <receipt-id>        # present when tied to a committed refactor
   ```

   `sync history --checkpoints-only` filters on the `mg-vault-checkpoint` trailer. No ref namespace, tag, or note object is used.
3. Staging is by **explicit pathspec**: M computes the intended path set from a fresh `git status` filtered through the exclusion evaluator, runs `git add -- <paths>`, then verifies that `git diff --cached --name-only -z` equals the intended set exactly. Any mismatch aborts before a commit object exists. `git add -A`, `git add .`, and `git commit -a` are never used.
4. The pre-commit secret scan runs over staged content. `deny`-class findings refuse the commit (exit 5). `warn`-class findings refuse under `--no-input` and require `--accept-warnings` interactively.
5. Hooks are disabled by default (`-c core.hooksPath=<empty>`), because a hook that rewrites files would violate the digest guarantees the transaction depends on. `sync.run_hooks = true` opts in; with hooks enabled, M re-reads every affected file after the commit and reports `hook_modified_worktree` with the drifted paths if anything changed.
6. Automatic checkpoints fire before destructive local operations listed in `sync.checkpoint.auto_before` (default `["refactor","restore","import","pull"]`). They are announced in the preview of the operation they precede, skippable with `--no-checkpoint`, and governed by `sync.checkpoint.on_failure` (default `refuse`) — a user who enabled the safety net does not get the operation without it.
7. **Relation to A's trash and H's undo store.** Three deliberately separate layers, never substituted for one another: A's trash holds the bytes of one deleted file and restores collision-refusingly; H's undo store holds per-refactor pre-images with bounded retention and all-or-nothing rollback; a checkpoint holds the whole tree, coarse and durable, survives deletion of `.mg-vault`, and can travel to another machine. M never reads or writes A's trash payloads or H's undo objects, and never presents a checkpoint as a substitute for either.

#### `sync push` — explicit, previewed, confirmed, never forced

1. Refuse if a transaction is active, if a `git_state_unsupported` state is present, or if the branch has no configured upstream and none was named.
2. Run the exclusion evaluator and the secret scan over the entire outgoing range `git rev-list <upstream>..<HEAD>` — because a commit made by plain `git`, or before the scanner existed, can carry a secret that this push would publish. `deny` findings block the push.
3. Preview: remote name, **redacted** URL (userinfo stripped; `ssh://git@host/path` shown, `https://user:token@host` shown as `https://host`), branch, each outgoing commit (short id, ISO timestamp, subject, file count), total objects and bytes as reported by a dry-run pack estimate, the exclusion summary, the scan verdict, and the literal line `will contact network: yes`.
4. Confirmation on `/dev/tty`. `--dry-run` stops before this with exit 0 and `changed: false`.
5. Refuse a non-fast-forward push outright with `push_not_fast_forward`, printing the diverged commit counts and instructing the user to pull first. M never passes `--force` or `--force-with-lease`, because a force push destroys history other clones hold and M cannot know who else holds it.
6. Run `git push --atomic <remote> <branch>` and report the remote's accepted ref. If the child is killed after the remote may have accepted, the receipt says `state: unknown` and names `mg-vault sync status --refresh` as the resolution — never a success verb.

#### `sync pull` — fetch, plan, preview, confirm, materialize, then record

Ordering is binding and is the answer to the crash-consistency question in §4.4:

1. **Guard.** Refuse if `.mg-vault/refactor/ACTIVE` or `.mg-vault/sync/ACTIVE` exists (`transaction_incomplete`, exit 7, recovery command named). Refuse if any unreconciled Syncthing artifact touches a path the merge would change.
2. **Fetch.** `git fetch --atomic <remote> <branch>`. This writes objects and a remote-tracking ref only. It is idempotent and safe to repeat, which is why it is first. `--fetch-only` stops here, needs no escalated confirmation, and writes nothing to the vault.
3. **Checkpoint.** Auto-checkpoint the current working tree (§3.2 checkpoint) unless `--no-checkpoint`, so the pre-pull state is recoverable from git even if `.mg-vault` is lost.
4. **Plan.** Compute the merge entirely in the object database with `git merge-tree --write-tree --name-only <ours> <theirs>`, which produces a merged tree and a conflict list **without touching the working tree or the index**. For every path in the resulting tree, read the *live working-tree bytes* — never `git show` of `HEAD` — and classify:

   | Class | Meaning | Default handling |
   |---|---|---|
   | `unchanged` | live bytes equal the merged blob | no write |
   | `incoming-only` | live equals base; remote changed | materialize |
   | `local-only` | remote equals base; live changed | no write |
   | `clean-merge` | both changed, disjoint hunks, no protected region touched | materialize merged bytes |
   | `conflict:overlap` | both changed the same hunk | conflict record, no write |
   | `conflict:frontmatter` | both changed bytes inside the YAML envelope | conflict record, no write |
   | `conflict:fenced` | a hunk starts or ends inside a fenced/math block | conflict record, no write |
   | `conflict:structured` | `.canvas`, binary, or `> max_file_bytes` | conflict record, no write |
   | `conflict:delete-vs-modify` | remote deleted, live modified | conflict record; the file is **never** deleted |
   | `conflict:rename-vs-modify` | remote renamed, live modified | conflict record, no write |
   | `local-dirty` | live differs from `HEAD` on a path the merge changes | conflict record, no write |
   | `excluded` | path is in the exclusion set | reported, never materialized |

5. **Preview.** Complete manifest — every affected path, no elision — grouped by class with counts, plus the incoming commit list, the checkpoint that was created, and the literal line `files that will be written: N`.
6. **Confirm.** Escalated confirmation (type `apply`) whenever `N > 0`. Under `--no-input`, `--yes` is honored only when every class present is a subset of `--allow`; **any `conflict:*` class refuses regardless of `--yes`**.
7. **Materialize.** Build one H transaction (`operation: SyncMaterialize`) whose steps carry each file's current fingerprint as a precondition, and commit it through `mg-vault-core::transaction`. M implements no second transaction engine.
8. **Record.** Only after the transaction's terminal record is durable, create the merge commit from the now-current working tree with both parents (`git add -- <paths>`, `write-tree`, `commit-tree -p <ours> -p <theirs>`, `update-ref`). The ref update is strictly last, so the only reachable inconsistent state is "working tree correct, history behind" — the safe direction, since git can always record a tree but can never invent lost bytes.
9. **Report.** Files written, conflicts opened, the commit id, `durability: synced`, and one line: `index: stale until reconciled (mg-vault index status)`.

#### Git conflict presentation and resolution

No conflict marker is ever written into a user's file. `<<<<<<<` in a Markdown note is source corruption: it is indexed, rendered, and re-synced. Instead each conflict opens a record with all three sides materialized under `.mg-vault/sync/conflicts/<id>/{base,ours,theirs}` and the working-tree file left byte-identical.

`sync resolve show ID` renders, in fixed order: id, kind, path, class, a plain-English reason, the three digests with byte lengths and provenance, a **semantic** frontmatter comparison when the class is `conflict:frontmatter` (key-by-key: `ours` / `theirs` / `absent`, never a textual merge of the envelope), the body hunks, the storage location of all three sides, the available resolution commands, and the literal closing line `nothing has been written to <path>`.

`sync resolve take ID --side ours|theirs|both --as PATH|--merged-file F` applies exactly one decision through an H transaction with the live fingerprint as precondition. `--side both` requires an explicit `--as PATH`. The losing side is always retained in the conflict store; resolution never deletes bytes. `sync resolve abandon ID` closes a record while retaining all three sides. There is no `--strategy ours`, no `--strategy theirs`, no `-X ours`, and no automatic resolution of any class, including `local-dirty` and `delete-vs-modify`.

#### `sync restore` and backups

`sync restore --from REV --path P…` materializes historical bytes into the vault. It requires explicit paths (no whole-tree restore without `--all`), previews each path as `identical` / `differs` / `create` / `absent-in-rev`, refuses every `differs` path unless selected per-path or covered by `--overwrite-differing`, carries the live fingerprint as a precondition on every write, auto-checkpoints first, and runs as one H transaction. Paths present in the vault but absent from the revision are **never deleted**.

`sync backup create --out FILE [--with-history] [--with-trash]` writes a tar containing: the working tree minus exclusions; a `git bundle` of all refs when `--with-history` (default on if a repo exists); `.mg-vault/sync.json`, `.mg-vault/refactor/history/`, and `.mg-vault/sync/conflicts/` (app state that is cheap and useful); `.mg-vault/trash/` only under `--with-trash`; and `MANIFEST.json` (v1) listing every member's relative path, byte length, mode, and SHA-256, plus a `manifest_digest`, tool version, timestamp, vault root identity, and the exclusion-set digest. After writing, M re-reads the archive and recomputes every digest **before** printing a success verb. A backup is never reported good without being read back. The receipt states plainly when the destination is on the same filesystem as the vault.

`sync backup verify FILE [--deep]` recomputes all member digests and the manifest digest; `--deep` additionally runs `git bundle verify` on the embedded bundle. `sync backup restore FILE --into DIR` defaults to a **new empty directory**. `--into-vault NAME` requires the same preview, per-path confirmation, fingerprint preconditions, transaction, and pre-restore checkpoint as `sync restore`, never deletes files absent from the backup (`--prune-extra` routes each deletion through A's trash with per-path confirmation), and refuses entirely if verification fails.

#### Recovery after a crash

1. Every vault-resolving invocation stats both `ACTIVE` markers. If either is present, mutating commands refuse and name the recovery command.
2. **Refactor recovery runs first**, always: `mg-vault refactor recover` converges the working tree to a complete pre- or post-state using H's digest-driven replay, including quarantine-on-drift.
3. `mg-vault sync recover` then reads `.mg-vault/sync/journal/<id>.json`. Because M's file materialization lives inside the H transaction, the only M-specific durable steps outside it are the fetch (idempotent, objects only) and the final ref update. The journal therefore resolves to one of:
   - `fetched: true, materialized: false` → nothing to do; report `rolled_back` and drop the marker. The fetched objects are harmless.
   - `materialized: true, ref_updated: false` → the working tree is correct and git history is behind. M **does not reconstruct a commit silently**; it reports `sync_incomplete: working tree materialized, history not advanced`, names both intended parents, and offers `mg-vault sync checkpoint --reason recovered --parents <a> <b>`, which is an explicit, confirmed step that writes no working-tree byte.
   - `materialized: true, ref_updated: true` → complete; drop the marker.
4. Recovery is idempotent, replayable, and writes no working-tree byte of its own.

### 3.3 Layout descriptions

Every human view uses one fact per line, leading literal label, then value; tables are space-aligned at ≥ 60 columns and become one-record-per-block below 60. **Nothing is truncated** — commit ids, paths, digests, remote names, detector names, and recovery commands wrap with a two-space continuation indent. JSON never varies with terminal width. Paths are rendered through A's escaping contract so a hostile filename cannot emit terminal control sequences.

`sync status` — fixed reading order: vault → repository → branch/head → remote → ahead/behind with fetch evidence → worktree counts → excluded → syncthing-conflicts → git-conflicts → transaction → last-checkpoint → index (informational).

```
vault: notes  (/home/j/notes)
repository: present   branch: main   head: 9f2c1ab
remote: origin  ssh://git@example.com/j/notes.git
ahead: 2   behind: 3   as-of-fetch: 2026-08-29T13:41:07Z (48m ago)
  counts are as of the last fetch; run `mg-vault sync pull --fetch-only` to refresh
worktree: modified 4  added 1  deleted 0  untracked 2
excluded: 17 paths (0 with local changes)
syncthing-conflicts: 2 unreconciled
git-conflicts: 0 unresolved
transaction: none active
last-checkpoint: 2026-08-29T09:12:44Z  (reason: manual, 6 files, 3ab91f0)
index: current (generation 8412)   [informational; sync never reads the index]
```

Conflict record — every field labeled, both sides always named, the closing guarantee always printed:

```
conflict 1 of 2   id: c-7f13a90c
  kind: git-merge      class: conflict:frontmatter
  path: projects/roadmap.md
  reason: both sides changed bytes inside the YAML frontmatter envelope;
          frontmatter is never merged line by line
  base:   sha256:1f0a…  1842 bytes  (merge base 3c1d99e)
  ours:   sha256:9b21…  1901 bytes  (worktree, modified 2026-08-29T11:02:14Z)
  theirs: sha256:44ce…  1877 bytes  (origin/main 77aa10d)
  frontmatter differences (semantic, not textual):
    status    ours: "active"   theirs: "archived"
    tags      ours: [a, b]     theirs: [a, b, c]
    reviewed  ours: (absent)   theirs: 2026-08-28
  body: 2 overlapping hunks — see `mg-vault sync resolve show c-7f13a90c --diff`
  all three sides stored at .mg-vault/sync/conflicts/c-7f13a90c/{base,ours,theirs}
  resolve with one of:
    mg-vault sync resolve take c-7f13a90c --side ours
    mg-vault sync resolve take c-7f13a90c --side theirs
    mg-vault sync resolve take c-7f13a90c --side both --as projects/roadmap.remote.md
    mg-vault sync resolve take c-7f13a90c --merged-file /tmp/roadmap.merged.md
  nothing has been written to projects/roadmap.md
```

Diff blocks open with `file: <path>` and `status: modified|added|deleted|renamed|binary`, then `@@` hunk headers, then lines with literal leading `+`, `-`, or space. Binary files print `binary: 12_402 bytes, not shown` rather than bytes. A capped diff ends with `12 more hunks not shown; narrow with --path or raise --diff-limit` — never silent elision.

Data sources: A's registry and path authority (vault identity), the git process wrapper (repo state, history, trees, merge computation), a fresh confined filesystem read (all live bytes and classifications), the conflict store and M journal (conflicts and recovery), the exclusion evaluator (excluded counts). No M view is driven by B's index.

Empty states are sentences: `No git repository in this vault. Run: mg-vault sync init`; `No history yet. Run: mg-vault sync checkpoint`; `No conflicts. 0 git, 0 Syncthing.`; `No remotes configured.`; `No backups found in <dir>.`; `Nothing to push — up to date with origin/main as of fetch <ts>.`; `No incomplete sync transaction.`

TUI sync panel (hosted by E): a status header identical in content to `sync status`, a conflict list on the leading side, a detail pane on the trailing side rendering the same record as `sync resolve show`, and a confirm affordance that produces the same confirmation value the CLI prompt produces. The panel renders M's JSON contract and adds no semantics of its own.

### 3.4 Input & gestures

- **Modality:** keyboard and argv only. Touch, mouse, stylus, voice, camera, and controller input are N/A for a CLI/TUI feature.
- **Grammar:** `mg-vault sync <verb> [OPERANDS] [FLAGS]`; verbs `status`, `init`, `remote`, `checkpoint`, `history`, `show`, `diff`, `push`, `pull`, `conflicts`, `resolve`, `restore`, `backup`, `exclude`, `scan`, `recover`.
- **Global flags:** A/C's `--vault`, `--json`, `--no-input`, `--no-color`, `--quiet`. M adds `--dry-run`, `--yes`, `--allow CLASSES`, `--progress auto|plain|none`, `--diff-format unified|linear`, `--diff-limit N`, `--no-checkpoint`, `--git-binary PATH`.
- **`--no-input`** guarantees no prompt, no `/dev/tty` read, no editor launch, and no credential prompt (`GIT_TERMINAL_PROMPT=0`, `GIT_ASKPASS` pointed at a failing helper), so a missing credential is a clean `auth_required` error rather than a hung process. `push`/`pull`/`restore`/`resolve`/`backup restore` refuse under `--no-input` without `--yes`, and refuse with any conflict class present even with `--yes`.
- **stdin/stdout:** `sync show`/`diff` write diff text to stdout suitable for piping into `less` or `delta`. `sync resolve take --merged-file -` reads merged bytes from stdin, consumed only when `-` is given. `--json` output is one object per invocation.
- **Color:** disabled by `--no-color`, by `NO_COLOR` with any value, and automatically when stdout is not a TTY. Git children always run with `-c color.ui=false` and `--no-pager`, so no ANSI or pager can leak from a subprocess.
- **TUI keybindings** (in E's sync panel, following E's grammar): `j`/`k` move in the conflict list, `Enter` opens detail, `o`/`t`/`b` stage the ours/theirs/both decision, `d` toggles the diff, `x` applies the staged decision behind a confirm, `r` refreshes status (local only), `g p` opens the guarded pull flow, `g P` the guarded push flow, `?` shows the panel's keymap. No decision is applied by a single keystroke; every mutating key routes through the same confirmation value the CLI uses.
- **Responsive behavior:** layouts at 40/60/80/120 columns per §3.3; the TUI panel collapses to a single stacked column below 80 columns with the detail pane reachable by `Enter`.

### 3.5 Transitions & animation

- There is no navigation animation, cursor addressing, alternate screen, sound, or haptic in the CLI views; output is append-only lines.
- Long operations (`fetch`, `push`, `backup create`, `--rescan`-class walks) print static, rate-limited progress to **stderr**, at most 4 updates per second, only when stderr is a TTY and `--progress` resolves to `auto`. `--progress plain` emits whole discrete lines with no carriage-return redraw — the correct mode for logs and screen readers. `--progress none` is the automatic value for non-TTY, `--quiet`, `--json`, and `NO_COLOR`-with-non-TTY, and is what `--no-input` implies. Progress never appears on stdout and never carries content, only counts, byte totals, and elapsed time.
- Reduced-motion alternative: `--progress plain` plus the final receipt is a complete substitute; no information exists only in the animated form.
- In E's sync panel, transitions follow E's reduced-motion setting; the panel is fully usable with all motion disabled, since every state is a labeled line.
- Cancellation: `SIGINT` before the transaction commit point cancels with zero working-tree writes (fetched objects may remain and are harmless). During `push`, a late kill yields `state: unknown` and a named resolution command, never a success verb.

### 3.6 Error states

| Code | Trigger | Presentation and recovery | Data-loss risk |
|---|---|---|---|
| `git_unavailable` | No `git` on `PATH`, or version below the 2.38 floor required for `merge-tree --write-tree` | stderr block naming the detected version and the floor; all non-git M commands (Syncthing conflicts, backup, exclude) still work | No |
| `repository_absent` | M command needing a repo in a vault with no `.git` | Names `mg-vault sync init`; status still reports everything else | No |
| `git_state_unsupported` | Mid-rebase/merge/cherry-pick/bisect or detached HEAD from the user's own git usage | Refuses all mutating M commands; tells the user to finish or abort it with their own git; M never cleans up state it did not create | No |
| `git_locked` | `.git/index.lock` held by another process | Names the lock path; M never removes a git lock file | No |
| `transaction_incomplete` | Either `ACTIVE` marker present | Names the kind, the transaction id, and the exact recovery command; blocks all mutating M commands | No if the protocol holds |
| `conflict_unreconciled` | Syncthing artifact touches a path the merge would change | Lists artifact and live paths; requires `sync resolve` first | No |
| `merge_conflict` | Any `conflict:*` class in a pull plan | Full conflict records; **zero writes to the conflicted paths**; the rest of the plan may still be applied if the user confirms | No — both sides preserved |
| `secret_detected` | `deny`-class scan finding on staged content or an outgoing range | File, line, detector name, redacted excerpt (first 4 / last 4 chars); commit or push refused; remediation printed | No |
| `push_not_fast_forward` | Remote has commits the local branch lacks | Diverged counts; instructs to pull; M never force-pushes | No |
| `auth_required` | Remote rejected credentials or a helper is missing | Names the redacted remote and that credentials are delegated to git's helper/SSH agent; M stores none | No |
| `network_unavailable` | DNS/TCP/TLS failure during an explicit sync | Redacted host, error class, retry suggestion; local state unchanged | No |
| `precondition_failed` | A file's fingerprint changed between plan and commit | Lists every drifted path; writes nothing; instructs to re-run the command | No |
| `backup_verification_failed` | Digest mismatch on create-readback or `verify` | Names the first mismatching member and both digests; a failed create is reported as failed and the artifact marked `.unverified` | No |
| `restore_would_overwrite` | A `differs` path without per-path selection or `--overwrite-differing` | Lists every such path with both digests; writes nothing | No |
| `excluded_path_requested` | User asked to commit or materialize an excluded path | Names the deciding exclusion layer and rule | No |
| `hook_modified_worktree` | Opted-in hooks changed files during a commit | Lists drifted paths and digests; the operation is reported incomplete | No — bytes still on disk, digests reported |
| `io` / `permission_denied` / `out_of_space` | Filesystem failure at any phase | Names the step and whether the change is committed, uncommitted, or `unknown`; never a success verb | No false success |

Presentation choice: every error is a single stderr block. A CLI has no banner, toast, or modal, and errors inside stdout would contaminate pipes and JSON consumers. Under `--json` the same information is C's error envelope on stderr with stdout empty, so a consumer reading only stdout cannot mistake a failure for an empty success. No error message ever contains note bytes, diff hunks, a full secret, or a credentialed URL.

### 3.7 Accessibility

- **Screen reader / linear reading:** every view is plain lines with a leading literal label; reading top to bottom conveys 100% of the information. There are no interactive widgets in the CLI, so roles and traits are N/A; in E's sync panel each list row is announced as `conflict <n> of <m>, <path>, class <class>, unresolved`, and the detail pane is a linear reading of the same record.
- **Custom actions:** in the TUI panel each conflict row exposes the named actions `take ours`, `take theirs`, `keep both`, `show diff`, and `abandon` as an explicit action list rather than requiring memorized chords; the CLI equivalents are printed inside every conflict record.
- **Color independence:** no state is conveyed by color anywhere. Diff lines carry literal `+`/`-`/space prefixes and each file block carries `status: <word>`. Push/pull outcomes are conveyed by the outcome verb and the exit code. Stripping all ANSI leaves every output semantically complete — asserted by test.
- **Glyph independence:** no state depends on a Unicode symbol, box character, or emoji; tables are space-aligned and parse without borders.
- **Text scaling / dynamic type:** N/A in a terminal; the equivalent obligation is width tolerance, met by the ≥ 60 / < 60 layouts and tested at 40, 60, 80, and 120 columns with nothing truncated.
- **Focus order / keyboard navigability:** the shell owns focus for CLI views; every M capability is reachable from argv with no prompt except the deliberate confirmations, and `--yes` makes even those scriptable within the safety envelope of §3.2. In the TUI panel, focus order is header → conflict list → detail pane → confirm affordance, and every action has a key.
- **`--diff-format linear`** emits diffs as sentences (`file projects/roadmap.md, hunk 1 of 3, lines 40 to 52; removed: "…"; added: "…"`) for users for whom column-aligned diffs are unreadable. This is a complete textual equivalent, not a summary.
- **Textual equivalents (5C):** M produces no graph, canvas, image, or spatial view. Progress, status, and conflict state are text by construction; the animated progress form carries no information absent from `--progress plain` and the final receipt.

---

## 4. Implementation Specification

### 4.1 Architecture placement

- **New crate `crates/mg-vault-sync/`** — all of M. Modules:
  - `git/mod.rs` — the process wrapper: argv construction, the subcommand allowlist, environment scrubbing, timeouts, exit-status typing, `--porcelain`/`-z` parsing.
  - `git/capability.rs` — binary discovery, version floor, feature probing, `git_unavailable` gating.
  - `status.rs` — repository state, worktree summary, fetch-evidence-stamped ahead/behind.
  - `checkpoint.rs` — pathspec staging, staged-set verification, trailer convention, commit creation.
  - `plan.rs` — `merge-tree`-based pull planning and per-path classification.
  - `merge.rs` — the merge policy: protected regions, frontmatter semantics, never-merge classes.
  - `conflict.rs` — the unified conflict store, records, retention, resolution application.
  - `syncthing.rs` — artifact detection, classification, reconciliation.
  - `exclude.rs` — the layered exclusion evaluator and the `.gitignore` composition/divergence check.
  - `scan.rs` — the secret detector set, findings, allowlist, redaction.
  - `backup.rs` — archive creation, manifest, readback verification, restore planning.
  - `journal.rs` — M's thin journal (fetch/materialize/ref-update phases) layered over H's transaction record.
  - `recover.rs` — the recovery decision table of §3.2.
- **`crates/mg-vault-core/`** — unchanged authority. M consumes `Vault`, `SourceFingerprint`, `atomic::{create_atomic, replace_atomic, sync_parent}`, and (once H lands) `transaction::{plan, commit, recover}`. M adds exactly one core API: `Vault::write_coexistence_path`, a privileged, opt-in-gated writer for `.obsidian` (§4.6), mirroring how A gates `.mg-vault/trash`.
- **`crates/mg-vault-cli/`** — a `sync` command module plus rendering. It owns no policy.
- **Dependency direction:** `mg-vault-cli → mg-vault-sync → mg-vault-core`. `mg-vault-sync` must not depend on `mg-vault-index`, and must expose no API that mutates a vault path outside the transaction engine.

Filesystem layout owned by M, all under the vault-local, portable, owner-only `.mg-vault/sync/`:

```
.mg-vault/sync.json                        # versioned M configuration (v1)
.mg-vault/sync/ACTIVE                      # id of the one in-flight sync transaction
.mg-vault/sync/journal/<id>.json           # M journal record, fsynced per phase
.mg-vault/sync/conflicts/<id>/base|ours|theirs   # all three sides, always retained
.mg-vault/sync/conflicts/<id>/record.json  # conflict record (v1)
.mg-vault/sync/receipts/<id>.json          # resolution and sync receipts
.mg-vault/sync/scan-allow.json             # per-finding allowlist, digest-keyed
```

Directories are `0700`, files `0600`. Every schema is versioned; an unknown newer version fails closed and is never rewritten by an older binary.

### 4.2 Data model

```rust
// Author: Jeff
// Date: 2026-08-29
// Description: Sync, conflict, and recovery types for the M feature
// Notes: No type here is note identity; commit ids and conflict ids are operational handles

// --- git process boundary ---

/// One validated git invocation. Constructed only by the wrapper, never by callers.
pub struct GitCommand {
    subcommand: GitSubcommand,       // allowlisted variant, not a free string
    args: Vec<OsString>,             // argv only; no shell string is ever built
    network: Option<NetworkGate>,    // Some(..) required for fetch/push/ls-remote
}

/// Proof that a network-capable git call was requested by an explicit user command.
/// Private constructor; no plugin, AI adapter, index response, or deserialized value can mint one.
pub struct NetworkGate(());

// --- status ---

/// Repository state as observed without contacting any remote.
pub struct SyncStatus {
    pub repository: RepositoryState,             // Absent | Present | Unsupported(String)
    pub branch: Option<String>,
    pub head: Option<CommitId>,
    pub remote: Option<RemoteSummary>,           // name + redacted URL
    pub ahead: Option<u32>,
    pub behind: Option<u32>,
    pub fetch_observed_at: Option<Timestamp>,    // evidence for ahead/behind; never omitted
    pub worktree: WorktreeCounts,
    pub excluded_paths: u32,
    pub syncthing_conflicts: u32,
    pub git_conflicts: u32,
    pub transaction: TransactionState,
    pub last_checkpoint: Option<CheckpointSummary>,
}

// --- pull planning ---

/// One path's classification in a pull plan. Every variant states whether bytes are written.
pub enum PathClass {
    Unchanged, IncomingOnly, LocalOnly, CleanMerge,
    ConflictOverlap, ConflictFrontmatter, ConflictFenced, ConflictStructured,
    ConflictDeleteVsModify, ConflictRenameVsModify, LocalDirty, Excluded,
}

/// A complete, previewable pull plan. Planning writes nothing to the working tree.
pub struct SyncPlan {
    pub version: u16,
    pub remote: RemoteSummary,
    pub incoming: Vec<CommitSummary>,
    pub merge_base: CommitId,
    pub entries: Vec<PlanEntry>,                 // one per affected path, never elided
    pub checkpoint: Option<CommitId>,
    pub plan_hash: String,                       // domain-separated SHA-256 over executable fields
}

/// One planned path with its preconditions and its post-image digest.
pub struct PlanEntry {
    pub path: NotePath,
    pub class: PathClass,
    pub live_fingerprint: Option<SourceFingerprint>,   // precondition, revalidated at commit
    pub post_digest: Option<String>,
    pub conflict_id: Option<ConflictId>,
}

// --- conflicts ---

/// A conflict of either origin. Both (or all three) sides are always materialized on disk.
pub struct ConflictRecord {
    pub version: u16,
    pub id: ConflictId,
    pub kind: ConflictKind,                      // GitMerge | SyncthingArtifact
    pub path: NotePath,
    pub class: PathClass,
    pub reason: String,                          // plain English, printed verbatim
    pub sides: Vec<ConflictSide>,                // base/ours/theirs, or live/incoming
    pub frontmatter_diff: Option<Vec<KeyDelta>>, // semantic, never a textual merge
    pub opened_at: Timestamp,
    pub resolution: Option<Resolution>,          // set on resolve; losing bytes stay retained
}

pub struct ConflictSide {
    pub label: SideLabel,                        // Base | Ours | Theirs | Live | Incoming
    pub fingerprint: SourceFingerprint,
    pub byte_len: u64,
    pub provenance: String,                      // "worktree", "origin/main 77aa10d", device id
    pub stored_at: PathBuf,                      // inside .mg-vault/sync/conflicts/<id>/
}

// --- exclusion and scanning ---

/// Layered, deny-biased exclusion decision. One evaluator serves staging, backup, and export.
pub struct ExclusionSet { builtin: GlobSet, user: GlobSet, gitignore: GitignoreView }
pub struct ExclusionDecision { pub excluded: bool, pub layer: ExclusionLayer, pub rule: String }

/// One secret-scan finding. Never carries the matched secret bytes.
pub struct ScanFinding {
    pub path: NotePath, pub line: u32, pub detector: &'static str,
    pub severity: Severity,                      // Deny | Warn
    pub redacted: String,                        // first 4 + "…" + last 4 characters
    pub match_digest: String,                    // sha256 of matched bytes; allowlist key
}

// --- journal and backup ---

/// M's thin phase record. File materialization itself lives in H's transaction journal.
pub struct SyncJournal {
    pub version: u16, pub id: SyncId, pub operation: SyncOperation,
    pub fetched: bool, pub checkpoint: Option<CommitId>,
    pub transaction: Option<TransactionId>, pub materialized: bool,
    pub intended_parents: Vec<CommitId>, pub ref_updated: bool,
}

/// Backup manifest, v1. Every member is digested; the manifest itself is digested.
pub struct BackupManifest {
    pub version: u16, pub tool_version: String, pub created_at: Timestamp,
    pub vault_identity: String, pub exclusion_digest: String,
    pub members: Vec<BackupMember>,              // path, byte_len, mode, sha256
    pub bundle: Option<BackupMember>, pub manifest_digest: String,
}
```

**Migrations:** `sync.json` v1, conflict record v1, journal v1, receipt v1, backup manifest v1. All additive-or-fail-closed. **No note-content migration exists or may ever exist**, and no schema owns note bytes. No identifier is written into a note: `ConflictId`, `SyncId`, `TransactionId`, and commit ids are operational handles only.

### 4.3 API contracts

```rust
// Observation is read-only and never opens a socket.
fn status(vault: &Vault, opts: StatusOptions) -> Result<SyncStatus>;
fn history(vault: &Vault, filter: HistoryFilter) -> Result<Vec<CommitSummary>>;
fn diff(vault: &Vault, spec: DiffSpec) -> Result<DiffReport>;

// Local history. Writes git objects and refs; writes no working-tree byte.
fn checkpoint(vault: &Vault, req: CheckpointRequest) -> Result<CheckpointReceipt>;

// Planning is side-effect free apart from the fetch that precedes it.
fn fetch(vault: &Vault, remote: &RemoteName, gate: NetworkGate) -> Result<FetchReceipt>;
fn plan_pull(vault: &Vault, fetched: &FetchReceipt) -> Result<SyncPlan>;
fn plan_restore(vault: &Vault, req: RestoreRequest) -> Result<SyncPlan>;
fn plan_backup_restore(vault: &Vault, archive: &Path, req: RestoreRequest) -> Result<SyncPlan>;

// Application requires an in-invocation confirmation value and runs as one H transaction.
fn apply(vault: &Vault, plan: &SyncPlan, ok: ConfirmedSync) -> Result<SyncReceipt>;
fn push(vault: &Vault, req: PushRequest, gate: NetworkGate, ok: ConfirmedSync)
    -> Result<PushReceipt>;

// Conflicts. Listing and showing never mutate; resolution is one explicit decision.
fn conflicts(vault: &Vault, filter: ConflictFilter) -> Result<Vec<ConflictRecord>>;
fn resolve(vault: &Vault, id: &ConflictId, choice: ResolutionChoice, ok: ConfirmedSync)
    -> Result<ResolutionReceipt>;

// Exclusion and scanning. Pure functions over paths and bytes.
fn exclusion_set(vault: &Vault) -> Result<ExclusionSet>;
fn scan_bytes(path: &NotePath, bytes: &[u8]) -> Vec<ScanFinding>;

// Backup. Create verifies by readback before returning Ok.
fn backup_create(vault: &Vault, req: BackupRequest) -> Result<BackupReceipt>;
fn backup_verify(archive: &Path, deep: bool) -> Result<VerifyReport>;

// Recovery is idempotent and writes no working-tree byte.
fn recover(vault: &Vault, mode: RecoverMode) -> Result<RecoveryReport>;
```

Binding contract rules:

- **One planner.** `--dry-run`, the interactive preview, and the applied run all call the same `plan_*` function. There is no second, weaker path.
- **A plan is not authorization.** `apply` revalidates every `live_fingerprint`, every destination, every path's confinement, and the merge base from freshly opened handles immediately before the first write. Drift returns `precondition_failed` and writes nothing.
- **`ConfirmedSync` and `NetworkGate` are private core types** constructible only from an in-invocation prompt answer or an explicit `--yes`/explicit sync verb. No plugin, AI adapter, index response, TUI event, or deserialized JSON can mint either (4C, 5A).
- **Exit codes** follow C's stable categories with no additions: `0` success or cancel-without-change, `2` usage/confirmation, `3` not found, `4` collision/ambiguity/conflict, `5` unsafe or denied (including `secret_detected`), `6` degraded dependency (`git_unavailable`, `network_unavailable`), `7` I/O or transaction failure, `130` interrupt. Exit code and JSON error code are asserted together.
- **JSON `data` objects:** `sync.status` → the `SyncStatus` fields; `sync.pull` dry-run → `{dry_run:true, plan, plan_hash, changed:false}`; applied → `{dry_run:false, plan_hash, transaction_id, files_written, conflicts_opened, commit, durability:"synced", changed:true}`; `sync.push` → `{remote, redacted_url, commits, refs_updated, state:"accepted"|"unknown"}`; `sync.conflicts` → `{conflicts:[…]}`; `sync.scan` → `{findings:[…], deny:N, warn:N}`; `sync.backup.create` → `{path, members, bytes, manifest_digest, verified:true}`. Every failure adds `error.details.transaction_id` and `phase` when a transaction was open.
- **Pagination** applies to `history` (`--limit`/`--after`, cursor bound to the ref tip) and to diff hunks (`--diff-limit` with an explicit "N more" line). Auth and rate limiting are N/A locally; remote authentication is entirely git's, and M stores no credential.

### 4.4 State management

- **Authoritative state:** the ordinary files in the working tree. Always. Git objects, refs, remote-tracking refs, backups, conflict copies, and journals are derived or historical, and losing all of them costs history and sync, never a note byte. An explicit test deletes `.git` entirely and asserts every note is byte-identical and every non-git M command still works.
- **Owner:** `mg-vault-sync` owns per-invocation state only. There is no daemon, no background thread, no timer, no cache of tree state between invocations. Every classification re-reads live bytes at use time.
- **Local vs. server-synced boundary:** everything M holds is local. The "server" is a git remote that is contacted only inside `fetch`/`push`/`ls-remote`, and no remote state is ever treated as authoritative for the working tree — a remote's version of a file becomes a *candidate* in a plan, never a write.
- **Transaction model:** M does **not** implement a transaction engine. Every working-tree materialization is one `mg-vault-core::transaction` plan with `operation: SyncMaterialize`, inheriting H's staging store, write-ahead journal, single commit-point barrier, canonical step order, digest-driven idempotent replay, and quarantine-on-drift. M layers only the phase record of §4.2 (`fetched` → `checkpoint` → `transaction` → `materialized` → `ref_updated`).
- **Ordering rule (binding), interaction with H:** fetch → checkpoint → plan → H transaction → ref update. The ref update is strictly last. Combined with the single `ACTIVE` marker per vault, this yields: a pull can never land mid-refactor, because H holds `ACTIVE` from its commit point to its terminal record and M refuses while it exists; a refactor can never start mid-pull for the same reason.
- **The uncovered window is defined, not hand-waved.** H deliberately holds no lock across its human preview. A pull that materializes files during that preview is therefore possible, and the outcome is specified: H's mandatory commit-time revalidation finds mismatched fingerprints and refuses with `conflict`, listing the drifted paths and requiring a replan. Nothing is lost and nothing is silently merged. The symmetric case — an H commit during M's preview — fails M's own `precondition_failed` check. Both directions are tested (§5.2).
- **Locks and their order:** `.mg-vault/sync/locks/git.lock` (advisory, held across any git invocation that writes objects or refs, and across a whole fetch-plus-plan) is always acquired **before** `.mg-vault/locks/refactor.lock`, never the reverse, so no deadlock is constructible. Git's own `.git/index.lock` is a third, git-owned lock that M never removes; if held, M reports `git_locked` and stops. Because external editors and plain `git` honor none of these locks, safety never rests on them — it rests on commit-time fingerprint revalidation.
- **Recovery order:** refactor recovery first, then sync recovery, per the decision table in §3.2. Recovery is idempotent, replayable, and never writes a working-tree byte whose current digest differs from a recorded precondition.
- **Offline/draft persistence:** conflict sides, plans (`--plan-out`), and the scan allowlist persist under `.mg-vault/sync/`. Conflict retention defaults to the tightest of 90 days, 200 conflicts, or 128 MiB, with the 20 most recent always retained; `sync conflicts forget ID` is the immediate-deletion escape hatch for content a user does not want lingering.
- **Index relationship (B):** M never reads the index and never lets it influence a decision — classification comes from git's object database plus fresh source reads. After materializing, M emits one informational line that the index is now stale, and, if B's IPC exposes a reconcile hint, calls it best-effort with a short deadline and ignores failure. B's correctness never depends on that hint; its watcher and fingerprint reconciliation cover the change regardless.

### 4.5 Dependencies

**Git integration mechanism — decision and justification.** M **shells out to the user's `git` binary** with argv arrays (never a shell string), rather than linking `git2`/libgit2 or embedding `gix`.

- **Against `unsafe_code = "forbid"`:** libgit2 is a C library reached through `git2-sys` FFI. The workspace lint constrains our crates, but linking libgit2 imports roughly a hundred thousand lines of C — with a real CVE history in its packet, index, and transport paths — into the same address space as every vault operation, entirely outside Rust's memory-safety guarantees and outside clippy's reach. A subprocess boundary is a process boundary: a malformed pack or hostile ref name crashes a child, not the tool holding the user's files open.
- **Against the deny-level clippy posture:** a thin typed wrapper over `Command` is trivially `clippy::pedantic`-clean. libgit2's handle-and-raw-pointer model is not, and would push us toward per-call allowances in exactly the code that must be most auditable.
- **For 1A (git must never become the authority):** the user's own `git` sees precisely what mg-vault sees. There is no second implementation of ignore rules, ref resolution, or merge computation to drift from git's behavior, and no risk of mg-vault writing objects a plain `git` disagrees about. Every user remedy in every error message is a command they can run themselves.
- **For 5A (no ambient network):** libgit2 compiles a network stack (libssh2/OpenSSL or equivalent) into every mg-vault binary, including builds of users who never sync. Shelling out means the only socket in the product belongs to a child process we spawn only from `push`, `pull`, and `remote test`.
- **`gix` was considered and rejected for now:** pure Rust and architecturally attractive, but M depends on `merge-tree --write-tree` semantics, full `.gitignore` composition, credential-helper delegation, and push-side ref negotiation, and betting the safest subsystem in the product on parity across all four is a risk with no user-visible benefit. Recorded as Q2 with a re-evaluation trigger.
- **Consequences accepted and mitigated:** a runtime dependency on `git >= 2.38` (probed at startup; `git_unavailable` with a clear message and full non-git functionality otherwise); a version-skew surface (machine-readable formats only — `--porcelain=v2`, `-z`, `--format=%H%x00…` — never human output parsing); and a spawn boundary (argv arrays only, no `sh -c`, no interpolation).

Every git child runs with: `--no-pager`; `-c color.ui=false`; `-c core.hooksPath=<empty dir>` unless hooks are opted in; `-c protocol.allow=never` on **every non-network invocation**, so an accidental transport attempt fails instead of connecting; `--no-recurse-submodules`; `GIT_TERMINAL_PROMPT=0`; `GIT_ASKPASS` pointed at a failing helper under `--no-input`; a scrubbed environment retaining only `PATH`, `HOME`, `SSH_AUTH_SOCK`, `GIT_*` allowlisted variables, and locale set to `C`; a per-invocation timeout; and captured stdout/stderr with a size cap.

- **New Rust crates:** `globset` (pure-Rust glob matching for the exclusion layers), `tar` (backup archive), `zstd` or `flate2` (optional backup compression — see Q4), `regex` (secret detectors, with compiled-once static patterns and a size/backtracking-safe engine). `sha2`, `serde`, `serde_json`, `thiserror`, `clap`, and `rustix` are already present.
- **Explicitly not added:** any HTTP, TLS, SSH, or async-runtime crate; any telemetry; any credential store; any libgit2 or embedded git implementation.
- **Assets:** none. No fonts, images, models, or bundled binaries.
- **Infrastructure:** none owned by M. A git remote and a Syncthing installation are user-provided and optional; the product is fully functional with neither.

### 4.6 Platform-specific considerations

- **Primary target:** Arch Linux; portable to Linux and macOS. Windows is out of scope for this milestone (path semantics, `RENAME_NOREPLACE`, directory `fsync`, and `.gitattributes` line-ending translation all differ enough to need their own gate).
- **Line endings:** mg-vault requires `core.autocrlf=false` and refuses to operate on a repository configured otherwise (`git_state_unsupported` with the exact `git config` command to fix it). CRLF translation would make git's blob differ from the working-tree bytes, breaking fingerprint equality and violating 1B token preservation. `.gitattributes` `text`/`eol` attributes on `*.md` are detected and refused for the same reason.
- **Case-insensitive filesystems:** destination checks use case-folded comparison in addition to exact comparison; a pull whose incoming tree contains two paths differing only in case is refused as `conflict:structured` rather than silently collapsing one onto the other.
- **Unicode normalization:** identity is the exact byte path; nothing is NFC/NFD-normalized. On macOS, a pull whose incoming path normalizes onto an existing path is refused, not merged.
- **`.obsidian` coexistence (1D):** by default `.obsidian/**` is in the managed exclusion set — not committed and not materialized — so it can never diverge between git and the tree, and A's prohibition on mutating it is never violated. `sync.manage_obsidian = true` opts in; materialization into it then goes through `Vault::write_coexistence_path`, a privileged core API unreachable from any generic note path, with its own confirmation class. Machine-local files (`workspace.json`, `workspace-mobile.json`, `.obsidian/plugins/**/data.json`) stay denied even when the opt-in is on, the last because plugin data files commonly carry API tokens.
- **Syncthing directories:** `.stfolder/`, `.stversions/`, and `~syncthing~*` are always excluded. `.stversions/` is additionally never scanned for conflicts, since it is Syncthing's own history area.
- **Network filesystems and coarse timestamps:** all correctness rests on content fingerprints, never on mtime or rename cookies, so a vault on SMB/NFS/Syncthing behaves identically, just slower.
- **Feature flags / rollout:** `mg-vault-sync` ships behind a build feature that is on by default; a build without it reports `sync: unavailable in this build` in `--version` and `status` rather than silently offering weaker guarantees. Safety behavior, JSON shape, and error codes never vary by build.

### 4.7 Performance budget

Reference: warm local SSD, `git >= 2.38`, reported with hardware/OS metadata; the 100,000-note / 1,000,000-block fixture from B.

- **Startup:** M adds two `stat` calls (`.mg-vault/sync/ACTIVE`, `.mg-vault/refactor/ACTIVE`) to any vault-resolving invocation, target ≤ 1 ms. No daemon, no background thread, no timer, no fetch.
- **`sync status`:** p95 ≤ 400 ms warm on the 100k fixture, ≤ 1.5 s cold; `--fast` (`--untracked-files=normal`) p95 ≤ 250 ms. The docs recommend `core.untrackedCache=true` and `feature.manyFiles=true`, and status reports when they are off.
- **`sync history --limit 50`:** p95 ≤ 150 ms. **`sync diff` for one file:** p95 ≤ 100 ms.
- **`sync checkpoint`** with ≤ 500 changed files: p95 ≤ 2 s including the staged-set verification and the secret scan.
- **Secret scan:** ≥ 40 MiB/s single-threaded over staged content; detectors are compiled once into a single `RegexSet` pass; a file over `scan.max_file_bytes` (default 16 MiB) is skipped with an explicit `skipped_too_large` finding rather than silently.
- **`sync pull` planning** for ≤ 1,000 changed paths: p95 ≤ 5 s excluding network. Fetch time is git's and is reported separately so a slow network is never attributed to mg-vault.
- **Memory:** ≤ 150 MiB RSS for any M command. Diffs, merges, and backups stream per file; a file over `sync.max_file_bytes` (default 64 MiB) is never loaded whole and is classified `conflict:structured`. Conflict sides are written to disk, not held in memory.
- **Backup:** ≥ 80 MiB/s uncompressed create; verification readback roughly doubles the I/O and is not optional. A 1 GiB vault backs up in ≤ 40 s including verification.
- **Storage:** git objects grow with history (checkpoints of an unchanged tree cost one ~200-byte commit object); conflict retention bounded per §4.4; backups bounded by the user's `--keep N` policy. Nothing grows without an explicit user action.
- **Network payload:** exactly what git negotiates; M adds nothing and uses no shallow clone, since a truncated history breaks merge-base computation.
- **Cancellation:** every long operation is cancellable and checks cancellation at least every 50 ms or 1 MiB. Cancellation before the transaction commit point mutates nothing.
- **Editing is never blocked:** M runs no daemon and holds its locks only across git object writes and the transaction apply window, so a 100k-note vault stays editable throughout.

---

## 5. Test Specification

### 5.1 Unit tests

| Test | Setup → assertion (edge case) |
|---|---|
| `git_subcommand_allowlist_rejects_worktree_writes` | Attempt to construct `GitCommand` for `checkout`, `merge`, `reset`, `pull`, `stash`, `clean`, `apply`, `filter-repo` → each fails to compile-or-construct; a runtime table test asserts the allowlist has exactly the documented members (working-tree write prevention) |
| `network_gate_cannot_be_forged` | Attempt to construct `NetworkGate` from a deserialized value, a plugin context, and a TUI event → all rejected; only the three sync verbs produce one (5A, 4C) |
| `non_network_invocations_carry_protocol_never` | Snapshot the argv of every non-network git call → each contains `-c protocol.allow=never` (defense in depth) |
| `checkpoint_trailer_roundtrip` | Build a checkpoint message, parse it back → subject, reason, txn, and receipt survive; a foreign commit without the trailer is not classified as a checkpoint |
| `staged_set_must_equal_intended_set` | Inject an extra staged path between `add` and `commit` → the commit is aborted before an object exists (pathspec integrity) |
| `exclusion_denies_builtin_over_negation` | `.gitignore` contains `!secrets.env` → the built-in deny still wins; the decision names the layer and rule (deny-by-default) |
| `exclusion_matches_git_check_ignore` | Cross-validate 500 fixture paths against `git check-ignore -z --stdin` → zero divergences for `.gitignore` layers |
| `syncthing_pattern_precision` | 40 fixture names including near-misses (`note.sync-conflict.md`, `note.sync-conflict-2026-08-29-140311-K5J3ABC.md`, a note legitimately named `…sync-conflict…`) → only exact-format names match |
| `frontmatter_is_never_line_merged` | Both sides edit different keys in one envelope → class is `conflict:frontmatter`, not `clean-merge`, even though the hunks are disjoint |
| `fence_straddling_hunk_is_conflict` | A merge hunk starting inside a fenced block → `conflict:fenced`; the fence is the conflict unit |
| `canvas_and_binary_never_text_merged` | `.canvas` and PNG fixtures changed on both sides → `conflict:structured`; no line merge attempted |
| `detectors_flag_known_secrets` | Fixture file per detector (PEM private key, `AKIA…`, `ghp_…`, `xoxb-…`, `AIza…`, `sk_live_…`, JWT, `.env` shape) → each detected with the right name and severity |
| `detectors_do_not_flag_ordinary_notes` | 200-note corpus with base64 images, sha256 digests, UUIDs, and lorem text → zero `deny` findings (false-positive budget) |
| `findings_never_carry_the_secret` | Serialize every finding → matched bytes appear nowhere; only first-4/last-4 and a digest |
| `redacted_url_strips_userinfo` | `https://user:token@host/p` and `ssh://git@host/p` → rendered without credentials in status, receipts, errors, and logs |
| `plan_hash_covers_executable_fields_only` | Reorder warnings and change timestamps → hash stable; change one entry's post digest → hash changes |
| `backup_manifest_digests_every_member` | Flip one byte in one archived member → `verify` names that member and both digests |
| `conflict_record_versioning_fails_closed` | A `version: 2` record → refused with a doctor finding, never rewritten |
| `ansi_stripping_preserves_meaning` | Strip all ANSI from status, diff, and conflict output → semantically complete (color independence) |
| `width_matrix_never_truncates` | Render at 40/60/80/120 columns → every id, path, digest, and recovery command present in full |

### 5.2 Integration tests

- `pull_writes_nothing_on_any_conflict_class`: for each `conflict:*` class, run a full pull and assert the conflicted path's bytes are byte-identical to the pre-run digest and all sides exist in the conflict store.
- `no_conflict_markers_anywhere`: after the full two-machine round trip, grep the entire vault for `<<<<<<<`, `=======`, `>>>>>>>` → zero matches in any tracked file.
- `both_sides_survive_resolution`: resolve each conflict every way (`ours`, `theirs`, `both`, `merged-file`) and assert the losing bytes are recoverable byte-for-byte from the conflict store afterward.
- `syncthing_artifact_is_never_auto_touched`: run `status`, `history`, `checkpoint`, `push`, `backup`, `scan`, and `recover` with artifacts present → the artifacts are byte-identical, still present, and never appear in any commit.
- `syncthing_identical_sidecar_needs_confirmation`: a byte-identical sidecar still requires one confirmation before removal.
- `pull_blocked_by_unreconciled_artifact_on_same_path`: assert `conflict_unreconciled` and zero writes.
- `restore_refuses_newer_source`: modify a file after the checkpoint, restore from that checkpoint without `--overwrite-differing` → `restore_would_overwrite`, zero writes, both digests reported. With per-path selection, assert the pre-restore checkpoint captured the newer bytes first.
- `backup_restore_never_deletes_extra_files`: restore a backup that lacks a file present in the vault → the file is untouched and listed under `extra_in_vault`; `--prune-extra` routes it to A's trash, never `unlink`.
- `backup_create_fails_loudly_on_readback_mismatch`: fault-inject a corrupted write → the command reports failure, the artifact is marked `.unverified`, and no success verb is printed.
- `secret_blocks_commit_and_push`: plant a `deny` secret; assert `checkpoint` refuses (exit 5) and, when the secret is introduced by plain `git`, `push` refuses over the outgoing range.
- `already_committed_secret_reports_without_rewriting`: assert the report names commits, paths, and pushed status, prints rotation-first remediation and the `git filter-repo` command as **text**, and that no history-rewriting or force-push git invocation was made.
- `excluded_content_never_leaves`: with secrets in excluded paths, run checkpoint + push + backup + scan; assert the secret bytes appear in no git object, no pushed pack, no backup member, and no log line.
- `pull_during_refactor_preview_is_refused_at_h_commit`: start an H preview, land a pull, then confirm the refactor → H refuses with `conflict` listing drifted paths, zero writes.
- `refactor_during_pull_preview_is_refused_at_m_commit`: the symmetric case → `precondition_failed`, zero writes.
- `active_marker_blocks_the_other_subsystem`: with `refactor/ACTIVE` present, every mutating M command refuses; with `sync/ACTIVE` present, every mutating H and C command refuses.
- `crash_matrix_converges`: kill at each of fetch, post-checkpoint, mid-transaction (each step boundary), post-transaction/pre-ref-update, and post-ref-update → next invocation converges to a complete pre- or post-state; the post-transaction/pre-ref-update case reports `sync_incomplete` and offers the explicit repair, never reconstructing a commit silently.
- `deleting_git_loses_no_bytes`: `rm -rf .git`, then assert every note byte-identical, `status` reports `repository: absent`, and Syncthing/backup/exclude commands still work.
- `git_index_disagreement_defers_to_worktree`: desynchronize the git index, then plan → classification comes from live bytes, not the index.
- `offline_suite`: run the entire non-`sync-push`/`sync-pull` test suite under a socket-denial harness; any `connect(2)` fails the run.
- `fake_git_shim_records_no_network_verbs`: with a recording `git` shim on `PATH`, run every non-sync command in the whole product → no invocation contains `fetch`, `push`, `clone`, `ls-remote`, or `remote update`.
- `unroutable_remote_does_not_slow_local_commands`: point the remote at an unroutable address with a zero timeout → `status`, `history`, `diff`, `checkpoint`, `conflicts`, `backup`, `scan` all complete within budget.

### 5.3 UI / E2E tests

- CLI E2E per view in both human and `--json`: status (repo absent / present / unsupported), history (empty and populated), diff (unified and linear), push (dry-run, refused non-ff, accepted, unknown), pull (fetch-only, clean, conflicted, `--no-input` refusal), conflicts (empty, git, Syncthing, mixed), resolve (all four choices), restore, backup (create/verify/restore), exclude, scan, recover.
- Golden JSON fixtures per command and per error code, asserted alongside the exit code, with volatile fields (timestamps, ids, durations) normalized. These fixtures are the compatibility contract.
- Confirmation E2E over a PTY: assert the escalated prompt appears for any plan with writes, that `--no-input` without `--yes` refuses, that `--yes` with a conflict class refuses regardless, and that a `--allow`-constrained plan proceeds.
- TUI E2E in E's sync panel: navigate the conflict list, open detail, stage each decision, cancel, apply behind the confirm, and assert the resulting file state matches the CLI path exactly.
- Error-recovery E2E: interrupt a pull at the transaction boundary, then run the recommended recovery command verbatim from the error message and assert it converges.

### 5.4 Visual / manual verification

- Widths 40, 60, 80, 120 for status, history, diff, and conflict records; assert nothing truncated and that below 60 columns each view becomes one-record-per-block.
- `NO_COLOR`, `--no-color`, non-TTY redirection, and a screen-reader linear transcript for every view; assert no essential state depends on color, glyph, column position, or a redrawing progress line.
- `--progress auto|plain|none` on a slow fetch and a large backup; assert `plain` emits discrete whole lines and `none` emits nothing to stderr.
- Empty vs. populated: no repository, no remote, no history, no conflicts, no backups; and a 12-conflict mixed inbox.
- Long Unicode paths, combining characters, RTL filenames, and a filename containing terminal control sequences; assert escaping and no cursor movement.
- Terminal light and dark themes (M emits no background colors; verify legibility of the diff prefixes without color).
- Manual degraded states: `git` absent, `git` below the version floor, mid-rebase repository, `.git/index.lock` held, `.mg-vault` read-only, disk full during backup, remote unreachable, credential helper missing.

---

## 6. Compliance & Safety Gate

### 6.1 Sensitive data classification

- [x] **Handles sensitive data.** M touches the most sensitive material in the product: complete note bodies (in conflict sides, backups, and git objects), diff hunks, file paths, remote URLs that may embed credentials, and — by definition — any credential a user pasted into a note. Protections: everything under `.mg-vault/sync/` is created `0700`/`0600`; conflict sides and backups are local files the user chose the location of; M stores, reads, and prompts for no credential and delegates entirely to git's credential helper and the SSH agent; remote URLs are redacted (userinfo stripped) in every status line, receipt, error, and log; log and error output carries only paths, codes, digests, counts, and durations — never note bytes, diff hunks, scan match text, or environment values; the secret scanner never persists a matched value, only a redacted excerpt and a digest; excluded content passes through one evaluator shared by staging, backup, and export, so a path excluded from a commit is excluded from a backup by construction; `sync conflicts forget ID` and backup deletion are explicit user-controlled erasure paths. An automated test asserts that a full push/pull/backup/scan cycle over a secret-bearing fixture emits no secret byte to any log, archive, or pushed object.
- [ ] No sensitive data involvement.
- [ ] Uses synthetic/test data only until compliance gate clears.

### 6.2 Asset provenance

- [x] **No third-party assets.** M ships no fonts, images, models, icons, sounds, or bundled data. It depends on the user's own `git` binary, which it does not distribute, and on Rust crates (`globset`, `regex`, `tar`, a compression crate) whose licenses are MIT/Apache-2.0-compatible and are recorded in the dependency-license policy — a policy that remains blocked on the unresolved project license decision carried from spec A (§8-Q1 there). Secret-detector patterns are written for this project from public credential-format documentation and carry no third-party license.
- [ ] Uses third-party assets.

### 6.3 Language / claims audit

- [ ] Makes claims not supported by evidence — **no.** Every success verb (`checkpointed`, `pushed`, `pulled`, `restored`, `resolved`, `backed up`, `verified`) is emitted only after the corresponding durable step returns: a commit after `update-ref`, a materialization after the transaction's terminal record, a backup after readback verification. Uncertain outcomes print `state: unknown` and a resolution command. Ahead/behind counts always carry their fetch timestamp rather than implying currency.
- [ ] Promises capabilities not yet built — **no.** §7 states plainly that every part of M is absent today and that H's transaction engine, which M builds on, is also absent. `manage_obsidian`, deep backup verification, and the TUI panel are described as target behavior with their gates named.
- [ ] Uses language restricted by domain regulations — **no.** M makes no medical, financial, legal, or safety claim. The word "atomic" refers precisely to the journal-bounded, same-filesystem transaction protocol H defines and M reuses, and is fault-tested rather than asserted. "Backup" is qualified: the receipt states when the destination shares a filesystem with the vault, and verification is mandatory rather than implied.

### 6.4 Regulatory alignment

The binding standard here is `docs/specs/QUALITY-CRITERIA.md`. Lens 3 (Knowledge Retrieval and Structure), which the template names explicitly:

- **3A Determinism.** M derives nothing that retrieval consumes. Its own classifications derive from git's content-addressed object database plus fresh confined source reads, so the same inputs always produce the same plan (asserted via `plan_hash`). M never mutates a link, a tag, or a property.
- **3B Ambiguity.** M's ambiguity is conflict, and it never resolves one silently: every `conflict:*` class stops with both sides preserved and requires an explicit per-conflict decision. `--yes` cannot resolve a conflict; no `--strategy ours/theirs` exists. A link ambiguity created by a pull that adds a same-basename note is G's to report; M does not rewrite links, so it cannot introduce one silently through an edit.
- **3C Query depth.** M exposes history query surfaces — `sync history --path/--limit/--after/--checkpoints-only`, `sync show REV`, `sync diff --rev A..B --path P` — over commits, checkpoints, paths, and dates. It adds no predicate to G's search grammar and no query of its own over note content.
- **3D Derived authority.** Git objects, refs, remote-tracking refs, conflict copies, backups, and journals are all views or copies over ordinary files; none may write the working tree except through A's atomic primitives inside H's transaction. Deleting `.git` and all of `.mg-vault/sync/` leaves every note byte-identical, which is tested.
- **3E Scale.** Budgets in §4.7 cover the 100,000-note fixture: status is bounded and has a `--fast` mode, planning is O(affected paths), memory is independent of vault size, large files are refused rather than loaded, and M runs no daemon and holds locks only across object writes and the apply window, so editing is never blocked.

Auto-fail rules, mapped:

| Auto-fail rule | How M forecloses it |
|---|---|
| Source-content loss | Git may never write the working tree (§3.2 allowlist); every write is an atomic replacement with a fingerprint precondition inside H's transaction; conflict sides are retained after resolution; restore never deletes files absent from the source revision |
| Partial multi-file mutation | M implements no transaction engine — every materialization is one H transaction with staging, a write-ahead journal, a single commit-point barrier, and digest-driven idempotent replay; `crash_matrix_converges` proves convergence |
| Index state overriding source | M never reads the index; classification comes from git objects plus live bytes; `git_index_disagreement_defers_to_worktree` proves the working tree wins |
| Stale index presented as current | M presents no index data as its own; ahead/behind always carries its fetch timestamp, and `--refresh` is the only, explicit way to update it |
| **Silent conflict winner** | No automatic merge resolution that discards a side exists anywhere: no `git merge`, no `-X ours/theirs`, no `--strategy`, no marker injection. Every conflict stops the affected path with zero writes; all three sides are materialized and retained; resolution is one explicit command; `--yes` and `--no-input` cannot resolve one. Syncthing artifacts are never auto-deleted, auto-renamed, auto-merged, or auto-committed |
| Unknown syntax loss | Only `clean-merge` regions outside protected areas are ever composed; frontmatter, fenced and math blocks, `.canvas`, binary, and oversized files are never text-merged; untouched bytes are copied verbatim by A's span-preserving write |
| Unconfirmed overwrite/import | Push and pull both preview and confirm; escalated confirmation for any plan that writes; restore defaults to a fresh directory and refuses every `differs` path without per-path selection; `--no-input` refuses rather than assuming yes |
| Unsafe traversal or symlink escape | Every path M touches is validated through A's path authority and (once landed) the descriptor-relative `VaultDir` backend; incoming tree entries that escape the root, name `.mg-vault`, or are symlinks/gitlinks are refused before planning |
| Capability/data-exfiltration bypass | `NetworkGate` and `ConfirmedSync` are private types no plugin, AI adapter, index response, or deserialized value can mint; the exclusion evaluator is deny-biased and its built-in layer is not overridable by `.gitignore` negation |
| Active raw HTML/script by default | N/A — M renders no HTML and executes nothing; git hooks are disabled by default precisely so no repository content can execute during an M operation |
| Non-atomic save claiming success | Success verbs follow the durable step; a killed push reports `state: unknown`; a backup is reported good only after readback verification |
| **Recovery overwriting newer source** | Restore and backup-restore carry the live fingerprint as a precondition on every write, default to a fresh directory, refuse `differs` paths without explicit per-path selection, auto-checkpoint first, and never delete extras; recovery never writes a path whose current digest matches neither the recorded pre- nor post-image — it quarantines and reports |
| Graph/Canvas lacking a textual equivalent | N/A — M produces no graph or spatial view; `.canvas` files are treated as opaque, never merged, and K owns their semantics |

### 6.5 Security controls

- Treat every remote ref name, tree entry path, commit message, and archive member name as untrusted input; validate shape before it reaches a path join, and escape terminal control characters before display.
- Never build a shell string; spawn `git` with argv arrays, a scrubbed environment, a timeout, and capped output.
- Never store, read, prompt for, or log a credential; delegate to git's helper and the SSH agent, and redact userinfo from every URL that reaches a human, a log, or a receipt.
- Never rewrite history, never force-push, never delete a remote ref, never remove a git lock file, and never clean up a repository state M did not create.
- Keep `unsafe_code = "forbid"` and denied `clippy::all` + `clippy::pedantic` for `mg-vault-sync`; the subprocess boundary is what keeps that credible with no FFI.
- Bound every resource: scan and merge file-size caps, output caps on git children, conflict-store retention, diff hunk caps, and archive member count/size limits, each refusing rather than degrading.
- Where the outcome cannot be proven, refuse and quarantine; no automatic repair may discard the only known copy of any bytes.

---

## 7. Gap Analysis vs. Current State

### 7.1 What exists today

State words are exact.

**Absent** — every part of M. There is no `mg-vault-sync` crate, no `sync` command, no git invocation anywhere in the product, no checkpoint, no history view, no diff renderer, no merge policy, no conflict store, no Syncthing artifact detection, no exclusion evaluator, no secret scanner, no backup or restore, no M journal, and no recovery command. `crates/mg-vault-cli/src/main.rs` (658 lines) exposes only `vault`, `note`, `index`, `search`, and `interop` verbs. `crates/mg-vault-core/src/error.rs` has no sync-related variant. No dependency in `Cargo.toml` opens a socket or spawns a process, and `docs/PRODUCT.md` and `README.md` both state plainly that there is "no sync adapter". The only git in the repository is the development repository itself and a three-line `.gitignore` (`/target`, `*.swp`, `*.tmp`) that has nothing to do with vault content.

**Absent (upstream, and blocking)** — H's multi-file transaction engine, which M builds on rather than duplicating: `crates/mg-vault-core/src/transaction/` does not exist, there is no write-ahead journal, no staging store, no undo store, no `ACTIVE` marker, and no digest-driven recovery. A's `trash_note`/`restore_note` today do best-effort inline rollback with ignored `let _ = fs::rename(...)` results and no durable record of intent.

**Implemented** — the foundation M consumes: `Vault::open` with canonical confinement, `validate_note_path` (relative, normal components, `.md`, `.obsidian`/`.mg-vault` rejected for mutation), `atomic::{create_atomic, replace_atomic, sync_parent}` with `renameat2(RENAME_NOREPLACE)`, `SourceFingerprint` (SHA-256), `read_note`/`write_note` with fingerprint preconditions, trash and collision-refusing restore, the XDG registry, and the version-1 JSON envelope with `--json`/`--no-input`/`--no-color`. Commits `bb2b723` → `dfe33cf`; tests in `crates/mg-vault-core/tests/foundation.rs` and `crates/mg-vault-cli/tests/cli.rs`.

**Prototyped** — `crates/mg-vault-core/src/index.rs::read_source_bytes` uses `openat2` with `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS`, proving the descriptor-relative primitive M's confinement will use, though the mutation path does not yet use it. `crates/mg-vault-index` provides disposable SQLite generations with observational freshness — the right shape for the freshness-honesty pattern M reuses for ahead/behind, and a subsystem M deliberately does not read.

**Gated** — nothing in M is gated today, because nothing exists to gate. On landing, the whole crate sits behind a default-on build feature (§4.6).

### 7.2 Delta to spec

**New files / modules**

- `crates/mg-vault-sync/` with `git/{mod,capability}.rs`, `status.rs`, `checkpoint.rs`, `plan.rs`, `merge.rs`, `conflict.rs`, `syncthing.rs`, `exclude.rs`, `scan.rs`, `backup.rs`, `journal.rs`, `recover.rs`, plus `Cargo.toml` inheriting the workspace lints.
- `crates/mg-vault-cli/src/commands/sync.rs` and `output/sync_{human,json}.rs`.
- `crates/mg-vault-sync/tests/{allowlist,exclusion,scan,merge_policy,syncthing,conflict,backup,crash_matrix,offline}.rs`.
- `tests/fixtures/sync/` — two-machine repo fixtures, conflict corpora (overlap, frontmatter, fence, canvas, binary, delete-vs-modify, rename-vs-modify), a secret corpus with a false-positive corpus, Syncthing artifact names including near-misses, and golden JSON envelopes.
- A recording `git` shim and a socket-denial harness under `tests/support/`.

**Modified files**

- `crates/mg-vault-core/src/error.rs` — add `GitUnavailable`, `RepositoryAbsent`, `GitStateUnsupported`, `GitLocked`, `ConflictUnreconciled`, `MergeConflict`, `SecretDetected`, `PushNotFastForward`, `AuthRequired`, `NetworkUnavailable`, `PreconditionFailed`, `BackupVerificationFailed`, `RestoreWouldOverwrite`, `ExcludedPathRequested`, `HookModifiedWorktree`.
- `crates/mg-vault-core/src/vault.rs` — add `write_coexistence_path`, the privileged opt-in-gated `.obsidian` writer, unreachable from any generic note path.
- `crates/mg-vault-core/src/transaction/` (H) — add the `SyncMaterialize` operation kind and a `sync_context` field on the journal record. No other change; M must not fork the engine.
- `crates/mg-vault-cli/src/main.rs` — register the `sync` command tree; add the `sync/ACTIVE` stat to vault resolution alongside H's.
- `Cargo.toml` — add the `mg-vault-sync` member and the `globset`, `regex`, `tar`, and compression workspace dependencies.
- `README.md`, `docs/PRODUCT.md`, `docs/ARCHITECTURE.md`, `docs/SECURITY.md` — record the git-subcommand allowlist, the explicit-network rule, the exclusion set, and the recovery ordering, only once each lands.

**Migrations / schema changes**

- New versioned on-disk schemas: `sync.json` v1, conflict record v1, M journal v1, receipt v1, backup manifest v1, scan allowlist v1. All fail closed on an unknown newer version and are never rewritten by an older binary. No note-content migration exists or may ever exist.

**New dependencies**

- `globset`, `regex`, `tar`, and one compression crate (Q4). A runtime dependency on the user's `git >= 2.38`, probed and gated. No network, TLS, SSH, async, or credential-store crate.

### 7.3 Estimated scope

**XL.** The command surface is large but the correctness surface is larger: a git process boundary with an enforced allowlist and environment discipline; a merge policy that must classify Markdown structurally and refuse everywhere it cannot prove safety; a conflict store that must preserve every side through every path including crashes; Syncthing reconciliation with a precise pattern and a no-touch guarantee; a deny-biased exclusion evaluator cross-validated against git itself; a secret scanner with a real false-positive budget and a redaction guarantee; a verified backup and restore that must never overwrite newer source; and a crash matrix crossed with H's own crash matrix. None of it can be validated by ordinary unit tests, and four separate auto-fail rules run straight through it.

Recommended gated increments, each behind the same allowlist and transaction contract: **M1** git wrapper + capability gate + `status`/`history`/`diff` (read-only, no network); **M2** exclusion evaluator + secret scanner + `checkpoint`; **M3** Syncthing detection and reconciliation (needs no git at all, and is independently valuable); **M4** `pull` planning, merge policy, conflict store, and resolution; **M5** `push` with the outgoing-range scan; **M6** backup, restore, and recovery. No increment may print a success verb before its own fault matrix is green.

### 7.4 Blocking dependencies

- **H Safe note refactoring — required and currently absent.** M builds every working-tree write on H's transaction engine, journal, staging store, `ACTIVE` marker, and digest-driven recovery. M must not ship a second transaction format. M4, M5, and M6 are blocked on H1 (the transaction engine and recovery with any planner). M1, M2, and M3 are not blocked, because they write no working-tree byte.
- **A Foundation — partially implemented, and its journal slice is required.** A's transaction journal, descriptor-relative `VaultDir` confinement backend, `durability_unavailable` gate, and stable exit categories all land beneath H and therefore beneath M.
- **C CLI and note operations — required.** M reuses C's envelope v1, exit-code categories, `--dry-run`/`--yes`/`--no-input` semantics, and status/doctor surfaces.
- **E TUI workspace — dependent, not blocking.** E's sync panel consumes M's JSON contract; M is fully usable from the CLI without E.
- **B Index service — neither required nor consulted.** M's only interaction is an informational staleness line and an optional best-effort reconcile hint.
- **F Markdown model — desirable, not blocking.** Until F's token model lands, `merge.rs` uses a conservative scanner whose documented rule is to widen the conflict unit rather than guess: anything it cannot classify with certainty becomes a conflict, never a clean merge.
- **K Canvas — not blocking.** `.canvas` is treated as opaque and never merged; a semantic canvas merge driver is K's to provide later.
- **External gate:** the project license decision carried from spec A blocks the dependency-license policy sign-off and a `LICENSE` file, not implementation.

### 7.5 Non-goals

- No background sync, no daemon, no timer, no autostart, no startup fetch, and no fetch as a side effect of any editing command.
- No history rewriting, force push, `filter-repo`/BFG execution, ref deletion, submodule support, LFS, or shallow clone.
- No credential storage, credential prompting, or an mg-vault-specific auth mechanism.
- No proprietary sync protocol, no server component, no real-time collaboration, and no CRDT.
- No automatic conflict resolution of any class, no merge strategy flags, and no conflict markers written into user files.
- No `.gitignore` rewriting without an explicit command, and no modification of `~/.gitconfig`.
- No Obsidian Sync, Git plugin, or Syncthing configuration management — M reads Syncthing's artifacts, it does not control Syncthing.
- No mutation of `.obsidian` unless the user explicitly opts in, and never of its machine-local files.

---

## 8. Open Questions

- **Q1:** Should `sync.checkpoint.auto_before` default to `["refactor","restore","import","pull"]` as specified, or ship empty so that no commit is ever created without the user asking? Auto-checkpointing is purely additive and is what makes restore meaningful, but it does silently grow history and it embeds note content in git objects for users who only wanted Syncthing. — **blocks:** the shipped default in §3.2 and §4.4, not the design.
- **Q2:** Confirm the shell-out-to-`git` decision in §4.5 and its re-evaluation trigger. Proposed trigger: revisit `gix` when it offers stable `merge-tree`-equivalent tree merging, full `.gitignore` composition, credential-helper delegation, and push-side ref negotiation, with a migration that keeps the same allowlist semantics. — **blocks:** §4.5 and the M1 wrapper design.
- **Q3:** Should `.mg-vault/sync.json` be JSON (consistent with A's registry, no new dependency) or TOML (far more pleasant to hand-edit, and this is the first user-facing config file in the product)? The spec currently assumes JSON. — **blocks:** §4.1 layout and the dependency list.
- **Q4:** Backup compression — uncompressed tar (fastest, simplest, verifiable member-by-member), `zstd`, or `gzip`? And should backups be encrypted at rest, given they contain complete note bodies and are likely to be copied to external media? Encryption would introduce key management, which this product has deliberately avoided everywhere else. — **blocks:** §4.5 dependencies and the §3.2 backup format.
- **Q5:** Should the `warn`-class scan allowlist in `.mg-vault/sync/scan-allow.json` key entries by a digest of the matched bytes (proposed — the allowance evaporates when the secret changes) even though that stores a weak fingerprint of a secret in a file, or by `(path, detector, line)` (leakier semantics, no digest)? — **blocks:** §3.2 scan behavior and §4.2.
- **Q6:** When a pull's incoming side deletes a file the user has not modified (`incoming-only` deletion), should M delete it as part of materialization, or route it through A's trash so it is recoverable? Trashing is safer but leaves the working tree differing from the merged tree until the trash entry is purged, which then shows as a permanent local difference. — **blocks:** §3.2 pull classification.
- **Q7:** Should `sync push` be blocked, not merely warned, when unreconciled Syncthing artifacts exist anywhere in the vault? The artifacts are excluded from the pushed content, so it is safe mechanically, but pushing while one side of a conflict sits unread may not match user intent. — **blocks:** §3.2 Syncthing rule 4.
- **Q8:** `sync.manage_obsidian` defaults off, so `.obsidian` is neither committed nor materialized. Is that the right default for a user who wants their Obsidian appearance and hotkeys to follow the vault across machines, or should the default be "committed but never materialized without confirmation"? — **blocks:** §4.6 coexistence default.
