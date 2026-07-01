use jolt_fmt_ir::{Doc, concat, force_group, group, hard_line, indent, line, soft_line, text};
use jolt_java_syntax::JavaSyntaxToken;

use crate::helpers::comments::{
    format_dangling_comments, format_leading_comments, format_trailing_comments,
    format_trailing_comments_before_line_break, trailing_comments_force_line,
};

pub(crate) struct CommaListItem {
    pub(crate) doc: Doc,
    pub(crate) comma: Option<JavaSyntaxToken>,
}

pub(crate) fn comma_list(items: Vec<CommaListItem>) -> Doc {
    let mut docs = Vec::new();
    let items_len = items.len();

    for (index, item) in items.into_iter().enumerate() {
        docs.push(item.doc);
        if let Some(comma) = item.comma {
            docs.push(comma_separator(&comma));
        } else if index + 1 < items_len {
            docs.push(line());
        }
    }

    concat(docs)
}

pub(crate) fn parenthesized_list(
    open: Option<&JavaSyntaxToken>,
    close: Option<&JavaSyntaxToken>,
    items: Vec<CommaListItem>,
) -> Doc {
    delimited_comma_list("(", ")", open, close, items)
}

pub(crate) fn semicolon_list(items: Vec<Doc>) -> Doc {
    jolt_fmt_ir::join(concat([text(";"), line()]), items)
}

fn delimited_comma_list(
    open_text: &'static str,
    close_text: &'static str,
    open: Option<&JavaSyntaxToken>,
    close: Option<&JavaSyntaxToken>,
    items: Vec<CommaListItem>,
) -> Doc {
    if items.is_empty() {
        return empty_delimited_list(open_text, close_text, open, close);
    }

    group(concat([
        format_open_delimiter(open, open_text),
        indent(concat([format_open_spacing(open), comma_list(items)])),
        format_close_with_spacing(close, close_text),
    ]))
}

fn empty_delimited_list(
    open_text: &'static str,
    close_text: &'static str,
    open: Option<&JavaSyntaxToken>,
    close: Option<&JavaSyntaxToken>,
) -> Doc {
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

fn has_dangling_delimiter_comments(
    open: Option<&JavaSyntaxToken>,
    close: Option<&JavaSyntaxToken>,
) -> bool {
    open.is_some_and(|token| !token.trailing_comments().is_empty())
        || close.is_some_and(|token| !token.leading_comments().is_empty())
}

fn format_open_delimiter(open: Option<&JavaSyntaxToken>, fallback: &'static str) -> Doc {
    open.map_or_else(
        || text(fallback),
        |open| concat([format_leading_comments(open), text(fallback)]),
    )
}

fn format_open_spacing(open: Option<&JavaSyntaxToken>) -> Doc {
    let Some(open) = open else {
        return soft_line();
    };

    if open.trailing_comments().is_empty() {
        return soft_line();
    }

    concat([
        format_trailing_comments_before_line_break(open),
        if trailing_comments_force_line(open) {
            hard_line()
        } else {
            soft_line()
        },
    ])
}

fn comma_separator(comma: &JavaSyntaxToken) -> Doc {
    concat([
        format_leading_comments(comma),
        text(","),
        format_trailing_comments_before_line_break(comma),
        if trailing_comments_force_line(comma) {
            hard_line()
        } else {
            line()
        },
    ])
}

fn format_close_with_spacing(close: Option<&JavaSyntaxToken>, fallback: &'static str) -> Doc {
    concat([
        if close.is_some_and(|token| !token.leading_comments().is_empty()) {
            line()
        } else {
            soft_line()
        },
        format_close_delimiter(close, fallback),
    ])
}

fn format_close_delimiter(close: Option<&JavaSyntaxToken>, fallback: &'static str) -> Doc {
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
                text(fallback),
                format_trailing_comments(close),
            ])
        },
    )
}

fn format_close_delimiter_without_leading(
    close: Option<&JavaSyntaxToken>,
    fallback: &'static str,
) -> Doc {
    close.map_or_else(
        || text(fallback),
        |close| concat([text(fallback), format_trailing_comments(close)]),
    )
}

fn format_delimiter_dangling_comments(
    open: Option<&JavaSyntaxToken>,
    close: Option<&JavaSyntaxToken>,
) -> Doc {
    let mut comments = Vec::new();

    if let Some(open) = open {
        comments.extend(open.trailing_comments());
    }
    if let Some(close) = close {
        comments.extend(close.leading_comments());
    }

    format_dangling_comments(comments)
}
