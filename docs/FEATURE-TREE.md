# mg-vault — Confirmed Feature Tree

Binding source: accepted plan and prior interview. This file records the confirmed target tree; only **A foundation/file authority** is in the current implementation slice.

- **A. Foundation and vault authority** — XDG/registry; `.mg-vault`; path identity/fingerprints; token preservation; atomic transactions; trash/history/recovery; error/JSON contracts.
- **B. Index service** — lifecycle/IPC; watching; incremental rebuild; SQLite; structural projections; provenance; degraded freshness; scale.
- **C. CLI and vault operations** — vault management; create/read/append/edit/move/refactor; pipelines; collisions; import/export reports.
- **G. Links, search, and graph** — deterministic resolution; relationships; mentions; full/structured search; saved queries; graph and textual view.
- **H. Safe note refactoring** — atomic previewable rename/move/extract/split/merge/section operations and rollback.
- **I. Properties, schemas, and Bases** — typed YAML; schema migration; compatible Bases; views; formulas/relations/rollups; cycle safety.
- **J. Periodic notes, templates, and tasks** — periods; safe templates; Markdown tasks; explicit `mg-calr` promotion.
- **K. Canvas** — JSON Canvas preservation; spatial and textual editing; embeds; unknown properties.
- **L. Capture, clipping, and extraction** — inbox/daily capture; handoff; web provenance/sanitization; PDF/OCR.
- **M. Git, filesystem sync, and recovery** — status/history/checkpoints; explicit push/pull; Syncthing conflicts; merge; backup; secret exclusions.
- **P. Import, export, and publishing** — dry-run provenance-aware import; Markdown/HTML/PDF export; staged allowlisted publishing.
- **Q. Quickshell and suite integration** — stable contracts; pill/card/keybinds; deep links; `mg-calr`; package suite.
- **R. Packaging and developer experience** — Arch/systemd; releases/checksums; docs/completions; examples/SDK; clean-machine and fixture CI.

Editor, TUI workspace, rich-content, plugins, and AI-adapter branches were deliberately pruned; their specs remain recoverable from Git history.
