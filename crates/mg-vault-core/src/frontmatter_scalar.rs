use std::ops::Range;
use std::str::FromStr;

use yaml_edit::Document;

/// Fail-closed errors emitted while locating a top-level frontmatter scalar.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum FrontmatterScalarError {
    #[error("frontmatter is missing")]
    NoFrontmatter,
    #[error("frontmatter key `{0}` is missing")]
    MissingKey(String),
    #[error("frontmatter key `{0}` occurs more than once")]
    DuplicateKey(String),
    #[error("frontmatter key `{0}` does not contain a scalar")]
    NotScalar(String),
    #[error("frontmatter YAML is invalid: {0}")]
    InvalidYaml(String),
}

/// Locate the exact byte span of a top-level scalar value in frontmatter.
///
/// YAML is parsed with `yaml-edit` for validation and type checking. The
/// returned range is then found in the original source, so CRLF, comments,
/// quoting, and spacing remain untouched.
///
/// # Errors
///
/// Returns an error when frontmatter is absent, malformed, ambiguous, missing
/// the requested key, or when the requested value is not a scalar.
pub fn locate_frontmatter_scalar(
    source: &str,
    key: &str,
) -> Result<Range<usize>, FrontmatterScalarError> {
    let envelope = super::frontmatter::scan_frontmatter(source)
        .map_err(|error| FrontmatterScalarError::InvalidYaml(error.to_string()))?
        .ok_or(FrontmatterScalarError::NoFrontmatter)?;
    let yaml = &source[envelope.yaml_range()];

    let document = Document::from_str(yaml)
        .map_err(|error| FrontmatterScalarError::InvalidYaml(error.to_string()))?;
    let mapping = document
        .as_mapping()
        .ok_or_else(|| FrontmatterScalarError::InvalidYaml("root is not a mapping".to_owned()))?;
    let node = mapping
        .get(key)
        .ok_or_else(|| FrontmatterScalarError::MissingKey(key.to_owned()))?;
    if node.as_scalar().is_none() {
        return Err(FrontmatterScalarError::NotScalar(key.to_owned()));
    }

    let mut match_range = None;
    let mut matches = 0;
    let yaml_start = envelope.yaml_range().start;
    let mut line_start = 0;
    for line in yaml.split_inclusive(['\n']) {
        let content = line.strip_suffix('\n').unwrap_or(line);
        let content = content.strip_suffix('\r').unwrap_or(content);
        if !content.starts_with(char::is_whitespace)
            && !content.starts_with('#')
            && let Some(colon) = content.find(':')
            && content[..colon].trim() == key
        {
            matches += 1;
            let value_start = content[..colon].len() + 1 + content[colon + 1..].len()
                - content[colon + 1..].trim_start().len();
            let value = content[value_start..].trim_end();
            let value = strip_comment(value);
            let value_end = value.trim_end().len();
            if value_end > 0 {
                match_range = Some(
                    yaml_start + line_start + value_start
                        ..yaml_start + line_start + value_start + value_end,
                );
            }
        }
        line_start += line.len();
    }

    match matches {
        0 => Err(FrontmatterScalarError::MissingKey(key.to_owned())),
        1 => match_range.ok_or_else(|| FrontmatterScalarError::NotScalar(key.to_owned())),
        _ => Err(FrontmatterScalarError::DuplicateKey(key.to_owned())),
    }
}

fn strip_comment(value: &str) -> &str {
    let mut quote = None;
    for (index, character) in value.char_indices() {
        match (quote, character) {
            (None, '\'' | '"') => quote = Some(character),
            (Some(active), character) if character == active => quote = None,
            (None, '#') if index == 0 || value.as_bytes()[index - 1].is_ascii_whitespace() => {
                return &value[..index];
            }
            _ => {}
        }
    }
    value
}
