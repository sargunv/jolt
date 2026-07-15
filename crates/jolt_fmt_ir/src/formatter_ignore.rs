//! Shared `@formatter:off` / `@formatter:on` range handling.
//!
//! Both the Java and Kotlin formatters consume this module to discover
//! formatter-ignore ranges from token trivia and to splice the raw source
//! spanned by those ranges back into the rendered document.

use std::borrow::Cow;
use std::ops::Range;

use jolt_syntax::{Comment, Language, SyntaxToken};
#[cfg(debug_assertions)]
use jolt_syntax::{SourceIdentity, TriviaKind};

use crate::{Doc, DocBuilder};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormatterIgnoreRange<'source> {
    pub raw_text: &'source str,
    pub raw_text_with_on: &'source str,
    pub interior: Range<usize>,
    #[cfg(debug_assertions)]
    claims: Vec<SourceIdentity<'source>>,
    #[cfg(debug_assertions)]
    claims_with_on: Vec<SourceIdentity<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormatterIgnoreRun<'source> {
    pub range: FormatterIgnoreRange<'source>,
    pub insert_index: usize,
    pub skip_start: usize,
    pub skip_end: usize,
    pub include_on_marker: bool,
}

impl FormatterIgnoreRun<'_> {
    #[must_use]
    pub fn skips(&self, item_index: usize) -> bool {
        (self.skip_start..self.skip_end).contains(&item_index)
    }
}

pub fn formatter_ignore_ranges<'source, L: Language>(
    source: &'source str,
    base_start: usize,
    tokens: impl IntoIterator<Item = SyntaxToken<'source, L>>,
) -> Vec<FormatterIgnoreRange<'source>> {
    // Formatter control markers are rare. Avoid walking every token and both
    // trivia lists for every file, block, and member body when the source
    // slice cannot contain one.
    if !source.contains("@formatter:") {
        return Vec::new();
    }

    let tokens: Vec<_> = tokens.into_iter().collect();

    let mut off_comment_start = None;
    let mut ranges = Vec::new();
    let mut lines = SourceLineCursor::new(source);

    let mut visit_comment =
        |comment: Comment<'source>, leading_comment_start: &mut Option<usize>| {
            let range = comment.text_range();
            let start_offset = range.start().get() - base_start;
            let end_offset = range.end().get() - base_start;
            let line = lines.comment_line(start_offset);
            let end_line = lines.comment_line(end_offset.saturating_sub(1).max(start_offset));
            let comment_text = comment.text();
            if is_formatter_off_marker(comment_text) {
                off_comment_start = Some(leading_comment_start.take().unwrap_or(line.start));
            } else if is_formatter_on_marker(comment_text)
                && let Some(start) = off_comment_start.take()
            {
                let end = line.start;
                if start < end {
                    ranges.push(FormatterIgnoreRange {
                        raw_text: strip_trailing_line_ending(&source[start..end]),
                        raw_text_with_on: strip_trailing_line_ending(
                            &source[start..end_line.raw_end],
                        ),
                        interior: start..end,
                        #[cfg(debug_assertions)]
                        claims: Vec::new(),
                        #[cfg(debug_assertions)]
                        claims_with_on: Vec::new(),
                    });
                }
            } else if off_comment_start.is_none()
                && leading_comment_start.is_none()
                && line.comment_starts_own_line
            {
                *leading_comment_start = Some(line.start);
            }
        };

    for token in &tokens {
        let mut leading_comment_start = None;
        for comment in token.leading_comments() {
            visit_comment(comment, &mut leading_comment_start);
        }

        let mut trailing_comment_start = None;
        for comment in token.trailing_comments() {
            visit_comment(comment, &mut trailing_comment_start);
        }
    }

    #[cfg(debug_assertions)]
    populate_claims(&mut ranges, base_start, tokens);

    ranges
}

#[cfg(debug_assertions)]
fn populate_claims<'source, L: Language>(
    ranges: &mut [FormatterIgnoreRange<'source>],
    base_start: usize,
    tokens: impl IntoIterator<Item = SyntaxToken<'source, L>>,
) {
    if ranges.is_empty() {
        return;
    }

    let mut range_index = 0;
    for token in tokens {
        for piece in token
            .leading_comments()
            .flat_map(|comment| comment.source_pieces())
        {
            append_identity(
                ranges,
                &mut range_index,
                base_start,
                piece.text_range().start().get()..piece.text_range().end().get(),
                SourceIdentity::Trivia(piece.id()),
                piece.trivia().kind() == TriviaKind::Newline,
            );
        }
        if !token.text().is_empty() {
            let range = token.token_text_range();
            append_identity(
                ranges,
                &mut range_index,
                base_start,
                range.start().get()..range.end().get(),
                SourceIdentity::Token(token.source_id()),
                false,
            );
        }
        for piece in token
            .trailing_comments()
            .flat_map(|comment| comment.source_pieces())
        {
            append_identity(
                ranges,
                &mut range_index,
                base_start,
                piece.text_range().start().get()..piece.text_range().end().get(),
                SourceIdentity::Trivia(piece.id()),
                piece.trivia().kind() == TriviaKind::Newline,
            );
        }
    }
}

#[cfg(debug_assertions)]
fn append_identity<'source>(
    ranges: &mut [FormatterIgnoreRange<'source>],
    range_index: &mut usize,
    base_start: usize,
    identity_range: Range<usize>,
    identity: SourceIdentity<'source>,
    is_line_ending: bool,
) {
    while *range_index < ranges.len() {
        let with_on_end = base_start
            + ranges[*range_index].interior.start
            + ranges[*range_index].raw_text_with_on.len();
        if with_on_end > identity_range.start
            || (is_line_ending && with_on_end == identity_range.start)
        {
            break;
        }
        *range_index += 1;
    }
    let Some(range) = ranges.get_mut(*range_index) else {
        return;
    };
    let start = base_start + range.interior.start;
    let without_on_end = start + range.raw_text.len();
    let with_on_end = start + range.raw_text_with_on.len();
    if start <= identity_range.start && identity_range.end <= without_on_end {
        range.claims.push(identity);
    }
    if (start <= identity_range.start && identity_range.end <= with_on_end)
        || (is_line_ending && identity_range.start == with_on_end)
    {
        range.claims_with_on.push(identity);
    }
}

#[must_use]
pub fn formatter_ignore_runs<'source>(
    ranges: &[FormatterIgnoreRange<'source>],
    item_ranges: &[Option<Range<usize>>],
) -> Vec<FormatterIgnoreRun<'source>> {
    let mut runs = Vec::with_capacity(ranges.len());
    let mut item_index = 0;

    // Both inputs are in source order. Keep a single cursor through the items
    // instead of rescanning the entire list for every ignored range.
    for range in ranges {
        while item_ranges.get(item_index).is_some_and(|item_range| {
            item_range
                .as_ref()
                .is_none_or(|item_range| item_range.start < range.interior.start)
        }) {
            item_index += 1;
        }

        let insert_index = item_index;
        let skip_start = item_index;
        let mut last_skipped = None;
        while item_ranges.get(item_index).is_some_and(|item_range| {
            item_range
                .as_ref()
                .is_none_or(|item_range| item_range.start < range.interior.end)
        }) {
            if item_ranges[item_index].is_some() {
                last_skipped = Some(item_index);
            }
            item_index += 1;
        }
        let skip_end = last_skipped.map_or(skip_start, |last| last + 1);

        if skip_start < skip_end {
            runs.push(FormatterIgnoreRun {
                range: range.clone(),
                insert_index,
                skip_start,
                skip_end,
                include_on_marker: item_index == item_ranges.len(),
            });
        }
    }

    for index in 0..runs.len().saturating_sub(1) {
        if !runs[index].include_on_marker && runs[index + 1].insert_index == runs[index].skip_end {
            runs[index].include_on_marker = true;
        }
    }

    runs
}

#[must_use]
pub fn formatter_ignore_run_doc<'source>(
    run: &FormatterIgnoreRun<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let raw_text = if run.include_on_marker {
        &run.range.raw_text_with_on
    } else {
        &run.range.raw_text
    };
    let stripped = strip_first_line_indent(raw_text);
    let contents = match stripped {
        Cow::Borrowed(text) => {
            let lines = text.split('\n');
            doc.concat_list(|docs| {
                for line in lines {
                    if !docs.is_empty() {
                        let line_break = docs.hard_line();
                        docs.push(line_break);
                    }
                    let line = docs.text(line);
                    docs.push(line);
                }
            })
        }
        Cow::Owned(text) => {
            let lines = text.split('\n');
            doc.concat_list(|docs| {
                for line in lines {
                    if !docs.is_empty() {
                        let line_break = docs.hard_line();
                        docs.push(line_break);
                    }
                    let line = docs.text(line.to_owned());
                    docs.push(line);
                }
            })
        }
    };
    #[cfg(debug_assertions)]
    let claims = if run.include_on_marker {
        run.range.claims_with_on.iter().copied()
    } else {
        run.range.claims.iter().copied()
    };
    #[cfg(not(debug_assertions))]
    let claims = std::iter::empty();
    doc.claimed_source(contents, claims)
}

#[must_use]
pub fn token_range_between<L: Language>(
    first: &SyntaxToken<'_, L>,
    last: &SyntaxToken<'_, L>,
) -> Range<usize> {
    first.token_text_range().start().get()..last.token_text_range().end().get()
}

#[must_use]
pub fn relative_token_range_between<L: Language>(
    first: &SyntaxToken<'_, L>,
    last: &SyntaxToken<'_, L>,
    base_start: usize,
) -> Range<usize> {
    let range = token_range_between(first, last);
    range.start - base_start..range.end - base_start
}

fn strip_first_line_indent(text: &str) -> Cow<'_, str> {
    let has_carriage_returns = text.as_bytes().contains(&b'\r');
    let Some(first_line) = first_non_empty_normalized_line(text) else {
        return if has_carriage_returns {
            Cow::Owned(normalized_without_first_indent(text, ""))
        } else {
            Cow::Borrowed(text)
        };
    };
    let indent = leading_indent(first_line);
    if indent.is_empty() && !has_carriage_returns {
        return Cow::Borrowed(text);
    }

    Cow::Owned(normalized_without_first_indent(text, indent))
}

fn first_non_empty_normalized_line(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    let mut line_start = 0;

    while line_start < text.len() {
        let line_end = normalized_line_end(bytes, line_start);
        let line = &text[line_start..line_end];
        if !line.trim().is_empty() {
            return Some(line);
        }
        line_start = next_line_start(bytes, line_end);
    }

    None
}

fn normalized_without_first_indent(text: &str, indent: &str) -> String {
    if text.is_empty() {
        return String::new();
    }

    let bytes = text.as_bytes();
    let mut stripped = String::with_capacity(text.len());
    let mut line_start = 0;
    let mut first = true;

    loop {
        let line_end = normalized_line_end(bytes, line_start);
        if !first {
            stripped.push('\n');
        }
        first = false;

        let line = &text[line_start..line_end];
        stripped.push_str(line.strip_prefix(indent).unwrap_or(line));

        if line_end == text.len() {
            break;
        }

        line_start = next_line_start(bytes, line_end);
        if line_start == text.len() {
            stripped.push('\n');
            break;
        }
    }

    stripped
}

fn normalized_line_end(bytes: &[u8], line_start: usize) -> usize {
    bytes[line_start..]
        .iter()
        .position(|byte| matches!(byte, b'\n' | b'\r'))
        .map_or(bytes.len(), |offset| line_start + offset)
}

fn next_line_start(bytes: &[u8], line_end: usize) -> usize {
    match bytes.get(line_end) {
        Some(b'\r') if bytes.get(line_end + 1) == Some(&b'\n') => line_end + 2,
        Some(_) => line_end + 1,
        None => line_end,
    }
}

fn leading_indent(line: &str) -> &str {
    let indent_end = line
        .char_indices()
        .find_map(|(index, character)| (!matches!(character, ' ' | '\t')).then_some(index))
        .unwrap_or(line.len());
    &line[..indent_end]
}

fn strip_trailing_line_ending(text: &str) -> &str {
    text.strip_suffix("\r\n")
        .or_else(|| text.strip_suffix('\n'))
        .or_else(|| text.strip_suffix('\r'))
        .unwrap_or(text)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SourceLine {
    start: usize,
    raw_end: usize,
    next_start: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CommentLine {
    start: usize,
    raw_end: usize,
    comment_starts_own_line: bool,
}

/// Monotonic line lookup for comments visited in represented source order.
///
/// `comment_line` advances through source bytes once as ordered comments
/// arrive and inspects a line prefix only for its first comment, so all marker
/// lookup is O(source bytes + comments) time and O(1) auxiliary storage.
struct SourceLineCursor<'source> {
    source: &'source str,
    line: SourceLine,
    line_has_comment: bool,
    previous_comment_start: Option<usize>,
}

impl<'source> SourceLineCursor<'source> {
    fn new(source: &'source str) -> Self {
        Self {
            source,
            line: source_line(source.as_bytes(), 0),
            line_has_comment: false,
            previous_comment_start: None,
        }
    }

    fn comment_line(&mut self, comment_start: usize) -> CommentLine {
        debug_assert!(
            self.previous_comment_start
                .is_none_or(|previous| previous <= comment_start),
            "formatter comments must be visited in source order"
        );
        self.previous_comment_start = Some(comment_start);

        while self.line.next_start < self.source.len() && comment_start >= self.line.next_start {
            self.line = source_line(self.source.as_bytes(), self.line.next_start);
            self.line_has_comment = false;
        }

        let line = self.line;
        debug_assert!((line.start..=line.raw_end).contains(&comment_start));
        let comment_starts_own_line =
            !self.line_has_comment && self.source[line.start..comment_start].trim().is_empty();
        self.line_has_comment = true;
        CommentLine {
            start: line.start,
            raw_end: line.raw_end,
            comment_starts_own_line,
        }
    }
}

fn source_line(bytes: &[u8], start: usize) -> SourceLine {
    let content_end = normalized_line_end(bytes, start);
    SourceLine {
        start,
        raw_end: content_end + usize::from(content_end < bytes.len()),
        next_start: next_line_start(bytes, content_end),
    }
}

fn is_formatter_off_marker(comment: &str) -> bool {
    formatter_marker_body(comment) == Some("@formatter:off")
}

#[must_use]
pub fn is_formatter_on_marker(comment: &str) -> bool {
    formatter_marker_body(comment) == Some("@formatter:on")
}

fn formatter_marker_body(comment: &str) -> Option<&str> {
    let comment = comment.trim();
    if let Some(body) = comment.strip_prefix("//") {
        return Some(body.trim());
    }
    comment
        .strip_prefix("/*")?
        .strip_suffix("*/")
        .map(str::trim)
}

#[must_use]
pub fn is_formatter_control_marker(comment: &str) -> bool {
    is_formatter_off_marker(comment) || is_formatter_on_marker(comment)
}

#[cfg(test)]
mod tests {
    use super::{CommentLine, SourceLineCursor, is_formatter_on_marker};

    #[test]
    fn formatter_on_marker_requires_the_complete_comment_body() {
        assert!(is_formatter_on_marker("// @formatter:on"));
        assert!(is_formatter_on_marker("  /* @formatter:on */  "));
        assert!(!is_formatter_on_marker("// text @formatter:on"));
        assert!(!is_formatter_on_marker("// @formatter:on later"));
        assert!(!is_formatter_on_marker("/* prefix @formatter:on */"));
        assert!(!is_formatter_on_marker("@formatter:on"));
    }

    #[test]
    fn source_line_cursor_handles_same_line_comments_and_mixed_line_endings() {
        let source = "  /* first */ /* second */\r\ncode /* third */\n  /* fourth */";
        let mut lines = SourceLineCursor::new(source);

        assert_eq!(
            lines.comment_line(source.find("/* first */").unwrap()),
            CommentLine {
                start: 0,
                raw_end: 27,
                comment_starts_own_line: true,
            }
        );
        assert!(
            !lines
                .comment_line(source.find("/* second */").unwrap())
                .comment_starts_own_line
        );
        assert!(
            !lines
                .comment_line(source.find("/* third */").unwrap())
                .comment_starts_own_line
        );
        assert!(
            lines
                .comment_line(source.find("/* fourth */").unwrap())
                .comment_starts_own_line
        );
    }
}
