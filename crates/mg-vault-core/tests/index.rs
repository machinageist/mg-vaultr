use std::fs;

use mg_vault_core::{IndexStatus, MarkdownIndex, Vault};
use tempfile::TempDir;

fn fixture() -> (TempDir, Vault) {
    let directory = tempfile::tempdir().unwrap();
    let vault = Vault::open(directory.path()).unwrap();
    (directory, vault)
}

#[test]
fn rebuild_extracts_notes_titles_text_and_wikilinks() {
    let (_directory, vault) = fixture();
    vault
        .create_note(
            "zeta.md",
            b"---\ntitle: Zed\nunknown: [preserve]\n---\nBody [[Alpha|alias]] and [[Beta#Heading]].\n",
        )
        .unwrap();
    vault
        .create_note("nested/alpha.md", b"# Alpha\nplain text\n")
        .unwrap();

    let mut index = MarkdownIndex::new();
    assert_eq!(index.status(), &IndexStatus::Empty);
    index.rebuild(&vault);

    assert_eq!(index.status(), &IndexStatus::Current { generation: 1 });
    assert_eq!(index.notes().len(), 2);
    assert_eq!(index.notes()[1].path.to_string_lossy(), "zeta.md");
    assert_eq!(index.notes()[1].title, "Zed");
    assert!(index.notes()[1].text.contains("unknown: [preserve]"));
    assert_eq!(index.notes()[1].outgoing_links, ["Alpha", "Beta#Heading"]);
}

#[test]
fn rebuild_is_authoritative_for_changed_and_deleted_sources() {
    let (_directory, vault) = fixture();
    vault.create_note("old.md", b"old needle").unwrap();
    vault.create_note("changed.md", b"before").unwrap();
    let mut index = MarkdownIndex::new();
    index.rebuild(&vault);

    fs::remove_file(vault.root().join("old.md")).unwrap();
    fs::write(vault.root().join("changed.md"), b"new needle").unwrap();
    index.rebuild(&vault);

    assert_eq!(index.status(), &IndexStatus::Current { generation: 2 });
    assert!(
        index
            .notes()
            .iter()
            .all(|note| note.path != std::path::Path::new("old.md"))
    );
    assert!(index.search("before").is_empty());
    assert_eq!(
        index.search("new needle")[0].path,
        std::path::Path::new("changed.md")
    );
}

#[test]
fn search_order_is_deterministic_and_case_insensitive() {
    let (_directory, vault) = fixture();
    vault.create_note("b.md", b"needle").unwrap();
    vault.create_note("a.md", b"NEEDLE").unwrap();
    let mut index = MarkdownIndex::new();
    index.rebuild(&vault);

    let paths: Vec<_> = index
        .search("Needle")
        .into_iter()
        .map(|note| note.path.to_string_lossy().into_owned())
        .collect();
    assert_eq!(paths, ["a.md", "b.md"]);
}

#[test]
fn malformed_markdown_is_indexed_without_rewriting_source() {
    let (_directory, vault) = fixture();
    let source = b"---\ntitle: [broken\n---\n# Still searchable\nunknown [[Target]] syntax\n";
    vault.create_note("odd.md", source).unwrap();
    let mut index = MarkdownIndex::new();
    index.rebuild(&vault);

    assert!(matches!(index.status(), IndexStatus::Current { .. }));
    assert_eq!(index.notes()[0].text.as_bytes(), source);
    assert_eq!(fs::read(vault.root().join("odd.md")).unwrap(), source);
    assert_eq!(
        index.search("still searchable")[0].path,
        std::path::Path::new("odd.md")
    );
}

#[test]
fn empty_vault_is_current_after_rebuild() {
    let (_directory, vault) = fixture();
    let mut index = MarkdownIndex::new();
    index.rebuild(&vault);
    assert_eq!(index.status(), &IndexStatus::Current { generation: 1 });
    assert!(index.notes().is_empty());
    assert!(index.search("anything").is_empty());
}

#[test]
fn unreadable_utf8_source_is_reported_as_degraded_without_stale_results() {
    let (_directory, vault) = fixture();
    vault.create_note("good.md", b"good").unwrap();
    vault.create_note("bad.md", b"valid\xffbytes").unwrap();
    let mut index = MarkdownIndex::new();
    index.rebuild(&vault);

    assert!(
        matches!(index.status(), IndexStatus::Degraded { generation: 1, errors } if errors.len() == 1)
    );
    assert_eq!(index.notes().len(), 1);
    assert!(index.search("valid").is_empty());
}

#[cfg(unix)]
#[test]
fn symlinked_markdown_is_not_read_outside_the_vault() {
    use std::os::unix::fs::symlink;

    let (_directory, vault) = fixture();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret.md"), b"secret needle").unwrap();
    symlink(
        outside.path().join("secret.md"),
        vault.root().join("linked.md"),
    )
    .unwrap();

    let mut index = MarkdownIndex::new();
    index.rebuild(&vault);

    assert!(matches!(index.status(), IndexStatus::Current { .. }));
    assert!(index.notes().is_empty());
    assert!(index.search("secret").is_empty());
}
