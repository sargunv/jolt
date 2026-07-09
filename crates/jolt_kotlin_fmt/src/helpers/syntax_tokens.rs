use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::KotlinSyntaxToken;

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_leading_comments, format_trailing_comments,
    format_trailing_comments_before_line_break, trailing_comments_force_line,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FormatterInsertedToken {
    ImportKeyword,
    ImportAliasKeyword,
    PrecedenceParenthesis,
}

pub(crate) fn inserted_syntax_token<'source>(
    doc: &mut DocBuilder<'source>,
    spelling: &'static str,
    _reason: FormatterInsertedToken,
) -> Doc<'source> {
    doc.text(spelling)
}

pub(crate) fn format_token_with_normalized_text<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
    spelling: &'static str,
    reason: FormatterInsertedToken,
    leading: LeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    let leading = match leading {
        LeadingTrivia::Preserve => format_leading_comments(doc, token),
        LeadingTrivia::SuppressAlreadyHandled => doc.nil(),
    };
    // Source-backed normalized token: the source token provides trivia;
    // formatter policy provides the printed spelling.
    let token_doc = inserted_syntax_token(doc, spelling, reason);
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
