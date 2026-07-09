use jolt_fmt_ir::{Doc, concat, empty_line, hard_line, indent, join};
use jolt_kotlin_syntax::KotlinSyntaxToken;

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token};

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
    let mut joined = Vec::with_capacity(items.len().saturating_mul(2).saturating_sub(1));
    for item in items {
        if !joined.is_empty() {
            joined.push(if item.starts_after_blank_line {
                empty_line()
            } else {
                hard_line()
            });
        }
        joined.push(item.doc);
    }
    concat(joined)
}

pub(crate) fn source_braced_body<'source>(
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
    body: Option<Doc<'source>>,
) -> Doc<'source> {
    concat([
        format_source_open_brace(open),
        source_braced_body_tail(close, body),
    ])
}

pub(crate) fn empty_source_braced_body<'source>(
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    concat([
        format_source_open_brace(open),
        format_source_close_brace(close),
    ])
}

fn source_braced_body_tail<'source>(
    close: Option<&KotlinSyntaxToken<'source>>,
    body: Option<Doc<'source>>,
) -> Doc<'source> {
    match body {
        Some(body) => concat([
            concat([indent(concat([hard_line(), body])), hard_line()]),
            format_source_close_brace_with_leading(close, LeadingTrivia::SuppressAlreadyHandled),
        ]),
        None => concat([hard_line(), format_source_close_brace(close)]),
    }
}

fn format_source_open_brace<'source>(open: Option<&KotlinSyntaxToken<'source>>) -> Doc<'source> {
    open.map_or_else(jolt_fmt_ir::nil, |open| {
        format_token(
            open,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    })
}

fn format_source_close_brace<'source>(close: Option<&KotlinSyntaxToken<'source>>) -> Doc<'source> {
    format_source_close_brace_with_leading(close, LeadingTrivia::Preserve)
}

fn format_source_close_brace_with_leading<'source>(
    close: Option<&KotlinSyntaxToken<'source>>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    close.map_or_else(jolt_fmt_ir::nil, |close| {
        format_token(close, leading, TrailingTrivia::Preserve)
    })
}
