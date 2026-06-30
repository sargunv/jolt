use jolt_fmt_ir::{Doc, concat, group, hard_line, indent_by, line, text};

use crate::helpers::expressions as java_expressions;
use crate::policy::JavaFormatPolicy;

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

pub(crate) fn if_statement(
    condition: Doc,
    then_statement: Doc,
    then_is_block: bool,
    else_statement: Option<(Doc, bool)>,
) -> Doc {
    let header = group(concat([
        text("if "),
        java_expressions::flat_parenthesized_expression(condition),
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
            java_expressions::flat_parenthesized_expression(condition),
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
    policy: JavaFormatPolicy,
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
        indent_by(policy.continuation_indent_levels(), concat(clauses)),
        text(")"),
    ]))
}

pub(crate) fn enhanced_for_header(variable: Doc, iterable: Doc, policy: JavaFormatPolicy) -> Doc {
    group(concat([
        text("for ("),
        variable,
        text(" :"),
        indent_by(
            policy.continuation_indent_levels(),
            concat([line(), iterable]),
        ),
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
        java_expressions::flat_parenthesized_expression(condition),
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
