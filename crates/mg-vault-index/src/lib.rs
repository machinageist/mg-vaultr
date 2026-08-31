#[cfg(target_os = "linux")]
use std::ffi::{OsStr, OsString};
#[cfg(not(target_os = "linux"))]
use std::fs;
#[cfg(target_os = "linux")]
use std::os::fd::OwnedFd;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use mg_vault_core::{IndexDiagnostic, IndexStatus, MarkdownIndex, SourceFingerprint, Vault};
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior, params,
};
use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 2;
pub const PARSER_VERSION: &str = "markdown-index-v1";

const LEGACY_SCHEMA_VERSION: u32 = 1;

const SCHEMA: &str = r"
CREATE TABLE metadata (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    schema_version INTEGER NOT NULL,
    parser_version TEXT NOT NULL,
    generation INTEGER,
    freshness TEXT NOT NULL CHECK (freshness IN ('empty', 'current', 'stale', 'degraded')),
    completed_at_unix_ms INTEGER,
    observed_at_unix_ms INTEGER
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
INSERT INTO metadata (
    singleton, schema_version, parser_version, generation, freshness,
    completed_at_unix_ms, observed_at_unix_ms
) VALUES (1, 2, 'markdown-index-v1', NULL, 'empty', NULL, NULL);
PRAGMA user_version = 2;
";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    Empty,
    Current,
    Stale,
    Degraded,
}

impl Freshness {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Current => "current",
            Self::Stale => "stale",
            Self::Degraded => "degraded",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "empty" => Ok(Self::Empty),
            "current" => Ok(Self::Current),
            "stale" => Ok(Self::Stale),
            "degraded" => Ok(Self::Degraded),
            other => Err(StoreError::InvalidMetadata(format!(
                "unknown freshness value {other:?}"
            ))),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Diagnostic {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IndexMetadata {
    pub schema_version: u32,
    pub parser_version: String,
    pub generation: Option<u64>,
    pub freshness: Freshness,
    pub note_count: u64,
    pub completed_at_unix_ms: Option<u64>,
    pub observed_at_unix_ms: Option<u64>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StoredNote {
    pub path: PathBuf,
    pub title: String,
    pub text: String,
    pub fingerprint: String,
    pub outgoing_links: Vec<String>,
    pub generation: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct QuerySnapshot {
    pub metadata: IndexMetadata,
    pub notes: Vec<StoredNote>,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("SQLite index error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("I/O error at {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("unsupported index schema version {found}; expected {expected}")]
    UnsupportedSchema { found: u32, expected: u32 },
    #[error("unsupported index parser version {found:?}; expected {expected:?}")]
    UnsupportedParser {
        found: String,
        expected: &'static str,
    },
    #[error("invalid index metadata: {0}")]
    InvalidMetadata(String),
    #[error("index schema initialization is incomplete")]
    IncompleteSchema,
    #[error("index generation is exhausted")]
    GenerationExhausted,
    #[error("current index required; freshness is {freshness:?}")]
    NotCurrent { freshness: Freshness },
    #[error("index publication aborted: {0}")]
    PublicationAborted(String),
    #[error(
        "freshness observation lost its published generation (expected {expected_generation:?})"
    )]
    FreshnessGenerationChanged { expected_generation: Option<u64> },
    #[error("unsafe disposable index path {}: {reason}", path.display())]
    UnsafeDatabasePath { path: PathBuf, reason: String },
}

pub type Result<T> = std::result::Result<T, StoreError>;

pub struct PersistentIndexStore {
    connection: Connection,
    #[cfg(target_os = "linux")]
    _parent_directory: OwnedFd,
}

/// Return the disposable database location for a canonical, opened vault.
#[must_use]
pub fn database_path(index_directory: &Path, vault: &Vault) -> PathBuf {
    let identity = SourceFingerprint::of(&path_to_bytes(vault.root()));
    index_directory.join(format!("{identity}.sqlite3"))
}

impl PersistentIndexStore {
    /// Open or atomically initialize a disposable index database.
    ///
    /// # Errors
    /// Returns an I/O or SQLite error, transactionally resets the legacy v1 cache,
    /// rejects other unsupported versions, and fails closed on an unversioned partial schema.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        #[cfg(target_os = "linux")]
        {
            let (parent_directory, database_name) = confined_database_parent(path)?;
            reject_sqlite_links_at(&parent_directory, &database_name, path)?;
            let sqlite_path = proc_fd_path(&parent_directory, &database_name);
            let connection = open_connection(&sqlite_path)?;
            Ok(Self {
                connection,
                _parent_directory: parent_directory,
            })
        }
        #[cfg(not(target_os = "linux"))]
        {
            let sqlite_path = prepare_portable_database_path(path)?;
            reject_sqlite_links(path)?;
            let connection = open_connection(&sqlite_path)?;
            Ok(Self { connection })
        }
    }

    /// Replace an unreadable disposable index with a verified durable rebuild.
    ///
    /// # Errors
    /// Returns an error when candidate construction, verification, durability,
    /// atomic replacement, or post-replacement verification fails.
    pub fn recover_and_rebuild(path: impl AsRef<Path>, vault: &Vault) -> Result<IndexMetadata> {
        Self::recover_and_rebuild_with_guard(path, vault, || Ok(()))
    }

    /// Recover with a deterministic guard after candidate sync and before replacement.
    ///
    /// # Errors
    /// Returns [`StoreError::PublicationAborted`] when the guard rejects replacement,
    /// or the same errors as [`Self::recover_and_rebuild`].
    pub fn recover_and_rebuild_with_guard<F>(
        path: impl AsRef<Path>,
        vault: &Vault,
        replacement_guard: F,
    ) -> Result<IndexMetadata>
    where
        F: FnOnce() -> std::result::Result<(), String>,
    {
        recover_database(path.as_ref(), vault, replacement_guard)
    }

    /// Read SQLite's schema version marker.
    ///
    /// # Errors
    /// Returns an error when SQLite cannot read the pragma value.
    pub fn sqlite_user_version(&self) -> Result<u32> {
        user_version(&self.connection)
    }

    /// Read the published generation metadata and diagnostics.
    ///
    /// # Errors
    /// Returns an error for invalid persisted metadata or a failed SQLite query.
    pub fn metadata(&self) -> Result<IndexMetadata> {
        let transaction = self.connection.unchecked_transaction()?;
        let result = metadata(&transaction)?;
        transaction.commit()?;
        Ok(result)
    }

    /// Rebuild and atomically publish a complete source projection.
    ///
    /// # Errors
    /// Returns an error when scanning, staging, metadata conversion, or SQLite
    /// publication fails. A degraded source scan never publishes partial rows.
    pub fn rebuild(&mut self, vault: &Vault) -> Result<IndexMetadata> {
        self.rebuild_with_guards(vault, || {}, || Ok(()))
    }

    /// Compare authoritative source observations with the published generation.
    ///
    /// This operation never publishes source rows or advances the generation. A
    /// complete mismatch marks the prior projection stale; a failed confined
    /// scan marks it degraded. Callers must rebuild explicitly to publish drift.
    ///
    /// # Errors
    /// Returns an error when persisted rows are invalid or freshness metadata
    /// cannot be updated atomically.
    pub fn verify_freshness(&mut self, vault: &Vault) -> Result<IndexMetadata> {
        self.verify_freshness_with_observation_guard(vault, || {})
    }

    /// Verify freshness using two complete observations separated by a caller hook.
    ///
    /// The hook makes source drift at the observation boundary deterministic in tests.
    /// Production callers use [`Self::verify_freshness`], whose no-op hook still requires
    /// two matching complete walks before a generation can be labeled current.
    ///
    /// # Errors
    /// Returns an error when persisted rows are invalid or freshness metadata
    /// cannot be updated atomically.
    pub fn verify_freshness_with_observation_guard<F>(
        &mut self,
        vault: &Vault,
        observation_guard: F,
    ) -> Result<IndexMetadata>
    where
        F: FnOnce(),
    {
        self.verify_freshness_with_guards(vault, observation_guard, || {})
    }

    /// Verify freshness with deterministic observation and certification hooks.
    ///
    /// The certification hook runs after the published snapshot is read and before
    /// its freshness metadata is updated. It exists to exercise publication races.
    ///
    /// # Errors
    /// Returns an error when persisted rows are invalid or freshness metadata
    /// cannot be updated atomically.
    ///
    /// # Panics
    /// This function does not panic; a missing generation is handled as an
    /// empty persisted projection before comparison.
    pub fn verify_freshness_with_guards<F, G>(
        &mut self,
        vault: &Vault,
        observation_guard: F,
        certification_guard: G,
    ) -> Result<IndexMetadata>
    where
        F: FnOnce(),
        G: FnOnce(),
    {
        let mut projection = MarkdownIndex::new();
        projection.rebuild(vault);
        if let IndexStatus::Degraded { errors, .. } = projection.status() {
            let diagnostics = normalize_diagnostics(vault, errors);
            let expected_generation = self.metadata()?.generation;
            certification_guard();
            self.record_freshness(
                expected_generation,
                Freshness::Degraded,
                &diagnostics,
                now_unix_ms()?,
            )?;
            return self.metadata();
        }

        observation_guard();
        let mut confirmation = MarkdownIndex::new();
        confirmation.rebuild(vault);
        if let IndexStatus::Degraded { errors, .. } = confirmation.status() {
            let diagnostics = normalize_diagnostics(vault, errors);
            let expected_generation = self.metadata()?.generation;
            certification_guard();
            self.record_freshness(
                expected_generation,
                Freshness::Degraded,
                &diagnostics,
                now_unix_ms()?,
            )?;
            return self.metadata();
        }
        if projection.notes() != confirmation.notes() {
            let expected_generation = self.metadata()?.generation;
            certification_guard();
            self.record_freshness(
                expected_generation,
                Freshness::Degraded,
                &[source_changed_during_observation()],
                now_unix_ms()?,
            )?;
            return self.metadata();
        }

        let published = self.published_snapshot()?;
        let expected_generation = published.metadata.generation;
        certification_guard();
        let mut certified = MarkdownIndex::new();
        certified.rebuild(vault);
        let observed_at = now_unix_ms()?;
        if !matches!(
            certified.status(),
            IndexStatus::Empty | IndexStatus::Current { .. }
        ) || projection.notes() != certified.notes()
        {
            self.record_freshness(
                expected_generation,
                Freshness::Degraded,
                &[source_changed_during_observation()],
                observed_at,
            )?;
            return self.metadata();
        }
        if published.metadata.generation.is_none() {
            self.record_freshness(expected_generation, Freshness::Empty, &[], observed_at)?;
            return self.metadata();
        }
        let Some(generation) = expected_generation else {
            self.record_freshness(None, Freshness::Empty, &[], observed_at)?;
            return self.metadata();
        };
        let comparison = compare_source_projection(&projection, &published.notes, generation);
        if !comparison.integrity_diagnostics.is_empty() {
            self.record_freshness(
                expected_generation,
                Freshness::Degraded,
                &comparison.integrity_diagnostics,
                observed_at,
            )?;
        } else if comparison.source_diagnostics.is_empty() {
            self.record_freshness(expected_generation, Freshness::Current, &[], observed_at)?;
        } else {
            self.record_freshness(
                expected_generation,
                Freshness::Stale,
                &comparison.source_diagnostics,
                observed_at,
            )?;
        }
        self.metadata()
    }

    /// Rebuild with a testable guard immediately before generation publication.
    ///
    /// # Errors
    /// Returns [`StoreError::PublicationAborted`] when the guard rejects the
    /// candidate, or the same scan, conversion, and SQLite errors as [`Self::rebuild`].
    pub fn rebuild_with_publication_guard<F>(
        &mut self,
        vault: &Vault,
        publication_guard: F,
    ) -> Result<IndexMetadata>
    where
        F: FnOnce() -> std::result::Result<(), String>,
    {
        self.rebuild_with_guards(vault, || {}, publication_guard)
    }

    /// Rebuild with hooks after candidate observation and before publication.
    ///
    /// The candidate hook runs before the write transaction begins. The publication
    /// hook runs while the write transaction is held and before the confirming source
    /// observation. These hooks make overlapping rebuild races deterministic in tests.
    ///
    /// # Errors
    /// Returns [`StoreError::PublicationAborted`] when the publication hook rejects the
    /// candidate, or the same scan, conversion, and SQLite errors as [`Self::rebuild`].
    pub fn rebuild_with_guards<F, G>(
        &mut self,
        vault: &Vault,
        candidate_guard: F,
        publication_guard: G,
    ) -> Result<IndexMetadata>
    where
        F: FnOnce(),
        G: FnOnce() -> std::result::Result<(), String>,
    {
        let mut projection = MarkdownIndex::new();
        projection.rebuild(vault);
        if let IndexStatus::Degraded { errors, .. } = projection.status() {
            let diagnostics = normalize_diagnostics(vault, errors);
            self.record_degraded(&diagnostics)?;
            return self.metadata();
        }

        candidate_guard();
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let prior_generation = transaction
            .query_row(
                "SELECT generation FROM metadata WHERE singleton = 1",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )?
            .map(u64::try_from)
            .transpose()
            .map_err(|error| StoreError::InvalidMetadata(error.to_string()))?
            .unwrap_or(0);
        let generation = prior_generation
            .checked_add(1)
            .ok_or(StoreError::GenerationExhausted)?;
        let generation_sql =
            i64::try_from(generation).map_err(|_| StoreError::GenerationExhausted)?;
        stage_complete_projection(&transaction, &projection, generation)?;
        let mut confirmation = MarkdownIndex::new();
        confirmation.rebuild(vault);
        let confirmation_diagnostics = match confirmation.status() {
            IndexStatus::Degraded { errors, .. } => normalize_diagnostics(vault, errors),
            IndexStatus::Empty | IndexStatus::Current { .. } => Vec::new(),
        };
        if !confirmation_diagnostics.is_empty() || projection.notes() != confirmation.notes() {
            transaction.rollback()?;
            let diagnostics = if confirmation_diagnostics.is_empty() {
                vec![source_changed_during_observation()]
            } else {
                confirmation_diagnostics
            };
            self.record_degraded(&diagnostics)?;
            return Err(StoreError::PublicationAborted(
                "source changed during index publication; retry rebuild".to_owned(),
            ));
        }
        if let Err(message) = publication_guard() {
            transaction.rollback()?;
            self.record_degraded(&[Diagnostic {
                path: PathBuf::from("."),
                message: message.clone(),
            }])?;
            return Err(StoreError::PublicationAborted(message));
        }
        let mut certification = MarkdownIndex::new();
        certification.rebuild(vault);
        if !matches!(
            certification.status(),
            IndexStatus::Empty | IndexStatus::Current { .. }
        ) || projection.notes() != certification.notes()
        {
            transaction.rollback()?;
            self.record_degraded(&[source_changed_during_observation()])?;
            return Err(StoreError::PublicationAborted(
                "source changed at index certification boundary; retry rebuild".to_owned(),
            ));
        }
        let completed_at_unix_ms = now_unix_ms()?;
        let completed_at_sql = i64::try_from(completed_at_unix_ms)
            .map_err(|error| StoreError::InvalidMetadata(error.to_string()))?;
        transaction.execute(
            "UPDATE metadata SET generation = ?1, freshness = ?2, completed_at_unix_ms = ?3, observed_at_unix_ms = ?3, parser_version = ?4, schema_version = ?5 WHERE singleton = 1",
            params![generation_sql, Freshness::Current.as_str(), completed_at_sql, PARSER_VERSION, SCHEMA_VERSION],
        )?;
        transaction.commit()?;
        self.metadata()
    }

    /// Return the last published rows together with their freshness metadata.
    ///
    /// # Errors
    /// Returns an error for invalid metadata, invalid stored rows, or SQLite failure.
    pub fn published_snapshot(&self) -> Result<QuerySnapshot> {
        let transaction = self.connection.unchecked_transaction()?;
        let snapshot = QuerySnapshot {
            metadata: metadata(&transaction)?,
            notes: Self::load_notes(&transaction)?,
        };
        transaction.commit()?;
        Ok(snapshot)
    }

    /// Search a persisted generation only after revalidating its source and rows.
    ///
    /// Source observations bracket acquisition of an immediate SQLite transaction.
    /// The transaction prevents a database writer from changing the generation or
    /// rows between validation and copying the search result into owned memory.
    ///
    /// # Errors
    /// Returns [`StoreError::NotCurrent`] when source drift, an incomplete scan, or
    /// persisted-row tampering is observed at the consumption boundary.
    pub fn search_verified(&mut self, vault: &Vault, query: &str) -> Result<QuerySnapshot> {
        self.search_verified_with_guards(vault, query, || {}, || {})
    }

    /// Search with deterministic hooks around database snapshot acquisition.
    ///
    /// The database hook runs after the first source observation and before the
    /// immediate transaction. The consumption hook runs after persisted rows are
    /// loaded while that transaction is held and before the final source observation.
    ///
    /// # Errors
    /// Returns the same errors as [`Self::search_verified`].
    pub fn search_verified_with_guards<F, G>(
        &mut self,
        vault: &Vault,
        query: &str,
        database_guard: F,
        consumption_guard: G,
    ) -> Result<QuerySnapshot>
    where
        F: FnOnce(),
        G: FnOnce(),
    {
        let mut initial = MarkdownIndex::new();
        initial.rebuild(vault);
        database_guard();

        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut persisted_metadata = metadata(&transaction)?;
        let persisted_notes = Self::load_notes(&transaction)?;
        let expected_generation = persisted_metadata.generation;
        consumption_guard();

        let mut final_observation = MarkdownIndex::new();
        final_observation.rebuild(vault);
        let observed_at = now_unix_ms()?;
        let (freshness, diagnostics) = match (initial.status(), final_observation.status()) {
            (IndexStatus::Degraded { errors, .. }, _)
            | (_, IndexStatus::Degraded { errors, .. }) => {
                (Freshness::Degraded, normalize_diagnostics(vault, errors))
            }
            _ if initial.notes() != final_observation.notes() => (
                Freshness::Degraded,
                vec![source_changed_during_observation()],
            ),
            _ => match expected_generation {
                None => (Freshness::Empty, Vec::new()),
                Some(generation) => {
                    let comparison =
                        compare_source_projection(&final_observation, &persisted_notes, generation);
                    if !comparison.integrity_diagnostics.is_empty() {
                        (Freshness::Degraded, comparison.integrity_diagnostics)
                    } else if comparison.source_diagnostics.is_empty() {
                        (Freshness::Current, Vec::new())
                    } else {
                        (Freshness::Stale, comparison.source_diagnostics)
                    }
                }
            },
        };
        record_freshness_in_transaction(
            &transaction,
            expected_generation,
            freshness,
            &diagnostics,
            observed_at,
        )?;
        persisted_metadata.freshness = freshness;
        persisted_metadata.observed_at_unix_ms = Some(observed_at);
        persisted_metadata.diagnostics = diagnostics;
        if freshness != Freshness::Current {
            transaction.commit()?;
            return Err(StoreError::NotCurrent { freshness });
        }

        let query = query.to_lowercase();
        let notes = persisted_notes
            .into_iter()
            .filter(|note| note_matches(note, &query))
            .collect();
        transaction.commit()?;
        Ok(QuerySnapshot {
            metadata: persisted_metadata,
            notes,
        })
    }

    /// Search only when the published snapshot is explicitly current.
    ///
    /// # Errors
    /// Returns [`StoreError::NotCurrent`] for empty or degraded metadata and
    /// returns conversion or SQLite errors for invalid persisted rows.
    pub fn search_current(&self, query: &str) -> Result<QuerySnapshot> {
        let transaction = self.connection.unchecked_transaction()?;
        let metadata = metadata(&transaction)?;
        if metadata.freshness != Freshness::Current {
            return Err(StoreError::NotCurrent {
                freshness: metadata.freshness,
            });
        }
        let query = query.to_lowercase();
        let notes = Self::load_notes(&transaction)?
            .into_iter()
            .filter(|note| note_matches(note, &query))
            .collect();
        transaction.commit()?;
        Ok(QuerySnapshot { metadata, notes })
    }

    fn load_notes(connection: &Connection) -> Result<Vec<StoredNote>> {
        let mut statement = connection.prepare(
            "SELECT path, title, text, fingerprint, outgoing_links_json, generation FROM notes ORDER BY path",
        )?;
        let rows = statement.query_map([], |row| {
            let path: Vec<u8> = row.get(0)?;
            let outgoing_links_json: String = row.get(4)?;
            let outgoing_links = serde_json::from_str(&outgoing_links_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    4,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            Ok(StoredNote {
                path: bytes_to_path(path),
                title: row.get(1)?,
                text: row.get(2)?,
                fingerprint: row.get(3)?,
                outgoing_links,
                generation: u64::try_from(row.get::<_, i64>(5)?).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        5,
                        rusqlite::types::Type::Integer,
                        Box::new(error),
                    )
                })?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    fn record_degraded(&mut self, diagnostics: &[Diagnostic]) -> Result<()> {
        let expected_generation = self.metadata()?.generation;
        self.record_freshness(
            expected_generation,
            Freshness::Degraded,
            diagnostics,
            now_unix_ms()?,
        )
    }

    fn record_freshness(
        &mut self,
        expected_generation: Option<u64>,
        freshness: Freshness,
        diagnostics: &[Diagnostic],
        observed_at_unix_ms: u64,
    ) -> Result<()> {
        let transaction = self.connection.transaction()?;
        record_freshness_in_transaction(
            &transaction,
            expected_generation,
            freshness,
            diagnostics,
            observed_at_unix_ms,
        )?;
        transaction.commit()?;
        Ok(())
    }
}

fn record_freshness_in_transaction(
    transaction: &Transaction<'_>,
    expected_generation: Option<u64>,
    freshness: Freshness,
    diagnostics: &[Diagnostic],
    observed_at_unix_ms: u64,
) -> Result<()> {
    let expected_generation_sql = expected_generation
        .map(i64::try_from)
        .transpose()
        .map_err(|error| StoreError::InvalidMetadata(error.to_string()))?;
    let updated = transaction.execute(
        "UPDATE metadata SET freshness = ?1, observed_at_unix_ms = ?2 WHERE singleton = 1 AND generation IS ?3",
        params![
            freshness.as_str(),
            i64::try_from(observed_at_unix_ms)
                .map_err(|error| StoreError::InvalidMetadata(error.to_string()))?,
            expected_generation_sql,
        ],
    )?;
    if updated != 1 {
        return Err(StoreError::FreshnessGenerationChanged {
            expected_generation,
        });
    }
    transaction.execute("DELETE FROM diagnostics", [])?;
    let mut statement = transaction
        .prepare("INSERT INTO diagnostics (ordinal, path, message) VALUES (?1, ?2, ?3)")?;
    for (ordinal, diagnostic) in diagnostics.iter().enumerate() {
        statement.execute(params![
            i64::try_from(ordinal)
                .map_err(|error| StoreError::InvalidMetadata(error.to_string()))?,
            path_to_bytes(&diagnostic.path),
            diagnostic.message,
        ])?;
    }
    Ok(())
}

fn note_matches(note: &StoredNote, query: &str) -> bool {
    note.path.to_string_lossy().to_lowercase().contains(query)
        || note.title.to_lowercase().contains(query)
        || note.text.to_lowercase().contains(query)
        || note
            .outgoing_links
            .iter()
            .any(|link| link.to_lowercase().contains(query))
}

fn stage_complete_projection(
    transaction: &Transaction<'_>,
    projection: &MarkdownIndex,
    generation: u64,
) -> Result<()> {
    let generation = i64::try_from(generation).map_err(|_| StoreError::GenerationExhausted)?;
    transaction.execute("DELETE FROM diagnostics", [])?;
    transaction.execute("DELETE FROM notes", [])?;
    let mut statement = transaction.prepare(
        "INSERT INTO notes (path, title, text, fingerprint, outgoing_links_json, generation) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for note in projection.notes() {
        statement.execute(params![
            path_to_bytes(&note.path),
            note.title,
            note.text,
            note.fingerprint.to_string(),
            serde_json::to_string(&note.outgoing_links).expect("serializing strings cannot fail"),
            generation,
        ])?;
    }
    Ok(())
}

fn source_changed_during_observation() -> Diagnostic {
    Diagnostic {
        path: PathBuf::from("."),
        message: "source changed during freshness verification; retry observation".to_owned(),
    }
}

struct ProjectionComparison {
    source_diagnostics: Vec<Diagnostic>,
    integrity_diagnostics: Vec<Diagnostic>,
}

fn compare_source_projection(
    projection: &MarkdownIndex,
    published: &[StoredNote],
    expected_generation: u64,
) -> ProjectionComparison {
    let mut source_diagnostics = Vec::new();
    let mut integrity_diagnostics = Vec::new();
    let mut source_index = 0;
    let mut stored_index = 0;
    while source_index < projection.notes().len() || stored_index < published.len() {
        match (
            projection.notes().get(source_index),
            published.get(stored_index),
        ) {
            (Some(source), Some(stored)) if source.path == stored.path => {
                if source.fingerprint.to_string() == stored.fingerprint {
                    if source.title != stored.title {
                        integrity_diagnostics.push(Diagnostic {
                            path: source.path.clone(),
                            message: "stored title does not match authoritative source projection"
                                .to_owned(),
                        });
                    }
                    if source.text != stored.text {
                        integrity_diagnostics.push(Diagnostic {
                            path: source.path.clone(),
                            message: "stored text does not match authoritative source projection"
                                .to_owned(),
                        });
                    }
                    if source.outgoing_links != stored.outgoing_links {
                        integrity_diagnostics.push(Diagnostic {
                            path: source.path.clone(),
                            message: "stored links do not match authoritative source projection"
                                .to_owned(),
                        });
                    }
                } else {
                    source_diagnostics.push(Diagnostic {
                        path: source.path.clone(),
                        message: "source changed after published generation".to_owned(),
                    });
                }
                if stored.generation != expected_generation {
                    integrity_diagnostics.push(Diagnostic {
                        path: source.path.clone(),
                        message: "stored row generation does not match published generation"
                            .to_owned(),
                    });
                }
                source_index += 1;
                stored_index += 1;
            }
            (Some(source), Some(stored)) if source.path < stored.path => {
                source_diagnostics.push(Diagnostic {
                    path: source.path.clone(),
                    message: "source added after published generation".to_owned(),
                });
                source_index += 1;
            }
            (Some(_) | None, Some(stored)) => {
                source_diagnostics.push(Diagnostic {
                    path: stored.path.clone(),
                    message: "source removed after published generation".to_owned(),
                });
                stored_index += 1;
            }
            (Some(source), None) => {
                source_diagnostics.push(Diagnostic {
                    path: source.path.clone(),
                    message: "source added after published generation".to_owned(),
                });
                source_index += 1;
            }
            (None, None) => break,
        }
    }
    ProjectionComparison {
        source_diagnostics,
        integrity_diagnostics,
    }
}

fn open_connection(path: &Path) -> Result<Connection> {
    let mut connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    connection.pragma_update(None, "foreign_keys", true)?;
    connection.pragma_update(None, "journal_mode", "DELETE")?;
    connection.busy_timeout(std::time::Duration::from_secs(5))?;
    let found = user_version(&connection)?;
    if found == 0 {
        if has_user_schema(&connection)? {
            return Err(StoreError::IncompleteSchema);
        }
        let transaction = connection.transaction()?;
        transaction.execute_batch(SCHEMA)?;
        transaction.commit()?;
    } else if found == LEGACY_SCHEMA_VERSION {
        reset_legacy_cache(&mut connection)?;
    } else if found != SCHEMA_VERSION {
        return Err(StoreError::UnsupportedSchema {
            found,
            expected: SCHEMA_VERSION,
        });
    }
    validate_metadata(&connection)?;
    Ok(connection)
}

#[cfg(target_os = "linux")]
fn confined_database_parent(path: &Path) -> Result<(OwnedFd, OsString)> {
    use std::path::Component;

    use rustix::fs::{Mode, OFlags, ResolveFlags, fchmod, mkdirat, open, openat2};
    use rustix::process::getuid;

    if !path.is_absolute() {
        return Err(StoreError::UnsafeDatabasePath {
            path: path.to_path_buf(),
            reason: "database path must be absolute".to_owned(),
        });
    }
    let name = path
        .file_name()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| StoreError::UnsafeDatabasePath {
            path: path.to_path_buf(),
            reason: "database path has no file name".to_owned(),
        })?
        .to_os_string();
    let mut directory = open(
        "/",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| io_error(path, error))?;
    let parent = path
        .parent()
        .ok_or_else(|| StoreError::UnsafeDatabasePath {
            path: path.to_path_buf(),
            reason: "database path has no parent".to_owned(),
        })?;
    for component in parent.components() {
        let Component::Normal(component) = component else {
            continue;
        };
        let opened = openat2(
            &directory,
            component,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS,
        );
        directory = match opened {
            Ok(opened) => opened,
            Err(rustix::io::Errno::NOENT) => {
                mkdirat(&directory, component, Mode::RUSR | Mode::WUSR | Mode::XUSR)
                    .map_err(|error| io_error(path, error))?;
                openat2(
                    &directory,
                    component,
                    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
                    Mode::empty(),
                    ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS,
                )
                .map_err(|error| io_error(path, error))?
            }
            Err(error) => {
                return Err(StoreError::UnsafeDatabasePath {
                    path: path.to_path_buf(),
                    reason: format!("database parent is not confined: {error}"),
                });
            }
        };
    }
    let stat = rustix::fs::fstat(&directory).map_err(|error| io_error(path, error))?;
    if stat.st_uid != getuid().as_raw() {
        return Err(StoreError::UnsafeDatabasePath {
            path: parent.to_path_buf(),
            reason: "database directory is not owned by the current user".to_owned(),
        });
    }
    fchmod(&directory, Mode::RUSR | Mode::WUSR | Mode::XUSR)
        .map_err(|error| io_error(parent, error))?;
    Ok((directory, name))
}

#[cfg(target_os = "linux")]
fn proc_fd_path(parent: &OwnedFd, name: &OsStr) -> PathBuf {
    use std::os::fd::AsRawFd;

    PathBuf::from(format!("/proc/self/fd/{}", parent.as_raw_fd())).join(name)
}

#[cfg(target_os = "linux")]
fn reject_sqlite_links_at(parent: &OwnedFd, name: &OsStr, display_path: &Path) -> Result<()> {
    use rustix::fs::{AtFlags, FileType, statat};

    for entry in sqlite_family_names(name) {
        match statat(parent, &entry, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(stat) => {
                let file_type = FileType::from_raw_mode(stat.st_mode);
                if file_type.is_symlink() || !file_type.is_file() {
                    return Err(StoreError::UnsafeDatabasePath {
                        path: display_path.with_file_name(entry),
                        reason: "SQLite database or sidecar is not a regular file".to_owned(),
                    });
                }
            }
            Err(rustix::io::Errno::NOENT) => {}
            Err(error) => return Err(io_error(display_path, error)),
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn sqlite_family_names(name: &OsStr) -> Vec<OsString> {
    ["", "-journal", "-wal", "-shm"]
        .into_iter()
        .map(|suffix| {
            let mut entry = name.to_os_string();
            entry.push(suffix);
            entry
        })
        .collect()
}

#[cfg(target_os = "linux")]
fn recover_database<F>(path: &Path, vault: &Vault, replacement_guard: F) -> Result<IndexMetadata>
where
    F: FnOnce() -> std::result::Result<(), String>,
{
    use rustix::fs::{AtFlags, Mode, OFlags, ResolveFlags, fsync, openat2, renameat, unlinkat};

    let (parent, database_name) = confined_database_parent(path)?;
    reject_sqlite_links_at(&parent, &database_name, path)?;
    let mut candidate_name = database_name.clone();
    candidate_name.push(format!(
        ".candidate-{}-{}",
        std::process::id(),
        now_unix_ms()?
    ));
    let candidate_path = proc_fd_path(&parent, &candidate_name);
    let candidate_connection = open_connection(&candidate_path)?;
    let mut candidate = PersistentIndexStore {
        connection: candidate_connection,
        _parent_directory: rustix::io::dup(&parent).map_err(|error| io_error(path, error))?,
    };
    let metadata = candidate.rebuild(vault)?;
    if metadata.freshness != Freshness::Current {
        return Err(StoreError::PublicationAborted(
            "recovery candidate did not reach current freshness".to_owned(),
        ));
    }
    let integrity: String =
        candidate
            .connection
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(StoreError::InvalidMetadata(format!(
            "recovery candidate failed integrity check: {integrity}"
        )));
    }
    drop(candidate);
    let candidate_fd = openat2(
        &parent,
        &candidate_name,
        OFlags::RDONLY | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS,
    )
    .map_err(|error| io_error(path, error))?;
    fsync(&candidate_fd).map_err(|error| io_error(path, error))?;
    replacement_guard().map_err(StoreError::PublicationAborted)?;
    let candidate_connection = open_connection(&candidate_path)?;
    let mut candidate = PersistentIndexStore {
        connection: candidate_connection,
        _parent_directory: rustix::io::dup(&parent).map_err(|error| io_error(path, error))?,
    };
    candidate.search_verified(vault, "").map_err(|error| {
        StoreError::PublicationAborted(format!(
            "recovery candidate lost freshness before replacement: {error}"
        ))
    })?;
    let integrity: String =
        candidate
            .connection
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(StoreError::InvalidMetadata(format!(
            "recovery candidate failed final integrity check: {integrity}"
        )));
    }
    drop(candidate);
    fsync(&candidate_fd).map_err(|error| io_error(path, error))?;
    for sidecar in sqlite_family_names(&database_name).into_iter().skip(1) {
        match unlinkat(&parent, &sidecar, AtFlags::empty()) {
            Ok(()) | Err(rustix::io::Errno::NOENT) => {}
            Err(error) => return Err(io_error(&path.with_file_name(sidecar), error)),
        }
    }
    renameat(&parent, &candidate_name, &parent, &database_name)
        .map_err(|error| io_error(path, error))?;
    fsync(&parent).map_err(|error| io_error(path, error))?;
    let reopened = PersistentIndexStore::open(path)?;
    let reopened_metadata = reopened.metadata()?;
    let final_integrity: String =
        reopened
            .connection
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if reopened_metadata.freshness != Freshness::Current || final_integrity != "ok" {
        return Err(StoreError::InvalidMetadata(
            "replacement index failed post-recovery verification".to_owned(),
        ));
    }
    Ok(reopened_metadata)
}

#[cfg(not(target_os = "linux"))]
fn recover_database<F>(path: &Path, vault: &Vault, replacement_guard: F) -> Result<IndexMetadata>
where
    F: FnOnce() -> std::result::Result<(), String>,
{
    let candidate = path.with_extension(format!("candidate-{}", std::process::id()));
    let mut store = PersistentIndexStore::open(&candidate)?;
    let metadata = store.rebuild(vault)?;
    if metadata.freshness != Freshness::Current {
        return Err(StoreError::PublicationAborted(
            "recovery candidate did not reach current freshness".to_owned(),
        ));
    }
    drop(store);
    fs::File::open(&candidate)
        .and_then(|file| file.sync_all())
        .map_err(|source| StoreError::Io {
            path: candidate.clone(),
            source,
        })?;
    replacement_guard().map_err(StoreError::PublicationAborted)?;
    let mut store = PersistentIndexStore::open(&candidate)?;
    store.search_verified(vault, "").map_err(|error| {
        StoreError::PublicationAborted(format!(
            "recovery candidate lost freshness before replacement: {error}"
        ))
    })?;
    let integrity: String = store
        .connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(StoreError::InvalidMetadata(format!(
            "recovery candidate failed final integrity check: {integrity}"
        )));
    }
    drop(store);
    fs::File::open(&candidate)
        .and_then(|file| file.sync_all())
        .map_err(|source| StoreError::Io {
            path: candidate.clone(),
            source,
        })?;
    fs::rename(&candidate, path).map_err(|source| StoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    PersistentIndexStore::open(path)?.metadata()
}

fn io_error(path: &Path, error: rustix::io::Errno) -> StoreError {
    StoreError::Io {
        path: path.to_path_buf(),
        source: std::io::Error::from_raw_os_error(error.raw_os_error()),
    }
}

#[cfg(not(target_os = "linux"))]
fn prepare_portable_database_path(path: &Path) -> Result<PathBuf> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| StoreError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    Ok(path.to_path_buf())
}

#[cfg(not(target_os = "linux"))]
fn reject_sqlite_links(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(StoreError::UnsafeDatabasePath {
                path: path.to_path_buf(),
                reason: "database path is not a regular file".to_owned(),
            })
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(StoreError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn metadata(connection: &Connection) -> Result<IndexMetadata> {
    let row = connection.query_row(
        "SELECT schema_version, parser_version, generation, freshness, completed_at_unix_ms, observed_at_unix_ms FROM metadata WHERE singleton = 1",
        [],
        |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<i64>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, Option<i64>>(5)?,
            ))
        },
    )?;
    let note_count =
        connection.query_row("SELECT COUNT(*) FROM notes", [], |row| row.get::<_, i64>(0))?;
    let mut statement =
        connection.prepare("SELECT path, message FROM diagnostics ORDER BY ordinal")?;
    let diagnostics = statement
        .query_map([], |row| {
            Ok(Diagnostic {
                path: bytes_to_path(row.get(0)?),
                message: row.get(1)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(IndexMetadata {
        schema_version: row.0,
        parser_version: row.1,
        generation: row
            .2
            .map(u64::try_from)
            .transpose()
            .map_err(|error| StoreError::InvalidMetadata(error.to_string()))?,
        freshness: Freshness::parse(&row.3)?,
        note_count: u64::try_from(note_count)
            .map_err(|error| StoreError::InvalidMetadata(error.to_string()))?,
        completed_at_unix_ms: row
            .4
            .map(u64::try_from)
            .transpose()
            .map_err(|error| StoreError::InvalidMetadata(error.to_string()))?,
        observed_at_unix_ms: row
            .5
            .map(u64::try_from)
            .transpose()
            .map_err(|error| StoreError::InvalidMetadata(error.to_string()))?,
        diagnostics,
    })
}

fn validate_metadata(connection: &Connection) -> Result<()> {
    let (schema_version, parser_version, generation, freshness) = connection
        .query_row(
            "SELECT schema_version, parser_version, generation, freshness FROM metadata WHERE singleton = 1",
            [],
            |row| {
                Ok((
                    row.get::<_, u32>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| StoreError::InvalidMetadata("missing singleton metadata row".to_owned()))?;
    if schema_version != SCHEMA_VERSION {
        return Err(StoreError::UnsupportedSchema {
            found: schema_version,
            expected: SCHEMA_VERSION,
        });
    }
    if parser_version != PARSER_VERSION {
        return Err(StoreError::UnsupportedParser {
            found: parser_version,
            expected: PARSER_VERSION,
        });
    }
    let freshness = Freshness::parse(&freshness)?;
    if freshness == Freshness::Current && generation.is_none() {
        return Err(StoreError::InvalidMetadata(
            "current index metadata is missing its generation".to_owned(),
        ));
    }
    Ok(())
}

fn has_user_schema(connection: &Connection) -> Result<bool> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite_%')",
            [],
            |row| row.get(0),
        )
        .map_err(StoreError::from)
}

fn user_version(connection: &Connection) -> Result<u32> {
    connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(StoreError::from)
}

fn reset_legacy_cache(connection: &mut Connection) -> Result<()> {
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    match user_version(&transaction)? {
        LEGACY_SCHEMA_VERSION => {
            validate_legacy_schema(&transaction)?;
            transaction.execute_batch(
                "DROP TABLE IF EXISTS diagnostics;
                 DROP TABLE IF EXISTS notes;
                 DROP TABLE IF EXISTS metadata;",
            )?;
            transaction.execute_batch(SCHEMA)?;
        }
        SCHEMA_VERSION => {}
        found => {
            return Err(StoreError::UnsupportedSchema {
                found,
                expected: SCHEMA_VERSION,
            });
        }
    }
    transaction.commit()?;
    Ok(())
}

fn validate_legacy_schema(connection: &Connection) -> Result<()> {
    let tables = connection.query_row(
        "SELECT group_concat(name, ',') FROM (
            SELECT name FROM sqlite_schema
            WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
            ORDER BY name
        )",
        [],
        |row| row.get::<_, Option<String>>(0),
    )?;
    if tables.as_deref() != Some("diagnostics,metadata,notes") {
        return Err(StoreError::InvalidMetadata(
            "schema version 1 marker does not describe the recognized legacy cache".to_owned(),
        ));
    }
    let (schema_version, parser_version) = connection
        .query_row(
            "SELECT schema_version, parser_version FROM metadata WHERE singleton = 1",
            [],
            |row| Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?)),
        )
        .map_err(|error| {
            StoreError::InvalidMetadata(format!("invalid legacy metadata: {error}"))
        })?;
    if schema_version != LEGACY_SCHEMA_VERSION || parser_version != PARSER_VERSION {
        return Err(StoreError::InvalidMetadata(
            "legacy metadata version does not match its schema marker".to_owned(),
        ));
    }
    Ok(())
}

fn normalize_diagnostics(vault: &Vault, errors: &[IndexDiagnostic]) -> Vec<Diagnostic> {
    let mut diagnostics: Vec<_> = errors
        .iter()
        .map(|error| Diagnostic {
            path: error
                .path
                .strip_prefix(vault.root())
                .unwrap_or(&error.path)
                .to_path_buf(),
            message: error.message.clone(),
        })
        .collect();
    diagnostics.sort_by(|left, right| left.path.cmp(&right.path));
    diagnostics
}

fn now_unix_ms() -> Result<u64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| StoreError::InvalidMetadata(error.to_string()))?;
    u64::try_from(duration.as_millis())
        .map_err(|error| StoreError::InvalidMetadata(error.to_string()))
}

#[cfg(unix)]
fn path_to_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}

#[cfg(not(unix))]
fn path_to_bytes(path: &Path) -> Vec<u8> {
    path.to_string_lossy().as_bytes().to_vec()
}

#[cfg(unix)]
fn bytes_to_path(bytes: Vec<u8>) -> PathBuf {
    use std::os::unix::ffi::OsStringExt;
    std::ffi::OsString::from_vec(bytes).into()
}

#[cfg(not(unix))]
fn bytes_to_path(bytes: Vec<u8>) -> PathBuf {
    String::from_utf8_lossy(&bytes).into_owned().into()
}
