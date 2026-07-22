use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{KotlinSyntaxListPart, KotlinSyntaxToken};

use crate::helpers::comments::{
    TrailingTrivia, delimiter_dangling_comments, format_dangling_comments, format_leading_comments,
    format_separator_with_comments, format_token_after_relocated_leading_comments,
    format_token_with_inline_leading_comments, has_delimiter_dangling_comments,
};

pub(crate) struct CommaListItem<'source> {
    pub(crate) doc: Doc<'source>,
    pub(crate) comma: Option<KotlinSyntaxToken<'source>>,
    pub(crate) layout_visible: bool,
}

pub(crate) fn push_recovery_item<'source>(
    items: &mut Vec<CommaListItem<'source>>,
    recovery: Doc<'source>,
    layout_visible: bool,
) {
    items.push(CommaListItem {
        doc: recovery,
        comma: None,
        layout_visible,
    });
}

pub(crate) fn delimited_comma_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    delimited_comma_list_with(doc, open, close, items, false, TrailingTrivia::Preserve)
}

pub(crate) fn annotation_parenthesized_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
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
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    delimited_comma_list_with(doc, open, close, items, true, TrailingTrivia::Preserve)
}

fn delimited_comma_list_with<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
    items: Vec<CommaListItem<'source>>,
    force_multiline: bool,
    close_trailing: TrailingTrivia,
) -> Doc<'source> {
    let visible_count = items.iter().filter(|item| item.layout_visible).count();
    if visible_count == 0 {
        let claims = doc.concat(items.into_iter().map(|item| item.doc));
        let list = empty_delimited_list(doc, open, close, close_trailing);
        return doc.concat([claims, list]);
    }

    let has_trailing_comma = items
        .iter()
        .rev()
        .find(|item| item.layout_visible)
        .is_some_and(|item| item.comma.is_some());
    let open_doc = format_open_delimiter_before_items(doc, open);
    let list = comma_list(doc, items);
    let close_comments = format_close_leading_comments(doc, close);
    let indented_contents = doc.concat([open_doc, list, close_comments]);
    let indented_contents = doc.indent(indented_contents);
    let close_doc = format_close_with_spacing(doc, close, close_trailing);
    let contents = doc.concat([indented_contents, close_doc]);

    if force_multiline || has_trailing_comma || has_delimiter_dangling_comments(open, close) {
        doc.force_group(contents)
    } else {
        doc.group(contents)
    }
}

pub(crate) fn comma_list<'source>(
    doc: &mut DocBuilder<'source>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    let item_count = items.iter().filter(|item| item.layout_visible).count();
    doc.concat_list(|docs| {
        let mut index = 0;
        for item in items {
            docs.push(item.doc);
            if !item.layout_visible {
                continue;
            }
            if let Some(comma) = item.comma {
                let line = if index + 1 < item_count {
                    docs.line()
                } else {
                    docs.nil()
                };
                let comma = format_separator_with_comments(docs, &comma, line);
                docs.push(comma);
            } else if index + 1 < item_count {
                let line = docs.line();
                docs.push(line);
            }
            index += 1;
        }
    })
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
                if let Some(item) = items.iter_mut().rev().find(|item| item.layout_visible)
                    && item.comma.is_none()
                {
                    item.comma = Some(comma);
                } else {
                    let comma = format_separator_with_comments(doc, &comma, Doc::nil());
                    items.push(CommaListItem {
                        doc: comma,
                        comma: None,
                        layout_visible: true,
                    });
                }
            }
            KotlinFormatListPart::Malformed(malformed) => {
                push_recovery_item(&mut items, malformed, true);
            }
            KotlinFormatListPart::Invisible(recovery) => {
                push_recovery_item(&mut items, recovery, false);
            }
        }
    }
    items
}

fn format_open_delimiter<'source>(
    doc: &mut DocBuilder<'source>,
    token: Option<&KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    format_open_delimiter_with_trailing(doc, token, TrailingTrivia::RelocatedToEnclosingContext)
}

fn empty_delimited_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
    close_trailing: TrailingTrivia,
) -> Doc<'source> {
    if !has_delimiter_dangling_comments(open, close) {
        let open = format_open_delimiter(doc, open);
        let close = format_close_delimiter(doc, close, close_trailing);
        return doc.concat([open, close]);
    }

    let open_doc = format_open_delimiter(doc, open);
    let line = doc.hard_line();
    let comments = format_delimiter_dangling_comments(doc, open, close);
    let body = doc.concat([line, comments]);
    let body = doc.indent(body);
    let close_line = doc.hard_line();
    let close = format_close_delimiter_without_leading(doc, close, close_trailing);
    let list = doc.concat([open_doc, body, close_line, close]);
    doc.force_group(list)
}

fn format_open_delimiter_before_items<'source>(
    doc: &mut DocBuilder<'source>,
    token: Option<&KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    if let Some(token) = token {
        format_token_with_inline_leading_comments(doc, token, TrailingTrivia::BeforeSoftLine)
    } else {
        doc.nil()
    }
}

fn format_open_delimiter_with_trailing<'source>(
    doc: &mut DocBuilder<'source>,
    token: Option<&KotlinSyntaxToken<'source>>,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    if let Some(token) = token {
        format_token_with_inline_leading_comments(doc, token, trailing)
    } else {
        doc.nil()
    }
}

fn format_close_with_spacing<'source>(
    doc: &mut DocBuilder<'source>,
    close: Option<&KotlinSyntaxToken<'source>>,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    let close_has_leading_comments =
        close.is_some_and(|token| !token.leading_comments().is_empty());

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
            let line = doc.hard_line();
            let comments = format_dangling_comments(doc, close.leading_comments());
            doc.concat([line, comments])
        }
    } else {
        doc.nil()
    }
}

fn format_close_delimiter<'source>(
    doc: &mut DocBuilder<'source>,
    token: Option<&KotlinSyntaxToken<'source>>,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    if let Some(token) = token {
        let close_has_leading_comments = !token.leading_comments().is_empty();
        let leading = if close_has_leading_comments {
            format_leading_comments(doc, token)
        } else {
            doc.nil()
        };
        let token = format_token_after_relocated_leading_comments(doc, token, trailing);
        doc.concat([leading, token])
    } else {
        doc.nil()
    }
}

fn format_close_delimiter_without_leading<'source>(
    doc: &mut DocBuilder<'source>,
    token: Option<&KotlinSyntaxToken<'source>>,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    if let Some(token) = token {
        format_token_after_relocated_leading_comments(doc, token, trailing)
    } else {
        doc.nil()
    }
}

fn format_delimiter_dangling_comments<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    let comments = delimiter_dangling_comments(open, close);
    format_dangling_comments(doc, comments)
}
