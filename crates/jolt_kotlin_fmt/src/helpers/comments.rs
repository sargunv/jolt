//! Kotlin-specific comment placement.
//!
//! Comment rendering and placement are shared in `jolt_fmt_ir::comments`. This
//! module owns only the Kotlin-specific placements: removed semicolon
//! separators and terminator lists.

use jolt_fmt_ir::{Doc, DocBuilder, InlineLeadingTrivia};
use jolt_kotlin_syntax::{KotlinRoleElement, KotlinSyntaxToken, TerminatorList};
use jolt_syntax::RemovalClaim;

use crate::helpers::recovery::{KotlinFormatListPart, resolve_list_part};

pub(crate) use jolt_fmt_ir::{
    LeadingTrivia, TrailingTrivia, comment_forces_line, delimiter_dangling_comments,
    format_comment, format_dangling_comments, format_leading_comments, format_removed_comments,
    format_separator_with_comments, format_token, format_token_after_relocated_leading_comments,
    has_delimiter_dangling_comments, token_has_comments, trailing_comments_force_line,
};

/// Kotlin keeps inline leading comments immediately before their token.
pub(crate) fn format_token_with_inline_leading_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    jolt_fmt_ir::format_token_with_inline_leading_comments(
        doc,
        token,
        InlineLeadingTrivia::BeforeToken,
        trailing,
    )
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
    let comments = format_removed_comments(
        doc,
        token.leading_comments().chain(token.trailing_comments()),
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
