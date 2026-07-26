use jolt_fmt_ir::{Doc, DocBuilder, LayoutDoc};
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
    layout: LayoutDoc<'source>,
    pub(crate) comma: Option<JavaSyntaxToken<'source>>,
}

impl<'source> CommaListItem<'source> {
    pub(crate) const fn visible(doc: Doc<'source>) -> Self {
        Self {
            layout: LayoutDoc::Visible(doc),
            comma: None,
        }
    }

    /// A recovery item that may claim source without occupying layout.
    pub(crate) const fn recovery(layout: LayoutDoc<'source>) -> Self {
        Self {
            layout,
            comma: None,
        }
    }

    pub(crate) const fn is_visible(&self) -> bool {
        self.layout.is_visible()
    }

    pub(crate) const fn doc(&self) -> Doc<'source> {
        self.layout.doc()
    }
}

/// Attaches a separator to the last visible item, or keeps it as its own item.
///
/// A separator never attaches to a claim-only recovery item: that item occupies
/// no layout, so a separator held there would never be emitted.
pub(crate) fn attach_comma_separator<'source>(
    doc: &mut DocBuilder<'source>,
    items: &mut Vec<CommaListItem<'source>>,
    separator: JavaSyntaxToken<'source>,
) {
    if let Some(item) = items.iter_mut().rev().find(|item| item.is_visible())
        && item.comma.is_none()
    {
        item.comma = Some(separator);
    } else {
        let separator = format_token(
            doc,
            &separator,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        );
        items.push(CommaListItem::visible(separator));
    }
}

pub(crate) fn comma_list<'source>(
    doc: &mut DocBuilder<'source>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
) -> Doc<'source> {
    comma_list_parts(doc, items).0
}

/// Formats comma-separated items, reporting whether the source ended with a
/// trailing separator.
///
/// Claim-only recovery items occupy no layout, so separators and breaks are
/// placed between *visible* items only. A trailing separator emits no break
/// after itself; the enclosing list decides how to lay out its close delimiter.
fn comma_list_parts<'source>(
    doc: &mut DocBuilder<'source>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
) -> (Doc<'source>, bool) {
    let items: Vec<_> = items.into_iter().collect();
    let visible_count = items.iter().filter(|item| item.is_visible()).count();
    let mut has_source_trailing_separator = false;
    let docs = doc.concat_list(|docs| {
        let mut visible_index = 0;
        for item in items {
            docs.push(item.doc());
            if !item.is_visible() {
                continue;
            }

            let is_last = visible_index + 1 == visible_count;
            if let Some(comma) = item.comma {
                has_source_trailing_separator |= is_last;
                let unforced_break = if is_last { Doc::nil() } else { docs.line() };
                let separator = format_separator_with_comments(docs, &comma, unforced_break);
                docs.push(separator);
            } else if !is_last {
                let line = docs.line();
                docs.push(line);
            }
            visible_index += 1;
        }
    });

    (docs, has_source_trailing_separator)
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
            JavaFormatListPart::Item(entry) => {
                items.push(CommaListItem::visible(format_entry(entry, doc)));
            }
            JavaFormatListPart::Separator(separator) => {
                attach_comma_separator(doc, &mut items, separator);
            }
            JavaFormatListPart::Recovery(malformed) => {
                items.push(CommaListItem::recovery(malformed));
            }
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
        return empty_delimited_list(doc, open, close, LeadingTrivia::Preserve);
    }

    let (items_doc, has_source_trailing_separator) =
        comma_list_with_trailing_separator(doc, items, trailing_comma);
    let should_break = has_delimiter_dangling_comments(open.source(), close.source())
        || has_source_trailing_separator;
    let open_spacing = format_braced_open_spacing(doc, open.source());
    let contents = doc_concat!(
        doc,
        [
            format_open_delimiter(doc, open, LeadingTrivia::Preserve),
            doc_indent!(doc, doc_concat!(doc, [open_spacing, items_doc])),
            doc.line_boundary(),
            format_close_delimiter(doc, close),
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
    delimited_comma_list_with_open_leading(doc, open, close, items, LeadingTrivia::Preserve)
}

pub(crate) fn delimited_comma_list_without_open_leading_comments<'source>(
    doc: &mut DocBuilder<'source>,
    open: JavaFormatDelimiter<'source>,
    close: JavaFormatDelimiter<'source>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
) -> Doc<'source> {
    delimited_comma_list_with_open_leading(
        doc,
        open,
        close,
        items,
        LeadingTrivia::SuppressAlreadyHandled,
    )
}

fn delimited_comma_list_with_open_leading<'source>(
    doc: &mut DocBuilder<'source>,
    open: JavaFormatDelimiter<'source>,
    close: JavaFormatDelimiter<'source>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
    open_leading: LeadingTrivia,
) -> Doc<'source> {
    let mut items = items.into_iter().peekable();
    if items.peek().is_none() {
        return empty_delimited_list(doc, open, close, open_leading);
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
    let open_doc = format_open_delimiter_before_items(doc, open, open_leading);
    let (items_doc, has_source_trailing_separator) = comma_list_parts(doc, items);
    let close_comments = format_close_leading_comments(doc, close.source());
    let indented = doc_indent!(doc, doc_concat!(doc, [open_doc, items_doc, close_comments]));
    let contents = doc_concat!(doc, [indented, format_close_with_spacing(doc, close)]);
    // A represented trailing separator is only valid Java in braced lists, but
    // it is preserved wherever it appears, so lay it out the same way there.
    let list = if has_source_trailing_separator {
        doc_force_group!(doc, contents)
    } else {
        doc_group!(doc, contents)
    };
    doc_concat!(doc, [list, trailing])
}

fn empty_delimited_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: JavaFormatDelimiter<'source>,
    close: JavaFormatDelimiter<'source>,
    open_leading: LeadingTrivia,
) -> Doc<'source> {
    if !has_delimiter_dangling_comments(open.source(), close.source()) {
        return doc_concat!(
            doc,
            [
                format_open_delimiter(doc, open, open_leading),
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
                format_open_delimiter(doc, open, open_leading),
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
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_open_delimiter_with_trailing(
        doc,
        open,
        leading,
        TrailingTrivia::RelocatedToEnclosingContext,
    )
}

fn format_open_delimiter_before_items<'source>(
    doc: &mut DocBuilder<'source>,
    open: JavaFormatDelimiter<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    match open {
        JavaFormatDelimiter::Source(open) => {
            format_source_open_delimiter(doc, &open, leading, TrailingTrivia::BeforeSoftLine)
        }
        JavaFormatDelimiter::Recovery(recovery) => {
            doc_concat!(doc, [recovery.doc(), doc.soft_line()])
        }
    }
}

fn format_open_delimiter_with_trailing<'source>(
    doc: &mut DocBuilder<'source>,
    open: JavaFormatDelimiter<'source>,
    leading: LeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    match open {
        JavaFormatDelimiter::Source(open) => {
            format_source_open_delimiter(doc, &open, leading, trailing)
        }
        JavaFormatDelimiter::Recovery(recovery) => recovery.doc(),
    }
}

fn format_source_open_delimiter<'source>(
    doc: &mut DocBuilder<'source>,
    open: &JavaSyntaxToken<'source>,
    leading: LeadingTrivia,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    match leading {
        LeadingTrivia::Preserve => format_token_with_inline_leading_comments(
            doc,
            open,
            InlineLeadingTrivia::BeforeToken,
            trailing,
        ),
        LeadingTrivia::SuppressAlreadyHandled => {
            format_token_after_relocated_leading_comments(doc, open, trailing)
        }
    }
}

fn comma_list_with_trailing_separator<'source>(
    doc: &mut DocBuilder<'source>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
    trailing_comma: Option<SynthesisClaim<'source>>,
) -> (Doc<'source>, bool) {
    let items: Vec<_> = items.into_iter().collect();
    let visible_count = items.iter().filter(|item| item.is_visible()).count();
    let mut has_source_trailing_separator = false;
    let mut trailing_comma = trailing_comma;
    let docs = doc.concat_list(|docs| {
        let mut visible_index = 0;
        for item in items {
            docs.push(item.doc());
            if !item.is_visible() {
                continue;
            }

            let is_last = visible_index + 1 == visible_count;
            visible_index += 1;
            has_source_trailing_separator |= is_last && item.comma.is_some();
            if let Some(comma) = item.comma {
                let separator = trailing_comma_separator(docs, &comma, is_last);
                docs.push(separator);
            } else if !is_last {
                let line = docs.line();
                docs.push(line);
            } else {
                let trailing_comma = trailing_comma.take().map_or_else(Doc::nil, |claim| {
                    // Intentional synthesized token: trailing comma policy adds a
                    // comma only when the list breaks across lines.
                    let breaks = docs.synthesized_source(claim);
                    let flat = Doc::nil();
                    docs.if_break(breaks, flat)
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
                if force_line {
                    doc.hard_line()
                } else if has_trailing_comments {
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
                JavaFormatDelimiter::Recovery(recovery) => recovery.doc(),
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
        JavaFormatDelimiter::Recovery(recovery) => recovery.doc(),
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
        JavaFormatDelimiter::Recovery(recovery) => recovery.doc(),
    }
}
