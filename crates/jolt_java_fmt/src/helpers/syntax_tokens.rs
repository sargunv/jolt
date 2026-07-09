use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::JavaSyntaxToken;

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_leading_comments, format_trailing_comments,
    format_trailing_comments_before_line_break, trailing_comments_force_line,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FormatterInsertedToken {
    BlockBrace,
    PrecedenceParenthesis,
    TrailingComma,
    EnumSeparator,
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
    token: &JavaSyntaxToken<'source>,
    spelling: &'static str,
    reason: FormatterInsertedToken,
    leading: LeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            match leading {
                LeadingTrivia::Preserve => format_leading_comments(doc, token),
                LeadingTrivia::SuppressAlreadyHandled => Doc::nil(),
            },
            // Source-backed normalized token: the source token provides trivia;
            // formatter policy provides the printed spelling.
            inserted_syntax_token(doc, spelling, reason),
            match trailing {
                TrailingTrivia::Preserve => format_trailing_comments(doc, token),
                TrailingTrivia::BeforeLineBreak => {
                    format_trailing_comments_before_line_break(doc, token)
                }
                TrailingTrivia::BeforeSoftLine => doc_concat!(
                    doc,
                    [
                        format_trailing_comments_before_line_break(doc, token),
                        if trailing_comments_force_line(token) {
                            doc.hard_line()
                        } else {
                            doc.soft_line()
                        },
                    ]
                ),
                TrailingTrivia::BeforeSpaceIfComments => {
                    if token.trailing_comments().is_empty() {
                        Doc::nil()
                    } else {
                        doc_concat!(
                            doc,
                            [
                                format_trailing_comments_before_line_break(doc, token),
                                if trailing_comments_force_line(token) {
                                    doc.hard_line()
                                } else {
                                    doc.space()
                                },
                            ]
                        )
                    }
                }
                TrailingTrivia::RelocatedToEnclosingContext => Doc::nil(),
            },
        ]
    )
}
