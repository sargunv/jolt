//! Shared `@formatter:off` / `@formatter:on` range handling.
//!
//! Both the Java and Kotlin formatters consume this module to discover
//! formatter-ignore ranges from token trivia and to splice the raw source
//! spanned by those ranges back into the rendered document.

use std::borrow::Cow;
#[cfg(test)]
use std::cell::Cell;
use std::ops::Range;

use jolt_syntax::{Comment, Language, SourceRangeClaim, SyntaxToken};
use jolt_text::{TextRange, TextSize};

use crate::source_fragment::{ExceptionalSeparators, exceptional_separators};
use crate::{
    Doc, DocBuilder, ExceptionalFragment, FragmentBoundary, LexicalAtom, LexicalAtomKind,
    LexicalSafety,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct FormatterIgnoreRange<'source> {
    raw_text: &'source str,
    raw_text_with_on: &'source str,
    interior: TextRange,
    claim_with_on: SourceRangeClaim<'source>,
    claim_without_on: SourceRangeClaim<'source>,
    separators_with_on: ExceptionalSeparators,
    separators_without_on: ExceptionalSeparators,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormatterIgnoreRun<'source> {
    range: FormatterIgnoreRange<'source>,
    pub insert_index: usize,
    pub skip_start: usize,
    pub skip_end: usize,
    on_marker_owner: OnMarkerOwner,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OnMarkerOwner {
    Item(usize),
    Boundary,
    IgnoreRun,
}

impl<'source> FormatterIgnoreRun<'source> {
    #[must_use]
    pub fn skips(&self, item_index: usize) -> bool {
        (self.skip_start..self.skip_end).contains(&item_index)
    }

    #[must_use]
    pub fn ends_with_on_marker(&self) -> bool {
        self.on_marker_owner == OnMarkerOwner::IgnoreRun
    }

    #[must_use]
    pub fn claimed_on_marker_range(&self) -> Option<Range<usize>> {
        self.ends_with_on_marker().then(|| {
            let start = self.range.interior.start().get();
            start..start + self.range.raw_text_with_on.len()
        })
    }

    fn raw_text(&self) -> &'source str {
        if self.ends_with_on_marker() {
            self.range.raw_text_with_on
        } else {
            self.range.raw_text
        }
    }
}

/// One formatting run's root-discovered formatter-ignore ranges.
///
/// Ignore-aware syntax lists query these ordered absolute ranges using their
/// own absolute interval and direct item ranges. The plan is immutable and can
/// be shared by every nested formatter rule in one run.
#[derive(Default)]
pub(crate) struct FormatterIgnorePlan<'source> {
    ranges: Vec<FormatterIgnoreRange<'source>>,
    #[cfg(test)]
    discovery_tokens: usize,
    #[cfg(test)]
    query_comparisons: Cell<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormatterIgnoreItemRange {
    owned: TextRange,
}

impl FormatterIgnoreItemRange {
    #[must_use]
    pub fn between<L: Language>(first: &SyntaxToken<'_, L>, last: &SyntaxToken<'_, L>) -> Self {
        Self {
            owned: TextRange::new(
                first.token_text_range().start(),
                last.token_text_range().end(),
            ),
        }
    }
}

impl FormatterIgnorePlan<'_> {
    #[cfg(test)]
    pub(crate) fn test_counts(&self) -> (usize, usize) {
        (self.ranges.len(), self.discovery_tokens)
    }

    #[cfg(test)]
    pub(crate) fn test_query_comparisons(&self) -> usize {
        self.query_comparisons.get()
    }
}

/// Returns the syntax-owned interval between optional source delimiters.
/// Missing/recovered delimiters retain the corresponding fallback boundary.
#[must_use]
pub fn formatter_ignore_content_range<L: Language>(
    fallback: TextRange,
    open: Option<SyntaxToken<'_, L>>,
    close: Option<SyntaxToken<'_, L>>,
) -> TextRange {
    let start = open.map_or(fallback.start(), |token| token.token_text_range().end());
    let end = close.map_or(fallback.end(), |token| token.token_text_range().start());
    if start <= end {
        TextRange::new(start, end)
    } else {
        fallback
    }
}

/// One step while splicing formatter-ignore runs into a linear item list.
#[derive(Clone, Copy, Debug)]
pub enum FormatterIgnoreSplice<'a, 'source> {
    /// Emit the ignored source run before the item at `insert_index` (or after
    /// the last item when trailing).
    Ignore(&'a FormatterIgnoreRun<'source>),
    /// Format the represented item at `index`. When `clear_blank_line_before` is
    /// set, the previous ignore run skipped the prior item and blank-line
    /// separators should be reset.
    Item {
        index: usize,
        clear_blank_line_before: bool,
    },
}

/// Walks `item_count` represented items together with precomputed ignore runs.
///
/// Callers own separators and item formatting; this only yields insert/skip
/// events in source order.
pub fn for_each_formatter_ignore_splice<'a, 'source>(
    item_count: usize,
    runs: &'a [FormatterIgnoreRun<'source>],
    mut visit: impl FnMut(FormatterIgnoreSplice<'a, 'source>),
) {
    let mut ignored_index = 0;
    let mut skip_index = 0;
    for index in 0..item_count {
        while ignored_index < runs.len() && runs[ignored_index].insert_index == index {
            visit(FormatterIgnoreSplice::Ignore(&runs[ignored_index]));
            ignored_index += 1;
        }
        while skip_index < runs.len() && runs[skip_index].skip_end <= index {
            skip_index += 1;
        }
        if skip_index < runs.len() && runs[skip_index].skips(index) {
            continue;
        }
        let clear_blank_line_before = skip_index > 0 && runs[skip_index - 1].skip_end == index;
        visit(FormatterIgnoreSplice::Item {
            index,
            clear_blank_line_before,
        });
    }
    while ignored_index < runs.len() {
        visit(FormatterIgnoreSplice::Ignore(&runs[ignored_index]));
        ignored_index += 1;
    }
}

/// Derives exact ignored runs for one source-ordered syntax item list.
/// Work is `O(items * log(ranges + 1) + items + runs)`; plan construction is
/// linear in root tokens and comments.
#[must_use]
pub(crate) fn formatter_ignore_runs<'source>(
    plan: &FormatterIgnorePlan<'source>,
    container: TextRange,
    item_ranges: &[Option<FormatterIgnoreItemRange>],
) -> Vec<FormatterIgnoreRun<'source>> {
    if plan.ranges.is_empty() || item_ranges.is_empty() {
        return Vec::new();
    }

    let mut runs = Vec::new();
    let mut index = 0;
    let mut pending_owner = None;
    while index < item_ranges.len() {
        let owner = pending_owner.take().or_else(|| {
            item_ranges[index]
                .and_then(|item| range_containing_start(plan, container, item.owned.start()))
        });
        let Some(range_index) = owner else {
            index += 1;
            continue;
        };

        let skip_start = index;
        let mut last_skipped = index;
        index += 1;
        while index < item_ranges.len() {
            let owner = item_ranges[index]
                .and_then(|item| range_containing_start(plan, container, item.owned.start()));
            match owner {
                Some(owner) if owner == range_index => last_skipped = index,
                Some(owner) => {
                    pending_owner = Some(owner);
                    break;
                }
                None => {}
            }
            index += 1;
        }
        let skip_end = last_skipped + 1;
        let raw = plan.ranges[range_index].interior;
        let skipped_are_owned = item_ranges[skip_start..skip_end]
            .iter()
            .flatten()
            .all(|item| raw.start() <= item.owned.start() && item.owned.end() <= raw.end());
        let overlaps_previous = item_ranges[..skip_start]
            .iter()
            .rev()
            .flatten()
            .next()
            .is_some_and(|item| raw.start() < item.owned.end() && item.owned.start() < raw.end());
        let next_physical = item_ranges[skip_end..]
            .iter()
            .position(Option::is_some)
            .map(|offset| skip_end + offset);
        let overlaps_next = next_physical
            .and_then(|next| item_ranges[next])
            .is_some_and(|item| raw.start() < item.owned.end() && item.owned.start() < raw.end());
        if !skipped_are_owned || overlaps_previous || overlaps_next {
            continue;
        }
        runs.push(FormatterIgnoreRun {
            range: plan.ranges[range_index].clone(),
            insert_index: skip_start,
            skip_start,
            skip_end,
            on_marker_owner: next_physical.map_or(OnMarkerOwner::Boundary, OnMarkerOwner::Item),
        });
    }

    for index in 0..runs.len().saturating_sub(1) {
        if let OnMarkerOwner::Item(owner) = runs[index].on_marker_owner
            && runs[index + 1].skips(owner)
            && runs[index].range.interior.start()
                + TextSize::new(runs[index].range.raw_text_with_on.len())
                <= runs[index + 1].range.interior.start()
        {
            runs[index].on_marker_owner = OnMarkerOwner::IgnoreRun;
        }
    }
    runs
}

pub(crate) fn formatter_ignore_may_apply(
    plan: &FormatterIgnorePlan<'_>,
    container: TextRange,
) -> bool {
    let index = plan
        .ranges
        .partition_point(|range| range.interior.start() < container.start());
    plan.ranges.get(index).is_some_and(|range| {
        container.start() <= range.interior.start() && range.interior.end() <= container.end()
    })
}

fn range_containing_start(
    plan: &FormatterIgnorePlan<'_>,
    container: TextRange,
    item_start: TextSize,
) -> Option<usize> {
    let index = plan.ranges.partition_point(|range| {
        #[cfg(test)]
        plan.query_comparisons
            .set(plan.query_comparisons.get().saturating_add(1));
        range.interior.end() <= item_start
    });
    let range = plan.ranges.get(index)?;
    (container.start() <= range.interior.start()
        && range.interior.end() <= container.end()
        && range.interior.start() <= item_start
        && item_start < range.interior.end())
    .then_some(index)
}

pub(crate) fn formatter_ignore_plan_with_safety<'source, L: Language>(
    source: &'source str,
    tokens: impl IntoIterator<Item = SyntaxToken<'source, L>>,
    safety: &mut impl LexicalSafety<L>,
) -> FormatterIgnorePlan<'source> {
    // Formatter control markers are rare. Avoid walking token trivia when the
    // root source cannot contain one.
    if !source.contains("@formatter:") {
        return FormatterIgnorePlan::default();
    }

    let tokens: Vec<_> = tokens.into_iter().collect();
    let Some(claim_anchor) = tokens.first() else {
        return FormatterIgnorePlan::default();
    };

    let mut off_comment_start = None;
    let mut ranges = Vec::new();
    let mut lines = SourceLineCursor::new(source);

    let mut visit_comment =
        |comment: Comment<'source>, leading_comment_start: &mut Option<usize>| {
            let range = comment.text_range();
            let start_offset = range.start().get();
            let end_offset = range.end().get();
            let line = lines.comment_line(start_offset);
            let end_line = lines.comment_line(end_offset.saturating_sub(1).max(start_offset));
            let comment_text = comment.text();
            // A complete pair is first-off-wins: nested/repeated off markers
            // remain ordinary raw contents until the matching on marker.
            if is_formatter_off_marker(comment_text) && off_comment_start.is_none() {
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
                        interior: TextRange::new(TextSize::new(start), TextSize::new(end)),
                        claim_with_on: claim_anchor.source_range_claim(
                            TextRange::new(
                                TextSize::new(start),
                                TextSize::new(
                                    start
                                        + strip_trailing_line_ending(
                                            &source[start..end_line.raw_end],
                                        )
                                        .len(),
                                ),
                            ),
                            true,
                        ),
                        claim_without_on: claim_anchor.source_range_claim(
                            TextRange::new(
                                TextSize::new(start),
                                TextSize::new(
                                    start + strip_trailing_line_ending(&source[start..end]).len(),
                                ),
                            ),
                            false,
                        ),
                        separators_with_on: ExceptionalSeparators {
                            before: crate::ExceptionalSeparator::None,
                            after: crate::ExceptionalSeparator::None,
                        },
                        separators_without_on: ExceptionalSeparators {
                            before: crate::ExceptionalSeparator::None,
                            after: crate::ExceptionalSeparator::None,
                        },
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

    populate_separators(&mut ranges, &tokens, safety);
    FormatterIgnorePlan {
        ranges,
        #[cfg(test)]
        discovery_tokens: tokens.len(),
        #[cfg(test)]
        query_comparisons: Cell::new(0),
    }
}

#[derive(Clone, Copy, Default)]
struct RangeBoundary<'source> {
    first: Option<LexicalAtom<'source>>,
    last: Option<LexicalAtom<'source>>,
}

#[derive(Clone, Copy, Default)]
struct RangeBoundaries<'source> {
    with_on: RangeBoundary<'source>,
    without_on: RangeBoundary<'source>,
}

fn populate_separators<L: Language>(
    ranges: &mut [FormatterIgnoreRange<'_>],
    tokens: &[SyntaxToken<'_, L>],
    safety: &mut impl LexicalSafety<L>,
) {
    let mut boundaries = vec![RangeBoundaries::default(); ranges.len()];
    let mut range_index = 0;
    for token in tokens {
        let token_range = token.token_text_range();
        for comment in token.leading_comments() {
            record_boundary_atom(
                ranges,
                &mut boundaries,
                &mut range_index,
                comment.text_range(),
                LexicalAtom::new(LexicalAtomKind::Comment, comment.text()),
            );
        }
        if !token.text().is_empty() {
            record_boundary_atom(
                ranges,
                &mut boundaries,
                &mut range_index,
                token_range,
                LexicalAtom::new(safety.classify(token), token.text()),
            );
        }
        for comment in token.trailing_comments() {
            record_boundary_atom(
                ranges,
                &mut boundaries,
                &mut range_index,
                comment.text_range(),
                LexicalAtom::new(LexicalAtomKind::Comment, comment.text()),
            );
        }
    }

    let mut previous_cursor = 0;
    let mut next_without_cursor = 0;
    let mut next_with_cursor = 0;
    let mut previous = None;
    for (index, range) in ranges.iter_mut().enumerate() {
        let start = range.interior.start();
        while let Some(token) = tokens.get(previous_cursor) {
            if token.token_text_range().end() > start {
                break;
            }
            if !token.text().is_empty() {
                previous = Some(LexicalAtom::new(safety.classify(token), token.text()));
            }
            previous_cursor += 1;
        }
        let without_end = TextSize::new(start.get() + range.raw_text.len());
        let with_end = TextSize::new(start.get() + range.raw_text_with_on.len());
        let next_without = next_token_atom(tokens, &mut next_without_cursor, without_end, safety);
        let next_with = next_token_atom(tokens, &mut next_with_cursor, with_end, safety);
        range.separators_without_on =
            separators(previous, boundaries[index].without_on, next_without, safety);
        range.separators_with_on =
            separators(previous, boundaries[index].with_on, next_with, safety);
    }
}

fn record_boundary_atom<'source>(
    ranges: &[FormatterIgnoreRange<'source>],
    boundaries: &mut [RangeBoundaries<'source>],
    range_index: &mut usize,
    atom_range: TextRange,
    atom: LexicalAtom<'source>,
) {
    while let Some(range) = ranges.get(*range_index) {
        let start = range.interior.start();
        let with_end = TextSize::new(start.get() + range.raw_text_with_on.len());
        if atom_range.start() < with_end {
            break;
        }
        *range_index += 1;
    }
    let Some(range) = ranges.get(*range_index) else {
        return;
    };
    let start = range.interior.start();
    let with_end = TextSize::new(start.get() + range.raw_text_with_on.len());
    if start <= atom_range.start() && atom_range.end() <= with_end {
        boundaries[*range_index].with_on.record(atom);
        let without_end = TextSize::new(start.get() + range.raw_text.len());
        if atom_range.end() <= without_end {
            boundaries[*range_index].without_on.record(atom);
        }
    }
}

impl<'source> RangeBoundary<'source> {
    fn record(&mut self, atom: LexicalAtom<'source>) {
        self.first.get_or_insert(atom);
        self.last = Some(atom);
    }
}

fn next_token_atom<'source, L: Language>(
    tokens: &[SyntaxToken<'source, L>],
    cursor: &mut usize,
    end: TextSize,
    safety: &mut impl LexicalSafety<L>,
) -> Option<LexicalAtom<'source>> {
    while let Some(token) = tokens.get(*cursor) {
        if end <= token.token_text_range().start() && !token.text().is_empty() {
            return Some(LexicalAtom::new(safety.classify(token), token.text()));
        }
        *cursor += 1;
    }
    None
}

fn separators<'source, L: Language>(
    previous: Option<LexicalAtom<'source>>,
    boundary: RangeBoundary<'source>,
    next: Option<LexicalAtom<'source>>,
    safety: &mut impl LexicalSafety<L>,
) -> ExceptionalSeparators {
    exceptional_separators(
        previous,
        ExceptionalFragment::new(
            Doc::nil(),
            FragmentBoundary {
                first: boundary.first,
                last: boundary.last,
                // Formatter-ignore runs are line-oriented section documents;
                // their caller owns the terminating hard line. Lexical safety
                // here only resolves fusing token/comment joins.
                ends_with_line_comment: false,
            },
        ),
        next,
        safety,
    )
}

#[must_use]
pub fn formatter_ignore_run_doc<'source>(
    run: &FormatterIgnoreRun<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let raw_text = run.raw_text();
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
    let claim = if run.ends_with_on_marker() {
        run.range.claim_with_on
    } else {
        run.range.claim_without_on
    };
    let separators = if run.ends_with_on_marker() {
        run.range.separators_with_on
    } else {
        run.range.separators_without_on
    };
    doc.formatter_ignore_source(contents, claim, separators)
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
    use jolt_java_syntax::{JavaLanguage, JavaSyntaxView, parse_compilation_unit};
    use jolt_syntax::{SyntaxNode, SyntaxToken};
    use jolt_text::TextRange;

    use crate::{ExceptionalSeparator, LexicalAtom, LexicalAtomKind, LexicalSafety};

    use super::{
        CommentLine, FormatterIgnoreItemRange, FormatterIgnoreSplice, SourceLineCursor,
        for_each_formatter_ignore_splice, formatter_ignore_content_range,
        formatter_ignore_plan_with_safety, formatter_ignore_runs, is_formatter_on_marker,
    };

    #[derive(Default)]
    struct TestSafety;

    impl LexicalSafety<JavaLanguage> for TestSafety {
        fn classify(&mut self, _token: &SyntaxToken<'_, JavaLanguage>) -> LexicalAtomKind {
            LexicalAtomKind::Identifier
        }

        fn separator(
            &mut self,
            _left: LexicalAtom<'_>,
            _right: LexicalAtom<'_>,
        ) -> ExceptionalSeparator {
            ExceptionalSeparator::Space
        }
    }

    fn token_range(root: &SyntaxNode<'_, JavaLanguage>, text: &str) -> TextRange {
        root.tokens()
            .find(|token| token.text() == text)
            .unwrap_or_else(|| panic!("missing token {text:?}"))
            .token_text_range()
    }

    fn item_range(root: &SyntaxNode<'_, JavaLanguage>, text: &str) -> FormatterIgnoreItemRange {
        let token = root
            .tokens()
            .find(|token| token.text() == text)
            .unwrap_or_else(|| panic!("missing token {text:?}"));
        FormatterIgnoreItemRange::between(&token, &token)
    }

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
    fn only_complete_pairs_form_ranges_and_first_off_wins() {
        let repeated = "class C {\n// @formatter:off\nint first;\n// @formatter:off\nint second;\n// @formatter:on\n}";
        let parse = parse_compilation_unit(repeated);
        let syntax = parse.syntax().expect("test source has syntax");
        let root = syntax.syntax_node().expect("test source has a root");
        let plan = formatter_ignore_plan_with_safety(repeated, root.tokens(), &mut TestSafety);
        assert_eq!(plan.ranges.len(), 1);
        assert_eq!(
            plan.ranges[0].interior.start().get(),
            repeated.find("// @formatter:off").unwrap()
        );
        assert!(plan.ranges[0].raw_text.contains("int first"));
        assert!(
            plan.ranges[0]
                .raw_text
                .contains("// @formatter:off\nint second")
        );

        for unmatched in [
            "class C {\n// @formatter:off\nint value;\n}",
            "class C {\n// @formatter:on\nint value;\n}",
        ] {
            let parse = parse_compilation_unit(unmatched);
            let syntax = parse.syntax().expect("test source has syntax");
            let root = syntax.syntax_node().expect("test source has a root");
            let plan = formatter_ignore_plan_with_safety(unmatched, root.tokens(), &mut TestSafety);
            assert!(plan.ranges.is_empty());
        }
    }

    #[test]
    fn same_line_on_off_transition_has_disjoint_exact_output() {
        let source = "class C {\n// @formatter:off\nint first;\n/* @formatter:on */ /* @formatter:off */\nint second;\n// @formatter:on\nint after;\n}";
        let parse = parse_compilation_unit(source);
        let syntax = parse.syntax().expect("test source has syntax");
        let root = syntax.syntax_node().expect("test source has a root");
        let plan = formatter_ignore_plan_with_safety(source, root.tokens(), &mut TestSafety);
        let runs = formatter_ignore_runs(
            &plan,
            root.text_range(),
            &[
                Some(item_range(&root, "first")),
                Some(item_range(&root, "second")),
                Some(item_range(&root, "after")),
            ],
        );
        assert_eq!(runs.len(), 2);
        assert!(!runs[0].ends_with_on_marker());
        assert_eq!(runs[0].raw_text(), "// @formatter:off\nint first;");
        assert_eq!(
            runs[1].raw_text(),
            "/* @formatter:on */ /* @formatter:off */\nint second;"
        );
        let first_end = runs[0].range.interior.start().get() + runs[0].raw_text().len();
        assert!(first_end <= runs[1].range.interior.start().get());
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

    #[test]
    fn partial_sibling_range_is_rejected_before_overlapping_a_retained_item() {
        let source = "class Outer { void first() {\n// @formatter:off\nint raw=1;\n} void second() {}\n// @formatter:on\nvoid after() {} }";
        let parse = parse_compilation_unit(source);
        let syntax = parse.syntax().expect("test source has syntax");
        let root = syntax.syntax_node().expect("test source has a root");
        let plan = formatter_ignore_plan_with_safety(source, root.tokens(), &mut TestSafety);
        let retained_first = TextRange::new(
            token_range(&root, "void").start(),
            jolt_text::TextSize::new(
                source
                    .find("} void second")
                    .expect("first method close exists")
                    + 1,
            ),
        );
        let raw = plan.ranges[0].interior;
        assert!(
            raw.start() < retained_first.end() && retained_first.start() < raw.end(),
            "characterization requires an overlapping partial sibling",
        );
        let parent_items = [
            Some(FormatterIgnoreItemRange {
                owned: retained_first,
            }),
            Some(item_range(&root, "second")),
            Some(item_range(&root, "after")),
        ];
        assert!(formatter_ignore_runs(&plan, root.text_range(), &parent_items).is_empty());

        let child_end = token_range(&root, "raw").end();
        let child = TextRange::new(token_range(&root, "first").start(), child_end);
        let child_runs = formatter_ignore_runs(&plan, child, &[Some(item_range(&root, "raw"))]);
        assert!(child_runs.is_empty());
    }

    #[test]
    fn child_owns_a_range_nested_inside_its_parent_item() {
        let source = "class Outer { void method() {\n// @formatter:off\nint raw=1;\n// @formatter:on\nint after=2;\n} }";
        let parse = parse_compilation_unit(source);
        let syntax = parse.syntax().expect("test source has syntax");
        let root = syntax.syntax_node().expect("test source has a root");
        let plan = formatter_ignore_plan_with_safety(source, root.tokens(), &mut TestSafety);
        let parent = formatter_ignore_runs(
            &plan,
            root.text_range(),
            &[Some(item_range(&root, "class"))],
        );
        assert!(parent.is_empty());

        let child = TextRange::new(
            token_range(&root, "method").start(),
            token_range(&root, "after").end(),
        );
        let child_runs = formatter_ignore_runs(
            &plan,
            child,
            &[
                Some(item_range(&root, "raw")),
                Some(item_range(&root, "after")),
            ],
        );
        assert_eq!(child_runs.len(), 1);
        assert_eq!(child_runs[0].skip_start..child_runs[0].skip_end, 0..1);
        assert_eq!(
            child_runs,
            formatter_ignore_runs(
                &plan,
                child,
                &[
                    Some(item_range(&root, "raw")),
                    Some(item_range(&root, "after")),
                ],
            )
        );
    }

    #[test]
    fn content_boundaries_reject_header_claims_with_present_or_missing_open() {
        let source = "class C { /* @formatter:off */\nint raw;\n// @formatter:on\n}";
        let parse = parse_compilation_unit(source);
        let syntax = parse.syntax().expect("test source has syntax");
        let root = syntax.syntax_node().expect("test source has a root");
        let plan = formatter_ignore_plan_with_safety(source, root.tokens(), &mut TestSafety);
        let open = root.tokens().find(|token| token.text() == "{").unwrap();
        let close = root.tokens().find(|token| token.text() == "}").unwrap();
        let raw = item_range(&root, "raw");
        let fallback = raw.owned;

        for container in [
            formatter_ignore_content_range(fallback, Some(open), Some(close)),
            formatter_ignore_content_range(fallback, None, Some(close)),
        ] {
            assert!(formatter_ignore_runs(&plan, container, &[Some(raw)]).is_empty());
        }
    }

    #[test]
    fn leading_layout_and_non_control_comment_stay_in_one_supported_raw_run() {
        let source = "class C {\n// ordinary\n// @formatter:off\nint raw=1;\n// @formatter:on\nint after=2;\n}";
        let parse = parse_compilation_unit(source);
        let syntax = parse.syntax().expect("test source has syntax");
        let root = syntax.syntax_node().expect("test source has a root");
        let plan = formatter_ignore_plan_with_safety(source, root.tokens(), &mut TestSafety);
        let runs = formatter_ignore_runs(
            &plan,
            root.text_range(),
            &[
                Some(item_range(&root, "raw")),
                Some(item_range(&root, "after")),
            ],
        );
        assert_eq!(runs.len(), 1);
        assert!(
            runs[0]
                .raw_text()
                .starts_with("// ordinary\n// @formatter:off")
        );
        assert!(!runs[0].ends_with_on_marker());
    }

    #[test]
    fn enum_constant_member_crossing_is_rejected_by_the_member_interval() {
        let source =
            "enum E { A,\n// @formatter:off\nB;\nint member;\n// @formatter:on\nint after;\n}";
        let parse = parse_compilation_unit(source);
        let syntax = parse.syntax().expect("test source has syntax");
        let root = syntax.syntax_node().expect("test source has a root");
        let plan = formatter_ignore_plan_with_safety(source, root.tokens(), &mut TestSafety);
        let member_items = [
            Some(item_range(&root, "member")),
            Some(item_range(&root, "after")),
        ];

        let member_container = TextRange::new(
            token_range(&root, "member").start(),
            token_range(&root, "after").end(),
        );
        assert!(plan.ranges[0].interior.start() < member_container.start());
        assert!(formatter_ignore_runs(&plan, member_container, &member_items).is_empty());
    }

    #[test]
    fn adjacent_runs_own_the_intervening_on_marker_once() {
        let source = "class C { void m() {\n// @formatter:off\nint first=1;\n// @formatter:on\n// @formatter:off\nint second=2;\n// @formatter:on\nint after=3;\n} }";
        let parse = parse_compilation_unit(source);
        let syntax = parse.syntax().expect("test source has syntax");
        let root = syntax.syntax_node().expect("test source has a root");
        let plan = formatter_ignore_plan_with_safety(source, root.tokens(), &mut TestSafety);
        let runs = formatter_ignore_runs(
            &plan,
            root.text_range(),
            &[
                Some(item_range(&root, "first")),
                Some(item_range(&root, "second")),
                Some(item_range(&root, "after")),
            ],
        );
        assert_eq!(runs.len(), 2);
        assert!(runs[0].ends_with_on_marker());
        assert!(!runs[1].ends_with_on_marker());
        assert!(runs[0].claimed_on_marker_range().is_some());
    }

    #[test]
    fn terminal_run_leaves_the_on_marker_to_the_eof_boundary() {
        let source = "class C {\n// @formatter:off\nint raw=1;\n// @formatter:on\n}";
        let parse = parse_compilation_unit(source);
        let syntax = parse.syntax().expect("test source has syntax");
        let root = syntax.syntax_node().expect("test source has a root");
        let plan = formatter_ignore_plan_with_safety(source, root.tokens(), &mut TestSafety);
        let runs =
            formatter_ignore_runs(&plan, root.text_range(), &[Some(item_range(&root, "raw"))]);
        assert_eq!(runs.len(), 1);
        assert!(!runs[0].ends_with_on_marker());
        assert!(runs[0].claimed_on_marker_range().is_none());
    }

    #[test]
    fn represented_malformed_item_can_be_fully_contained() {
        let source = "class C {\n// @formatter:off\n+ ;\n// @formatter:on\nint after;\n}";
        let parse = parse_compilation_unit(source);
        let syntax = parse.syntax().expect("test source has syntax");
        let root = syntax.syntax_node().expect("test source has a root");
        let plan = formatter_ignore_plan_with_safety(source, root.tokens(), &mut TestSafety);
        let runs = formatter_ignore_runs(
            &plan,
            root.text_range(),
            &[
                Some(item_range(&root, "+")),
                Some(item_range(&root, "after")),
            ],
        );
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].skip_start..runs[0].skip_end, 0..1);
    }

    #[test]
    fn missing_parts_between_physical_items_stay_inside_the_skip_span() {
        let source =
            "class C {\n// @formatter:off\nint first; int second;\n// @formatter:on\nint after; }";
        let parse = parse_compilation_unit(source);
        let syntax = parse.syntax().expect("test source has syntax");
        let root = syntax.syntax_node().expect("test source has a root");
        let plan = formatter_ignore_plan_with_safety(source, root.tokens(), &mut TestSafety);
        let item_ranges = [
            Some(item_range(&root, "first")),
            None,
            Some(item_range(&root, "second")),
            Some(item_range(&root, "after")),
        ];
        let runs = formatter_ignore_runs(&plan, root.text_range(), &item_ranges);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].skip_start..runs[0].skip_end, 0..3);

        let mut visited = Vec::new();
        for_each_formatter_ignore_splice(item_ranges.len(), &runs, |event| match event {
            FormatterIgnoreSplice::Ignore(_) => visited.push("ignore"),
            FormatterIgnoreSplice::Item { index, .. } => {
                visited.push(if index == 3 { "after" } else { "unexpected" });
            }
        });
        assert_eq!(visited, ["ignore", "after"]);
    }

    #[test]
    fn query_comparisons_are_bounded_by_items_and_range_depth() {
        let mut source = String::from("class C { void m() {\n");
        for index in 0..64 {
            source.push_str("// @formatter:off\n");
            source.push_str(&format!("int value{index}=0;\n"));
            source.push_str("// @formatter:on\n");
        }
        source.push_str("} }");
        let parse = parse_compilation_unit(&source);
        let syntax = parse.syntax().expect("test source has syntax");
        let root = syntax.syntax_node().expect("test source has a root");
        let token_count = root.tokens().count();
        let plan = formatter_ignore_plan_with_safety(&source, root.tokens(), &mut TestSafety);
        let items = (0..64)
            .map(|index| Some(item_range(&root, &format!("value{index}"))))
            .collect::<Vec<_>>();
        let runs = formatter_ignore_runs(&plan, root.text_range(), &items);
        let (range_count, discovery_tokens) = plan.test_counts();
        assert_eq!(runs.len(), 64);
        assert_eq!(range_count, 64);
        assert_eq!(discovery_tokens, token_count);
        let comparisons = plan.test_query_comparisons();
        let max_probes = usize::BITS - range_count.leading_zeros();
        assert!(comparisons > 0);
        assert!(comparisons <= items.len() * usize::try_from(max_probes).unwrap());
    }
}
