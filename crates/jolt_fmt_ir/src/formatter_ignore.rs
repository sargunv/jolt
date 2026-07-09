//! Shared `@formatter:off` / `@formatter:on` range handling.
//!
//! Both the Java and Kotlin formatters consume this module to discover
//! formatter-ignore ranges from token trivia and to splice the raw source
//! spanned by those ranges back into the rendered document.

use std::borrow::Cow;
use std::ops::Range;

use jolt_syntax::{Comment, Language, SyntaxToken};

use crate::{Doc, concat, hard_line, text as doc_text};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormatterIgnoreRange<'source> {
    pub raw_text: &'source str,
    pub raw_text_with_on: &'source str,
    pub interior: Range<usize>,
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
    let mut off_comment_start = None;
    let mut ranges = Vec::new();

    let mut visit_comment =
        |comment: Comment<'source>, leading_comment_start: &mut Option<usize>| {
            let range = comment.text_range();
            let start_offset = range.start().get() - base_start;
            let end_offset = range.end().get() - base_start;
            let comment_text = comment.text();
            if is_formatter_off_marker(comment_text) {
                off_comment_start = Some(
                    leading_comment_start
                        .take()
                        .unwrap_or_else(|| line_start(source, start_offset)),
                );
            } else if is_formatter_on_marker(comment_text)
                && let Some(start) = off_comment_start.take()
            {
                let end = line_start(source, start_offset);
                if start < end {
                    ranges.push(FormatterIgnoreRange {
                        raw_text: strip_trailing_line_ending(&source[start..end]),
                        raw_text_with_on: strip_trailing_line_ending(
                            &source[start..line_end(source, end_offset)],
                        ),
                        interior: start..end,
                    });
                }
            } else if off_comment_start.is_none()
                && leading_comment_start.is_none()
                && comment_starts_own_line(source, start_offset)
            {
                *leading_comment_start = Some(line_start(source, start_offset));
            }
        };

    for token in tokens {
        let mut leading_comment_start = None;
        for comment in token.leading_comments() {
            visit_comment(comment, &mut leading_comment_start);
        }

        let mut trailing_comment_start = None;
        for comment in token.trailing_comments() {
            visit_comment(comment, &mut trailing_comment_start);
        }
    }

    ranges
}

#[must_use]
pub fn formatter_ignore_runs<'source>(
    ranges: &[FormatterIgnoreRange<'source>],
    item_ranges: &[Option<Range<usize>>],
) -> Vec<FormatterIgnoreRun<'source>> {
    let mut runs = ranges
        .iter()
        .map(|range| formatter_ignore_run(range, item_ranges))
        .filter(|run| run.skip_start < run.skip_end)
        .collect::<Vec<_>>();

    for index in 0..runs.len().saturating_sub(1) {
        if !runs[index].include_on_marker && runs[index + 1].insert_index == runs[index].skip_end {
            runs[index].include_on_marker = true;
        }
    }

    runs
}

#[must_use]
pub fn formatter_ignore_run_doc<'source>(run: &FormatterIgnoreRun<'source>) -> Doc<'source> {
    let raw_text = if run.include_on_marker {
        &run.range.raw_text_with_on
    } else {
        &run.range.raw_text
    };
    let stripped = strip_first_line_indent(raw_text);
    match stripped {
        Cow::Borrowed(text) => {
            let lines = text.split('\n');
            let (lower, upper) = lines.size_hint();
            let mut docs = Vec::with_capacity(upper.unwrap_or(lower).saturating_mul(2));
            for line in lines {
                if !docs.is_empty() {
                    docs.push(hard_line());
                }
                docs.push(doc_text(line));
            }
            concat(docs)
        }
        Cow::Owned(text) => {
            let lines = text.split('\n');
            let (lower, upper) = lines.size_hint();
            let mut docs = Vec::with_capacity(upper.unwrap_or(lower).saturating_mul(2));
            for line in lines {
                if !docs.is_empty() {
                    docs.push(hard_line());
                }
                docs.push(doc_text(line.to_owned()));
            }
            concat(docs)
        }
    }
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

fn formatter_ignore_run<'source>(
    range: &FormatterIgnoreRange<'source>,
    item_ranges: &[Option<Range<usize>>],
) -> FormatterIgnoreRun<'source> {
    let mut first_skipped = None;
    let mut last_skipped = None;
    for (index, item_range) in item_ranges.iter().enumerate() {
        let Some(item_range) = item_range.as_ref() else {
            continue;
        };
        if range.interior.contains(&item_range.start) {
            first_skipped.get_or_insert(index);
            last_skipped = Some(index);
        }
    }

    let insert_index = first_skipped.unwrap_or_else(|| {
        item_ranges
            .iter()
            .position(|item_range| {
                item_range
                    .as_ref()
                    .is_some_and(|item_range| range.interior.start < item_range.start)
            })
            .unwrap_or(item_ranges.len())
    });
    let skip_start = first_skipped.unwrap_or(insert_index);
    let skip_end = last_skipped.map_or(skip_start, |last| last + 1);
    let include_on_marker = !item_ranges.iter().any(|item_range| {
        item_range
            .as_ref()
            .is_some_and(|item_range| range.interior.end <= item_range.start)
    });

    FormatterIgnoreRun {
        range: range.clone(),
        insert_index,
        skip_start,
        skip_end,
        include_on_marker,
    }
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

fn comment_starts_own_line(source: &str, offset: usize) -> bool {
    source[line_start(source, offset)..offset].trim().is_empty()
}

fn line_start(source: &str, offset: usize) -> usize {
    source[..offset]
        .rfind(['\n', '\r'])
        .map_or(0, |newline| newline + 1)
}

fn line_end(source: &str, offset: usize) -> usize {
    source[offset..]
        .find(['\n', '\r'])
        .map_or(source.len(), |newline| offset + newline + 1)
}

fn is_formatter_off_marker(comment: &str) -> bool {
    comment.contains("@formatter:off")
}

fn is_formatter_on_marker(comment: &str) -> bool {
    comment.contains("@formatter:on")
}

#[must_use]
pub fn is_formatter_control_marker(comment: &str) -> bool {
    is_formatter_off_marker(comment) || is_formatter_on_marker(comment)
}
