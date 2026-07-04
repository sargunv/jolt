use jolt_fmt_ir::space;
use jolt_fmt_ir::{Doc, text};
use jolt_java_syntax::JavaSyntaxToken;

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_leading_comments, format_trailing_comments,
    format_trailing_comments_before_line_break, trailing_comments_force_line,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InsertedSyntaxToken {
    BlockBrace,
    MissingSource,
    PrecedenceParenthesis,
    TrailingComma,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NormalizedSyntaxToken {
    EnumSeparator,
    NameSeparator,
}

pub(crate) fn inserted_syntax_token<'source>(
    spelling: &'static str,
    _reason: InsertedSyntaxToken,
) -> Doc<'source> {
    text(spelling)
}

pub(crate) fn normalized_syntax_token<'source>(
    spelling: &'static str,
    _reason: NormalizedSyntaxToken,
) -> Doc<'source> {
    text(spelling)
}

pub(crate) fn format_token_with_normalized_text<'source>(
    token: &JavaSyntaxToken<'source>,
    spelling: &'static str,
    reason: NormalizedSyntaxToken,
    leading: LeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    jolt_fmt_ir::concat([
        match leading {
            LeadingTrivia::Preserve => format_leading_comments(token),
            LeadingTrivia::SuppressAlreadyHandled => jolt_fmt_ir::nil(),
        },
        normalized_syntax_token(spelling, reason),
        match trailing {
            TrailingTrivia::Preserve => format_trailing_comments(token),
            TrailingTrivia::BeforeLineBreak => format_trailing_comments_before_line_break(token),
            TrailingTrivia::BeforeSoftLine => jolt_fmt_ir::concat([
                format_trailing_comments_before_line_break(token),
                if trailing_comments_force_line(token) {
                    jolt_fmt_ir::hard_line()
                } else {
                    jolt_fmt_ir::soft_line()
                },
            ]),
            TrailingTrivia::BeforeSpaceIfComments => {
                if token.trailing_comments().is_empty() {
                    jolt_fmt_ir::nil()
                } else {
                    jolt_fmt_ir::concat([
                        format_trailing_comments_before_line_break(token),
                        if trailing_comments_force_line(token) {
                            jolt_fmt_ir::hard_line()
                        } else {
                            space()
                        },
                    ])
                }
            }
            TrailingTrivia::RelocatedToEnclosingContext => jolt_fmt_ir::nil(),
        },
    ])
}
