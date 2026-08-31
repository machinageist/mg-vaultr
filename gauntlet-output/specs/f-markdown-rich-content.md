# Spec: Markdown and Rich Content

**Feature ID:** f-markdown-rich-content
**Parent feature:** root
**Spec author agent:** Hermes markdown/rich-content spec subagent
**Date:** 2026-08-29
**Iteration:** 1

---

## 1. Purpose

### 1.1 One-sentence job

Give `mg-vault` one shared, versioned, token-preserving model of Markdown — CommonMark, GFM, and documented Obsidian forms plus math, code, diagrams, transclusion, and attachments — and one deny-by-default renderer that turns that model into readable terminal output and complete textual equivalents, so a user can read and structurally edit rich notes without a single byte of unknown syntax being rewritten, dropped, or executed.

### 1.2 Why it matters

Every other feature branch needs to know where a heading, a wikilink, an embed, a tag, a task, or a property starts and ends: **B** indexes those spans, **D** offers structural text objects over them, **E** paints a preview pane, **G** resolves links, **H** rewrites them, **I** types properties, and **P** exports them. If each branch invents its own scanner, the product acquires several incompatible Markdown dialects and the vault stops being portable. Worse, the ordinary way to build a renderer — parse to an AST and print it back — silently normalizes exactly the syntax `mg-vault` promised to keep: an Obsidian callout, a Dataview block, a `%%comment%%`, an unusual YAML quoting style, a CRLF line ending, or a plugin construct nobody has heard of.

This feature exists to make preservation mechanical rather than aspirational, following `docs/spikes/token-preserving-markdown-yaml.md`: the file's bytes are the document, a parse is a *description* of byte ranges over those bytes, and the only mutation primitive is a fingerprint-bound splice of an explicitly named range. It also exists because rich content is where a local-first tool usually leaks: an image that phones home, an HTML block that becomes live markup on export, a diagram that renders as pixels a screen-reader user cannot read. All three are handled here, at the output boundary, and never by editing the user's file.

### 1.3 Success signal

Over a versioned compatibility corpus of CommonMark 0.31.2, GFM, Obsidian, math, diagram, malformed, and hostile-HTML fixtures: (a) `cover(parse(source))` concatenates back to the exact input bytes for 100% of fixtures including fuzzed inputs; (b) committing a targeted structural edit to one span in every fixture leaves all other bytes byte-identical, verified by prefix/suffix equality; (c) no public API can serialize a document, proven by an API-surface golden test; (d) every hostile-HTML fixture renders inert in all four terminal tiers and in the export sanitizer; and (e) every image, math block, and diagram in the corpus has a non-empty textual equivalent in `--json` and in `--plain` output, with zero constructs reachable only through a graphics protocol.

---

## 2. User Stories

> As a writer who also uses Obsidian, I want my callouts, wikilinks, embeds, tags, block IDs, and `%%comments%%` to survive an edit made anywhere else in the note, so that switching tools never quietly reformats my vault.

> As a plugin-heavy Obsidian user, I want syntax `mg-vault` has never heard of — a Dataview block, a custom `:::admonition:::`, an emoji shortcode — to be preserved verbatim and displayed as literal text, so that unsupported does not mean destroyed.

> As a reader in a terminal, I want tables, task lists, footnotes, code fences, math, and diagrams to render legibly in my preview pane, and to degrade to plain ASCII in a dumb terminal or over SSH, so that reading a note never depends on my terminal emulator's feature set.

> As a screen-reader user, I want every image, chart, and diagram to have a complete textual equivalent — node and edge lists, alt text, an explicit `description: unavailable` when there is none — so that no part of a note is available only as pixels.

> As a security-minded user, I want raw HTML, `<script>`, `javascript:` links, and remote image URLs to be inert and never fetched by default, so that reading an untrusted clipped note cannot execute anything or signal a third party.

> As a note author, I want `![[Design#Goals]]` to show the target section inline, and an unresolvable or circular embed to say exactly why in place, so that a broken embed is visible instead of an empty gap.

> As an automation author, I want `mg-vault render --json` to emit a stable, versioned inventory of every block with its byte range, kind, and textual equivalent, so that I can build tooling without scraping ANSI output.

> As a maintainer of the index, I want one shared parser contract with an explicit syntax-profile version, so that changing Markdown semantics forces an index rebuild rather than producing two disagreeing dialects.

---

## 3. UX Specification

### 3.1 Screen / view inventory

This is a CLI and TUI product. F introduces no graphical screens, windows, or pointer affordances. It introduces the following line-oriented views and one view model consumed by **E**:

| View | Entry point | New / modified | Layout pattern |
|---|---|---|---|
| Rendered note stream | `mg-vault render PATH` | New | Full-width sequential block stream on stdout; ANSI only when the tier allows |
| Render inventory | `mg-vault render PATH --json` | New | Versioned `mg.render/1` JSON document; ordered blocks with byte ranges |
| Structure outline | `mg-vault outline PATH` | New | Indented heading/block tree with byte offsets and block IDs |
| Media inventory | `mg-vault media list PATH` | New | One record per media reference: raw text, resolution state, resolved path, kind, size, alt |
| Media resolution detail | `mg-vault media resolve REF --from PATH` | New | Single record plus the ordered candidate/attempt list |
| Attachment policy report | `mg-vault media config` | New | Effective attachment folder, its source (`.obsidian/app.json`, `.mg-vault`, default), and precedence trace |
| Transclusion expansion | `mg-vault transclude PATH [--depth N]` | New | Depth-indented expansion tree with per-node state |
| Capability report | `mg-vault render --capabilities` | New | Detected terminal tier, graphics protocol, Unicode width table, color depth, and the override flags |
| Preview pane view model | consumed by **E** | New (F supplies model; E owns placement) | Ordered `RenderBlock` list with width-independent semantics; E maps to its pane |
| Source pane | owned by **D**/**E** | Unmodified | F never alters source rendering; it supplies token kinds for highlighting only |

`render` writes to stdout; warnings and diagnostics go to stderr. `--json` follows the version-1 envelope convention established in `crates/mg-vault-cli/src/main.rs`: exactly one JSON object plus newline on stdout for success, one error object on stderr for failure, the other stream empty.

### 3.2 Interaction flows

**Primary flow — render a note to the terminal.**

1. Resolve the vault and the exact vault-relative `.md` path through existing `Vault` confinement. `render` never performs fuzzy resolution.
2. Read exact bytes and capture the `SourceFingerprint`. All downstream work is bound to that revision.
3. Scan: locate the frontmatter envelope with the existing `scan_frontmatter`, then build the total token cover over the whole file (§4.2). Unclassified byte runs become `Unrecognized` tokens; nothing is skipped.
4. Detect the terminal tier (§3.4 capability ladder) unless overridden by `--tier`, `--ascii`, `--plain`, `--no-color`, or `NO_COLOR`.
5. Build the render model: every token cover node becomes exactly one `RenderBlock` or is folded into its parent block's inlines. Every block carries `text_equivalent`, which is never empty.
6. Resolve media and transclusions with bounded budgets (§3.2 sub-flows below). Each returns a state, never a silent omission.
7. Sanitize: all HTML, all link and image destinations, and all diagram/math payloads pass through the output-side sanitizer. Only sanitizer-constructed values reach the writer.
8. Paint. If the tier is `graphics`, image cells are emitted **after** the textual card line, never instead of it.

**Branch — unsupported construct.** A byte run that no scanner claims stays an `Unrecognized` token. It renders as literal source text in the default paragraph style with a `⟨unsupported⟩` marker in `unicode`/`graphics` tiers and `[unsupported]` in `ascii`/`plain`, and appears in `--json` as `{"kind":"unrecognized","raw":"…","byte_range":[…]}`. It is never dropped, never reformatted, never escaped into the file.

**Branch — media reference.** Resolve per §4.3. `resolved` renders the card (plus inline pixels in the `graphics` tier); `missing`, `ambiguous`, and `blocked` each render a distinct card with the raw reference, the state, and the reason. `mg-vault` never creates, moves, downloads, or rewrites an attachment while rendering.

**Branch — transclusion.** Expand depth-first with cycle detection and budgets per §4.3. Every terminal state — `resolved`, `unresolved`, `ambiguous`, `cycle`, `depth_limit`, `budget_exceeded`, `blocked` — has a rendered card containing the original reference text verbatim and a one-line reason. Rendering the rest of the document continues.

**Branch — hostile input.** Raw HTML, an unknown URL scheme, an oversized image, an unterminated fence, an unclosed math delimiter, a malformed mermaid graph: each downgrades to literal text plus a labeled diagnostic. No case aborts the whole render, and no case mutates source.

**Structural-edit flow (consumed by D and H, not exposed as its own command in this feature).**

1. Caller supplies a buffer snapshot with its revision and requests spans by kind (`heading_section`, `fence_body`, `link_target`, `callout_type`, `frontmatter_value`, …).
2. F returns `TokenSpan { revision, byte_range, token_kind, lexeme_hash, surrounding_hash }` or `StructuralTargetUnavailable`. It never guesses.
3. Caller builds an `EditPlan` of non-overlapping `SpanReplacement`s and passes it back for validation: revision equality, sorted non-overlapping ranges, UTF-8 boundaries, `lexeme_hash` still matching the live bytes, `surrounding_hash` matching, and expected token kind matching.
4. A validated plan commits through one fingerprint-checked atomic multi-splice (§4.3). Validation failure aborts with zero mutation.

No haptic or sound cues exist. Animation is covered in §3.5.

### 3.3 Layout descriptions

The rendered stream is a vertical sequence of blocks in source order. There is no column layout, sidebar, or floating element. Component hierarchy per block, top to bottom, leading to trailing:

- **Frontmatter** (when `--show-frontmatter`, default off in preview, on in `--json`): a key/value table, keys left, values right, unknown keys and unknown YAML shapes shown with their raw text.
- **Heading:** level marker, then text. Tiers `unicode`/`graphics` use a rule under levels 1–2; `ascii` uses `=` and `-` rules; `plain` uses `# ` prefixes so depth survives linearization.
- **Paragraph:** grapheme-aware wrapped inlines at `min(terminal_width, wrap_width)`, default `wrap_width = 100`.
- **List / task list:** `unicode` bullets `•`/`◦`, task boxes `☐`/`☑`; `ascii` `*`/`-` and `[ ]`/`[x]`. Nesting is two spaces per level in every tier.
- **Table:** box-drawn in `unicode`, `+-|` in `ascii`. Below 60 columns any tier switches to one-record-per-block form with `column: value` lines; cells are never truncated.
- **Code fence:** a header line `code · <language or "plain">`, then the body verbatim with no reflow, then a closing rule. Syntax highlighting is decoration supplied by D's grammars when available; absence is not an error.
- **Math:** a header line `math · inline|display`, the LaTeX source verbatim, and — for the documented subset — a Unicode approximation line beneath it labeled `approx`.
- **Diagram:** a header line `diagram · mermaid`, the ASCII/Unicode layout when the subset renders, and always an outline: `nodes:` then one line per node, `edges:` then one line per edge, in deterministic order.
- **Callout:** a leading bar, `[TYPE] Title` (literal type name for unknown types), then the body indented.
- **Media card:** `image|audio|video|pdf|file · <state>`, then `path:`, `alt:` (or literally `description: unavailable`), `size:`, and for `missing`/`ambiguous`/`blocked` a `reason:` and an ordered `tried:` list.
- **Transclusion card:** `embed · <state>` with the raw reference, then either the indented expanded body or the reason line.
- **Block quote / thematic break / footnote definitions:** conventional, with footnotes collected in source order under a `footnotes` rule at the end.

Data sources: the render model derives solely from the immutable `SourceDocument` snapshot plus, for resolution only, **G**'s link resolver and **B**'s attachment path projection. When those are unavailable, F falls back to a direct confined filesystem probe for a single reference and labels resolution `direct` rather than `indexed`; it never presents an index answer as source.

Empty states: an empty note renders `(empty note)`. A note with only frontmatter renders `(no body content)`. `media list` on a note with no references prints `No media references in PATH.` `transclude` on a note with no embeds prints `No embeds in PATH.` `outline` with no headings prints `No headings; 1 block.`

### 3.4 Input & gestures

Mouse, touch, stylus, camera, voice, and controller input are **N/A** — this is a terminal product. All interaction is keyboard and flags.

CLI flags on `render`, `outline`, `media`, and `transclude` as applicable: `--json`, `--plain`, `--ascii`, `--tier {graphics|unicode|ascii|plain}`, `--no-color`, `--width N`, `--wrap N`, `--depth N`, `--max-expansions N`, `--show-comments`, `--show-frontmatter`, `--html-mode {literal|sanitized}`, `--images {auto|off}`, `--no-input`, plus the global `--vault NAME`. `NO_COLOR` in the environment is equivalent to `--no-color` and wins over any auto-detection. `--plain` implies `--no-color` and `--images off`. Content can be piped: `--from-stdin` renders bytes from stdin with `--base-path PATH` supplying the resolution base; without a base path, media and embeds resolve to `blocked` rather than guessing a root.

Preview-pane keys (F defines the actions; **E** owns binding and may remap): `p` toggle source/preview, `zc`/`zo` fold and unfold the block under the cursor, `gd` jump from a rendered link or embed to its target, `gr` reveal the source byte range of the block under the cursor, `yc` yank the block's textual equivalent, `]b`/`[b` next/previous block, `]d`/`[d` next/previous diagnostic, `gi` cycle the image mode for the block under the cursor, `gh` toggle `--show-comments`. Every action is reachable from the command line as `:render <action>`; there is no pointer-only or graphics-only action.

Responsive behavior across terminal widths: 120 and 80 columns use full block layout; below 60 columns tables and media cards switch to record form; below 40 columns diagrams drop the spatial layout and emit only the node/edge outline, and long paths wrap with a two-space continuation indent rather than truncating. Width never changes `--json`.

**Capability ladder (the ASCII fallback path).** Detection is explicit and reported by `render --capabilities`:

| Tier | Detection | Images | Boxes / bullets | Color |
|---|---|---|---|---|
| `graphics` | Kitty graphics, iTerm2 inline images, or Sixel confirmed by a bounded terminal query with a 150 ms deadline | Inline cells **after** the textual card | Unicode box drawing | Full |
| `unicode` | UTF-8 locale, no graphics protocol | Textual card only | Unicode box drawing | Full |
| `ascii` | Non-UTF-8 locale, `TERM=dumb`, `--ascii`, or an unknown width table | Textual card only | `+ - |` and `* [ ] [x]` | Optional |
| `plain` | `--plain`, non-TTY stdout, or screen-reader mode | Textual card only | Semantic line prefixes only | None |

A tier only removes decoration. The semantic inventory — every block, its kind, its text equivalent, its state — is identical in all four tiers and is asserted by test (§5.1). Detection failure falls to `unicode`, never to a richer tier.

### 3.5 Transitions & animation

There are no navigation transitions, no easing, and no sound. Three time-varying behaviors exist and each has a static alternative:

- **Image decode placeholder.** A media card renders immediately with its textual content; if a decode is in flight in the `graphics` tier, the cell region shows a static `decoding…` line that is replaced once. There is no spinner and no repaint loop.
- **Progressive render of a long note.** Blocks stream in source order as they are built. In `--json`, `--plain`, non-TTY output, or when `--no-progress` is set, the output is buffered and emitted once so no partial repaint occurs.
- **Fold/unfold in E's preview.** F exposes fold state as a boolean per block; no animated height change is defined. E must repaint statically.

Reduced-motion alternative: setting `MG_VAULT_REDUCED_MOTION=1`, `--plain`, or any non-TTY stdout disables progressive painting and the decode placeholder entirely. No information is conveyed by motion in any tier.

### 3.6 Error states

| Code | Trigger | Presentation and recovery | Data-loss risk |
|---|---|---|---|
| `unsafe_path` | Note or attachment path escapes the vault, is absolute, contains `..`, is a symlink leaving the root, or targets `.obsidian`/`.mg-vault` for mutation | Inline block card `blocked`, plus stderr diagnostic naming the rule not the resolved secret path | No |
| `frontmatter_ambiguous` | `scan_frontmatter` returns `UnsupportedBom` or `Unclosed` | Frontmatter renders as literal body text with a banner diagnostic; body still renders; structural frontmatter edits are refused | No |
| `unrecognized_syntax` | Byte run no scanner claims | Inline literal text with `[unsupported]` marker; listed in `--json` | No |
| `media_missing` | Reference resolves to no existing file | Media card `missing` with raw reference and the ordered `tried:` list; recovery text names `mg-vault media resolve` | No |
| `media_ambiguous` | Shortest-path filename match has ≥2 candidates | Card `ambiguous` with ordered candidates; **never auto-picked**; recovery is to write an explicit path | No |
| `media_blocked` | Denied URL scheme, remote URL with `--images off` (the default), oversize, or confinement failure | Card `blocked` with the reason and the literal destination as inert text | No |
| `media_type_mismatch` | Magic bytes disagree with the extension | Card renders as the sniffed kind with an explicit `declared: .png, detected: text/plain` line; no inline decode | No |
| `embed_unresolved` | Target note, heading, or block ID not found | Card `unresolved` with the raw `![[…]]` and reason | No |
| `embed_ambiguous` | Duplicate note name, heading text, or block ID | Card `ambiguous` with ordered candidates; no target chosen | No |
| `embed_cycle` | Expansion stack repeats a `(path, anchor)` pair | Card `cycle` printing the full ordered chain `a.md → b.md#X → a.md` | No |
| `embed_depth_limit` / `embed_budget` | Depth > `max_depth`, or expansion/byte budget exhausted | Card naming the limit and its current value; deeper content is reachable by opening the target | No |
| `html_blocked` | Element or attribute outside the allowlist | Escaped literal text in the render, dropped element in export, both with a diagnostic | No; source untouched |
| `diagram_unsupported` / `diagram_budget` | Mermaid syntax outside the pinned subset, parse failure, or node/edge/time budget exceeded | Code fence with verbatim source plus `diagram unavailable: <reason>`, and the node/edge outline when partially parsed | No |
| `math_unbalanced` | Unclosed `$` or `$$` under the active policy | The run stays literal text with a diagnostic naming the policy | No |
| `document_too_large` | Source exceeds the per-document cover ceiling (§4.7) | Large-document mode: windowed render and outline still work; whole-document structural operations refuse | No |
| `structural_target_unavailable` | Requested token kind absent, stale revision, hash mismatch, or overlapping plan | Typed error to the caller (D/H); zero mutation | No |
| `conflict` | Source fingerprint changed between span discovery and commit | Existing `Error::Conflict` with expected/actual digests; caller re-reads | No; both versions intact |
| `resolver_unavailable` | G/B not running | Resolution falls back to a labeled `direct` probe or reports `unknown`; render continues and never claims `resolved` on no evidence | No |

Every presentation is inline in the block stream (so the error appears exactly where the content is) plus a stderr diagnostic line; no modal, toast, or banner idiom exists in a terminal. No error in this table mutates a file, so **data-loss risk is `No` for every row** — F is a read-and-describe feature, and the only write path is the caller-driven, fingerprint-checked splice in §4.3.

### 3.7 Accessibility

- **Textual equivalents are mandatory and structural.** `RenderBlock::text_equivalent` is a non-`Option` field; a block cannot be constructed without one. Images carry alt text, or literally `description: unavailable` — a filename is never promoted to a description. Diagrams carry an ordered node list and edge list. Math carries its LaTeX source and, for the documented subset, a Unicode approximation. Tables carry a record-form linearization with column labels repeated per record. This satisfies Lens 5C and the graph/Canvas/media auto-fail rule.
- **Screen-reader labels and traits.** Every block emits a role word at the start of its first line (`heading level 2`, `table 4 columns 12 rows`, `image`, `diagram`, `embed unresolved`, `code rust`, `callout warning`). Every interactive preview action has a spoken label and a keyboard binding; there are no custom gestures to describe because there are no gestures.
- **Custom actions for complex constructs.** A table exposes `read row N`, `read column NAME`; a diagram exposes `read nodes`, `read edges`, `read path FROM TO`; a transclusion exposes `expand`, `collapse`, `open target`. All are `:render <action>` commands, so nothing requires spatial navigation.
- **Text scaling / dynamic type is N/A in a terminal**; the equivalent is width. Layout is verified at 40, 60, 80, and 120 columns, and no essential value is right-aligned, spatially encoded, or truncated at any width.
- **Color-independent state.** Every state — `resolved`, `missing`, `ambiguous`, `blocked`, `cycle`, `unsupported`, `unresolved` — is a literal word in the output. Color is decoration only, and `plain`/`ascii` tiers carry the full semantics with zero ANSI. Task completion is `[x]` not green; diagnostics are `warning:`/`error:` not red.
- **Focus order and keyboard navigability.** Preview focus order is strict source order of blocks; `]b`/`[b` move linearly; folding never removes a block from the order, it only collapses its body and keeps the header focusable. Every construct is reachable by keyboard alone.
- **Unicode correctness.** Wrapping, alignment, and cursor mapping use grapheme clusters and a pinned width table; combining marks, emoji ZWJ sequences, CJK wide cells, and RTL runs must not corrupt alignment or byte offsets. Bidi and C0/C1 control characters in *metadata* (paths, alt text, diagnostics) are escaped as `\u{…}` to prevent terminal-state injection; *note body* bytes passed to `--json` and to raw source output remain byte-exact.

---

## 4. Implementation Specification

### 4.1 Architecture placement

A new crate `crates/mg-vault-markdown/` owns the syntax model and the renderer. It is the single Markdown authority for the workspace; **B**'s `PARSER_VERSION` becomes derived from F's syntax-profile version, and B stops carrying its own `wikilinks()` scanner.

```
crates/mg-vault-markdown/src/
  lib.rs              # SourceDocument, SyntaxProfile, public re-exports
  source.rs           # token cover, coverage invariant, node arena
  scan/commonmark.rs  # pulldown-cmark OffsetIter driver + gap filler
  scan/obsidian.rs    # wikilink, embed, callout, tag, block id, %%comment%%, ==highlight==
  scan/math.rs        # delimiter policies
  scan/frontmatter.rs # thin layer over mg-vault-core::scan_frontmatter + yaml-edit spans
  edit.rs             # TokenSpan, EditPlan, SpanReplacement, validation
  transclude.rs       # resolution, cycle detection, budgets
  media.rs            # attachment policy, path resolution, sniffing
  render/model.rs     # RenderDocument, RenderBlock, RenderInline, text equivalents
  render/sanitize.rs  # HTML + URL allowlist; sole constructor of SafeInline/SafeBlock
  render/terminal.rs  # tier ladder, width/grapheme layout, ANSI writer
  render/math.rs      # Unicode approximation for the documented subset
  render/mermaid.rs   # pinned subset parser + ASCII layout + outline
  capabilities.rs     # terminal protocol/width/color detection
```

`mg-vault-core` remains the sole filesystem and durability authority. F adds two narrow primitives there (§4.3) and otherwise consumes `Vault`, `SourceFingerprint`, `scan_frontmatter`, and `edit_note_span`. `mg-vault-cli` gains `render`, `outline`, `media`, and `transclude` subcommands that own no policy. `mg-vault-index` (**B**) depends on `mg-vault-markdown` for structural projections. **D** depends on it for the `TokenSpan` contract already named in `specs/d-editor-engine.md` §4.5. **E** consumes `RenderDocument`. **P** consumes `render/sanitize.rs` at the export boundary. Nothing in `mg-vault-markdown` opens a file by absolute path, spawns a process, or opens a socket; it takes bytes and a confined reader capability.

### 4.2 Data model

```rust
/// Versioned identity of the supported syntax surface. Changing any variant
/// invalidates derived indexes; it never authorizes a source rewrite.
pub const SYNTAX_PROFILE: &str = "mg-vault.syntax/1";
pub const HTML_ALLOWLIST_VERSION: &str = "mg-vault.html-allowlist/1";
pub const RENDER_SCHEMA: &str = "mg.render/1";

/// An immutable snapshot of one note's exact bytes plus a total description of them.
pub struct SourceDocument {
    source: Arc<str>,
    revision: SourceFingerprint,
    /// Ordered, gapless cover of `0..source.len()`. INVARIANT: concatenating
    /// `source[t.byte_range]` for every token reproduces `source` byte-for-byte.
    tokens: Vec<Token>,
    /// Structure references token indices; it owns no text of its own.
    nodes: Vec<Node>,
    line_endings: LineEndings,
    diagnostics: Vec<ScanDiagnostic>,
}

/// One classified byte run. `Unrecognized` is a first-class kind, not a failure.
pub struct Token {
    pub byte_range: Range<usize>,
    pub kind: TokenKind,
}

pub enum TokenKind {
    FrontmatterDelimiter, FrontmatterYaml,
    HeadingMarker, HeadingText, ParagraphText, ThematicBreak,
    ListMarker, TaskMarker, BlockQuoteMarker, TableCell, TableRule,
    FenceOpen { info: Range<usize> }, FenceBody, FenceClose,
    CodeSpan, HtmlBlock, HtmlInline,
    LinkText, LinkDestination, LinkTitle, Autolink, FootnoteRef, FootnoteDef,
    Emphasis, Strong, Strikethrough, Highlight,
    WikiLink { target: Range<usize>, anchor: Option<Range<usize>>, alias: Option<Range<usize>> },
    Embed  { target: Range<usize>, anchor: Option<Range<usize>>, params: Option<Range<usize>> },
    CalloutMarker { kind: Range<usize>, fold: Option<Fold> },
    Tag, BlockId, ObsidianComment,
    MathInline, MathDisplay,
    Whitespace, LineEnding,
    /// Any byte run no scanner claims. Preserved verbatim, rendered literally.
    Unrecognized,
}

/// A revision-bound handle to one editable byte range. This is the exact contract
/// `specs/d-editor-engine.md` requires before enabling structural edits.
pub struct TokenSpan {
    pub revision: SourceFingerprint,
    pub byte_range: Range<usize>,
    pub token_kind: TokenKind,
    /// SHA-256 of `source[byte_range]` at discovery time.
    pub lexeme_hash: [u8; 32],
    /// SHA-256 of up to 64 bytes on each side, clamped to document bounds.
    pub surrounding_hash: [u8; 32],
}

/// A set of non-overlapping replacements committed as one atomic splice.
pub struct EditPlan {
    pub revision: SourceFingerprint,
    pub replacements: Vec<SpanReplacement>, // sorted, disjoint, UTF-8 aligned
}

pub struct SpanReplacement {
    pub span: TokenSpan,
    pub replacement: Vec<u8>, // validated UTF-8
}

/// A renderable block. `text_equivalent` is not optional; a block cannot exist
/// without one, which is how the accessibility guarantee is enforced by type.
pub struct RenderBlock {
    pub byte_range: Range<usize>,
    pub kind: RenderBlockKind,
    pub state: BlockState, // Ok | Unsupported | Missing | Ambiguous | Blocked | Cycle | DepthLimit | BudgetExceeded
    pub text_equivalent: String,
    pub diagnostics: Vec<RenderDiagnostic>,
    pub children: Vec<RenderBlock>,
}

/// Sanitizer-only constructors. Private fields make an unsanitized write a compile error.
pub struct SafeInline(String);
pub struct SafeBlock(Vec<SafeInline>);

pub struct MediaReference {
    pub raw: Range<usize>,
    pub declared: String,
    pub alt: Option<String>,
    pub resolution: MediaResolution, // Resolved{path,kind,bytes,sniffed} | Missing{tried} | Ambiguous{candidates} | Blocked{reason}
}

pub struct AttachmentPolicy {
    pub folder: AttachmentFolder, // VaultRoot | Fixed(PathBuf) | NoteRelative(Option<PathBuf>)
    pub source: PolicySource,     // ObsidianAppJson | MgVaultConfig | Default
}
```

There is **no** `Display`, `to_string`, `serialize`, `write_source`, or `render_markdown` on `SourceDocument`, `Token`, or `Node`, and none may be added; an API-surface golden test enforces this (§5.1). No database migration is introduced — F stores nothing durable except an optional owner-only render cache under XDG cache, keyed by `(path, fingerprint, tier, width, syntax profile)`, which is disposable and never consulted for source bytes.

### 4.3 API contracts

**Parsing and spans.**

```rust
pub fn parse(source: &str, revision: SourceFingerprint, profile: &SyntaxProfile)
    -> SourceDocument;                                   // total; never fails
pub fn cover_is_total(doc: &SourceDocument) -> bool;      // debug assert + test oracle
pub fn find_span(doc: &SourceDocument, target: StructuralTarget)
    -> Result<TokenSpan, StructuralError>;                // Ambiguous/Missing/Stale -> Err
pub fn validate_plan(doc: &SourceDocument, plan: &EditPlan) -> Result<(), StructuralError>;
```

`parse` is total by construction: the CommonMark driver walks `pulldown_cmark::OffsetIter`, and every byte run *not* covered by an event range is emitted as a `Whitespace`, `LineEnding`, or `Unrecognized` token by the gap filler. This is the mechanism that turns an incomplete upstream parser into a lossless cover, and it is why `parse` has no error case. Obsidian scanners run as a second pass restricted to `ParagraphText`, `HeadingText`, `TableCell`, and `Unrecognized` regions — never inside `CodeSpan`, `FenceBody`, `HtmlBlock`, or `MathInline`/`MathDisplay` — so misleading syntax inside a fence is never reclassified. Per the spike, every discovered span is checked against the expected source slice before it may authorize an edit.

**Mutation.** F performs no writes. It emits plans; `mg-vault-core` commits them. F requires one new core primitive:

```rust
/// Replace several disjoint UTF-8 byte spans in one fingerprint-checked atomic write.
/// Overlapping, unsorted, out-of-bounds, or non-boundary spans are rejected before I/O.
pub fn edit_note_spans(
    &self, relative: impl AsRef<Path>,
    edits: &[(Range<usize>, Vec<u8>)],
    expected: &SourceFingerprint,
) -> Result<SourceFingerprint>;
```

This generalizes the existing `edit_note_span` and satisfies the spike's rule that multi-edit operations use one plan, one fingerprint check, overlap rejection, and one atomic replacement — repeated single-span commits are explicitly not a substitute.

**Attachment reads.** `validate_note_path` currently requires a `.md` extension, so attachments cannot be read through it. F requires a sibling primitive:

```rust
/// Read an attachment with the same confinement rules as a note, minus the `.md`
/// requirement, with a caller-supplied byte ceiling and no symlink following.
pub fn read_attachment(&self, relative: impl AsRef<Path>, max_bytes: u64)
    -> Result<AttachmentBytes>;
```

On Linux it uses the `openat2` + `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS` path already used by `index::read_source_bytes`. It rejects a first component of `.obsidian` or `.mg-vault` for generic reads; the attachment-policy reader is a separate, narrowly scoped capability that may read exactly `.obsidian/app.json` and nothing else, read-only.

**Attachment path resolution**, in fixed order:

1. A destination containing `/`, or starting with `./` or `../`, is a path: note-relative for `./`/`../`, otherwise vault-relative. Markdown `](dest)` destinations are percent-decoded once and `<…>` bracket form is supported; wikilink targets are **not** percent-decoded, matching Obsidian.
2. Otherwise, a shortest-unique-filename lookup: exact filename first, then filename + each extension in the pinned extension list, over the attachment projection from **B** or, when B is unavailable, a bounded confined probe of the effective attachment folder plus the note's own folder.
3. Zero matches → `Missing { tried }`. Two or more → `Ambiguous { candidates }`, ordered by vault-relative byte order, **never auto-selected**.
4. Absolute paths, `file:`, `..` escapes, and symlinks leaving the root → `Blocked`.
5. `http:`/`https:` destinations are never fetched by the terminal renderer; they render as a link card, `Blocked { reason: "remote fetch disabled" }` under the default `--images auto`.

**Attachment folder policy** precedence: `.obsidian/app.json` `attachmentFolderPath` (values `/`, `folder`, `./`, `./subfolder`) when readable → `.mg-vault/config.toml` `attachments.folder` → default `attachments/`. `mg-vault media config` prints the effective value and the full precedence trace. `.obsidian` is read, never written, by any code path in this feature.

**Media is never inlined into note source.** F has no code path that base64-encodes or otherwise embeds binary content into a `.md` file. Export embedding, if **P** chooses it, is an export artifact only.

**Transclusion.**

```rust
pub fn expand(doc: &SourceDocument, ctx: &TransclusionContext) -> TransclusionTree;

pub struct TransclusionContext {
    pub max_depth: u8,          // default 4, configurable 0..=8
    pub max_expansions: u16,    // default 256
    pub max_expanded_bytes: u64,// default 4 MiB
    pub stack: Vec<(PathBuf, Option<Anchor>)>, // cycle key
}
```

Only `![[…]]` transcludes. `![alt](note.md)` is a Markdown image with a `.md` destination and renders as a `Blocked` media card, not an embed. Anchors: `#Heading` extracts from the matched heading to the next heading of the same or lower depth; `#^blockid` extracts the block carrying that trailing identifier (`[A-Za-z0-9-]+`). A duplicate heading text or duplicate block ID in the target is `Ambiguous` with ordered candidates and no chosen target. The embedded note's frontmatter is excluded from the expansion by default. Relative attachment references inside an expansion resolve against the **embedded** note's path, not the host's. Cycle detection keys on `(canonical resolved path, anchor)`; a repeat yields `Cycle` with the full ordered chain. Exceeding depth, expansion count, or byte budget yields the corresponding state. Expansion is render-only: no target bytes are ever written into the host file.

**HTML and URL sanitization — deny by default, output side only.**

- Source HTML is preserved byte-identically and is never rewritten in the file.
- `--html-mode literal` (default everywhere, including export unless P overrides it explicitly) renders HTML as escaped literal text.
- `--html-mode sanitized` permits exactly: `br, em, strong, i, b, code, kbd, mark, sub, sup, u, del, ins, span, p, ul, ol, li, blockquote, hr, h1..h6, a, img, table, thead, tbody, tr, th, td`. Attributes allowed per element only: `a[href, title]`, `img[src, alt, title, width, height]`, `th/td[colspan, rowspan]`. Everything else — every other element, every other attribute, every `on*` handler, every `style`, every `class`, comments, CDATA, processing instructions, namespace-prefixed names, and every unbalanced or nested-parse-differential construct — is dropped from output and reported.
- URL schemes are allowlisted to `http`, `https`, `mailto`, and vault-relative paths. `javascript:`, `data:`, `vbscript:`, `file:`, and unknown schemes render as inert literal text. Percent, unicode, and whitespace-obfuscated variants are normalized before the check, and any scheme that fails to parse is denied.
- Exported anchors are forced to `rel="noopener noreferrer nofollow"` with `target` removed.
- SVG is never executed or inlined; it is a media card in the terminal and a link (not an embed) on export.
- **Enforcement is structural:** the writer accepts only `SafeInline`/`SafeBlock`, whose fields are private to `render/sanitize.rs`. Any render or export path that tries to write unsanitized text fails to compile, which is what makes "applied at every render and export boundary" checkable rather than a promise.

**CLI contracts.** `render`, `outline`, `media`, `transclude` are read-only; they take an exact vault-relative path, never a fuzzy selector, and never mutate. `--json` emits `{"version":1,"ok":true,"command":"render","data":{"schema":"mg.render/1","syntax_profile":"mg-vault.syntax/1","html_allowlist":"mg-vault.html-allowlist/1","tier":"…","path":"…","fingerprint":"sha256:…","blocks":[…],"diagnostics":[…]},"warnings":[]}`. Arrays always appear even when empty and have documented stable ordering (source byte order). Exit codes follow the categories in `specs/c-cli-note-operations.md` §4.3. No auth or rate limiting applies; these are local read-only commands.

### 4.4 State management

- **Authoritative state:** the note bytes and the attachment files. F reads them and never owns them.
- **Owned state:** none that is durable and load-bearing. A `SourceDocument` is an in-memory snapshot bound to one fingerprint; per the spike, documents and token covers are **not retained for closed notes**. The render cache under `$XDG_CACHE_HOME/mg-vault/render/` is owner-only, keyed by `(path, fingerprint, tier, width, syntax profile, html allowlist version)`, disposable, and invalidated by any key change; deleting it changes nothing but latency.
- **Store ownership:** in the TUI, **E** owns the preview pane store and holds a `RenderDocument` per visible buffer; **D** owns the buffer and passes immutable snapshots. F introduces no new global state container and no singleton.
- **Local vs. server-synced:** entirely local. There is no server and no synced state.
- **Offline / draft persistence:** F persists no drafts. A structural edit lives in D's buffer and undo journal until D commits it through core; if the process dies mid-flow, D's recovery journal owns the proposed bytes and F contributes nothing that could be replayed over a newer source.
- **Staleness discipline:** resolution answers sourced from B are labeled `indexed` with B's generation and freshness; direct probes are labeled `direct`; absence is labeled `unknown`. A `resolved` state is never emitted without positive evidence, and a stale index answer is never presented as current.

### 4.5 Dependencies

New crates: `pulldown-cmark` (CommonMark 0.31.2 + GFM tables, task lists, strikethrough, extended autolinks, footnotes; MIT) with `OffsetIter`; `unicode-segmentation` and `unicode-width` (grapheme clusters and pinned cell widths; MIT/Apache-2.0); `image` (decode only, MIT/Apache-2.0), gated behind an `images` feature; a terminal-graphics encoder for Kitty/iTerm2/Sixel, gated behind the same feature. `yaml-edit 0.2.3` is **already a workspace dependency** and is used only as a YAML span locator behind the project-owned adapter, exactly as the spike prescribes. `markdown-rs` and `yaml-rust2` are **dev-dependencies only**, used as differential oracles in tests; neither is in the mutation path. The mermaid subset parser and layout engine are project-owned Rust — no JavaScript runtime, no headless browser, no `eval`, no network.

Explicitly rejected: `serde_yaml` (loses comments, quoting, order, whitespace; unmaintained), any whole-document Markdown serializer, and Tree-sitter as a mutation authority — Tree-sitter stays editor-only for highlighting and navigation under **D**.

New assets: a versioned compatibility corpus under `crates/mg-vault-markdown/fixtures/`, authored originally for this project (see §6.2). Infrastructure changes: none. No database, no CDN, no third-party service, no network egress of any kind.

### 4.6 Platform-specific considerations

- Arch Linux is first support; the parser, sanitizer, and render model are platform-independent pure Rust. Only `capabilities.rs` and `read_attachment` have platform-specific paths.
- Confined attachment reads use `openat2` on Linux. On a platform where descriptor-relative confinement is unavailable, attachment reading is **disabled** and every media reference resolves to `Blocked { reason: "confinement unavailable" }` rather than falling back to a lexical check. Note rendering continues.
- Terminal graphics detection is a bounded, cancellable query with a 150 ms deadline; a terminal that does not answer is treated as `unicode`. Detection never blocks the first paint and never writes escape sequences to a non-TTY stdout.
- Unicode width tables vary between emulators. F pins one `unicode-width` version, records it in `--capabilities`, and treats width purely as presentation — width never affects byte offsets, identity, or `--json`.
- Line endings are preserved as observed (`LineEndings::{Lf, CrLf, Mixed}` already exists in `frontmatter.rs`) and are never normalized, including inside a rendered or transcluded region.
- Feature flags: `images` (graphics tier and decoders) and `diagrams` (mermaid subset) are compile-time gates for constrained builds. A disabled feature degrades to the textual card/outline, which is already the mandatory path, so semantics do not vary by build — only decoration does. `SYNTAX_PROFILE` and `HTML_ALLOWLIST_VERSION` never vary by feature flag.
- Version compatibility: bumping `pulldown-cmark`, the syntax profile, or the allowlist requires re-running the parser-upgrade snapshot gate over the whole corpus and forces a **B** index rebuild; it never authorizes a source rewrite.

### 4.7 Performance budget

Measured on a warm local SSD reference machine, reported with hardware/OS metadata:

- **Parse throughput** ≥ 20 MiB/s single-threaded over the compatibility corpus. Per-note parse of a 100 KiB note: p50 ≤ 3 ms, p95 ≤ 8 ms, p99 ≤ 16 ms. Amortized cost ≤ 40 µs/KiB, which keeps **B**'s 100,000-note / 1,000,000-block corpus inside its own budget; F publishes p50/p95/p99 and peak RSS against that corpus.
- **Memory.** Token cover plus node arena ≤ 6× source bytes. Hard ceiling 64 MiB of cover per document; above it the document enters large-document mode (windowed cover over the visible range; whole-document structural operations return `document_too_large`). Documents are dropped when a note closes; no cover is cached across notes.
- **Render.** One 200-line viewport: p95 ≤ 16 ms so preview scrolling holds 60 Hz. Incremental re-render after a single-line edit: p95 ≤ 4 ms via a block cache keyed by `(block byte range, revision, tier, width)`. Every render, resolution, decode, and expansion is cancellable and checks cancellation at least every 4 ms or 64 KiB.
- **Media.** Decode ceiling 32 MiB decoded and 8192×8192 pixels; above either, the card says `too large to preview`. Attachment reads are capped by `max_bytes` (default 64 MiB) and are streamed, not slurped, for sniffing. Decode runs off the render thread.
- **Diagrams.** Mermaid subset bounded at 500 nodes, 1000 edges, and 250 ms of layout; exceeding any bound emits the node/edge outline with `diagram_budget`.
- **Transclusion.** Depth 4, 256 expansions, 4 MiB expanded bytes by default; each is configurable and each failure is an explicit state.
- **Storage.** Client-side render cache bounded at 64 MiB with LRU eviction and a documented purge command; zero server storage.
- **Startup.** F adds no work to process start: the crate has no initializer, capability detection is lazy and deadline-bounded, and no corpus or grammar is loaded until the first render.
- **Network payload: zero bytes.** No renderer path performs DNS, HTTP, or socket I/O; remote images are never fetched. This is asserted by a test that fails the build if a network syscall occurs during the corpus render (§5.2).

---

## 5. Test Specification

### 5.1 Unit tests

| Test | Setup and assertion |
|---|---|
| `cover_is_total_and_ordered` | Property/fuzz over generated and corpus sources: tokens are sorted, gapless, non-overlapping, start at 0, end at `len`, and concatenate to the exact input. Edge cases: empty file, no final newline, lone `---`, BOM, mixed CRLF/LF. |
| `parse_is_total_and_panic_free` | Fuzz with arbitrary UTF-8 including control characters, unbalanced fences, and 1 MiB single lines; `parse` never panics and never returns an error. |
| `unknown_syntax_becomes_unrecognized` | Dataview blocks, `:::admonition:::`, emoji shortcodes, unknown directives: each is a covered `Unrecognized` run with exact bounds, not silently merged into a neighbor. |
| `no_document_serializer_in_public_api` | API-surface golden over the crate's public items; fails if any `Display`/`to_string`/`serialize`/`write_source` on a document/token/node type appears. Guards the spike's prohibition on whole-document serializers. |
| `edit_plan_validation_matrix` | Overlapping, unsorted, out-of-bounds, mid-codepoint, stale-revision, wrong-kind, and hash-mismatched plans all reject before any I/O. |
| `targeted_edit_preserves_prefix_and_suffix` | Property test: for every corpus fixture and every discoverable span, splice a replacement and assert bytes before and after the span are byte-identical. |
| `obsidian_scanners_respect_code_context` | `[[link]]`, `#tag`, `$x$`, `%%c%%`, `^id` inside code spans, fences, HTML blocks, and math are never reclassified. |
| `math_policy_matrix` | `obsidian-dollar-v1` and `strict-dollar-v1`: whitespace adjacency, escaped `\$`, currency runs, blank line inside `$…$`, multi-line `$$`, `$$` inside a fence. Assert classification only; assert bytes unchanged under every policy. |
| `html_allowlist_is_deny_by_default` | Every element/attribute/scheme in the hostile corpus: allowed set renders styled, everything else renders escaped and is reported. No fixture produces active markup. |
| `sanitizer_is_the_only_constructor` | `trybuild` compile-fail: constructing `SafeInline`/`SafeBlock` outside `render/sanitize.rs` fails to compile. |
| `url_scheme_normalization` | Percent-, unicode-, tab-, and newline-obfuscated `javascript:`/`data:` variants all deny. |
| `media_resolution_matrix` | Exact path, note-relative, vault-relative, bare filename unique, bare filename ambiguous, missing, absolute, `..`, symlink escape, remote URL, percent-encoded, `<bracketed path>`, extension inference. Ambiguity never auto-picks. |
| `attachment_policy_precedence` | `.obsidian/app.json` present/absent/malformed × `.mg-vault` present/absent; assert effective folder, source label, and that `.obsidian` bytes are unchanged after every case. |
| `magic_byte_sniffing_reports_mismatch` | `.png` containing text, `.md` containing PNG: reported, never silently corrected, never decoded. |
| `transclusion_cycle_and_budgets` | Self-embed, two-cycle, three-cycle, anchored cycle, depth 5 at limit 4, 257 expansions, 5 MiB expansion. Each yields the correct state with a complete chain/reason and the rest of the document still renders. |
| `transclusion_anchor_extraction` | Heading section boundaries at equal/lower/higher depth, duplicate headings, duplicate block IDs, missing anchor, frontmatter exclusion, embedded-note-relative attachment base. |
| `render_block_has_text_equivalent` | Property test over every corpus fixture: no `RenderBlock` has an empty `text_equivalent`, including images, diagrams, math, and every error state. |
| `mermaid_fallback_always_has_text` | Unsupported syntax, parse failure, and budget exhaustion each produce verbatim source plus a reason plus any partial node/edge outline. |
| `grapheme_layout_is_correct` | Combining marks, ZWJ emoji, CJK wide cells, RTL runs: wrapping and table alignment are stable and byte offsets are unaffected. |
| `metadata_controls_are_escaped` | ESC/C0/C1/bidi in paths, alt text, and diagnostics are escaped; note body bytes in `--json` remain byte-exact. |

### 5.2 Integration tests

- `corpus_round_trip`: for every fixture, parse then commit an empty `EditPlan`; the file on disk is byte-identical, verified by SHA-256 of the whole file.
- `structural_edit_preserves_everything_else`: for each fixture, edit one heading text, one link target, one fence body, and one frontmatter scalar; assert every other byte — including unknown syntax, comments, CRLF, quoting, and Obsidian constructs — is unchanged.
- `obsidian_directory_never_written`: run every F command and every structural edit against a vault with a populated `.obsidian`; assert `.obsidian` mtimes and bytes are unchanged, and that a write attempt through generic paths is rejected by `validate_note_path`.
- `multi_span_commit_is_atomic`: inject failure between splice construction and rename; assert the file is either wholly old or wholly new and never an intermediate state, and that no partial subset of the plan is observable.
- `stale_revision_rejects_plan`: change the source between span discovery and commit; assert `Error::Conflict` with expected/actual digests and zero mutation.
- `differential_oracle_agreement`: compare block boundaries against `markdown-rs` positions and YAML spans against `yaml-rust2` markers; any disagreement is a hard failure requiring an explicit documented divergence entry.
- `parser_upgrade_snapshot`: pin the render inventory and token cover for the whole corpus; a `pulldown-cmark` or profile bump must be an explicit reviewed snapshot change.
- `render_tier_semantic_equivalence`: render every fixture in all four tiers; assert the ordered `(kind, state, text_equivalent)` inventory is identical across tiers and equal to `--json`.
- `render_performs_no_writes_and_no_network`: run the corpus render under a filesystem shim that fails all writes and a syscall shim that fails all socket operations; every render succeeds.
- `render_never_reads_outside_the_vault`: confinement shim asserts every open is beneath the canonical root and follows no symlink out.
- `index_uses_the_shared_parser`: assert `mg-vault-index` derives its `PARSER_VERSION` from `SYNTAX_PROFILE` and contains no independent wikilink scanner; a profile change invalidates the published generation.
- `degraded_without_index`: with B stopped, media and embed resolution falls back to labeled `direct` probes or `unknown`; no result claims `resolved` without evidence and nothing claims index freshness.
- `large_document_mode`: a 200 MiB note renders windowed, `outline` works, whole-document structural operations return `document_too_large`, and RSS stays under the ceiling.
- `export_boundary_sanitizes`: drive the P export seam with the hostile corpus; assert output contains no disallowed element, attribute, scheme, comment, or `target`, and that `rel` is forced on every anchor.

### 5.3 UI / E2E tests

There is no graphical UI. E2E tests drive a pseudo-terminal:

1. `render` in a Kitty-graphics PTY, an xterm PTY, `TERM=dumb`, and a non-TTY pipe; assert the tier chosen, that images appear only in the graphics tier, and that the textual card is present in all four.
2. Terminal resize during a progressive render at 120 → 80 → 40 columns; assert no truncation of paths, IDs, states, or reasons, and that record-form kicks in at the documented thresholds.
3. A graphics-capable terminal that never answers the capability query; assert the 150 ms deadline fires and the render completes in `unicode`.
4. Preview-pane navigation through **E**: `]b`/`[b` across every block kind including error cards, `zc`/`zo` folding, `gd` into a resolved embed and into an unresolved one, `gr` revealing byte ranges, all keyboard-only.
5. Screen-reader linearization: capture `--plain` output for every fixture and assert every role word, state word, alt text or `description: unavailable`, node list, and edge list is present in reading order.
6. `NO_COLOR=1`, `--no-color`, `--plain`, `MG_VAULT_REDUCED_MOTION=1`, and redirected stdout: assert zero ANSI bytes and zero repaints.
7. `--json` and `--from-stdin` piped through `jq` and back; assert stable schema, stable ordering, and clean stdout/stderr separation.

### 5.4 Visual / manual verification

- Render the corpus at 40, 60, 80, and 120 columns in all four tiers.
- Light and dark terminal themes: confirm color is decoration only by diffing ANSI-stripped output between themes — it must be identical.
- Text-size extremes are N/A in a terminal; the equivalent width extremes are covered above. Verify with a 200-column ultrawide and a 30-column phone SSH session.
- Empty note, frontmatter-only note, single-emoji note, 5,000-row table, 20-level nested list, 10 MiB fence, and a note that is one 1 MiB line.
- Every media state and every embed state side by side on one page.
- Kitty, WezTerm, foot, Alacritty, xterm, `TERM=dumb`, and `tmux` inside each.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace --all-targets --all-features`.

---

## 6. Compliance & Safety Gate

### 6.1 Sensitive data classification

- [ ] No sensitive data involvement
- [x] **Handles sensitive data** — note bodies, headings, tags, properties, attachment filenames, and image/PDF content may all be private. Protections: F performs zero network I/O and never fetches a remote image, so reading an untrusted clipped note cannot beacon; the render cache is owner-only under XDG cache and keyed by fingerprint so it cannot outlive a change; diagnostics and error strings name rules and vault-relative paths but never note bodies, absolute host paths, or environment values; the interop export already excludes `private`, `.private`, `secrets`, `.secrets`, and F's export seam honors the same exclusions; no telemetry, no logging of content, and no query history exists.
- [ ] Uses synthetic/test data only until compliance gate clears

### 6.2 Asset provenance

- [ ] No third-party assets
- [x] **Uses third-party assets** —

| Asset | Source | License | Rights status |
|---|---|---|---|
| `pulldown-cmark` | crates.io | MIT | Permissive; compatible with either candidate project license |
| `yaml-edit` | crates.io (already a workspace dependency) | MIT/Apache-2.0 | Already vetted for the workspace |
| `unicode-segmentation`, `unicode-width` | crates.io | MIT/Apache-2.0 | Permissive |
| `image` (decode only, feature-gated) | crates.io | MIT/Apache-2.0 | Permissive; decoder-only usage |
| `markdown-rs`, `yaml-rust2` (dev-only oracles) | crates.io | MIT | Test-only; not shipped in the mutation path |
| Compatibility corpus fixtures | Authored for this project | Project license | **Must be original.** CommonMark and Obsidian example text may not be copied from upstream documentation; fixtures are written from the specified behavior, and the mermaid subset is implemented from the published grammar rather than by vendoring mermaid's own test suite |
| Unicode width tables | Unicode Consortium via `unicode-width` | Unicode license | Permissive; version pinned and reported |

No fonts, images, models, or trained data are bundled. The unresolved MIT-versus-Apache-2.0 project license decision (§8) gates only the LICENSE file, not these dependencies.

### 6.3 Language / claims audit

- [x] **No claim unsupported by evidence.** Every capability statement in §§1–5 describes target state; §7 separates it from what exists. `render --capabilities` reports detected facts, not aspirations.
- [x] **No promise of unbuilt capability in user-visible text.** A construct outside the supported surface says `unsupported`, never "coming soon". A resolution without evidence says `unknown`, never `resolved`. A diagram outside the subset says `diagram unavailable: <reason>`, never renders a blank and calls it success.
- [x] **No restricted-domain language.** F makes no medical, financial, legal, or safety claim. "Sanitized" is scoped to the versioned allowlist in §4.3 and is verified by the hostile corpus; it is never used to imply general safety against a malicious kernel or root process.
- [x] **"Preserved" is exact and testable:** byte-identical outside explicitly edited spans, asserted by prefix/suffix property tests, not by inspection.

### 6.4 Regulatory alignment

The template names Lens 3, but F crosses every lens, so each binding criterion is addressed:

| Criterion | How F addresses it |
|---|---|
| **1A Authority** | F never writes. The token cover, render model, and cache are derived and disposable; deleting the cache changes only latency. Source bytes are the only authority. |
| **1B Preservation** | Total gapless token cover with `Unrecognized` as a first-class kind; no serializer exists (compile- and API-golden-enforced); the only mutation is a fingerprint-bound disjoint splice; unknown syntax and unknown YAML survive any edit elsewhere in the file. |
| **1C Identity** | Paths remain identity. F reads block IDs and heading anchors but never injects a UUID, a `^blockid`, or any identifier into a note. |
| **1D Coexistence** | Wikilinks, embeds, callouts, tags, block refs, and `%%comments%%` are first-class token kinds with round-trip fixtures. `.obsidian` is read-only (`app.json` only, through a narrowly scoped capability) and is asserted byte-unchanged by test. Portable app state stays in `.mg-vault`. |
| **1E Transactions** | Overlapping, unsorted, stale, mid-codepoint, and hash-mismatched plans fail closed before I/O; multi-span commits are one atomic replacement, never a sequence of single splices; ambiguity in link, embed, or media resolution never picks a winner. |
| **2A Keyboard completeness** | Every preview action has a key and a `:render <action>` command; no pointer or graphics-only affordance exists. |
| **2B Editing durability** | F holds no durable editing state and emits no write; D's journal and core's atomic replacement own durability. F can never overwrite a newer source because every plan carries a revision that core revalidates. |
| **2C Workspace** | F supplies the preview-pane view model and the source/preview seam; E owns panes, tabs, splits, and session restore. |
| **2D Text correctness** | Grapheme-cluster wrapping, pinned width tables, UTF-8 boundary enforcement on every span, and width-never-affects-offsets. |
| **2E Degraded experience** | Missing graphics protocol, missing image feature, missing diagram feature, missing index, and missing confinement each degrade to an explicit labeled textual path; source reading and editing remain available in every case. |
| **3A Determinism** | One shared versioned parser for render, index, links, and refactors; ordering is source byte order everywhere; the same bytes always produce the same cover. |
| **3B Ambiguity** | `Ambiguous` is a terminal state with ordered candidates for links, embeds, headings, block IDs, and attachment filenames. Nothing is auto-selected and nothing is mutated. |
| **3C Query depth** | F emits byte-ranged structural projections — headings, blocks, block IDs, tags, properties, tasks, links, embeds, code, math, diagrams, media — which is the substrate B queries by text, title, regex, tag, property, path, relationship, task, and date. |
| **3D Derived authority** | The render model, outline, and cache are views over ordinary files; none can be edited to change a note, and none is consulted for source bytes. |
| **3E Scale** | Per-note parse and memory budgets are set so B's 100,000-note / 1,000,000-block corpus stays in budget; covers are dropped for closed notes; large documents degrade to windowed mode instead of blocking or OOM; every long operation is cancellable within 4 ms. |
| **4A Confinement** | Attachment reads use `openat2` with `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS`; `.obsidian`/`.mg-vault` are rejected for generic access; where confinement is unavailable, attachment reading is disabled rather than downgraded. |
| **4B Concurrency** | Every `TokenSpan` carries a revision, a lexeme hash, and a surrounding hash; core revalidates the fingerprint immediately before the atomic replacement. |
| **4C Least privilege** | Renderers are pure functions over immutable snapshots with no write, process, or socket capability. The mermaid and math renderers are in-process Rust with hard budgets — no JS runtime, no shell, no `eval`. Plugins and AI get render output, never a mutation authority. |
| **4D Recovery** | F is read-only, so there is nothing to roll back; the splice it proposes is atomic and previewable, and a failed validation leaves the file untouched. |
| **4E Contracts** | `mg-vault.syntax/1`, `mg-vault.html-allowlist/1`, and `mg.render/1` are explicit and versioned; a semantic change bumps the version and forces an index rebuild rather than silently changing meaning. |
| **4F Privacy** | Zero network egress, no remote image fetch, no telemetry, owner-only fingerprint-keyed cache, content excluded from diagnostics, and the existing private/secret directory exclusions honored at the export seam. |
| **5A Offline/local-first** | Every F workflow is fully local; remote URLs are rendered as inert link cards, never fetched. |
| **5B Responsiveness** | Explicit parse, render, incremental, memory, decode, layout, and cancellation budgets in §4.7; zero startup cost. |
| **5C Accessible equivalents** | `text_equivalent` is a non-optional field; images, diagrams, math, tables, and every error state carry complete textual modes; `description: unavailable` is literal and filenames are never promoted to descriptions. |
| **5D Terminal resilience** | Four-tier ladder down to pure ASCII and plain text, 40-column record form, literal state words, `NO_COLOR`, and reduced-motion handling. |
| **5E Automation** | Human, `--json` (`mg.render/1`), stdin input, stdout output, `--no-input`, `--no-color`, `--ascii`, `--plain`, and `NO_COLOR` are all supported and fixture-tested. |

**Auto-fail review.** Source-content loss: impossible by construction — F has no write path and no serializer, and preservation is asserted by prefix/suffix property tests. Partial multi-file mutation: F mutates nothing; the multi-span splice it proposes is single-file and atomic. Index state overriding source and stale index presented as current: resolution answers are labeled `indexed`/`direct`/`unknown` with B's freshness, and never supply note bytes. Silent conflict winner: every ambiguity is a terminal state with ordered candidates. **Unknown syntax loss: prevented by the total token cover, `TokenKind::Unrecognized`, the absent serializer, and the round-trip corpus gate.** Unconfirmed overwrite/import: F imports and overwrites nothing. Unsafe traversal or symlink escape: `openat2` confinement, with attachment reading disabled where unavailable. Capability/data-exfiltration bypass: no network, no process spawn, no `eval`, sanitizer-only output constructors. **Active raw HTML/script by default: prevented by deny-by-default `--html-mode literal`, the versioned allowlist, scheme denial, and the `SafeInline`/`SafeBlock` compile-time enforcement at every render and export boundary; source HTML is preserved but never re-written and never activated.** Non-atomic save claiming success: F never claims a save. Recovery overwriting newer source: F holds no recovery state. **Graph/Canvas information lacking a textual equivalent: prevented by the non-optional `text_equivalent` field, the diagram node/edge outline, and the four-tier semantic-equivalence test.**

---

## 7. Gap Analysis vs. Current State

### 7.1 What exists today

At commit `dfe33cf`:

- **implemented** — `crates/mg-vault-core/src/frontmatter.rs`: `scan_frontmatter` locates exact YAML and body byte ranges, requires an exact `---` first and closing line, reports `LineEndings::{Lf, CrLf, Mixed}` without normalizing, and fails closed on BOM and unclosed envelopes. Seven unit tests cover LF, CRLF, mixed, delimiter-like YAML values, EOF close, leading whitespace, and rejection cases.
- **implemented** — `crates/mg-vault-core/src/frontmatter_scalar.rs`: `locate_frontmatter_scalar` returns a source byte range for a top-level scalar. It parses the YAML with `yaml-edit`'s `Document` for validation and scalar type-checking, then re-locates the value with a line scanner over the original bytes so CRLF, comments, quoting, and spacing are untouched. It rejects missing frontmatter, missing keys, duplicate keys, non-scalar targets, and invalid YAML.
- **implemented** — `crates/mg-vault-core/src/vault.rs`: `edit_note_span` performs one fingerprint-checked, UTF-8-boundary-validated, checked-arithmetic, atomic single-span splice with exact prefix/suffix copying. `validate_note_path` rejects absolute, non-`Normal`, and non-`.md` paths and rejects `.obsidian`/`.mg-vault` first components for mutation.
- **implemented** — `yaml-edit 0.2.3` with `default-features = false` is already a workspace dependency in `vault/Cargo.toml`.
- **implemented** — `crates/mg-vault-core/src/index.rs`: `read_source_bytes` uses `openat2` with `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS` on Linux and refuses on other platforms; the walker skips `.obsidian` and `.mg-vault`. `crates/mg-vault-core/src/interop.rs` additionally skips `private`, `.private`, `secrets`, `.secrets`.
- **prototyped** — `index.rs::wikilinks` is a naive substring scan for `[[`…`]]` that takes the text before the first `|`. It has no code-span or fence exclusion, no embed/`![[`, heading, or block-anchor distinction, and produces no byte ranges. `index.rs::title_for` is a prototype that reads a `title` scalar then falls back to the first `# ` line, trimming quotes lexically.
- **planned** — `docs/spikes/token-preserving-markdown-yaml.md` records the accepted architecture: targeted source spans as the primary mutation model, `yaml-edit` as a span locator behind a project adapter, `pulldown-cmark::OffsetIter` for Markdown discovery, narrow lexical scanners for Obsidian constructs, Tree-sitter editor-only, one plan and one atomic replacement for multi-edits, and a prohibition on whole-document serializers. It also fixes the required fixture gates and the failure policy.
- **absent** — everything else in this spec: `pulldown-cmark` is not a dependency; there is no CommonMark or GFM parse, no token cover, no `TokenSpan`, no `EditPlan`, no multi-span splice, no Obsidian scanners for embeds/callouts/tags/block IDs/comments/highlights, no math or diagram handling, no transclusion, no attachment policy or resolution, no attachment read primitive (`validate_note_path` requires `.md`, so attachments cannot be read at all today), no HTML sanitizer, no renderer, no terminal capability detection, no `render`/`outline`/`media`/`transclude` commands, and no compatibility corpus.
- **absent** — `yaml-edit`'s `Scalar::byte_range` is not used; the current locator is a hand-rolled top-level line scan that handles only single-line plain scalars with trailing comments. Block scalars, flow collections, multi-line values, nested keys, and duplicate-occurrence selection are unsupported.
- **absent** — a LICENSE file; the MIT-versus-Apache-2.0 decision remains open, which gates fixture and dependency licensing paperwork but not implementation.

### 7.2 Delta to spec

**New crate:** `crates/mg-vault-markdown/` with the module layout in §4.1, plus `fixtures/` holding the versioned compatibility corpus (CommonMark 0.31.2, GFM, Obsidian, math, mermaid, malformed, hostile-HTML, Unicode, and line-ending families) and the parser-upgrade snapshots.

**New modules within it:** token cover and coverage invariant; the `OffsetIter` driver and gap filler; Obsidian lexical scanners; math delimiter policies; the YAML span adapter that finally uses `yaml-edit` byte ranges and adds block/flow/multi-line/duplicate-occurrence support; `TokenSpan`/`EditPlan` validation; transclusion; media policy and resolution; render model, sanitizer, terminal writer, math approximator, mermaid subset, and capability detection.

**Modified files:**
- `crates/mg-vault-core/src/vault.rs` — add `edit_note_spans` (disjoint multi-splice, one fingerprint, one atomic replace) and `read_attachment` with a non-`.md` sibling of `validate_note_path` that keeps confinement and protected-directory rules.
- `crates/mg-vault-core/src/frontmatter_scalar.rs` — migrate the locator to `yaml-edit` byte ranges behind the adapter; keep the existing public behavior and tests green.
- `crates/mg-vault-core/src/index.rs` — delete `wikilinks`; consume F's scanners. Keep `title_for` behavior but source it from F's heading and frontmatter tokens.
- `crates/mg-vault-index/src/lib.rs` — derive `PARSER_VERSION` from `SYNTAX_PROFILE`; a profile change must invalidate the published generation through the existing `validate_metadata` mismatch path.
- `crates/mg-vault-cli/src/main.rs` — add `render`, `outline`, `media`, and `transclude` subcommands and the new flags; extend the version-1 envelope with the `mg.render/1` data shape.
- `vault/Cargo.toml` — add `pulldown-cmark`, `unicode-segmentation`, `unicode-width`, feature-gated `image` and a graphics encoder, and dev-only `markdown-rs`, `yaml-rust2`, `trybuild`.
- `README.md`, `docs/PRODUCT.md`, `docs/ARCHITECTURE.md`, `docs/SECURITY.md` — record the new crate, the syntax/allowlist/render versions, and the sanitization boundary once shipped.

**Migrations / schema changes:** none to note content — F never rewrites a file. The only schema effect is that `SYNTAX_PROFILE` participates in B's parser-version check, so a profile bump triggers a side-by-side index rebuild through machinery that already exists.

### 7.3 Estimated scope

**XL.** The parse-side alone spans a lossless cover over a parser that is not itself total, a second scanner family for Obsidian syntax with strict code-context exclusion, a YAML span adapter that must handle every scalar style, and a differential-oracle test strategy. The render side adds a four-tier terminal ladder, a security-critical sanitizer with compile-time enforcement, a math approximator, a project-owned mermaid subset, attachment policy and confined binary reads, and transclusion with cycle and budget semantics. It is also the shared contract three other feature branches block on, so its public surface must be right before B, D, and H build on it.

It should ship as gated increments behind one profile version: **F1** token cover + coverage invariant + `TokenSpan`/`EditPlan` + corpus round-trip gate (unblocks D and H); **F2** Obsidian scanners + YAML span adapter + structural projections (unblocks B); **F3** render model + sanitizer + tier ladder + `render`/`outline`; **F4** media policy, resolution, and the graphics tier; **F5** transclusion; **F6** math and diagrams. F1 and F2 are the preservation-critical slices and must pass the full fixture gate before F3 begins.

### 7.4 Blocking dependencies

- **A Foundation and vault authority** — partially implemented and sufficient for F1/F2. `edit_note_spans` and `read_attachment` are the two primitives F needs added there; the descriptor-relative hardening A defers is required before the graphics/media slice (F4) may read attachments on any platform.
- **B Index service** — not a blocker for parsing or rendering. It is required for indexed shortest-path attachment resolution and for embed target lookup at scale; without it, F falls back to labeled `direct` probes with a documented latency cost.
- **G Links, search, and graph** — required before embed and wikilink resolution can report `resolved` with vault-wide authority. Until G lands, F resolves through the direct probe and reports `unknown` where it cannot prove a target, which is the specified degraded state, not a blocker for F1–F3.
- **D Editor engine** — a consumer, not a blocker. D's spec already names the `TokenSpan { revision, byte_range, token_kind, surrounding_hash }` contract as its A4/F prerequisite; F1 must ship that shape unchanged.
- **E TUI workspace** — a consumer. F supplies the preview view model; E owns pane placement and terminal input.
- **P Import, export, and publishing** — a consumer of `render/sanitize.rs`. P must not implement its own HTML policy; the allowlist version is shared.
- **External gate:** the MIT-versus-Apache-2.0 license decision blocks publishing the fixture corpus, not writing it.

---

## 8. Open Questions

- **Q1:** Which `$…$` delimiter policy is the default — `obsidian-dollar-v1` (whitespace-adjacency rules, matching documented Obsidian behavior, which classifies `$5 … $10` as math) or `strict-dollar-v1` (requires `$$` for display and rejects digit-adjacent inline runs, better for prose containing currency)? Both are implemented and both preserve bytes identically; only render classification differs. — blocks: §4.3 default configuration, §5.1 `math_policy_matrix` expectations.
- **Q2:** How large a mermaid subset does F1–F6 commit to? Proposed floor is `flowchart TD/LR` and `sequenceDiagram`; `classDiagram`, `stateDiagram-v2`, and `gantt` would each add layout work. Anything outside the committed subset falls back to verbatim source plus the node/edge outline, so this is a scope question, not a correctness one. — blocks: §4.5 mermaid parser scope, §4.7 diagram budget.
- **Q3:** When `.obsidian/app.json` and `.mg-vault/config.toml` both specify an attachment folder and disagree, does `.obsidian` win (maximum Obsidian fidelity) or `.mg-vault` win (mg-vault's own portable settings)? §4.3 currently proposes `.obsidian` first; either way `mg-vault media config` prints the full trace. — blocks: §4.3 precedence, §5.1 `attachment_policy_precedence`.
- **Q4:** What is P's default `--html-mode` on export — `literal` (escape everything, safest, but breaks intentional HTML in notes) or `sanitized` (allowlist applies)? F defaults to `literal` everywhere; P may need a different default for publishing. The allowlist itself is not in question. — blocks: §4.3 export boundary defaults.
- **Q5:** Should the render cache exist at all in the first release? It is the only durable artifact F introduces, and dropping it removes a class of staleness and privacy surface at a latency cost that the §4.7 budgets may already absorb. — blocks: §4.4 state management, §4.7 storage budget.
- **Q6:** Is SVG ever rasterized for the graphics tier, or permanently a placeholder card? Rasterizing means running an untrusted vector parser on note content; the safe default is never. — blocks: §4.3 SVG policy, §6.1 protections.
- **Q7:** MIT versus Apache-2.0 for the project. It does not block implementation but does block publishing the compatibility corpus and adding a LICENSE file, and it must be settled before any fixture is distributed. — blocks: §6.2 asset provenance.
