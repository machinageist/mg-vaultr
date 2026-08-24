use mg_vault_core::{FrontmatterScalarError, locate_frontmatter_scalar};

#[test]
fn locates_only_the_existing_scalar_token() {
    let source = "---\n# keep this comment\ntitle:  'Old title' # and this one\nunknown:  keep  spacing\n---\nBody [[Link]]\n";

    let span = locate_frontmatter_scalar(source, "title").unwrap();

    assert_eq!(&source[span], "'Old title'");
}

#[test]
fn translates_yaml_offsets_across_crlf_frontmatter() {
    let source = "---\r\ntitle: \"Old\"\r\nunknown: true\r\n---\r\nBody\r\n";

    let span = locate_frontmatter_scalar(source, "title").unwrap();

    assert_eq!(&source[span], "\"Old\"");
}

#[test]
fn ignores_nested_keys_when_locating_a_top_level_property() {
    let source = "---\nparent:\n  title: nested\ntitle: top-level\n---\n";

    let span = locate_frontmatter_scalar(source, "title").unwrap();

    assert_eq!(&source[span], "top-level");
}

#[test]
fn rejects_duplicate_target_keys() {
    let source = "---\ntitle: first\ntitle: second\n---\n";

    assert_eq!(
        locate_frontmatter_scalar(source, "title"),
        Err(FrontmatterScalarError::DuplicateKey("title".to_owned()))
    );
}

#[test]
fn rejects_missing_frontmatter_and_missing_keys() {
    assert_eq!(
        locate_frontmatter_scalar("Body only\n", "title"),
        Err(FrontmatterScalarError::NoFrontmatter)
    );
    assert_eq!(
        locate_frontmatter_scalar("---\nother: value\n---\n", "title"),
        Err(FrontmatterScalarError::MissingKey("title".to_owned()))
    );
}

#[test]
fn rejects_non_scalar_targets_and_malformed_yaml() {
    assert_eq!(
        locate_frontmatter_scalar("---\ntitle: [one, two]\n---\n", "title"),
        Err(FrontmatterScalarError::NotScalar("title".to_owned()))
    );
    assert!(matches!(
        locate_frontmatter_scalar("---\ntitle: [unterminated\n---\n", "title"),
        Err(FrontmatterScalarError::InvalidYaml(_))
    ));
}
