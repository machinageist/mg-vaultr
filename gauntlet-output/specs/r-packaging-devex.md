# Spec: Packaging and Developer Experience

**Feature ID:** r-packaging-devex
**Parent feature:** root
**Spec author agent:** Packaging Spec Agent (mg-vault Spec Gauntlet)
**Date:** 2026-08-30
**Iteration:** 1

---

## 1. Purpose

### 1.1 One-sentence job

Let an Arch Linux user install `mg-vault` from a package or a source checkout and reach a working, self-documenting first run — binary, shell completions, man pages, an optional **user-level** index service under a least-privilege systemd sandbox, and a plugin SDK versioned against the host API — while giving the maintainer a CI gate that proves, on a fresh machine and against a versioned adversarial fixture corpus, that the shipped artifact actually installs, runs offline, and returns a user's Markdown bytes unchanged.

### 1.2 Why it matters

`mg-vault`'s entire product claim is that **ordinary files stay authoritative and unchanged**. That claim is only as good as the artifact a user actually installs. Today there is no artifact at all: no `PKGBUILD`, no CI, no `LICENSE`, no man page, no completion script, no systemd unit, no release tarball, no checksum, and no adversarial corpus. `README.md` documents `cargo fmt`/`clippy`/`test` as prose; nothing enforces them. The workspace already declares `unsafe_code = "forbid"` and denies `clippy::all` plus `clippy::pedantic`, but no gate runs those lints, so the strongest safety posture in the repository is currently honour-system.

Three risks make packaging load-bearing rather than cosmetic for this product specifically:

1. **Byte preservation is a claim about a build, not only about source code.** Optimization level, a locale, a `git` end-of-line filter, or a rebuilt dependency could change what round-trips. Only a fixture corpus exercised through the *installed* binary demonstrates the claim.
2. **The index service (branch **B**) is a long-running daemon with a filesystem watcher over the user's entire vault.** Packaged wrong — a system unit, a broad `ReadWritePaths`, ambient network — it becomes the largest privilege in the product and a standing contradiction of criterion **1A** (indexes are disposable, source is authority). Packaged right, systemd enforces at the kernel level what the code promises: the indexer can *read* vault roots and can *write* only to disposable XDG cache and state.
3. **The plugin SDK (branch **N**) is a versioned contract shipped to third parties.** A WIT world that drifts from the host is how a sandbox turns into a compatibility guess.

`mg-vault` also ships as a suite with the sibling product `mg-calr`, whose packaging spec (`calendar/gauntlet-output/specs/h-packaging-devex.md`) this one deliberately mirrors: same PKGBUILD discipline, same static-completion rule, same reproducible-tarball and checksum-truth model, same "no privileged operation in any scriptlet" rule, and the same unresolved MIT-versus-Apache-2.0 licence question.

### 1.3 Success signal

A tag build produces a byte-identical source tarball on two runners differing in `TZ`, `umask`, hostname, and checkout path; `clean-machine-smoke` installs the resulting package into a fresh `archlinux:base` container with no Rust toolchain and, **inside `unshare -rn` with zero network interfaces**, completes a synthetic end-to-end flow (`vault register` → `vault select` → `note create` → `note read` → `note write` → `index rebuild` → `index status --json` → `search` → `interop export` → `note trash` → `note restore` → `pacman -R`); the `fixture-corpus` job round-trips all **147** adversarial fixtures through the packaged binary with **zero** changed bytes outside explicitly edited spans and zero fixtures silently normalized; `systemd-analyze security --user mg-vault-indexd.service` reports an exposure score at or below the committed ceiling; regenerated completions and man pages produce an empty `git diff`; and `pacman -R` leaves every vault, every `.obsidian` directory, and every XDG directory byte-identical.

---

## 2. User Stories

> As an Arch user, I want `makepkg -si` (or `sudo pacman -U mg-vault-0.1.0-1-x86_64.pkg.tar.zst`) to install a working binary plus completions and man pages, so that I can use the tool without reading the source tree.

> As a first-time user with no vault registered, I want my first command to fail with a short diagnosis and the exact two commands that fix it, so that I am never left guessing — and I want to be certain the tool changed nothing on my system while diagnosing.

> As a privacy-conscious user, I want the background index service to run as **my** user, to be able to *read* my vault but never *write* to it, and to have no network access at all, so that a bug or a compromised dependency in the indexer cannot alter or exfiltrate my notes.

> As a keyboard-first user in zsh, I want `mg-vault no<TAB>` and `mg-vault index --<TAB>` to complete subcommands and flags with descriptions, and I want TAB to stay instant and offline even with the index service stopped, so that completion is never a hidden code path.

> As a note-taker with a decade-old Obsidian vault full of odd YAML, CRLF files, emoji filenames, and a `.canvas` board, I want CI to prove on every commit that installing this software and reading my files back returns identical bytes, so that "your files stay yours" is evidence rather than a slogan.

> As a plugin author, I want the WIT world, the host-API compatibility table, and buildable example plugins distributed as a versioned SDK package, so that I can target a stable contract instead of reverse-engineering the host binary.

> As the maintainer, I want CI to fail on a clippy warning, drifted completions, a leaked secret, an unbuildable package, an untruthful `sha256sums` entry, a corpus fixture that changed without a manifest bump, or a scale-budget regression, so that a green run is evidence rather than habit.

---

## 3. UX Specification

This is a CLI and TUI product. "Screen" below means an installation surface, a printed document, a unit file, or a terminal transcript. **No graphical screen, modal, sheet, drawer, or popover exists anywhere in this feature.**

### 3.1 Screen / view inventory

| Surface | How reached | New / modified | Layout pattern |
|---|---|---|---|
| `makepkg -si` transcript | `cd packaging/arch && makepkg -si` | New | makepkg-owned build log + post-install message |
| `pacman -U` / `pacman -S` transcript | package install | New | pacman-owned progress + same post-install message |
| Post-install message (`mg-vault.install`) | printed by both installs | New | ≤ 10 plain-ASCII lines, ≤ 78 columns, no ANSI |
| `mg-vault --version` / `--version --json` | explicit | Modification of clap's default | Build, SQLite, schema, and plugin-API identity block |
| `mg-vault doctor --check packaging` | explicit | Modification of **C**'s `doctor` | Non-mutating check matrix, one row per stable check ID |
| First-run "no vault" error | any note/index/search command with an empty registry | Modification (`Error::NoVaultSelected` exists) | Two-line stderr diagnosis + two copyable commands |
| "index service not running" notice | `search` / `index status` with the socket absent | Modification (degraded fallback exists) | One stderr line + the `systemctl --user` command, never executed |
| bash completion menu | `mg-vault <TAB>` in bash | New | bash-completion word list |
| zsh completion menu | `mg-vault <TAB>` in zsh | New | `_arguments`-driven grouped list with descriptions |
| fish completion menu | `mg-vault <TAB>` in fish | New | description-annotated list |
| `man mg-vault` (man 1) | `man mg-vault` | New | generated roff, standard sections |
| `man mg-vault-note` … (man 1, per top-level subcommand) | `man mg-vault-note` | New | generated roff |
| `man 5 mg-vault` | `man 5 mg-vault` | New | hand-written roff: registry, `.mg-vault`, XDG layout |
| `man 5 mg-vault-plugin` | `man 5 mg-vault-plugin` | New | hand-written roff: `plugin.toml` manifest schema |
| `systemd-analyze security --user` report | maintainer / CI | New surface over new units | systemd-owned table, textual |
| `scripts/dev-install.sh` transcript | developer invocation | New | file plan → confirmation → per-file result lines |
| `docs/INSTALL.md`, `docs/SERVICE.md`, `docs/PLUGIN-SDK.md`, `docs/RELEASING.md`, `docs/PACKAGING.md`, `docs/DEPENDENCIES.md`, `docs/CORPUS.md` | `/usr/share/doc/mg-vault/`, repo | New | Markdown with fenced, machine-extractable command blocks |
| `THIRD-PARTY-LICENSES.md` | `/usr/share/licenses/mg-vault/` | New | generated attribution file |

**Nothing in this feature installs a file under `/etc`, `/var`, or `/usr/lib/systemd/system/.`** A system-wide configuration file would shadow the XDG resolution `XdgPaths::from_env` implements, and a *system* unit would run a vault indexer outside the owning user's identity — both are forbidden by the package file-list assertion in §5.2.

### 3.2 Interaction flows

**Primary flow — package install to first successful command.**

1. `sudo pacman -U ./mg-vault-0.1.0-1-x86_64.pkg.tar.zst`. pacman prints its own progress; the scriptlet prints:

```text
mg-vault stores nothing outside your vaults and your XDG directories.
Next steps:
  1) mg-vault vault register notes ~/Notes
  2) mg-vault vault select notes
  3) mg-vault index rebuild          # optional; the index is disposable
Optional background indexing (per-user, never system-wide):
  systemctl --user enable --now mg-vault-indexd.socket
Shell completions installed for bash, zsh, and fish. See mg-vault(1).
```

No command in that message is executed by the scriptlet. `mg-vault` itself never invokes `systemctl`, `sudo`, or `pacman` — asserted by a source grep and a scriptlet grep in §5.1.

2. The user runs `mg-vault note create ideas/first.md --body '# First'` with an empty registry. It fails before touching anything:

```text
mg-vault: no vault is selected and no --vault was given
register one first, then select it:
  mg-vault vault register NAME /path/to/vault
  mg-vault vault select NAME
see mg-vault(1) FILES for where the registry lives
```

Exit `3` (not found / no selection), JSON code `no_vault_selected`. Nothing was created, no directory was made, no registry file was written.

3. The user registers and selects a vault, then creates and reads a note. All of this works with the index service absent — direct-file operations never depend on it.

4. `mg-vault search first` with no index and no service:

```text
notes/first.md	First
index search: status=degraded generation=- notes=1 source=authoritative_vault_files freshness=direct_rebuild_snapshot persistence=none
degraded: indexed features (backlinks, tags, properties, tasks) are unavailable
start background indexing with: systemctl --user enable --now mg-vault-indexd.socket
```

The results are labelled as a direct source scan, never presented as a current indexed generation. The `systemctl` line is printed text; the CLI does not run it.

5. The user enables the socket. On the next `search`, the CLI connects to `$XDG_RUNTIME_DIR/mg-vault/indexd.sock`; systemd socket-activates `mg-vault-indexd.service`, which starts inside the sandbox of §4.3 and reports `current` only after a verified reconciliation.

**Branch — the sandbox cannot be applied.** If `systemd-analyze` or the daemon's startup probe finds that unprivileged user namespaces are unavailable (so `ProtectHome=`/`PrivateNetwork=` cannot take effect in a user unit), the service **still starts** under the baseline tier — which does not need namespaces — and `mg-vault doctor` reports `packaging.sandbox_tier: baseline` with the reason. It never silently claims the strict tier. There is no mode in which the service runs with *no* confinement: `RestrictAddressFamilies=AF_UNIX`, `NoNewPrivileges=yes`, and the read-only vault binds are baseline, not optional.

**Branch — vault roots changed since the drop-in was written.** `vault register` and `vault unregister` print one line noting that the service sandbox drop-in is now out of date and the exact regeneration command. `mg-vault doctor` reports `packaging.unit_dropin_current: fail` and names the missing root. The CLI never edits a unit file implicitly; `mg-vault service write-dropin` does it, prints a diff first, and writes only under `$XDG_CONFIG_HOME/systemd/user/` — never under `/usr` or `/etc`.

**Branch — prerequisite missing during a source build.** `makepkg` checksum mismatch stops before extraction with makepkg's own `FAILED` line. A `check()` failure (clippy, tests, or the fixture corpus) aborts before `package()`; CI never passes `--nocheck`.

**Completion flow.** In zsh, `mg-vault no<TAB>` completes to `note`; `mg-vault index --<TAB>` lists `--json --no-input --no-color --vault` with their doc-comment descriptions. Completion is **static**: the generated scripts contain no `$(`, no backtick, and no invocation of `mg-vault`, so pressing TAB never opens the index socket, never reads a note, never writes a file, and works with the service stopped and no network namespace.

**Secondary flow — dev install.** `scripts/dev-install.sh --dry-run` prints the complete file plan; without `--dry-run` it installs into `~/.local` and records a manifest. It refuses to run as root (exit `5`), refuses any destination outside `$HOME`, and refuses to overwrite any path `pacman -Qo` reports as owned. `--uninstall` removes only manifest-recorded paths that still hash to their recorded digest.

No haptics, sounds, or animations exist anywhere in this feature.

### 3.3 Layout descriptions

**Post-install message.** Top → bottom: one-sentence non-mutation statement; numbered next steps; the optional service line; the completion note. ≤ 10 lines, ≤ 78 columns, plain ASCII — a pacman scriptlet can render on a bare VT. Data source: a static string in `packaging/arch/mg-vault.install`. A unit test asserts it is ASCII-only, ≤ 10 lines, and contains no executed `sudo`, `systemctl`, `pacman`, or `mg-vault` invocation.

**`mg-vault --version` (human).** Fixed field order, one field per line, no colour, no alignment art:

```text
mg-vault 0.1.0
build       release, rustc 1.90.0, edition 2024
index       sqlite 3.53.2 (bundled), schema 1, parser 1
plugin api  supported 1.0-1.3, wit sha256:9f3a1c...
source      <repository URL>, commit 9f3a1c2, tag v0.1.0
```

`--version --json` emits the same facts inside the existing `{"version":1,"ok":true,"data":{…}}` envelope. This block is the anchor for CVE audits of the bundled SQLite, for host/SDK skew diagnosis, and for the `packaging.version_agreement` check.

**`man mg-vault` (man 1).** Section order: `NAME`, `SYNOPSIS`, `DESCRIPTION`, `OPTIONS` (`--json`, `--no-input`, `--no-color`, `--vault`), `COMMANDS` (one line per top-level subcommand with its clap `about`, linking to `mg-vault-<sub>(1)`), `EXIT STATUS`, `ENVIRONMENT`, `FILES`, `DATA AUTHORITY`, `JSON OUTPUT`, `EXAMPLES`, `SECURITY`, `SEE ALSO`, `BUGS`, `AUTHORS`. `NAME`/`SYNOPSIS`/`DESCRIPTION`/`OPTIONS`/`COMMANDS` are generated by `clap_mangen` from the same `clap::Command` the binary parses with. `EXIT STATUS` is generated from the single `EXIT_STATUS_TABLE` const that also drives `CliError`'s exit mapping, so documentation and behaviour cannot drift: `0` success/no-op, `2` usage or invalid input, `3` not found or no selection, `4` collision/conflict/ambiguity, `5` unsafe or denied, `6` degraded dependency, `7` I/O or transaction failure, `130` interrupt. `ENVIRONMENT` lists `HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_STATE_HOME`, `XDG_CACHE_HOME`, `XDG_RUNTIME_DIR`, `NO_COLOR`, `VISUAL`, `EDITOR`. `FILES` lists the registry path, `.mg-vault/` layout, and `/usr/share/doc/mg-vault/`. `DATA AUTHORITY` states verbatim: Markdown and attachment files are the sole authority; the SQLite index is disposable and may be deleted at any time; `pacman -R` removes no vault data. `SECURITY` states the three packaging invariants: mg-vault never invokes `sudo`, `systemctl`, or a package manager; the index service is a per-user unit with no network access and read-only vault access; nothing is installed under `/etc`.

**`man 5 mg-vault`.** `NAME`, `DESCRIPTION`, `XDG RESOLUTION` (the exact precedence `XdgPaths::from_values` implements), `REGISTRY FILE` (`registry.json`, versioned, fails closed on an unknown newer version), `VAULT CONTROL DIRECTORY` (`.mg-vault/` — trash, history, app-local settings; never synced authority), `COEXISTENCE` (`.obsidian` is read and preserved, never written by mg-vault), `INDEX LOCATION` (XDG cache, disposable), `EXAMPLE`, `SEE ALSO`.

**Unit files.** `packaging/systemd/mg-vault-indexd.socket` and `.service` install to `/usr/lib/systemd/user/`, mode 0644, root-owned, never enabled by the package. The user-writable sandbox drop-in generated by `mg-vault service write-dropin` lands at `$XDG_CONFIG_HOME/systemd/user/mg-vault-indexd.service.d/10-vault-roots.conf`.

**Empty / degraded states.** `doctor --check packaging` run on a `cargo run` binary reports every packaging row as `status: "blocked"`, `reason: "not installed from a package"`, and exits `0` — a development checkout is a legitimate configuration, not a failure. With systemd absent entirely (a container, a non-systemd init), every service row is `blocked` with `reason: "no user service manager"`, and the CLI's direct-file and direct-rebuild paths remain fully functional; this is the packaged expression of criterion **2E**.

### 3.4 Input & gestures

Everything is keyboard-driven text. Completions respond to the shell's own binding (TAB in bash/zsh/fish defaults). `man` navigation is the pager's. `scripts/dev-install.sh` prompts once for `y/N`, skips the prompt under `--yes`, and **refuses to proceed when stdin is not a TTY unless `--yes` was passed** — consent is never inferred from a pipe. `mg-vault service write-dropin` prints a unified diff and requires `y` or `--yes`; under `--no-input` without `--yes` it prints the diff, writes nothing, and exits `2`. No pointer, touch, stylus, controller, voice, or camera input exists. Responsive behaviour means terminal width only: every R-authored string wraps at or below 78 columns; man pages are validated at `MANWIDTH` 40, 80, and 200; the `--version` block and the `doctor` matrix use one field per line below 60 columns rather than truncating.

### 3.5 Transitions & animation

**N/A — no animation, transition, spinner, or progress indicator is introduced by this feature.** `pacman`, `makepkg`, `cargo`, and `systemd` render their own progress and are neither wrapped nor modified. Every R surface is write-once: no cursor repositioning, no in-place redraw, no ANSI escape emitted anywhere — asserted by a test that greps all R output for `\x1b[`. Because nothing animates, reduced-motion behaviour is identical to default behaviour and no alternative rendering path is required. `scripts/dev-install.sh` and the benchmark harness print one line per item rather than a progress bar, so both are legible in a pipe, in a CI log, and to a screen reader.

### 3.6 Error states

| Trigger | Presentation | Recovery | Data-loss risk |
|---|---|---|---|
| No vault registered/selected at first run | inline stderr; JSON `no_vault_selected`; exit 3 | the two printed commands | None — nothing was written |
| `--vault NAME` names an unregistered vault | inline stderr naming the name and listing registered names; `unknown_vault`; exit 3 | `vault register` or correct the name | None |
| `HOME` unset (a bad service environment) | inline stderr; `unsafe_path`; exit 2 | run under a real user session | None |
| Index socket absent, service not enabled | one stderr line + the `systemctl --user` command; results labelled `degraded`, `persistence=none` | enable the socket, or keep using direct scans | None — direct source read is unaffected |
| Service running but a schema/parser version is newer than this binary | `protocol_incompatible`; exit 6; both versions named; the database is **not** opened or rewritten | upgrade the client, or delete the disposable index | None — index is derived |
| Service restart-loops (unit fails 5× in 60 s) | systemd's own `failed` state; `doctor` reports `packaging.service_state: failed` with the last exit code | `journalctl --user -u mg-vault-indexd` | None — no partial index is published |
| Sandbox strict tier unavailable (no user namespaces) | `doctor` row `packaging.sandbox_tier: baseline` with reason | enable `kernel.unprivileged_userns_clone`, or accept baseline | None |
| Vault roots changed, drop-in stale | one line after `vault register`/`unregister`; `doctor` row `fail` naming the root | `mg-vault service write-dropin` | None |
| `makepkg` checksum mismatch | makepkg's `FAILED` line; stops before extraction | re-download; report a stale or hostile mirror | None |
| `makepkg` `check()` fails (clippy, tests, corpus) | build aborts before `package()` | fix source; `--nocheck` forbidden in CI | None |
| `dev-install.sh` run as root | banner + exit 5 before any write | rerun as the normal user | None |
| `dev-install.sh` target owned by pacman | per-file `REFUSED (owned by mg-vault)`; nonzero exit having written nothing | remove the package or change prefix | None — the plan is computed fully before any write |
| Completion script sourced by an old shell | the shell's own syntax error | documented minimum shell versions in `docs/INSTALL.md` | None |
| `pacman -R` while `mg-vault-indexd` is running | pacman unlinks the binary; systemd reports the unit failed on next restart | rerun `systemctl --user disable --now` then reinstall | None — vaults and XDG dirs are not package-owned |
| Package upgrade across an index schema bump | no automatic rebuild; first `index status` reports `stale` with the version evidence | `mg-vault index rebuild` | None — upgrades never write source |
| SDK installed for a host API the binary does not support | `plugin_api_unsupported`; exit 5; both the required and the supported set are named | install the matching `mg-vault-plugin-sdk` | None |

Presentation is **inline stderr in every case**: these are single-shot CLI commands with no persistent frame in which to hang a banner or a toast, stdout must stay clean for pipes and `--json`, and a nonzero exit plus one stderr line is the form that both shell scripts and `journalctl` consume correctly. Every R message is plain text with no ANSI, names a path, an object class, or a version — and never note bytes, a vault's file listing, or a secret.

### 3.7 Accessibility

- **Screen-reader labels, hints, traits.** Every R surface is plain text; there is no widget with a role to announce. The equivalent obligation is discharged by making every field self-labelling: `sandbox_tier: baseline (no user namespaces)`, `REFUSED (owned by mg-vault)`, `administrator step (not run for you)`. No state is carried by position, indentation depth, or an unlabelled symbol.
- **Custom actions for complex interactions.** The only multi-step interaction is `service write-dropin`, decomposed into: print the resolved vault roots as a numbered list → print the unified diff → one `y/N` question. Each step is a separate readable block rather than one compound prompt, so a screen-reader user hears the same decomposition a sighted user sees.
- **Text scaling / dynamic type.** The terminal's. No fixed-width art, box drawing, or column alignment is load-bearing in any R output; the `doctor` matrix and `--version` block degrade to one labelled field per line below 60 columns.
- **Color-independent state communication.** R introduces **no ANSI emitter at all**, so `--no-color` and `NO_COLOR` are trivially satisfied and are asserted by a grep test over every R surface. State words (`pass`, `fail`, `blocked`, `degraded`, `stale`) carry all meaning; stripping all colour from any R output leaves it semantically complete.
- **Focus order and keyboard navigability.** The shell's and the pager's. R adds no interactive widget with its own focus model. Every R capability is reachable from argv alone, and `--no-input` proves no path requires interaction to *refuse*.
- **Textual equivalents.** No diagram, screenshot, or image is a required carrier of any instruction in any R document; `docs/SERVICE.md`'s sandbox tiers are a table of directive names and values, not a picture. Man pages are validated with `man --warnings` at `MANWIDTH` 40/80/200 so no example is ever truncated. Completion descriptions in zsh and fish come from the same doc comments as `--help`, so the completion menu and the help text say the same thing; bash's description-free list stays usable because every subcommand name is a self-describing word.

---

## 4. Implementation Specification

### 4.1 Architecture placement

The three existing crates keep their boundaries (`docs/ARCHITECTURE.md`): `mg-vault-core` owns filesystem authority, `mg-vault-index` owns the disposable SQLite projection, `mg-vault-cli` owns argv translation and rendering and holds no filesystem policy. R adds packaging siblings and makes **one** structural source change:

- **`crates/mg-vault-cli/src/lib.rs` (new).** `mg-vault-cli` is today a binary-only crate (`src/main.rs`, package name `mg-vault`). The `Cli`, `Command`, `VaultCommand`, `NoteCommand`, `IndexCommand`, and `InteropCommand` types move into `pub mod cli` in a new library target; `main.rs` keeps `fn main`, dispatch, and rendering and imports the parser. **This move is what makes the completion and man-page contract structurally true** — both are generated from the same `clap::Command` value that parses real user input, so neither can describe a CLI that does not exist. `EXIT_STATUS_TABLE` and `error_code` also move to the library so the man page and the JSON envelope share one source.
- **`crates/mg-vault-cli/src/bin/mg-vault-gen.rs` (new).** Generator behind `required-features = ["gen"]`. Subcommands `completions --out-dir DIR` and `man --out-dir DIR`. Never installed by the package.
- **`crates/mg-vault-bench` (new, `publish = false`).** Deterministic corpus generator and the scale harness. Not shipped.
- **`packaging/arch/{PKGBUILD,PKGBUILD-git,mg-vault.install}`**, **`packaging/arch/sdk/PKGBUILD`** (the `mg-vault-plugin-sdk` package), **`packaging/systemd/{mg-vault-indexd.service,mg-vault-indexd.socket}`**, **`packaging/systemd/hardening/{baseline.conf,strict.conf}`**.
- **`completions/{mg-vault.bash,_mg-vault,mg-vault.fish}`** and **`man/{mg-vault.1,mg-vault-*.1,mg-vault.5,mg-vault-plugin.5}`** — committed generated (man 1) and hand-written (man 5) artifacts.
- **`fixtures/corpus/v1/`** with `corpus.toml`, `PROVENANCE.md`, and the fixture tree.
- **`scripts/{dev-install.sh,ci-local.sh,release.sh}`**, **`ci/{Containerfile.clean,smoke.sh,smoke-manifest.toml}`**, **`.github/workflows/{ci.yml,release.yml,scale.yml}`**, **`deny.toml`**, **`about.toml`**, **`.gitattributes`**, **`bench/baseline.json`**.
- **`crates/mg-vault-cli/tests/packaging_contract.rs`** — pure Rust assertions over packaging files, no container required.

**Layering rule:** `packaging/`, `scripts/`, `ci/`, `man/`, and `fixtures/` may read the crates' public surfaces and the committed artifacts; **no library or binary source may read anything under `packaging/` at runtime.** A bare `cargo build` must produce a runnable binary with no packaging artifact present.

### 4.2 Data model

R introduces **no vault-format change, no note-content change, and no index schema migration.** Its persistent state is a dev-install manifest and a benchmark baseline, both outside every vault.

```rust
/// Version and identity facts that must agree across every release artifact.
/// Produced once and asserted by CI; never read at runtime.
pub struct ReleaseIdentity {
    /// `CARGO_PKG_VERSION` — the single source of truth.
    pub crate_version: String,
    /// Annotated git tag, expected to be `v{crate_version}`.
    pub git_tag: String,
    /// PKGBUILD `pkgver`, expected to equal `crate_version`.
    pub pkgver: String,
    /// PKGBUILD `pkgrel`; resets to 1 on every `pkgver` bump.
    pub pkgrel: u32,
    /// Lowercase hex SHA-256 of the reproducible source tarball.
    pub source_sha256: String,
    /// Bundled SQLite amalgamation version, e.g. "3.53.2" — recorded so a
    /// SQLite advisory can be matched to a released binary.
    pub sqlite_version: String,
    /// Host plugin-API worlds this build supports, e.g. ["1.0","1.1"].
    pub plugin_api_supported: Vec<String>,
    /// SHA-256 over the canonical bytes of the shipped `wit/` tree.
    pub wit_digest: String,
}

/// One row of the non-mutating packaging readiness matrix, serialized inside
/// the existing `{"version":1,"ok":true,"data":…}` envelope.
pub struct PackagingCheck {
    /// Stable machine identifier, frozen for envelope version 1.
    pub id: &'static str,
    pub status: CheckStatus,          // Pass | Fail | Blocked
    /// Stable code when `status != Pass`; never a free-form string alone.
    pub code: Option<&'static str>,
    /// Actionable recovery text; contains no note bytes, path listing, or secret.
    pub recovery: Option<String>,
}

/// One file written by `scripts/dev-install.sh`, stored as JSON at
/// `$XDG_STATE_HOME/mg-vault/dev-install.json`. Uninstall removes only entries
/// whose current digest still equals `sha256`, so a hand-edited file survives.
pub struct DevInstallEntry { pub path: PathBuf, pub sha256: String, pub mode: u32 }

/// One corpus fixture's recorded identity. `corpus.toml` is the manifest; a
/// test asserts the tree matches it byte for byte, so a fixture cannot be
/// silently normalized by an editor, a formatter, or a git EOL filter.
pub struct CorpusFixture {
    pub path: String,          // vault-relative
    pub sha256: String,
    pub class: FixtureClass,   // Yaml | ObsidianSyntax | Unicode | Huge | Symlink | Canvas | Obsidian
    /// Why this fixture exists, in one sentence — read by `docs/CORPUS.md`.
    pub rationale: String,
    /// Named tests that must exercise it; the coverage gate fails on an empty set.
    pub exercised_by: Vec<String>,
}
```

Frozen check IDs for envelope version 1: `packaging.binary_source`, `packaging.version_agreement`, `packaging.completions_installed`, `packaging.man_installed`, `packaging.units_installed`, `packaging.no_system_unit`, `packaging.unit_dropin_current`, `packaging.sandbox_tier`, `packaging.service_state`, `packaging.sqlite_source`, `packaging.sdk_api_agreement`, `suite.calr_contract`.

### 4.3 API contracts

**Generator interface** (`mg-vault-gen`, feature `gen`):

```text
mg-vault-gen completions --out-dir DIR   # mg-vault.bash, _mg-vault, mg-vault.fish
mg-vault-gen man         --out-dir DIR   # mg-vault.1 and mg-vault-<sub>.1
```

Both are pure functions of `mg_vault_cli::cli::Cli::command()` plus the static roff fragments in `man/sections/`. Both are byte-deterministic: no timestamp, hostname, build path, locale-dependent ordering, or environment value may appear in the output; `clap_mangen`'s date field is pinned to `SOURCE_DATE_EPOCH`, defaulting to the tag commit date rather than "now". Exit `0` on success, `7` on I/O failure.

**Static-completion rule (binding).** Generation uses `clap_complete::generate` only. `clap_complete::env::CompleteEnv` and every other dynamic engine are **forbidden**, because a dynamic engine re-invokes the binary on each TAB, which would make completion a socket-opening, index-reading, vault-walking code path. Enforced by `packaging_contract.rs`: the committed bash and zsh scripts must contain no `$(`, no backtick, and no bare `mg-vault` invocation.

**systemd user units (the least-privilege contract).** `mg-vault-indexd.socket`:

```ini
[Socket]
ListenStream=%t/mg-vault/indexd.sock
SocketMode=0600
DirectoryMode=0700
Accept=no
RemoveOnStop=yes
[Install]
WantedBy=sockets.target
```

`mg-vault-indexd.service` — **user unit only; the package installs nothing under `/usr/lib/systemd/system/`**:

```ini
[Unit]
Description=mg-vault index service (per-user, read-only over vaults)
Requires=mg-vault-indexd.socket
After=mg-vault-indexd.socket
[Service]
Type=notify
ExecStart=/usr/bin/mg-vault-indexd --systemd
Restart=on-failure
RestartSec=2s
StartLimitIntervalSec=60
StartLimitBurst=5
RestartPreventExitStatus=2 5 6
WatchdogSec=60s
UMask=0077
# ---- baseline tier: no user namespace required ----
NoNewPrivileges=yes
CapabilityBoundingSet=
AmbientCapabilities=
RestrictAddressFamilies=AF_UNIX
RestrictNamespaces=yes
RestrictRealtime=yes
RestrictSUIDSGID=yes
LockPersonality=yes
MemoryDenyWriteExecute=yes
SystemCallArchitectures=native
SystemCallFilter=@system-service
SystemCallFilter=~@privileged @resources @obsolete @mount @debug @swap @reboot @module @cpu-emulation
SystemCallErrorNumber=EPERM
RuntimeDirectory=mg-vault
RuntimeDirectoryMode=0700
MemoryMax=768M
TasksMax=64
Nice=10
IOSchedulingClass=idle
```

and the **strict tier**, shipped as `/usr/lib/systemd/user/mg-vault-indexd.service.d/20-strict.conf` and applied only when the startup probe finds unprivileged user namespaces available:

```ini
[Service]
PrivateNetwork=yes
PrivateDevices=yes
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=tmpfs
ProtectProc=invisible
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectKernelLogs=yes
ProtectClock=yes
ProtectControlGroups=yes
ProtectHostname=yes
```

Design notes that make this a real contract rather than a directive list:

- **`ProtectHome=tmpfs` plus generated binds is the load-bearing pair.** A user unit whose whole job is reading `$HOME` cannot use `ProtectHome=yes`. `tmpfs` blanks the home and the generated drop-in re-admits exactly the needed paths: `BindReadOnlyPaths=` for **each registered vault root** and `BindPaths=` for `$XDG_CACHE_HOME/mg-vault` and `$XDG_STATE_HOME/mg-vault` only. The vault roots being **read-only at the kernel level** is the packaging-level enforcement of criterion **1A**: the index service is structurally incapable of writing a note, an attachment, or `.obsidian`, regardless of any bug in the watcher or in a dependency.
- **`PrivateNetwork=yes` is honest, not decorative** — B specifies zero network bytes for the service, search, and Markdown indexing, so the indexer genuinely needs no network. Because `PrivateNetwork=` in a *user* unit requires user namespaces, `RestrictAddressFamilies=AF_UNIX` is in the **baseline** tier, where it needs no namespace and already forecloses every IP socket. The strict tier adds the empty netns on top. Neither tier is optional in the sense of "the service may open a network socket".
- **`MemoryDenyWriteExecute=yes` is safe here** because the indexer contains no JIT. Plugins (**N**) run `wasmtime` in the CLI/TUI process, **not** in `mg-vault-indexd`; if that ever changes, MDWX must be removed from this unit in the same commit, and `packaging_contract.rs` asserts the indexer's dependency graph contains no WASM runtime crate.
- **`Type=notify` without libsystemd.** Readiness is signalled by writing `READY=1` to the `AF_UNIX` datagram address in `$NOTIFY_SOCKET` using `rustix` (already a workspace dependency). No `libsystemd` dependency, no `unsafe`, no FFI — the workspace's `unsafe_code = "forbid"` holds.
- **Directory authority.** `RuntimeDirectory=` is used for the socket directory. `StateDirectory=`/`CacheDirectory=` are deliberately **not** used, because their user-unit mapping changed across systemd versions (`$XDG_DATA_HOME` before v255, `$XDG_STATE_HOME` after). The daemon resolves its own paths through `XdgPaths::from_env()`, exactly as the CLI does, and treats any systemd-created directory as a hint, never as authority. This removes a whole class of version-skew bug rather than documenting it.
- **Restart policy.** `Restart=on-failure` with a 5-in-60 s limit recovers from a crash; `RestartPreventExitStatus=2 5 6` means a usage error, a denied/unsafe condition, or an incompatible schema **fails closed and stays down** instead of restart-looping against an unfixable state. `WatchdogSec=60s` catches a wedged watcher.
- **Activation choice.** Socket activation, not `.path` activation. A `.path` unit would need to watch vault roots, duplicating the daemon's own watcher, and vault roots are dynamic registry data a static unit cannot know. Socket activation also makes the CLI's "is the service available?" question a single `connect()` with no `systemctl` invocation.
- **Never enabled by the package.** `mg-vault.install` prints the `systemctl --user enable --now` line; it does not run it. Background indexing is a user decision.

**PKGBUILD contract** (`packaging/arch/PKGBUILD`, normative fields):

```bash
pkgname=mg-vault
pkgver=0.1.0                       # == CARGO_PKG_VERSION, CI-asserted
pkgrel=1
pkgdesc='Local-first Markdown vault authority with a disposable index'
arch=('x86_64')                    # aarch64 added only when CI builds it
url='<canonical repository URL>'
license=('MIT')                    # placeholder; releases gated until Q1 resolves
depends=('gcc-libs' 'glibc')
optdepends=('bash-completion: command completion for bash'
            'mg-calr>=0.1.0: task promotion to the calendar suite'
            'mg-vault-plugin-sdk: WIT world and example plugins')
makedepends=('cargo' 'clang')
source=("$pkgname-$pkgver.tar.gz::<release tarball URL>")
sha256sums=('<64 hex>')            # 'SKIP' is forbidden; CI greps for it
options=('!lto')                   # LTO is set in [profile.release], not here

prepare() { export RUSTUP_TOOLCHAIN=stable
            cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"; }
build()   { export RUSTUP_TOOLCHAIN=stable CARGO_TARGET_DIR=target
            cargo build --frozen --release --workspace; }
check()   { export RUSTUP_TOOLCHAIN=stable
            cargo clippy --frozen --workspace --all-targets -- -D warnings
            cargo test  --frozen --workspace --all-targets; }
package() { install -Dm755 target/release/mg-vault    "$pkgdir/usr/bin/mg-vault"
            install -Dm755 target/release/mg-vault-indexd "$pkgdir/usr/bin/mg-vault-indexd"
            install -Dm644 packaging/systemd/mg-vault-indexd.service \
              "$pkgdir/usr/lib/systemd/user/mg-vault-indexd.service"
            # socket, strict drop-in, completions, man 1/5, docs, licences — §3.1
          }
```

`check()` runs offline and needs no service, no network, and no database server. `package()` installs no `/etc` path, no `/usr/lib/systemd/system/` unit, no `sysusers.d` or `tmpfiles.d` fragment, and no prebuilt `.wasm`. `mg-vault.install` contains only `post_install`/`post_upgrade` functions that `echo`; no `sudo`, `systemctl`, `pacman`, `mg-vault`, or `rm` may appear in any scriptlet, asserted in §5.1.

`PKGBUILD-git` builds `mg-vault-git` with `provides=('mg-vault')`, `conflicts=('mg-vault')`, and `pkgver()` from `git describe`.

**SDK package contract** (`packaging/arch/sdk/PKGBUILD` → `mg-vault-plugin-sdk`). Its **`pkgver` is the host plugin-API version, not the host binary version** — `1.3.0`, not `0.1.0` — because that is the number a plugin author targets. It installs:

```text
/usr/share/mg-vault/wit/mg-vault-plugin@1.3.0.wit      # the versioned world
/usr/share/mg-vault/sdk/compat.toml                    # api ↔ host-version table
/usr/share/mg-vault/sdk/examples/{wordcount,frontmatter-lint,ledger-bridge}/
/usr/share/mg-vault/sdk/template/                      # skeleton crate + plugin.toml
/usr/share/doc/mg-vault-plugin-sdk/PLUGIN-SDK.md
/usr/share/man/man5/mg-vault-plugin.5.gz               # manifest schema
```

`depends=('mg-vault>=0.1.0')` with the floor computed from `compat.toml`, never hand-written. Three binding rules: (1) the shipped WIT bytes must hash to the digest the host reports from `mg-vault --version --json`, asserted by the `sdk-contract` CI job, so the SDK cannot describe a world the host does not implement; (2) **examples ship as source plus a build recipe, never as prebuilt `.wasm`** — a binary blob in a package is unauditable, and the whole point of **N**'s sandbox is that the user can know what code runs; (3) `ledger-bridge`, the only example requesting `process.spawn`, ships with that capability declared and **ungranted**, and its README states in its first paragraph that installing it grants nothing. Deprecation follows **N**'s policy verbatim: an import marked `@deprecated` keeps working for at least two minor releases and twelve months, and the host serves major `N` and `N-1` concurrently for one full release cycle; `compat.toml` records the window and CI asserts no existing signature changed within a major.

**`mg-vault doctor --check packaging [--json]`.** Read-only. Reads `/proc/self/exe`, the installed artifact paths, `systemctl --user show` (query only), and `pacman -Qo` when present. It executes no privileged command, starts no unit, opens no network socket, and writes nothing. Auth: none beyond the invoking user's own filesystem access.

**Release artifact contract.** `scripts/release.sh v0.1.0` produces into `dist/`:

| Artifact | Construction | Determinism guarantee |
|---|---|---|
| `mg-vault-0.1.0.tar.gz` | `TZ=UTC git archive --format=tar --prefix=mg-vault-0.1.0/ v0.1.0 \| gzip -9n` | git tree order is deterministic; `--prefix` fixes paths; `gzip -n` drops name/mtime; entries stamped from the tag commit; `.gitattributes` `export-ignore` fixes the file set |
| `mg-vault-0.1.0-vendor.tar.zst` (optional) | `cargo vendor --locked`, then `tar --sort=name --owner=0 --group=0 --numeric-owner --mtime=@$SOURCE_DATE_EPOCH` | lockfile-pinned inputs, normalized metadata |
| `SHA256SUMS` | `sha256sum` over the artifacts, `LC_ALL=C` sorted | plain text, one line per artifact |
| `SHA256SUMS.asc` | detached OpenPGP signature | **gated** — produced only once a release key exists (Q3) |

**No prebuilt binary is published in v1**, because a binary would carry a reproducibility claim (toolchain, glibc, bundled-SQLite build path) the project cannot yet substantiate, and the PKGBUILD builds from source regardless.

**Checksum truthfulness.** `sha256sums` in both PKGBUILDs is regenerated by `updpkgsums` inside `scripts/release.sh` and never hand-edited. CI job `checksums` independently rebuilds the tarball from the tag, recomputes SHA-256, asserts three-way equality with `SHA256SUMS` and both PKGBUILD arrays, then runs `makepkg --verifysource`. A `SKIP` entry, a missing entry, or an array-length mismatch fails the job.

**Signing posture (stated honestly, in tiers).** Tier 1, in force today: SHA-256 checksums over a tarball any third party can reproduce from the public git tag — integrity without identity. Tier 2, the next step and cheap: **signed annotated git tags** (`git tag -s`), which makes the tarball's provenance verifiable from the repository alone. Tier 3: a detached `SHA256SUMS.asc` plus `validpgpkeys` in the PKGBUILD, requiring a long-lived, published release key. **No key is claimed to exist today.** `release.yml` records which tier a release used in its notes, so a consumer is never misled about what a checksum does and does not prove.

### 4.4 State management

R introduces no application runtime state, no store, and no controller. Ownership:

- **Installed-file state** is owned by pacman's local database. R never duplicates or second-guesses it; `doctor` queries `pacman -Qo` read-only and degrades to `blocked` when pacman is absent.
- **Service lifecycle state** is owned by the **user's** systemd manager. `mg-vault` reads it (`systemctl --user show`) and never mutates it: no `start`, `enable`, `daemon-reload`, or `restart` is ever invoked by the binary or by a scriptlet.
- **Sandbox drop-in state** is owned by the user, under `$XDG_CONFIG_HOME/systemd/user/`. It is derived from the vault registry, regenerable at any time, and never authoritative: deleting it degrades the service to the packaged tiers, never to zero confinement.
- **Index state** remains owned by `mg-vault-index` under `$XDG_CACHE_HOME/mg-vault/indexes/` and remains **disposable**. Install, upgrade, and removal never rebuild, migrate, or delete it; a schema bump is reported as `stale`, and the user rebuilds. This is the packaging-side expression of criterion **1A**: packaging must not become a second authority.
- **Vault state** — Markdown, attachments, `.obsidian`, `.mg-vault` — is owned by the user and is **never touched by any packaging path**. `pacman -R` removes only `/usr` files.
- **Dev-install state** is the only file R writes outside a package: `$XDG_STATE_HOME/mg-vault/dev-install.json`, written atomically (same-directory temp + `rename`, matching `mg-vault-core`'s existing `atomic.rs` discipline) so an interrupted install leaves either the old manifest or the new one.
- **Local vs. server-synced boundary:** N/A in the sync sense — nothing in R synchronizes, and R adds no network client. The only boundary is package-owned (`/usr`, replaced wholesale on upgrade) versus user-owned (`$HOME`, XDG directories, vaults — never touched by pacman).
- **Offline / draft persistence:** N/A — R has no editable document, no session, and no partially entered input. `dev-install.sh` and `service write-dropin` both compute their complete plan before writing anything, so there is no half-applied state to resume; rerunning either is idempotent.

### 4.5 Dependencies

**New Rust crates.**

| Crate | Scope | Purpose | License |
|---|---|---|---|
| `clap_complete` 4.x | `gen` feature only | completion generation from the clap tree | MIT OR Apache-2.0 |
| `clap_mangen` 0.2 | `gen` feature only | roff generation from the clap tree | MIT OR Apache-2.0 |

Both are behind `required-features = ["gen"]`, so a default `cargo build` compiles neither into the shipped binary. **No new runtime dependency is added.** `rustix` (already present) covers the `sd_notify` write.

**New non-Rust tooling** — CI and maintainer machines only, never a runtime dependency of the installed binary: `pacman-contrib` (`updpkgsums`), `namcap`, `shellcheck`, `gitleaks`, `cargo-deny`, `cargo-about`, `podman` or `docker`, `systemd-analyze` (from `systemd`, already present on Arch). The installed package depends on `gcc-libs` and `glibc` alone.

**New assets/resources.** Committed generated artifacts (`completions/*`, `man/*.1`), hand-written `man/mg-vault.5`, `man/mg-vault-plugin.5`, and `man/sections/*.roff`; the versioned fixture corpus under `fixtures/corpus/v1/`; example plugin sources; `THIRD-PARTY-LICENSES.md`. **No fonts, images, icons, audio, video, or ML models** — the product has no graphical surface and the TUI (**E**) ships no font and requires none.

**Infrastructure.** No CDN, no hosted service, no telemetry endpoint, no database server. A git forge is required for hosted CI and release hosting; **the repository currently has no remote** (`git remote -v` is empty), which makes the forge choice an open question (Q4) rather than an assumption. `scripts/ci-local.sh` runs the identical job list on the workstation, so every gate exists before a forge does.

### 4.6 Platform-specific considerations

- **Primary target:** Arch Linux `x86_64`, glibc, dynamically linked, on the developer's Hyprland workstation. `arch=('x86_64')` only; `aarch64` is added to the array in the same commit that adds an `aarch64` CI builder, never before, so the package never advertises an untested architecture.
- **Toolchain:** Rust edition 2024, `rust-version = "1.85"`. A dedicated `msrv` job builds at exactly 1.85 so the declared MSRV is evidence-backed. `RUSTUP_TOOLCHAIN=stable` is exported in `prepare`/`build`/`check` per Arch Rust packaging guidance so a rustup override cannot silently change the build.
- **`[profile.release]` (new — the workspace has none today):** `lto = "thin"`, `codegen-units = 1`, `strip = false` (makepkg strips and can emit a `-debug` package), `panic = "unwind"` explicitly retained. `panic = "abort"` is rejected: it would replace a deterministic exit with a SIGABRT and break the exit-status contract the man page is generated from.
- **`rusqlite` `bundled` — the decision that must be stated, not assumed.** The workspace sets `rusqlite = { version = "0.40.2", features = ["bundled"] }`, so `libsqlite3-sys` compiles the **SQLite 3.53.2 amalgamation** into the binary via the `cc` crate. Consequences, all real: (a) the shipped binary contains ~9 MB of C source' worth of object code, so `unsafe_code = "forbid"` governs *our Rust*, not the whole artifact — stated plainly rather than implied away; (b) a SQLite advisory requires a `mg-vault` rebuild, **not** a `pacman -Syu` of the system `sqlite`, which is a genuine maintenance obligation and the reason `--version` reports `sqlite 3.53.2 (bundled)` and `doctor` exposes `packaging.sqlite_source`; (c) Arch packaging convention prefers system libraries, so `namcap` reasoning is recorded in `docs/PACKAGING.md` rather than left as an unexplained deviation; (d) it buys version determinism and a guaranteed FTS5 build that **B** depends on. Mitigations shipped with the decision: a `system-sqlite` cargo feature (`rusqlite` without `bundled`, `depends=('sqlite')`) kept compiling in CI so the choice stays reversible, and hardening defines passed through `SQLITE3_FLAGS`/`CFLAGS` — `-DSQLITE_OMIT_LOAD_EXTENSION` (removes the `load_extension` code-loading surface entirely), `-DSQLITE_DQS=0`, `-DSQLITE_DEFAULT_MEMSTATUS=0`, `-DSQLITE_OMIT_DEPRECATED`, `-DSQLITE_THREADSAFE=1`, plus Arch's `makepkg.conf` `CFLAGS` (`-D_FORTIFY_SOURCE=3 -fstack-protector-strong`), which the `cc` crate honours. Which variant ships by default is Q2.
- **systemd version floor:** the units require systemd ≥ 249 for the baseline tier and note that `ProtectHome=tmpfs` plus `BindReadOnlyPaths=` in a *user* unit needs unprivileged user namespaces; the strict drop-in is applied only after the probe confirms them. `docs/SERVICE.md` states both tiers and how to check which is active. Nothing claims a hardening directive is in force when it is not.
- **Shells:** bash ≥ 4.2 with `bash-completion` (so the completion file must be named `mg-vault`, matching the command); zsh ≥ 5.0 with `compinit` and `/usr/share/zsh/site-functions` on `$fpath` (Arch default); fish ≥ 3.0 reading `/usr/share/fish/vendor_completions.d`.
- **Non-systemd and non-Arch systems:** `mg-vault` remains fully functional without systemd — `index rebuild` and the direct-file paths need no daemon. `docs/INSTALL.md` documents `cargo install --path crates/mg-vault-cli` plus manual completion/man installation for other distributions; no claim of packaged support for them is made.
- **Feature flags / rollout:** `gen` (developer generator), `system-sqlite` (build variant). Staged delivery is expressed by `ci/smoke-manifest.toml`: every new user-facing command must be added to the smoke manifest, or explicitly listed as `excluded_with_reason`, before CI passes — so the packaged surface and the tested surface grow together.

### 4.7 Performance budget

Each number is a CI-enforced ceiling on the reference Arch workstation or in the CI container, not an aspiration.

- **Binary size:** stripped `mg-vault` ≤ 14 MiB and `mg-vault-indexd` ≤ 14 MiB (both include the SQLite amalgamation); `.pkg.tar.zst` ≤ 8 MiB. Recorded per build; a >15% regression versus the previous tag fails.
- **Startup:** `mg-vault --version` and `mg-vault vault list` p95 ≤ 20 ms cold, ≤ 10 ms warm — these read only the environment and a small JSON registry and must not open SQLite. `mg-vault index status` against a running service p95 ≤ 150 ms including connect (**B**'s figure).
- **Completion latency:** sourcing the bash script ≤ 5 ms; producing candidates for `mg-vault <TAB>` ≤ 10 ms. Static scripts spawn no subprocess, so latency is independent of service and network state — verified with the service stopped.
- **Memory:** `--version` / `vault list` RSS ≤ 12 MiB. Service RSS is **B**'s budget (≤ 256 MiB steady state for the baseline corpus, 512 MiB hard ceiling); R enforces it as `MemoryMax=768M` in the unit, deliberately above the application ceiling so the application's own backpressure reports lag before the kernel kills the process.
- **Build:** clean `makepkg` (fetch + build + check, including the SQLite C compile) ≤ 10 min on the reference workstation.
- **CI:** full PR pipeline ≤ 20 min with jobs parallelized; `clean-machine-smoke` ≤ 6 min end to end; `fixture-corpus` ≤ 3 min.
- **Scale gate (criterion 3E), nightly and on-tag.** A deterministic generated corpus of **100,000 notes / 1,000,000 blocks / 2,000,000 combined link, tag, property, and task projections** (`mg-vault-bench gen --notes 100000 --blocks 1000000 --seed 1`, ≈ 3.2 GiB, ≤ 6 min to generate). Named budgets, each failing on a >15% p95 regression against the committed `bench/baseline.json` for the same runner class: cold full `index rebuild` ≤ 12 min wall and ≤ 512 MiB peak RSS; `index status` on a current generation p95 ≤ 50 ms; first-page (`limit=50`) search p95 ≤ 250 ms and regex p95 ≤ 500 ms (**C**'s figures); `note read` by exact path p95 ≤ 15 ms **while a full rebuild runs concurrently**, with p99 added latency ≤ 20 ms (**B**'s editing-isolation figure — this is the number that proves indexing never blocks writing); index database size ≤ 2.5× the corpus Markdown bytes. Updating the baseline requires an explicit commit whose message names the cause.
- **Why the scale gate is scheduled, not per-commit.** Generation plus a cold rebuild plus the latency matrix is a ~25-minute, ~4 GiB-of-disk job; running it on every push would multiply PR wall clock by more than three and make the queue the bottleneck on a single-maintainer project, which reliably ends in the gate being disabled. Instead: (a) `scale.yml` runs the full 100,000/1,000,000 matrix nightly on `main` and on every tag, and a tag cannot be released while it is red; (b) **every** PR runs a 1/50-scale shape job (2,000 notes / 20,000 blocks, ≤ 90 s) through the identical harness, which catches complexity regressions because it asserts the *ratio* between the 2,000- and 20,000-note points, not just absolute times; (c) every PR runs a structural guard asserting that no direct-file command path performs a recursive directory walk or an index call — the property **A** relies on to keep a 100,000-note vault responsive.
- **Network payload:** release tarball ≤ 2 MiB; optional vendor tarball ≤ 30 MiB. **The installed application transfers zero bytes over the network** — see §6.4, Lens 5A.
- **Storage:** installed footprint ≤ 34 MiB (two binaries + 3 completion scripts + man pages + docs + licences). Corpus in-repo ≤ 40 MiB (the "huge file" fixtures are generated at test time from a recorded recipe and digest, not committed as blobs). Dev-install manifest ≤ 8 KiB. R creates no cache directory of its own.

---

## 5. Test Specification

### 5.1 Unit tests

In `crates/mg-vault-cli/tests/packaging_contract.rs` — pure Rust, no container, runs in the default `cargo test`:

| Test | Setup / assertion | Edge case covered |
|---|---|---|
| `completions_are_regenerable_and_byte_stable` | Generate into a temp dir twice under different `TZ`, `LANG`, `PWD`; assert identical bytes and equality with the committed `completions/*` | Locale-dependent ordering |
| `man_pages_match_committed_artifacts` | Same for `man/*.1`; assert every top-level subcommand in the clap tree has a `mg-vault-<sub>.1` | A subcommand added without regeneration |
| `completions_are_static_and_offline` | Committed bash/zsh/fish contain no `$(`, no backtick, no line invoking `mg-vault` | An accidental switch to a dynamic completion engine |
| `exit_status_table_is_total_and_documented` | Every `CliError`/`Error` variant's code appears in `EXIT_STATUS_TABLE`, and every row appears in the rendered `mg-vault.1` | A new error variant with an undocumented code |
| `scriptlet_runs_no_privileged_command` | `mg-vault.install` contains no `sudo`, `systemctl`, `pacman`, `rm`, or `mg-vault` invocation; is ASCII-only; ≤ 10 lines; ≤ 78 columns | A helpful-but-forbidden `systemctl enable` |
| `binary_never_invokes_a_service_manager` | Grep all crate sources for `systemctl`, `sudo`, `pkexec`, `pacman` outside string literals that are *printed* guidance | A convenience auto-start creeping in |
| `unit_is_user_scope_and_hardened` | Parse both units: no `[Install] WantedBy=multi-user.target`; every baseline directive from §4.3 present with the exact value; `RestrictAddressFamilies` is exactly `AF_UNIX` | A directive silently dropped in a refactor |
| `indexd_has_no_wasm_runtime_dependency` | `cargo metadata` over `mg-vault-indexd`'s graph contains no WASM runtime crate | `MemoryDenyWriteExecute` silently breaking a future JIT |
| `dropin_paths_are_read_only_over_vault_roots` | Generate a drop-in for three registered roots; assert each appears in `BindReadOnlyPaths` and none in `BindPaths`; assert only XDG cache/state are writable | A writable vault bind |
| `no_r_surface_emits_ansi` | Every R-authored string constant contains no `\x1b[` | A coloured "success" message |
| `pkgbuild_fields_agree_with_cargo` | `pkgver == CARGO_PKG_VERSION`; `sha256sums` has no `SKIP`; array length equals `source` length; `arch` is exactly `('x86_64')` | A version bump in one file only |
| `corpus_manifest_matches_tree` | Every `corpus.toml` entry's SHA-256 equals the on-disk file; every on-disk file has an entry | A fixture normalized by an editor or a git EOL filter |
| `corpus_coverage_is_total` | Every fixture's `exercised_by` names at least one test that exists | A fixture added and never asserted on |
| `wit_digest_matches_reported` | The `sdk/compat.toml` digest equals a fresh hash of `wit/` and the value `--version --json` reports | SDK describing a world the host lacks |

`.gitattributes` marks `fixtures/corpus/**` as `-text` and `binary` so git can never convert an end-of-line in a fixture; `corpus_manifest_matches_tree` is the test that would catch it if the attribute were removed.

### 5.2 Integration tests

Container-based in CI, reproducible locally via `scripts/ci-local.sh`:

1. **Lint and test gate.** `cargo fmt --all --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings` (the workspace already denies `clippy::all` and `pedantic` and forbids `unsafe_code`, so this job turns a declaration into a gate, and also catches rustc lints the manifest does not cover); `cargo test --workspace --all-targets --all-features`; `msrv` build at exactly 1.85.
2. **Package build gate.** `archlinux:base-devel`, non-root builder: `makepkg --syncdeps --noconfirm --check` for both `PKGBUILD` and `PKGBUILD-git`, then `namcap PKGBUILD` and `namcap *.pkg.tar.zst` with zero errors and a reviewed, committed warning allowlist.
3. **Package content assertion.** `bsdtar -tf` the built package; assert the exact expected path set (both binaries, three completion files at their canonical shell paths, `mg-vault.1`, one `mg-vault-<sub>.1` per subcommand, `mg-vault.5`, `/usr/lib/systemd/user/mg-vault-indexd.{service,socket}`, the strict drop-in, docs, `THIRD-PARTY-LICENSES.md`); assert **no** path under `/etc`, `/var`, `/usr/lib/systemd/system/`, `/usr/lib/sysusers.d`, or `/usr/lib/tmpfiles.d`; assert no `.wasm` file.
4. **Unit verification and security score.** `systemd-analyze verify` on both units (zero errors); `systemd-analyze security --user mg-vault-indexd.service` with the exposure score at or below the committed ceiling in `packaging/systemd/security-baseline.txt`. A directive removal that raises the score fails the job.
5. **Sandbox behaviour test.** Start the service under both tiers in a container. Assert: connecting to the socket activates it; the process holds **zero** non-`AF_UNIX` sockets; an attempted write to a vault file from inside the service's mount namespace fails with `EROFS` under the strict tier; the service's writable set is exactly the XDG cache and state directories; a forced exit `5` does **not** trigger a restart (`RestartPreventExitStatus`); a `SIGKILL` **does**, at most five times per minute.
6. **Checksum truth.** Rebuild the tarball from the tag, recompute SHA-256, assert three-way equality with `SHA256SUMS` and both PKGBUILD arrays; run `makepkg --verifysource`.
7. **Reproducibility twin build.** Build the tarball twice in containers differing in `TZ`, `umask`, hostname, and checkout path; assert identical SHA-256. Failure prints a `diffoscope` summary.
8. **Licence and dependency gate.** `cargo deny check licenses advisories bans sources` against the explicit allowlist in §6.2; `cargo about generate` regenerates `THIRD-PARTY-LICENSES.md` and `git diff --exit-code` proves it is current. A crate with no licence field, an unlisted licence, or an open advisory fails.
9. **Secret scan.** `gitleaks detect` over full history, the working tree, `dist/`, the generated artifacts, and the **contents of the built package**; plus a project rule set for vault-shaped secrets. The corpus's deliberately secret-shaped fixture is allowlisted by path and digest so the gate stays sharp instead of being globally suppressed.
10. **Offline assertion.** Assert `Cargo.lock` contains no HTTP, TLS, or async-network client crate in the default feature set, so the shipped binary has no network capability to misuse. When **L** or **O** later add one, it becomes an explicit named allowlist entry rather than a silent addition.
11. **`fixture-corpus` — the byte-preservation gate.** A versioned corpus at `fixtures/corpus/v1/` (147 fixtures), run through the **packaged** binary, not a debug build:
    - **Unusual YAML:** duplicate keys; anchors, aliases, and merge keys; tabs in indentation; `---` inside the body; an unterminated frontmatter fence; CRLF-only frontmatter; block scalars `|`, `>`, `|-`, `>+`; flow maps and flow sequences; YAML 1.1 booleans (`yes`, `no`, `on`, `off`, `y`, `n`); sexagesimal `12:30:00`; leading-zero octals; `null`, `~`, and empty values; a 40 KiB single scalar; non-string keys; a BOM before `---`.
    - **Unknown Obsidian syntax:** `%%comments%%`, `==highlight==`, `![[embed#^block-id]]`, `[[link|alias]]`, `#nested/tag`, foldable callouts `> [!warning]- Title`, block IDs `^abc123`, footnotes, `$$math$$`, Dataview inline fields `key:: value`, Templater `<% tp.date.now() %>`, and a `.base` file.
    - **Unicode edge cases:** NFC/NFD pairs that compare equal but differ in bytes; combining marks; ZWJ emoji families; RTL override U+202E; zero-width joiners and non-joiners; astral-plane code points; a filename that is **invalid UTF-8 raw bytes** (legal on Linux, already handled by `path_json`'s `unix_bytes_hex` encoding); mixed CRLF/LF/CR; NEL, LS, PS separators; a file with no trailing newline; a file that is only a BOM.
    - **Huge files:** a 64 MiB note, a 1,000,000-line note, a note with a single 200,000-character line, a note with 50,000 frontmatter keys — generated at test time from a recorded recipe plus digest rather than committed as blobs.
    - **Symlinks and special files:** an intra-vault symlink; a symlink escaping the vault; a symlink loop; a dangling symlink; a directory symlink into `.obsidian`; a hardlink; a FIFO; a unix socket; a device node.
    - **JSON Canvas:** a valid 1.0 file; one with an unknown top-level key; one with an unknown node type and unknown node/edge properties; `\u`-escaped strings; negative and non-integer coordinates; tab versus 4-space indentation; no trailing newline.
    - **Coexistence:** a populated `.obsidian/` tree with plugin settings and a workspace file.

    **Assertions, per fixture:** (a) read-then-write-back with no edit produces **byte-identical** output including BOM, EOL style, and trailing-newline presence; (b) a frontmatter scalar edit changes only the bytes inside the located span, proven by a diff whose hunks are confined to that span; (c) `interop export` is deterministic across two runs and across debug and release builds — so optimization level can never alter serialization; (d) `index rebuild` followed by deleting the database and rebuilding produces equivalent ordered results and **zero** changed source fingerprints; (e) every unsafe path (escape, loop, special file) is **refused with a typed error**, never partially processed; (f) `.obsidian` bytes are identical before and after the full sweep; (g) no fixture's bytes appear in any log, diagnostic, benchmark report, or CI output.
12. **Clean-machine smoke.** Fresh `archlinux:base` with **no Rust toolchain**, running under `unshare -rn` (zero network interfaces): `pacman -U` the built package; run the §1.3 flow; exercise `--json`, `--no-input`, `--no-color`, and `NO_COLOR=1`; pipe `note read` into `note create` in another vault and assert a byte-exact round trip; source each completion script in its shell and assert `mg-vault <TAB>` produces candidates with the service absent; render every man page at `MANWIDTH` 40/80/200 with `man --warnings` and zero warnings; then `pacman -R` and assert both vaults, both `.obsidian` trees, and every XDG directory are byte-identical to their pre-removal state.
13. **Suite coherence.** With `mg-calr` installed, assert `doctor` row `suite.calr_contract` reports `pass` and the version pair; with it absent, `blocked` — never `fail`, because the calendar is optional. Assert `mg-vault` declares `mg-calr` in `optdepends` and never in `depends`.

### 5.3 UI / E2E tests

R has no GUI; "E2E" means a scripted terminal session driven under a PTY so completion, paging, and TTY-dependent behaviour are exercised as a user experiences them.

1. **Install-to-first-note.** PTY session: install → run a note command with an empty registry → assert the exact two-line diagnosis, the two copyable commands, exit `3`, and that `$XDG_CONFIG_HOME/mg-vault/` was **not** created → register → select → create → read → assert byte equality.
2. **Completion navigation, three shells.** In bash, zsh, and fish under a PTY with the service stopped: `mg-vault <TAB>` lists all top-level subcommands; `mg-vault no<TAB>` uniquely completes `note`; `mg-vault index --<TAB>` lists the global flags; zsh and fish show descriptions matching `--help`. Assert no `mg-vault` process was spawned by completion (process-accounting check) and no socket was opened.
3. **Service enable and degrade.** Enable the socket → `search` connects and reports `current` → `systemctl --user stop` → `search` reports `degraded`, prints the enable hint, and still returns direct-scan results → delete the index database entirely → `search` still succeeds from source. At no point does the CLI run `systemctl`.
4. **Sandbox tier honesty.** Run with user namespaces available and unavailable; assert `doctor` reports `strict` and `baseline` respectively, with a reason string in the second case, and that both start successfully.
5. **Drop-in staleness.** Register a second vault → assert the one-line staleness notice → `doctor` reports `unit_dropin_current: fail` naming the new root → `service write-dropin` prints a diff, asks once, writes only under `$XDG_CONFIG_HOME` → `doctor` reports `pass`. Under `--no-input` without `--yes`, assert the diff prints, nothing is written, and the exit is `2`.
6. **Narrow terminal and no-colour.** Repeat flows 1 and 3 at `COLUMNS=40` with `NO_COLOR=1`; assert one labelled field per line, no truncation of any command the user must copy, and zero ANSI bytes in the captured stream.
7. **Dev-install refusals.** As root → exit `5`, nothing written. Targeting a pacman-owned path → per-file `REFUSED`, nothing written. `--uninstall` after hand-editing one installed file → that file is kept and reported, the rest are removed.
8. **Upgrade path.** Install tag N-1 → create notes and build an index → install tag N → assert every note byte is unchanged, `.obsidian` is unchanged, the index reports `stale` rather than being silently rebuilt or deleted, and one explicit `index rebuild` restores `current`.

### 5.4 Visual / manual verification

- **Theme variants:** N/A as light/dark — R emits no colour at all. The equivalent check is that every R surface renders identically under a light terminal, a dark terminal, `NO_COLOR=1`, `--no-color`, and a `TERM=dumb` session, verified by a captured-stream comparison rather than by eye.
- **Text size extremes:** man pages read at `MANWIDTH` 40, 80, and 200; the `doctor` matrix and `--version` block reviewed at `COLUMNS` 40, 80, and 200 for wrapping that never splits a copyable command.
- **Screen size extremes:** the post-install message reviewed on a bare VT (80×25, no font loaded, ASCII only) and in a 200-column terminal.
- **Empty vs. populated states:** `doctor --check packaging` reviewed in four configurations — a `cargo run` development checkout (all `blocked`), a packaged install with no service enabled, a packaged install with the service running under the strict tier, and a packaged install on a system with no systemd at all. All four must be self-explanatory without consulting the source.
- **Screen-reader pass:** one full install-to-first-note session read aloud through a terminal screen reader, confirming that field labels, state words, and the two recovery commands are unambiguous in linear reading order.
- **Manual review of every generated artifact before a tag:** the rendered `mg-vault.1`, each completion script, and the `systemd-analyze security` report are read by a human as part of `docs/RELEASING.md`'s checklist — generation makes drift impossible, not review unnecessary.

---

## 6. Compliance & Safety Gate

### 6.1 Sensitive data classification

- [ ] No sensitive data involvement
- [x] **Handles sensitive data — describe protection measures.** R does not read, parse, or transmit note content in normal operation, but it defines three surfaces through which private content could leak, so it is treated as content-adjacent. **(1) The fixture corpus and benchmark corpora** are entirely synthetic and generated from recorded seeds; no real vault, no real note, and no personal `.obsidian` configuration enters the repository or CI. Assertion (g) in §5.2 job 11 requires that no fixture byte ever appears in a CI log, a diagnostic, or a benchmark report. **(2) The index service's sandbox** is the primary protection for real user content: vault roots are bound **read-only**, the writable set is exactly two disposable XDG directories, `RestrictAddressFamilies=AF_UNIX` (baseline) and `PrivateNetwork=yes` (strict) mean the daemon holding a full view of the user's notes has no path to send them anywhere, `UMask=0077` and `RuntimeDirectoryMode=0700` keep the socket owner-only, and `ProtectProc=invisible` keeps other processes' state out of view. **(3) Packaging diagnostics** — `doctor`, `--version`, the drop-in diff, and every error string — name paths, versions, and object classes and never note bytes or a vault's file listing. The secret-scan gate covers history, tree, `dist/`, generated artifacts, and the built package.
- [x] **Uses synthetic/test data only until compliance gate clears.** Every fixture, benchmark corpus, example plugin, and documentation example is synthetic or project-authored. No real user vault is used anywhere in CI or in any document.

### 6.2 Asset provenance

- [ ] No third-party assets
- [x] **Uses third-party assets — list each with source, license, and rights status.**

No fonts, images, icons, audio, video, datasets, or ML models are bundled; the product has no graphical surface. The third-party material is Rust crates, one vendored C library, and the fixture corpus. Licences below were read from each crate's manifest in this workstation's registry, not assumed.

| Class | Crates in `Cargo.lock` | License posture | Rights status |
|---|---|---|---|
| **Vendored C — SQLite amalgamation 3.53.2** | shipped inside `libsqlite3-sys` via the `bundled` feature | **The SQLite source itself is in the public domain**; the `libsqlite3-sys` wrapper is **MIT** | Clear, but **must be stated in `THIRD-PARTY-LICENSES.md` and `docs/PACKAGING.md`**: the shipped binary contains a complete third-party C library, its version is reported by `--version`, and a SQLite advisory obliges a `mg-vault` rebuild rather than a system `sqlite` upgrade |
| SQLite bindings | `rusqlite` (**MIT**), `libsqlite3-sys` (**MIT**), `hashlink`, `fallible-iterator`, `fallible-streaming-iterator`, `smallvec` | MIT, or MIT OR Apache-2.0 | Clear; permissive |
| **YAML editing** | **`yaml-edit` — `Apache-2.0` only**, plus `rowan`, `text-size`, `countme` (MIT OR Apache-2.0) | **Apache-2.0 without an MIT alternative** | Clear, and **materially relevant to Q1**: this is the one dependency that is not dual-licensed. Apache-2.0 is permissive and imposes no copyleft, so either project licence works — but the distributed binary carries Apache-2.0's attribution and NOTICE obligations regardless of what `mg-vault` itself is licensed as, which is precisely why `THIRD-PARTY-LICENSES.md` ships in `/usr/share/licenses/mg-vault/` and is CI-regenerated |
| CLI parsing / terminal styling | `clap`, `clap_builder`, `clap_derive`, `clap_lex`, `anstream`, `anstyle*`, `colorchoice`, `strsim`, `heck`, `is_terminal_polyfill` | MIT OR Apache-2.0 | Clear |
| Generation (new, build-time only) | `clap_complete`, `clap_mangen` | MIT OR Apache-2.0 | Clear; behind `gen`, never linked into the shipped binary |
| Serialization | `serde`, `serde_core`, `serde_derive`, `serde_json`, `itoa`, `memchr`, **`zmij` (MIT only)** | MIT OR Apache-2.0, one MIT-only | Clear; `zmij` is a less-familiar transitive float-formatting crate and is flagged for individual review at its next version bump |
| Hashing / digest | `sha2`, `digest`, `block-buffer`, `crypto-common`, `generic-array`, `typenum`, `cpufeatures` | MIT OR Apache-2.0 | Clear |
| Platform / syscalls | `rustix` (Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT), `linux-raw-sys`, `errno`, `libc`, `bitflags`, `getrandom`, `once_cell` | Permissive; the LLVM exception is compatible with either project licence | Clear |
| Hash tables | `hashbrown`, **`foldhash` (Zlib)**, `rustc-hash` | Permissive; Zlib is in the allowlist explicitly | Clear |
| Build-time | `cc`, `pkg-config`, `shlex`, `version_check`, `proc-macro2`, `quote`, `syn`, `unicode-ident`, `rustversion` | MIT OR Apache-2.0 | Clear; not shipped |
| Errors | `thiserror`, `thiserror-impl` | MIT OR Apache-2.0 | Clear |
| Dev-only, never shipped | `tempfile`, `fastrand` | MIT OR Apache-2.0 | Clear; excluded from the package |
| Target-gated, never built on Linux | `windows-sys`, `windows-link`, `find-msvc-tools`, `vcpkg`, `wasm-bindgen*`, `js-sys`, `sqlite-wasm-rs`, `rsqlite-vfs`, `r-efi`, `bumpalo`, `once_cell_polyfill` | Permissive | Present in the lockfile, absent from the Linux build; `cargo deny` still evaluates them |
| **Fixture corpus** | `fixtures/corpus/v1/` | Project-authored | Every fixture is authored by this project or generated from a recorded recipe; **none is copied from a public vault.** `PROVENANCE.md` records origin, generating tool and version, rationale, and licence status per fixture. **JSON Canvas 1.0 is an open specification (MIT) that is implemented, not vendored** |
| Terminal glyphs | box drawing, Nerd Font icons | N/A | Unicode code points supplied by the user's terminal font; **no font is shipped or required** (**E**'s decision, restated here as a packaging fact) |

**Enforcement, not assertion.** `deny.toml` sets an explicit allowlist — `MIT`, `Apache-2.0`, `Apache-2.0 WITH LLVM-exception`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Zlib`, `Unicode-3.0`, `Unlicense`, `CC0-1.0`, `blessed { public-domain: sqlite }` — and CI fails on any crate outside it, any crate with no licence field, any duplicate-version ban violation, and any open advisory. `cargo about` regenerates `THIRD-PARTY-LICENSES.md` and a `git diff --exit-code` proves it current, so a new transitive crate cannot enter without its notice shipping.

**Unresolved project licence (Q1).** `mg-vault` ships **no `LICENSE` file today** — verified absent in the working tree — and `README.md` states plainly: *"No license file is present: the project license choice (MIT versus Apache-2.0) remains unresolved."* Other specs in this repository flag the same open item (`h-safe-note-refactoring.md` §7.4 records it as an external gate). This is a real blocker for R, not a footnote: a PKGBUILD without a resolvable `license=()` is not publishable to the AUR, and a source tarball with no grant is not redistributable. `release.yml` therefore contains a hard `license-gate` job that fails every tagged release while `LICENSE` is absent, naming Q1. Two facts for the decision, neither of which decides it: the dependency graph is overwhelmingly dual `MIT OR Apache-2.0`, so either choice is compatible; and **`yaml-edit` is Apache-2.0-only**, so Apache-2.0 attribution obligations attach to the distributed binary under either choice, making the shipped `THIRD-PARTY-LICENSES.md` mandatory either way. The Rust-ecosystem convention of dual `MIT OR Apache-2.0` would maximize downstream reuse and add an explicit patent grant. **This spec does not decide it.**

### 6.3 Language / claims audit

- [x] **Makes claims not supported by evidence?** **No — and several statements were deliberately narrowed to keep it that way.** `arch=('x86_64')` only, because no `aarch64` builder exists. The strict sandbox tier is described as *probed and reported*, never as always in force, because `ProtectHome=`/`PrivateNetwork=` in a user unit require user namespaces that may be unavailable. No prebuilt binary is published, because binary reproducibility is not yet demonstrated. `SHA256SUMS.asc` is described as gated on a key that is **not** claimed to exist. MSRV 1.85 is claimed only because a job builds at exactly 1.85. `unsafe_code = "forbid"` is stated as governing *our Rust crates*, with the bundled SQLite C explicitly excluded from that claim. Scale budgets are attributed to the nightly job that measures them, with the per-commit substitute described as a *shape* check, not as full-scale evidence.
- [x] **Promise capabilities not yet built?** **No.** The systemd units, the SDK package, and the scale gate all describe artifacts for features (**B**, **N**) that are specified but unimplemented; §7.1 marks each `absent` or `planned`, §7.4 records the blocking order, and no post-install message, man page, or document tells a user that installing the package gives them an index daemon, a plugin host, or a TUI today. The post-install message lists only steps that work against the current implementation. `mg-vault-gen` is not installed, so no advertised command lacks a man page.
- [x] **Use language restricted by domain regulations?** **No.** This is a personal knowledge tool; no health, financial, legal, or safety claim appears in any packaging string, unit description, man page, or document. The word "secure" is never used as a bare adjective: security statements are specific and testable ("vault roots are bound read-only", "`RestrictAddressFamilies=AF_UNIX`", "mg-vault never invokes systemctl"), and each is backed by a named test in §5.

### 6.4 Regulatory alignment

Walking `criteria.md` lens by lens. Lens 2 and Lens 3 are **largely out of scope for a packaging spec** and are marked N/A with explicit deferral rather than padded.

**Lens 1 — Data ownership and format compatibility (addressed).**
- **1A Authority.** No packaging path makes an index authoritative. Install, upgrade, and removal never rebuild, migrate, or delete an index, and never write a note. The service's vault binds are **read-only at the kernel level** (§4.3), which is a stronger guarantee than code discipline. `pacman -R` removes only `/usr` files, proven byte-for-byte in §5.3 flow 8 and §5.2 job 12.
- **1B Preservation.** This is R's strongest contribution: the 147-fixture corpus asserts byte-identical round trips through the **packaged** binary across unusual YAML, unknown Obsidian syntax, Unicode edge cases, huge files, and JSON Canvas, plus span-confined edits and debug/release serialization equality — so a build configuration can never become a lossy transform.
- **1C Identity.** Nothing in packaging injects, derives, or persists a note identifier. `pkgver`, `pkgrel`, and the release identity describe the *software*, never a note. A corpus assertion confirms no fixture gains a property or an ID across the full sweep.
- **1D Coexistence.** The corpus contains a populated `.obsidian/` tree asserted byte-identical before and after the sweep; the service's read-only binds cover it; `man 5 mg-vault` documents `.obsidian` as read-and-preserved and `.mg-vault` as app-local; no scriptlet touches either.
- **1E Transactions.** No packaging step performs a multi-file mutation. `makepkg` fails closed before `package()`; `dev-install.sh` and `service write-dropin` compute their full plan before writing and are idempotent; the dev-install manifest is written with the same same-directory-temp-plus-`rename` discipline `mg-vault-core::atomic` already uses; the corpus asserts every unsafe path is refused rather than partially processed.

**Lens 2 — Editor and TUI excellence (N/A with deferral, one criterion genuinely addressed).**
- **2A–2D — N/A.** R ships no editor, no modal grammar, no buffer, no undo stack, no pane model, and no text-manipulation code. Keyboard completeness, editing durability, workspace coherence, and grapheme-safe cursor operations are wholly owned by **D** (editor engine) and **E** (TUI workspace) and are specified there. Claiming coverage here would be fabricated: R's only keyboard surface is shell completion, and asserting that TAB works is not a claim about modal editing. **Deferred to D and E without qualification.**
- **2E Degraded experience — addressed, because it is a packaging property.** The installed product must remain usable when its optional parts are absent, and R proves it: §5.3 flow 3 stops the service, deletes the index database entirely, and asserts direct source read and write still succeed with results **explicitly labelled** degraded; §3.3 specifies that with no systemd at all every service check reports `blocked` with a reason rather than failing; the "no vault selected" and "service not running" messages name the missing prerequisite and the exact recovery command instead of failing opaquely.

**Lens 3 — Knowledge retrieval and structure (N/A with deferral, except 3E).**
- **3A–3D — N/A.** R defines no link resolver, no ambiguity policy, no query grammar, and no derived view. Determinism of links and search, ambiguity handling, query depth, and the "derived data stays a view" rule belong to **B** (index service and search) and **G** (links, search, graph). R's only relationship to them is negative and enforced: the package installs no `/etc` file that could shadow XDG resolution, no unit that writes to a vault, and no path by which a derived store could become authoritative. **Deferred to B and G.**
- **3E Scale — addressed, and R owns the evidence.** §4.7 specifies the nightly 100,000-note / 1,000,000-block generated corpus with named budgets (cold rebuild ≤ 12 min and ≤ 512 MiB peak RSS; `index status` p95 ≤ 50 ms; first-page search p95 ≤ 250 ms and regex p95 ≤ 500 ms; **`note read` p95 ≤ 15 ms with p99 added latency ≤ 20 ms while a full rebuild runs concurrently** — the number that proves indexing never blocks editing; index size ≤ 2.5× corpus bytes), a ratcheted `bench/baseline.json` failing on a >15% p95 regression, the precise reason the full matrix runs nightly and on tags rather than per-commit, and the two per-commit substitutes: a 1/50-scale shape job asserting the ratio between scale points, and a structural guard that no direct-file command path performs a recursive walk or an index call.

**Lens 4 — Security, reliability, extensibility (addressed).**
- **4A Confinement.** Two independent layers. In code, `mg-vault-core`'s existing path validation rejects traversal, symlink escape, and `.obsidian`/`.mg-vault` mutation; the corpus's nine symlink and special-file fixtures assert refusal, not partial processing. In packaging, the service's mount namespace makes vault roots read-only and everything else in `$HOME` invisible, so even a total failure of the code layer cannot produce a write.
- **4B Concurrency.** R adds no writer and therefore no new stale-write surface. Its obligation is not to damage the existing one, discharged by: the upgrade test asserting `SourceFingerprint` semantics are unchanged across versions; the corpus asserting `interop export` is identical between debug and release builds so optimization cannot alter a fingerprint input; and no scriptlet, unit, or upgrade hook writing a vault file at all.
- **4C Least privilege.** The centrepiece. The index daemon is a **user** unit — never system-level — with `NoNewPrivileges=yes`, an empty capability bounding set, `RestrictAddressFamilies=AF_UNIX` and `PrivateNetwork=yes`, `ProtectSystem=strict` and `ProtectHome=tmpfs` with generated `BindReadOnlyPaths=` for vault roots and writable binds for exactly two disposable XDG directories, a `@system-service` seccomp filter minus seven dangerous groups, `MemoryDenyWriteExecute=yes`, `RestrictNamespaces=yes`, `ProtectProc=invisible`, `UMask=0077`, a 0600 socket in a 0700 directory, and hard `MemoryMax`/`TasksMax` caps. The active tier is *reported*, never assumed. `mg-vault` itself never invokes `systemctl`, `sudo`, or `pacman`, and no scriptlet performs a privileged action. The SDK ships examples with capabilities declared and **ungranted**, preserving **N**'s deny-by-default posture across the distribution boundary. `-DSQLITE_OMIT_LOAD_EXTENSION` removes SQLite's own code-loading surface from the shipped build.
- **4D Recovery.** `pacman -R` is proven non-destructive to vaults, `.obsidian`, and XDG data. A schema bump reports `stale` rather than silently rebuilding or deleting. `docs/RELEASING.md` documents downgrade as "install the older package; the index is disposable, rebuild it" — never as a data migration. `dev-install.sh --uninstall` is digest-guarded and removes only unmodified files it recorded. `RestartPreventExitStatus=2 5 6` means a fail-closed condition stays closed instead of restart-looping, and no partial index is ever published.
- **4E Contracts.** Completions and man pages are generated from the live clap tree with a CI drift gate, so documentation cannot describe a CLI that does not exist. `EXIT_STATUS_TABLE` is one const shared by the exit mapping and the man page. The JSON envelope's `version: 1` is exercised in the smoke flow. The SDK's WIT digest must equal what the host reports, and no signature may change within a major. `--version --json` publishes the full release identity including the bundled SQLite version and the supported plugin-API set.
- **4F Privacy.** The secret-scan gate covers history, tree, `dist/`, generated artifacts, and the built package. Every fixture and benchmark corpus is synthetic. No R diagnostic prints note bytes or a vault listing. The service's socket is owner-only, its egress is `AF_UNIX`-only, and `Cargo.lock` is asserted to contain no HTTP or TLS client in the default feature set, so the shipped binary has no network capability to misuse.

**Lens 5 — Performance and accessibility (addressed).**
- **5A Offline / local-first.** Install, first run, `doctor`, `--version`, completion, man rendering, `index rebuild`, and `search` are all offline. `clean-machine-smoke` runs the entire flow inside `unshare -rn` with zero network interfaces; the completion E2E runs with the service stopped and asserts no socket is opened; §5.2 job 10 asserts the lockfile has no network client. Network use during `makepkg`'s source fetch and `cargo fetch --locked` is the *build system's*, happens before the artifact exists, and is bounded by lockfile pinning plus SHA-256 verification — it is not application behaviour.
- **5B Responsiveness.** §4.7 gives explicit binary-size, startup, completion-latency, memory, build, CI, and scale budgets, each with a CI ceiling and a regression threshold; `MemoryMax`/`TasksMax` bound the daemon at the kernel level.
- **5C Accessible equivalents.** Every R surface is text; no diagram, screenshot, or image carries any instruction; the `systemd-analyze security` report and the sandbox tier tables are textual; man pages are validated at three widths with zero warnings. R introduces no graph, canvas, media, or card representation that would need a textual equivalent.
- **5D Terminal resilience.** All R text is ≤ 78 columns and ASCII; the `doctor` matrix and `--version` block degrade to one labelled field per line below 60 columns; R emits **no ANSI at all**, asserted by a grep test, so colour independence is structural; `NO_COLOR` and `--no-color` are honoured and exercised in the smoke flow; nothing animates, so reduced motion is the only motion.
- **5E Automation.** The smoke flow exercises `--json`, `--no-input`, `--no-color`, `NO_COLOR`, and stdin/stdout piping with a byte-exact round trip. `doctor --check packaging --json` returns the frozen check-ID matrix. `--no-input` without `--yes` refuses to write a drop-in and exits `2` — consent is never inferred from a non-TTY.

---

## 7. Gap Analysis vs. Current State

### 7.1 What exists today

- **absent** — `packaging/` in any form. No `PKGBUILD`, no `PKGBUILD-git`, no `.install` scriptlet. Nothing in the repository produces an installable artifact. Verified: `find . -iname 'PKGBUILD*'` returns nothing.
- **absent** — any CI configuration. No `.github/`, no workflow file, no `.gitlab-ci.yml`. `git remote -v` is empty, so there is no forge. The quality commands exist only as prose in `README.md` under "Development".
- **absent** — man pages and any `man/` directory.
- **absent** — shell completions and any generation path. `clap_complete` and `clap_mangen` are not in `Cargo.toml` and no `[features]` section exists.
- **absent** — `LICENSE`. `README.md` states explicitly that the MIT-versus-Apache-2.0 choice is unresolved. This **gates** AUR publication and every tagged release.
- **absent** — `scripts/`, `deny.toml`, `about.toml`, `.gitattributes`, `THIRD-PARTY-LICENSES.md`, secret scanning, release tooling, checksums, `dist/`.
- **absent** — every systemd unit, and the `mg-vault-indexd` binary they would start. There is no daemon: `README.md` states "There is not yet a long-running watcher/service … or daemon lifecycle."
- **absent** — the fixture corpus. Existing tests construct their own small temporary vaults; there is no versioned adversarial corpus, no `PROVENANCE.md`, and no byte-preservation sweep across unusual YAML, Obsidian syntax, Unicode, huge files, symlinks, or JSON Canvas.
- **absent** — any benchmark harness or scale evidence. No `bench/`, no generated corpus, no baseline file.
- **absent** — the plugin SDK, the WIT world, and every example plugin. **N** is specified, not built.
- **implemented, but privately inside the binary** — the clap command tree. `Cli`, `Command`, `VaultCommand`, `NoteCommand`, `IndexCommand`, and `InteropCommand` are defined in `crates/mg-vault-cli/src/main.rs` and the crate has **no library target**, so nothing outside `main` can reach them. This is the single structural blocker for generated completions and man pages.
- **implemented** — the CLI surface the artifacts must document: `vault register|list|select`, `note create|read|write|trash|restore`, `index rebuild|status`, `search`, `interop export`, with global `--json`, `--no-input`, `--no-color`, `--vault`, and a `{"version":1,"ok":…}` envelope.
- **implemented** — lint posture in the workspace manifest: `unsafe_code = "forbid"`, `clippy::all = "deny"`, `clippy::pedantic = "deny"`. **No gate runs them**, and there is no `[profile.release]` section.
- **implemented** — `rusqlite 0.40.2` with the `bundled` feature, which vendors the **SQLite 3.53.2** amalgamation and compiles it via `cc`. Its licensing and maintenance consequences are documented nowhere today.
- **implemented** — XDG resolution (`crates/mg-vault-core/src/xdg.rs`), the vault registry, path confinement, atomic replacement, fingerprints, trash/restore, byte-preserving frontmatter edits, the disposable SQLite projection, and 1,615 lines of tests across four test files.
- **implemented, narrower than the target** — error handling collapses every failure to `ExitCode::from(1)`. The target taxonomy (`0/2/3/4/5/6/7/130`) that `EXIT STATUS` will be generated from is specified in **A** §365 and **C** §350 but not yet implemented.
- **prototyped** — developer install. `README.md` documents `cargo fmt`/`clippy`/`test` and `cargo run`; `cargo install --path` would work implicitly but installs no completions, no man pages, and no units, and is not documented.
- **planned** — the index service and daemon (**B**), the plugin host and SDK (**N**), the TUI (**E**), and Quickshell/suite integration (**Q**, whose spec does not yet exist in this repository). R specifies their packaging without implementing them.

### 7.2 Delta to spec

**New files.** `packaging/arch/{PKGBUILD,PKGBUILD-git,mg-vault.install}`; `packaging/arch/sdk/PKGBUILD`; `packaging/systemd/{mg-vault-indexd.service,mg-vault-indexd.socket,security-baseline.txt}` and `packaging/systemd/hardening/{baseline.conf,strict.conf}`; `crates/mg-vault-cli/src/lib.rs` and `src/cli.rs`; `crates/mg-vault-cli/src/bin/mg-vault-gen.rs`; `crates/mg-vault-bench/`; `completions/{mg-vault.bash,_mg-vault,mg-vault.fish}`; `man/{mg-vault.1,mg-vault-*.1,mg-vault.5,mg-vault-plugin.5}` and `man/sections/*.roff`; `fixtures/corpus/v1/**` with `corpus.toml` and `PROVENANCE.md`; `scripts/{dev-install.sh,ci-local.sh,release.sh}`; `ci/{Containerfile.clean,smoke.sh,smoke-manifest.toml}`; `.github/workflows/{ci.yml,release.yml,scale.yml}`; `deny.toml`; `about.toml`; `.gitattributes`; `bench/baseline.json`; `THIRD-PARTY-LICENSES.md`; `docs/{INSTALL.md,PACKAGING.md,SERVICE.md,RELEASING.md,DEPENDENCIES.md,PLUGIN-SDK.md,CORPUS.md}`; `CONTRIBUTING.md`; `crates/mg-vault-cli/tests/packaging_contract.rs`; `LICENSE` (gated on Q1).

**Modified files.** `crates/mg-vault-cli/src/main.rs` — move every clap type and `error_code` into the new library, keep `main`, dispatch, and rendering; add `EXIT_STATUS_TABLE` and implement the `0/2/3/4/5/6/7/130` taxonomy in place of the current blanket exit `1`; extend `--version` to the identity block of §3.3. `crates/mg-vault-cli/Cargo.toml` — add `[lib]`, `[features] gen`, the two optional generator dependencies, and `[[bin]] mg-vault-gen` with `required-features`. `Cargo.toml` (workspace) — add `crates/mg-vault-bench` to `members`, add `[profile.release]` (`lto = "thin"`, `codegen-units = 1`, `strip = false`, `panic = "unwind"`), and add the `system-sqlite` feature plumbing. `README.md` — add Installation, Packaging, and Service sections and link the new documents. `docs/ARCHITECTURE.md` — record the `cli` library boundary and the packaging layering rule. `docs/SECURITY.md` — record the service sandbox tiers and the bundled-SQLite posture. `.gitignore` — add `/dist`.

**Migrations / schema changes.** **None.** R adds no index migration, no registry schema change, and no note-format change. The packaging-content assertion and the scriptlet grep exist specifically to keep it that way.

**New dependencies.** `clap_complete` and `clap_mangen`, both feature-gated and build-time only. CI/maintainer tooling: `pacman-contrib`, `namcap`, `shellcheck`, `gitleaks`, `cargo-deny`, `cargo-about`, a container runtime. **No new runtime dependency**; the installed package depends on `gcc-libs` and `glibc` alone.

### 7.3 Estimated scope

**XL.** The Rust delta is modest — one module extraction, a ~70-line generator, an exit-code table, a `--version` block — but R spans seven independently hard workstreams, each carrying its own evidence requirement: two release PKGBUILDs plus an SDK package whose `pkgver` tracks the host API rather than the binary; a hardened two-tier systemd user unit with a *generated*, registry-derived sandbox drop-in and a probe that must report the active tier honestly; three shells' completions plus roff generation with a drift gate; a 147-fixture adversarial corpus with a digest manifest, a coverage gate, and git attributes that keep CRLF fixtures intact; a reproducible release pipeline with three-way checksum agreement and a tiered signing posture; a thirteen-job CI pipeline whose flagship installs into a fresh container under an empty network namespace; and a nightly 100,000-note/1,000,000-block scale gate with a ratcheted baseline. The container and scale jobs are also the slowest to iterate on.

Deliver as dependency-ordered slices, not one patch: **(1)** library extraction + exit-code taxonomy + `--version` block + generator + committed artifacts + drift test *(M)*; **(2)** the fixture corpus, `.gitattributes`, manifest and coverage gates *(M — the highest value per unit of effort, and it is useful before any package exists)*; **(3)** PKGBUILD + `.install` + package-content assertions + `namcap` *(M)*; **(4)** release, checksum, licence, and `cargo-about` tooling *(S)*; **(5)** the CI pipeline and `ci-local.sh` *(M)*; **(6)** clean-machine smoke *(M)*; **(7)** systemd units, the drop-in generator, and the sandbox tests — **blocked on B's daemon existing** *(L)*; **(8)** the SDK package and examples — **blocked on N** *(M)*. Slices 1–6 are deliverable against the code that exists today.

### 7.4 Blocking dependencies

- **The `mg-vault-cli` library extraction blocks completions and man pages absolutely.** Generated artifacts cannot come from a `Cli` type that lives privately in a binary crate. First task in the first slice; nothing else in R depends on anything outside the repository.
- **The exit-code taxonomy (**A** §365, **C** §350) blocks the generated `EXIT STATUS` section.** Generating a table from today's blanket exit `1` would produce a truthful but useless man page. Implement the table with the extraction.
- **Q1 (project licence) blocks publication and tagged releases, not local `makepkg`.** The `license-gate` job makes the block explicit rather than letting an unlicensed tarball ship.
- **Q4 (CI host) blocks the *hosted* pipeline, not the gates.** `scripts/ci-local.sh` runs the identical job list on the workstation, so R's substance is deliverable before a forge exists.
- **B (index service) blocks the systemd slice entirely.** There is no `mg-vault-indexd` binary to start. The units, the drop-in generator, and the sandbox tests are specified now so that B's daemon lands into a designed confinement rather than acquiring one afterwards — but they cannot be built or tested until the daemon exists. B's own spec §481 already names R as the owner of the systemd user unit and the clean-machine test, so the coupling is mutual and recorded on both sides.
- **B also blocks the full scale gate's *subject*, not its harness.** The corpus generator and the shape job are buildable today against `index rebuild` and `search`; the `note read` p95-during-rebuild budget requires a concurrent rebuild, which requires the service.
- **N (plugins) blocks the SDK package, the WIT world, and every example plugin.** R specifies distribution and versioning; N owns the API itself. `mg-vault-plugin-sdk` cannot be published before N's world is frozen at 1.0.
- **Q (Quickshell and suite integration) has no spec in this repository yet**, so the `mg-suite` meta-package and any Quickshell deep-link handler are out of scope for R iteration 1 and are recorded as Q6 rather than assumed. The `mg-calr` `optdepends` and the `suite.calr_contract` doctor check are deliverable now because `mg-calr`'s CLI contract is already published in its own H spec.
- **E (TUI) does not block R**, but `docs/DEPENDENCIES.md` — which E's spec §532 names as a release gate for its terminal crates — is created by R slice 4 and must exist before E's dependencies can be released.

---

## 8. Open Questions

- **Q1:** MIT, Apache-2.0, or the Rust-conventional dual `MIT OR Apache-2.0` for `mg-vault` itself? — blocks: §6.2 rights status, the PKGBUILD `license=()` field, AUR publication, and every tagged release via `license-gate`. New evidence for the decision: **`yaml-edit` is Apache-2.0-only**, so Apache-2.0 attribution obligations attach to the distributed binary under either choice and `THIRD-PARTY-LICENSES.md` must ship regardless. Recommendation for consideration only, matching `mg-calr`'s: dual `MIT OR Apache-2.0`. The decision is the user's.
- **Q2:** Keep `rusqlite`'s `bundled` feature (SQLite 3.53.2 compiled in; version determinism and guaranteed FTS5, but a rebuild is required for every SQLite advisory and Arch convention prefers system libraries), or ship `depends=('sqlite')` with the `system-sqlite` feature as the default and `bundled` as a fallback? — blocks: §4.6's default build variant, the `depends` array, and the maintenance policy in `docs/PACKAGING.md`. Both variants stay compiling in CI either way, so this is reversible; it should still be decided before the first tag.
- **Q3:** Is there an OpenPGP key available for signing, and should the project adopt tier 2 (signed annotated git tags) now and tier 3 (`SHA256SUMS.asc` + `validpgpkeys`) later? — blocks: §4.3's signing rows only. SHA-256 checksums are mandatory regardless and are not gated on this.
- **Q4:** Where does CI run — GitHub Actions, a self-hosted runner on the Proxmox homelab, or `scripts/ci-local.sh` on the workstation only? The repository has no git remote today. — blocks: the concrete workflow files and the nightly scale job's runner class (which the ratcheted baseline is keyed to), not the job definitions.
- **Q5:** Publish to the AUR as `mg-vault` / `mg-vault-git` / `mg-vault-plugin-sdk`, or keep all PKGBUILDs repository-local for now? — blocks: whether `release.yml` needs an AUR push step and a maintainer identity line. Does not block local `makepkg -si`.
- **Q6:** Should an `mg-suite` meta-package (`depends=('mg-vault' 'mg-calr')`) exist, and does it wait for the **Q** Quickshell spec — which does not exist in this repository yet — to define the suite's shared contracts and version-skew policy? — blocks: §4.3's package list only. `optdepends` on `mg-calr` plus the `suite.calr_contract` doctor check is deliverable now either way.
- **Q7:** Is 147 the right corpus size, and should the "huge file" fixtures be generated at test time from a recorded recipe (keeping the repository small, as specified) or committed as real blobs (making the corpus self-contained but adding ~70 MiB to every clone)? — blocks: §4.7's storage budget and `docs/CORPUS.md` only.
- **Q8:** Should the nightly scale gate also run under the strict sandbox tier, or only under baseline? Running it under strict measures what users actually get but makes the job depend on the runner having unprivileged user namespaces available. — blocks: `scale.yml`'s runner requirements, not the budgets.

Resolved by this spec and not open for implementation reinterpretation: no `/etc` configuration and no system-level systemd unit; no privileged operation in any scriptlet and no `systemctl`/`sudo`/`pacman` invocation by the binary; static (never dynamic) completions; generation from the live clap tree with a CI drift gate; committed generated artifacts; vault roots bound **read-only** in the service unit; `RestrictAddressFamilies=AF_UNIX` in the baseline tier, not only the strict one; the active sandbox tier reported, never assumed; no prebuilt binary and no prebuilt `.wasm` in v1; `panic = "unwind"`; `arch=('x86_64')` until a builder exists; the SDK package versioned by host API rather than binary version; and a byte-preservation corpus gate that runs against the packaged binary rather than a debug build.
