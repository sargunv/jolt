use jolt_fmt_ir::{
    Doc, concat, fill, fill_entry, group, hard_line, indent, indent_by, join, line, soft_line, text,
};

const CONTINUATION_INDENT_LEVELS: u16 = 2;

fn continuation_indent(doc: Doc) -> Doc {
    indent_by(CONTINUATION_INDENT_LEVELS, doc)
}

pub(crate) fn space_separated(parts: impl IntoIterator<Item = Doc>) -> Doc {
    group(join(line(), parts))
}

pub(crate) fn declaration_header(parts: impl IntoIterator<Item = Doc>) -> Doc {
    space_separated(parts)
}

pub(crate) fn parenthesized_comma_list(items: impl IntoIterator<Item = Doc>) -> Doc {
    delimited_comma_list("(", ")", items)
}

pub(crate) fn angle_comma_list(items: impl IntoIterator<Item = Doc>) -> Doc {
    delimited_comma_list("<", ">", items)
}

fn delimited_comma_list(
    open: &'static str,
    close: &'static str,
    items: impl IntoIterator<Item = Doc>,
) -> Doc {
    let mut items = items.into_iter().collect::<Vec<_>>();
    if items.is_empty() {
        return text(format!("{open}{close}"));
    }

    let last = items.pop().expect("non-empty items checked above");
    let entries = items
        .into_iter()
        .map(|item| fill_entry(item, concat([text(","), line()])));

    group(concat([
        text(open),
        continuation_indent(concat([
            soft_line(),
            fill(entries, concat([last, text(close)])),
        ])),
    ]))
}

pub(crate) fn comma_list(items: impl IntoIterator<Item = Doc>) -> Doc {
    group(join(concat([text(","), line()]), items))
}

pub(crate) fn variable_declaration(prefix: impl IntoIterator<Item = Doc>, declarators: Doc) -> Doc {
    group(concat([
        variable_declaration_header(prefix, declarators),
        text(";"),
    ]))
}

pub(crate) fn variable_declaration_header(
    prefix: impl IntoIterator<Item = Doc>,
    declarators: Doc,
) -> Doc {
    group(concat([space_separated(prefix), text(" "), declarators]))
}

pub(crate) fn variable_declarator(name: Doc, initializer: Option<Doc>) -> Doc {
    let Some(initializer) = initializer else {
        return name;
    };

    group(concat([
        name,
        text(" ="),
        continuation_indent(concat([line(), initializer])),
    ]))
}

pub(crate) fn braced_block(items: impl IntoIterator<Item = Doc>) -> Doc {
    let items = items.into_iter().collect::<Vec<_>>();
    if items.is_empty() {
        return group(concat([text("{"), soft_line(), text("}")]));
    }

    concat([
        text("{"),
        indent(concat([hard_line(), join(hard_line(), items)])),
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
    let mut parts = vec![text("if "), parenthesized_expression(condition)];
    if then_is_block {
        parts.push(text(" "));
        parts.push(then_statement);
    } else {
        parts.push(indent(concat([hard_line(), then_statement])));
    }

    if let Some((else_statement, else_follows_keyword)) = else_statement {
        if then_is_block {
            parts.push(text(" "));
        } else {
            parts.push(hard_line());
        }
        parts.push(text("else"));
        if else_follows_keyword {
            parts.push(text(" "));
            parts.push(else_statement);
        } else {
            parts.push(indent(concat([hard_line(), else_statement])));
        }
    }

    concat(parts)
}

pub(crate) fn while_statement(condition: Doc, body: Doc, body_is_block: bool) -> Doc {
    loop_statement(
        concat([text("while "), parenthesized_expression(condition)]),
        body,
        body_is_block,
    )
}

pub(crate) fn for_statement(header: Doc, body: Doc, body_is_block: bool) -> Doc {
    loop_statement(header, body, body_is_block)
}

fn loop_statement(header: Doc, body: Doc, body_is_block: bool) -> Doc {
    let mut parts = vec![header];
    if body_is_block {
        parts.push(text(" "));
        parts.push(body);
    } else {
        parts.push(indent(concat([hard_line(), body])));
    }

    concat(parts)
}

pub(crate) fn do_statement(body: Doc, body_is_block: bool, condition: Doc) -> Doc {
    let mut parts = vec![text("do")];
    if body_is_block {
        parts.push(text(" "));
        parts.push(body);
        parts.push(text(" "));
    } else {
        parts.push(indent(concat([hard_line(), body])));
        parts.push(hard_line());
    }
    parts.push(text("while "));
    parts.push(parenthesized_expression(condition));
    parts.push(text(";"));

    concat(parts)
}

pub(crate) fn try_statement(
    body: Doc,
    catches: impl IntoIterator<Item = Doc>,
    finally_clause: Option<Doc>,
) -> Doc {
    let mut parts = vec![text("try "), body];
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

pub(crate) fn assignment_expression(left: Doc, operator: Doc, right: Doc) -> Doc {
    group(concat([
        left,
        text(" "),
        operator,
        continuation_indent(concat([line(), right])),
    ]))
}

pub(crate) fn binary_chain(first: Doc, rest: impl IntoIterator<Item = (Doc, Doc)>) -> Doc {
    let continuations = rest
        .into_iter()
        .map(|(operator, operand)| concat([line(), operator, text(" "), operand]));

    group(concat([first, continuation_indent(concat(continuations))]))
}

pub(crate) fn dot_chain(base: Doc, selectors: impl IntoIterator<Item = Doc>) -> Doc {
    let mut selectors = selectors.into_iter().collect::<Vec<_>>();
    if selectors.is_empty() {
        return base;
    }

    let first_selector = selectors.remove(0);
    let base = concat([base, text("."), first_selector]);
    if selectors.is_empty() {
        return base;
    }

    group(concat([
        base,
        continuation_indent(concat(
            selectors
                .into_iter()
                .map(|selector| concat([soft_line(), text("."), selector])),
        )),
    ]))
}
