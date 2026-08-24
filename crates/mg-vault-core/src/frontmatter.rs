use std::ops::Range;

/// Line-ending pattern observed in a frontmatter envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineEndings {
    Lf,
    CrLf,
    Mixed,
}

/// Exact source spans for a YAML frontmatter envelope and Markdown body.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontmatterEnvelope {
    yaml: Range<usize>,
    body: Range<usize>,
    line_endings: LineEndings,
}

impl FrontmatterEnvelope {
    #[must_use]
    pub fn yaml_range(&self) -> Range<usize> {
        self.yaml.clone()
    }

    #[must_use]
    pub fn body_range(&self) -> Range<usize> {
        self.body.clone()
    }

    #[must_use]
    pub const fn line_endings(&self) -> LineEndings {
        self.line_endings
    }
}

/// Fail-closed errors emitted while locating a frontmatter envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum FrontmatterError {
    #[error("UTF-8 BOM before frontmatter is not supported")]
    UnsupportedBom,
    #[error("frontmatter opening delimiter has no closing delimiter")]
    Unclosed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Eol {
    Lf,
    CrLf,
}

/// Locate exact frontmatter and body byte spans without normalizing source.
///
/// Only an exact `---` first line is treated as an opening delimiter. The
/// closing delimiter must also be an exact `---` line. Missing or ambiguous
/// delimiters fail closed rather than guessing an editable YAML region.
///
/// # Errors
///
/// Returns [`FrontmatterError::UnsupportedBom`] when a BOM precedes the first
/// line, or [`FrontmatterError::Unclosed`] when an opening delimiter has no
/// exact closing line.
pub fn scan_frontmatter(source: &str) -> Result<Option<FrontmatterEnvelope>, FrontmatterError> {
    if source.starts_with('\u{feff}') {
        return Err(FrontmatterError::UnsupportedBom);
    }

    let (yaml_start, opening_eol) = if source.starts_with("---\r\n") {
        (5, Eol::CrLf)
    } else if source.starts_with("---\n") {
        (4, Eol::Lf)
    } else if source == "---" {
        return Err(FrontmatterError::Unclosed);
    } else {
        return Ok(None);
    };

    let bytes = source.as_bytes();
    let mut cursor = yaml_start;
    let mut observed = opening_eol;
    let mut mixed = false;

    while cursor < bytes.len() {
        let (content_end, line_end, eol) = next_line(bytes, cursor);
        if let Some(eol) = eol {
            mixed |= eol != observed;
            observed = eol;
        }
        if &source[cursor..content_end] == "---" {
            return Ok(Some(FrontmatterEnvelope {
                yaml: yaml_start..cursor,
                body: line_end..source.len(),
                line_endings: if mixed {
                    LineEndings::Mixed
                } else {
                    match observed {
                        Eol::Lf => LineEndings::Lf,
                        Eol::CrLf => LineEndings::CrLf,
                    }
                },
            }));
        }
        cursor = line_end;
    }

    Err(FrontmatterError::Unclosed)
}

fn next_line(bytes: &[u8], start: usize) -> (usize, usize, Option<Eol>) {
    let Some(relative_lf) = bytes[start..].iter().position(|byte| *byte == b'\n') else {
        return (bytes.len(), bytes.len(), None);
    };
    let lf = start + relative_lf;
    if lf > start && bytes[lf - 1] == b'\r' {
        (lf - 1, lf + 1, Some(Eol::CrLf))
    } else {
        (lf, lf + 1, Some(Eol::Lf))
    }
}

#[cfg(test)]
mod tests {
    use super::{FrontmatterError, LineEndings, scan_frontmatter};

    #[test]
    fn locates_lf_yaml_and_body_without_changing_offsets() {
        let source = "---\ntitle: Old\nunknown:  keep\n---\n# Body\n";
        let envelope = scan_frontmatter(source).unwrap().unwrap();

        assert_eq!(
            &source[envelope.yaml_range()],
            "title: Old\nunknown:  keep\n"
        );
        assert_eq!(&source[envelope.body_range()], "# Body\n");
        assert_eq!(envelope.line_endings(), LineEndings::Lf);
    }

    #[test]
    fn locates_crlf_yaml_and_body_without_normalization() {
        let source = "---\r\ntitle: Old\r\n---\r\nBody\r\n";
        let envelope = scan_frontmatter(source).unwrap().unwrap();

        assert_eq!(&source[envelope.yaml_range()], "title: Old\r\n");
        assert_eq!(&source[envelope.body_range()], "Body\r\n");
        assert_eq!(envelope.line_endings(), LineEndings::CrLf);
    }

    #[test]
    fn reports_mixed_line_endings_without_normalization() {
        let source = "---\r\ntitle: Old\n---\r\nBody";
        let envelope = scan_frontmatter(source).unwrap().unwrap();

        assert_eq!(&source[envelope.yaml_range()], "title: Old\n");
        assert_eq!(&source[envelope.body_range()], "Body");
        assert_eq!(envelope.line_endings(), LineEndings::Mixed);
    }

    #[test]
    fn ignores_non_delimiter_lines_and_leading_whitespace() {
        assert_eq!(scan_frontmatter(" ---\ntitle: Old\n---\n").unwrap(), None);
        assert_eq!(scan_frontmatter("---not yaml\nbody\n").unwrap(), None);
    }

    #[test]
    fn delimiter_like_yaml_lines_do_not_close_the_envelope() {
        let source = "---\nvalue: ---x\n---\nbody";
        let envelope = scan_frontmatter(source).unwrap().unwrap();

        assert_eq!(&source[envelope.yaml_range()], "value: ---x\n");
        assert_eq!(&source[envelope.body_range()], "body");
    }

    #[test]
    fn accepts_a_closing_delimiter_at_end_of_file() {
        let source = "---\ntitle: Old\n---";
        let envelope = scan_frontmatter(source).unwrap().unwrap();

        assert_eq!(&source[envelope.yaml_range()], "title: Old\n");
        assert_eq!(&source[envelope.body_range()], "");
    }

    #[test]
    fn rejects_bom_and_unclosed_envelopes() {
        assert_eq!(
            scan_frontmatter("\u{feff}---\ntitle: Old\n---\n"),
            Err(FrontmatterError::UnsupportedBom)
        );
        assert_eq!(
            scan_frontmatter("---\ntitle: Old\nbody"),
            Err(FrontmatterError::Unclosed)
        );
        assert_eq!(scan_frontmatter("---"), Err(FrontmatterError::Unclosed));
    }
}
