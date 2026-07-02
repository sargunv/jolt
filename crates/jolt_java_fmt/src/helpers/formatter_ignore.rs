use std::ops::Range;

use jolt_fmt_ir::{Doc, concat, hard_line, literal_text};
use jolt_java_syntax::{JavaLexer, JavaSyntaxKind, JavaSyntaxToken, TriviaKind};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FormatterIgnoreRange {
    pub(crate) raw_text: String,
    pub(crate) raw_text_with_on: String,
    pub(crate) interior: Range<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FormatterIgnoreRun {
    pub(crate) range: FormatterIgnoreRange,
    pub(crate) insert_index: usize,
    pub(crate) skip_start: usize,
    pub(crate) skip_end: usize,
    pub(crate) include_on_marker: bool,
}

impl FormatterIgnoreRun {
    pub(crate) fn skips(&self, item_index: usize) -> bool {
        (self.skip_start..self.skip_end).contains(&item_index)
    }
}

pub(crate) fn formatter_ignore_ranges(source: &str) -> Vec<FormatterIgnoreRange> {
    let mut lexer = JavaLexer::new(source);
    let mut off_comment_start = None;
    let mut ranges = Vec::new();

    loop {
        let token = lexer.next_token();
        let mut visit_trivia =
            |trivia: &jolt_java_syntax::Trivia, leading_comment_start: &mut Option<usize>| {
                if !matches!(
                    trivia.kind,
                    TriviaKind::LineComment | TriviaKind::BlockComment | TriviaKind::JavadocComment
                ) {
                    return;
                }

                let comment_text = &source[trivia.range.start().get()..trivia.range.end().get()];
                if is_formatter_off_marker(comment_text) {
                    off_comment_start = Some(
                        leading_comment_start
                            .take()
                            .unwrap_or_else(|| line_start(source, trivia.range.start().get())),
                    );
                } else if is_formatter_on_marker(comment_text)
                    && let Some(start) = off_comment_start.take()
                {
                    let end = line_start(source, trivia.range.start().get());
                    if start < end {
                        ranges.push(FormatterIgnoreRange {
                            raw_text: strip_trailing_line_ending(&source[start..end]).to_owned(),
                            raw_text_with_on: strip_trailing_line_ending(
                                &source[start..line_end(source, trivia.range.end().get())],
                            )
                            .to_owned(),
                            interior: start..end,
                        });
                    }
                } else if off_comment_start.is_none()
                    && leading_comment_start.is_none()
                    && comment_starts_own_line(source, trivia.range.start().get())
                {
                    *leading_comment_start = Some(line_start(source, trivia.range.start().get()));
                }
            };

        let mut leading_comment_start = None;
        for trivia in &token.leading {
            visit_trivia(trivia, &mut leading_comment_start);
        }

        let mut trailing_comment_start = None;
        for trivia in &token.trailing {
            visit_trivia(trivia, &mut trailing_comment_start);
        }

        if token.kind == JavaSyntaxKind::Eof {
            break;
        }
    }

    ranges
}

pub(crate) fn formatter_ignore_runs(
    ranges: &[FormatterIgnoreRange],
    item_ranges: &[Option<Range<usize>>],
) -> Vec<FormatterIgnoreRun> {
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

pub(crate) fn formatter_ignore_run_doc(run: &FormatterIgnoreRun) -> Doc {
    let raw_text = if run.include_on_marker {
        &run.range.raw_text_with_on
    } else {
        &run.range.raw_text
    };
    let mut docs = Vec::new();
    for line in strip_first_line_indent(raw_text).split('\n') {
        if !docs.is_empty() {
            docs.push(hard_line());
        }
        docs.push(literal_text(line.to_owned()));
    }
    concat(docs)
}

pub(crate) fn token_range(tokens: &[JavaSyntaxToken]) -> Option<Range<usize>> {
    let first = tokens.first()?;
    let last = tokens.last()?;
    Some(first.token_text_range().start().get()..last.token_text_range().end().get())
}

pub(crate) fn relative_token_range(
    tokens: &[JavaSyntaxToken],
    base_start: usize,
) -> Option<Range<usize>> {
    let range = token_range(tokens)?;
    Some(range.start - base_start..range.end - base_start)
}

fn formatter_ignore_run(
    range: &FormatterIgnoreRange,
    item_ranges: &[Option<Range<usize>>],
) -> FormatterIgnoreRun {
    let skipped = item_ranges
        .iter()
        .enumerate()
        .filter_map(|(index, item_range)| {
            let item_range = item_range.as_ref()?;
            range.interior.contains(&item_range.start).then_some(index)
        })
        .collect::<Vec<_>>();

    let insert_index = skipped.first().copied().unwrap_or_else(|| {
        item_ranges
            .iter()
            .position(|item_range| {
                item_range
                    .as_ref()
                    .is_some_and(|item_range| range.interior.start < item_range.start)
            })
            .unwrap_or(item_ranges.len())
    });
    let skip_start = skipped.first().copied().unwrap_or(insert_index);
    let skip_end = skipped.last().map_or(skip_start, |last| last + 1);
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

fn strip_first_line_indent(text: &str) -> String {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let Some(first_line) = normalized.lines().find(|line| !line.trim().is_empty()) else {
        return normalized;
    };
    let indent = leading_indent(first_line);
    if indent.is_empty() {
        return normalized;
    }

    normalized
        .split('\n')
        .map(|line| line.strip_prefix(indent).unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
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

pub(crate) fn is_formatter_control_marker(comment: &str) -> bool {
    is_formatter_off_marker(comment) || is_formatter_on_marker(comment)
}
