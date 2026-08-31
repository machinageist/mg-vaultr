# Spec: Capture, Clipping, and Extraction

**Feature ID:** l-capture-clipping-extraction
**Parent feature:** root
**Spec author agent:** Capture/clipping spec agent (L)
**Date:** 2026-08-29
**Iteration:** 1

---

## 1. Purpose

### 1.1 One-sentence job

Let a person move an idea, a web page, or a document into their vault in one keystroke-cheap command — and have what lands there be an ordinary, inert, honestly-labeled Markdown file that records where it came from, when, and how faithfully.

### 1.2 Why it matters

Every other branch of `mg-vault` assumes notes already exist. L is the only branch that manufactures them from outside material, and it is the only branch that legitimately opens a socket or spawns a foreign parser. That makes it the highest-risk surface in the product and the one users are most likely to point at untrusted input.

Three failures are specific to this branch. First, **capture friction**: a capture that takes three seconds and a decision is a capture that does not happen, so the fast path must be a single append with no prompts, no index, and no network. Second, **clipped content is hostile by default** — a page is authored by someone else and may contain script, event handlers, `javascript:` URLs, a title engineered into `../../.obsidian/app.json`, or a response that never ends. Third, **derived text lies quietly**: an HTML-to-Markdown conversion drops content, and OCR invents characters. A knowledge base that cannot distinguish a verbatim quote from a machine transcription is worse than one that has neither.

L answers all three at the ingest boundary, before bytes become a file, and records what it did in ordinary visible YAML so the user can audit it with `cat`.

### 1.3 Success signal

Over the versioned capture corpus — hostile-HTML fixtures shared with **F**, adversarial page titles, a loopback fault-injecting HTTP server, and a scanned/text/encrypted PDF set — all of the following hold: (a) **zero** written vault byte across the entire corpus contains an active script, event handler, `javascript:`/`data:` destination, `<iframe>`, `<object>`, or `<style>` — asserted against the file on disk, not the render; (b) every produced path passes `Vault::validate_note_path` and canonicalizes beneath the vault root, over a property test of 100,000 fuzzed titles; (c) a socket-denying test harness proves **no** command other than an explicit `clip`/`clip --images download` invocation attempts a connection, including `capture`, `inbox drain`, `extract`, and the TUI; (d) clipping the same URL twice with unchanged upstream bytes leaves the file byte-identical and reports `changed: false`; (e) a crash injected at each of 14 phases of capture and refile loses no capture and produces no double-append or inbox-deleted-first state; and (f) every OCR-derived note carries `extraction_fidelity: machine-transcribed` plus the literal body callout, with `verbatim: false` in JSON.

---

## 2. User Stories

> As a person mid-thought, I want `mg-vault capture "check the fsync ordering"` to land in my inbox instantly with no prompt, index, or network, so that capturing costs less than the idea is worth.

> As a researcher, I want `mg-vault clip URL` to save the article as Markdown with its source URL, retrieval time, and content hash written as ordinary visible properties, so that I can cite it later and prove what I actually read.

> As a security-minded user, I want a clipped page to be stripped of script, handlers, and dangerous URL schemes *before* it is written to disk, and I want the network touched only when I type `clip`, so that reading my own vault can never execute anything and my vault never phones home.

> As someone reviewing a backlog, I want an inbox triage flow where each entry can be refiled, promoted to a new note, or dropped, with a preview before anything moves, so that clearing the inbox never loses an entry I meant to keep.

> As a user on a machine without `tesseract`, I want OCR to say "not installed, run `pacman -S tesseract tesseract-data-eng`" and write nothing, rather than silently writing an empty note, so that a missing capability is visible rather than a mystery.

> As a screen-reader and no-color terminal user, I want the pre-write confirmation, the sanitizer report, the redirect chain, and extraction progress to be plain ordered lines with literal state words, so that nothing important is conveyed by color, spinner position, or a redrawing progress bar.

> As an automation author, I want `clip --print-only --json` to emit the derived note and its provenance to stdout without touching the vault, and `--no-input` to never hang on a confirmation, so that I can review or pipeline a clip before it is committed.

> As someone who clipped a page from a logged-in session, I want the cookies and auth headers used for that request to exist nowhere on disk afterwards, so that a synced vault does not become a credential leak.

---

## 3. UX Specification

### 3.1 Screen / view inventory

L is a CLI-and-TUI feature. It introduces no graphical screens, windows, modals, or pointer gestures. It adds these line-oriented views and one TUI pane:

| View | Entry point | New / modified | Layout pattern |
|---|---|---|---|
| Capture result line | `mg-vault capture …` | New | One stdout line (or silent under `--quiet`) |
| Clip pre-write confirmation | `mg-vault clip URL` | New | Full-width ordered record block on stderr; prompt on `/dev/tty` |
| Clip fetch progress | `mg-vault clip URL` | New | Two static stderr lines (`fetching …`, `received …`) |
| Sanitizer report | inside the confirmation, or `clip --report` | New | Ordered counted list, one dropped-construct kind per line |
| Extraction progress | `mg-vault extract FILE` | New | Static appended stderr lines, one per page or per 10% |
| Extraction pre-write confirmation | `mg-vault extract FILE` | New | Same record block as clip, with tool/version and fidelity |
| Inbox listing | `mg-vault inbox list` | New | Ordinal-numbered record blocks; `--jsonl` for streams |
| Inbox review loop | `mg-vault inbox review` | New | One entry at a time, then a labeled single-key action menu |
| Capability report | `mg-vault capabilities` | Modified (**C**-owned command) | Adds `clip`, `pdf_extract`, `ocr` rows |
| Spool / pending report | `mg-vault doctor`, `inbox pending` | Modified (**C**-owned `doctor`) | Adds pending/quarantined capture checks |
| Inbox pane | **E** workspace, `:inbox` | New model, **E**-owned pane | List pane bound to the same `InboxModel` |

Human output goes to stdout on success and stderr on warning, error, progress, and confirmation. Under `--json` a success is exactly one JSON object plus newline on stdout with stdout otherwise empty; an error is exactly one JSON object on stderr. `clip --print-only` writes the derived note bytes to stdout with nothing else, so it composes in a pipe. Prompts are written to and read from `/dev/tty`, never a redirected stream.

### 3.2 Interaction flows

#### Fast capture (the path that must never be slow)

1. `mg-vault capture [TEXT…]` resolves the vault and the capture target: `--to inbox` (default), `--to daily`, or `--to PATH` restricted to `capture.allowed_targets`. Body comes from operands, `--stdin`/`-`, or `--body-file`; exactly one source, conflicts are a parse error.
2. The entry is rendered from the fixed template (§4.2) — literal substitution of a closed placeholder set only. **No shell, no eval, no template language, no file inclusion.**
3. The entry is written to the owner-only capture spool and `fsync`ed **before** any vault write is attempted. From this instant the capture cannot be lost.
4. The target note is read, its fingerprint captured, the append point located (end of file, or under the matching date heading when `capture.group_by_day` is set), and the new bytes committed by **A**'s atomic same-directory replace under a fingerprint precondition. A missing target is created with `create_new` semantics.
5. A `conflict` (another writer appended first) retries at most twice with a fresh read. After that the command exits `4`, the spool entry remains, and the message names `mg-vault inbox drain` as the recovery. Nothing is lost and nothing is guessed.
6. On commit, the resulting fingerprint is recorded in the spool entry, then the entry is unlinked and the spool directory `fsync`ed.
7. Output is one line: `captured → inbox.md (+62 bytes)`. `--quiet` prints nothing. Exit `0`.

No prompt, no confirmation, no index query, no network, no Markdown parse beyond locating the append point. Capture does not require confirmation because it can only append to a pre-declared target and can never overwrite, create outside the allowlist, or import foreign bytes — that is the rationale for treating it differently from `clip` in §3.2's confirmation rules.

#### Handoff — inbound spool and outbound refile

1. **Inbound.** External front-ends (Quickshell's capture pill, `mg-calr`, a shell function, an editor keybind) may write a `mg.capture/1` JSON entry into the spool directly instead of invoking the binary, which keeps their latency at a file write. The spool is **not a trusted channel**: on drain every entry is revalidated — schema version, ≤ 256 KiB, valid UTF-8, target path through `validate_note_path`, closed placeholder set. A spool entry can request an append; **it can never request a network fetch, a process spawn, a path outside the vault, or a mutation of an existing note other than the declared append.** This is the exfiltration boundary for the handoff channel and it is tested by name (§5.2).
2. `mg-vault inbox drain` applies all pending entries in spool-ID order, idempotently. Replay after a crash compares the target's current bytes against the recorded post-append fingerprint and the exact entry bytes at the tail; a match means "already applied" and the entry is retired. Anything ambiguous is **quarantined** and reported by `doctor` — never double-appended, never discarded.
3. **Outbound.** `mg-vault inbox refile N --to PATH` moves one entry out of the inbox into an existing note; `inbox promote N --to PATH` creates a new note from it; `inbox open N` hands the containing note to **C**'s exact-path `note open` contract. All three are two-note transactions and follow the ordering in §3.2's refile flow.
4. Nothing in the handoff path resolves a URL, renders HTML, or reads a PDF. Drain is offline by construction.

#### Web clip

1. `mg-vault clip URL` parses and validates the URL: scheme must be `https` (or `http` with explicit `--allow-insecure-http`); userinfo is stripped from everything that will be recorded; the host is IDNA-normalized.
2. Optional pre-fetch dedupe: with `--dedupe-by-url` and a `current` **B** index, a `source_url` property query may find an existing clip and end the command with `changed: false` **before any request is made**. Without a current index, L says `dedupe: path-only (index unavailable)` and continues.
3. **The single network moment.** One request is issued under the policy in §4.3 — explicit user invocation only, capped timeouts, capped redirects, capped size, no cookies, no vault content outbound. A static line `fetching example.com…` goes to stderr; on completion, `received 48.2 KiB in 310 ms via 2 redirects`.
4. The response is decoded (charset from the HTTP header, then `<meta charset>`, then UTF-8 with replacement and a recorded warning), parsed with a spec-compliant HTML5 tokenizer, and reduced by the selected `--mode` (`article` readability heuristic, `full` body, or `raw` = no conversion).
5. **Sanitization runs here, on the tree, before any conversion output exists** (§4.3). Only sanitizer-approved nodes reach the Markdown writer.
6. The title is taken from `<title>`/`og:title`/`<h1>` and run through the filename sanitizer (§4.3) to derive a candidate path. `--to PATH` skips the slugger but not `validate_note_path`.
7. Idempotence check on the candidate path (§4.4): identical URL and identical content hash ⇒ no write, `changed: false`, exit `0`. Same URL with changed bytes ⇒ `--on-existing {ask|version|revision|skip|fail}`, default `ask` interactively and `fail` under `--no-input`. Different URL at the same path ⇒ `collision`. **There is no overwrite flag.**
8. The **confirmation surface** is printed: target path and whether it is a create or a mutation, byte length, title, display-redacted source URL, final URL, redirect chain, content hash, extraction mode and fidelity, sanitizer counts, image policy, attachment retention decision, and the exact frontmatter block that will be written. Then `Write this note? [Y/n]` for a create at a free path, `[y/N]` for anything touching an existing note.
9. `--no-input` rules: a create at a free path proceeds (nothing is overwritten and a collision fails closed); **any** mutation of an existing note additionally requires `--yes` and either `--expected FINGERPRINT` or explicit `--read-current`.
10. Commit: the note is written through **A**'s atomic create/replace; a retained raw source, if any, is written first as an attachment so the note never references a missing file. Success is reported only after both `fsync`s return.
11. `--print-only` performs steps 1–8 and writes the derived note to stdout without touching the vault. `--dry-run` performs steps 1–8 and prints the plan without writing.

**Branch — anything refuses.** DNS failure, TLS failure, non-2xx status, timeout, size cap, redirect cap, scheme downgrade, private address, or a body that is not text ⇒ typed error, nothing written, no partial file, no empty note. **Branch — hostile page.** Script, handlers, `javascript:` links, nested-parse-differential markup, a title of `../../../etc/passwd`, a 40 MiB response: each is a named refusal or a counted drop in the sanitizer report; none produces active output and none escapes the vault root.

#### PDF and OCR extraction

1. `mg-vault extract FILE.pdf` probes the required external tool **at command time only** (never at process start) and records its exact version string. Absent ⇒ `tool_unavailable`, exit `6`, the Arch package name, the probed `PATH`, and nothing written.
2. Text extraction streams the PDF bytes to `pdftotext - -` on stdin — the child is given bytes, not a path, so it never learns a vault location (§4.6).
3. Progress lines `page 7/40` go to stderr in interactive human mode; under reduced motion, one line per 10%; under `--quiet`, `--json`, or a non-TTY, none.
4. If the mean extracted characters per page falls below `capture.text_layer_threshold` (default 16), L reports `pdf_text_layer_absent` and **stops**. It does not silently OCR. The message names `--ocr` as the explicit opt-in.
5. `--ocr` rasterizes via `pdftoppm` and transcribes via `tesseract`, both stdin/stdout. Output is marked machine-transcribed in three independent places: `extraction_fidelity: machine-transcribed` and `verbatim: false` in frontmatter, a literal body callout above the text, and `verbatim: false` in the JSON envelope.
6. Encrypted PDF ⇒ `pdf_encrypted`; malformed ⇒ `pdf_unreadable`; child timeout or output cap ⇒ `extraction_timeout` / `extraction_too_large`. Every one writes nothing.
7. The source document is **never** moved, modified, or deleted. Default records an absolute reference plus its hash; `--attach` copies it into the vault with collision-refusing creation.
8. Same confirmation surface and same `--no-input` rules as clip.

#### Inbox review

1. `mg-vault inbox list` prints ordinal-numbered entries with byte offsets, first line, timestamp, and tags, in source byte order. `--jsonl` streams the same records.
2. `mg-vault inbox review` shows one entry, then a labeled menu: `[k]eep  [r]efile  [n]ew note  [t]ag  [d]rop  [s]kip  [q]uit`. Every letter has a flag equivalent (`inbox refile N --to`, `inbox promote N --to`, `inbox drop N`, `inbox tag N --tag`), each with `--dry-run`.
3. **Refile ordering is fixed and is the multi-file safety rule.** (a) journal the plan; (b) write the destination — create-new, or fingerprint-bound append; (c) `fsync`; (d) journal `destination_committed`; (e) remove the entry span from the inbox by fingerprint-bound span edit; (f) `fsync`; (g) retire the journal. A failure between (d) and (g) leaves the content in **both** files and `doctor` reports `refile_incomplete: content is in both PATH and inbox.md; rerun to finish`. **The inbox entry is never removed before the destination is durable.**
4. `drop` moves the entry text into **A**'s vault-local trash as a recoverable fragment, never a bare deletion.
5. Any inbox change since the listing invalidates the ordinals; the command re-reads and, on a fingerprint mismatch, exits `4` and asks for a rerun rather than acting on a stale ordinal.

### 3.3 Layout descriptions

Every L view uses one hierarchy, top to bottom: **outcome or action → vault name and affected path(s) → operation facts → warnings → one actionable recovery command.**

**Confirmation record** (the pre-write surface), one `key: value` per line in this fixed order: `action`, `vault`, `path`, `exists`, `bytes`, `title`, `source` (display-redacted), `final_url`, `redirects`, `content_hash`, `content_type`, `mode`, `fidelity`, `sanitized`, `images`, `attachment`, then a blank line, then the literal frontmatter block indented two spaces, then the prompt. Data sources: the parsed `ClipDocument`, the `FetchReceipt`, the `SanitizerReport`, and the resolved `Vault` — no index and no network is consulted to build it.

**Sanitizer report**: one line per dropped construct kind, `dropped 12 × <script>`, `dropped 4 × on* handler`, `denied 3 × javascript: URL`, `denied 1 × data: URL`, ordered by count descending then kind name. Zero drops prints the literal `sanitized: nothing removed`.

**Inbox entry block**: `[3] 2026-08-29T09:14:02-05:00  inbox.md:1420..1487`, then the entry's first line truncated to width with an explicit ` …(+4 lines)` suffix, then `tags: …` when present. Below 60 columns every block switches to one field per line; IDs, paths, hashes, and recovery commands are **never** truncated, only wrapped with indentation.

**Empty states**, literal: `Inbox is empty.`, `No pending captures.`, `sanitized: nothing removed`, `alt: description: unavailable` (F's rule — a filename is never promoted to a description).

**Extraction progress**: `page 7/40`, appended, never rewritten in place.

### 3.4 Input & gestures

- Keyboard and pipes only. Mouse, touch, stylus, voice, camera, and controller input are N/A for this branch.
- Grammar: `capture [TEXT…]`, `clip URL`, `extract PATH`, `inbox {list|review|refile|promote|tag|drop|drain|pending}`, all under the existing `mg-vault` global flags.
- Global flags honored: `--vault NAME`, `--json`, `--jsonl` (paginated reads only), `--no-input`, `--no-color`, `--quiet`, `--verbose`, `--yes`, `--dry-run`.
- L-specific flags: `--to`, `--stdin`/`-`, `--body-file`, `--tag`, `--title`, `--mode {article|full|raw}`, `--on-existing`, `--print-only`, `--report`, `--images {none|reference|download}`, `--keep-source {auto|always|never}`, `--timeout-ms`, `--max-bytes`, `--max-redirects`, `--allow-insecure-http`, `--allow-private-address`, `--no-proxy`, `--header-file`, `--from-file`, `--source-url`, `--keep-query`, `--ocr`, `--attach`, `--no-progress`, `--dedupe-by-url`, `--expected`, `--read-current`.
- **stdin piping** is explicit, never guessed: `pandoc … | mg-vault capture --stdin`, `curl … | mg-vault clip --from-file - --source-url URL`. Unrelated stdin is never consumed as content.
- `--no-input` guarantees no prompt, no `/dev/tty` read, no editor launch, and no confirmation wait; a missing required decision is a typed error, not a default.
- `NO_COLOR` in the environment equals `--no-color` and wins over auto-detection. `--quiet`, `--json`, and a non-TTY stderr each independently suppress progress.
- `SIGINT` during a fetch aborts the connection and writes nothing. `SIGINT` during a commit lets the single atomic rename finish or leaves the journal intact for deterministic recovery; it never reports success because the signal was late. Exit `130`.
- Responsive behavior: the record layout switches at 60 columns; tested at 40, 60, 80, and 120.

### 3.5 Transitions & animation

There are no view transitions, no navigation animations, and no sound or haptics in a CLI. The only time-varying output in this branch is **progress reporting**, and it is constrained: static appended lines, never a spinner, never a carriage-return redraw, never an ANSI cursor movement when stderr is not a TTY. `--no-progress`, `--quiet`, `--json`, a non-TTY stderr, and `MG_VAULT_REDUCED_MOTION=1` each reduce output to a single start line and a single completion line. Reduced motion is therefore the default in every non-interactive context. TUI pane transitions for the inbox pane are **E**'s, governed by **E**'s reduced-motion contract; L supplies only the model.

### 3.6 Error states

| Code | Exit | Trigger | Presentation and recovery | Data-loss risk |
|---|---:|---|---|---|
| `invalid_url` | 2 | Unparseable URL, missing host | Echo the rejected operand with userinfo redacted | No |
| `scheme_denied` | 5 | Non-`http(s)` scheme, or `http` without opt-in | Name the rule and `--allow-insecure-http` | No |
| `private_address_denied` | 5 | Resolved IP is loopback/link-local/private/reserved/multicast | Name the resolved class, not the address; `--allow-private-address` | No |
| `redirect_denied` | 5 | >`--max-redirects` hops, scheme downgrade, or non-`http(s)` `Location` | Print the full chain and the failing hop | No |
| `response_too_large` | 5 | Body exceeded `--max-bytes` mid-stream | Abort the connection; state the cap | No; nothing written |
| `fetch_timeout` | 6 | Connect/TLS/TTFB/total deadline exceeded | Name which deadline and its value | No |
| `http_error` | 6 | Non-2xx status | Status code and reason phrase only, no body echo | No |
| `tls_error` | 6 | Certificate or handshake failure | State the failure; **no bypass flag exists** | No |
| `unsupported_content_type` | 2 | Response is not a supported text/HTML/PDF type | Suggest `--keep-source always` + manual handling | No |
| `tool_unavailable` | 6 | `pdftotext`/`pdftoppm`/`tesseract` absent or unrunnable | Package name, probed `PATH`, install command | No; nothing written |
| `pdf_text_layer_absent` | 6 | Below the per-page character threshold | Explicit `--ocr` opt-in named; **never silent OCR** | No |
| `pdf_encrypted` / `pdf_unreadable` | 2 | Encrypted or malformed document | State which; suggest decrypting externally first | No |
| `extraction_timeout` / `extraction_too_large` | 6 | Child wall-clock or output cap exceeded | Kill the child process group; state the cap | No |
| `unsafe_path` | 5 | Derived or supplied path fails `validate_note_path` | Print the rejected path escaped, and the rule | No |
| `title_unusable` | 0 | Title sanitizes to empty | **Not an error** — falls back to `clip-<date>-<hash8>.md` with a warning | No |
| `collision` | 4 | Target exists with a different `source_url` | Show both URLs; offer `--to` or `--on-existing` | No; never overwrites |
| `conflict` | 4 | Fingerprint changed between read and commit | Show expected/actual; spool entry retained | No; capture recoverable |
| `confirmation_required` | 2 | Existing-note mutation under `--no-input` without `--yes` | List the exact required flags | No |
| `spool_entry_invalid` | 7 | Malformed/oversized/unsafe spool entry | Quarantine it; `doctor` reports it; never applied | No; quarantined |
| `refile_incomplete` | 7 | Crash between destination commit and inbox removal | Content is in **both**; rerun finishes it | No; duplicated, never lost |
| `capture_ambiguous_replay` | 7 | Replay cannot prove whether an append committed | Quarantine; `doctor` shows both bytes | No; quarantined |
| `io` / `permission_denied` / `out_of_space` | 7 | Filesystem failure | State uncommitted/unknown; name `doctor` | No false success |

No error message contains note content, clipped body text, OCR text, header values, cookie values, URL userinfo, or environment values. Error JSON carries `version`, `ok: false`, `error.code`, `error.message`, `error.details`, `retryable`, and `recovery`.

### 3.7 Accessibility

- **Screen reader / linear reading:** every view is ordered `key: value` lines or numbered blocks with deterministic reading order. Nothing is conveyed by column position, right alignment, box drawing, or a progress bar's fill.
- **Labels and traits:** each prompt states its question, its accepted values, its default in brackets, and its cancel key. The review menu prints every action word in full alongside its key; no action is key-only or discoverable only by trying it.
- **Custom actions:** the review loop's complex action (refile) always asks for the destination as an explicit typed path with tab completion, and always shows the resulting plan before acting. There is no gesture-equivalent to substitute.
- **Color independence:** state is always a literal word — `exists: yes`, `fidelity: machine-transcribed`, `sanitized: 12 removed`, `freshness: unavailable`. Color decorates and never encodes. Verified by stripping the attribute plane in §5.4.
- **Text scaling:** N/A in a terminal; the equivalent is cell width, and every view is verified at 40, 60, 80, and 120 columns with no truncation of identity-bearing values.
- **Focus order and keyboard navigability:** the shell's natural prompt order; every prompt has a flag equivalent so `--no-input` is a complete alternative path to the entire feature.
- **Unicode safety:** titles, tags, and clipped text may contain bidi controls, combining marks, and zero-width characters. In *metadata display* these are escaped as `\u{202E}` so a right-to-left override cannot spoof a filename in the confirmation surface; in *note content* bytes pass through unmodified. Widths are computed by grapheme cluster for wrapping only, never for identity.
- **Textual equivalents (5C):** images in a clipped page become Markdown image references carrying the page's `alt` text, or the literal `description: unavailable` when there is none — a filename or URL is **never** promoted into a description. Redirect chains, sanitizer reports, and per-page OCR confidence are ordered text and appear identically in `--json`. No L output is available only as a rendered or graphical form.

---

## 4. Implementation Specification

### 4.1 Architecture placement

New crate `crates/mg-vault-capture/`:

```
src/lib.rs
src/entry.rs          # CaptureEntry, closed placeholder set, rendering
src/spool.rs          # owner-only spool, IDs, drain, replay, quarantine
src/inbox.rs          # InboxModel, entry spans, refile/promote/drop plans
src/slug.rs           # filename sanitizer (§4.3) — no filesystem access
src/provenance.rs     # Provenance struct ⇄ visible YAML block
src/net/policy.rs     # NetworkGrant, URL/address/redirect/size policy   [feature = "clip"]
src/net/fetch.rs      # HttpTransport trait + ureq backend               [feature = "clip"]
src/html/parse.rs     # html5ever tree build                             [feature = "clip"]
src/html/readability.rs
src/html/to_markdown.rs
src/extract/tool.rs   # sandboxed child-process runner + probing         [feature = "extract"]
src/extract/pdf.rs                                                       [feature = "extract"]
src/extract/ocr.rs                                                       [feature = "ocr"]
```

CLI subcommands go in `crates/mg-vault-cli/src/commands/{capture,clip,inbox,extract}.rs` and own no policy, exactly as `crates/mg-vault-cli/src/main.rs` owns none today.

**Layering invariants, enforced by tests, not convention:**

- `mg-vault-core` gains **no** network code and **no** process-spawn code, ever. It gains only two narrow primitives (§4.3). A `cargo tree` assertion in CI fails the build if `mg-vault-core` acquires a transitive dependency on any HTTP or TLS crate.
- The socket-opening code exists only under `#[cfg(feature = "clip")]` inside `net/`. A build with `--no-default-features` links no TLS stack at all, and a symbol-level test asserts it.
- Sanitization policy is **not** duplicated here. `mg-vault-capture` consumes `mg_vault_markdown::sanitize::Policy` and `HTML_ALLOWLIST_VERSION` from **F**; there is exactly one allowlist in the workspace.
- `mg-vault-index` (**B**) is an optional, read-only, dedupe-only dependency. L never writes to the index and never accepts bytes or authorization from it.

**Reconciliation with `specs/f-markdown-rich-content.md`.** F's rule — *source HTML is preserved byte-identically and never rewritten in the file; sanitization is output-side* — governs bytes that are **already** the user's vault source. A fetched page is not vault source; it is foreign input on its way in. L therefore applies F's identical allowlist at the **ingest** boundary, so that what becomes source is already inert. The two rules compose into defense in depth: L guarantees no active construct is ever written, and F guarantees that even if one somehow were, `--html-mode literal` renders it as escaped text. L adds a constraint and contradicts nothing: once L writes the note, F preserves those bytes byte-for-byte forever after, and L never re-sanitizes an existing note.

### 4.2 Data model

```rust
/// Version of the capture/provenance contract. Bumping it is a breaking change.
pub const CAPTURE_SCHEMA: &str = "mg-vault.capture/1";

/// One pending capture. Lives in the spool, never in a note.
/// The id is spool-local: it is deliberately NOT written into the vault,
/// because file paths remain the only public note identity (criterion 1C).
pub struct CaptureEntry {
    pub schema: String,              // CAPTURE_SCHEMA
    pub id: SpoolId,                 // same shape/validation as A's trash id
    pub vault_name: String,
    pub target: CaptureTarget,       // Inbox | Daily | Explicit(NotePath)
    pub body: String,                // <= 256 KiB, valid UTF-8
    pub tags: Vec<String>,
    pub created_at: Timestamp,       // RFC 3339 with explicit offset
    pub committed: Option<SourceFingerprint>, // post-append proof, for replay
}

/// Everything L records about where a note came from. All fields are written
/// as ordinary, visible YAML properties — never hidden, never a UUID.
pub struct Provenance {
    pub capture_schema: String,      // CAPTURE_SCHEMA
    pub source_url: RedactedUrl,     // userinfo stripped; secret-ish params masked
    pub source_final_url: Option<RedactedUrl>,
    pub redirect_chain: Vec<RedactedUrl>,
    pub retrieved_at: Timestamp,     // RFC 3339 WITH timezone offset, never naive
    pub retrieved_by: String,        // "mg-vault clip 0.1.0"
    pub content_type: Option<String>,
    pub content_hash: String,        // sha256 of the EXACT fetched bytes
    pub content_length: u64,
    pub extraction: String,          // "readability/1" | "html-full/1" | "pdftotext/24.08" | "ocr/tesseract-5.3.0"
    pub extraction_fidelity: Fidelity,
    pub sanitizer: String,           // HTML_ALLOWLIST_VERSION from F
    pub sanitizer_removed: u32,
    pub url_redacted_params: Vec<String>,
    pub cookies_ignored: u32,
    pub source_attachment: Option<NotePath>,
}

/// The one field a reader must never misread. `Verbatim` is claimed only for
/// bytes copied unchanged from the source document.
pub enum Fidelity { Verbatim, Lossy, MachineTranscribed }

/// Private satisfaction type. Constructible ONLY inside the `clip` command
/// handler from a parsed interactive invocation. No plugin, AI adapter, TUI
/// background task, spool entry, config value, or index response can mint one,
/// and `net::fetch` accepts nothing else. This is the capability boundary.
pub struct NetworkGrant { /* private fields */ }

pub struct FetchReceipt {
    pub status: u16,
    pub final_url: RedactedUrl,
    pub redirects: Vec<RedactedUrl>,
    pub bytes: u64,
    pub elapsed_ms: u64,
    pub content_type: Option<String>,
    pub cookies_ignored: u32,        // count only; values are dropped, never stored
}

pub struct SanitizerReport {
    pub removed: Vec<(DroppedKind, u32)>, // ordered: count desc, then kind name
    pub denied_urls: Vec<(String, u32)>,  // scheme -> count; values never echoed
}
```

The frontmatter L writes, verbatim in shape, on top of the note body:

```yaml
---
title: "Atomic rename semantics on ext4"
source_url: https://example.org/posts/atomic-rename
source_final_url: https://example.org/posts/atomic-rename
retrieved_at: 2026-08-29T14:03:11-05:00
retrieved_by: mg-vault clip 0.1.0
content_type: text/html; charset=utf-8
content_hash: sha256:9f2c…
content_length: 48213
capture_schema: mg-vault.capture/1
extraction: readability/1
extraction_fidelity: lossy
sanitizer: mg-vault.html-allowlist/1
sanitizer_removed: 14
cookies_ignored: 0
---
```

**No migration is required**: L creates new files and appends to existing ones. It defines no database table and alters no existing schema. The spool is new state under `$XDG_STATE_HOME`, versioned by `schema`, and an unknown schema is quarantined rather than parsed.

### 4.3 API contracts

```rust
// mg-vault-capture
pub fn plan_capture(vault: &Vault, req: CaptureRequest) -> Result<CapturePlan>;
pub fn commit_capture(vault: &Vault, plan: &CapturePlan) -> Result<CaptureReceipt>;
pub fn drain(vault: &Vault, spool: &Spool) -> Result<DrainReport>;
pub fn inbox_model(vault: &Vault) -> Result<InboxModel>;
pub fn plan_refile(vault: &Vault, req: RefileRequest) -> Result<RefilePlan>;
pub fn commit_refile(vault: &Vault, plan: &RefilePlan) -> Result<RefileReceipt>;
pub fn safe_filename(title: &str, profile: SlugProfile) -> SlugOutcome;
pub fn plan_clip(doc: ClipDocument, req: ClipRequest) -> Result<ClipPlan>;   // no I/O, no network
pub fn commit_clip(vault: &Vault, plan: &ClipPlan, c: Confirmed) -> Result<MutationReceipt>;

#[cfg(feature = "clip")]
pub fn fetch(grant: &NetworkGrant, url: &Url, p: &FetchPolicy) -> Result<(Vec<u8>, FetchReceipt)>;

#[cfg(feature = "extract")]
pub fn probe(tool: Tool) -> ToolStatus;   // Available{version} | Missing{package, path_probed}
#[cfg(feature = "extract")]
pub fn extract_pdf(bytes: &[u8], p: &ExtractPolicy) -> Result<(String, ExtractReceipt)>;
```

**Two new `mg-vault-core` primitives** (and only two): `Vault::append_note(path, bytes, expected) -> Result<SourceFingerprint>` — a fingerprint-bound append that reuses `replace_atomic`; and `Vault::create_attachment(path, bytes, mode) -> Result<SourceFingerprint>` — collision-refusing creation for a non-`.md` extension from a closed allowlist (`.html`, `.pdf`, `.png`, `.jpg`, `.txt`), since today's `validate_note_path` requires `.md` and therefore cannot write an attachment at all. Both go through `validate_note_path`'s component rules and `prepare_destination`'s per-component symlink walk unchanged.

**Network policy — exact, and the only network in the product.**

- **Trigger:** exactly one explicit user-typed `clip` (or `clip --images download`) invocation. **No background thread, no timer, no watcher, no daemon, no retry loop, no prefetch, no link preview, no update check, no telemetry, no crash reporting, no analytics, and no side effect of any other command opens a socket.** `capture`, `inbox` (including `drain`), `extract`, `search`, `note *`, `index *`, and the TUI are offline by construction; the socket-denial test in §5.2 covers all of them.
- **Capability:** `fetch` accepts a `NetworkGrant` whose fields are private to the `clip` command handler's module. Every other caller is a compile error. A config file cannot enable clipping; a plugin cannot request it; **N**'s WASM sandbox is denied host sockets outright; **O**'s AI adapters are outside this crate and cannot construct a grant.
- **Egress content:** the request line, `Host`, a fixed `User-Agent: mg-vault/<version>`, `Accept: text/html,application/xhtml+xml,application/pdf;q=0.9,*/*;q=0.5`, `Accept-Encoding: gzip`, and `Connection: close`. **Nothing else.** No `Referer` ever. No `Cookie` header. No vault name, path, note content, fingerprint, hostname, username, locale, or machine identifier. `--header-file` may add caller-supplied headers, read into a zeroizing buffer, never logged, never written to the vault, and zeroized after the request; `--verbose` prints header *names* only. §5.2's marker-string test asserts the outbound bytes contain no vault-derived value.
- **Timeouts:** connect 5 s, TLS handshake 5 s, time-to-first-byte 10 s, total wall clock 30 s. `--timeout-ms` scales the total, capped at 120 000 ms. Each is enforced independently; a slow-loris trickle trips TTFB or wall clock.
- **Redirects:** followed by L, not the HTTP library (the library's own follower is disabled). Maximum **5** hops, `--max-redirects` may only lower it. Each hop must be `http`/`https`; **the scheme may not change between hops**, which makes `https → http` downgrade impossible even when the origin asks. Each hop's resolved address is re-checked against the address policy before connecting, closing DNS rebinding. Every hop is recorded in `redirect_chain` and shown in the confirmation. A `Location` with any other scheme is `redirect_denied`.
- **Address policy:** resolved addresses in loopback, link-local, unique-local, RFC 1918, CGNAT, multicast, broadcast, or reserved ranges are denied unless `--allow-private-address` is passed explicitly (for a personal wiki on the LAN). The connection is made to the address that was checked.
- **Size:** hard cap **10 MiB** decoded (`--max-bytes` may only lower it), enforced while streaming, including decompressed size so a zip bomb aborts at the cap rather than after. Exceeded ⇒ connection aborted, `response_too_large`, nothing written.
- **TLS:** platform trust store, full certificate and hostname verification. **There is no flag to disable verification, and none will be added.**
- **Proxy:** `HTTPS_PROXY`/`NO_PROXY` are honored, and the effective proxy is printed in the confirmation surface before the request so egress destination is never a surprise. `--no-proxy` ignores them.
- **Cookies and credentials:** there is no cookie jar and no credential store. `Set-Cookie` responses are dropped (count recorded, values never stored, never carried across redirects, never written anywhere). URL userinfo is stripped from every recorded, displayed, and logged form; the note records `credentials_stripped: true`. Query parameters whose key matches the secret denylist (`token`, `access_token`, `id_token`, `refresh_token`, `api_key`, `apikey`, `key`, `sig`, `signature`, `password`, `passwd`, `secret`, `auth`, `session`, `sessionid`, `code`) are replaced with `REDACTED` in the recorded URL and listed in `url_redacted_params`; `--keep-query` opts out with a printed warning.
- **Inbound is not trusted either:** the response body is bytes, not code. No JavaScript is executed, no headless browser is used, no external subresource is fetched unless `--images download` was explicitly passed (then each image URL passes the identical policy, capped at 25 images and sharing the 10 MiB budget).

**HTML sanitization — deny by default, on ingest, before any byte reaches disk.**

- Parsing uses a spec-compliant HTML5 tokenizer (`html5ever`), so nested-parse-differential markup cannot smuggle a construct past a regex.
- Sanitization walks the parsed **tree** and is applied **before** the Markdown writer runs; the writer's input type can only be produced by the sanitizer, mirroring F's `SafeInline`/`SafeBlock` compile-time enforcement. Unsanitized text cannot reach the writer without failing to compile.
- The allowlist is F's `mg-vault.html-allowlist/1`, unchanged: elements `br, em, strong, i, b, code, kbd, mark, sub, sup, u, del, ins, span, p, ul, ol, li, blockquote, hr, h1..h6, a, img, table, thead, tbody, tr, th, td`; attributes only `a[href, title]`, `img[src, alt, title, width, height]`, `th/td[colspan, rowspan]`.
- Dropped unconditionally: `script`, `style`, `iframe`, `object`, `embed`, `applet`, `frame`, `frameset`, `noscript`, `template`, `svg`, `math`, `form`, `input`, `button`, `link`, `meta`, `base`; **every** `on*` event handler attribute; `style`, `class`, `id`, `srcset`, `formaction`, `xlink:href`, and every attribute not in the allowlist; comments, CDATA, processing instructions, doctype, and namespace-prefixed names.
- URL schemes are allowlisted to `http`, `https`, `mailto`, and relative references resolved against the final URL. `javascript:`, `data:`, `vbscript:`, `file:`, `blob:`, and unknown schemes are **denied and dropped**, not linkified, not preserved as text with an active target. Percent-, Unicode-, tab-, newline-, and whitespace-obfuscated variants are normalized before the check; an unparseable scheme is denied.
- Anchors that survive get `rel="nofollow noopener noreferrer ugc"` when HTML is emitted at all; in the normal Markdown output there is no anchor element to attribute.
- The output of `--mode article|full` is **Markdown**, so the common case contains no HTML element whatsoever. `--mode raw` writes **no** converted body — only the provenance stub plus a retained attachment — so raw HTML never becomes note body under any mode.
- The retained raw source (`--keep-source`) is stored as `attachments/clips/<sha256-prefix>.html` with mode `0600`, its extension forced from the sniffed/declared content type rather than the URL path, no execute bit, and `mg-vault` never renders or opens it — **F**'s media policy resolves it as an inert `file` card.

**Filename sanitization (a page title becomes a filename — this is the traversal surface).** `safe_filename` applies, in order: (1) trim; (2) reject the whole title if it contains a path separator, and never split it into components — a title yields exactly **one** path component; (3) delete NUL, all C0/C1 controls (U+0000–U+001F, U+007F–U+009F), bidi controls (U+202A–U+202E, U+2066–U+2069), and zero-width characters (U+200B–U+200D, U+FEFF); (4) replace `/ \ : * ? " < > |` with `-` for cross-platform sync safety; (5) collapse whitespace runs to a single `-`; (6) strip leading dots (no hidden files), leading/trailing dashes, dots, and spaces; (7) reject the exact results `.`, `..`, and the Windows device names `CON PRN AUX NUL COM1-9 LPT1-9` case-insensitively; (8) truncate to **200 UTF-8 bytes on a grapheme boundary** — 200 not 255, because `atomic.rs::temp_path` prefixes `.` and appends `.mg-vault-{pid}-{serial}.tmp`, roughly 30 bytes, and the temp file must also fit the filesystem limit; (9) append `.md`; (10) if the result is empty, fall back to `clip-<YYYY-MM-DD>-<hash8>.md`, never to a generic colliding name. `--slug-profile ascii` additionally transliterates to `[A-Za-z0-9._-]`.

The slugger is a **convenience, not the authority**. Its output is joined to `capture.clip_folder` and then passed to `Vault::validate_note_path`, which independently rejects absolute paths, any non-`Normal` component, any non-`.md` extension, and any first component of `.obsidian`/`.mg-vault`; `prepare_destination` then walks each parent component rejecting symlinks and non-directories and re-checks canonical containment. **Two independent layers, and the authoritative one is A's, already implemented.** `--to PATH` skips only layer one.

**CLI/JSON contract.** Success: `{"version":1,"ok":true,"command":"clip","data":{"schema":"mg.capture/1","path":"…","changed":true,"created":true,"fingerprint":"sha256:…","provenance":{…},"sanitizer":{"removed":[…],"denied_urls":[…]},"fetch":{…},"verbatim":false},"warnings":[]}`. Arrays always present even when empty, ordering documented and stable. Exit codes reuse **C**'s categories: `0` success/no-op, `2` usage/input, `3` not found, `4` collision/conflict, `5` unsafe or denied, `6` degraded dependency (network, tool absent, index unavailable), `7` I/O/transaction, `130` interrupt. No auth or rate limiting applies to these local commands; the only rate limit is that L issues one request per explicit invocation and never retries a failed fetch automatically.

### 4.4 State management

- **Authoritative state:** the ordinary Markdown notes and attachments L creates or appends to. Nothing else in L is authority.
- **Owned durable state:** the capture spool at `$XDG_STATE_HOME/mg-vault/capture/<vault-id>/{pending,quarantine}/`, directories `0700`, entries `0600`. Deleting the spool loses only un-drained captures and is reported, never silently swallowed.
- **Transaction state:** refile uses **A**/**C**'s journal under `.mg-vault/`. L introduces no second journal format.
- **Derived/optional state:** **B**'s index, consulted read-only for `--dedupe-by-url` and never as authority. Unavailable or stale ⇒ L prints `dedupe: path-only (index unavailable)` and proceeds with the direct path check. An index answer can never authorize a write or supply note bytes.
- **In-memory only:** the fetched response, the parsed tree, header-file contents (zeroized after use), and OCR intermediate images (in an owner-only temp dir under `$XDG_RUNTIME_DIR`, unlinked on exit and on panic via a drop guard).
- **Offline/draft persistence:** the spool *is* the draft store. A capture survives a crash, a full disk on the vault volume, and a stale-fingerprint conflict, because it is durable before the vault write is attempted and retired only after the post-append fingerprint is recorded.
- **Idempotence of a repeat clip:** the target path is computed, then read directly. Identical `source_url` (after normalization: lowercased scheme/host, default port removed, fragment dropped, userinfo stripped) **and** identical `content_hash` ⇒ no write, `changed: false`, exit `0` — the file is byte-identical, so a scripted re-clip is safe. Identical URL with a different hash ⇒ `--on-existing`: `ask` (interactive default), `version` (a sibling `<name>--<retrieved-date>.md`, create-new), `revision` (a fingerprint-bound append of a `## Retrieved <ts>` section), `skip`, or `fail` (default under `--no-input`). A different URL at the same path ⇒ `collision`. **No path overwrites another page's note under any option.**
- **Local vs. server-synced:** N/A — there is no server and no account. The only remote interaction in the entire product is the outbound HTTP GET described in §4.3, and it is read-only: L uploads nothing, registers nothing, and has no endpoint of its own.

### 4.5 Dependencies

New Rust crates, all gated:

| Crate | Feature | Purpose | License |
|---|---|---|---|
| `ureq` 3 + `rustls` | `clip` | Blocking HTTP/1.1 with TLS; its own redirect follower **disabled** | MIT / Apache-2.0 |
| `url` | `clip` | Parsing, IDNA, normalization, userinfo stripping | MIT / Apache-2.0 |
| `html5ever` + `markup5ever_rcdom` | `clip` | Spec-compliant HTML5 tree construction | MIT / Apache-2.0 |
| `flate2` | `clip` | gzip decode with a decompressed-size cap | MIT / Apache-2.0 |
| `jiff` | always | RFC 3339 timestamps **with** timezone offset | MIT / Apache-2.0 |
| `unicode-segmentation`, `unicode-normalization` | always | Grapheme-boundary truncation, slug folding | MIT / Apache-2.0 |
| `zeroize` | `clip` | Wiping header-file buffers | MIT / Apache-2.0 |

Reused, not re-implemented: `mg_vault_markdown::sanitize` (F's allowlist), `mg_vault_core::{Vault, SourceFingerprint, atomic}`, `sha2`, `serde`/`serde_json`, `clap`, `thiserror`. `ammonia` is deliberately **not** used — it would introduce a second allowlist.

External tools (separate processes, dynamically discovered, never linked):

| Tool | Arch package | Feature | License note |
|---|---|---|---|
| `pdftotext`, `pdftoppm` | `poppler` | `extract` | GPL-2.0; invoked as a subprocess, so no linking and no license propagation to `mg-vault` |
| `tesseract` | `tesseract` | `ocr` | Apache-2.0 |
| language data | `tesseract-data-eng` (etc.) | `ocr` | Apache-2.0 |

**No JavaScript runtime, no headless browser, no Electron, no WebView, no `eval`.** A page that requires JavaScript to render is not rendered; when yield is implausibly low L reports `low_yield` and suggests saving the page from a browser and using `--from-file`. New assets: the versioned capture corpus fixtures (§6.2). Infrastructure changes: none — no database, no CDN, no third-party service, no account.

### 4.6 Platform-specific considerations

- **Arch Linux is first support.** Everything except the child-process sandbox and `openat2` confinement is portable pure Rust.
- **Child-process sandbox** (`extract/tool.rs`): spawn with `Command`, **never a shell**; argv only, so a filename can never be word-split or interpreted. The environment is cleared and rebuilt from an allowlist (`PATH`, `LANG`, `LC_ALL`, `TESSDATA_PREFIX`) so no secret in the parent environment reaches the child. cwd is an empty owner-only temp dir. **Input is streamed on stdin (`pdftotext - -`, `tesseract stdin stdout`), so the child is never told a vault path.** stdout/stderr are captured with a 20 MiB cap; the child runs in its own process group and is killed group-wide on timeout, `SIGINT`, or parent panic.
- **Feature gates:** `clip`, `extract`, `ocr`, default-on in the distributed binary. `--no-default-features` produces an offline build that links no TLS stack and cannot spawn a parser — the intended profile for a hardened or air-gapped install. A disabled feature reports `available: false, reason: "not built"` in `capabilities`; it never silently degrades and never produces an empty note.
- **Confinement:** attachment and note writes use **A**'s existing per-component symlink-rejecting walk today, and inherit `openat2`/`RESOLVE_BENEATH` when A's descriptor-relative hardening lands (A §7). On a platform without descriptor-relative confinement, attachment writing is **disabled** and clipping continues with `--keep-source never` forced and stated, rather than silently falling back to a weaker lexical check.
- **Version compatibility:** the external tool version string is recorded in `extraction:` on every note, so a corpus of notes remains auditable across a poppler or tesseract upgrade. Changing the sanitizer allowlist, the slug algorithm, or the provenance shape bumps `mg-vault.capture/1`; it never rewrites existing notes.
- **Rollout:** `clip`, `extract`, and `ocr` ship as independently gated increments (§7.3) behind one schema version.

### 4.7 Performance budget

Measured on a warm local SSD reference machine and reported with hardware/OS metadata.

- **`capture` end to end:** p95 ≤ 15 ms of CPU/syscall work excluding `fsync`; ≤ 60 ms p95 including the spool `fsync`, the note `fsync`, and the parent-directory `fsync`. Process startup ≤ 20 ms. This is the budget that makes capture worth using, and it is a gate, not an aspiration.
- **Capture does not scale with the vault:** it reads exactly one note and writes exactly one note. It performs no directory walk, no index query, and no Markdown parse. A 100,000-note vault and an empty vault have the same capture cost. Appending to a 10 MiB inbox is bounded by rewriting that one file; above `capture.inbox_rotate_bytes` (default 4 MiB) L warns and suggests rotation rather than degrading silently.
- **`clip`:** dominated by the network, which is why the wall-clock cap is 30 s. Local work after the response: parse + sanitize + convert p95 ≤ 250 ms for a 1 MiB page; ≤ 40 ms for a typical 120 KiB article.
- **`extract`:** the child dominates. L's own overhead ≤ 50 ms. Wall clock capped at 60 s (text) and 300 s (OCR), both `--timeout-ms` adjustable, both hard-killed at the cap. Cancellation is checked at least every 100 ms; a cancelled run emits `complete: false` and writes nothing — a partial extraction is never presented as whole.
- **Memory:** response body ≤ 10 MiB by cap; the HTML tree is roughly 4–6× the body, so the clip path is bounded at ~60 MiB RSS worst case and ~10 MiB typical. `capture` stays under 8 MiB RSS beyond the note itself. OCR page images are processed one at a time and dropped.
- **Network payload:** outbound request ≤ 1 KiB (§4.3 enumerates every byte class); inbound ≤ 10 MiB hard cap. Exactly one request per invocation, plus at most 25 image requests under an explicit `--images download`.
- **Storage:** one note per clip (typically 4–60 KiB) plus, when retained, one raw source attachment ≤ 2 MiB under `--keep-source auto`. The spool is transient and bounded at 256 KiB per entry with a warning past 1,000 pending entries.
- **Startup time impact on the rest of the product: zero.** No L code runs unless an L command is invoked. There is no daemon, no watcher, no timer, no background drain, and no work added to the note save path.

---

## 5. Test Specification

### 5.1 Unit tests

| Name | Setup → assertion | Edge case |
|---|---|---|
| `slug_rejects_traversal` | Titles `../../etc/passwd`, `..`, `.`, `a/b`, `C:\x` → single safe component or typed error | Path escape via title |
| `slug_strips_control_and_bidi` | Titles with NUL, U+0007, U+202E, U+200B, CRLF → none survive in the filename | Filename spoofing |
| `slug_reserved_and_hidden` | `CON`, `NUL`, `.hidden`, `  spaced  .` → safe, non-hidden, non-reserved | Cross-platform sync |
| `slug_truncates_on_grapheme_boundary` | 10,000-char emoji/CJK title → ≤ 200 bytes, no split cluster, temp-name still fits | `atomic.rs` temp prefix budget |
| `slug_empty_falls_back` | Titles `""`, `"…"`, `"///"` → `clip-<date>-<hash8>.md` | Never an empty or generic name |
| `slug_output_always_validates` | **Property test, 100,000 fuzzed titles** → every output passes `validate_note_path` | The traversal auto-fail |
| `sanitizer_drops_active_constructs` | Full hostile-HTML corpus shared with F → no `<script>`, `<style>`, `<iframe>`, `<object>`, `on*`, or comment in the writer's input | Active-HTML auto-fail |
| `sanitizer_denies_url_schemes` | `javascript:`, `data:`, `vbscript:`, `file:`, plus percent/unicode/tab/newline-obfuscated variants → all denied | Scheme smuggling |
| `sanitizer_is_the_only_writer_input` | `trybuild` compile-fail: constructing the writer's input outside the sanitizer module | Structural enforcement |
| `url_redaction` | `https://u:hunter2@h/p?token=abc&q=x` → userinfo gone, `token` masked, `q` kept, `url_redacted_params: [token]` | Credential leak |
| `redirect_policy` | 6 hops; `https→http`; `Location: javascript:…`; `Location: http://127.0.0.1` → each `redirect_denied` with the chain shown | Downgrade/SSRF |
| `address_policy` | Resolutions to `127.0.0.1`, `169.254.1.1`, `10.0.0.5`, `::1`, `224.0.0.1` → denied without the flag | SSRF |
| `size_cap_streaming` | Declared-small, actually-40 MiB body; gzip bomb → aborted at cap, nothing written | Zip bomb |
| `timeout_matrix` | 1 byte/s trickle; connect blackhole; TLS stall → the correct named deadline fires | Slow loris |
| `provenance_roundtrip` | `Provenance` → YAML → parse → equal; `retrieved_at` always carries an offset | Naive timestamp |
| `fidelity_never_overclaims` | OCR path cannot construct `Fidelity::Verbatim`; readability path cannot either | Truthful contract |
| `entry_template_is_inert` | Entry bodies containing `$(id)`, backticks, `{{x}}`, `%s` → written literally, no expansion | No template injection |
| `spool_entry_validation` | Oversized, bad-UTF-8, unknown-schema, absolute-target, `.obsidian`-target entries → quarantined, never applied | Handoff channel trust |

### 5.2 Integration tests

- **`no_network_except_clip`** — an `HttpTransport` test double that panics on use, plus a socket-denying wrapper, runs the entire command matrix (`capture`, `inbox list/review/refile/drop/drain`, `extract`, `note *`, `index *`, `search`, TUI startup). **Any connection attempt fails the test.** Additionally a symbol-level assertion that a `--no-default-features` build contains no TLS symbols, and a `cargo tree` assertion that `mg-vault-core` has no HTTP/TLS dependency.
- **`no_vault_content_egresses`** — a vault seeded with distinctive marker strings in note bodies, paths, vault name, and tags; a loopback server records the full request bytes; assert **no marker, no path, no username, no machine hostname, and no `Referer`** appears. Repeated with `--images download`.
- **`clip_is_idempotent`** — clip a fixture URL twice with unchanged bytes → second run writes nothing, reports `changed: false`, exit `0`, file byte-identical. Then change one upstream byte → default `fail` under `--no-input` with exit `4`; `--on-existing version` creates a dated sibling and leaves the original untouched.
- **`no_active_html_on_disk`** — for every hostile fixture, clip it and grep the **written file bytes** for `<script`, `onerror=`, `javascript:`, `<iframe`, `data:text/html`. Zero hits. This asserts the disk, not the renderer.
- **`cookies_and_headers_never_persist`** — server sets three cookies and the request uses `--header-file` with `Authorization: Bearer sekrit`; after the clip, recursively grep the vault, `.mg-vault`, the spool, the journal, the trash, and all captured stderr for the cookie values and `sekrit`. Zero hits; note records `cookies_ignored: 3`.
- **`capture_crash_matrix`** — inject a crash at 14 phases (before spool write, after spool write, before note read, after read, before rename, after rename before fingerprint record, after record before unlink, …). Assert: no capture lost, no double append, ambiguity quarantined and reported by `doctor`, never guessed.
- **`refile_fault_matrix`** — fail after destination commit and before inbox removal → both copies present, `refile_incomplete` reported, rerun completes idempotently. Assert the inbox entry is **never** removed first, at every injection point.
- **`tool_unavailable_is_explicit`** — with an empty `PATH`, `extract` and `extract --ocr` exit `6` with `tool_unavailable`, name the Arch package, write nothing, and `capabilities --json` reports `available: false` with a reason.
- **`ocr_is_marked`** — every OCR note contains `extraction_fidelity: machine-transcribed`, the literal body callout, and `verbatim: false` in JSON; a text-layer PDF produces `extraction_fidelity: lossy` and never `machine-transcribed`.
- **`pdf_never_silently_ocrs`** — a scanned PDF without `--ocr` exits `6` with `pdf_text_layer_absent` and writes nothing.
- **`source_document_untouched`** — after every `extract` run, the input PDF's mtime, size, and hash are unchanged; with `--attach`, the original is copied, not moved.
- **`obsidian_coexistence`** — clipping into a vault containing `.obsidian` leaves `.obsidian` byte-identical; a `--to .obsidian/x.md` or `--to .mg-vault/x.md` request is rejected with `unsafe_path`.
- **`index_absent_degrades_truthfully`** — with no index, `--dedupe-by-url` prints `dedupe: path-only (index unavailable)` and still clips; with a stale index it never reports a dedupe hit as authoritative.

### 5.3 UI / E2E tests

PTY-driven, since every L view is a terminal view.

- **Confirmation gate:** `clip` against an existing note under `--no-input` without `--yes` exits `2` with `confirmation_required` and writes nothing; with `--yes` but no `--expected`/`--read-current` it still refuses; with both it commits. Answering `n` at the interactive prompt writes nothing and exits `0`.
- **Review loop:** navigate 20 inbox entries, refile one, promote one, tag one, drop one, quit. Assert the resulting inbox, the destination notes, and the trash. Then run the identical sequence entirely through non-interactive flags and assert byte-identical results — this is the keyboard/automation parity gate (2A, 5E).
- **Stale ordinals:** modify `inbox.md` externally between `inbox list` and `inbox refile 3` → exit `4`, nothing moved, message asks for a rerun.
- **Progress:** `extract` on a 40-page PDF to a TTY emits static appended page lines; with `--no-progress`, `--quiet`, `--json`, a piped stderr, or `MG_VAULT_REDUCED_MOTION=1` emits at most a start and end line and never an ANSI cursor movement.
- **Pipelines:** `mg-vault clip URL --print-only | wc -c` succeeds with an empty vault mutation; `echo text | mg-vault capture --stdin`; `mg-vault inbox list --jsonl | jq` — all with stdout carrying only data.
- **Interrupt:** `SIGINT` during a fetch → exit `130`, nothing written, no partial file, no leftover temp file in the vault.

### 5.4 Visual / manual verification

- Light and dark terminal themes; 256-color, 16-color, and monochrome. Strip the attribute plane and confirm every state — `exists`, `fidelity`, `sanitized`, `dedupe`, progress — remains distinguishable from text alone.
- Widths 40, 60, 80, 120 and heights 10, 24, 60, against an empty inbox, a 1-entry inbox, a typical 30-entry inbox, and a 2,000-entry inbox. Confirm no hash, path, URL, or recovery command is ever truncated.
- Kitty, foot, Alacritty, xterm, `tmux`, and `TERM=dumb`; confirm the reduced-output path looks deliberate, not broken.
- Clip pages with CJK, RTL, emoji, and combining-mark titles; confirm the confirmation surface shows escaped bidi controls, the resulting filename is safe, and no border or column is corrupted.
- Open a clipped note in Obsidian and confirm the provenance properties appear as ordinary readable properties and the body renders as plain Markdown with nothing active.
- Terminal text scaling extremes (the terminal's own font sizing) — confirm only cell counts matter.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-targets --all-features`.

---

## 6. Compliance & Safety Gate

### 6.1 Sensitive data classification

- [ ] No sensitive data involvement
- [x] **Handles sensitive data.** Three classes. **(1) Clipped content and OCR text** are private user material: they appear only in the note the user asked for and in explicitly requested stdout, never in error `details`, `doctor` findings, the journal, the spool beyond the requested body, or any log; there is no telemetry, no crash reporting, and no analytics anywhere in the product. **(2) Credentials** — URL userinfo, `--header-file` contents, and response cookies — are never written to the vault, the spool, the journal, the trash, stderr, or a temp file; header buffers are zeroized after the request; cookies are dropped with only a count retained; userinfo is stripped from every recorded and displayed form; secret-shaped query parameters are masked in the recorded URL with the masked keys listed. **(3) Browsing history** — the set of URLs a user clips is itself sensitive; L persists no history file, no cache of visited URLs, and no query log. The only record is the note the user chose to create. Spool and quarantine directories are `0700`, entries `0600`, retained raw sources `0600`.
- [ ] Uses synthetic/test data only until compliance gate clears

### 6.2 Asset provenance

- [ ] No third-party assets
- [x] **Uses third-party assets.** (a) The **hostile-HTML corpus** is shared with **F** and is project-authored or drawn from public security test vectors with each vector's origin and license recorded in `crates/mg-vault-capture/tests/fixtures/PROVENANCE.md`. (b) The **HTTP fixture pages** are project-authored; **no real web page is vendored** and CI never fetches the live internet — every network test runs against a loopback server. (c) The **PDF corpus** is project-generated (a text-layer PDF, a rasterized scan of a project-authored page, an encrypted PDF, a malformed PDF) with no third-party creative content. (d) The **readability heuristic** is a project-owned implementation of the published Readability scoring rules, documented as `readability/1`, not a vendored copy of Mozilla's code; if a crate is adopted instead, its license is recorded here before merge. (e) External tools `poppler` (GPL-2.0) and `tesseract` (Apache-2.0) are **invoked as separate processes and never linked**, so no license obligation propagates to `mg-vault`; this reasoning is recorded in `docs/` alongside the dependency table. (f) All new Rust crates in §4.5 are MIT/Apache-2.0.

### 6.3 Language / claims audit

- [x] **No claim unsupported by evidence.** Every guarantee here maps to a named test in §5. "Sanitized" means F's versioned `mg-vault.html-allowlist/1` applied before the writer, proven by the hostile corpus asserted against file bytes on disk — not a general safety claim, and explicitly not a claim of safety against a malicious kernel or root process.
- [x] **No promise of unbuilt capability.** §7.1 states that this entire branch is **absent** today. `capabilities` reports `clip`, `pdf_extract`, and `ocr` as `available: false, reason: "not built"` in any build that lacks them; no command implies a capability the running binary does not have.
- [x] **No restricted-domain language.** L makes no medical, financial, legal, or safety claim. The word "secure" is not used as a product claim; specific mechanisms are named instead.
- [x] **Fidelity language is regulated by the data model.** `Fidelity::Verbatim` is unconstructible on the readability and OCR paths, so the product cannot label a lossy or machine-transcribed derivation as verbatim even by mistake. The phrase "machine-transcribed" appears literally in the note body, not only in metadata.
- [x] **"Atomic" is scoped.** It means **A**'s single-file same-directory temp + `rename` + parent `fsync`. Refile spans two files and is described as *journaled and recoverable*, explicitly **not** atomic.

### 6.4 Regulatory alignment

The template names Lens 3, but L crosses every lens, so all 26 binding criteria are addressed:

| Criterion | How L satisfies it |
|---|---|
| **1A Authority** | L only creates and appends ordinary Markdown; the index is optional, read-only, and dedupe-only. Delete every index and every clip still reads as a plain file. |
| **1B Preservation** | L never rewrites an existing note's unrelated bytes: capture is a tail append, refile is a fingerprint-bound span removal, and revision is an append. No existing note is ever parsed-and-reserialized. |
| **1C Identity** | Provenance is **visible YAML** the user can read and edit. No UUID, no hidden marker, no invisible comment. The spool ID stays in `$XDG_STATE_HOME` and is never written into a note. Path remains identity. |
| **1D Coexistence** | `validate_note_path` rejects `.obsidian`/`.mg-vault` targets; a coexistence test asserts `.obsidian` is byte-identical after clipping. Capture config lives under `.mg-vault`/XDG, never in `.obsidian`. |
| **1E Transactions** | Collisions fail closed with no overwrite flag; conflicts return both versions; refile is journaled destination-first so a partial state is *duplicated and reported*, never lost; spool ambiguity is quarantined. |
| **2A Keyboard completeness** | Every review action has a documented key **and** a flag; §5.3 asserts the interactive and non-interactive sequences produce byte-identical results. |
| **2B Editing durability** | Spool-before-write means an interrupted capture is replayed, not lost. Commits are **A**'s atomic replace under a fingerprint precondition. |
| **2C Workspace** | L supplies `InboxModel`; **E** owns the pane, splits, and session restore. L claims no workspace behavior of its own. |
| **2D Text correctness** | Grapheme-boundary truncation in the slugger; bytes pass through unmodified in note content; no Unicode normalization is applied to a path. |
| **2E Degraded experience** | A missing `clip`/`extract`/`ocr` feature or tool is an explicit `available: false` with a reason and a package name; it never degrades silently, never writes an empty note, and never blocks editing the vault by hand. |
| **3A Determinism** | The slug algorithm, URL normalization, sanitizer allowlist, and provenance shape are all versioned and fixture-pinned; the same input yields the same path and the same bytes. |
| **3B Ambiguity** | A path collision with a different `source_url` is a terminal `collision` with both URLs shown. L never merges two pages or picks a winner. |
| **3C Query depth** | L's contribution is to make clipped material *queryable by ordinary means*: `source_url`, `retrieved_at`, `extraction_fidelity`, and tags are ordinary properties that **B**/**G** index and **C** queries with existing predicates. L adds no private query path. |
| **3D Derived authority** | The index is used only to *skip* work; it can never authorize a write, supply bytes, or override the direct path check. Unavailable ⇒ path-only, stated. |
| **3E Scale** | Capture and clip cost is independent of vault size — one note read, one note write, no walk, no index. Verified at 100,000 notes in §4.7. |
| **4A Confinement** | Two independent layers: the slugger, then **A**'s `validate_note_path` + `prepare_destination` symlink walk + canonical containment. Property-tested over 100,000 fuzzed titles. Attachment writing is disabled where descriptor-relative confinement is unavailable. |
| **4B Concurrency** | Every existing-note mutation is fingerprint-bound; a conflict retains the capture in the spool and returns both fingerprints. Concurrent captures never interleave into a corrupt append. |
| **4C Least privilege** | Network egress requires a `NetworkGrant` constructible only in the `clip` handler; plugins, AI adapters, spool entries, config, and the index cannot mint one. Child processes get a cleared environment, no shell, no path, and a killed process group. |
| **4D Recovery** | `--dry-run`/`--print-only` preview everything; commits are atomic; dropped inbox entries go to trash; refile is journaled and idempotent; nothing claims success before both `fsync`s return. |
| **4E Contracts** | `mg-vault.capture/1` and `mg.capture/1` are explicit and versioned; `available`/`reason`, `changed`, `verbatim`, and `extraction_fidelity` are truthful fields, and a semantic change bumps the version. |
| **4F Privacy** | See §6.1 — content, URLs, credentials, and history are excluded from logs, errors, journals, exports, and outbound payloads, with grep-the-whole-disk tests. |
| **5A Offline/local-first** | Core workflows — capture, inbox, review, refile, drain, extract — require **no network at all**. The single network moment is an explicit `clip`, which is not a core workflow; a `--no-default-features` build removes it entirely and everything else still works. |
| **5B Responsiveness** | §4.7 gives explicit latency, memory, payload, cap, cancellation, and zero-startup-impact budgets. |
| **5C Accessible equivalents** | Redirect chains, sanitizer reports, and OCR confidence are ordered text; images carry alt text or the literal `description: unavailable`; nothing is available only as a rendered or graphical form. |
| **5D Terminal resilience** | 40-column record layout, literal state words, `NO_COLOR`, escaped bidi/control characters in metadata, static non-redrawing progress, reduced motion by default in every non-interactive context. |
| **5E Automation** | `--json`, `--jsonl`, stdin/stdout, `--print-only`, `--dry-run`, `--no-input`, `--no-color`, `NO_COLOR`, and stable exit categories; §5.3 asserts interactive/flag parity. |

**Auto-fail review, by name.**

- **Unconfirmed overwrite or import:** there is no overwrite flag anywhere in L. Creation is `create_new`. Any mutation of an existing note requires an interactive `[y/N]` confirmation, or under `--no-input` both `--yes` and a fingerprint precondition. A dry-run cannot manufacture a confirmation.
- **Source-content loss:** L never deletes or truncates an existing note; the only removal is a fingerprint-bound inbox span whose content is already durable at the destination. The user's PDF is never moved or modified. Lossy derivations are declared as `lossy`/`machine-transcribed`, and `--keep-source` retains the exact fetched bytes so a lossy conversion is never the only copy.
- **Partial multi-file mutation:** refile is journaled destination-first; a crash duplicates and reports, never loses, and rerun is idempotent.
- **Capability or data-exfiltration bypass:** `NetworkGrant` is a private satisfaction type; the spool cannot request a fetch; plugins and AI cannot construct one; the outbound request is byte-enumerated and marker-tested to contain no vault content; a non-default build links no TLS at all.
- **Active raw HTML or script by default:** deny-by-default allowlist applied to the parsed tree **before** the writer, with compile-time enforcement, script/style/iframe/object/handler stripping, and `javascript:`/`data:` scheme denial including obfuscated forms. The normal output is Markdown containing no HTML element. Asserted against bytes on disk.
- **Unsafe traversal or symlink escape:** a title yields exactly one path component; controls, separators, bidi, `..`, and device names are removed or rejected; the result then passes **A**'s independent authority. Property-tested.
- **Index state overriding source / stale index presented as current:** the index is dedupe-only and never authoritative; unavailable or stale is stated literally.
- **Silent conflict winner:** collisions and conflicts are terminal states with both sides shown.
- **Unknown syntax loss:** L appends and creates; it never round-trips an existing note through a parser.
- **Non-atomic save claiming success:** success is printed only after the rename and both `fsync`s return; "atomic" is scoped to single-file writes and refile is called journaled, not atomic.
- **Recovery overwriting newer source:** spool replay writes only when the target's current fingerprint matches the journal's precondition; otherwise it quarantines and reports.
- **Graph/Canvas information lacking a textual equivalent:** L produces no graphical representation at all; every artifact is text.

---

## 7. Gap Analysis vs. Current State

### 7.1 What exists today

State words are exact: implemented / prototyped / planned / gated / absent.

**Implemented — the foundation L builds on, and nothing more:**

- `crates/mg-vault-core/src/vault.rs` — `Vault::open`, `validate_note_path` (relative, `Normal` components only, `.md` required, `.obsidian`/`.mg-vault` rejected for mutation), `prepare_destination` (per-component symlink and non-directory rejection with canonical containment re-check), `existing_mutation_path`, `create_note`, `read_note`, `write_note`, `edit_note_span`, `trash_note`, `restore_note`, `SourceFingerprint`. **Implemented** — these are exactly the authority primitives L's path safety and fingerprint-bound writes depend on.
- `crates/mg-vault-core/src/atomic.rs` — same-directory temp file, `write_all`, `sync_all`, `renameat2(RENAME_NOREPLACE)`, replacing rename, `sync_parent`, cleanup on every failure. **Implemented.** Its temp-name shape `.{name}.mg-vault-{pid}-{serial}.tmp` is the reason the slugger caps components at 200 bytes rather than 255.
- `crates/mg-vault-core/src/{frontmatter,frontmatter_scalar}.rs` — exact envelope/body span scanning and top-level scalar location. **Implemented** — L writes new frontmatter rather than editing it, but the inbox append point locator reuses the envelope scan.
- `crates/mg-vault-cli/src/main.rs` — `vault register/list/select`, `note create/read/write/trash/restore`, `index rebuild/status`, `search`, `interop export`, global `--vault/--json/--no-input/--no-color`, version-1 JSON envelopes. **Implemented.**
- `crates/mg-vault-index` — disposable SQLite projection with atomic generations and truthful freshness. **Implemented**, and is the optional dedupe source L consults read-only.

**Absent — the entire L branch.** There is no `capture`, `clip`, `extract`, or `inbox` command; no `mg-vault-capture` crate; no capture spool, entry format, drain, replay, or quarantine; no inbox model, review loop, refile, promote, drop, or ordinal listing; no HTTP client, TLS stack, URL parser, address policy, redirect policy, timeout policy, or size cap — `Cargo.toml` contains **no network dependency of any kind**, which is the honest starting point; no HTML parser, sanitizer, or Markdown converter (F's `mg-vault-markdown` crate is itself absent, so L's allowlist source does not yet exist); no readability heuristic; no filename slugger; no `Provenance` type or provenance frontmatter; no external-tool probing, sandboxed spawn, PDF extraction, or OCR; no `capabilities` command rows for `clip`/`pdf_extract`/`ocr`; no attachment write path at all (`validate_note_path` requires `.md`, so a `.html` or `.pdf` attachment is currently unwritable); no `Vault::append_note`; and no capture corpus, loopback fixture server, or hostile-HTML fixtures.

**Gated:** `--no-input` and `--no-color` are accepted globally and their guarantees currently hold only because no command prompts and no command emits ANSI. L introduces the first prompting and first progress-emitting commands in the product, so both flags must be genuinely wired before L ships. **Gated** on **A** §7's color/input policy wiring.

**Planned elsewhere, depended upon:** transaction journaling and descriptor-relative confinement (**A**), the HTML allowlist and Markdown model (**F**), the daily-note path resolver (**J**), the inbox pane (**E**), and the Quickshell capture pill (**Q**).

### 7.2 Delta to spec

**New crate:** `crates/mg-vault-capture/` with the module layout in §4.1, plus `tests/fixtures/` holding the hostile-HTML corpus (shared with **F**), the loopback HTTP fixture pages and fault behaviors, the adversarial-title corpus, the PDF set, and `PROVENANCE.md`.

**New modules within it:** entry model and closed placeholder rendering; spool with IDs, durability ordering, drain, idempotent replay, and quarantine; inbox model with entry spans and refile/promote/drop planners; the filename sanitizer; the `Provenance` type and its YAML emitter; `NetworkGrant`, `FetchPolicy`, the URL/address/redirect/size policy, and the `HttpTransport` trait with a `ureq` backend and a recording test double; the HTML tree builder, the ingest-side sanitizer binding to F's `Policy`, the readability heuristic, and the Markdown writer; the sandboxed child runner, tool prober, PDF text path, and OCR path.

**Modified files:**

- `crates/mg-vault-cli/src/main.rs` → add `capture`, `clip`, `extract`, `inbox` subcommands (in `commands/` modules per **C**'s refactor), the confirmation surface renderer, the progress renderer, and the L rows in `capabilities`.
- `crates/mg-vault-core/src/vault.rs` → add `append_note` and `create_attachment` with a closed non-`.md` extension allowlist; no other change.
- `crates/mg-vault-core/src/error.rs` → add the typed causes in §3.6 that belong to core (`Collision` and `Conflict` already exist and are reused).
- `Cargo.toml` (workspace) → add the §4.5 crates as **optional** workspace dependencies plus the `clip`/`extract`/`ocr` features; add the CI `cargo tree` assertion that `mg-vault-core` stays network-free.
- `crates/mg-vault-cli/src/main.rs` + `doctor` → add pending/quarantined capture checks and `refile_incomplete` reporting.
- `README.md`, `docs/PRODUCT.md`, `docs/ARCHITECTURE.md`, `docs/SECURITY.md` → record the capture crate, the single-network-moment rule, the ingest sanitization boundary, and the external tool dependencies **once shipped**, not before.

**Migrations / schema changes:** none to notes or the index. The spool is new versioned state under `$XDG_STATE_HOME`; an unknown `schema` is quarantined, never parsed.

**New dependencies:** the seven optional Rust crates and three external tools in §4.5. This is the first network dependency in the workspace and the first process spawn, which is why both are feature-gated and architecturally fenced.

### 7.3 Estimated scope

**L (large).** The capture and inbox half is a moderate, well-understood filesystem feature that rides entirely on primitives **A** already implements. The clipping half is where the size lives: an HTTP client with a hand-rolled redirect and address policy, an HTML5 tree walk bound to another crate's allowlist, a readability heuristic, a filename sanitizer that is a traversal boundary, and a test strategy requiring a fault-injecting loopback server and a socket-denial harness. The extraction half is smaller in code but carries the child-process sandbox and the honesty machinery around fidelity.

Ship as gated increments behind one schema version: **L1** entry model + spool + `capture` + drain/replay + crash matrix (this is the highest-value, lowest-risk slice and depends only on **A**); **L2** inbox model + `list`/`review`/`refile`/`promote`/`drop` + the journaled two-file ordering; **L3** the filename sanitizer + `Provenance` + `clip --from-file` (**offline clipping** — full sanitization and provenance with no network at all, which lets the security-critical code land and be fuzzed before a socket ever opens); **L4** the network policy and live `clip`; **L5** `extract` (PDF text); **L6** `--ocr`. L3 must pass the full hostile corpus and the traversal property test before L4 begins.

### 7.4 Blocking dependencies

- **A Foundation and vault authority** — partially implemented and sufficient for L1/L2. L needs `append_note` and `create_attachment` added there. A's transaction journal is required before L2's refile may claim recoverability, and A's descriptor-relative confinement is required before L may write attachments on any platform.
- **F Markdown and rich content** — **hard blocker for L3/L4.** L must not define its own allowlist; `mg_vault_markdown::sanitize::Policy` and `HTML_ALLOWLIST_VERSION` must exist first. If F slips, L3 ships offline-only against a temporarily vendored copy of F's table **under F's version string**, with a merge gate that deletes the copy — it never forks the policy.
- **C CLI and note operations** — L's flags, exit categories, JSON envelope, plan/commit shape, `--no-input` semantics, and `doctor` all extend C's contracts. L should land after C's CLI module refactor to avoid two conventions.
- **J Periodic notes and templates** — owns the daily-note path resolver and the template engine. Until J exists, `--to daily` uses L's local `capture.daily_format` and says so; when J lands, L delegates and the local format becomes a fallback. **Not a blocker for L1.**
- **E TUI workspace** — owns the inbox pane. L supplies `InboxModel` and claims no pane behavior; E's absence does not block any CLI slice.
- **B Index service** — optional and dedupe-only. Absent ⇒ path-only dedupe, stated. Never a blocker.
- **N Plugins / O AI adapters** — not dependencies, but both are explicitly denied `NetworkGrant`; their specs must not claim host socket access through L.
- **External:** `poppler` and `tesseract` packaged on the target distribution; a decision on whether the distributed binary enables `clip` by default (§8-Q1).

---

## 8. Open Questions

- **Q1:** Should the distributed `mg-vault` binary enable the `clip` feature by default, or should the default build be offline with clipping as a separate `mg-vault-clip` package? Default-off is the stronger security posture and makes "no network in the default install" a provable property; default-on is what most users expect from a clipper. — blocks: §4.6 feature defaults, §7.3 packaging, **R**'s Arch packaging.
- **Q2:** Should `--keep-source auto` default to retaining the raw response (current proposal: yes, when ≤ 2 MiB), given that it doubles storage per clip and stores third-party HTML inside a possibly-synced vault? The alternative is `never` by default with a warning that the Markdown is the only copy. — blocks: §4.4 retention default, §4.7 storage budget.
- **Q3:** Should encrypted PDFs be supported via `--password-file`? It is a real user need but adds a second credential-handling surface to a branch that currently has exactly one (`--header-file`). — blocks: §3.6 `pdf_encrypted`, §6.1.
- **Q4:** Is `--images download` in scope for iteration 1 at all? It multiplies the network surface by 25 requests per clip and is the only place L issues more than one request. Deferring it to a later slice would let §4.3's "one request per invocation" become an unconditional invariant. — blocks: §4.3 network policy, §4.7 payload budget.
- **Q5:** Where does the inbox live and who owns rotation — `inbox.md` at the vault root (proposed), a configurable path, or **J**'s daily note as the sole capture target? And at what size does L rotate rather than warn? — blocks: §3.2 capture targets, §4.7 inbox growth.
- **Q6:** Should `capture` be allowed to create its target note when missing (proposed: yes, via `create_new`), or should a missing inbox be an error so a typo in `capture.inbox_path` cannot silently scatter notes? — blocks: §3.2 step 4.
