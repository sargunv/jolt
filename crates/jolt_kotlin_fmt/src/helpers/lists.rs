use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{KotlinSyntaxListPart, KotlinSyntaxToken, KotlinSyntaxView};

use crate::helpers::recovery::KotlinFormatDelimiter;

use crate::helpers::comments::{
    TrailingTrivia, format_dangling_comments, format_delimiter_dangling_comments,
    format_leading_comments, format_separator_with_comments,
    format_token_after_relocated_leading_comments, format_token_with_inline_leading_comments,
    has_delimiter_dangling_comments,
};
use jolt_fmt_ir::formatter_ignore::{
    FormatterIgnoreItemRange, FormatterIgnoreRun, FormatterIgnoreSplice,
    for_each_formatter_ignore_splice, formatter_ignore_content_range, formatter_ignore_run_doc,
    formatter_ignore_runs_claim_boundary_comment,
};

/// Kotlin stages physical items and separators with the shared representation.
/// Separators remain distinct until formatter-ignore runs have been spliced,
/// so an ignored item can never accidentally take an unignored comma with it.
pub(crate) type CommaListItem<'source> =
    jolt_fmt_ir::CommaListItem<'source, jolt_kotlin_syntax::KotlinLanguage>;

pub(crate) fn attach_comma_separator<'source>(
    items: &mut Vec<CommaListItem<'source>>,
    separator: KotlinSyntaxToken<'source>,
) {
    items.push(CommaListItem::physical_separator(separator));
}

pub(crate) fn comma_list<'source>(
    doc: &mut DocBuilder<'source>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    let items = prepare_comma_list_items(doc, items);
    jolt_fmt_ir::comma_list(doc, items)
}

pub(crate) fn prepare_comma_list_items<'source>(
    doc: &mut DocBuilder<'source>,
    items: Vec<CommaListItem<'source>>,
) -> Vec<CommaListItem<'source>> {
    prepare_comma_list_items_between(doc, items, None, None)
}

pub(crate) fn prepare_comma_list_items_between<'source>(
    doc: &mut DocBuilder<'source>,
    items: Vec<CommaListItem<'source>>,
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
) -> Vec<CommaListItem<'source>> {
    let runs = formatter_ignore_list_runs(doc, open, close, &items);
    let items = splice_formatter_ignore_items(doc, items, &runs);
    attach_staged_separators(doc, items)
}

pub(crate) fn comma_list_item_range<'source>(
    item: &impl KotlinSyntaxView<'source>,
) -> Option<FormatterIgnoreItemRange> {
    item.first_token()
        .zip(item.syntax_node().and_then(|syntax| syntax.last_token()))
        .map(|(first, last)| FormatterIgnoreItemRange::between(&first, &last))
}

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
    let ignored_runs = formatter_ignore_list_runs(doc, open.source(), close.source(), &items);
    let (close_comments, close_has_leading_comments) =
        format_close_leading_comments(doc, close.source(), &ignored_runs);
    let items = splice_formatter_ignore_items(doc, items, &ignored_runs);
    let items = attach_staged_separators(doc, items);
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
    let list = jolt_fmt_ir::comma_list(doc, items);
    let indented_contents = doc.concat([open_doc, list, close_comments]);
    let indented_contents = doc.indent(indented_contents);
    let close_doc =
        format_close_with_spacing(doc, close, close_trailing, close_has_leading_comments);
    let contents = doc.concat([indented_contents, close_doc]);

    if force_multiline
        || has_trailing_comma
        || !ignored_runs.is_empty()
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
) -> Vec<CommaListItem<'source>>
where
    Entry: KotlinSyntaxView<'source>,
{
    use crate::helpers::recovery::{KotlinFormatListPart, resolve_list_part};

    let mut items = Vec::new();
    for part in entries {
        match resolve_list_part(part, doc) {
            KotlinFormatListPart::Item(entry) => {
                let range = comma_list_item_range(&entry);
                items.push(format_entry(doc, entry).with_ignore_range(range));
            }
            KotlinFormatListPart::Separator(comma) => {
                attach_comma_separator(&mut items, comma);
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
    close_has_leading_comments: bool,
) -> Doc<'source> {
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
    ignored_runs: &[FormatterIgnoreRun<'source>],
) -> (Doc<'source>, bool) {
    if let Some(close) = close {
        let comments = close
            .leading_comments()
            .filter(|comment| !formatter_ignore_runs_claim_boundary_comment(ignored_runs, comment))
            .collect::<Vec<_>>();
        if comments.is_empty() {
            (doc.nil(), false)
        } else {
            // A line boundary, because the last item's own trailing comment may
            // already have ended the line: the gap names the state to reach,
            // not the breaks to append.
            let line = if close.has_leading_blank_line() {
                doc.empty_line_boundary()
            } else {
                doc.hard_line_boundary()
            };
            let comments = format_dangling_comments(doc, comments);
            (doc.concat([line, comments]), true)
        }
    } else {
        (doc.nil(), false)
    }
}

fn formatter_ignore_list_runs<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
    items: &[CommaListItem<'source>],
) -> Vec<FormatterIgnoreRun<'source>> {
    let first = items.iter().find_map(CommaListItem::ignore_range);
    let last = items
        .iter()
        .filter_map(CommaListItem::ignore_range)
        .next_back();
    let fallback = first
        .zip(last)
        .map(|(first, last)| FormatterIgnoreItemRange::spanning(first, last))
        .or_else(|| open.map(KotlinSyntaxToken::token_text_range))
        .or_else(|| close.map(KotlinSyntaxToken::token_text_range));
    let Some(fallback) = fallback else {
        return Vec::new();
    };
    let container = formatter_ignore_content_range(fallback, open.copied(), close.copied());
    doc.formatter_ignore_runs(container, items.iter().map(CommaListItem::ignore_range))
}

fn splice_formatter_ignore_items<'source>(
    doc: &mut DocBuilder<'source>,
    items: Vec<CommaListItem<'source>>,
    runs: &[FormatterIgnoreRun<'source>],
) -> Vec<CommaListItem<'source>> {
    if runs.is_empty() {
        return items;
    }
    let mut items = items.into_iter().map(Some).collect::<Vec<_>>();
    let mut spliced = Vec::with_capacity(items.len().saturating_add(runs.len()));
    for_each_formatter_ignore_splice(items.len(), runs, |event| match event {
        FormatterIgnoreSplice::Ignore(run) => {
            // Ignore ranges are line-oriented source sections. Make that
            // boundary part of the staged item so inline and non-delimited
            // callers both re-indent its first raw line in their own context.
            let boundary = doc.hard_line_boundary();
            let run = formatter_ignore_run_doc(run, doc);
            spliced.push(CommaListItem::visible(doc.concat([boundary, run])).with_line_before());
        }
        FormatterIgnoreSplice::Item { index, .. } => {
            if let Some(item) = items[index].take() {
                spliced.push(item);
            }
        }
    });
    spliced
}

fn attach_staged_separators<'source>(
    doc: &mut DocBuilder<'source>,
    items: Vec<CommaListItem<'source>>,
) -> Vec<CommaListItem<'source>> {
    let mut attached = Vec::with_capacity(items.len());
    for item in items {
        if let Some(separator) = item.staged_separator() {
            jolt_fmt_ir::attach_comma_separator(&mut attached, separator, |separator| {
                let separator = format_separator_with_comments(doc, &separator, Doc::nil());
                CommaListItem::visible(separator)
            });
        } else {
            attached.push(item);
        }
    }
    attached
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
