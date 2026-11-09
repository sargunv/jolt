//! The frame: the text of a doc comment as javac reads it, line by line,
//! borrowed from the source.
//!
//! A port of javac's comment framing (JDK 25: `JavaTokenizer.BasicComment.
//! scanDocComment`, and `JavadocTokenizer.StrippedComment`, which removes
//! the indentation that the lines share).

use crate::java;

/// One line of a doc comment, as javadoc reads it. The lines of a comment
/// are read joined by `\n`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Line<'a> {
    /// What javadoc reads: the line without its frame and without the
    /// indentation that the comment's lines share.
    pub(crate) text: &'a str,
    /// `/**`: the line without its frame (leading whitespace and run of `*`s)
    /// but with the shared indentation, so `text` is a suffix of it.
    /// `///`: the source line from its `///`, which a line printed as
    /// written starts with.
    pub(crate) raw: &'a str,
}

impl Line<'_> {
    /// Whether javadoc reads the line as blank: only whitespace.
    pub(crate) fn is_blank(&self) -> bool {
        self.text.chars().all(java::is_whitespace)
    }
}

/// javac's whitespace inside a comment's frame (`UnicodeReader.isWhitespace`).
const fn is_frame_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | 0x0c)
}

/// Whether `text` has a Unicode escape (`@`): a `\` that no other `\`
/// escapes, followed by `u`. javac decodes escapes before it reads a comment,
/// so the source text doesn't show what javadoc reads; such a comment is
/// printed as written.
pub(crate) fn has_unicode_escape(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut at = 0;
    while let Some(found) = bytes[at..].iter().position(|&byte| byte == b'\\') {
        let start = at + found;
        let run = bytes[start..]
            .iter()
            .take_while(|&&byte| byte == b'\\')
            .count();
        // The last `\` of an odd run escapes nothing.
        if run % 2 == 1 && bytes.get(start + run) == Some(&b'u') {
            return true;
        }
        at = start + run;
    }
    false
}

/// The lines of a `/** … */` comment as javac reads them, or `None` when the
/// comment is printed as written: a banner (`/***`, or a line made only of
/// two or more `*`s), `/**/`, an unterminated comment, or one with a Unicode
/// escape. The flag tells whether the first line is on the line of `/**`.
pub(crate) fn javadoc(text: &str) -> Option<(Vec<Line<'_>>, bool)> {
    if text.starts_with("/***")
        || text.starts_with("/**/")
        || !text.starts_with("/**")
        || !text.ends_with("*/")
        || text.len() < "/***/".len()
        || has_unicode_escape(text)
    {
        return None;
    }
    let bytes = text.as_bytes();
    let mut pos = 3;
    while pos < bytes.len() && is_frame_space(bytes[pos]) {
        pos += 1;
    }
    // A first line with nothing after `/**` is no line.
    let opener_text = !matches!(bytes.get(pos), Some(b'\r' | b'\n'));
    if !opener_text {
        if bytes[pos] == b'\r' {
            pos += 1;
        }
        if bytes.get(pos) == Some(&b'\n') {
            pos += 1;
        }
    }
    let mut lines = Vec::new();
    while pos < bytes.len() {
        let end = bytes[pos..]
            .iter()
            .position(|&byte| matches!(byte, b'\r' | b'\n'))
            .map_or(bytes.len(), |found| pos + found);
        let source = &text[pos..end];
        let trimmed = source.trim_matches(|ch: char| ch.is_ascii_whitespace());
        if trimmed.len() >= 2 && trimmed.bytes().all(|byte| byte == b'*') {
            return None;
        }
        // Only the last line holds the closer.
        lines.push(content(source, end == bytes.len()));
        pos = end;
        if bytes.get(pos) == Some(&b'\r') {
            pos += 1;
        }
        if bytes.get(pos) == Some(&b'\n') {
            pos += 1;
        }
    }
    strip_indent(&mut lines);
    Some((lines, opener_text))
}

/// `trimJavadocComment`: a line without the comment's closer and without its
/// frame (leading whitespace and its run of `*`s). A line without `*` keeps
/// its leading whitespace.
fn content(line: &str, last: bool) -> Line<'_> {
    let line = if last {
        trim_end_of_comment(line)
    } else {
        line
    };
    let bytes = line.as_bytes();
    let mut at = 0;
    while at < bytes.len() && is_frame_space(bytes[at]) {
        at += 1;
    }
    if at == bytes.len() {
        let rest = &line[at..];
        return Line {
            text: rest,
            raw: rest,
        };
    }
    let stars = bytes[at..].iter().take_while(|&&byte| byte == b'*').count();
    let rest = if stars == 0 {
        line
    } else {
        &line[at + stars..]
    };
    Line {
        text: rest,
        raw: rest,
    }
}

/// `trimEndOfComment`: the line up to the run of `*`s before the closing
/// `/`, or nothing when only whitespace (and `*`s each followed by
/// whitespace) comes before it; the whole line when it has no closer.
fn trim_end_of_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut at = 0;
    let mut all_whitespace = true;
    while at < bytes.len() {
        let end = at;
        let stars = bytes[at..].iter().take_while(|&&byte| byte == b'*').count();
        at += stars;
        if stars > 0 && bytes.get(at) == Some(&b'/') {
            return if all_whitespace {
                &line[end..end]
            } else {
                &line[..end]
            };
        }
        match line[at..].chars().next() {
            Some(ch) => {
                all_whitespace &= ch.is_ascii() && is_frame_space(ch as u8);
                at += ch.len_utf8();
            }
            None => all_whitespace = false,
        }
    }
    line
}

/// `StrippedComment`: removes from each line the indentation that every
/// line with text shares, counted in `Character.isWhitespace` characters.
fn strip_indent(lines: &mut [Line<'_>]) {
    let indent = lines
        .iter()
        .filter_map(|line| line.text.chars().position(|ch| !java::is_whitespace(ch)))
        .min()
        .unwrap_or(0);
    if indent == 0 {
        return;
    }
    for line in lines {
        let skip: usize = line
            .text
            .chars()
            .take(indent)
            .take_while(|&ch| java::is_whitespace(ch))
            .map(char::len_utf8)
            .sum();
        line.text = &line.text[skip..];
    }
}

/// The lines of a run of `///` lines (each with its `///` and without its
/// line break) as javac reads them, and the run's frame (`///` and the
/// indentation that javac removes, from a line that has it), or `None` when
/// the run has a Unicode escape.
pub(crate) fn markdown<'a>(run: &[&'a str]) -> Option<(Vec<Line<'a>>, &'a str)> {
    const OPENER: usize = "///".len();
    if run.iter().any(|line| has_unicode_escape(line)) {
        return None;
    }
    // `getJavadocLineCommentIndent`: the least whitespace after `///` of a
    // line with text.
    let mut shared: Option<(usize, &'a str)> = None;
    for &line in run {
        let body = &line.as_bytes()[OPENER..];
        let indent = body
            .iter()
            .take_while(|&&byte| is_frame_space(byte))
            .count();
        if indent < body.len() && shared.is_none_or(|(least, _)| indent < least) {
            shared = Some((indent, &line[..OPENER + indent]));
        }
    }
    let (indent, frame) = shared.unwrap_or((0, &run[0][..OPENER]));
    // `trimJavadocLineComment`: `indent` characters go, whatever they are.
    let lines = run
        .iter()
        .map(|&line| {
            let body = &line[OPENER..];
            let skip: usize = body.chars().take(indent).map(char::len_utf8).sum();
            Line {
                text: &body[skip..],
                raw: line,
            }
        })
        .collect();
    Some((lines, frame))
}
