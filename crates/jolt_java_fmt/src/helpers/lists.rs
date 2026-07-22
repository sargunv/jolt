use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{JavaSyntaxListPart, JavaSyntaxToken, SynthesisClaim};

use crate::helpers::comments::{
    InlineLeadingTrivia, LeadingTrivia, TrailingTrivia, delimiter_dangling_comments,
    format_dangling_comments, format_leading_comments, format_separator_with_comments,
    format_token, format_token_after_relocated_leading_comments,
    format_token_with_inline_leading_comments, format_trailing_comments_before_line_break,
    has_delimiter_dangling_comments, trailing_comments_force_line,
};
use crate::helpers::recovery::{JavaFormatDelimiter, JavaFormatListPart, resolve_list_part};

pub(crate) struct CommaListItem<'source> {
    pub(crate) doc: Doc<'source>,
    pub(crate) comma: Option<JavaSyntaxToken<'source>>,
}

pub(crate) fn comma_list<'source>(
    doc: &mut DocBuilder<'source>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
) -> Doc<'source> {
    let mut items = items.into_iter().peekable();
    doc.concat_list(|docs| {
        while let Some(item) = items.next() {
            docs.push(item.doc);
            if let Some(comma) = item.comma {
                let line = docs.line();
                let separator = format_separator_with_comments(docs, &comma, line);
                docs.push(separator);
            } else if items.peek().is_some() {
                let line = docs.line();
                docs.push(line);
            }
        }
    })
}

pub(crate) fn syntax_comma_list_items<'source, Entry>(
    doc: &mut DocBuilder<'source>,
    entries: impl IntoIterator<Item = JavaSyntaxListPart<'source, Entry>>,
    mut format_entry: impl FnMut(Entry, &mut DocBuilder<'source>) -> Doc<'source>,
) -> Vec<CommaListItem<'source>> {
    let entries = entries.into_iter();
    let (lower, _) = entries.size_hint();
    // The represented list is already a bounded physical syntax node. Reserve
    // from that exact traversal instead of geometrically reallocating a second
    // recovery staging buffer while attaching separators to their items.
    let mut items = Vec::with_capacity(lower);
    for entry in entries {
        match resolve_list_part(entry, doc) {
            JavaFormatListPart::Item(entry) => items.push(CommaListItem {
                doc: format_entry(entry, doc),
                comma: None,
            }),
            JavaFormatListPart::Separator(separator) => {
                if let Some(item) = items.last_mut()
                    && item.comma.is_none()
                {
                    item.comma = Some(separator);
                } else {
                    items.push(CommaListItem {
                        doc: format_token(
                            doc,
                            &separator,
                            LeadingTrivia::Preserve,
                            TrailingTrivia::Preserve,
                        ),
                        comma: None,
                    });
                }
            }
            JavaFormatListPart::Malformed(malformed) => items.push(CommaListItem {
                doc: malformed,
                comma: None,
            }),
        }
    }
    items
}

pub(crate) fn braced_comma_list_with_trailing_separator<'source>(
    doc: &mut DocBuilder<'source>,
    open: JavaFormatDelimiter<'source>,
    close: JavaFormatDelimiter<'source>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
    trailing_comma: Option<SynthesisClaim<'source>>,
) -> Doc<'source> {
    let mut items = items.into_iter().peekable();
    if items.peek().is_none() {
        return empty_delimited_list(doc, open, close);
    }

    let (items_doc, has_source_trailing_separator) =
        comma_list_with_trailing_separator(doc, items, trailing_comma);
    let should_break = has_delimiter_dangling_comments(open.source(), close.source())
        || has_source_trailing_separator;
    let open_spacing = format_braced_open_spacing(doc, open.source());
    let contents = doc_concat!(
        doc,
        [
            format_open_delimiter(doc, open),
            doc_indent!(doc, doc_concat!(doc, [open_spacing, items_doc])),
            format_braced_close_with_spacing(doc, close),
        ]
    );

    if should_break {
        doc_force_group!(doc, contents)
    } else {
        doc_group!(doc, contents)
    }
}

pub(crate) fn delimited_comma_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: JavaFormatDelimiter<'source>,
    close: JavaFormatDelimiter<'source>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
) -> Doc<'source> {
    let mut items = items.into_iter().peekable();
    if items.peek().is_none() {
        return empty_delimited_list(doc, open, close);
    }

    let trailing = close.source().map_or_else(Doc::nil, |close| {
        if close.trailing_comments().is_empty() {
            Doc::nil()
        } else {
            doc_concat!(
                doc,
                [
                    format_trailing_comments_before_line_break(doc, close),
                    if trailing_comments_force_line(close) {
                        doc.hard_line()
                    } else {
                        Doc::nil()
                    },
                ]
            )
        }
    });
    let list = doc_group!(
        doc,
        doc_concat!(
            doc,
            [
                doc_indent!(
                    doc,
                    doc_concat!(
                        doc,
                        [
                            format_open_delimiter_before_items(doc, open),
                            comma_list(doc, items),
                            format_close_leading_comments(doc, close.source()),
                        ]
                    )
                ),
                format_close_with_spacing(doc, close),
            ]
        )
    );
    doc_concat!(doc, [list, trailing])
}

fn empty_delimited_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: JavaFormatDelimiter<'source>,
    close: JavaFormatDelimiter<'source>,
) -> Doc<'source> {
    if !has_delimiter_dangling_comments(open.source(), close.source()) {
        return doc_concat!(
            doc,
            [
                format_open_delimiter(doc, open),
                format_close_delimiter(doc, close),
            ]
        );
    }

    let dangling = format_dangling_comments(
        doc,
        delimiter_dangling_comments(open.source(), close.source()),
    );

    doc_force_group!(
        doc,
        doc_concat!(
            doc,
            [
                format_open_delimiter(doc, open),
                doc_indent!(doc, doc_concat!(doc, [doc.hard_line(), dangling,])),
                doc.hard_line(),
                format_close_delimiter_without_leading(doc, close),
            ]
        )
    )
}

fn format_open_delimiter<'source>(
    doc: &mut DocBuilder<'source>,
    open: JavaFormatDelimiter<'source>,
) -> Doc<'source> {
    format_open_delimiter_with_trailing(doc, open, TrailingTrivia::RelocatedToEnclosingContext)
}

fn format_open_delimiter_before_items<'source>(
    doc: &mut DocBuilder<'source>,
    open: JavaFormatDelimiter<'source>,
) -> Doc<'source> {
    match open {
        JavaFormatDelimiter::Source(open) => format_token_with_inline_leading_comments(
            doc,
            &open,
            InlineLeadingTrivia::BeforeToken,
            TrailingTrivia::BeforeSoftLine,
        ),
        JavaFormatDelimiter::Recovery(recovery) => {
            doc_concat!(doc, [recovery, doc.soft_line()])
        }
    }
}

fn format_open_delimiter_with_trailing<'source>(
    doc: &mut DocBuilder<'source>,
    open: JavaFormatDelimiter<'source>,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    match open {
        JavaFormatDelimiter::Source(open) => format_token_with_inline_leading_comments(
            doc,
            &open,
            InlineLeadingTrivia::BeforeToken,
            trailing,
        ),
        JavaFormatDelimiter::Recovery(recovery) => recovery,
    }
}

fn comma_list_with_trailing_separator<'source>(
    doc: &mut DocBuilder<'source>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
    trailing_comma: Option<SynthesisClaim<'source>>,
) -> (Doc<'source>, bool) {
    let mut items = items.into_iter().peekable();
    let mut has_source_trailing_separator = false;
    let mut trailing_comma = trailing_comma;
    let docs = doc.concat_list(|docs| {
        while let Some(item) = items.next() {
            let is_last = items.peek().is_none();
            has_source_trailing_separator |= is_last && item.comma.is_some();
            docs.push(item.doc);
            if let Some(comma) = item.comma {
                let separator = trailing_comma_separator(docs, &comma, is_last);
                docs.push(separator);
            } else if !is_last {
                let line = docs.line();
                docs.push(line);
            } else {
                let trailing_comma = trailing_comma.take().map_or_else(Doc::nil, |claim| {
                    doc_if_break!(
                        docs,
                        // Intentional synthesized token: trailing comma policy adds a
                        // comma only when the list breaks across lines.
                        docs.synthesized_source(claim),
                        Doc::nil(),
                    )
                });
                docs.push(trailing_comma);
            }
        }
    });

    (docs, has_source_trailing_separator)
}

fn trailing_comma_separator<'source>(
    doc: &mut DocBuilder<'source>,
    comma: &JavaSyntaxToken<'source>,
    is_last: bool,
) -> Doc<'source> {
    let trailing_comments = comma.trailing_comments();
    let has_trailing_comments = !trailing_comments.is_empty();
    let force_line = trailing_comments_force_line(comma);

    doc_concat!(
        doc,
        [
            format_token(
                doc,
                comma,
                LeadingTrivia::Preserve,
                TrailingTrivia::BeforeLineBreak,
            ),
            if is_last {
                if has_trailing_comments && !force_line {
                    doc.space()
                } else {
                    Doc::nil()
                }
            } else if force_line {
                doc.hard_line()
            } else if has_trailing_comments {
                doc.space()
            } else {
                doc.line()
            },
        ]
    )
}

fn format_braced_open_spacing<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    let Some(open) = open else {
        return doc.soft_line();
    };

    let comments = open.trailing_comments();
    if comments.is_empty() {
        return doc.line();
    }

    doc_concat!(
        doc,
        [
            doc.hard_line(),
            format_dangling_comments(doc, comments),
            doc.hard_line(),
        ]
    )
}

fn format_braced_close_with_spacing<'source>(
    doc: &mut DocBuilder<'source>,
    close: JavaFormatDelimiter<'source>,
) -> Doc<'source> {
    doc_concat!(doc, [doc.line(), format_close_delimiter(doc, close)])
}

fn format_close_with_spacing<'source>(
    doc: &mut DocBuilder<'source>,
    close: JavaFormatDelimiter<'source>,
) -> Doc<'source> {
    let close_has_leading_comments = close
        .source()
        .is_some_and(|token| !token.leading_comments().is_empty());

    doc_concat!(
        doc,
        [
            if close_has_leading_comments {
                doc.hard_line()
            } else {
                doc.soft_line()
            },
            match close {
                JavaFormatDelimiter::Source(close) => {
                    format_token_after_relocated_leading_comments(
                        doc,
                        &close,
                        TrailingTrivia::RelocatedToEnclosingContext,
                    )
                }
                JavaFormatDelimiter::Recovery(recovery) => recovery,
            },
        ]
    )
}

fn format_close_leading_comments<'source>(
    doc: &mut DocBuilder<'source>,
    close: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    close.map_or_else(Doc::nil, |close| {
        if close.leading_comments().is_empty() {
            Doc::nil()
        } else {
            doc_concat!(
                doc,
                [
                    doc.hard_line(),
                    format_dangling_comments(doc, close.leading_comments()),
                ]
            )
        }
    })
}

fn format_close_delimiter<'source>(
    doc: &mut DocBuilder<'source>,
    close: JavaFormatDelimiter<'source>,
) -> Doc<'source> {
    let close_has_leading_comments = close
        .source()
        .is_some_and(|token| !token.leading_comments().is_empty());
    match close {
        JavaFormatDelimiter::Source(close) => doc_concat!(
            doc,
            [
                if close_has_leading_comments {
                    format_leading_comments(doc, &close)
                } else {
                    Doc::nil()
                },
                format_token_after_relocated_leading_comments(
                    doc,
                    &close,
                    TrailingTrivia::Preserve
                ),
            ]
        ),
        JavaFormatDelimiter::Recovery(recovery) => recovery,
    }
}

fn format_close_delimiter_without_leading<'source>(
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
