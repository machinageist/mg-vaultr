# Spec status

**This tracks specification authoring, not implementation.** Most branches below
are specified and absent. `../README.md` is the authority on what is actually built
— today that is branch A in full, plus four narrow slices of branch B.

Branch letters come from [FEATURE-TREE.md](FEATURE-TREE.md). A branch is `authored`
once its spec states binding scope; `reviewed` once a blind scorecard exists under
[reviews/](reviews/). Neither implies code.

| Branch | Name | Spec | Blind review | Built |
|---|---|---|---|---|
| A | Foundation and file authority | [authored](specs/a-foundation-file-authority.md) | [reviewed](reviews/a-foundation-file-authority.md) | yes |
| B | Index, service, search | [authored](specs/b-index-service-search.md) | [reviewed](reviews/b-index-service-search.md) | slices only |
| B0 | Markdown index foundation | [authored](specs/b0-markdown-index-foundation.md) | — | yes |
| B1 | CLI index and search | [authored](specs/b1-cli-index-search.md) | — | yes |
| B2 | Local IPC health slice | [authored](specs/b2-local-ipc-health-slice.md) | — | health ping only |
| B3–B4 | Persistent index store | [authored](specs/b3-b4-persistent-index-store-slice.md) | — | yes |
| C | CLI note operations | [authored](specs/c-cli-note-operations.md) | [reviewed](reviews/c-cli-note-operations.md) | no |
| G | Links, search, graph | [authored](specs/g-links-search-graph.md) | [reviewed](reviews/g-links-search-graph.md) | no |
| H | Safe note refactoring | [authored](specs/h-safe-note-refactoring.md) | — | no |
| I | Properties, schemas, bases | [authored](specs/i-properties-schemas-bases.md) | — | no |
| J | Periodic notes, templates, tasks | [authored](specs/j-periodic-templates-tasks.md) | — | no |
| K | Canvas | [authored](specs/k-canvas.md) | — | no |
| L | Capture, clipping, extraction | [authored](specs/l-capture-clipping-extraction.md) | — | no |
| M | Git sync and recovery | [authored](specs/m-git-sync-recovery.md) | — | no |
| P | Import, export, publishing | [authored](specs/p-import-export-publishing.md) | — | no |
| Q | Quickshell suite integration | [authored](specs/q-quickshell-suite-integration.md) | — | no |
| R | Packaging and devex | [authored](specs/r-packaging-devex.md) | — | no |

Quality bar these were written against: [QUALITY-CRITERIA.md](specs/QUALITY-CRITERIA.md).

Eleven of the seventeen remaining target rows are specified and unbuilt. That is deliberate — the
specs are target state written ahead of implementation — but it does mean the volume
of specification here is not a claim about the software.
