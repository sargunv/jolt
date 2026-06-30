use jolt_fmt_ir::{
    Doc, FlatLine, break_, concat, group, hard_line, indent, indent_by, join, line, soft_line, text,
};

use crate::helpers::lists as java_lists;

const CONTINUATION_INDENT_LEVELS: u16 = 2;

fn continuation_indent(doc: Doc) -> Doc {
    indent_by(CONTINUATION_INDENT_LEVELS, doc)
}

pub(crate) fn space_separated(parts: impl IntoIterator<Item = Doc>) -> Doc {
    group(join(line(), parts))
}

pub(crate) fn comma_list(items: impl IntoIterator<Item = Doc>) -> Doc {
    java_lists::comma_list(items)
}

pub(crate) fn braced_block(items: impl IntoIterator<Item = Doc>) -> Doc {
    braced_block_with_separator(items, hard_line())
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct BracedBodyLayout {
    pub leading_blank_line: bool,
    pub trailing_blank_line: bool,
}

pub(crate) fn braced_body(items: Vec<Doc>, separators: Vec<Doc>, layout: BracedBodyLayout) -> Doc {
    let BracedBodyLayout {
        leading_blank_line,
        trailing_blank_line,
    } = layout;

    if items.is_empty() && !leading_blank_line && !trailing_blank_line {
        return concat([text("{"), hard_line(), text("}")]);
    }

    let mut body = Vec::new();
    let mut items = items.into_iter();
    if let Some(first) = items.next() {
        body.push(first);
    }
    for (separator, item) in separators.into_iter().zip(items) {
        body.push(separator);
        body.push(item);
    }

    if body.is_empty() {
        return concat([text("{"), hard_line(), text("}")]);
    }

    let mut parts = vec![text("{")];
    if leading_blank_line {
        parts.push(break_(FlatLine::Empty, i16::MIN));
    }
    parts.push(indent(concat([hard_line(), concat(body)])));
    if trailing_blank_line {
        parts.push(break_(FlatLine::Empty, i16::MIN));
    }
    parts.push(hard_line());
    parts.push(text("}"));
    concat(parts)
}

pub(crate) fn braced_block_with_separators(
    items: impl IntoIterator<Item = Doc>,
    separators: impl IntoIterator<Item = Doc>,
) -> Doc {
    let items = items.into_iter().collect::<Vec<_>>();
    let separators = separators.into_iter().collect::<Vec<_>>();
    if items.is_empty() {
        return group(concat([text("{"), soft_line(), text("}")]));
    }

    let mut body = Vec::new();
    let mut items = items.into_iter();
    if let Some(first) = items.next() {
        body.push(first);
    }
    for (separator, item) in separators.into_iter().zip(items) {
        body.push(separator);
        body.push(item);
    }

    concat([
        text("{"),
        indent(concat([hard_line(), concat(body)])),
        hard_line(),
        text("}"),
    ])
}

fn braced_block_with_separator(items: impl IntoIterator<Item = Doc>, separator: Doc) -> Doc {
    let items = items.into_iter().collect::<Vec<_>>();
    if items.is_empty() {
        return group(concat([text("{"), soft_line(), text("}")]));
    }

    concat([
        text("{"),
        indent(concat([hard_line(), join(separator, items)])),
        hard_line(),
        text("}"),
    ])
}

pub(crate) fn keyword_expression_statement(keyword: &'static str, expression: Option<Doc>) -> Doc {
    let Some(expression) = expression else {
        return text(format!("{keyword};"));
    };

    group(concat([text(keyword), text(" "), expression, text(";")]))
}

pub(crate) fn keyword_label_statement(keyword: &'static str, label: Option<Doc>) -> Doc {
    let Some(label) = label else {
        return text(format!("{keyword};"));
    };

    group(concat([text(keyword), text(" "), label, text(";")]))
}

pub(crate) fn expression_statement(expression: Doc) -> Doc {
    group(concat([expression, text(";")]))
}

pub(crate) fn flat_parenthesized_expression(expression: Doc) -> Doc {
    group(concat([text("("), expression, text(")")]))
}

pub(crate) fn parenthesized_expression(expression: Doc) -> Doc {
    group(concat([
        text("("),
        continuation_indent(concat([soft_line(), expression])),
        soft_line(),
        text(")"),
    ]))
}

pub(crate) fn if_statement(
    condition: Doc,
    then_statement: Doc,
    then_is_block: bool,
    else_statement: Option<(Doc, bool)>,
) -> Doc {
    let header = group(concat([
        text("if "),
        flat_parenthesized_expression(condition),
    ]));
    let mut parts = Vec::new();
    if then_is_block {
        parts.push(header);
        parts.push(text(" "));
        parts.push(then_statement);
    } else {
        parts.push(group(concat([header, line(), then_statement])));
    }

    if let Some((else_statement, else_follows_keyword)) = else_statement {
        if then_is_block {
            parts.push(text(" "));
        } else {
            parts.push(hard_line());
        }
        if else_follows_keyword {
            parts.push(group(concat([text("else "), else_statement])));
        } else {
            parts.push(group(concat([text("else"), line(), else_statement])));
        }
    }

    concat(parts)
}

pub(crate) fn while_statement(condition: Doc, body: Doc, body_is_block: bool) -> Doc {
    loop_statement(
        group(concat([
            text("while "),
            flat_parenthesized_expression(condition),
        ])),
        body,
        body_is_block,
    )
}

pub(crate) fn for_statement(header: Doc, body: Doc, body_is_block: bool) -> Doc {
    loop_statement(header, body, body_is_block)
}

pub(crate) fn basic_for_header(
    initializer: Option<Doc>,
    condition: Option<Doc>,
    update: Option<Doc>,
) -> Doc {
    if initializer.is_none() && condition.is_none() && update.is_none() {
        return text("for (; ; )");
    }

    let mut clauses = Vec::new();
    if let Some(initializer) = initializer {
        clauses.push(initializer);
    }
    clauses.push(text(";"));
    clauses.push(line());
    if let Some(condition) = condition {
        clauses.push(condition);
    }
    clauses.push(text(";"));
    if let Some(update) = update {
        clauses.push(line());
        clauses.push(update);
    } else {
        clauses.push(text(" "));
    }

    group(concat([
        text("for ("),
        continuation_indent(concat(clauses)),
        text(")"),
    ]))
}

pub(crate) fn enhanced_for_header(variable: Doc, iterable: Doc) -> Doc {
    group(concat([
        text("for ("),
        variable,
        text(" :"),
        continuation_indent(concat([line(), iterable])),
        text(")"),
    ]))
}

fn loop_statement(header: Doc, body: Doc, body_is_block: bool) -> Doc {
    if body_is_block {
        concat([header, text(" "), body])
    } else {
        group(concat([header, line(), body]))
    }
}

pub(crate) fn do_statement(body: Doc, body_is_block: bool, condition: Doc) -> Doc {
    let while_clause = group(concat([
        text("while "),
        flat_parenthesized_expression(condition),
        text(";"),
    ]));

    if body_is_block {
        group(concat([text("do "), body, text(" "), while_clause]))
    } else {
        group(concat([text("do"), line(), body, line(), while_clause]))
    }
}

pub(crate) fn try_statement(
    body: Doc,
    catches: impl IntoIterator<Item = Doc>,
    finally_clause: Option<Doc>,
) -> Doc {
    try_statement_with_header(concat([text("try "), body]), catches, finally_clause)
}

pub(crate) fn try_statement_with_header(
    header: Doc,
    catches: impl IntoIterator<Item = Doc>,
    finally_clause: Option<Doc>,
) -> Doc {
    let mut parts = vec![header];
    for catch in catches {
        parts.push(text(" "));
        parts.push(catch);
    }
    if let Some(finally_clause) = finally_clause {
        parts.push(text(" "));
        parts.push(finally_clause);
    }

    concat(parts)
}
