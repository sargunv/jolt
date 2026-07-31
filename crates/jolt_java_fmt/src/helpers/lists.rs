use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{JavaSyntaxListPart, JavaSyntaxToken, JavaSyntaxView, SynthesisClaim};

use crate::helpers::comments::{
    InlineLeadingTrivia, LeadingTrivia, TrailingTrivia, comment_forces_line,
    format_dangling_comments, format_delimiter_dangling_comments, format_leading_comments,
    format_leading_comments_before_group, format_token,
    format_token_after_relocated_leading_comments, format_token_with_inline_leading_comments,
    format_trailing_comments_before_line_break, has_delimiter_dangling_comments,
    trailing_comments_force_line,
};
use crate::helpers::recovery::{JavaFormatDelimiter, JavaFormatListPart, resolve_list_part};

/// Java stages list elements with the shared representation; only the orphan
/// separator placement below is Java's own.
pub(crate) type CommaListItem<'source> =
    jolt_fmt_ir::CommaListItem<'source, jolt_java_syntax::JavaLanguage>;

pub(crate) use jolt_fmt_ir::{comma_list, comma_list_parts};

/// Stages a separator that has no element to attach to, preserving its trivia.
pub(crate) fn attach_comma_separator<'source>(
    doc: &mut DocBuilder<'source>,
    items: &mut Vec<CommaListItem<'source>>,
    separator: JavaSyntaxToken<'source>,
) {
    jolt_fmt_ir::attach_comma_separator(items, separator, false, |separator| {
        let separator = format_token(
            doc,
            &separator,
            LeadingTrivia::Preserve,
            TrailingTrivia::Preserve,
        );
        CommaListItem::visible(separator)
    });
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

/// Same as [`syntax_comma_list_items`], but for a list behind an open
/// delimiter. The first element's leading comments have two possible owners
/// across a reparse -- the element's first token and the open delimiter's
/// trailing trivia -- and those owners place them differently unless the
/// element keeps them on lines of their own. Register the first element as
/// beginning its line; a leading comment forces the list's group to break,
/// which is what makes the registration true.
pub(crate) fn delimited_syntax_comma_list_items<'source, Entry>(
    doc: &mut DocBuilder<'source>,
    entries: impl IntoIterator<Item = JavaSyntaxListPart<'source, Entry>>,
    mut format_entry: impl FnMut(Entry, &mut DocBuilder<'source>) -> Doc<'source>,
) -> Vec<CommaListItem<'source>>
where
    Entry: JavaSyntaxView<'source>,
{
    let mut first = true;
    syntax_comma_list_items(doc, entries, |entry, doc| {
        let first_token = if first { entry.first_token() } else { None };
        first = false;
        match first_token {
            Some(token) => doc.with_line_start_leading(&token, |doc| format_entry(entry, doc)),
            None => format_entry(entry, doc),
        }
    })
}

pub(crate) fn braced_comma_list_with_trailing_separator<'source>(
    doc: &mut DocBuilder<'source>,
    open: JavaFormatDelimiter<'source>,
    close: JavaFormatDelimiter<'source>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
    trailing_comma: Option<SynthesisClaim<'source>>,
) -> Doc<'source> {
    braced_comma_list_with_open_leading(
        doc,
        open,
        close,
        items,
        trailing_comma,
        LeadingTrivia::Preserve,
    )
}

/// Same, but the caller says whether it already emitted the open delimiter's
/// leading comments.
pub(crate) fn braced_comma_list_with_open_leading<'source>(
    doc: &mut DocBuilder<'source>,
    open: JavaFormatDelimiter<'source>,
    close: JavaFormatDelimiter<'source>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
    trailing_comma: Option<SynthesisClaim<'source>>,
    open_leading: LeadingTrivia,
) -> Doc<'source> {
    let mut items = items.into_iter().peekable();
    if items.peek().is_none() {
        return empty_delimited_list(
            doc,
            open,
            close,
            open_leading,
            InlineLeadingTrivia::BeforeToken,
        );
    }

    let (items_doc, has_source_trailing_separator) =
        comma_list_with_trailing_separator(doc, items, trailing_comma);
    let should_break = has_delimiter_dangling_comments(open.source(), close.source())
        || has_source_trailing_separator;
    let (hoisted, open_leading) =
        hoist_forcing_open_leading(doc, &open, open_leading, InlineLeadingTrivia::BeforeToken);
    let open_spacing = format_braced_open_spacing(doc, open.source());
    let contents = doc_concat!(
        doc,
        [
            format_open_delimiter(doc, open, open_leading, InlineLeadingTrivia::BeforeToken),
            doc_indent!(doc, doc_concat!(doc, [open_spacing, items_doc])),
            doc.line_boundary(),
            format_close_delimiter(doc, close),
        ]
    );

    let list = if should_break {
        doc_force_group!(doc, contents)
    } else {
        doc_group!(doc, contents)
    };
    doc_concat!(doc, [hoisted, list])
}

pub(crate) fn delimited_comma_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: JavaFormatDelimiter<'source>,
    close: JavaFormatDelimiter<'source>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
) -> Doc<'source> {
    delimited_comma_list_with_open_leading(doc, open, close, items, LeadingTrivia::Preserve)
}

/// Same, but the caller names the open delimiter's inline leading placement:
/// after a name whose trailing comments are padded, it is `BetweenSpaces`.
pub(crate) fn delimited_comma_list_with_open_placement<'source>(
    doc: &mut DocBuilder<'source>,
    open: JavaFormatDelimiter<'source>,
    close: JavaFormatDelimiter<'source>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
    open_placement: InlineLeadingTrivia,
) -> Doc<'source> {
    delimited_comma_list_with(
        doc,
        open,
        close,
        items,
        LeadingTrivia::Preserve,
        open_placement,
    )
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
    delimited_comma_list_with(
        doc,
        open,
        close,
        items,
        open_leading,
        InlineLeadingTrivia::AfterPreviousToken,
    )
}

#[allow(clippy::too_many_arguments)]
fn delimited_comma_list_with<'source>(
    doc: &mut DocBuilder<'source>,
    open: JavaFormatDelimiter<'source>,
    close: JavaFormatDelimiter<'source>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
    open_leading: LeadingTrivia,
    open_placement: InlineLeadingTrivia,
) -> Doc<'source> {
    let mut items = items.into_iter().peekable();
    if items.peek().is_none() {
        return empty_delimited_list(doc, open, close, open_leading, open_placement);
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
    let (hoisted, open_leading) =
        hoist_forcing_open_leading(doc, &open, open_leading, open_placement);
    let open_doc = format_open_delimiter_before_items(doc, open, open_leading, open_placement);
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
    doc_concat!(doc, [hoisted, list, trailing])
}

/// Splits off an open delimiter's leading comments when they end their line.
///
/// Inside the list's group the run's hard line would break the group's fit
/// even though the reparse reads the same comments as the previous token's
/// trailing trivia, which that token renders outside the group. Hoisting the
/// run before the group makes the two passes agree; the delimiter's own
/// leading placement is then suppressed.
fn hoist_forcing_open_leading<'source>(
    doc: &mut DocBuilder<'source>,
    open: &JavaFormatDelimiter<'source>,
    leading: LeadingTrivia,
    placement: InlineLeadingTrivia,
) -> (Doc<'source>, LeadingTrivia) {
    let JavaFormatDelimiter::Source(token) = open else {
        return (doc.nil(), leading);
    };
    let hoists = matches!(leading, LeadingTrivia::Preserve)
        && token
            .leading_comments()
            .last()
            .is_some_and(|comment| comment_forces_line(&comment));
    if hoists {
        let comments = format_leading_comments_before_group(doc, token, placement);
        (comments, LeadingTrivia::SuppressAlreadyHandled)
    } else {
        (doc.nil(), leading)
    }
}

fn empty_delimited_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: JavaFormatDelimiter<'source>,
    close: JavaFormatDelimiter<'source>,
    open_leading: LeadingTrivia,
    open_placement: InlineLeadingTrivia,
) -> Doc<'source> {
    if !has_delimiter_dangling_comments(open.source(), close.source()) {
        return doc_concat!(
            doc,
            [
                format_open_delimiter(doc, open, open_leading, open_placement),
                format_close_delimiter(doc, close),
            ]
        );
    }

    let dangling = format_delimiter_dangling_comments(doc, open.source(), close.source());

    doc_force_group!(
        doc,
        doc_concat!(
            doc,
            [
                format_open_delimiter(doc, open, open_leading, open_placement),
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
    placement: InlineLeadingTrivia,
) -> Doc<'source> {
    format_open_delimiter_with_trailing(
        doc,
        open,
        leading,
        TrailingTrivia::RelocatedToEnclosingContext,
        placement,
    )
}

fn format_open_delimiter_before_items<'source>(
    doc: &mut DocBuilder<'source>,
    open: JavaFormatDelimiter<'source>,
    leading: LeadingTrivia,
    placement: InlineLeadingTrivia,
) -> Doc<'source> {
    match open {
        // A delimited list's open paren or angle bracket is glued to the name
        // before it, so its leading comments take the previous token's
        // trailing form -- the placement the reparse reads back identically.
        JavaFormatDelimiter::Source(open) => format_source_open_delimiter(
            doc,
            &open,
            leading,
            TrailingTrivia::BeforeSoftLine,
            placement,
        ),
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
    placement: InlineLeadingTrivia,
) -> Doc<'source> {
    match open {
        JavaFormatDelimiter::Source(open) => {
            format_source_open_delimiter(doc, &open, leading, trailing, placement)
        }
        JavaFormatDelimiter::Recovery(recovery) => recovery.doc(),
    }
}

fn format_source_open_delimiter<'source>(
    doc: &mut DocBuilder<'source>,
    open: &JavaSyntaxToken<'source>,
    leading: LeadingTrivia,
    trailing: TrailingTrivia,
    placement: InlineLeadingTrivia,
) -> Doc<'source> {
    // An open delimiter is never a join's line-start token, so its leading
    // comments take an explicit inline placement.
    match leading {
        LeadingTrivia::Preserve => {
            format_token_with_inline_leading_comments(doc, open, placement, trailing)
        }
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
            has_source_trailing_separator |= is_last && item.comma().is_some();
            if let Some(comma) = item.comma() {
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
                doc.soft_line_boundary()
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
            // A line boundary, because the last item's own trailing comment may
            // already have ended the line: the gap names the state to reach,
            // not the breaks to append.
            doc_concat!(
                doc,
                [
                    if close.has_leading_blank_line() {
                        doc.empty_line_boundary()
                    } else {
                        doc.hard_line_boundary()
                    },
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
