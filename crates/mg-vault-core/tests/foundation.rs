use std::fs;

use mg_vault_core::{Error, SourceFingerprint, Vault, VaultRegistry, XdgPaths};
use tempfile::TempDir;

fn vault() -> (TempDir, Vault) {
    let dir = tempfile::tempdir().unwrap();
    let vault = Vault::open(dir.path()).unwrap();
    (dir, vault)
}

#[test]
fn xdg_paths_honor_overrides() {
    let home = tempfile::tempdir().unwrap();
    let paths = XdgPaths::from_values(
        home.path(),
        Some(home.path().join("cfg")),
        Some(home.path().join("data")),
        Some(home.path().join("state")),
        Some(home.path().join("cache")),
    );
    assert_eq!(paths.config_dir(), home.path().join("cfg/mg-vault"));
    assert_eq!(paths.data_dir(), home.path().join("data/mg-vault"));
    assert_eq!(paths.state_dir(), home.path().join("state/mg-vault"));
    assert_eq!(paths.cache_dir(), home.path().join("cache/mg-vault"));
}

#[test]
fn registry_round_trips_and_selects_canonical_vault() {
    let root = tempfile::tempdir().unwrap();
    let vault_dir = root.path().join("vault");
    fs::create_dir(&vault_dir).unwrap();
    let registry_path = root.path().join("config/registry.json");
    let mut registry = VaultRegistry::load(&registry_path).unwrap();
    registry.register("work", &vault_dir).unwrap();
    registry.select("work").unwrap();

    let loaded = VaultRegistry::load(&registry_path).unwrap();
    assert_eq!(loaded.selected_name(), Some("work"));
    assert_eq!(
        loaded.resolve(None).unwrap(),
        fs::canonicalize(vault_dir).unwrap()
    );
}

#[test]
fn rejects_traversal_absolute_and_protected_mutations() {
    let (_dir, vault) = vault();
    for path in [
        "../escape.md",
        "/tmp/escape.md",
        ".obsidian/config.md",
        ".mg-vault/config.md",
    ] {
        assert!(matches!(
            vault.create_note(path, b"x"),
            Err(Error::UnsafePath(_))
        ));
    }
}

#[cfg(unix)]
#[test]
fn rejects_symlink_escape() {
    use std::os::unix::fs::symlink;
    let (_dir, vault) = vault();
    let outside = tempfile::tempdir().unwrap();
    symlink(outside.path(), vault.root().join("linked")).unwrap();
    assert!(matches!(
        vault.create_note("linked/escape.md", b"x"),
        Err(Error::UnsafePath(_))
    ));
}

#[test]
fn create_refuses_collision_and_read_is_byte_exact() {
    let (_dir, vault) = vault();
    let first = vault
        .create_note("notes/a.md", b"# A\nunknown [[syntax]]\n")
        .unwrap();
    assert!(matches!(
        vault.create_note("notes/a.md", b"changed"),
        Err(Error::Collision(_))
    ));
    let read = vault.read_note("notes/a.md").unwrap();
    assert_eq!(read.bytes, b"# A\nunknown [[syntax]]\n");
    assert_eq!(read.fingerprint, first);
}

#[test]
fn optimistic_write_rejects_changed_source() {
    let (_dir, vault) = vault();
    let fingerprint = vault.create_note("a.md", b"one").unwrap();
    fs::write(vault.root().join("a.md"), b"external").unwrap();
    assert!(matches!(
        vault.write_note("a.md", b"ours", &fingerprint),
        Err(Error::Conflict { .. })
    ));
    assert_eq!(fs::read(vault.root().join("a.md")).unwrap(), b"external");
}

#[test]
fn atomic_write_changes_fingerprint_and_content() {
    let (_dir, vault) = vault();
    let before = vault.create_note("a.md", b"one").unwrap();
    let after = vault.write_note("a.md", b"two", &before).unwrap();
    assert_ne!(before, after);
    assert_eq!(after, SourceFingerprint::of(b"two"));
    assert_eq!(fs::read(vault.root().join("a.md")).unwrap(), b"two");
}

#[test]
fn narrow_edit_preserves_all_bytes_outside_the_selected_span() {
    let (_dir, vault) = vault();
    let source = b"---\ntitle: Old\nunknown:  keep  spacing\n---\nBefore [[Target]] after\n";
    let before = vault.create_note("a.md", source).unwrap();
    let start = source
        .windows(b"Old".len())
        .position(|window| window == b"Old")
        .unwrap();
    let end = start + b"Old".len();

    let after = vault
        .edit_note_span("a.md", start..end, b"New title", &before)
        .unwrap();

    let expected =
        b"---\ntitle: New title\nunknown:  keep  spacing\n---\nBefore [[Target]] after\n";
    assert_eq!(fs::read(vault.root().join("a.md")).unwrap(), expected);
    assert_eq!(after, SourceFingerprint::of(expected));
}

#[test]
fn narrow_edit_rejects_invalid_utf8_boundaries_without_mutation() {
    let (_dir, vault) = vault();
    let source = "before café after".as_bytes();
    let before = vault.create_note("a.md", source).unwrap();
    let inside_multibyte_character = "before caf".len() + 1;

    assert!(matches!(
        vault.edit_note_span(
            "a.md",
            inside_multibyte_character..inside_multibyte_character + 1,
            b"x",
            &before,
        ),
        Err(Error::InvalidEditSpan { .. })
    ));
    assert_eq!(fs::read(vault.root().join("a.md")).unwrap(), source);
}

#[test]
fn narrow_edit_rejects_reversed_and_out_of_bounds_spans_without_mutation() {
    let (_dir, vault) = vault();
    let source = b"unchanged";
    let before = vault.create_note("a.md", source).unwrap();

    let reversed_start = source.len() - 1;
    let reversed_end = 3;
    for span in [reversed_start..reversed_end, 0..source.len() + 1] {
        assert!(matches!(
            vault.edit_note_span("a.md", span, b"x", &before),
            Err(Error::InvalidEditSpan { .. })
        ));
        assert_eq!(fs::read(vault.root().join("a.md")).unwrap(), source);
    }
}

#[test]
fn narrow_edit_rejects_invalid_existing_utf8_without_mutation() {
    let (_dir, vault) = vault();
    let source = b"before\xffafter";
    let before = vault.create_note("a.md", source).unwrap();

    assert!(matches!(
        vault.edit_note_span("a.md", 0..1, b"x", &before),
        Err(Error::InvalidUtf8(_))
    ));
    assert_eq!(fs::read(vault.root().join("a.md")).unwrap(), source);
}

#[test]
fn narrow_edit_rejects_invalid_utf8_replacement_without_mutation() {
    let (_dir, vault) = vault();
    let source = b"unchanged";
    let before = vault.create_note("a.md", source).unwrap();

    assert!(matches!(
        vault.edit_note_span("a.md", 0..1, b"\xff", &before),
        Err(Error::InvalidUtf8(_))
    ));
    assert_eq!(fs::read(vault.root().join("a.md")).unwrap(), source);
}

#[test]
fn narrow_edit_rejects_a_stale_fingerprint_without_mutation() {
    let (_dir, vault) = vault();
    let before = vault.create_note("a.md", b"original").unwrap();
    fs::write(vault.root().join("a.md"), b"external").unwrap();

    assert!(matches!(
        vault.edit_note_span("a.md", 0..8, b"ours", &before),
        Err(Error::Conflict { .. })
    ));
    assert_eq!(fs::read(vault.root().join("a.md")).unwrap(), b"external");
}

#[test]
fn trash_restore_round_trip_and_collision_refusal() {
    let (_dir, vault) = vault();
    vault.create_note("folder/a.md", b"recover me").unwrap();
    let receipt = vault.trash_note("folder/a.md").unwrap();
    assert!(!vault.root().join("folder/a.md").exists());
    fs::create_dir_all(vault.root().join("folder")).unwrap();
    fs::write(vault.root().join("folder/a.md"), b"collision").unwrap();
    assert!(matches!(
        vault.restore_note(&receipt.id),
        Err(Error::Collision(_))
    ));
    fs::remove_file(vault.root().join("folder/a.md")).unwrap();
    vault.restore_note(&receipt.id).unwrap();
    assert_eq!(
        fs::read(vault.root().join("folder/a.md")).unwrap(),
        b"recover me"
    );
}
