use jolt_fmt_ir::{Doc, concat, force_group, group, hard_line, indent, line, soft_line};
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
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    delimited_comma_list(open, close, items, false)
}

pub(crate) fn force_parenthesized_list<'source>(
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    delimited_comma_list(open, close, items, true)
}

pub(crate) fn angle_bracket_list<'source>(
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    delimited_comma_list(open, close, items, false)
}

pub(crate) fn square_bracket_list<'source>(
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    delimited_comma_list(open, close, items, false)
}

pub(crate) fn compact_angle_bracket_list<'source>(
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    angle_bracket_list(open, close, items)
}

pub(crate) fn compact_parenthesized_list<'source>(
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    parenthesized_list(open, close, items)
}

pub(crate) fn compact_square_bracket_list<'source>(
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    square_bracket_list(open, close, items)
}

fn delimited_comma_list<'source>(
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
    items: Vec<CommaListItem<'source>>,
    force_multiline: bool,
) -> Doc<'source> {
    if items.is_empty() {
        return empty_delimited_list(open, close);
    }
    if let [item] = items.as_slice()
        && !force_multiline
        && item.comma.is_none()
        && !has_delimiter_dangling_comments(open, close)
    {
        return concat([
            format_open_delimiter(open),
            item.doc.clone(),
            format_close_delimiter(close),
        ]);
    }

    let has_trailing_comma = items.last().is_some_and(|item| item.comma.is_some());
    let doc = concat([
        indent(concat([
            format_open_delimiter_before_items(open),
            comma_list(items),
            format_close_leading_comments(close),
        ])),
        format_close_with_spacing(close),
    ]);

    if force_multiline || has_trailing_comma || has_delimiter_dangling_comments(open, close) {
        force_group(doc)
    } else {
        group(doc)
    }
}

pub(crate) fn comma_list(items: Vec<CommaListItem<'_>>) -> Doc<'_> {
    let item_count = items.len();
    let mut docs = Vec::with_capacity(item_count.saturating_mul(2));

    for (index, item) in items.into_iter().enumerate() {
        docs.push(item.doc);
        if let Some(comma) = item.comma {
            docs.push(format_separator_with_comments(
                &comma,
                if index + 1 < item_count {
                    line()
                } else {
                    jolt_fmt_ir::nil()
                },
            ));
        } else if index + 1 < item_count {
            docs.push(line());
        }
    }

    concat(docs)
}

pub(crate) fn recovered_comma_list_items<'source, Entry>(
    entries: impl IntoIterator<Item = RecoveredSeparatedListEntry<'source, Entry>>,
    mut format_entry: impl FnMut(Entry) -> CommaListItem<'source>,
) -> Vec<CommaListItem<'source>> {
    entries
        .into_iter()
        .map(move |entry| match entry {
            RecoveredSeparatedListEntry::Entry(entry) => format_entry(entry),
            RecoveredSeparatedListEntry::Token(token) => CommaListItem {
                doc: format_token(&token, LeadingTrivia::Preserve, TrailingTrivia::Preserve),
                comma: None,
            },
            RecoveredSeparatedListEntry::Error(error) => CommaListItem {
                doc: format_token_sequence(error.token_iter(), LeadingTrivia::Preserve),
                comma: None,
            },
            RecoveredSeparatedListEntry::Node(node) => CommaListItem {
                doc: format_token_sequence(node.token_iter(), LeadingTrivia::Preserve),
                comma: None,
            },
        })
        .collect()
}

fn format_open_delimiter<'source>(token: Option<&KotlinSyntaxToken<'source>>) -> Doc<'source> {
    format_open_delimiter_with_trailing(token, TrailingTrivia::RelocatedToEnclosingContext)
}

fn empty_delimited_list<'source>(
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    if !has_delimiter_dangling_comments(open, close) {
        return concat([format_open_delimiter(open), format_close_delimiter(close)]);
    }

    force_group(concat([
        format_open_delimiter(open),
        indent(concat([
            hard_line(),
            format_delimiter_dangling_comments(open, close),
        ])),
        hard_line(),
        format_close_delimiter_without_leading(close),
    ]))
}

fn format_open_delimiter_before_items<'source>(
    token: Option<&KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    token.map_or_else(jolt_fmt_ir::nil, |token| {
        format_token_with_inline_leading_comments(
            token,
            InlineLeadingTrivia::BeforeToken,
            TrailingTrivia::BeforeSoftLine,
        )
    })
}

fn format_open_delimiter_with_trailing<'source>(
    token: Option<&KotlinSyntaxToken<'source>>,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    token.map_or_else(jolt_fmt_ir::nil, |token| {
        format_token_with_inline_leading_comments(token, InlineLeadingTrivia::BeforeToken, trailing)
    })
}

fn format_close_with_spacing<'source>(close: Option<&KotlinSyntaxToken<'source>>) -> Doc<'source> {
    let close_has_leading_comments =
        close.is_some_and(|token| !token.leading_comments().is_empty());

    concat([
        if close_has_leading_comments {
            hard_line()
        } else {
            soft_line()
        },
        format_close_delimiter_without_leading(close),
    ])
}

fn format_close_leading_comments<'source>(
    close: Option<&KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    close.map_or_else(jolt_fmt_ir::nil, |close| {
        if close.leading_comments().is_empty() {
            jolt_fmt_ir::nil()
        } else {
            concat([
                hard_line(),
                format_dangling_comments(close.leading_comments()),
            ])
        }
    })
}

fn format_close_delimiter<'source>(token: Option<&KotlinSyntaxToken<'source>>) -> Doc<'source> {
    token.map_or_else(jolt_fmt_ir::nil, |token| {
        let close_has_leading_comments = !token.leading_comments().is_empty();
        concat([
            if close_has_leading_comments {
                format_leading_comments(token)
            } else {
                jolt_fmt_ir::nil()
            },
            format_token_after_relocated_leading_comments(token, TrailingTrivia::Preserve),
        ])
    })
}

fn format_close_delimiter_without_leading<'source>(
    token: Option<&KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    token.map_or_else(jolt_fmt_ir::nil, |token| {
        format_token_after_relocated_leading_comments(token, TrailingTrivia::Preserve)
    })
}

fn format_delimiter_dangling_comments<'source>(
    open: Option<&KotlinSyntaxToken<'source>>,
    close: Option<&KotlinSyntaxToken<'source>>,
) -> Doc<'source> {
    format_dangling_comments(delimiter_dangling_comments(open, close))
}
