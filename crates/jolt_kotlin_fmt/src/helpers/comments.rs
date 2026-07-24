use jolt_fmt_ir::{
    Doc, DocBuilder, format_comment_lines, format_star_block_comment,
    format_token_doc as assemble_token_doc, is_empty_single_line_block_comment,
    preserved_block_comment_lines, preserved_comment_lines,
};
use jolt_kotlin_syntax::{
    KotlinComment, KotlinCommentKind, KotlinRoleElement, KotlinSyntaxToken, TerminatorList,
};
use jolt_syntax::RemovalClaim;

use crate::helpers::recovery::{KotlinFormatListPart, resolve_list_part};

pub(crate) use jolt_fmt_ir::{LeadingTrivia, TrailingTrivia};

pub(crate) fn format_leading_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    format_leading_comment_list(doc, token.leading_comments())
}

pub(crate) fn format_leading_comment_list<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = KotlinComment<'source>>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for comment in comments {
            let comment = format_comment(docs, &comment);
            docs.push(comment);
            let hard_line = docs.hard_line();
            docs.push(hard_line);
        }
    })
}

pub(crate) fn format_trailing_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for comment in token.trailing_comments() {
            let space = docs.space();
            docs.push(space);
            let comment_doc = format_comment(docs, &comment);
            docs.push(comment_doc);
            if comment_forces_line(&comment) {
                let hard_line = docs.hard_line();
                docs.push(hard_line);
            }
        }
    })
}

pub(crate) fn format_trailing_comments_before_line_break<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    let mut comments = token.trailing_comments().peekable();
    doc.concat_list(|docs| {
        while let Some(comment) = comments.next() {
            let space = docs.space();
            docs.push(space);
            let comment_doc = format_comment(docs, &comment);
            docs.push(comment_doc);
            if comments.peek().is_some() && comment_forces_line(&comment) {
                let hard_line = docs.hard_line();
                docs.push(hard_line);
            }
        }
    })
}

pub(crate) fn format_dangling_comments<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = KotlinComment<'source>>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for comment in comments {
            if !docs.is_empty() {
                let hard_line = docs.hard_line();
                docs.push(hard_line);
            }
            let comment = format_comment(docs, &comment);
            docs.push(comment);
        }
    })
}

pub(crate) fn format_removed_comments<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = KotlinComment<'source>>,
) -> Option<Doc<'source>> {
    let mut has_comments = false;
    let comments = doc.concat_list(|docs| {
        for comment in comments {
            if has_comments {
                let hard_line = docs.hard_line();
                docs.push(hard_line);
            }
            let comment = format_comment(docs, &comment);
            docs.push(comment);
            has_comments = true;
        }
    });
    has_comments.then_some(comments)
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

pub(crate) fn has_delimiter_dangling_comments<'source>(
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
) -> bool {
    open.is_some_and(|token| !token.trailing_comments().is_empty())
        || close.is_some_and(|token| !token.leading_comments().is_empty())
}

pub(crate) fn delimiter_dangling_comments<'source>(
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
) -> impl Iterator<Item = KotlinComment<'source>> {
    open.into_iter()
        .flat_map(KotlinSyntaxToken::trailing_comments)
        .chain(
            close
                .into_iter()
                .flat_map(KotlinSyntaxToken::leading_comments),
        )
}

pub(crate) fn format_separator_with_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
    unforced_break: Doc<'source>,
) -> Doc<'source> {
    let token_doc = format_token(
        doc,
        token,
        LeadingTrivia::Preserve,
        TrailingTrivia::BeforeLineBreak,
    );
    let line = if trailing_comments_force_line(token) {
        doc.hard_line()
    } else {
        unforced_break
    };
    doc.concat([token_doc, line])
}

pub(crate) fn format_token_after_relocated_leading_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    format_token(doc, token, LeadingTrivia::SuppressAlreadyHandled, trailing)
}

pub(crate) fn format_token<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
    leading: LeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    let token_doc = doc.source_token(token);
    assemble_token_doc(
        doc,
        token_doc,
        leading,
        trailing,
        |doc| format_leading_comments(doc, token),
        |doc| format_trailing_comments(doc, token),
        |doc| format_trailing_comments_before_line_break(doc, token),
        trailing_comments_force_line(token),
        !token.trailing_comments().is_empty(),
    )
}

pub(crate) fn format_token_with_inline_leading_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    let leading = token.leading_comments();
    let leading = if leading.is_empty() {
        doc.nil()
    } else {
        let comments = leading
            .map(|comment| format_comment(doc, &comment))
            .collect::<Vec<_>>();
        let space = doc.space();
        let comments = doc.join(space, comments);
        let space = doc.space();
        doc.concat([comments, space])
    };
    let token = format_token_after_relocated_leading_comments(doc, token, trailing);
    doc.concat([leading, token])
}

pub(crate) fn format_comment<'source>(
    doc: &mut DocBuilder<'source>,
    comment: &KotlinComment<'source>,
) -> Doc<'source> {
    doc.source_trivia(comment.source_pieces(), |doc| {
        if !comment.is_terminated() {
            return doc.literal_text(comment.text());
        }

        if is_empty_single_line_block_comment(comment.text()) {
            return format_comment_lines(doc, preserved_comment_lines(comment.text()));
        }

        match comment.kind() {
            KotlinCommentKind::Line => {
                format_comment_lines(doc, preserved_comment_lines(comment.text()))
            }
            KotlinCommentKind::Block => {
                format_comment_lines(doc, preserved_block_comment_lines(comment.text()))
            }
            KotlinCommentKind::Doc => format_star_block_comment(doc, comment.text(), "/**"),
        }
    })
}

pub(crate) fn comment_forces_line(comment: &KotlinComment<'_>) -> bool {
    comment.kind() == KotlinCommentKind::Line || comment.text().contains(['\n', '\r'])
}

pub(crate) fn trailing_comments_force_line(token: &KotlinSyntaxToken<'_>) -> bool {
    token
        .trailing_comments()
        .any(|comment| comment_forces_line(&comment))
}

pub(crate) fn token_has_comments(token: &KotlinSyntaxToken<'_>) -> bool {
    !token.leading_comments().is_empty() || !token.trailing_comments().is_empty()
}
