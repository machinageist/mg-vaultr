use std::fs;
use std::path::Path;

use mg_vault_core::Vault;
use mg_vault_index::{Freshness, PersistentIndexStore, SCHEMA_VERSION, StoreError};
use rusqlite::Connection;

fn paths(store: &PersistentIndexStore, query: &str) -> Vec<String> {
    store
        .search_current(query)
        .unwrap()
        .notes
        .iter()
        .map(|note| note.path.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn deleting_database_and_rebuilding_is_deterministic_and_source_safe() {
    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path()).unwrap();
    vault.create_note("z.md", b"needle z").unwrap();
    vault.create_note("a.md", b"needle a").unwrap();
    let expected_sources = ["a.md", "z.md"].map(|path| fs::read(root.path().join(path)).unwrap());
    let database = root.path().join("outside-vault-index.sqlite3");

    let first = {
        let mut store = PersistentIndexStore::open(&database).unwrap();
        let metadata = store.rebuild(&vault).unwrap();
        assert_eq!(metadata.generation, Some(1));
        paths(&store, "needle")
    };
    fs::remove_file(&database).unwrap();
    let mut rebuilt = PersistentIndexStore::open(&database).unwrap();
    let metadata = rebuilt.rebuild(&vault).unwrap();

    assert_eq!(metadata.generation, Some(1));
    assert_eq!(first, ["a.md", "z.md"]);
    assert_eq!(paths(&rebuilt, "needle"), first);
    assert_eq!(
        ["a.md", "z.md"].map(|path| fs::read(root.path().join(path)).unwrap()),
        expected_sources
    );
}

#[test]
fn failed_publication_keeps_previous_generation_and_rows_visible() {
    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path()).unwrap();
    vault.create_note("note.md", b"old value").unwrap();
    let database = root.path().join("index.sqlite3");
    let mut store = PersistentIndexStore::open(&database).unwrap();
    store.rebuild(&vault).unwrap();
    fs::write(root.path().join("note.md"), b"candidate value").unwrap();

    let error = store
        .rebuild_with_publication_guard(&vault, || Err("simulated interruption".to_owned()))
        .unwrap_err();
    assert!(error.to_string().contains("simulated interruption"));
    drop(store);

    let reopened = PersistentIndexStore::open(&database).unwrap();
    let metadata = reopened.metadata().unwrap();
    assert_eq!(metadata.generation, Some(1));
    assert_eq!(metadata.freshness, Freshness::Degraded);
    assert!(matches!(
        reopened.search_current("old value"),
        Err(StoreError::NotCurrent {
            freshness: Freshness::Degraded
        })
    ));
    let published = reopened.published_snapshot().unwrap();
    assert_eq!(published.metadata.freshness, Freshness::Degraded);
    assert_eq!(published.notes[0].path, Path::new("note.md"));
    assert!(published.notes[0].text.contains("old value"));
    assert!(!published.notes[0].text.contains("candidate value"));
}

#[test]
fn rebuild_replaces_changed_and_deleted_sources_without_stale_rows() {
    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path()).unwrap();
    vault.create_note("changed.md", b"before").unwrap();
    vault.create_note("deleted.md", b"obsolete").unwrap();
    let mut store = PersistentIndexStore::open(root.path().join("index.sqlite3")).unwrap();
    store.rebuild(&vault).unwrap();

    fs::write(root.path().join("changed.md"), b"after").unwrap();
    fs::remove_file(root.path().join("deleted.md")).unwrap();
    let metadata = store.rebuild(&vault).unwrap();

    assert_eq!(metadata.generation, Some(2));
    assert_eq!(paths(&store, "after"), ["changed.md"]);
    assert!(paths(&store, "before").is_empty());
    assert!(paths(&store, "obsolete").is_empty());
    assert!(
        store
            .published_snapshot()
            .unwrap()
            .notes
            .iter()
            .all(|note| note.path != Path::new("deleted.md"))
    );
}

#[test]
fn query_order_is_total_and_stable_across_reopen_and_rebuild() {
    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path()).unwrap();
    for path in ["b.md", "nested/c.md", "a.md"] {
        vault.create_note(path, b"same score needle").unwrap();
    }
    let database = root.path().join("index.sqlite3");
    let expected = ["a.md", "b.md", "nested/c.md"];
    {
        let mut store = PersistentIndexStore::open(&database).unwrap();
        store.rebuild(&vault).unwrap();
        assert_eq!(paths(&store, "needle"), expected);
    }
    let mut reopened = PersistentIndexStore::open(&database).unwrap();
    assert_eq!(paths(&reopened, "needle"), expected);
    reopened.rebuild(&vault).unwrap();
    assert_eq!(paths(&reopened, "needle"), expected);
}

#[test]
fn malformed_utf8_is_diagnostic_and_does_not_publish_partial_generation() {
    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path()).unwrap();
    vault.create_note("good.md", b"published value").unwrap();
    let database = root.path().join("index.sqlite3");
    let mut store = PersistentIndexStore::open(&database).unwrap();
    store.rebuild(&vault).unwrap();
    vault.create_note("bad.md", b"broken \xff bytes").unwrap();

    let metadata = store.rebuild(&vault).unwrap();

    assert_eq!(metadata.generation, Some(1));
    assert_eq!(metadata.freshness, Freshness::Degraded);
    assert_eq!(metadata.diagnostics.len(), 1);
    assert_eq!(metadata.diagnostics[0].path, Path::new("bad.md"));
    assert!(metadata.diagnostics[0].message.contains("utf-8"));
    assert!(matches!(
        store.search_current("published value"),
        Err(StoreError::NotCurrent {
            freshness: Freshness::Degraded
        })
    ));
    let published = store.published_snapshot().unwrap();
    assert_eq!(published.metadata.freshness, Freshness::Degraded);
    assert_eq!(published.notes[0].path, Path::new("good.md"));
    assert!(
        published
            .notes
            .iter()
            .all(|note| note.path != Path::new("bad.md"))
    );
}

#[test]
fn schema_parser_and_freshness_metadata_are_explicit() {
    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path()).unwrap();
    let mut store = PersistentIndexStore::open(root.path().join("index.sqlite3")).unwrap();
    let empty = store.metadata().unwrap();
    assert_eq!(empty.schema_version, SCHEMA_VERSION);
    assert_eq!(empty.freshness, Freshness::Empty);
    assert!(!empty.parser_version.is_empty());

    let current = store.rebuild(&vault).unwrap();
    assert_eq!(current.freshness, Freshness::Current);
    assert_eq!(current.note_count, 0);
    assert!(current.completed_at_unix_ms.is_some());
    assert_eq!(store.sqlite_user_version().unwrap(), SCHEMA_VERSION);
}

#[test]
fn partial_unversioned_schema_fails_closed() {
    let root = tempfile::tempdir().unwrap();
    let database = root.path().join("partial.sqlite3");
    Connection::open(&database)
        .unwrap()
        .execute("CREATE TABLE orphaned_partial_table (id INTEGER)", [])
        .unwrap();

    assert!(matches!(
        PersistentIndexStore::open(&database),
        Err(StoreError::IncompleteSchema)
    ));
}

#[test]
fn parser_version_mismatch_fails_closed() {
    let root = tempfile::tempdir().unwrap();
    let database = root.path().join("parser-mismatch.sqlite3");
    PersistentIndexStore::open(&database).unwrap();
    Connection::open(&database)
        .unwrap()
        .execute(
            "UPDATE metadata SET parser_version = 'incompatible-parser' WHERE singleton = 1",
            [],
        )
        .unwrap();

    assert!(matches!(
        PersistentIndexStore::open(&database),
        Err(StoreError::UnsupportedParser { .. })
    ));
}

#[test]
fn current_metadata_without_generation_fails_closed() {
    let root = tempfile::tempdir().unwrap();
    let database = root.path().join("invalid-current.sqlite3");
    PersistentIndexStore::open(&database).unwrap();
    Connection::open(&database)
        .unwrap()
        .execute(
            "UPDATE metadata SET freshness = 'current', generation = NULL WHERE singleton = 1",
            [],
        )
        .unwrap();

    assert!(matches!(
        PersistentIndexStore::open(&database),
        Err(StoreError::InvalidMetadata(message)) if message.contains("missing its generation")
    ));
}

#[test]
fn signed_sqlite_generation_limit_fails_with_typed_error() {
    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path()).unwrap();
    vault.create_note("note.md", b"value").unwrap();
    let database = root.path().join("index.sqlite3");
    {
        let mut store = PersistentIndexStore::open(&database).unwrap();
        store.rebuild(&vault).unwrap();
    }
    Connection::open(&database)
        .unwrap()
        .execute(
            "UPDATE metadata SET generation = ?1 WHERE singleton = 1",
            [i64::MAX],
        )
        .unwrap();
    let mut store = PersistentIndexStore::open(&database).unwrap();

    assert!(matches!(
        store.rebuild(&vault),
        Err(StoreError::GenerationExhausted)
    ));
    assert_eq!(store.metadata().unwrap().generation, Some(i64::MAX as u64));
}
