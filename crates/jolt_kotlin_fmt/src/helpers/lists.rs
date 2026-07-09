use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{KotlinSyntaxToken, RecoveredSeparatedListEntry};

use crate::helpers::comments::{
    InlineLeadingTrivia, LeadingTrivia, TrailingTrivia, delimiter_dangling_comments,
    format_dangling_comments, format_leading_comments, format_separator_with_comments,
    format_token, format_token_after_relocated_leading_comments, format_token_sequence,
    format_token_with_inline_leading_comments, has_delimiter_dangling_comments,
};

pub(crate) struct CommaListItem<'source> {
    pub(crate) doc: Doc<'source>,
    pub(crate) comma: Option<KotlinSyntaxToken<'source>>,
}

pub(crate) fn parenthesized_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    delimited_comma_list(doc, open, close, items, false)
}

pub(crate) fn force_parenthesized_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    delimited_comma_list(doc, open, close, items, true)
}

pub(crate) fn angle_bracket_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    delimited_comma_list(doc, open, close, items, false)
}

pub(crate) fn square_bracket_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    delimited_comma_list(doc, open, close, items, false)
}

pub(crate) fn compact_angle_bracket_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    angle_bracket_list(doc, open, close, items)
}

pub(crate) fn compact_parenthesized_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    parenthesized_list(doc, open, close, items)
}

pub(crate) fn compact_square_bracket_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    square_bracket_list(doc, open, close, items)
}

fn delimited_comma_list<'source>(
    doc: &mut DocBuilder<'source>,
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
    items: Vec<CommaListItem<'source>>,
    force_multiline: bool,
) -> Doc<'source> {
    if items.is_empty() {
        return empty_delimited_list(doc, open, close);
    }
    if let [item] = items.as_slice()
        && !force_multiline
        && item.comma.is_none()
        && !has_delimiter_dangling_comments(open, close)
    {
        let open = format_open_delimiter(doc, open);
        let close = format_close_delimiter(doc, close);
        return doc.concat([open, item.doc, close]);
    }

    let has_trailing_comma = items.last().is_some_and(|item| item.comma.is_some());
    let open_doc = format_open_delimiter_before_items(doc, open);
    let list = comma_list(doc, items);
    let close_comments = format_close_leading_comments(doc, close);
    let indented_contents = doc.concat([open_doc, list, close_comments]);
    let indented_contents = doc.indent(indented_contents);
    let close_doc = format_close_with_spacing(doc, close);
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
    let item_count = items.len();
    let mut docs = doc.list();

    for (index, item) in items.into_iter().enumerate() {
        docs.push(item.doc, doc);
        if let Some(comma) = item.comma {
            let line = if index + 1 < item_count {
                doc.line()
            } else {
                doc.nil()
            };
            let comma = format_separator_with_comments(doc, &comma, line);
            docs.push(comma, doc);
        } else if index + 1 < item_count {
            let line = doc.line();
            docs.push(line, doc);
        }
    }

    docs.finish(doc)
}

pub(crate) fn recovered_comma_list_items<'source, Entry>(
    doc: &mut DocBuilder<'source>,
    entries: impl IntoIterator<Item = RecoveredSeparatedListEntry<'source, Entry>>,
    mut format_entry: impl FnMut(&mut DocBuilder<'source>, Entry) -> CommaListItem<'source>,
) -> Vec<CommaListItem<'source>> {
    entries
        .into_iter()
        .map(move |entry| match entry {
            RecoveredSeparatedListEntry::Entry(entry) => format_entry(doc, entry),
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
) -> Doc<'source> {
    if !has_delimiter_dangling_comments(open, close) {
        let open = format_open_delimiter(doc, open);
        let close = format_close_delimiter(doc, close);
        return doc.concat([open, close]);
    }

    let open_doc = format_open_delimiter(doc, open);
    let line = doc.hard_line();
    let comments = format_delimiter_dangling_comments(doc, open, close);
    let body = doc.concat([line, comments]);
    let body = doc.indent(body);
    let close_line = doc.hard_line();
    let close = format_close_delimiter_without_leading(doc, close);
    let list = doc.concat([open_doc, body, close_line, close]);
    doc.force_group(list)
}

fn format_open_delimiter_before_items<'source>(
    doc: &mut DocBuilder<'source>,
    token: Option<&KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    if let Some(token) = token {
        format_token_with_inline_leading_comments(
            doc,
            token,
            InlineLeadingTrivia::BeforeToken,
            TrailingTrivia::BeforeSoftLine,
        )
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
        format_token_with_inline_leading_comments(
            doc,
            token,
            InlineLeadingTrivia::BeforeToken,
            trailing,
        )
    } else {
        doc.nil()
    }
}

fn format_close_with_spacing<'source>(
    doc: &mut DocBuilder<'source>,
    close: Option<&KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    let close_has_leading_comments =
        close.is_some_and(|token| !token.leading_comments().is_empty());

    let line = if close_has_leading_comments {
        doc.hard_line()
    } else {
        doc.soft_line()
    };
    let close = format_close_delimiter_without_leading(doc, close);
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
) -> Doc<'source> {
    if let Some(token) = token {
        let close_has_leading_comments = !token.leading_comments().is_empty();
        let leading = if close_has_leading_comments {
            format_leading_comments(doc, token)
        } else {
            doc.nil()
        };
        let token =
            format_token_after_relocated_leading_comments(doc, token, TrailingTrivia::Preserve);
        doc.concat([leading, token])
    } else {
        doc.nil()
    }
}

fn format_close_delimiter_without_leading<'source>(
    doc: &mut DocBuilder<'source>,
    token: Option<&KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    if let Some(token) = token {
        format_token_after_relocated_leading_comments(doc, token, TrailingTrivia::Preserve)
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
