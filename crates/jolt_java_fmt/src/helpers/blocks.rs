use jolt_fmt_ir::{Doc, concat, empty_line, hard_line, join};
use jolt_java_syntax::JavaSyntaxToken;

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_token_after_relocated_leading_comments,
    format_token_before_relocated_trailing_comments,
};
use crate::helpers::syntax_tokens::{InsertedSyntaxToken, inserted_syntax_token};

pub(crate) struct BodyItem<'source> {
    doc: Doc<'source>,
    starts_after_blank_line: bool,
}

impl<'source> BodyItem<'source> {
    pub(crate) fn new(doc: Doc<'source>, starts_after_blank_line: bool) -> Self {
        Self {
            doc,
            starts_after_blank_line,
        }
    }

    pub(crate) fn without_blank_line_before(self) -> Self {
        Self {
            starts_after_blank_line: false,
            ..self
        }
    }
}

pub(crate) fn inserted_braced_body(body: Option<Doc<'_>>) -> Doc<'_> {
    concat([
        inserted_syntax_token("{", InsertedSyntaxToken::BlockBrace),
        inserted_braced_body_tail(body),
    ])
}

pub(crate) fn source_braced_body<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    close: Option<&JavaSyntaxToken<'source>>,
    body: Option<Doc<'source>>,
) -> Doc<'source> {
    concat([
        format_source_open_brace(open),
        source_braced_body_tail(close, body),
    ])
}

pub(crate) fn inserted_braced_body_tail(body: Option<Doc<'_>>) -> Doc<'_> {
    concat([
        body.map_or_else(hard_line, |body| {
            concat([
                jolt_fmt_ir::indent(concat([hard_line(), body])),
                hard_line(),
            ])
        }),
        inserted_syntax_token("}", InsertedSyntaxToken::BlockBrace),
    ])
}

pub(crate) fn source_braced_body_tail<'source>(
    close: Option<&JavaSyntaxToken<'source>>,
    body: Option<Doc<'source>>,
) -> Doc<'source> {
    concat([
        body.map_or_else(hard_line, |body| {
            concat([
                jolt_fmt_ir::indent(concat([hard_line(), body])),
                hard_line(),
            ])
        }),
        format_source_close_brace(close),
    ])
}

pub(crate) fn empty_block<'source>() -> Doc<'source> {
    inserted_braced_body(None)
}

pub(crate) fn join_hard_lines<'source>(
    docs: impl IntoIterator<Item = Doc<'source>>,
) -> Doc<'source> {
    join(&hard_line(), docs)
}

pub(crate) fn join_empty_lines<'source>(
    docs: impl IntoIterator<Item = Doc<'source>>,
) -> Doc<'source> {
    join(&empty_line(), docs)
}

pub(crate) fn join_body_items(items: Vec<BodyItem<'_>>) -> Doc<'_> {
    let mut joined = Vec::new();
    for item in items {
        if !joined.is_empty() {
            joined.push(if item.starts_after_blank_line {
                jolt_fmt_ir::empty_line()
            } else {
                hard_line()
            });
        }
        joined.push(item.doc);
    }
    concat(joined)
}

fn format_source_open_brace<'source>(open: Option<&JavaSyntaxToken<'source>>) -> Doc<'source> {
    open.map_or_else(
        || inserted_syntax_token("{", InsertedSyntaxToken::MissingSource),
        |open| format_token_before_relocated_trailing_comments(open, LeadingTrivia::Preserve),
    )
}

fn format_source_close_brace<'source>(close: Option<&JavaSyntaxToken<'source>>) -> Doc<'source> {
    close.map_or_else(
        || inserted_syntax_token("}", InsertedSyntaxToken::MissingSource),
        |close| format_token_after_relocated_leading_comments(close, TrailingTrivia::Preserve),
    )
}
