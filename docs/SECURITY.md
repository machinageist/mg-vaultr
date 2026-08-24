# Security boundary

All caller-provided note paths must be relative, free of `..`, and resolve beneath the canonical registered vault root. Existing path resolution canonicalizes the target; creation canonicalizes the parent. This rejects traversal and symlink escape. Generic mutation also rejects any path whose first component is `.obsidian` or `.mg-vault`.

Note creation uses `create_new` semantics and refuses collisions. Existing-note replacement requires the caller's prior SHA-256 fingerprint and fails closed if bytes changed. Atomic replacement writes a unique same-directory temporary file, syncs it, renames it, and syncs the parent directory.

Trash data is local to `.mg-vault/trash`. Dedicated trash/restore code validates metadata paths through the same public path authority and refuses restore collisions. Permanent purge is not implemented.

This is a foundation, not a complete hostile-filesystem sandbox. Operations defend against path state at each use boundary, but Linux directory-entry races would require descriptor-relative APIs (`openat2`/capability directories) in a later hardening slice.
