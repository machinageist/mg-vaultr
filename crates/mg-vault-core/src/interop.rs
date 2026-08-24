use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::index::{IndexDiagnostic, read_indexed_note};
use crate::{Result, Vault};

const INTEROP_SCHEMA: &str = "mg.interop/1";
const PRODUCER_APP: &str = "mg-vault";
const PRODUCER_VERSION: &str = env!("CARGO_PKG_VERSION");
const EPOCH: &str = "1970-01-01T00:00:00Z";

/// Export the authoritative Markdown sources as a deterministic read-only snapshot.
///
/// The export never writes the vault or its derived index. Filesystem metadata does
/// not contain a trustworthy creation time in the current authority model, so the
/// epoch is used explicitly rather than inventing a timestamp.
#[allow(clippy::too_many_lines)]
///
/// # Errors
///
/// Returns a serialization error if the deterministic envelope cannot be encoded.
///
/// # Panics
///
/// Panics only if an indexed note contains a non-UTF-8 path after path collection,
/// which would violate the collector's fail-closed invariant.
pub fn export_snapshot(vault: &Vault) -> Result<Value> {
    let vault_namespace = vault_namespace(vault);
    let mut paths = Vec::new();
    let mut diagnostics = Vec::new();
    collect_paths(vault.root(), vault.root(), &mut paths, &mut diagnostics);
    paths.sort();

    let mut notes = Vec::new();
    for path in paths {
        match read_indexed_note(vault.root(), &path) {
            Ok(note) => notes.push(note),
            Err(message) => diagnostics.push(IndexDiagnostic { path, message }),
        }
    }
    notes.sort_by_key(|left| {
        normalized_path(&left.path).expect("indexed notes must have UTF-8 paths")
    });
    let paths: Vec<String> = notes
        .iter()
        .map(|note| normalized_path(&note.path).expect("indexed notes must have UTF-8 paths"))
        .collect();

    let records: Vec<Value> = notes
        .iter()
        .map(|note| {
            let path = normalized_path(&note.path).expect("indexed notes must have UTF-8 paths");
            let fingerprint = note.fingerprint.to_string();
            json!({
                "global_id": note_global_id(&vault_namespace, &path),
                "origin": {"app": PRODUCER_APP, "kind": "note", "local_id": path},
                "revision": fingerprint,
                "observed_at": EPOCH,
                "payload": {
                    "path": path,
                    "title": note.title,
                    "text": note.text,
                    "wikilinks": note.outgoing_links,
                    "fingerprint": fingerprint,
                }
            })
        })
        .collect();

    let mut links = Vec::new();
    for note in &notes {
        let source_path = normalized_path(&note.path).expect("indexed notes must have UTF-8 paths");
        let source = note_global_id(&vault_namespace, &source_path);
        for (occurrence, target) in note.outgoing_links.iter().enumerate() {
            let resolution = resolve_target(target, &paths);
            let target_path = resolution.path();
            let target_id = note_global_id(
                &vault_namespace,
                target_path.map_or_else(|| target.to_owned(), Clone::clone),
            );
            let resolved = target_path.is_some();
            links.push(json!({
                "link_id": link_id(&vault_namespace, &source, &target_id, occurrence),
                "occurrence": occurrence,
                "source_global_id": source,
                "target_global_id": target_id,
                "relation": "wikilink",
                "created_by": PRODUCER_APP,
                "created_at": Value::Null,
                "resolved": resolved,
                "provenance": "derived from authoritative Markdown Wikilink; creation time unavailable"
            }));
            if !resolved {
                diagnostics.push(IndexDiagnostic {
                    path: note.path.clone(),
                    message: resolution.diagnostic(target),
                });
            }
        }
    }
    links.sort_by(|left, right| {
        left["source_global_id"]
            .as_str()
            .cmp(&right["source_global_id"].as_str())
            .then(
                left["occurrence"]
                    .as_u64()
                    .cmp(&right["occurrence"].as_u64()),
            )
    });
    diagnostics.sort_by(|left, right| {
        diagnostic_path(&left.path)
            .cmp(&diagnostic_path(&right.path))
            .then(left.message.cmp(&right.message))
    });

    let diagnostics_json: Vec<Value> = diagnostics
        .iter()
        .map(|diagnostic| {
            json!({
                "severity": "warning",
                "code": if diagnostic.message.starts_with("unresolved Wikilink") {
                    "unresolved_wikilink"
                } else if diagnostic.message.starts_with("ambiguous Wikilink") {
                    "ambiguous_wikilink"
                } else {
                    "index"
                },
                "path": diagnostic_path(&diagnostic.path),
                "message": diagnostic.message.replace(['\n', '\r'], " ")
            })
        })
        .collect();
    let degraded = !diagnostics_json.is_empty();
    let provenance = vec![json!({
        "source": PRODUCER_APP,
        "boundary": "authoritative Markdown files under the selected vault root",
        "read_only": true
    })];
    let created_at = EPOCH;
    let identity = json!({
        "interop_schema": INTEROP_SCHEMA,
        "kind": "snapshot",
        "producer": {"app": PRODUCER_APP, "app_version": PRODUCER_VERSION},
        "vault_namespace": vault_namespace,
        "created_at": created_at,
        "records": records,
        "links": links,
        "provenance": provenance,
        "degraded": degraded,
        "diagnostics": diagnostics_json
    });
    let digest = hex_digest(&serde_json::to_vec(&identity)?);
    Ok(json!({
        "interop_schema": INTEROP_SCHEMA,
        "kind": "snapshot",
        "producer": {"app": PRODUCER_APP, "app_version": PRODUCER_VERSION},
        "vault_namespace": identity["vault_namespace"],
        "export_id": format!("{PRODUCER_APP}:snapshot:{digest}"),
        "created_at": created_at,
        "source_revision": format!("sha256:{digest}"),
        "canonical_hash": format!("sha256:{digest}"),
        "records": identity["records"],
        "links": identity["links"],
        "provenance": identity["provenance"],
        "degraded": identity["degraded"],
        "diagnostics": identity["diagnostics"]
    }))
}

fn collect_paths(
    root: &Path,
    directory: &Path,
    paths: &mut Vec<PathBuf>,
    diagnostics: &mut Vec<IndexDiagnostic>,
) {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => {
            diagnostics.push(IndexDiagnostic {
                path: relative(root, directory),
                message: error.to_string(),
            });
            return;
        }
    };
    let mut readable_entries = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => readable_entries.push(entry),
            Err(error) => diagnostics.push(IndexDiagnostic {
                path: relative(root, directory),
                message: format!("failed to read directory entry: {error}"),
            }),
        }
    }
    let mut entries = readable_entries;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(kind) => kind,
            Err(error) => {
                diagnostics.push(IndexDiagnostic {
                    path: relative(root, &path),
                    message: error.to_string(),
                });
                continue;
            }
        };
        let name = entry.file_name();
        if name.to_str().is_none() {
            diagnostics.push(IndexDiagnostic {
                path: relative(root, &path),
                message: "non-UTF-8 path is not exportable".to_owned(),
            });
            continue;
        }
        if file_type.is_dir() {
            if !matches!(
                name.to_str(),
                Some(".mg-vault" | ".obsidian" | "private" | ".private" | "secrets" | ".secrets")
            ) {
                collect_paths(root, &path, paths, diagnostics);
            }
        } else if file_type.is_file() && path.extension().is_some_and(|extension| extension == "md")
        {
            paths.push(path);
        }
    }
}

fn relative(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

fn normalized_path(path: &Path) -> Option<String> {
    path.to_str().map(|path| {
        #[cfg(windows)]
        {
            path.replace('\\', "/")
        }
        #[cfg(not(windows))]
        {
            path.to_owned()
        }
    })
}

fn vault_namespace(vault: &Vault) -> String {
    format!("sha256:{}", hex_digest(&vault_root_bytes(vault.root())))
}

#[cfg(unix)]
fn vault_root_bytes(root: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;

    root.as_os_str().as_bytes().to_vec()
}

#[cfg(windows)]
fn vault_root_bytes(root: &Path) -> Vec<u8> {
    use std::os::windows::ffi::OsStrExt;

    root.as_os_str()
        .encode_wide()
        .flat_map(u16::to_le_bytes)
        .collect()
}

#[cfg(not(any(unix, windows)))]
fn vault_root_bytes(root: &Path) -> Vec<u8> {
    root.as_os_str().to_string_lossy().into_owned().into_bytes()
}

fn note_global_id(namespace: &str, path: impl AsRef<str>) -> String {
    format!(
        "{PRODUCER_APP}:vault:{namespace}:note:path:{}",
        path.as_ref()
    )
}

fn link_id(namespace: &str, source_id: &str, target_id: &str, occurrence: usize) -> String {
    let mut identity = Vec::new();
    for field in [
        namespace.as_bytes(),
        source_id.as_bytes(),
        target_id.as_bytes(),
    ] {
        identity.extend_from_slice(&(field.len() as u64).to_be_bytes());
        identity.extend_from_slice(field);
    }
    identity.extend_from_slice(&(occurrence as u64).to_be_bytes());
    format!("{PRODUCER_APP}:wikilink:sha256:{}", hex_digest(&identity))
}

fn diagnostic_path(path: &Path) -> String {
    normalized_path(path).unwrap_or_else(|| "<non-UTF-8 path>".to_owned())
}

enum TargetResolution {
    Resolved(String),
    Unresolved,
    Ambiguous(Vec<String>),
}

impl TargetResolution {
    fn path(&self) -> Option<&String> {
        match self {
            Self::Resolved(path) => Some(path),
            Self::Unresolved | Self::Ambiguous(_) => None,
        }
    }

    fn diagnostic(&self, target: &str) -> String {
        match self {
            Self::Resolved(_) => unreachable!("resolved targets do not have diagnostics"),
            Self::Unresolved => format!("unresolved Wikilink target {target:?}"),
            Self::Ambiguous(candidates) => format!(
                "ambiguous Wikilink target {target:?}; candidates: {}",
                candidates.join(", ")
            ),
        }
    }
}

fn resolve_target(target: &str, paths: &[String]) -> TargetResolution {
    let target = target.split('#').next().unwrap_or(target).trim();
    let target = target.strip_suffix(".md").unwrap_or(target);
    let exact: Vec<_> = paths
        .iter()
        .filter(|path| path.strip_suffix(".md").unwrap_or(path) == target)
        .cloned()
        .collect();
    if exact.len() == 1 {
        return TargetResolution::Resolved(exact[0].clone());
    }
    if exact.len() > 1 {
        return TargetResolution::Ambiguous(exact);
    }
    let basename: Vec<_> = paths
        .iter()
        .filter(|path| {
            let stem = path.strip_suffix(".md").unwrap_or(path);
            Path::new(stem)
                .file_name()
                .is_some_and(|name| name == target)
        })
        .cloned()
        .collect();
    match basename.as_slice() {
        [path] => TargetResolution::Resolved(path.clone()),
        [] => TargetResolution::Unresolved,
        _ => TargetResolution::Ambiguous(basename),
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn export_is_deterministic_and_keeps_unresolved_links() {
        let directory = tempdir().unwrap();
        let vault = Vault::open(directory.path()).unwrap();
        vault
            .create_note("nested/a.md", b"# A\n[[b]] [[missing#Heading]]")
            .unwrap();
        vault.create_note("b.md", b"B").unwrap();
        let first = export_snapshot(&vault).unwrap();
        let second = export_snapshot(&vault).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            first["records"][0]["global_id"],
            note_global_id(&vault_namespace(&vault), "b.md")
        );
        assert!(
            first["links"]
                .as_array()
                .unwrap()
                .iter()
                .any(|link| link["resolved"] == false)
        );
    }

    #[test]
    fn ambiguous_basename_stays_unresolved_with_diagnostic() {
        let paths = vec!["one/topic.md".to_owned(), "two/topic.md".to_owned()];
        let resolution = resolve_target("topic", &paths);
        assert!(resolution.path().is_none());
        assert_eq!(
            resolution.diagnostic("topic"),
            "ambiguous Wikilink target \"topic\"; candidates: one/topic.md, two/topic.md"
        );
    }

    #[test]
    fn repeated_wikilinks_have_distinct_stable_ids() {
        let directory = tempdir().unwrap();
        let vault = Vault::open(directory.path()).unwrap();
        vault.create_note("a.md", b"[[b]] [[b]] [[b]]").unwrap();
        vault.create_note("b.md", b"B").unwrap();

        let links = export_snapshot(&vault).unwrap()["links"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(links.len(), 3);
        let ids: Vec<_> = links
            .iter()
            .map(|link| link["link_id"].as_str().unwrap())
            .collect();
        let namespace = vault_namespace(&vault);
        let source = note_global_id(&namespace, "a.md");
        let target = note_global_id(&namespace, "b.md");
        assert_eq!(
            ids,
            vec![
                link_id(&namespace, &source, &target, 0),
                link_id(&namespace, &source, &target, 1),
                link_id(&namespace, &source, &target, 2),
            ]
        );
    }

    #[test]
    fn vault_namespace_prevents_same_path_collisions_between_vaults() {
        let first_directory = tempdir().unwrap();
        let second_directory = tempdir().unwrap();
        let first = Vault::open(first_directory.path()).unwrap();
        let second = Vault::open(second_directory.path()).unwrap();
        first.create_note("same.md", b"first").unwrap();
        second.create_note("same.md", b"second").unwrap();

        let first_snapshot = export_snapshot(&first).unwrap();
        let second_snapshot = export_snapshot(&second).unwrap();
        assert_ne!(
            first_snapshot["vault_namespace"],
            second_snapshot["vault_namespace"]
        );
        assert_ne!(
            first_snapshot["records"][0]["global_id"],
            second_snapshot["records"][0]["global_id"]
        );
        assert!(
            !first_snapshot["records"][0]["global_id"]
                .as_str()
                .unwrap()
                .contains(first.root().to_string_lossy().as_ref())
        );
    }

    #[test]
    fn delimiter_containing_paths_cannot_collide_in_wikilink_ids() {
        let directory = tempdir().unwrap();
        let vault = Vault::open(directory.path()).unwrap();
        vault.create_note("a.md", b"[[b--wikilink--c]]").unwrap();
        vault.create_note("a--wikilink--b.md", b"[[c]]").unwrap();
        vault
            .create_note("b--wikilink--c.md", b"target one")
            .unwrap();
        vault.create_note("c.md", b"target two").unwrap();

        let links = export_snapshot(&vault).unwrap()["links"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(links.len(), 2);
        let ids: std::collections::HashSet<_> = links
            .iter()
            .map(|link| link["link_id"].as_str().unwrap())
            .collect();
        assert_eq!(ids.len(), 2);
        assert!(
            ids.iter()
                .all(|id| id.starts_with("mg-vault:wikilink:sha256:"))
        );
        assert!(ids.iter().all(|id| !id.contains("wikilink--")));
    }

    #[cfg(unix)]
    #[test]
    fn unix_backslashes_remain_distinct_path_identity() {
        let directory = tempdir().unwrap();
        let vault = Vault::open(directory.path()).unwrap();
        vault.create_note("a\\b.md", b"backslash").unwrap();
        vault.create_note("a/b.md", b"slash").unwrap();

        let records = export_snapshot(&vault).unwrap()["records"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(records.len(), 2);
        let paths: Vec<_> = records
            .iter()
            .map(|record| record["payload"]["path"].as_str().unwrap())
            .collect();
        assert!(paths.contains(&"a\\b.md"));
        assert!(paths.contains(&"a/b.md"));
        assert_ne!(records[0]["global_id"], records[1]["global_id"]);
    }

    #[cfg(unix)]
    #[test]
    fn unix_non_utf8_vault_roots_have_distinct_namespaces() {
        use std::os::unix::ffi::OsStringExt;

        let parent = tempdir().unwrap();
        let first_root = parent
            .path()
            .join(std::ffi::OsString::from_vec(b"vault-\x80".to_vec()));
        let second_root = parent
            .path()
            .join(std::ffi::OsString::from_vec(b"vault-\x81".to_vec()));
        fs::create_dir(&first_root).unwrap();
        fs::create_dir(&second_root).unwrap();
        let first = Vault::open(&first_root).unwrap();
        let second = Vault::open(&second_root).unwrap();

        assert_ne!(vault_namespace(&first), vault_namespace(&second));
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_paths_are_excluded_with_a_degraded_diagnostic() {
        use std::os::unix::ffi::OsStringExt;

        let directory = tempdir().unwrap();
        let vault = Vault::open(directory.path()).unwrap();
        let invalid_name = std::ffi::OsString::from_vec(b"bad-\xff.md".to_vec());
        fs::write(directory.path().join(invalid_name), b"not exportable").unwrap();

        let snapshot = export_snapshot(&vault).unwrap();
        assert!(snapshot["records"].as_array().unwrap().is_empty());
        assert_eq!(snapshot["degraded"], true);
        assert!(
            snapshot["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|diagnostic| diagnostic["message"] == "non-UTF-8 path is not exportable")
        );
    }
}
