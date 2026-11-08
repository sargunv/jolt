//! The frame: a doc comment's content lines, borrowed from its source.

/// One content line of a doc comment.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Line<'a> {
    /// The text after the frame, without trailing whitespace: for `/**`, the
    /// text after the line's run of `*` and one space.
    pub(crate) text: &'a str,
    /// The byte offset of `text` in the comment.
    pub(crate) offset: usize,
    /// The line from its frame on (the first `*`), without trailing
    /// whitespace, for printing it as written. Empty for a line without a
    /// `*`: a blank line, or the comment's first line.
    pub(crate) raw: &'a str,
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
        });
    }
    Some(lines)
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
