use std::fs;
use std::path::Path;
use std::sync::mpsc;
use std::thread;

use mg_vault_core::Vault;
use mg_vault_index::{Freshness, PersistentIndexStore, SCHEMA_VERSION, StoreError};
use rusqlite::{Connection, ErrorCode};

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
fn freshness_verification_detects_added_changed_and_removed_sources() {
    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path()).unwrap();
    vault.create_note("changed.md", b"before").unwrap();
    vault.create_note("removed.md", b"removed").unwrap();
    let mut store = PersistentIndexStore::open(root.path().join("index.sqlite3")).unwrap();
    store.rebuild(&vault).unwrap();

    fs::write(root.path().join("changed.md"), b"after").unwrap();
    fs::remove_file(root.path().join("removed.md")).unwrap();
    vault.create_note("added.md", b"added").unwrap();

    let metadata = store.verify_freshness(&vault).unwrap();

    assert_eq!(metadata.generation, Some(1));
    assert_eq!(metadata.freshness, Freshness::Stale);
    assert_eq!(
        metadata
            .diagnostics
            .iter()
            .map(|diagnostic| (diagnostic.path.as_path(), diagnostic.message.as_str()))
            .collect::<Vec<_>>(),
        [
            (
                Path::new("added.md"),
                "source added after published generation"
            ),
            (
                Path::new("changed.md"),
                "source changed after published generation"
            ),
            (
                Path::new("removed.md"),
                "source removed after published generation"
            ),
        ]
    );
    assert!(matches!(
        store.search_current("before"),
        Err(StoreError::NotCurrent {
            freshness: Freshness::Stale
        })
    ));
    let published = store.published_snapshot().unwrap();
    assert!(published.notes.iter().any(|note| note.text == "before"));
    assert!(published.notes.iter().any(|note| note.text == "removed"));
    assert!(
        published
            .notes
            .iter()
            .all(|note| note.path != Path::new("added.md"))
    );
}

#[test]
fn freshness_verification_is_observational_and_preserves_current_generation() {
    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path()).unwrap();
    vault.create_note("note.md", b"same bytes").unwrap();
    let mut store = PersistentIndexStore::open(root.path().join("index.sqlite3")).unwrap();
    let rebuilt = store.rebuild(&vault).unwrap();

    let verified = store.verify_freshness(&vault).unwrap();

    assert_eq!(verified.freshness, Freshness::Current);
    assert_eq!(verified.generation, rebuilt.generation);
    assert_eq!(verified.completed_at_unix_ms, rebuilt.completed_at_unix_ms);
    assert!(verified.observed_at_unix_ms >= rebuilt.observed_at_unix_ms);
    assert!(verified.diagnostics.is_empty());
}

#[test]
fn complete_matching_observation_recovers_degraded_generation() {
    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path()).unwrap();
    vault.create_note("note.md", b"unchanged").unwrap();
    let mut store = PersistentIndexStore::open(root.path().join("index.sqlite3")).unwrap();
    store.rebuild(&vault).unwrap();
    store
        .rebuild_with_publication_guard(&vault, || Err("temporary failure".to_owned()))
        .unwrap_err();
    assert_eq!(store.metadata().unwrap().freshness, Freshness::Degraded);

    let recovered = store.verify_freshness(&vault).unwrap();

    assert_eq!(recovered.generation, Some(1));
    assert_eq!(recovered.freshness, Freshness::Current);
    assert!(recovered.diagnostics.is_empty());
    assert_eq!(paths(&store, "unchanged"), ["note.md"]);
}

#[test]
fn freshness_verification_refuses_a_source_that_changes_between_observations() {
    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path()).unwrap();
    vault.create_note("note.md", b"published").unwrap();
    let mut store = PersistentIndexStore::open(root.path().join("index.sqlite3")).unwrap();
    store.rebuild(&vault).unwrap();

    let metadata = store
        .verify_freshness_with_observation_guard(&vault, || {
            fs::write(root.path().join("note.md"), b"changed during observation").unwrap();
        })
        .unwrap();

    assert_eq!(metadata.generation, Some(1));
    assert_eq!(metadata.freshness, Freshness::Degraded);
    assert!(metadata.diagnostics.iter().any(|diagnostic| {
        diagnostic
            .message
            .contains("source changed during freshness verification")
    }));
    assert!(matches!(
        store.search_current("published"),
        Err(StoreError::NotCurrent {
            freshness: Freshness::Degraded
        })
    ));
}

#[test]
fn freshness_certification_does_not_overwrite_a_concurrently_published_generation() {
    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path()).unwrap();
    vault.create_note("note.md", b"generation one").unwrap();
    let database = root.path().join("index.sqlite3");
    let mut verifier = PersistentIndexStore::open(&database).unwrap();
    verifier.rebuild(&vault).unwrap();
    let mut publisher = PersistentIndexStore::open(&database).unwrap();

    let error = verifier
        .verify_freshness_with_guards(
            &vault,
            || {},
            || {
                fs::write(root.path().join("note.md"), b"generation two").unwrap();
                publisher.rebuild(&vault).unwrap();
                fs::write(root.path().join("note.md"), b"drift after generation two").unwrap();
                let stale = publisher.verify_freshness(&vault).unwrap();
                assert_eq!(stale.generation, Some(2));
                assert_eq!(stale.freshness, Freshness::Stale);
            },
        )
        .unwrap_err();

    assert!(matches!(
        error,
        StoreError::FreshnessGenerationChanged {
            expected_generation: Some(1)
        }
    ));
    let metadata = verifier.metadata().unwrap();
    assert_eq!(metadata.generation, Some(2));
    assert_eq!(metadata.freshness, Freshness::Stale);
    assert_eq!(metadata.diagnostics[0].path, Path::new("note.md"));
}

#[test]
fn overlapping_rebuilds_publish_unique_generations_and_invalidate_prior_certification() {
    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path()).unwrap();
    vault.create_note("note.md", b"generation one").unwrap();
    let database = root.path().join("index.sqlite3");
    let mut verifier = PersistentIndexStore::open(&database).unwrap();
    verifier.rebuild(&vault).unwrap();

    let error = verifier
        .verify_freshness_with_guards(
            &vault,
            || {},
            || {
                thread::scope(|scope| {
                    fs::write(root.path().join("note.md"), b"generation two").unwrap();
                    let (first_scanned_tx, first_scanned_rx) = mpsc::sync_channel(0);
                    let (start_first_tx, start_first_rx) = mpsc::sync_channel(0);
                    let first_database = &database;
                    let first_vault = &vault;
                    let first_root = root.path();
                    let first = scope.spawn(move || {
                        let mut store = PersistentIndexStore::open(first_database).unwrap();
                        store.rebuild_with_guards(
                            first_vault,
                            || {
                                first_scanned_tx.send(()).unwrap();
                                start_first_rx.recv().unwrap();
                            },
                            || {
                                fs::write(first_root.join("note.md"), b"generation two").unwrap();
                                Ok(())
                            },
                        )
                    });
                    first_scanned_rx.recv().unwrap();

                    fs::write(root.path().join("note.md"), b"generation three").unwrap();
                    let (second_scanned_tx, second_scanned_rx) = mpsc::sync_channel(0);
                    let (start_second_tx, start_second_rx) = mpsc::sync_channel(0);
                    let second_database = &database;
                    let second_vault = &vault;
                    let second_root = root.path();
                    let second = scope.spawn(move || {
                        let mut store = PersistentIndexStore::open(second_database).unwrap();
                        store.rebuild_with_guards(
                            second_vault,
                            || {
                                second_scanned_tx.send(()).unwrap();
                                start_second_rx.recv().unwrap();
                            },
                            || {
                                fs::write(second_root.join("note.md"), b"generation three")
                                    .unwrap();
                                Ok(())
                            },
                        )
                    });
                    second_scanned_rx.recv().unwrap();

                    start_first_tx.send(()).unwrap();
                    let first_error = first.join().unwrap().unwrap_err();
                    assert!(matches!(first_error, StoreError::PublicationAborted(_)));

                    start_second_tx.send(()).unwrap();
                    let second_metadata = second.join().unwrap().unwrap();
                    assert_eq!(second_metadata.generation, Some(2));
                });
            },
        )
        .unwrap_err();

    assert!(matches!(
        error,
        StoreError::FreshnessGenerationChanged {
            expected_generation: Some(1)
        }
    ));
    let publication = verifier.published_snapshot().unwrap();
    assert_eq!(publication.metadata.generation, Some(2));
    assert_eq!(publication.metadata.freshness, Freshness::Current);
    assert_eq!(publication.notes[0].generation, 2);
    assert_eq!(publication.notes[0].text, "generation three");
}

#[test]
fn rebuild_refuses_source_drift_between_candidate_and_publication() {
    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path()).unwrap();
    vault.create_note("note.md", b"published").unwrap();
    let mut store = PersistentIndexStore::open(root.path().join("index.sqlite3")).unwrap();
    store.rebuild(&vault).unwrap();
    fs::write(root.path().join("note.md"), b"candidate").unwrap();

    let error = store
        .rebuild_with_publication_guard(&vault, || {
            fs::write(root.path().join("note.md"), b"changed before publication").unwrap();
            Ok(())
        })
        .unwrap_err();

    assert!(matches!(error, StoreError::PublicationAborted(_)));
    let published = store.published_snapshot().unwrap();
    assert_eq!(published.metadata.generation, Some(1));
    assert_eq!(published.metadata.freshness, Freshness::Degraded);
    assert_eq!(published.notes[0].text, "published");
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

#[test]
fn legacy_v1_cache_is_transactionally_reset_and_rebuilt_from_authoritative_files() {
    const LEGACY_SCHEMA: &str = r"
        CREATE TABLE metadata (
            singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
            schema_version INTEGER NOT NULL,
            parser_version TEXT NOT NULL,
            generation INTEGER,
            freshness TEXT NOT NULL CHECK (freshness IN ('empty', 'current', 'degraded')),
            completed_at_unix_ms INTEGER
        ) STRICT;
        CREATE TABLE notes (
            path BLOB PRIMARY KEY,
            title TEXT NOT NULL,
            text TEXT NOT NULL,
            fingerprint TEXT NOT NULL,
            outgoing_links_json TEXT NOT NULL,
            generation INTEGER NOT NULL
        ) STRICT;
        CREATE TABLE diagnostics (
            ordinal INTEGER PRIMARY KEY,
            path BLOB NOT NULL,
            message TEXT NOT NULL
        ) STRICT;
        INSERT INTO metadata VALUES (1, 1, 'markdown-index-v1', 7, 'current', 1);
        INSERT INTO notes VALUES (x'7374616c652e6d64', 'stale', 'stale cache', 'old', '[]', 7);
        PRAGMA user_version = 1;
    ";
    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path()).unwrap();
    vault
        .create_note("source.md", b"authoritative value")
        .unwrap();
    let source_before = fs::read(root.path().join("source.md")).unwrap();
    let database = root.path().join("index.sqlite3");
    Connection::open(&database)
        .unwrap()
        .execute_batch(LEGACY_SCHEMA)
        .unwrap();

    let mut store = PersistentIndexStore::open(&database).unwrap();
    let reset = store.metadata().unwrap();
    assert_eq!(store.sqlite_user_version().unwrap(), SCHEMA_VERSION);
    assert_eq!(reset.generation, None);
    assert_eq!(reset.freshness, Freshness::Empty);
    assert_eq!(reset.note_count, 0);

    let rebuilt = store.rebuild(&vault).unwrap();
    assert_eq!(rebuilt.generation, Some(1));
    assert_eq!(rebuilt.freshness, Freshness::Current);
    assert_eq!(paths(&store, "authoritative"), ["source.md"]);
    assert_eq!(
        fs::read(root.path().join("source.md")).unwrap(),
        source_before
    );
    drop(store);

    let reopened = PersistentIndexStore::open(&database).unwrap();
    assert_eq!(reopened.metadata().unwrap().generation, Some(1));
}

#[test]
fn persisted_derived_field_tampering_cannot_be_certified_as_current() {
    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path()).unwrap();
    vault
        .create_note(
            "note.md",
            b"# Source title\nsource needle [[Source Link]]\n",
        )
        .unwrap();
    let database = root.path().join("index.sqlite3");
    let mut store = PersistentIndexStore::open(&database).unwrap();
    store.rebuild(&vault).unwrap();
    drop(store);
    Connection::open(&database)
        .unwrap()
        .execute(
            "UPDATE notes SET title = 'forged title', text = 'forged needle', outgoing_links_json = '[\"Forged Link\"]', generation = 99",
            [],
        )
        .unwrap();

    let mut store = PersistentIndexStore::open(&database).unwrap();
    let metadata = store.verify_freshness(&vault).unwrap();

    assert_eq!(metadata.generation, Some(1));
    assert_eq!(metadata.freshness, Freshness::Degraded);
    assert_eq!(metadata.diagnostics.len(), 4);
    assert!(
        metadata
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("stored title"))
    );
    assert!(
        metadata
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("stored text"))
    );
    assert!(
        metadata
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("stored links"))
    );
    assert!(
        metadata
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("row generation"))
    );
    assert!(matches!(
        store.search_current("forged"),
        Err(StoreError::NotCurrent {
            freshness: Freshness::Degraded
        })
    ));
}

#[cfg(unix)]
#[test]
fn database_file_symlink_fails_closed_without_touching_its_target() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("authority.md");
    let original = b"ordinary file remains authoritative";
    fs::write(&target, original).unwrap();
    let database = root.path().join("index.sqlite3");
    symlink(&target, &database).unwrap();

    assert!(matches!(
        PersistentIndexStore::open(&database),
        Err(StoreError::UnsafeDatabasePath { .. })
    ));
    assert_eq!(fs::read(&target).unwrap(), original);
}

#[cfg(unix)]
#[test]
fn database_parent_symlink_fails_closed() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let linked_parent = root.path().join("indexes");
    symlink(outside.path(), &linked_parent).unwrap();

    assert!(matches!(
        PersistentIndexStore::open(linked_parent.join("index.sqlite3")),
        Err(StoreError::UnsafeDatabasePath { .. })
    ));
    assert!(!outside.path().join("index.sqlite3").exists());
}

#[test]
fn spoofed_legacy_version_without_the_legacy_schema_fails_closed() {
    let root = tempfile::tempdir().unwrap();
    let database = root.path().join("spoofed-v1.sqlite3");
    Connection::open(&database)
        .unwrap()
        .execute_batch("CREATE TABLE unrelated (value TEXT); PRAGMA user_version = 1;")
        .unwrap();

    assert!(PersistentIndexStore::open(&database).is_err());
    let connection = Connection::open(&database).unwrap();
    let unrelated_exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = 'unrelated')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(unrelated_exists);
}

#[test]
fn freshness_certification_rejects_source_change_at_boundary() {
    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path()).unwrap();
    vault.create_note("note.md", b"published").unwrap();
    let mut store = PersistentIndexStore::open(root.path().join("index.sqlite3")).unwrap();
    store.rebuild(&vault).unwrap();
    let metadata = store
        .verify_freshness_with_guards(
            &vault,
            || {},
            || {
                fs::write(root.path().join("note.md"), b"changed at certification").unwrap();
            },
        )
        .unwrap();
    assert_eq!(metadata.freshness, Freshness::Degraded);
    assert!(matches!(
        store.search_current("published"),
        Err(StoreError::NotCurrent { .. })
    ));
}

#[test]
fn rebuild_rejects_post_confirmation_pre_commit_source_change() {
    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path()).unwrap();
    vault.create_note("note.md", b"published").unwrap();
    let mut store = PersistentIndexStore::open(root.path().join("index.sqlite3")).unwrap();
    store.rebuild(&vault).unwrap();
    let error = store
        .rebuild_with_publication_guard(&vault, || {
            fs::write(root.path().join("note.md"), b"changed at commit boundary").unwrap();
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(error, StoreError::PublicationAborted(_)));
    assert_eq!(store.metadata().unwrap().freshness, Freshness::Degraded);
}

#[test]
fn corruption_recovery_replaces_invalid_truncated_and_malformed_databases() {
    for corrupt in [
        b"not sqlite".as_slice(),
        b"SQLite format 3\0truncated".as_slice(),
    ] {
        let root = tempfile::tempdir().unwrap();
        let vault = Vault::open(root.path()).unwrap();
        vault
            .create_note("note.md", b"authoritative needle")
            .unwrap();
        let database = root.path().join("index.sqlite3");
        fs::write(&database, corrupt).unwrap();
        let metadata = PersistentIndexStore::recover_and_rebuild(&database, &vault).unwrap();
        assert_eq!(metadata.freshness, Freshness::Current);
        let reopened = PersistentIndexStore::open(&database).unwrap();
        assert_eq!(paths(&reopened, "needle"), ["note.md"]);
        assert_ne!(fs::read(&database).unwrap(), corrupt);
    }

    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path()).unwrap();
    vault.create_note("note.md", b"source").unwrap();
    let database = root.path().join("index.sqlite3");
    Connection::open(&database)
        .unwrap()
        .execute_batch("CREATE TABLE attacker (value TEXT); PRAGMA user_version = 2;")
        .unwrap();
    let metadata = PersistentIndexStore::recover_and_rebuild(&database, &vault).unwrap();
    assert_eq!(metadata.freshness, Freshness::Current);
}

#[test]
fn recovery_failure_after_candidate_sync_preserves_corrupt_target() {
    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path()).unwrap();
    vault.create_note("note.md", b"source").unwrap();
    let database = root.path().join("index.sqlite3");
    let corrupt = b"corrupt original";
    fs::write(&database, corrupt).unwrap();
    let error = PersistentIndexStore::recover_and_rebuild_with_guard(&database, &vault, || {
        Err("stop before atomic replacement".to_owned())
    })
    .unwrap_err();
    assert!(matches!(error, StoreError::PublicationAborted(_)));
    assert_eq!(fs::read(&database).unwrap(), corrupt);
}

#[cfg(unix)]
#[test]
fn sqlite_sidecar_symlinks_and_missing_symlink_descendants_are_rejected_without_mutation() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let linked = root.path().join("cache");
    symlink(outside.path(), &linked).unwrap();
    let database = linked.join("missing/indexes/index.sqlite3");
    assert!(PersistentIndexStore::open(&database).is_err());
    assert!(fs::read_dir(outside.path()).unwrap().next().is_none());

    let safe = root.path().join("safe");
    fs::create_dir(&safe).unwrap();
    let target = outside.path().join("authority");
    fs::write(&target, b"untouched").unwrap();
    symlink(&target, safe.join("index.sqlite3-wal")).unwrap();
    assert!(matches!(
        PersistentIndexStore::open(safe.join("index.sqlite3")),
        Err(StoreError::UnsafeDatabasePath { .. })
    ));
    assert_eq!(fs::read(target).unwrap(), b"untouched");
}

#[test]
fn recovery_reobserves_source_after_replacement_guard() {
    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path()).unwrap();
    vault.create_note("note.md", b"candidate source").unwrap();
    let database = root.path().join("index.sqlite3");
    let corrupt = b"corrupt original";
    fs::write(&database, corrupt).unwrap();

    let error = PersistentIndexStore::recover_and_rebuild_with_guard(&database, &vault, || {
        fs::write(
            root.path().join("note.md"),
            b"changed at replacement boundary",
        )
        .unwrap();
        Ok(())
    })
    .unwrap_err();

    assert!(matches!(error, StoreError::PublicationAborted(_)));
    assert_eq!(fs::read(&database).unwrap(), corrupt);
}

#[test]
fn verified_search_rejects_database_tampering_after_prior_freshness_check() {
    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path()).unwrap();
    vault
        .create_note("note.md", b"authoritative needle")
        .unwrap();
    let database = root.path().join("index.sqlite3");
    let mut store = PersistentIndexStore::open(&database).unwrap();
    store.rebuild(&vault).unwrap();
    assert_eq!(
        store.verify_freshness(&vault).unwrap().freshness,
        Freshness::Current
    );

    let error = store
        .search_verified_with_guards(
            &vault,
            "forged",
            || {
                Connection::open(&database)
                    .unwrap()
                    .execute(
                        "UPDATE notes SET title = 'forged', text = 'forged result'",
                        [],
                    )
                    .unwrap();
            },
            || {},
        )
        .unwrap_err();

    assert!(matches!(
        error,
        StoreError::NotCurrent {
            freshness: Freshness::Degraded
        }
    ));
    let metadata = store.metadata().unwrap();
    assert_eq!(metadata.freshness, Freshness::Degraded);
    assert!(
        metadata
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("stored title"))
    );
}

#[test]
fn verified_search_rejects_source_change_at_consumption_boundary() {
    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path()).unwrap();
    vault.create_note("note.md", b"published needle").unwrap();
    let mut store = PersistentIndexStore::open(root.path().join("index.sqlite3")).unwrap();
    store.rebuild(&vault).unwrap();

    let error = store
        .search_verified_with_guards(
            &vault,
            "published",
            || {},
            || {
                fs::write(root.path().join("note.md"), b"changed during consumption").unwrap();
            },
        )
        .unwrap_err();

    assert!(matches!(
        error,
        StoreError::NotCurrent {
            freshness: Freshness::Degraded
        }
    ));
    assert_eq!(store.metadata().unwrap().freshness, Freshness::Degraded);
}

#[test]
fn verified_search_holds_database_write_lock_until_results_are_owned() {
    let root = tempfile::tempdir().unwrap();
    let vault = Vault::open(root.path()).unwrap();
    vault
        .create_note("note.md", b"authoritative needle")
        .unwrap();
    let database = root.path().join("index.sqlite3");
    let mut store = PersistentIndexStore::open(&database).unwrap();
    store.rebuild(&vault).unwrap();
    let attacker = Connection::open(&database).unwrap();
    attacker.busy_timeout(std::time::Duration::ZERO).unwrap();

    let snapshot = store
        .search_verified_with_guards(
            &vault,
            "needle",
            || {},
            || {
                let error = attacker
                    .execute("UPDATE notes SET text = 'forged result'", [])
                    .unwrap_err();
                assert!(matches!(
                    error,
                    rusqlite::Error::SqliteFailure(ref failure, _)
                        if matches!(failure.code, ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked)
                ));
            },
        )
        .unwrap();

    assert_eq!(snapshot.metadata.freshness, Freshness::Current);
    assert_eq!(snapshot.notes.len(), 1);
    assert_eq!(snapshot.notes[0].text, "authoritative needle");
}
