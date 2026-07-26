use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{KotlinSyntaxListPart, KotlinSyntaxToken};

use crate::helpers::recovery::KotlinFormatDelimiter;

use crate::helpers::comments::{
    TrailingTrivia, format_dangling_comments, format_delimiter_dangling_comments,
    format_leading_comments, format_separator_with_comments,
    format_token_after_relocated_leading_comments, format_token_with_inline_leading_comments,
    has_delimiter_dangling_comments,
};

/// Kotlin stages list elements with the shared representation; only the orphan
/// separator placements at each call site are Kotlin's own.
pub(crate) type CommaListItem<'source> =
    jolt_fmt_ir::CommaListItem<'source, jolt_kotlin_syntax::KotlinLanguage>;

pub(crate) use jolt_fmt_ir::{attach_comma_separator, comma_list};

pub(crate) fn delimited_comma_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: KotlinFormatDelimiter<'source>,
    close: KotlinFormatDelimiter<'source>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    delimited_comma_list_with(doc, open, close, items, false, TrailingTrivia::Preserve)
}

pub(crate) fn annotation_parenthesized_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: KotlinFormatDelimiter<'source>,
    close: KotlinFormatDelimiter<'source>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    delimited_comma_list_with(
        doc,
        open,
        close,
        items,
        false,
        TrailingTrivia::BeforeLineBreak,
    )
}

pub(crate) fn force_parenthesized_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: KotlinFormatDelimiter<'source>,
    close: KotlinFormatDelimiter<'source>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    delimited_comma_list_with(doc, open, close, items, true, TrailingTrivia::Preserve)
}

fn delimited_comma_list_with<'source>(
    doc: &mut DocBuilder<'source>,
    open: KotlinFormatDelimiter<'source>,
    close: KotlinFormatDelimiter<'source>,
    items: Vec<CommaListItem<'source>>,
    force_multiline: bool,
    close_trailing: TrailingTrivia,
) -> Doc<'source> {
    let visible_count = items.iter().filter(|item| item.is_visible()).count();
    if visible_count == 0 {
        let claims = doc.concat(items.iter().map(CommaListItem::doc));
        let list = empty_delimited_list(doc, open, close, close_trailing);
        return doc.concat([claims, list]);
    }

    let has_trailing_comma = items
        .iter()
        .rev()
        .find(|item| item.is_visible())
        .is_some_and(|item| item.comma().is_some());
    let open_doc = format_open_delimiter_with_trailing(doc, open, TrailingTrivia::BeforeSoftLine);
    let list = comma_list(doc, items);
    let close_comments = format_close_leading_comments(doc, close.source());
    let indented_contents = doc.concat([open_doc, list, close_comments]);
    let indented_contents = doc.indent(indented_contents);
    let close_doc = format_close_with_spacing(doc, close, close_trailing);
    let contents = doc.concat([indented_contents, close_doc]);

    if force_multiline
        || has_trailing_comma
        || has_delimiter_dangling_comments(open.source(), close.source())
    {
        doc.force_group(contents)
    } else {
        doc.group(contents)
    }
}

pub(crate) fn physical_comma_list_items<'source, Entry>(
    doc: &mut DocBuilder<'source>,
    entries: impl IntoIterator<Item = KotlinSyntaxListPart<'source, Entry>>,
    mut format_entry: impl FnMut(&mut DocBuilder<'source>, Entry) -> CommaListItem<'source>,
) -> Vec<CommaListItem<'source>> {
    use crate::helpers::recovery::{KotlinFormatListPart, resolve_list_part};

    let mut items = Vec::new();
    for part in entries {
        match resolve_list_part(part, doc) {
            KotlinFormatListPart::Item(entry) => items.push(format_entry(doc, entry)),
            KotlinFormatListPart::Separator(comma) => {
                attach_comma_separator(&mut items, comma, |comma| {
                    let comma = format_separator_with_comments(doc, &comma, Doc::nil());
                    CommaListItem::visible(comma)
                });
            }
            KotlinFormatListPart::Recovery(recovery) => {
                items.push(CommaListItem::recovery(recovery));
            }
        }
    }
    items
}

fn empty_delimited_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: KotlinFormatDelimiter<'source>,
    close: KotlinFormatDelimiter<'source>,
    close_trailing: TrailingTrivia,
) -> Doc<'source> {
    if !has_delimiter_dangling_comments(open.source(), close.source()) {
        let open = format_open_delimiter_with_trailing(
            doc,
            open,
            TrailingTrivia::RelocatedToEnclosingContext,
        );
        let close = format_close_delimiter(doc, close, close_trailing);
        return doc.concat([open, close]);
    }

    let open_doc =
        format_open_delimiter_with_trailing(doc, open, TrailingTrivia::RelocatedToEnclosingContext);
    let line = doc.hard_line();
    let comments = format_delimiter_dangling_comments(doc, open.source(), close.source());
    let body = doc.concat([line, comments]);
    let body = doc.indent(body);
    let close_line = doc.hard_line();
    let close = format_close_delimiter_without_leading(doc, close, close_trailing);
    let list = doc.concat([open_doc, body, close_line, close]);
    doc.force_group(list)
}

fn format_open_delimiter_with_trailing<'source>(
    doc: &mut DocBuilder<'source>,
    open: KotlinFormatDelimiter<'source>,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    match open {
        KotlinFormatDelimiter::Source(open) => {
            format_token_with_inline_leading_comments(doc, &open, trailing)
        }
        KotlinFormatDelimiter::Recovery(recovery) => recovery.doc(),
    }
}

fn format_close_with_spacing<'source>(
    doc: &mut DocBuilder<'source>,
    close: KotlinFormatDelimiter<'source>,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    let close_has_leading_comments = close
        .source()
        .is_some_and(|token| !token.leading_comments().is_empty());

    let line = if close_has_leading_comments {
        doc.hard_line()
    } else {
        doc.soft_line()
    };
    let close = format_close_delimiter_without_leading(doc, close, trailing);
    doc.concat([line, close])
}

fn format_close_leading_comments<'source>(
    doc: &mut DocBuilder<'source>,
    close: Option<&KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    if let Some(close) = close {
        if close.leading_comments().is_empty() {
            doc.nil()
        } else {
            // A line boundary, because the last item's own trailing comment may
            // already have ended the line: the gap names the state to reach,
            // not the breaks to append.
            let line = if close.has_leading_blank_line() {
                doc.empty_line_boundary()
            } else {
                doc.hard_line_boundary()
            };
            let comments = format_dangling_comments(doc, close.leading_comments());
            doc.concat([line, comments])
        }
    } else {
        doc.nil()
    }
}

fn format_close_delimiter<'source>(
    doc: &mut DocBuilder<'source>,
    close: KotlinFormatDelimiter<'source>,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    match close {
        KotlinFormatDelimiter::Source(close) => {
            let leading = if close.leading_comments().is_empty() {
                doc.nil()
            } else {
                format_leading_comments(doc, &close)
            };
            let close = format_token_after_relocated_leading_comments(doc, &close, trailing);
            doc.concat([leading, close])
        }
        KotlinFormatDelimiter::Recovery(recovery) => recovery.doc(),
    }
}

fn format_close_delimiter_without_leading<'source>(
    doc: &mut DocBuilder<'source>,
    close: KotlinFormatDelimiter<'source>,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    match close {
        KotlinFormatDelimiter::Source(close) => {
            format_token_after_relocated_leading_comments(doc, &close, trailing)
        }
        KotlinFormatDelimiter::Recovery(recovery) => recovery.doc(),
    }
}
