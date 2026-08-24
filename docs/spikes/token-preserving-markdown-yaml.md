# Token-preserving Markdown and YAML strategy

## Decision

Original note bytes are authoritative. Parsers may discover and validate UTF-8
byte spans, but they may not serialize an entire note or frontmatter block back
to storage. Every mutation is represented as a source edit bound to a
`SourceFingerprint` and committed through the atomic file-authority boundary.

This makes preservation mechanical: bytes outside declared replacement spans
are copied unchanged.

## Staged architecture

1. `Vault::edit_note_span` performs one fingerprint-checked atomic splice.
2. `scan_frontmatter` locates exact delimiter, YAML, and Markdown-body ranges
   without normalizing LF, CRLF, or mixed line endings. Ambiguous envelopes fail
   closed.
3. A project-owned YAML adapter will use `yaml-edit` only to locate scalar spans.
   Replacement still occurs against original bytes. Candidate YAML must be
   reparsed before commit.
4. Markdown discovery will use `pulldown-cmark::OffsetIter` where supported and
   narrow lexical scanners for Obsidian constructs not represented faithfully.
   Parser output must match the expected source slice before authorizing an edit.
5. Tree-sitter remains editor-only for highlighting and navigation; it is not
   mutation authority.
6. Multi-edit operations require one plan, one fingerprint check, overlap
   rejection, and one atomic replacement. Repeated single-span commits are not a
   substitute because they expose intermediate states.

## Decision matrix

| Approach | Preservation | Role |
|---|---|---|
| Targeted source spans | Exact outside edited ranges | Primary mutation architecture |
| `yaml-edit`/Rowan | Lossless YAML CST and scalar byte ranges | YAML span locator behind an adapter |
| `yaml-rust2` | Semantic YAML parser with byte markers | Independent validation oracle |
| `pulldown-cmark::OffsetIter` | Markdown event source ranges | Markdown discovery/indexing |
| `markdown-rs` positions | Byte-accounted positional AST | Secondary oracle/spike candidate |
| Tree-sitter Markdown | Incremental byte ranges but known grammar inaccuracies | Editor highlighting/navigation only |
| `serde_yaml` | Does not retain comments, quoting, order, or whitespace; unmaintained | Rejected for source edits |
| Whole-document serializer | Can normalize unknown syntax and formatting | Prohibited |

## Binding failure policy

- A UTF-8 BOM before frontmatter is currently rejected for structural edits.
- Frontmatter must begin with an exact `---` first line and end with an exact
  `---` line.
- Leading whitespace, delimiter-like content, and unclosed envelopes are never
  guessed into editable YAML.
- Mixed line endings are reported and preserved, not normalized.
- Duplicate YAML target keys require explicit occurrence selection.
- Entry removal with ambiguous comment ownership is refused until fixtures
  establish a policy.
- Parser errors, stale fingerprints, overlapping ranges, invalid UTF-8
  boundaries, and source/lexeme mismatches abort without mutation.

## Required fixture gates

- LF, CRLF, mixed endings, missing final newline, BOM, and unclosed envelopes.
- Unicode keys/values, combining marks, emoji, and mid-codepoint rejection.
- Plain, quoted, flow, and block scalar styles.
- Comments, blank lines, indentation, anchors, aliases, tags, duplicate and
  complex keys, directives, and malformed YAML.
- Wikilinks, embeds, heading/block links, callouts, comments, tags, math,
  Dataview/plugin blocks, and misleading syntax inside code spans/fences.
- Property/fuzz tests proving prefix/suffix equality and panic freedom.
- Parser-upgrade snapshots over the compatibility corpus.
- A synthetic 100,000-note benchmark reporting throughput, peak RSS, and
  p50/p95/p99 parse latency; parsers/CSTs are not retained for closed notes.

## Evidence

- `pulldown-cmark::OffsetIter` source ranges:
  https://docs.rs/pulldown-cmark/latest/pulldown_cmark/struct.OffsetIter.html
- `pulldown-cmark` parser options:
  https://docs.rs/pulldown-cmark/latest/pulldown_cmark/struct.Options.html
- `markdown-rs` positional tokens:
  https://github.com/wooorm/markdown-rs
- `yaml-edit` lossless CST:
  https://docs.rs/yaml-edit/latest/yaml_edit/
- `yaml-edit::Scalar::byte_range`:
  https://docs.rs/yaml-edit/latest/yaml_edit/struct.Scalar.html
- `yaml-rust2` markers:
  https://docs.rs/yaml-rust2/latest/yaml_rust2/scanner/struct.Marker.html
- `serde_yaml` maintenance status:
  https://docs.rs/serde_yaml/latest/serde_yaml/
- Rowan byte offsets:
  https://docs.rs/rowan/latest/rowan/struct.TextSize.html
- Tree-sitter positions and incremental parsing:
  https://tree-sitter.github.io/tree-sitter/using-parsers/2-basic-parsing.html
  and https://tree-sitter.github.io/tree-sitter/using-parsers/3-advanced-parsing.html
- Tree-sitter Markdown grammar warning:
  https://github.com/tree-sitter-grammars/tree-sitter-markdown

## Current evidence

At commit `4e2ebfc`, `Vault::edit_note_span` already enforces UTF-8 boundaries,
source fingerprints, checked allocation arithmetic, exact prefix/suffix copying,
and durable atomic replacement. The frontmatter-envelope scanner is the first
planner-layer implementation built on that invariant.
