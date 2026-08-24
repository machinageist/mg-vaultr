# Product contract

`mg-vault` is a local-first, keyboard-driven Rust knowledge system. Ordinary Markdown and attachment files remain user-owned authority; paths are public note identity; no hidden UUID is injected. A future SQLite index is disposable and must never override source. Vaults remain usable without this application and may coexist with Obsidian.

The target product includes a TUI/editor, token-preserving Obsidian-compatible Markdown, search/index service, links/refactors, properties/Bases, Canvas, periodic notes, Git/Syncthing workflows, sandboxed plugins, explicit AI adapters, import/export, and Quickshell integration. Those are target-state commitments, not claims about this foundation implementation.

The current accepted implementation slice is only file authority: XDG locations, registered vault selection, confined paths, durable note I/O, fingerprints, and recoverable trash/restore with stable CLI output.
