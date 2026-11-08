//! The frame: a doc comment's content lines, borrowed from its source.

/// One content line of a doc comment.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Line<'a> {
    /// The text after the frame, without trailing whitespace: for `/**`, the
    /// text after the line's run of `*` and one space.
    pub(crate) text: &'a str,
    /// The byte offset of `text` in the comment.
    pub(crate) offset: usize,
    /// The line from its frame on (the first `*`, or `///`), without
    /// trailing whitespace, for printing it as written. Empty for a `/**`
    /// line without a `*`: a blank line, or the comment's first line.
    pub(crate) raw: &'a str,
    /// The source line ends in two or more spaces: in a Markdown paragraph,
    /// a hard line break. Always false for `/**`.
    pub(crate) hard_break: bool,
}

/// The content lines of a `/**` comment, or `None` when the comment is laid
/// out as written:
///
/// - it starts with `/***` (a banner) or is `/**/`, or isn't terminated;
/// - a content line other than the first has text but no leading `*`
///   (javadoc keeps the indentation of such a line);
/// - a line is made only of two or more `*`s (a ruler of a banner).
///
/// Each line loses its leading whitespace, its run of `*`s and one space
/// after them, as javadoc reads it (` ** Double` is `Double`), and its
/// trailing whitespace. A comment on one line keeps its `*`s: only its
/// delimiters and the whitespace inside them go. This is the reading of
/// `jolt_test_support`'s doc comment oracle, and N9 keeps the same comments
/// as written (see `docs/internals/formatter.md`).
pub(crate) fn javadoc(text: &str) -> Option<Vec<Line<'_>>> {
    if text.starts_with("/***") {
        return None;
    }
    let body = text.strip_prefix("/**")?.strip_suffix("*/")?;
    let offset_of = |part: &str| part.as_ptr() as usize - text.as_ptr() as usize;
    let mut lines = Vec::new();
    let mut parts = split_lines(body);
    let first = parts.next().unwrap_or("");
    let Some(second) = parts.next() else {
        let content = first.trim();
        lines.push(Line {
            text: content,
            offset: offset_of(content),
            raw: "",
            hard_break: false,
        });
        return Some(lines);
    };
    for (index, line) in [first, second].into_iter().chain(parts).enumerate() {
        let trimmed = line.trim_start();
        let (content, raw) = if let Some(after) = trimmed.strip_prefix('*') {
            let rest = after.trim_start_matches('*');
            let stars = 1 + after.len() - rest.len();
            let rest = rest.trim_end();
            if stars >= 2 && rest.is_empty() {
                return None;
            }
            (rest.strip_prefix(' ').unwrap_or(rest), trimmed.trim_end())
        } else {
            let content = trimmed.trim_end();
            if index > 0 && !content.is_empty() {
                return None;
            }
            (content, "")
        };
        lines.push(Line {
            text: content,
            offset: offset_of(content),
            raw,
            hard_break: false,
        });
    }
    Some(lines)
}

/// The content lines of a run of `///` lines (each with its `///` and
/// without its line break), and the run's frame: `///` and the indentation
/// that every line with text shares, which JEP 467 strips (` ` for most
/// runs, nothing for `///CLOVER:OFF`). Each line's text is what follows the
/// frame, without trailing whitespace. Offsets count from the run's first
/// line, as if its lines were joined by one line break.
pub(crate) fn markdown<'a>(run: &[&'a str]) -> (Vec<Line<'a>>, &'a str) {
    const OPENER: usize = "///".len();
    let indentation = |line: &str| {
        let body = &line[OPENER..];
        body.len() - body.trim_start_matches([' ', '\t']).len()
    };
    let mut shared: Option<(usize, &'a str)> = None;
    for &line in run {
        if line[OPENER..].trim_end().is_empty() {
            continue;
        }
        let indent = indentation(line);
        if shared.is_none_or(|(least, _)| indent < least) {
            shared = Some((indent, &line[..OPENER + indent]));
        }
    }
    let (shared, frame) = shared.unwrap_or((0, &run[0][..OPENER]));
    let mut offset = 0;
    let lines = run
        .iter()
        .map(|&line| {
            let raw = line.trim_end();
            let text = raw.get(OPENER + shared..).unwrap_or("");
            let start = offset + raw.len() - text.len();
            offset += line.len() + 1;
            Line {
                text,
                offset: start,
                raw,
                hard_break: line[raw.len()..].ends_with("  "),
            }
        })
        .collect();
    (lines, frame)
}

/// Splits on `\r\n`, `\r` and `\n`.
fn split_lines(text: &str) -> impl Iterator<Item = &str> {
    let mut rest = Some(text);
    std::iter::from_fn(move || {
        let text = rest?;
        let bytes = text.as_bytes();
        if let Some(end) = bytes.iter().position(|&byte| matches!(byte, b'\n' | b'\r')) {
            let skip = if bytes[end..].starts_with(b"\r\n") {
                2
            } else {
                1
            };
            rest = Some(&text[end + skip..]);
            Some(&text[..end])
        } else {
            rest = None;
            Some(text)
        }
    })
}
