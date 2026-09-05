# Spec: Plugins and Automation

**Feature ID:** n-plugins-automation
**Parent feature:** root
**Spec author agent:** Plugins/automation spec agent
**Date:** 2026-08-29
**Iteration:** 1

---

## 1. Purpose

### 1.1 One-sentence job

Let a user run third-party code against their vault — new commands, generated content, external tool bridges — inside a WebAssembly sandbox that has **no** ambient filesystem, network, process, clock, or randomness access, so that every byte a plugin reads, writes, sends, or executes was authorized by a specific, scoped, revocable capability the user granted after being shown exactly what it permits.

### 1.2 Why it matters

This is the feature that decides whether `mg-vault` is trustworthy. Every other branch is careful — **A** confines paths, **H** journals multi-file writes, **J** deliberately refuses to make templates a scripting language and defers real computation here, **C** makes confirmation a private type no caller can forge. A plugin host is where all of that is either enforced or thrown away, because a plugin is by definition code the user did not write, running against a directory that may contain a journal, client notes, or a pasted credential.

The ecosystem this competes with sets the bar low in an instructive way. Obsidian plugins are Node/Electron JavaScript with the app's full filesystem and network privileges; there is no capability model, no scope, no per-permission consent, and no attribution — a plugin that silently POSTs a note to a server is indistinguishable from one that does not, and the user's only real control is "installed" or "not installed". `mg-vault` cannot repeat that trade and still claim criterion **4C**. The user pain is concrete: *"I want the one plugin that renders my expense table, and I do not want to find out later that it also uploaded my journal."*

So this feature exists to make three sentences literally true: **a plugin can do nothing by default; everything it can do was named in a prompt the user read; and everything it did is attributable, previewed, bounded, and undoable.** It also exists so that the rest of the product can stay simple — because there is a real place to put computation, `mg-vault`'s templates, queries, and CLI never need an `exec` escape hatch.

### 1.3 Success signal

An adversarial plugin corpus (§5.2) — 30+ hostile components attempting ambient WASI access, path traversal, symlink escape, scope-boundary reads, control-directory writes, infinite loops, unbounded allocation, host-call reentrancy, shell metacharacter injection into an external command, undeclared network egress, terminal escape injection, and partially-applied multi-file mutation — runs under a syscall-monitoring harness and produces **zero** successful accesses outside granted capabilities, **zero** bytes written to any path outside a granted scope, **zero** child processes not matching a user-confirmed argv vector, **zero** partially applied multi-file mutations, and **zero** editor or host crashes. Every one of the 30+ is either refused with a typed `plugin_*` error naming the capability, or trapped and contained with the vault byte-identical to its pre-invocation state. Concurrently, the measured overhead is ≤ 1 ms added keystroke latency with 8 plugins loaded.

---

## 2. User Stories

> As a note-taker, I want to install a plugin that adds a `:wordcount` command, and have it work without granting it any ability to read my notes beyond the open buffer, so that a trivial plugin stays trivially safe.

> As a cautious user, I want to see, before I grant anything, one plain-English line per permission with its exact scope — "read notes under `work/clients/**`", "send data to `api.example.com:443`" — and a sentence saying that those two together mean this plugin can upload those notes, so that I am consenting to the actual consequence rather than to a word.

> As someone who granted too much last month, I want `plugin grants` to list every permission I ever gave, and `plugin revoke` to take one back immediately and kill any running instance, so that consent is reversible rather than a one-way door.

> As a user whose plugin just proposed changes to nine notes, I want a diff preview attributed to that plugin, applied as one journaled transaction or not at all, and undoable as one step, so that third-party code can never leave my vault half-edited.

> As a user whose plugin has a bug, I want an infinite loop or a memory explosion to be killed by the host, leave my unsaved buffer intact, and disable the plugin after it fails three times, so that a bad plugin degrades the feature and never the editor.

> As a plugin author, I want a versioned WIT world, a semver'd host API, a documented deprecation window, and an error that names the exact unsupported version when my plugin is too new, so that I can ship against a stable contract instead of guessing.

> As a scripting user on a headless box, I want `--no-input` to refuse to grant anything implicitly and `plugin grant --yes` to require me to restate the exact scope, so that automation can never widen my permissions by accident.

> As a screen-reader user, I want the consent prompt, the risk marker, the fault message, and the plugin list to be literal words on their own lines with no color-only or glyph-only meaning, so that I can evaluate a security decision without sight.

---

## 3. UX Specification

`mg-vault` is a CLI plus a TUI. There are no GUI screens, windows, dialogs, mouse-first surfaces, sounds, or haptics anywhere in this feature. §3 is terminal UX: command grammar, line-oriented output, one TUI overlay, and one prompt.

### 3.1 Command / view inventory

| View | Invocation (CLI) / TUI path | New / modified | Shape |
|---|---|---|---|
| Plugin inventory | `plugin list [--all] [--json]` · TUI `:plugins`, `<leader>Pl` | New | One row per plugin: id, version, state, granted-capability count, health |
| Plugin detail | `plugin show ID [--json]` · TUI overlay `Enter` on a row | New | Manifest fields, artifact digest, API requirement, per-capability grant status, last 5 audit lines |
| Install result | `plugin install SOURCE [--version V] [--digest D] [--yes]` | New | Manifest summary → **consent prompt** → receipt with id, version, digest, granted set |
| **Consent prompt** | Emitted by `install`, `grant`, first use of an undeclared-at-install capability, and every artifact-digest change | New | Full-width, blocking, keyboard-only, per-capability answers, default **no** |
| Grant inventory | `plugin grants [ID] [--json]` · TUI `<leader>Pg` | New | One row per (plugin, capability, scope, granted-at, artifact digest) |
| Grant / revoke result | `plugin grant ID CAP [--scope P] [--host H] [--yes]`, `plugin revoke ID [CAP]` | New | Receipt naming exactly what changed; revoke names instances killed |
| Enable / disable / uninstall | `plugin enable\|disable\|uninstall ID [--purge-storage]` | New | Receipt; uninstall states whether grants and storage were removed |
| Run a plugin command | `plugin run ID COMMAND [ARGS…] [--dry-run] [--yes]` · TUI palette entry | New | Plugin output, then any proposal preview and confirmation |
| **Proposal preview** | Any plugin mutation, from CLI or TUI | Modification of H's preview | H's per-file unified diff plus an actor banner `proposed by plugin:<id>@<ver>` |
| **External command confirmation** | Any `process.spawn` invocation | New | Exact resolved argv, one element per line, scrubbed environment listing, then y/N |
| Fault surface | Automatic | New | CLI: stderr block. TUI: status-line flag `plugin!` + `:messages` entry + `:plugins` health column |
| Audit log view | `plugin audit [--plugin ID] [--since T] [--json\|--jsonl]` | New | One record per host call that touched a capability |
| Plugin doctor | `plugin doctor [--json]` · included in `mg-vault doctor` | Modification of C's `doctor` | Runtime backend, host API versions, per-plugin load/verify status, quarantine list |
| Host API report | `plugin api [--json]` | New | Supported world versions, WIT digest, deprecation schedule |
| Quarantine control | `plugin quarantine list`, `plugin quarantine clear ID [--force]` | New | Failure ledger and the reason for each quarantine |

Not owned here: note mutation mechanics (**A**, **H**), palette rendering (**E**), buffer/undo (**D**), index queries (**B**, **G**), AI providers (**O** — an AI adapter is *not* a plugin and does not share this grant store; it has its own consent surface and reuses only this feature's sandbox and audit primitives).

Stream discipline is inherited unchanged from **A**/**C**: human success on stdout, warnings and errors on stderr, `--json` yields exactly one envelope on exactly one stream, and no prompt or ANSI ever appears in a JSON stream.

### 3.2 Interaction flows

#### Installing a plugin (primary flow)

1. `plugin install ./expenses.mgplug` (or `--from-dir`, or a path to a `.wasm` plus `--manifest`). Nothing executes yet.
2. The host reads `plugin.toml`, validates it against manifest schema version 1, computes the artifact's SHA-256, and checks `api` against the supported host-API set. Any failure here — `plugin_manifest_invalid`, `plugin_api_unsupported`, `plugin_id_invalid` — aborts **before** the component is compiled, let alone instantiated.
3. If `--digest sha256:…` was supplied and does not match, abort with `plugin_digest_mismatch` naming both digests. If the artifact carries a detached signature and a trusted key is configured, verify it; an unsigned artifact is **not** rejected but is labeled `signature: none (unverified)` in the consent prompt and in `plugin list`.
4. The component is compiled and **link-checked**: the host builds the linker with exactly the granted-capability import set for a hypothetical full grant and reports every import the component requires. An import outside the host world — `wasi:filesystem/*`, `wasi:sockets/*`, `wasi:cli/environment`, anything unknown — fails with `plugin_missing_import`, naming the import. This is deliberate: *the error tells the user the plugin wanted ambient access.*
5. The **consent prompt** (§3.3) renders every capability the manifest declares, with scope, risk marker, and combination warnings. The default answer is no. Per-capability answers are `y`/`n`; `A` accepts all; `Esc`/`Ctrl-c`/`Enter` on the default declines everything.
6. Declining a capability does not abort the install. The plugin installs with the accepted subset; ungranted capabilities are recorded as `declined` and every host call against one returns `plugin_capability_denied` naming the capability and the `plugin grant` command that would fix it. A plugin that requires a declined capability must degrade or refuse — it may not retry-loop, because each denied call still consumes fuel.
7. Under `--no-input`, install **never** grants anything: the plugin installs with an empty grant set, the receipt lists every declined capability, and the exit code is `0` with a warning. Granting non-interactively requires a separate explicit `plugin grant ID CAP --scope … --yes`, one capability per invocation, which prints the same risk lines to stderr before acting.
8. Artifacts land in `$XDG_DATA_HOME/mg-vault/plugins/<id>/<version>/` — **outside** the vault. Grants land in `$XDG_STATE_HOME/mg-vault/grants/<vault-id>.json`, mode `0600`, keyed by `(canonical vault root, plugin id, artifact digest)`. Consequence: **copying or syncing a vault carries no plugin code and no grants.** A vault may carry a portable *recommendation* list at `.mg-vault/plugins.toml` (id, version, digest, source); opening such a vault prints `this vault recommends 3 plugins; none are installed` and **never** auto-installs or auto-grants.

#### Granting, using, and revoking

1. `plugin grant expenses vault.read --scope 'finance/**'` re-renders the single-capability consent block, including the combination warning if the plugin already holds `net.http`, and requires an explicit answer.
2. Every grant record stores the artifact digest it was made against. On the next load, if the digest changed (an update, or tampering), **all grants for that plugin are suspended** and the user sees an update prompt with a *capability diff*: `unchanged: vault.read finance/**` / `NEW: process.spawn /usr/bin/ledger`. An unchanged capability set can be re-accepted with one keystroke (`s` = same capabilities, accept). A set with any addition requires per-capability answers again. There is no "trust future versions" option.
3. `plugin revoke expenses net.http` removes the record, bumps the vault's grant epoch, and **kills any running instance of that plugin** by setting its epoch deadline to now; the trap is contained per §3.6 and the receipt says `1 running instance terminated`. Revocation is never deferred to the next invocation.
4. `plugin revoke expenses` with no capability removes all of them. `plugin disable expenses` keeps grants but refuses to instantiate. `plugin uninstall expenses` removes the artifact, and removes grants and per-plugin storage unless `--keep-storage`; the receipt states exactly which of the three were removed.

#### A plugin mutating notes (the attribution and preview flow)

1. A plugin never writes. It returns a `mutation-proposal`: an ordered list of `create` / `replace` / `relocate` / `trash` operations, each with the vault-relative path, the expected pre-image fingerprint, and the post-image bytes or spans.
2. The host validates every operation *as the host*, ignoring anything the plugin asserted: each path is resolved through **A**'s descriptor-relative `VaultDir` (`openat2` with `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS | RESOLVE_NO_MAGICLINKS`), and the **resolved, canonical, vault-relative path** — not the string the plugin sent — is matched against the granted `vault.write` scope. Control directories, the secret denylist, and out-of-scope paths are rejected here (§4.3).
3. The surviving operation set is handed to **H** as an ordinary refactor plan. The preview is H's: per-file unified diff, counts, and the standard confirmation. One line is added above it: `proposed by plugin:expenses@1.4.0 (sha256:9f3c…)`.
4. Commit is **H**'s journaled multi-file transaction — the same write-ahead log, staging, undo store, commit barrier, and digest-driven recovery. A plugin trap between the proposal and the commit is irrelevant, because the plugin is no longer running; a crash during the commit is recovered by H's replay. **A partial multi-file plugin mutation is structurally unreachable: the plugin does not hold the transaction.**
5. The receipt records `actor: {kind: "plugin", id, version, artifact_digest}`. `refactor history` and `note history` show it. In the TUI the change is one undo entry labeled `plugin:expenses`. **No attribution is ever written into note content** — no UUID, no `generated-by:` property, no marker comment (criterion 1C).
6. Standing approval exists but is narrow: `plugin trust ID --auto-apply create --scope 'inbox/**'` skips the preview for that operation class and scope only. It is off by default, requires its own consent block, **can never cover `trash` or `relocate`**, and every auto-applied change is announced in the status line as `plugin:expenses changed 2 files — u to undo` and written to the audit log.

#### A plugin calling an external command (the strongly gated flow)

1. The manifest must declare the command as a **template**, not a string: `program` (absolute path, or a bare name resolved once at grant time to an absolute path whose digest is recorded), and `argv` as an array of literal elements and typed holes (`{note_path}`, `{selection}`, `{string:NAME}`, `{plugin_scratch}`). A hole expands to **exactly one** argv element; no hole may add, split, or remove elements.
2. There is no shell. The host never constructs a command *string*, never invokes `sh`, `-c`, `system`, or a login shell, and performs no glob, tilde, brace, backtick, `$`, or variable expansion. A plugin supplying `; rm -rf ~` as a `{string:NAME}` value produces one literal argv element containing those characters. This is asserted by a fixture (§5.1).
3. The program is rejected if it is inside the vault, inside any `mg-vault` cache/data directory, a symlink, setuid/setgid, or writable by group or other. A plugin therefore cannot drop a script into the vault and run it.
4. The environment is **constructed, not inherited**: exactly `PATH=/usr/bin:/bin`, `LC_ALL=C.UTF-8`, `HOME=<per-invocation scratch dir>`, and nothing else. No `LD_PRELOAD`, `LD_LIBRARY_PATH`, `SHELL`, `IFS`, `SSH_AUTH_SOCK`, `GPG_TTY`, no token or API-key variable, no user environment of any kind. cwd is the empty per-invocation scratch directory outside the vault. All inherited file descriptors are closed; stdin is supplied bytes or `/dev/null`; stdout/stderr are captured into bounded buffers (1 MiB each, truncation reported). `no_new_privs` is set via `rustix`. The child gets its own process group and session so it cannot read the terminal or signal `mg-vault`.
5. **Before the first execution the user sees the exact resolved argv**, one element per line, control characters escaped, plus the program's resolved path and digest, the scrubbed environment, the cwd, and the timeout. Default is no.
6. Confirmation caching is keyed by a digest of the **resolved argv vector**, not the template — so `ledger balance` approved once does not approve `ledger --file /home/me/.ssh/id_ed25519 print`. `plugins.external_commands` is `off | ask | trusted` with default `ask`; `--no-external-commands` is a global kill switch; under `--no-input` a spawn with no cached approval fails with `plugin_spawn_denied`.
7. On timeout (default 10 s, manifest may request up to 120 s, shown in the prompt) the child and its whole process group are killed; the plugin receives `spawn-error::timeout`. Exit status, byte counts, and duration are audited; **argument *values* are not audited by default** (§6.1).

#### Plugin-contributed commands in the TUI

Commands appear in **E**'s palette in a `Plugin` namespace with the id shown in the row: `[expenses] Insert month total · plugin:expenses:month-total · <unbound> · available`. Availability is truthful — `unavailable: capability not granted (vault.read finance/**)` or `unavailable: plugin quarantined (3 traps)`. A plugin **may not bind a key**; it may *propose* a default binding, which appears in `plugin show` and is inert until the user accepts it into their keymap. A proposed binding that would shadow a buffer-namespace key in Insert or Replace mode is rejected by **E**'s keymap loader outright — a plugin can never take `dd`, `Esc`, or `Ctrl-c`.

### 3.3 Layout descriptions

**Consent prompt (the security-critical surface).** Full width, no borders required, rendered identically by CLI and TUI. Order top → bottom: identity block, capability block, combination block, answer line.

```
Install plugin: expenses 1.4.0
  source     ./expenses.mgplug
  digest     sha256:9f3c1a…e07b
  signature  none (unverified)
  author     unverified: "A. Ledger <a@example.com>"
  api        requires host plugin API ^1.2  (this host: 1.3)

This plugin is requesting 3 permissions. Default is NO for each.

  1. Read notes                                          RISK: MEDIUM
     scope: finance/**   (matches 214 notes right now)
     It can read the full text of every note under finance/.
     Grant? [y/N]

  2. Send network requests                               RISK: HIGH
     to: api.example.com:443 (https only, GET/POST)
     limits: 30 requests/min, 1 MiB up, 4 MiB down per session
     Grant? [y/N]

  3. Run an external program                             RISK: HIGH
     program: /usr/bin/ledger  (sha256:31aa…)
     argv:    ledger -f {plugin_scratch}/data.dat balance
     env:     PATH, LC_ALL, HOME only — your environment is not passed
     You will be asked again the first time each exact command runs.
     Grant? [y/N]

  WHAT THESE COMBINE TO:
    Permissions 1 and 2 together mean this plugin CAN UPLOAD the text
    of your notes under finance/ to api.example.com. mg-vault cannot
    prevent that once you grant both; it can only show you the traffic
    (plugin audit) and cap the volume.

  [y] grant this one   [n] deny this one   [A] grant all   [Esc] deny all
```

Rules that make this surface honest: risk is a **literal word** (`RISK: HIGH`), never a color; scope is shown expanded with a live match count so `**` is not abstract; the combination block is computed from the accepted-so-far set and re-rendered after each answer; and there is no "remember for all plugins", no "trust this author", and no way to reach a grant without passing through this text.

**`plugin list`** (≥ 60 columns): `ID  VERSION  STATE  CAPS  HEALTH  SIGNATURE`, sorted by id. `STATE` ∈ `enabled | disabled | quarantined | needs-consent | api-unsupported`. `CAPS` is `granted/declared` (e.g. `2/3`). `HEALTH` is `ok`, or `traps:3 last:fuel_exhausted`. Below 60 columns it becomes two lines per plugin with the same literal fields, never truncated. Empty state: `No plugins installed. Run: mg-vault plugin install PATH`.

**`plugin grants`**: `PLUGIN  CAPABILITY  SCOPE  GRANTED  DIGEST`, one row per grant, plus a trailing summary `3 grants across 2 plugins; 1 plugin can both read notes and reach the network`.

**TUI `:plugins` overlay**: E-standard overlay — title row, filter line, scrollable rows, detail pane below. Data source is the host's `PluginRegistry` projection; it holds no note state. Empty state text is identical to the CLI's.

**Fault surface**: the TUI status line gains a `plugin!` segment (part of E's non-droppable set only while a fault is unacknowledged); `:messages` carries the full record. No modal, ever — a plugin fault must not steal focus from an editing buffer.

### 3.4 Input & gestures

- **CLI:** everything is reachable from argv. `--json`, `--no-input`, `--no-color`, `NO_COLOR`, `--vault` behave exactly as in **A**/**C**. `--yes` is accepted only where a grant or spawn is fully restated on the command line.
- **Consent prompt keys:** `y`, `n`, `A`, `Esc`, `Ctrl-c`. Bare `Enter` takes the default (deny). Any other key re-prompts. The prompt reads from `/dev/tty` when stdin is a pipe so that a piped body can never answer it, and refuses entirely if no tty exists.
- **TUI:** `<leader>P` opens the plugin overlay; `l` list, `g` grants, `a` audit, `d` doctor; `Enter` detail, `e` enable, `x` disable, `r` revoke (with confirm), `q`/`Esc` close. Palette entries are reachable by name with no binding. Every one of these is listed in E's generated `docs/TUI.md` table.
- **Specialized input:** N/A — a terminal application has no stylus, controller, voice, or camera input. Mouse is off by default (E's rule) and every plugin action has a keyboard path; there are zero mouse-only paths.
- **Responsive behavior:** ≥ 100 cols full table; 60–99 drops `SIGNATURE` and `DIGEST` columns into the detail pane; < 60 switches to the two-line-per-record form. **The consent prompt never abbreviates**: below 60 columns it wraps each capability's scope and combination text onto continuation lines rather than truncating, because a truncated permission is a lie.

### 3.5 Transitions & animation

There is no animation, spinner, fade, or slide. A running plugin shows a static status-line word `plugin:expenses running` and, after 500 ms, appends the elapsed whole seconds, updated no more than once per second — a counter, not a spinner, so it is meaningful under `--no-color`, in a screen reader, and with reduced motion. Overlays and prompts appear as instantaneous full redraws of their region. Reduced-motion alternative: N/A because there is no motion to reduce; this is asserted by the same test that asserts E emits no animation frames.

### 3.6 Error states

| Code | Trigger | Presentation and recovery | Data-loss risk |
|---|---|---|---|
| `plugin_manifest_invalid` | Unknown manifest version, bad id, malformed capability declaration | stderr block naming field and rule; nothing installed | No |
| `plugin_api_unsupported` | Manifest `api` requirement outside the host's supported set | Names required and supported versions and the `plugin api` command | No |
| `plugin_missing_import` | Component imports anything outside the host world (incl. any `wasi:filesystem`/`wasi:sockets`) | Names each missing import literally; explains that ambient access is not offered | No |
| `plugin_digest_mismatch` | `--digest` mismatch, or on-disk artifact differs from the recorded digest | Both digests printed; plugin is not loaded and grants stay suspended | No |
| `plugin_capability_denied` | Host call for a capability that was never granted or was revoked | Typed error returned to the guest *and* one stderr/`:messages` line naming plugin, capability, and the `plugin grant` fix | No |
| `plugin_scope_violation` | Resolved canonical path outside the granted scope, or a control/secret path | Names the plugin, the capability, and the *scope*, never the resolved outside path | No |
| `plugin_trap` | Guest trap, unreachable, panic, or host-call contract violation | Instance destroyed; status flag + `:messages` + audit record; failure ledger incremented | No |
| `plugin_fuel_exhausted` | Instruction budget consumed | Same containment; message names budget and hook; suggests `plugin run` (higher budget) for user-invoked work | No |
| `plugin_deadline_exceeded` | Epoch (wall-clock) deadline for the hook class | Same containment; names the deadline | No |
| `plugin_memory_exceeded` | `ResourceLimiter` refuses growth past the store cap | Same containment; names requested and cap | No |
| `plugin_quarantined` | Failure ledger threshold reached, or a load-time integrity failure | Plugin disabled; `plugin quarantine list` shows the ledger; `plugin quarantine clear ID` re-enables | No |
| `plugin_network_denied` | Host, port, scheme, method, or resolved IP outside the grant | Names which rule failed; request never sent | No |
| `plugin_network_budget_exceeded` | Rate, per-request, or per-session byte budget exceeded | Request refused; plugin trapped if it retries past the budget; audit records the attempt | No |
| `plugin_spawn_denied` | No spawn grant, no cached argv approval under `--no-input`, or a rejected program path | Shows the argv that would have run and why it was refused | No |
| `plugin_spawn_timeout` | Child exceeded its timeout | Process group killed; partial stdout/stderr returned with `truncated: true` | No — the vault is untouched |
| `plugin_proposal_rejected` | Proposal references a path, operation, or size the host refuses | Names the first offending operation; **the whole proposal is dropped**, never partially applied | No |
| `plugin_proposal_conflict` | A pre-image fingerprint moved between proposal and commit | H's `conflict`: both versions preserved, nothing written | No |
| `plugin_storage_quota` | Plugin KV store exceeds its quota | Write refused with a typed error; existing data intact | No |
| `plugin_host_call_invalid` | Malformed arguments, oversized string, invalid UTF-8, reentrancy attempt | Typed error to the guest; repeated violations count toward quarantine | No |

Presentation choice: every case is a single stderr block in the CLI (a CLI has no toast or modal, and inline stdout output would corrupt pipes and JSON), and in the TUI a **non-focus-stealing** status flag plus a `:messages` record (a modal would interrupt typing for a fault that, by construction, cannot harm the buffer). Under `--json` each is the standard error envelope on stderr with stdout empty. Exit codes reuse C's categories: `2` usage/manifest, `3` unknown plugin, `4` conflict, `5` denied/scope/spawn/api-unsupported, `6` runtime backend unavailable, `7` I/O.

**Data-loss column is `No` for every row, by construction:** a plugin never holds a write transaction, so no failure mode of a plugin can leave a note partially written. The only failure that can touch source is a commit crash, which is **H**'s journaled path with its own digest-driven recovery.

### 3.7 Accessibility

- **Screen reader / linear reading:** every surface is plain lines with a leading literal label (`scope:`, `program:`, `RISK:`, `state:`, `error:`). Reading top to bottom conveys 100% of the information, including every security-relevant fact. The consent prompt's answer line spells out each key and its meaning rather than relying on a `[y/N]` convention alone.
- **Custom actions for complex interactions:** the consent prompt is the only multi-step interaction; it is decomposed into one question per capability with an explicit answer line, rather than a single compound "accept all?" — so a screen-reader user hears each permission separately, in the same order and wording as a sighted user.
- **Color independence:** risk is the literal word `LOW`/`MEDIUM`/`HIGH`; state is a literal word; health is `ok` or `traps:N last:<code>`; grant status is `granted`/`declined`/`revoked`/`suspended`. Stripping all ANSI leaves every surface semantically complete — an executable test (§5.1).
- **Glyph independence:** no state depends on a Unicode symbol, badge, or emoji. ASCII mode (E's detection) changes only separators.
- **Text scaling:** N/A in a terminal — the terminal owns font size. The equivalent obligation is width tolerance: tested at 40, 60, 80, and 120 columns, with the rule that the consent prompt wraps and never truncates.
- **Focus order and keyboard navigability:** the overlay's focus order is title → filter → rows → detail; `Tab` cycles, `Esc` closes without acting. Every capability is reachable from argv alone, which is the CLI form of keyboard completeness; `--no-input` proves no path requires interaction to *deny*.
- **Sanitization of untrusted display strings:** command titles, notification text, status contributions, error strings, and audit fields originate in third-party code. All are sanitized before display — C0/C1 control characters and ANSI CSI/OSC sequences escaped, length-capped (titles 64, messages 512 graphemes), newlines rendered as `\n` — so a plugin cannot move the cursor, repaint the status line, spoof a consent prompt, or clear the screen. This is enforced when a string enters the frame or the output buffer, so no code path can bypass it (§5.1).
- **Textual equivalents:** this feature produces no graph, canvas, chart, image, or spatial view. Every value it emits is already text.

---

## 4. Implementation Specification

### 4.1 Architecture placement

Four new crates, plus small additions to existing ones. The split is the point: the sandbox host is the only crate that links the WASM runtime, and it depends on `mg-vault-core` rather than the other way round, so no authority code can ever call into plugin code.

```
crates/mg-vault-plugin-api/     # manifest schema, capability enum, grant types, WIT-generated bindings
crates/mg-vault-plugin-host/    # wasmtime engine, linker, store limits, host-function impls, faults
crates/mg-vault-plugin-caps/    # grant store, scope matcher, consent-prompt model, audit writer
crates/mg-vault-plugin-exec/    # external-command construction, env scrubbing, spawn, reaping
wit/mg-vault-plugin@1.3.0.wit   # the versioned world; the published contract
```

- `mg-vault-plugin-api` is dependency-light and contains **no** I/O: it is the shared vocabulary the CLI, TUI, and host all use, and it can be published for plugin authors.
- `mg-vault-plugin-caps` owns the grant store and the scope matcher and has **no** WASM dependency, so scope logic is unit-testable without a runtime.
- `mg-vault-plugin-exec` is the only crate in the workspace permitted to construct a `std::process::Command`. A `clippy.toml` `disallowed-methods` entry forbids `Command::new` everywhere else, so "only this crate can spawn" is a lint-enforced fact rather than a convention.
- `mg-vault-plugin-host` links `wasmtime` and implements every host function. It calls `mg-vault-core` (`Vault`, `VaultDir`, `SourceFingerprint`) for filesystem access and `mg-vault-refactor` (**H**) for mutation, and holds no filesystem logic of its own.
- `mg-vault-cli` gains the `plugin` command group; `mg-vault-tui`/`mg-vault-app` (**E**) gain the overlay, the palette `Plugin` namespace, and the status segment.
- Directional rule: `cli → plugin-host → {plugin-caps, plugin-exec, plugin-api, core, refactor}`. Nothing in `mg-vault-core` knows plugins exist.

On-disk layout:

```
$XDG_DATA_HOME/mg-vault/plugins/<id>/<version>/{plugin.toml,plugin.wasm,plugin.sig?}
$XDG_STATE_HOME/mg-vault/grants/<vault-id>.json          # 0600, per-user, per-vault; not portable
$XDG_STATE_HOME/mg-vault/plugin-audit/<vault-id>.jsonl    # 0600; security record, not portable
$XDG_STATE_HOME/mg-vault/plugin-storage/<vault-id>/<id>/  # 0700; per-plugin KV, quota'd
$XDG_CACHE_HOME/mg-vault/wasm-cache/                      # wasmtime's own compilation cache
<vault>/.mg-vault/plugins.toml                            # portable *recommendations* only
```

Nothing plugin-related is executable from inside the vault, and no grant travels with a vault.

### 4.2 Data model

```rust
// --- manifest (authored TOML, schema version 1) ---

/// One plugin's declared identity, host-API requirement, and requested capabilities.
pub struct PluginManifest {
    pub manifest_version: u16,          // 1; unknown newer => fail closed, never rewrite
    pub id: PluginId,                   // ^[a-z][a-z0-9-]{2,63}$ ; unique per user
    pub version: Version,               // semver of the plugin itself
    pub name: String,
    pub description: String,
    pub authors: Vec<String>,
    pub license: Option<String>,
    pub api: VersionReq,                // host API requirement, e.g. "^1.2"
    pub entry: RelPath,                 // "plugin.wasm", inside the package only
    pub capabilities: Vec<CapabilityRequest>,
    pub commands: Vec<CommandDecl>,     // id, title, proposed binding (inert), needed caps
    pub hooks: Vec<HookDecl>,           // event name + hook class (§4.7 deadlines)
    pub limits: RequestedLimits,        // memory/fuel requests; host caps them
    pub settings_schema: Option<SettingsSchema>,
}

/// A capability as *requested* in a manifest. Never a grant.
pub struct CapabilityRequest { pub cap: Capability, pub reason: String }  // reason shown in the prompt

/// The complete, closed capability set. Adding a variant is a host-API MINOR bump.
pub enum Capability {
    VaultRead   { scope: ScopeSet },        // read note bytes under a scope
    VaultList   { scope: ScopeSet },        // enumerate paths (separate: listing leaks titles)
    VaultWrite  { scope: ScopeSet },        // *propose* mutations under a scope
    AttachmentRead { scope: ScopeSet },     // non-Markdown bytes
    IndexQuery  { scope: ScopeSet },        // B/G queries, results clipped to scope
    NetHttp     { hosts: Vec<HostPort>, methods: Vec<Method>,
                  allow_private_addresses: bool, budget: NetBudget },
    ProcessSpawn{ template: CommandTemplate, timeout: Duration },
    ClipboardRead, ClipboardWrite,
    ClockRead,                              // coarse, 1 s granularity
    Random,                                 // host CSPRNG bytes
    UiCommand, UiStatus, UiNotify,          // register commands / status segment / messages
    TemplateVariable,                       // J's plugin.<id>.<name> namespace
    Storage     { quota_bytes: u64 },       // plugin-private KV under XDG state
    SettingsRead, SettingsWrite,            // own namespace only
}

/// Vault-relative glob scope. Compiled once; matched against *resolved* paths.
pub struct ScopeSet { include: Vec<Glob>, exclude: Vec<Glob> }

/// A persisted, user-made decision. Bound to one vault and one artifact digest.
pub struct Grant {
    pub plugin: PluginId,
    pub capability: Capability,
    pub artifact_digest: Digest,        // grants suspend when the artifact changes
    pub granted_at: OffsetDateTime,
    pub granted_by: GrantSource,        // Prompt | ExplicitFlag
}

/// The per-vault, per-user grant file. Versioned; unknown newer fails closed.
pub struct GrantStore { version: u16, vault_root: PathBuf, epoch: u64, grants: Vec<Grant> }

/// A never-shell-interpolated external command.
pub struct CommandTemplate {
    pub program: PathBuf,               // absolute, resolved and digested at grant time
    pub program_digest: Digest,
    pub argv: Vec<ArgvPart>,            // each part yields exactly one argv element
}
pub enum ArgvPart { Literal(String), NotePath, Selection, Named(String), Scratch }

/// What a plugin returns instead of writing. The host, not the plugin, mutates.
pub struct MutationProposal {
    pub summary: String,                            // sanitized before display
    pub ops: Vec<ProposedOp>,                       // <= 512 ops, <= 32 MiB total post-image
}
pub enum ProposedOp {
    Create   { path: String, bytes: Vec<u8> },
    Replace  { path: String, expected: SourceFingerprint, edits: Vec<SpanEdit> },
    Relocate { from: String, to: String, expected: SourceFingerprint },
    Trash    { path: String, expected: SourceFingerprint },
}

/// Runtime state of one installed plugin.
pub struct PluginRecord {
    pub manifest: PluginManifest,
    pub artifact_digest: Digest,
    pub signature: SignatureState,      // Verified{key_id} | Present{unverified} | None
    pub state: PluginState,             // Enabled|Disabled|Quarantined{reason}|NeedsConsent|ApiUnsupported
    pub failures: FailureLedger,        // rolling window of faults
}
pub struct FailureLedger { window: Duration, entries: VecDeque<(OffsetDateTime, FaultKind)> }
pub enum FaultKind { Trap, Fuel, Deadline, Memory, HostCallInvalid, LoadFailure }

/// One audit record. Content-free by default (see 6.1).
pub struct AuditRecord {
    pub at: OffsetDateTime, pub plugin: PluginId, pub artifact_digest: Digest,
    pub capability: CapabilityKind, pub operation: &'static str,
    pub target: Option<String>,          // vault-relative path, or host:port
    pub bytes_in: u64, pub bytes_out: u64,
    pub outcome: Outcome,                // Allowed | Denied{code} | Faulted{kind}
}
```

**Persisted schemas.** `plugin.toml` (`manifest_version: 1`), `grants/<vault-id>.json` (`version: 1`), `plugin-storage` records, and `.mg-vault/plugins.toml` (`version: 1`) are all versioned and **fail closed on an unknown newer version, refusing to rewrite** — the same rule as **A**'s registry. There is no note-content migration and no schema that owns note bytes.

### 4.3 API contracts

**The guest world (WIT, versioned).** `wit/mg-vault-plugin@1.3.0.wit` defines world `plugin`. Guest **exports**: `init(host-info) -> result<plugin-info, string>`, `run-command(command-id, args) -> result<command-output, plugin-error>`, `on-event(event) -> result<list<contribution>, plugin-error>`, `template-variable(name, ctx) -> result<string, plugin-error>`. Host **imports**, each capability-checked at entry:

```wit
// every one of these returns result<_, denied> ; none of them can be called ungranted
vault-read:      read-note(path: string) -> result<note, host-error>
vault-list:      list-notes(prefix: string, limit: u32) -> result<list<string>, host-error>
vault-write:     propose(proposal) -> result<proposal-receipt, host-error>   // host previews+commits
index:           query(q: query) -> result<result-page, host-error>
net:             http-request(req: request) -> result<response, host-error>
process:         run(args: list<string>, stdin: option<list<u8>>) -> result<exit, host-error>
clipboard:       read() / write(s: string) -> result<_, host-error>
clock:           now-seconds() -> result<u64, host-error>          // 1 s granularity
random:          bytes(n: u32) -> result<list<u8>, host-error>
storage:         get(k) / set(k, v) / delete(k) / list(prefix) -> result<_, host-error>
ui:              notify(level, text) / set-status(text) -> result<_, host-error>
log:             log(level, text)                                   // always available, sanitized, capped
```

There is **no** `wasi:filesystem`, `wasi:sockets`, `wasi:cli/environment`, `wasi:clocks`, `wasi:random`, or `wasi:filesystem/preopens` in the world, and none is added to the linker under any configuration, flag, or build. A component importing one fails to link with `plugin_missing_import` (§3.6).

**Host-side enforcement contract (binding, and the answer to auto-fail "capability bypass"):** every host function begins with the same six steps, implemented once in a `guard!` helper so no import can skip them.

1. **Grant lookup** by `(plugin id, capability kind, current grant epoch)`. Absent → `plugin_capability_denied`. A grant fetched before a revocation is never reused: the epoch is compared per call.
2. **Argument validation** — UTF-8, length caps (path ≤ 4096 B, string args ≤ 64 KiB, list lengths bounded), no interior NUL. Failure → `plugin_host_call_invalid` and a quarantine tick.
3. **Reentrancy check** — a host call may not re-enter the guest; a per-store flag makes recursion a typed error rather than a stack hazard.
4. **Path resolution through A** — for any path argument, `VaultDir::resolve(path)` performs the descriptor-relative, symlink-refusing, root-confined open. This is the *only* way a plugin path becomes a filesystem object. `.obsidian` and `.mg-vault` are rejected as first components exactly as in **A**.
5. **Scope match on the resolved path** — the granted `ScopeSet` is matched against the **canonical vault-relative path returned by step 4**, never the string the guest supplied. This ordering is the whole defense against symlink-scoped escape: a plugin granted `notes/**` that passes `notes/link-to-secrets.md` gets a resolution refusal at step 4 (symlinks are refused) and, even in a future configuration that permitted in-vault symlinks, would be scope-matched on the *target's* canonical path at step 5. Mismatch → `plugin_scope_violation`.
6. **Secret denylist** — paths matching the vault's secret-exclusion patterns (`.mg-vault/secrets.toml`, **M**'s list, plus built-ins `**/.env`, `**/*.key`, `**/*.pem`, `**/id_*`) are invisible to **every** plugin read, list, and index result, **regardless of scope**, and are never proposable. This denylist overrides any grant and has no override flag.

Then, and only then, the operation runs, and an `AuditRecord` is written whether it was allowed or denied.

**Host API semver.** The world is versioned `major.minor`. MAJOR = a removed or signature-changed import/export, or a semantic change to an existing one. MINOR = an added import, added export, or added `Capability` variant. PATCH = documentation only, no wire change. The host advertises a set of supported worlds (`plugin api --json` → `{"supported":["1.0","1.1","1.2","1.3"],"wit_digest":"sha256:…"}`) and instantiates a plugin against the highest supported world satisfying its `api` requirement. A requirement the host cannot satisfy is `plugin_api_unsupported`, naming both sides — never a best-effort load. **Deprecation policy:** an import may be marked `@deprecated(since = "1.4", removed-in = "2.0")` in the WIT; it keeps working for at least **two minor releases and twelve months**, emits a load-time warning naming the plugin and the import, appears in `plugin doctor`, and is removed only at the next MAJOR. The host hosts major `N` and `N-1` concurrently for at least one full release cycle. CI asserts the shipped `wit/` matches its published digest and that no existing signature changed within a major (§5.1).

**CLI/JSON contract.** Every `plugin` subcommand emits **A**'s version-1 envelope with `command: "plugin.list"` etc. `plugin audit --jsonl` streams one `AuditRecord` per line for log shipping. Field removal or type change requires envelope v2; additive optional fields do not. Golden fixtures per command and per error code are the compatibility contract.

**Auth/permissions.** There is no account or role. Authority is: filesystem permissions, **A**'s confinement, and this feature's grants. Critically, **`ConfirmedRefactor` (H) and `VerifiedInput`/`ConfirmedPermanentDelete` (C) remain private core types a plugin cannot mint** — a proposal is a request, and the host re-derives the plan from the declarative operation exactly as H's `--from-plan` does. **Rate limiting** applies to `net.http` (per-grant requests/min and bytes/session) and to host calls generally (a call-count budget per invocation, exceeding which is a trap). **Pagination:** `list-notes` and `query` are page-based with a host-capped `limit`.

### 4.4 State management

- **Authoritative state:** ordinary vault files. Plugins never hold it. Grants, audit, storage, and artifacts are all app state whose loss costs functionality, never note content.
- **Owner:** `PluginHost` (in `mg-vault-plugin-host`) owns the `wasmtime::Engine` (one per process, shared), the `PluginRegistry` (records, states, failure ledgers), and a **worker thread pool** — plugin execution never runs on the render thread or the CLI's main thread. `GrantStore` (in `mg-vault-plugin-caps`) owns grants and the epoch counter; it is the single source of truth for authorization and is consulted per host call, never cached inside a `Store`.
- **New state container:** yes — `PluginHost`, injected into `App` (**E**) and constructed on demand by the CLI. It holds no note state and no `TextStore` handle; the crate boundary prevents a direct write, exactly as E's does.
- **Instance lifetime:** one `wasmtime::Store` per *invocation*, not per plugin. State that must persist across invocations goes to the `storage` capability. This makes a trap trivially containable (drop the store), makes fuel accounting per-invocation, and denies a plugin a long-lived resident heap.
- **Local vs. server-synced:** everything is local. There is no server, no account, and no sync of grants or artifacts. The only network traffic that exists at all is a plugin's own granted `net.http` — the host itself never phones home, checks for updates, or reports telemetry.
- **Offline / draft persistence:** a plugin's proposal is held in memory for the duration of the preview. If the user is interrupted mid-preview, the proposal is discarded, not spooled — re-running the command re-derives it. Rationale: a spooled proposal is a stale edit list that could be applied against moved source, which is exactly the class of bug **H**'s re-derivation rule exists to prevent.
- **Concurrency:** at most one instance per plugin per vault at a time (a second invocation queues, with a bounded queue of 8 and a typed `busy` error beyond it). Grant reads take a snapshot of the epoch; a revoke bumps the epoch and sets every affected store's epoch deadline to now, so in-flight guests trap at their next instruction or host call.

### 4.5 Dependencies

- **`wasmtime` (pinned exact minor, Bytecode Alliance) — the runtime.** Chosen over the alternatives because it is the only mature Rust runtime that gives us all four of the things this spec requires simultaneously: the **Component Model + WIT** (so the host interface is a typed, versionable contract rather than a pile of `i32` pointers, which is what makes §4.3's semver policy real), **`Config::consume_fuel`** for deterministic instruction budgets, **`Config::epoch_interruption`** for wall-clock deadlines that stop a spinning guest, and **`ResourceLimiter`** for hard memory/table/instance caps. It also has a published security policy, a CVE history, and continuous OSS-Fuzz coverage.
- **Honest treatment of `unsafe_code = "forbid"`.** That workspace lint governs **our** crates and continues to hold in all four new crates — none of them writes `unsafe`. `wasmtime` uses `unsafe` internally, as any JIT must. That is a deliberate, recorded dependency-trust decision, not an oversight, and it is mitigated four ways: (1) we never call an `unsafe fn` from wasmtime's API — in particular we do **not** use `Component::deserialize`, and instead use wasmtime's own safe on-disk compilation cache, so no `unsafe` block is needed anywhere in this feature; (2) we pin an exact version and disable every proposal we do not need (`wasm_threads(false)`, shared memory off, SIMD off, `wasm_component_model(true)` and bulk-memory only as the component model requires), shrinking the attack surface; (3) the pooling allocator with guard pages and `memory_init_cow` is enabled so linear-memory bounds are hardware-enforced; (4) `wasmi` — a pure-Rust *interpreter* whose core forbids `unsafe` — is specified as a build-time-selectable fallback backend behind `--features wasm-backend-wasmi`, for a maximal-assurance build or a platform without a Cranelift backend, at a documented 10–50× execution cost. The backend in use is reported by `--version` and `plugin doctor`; **capability semantics are identical across backends and are never weakened by the choice** (the same rule **A** applies to its confinement backend).
- **Rejected alternatives:** *Wasmer* (component-model support and licensing story weaker for our use); *WAMR/wasm3* (C runtimes — adding a C toolchain and a non-Rust memory-safety surface to the single most security-critical component is the wrong trade); *native dynamic libraries / `dlopen`* (no sandbox at all); *an embedded scripting language* (Lua/Rhai give a sandbox but no memory limit, no preemption, and no typed versioned ABI — and Obsidian's JS model is precisely the failure this feature exists to avoid); *subprocess-only plugins* (would make external commands the *only* extension mechanism, i.e. maximum privilege as the baseline).
- **Other crates:** `wasmtime-wasi` is **not** taken as a dependency (nothing from it is linked). `ureq` 3 + `rustls` + `webpki-roots` for the blocking, no-async-runtime HTTP client behind `net.http`. `globset` for scope matching. `semver` for API and manifest versions. `rustix` (already present) for `no_new_privs`, process-group control, and descriptor hygiene in `plugin-exec`. `time` for audit timestamps. `toml` for manifests. `ed25519-dalek` **only if** signing is adopted (§8-Q2). No async runtime, no database, no telemetry SDK.
- **Assets:** none — no fonts, icons, images, or bundled data. Test fixtures include hand-built `.wasm` components compiled in CI from in-repo Rust sources; their provenance is the repository itself.
- **Infrastructure:** none required. No registry service, no CDN, no update server. `plugin install` takes a local path; installing from a URL is explicitly out of scope for this milestone (§7.5).

### 4.6 Platform-specific considerations

- **Primary target:** Arch Linux, x86_64 and aarch64, where Cranelift and `openat2` are both available. macOS is supported with the `openat`+`(dev, ino)` confinement backend from **A**/**C** and without `no_new_privs` (its absence is reported by `plugin doctor` and raises the spawn prompt's risk line). Windows is out of scope for this milestone.
- **Capability gate (fail-closed):** the plugin host **refuses to instantiate any plugin** unless the platform passes the same gate **C** defines for mutation — descriptor-relative confinement plus proven file and directory sync — *and* the runtime backend passes its own startup self-test (fuel accounting, epoch interruption, and memory-limit refusal each demonstrated on a built-in probe component in < 20 ms). Failure yields `confinement_unavailable` or `plugin_runtime_unavailable` with exit 6, **before** any third-party code is compiled. There is no reduced-sandbox mode.
- **Version compatibility:** Rust edition 2024, `rust-version` 1.85 or the higher floor `wasmtime` requires, recorded in `Cargo.toml`. Guests target `wasm32-wasip2` components; a core module (not a component) is rejected with a message naming the required target.
- **Feature flags / rollout:** `wasm-backend-wasmi` selects the fallback backend. `plugin-external-commands` may ship off in early releases, in which case `process.spawn` is *absent from the capability enum surface*, the manifest field is rejected with a named error, and `plugin api` reports the reduced world — never a silently ignored declaration. **Never feature-gated and never overridable:** deny-by-default, scope matching on resolved paths, the secret denylist, the no-ambient-WASI rule, memory/fuel/epoch limits, proposal-based mutation, and consent for every grant.
- **Multiplexers/terminals:** inherited from **E**; the consent prompt requires a real tty and refuses to run without one, so it cannot be answered by a pipe.

### 4.7 Performance budget

Reference: warm local SSD, reported with hardware/OS metadata.

- **Startup:** the plugin host adds ≤ 15 ms to `mg-vault` startup with 8 plugins installed — manifests and grants are read (small JSON/TOML), **no component is compiled or instantiated at startup**. Compilation is lazy, on first invocation, and cached by wasmtime keyed by artifact digest + runtime version + config hash.
- **Instantiation:** ≤ 20 ms p95 warm (cache hit) for a 2 MiB component; ≤ 400 ms cold compile, which happens once per artifact and is reported as `compiling plugin…` if it exceeds 200 ms.
- **Execution deadlines (epoch, wall clock) by hook class:** UI contribution hook 50 ms; event hook 500 ms; user-invoked command 5 s; background task 30 s. **Fuel budgets** (deterministic, machine-independent): 20 M units for a UI hook, 200 M for a command, 2 G for a background task; manifests may request less, never more. Exceeding either is a contained fault.
- **Keystroke latency:** ≤ 1 ms added at p99 with 8 plugins loaded, achieved structurally — UI contributions are pre-computed off the render thread and cached, and the renderer reads a snapshot. A plugin cannot be on the keystroke path.
- **Memory:** store cap 64 MiB default, manifest may request up to a 256 MiB host maximum (the request is shown in the consent prompt). Host-side overhead ≤ 8 MiB per *loaded* plugin (compiled artifact plus registry entry) and ≤ 2 MiB per idle installed-but-unloaded plugin. Total plugin subsystem RSS is capped and reported by `plugin doctor`.
- **Network:** per-grant defaults 30 requests/min, 1 MiB request body, 4 MiB response body, 16 MiB per session, 10 s timeout, ≤ 3 redirects within the allowlist. All are shown in the consent prompt and enforced host-side.
- **Storage:** plugin KV quota 8 MiB default per plugin (declared in the manifest, shown in the prompt); audit log rotates at 32 MiB with 4 generations; wasm cache capped at 512 MiB with LRU eviction; all are disposable.
- **Scale posture:** no plugin operation walks the vault on the host's behalf; `list-notes` is page-limited and `query` goes through **B**'s bounded API, so a plugin cannot make a 100,000-note vault unresponsive. A plugin that saturates its budgets degrades only itself.

---

## 5. Test Specification

### 5.1 Unit tests

| Name | Setup → assertion | Edge case / criterion |
|---|---|---|
| `manifest_unknown_version_fails_closed` | `manifest_version = 2` → `plugin_manifest_invalid`, file never rewritten | 4E versioned contracts |
| `api_requirement_outside_host_set_refuses` | `api = "^2.0"` on a 1.3 host → `plugin_api_unsupported` naming both | 4E |
| `wit_world_signatures_are_frozen_within_a_major` | Compare shipped `wit/` to the published digest and per-function signature table | 4E deprecation policy |
| `no_wasi_import_is_ever_linked` | Enumerate the linker's registered imports; assert the set equals the world's and contains no `wasi:filesystem`/`sockets`/`clocks`/`random`/`environment` | **4C auto-fail** |
| `guest_importing_wasi_filesystem_fails_to_link` | Corpus component → `plugin_missing_import` naming the import | 4C |
| `ungranted_call_is_denied_for_every_import` | Table-driven over **every** host import with an empty grant set → all `plugin_capability_denied` | 4C deny-by-default |
| `scope_is_matched_on_resolved_path_not_input` | Grant `a/**`; call with `a/../b/x.md`, `a/link.md` (symlink → `b/`), `a//x.md`, `A/x.md` on a case-insensitive fs → refusal each time | **4A auto-fail** |
| `control_and_secret_paths_are_invisible_at_any_scope` | Grant `**`; read/list/query/propose `.obsidian/*`, `.mg-vault/*`, `.env`, `id_ed25519` → denied, and absent from list/query results | 1D, 4F |
| `revocation_takes_effect_on_the_next_host_call` | Grant, start a long guest, revoke mid-run → next call denied, instance terminated, receipt counts it | 4C revocable |
| `artifact_digest_change_suspends_all_grants` | Swap the `.wasm` → every grant suspended, consent re-required with a capability diff | 4C |
| `argv_holes_are_never_shell_interpolated` | Fixture values `; rm -rf ~`, `$(id)`, `` `id` ``, `a b`, `*`, `~`, newline, NUL → each becomes exactly one literal argv element; NUL rejected | External-command injection |
| `spawn_environment_is_constructed_not_inherited` | Set 20 env vars incl. `LD_PRELOAD`, `SSH_AUTH_SOCK` → child sees exactly `PATH`, `LC_ALL`, `HOME` | 4F |
| `spawn_rejects_unsafe_program_paths` | Program inside the vault, symlink, setuid, group-writable, relative → refused with the reason | 4C |
| `fuel_exhaustion_traps_and_contains` | Guest infinite loop → `plugin_fuel_exhausted`, store dropped, host state unchanged | Fault isolation |
| `epoch_deadline_interrupts_a_spinning_guest` | Guest busy-loop under a 50 ms UI deadline → trap within 60 ms | Fault isolation |
| `memory_growth_beyond_cap_is_refused` | Guest allocates past 64 MiB → refusal or contained trap, no host OOM | Fault isolation |
| `failure_ledger_quarantines_after_threshold` | 3 traps in the window → `Quarantined`, disabled, reason recorded, re-enable requires an explicit command | Fault isolation |
| `proposal_touching_a_denied_path_drops_the_whole_proposal` | 5 ops, op 3 out of scope → zero files written | **partial-mutation auto-fail** |
| `plugin_cannot_mint_a_confirmation_type` | Compile-time: `ConfirmedRefactor`/`VerifiedInput` are private and unconstructible from plugin crates | 4C |
| `only_plugin_exec_may_construct_a_command` | Clippy `disallowed-methods` for `Command::new` outside `mg-vault-plugin-exec` | Structural |
| `plugin_strings_are_sanitized_before_display` | Titles/status/messages containing CSI, OSC 52, `\r`, 10 KiB of text → escaped, capped, single-line | 5D, spoofing |
| `no_color_output_is_semantically_complete` | Strip ANSI from every plugin surface incl. the consent prompt → all state words present | 5D |
| `consent_prompt_never_truncates_below_60_columns` | Render at 40/60/80/120 → scope and combination text wrap, never elide | 5D, informed consent |
| `network_grant_enforces_host_port_scheme_method_and_budget` | Table over disallowed host, port, scheme, method, private IP, oversize body, redirect off-allowlist | 4C, 4F |
| `audit_records_every_allowed_and_denied_call` | Exercise each capability twice (granted/denied) → one record each, no note content in any field | 4F |

### 5.2 Integration tests

- **Adversarial plugin corpus (the capability-bypass gate).** 30+ purpose-built hostile components run under a syscall-monitoring harness (`seccomp`-audit or `strace`-based) that **fails the test if the `mg-vault` process opens any path outside the vault and its own XDG dirs, spawns any unapproved child, opens any socket to a non-allowlisted host, or reads any environment variable on a plugin's behalf.** Cases: ambient-WASI import; `..` traversal; absolute path; symlink-to-outside; symlink-inside-scope-to-outside-scope; hardlink; `.obsidian` write; `.mg-vault` write; secret-file read; scope-boundary read; infinite loop; unbounded `memory.grow`; deep recursion; host-call reentrancy; 10 M-element proposal; 4 GiB post-image; malformed UTF-8; NUL in a path; ANSI/OSC injection in a title and in a proposal summary; `sh -c` attempt via `program`; metacharacters in every `ArgvPart`; undeclared network host; redirect to `169.254.169.254`; DNS rebinding to a private address; oversized upload; retry-loop after denial; digest-mismatched artifact; future-API manifest; a manifest declaring fewer capabilities than the component uses.
- **Fault-injection matrix.** Kill the host at each phase of a plugin-proposed multi-file transaction (prepare, commit barrier, mid-apply, sync, terminal record) and assert **H**'s recovery leaves every file byte-equal to either its exact pre-image or its exact committed image — never anything else — and that the receipt's `actor` survives recovery.
- **Grant lifecycle round-trip.** install → decline → grant → use → revoke → re-grant → update artifact → suspended → capability-diff → re-consent, asserting the exact JSON envelope and exit code at each step against golden fixtures.
- **Cross-feature contracts.** **J**: `{{plugin.x.y}}` fails closed with `template_capability_denied` when ungranted and resolves with attribution when granted. **H**: a plugin proposal produces an ordinary journaled refactor with a plugin actor. **E**: plugin commands appear in the palette with truthful availability and cannot bind a buffer-namespace key. **B/G**: query results are clipped to scope and carry unchanged freshness.
- **Coexistence.** An Obsidian vault with `.obsidian/plugins/` present: assert `mg-vault` never reads, writes, executes, or imports anything from it, and that the vault still opens in Obsidian afterwards.

### 5.3 UI / E2E tests

Driven through **E**'s headless terminal backend (no TTY required) and a scripted CLI harness:

1. **Consent path:** install → prompt renders → answer `n` to all → plugin installed with 0 grants → invoking its command shows `unavailable: capability not granted` and writes nothing.
2. **Grant-and-use path:** grant `vault.read finance/**` → run command → output rendered → no mutation.
3. **Mutation path:** plugin proposes 3 edits → diff preview with the actor banner → confirm → one journaled commit → `u` undoes all three as one entry labeled `plugin:expenses`.
4. **Error recovery path:** plugin traps mid-command while a dirty buffer is open → status flag appears, buffer bytes and cursor unchanged, `:messages` names the fault, second and third traps quarantine the plugin, `plugin quarantine clear` restores it.
5. **External command path:** argv confirmation renders each element on its own line → deny → nothing spawns; approve → runs; a changed argv re-prompts.
6. **Non-interactive path:** `--no-input` install grants nothing and exits 0 with a warning; `--no-input` spawn without a cached approval exits 5; no prompt is ever emitted into a JSON stream.
7. **Revocation path:** revoke while a plugin is running → instance terminated, receipt reports it, subsequent call denied.

### 5.4 Visual / manual verification

- **Theme variants:** truecolor, 256, 16, and `none`/`NO_COLOR` — verify the consent prompt, risk markers, plugin states, and health column are fully legible and unambiguous with the attribute plane discarded.
- **Text size / width extremes:** 40, 60, 80, 120, and 300 columns; 8-row and 100-row terminals. Confirm the consent prompt wraps rather than truncates at every width and that no permission line is ever cut.
- **Unicode and ASCII modes:** a plugin whose name, title, and proposal summary contain CJK, RTL, combining marks, emoji, and zero-width characters — verify column alignment, that RTL/zero-width cannot visually reorder a permission line, and that ASCII mode escapes rather than drops.
- **Empty vs. populated:** no plugins; one plugin with zero grants; eight plugins with mixed states including one quarantined and one `needs-consent`; an audit log with 10,000 records.
- **Screen-reader pass:** read the consent prompt and a fault message with a screen reader and confirm every security-relevant fact — scope, host, program path, risk word, combination warning — is announced.

---

## 6. Compliance & Safety Gate

### 6.1 Sensitive data classification

- [ ] No sensitive data involvement
- [x] **Handles sensitive data.** This feature can expose note bodies, titles, paths, clipboard contents, and index results to third-party code, and can transmit them to a network host or an external program. Protections: **deny by default** — none of it is reachable without a specific grant; **scoped** — a grant names a subtree, a host list, or one argv template; **denylisted** — secret-pattern files and the control directories are unreachable at any scope, with no override; **consented** — the prompt states the read scope and the egress destination together and spells out the exfiltration consequence in one sentence (§3.3); **bounded** — per-session upload budgets cap the volume; **attributed** — every allowed and denied capability use is audited; **revocable** — a revoke kills running instances; **not portable** — grants and artifacts live outside the vault, so copying a vault carries neither.
  The **audit log is content-free by default**: it records capability, operation, target path or `host:port`, byte counts, and outcome — never note bytes, never request/response bodies, and never external-command *argument values* (only the template shape and each hole's byte length). `plugins.audit.record_arguments = true` opts into full argv recording and prints a warning when enabled. The audit log, grant store, and plugin storage are owner-only (`0600`/`0700`) and live under `$XDG_STATE_HOME`, outside the vault, so they are not synced, exported, published, or shared with a vault.
- [ ] Uses synthetic/test data only until compliance gate clears

### 6.2 Asset provenance

- [x] **No third-party assets** — no fonts, images, icons, models, sounds, or bundled datasets. Rust crate dependencies (§4.5) are implementation libraries; each must pass repository dependency-license policy, which is tracked with the unresolved project-license question inherited from **A**-§8-Q1. Test-corpus `.wasm` components are compiled in CI from Rust sources in this repository, so their provenance is the repository itself. **Third-party plugin artifacts are user-supplied content, not project assets**; the host records each artifact's digest, its declared license field, and its signature state, and displays them, but makes no claim about their provenance.
- [ ] Uses third-party assets

### 6.3 Language / claims audit

- [x] **No claims unsupported by evidence.** Every latency, memory, fuel, and budget figure in §4.7 is an acceptance budget, not a measurement. The sandbox claims are stated as properties enforced by named tests over named mechanisms (§5.1's linker-import enumeration, resolved-path scope matching, and syscall-monitored corpus), not as assurances.
- [x] **No promised-but-unbuilt capability presented as available.** This entire branch is **absent** today and §7.1 says so in the required state words. Signing, remote install, and a registry are labeled out of scope (§7.5) or open (§8), not "coming".
- [x] **"Sandboxed" is used precisely.** It means: no ambient WASI, every host import capability-checked, hardware-enforced linear-memory bounds, fuel and epoch limits, per-invocation store. It is **not** claimed to defend against a hostile kernel or root, a CPU side channel, or a wasmtime 0-day; those are named residual risks in §6.5.
- [x] **Exfiltration is described truthfully, not minimized.** The spec states plainly that read-scope plus a network grant permits upload, that the host cannot prevent it once both are granted, and that the mitigations are visibility and volume caps. No text implies otherwise.
- [x] **No regulated-domain language.** No medical, financial, legal, or safety claim is made.

### 6.4 Regulatory alignment

Lens 3 (the template's named reference) is mostly owned elsewhere; each criterion is addressed as an N-owned property or a named deferral, never as fake coverage.

- **3A Determinism** — *N-owned constraint.* A plugin never becomes a source of derived truth: it reads through **B**/**G**'s existing contracts and its results carry their `freshness`/`generation` unchanged. A plugin cannot write to the index, cannot mark a stale generation current, and cannot supply link resolutions. Determinism of links, search, and graph stays **deferred to B and G**.
- **3B Ambiguity** — *N-owned analogue.* Every ambiguous or unresolvable plugin input fails closed: an unmatched scope, a moved fingerprint, a path the resolver cannot canonicalize, or a proposal the host cannot fully validate is refused whole. The host never "picks the likely one" on a plugin's behalf.
- **3C Query depth** — **Deferred to B and G.** N adds no query language; `index.query` is a scope-clipped pass-through of G's contract.
- **3D Derived authority** — *N-owned property.* Nothing a plugin produces is authority: proposals are requests that the host re-derives and previews; contributions are display-only; plugin storage is disposable app state. Deleting every plugin, grant, and storage record changes no note byte.
- **3E Scale** — *N-owned constraint.* No plugin path walks the vault; `list-notes` is page-capped and `query` is bounded by B. A plugin's budgets are per-invocation, so plugin load cannot scale with vault size. The 100,000-note budgets remain **B**'s.

**Binding acceptance traceability (all five lenses).**

| Criterion | N-owned acceptance, or explicit deferral |
|---|---|
| **1A Authority** | Plugins read and propose against ordinary files only. No plugin state is authority; deleting `$XDG_STATE_HOME/mg-vault/{grants,plugin-storage,plugin-audit}` loses no note content. Artifacts live outside the vault entirely. |
| **1B Preservation** | Plugin `Replace` operations are **span edits** applied by **H** to the exact pre-image; all other bytes are copied verbatim. A plugin cannot request "reserialize this file" — there is no such operation in the world. Unknown syntax outside an edited span survives byte-for-byte. |
| **1C Identity** | Attribution lives in H's receipts and the audit log, never in note content. No UUID, `generated-by` property, marker comment, or zero-width character is ever injected by a plugin path, and the host rejects a proposal whose only change is such an injection into frontmatter it did not declare. |
| **1D Coexistence** | `.obsidian` is unreachable at any scope, read or write; `.obsidian/plugins/` is never read, imported, or executed. mg-vault's plugin state is outside the vault; only a portable, inert *recommendation* list sits under `.mg-vault`. |
| **1E Transactions** | Plugin mutations are **H**'s journaled multi-file transactions: staged, digest-guarded, commit-barriered, idempotently recoverable, quarantining on drift. A rejected operation drops the entire proposal. Fingerprint mismatch is a `conflict` preserving both versions. |
| **2A Keyboard completeness** | Every plugin action — install, consent, grant, revoke, run, preview, undo, quarantine — is reachable from argv and from a documented TUI binding or palette entry. Plugins cannot bind keys; they propose bindings the user accepts. |
| **2B Editing durability** | A plugin fault cannot touch a buffer: plugins run off the render thread, hold no `TextStore`, and mutate only via proposals. Applied proposals are one **D** undo entry labeled with the plugin id. |
| **2C Workspace** | Deferred to **E**; N contributes one overlay, one status segment, and a palette namespace that follow E's existing conventions and its non-droppable-status rules. |
| **2D Text correctness** | Plugin-supplied strings are validated as UTF-8, grapheme-capped for display, and sanitized; span edits use **A**'s character-boundary-checked spans, so a plugin cannot split a grapheme or produce invalid UTF-8 in source. |
| **2E Degraded experience** | With the runtime backend unavailable, plugins disabled, or every grant declined, all core editing continues and each unavailable plugin command says exactly why (`capability not granted`, `plugin quarantined`, `runtime unavailable`). Nothing silently no-ops. |
| **3A–3E** | See the itemized paragraphs above. |
| **4A Confinement** | Every plugin path goes through **A**'s descriptor-relative `VaultDir` (`RESOLVE_BENEATH|RESOLVE_NO_SYMLINKS|RESOLVE_NO_MAGICLINKS`), and **scope is matched on the resolved canonical path, not the requested string**. Control directories and secret patterns are unreachable at any scope. The host refuses to run at all on a platform failing the confinement gate. |
| **4B Concurrency** | Every proposal carries pre-image fingerprints revalidated at commit from opened handles; a moved file yields `conflict` with both versions preserved. One instance per plugin per vault; revocation bumps an epoch checked per host call. |
| **4C Least privilege** | The whole feature: closed capability enum, deny-by-default, per-capability scoped consent, combination warnings, revocation with instance termination, digest-bound grants, enforcement in a single unskippable guard at the host-call boundary, no ambient WASI, and confirmation types a plugin cannot mint. |
| **4D Recovery** | Every destructive plugin action is previewed with a diff, committed atomically through H's journal, trash-backed, and undoable as one labeled step. Faults never print a success verb. Auto-apply is opt-in, scoped, and can never cover `trash` or `relocate`. |
| **4E Contracts** | Versioned WIT world with semver rules and a two-minor/twelve-month deprecation window; versioned manifest and grant schemas that fail closed on unknown newer versions; **A**'s v1 JSON envelope with golden fixtures per command and error code; truthful availability strings everywhere. |
| **4F Privacy** | Secret-pattern and control-directory denylist overriding all grants; content-free audit log by default; scrubbed child environment; no host telemetry, update check, or network call of its own; egress budgets and per-plugin byte meters; the consent prompt states the exfiltration consequence explicitly. |
| **5A Offline/local-first** | The host makes zero network calls. The only network traffic possible is a plugin's granted `net.http`, which is off by default and killable with `--no-network`. Install is from a local path. Everything works with networking disabled. |
| **5B Responsiveness** | §4.7: lazy compilation, ≤ 15 ms startup impact, per-hook epoch and fuel budgets, ≤ 1 ms keystroke impact by construction, hard memory caps, and cancellation via `Ctrl-c` mapping to an epoch deadline. |
| **5C Accessible equivalents** | N produces no graph, canvas, or media. Every surface — including the security-critical consent prompt — is line-oriented text with literal labels and no color- or glyph-only meaning. |
| **5D Terminal resilience** | 40/60/80/120-column layouts; consent prompt wraps and never truncates; ASCII mode; literal state and risk words; mandatory sanitization of third-party strings so a plugin cannot corrupt the terminal; no animation. |
| **5E Automation** | Every command has `--json`; `plugin audit --jsonl` streams records; `--no-input` never grants and never prompts; `--no-color`/`NO_COLOR` honored; stdin/stdout discipline inherited from A/C. |

**Auto-fail review, by name.**

- **Capability or data-exfiltration bypass** — the single most important line here. Enforcement is at the host-call boundary in one shared guard, not by convention: no ambient WASI is linked under any build or flag; every import is grant-checked against a per-call epoch; scope is matched on the **resolved canonical path**, closing the symlink-scoped-escape hole; a secret/control denylist overrides every grant with no override flag; network egress is host-allowlisted, budgeted, SSRF-filtered, and metered; external commands are argv vectors with a scrubbed environment and no shell; and §5.1/§5.2 assert all of it mechanically, including a syscall-monitored corpus that fails on any unauthorized open, spawn, or socket. Where a granted combination *does* permit exfiltration (read scope + network), the consent prompt says so in plain language rather than the spec pretending otherwise.
- **Unsafe traversal or symlink escape** — plugins are the likeliest source, so paths never touch the filesystem except through **A**'s `VaultDir`, symlinks are refused at every component, and the scope check happens *after* resolution. Tested by `scope_is_matched_on_resolved_path_not_input` and the traversal corpus.
- **Partial multi-file mutation** — structurally unreachable: a plugin never holds a transaction. It returns a proposal; the host validates it whole (one bad operation drops all of it) and commits through **H**'s write-ahead-logged, digest-recoverable transaction. Fault injection at every phase asserts pre-image-or-committed-image.
- **Source-content loss** — plugins cannot truncate, reserialize, or overwrite: writes are span edits on the exact pre-image with fingerprint preconditions, every destructive op is trash-backed, and every failure path leaves the pre-image.
- **Unconfirmed overwrite / import** — every destructive proposal is previewed with a diff and confirmed; auto-apply is opt-in, scoped, and excludes `trash`/`relocate`; install never grants under `--no-input`; a changed artifact suspends grants.
- **Recovery overwriting newer source** — inherited from **H**: recovery is digest-driven and quarantines on drift rather than writing.
- **Silent conflict winner** — a moved fingerprint is a typed `plugin_proposal_conflict`; nothing is applied and both versions survive.
- **Unknown syntax loss** — no plugin operation reserializes a document; edits are byte splices outside declared spans.
- **Index state overriding source / stale index presented as current** — a plugin cannot write to the index and its query results carry B's freshness unchanged.
- **Non-atomic save claiming success** — success verbs come from H's terminal record only.
- **Active raw HTML/script by default** — plugin-contributed strings are inert sanitized text; nothing a plugin returns is executed, rendered as HTML, or interpreted as markup. **F** owns sanitization for note content and a plugin cannot bypass it.
- **Graph/Canvas without a textual equivalent** — N emits no graphical representation at all.

### 6.5 Security controls

- Treat every byte from a guest as hostile input: validate UTF-8, length, and shape before use; reject interior NUL; cap list lengths and total proposal size; refuse reentrancy.
- Keep `unsafe_code = "forbid"` in all four new crates and call no `unsafe` wasmtime API (notably not `Component::deserialize`); pin the runtime version and disable unneeded proposals; enable the pooling allocator and guard pages.
- Enforce "only `mg-vault-plugin-exec` may spawn" with a clippy `disallowed-methods` lint, so the rule is checked by CI rather than by review.
- Sanitize every third-party display string at frame/output entry, never at the byte stream, so no path bypasses it.
- Fail closed on every unknown: unknown manifest version, unknown API version, unknown capability name, unknown import, unparseable grant file, unverified digest — refuse and never rewrite.
- Never log, audit, or place note content in argv, environment, process titles, or diagnostics.
- **Named residual risks, stated rather than hidden:** a wasmtime sandbox-escape vulnerability (mitigated by pinning, fuzzed upstream, a no-JIT fallback backend, and prompt patching); CPU micro-architectural side channels, which no WASM runtime prevents; a granted read+network combination, which is exfiltration *by user consent* and is surfaced, metered, and budgeted rather than prevented; and an external command's own behavior once the user approves its argv, which is outside any sandbox this feature can impose. The threat model is hostile *plugin code and plugin input*, not a hostile kernel or root.

---

## 7. Gap Analysis vs. Current State

### 7.1 What exists today

State words are exact: implemented / prototyped / planned / gated / absent.

- **Absent — the entire branch.** There is no plugin host, no WASM runtime dependency, no manifest, no capability model, no grant store, no audit log, no external-command path, no `plugin` command group, and no WIT world. `crates/` contains exactly `mg-vault-core`, `mg-vault-index`, and `mg-vault-cli`; `Cargo.toml` lists `clap`, `serde`, `serde_json`, `rustix`, `rusqlite`, `sha2`, `thiserror`, and `yaml-edit` — no `wasmtime`, no HTTP client, no TLS stack, and no crate that can open a socket. `crates/mg-vault-cli/src/main.rs` defines only `vault`, `note`, `index`, `search`, and `interop`. `README.md` states plainly that there is "no TUI, editor, renderer, plugin host, sync adapter, broad import pipeline, publishing workflow, or AI adapter", and `docs/ARCHITECTURE.md` records the plugin crate as deferred. **Absent.**
- **Implemented — the substrate this feature will stand on.** `Vault` path validation with traversal, symlink, and `.obsidian`/`.mg-vault` mutation rejection; `SourceFingerprint`; `atomic::{create_atomic, replace_atomic, sync_parent}`; `edit_note_span`'s character-boundary-checked byte splice; trash/restore with collision refusal; the version-1 JSON envelope; `--no-input`/`--no-color`. **Implemented** (`crates/mg-vault-core/src/{vault,atomic,error,frontmatter_scalar}.rs`, `crates/mg-vault-cli/src/main.rs`, `crates/mg-vault-core/tests/foundation.rs`).
- **Prototyped — the confinement primitive this feature depends on most.** `crates/mg-vault-core/src/index.rs::read_source_bytes` already uses `openat2` with `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS` on Linux and refuses to open on other platforms. That is exactly the `VaultDir` primitive §4.3 step 4 requires, but it exists only on the read/projection path. **Prototyped.**
- **Planned (specified elsewhere, not built).** **A** specifies `VaultDir`, the transaction journal, quarantine-based recovery, exit-code categories, and `.mg-vault/settings.json`. **C** specifies plan/commit, `--dry-run`/`--from-plan`, `doctor`, and the private confirmation types. **H** specifies the multi-file journaled transaction, receipts, undo store, and rollback that plugin mutations use wholesale. **E** specifies the palette, keymap namespaces, overlay conventions, status line, and sanitization discipline. **J** specifies the `plugin.<id>.<name>` template-variable contract and its fail-closed behavior. **B**/**G** specify the query contract `index.query` passes through. **M** specifies the secret-exclusion patterns the denylist reuses. None of these are built. **Planned.**
- **Gated.** Nothing in this feature can be enabled today; the whole branch is gated on **A**'s `VaultDir` and journal and on **H**'s transaction landing first (§7.4). **Gated.**

### 7.2 Delta to spec

**New crates / modules**

- `crates/mg-vault-plugin-api/` — manifest schema and parser, `Capability`, `ScopeSet`, `Grant`, `MutationProposal`, `wit-bindgen`-generated types, semver policy constants.
- `crates/mg-vault-plugin-caps/` — `GrantStore` (versioned, `0600`, epoch counter), the resolved-path scope matcher, the secret/control denylist, the consent-prompt model (risk classification and combination analysis), and the audit writer with rotation.
- `crates/mg-vault-plugin-host/` — `Engine` construction and configuration, the linker with the exact import set, `ResourceLimiter`, epoch thread, fuel accounting, the shared six-step host-call `guard!`, every host function, the worker pool, the failure ledger and quarantine logic, and the backend self-test.
- `crates/mg-vault-plugin-exec/` — `CommandTemplate` expansion (one part → one argv element), program-path vetting, environment construction, scratch directory, process-group spawn with `no_new_privs`, bounded capture, timeout and group kill, approval cache keyed by resolved-argv digest.
- `wit/mg-vault-plugin@1.x.wit` plus `docs/PLUGIN-API.md` (the published contract, changelog, and deprecation schedule) and `docs/PLUGIN-AUTHORING.md`.
- `crates/mg-vault-cli/src/plugin.rs` — the `plugin` command group and its renderers.
- Test assets: `tests/plugins/adversarial/*` (30+ hostile components with a CI build step), `tests/plugins/fixtures/*` (well-behaved reference plugins), the syscall-monitoring harness, and golden JSON fixtures per command and error code.

**Modified files**

- `Cargo.toml` — add the four crates and the `wasmtime`, `ureq`/`rustls`, `globset`, `semver`, `toml`, `time` workspace dependencies; add the `wasm-backend-wasmi` and `plugin-external-commands` features.
- `clippy.toml` — `disallowed-methods` entry for `std::process::Command::new` outside `mg-vault-plugin-exec`.
- `crates/mg-vault-core/src/vault.rs` (or A's new `confine.rs`) — promote the prototyped `openat2` reader into the shared `VaultDir` capability this feature calls.
- `crates/mg-vault-cli/src/main.rs` — register the `plugin` group; extend `doctor` with the plugin section.
- **E**'s crates — the `:plugins` overlay, the `Plugin` palette namespace with truthful availability, the `plugin!` status segment, the proposed-binding acceptance path, and the sanitization entry point for third-party strings.
- **H**'s receipt type — add `actor: Actor` and surface it in `refactor history`.
- **J**'s template resolver — wire `plugin.<id>.<name>` to a granted `TemplateVariable` capability.
- `docs/SECURITY.md`, `docs/ARCHITECTURE.md`, `README.md` — the sandbox model, the capability list, the residual risks, and the honest capability state.

**Migrations / schema changes**

- New versioned schemas only: `plugin.toml` v1, `grants/<vault-id>.json` v1, `.mg-vault/plugins.toml` v1, audit JSONL v1. Each fails closed on an unknown newer version and refuses to rewrite. **No note-content migration exists or may ever exist.**

**New dependencies:** `wasmtime` (and optionally `wasmi`), `ureq` + `rustls` + `webpki-roots`, `globset`, `semver`, `toml`, `time`, and `ed25519-dalek` only if §8-Q2 resolves toward signing. This is the first network-capable dependency in the workspace, which is itself a reviewable event: it must be reachable **only** from `mg-vault-plugin-host`'s `net.http` implementation, asserted by a dependency-graph test mirroring **J**'s.

### 7.3 Estimated scope

**XL.** The API surface is large (a WIT world, a capability enum, four crates, a command group, a TUI overlay) and, more importantly, nearly all of it is adversarial: this feature owns two auto-fail rules directly (capability bypass, traversal/symlink escape) and touches two more (partial multi-file mutation, unconfirmed overwrite). The runtime integration alone — engine configuration, linker discipline, fuel plus epoch, resource limiting, backend self-test, and a second interpreter backend — is a multi-week body of work before a single capability is enforced. The adversarial corpus and its syscall-monitoring harness are a deliverable in their own right and cannot be compressed, because they are the evidence for §1.3.

Recommended slicing, each independently reviewable: (1) manifest, capability enum, grant store, and scope matcher with **no runtime at all** — pure logic, fully unit-testable; (2) the runtime host with **only** `log` and `ui.notify` linked, plus the fault/quarantine machinery and the backend self-test; (3) `vault.read`/`vault.list` with the resolved-path guard and the denylist, plus the consent prompt and audit log; (4) `vault.write` proposals on top of **H**; (5) `index.query`, `storage`, `settings`, `clipboard`, `clock`, `random`, and **J**'s template variables; (6) `net.http` with budgets and SSRF filtering; (7) `process.spawn`, last and separately reviewed, behind its own feature flag; (8) the TUI surface. Slices 6 and 7 must not land before 1–4 are green under the adversarial corpus.

### 7.4 Blocking dependencies

- **A (foundation)** — hard blocker. `VaultDir` descriptor-relative confinement must be the mutation-path authority, not just the projection reader, and the transaction journal must exist. A plugin host on top of lexical path checks would be an auto-fail on day one.
- **H (safe note refactoring)** — hard blocker for `vault.write`. Plugin mutations *are* H transactions; without H's journal, staging, digest-driven recovery, and receipts, a multi-file plugin proposal is exactly the partial-mutation auto-fail. Slices 1–3 can proceed without H.
- **C (CLI and note operations)** — required for exit categories, `--dry-run`/`--from-plan` conventions, `doctor`, and the private confirmation types.
- **E (TUI workspace)** — required for the overlay, palette namespace, status segment, keymap rules, and sanitization discipline. Absent E, every capability remains fully available from the CLI, so E blocks only the TUI surface.
- **D (editor engine)** — required for plugin-attributed single-step undo of an applied proposal.
- **B / G** — required for `index.query` only; absent them, that capability reports `unavailable: requires index` and everything else works.
- **M** — supplies the secret-exclusion pattern list; absent M, the built-in denylist applies and is documented as the floor.
- **J** — consumer, not blocker: `plugin.<id>.<name>` variables stay fail-closed until this branch ships, which is exactly what J already specifies.
- **O (AI adapters)** — sibling, not dependency. An AI adapter is **not** a plugin: it has its own consent surface and its own contract, and it may reuse this feature's sandbox, audit, and proposal machinery but must never share a grant record with a plugin. The boundary is stated here so it is not blurred later.
- **External gates:** the project license decision inherited from **A**-§8-Q1 blocks dependency-license sign-off for a much larger dependency set, and the signing/distribution question (§8-Q2) blocks any story beyond local-path installation.

### 7.5 Non-goals

- No plugin registry, marketplace, discovery service, remote install by URL, or auto-update. Installation is from a local path in this milestone.
- No JavaScript, Python, Lua, or other embedded interpreter; no native dynamic-library plugins; no `dlopen`; no in-process third-party Rust code.
- No plugin-authored keybinding, theme override, or command that shadows a buffer-namespace key.
- No plugin access to another plugin's storage, to the grant store, to the audit log, to `$HOME`, to the vault registry, or to the process environment — none of these has a capability and none may be added without a host-API MINOR bump and its own consent line.
- No background daemon, timer, watcher, or scheduled plugin execution in this milestone; plugins run on user action or on a host-emitted event.
- No "trust this author" or "trust all future versions" option; no global permission preset; no capability that grants another capability.
- No claim of protection against a hostile kernel or root, a CPU side channel, or a runtime 0-day; and no claim that a user-granted read+network combination can be prevented rather than surfaced.

---

## 8. Open Questions

- **Q1:** Should `process.spawn` ship in the first plugin release at all, or should the first release be sandbox-only (no external commands) with spawn deferred to a second milestone? This spec specifies it fully and gates it behind the `plugin-external-commands` feature flag defaulting off, which is the conservative reading — confirm. — **blocks:** §4.6 flag default and §7.3 slice 7.
- **Q2:** Artifact signing and distribution policy: unsigned-with-a-warning (this spec's assumption), required signature from a user-managed trusted-key set, or TOFU pinning on first install? — **blocks:** §3.2 step 3, §4.5 (`ed25519-dalek`), and any future remote-install story.
- **Q3:** Should `net.http` host allowlists permit a single-label wildcard (`*.example.com`) with an explicit prompt warning, or exact `host:port` entries only? This spec assumes exact-only, which is safer but will frustrate real APIs that shard across subdomains. — **blocks:** §4.2 `HostPort` and the consent-prompt copy.
- **Q4:** Default runtime backend: `wasmtime` (fast, JIT, internal `unsafe`) or `wasmi` (interpreter, no `unsafe` in its core, 10–50× slower)? This spec chooses wasmtime as default with wasmi as a build-time fallback — confirm that trade, since it is the one place this feature knowingly accepts a dependency with internal `unsafe`. — **blocks:** §4.5 and the `--version`/`doctor` backend reporting.
- **Q5:** Should grants ever be portable? This spec makes them per-user, per-vault, non-portable, so a shared or synced vault carries no authority. A team that wants a shared plugin configuration would need to re-consent per machine. Confirm that friction is the right price. — **blocks:** §3.2 step 8 and §4.1 layout.
- **Q6:** Is the `plugin trust --auto-apply` standing approval acceptable at all, or should every plugin mutation always be previewed? This spec permits it for `create`/`replace` within a scope, off by default, never for `trash`/`relocate`. — **blocks:** §3.2 step 6 and criterion 4D's preview guarantee.
- **Q7:** What is the right default for `clock.read`? Denying a clock blunts timing side channels and fingerprinting but breaks legitimate "insert timestamp" plugins that would otherwise need no other capability. This spec makes it a granted capability at 1 s granularity. — **blocks:** §4.2 `Capability::ClockRead` and the risk classification.
