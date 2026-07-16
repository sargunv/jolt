use std::borrow::Cow;

use jolt_fmt_ir::{ConcatBuilder, Doc, DocBuilder};
use jolt_kotlin_syntax::{
    KotlinComment, KotlinCommentKind, KotlinRoleElement, KotlinSyntaxToken, TerminatorList,
};
use jolt_syntax::RemovalClaim;

use crate::helpers::formatter_ignore::is_formatter_control_marker;
use crate::helpers::recovery::{KotlinFormatListPart, resolve_list_part};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LeadingTrivia {
    Preserve,
    SuppressAlreadyHandled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TrailingTrivia {
    Preserve,
    BeforeLineBreak,
    BeforeSoftLine,
    BeforeSpaceIfComments,
    RelocatedToEnclosingContext,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InlineLeadingTrivia {
    BeforeToken,
}

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

pub(crate) fn comments_from_tokens<'source>(
    tokens: impl IntoIterator<Item = KotlinSyntaxToken<'source>>,
) -> impl Iterator<Item = KotlinComment<'source>> {
    tokens
        .into_iter()
        .flat_map(|token| token.leading_comments().chain(token.trailing_comments()))
}

pub(crate) fn format_removed_comments<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = KotlinComment<'source>>,
) -> Option<Doc<'source>> {
    let mut has_claims = false;
    let mut has_output = false;
    let comments = doc.concat_list(|docs| {
        for comment in comments {
            if is_formatter_control_marker(comment.text()) {
                let claim = docs.claimed_trivia(Doc::nil(), comment.source_pieces());
                docs.push(claim);
                has_claims = true;
                continue;
            }
            if has_output {
                let hard_line = docs.hard_line();
                docs.push(hard_line);
            }
            let comment = format_comment(docs, &comment);
            docs.push(comment);
            has_claims = true;
            has_output = true;
        }
    });
    has_claims.then_some(comments)
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
    let comments = format_removed_comments(doc, comments_from_tokens([*token]));
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
    space_before_comments: bool,
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
                KotlinFormatListPart::Malformed(recovery) => {
                    docs.push(recovery);
                    continue;
                }
            };
            let claim = terminators.separator_removal_claim(token);
            let removed = format_removed_separator(docs, &token, claim, space_before_comments);
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
    format_token_doc(doc, token, token_doc, leading, trailing)
}

pub(crate) fn format_token_doc<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
    token_doc: Doc<'source>,
    leading: LeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    let leading = match leading {
        LeadingTrivia::Preserve => format_leading_comments(doc, token),
        LeadingTrivia::SuppressAlreadyHandled => doc.nil(),
    };
    let trailing = match trailing {
        TrailingTrivia::Preserve => format_trailing_comments(doc, token),
        TrailingTrivia::BeforeLineBreak => format_trailing_comments_before_line_break(doc, token),
        TrailingTrivia::BeforeSoftLine => {
            let comments = format_trailing_comments_before_line_break(doc, token);
            let line = if trailing_comments_force_line(token) {
                doc.hard_line()
            } else {
                doc.soft_line()
            };
            doc.concat([comments, line])
        }
        TrailingTrivia::BeforeSpaceIfComments => {
            if token.trailing_comments().is_empty() {
                doc.nil()
            } else {
                let comments = format_trailing_comments_before_line_break(doc, token);
                let line = if trailing_comments_force_line(token) {
                    doc.hard_line()
                } else {
                    doc.space()
                };
                doc.concat([comments, line])
            }
        }
        TrailingTrivia::RelocatedToEnclosingContext => doc.nil(),
    };
    doc.concat([leading, token_doc, trailing])
}

pub(crate) fn format_token_with_inline_leading_comments<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
    placement: InlineLeadingTrivia,
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
        match placement {
            InlineLeadingTrivia::BeforeToken => {
                let space = doc.space();
                doc.concat([comments, space])
            }
        }
    };
    let token = format_token_after_relocated_leading_comments(doc, token, trailing);
    doc.concat([leading, token])
}

pub(crate) fn format_comment<'source>(
    doc: &mut DocBuilder<'source>,
    comment: &KotlinComment<'source>,
) -> Doc<'source> {
    let formatted = match comment.kind() {
        KotlinCommentKind::Line | KotlinCommentKind::Block => {
            format_comment_lines(doc, preserved_comment_lines(comment.text()))
        }
        KotlinCommentKind::Doc => format_star_block_comment(doc, comment.text()),
    };
    doc.claimed_trivia(formatted, comment.source_pieces())
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

fn format_comment_lines<'source>(
    doc: &mut DocBuilder<'source>,
    lines: impl IntoIterator<Item = impl Into<Cow<'source, str>>>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for line in lines {
            if !docs.is_empty() {
                let hard_line = docs.hard_line();
                docs.push(hard_line);
            }
            let line = docs.text(line);
            docs.push(line);
        }
    })
}

fn format_star_block_comment<'source>(
    doc: &mut DocBuilder<'source>,
    comment: &'source str,
) -> Doc<'source> {
    let content = strip_block_comment_delimiters(doc, comment);
    doc.concat_list(|docs| {
        let open = docs.literal_text("/**");
        docs.push(open);

        let mut has_content = false;
        let mut pending_blank_lines = 0;
        for line in content.lines().map(normalize_star_block_body_line) {
            if line.is_empty() {
                if has_content {
                    pending_blank_lines += 1;
                }
                continue;
            }

            has_content = true;
            for _ in 0..pending_blank_lines {
                let blank = docs.literal_text(" *");
                push_comment_line(docs, blank);
            }
            pending_blank_lines = 0;
            let prefix = docs.literal_text(" * ");
            let line = docs.text(line);
            let line = docs.concat([prefix, line]);
            push_comment_line(docs, line);
        }

        let close = docs.literal_text(" */");
        push_comment_line(docs, close);
    })
}

fn push_comment_line<'source>(docs: &mut ConcatBuilder<'_, 'source>, line: Doc<'source>) {
    if !docs.is_empty() {
        let hard_line = docs.hard_line();
        docs.push(hard_line);
    }
    docs.push(line);
}

fn preserved_comment_lines(comment: &str) -> impl Iterator<Item = &str> {
    comment.trim().lines().map(str::trim)
}

fn strip_block_comment_delimiters<'source>(
    _doc: &mut DocBuilder<'_>,
    comment: &'source str,
) -> &'source str {
    comment
        .trim()
        .strip_prefix("/**")
        .or_else(|| comment.trim().strip_prefix("/*"))
        .unwrap_or(comment.trim())
        .strip_suffix("*/")
        .unwrap_or_else(|| {
            comment
                .trim()
                .strip_prefix("/**")
                .or_else(|| comment.trim().strip_prefix("/*"))
                .unwrap_or(comment.trim())
        })
}

fn normalize_star_block_body_line(line: &str) -> &str {
    line.trim_start()
        .strip_prefix('*')
        .map_or_else(|| line.trim(), str::trim_start)
}
