//! Shared recovery and trivia-gap helpers used by per-language node accessors.
//!
//! These helpers operate on the borrowed syntax tree (`SyntaxToken<L: Language>`,
//! `Comment<'source>`), filtering tokens by source ranges and classifying gaps
//! between tokens as comment-only or whitespace-only. They exist here so the
//! language crates' node accessor modules don't each need to maintain a copy.

use crate::{Comment, Language, SyntaxToken};
use jolt_text::TextRange;

/// Filters an iterator of tokens to those whose `token_text_range` lies fully
/// inside `[start, end]`.
pub fn tokens_between<'source, L: Language>(
    tokens: impl IntoIterator<Item = SyntaxToken<'source, L>>,
    start: usize,
    end: usize,
) -> impl Iterator<Item = SyntaxToken<'source, L>> {
    tokens.into_iter().filter(move |token| {
        let range = token.token_text_range();
        range.start().get() >= start && range.end().get() <= end
    })
}

/// Returns true when the source gap `[start, end)` between tokens contains only
/// whitespace and comment trivia (no represented tokens).
///
/// `source` is the borrowed source text and `source_start` is the source offset
/// of the slice the byte indices are relative to.
pub fn source_gap_is_trivia<'source, L: Language>(
    source: &'source str,
    source_start: usize,
    tokens: impl IntoIterator<Item = SyntaxToken<'source, L>>,
    start: usize,
    end: usize,
) -> bool {
    let mut comment_ranges: Vec<(usize, usize)> = tokens
        .into_iter()
        .flat_map(|token| {
            let leading = token.leading_comments();
            let trailing = token.trailing_comments();
            leading.chain(trailing)
        })
        .filter_map(|comment: Comment<'_>| {
            let range: TextRange = comment.text_range();
            let comment_start = range.start().get();
            let comment_end = range.end().get();
            (comment_start >= start && comment_end <= end).then_some((comment_start, comment_end))
        })
        .collect();
    comment_ranges.sort_unstable();

    let mut cursor = start;
    for (comment_start, comment_end) in comment_ranges {
        if comment_start < cursor {
            if comment_end > cursor {
                cursor = comment_end;
            }
            continue;
        }
        if !source_slice_is_whitespace(source, source_start, cursor, comment_start) {
            return false;
        }
        cursor = comment_end;
    }

    source_slice_is_whitespace(source, source_start, cursor, end)
}

/// Returns true when `source[source_start + (start - source_start) .. source_start + (end - source_start)]`
/// contains only whitespace.
pub fn source_slice_is_whitespace(
    source: &str,
    source_start: usize,
    start: usize,
    end: usize,
) -> bool {
    let Some(slice_start) = start.checked_sub(source_start) else {
        return false;
    };
    let Some(slice_end) = end.checked_sub(source_start) else {
        return false;
    };
    let Some(slice) = source.get(slice_start..slice_end) else {
        return false;
    };

    slice.chars().all(char::is_whitespace)
}
