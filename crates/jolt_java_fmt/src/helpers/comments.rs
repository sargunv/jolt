//! Java-specific comment placement.
//!
//! Comment rendering and placement are shared in `jolt_fmt_ir::comments`. This
//! module owns only the Java-specific placements: construct-relocated leading
//! comments, removed-token salvage, and lexically ignored trivia.

use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{JavaComment, JavaSyntaxToken, RemovalClaim};

pub(crate) use jolt_fmt_ir::{
    InlineLeadingTrivia, LeadingTrivia, TrailingTrivia, comment_forces_line, comment_is_star_block,
    format_byte_order_mark, format_comment, format_dangling_comments,
    format_delimiter_dangling_comments, format_inline_trailing_comment_list,
    format_leading_comment_list, format_leading_comments, format_leading_comments_before_group,
    format_removed_comments, format_separator_with_comments, format_token,
    format_token_after_relocated_leading_comments, format_token_body as format_token_doc,
    format_token_with_inline_leading_comments, format_trailing_comment,
    format_trailing_comments_before_line_break, format_trailing_substitute,
    has_delimiter_dangling_comments, token_has_comments, trailing_comments_force_line,
};

pub(crate) fn comments_from_tokens<'source>(
    tokens: impl IntoIterator<Item = JavaSyntaxToken<'source>>,
) -> impl Iterator<Item = JavaComment<'source>> {
    tokens
        .into_iter()
        .flat_map(|token| token.leading_comments().chain(token.trailing_comments()))
}

pub(crate) fn has_removed_comments<'source>(
    comments: impl IntoIterator<Item = JavaComment<'source>>,
) -> bool {
    comments.into_iter().next().is_some()
}

pub(crate) fn format_construct_leading_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    format_leading_comment_list(
        doc,
        token
            .into_iter()
            .flat_map(JavaSyntaxToken::leading_comments),
    )
}

/// Formats a construct whose first token begins its line: the enclosing join
/// already emitted a hard line boundary in front of it, so the first token's
/// preserved leading comments keep lines of their own.
pub(crate) fn format_line_start_construct<'source, T>(
    doc: &mut DocBuilder<'source>,
    first_token: Option<JavaSyntaxToken<'source>>,
    format: impl FnOnce(&mut DocBuilder<'source>) -> T,
) -> T {
    match first_token {
        Some(token) => doc.with_line_start_leading(&token, format),
        None => format(doc),
    }
}

/// Removes a source token only when syntax issued the exact claim.
///
/// A denied claim is expected for malformed syntax and preserves the original
/// token and trivia instead of treating recovery as a formatter invariant.
pub(crate) fn format_token_removal<'source>(
    doc: &mut DocBuilder<'source>,
    token: &JavaSyntaxToken<'source>,
    claim: Option<RemovalClaim<'source>>,
) -> (Doc<'source>, bool) {
    let Some(claim) = claim else {
        return (format_token_with_comments(doc, token), false);
    };
    let removed = doc.removed_source(claim);
    let comments =
        format_removed_comments(doc, comments_from_tokens([*token])).unwrap_or_else(Doc::nil);
    (doc.concat([removed, comments]), true)
}

pub(crate) fn format_token_with_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &JavaSyntaxToken<'source>,
) -> Doc<'source> {
    format_token(
        doc,
        token,
        LeadingTrivia::Preserve,
        TrailingTrivia::Preserve,
    )
}

pub(crate) fn format_token_before_relocated_trailing_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &JavaSyntaxToken<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_token(
        doc,
        token,
        leading,
        TrailingTrivia::RelocatedToEnclosingContext,
    )
}

pub(crate) fn format_token_after_construct_leading_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &JavaSyntaxToken<'source>,
    construct_first_token: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    if construct_first_token == Some(token) {
        format_token_after_relocated_leading_comments(doc, token, TrailingTrivia::Preserve)
    } else {
        format_token_with_comments(doc, token)
    }
}
