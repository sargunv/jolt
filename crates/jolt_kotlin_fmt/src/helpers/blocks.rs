use jolt_fmt_ir::{Doc, DocBuilder};
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
    doc: &mut DocBuilder<'source>,
    docs: impl IntoIterator<Item = Doc<'source>>,
) -> Doc<'source> {
    let separator = doc.hard_line();
    doc.join(separator, docs)
}

pub(crate) fn join_body_items<'source>(
    doc: &mut DocBuilder<'source>,
    items: Vec<BodyItem<'source>>,
) -> Doc<'source> {
    let mut joined = doc.list();
    for item in items {
        if !joined.is_empty() {
            let separator = if item.starts_after_blank_line {
                doc.empty_line()
            } else {
                doc.hard_line()
            };
            joined.push(separator, doc);
        }
        joined.push(item.doc, doc);
    }
    joined.finish(doc)
}

pub(crate) fn source_braced_body<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
    body: Option<Doc<'source>>,
) -> Doc<'source> {
    let open = format_source_open_brace(doc, open);
    let tail = source_braced_body_tail(doc, close, body);
    doc.concat([open, tail])
}

pub(crate) fn empty_source_braced_body<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    let open = format_source_open_brace(doc, open);
    let close = format_source_close_brace(doc, close);
    doc.concat([open, close])
}

fn source_braced_body_tail<'source>(
    doc: &mut DocBuilder<'source>,
    close: Option<&KotlinSyntaxToken<'source>>,
    body: Option<Doc<'source>>,
) -> Doc<'source> {
    if let Some(body) = body {
        let line = doc.hard_line();
        let body = doc.concat([line, body]);
        let body = doc.indent(body);
        let line = doc.hard_line();
        let body = doc.concat([body, line]);
        let close = format_source_close_brace_with_leading(
            doc,
            close,
            LeadingTrivia::SuppressAlreadyHandled,
        );
        doc.concat([body, close])
    } else {
        let line = doc.hard_line();
        let close = format_source_close_brace(doc, close);
        doc.concat([line, close])
    }
}

fn format_source_open_brace<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    if let Some(open) = open {
        format_token(
            doc,
            open,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    } else {
        doc.nil()
    }
}

fn format_source_close_brace<'source>(
    doc: &mut DocBuilder<'source>,
    close: Option<&KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    format_source_close_brace_with_leading(doc, close, LeadingTrivia::Preserve)
}

fn format_source_close_brace_with_leading<'source>(
    doc: &mut DocBuilder<'source>,
    close: Option<&KotlinSyntaxToken<'source>>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    if let Some(close) = close {
        format_token(doc, close, leading, TrailingTrivia::Preserve)
    } else {
        doc.nil()
    }
}
