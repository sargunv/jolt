use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{JavaSyntaxToken, RecoveredSeparatedListEntry};

use crate::helpers::comments::{
    InlineLeadingTrivia, LeadingTrivia, TrailingTrivia, delimiter_dangling_comments,
    format_dangling_comments, format_leading_comments, format_separator_with_comments,
    format_token, format_token_after_relocated_leading_comments, format_token_sequence,
    format_token_with_inline_leading_comments, has_delimiter_dangling_comments,
    trailing_comments_force_line,
};
use crate::helpers::syntax_tokens::{FormatterInsertedToken, inserted_syntax_token};

pub(crate) struct CommaListItem<'source> {
    pub(crate) doc: Doc<'source>,
    pub(crate) comma: Option<JavaSyntaxToken<'source>>,
}

pub(crate) fn comma_list<'source>(
    doc: &mut DocBuilder<'source>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
) -> Doc<'source> {
    let mut items = items.into_iter().peekable();
    let mut docs = doc.list();

    while let Some(item) = items.next() {
        docs.push(item.doc, doc);
        if let Some(comma) = item.comma {
            let line = doc.line();
            let separator = format_separator_with_comments(doc, &comma, line);
            docs.push(separator, doc);
        } else if items.peek().is_some() {
            let line = doc.line();
            docs.push(line, doc);
        }
    }

    docs.finish(doc)
}

pub(crate) fn recovered_comma_list_items<'source, Entry>(
    doc: &mut DocBuilder<'source>,
    entries: impl IntoIterator<Item = RecoveredSeparatedListEntry<'source, Entry>>,
    mut format_entry: impl FnMut(Entry, &mut DocBuilder<'source>) -> CommaListItem<'source>,
) -> Vec<CommaListItem<'source>> {
    entries
        .into_iter()
        .map(|entry| match entry {
            RecoveredSeparatedListEntry::Entry(entry) => format_entry(entry, doc),
            RecoveredSeparatedListEntry::Token(token) => CommaListItem {
                doc: format_token(
                    doc,
                    &token,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::Preserve,
                ),
                comma: None,
            },
            RecoveredSeparatedListEntry::Error(error) => CommaListItem {
                doc: format_token_sequence(doc, error.token_iter(), LeadingTrivia::Preserve),
                comma: None,
            },
            RecoveredSeparatedListEntry::Node(node) => CommaListItem {
                doc: format_token_sequence(doc, node.token_iter(), LeadingTrivia::Preserve),
                comma: None,
            },
        })
        .collect()
}

pub(crate) fn parenthesized_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&JavaSyntaxToken<'source>>,
    close: Option<&JavaSyntaxToken<'source>>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
) -> Doc<'source> {
    delimited_comma_list(doc, open, close, items)
}

pub(crate) fn angle_bracket_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&JavaSyntaxToken<'source>>,
    close: Option<&JavaSyntaxToken<'source>>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
) -> Doc<'source> {
    delimited_comma_list(doc, open, close, items)
}

pub(crate) fn braced_comma_list_with_trailing_separator<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&JavaSyntaxToken<'source>>,
    close: Option<&JavaSyntaxToken<'source>>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
) -> Doc<'source> {
    let mut items = items.into_iter().peekable();
    if items.peek().is_none() {
        return empty_delimited_list(doc, open, close);
    }

    let (items_doc, has_source_trailing_separator) = comma_list_with_trailing_separator(doc, items);
    let should_break =
        has_delimiter_dangling_comments(open, close) || has_source_trailing_separator;
    let contents = doc_concat!(
        doc,
        [
            format_open_delimiter(doc, open),
            doc_indent!(
                doc,
                doc_concat!(doc, [format_braced_open_spacing(doc, open), items_doc])
            ),
            format_braced_close_with_spacing(doc, close),
        ]
    );

    if should_break {
        doc_force_group!(doc, contents)
    } else {
        doc_group!(doc, contents)
    }
}

fn delimited_comma_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&JavaSyntaxToken<'source>>,
    close: Option<&JavaSyntaxToken<'source>>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
) -> Doc<'source> {
    let mut items = items.into_iter().peekable();
    if items.peek().is_none() {
        return empty_delimited_list(doc, open, close);
    }

    doc_group!(
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
                            format_close_leading_comments(doc, close),
                        ]
                    )
                ),
                format_close_with_spacing(doc, close),
            ]
        )
    )
}

fn empty_delimited_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&JavaSyntaxToken<'source>>,
    close: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    if !has_delimiter_dangling_comments(open, close) {
        return doc_concat!(
            doc,
            [
                format_open_delimiter(doc, open),
                format_close_delimiter(doc, close),
            ]
        );
    }

    doc_force_group!(
        doc,
        doc_concat!(
            doc,
            [
                format_open_delimiter(doc, open),
                doc_indent!(
                    doc,
                    doc_concat!(
                        doc,
                        [
                            doc.hard_line(),
                            format_delimiter_dangling_comments(doc, open, close),
                        ]
                    )
                ),
                doc.hard_line(),
                format_close_delimiter_without_leading(doc, close),
            ]
        )
    )
}

fn format_open_delimiter<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    format_open_delimiter_with_trailing(doc, open, TrailingTrivia::RelocatedToEnclosingContext)
}

fn format_open_delimiter_before_items<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    match open {
        Some(open) => format_token_with_inline_leading_comments(
            doc,
            open,
            InlineLeadingTrivia::BeforeToken,
            TrailingTrivia::BeforeSoftLine,
        ),
        None => doc.soft_line(),
    }
}

fn format_open_delimiter_with_trailing<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&JavaSyntaxToken<'source>>,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    match open {
        Some(open) => format_token_with_inline_leading_comments(
            doc,
            open,
            InlineLeadingTrivia::BeforeToken,
            trailing,
        ),
        None => Doc::nil(),
    }
}

fn comma_list_with_trailing_separator<'source>(
    doc: &mut DocBuilder<'source>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
) -> (Doc<'source>, bool) {
    let mut items = items.into_iter().peekable();
    let mut docs = doc.list();
    let mut has_source_trailing_separator = false;

    while let Some(item) = items.next() {
        let is_last = items.peek().is_none();
        has_source_trailing_separator |= is_last && item.comma.is_some();
        docs.push(item.doc, doc);
        if let Some(comma) = item.comma {
            docs.push(trailing_comma_separator(doc, &comma, is_last), doc);
        } else if !is_last {
            docs.push(doc.line(), doc);
        } else {
            let trailing_comma = doc_if_break!(
                doc,
                // Intentional synthesized token: trailing comma policy adds a
                // comma only when the list breaks across lines.
                inserted_syntax_token(doc, ",", FormatterInsertedToken::TrailingComma),
                Doc::nil(),
            );
            docs.push(trailing_comma, doc);
        }
    }

    (docs.finish(doc), has_source_trailing_separator)
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
    close: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    doc_concat!(doc, [doc.line(), format_close_delimiter(doc, close)])
}

fn format_close_with_spacing<'source>(
    doc: &mut DocBuilder<'source>,
    close: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    let close_has_leading_comments =
        close.is_some_and(|token| !token.leading_comments().is_empty());

    doc_concat!(
        doc,
        [
            if close_has_leading_comments {
                doc.hard_line()
            } else {
                doc.soft_line()
            },
            format_close_delimiter_without_leading(doc, close),
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
    close: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    let close_has_leading_comments =
        close.is_some_and(|token| !token.leading_comments().is_empty());
    close.map_or_else(Doc::nil, |close| {
        doc_concat!(
            doc,
            [
                if close_has_leading_comments {
                    format_leading_comments(doc, close)
                } else {
                    Doc::nil()
                },
                format_token_after_relocated_leading_comments(doc, close, TrailingTrivia::Preserve),
            ]
        )
    })
}

fn format_close_delimiter_without_leading<'source>(
    doc: &mut DocBuilder<'source>,
    close: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    close.map_or_else(Doc::nil, |close| {
        format_token_after_relocated_leading_comments(doc, close, TrailingTrivia::Preserve)
    })
}

fn format_delimiter_dangling_comments<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&JavaSyntaxToken<'source>>,
    close: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    format_dangling_comments(doc, delimiter_dangling_comments(open, close))
}
