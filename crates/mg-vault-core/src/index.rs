use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::{SourceFingerprint, Vault};

/// The freshness of the in-memory projection relative to its source vault.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IndexStatus {
    /// No rebuild has been completed yet.
    Empty,
    /// The projection was rebuilt from a complete filesystem walk.
    Current { generation: u64 },
    /// Some source files could not be read, so results are incomplete.
    Degraded {
        generation: u64,
        errors: Vec<IndexDiagnostic>,
    },
}

/// A source-file problem that prevents a complete current projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexDiagnostic {
    pub path: PathBuf,
    pub message: String,
}

/// One immutable observation of a Markdown source file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexedNote {
    pub path: PathBuf,
    pub title: String,
    pub text: String,
    pub fingerprint: SourceFingerprint,
    pub outgoing_links: Vec<String>,
}

/// A deterministic, disposable projection of Markdown files in one vault.
///
/// The vault files remain authoritative: this type stores only derived values,
/// and [`Self::rebuild`] always replaces the projection from a fresh walk.
#[derive(Clone, Debug)]
pub struct MarkdownIndex {
    status: IndexStatus,
    notes: Vec<IndexedNote>,
}

impl Default for MarkdownIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownIndex {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            status: IndexStatus::Empty,
            notes: Vec::new(),
        }
    }

    #[must_use]
    pub const fn status(&self) -> &IndexStatus {
        &self.status
    }

    #[must_use]
    pub fn notes(&self) -> &[IndexedNote] {
        &self.notes
    }

    /// Rebuild the complete projection from the current `.md` files.
    ///
    /// Files that cannot be decoded as UTF-8 are omitted and make the result
    /// explicitly degraded; no previous entry is retained for a deleted or
    /// unreadable source. Directory traversal is lexical and deterministic.
    pub fn rebuild(&mut self, vault: &Vault) -> &IndexStatus {
        let mut paths = Vec::new();
        let mut errors = Vec::new();
        collect_markdown_paths(vault.root(), vault.root(), &mut paths, &mut errors);
        paths.sort();

        let mut notes = Vec::new();
        for path in paths {
            match read_indexed_note(vault.root(), &path) {
                Ok(note) => notes.push(note),
                Err(message) => errors.push(IndexDiagnostic { path, message }),
            }
        }
        notes.sort_by(|left, right| left.path.cmp(&right.path));
        let generation = match &self.status {
            IndexStatus::Empty => 1,
            IndexStatus::Current { generation } | IndexStatus::Degraded { generation, .. } => {
                generation.saturating_add(1)
            }
        };
        self.notes = notes;
        self.status = if errors.is_empty() {
            IndexStatus::Current { generation }
        } else {
            IndexStatus::Degraded { generation, errors }
        };
        &self.status
    }

    /// Return notes whose path, title, text, or links contain `query`.
    /// Results are always ordered by normalized relative path.
    #[must_use]
    pub fn search(&self, query: &str) -> Vec<&IndexedNote> {
        let query = query.to_lowercase();
        let mut results: Vec<_> = self
            .notes
            .iter()
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
        results.sort_by(|left, right| left.path.cmp(&right.path));
        results
    }
}

fn collect_markdown_paths(
    root: &Path,
    directory: &Path,
    paths: &mut Vec<PathBuf>,
    errors: &mut Vec<IndexDiagnostic>,
) {
    let read_entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => {
            errors.push(IndexDiagnostic {
                path: directory
                    .strip_prefix(root)
                    .unwrap_or(directory)
                    .to_path_buf(),
                message: error.to_string(),
            });
            return;
        }
    };
    let mut entries = Vec::new();
    for entry in read_entries {
        match entry {
            Ok(entry) => entries.push(entry),
            Err(error) => errors.push(IndexDiagnostic {
                path: directory
                    .strip_prefix(root)
                    .unwrap_or(directory)
                    .to_path_buf(),
                message: error.to_string(),
            }),
        }
    }
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) => {
                errors.push(IndexDiagnostic {
                    path: path.strip_prefix(root).unwrap_or(&path).to_path_buf(),
                    message: error.to_string(),
                });
                continue;
            }
        };
        if file_type.is_dir() {
            if entry.file_name() != ".obsidian" && entry.file_name() != ".mg-vault" {
                collect_markdown_paths(root, &path, paths, errors);
            }
        } else if file_type.is_file() && path.extension().is_some_and(|extension| extension == "md")
        {
            paths.push(path);
        }
    }
}

fn read_indexed_note(root: &Path, path: &Path) -> Result<IndexedNote, String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|error| error.to_string())?
        .to_path_buf();
    let bytes = read_source_bytes(root, &relative)?;
    let confirmation = read_source_bytes(root, &relative)?;
    if bytes != confirmation {
        return Err("source changed during indexing; retry rebuild".to_owned());
    }
    let text = String::from_utf8(bytes.clone()).map_err(|error| error.to_string())?;
    let title = title_for(&text, &relative);
    Ok(IndexedNote {
        path: relative,
        title,
        text: text.clone(),
        fingerprint: SourceFingerprint::of(&bytes),
        outgoing_links: wikilinks(&text),
    })
}

#[cfg(target_os = "linux")]
fn read_source_bytes(root: &Path, relative: &Path) -> Result<Vec<u8>, String> {
    use rustix::fs::{Mode, OFlags, ResolveFlags, open, openat2};

    let root_fd = open(root, OFlags::RDONLY | OFlags::DIRECTORY, Mode::empty())
        .map_err(|error| error.to_string())?;
    let file_fd = openat2(
        &root_fd,
        relative,
        OFlags::RDONLY,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS,
    )
    .map_err(|error| error.to_string())?;
    let mut file = File::from(file_fd);
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}

#[cfg(not(target_os = "linux"))]
fn read_source_bytes(root: &Path, relative: &Path) -> Result<Vec<u8>, String> {
    let path = root.join(relative);
    let canonical = fs::canonicalize(&path).map_err(|error| error.to_string())?;
    let canonical_root = fs::canonicalize(root).map_err(|error| error.to_string())?;
    if !canonical.starts_with(&canonical_root) {
        return Err("source path escapes the vault".to_owned());
    }
    fs::read(canonical).map_err(|error| error.to_string())
}

fn title_for(text: &str, path: &Path) -> String {
    if let Ok(range) = crate::locate_frontmatter_scalar(text, "title") {
        let value = text[range].trim();
        if !value.is_empty() {
            return value.trim_matches(['\'', '"']).to_owned();
        }
    }
    text.lines()
        .find_map(|line| {
            line.strip_prefix("# ")
                .map(str::trim)
                .filter(|title| !title.is_empty())
        })
        .map_or_else(
            || {
                path.file_stem()
                    .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned())
            },
            ToOwned::to_owned,
        )
}

fn wikilinks(text: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut remaining = text;
    while let Some(start) = remaining.find("[[") {
        let after = &remaining[start + 2..];
        let Some(end) = after.find("]]") else { break };
        let target = after[..end].split('|').next().unwrap_or_default().trim();
        if !target.is_empty() {
            links.push(target.to_owned());
        }
        remaining = &after[end + 2..];
    }
    links
}
