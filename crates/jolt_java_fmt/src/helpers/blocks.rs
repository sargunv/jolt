use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::JavaSyntaxToken;

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_token_after_relocated_leading_comments,
    format_token_before_relocated_trailing_comments,
};
use crate::helpers::syntax_tokens::{FormatterInsertedToken, inserted_syntax_token};

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

pub(crate) fn inserted_braced_body<'source>(
    doc: &mut DocBuilder<'source>,
    body: Option<Doc<'source>>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            // Intentional synthesized token: normalized braced bodies add braces
            // around source statements that did not have a block.
            inserted_syntax_token(doc, "{", FormatterInsertedToken::BlockBrace),
            inserted_braced_body_tail(doc, body),
        ]
    )
}

pub(crate) fn source_braced_body<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&JavaSyntaxToken<'source>>,
    close: Option<&JavaSyntaxToken<'source>>,
    body: Option<Doc<'source>>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            format_source_open_brace(doc, open),
            source_braced_body_tail(doc, close, body),
        ]
    )
}

fn inserted_braced_body_tail<'source>(
    doc: &mut DocBuilder<'source>,
    body: Option<Doc<'source>>,
) -> Doc<'source> {
    let body = match body {
        Some(body) => {
            let hard_line_before = doc.hard_line();
            let body = doc_concat!(doc, [hard_line_before, body]);
            let body = doc_indent!(doc, body);
            let hard_line_after = doc.hard_line();
            doc_concat!(doc, [body, hard_line_after])
        }
        None => doc.hard_line(),
    };
    let close = inserted_syntax_token(doc, "}", FormatterInsertedToken::BlockBrace);
    doc_concat!(doc, [body, close])
}

fn source_braced_body_tail<'source>(
    doc: &mut DocBuilder<'source>,
    close: Option<&JavaSyntaxToken<'source>>,
    body: Option<Doc<'source>>,
) -> Doc<'source> {
    let body = match body {
        Some(body) => {
            let hard_line_before = doc.hard_line();
            let body = doc_concat!(doc, [hard_line_before, body]);
            let body = doc_indent!(doc, body);
            let hard_line_after = doc.hard_line();
            doc_concat!(doc, [body, hard_line_after])
        }
        None => doc.hard_line(),
    };
    let close = format_source_close_brace(doc, close);
    doc_concat!(doc, [body, close])
}

pub(crate) fn empty_block<'source>(doc: &mut DocBuilder<'source>) -> Doc<'source> {
    inserted_braced_body(doc, None)
}

pub(crate) fn join_empty_lines<'source>(
    doc: &mut DocBuilder<'source>,
    docs: impl IntoIterator<Item = Doc<'source>>,
) -> Doc<'source> {
    doc_join!(doc, doc.empty_line(), docs)
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

fn format_source_open_brace<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    open.map_or_else(Doc::nil, |open| {
        format_token_before_relocated_trailing_comments(doc, open, LeadingTrivia::Preserve)
    })
}

fn format_source_close_brace<'source>(
    doc: &mut DocBuilder<'source>,
    close: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    close.map_or_else(Doc::nil, |close| {
        format_token_after_relocated_leading_comments(doc, close, TrailingTrivia::Preserve)
    })
}
