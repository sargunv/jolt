use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::JavaDelimiterSynthesis;

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_token_after_relocated_leading_comments,
    format_token_before_relocated_trailing_comments,
};
use crate::helpers::recovery::JavaFormatDelimiter;
use crate::helpers::syntax_tokens::inserted_syntax_token;

#[derive(Clone, Copy)]
pub(crate) struct BodyContent<'source> {
    pub(crate) doc: Doc<'source>,
    pub(crate) present: bool,
    pub(crate) visible: bool,
}

impl<'source> BodyContent<'source> {
    pub(crate) fn new(doc: Doc<'source>, present: bool, visible: bool) -> Self {
        Self {
            doc,
            present,
            visible,
        }
    }
}

impl<'source> From<Option<Doc<'source>>> for BodyContent<'source> {
    fn from(doc: Option<Doc<'source>>) -> Self {
        match doc {
            Some(doc) => Self::new(doc, true, true),
            None => Self::new(Doc::nil(), false, false),
        }
    }
}

pub(crate) struct BodyItem<'source> {
    doc: Doc<'source>,
    starts_after_blank_line: bool,
    pub(crate) visible: bool,
}

impl<'source> BodyItem<'source> {
    pub(crate) fn new(doc: Doc<'source>, starts_after_blank_line: bool) -> Self {
        Self {
            doc,
            starts_after_blank_line,
            visible: true,
        }
    }

    pub(crate) fn invisible(doc: Doc<'source>) -> Self {
        Self {
            doc,
            starts_after_blank_line: false,
            visible: false,
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
    claims: JavaDelimiterSynthesis<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            // Intentional synthesized token: normalized braced bodies add braces
            // around source statements that did not have a block.
            inserted_syntax_token(doc, claims.open),
            inserted_braced_body_tail(doc, body, claims.close),
        ]
    )
}

pub(crate) fn source_braced_body<'source>(
    doc: &mut DocBuilder<'source>,
    open: JavaFormatDelimiter<'source>,
    close: JavaFormatDelimiter<'source>,
    body: impl Into<BodyContent<'source>>,
) -> Doc<'source> {
    let body = body.into();
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
    close: jolt_fmt_ir::SynthesisClaim<'source>,
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
    let close = inserted_syntax_token(doc, close);
    doc_concat!(doc, [body, close])
}

fn source_braced_body_tail<'source>(
    doc: &mut DocBuilder<'source>,
    close: JavaFormatDelimiter<'source>,
    body: BodyContent<'source>,
) -> Doc<'source> {
    let layout = if body.visible {
        let hard_line_before = doc.hard_line();
        let contents = doc_concat!(doc, [hard_line_before, body.doc]);
        let contents = doc_indent!(doc, contents);
        let hard_line_after = doc.hard_line();
        doc_concat!(doc, [contents, hard_line_after])
    } else {
        doc_concat!(doc, [body.doc, doc.hard_line()])
    };
    let close = format_source_close_brace(doc, close);
    doc_concat!(doc, [layout, close])
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
    doc.concat_list(|joined| {
        let mut saw_visible = false;
        for item in items {
            if item.visible && saw_visible {
                let separator = if item.starts_after_blank_line {
                    joined.empty_line()
                } else {
                    joined.hard_line()
                };
                joined.push(separator);
            }
            joined.push(item.doc);
            saw_visible |= item.visible;
        }
    })
}

fn format_source_open_brace<'source>(
    doc: &mut DocBuilder<'source>,
    open: JavaFormatDelimiter<'source>,
) -> Doc<'source> {
    match open {
        JavaFormatDelimiter::Source(open) => {
            format_token_before_relocated_trailing_comments(doc, &open, LeadingTrivia::Preserve)
        }
        JavaFormatDelimiter::Recovery(recovery) => recovery,
    }
}

fn format_source_close_brace<'source>(
    doc: &mut DocBuilder<'source>,
    close: JavaFormatDelimiter<'source>,
) -> Doc<'source> {
    match close {
        JavaFormatDelimiter::Source(close) => {
            format_token_after_relocated_leading_comments(doc, &close, TrailingTrivia::Preserve)
        }
        JavaFormatDelimiter::Recovery(recovery) => recovery,
    }
}
