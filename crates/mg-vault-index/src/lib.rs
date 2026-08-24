use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use mg_vault_core::{IndexDiagnostic, IndexStatus, MarkdownIndex, SourceFingerprint, Vault};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;
pub const PARSER_VERSION: &str = "markdown-index-v1";

const SCHEMA: &str = r"
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
INSERT INTO metadata (
    singleton, schema_version, parser_version, generation, freshness, completed_at_unix_ms
) VALUES (1, 1, 'markdown-index-v1', NULL, 'empty', NULL);
PRAGMA user_version = 1;
";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    Empty,
    Current,
    Degraded,
}

impl Freshness {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Current => "current",
            Self::Degraded => "degraded",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "empty" => Ok(Self::Empty),
            "current" => Ok(Self::Current),
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
}

pub type Result<T> = std::result::Result<T, StoreError>;

pub struct PersistentIndexStore {
    connection: Connection,
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
    /// Returns an I/O or SQLite error, rejects unsupported schema versions,
    /// and fails closed when an unversioned partial user schema already exists.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| StoreError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let mut connection = Connection::open(path)?;
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
        } else if found != SCHEMA_VERSION {
            return Err(StoreError::UnsupportedSchema {
                found,
                expected: SCHEMA_VERSION,
            });
        }
        validate_metadata(&connection)?;
        Ok(Self { connection })
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
        self.rebuild_with_publication_guard(vault, || Ok(()))
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
        let mut projection = MarkdownIndex::new();
        projection.rebuild(vault);
        if let IndexStatus::Degraded { errors, .. } = projection.status() {
            let diagnostics = normalize_diagnostics(vault, errors);
            self.record_degraded(&diagnostics)?;
            return self.metadata();
        }

        let prior_generation = self.metadata()?.generation.unwrap_or(0);
        let generation = prior_generation
            .checked_add(1)
            .ok_or(StoreError::GenerationExhausted)?;
        let completed_at_unix_ms = now_unix_ms()?;
        let generation_sql =
            i64::try_from(generation).map_err(|_| StoreError::GenerationExhausted)?;
        let completed_at_sql = i64::try_from(completed_at_unix_ms)
            .map_err(|error| StoreError::InvalidMetadata(error.to_string()))?;
        let transaction = self.connection.transaction()?;
        stage_complete_projection(&transaction, &projection, generation)?;
        if let Err(message) = publication_guard() {
            transaction.rollback()?;
            self.record_degraded(&[Diagnostic {
                path: PathBuf::from("."),
                message: message.clone(),
            }])?;
            return Err(StoreError::PublicationAborted(message));
        }
        transaction.execute(
            "UPDATE metadata SET generation = ?1, freshness = ?2, completed_at_unix_ms = ?3, parser_version = ?4, schema_version = ?5 WHERE singleton = 1",
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
            .filter(|note| {
                note.path.to_string_lossy().to_lowercase().contains(&query)
                    || note.title.to_lowercase().contains(&query)
                    || note.text.to_lowercase().contains(&query)
                    || note
                        .outgoing_links
                        .iter()
                        .any(|link| link.to_lowercase().contains(&query))
            })
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
        let transaction = self.connection.transaction()?;
        transaction.execute("DELETE FROM diagnostics", [])?;
        {
            let mut statement = transaction
                .prepare("INSERT INTO diagnostics (ordinal, path, message) VALUES (?1, ?2, ?3)")?;
            for (ordinal, diagnostic) in diagnostics.iter().enumerate() {
                statement.execute(params![
                    i64::try_from(ordinal)
                        .map_err(|error| { StoreError::InvalidMetadata(error.to_string()) })?,
                    path_to_bytes(&diagnostic.path),
                    diagnostic.message,
                ])?;
            }
        }
        transaction.execute(
            "UPDATE metadata SET freshness = ?1 WHERE singleton = 1",
            [Freshness::Degraded.as_str()],
        )?;
        transaction.commit()?;
        Ok(())
    }
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

fn metadata(connection: &Connection) -> Result<IndexMetadata> {
    let row = connection.query_row(
        "SELECT schema_version, parser_version, generation, freshness, completed_at_unix_ms FROM metadata WHERE singleton = 1",
        [],
        |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<i64>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<i64>>(4)?,
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
