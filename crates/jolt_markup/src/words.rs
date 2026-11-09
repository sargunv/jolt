//! The words a wrapped line must not start with, and the lines whose line
//! breaks stay: shared by both flavours.

use crate::frame::Line;
use crate::parser::{Pos, Span, to_u32};

/// The pieces of a reference (or a block tag's other arguments) that spans
/// lines, to be joined by single spaces on one line: each line's part
/// without the whitespace around it. Whitespace inside a reference doesn't
/// change what javadoc reads in it. A reference with a later line that
/// starts with `@` stays whole: javac ends a reference at an `@` that
/// starts a line, which a joined line would hide. `None` when the span is
/// on one line, or must stay whole.
pub(crate) fn joined_pieces(lines: &[Line<'_>], span: Span) -> Option<Vec<Span>> {
    if span.end.line == span.start.line {
        return None;
    }
    let spaces = [' ', '\t', '\u{c}'];
    let mut pieces = Vec::new();
    for line in span.start.line..=span.end.line {
        let text = lines[line as usize].text;
        let from = if line == span.start.line {
            span.start.off as usize
        } else {
            0
        };
        let to = if line == span.end.line {
            span.end.off as usize
        } else {
            text.len()
        };
        let part = &text[from..to];
        let trimmed = part.trim_start_matches(spaces);
        if line > span.start.line && trimmed.starts_with('@') {
            return None;
        }
        let start = from + part.len() - trimmed.len();
        let trimmed = trimmed.trim_end_matches(spaces);
        if !trimmed.is_empty() {
            pieces.push(Span {
                start: Pos {
                    line,
                    off: to_u32(start),
                },
                end: Pos {
                    line,
                    off: to_u32(start + trimmed.len()),
                },
            });
        }
    }
    Some(pieces)
}

/// Whether `word` is a plain-text list marker: `-`, `*`, `+`, `•`, or a
/// number and `.` or `)`.
pub(crate) fn is_list_marker(word: &str) -> bool {
    match word {
        "-" | "*" | "+" | "•" => true,
        _ => word
            .strip_suffix(['.', ')'])
            .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())),
    }
}

/// Whether `word` would start new structure at the start of a line, so a
/// wrap must not put it there: a block tag (`@`) or a list marker, and in
/// Markdown also a word that could start a block that interrupts a
/// paragraph or ends it: an ATX heading (`#` to `######`), a block quote
/// (`>`), a fence, an HTML block (`<` and a letter, `/`, `!` or `?`), a
/// table row (`|`), a thematic break or setext underline (a word made only
/// of `*`, `-`, `_`, `=` or `+`), or a table's delimiter row, which turns the
/// line before it into a table header (a word made only of `-`, `:` and `|`,
/// with a `-`).
pub(crate) fn starts_structure(word: &str, markdown: bool) -> bool {
    if word.starts_with('@') || is_list_marker(word) {
        return true;
    }
    if !markdown {
        return false;
    }
    let bytes = word.as_bytes();
    let Some(&first) = bytes.first() else {
        return false;
    };
    match first {
        b'>' | b'|' => true,
        b'#' => bytes.len() <= 6 && bytes.iter().all(|&byte| byte == b'#'),
        fence @ (b'`' | b'~') => bytes.len() >= 3 && bytes[..3].iter().all(|&byte| byte == fence),
        b'<' => bytes
            .get(1)
            .is_some_and(|&next| next.is_ascii_alphabetic() || matches!(next, b'/' | b'!' | b'?')),
        b'0'..=b'9' => {
            let digits = bytes
                .iter()
                .take_while(|byte| byte.is_ascii_digit())
                .count();
            matches!(bytes.get(digits), Some(b'.' | b')'))
        }
        _ => {
            bytes.iter().all(|byte| b"*-_=+".contains(byte))
                || bytes.contains(&b'-') && bytes.iter().all(|byte| b"-:|".contains(byte))
        }
    }
}

/// Whether a Markdown word that starts a line keeps starting one: one where
/// Markdown readers don't all agree on what the line is, a word made only of
/// the characters of a setext underline or a thematic break (`Foo` and
/// `=-`) or a table's delimiter row (with fewer cells than its header), or one
/// that starts like a fence, which javac reads as one after a paragraph
/// however indented.
pub(crate) fn keeps_line_start(word: &str) -> bool {
    word.bytes().all(|byte| b"*-_=+".contains(&byte))
        || is_delimiter_row(word)
        || word.starts_with("```")
        || word.starts_with("~~~")
}

/// Whether a word is made only of `-`, `:` and `|`, with a `-`, like a
/// table's delimiter row (`|---|:--:|`).
pub(crate) fn is_delimiter_row(word: &str) -> bool {
    word.contains('-') && word.bytes().all(|byte| b"-:|".contains(&byte))
}

/// Whether a Markdown line that starts with `word` ends javadoc's first
/// sentence there (`DocTreeMaker`'s `endPara`: a line that starts with
/// `-`, `+`, `*`, `#`, `=`, `_` or `>`).
pub(crate) fn ends_first_paragraph(word: &str) -> bool {
    word.starts_with(['-', '+', '*', '#', '=', '_', '>'])
}

/// Whether a line's text is a ruler: three or more punctuation characters
/// (`-------`, `|-----|`), with nothing else but spaces.
pub(crate) fn is_ruler(text: &str) -> bool {
    let mut count = 0;
    for byte in text.bytes() {
        match byte {
            b' ' | b'\t' => {}
            b'-' | b'=' | b'_' | b'*' | b'+' | b'~' | b'#' | b'|' | b'.' | b':' | b'/' | b'\\' => {
                count += 1;
            }
            _ => return false,
        }
    }
    count >= 3
}
