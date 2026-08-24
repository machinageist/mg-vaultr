use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::atomic::{create_atomic, replace_atomic, sync_parent};
use crate::{Error, Result};

static TRASH_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Stable digest of the exact source bytes observed by a caller.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SourceFingerprint(String);

impl SourceFingerprint {
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        Self(format!("sha256:{:x}", Sha256::digest(bytes)))
    }
}

impl std::fmt::Display for SourceFingerprint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

impl std::str::FromStr for SourceFingerprint {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        let digest = value.strip_prefix("sha256:").unwrap_or_default();
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(Error::UnsafePath("invalid source fingerprint".into()));
        }
        Ok(Self(format!("sha256:{}", digest.to_ascii_lowercase())))
    }
}

/// Exact note bytes and the fingerprint that authorizes a later replacement.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Note {
    #[serde(skip)]
    pub bytes: Vec<u8>,
    pub content: String,
    pub fingerprint: SourceFingerprint,
}

/// Opaque handle returned by a successful trash operation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TrashReceipt {
    pub id: String,
    pub original_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct TrashMetadata {
    version: u8,
    original_path: PathBuf,
    fingerprint: SourceFingerprint,
}

/// Canonical authority boundary around one ordinary vault directory.
#[derive(Clone, Debug)]
pub struct Vault {
    root: PathBuf,
}

impl Vault {
    /// Open and canonicalize an ordinary vault directory.
    ///
    /// # Errors
    ///
    /// Returns an error when the path is inaccessible or not a directory.
    pub fn open(path: &Path) -> Result<Self> {
        let root = fs::canonicalize(path).map_err(|error| Error::io(path, error))?;
        if !fs::metadata(&root)
            .map_err(|error| Error::io(&root, error))?
            .is_dir()
        {
            return Err(Error::UnsafePath(format!(
                "{} is not a directory",
                root.display()
            )));
        }
        Ok(Self { root })
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Create a Markdown note without overwriting an existing path.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsafe path, collision, or failed durable write.
    pub fn create_note(
        &self,
        relative: impl AsRef<Path>,
        bytes: &[u8],
    ) -> Result<SourceFingerprint> {
        let relative = validate_note_path(relative.as_ref(), true)?;
        let destination = self.prepare_destination(&relative)?;
        create_atomic(&destination, bytes)?;
        Ok(SourceFingerprint::of(bytes))
    }

    /// Read exact Markdown bytes and return their source fingerprint.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsafe/non-file path, invalid UTF-8, or failed I/O.
    pub fn read_note(&self, relative: impl AsRef<Path>) -> Result<Note> {
        let relative = validate_note_path(relative.as_ref(), false)?;
        let lexical = self.root.join(&relative);
        let canonical = fs::canonicalize(&lexical).map_err(|error| Error::io(&lexical, error))?;
        ensure_beneath(&self.root, &canonical)?;
        let metadata = fs::metadata(&canonical).map_err(|error| Error::io(&canonical, error))?;
        if !metadata.is_file() {
            return Err(Error::UnsafePath(format!(
                "{} is not a file",
                relative.display()
            )));
        }
        let mut file = OpenOptions::new()
            .read(true)
            .open(&canonical)
            .map_err(|error| Error::io(&canonical, error))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|error| Error::io(&canonical, error))?;
        let content =
            String::from_utf8(bytes.clone()).map_err(|_| Error::InvalidUtf8(relative.clone()))?;
        Ok(Note {
            fingerprint: SourceFingerprint::of(&bytes),
            bytes,
            content,
        })
    }

    /// Atomically replace a note only if its fingerprint remains unchanged.
    ///
    /// # Errors
    ///
    /// Returns an error for unsafe paths, source conflicts, or failed durable I/O.
    pub fn write_note(
        &self,
        relative: impl AsRef<Path>,
        bytes: &[u8],
        expected: &SourceFingerprint,
    ) -> Result<SourceFingerprint> {
        let relative = validate_note_path(relative.as_ref(), true)?;
        let destination = self.existing_mutation_path(&relative)?;
        let current = fs::read(&destination).map_err(|error| Error::io(&destination, error))?;
        let actual = SourceFingerprint::of(&current);
        if &actual != expected {
            return Err(Error::Conflict {
                expected: expected.to_string(),
                actual: actual.to_string(),
            });
        }
        replace_atomic(&destination, bytes)?;
        Ok(SourceFingerprint::of(bytes))
    }

    /// Move a note into recoverable vault-local trash.
    ///
    /// # Errors
    ///
    /// Returns an error for unsafe paths or any move/metadata persistence failure.
    pub fn trash_note(&self, relative: impl AsRef<Path>) -> Result<TrashReceipt> {
        let relative = validate_note_path(relative.as_ref(), true)?;
        let source = self.existing_mutation_path(&relative)?;
        let bytes = fs::read(&source).map_err(|error| Error::io(&source, error))?;
        let id = trash_id();
        let files = self.internal_trash_dir().join("files");
        let entries = self.internal_trash_dir().join("entries");
        fs::create_dir_all(&files).map_err(|error| Error::io(&files, error))?;
        fs::create_dir_all(&entries).map_err(|error| Error::io(&entries, error))?;
        let payload = files.join(&id);
        let metadata_path = entries.join(format!("{id}.json"));
        fs::rename(&source, &payload).map_err(|error| Error::io(&source, error))?;
        let sync_result = sync_parent(&source).and_then(|()| sync_parent(&payload));
        if let Err(error) = sync_result {
            let _ = fs::rename(&payload, &source);
            let _ = sync_parent(&source);
            return Err(error);
        }
        let metadata = TrashMetadata {
            version: 1,
            original_path: relative.clone(),
            fingerprint: SourceFingerprint::of(&bytes),
        };
        let encoded = serde_json::to_vec_pretty(&metadata)?;
        if let Err(error) = create_atomic(&metadata_path, &encoded) {
            let _ = fs::rename(&payload, &source);
            let _ = sync_parent(&source);
            return Err(error);
        }
        Ok(TrashReceipt {
            id,
            original_path: relative,
        })
    }

    /// Restore a trashed note without overwriting a current note.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid metadata, collision, or failed durable I/O.
    pub fn restore_note(&self, id: &str) -> Result<TrashReceipt> {
        validate_trash_id(id)?;
        let payload = self.internal_trash_dir().join("files").join(id);
        let metadata_path = self
            .internal_trash_dir()
            .join("entries")
            .join(format!("{id}.json"));
        let encoded = fs::read(&metadata_path).map_err(|error| Error::io(&metadata_path, error))?;
        let metadata: TrashMetadata = serde_json::from_slice(&encoded)?;
        if metadata.version != 1 {
            return Err(Error::InvalidTrashEntry(id.to_owned()));
        }
        let destination =
            self.prepare_destination(&validate_note_path(&metadata.original_path, true)?)?;
        fs::hard_link(&payload, &destination).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                Error::Collision(destination.clone())
            } else {
                Error::io(&destination, error)
            }
        })?;
        sync_parent(&destination)?;
        fs::remove_file(&payload).map_err(|error| Error::io(&payload, error))?;
        fs::remove_file(&metadata_path).map_err(|error| Error::io(&metadata_path, error))?;
        sync_parent(&payload)?;
        sync_parent(&metadata_path)?;
        Ok(TrashReceipt {
            id: id.to_owned(),
            original_path: metadata.original_path,
        })
    }

    fn internal_trash_dir(&self) -> PathBuf {
        self.root.join(".mg-vault/trash")
    }

    fn prepare_destination(&self, relative: &Path) -> Result<PathBuf> {
        let parent_relative = relative.parent().unwrap_or_else(|| Path::new(""));
        let mut current = self.root.clone();
        for component in parent_relative.components() {
            let Component::Normal(name) = component else {
                return Err(Error::UnsafePath(relative.display().to_string()));
            };
            current.push(name);
            match fs::symlink_metadata(&current) {
                Ok(metadata) => {
                    if metadata.file_type().is_symlink() || !metadata.is_dir() {
                        return Err(Error::UnsafePath(current.display().to_string()));
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    fs::create_dir(&current).map_err(|source| Error::io(&current, source))?;
                    sync_parent(&current)?;
                }
                Err(error) => return Err(Error::io(&current, error)),
            }
            ensure_beneath(
                &self.root,
                &fs::canonicalize(&current).map_err(|e| Error::io(&current, e))?,
            )?;
        }
        Ok(self.root.join(relative))
    }

    fn existing_mutation_path(&self, relative: &Path) -> Result<PathBuf> {
        let lexical = self.root.join(relative);
        let metadata =
            fs::symlink_metadata(&lexical).map_err(|error| Error::io(&lexical, error))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(Error::UnsafePath(relative.display().to_string()));
        }
        let canonical = fs::canonicalize(&lexical).map_err(|error| Error::io(&lexical, error))?;
        ensure_beneath(&self.root, &canonical)?;
        Ok(lexical)
    }
}

fn validate_note_path(path: &Path, mutation: bool) -> Result<PathBuf> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(Error::UnsafePath(path.display().to_string()));
    }
    let mut components = path.components();
    let first = components.next();
    if mutation
        && matches!(
            first,
            Some(Component::Normal(name))
                if name == OsStr::new(".obsidian") || name == OsStr::new(".mg-vault")
        )
    {
        return Err(Error::UnsafePath(path.display().to_string()));
    }
    if !path
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
        || path.extension() != Some(OsStr::new("md"))
    {
        return Err(Error::UnsafePath(path.display().to_string()));
    }
    Ok(path.to_path_buf())
}

fn ensure_beneath(root: &Path, candidate: &Path) -> Result<()> {
    if candidate.starts_with(root) && candidate != root {
        Ok(())
    } else {
        Err(Error::UnsafePath(candidate.display().to_string()))
    }
}

fn trash_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let serial = TRASH_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{nanos:032x}-{serial:016x}")
}

fn validate_trash_id(id: &str) -> Result<()> {
    if id.len() == 49
        && id.as_bytes()[32] == b'-'
        && id
            .bytes()
            .enumerate()
            .all(|(index, byte)| index == 32 || byte.is_ascii_hexdigit())
    {
        Ok(())
    } else {
        Err(Error::InvalidTrashEntry(id.to_owned()))
    }
}
