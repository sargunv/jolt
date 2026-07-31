//! Kotlin-specific comment placement.
//!
//! Comment rendering and placement are shared in `jolt_fmt_ir::comments`. This
//! module owns only the Kotlin-specific placements: removed semicolon
//! separators and terminator lists.

use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{KotlinRoleElement, KotlinSyntaxToken, TerminatorList};
use jolt_syntax::RemovalClaim;

use crate::helpers::recovery::{KotlinFormatListPart, resolve_list_part};

pub(crate) use jolt_fmt_ir::{
    InlineLeadingTrivia, LeadingTrivia, TrailingTrivia, comment_forces_line,
    format_byte_order_mark, format_comment, format_dangling_comments,
    format_delimiter_dangling_comments, format_leading_comments,
    format_leading_comments_before_group, format_removed_comments, format_separator_with_comments,
    format_token, format_token_after_relocated_leading_comments,
    format_trailing_comment_list_before_line_break, format_trailing_comments_before_line_break,
    has_delimiter_dangling_comments, token_has_comments, trailing_comments_force_line,
};

/// Formats a construct whose first token begins its line: the enclosing join
/// already emitted a hard line boundary in front of it, so the first token's
/// preserved leading comments keep lines of their own.
pub(crate) fn format_line_start_construct<'source, T>(
    doc: &mut DocBuilder<'source>,
    first_token: Option<KotlinSyntaxToken<'source>>,
    format: impl FnOnce(&mut DocBuilder<'source>) -> T,
) -> T {
    match first_token {
        Some(token) => doc.with_line_start_leading(&token, format),
        None => format(doc),
    }
}

/// Formats a token glued to the previous token — a navigation operator or a
/// type colon — placing preserved leading comments in the previous token's
/// trailing form, the placement the reparse reads back identically.
pub(crate) fn format_glued_token<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
    leading: LeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    match leading {
        LeadingTrivia::Preserve => jolt_fmt_ir::format_token_with_inline_leading_comments(
            doc,
            token,
            InlineLeadingTrivia::AfterPreviousToken,
            trailing,
        ),
        LeadingTrivia::SuppressAlreadyHandled => format_token(doc, token, leading, trailing),
    }
}

pub(crate) fn format_removed_separator<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
    claim: Option<RemovalClaim<'source>>,
    space_before_comments: bool,
) -> Doc<'source> {
    let Some(claim) = claim else {
        return format_token(
            doc,
            token,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        );
    };
    let removed = doc.removed_source(claim);
    let trailing_is_relocated = doc.relocates_trailing_trivia(token);
    let comments = format_removed_comments(
        doc,
        token.leading_comments().chain(
            (!trailing_is_relocated)
                .then(|| token.trailing_comments())
                .into_iter()
                .flatten(),
        ),
    );
    match comments {
        Some(comments) if space_before_comments => {
            let space = doc.space();
            doc.concat([removed, space, comments])
        }
        Some(comments) => doc.concat([removed, comments]),
        None => removed,
    }
}

pub(crate) fn format_terminator_list<'source>(
    doc: &mut DocBuilder<'source>,
    terminators: &TerminatorList<'source>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for part in terminators.parts() {
            let token = match resolve_list_part(part, docs) {
                KotlinFormatListPart::Item(KotlinRoleElement::Token(token))
                | KotlinFormatListPart::Separator(token) => token,
                KotlinFormatListPart::Item(KotlinRoleElement::Node(_)) => {
                    docs.block_on_invariant("Kotlin terminator list contained a node");
                    continue;
                }
                KotlinFormatListPart::Recovery(recovery) => {
                    docs.push(recovery.doc());
                    continue;
                }
            };
            let claim = terminators.separator_removal_claim(token);
            let removed = format_removed_separator(docs, &token, claim, true);
            docs.push(removed);
        }
    })
}
