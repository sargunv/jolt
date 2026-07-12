//! Shared recovery and trivia-gap helpers used by per-language node accessors.
//!
//! These helpers operate on the borrowed syntax tree (`SyntaxToken<L: Language>`,
//! `Comment<'source>`), filtering tokens by source ranges and classifying gaps
//! between tokens as comment-only or whitespace-only. They exist here so the
//! language crates' node accessor modules don't each need to maintain a copy.

use crate::{Language, SyntaxToken, TriviaKind};

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

/// Returns whether `[start, end)` contains only represented whitespace,
/// newline, or comment trivia.
///
/// Unlike a raw source-gap scan, this walks the lossless token and trivia
/// representation once. Lexer-ignored bytes are not safe trivia: callers must
/// preserve them through a malformed-syntax path instead of selecting a valid
/// structured layout.
pub fn represented_range_is_trivia<'source, L: Language>(
    tokens: impl IntoIterator<Item = SyntaxToken<'source, L>>,
    start: usize,
    end: usize,
) -> bool {
    if start > end {
        return false;
    }
    if start == end {
        return true;
    }

    let mut cursor = start;
    for token in tokens {
        if !advance_over_trivia(
            token.leading(),
            token.offset().get(),
            start,
            end,
            &mut cursor,
        ) {
            return false;
        }

        let token_range = token.token_text_range();
        if ranges_overlap(
            token_range.start().get(),
            token_range.end().get(),
            start,
            end,
        ) {
            return false;
        }

        if !advance_over_trivia(
            token.trailing(),
            token_range.end().get(),
            start,
            end,
            &mut cursor,
        ) {
            return false;
        }
    }
    cursor >= end
}

fn advance_over_trivia(
    trivia: &[crate::SyntaxTrivia],
    mut offset: usize,
    start: usize,
    end: usize,
    cursor: &mut usize,
) -> bool {
    for piece in trivia {
        let piece_start = offset;
        offset += piece.text_len().get();
        if !ranges_overlap(piece_start, offset, start, end) {
            continue;
        }
        if piece.kind() == TriviaKind::Ignored {
            return false;
        }

        let overlap_start = piece_start.max(start);
        if overlap_start > *cursor {
            return false;
        }
        *cursor = (*cursor).max(offset.min(end));
    }
    true
}

const fn ranges_overlap(
    left_start: usize,
    left_end: usize,
    right_start: usize,
    right_end: usize,
) -> bool {
    left_start < right_end && left_end > right_start
}
