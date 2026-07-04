use jolt_fmt_ir::{
    Doc, concat, force_group, group, hard_line, if_break, indent, line, soft_line, text,
};
use jolt_java_syntax::JavaSyntaxToken;

use crate::helpers::comments::{
    InlineLeadingTrivia, LeadingTrivia, TrailingTrivia, delimiter_dangling_comments,
    format_dangling_comments, format_leading_comments, format_separator_with_comments,
    format_token, format_token_after_relocated_leading_comments,
    format_token_with_inline_leading_comments, has_delimiter_dangling_comments,
    trailing_comments_force_line,
};

pub(crate) struct CommaListItem<'source> {
    pub(crate) doc: Doc<'source>,
    pub(crate) comma: Option<JavaSyntaxToken<'source>>,
}

pub(crate) fn comma_list<'source>(
    items: impl IntoIterator<Item = CommaListItem<'source>>,
) -> Doc<'source> {
    let mut docs = Vec::new();
    let mut items = items.into_iter().peekable();

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

pub(crate) fn parenthesized_list<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    close: Option<&JavaSyntaxToken<'source>>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
) -> Doc<'source> {
    delimited_comma_list("(", ")", open, close, items)
}

pub(crate) fn angle_bracket_list<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    close: Option<&JavaSyntaxToken<'source>>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
) -> Doc<'source> {
    delimited_comma_list("<", ">", open, close, items)
}

pub(crate) fn braced_comma_list_with_trailing_separator<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    close: Option<&JavaSyntaxToken<'source>>,
    items: Vec<CommaListItem<'source>>,
) -> Doc<'source> {
    if items.is_empty() {
        return empty_delimited_list("{", "}", open, close);
    }

    let should_break = has_dangling_delimiter_comments(open, close)
        || items.last().is_some_and(|item| item.comma.is_some());
    let doc = concat([
        format_open_delimiter(open, "{"),
        indent(concat([
            format_braced_open_spacing(open),
            comma_list_with_trailing_separator(items),
        ])),
        format_braced_close_with_spacing(close, "}"),
    ]);

    if should_break {
        force_group(doc)
    } else {
        group(doc)
    }
}

pub(crate) fn semicolon_list(items: Vec<Doc<'_>>) -> Doc<'_> {
    jolt_fmt_ir::join(&concat([text(";"), line()]), items)
}

fn delimited_comma_list<'source>(
    open_text: &'static str,
    close_text: &'static str,
    open: Option<&JavaSyntaxToken<'source>>,
    close: Option<&JavaSyntaxToken<'source>>,
    items: impl IntoIterator<Item = CommaListItem<'source>>,
) -> Doc<'source> {
    let mut items = items.into_iter().peekable();
    if items.peek().is_none() {
        return empty_delimited_list(open_text, close_text, open, close);
    }

    group(concat([
        indent(concat([
            format_open_delimiter_before_items(open, open_text),
            comma_list(items),
            format_close_leading_comments(close),
        ])),
        format_close_with_spacing(close, close_text),
    ]))
}

fn empty_delimited_list<'source>(
    open_text: &'static str,
    close_text: &'static str,
    open: Option<&JavaSyntaxToken<'source>>,
    close: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    if !has_dangling_delimiter_comments(open, close) {
        return concat([
            format_open_delimiter(open, open_text),
            format_close_delimiter(close, close_text),
        ]);
    }

    force_group(concat([
        format_open_delimiter(open, open_text),
        indent(concat([
            hard_line(),
            format_delimiter_dangling_comments(open, close),
        ])),
        hard_line(),
        format_close_delimiter_without_leading(close, close_text),
    ]))
}

fn has_dangling_delimiter_comments<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    close: Option<&JavaSyntaxToken<'source>>,
) -> bool {
    has_delimiter_dangling_comments(open, close)
}

fn format_open_delimiter<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    fallback: &'static str,
) -> Doc<'source> {
    format_open_delimiter_with_trailing(open, fallback, TrailingTrivia::RelocatedToEnclosingContext)
}

fn format_open_delimiter_before_items<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    fallback: &'static str,
) -> Doc<'source> {
    open.map_or_else(
        || concat([text(fallback), soft_line()]),
        |open| {
            format_open_delimiter_with_trailing(
                Some(open),
                fallback,
                TrailingTrivia::BeforeSoftLine,
            )
        },
    )
}

fn format_open_delimiter_with_trailing<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    fallback: &'static str,
    trailing: TrailingTrivia,
) -> Doc<'source> {
    open.map_or_else(
        || text(fallback),
        |open| {
            format_token_with_inline_leading_comments(
                open,
                InlineLeadingTrivia::BeforeToken,
                trailing,
            )
        },
    )
}

fn comma_list_with_trailing_separator(items: Vec<CommaListItem<'_>>) -> Doc<'_> {
    let mut docs = Vec::new();
    let items_len = items.len();

    for (index, item) in items.into_iter().enumerate() {
        docs.push(item.doc);
        if let Some(comma) = item.comma {
            docs.push(trailing_comma_separator(&comma, index + 1 == items_len));
        } else if index + 1 < items_len {
            docs.push(line());
        } else {
            docs.push(if_break(text(","), jolt_fmt_ir::nil()));
        }
    }

    concat(docs)
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
                text(" ")
            } else {
                jolt_fmt_ir::nil()
            }
        } else if force_line {
            hard_line()
        } else if has_trailing_comments {
            text(" ")
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
    fallback: &'static str,
) -> Doc<'source> {
    concat([line(), format_close_delimiter(close, fallback)])
}

fn format_close_with_spacing<'source>(
    close: Option<&JavaSyntaxToken<'source>>,
    fallback: &'static str,
) -> Doc<'source> {
    let close_has_leading_comments =
        close.is_some_and(|token| !token.leading_comments().is_empty());

    concat([
        if close_has_leading_comments {
            hard_line()
        } else {
            soft_line()
        },
        format_close_delimiter_without_leading(close, fallback),
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

fn format_close_delimiter<'source>(
    close: Option<&JavaSyntaxToken<'source>>,
    fallback: &'static str,
) -> Doc<'source> {
    let close_has_leading_comments =
        close.is_some_and(|token| !token.leading_comments().is_empty());
    close.map_or_else(
        || text(fallback),
        |close| {
            concat([
                if close_has_leading_comments {
                    format_leading_comments(close)
                } else {
                    jolt_fmt_ir::nil()
                },
                format_token_after_relocated_leading_comments(close, TrailingTrivia::Preserve),
            ])
        },
    )
}

fn format_close_delimiter_without_leading<'source>(
    close: Option<&JavaSyntaxToken<'source>>,
    fallback: &'static str,
) -> Doc<'source> {
    close.map_or_else(
        || text(fallback),
        |close| format_token_after_relocated_leading_comments(close, TrailingTrivia::Preserve),
    )
}

fn format_delimiter_dangling_comments<'source>(
    open: Option<&JavaSyntaxToken<'source>>,
    close: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    format_dangling_comments(delimiter_dangling_comments(open, close))
}
