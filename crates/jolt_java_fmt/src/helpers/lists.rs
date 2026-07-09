use jolt_fmt_ir::space;
use jolt_fmt_ir::{Doc, concat, force_group, group, hard_line, if_break, indent, line, soft_line};
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
    items: impl IntoIterator<Item = CommaListItem<'source>>,
) -> Doc<'source> {
    let mut items = items.into_iter().peekable();
    let (lower, _) = items.size_hint();
    let mut docs = Vec::with_capacity(lower.saturating_mul(2));

    while let Some(item) = items.next() {
        docs.push(item.doc);
        if let Some(comma) = item.comma {
            docs.push(format_separator_with_comments(&comma, line()));
        } else if items.peek().is_some() {
            docs.push(line());
        }
    }

    concat(docs)
}

pub(crate) fn recovered_comma_list_items<'source, Entry>(
    entries: impl IntoIterator<Item = RecoveredSeparatedListEntry<'source, Entry>>,
    mut format_entry: impl FnMut(Entry) -> CommaListItem<'source>,
) -> impl Iterator<Item = CommaListItem<'source>> {
    entries.into_iter().map(move |entry| match entry {
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
}

pub(crate) fn parenthesized_list<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    close: Option<&JavaSyntaxToken<'source>>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
) -> Doc<'source> {
    delimited_comma_list(open, close, items)
}

pub(crate) fn angle_bracket_list<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    close: Option<&JavaSyntaxToken<'source>>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
) -> Doc<'source> {
    delimited_comma_list(open, close, items)
}

pub(crate) fn braced_comma_list_with_trailing_separator<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    close: Option<&JavaSyntaxToken<'source>>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
) -> Doc<'source> {
    let mut items = items.into_iter().peekable();
    if items.peek().is_none() {
        return empty_delimited_list(open, close);
    }

    let (items_doc, has_source_trailing_separator) = comma_list_with_trailing_separator(items);
    let should_break =
        has_delimiter_dangling_comments(open, close) || has_source_trailing_separator;
    let doc = concat([
        format_open_delimiter(open),
        indent(concat([format_braced_open_spacing(open), items_doc])),
        format_braced_close_with_spacing(close),
    ]);

    if should_break {
        force_group(doc)
    } else {
        group(doc)
    }
}

fn delimited_comma_list<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    close: Option<&JavaSyntaxToken<'source>>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
) -> Doc<'source> {
    let mut items = items.into_iter().peekable();
    if items.peek().is_none() {
        return empty_delimited_list(open, close);
    }

    group(concat([
        indent(concat([
            format_open_delimiter_before_items(open),
            comma_list(items),
            format_close_leading_comments(close),
        ])),
        format_close_with_spacing(close),
    ]))
}

fn empty_delimited_list<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    close: Option<&JavaSyntaxToken<'source>>,
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

fn format_open_delimiter<'source>(open: Option<&JavaSyntaxToken<'source>>) -> Doc<'source> {
    format_open_delimiter_with_trailing(open, TrailingTrivia::RelocatedToEnclosingContext)
}

fn format_open_delimiter_before_items<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    open.map_or_else(soft_line, |open| {
        format_token_with_inline_leading_comments(
            open,
            InlineLeadingTrivia::BeforeToken,
            TrailingTrivia::BeforeSoftLine,
        )
    })
}

fn format_open_delimiter_with_trailing<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    open.map_or_else(jolt_fmt_ir::nil, |open| {
        format_token_with_inline_leading_comments(open, InlineLeadingTrivia::BeforeToken, trailing)
    })
}

fn comma_list_with_trailing_separator<'source>(
    items: impl IntoIterator<Item = CommaListItem<'source>>,
) -> (Doc<'source>, bool) {
    let mut items = items.into_iter().peekable();
    let (lower, _) = items.size_hint();
    let mut docs = Vec::with_capacity(lower.saturating_mul(2));
    let mut has_source_trailing_separator = false;

    while let Some(item) = items.next() {
        let is_last = items.peek().is_none();
        has_source_trailing_separator |= is_last && item.comma.is_some();
        docs.push(item.doc);
        if let Some(comma) = item.comma {
            docs.push(trailing_comma_separator(&comma, is_last));
        } else if !is_last {
            docs.push(line());
        } else {
            docs.push(if_break(
                // Intentional synthesized token: trailing comma policy adds a
                // comma only when the list breaks across lines.
                inserted_syntax_token(",", FormatterInsertedToken::TrailingComma),
                jolt_fmt_ir::nil(),
            ));
        }
    }

    (concat(docs), has_source_trailing_separator)
}

fn trailing_comma_separator<'source>(
    comma: &JavaSyntaxToken<'source>,
    is_last: bool,
) -> Doc<'source> {
    let trailing_comments = comma.trailing_comments();
    let has_trailing_comments = !trailing_comments.is_empty();
    let force_line = trailing_comments_force_line(comma);

    concat([
        format_token(
            comma,
            LeadingTrivia::Preserve,
            TrailingTrivia::BeforeLineBreak,
        ),
        if is_last {
            if has_trailing_comments && !force_line {
                space()
            } else {
                jolt_fmt_ir::nil()
            }
        } else if force_line {
            hard_line()
        } else if has_trailing_comments {
            space()
        } else {
            line()
        },
    ])
}

fn format_braced_open_spacing<'source>(open: Option<&JavaSyntaxToken<'source>>) -> Doc<'source> {
    let Some(open) = open else {
        return soft_line();
    };

    let comments = open.trailing_comments();
    if comments.is_empty() {
        return line();
    }

    concat([hard_line(), format_dangling_comments(comments), hard_line()])
}

fn format_braced_close_with_spacing<'source>(
    close: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    concat([line(), format_close_delimiter(close)])
}

fn format_close_with_spacing<'source>(close: Option<&JavaSyntaxToken<'source>>) -> Doc<'source> {
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
    close: Option<&JavaSyntaxToken<'source>>,
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

fn format_close_delimiter<'source>(close: Option<&JavaSyntaxToken<'source>>) -> Doc<'source> {
    let close_has_leading_comments =
        close.is_some_and(|token| !token.leading_comments().is_empty());
    close.map_or_else(jolt_fmt_ir::nil, |close| {
        concat([
            if close_has_leading_comments {
                format_leading_comments(close)
            } else {
                jolt_fmt_ir::nil()
            },
            format_token_after_relocated_leading_comments(close, TrailingTrivia::Preserve),
        ])
    })
}

fn format_close_delimiter_without_leading<'source>(
    close: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    close.map_or_else(jolt_fmt_ir::nil, |close| {
        format_token_after_relocated_leading_comments(close, TrailingTrivia::Preserve)
    })
}

fn format_delimiter_dangling_comments<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    close: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    format_dangling_comments(delimiter_dangling_comments(open, close))
}
