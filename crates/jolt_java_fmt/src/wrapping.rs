use jolt_fmt_ir::{Doc, concat, group, hard_line, indent, join, line, soft_line, text};

pub(crate) fn space_separated(parts: impl IntoIterator<Item = Doc>) -> Doc {
    group(join(line(), parts))
}

pub(crate) fn declaration_header(parts: impl IntoIterator<Item = Doc>) -> Doc {
    space_separated(parts)
}

pub(crate) fn parenthesized_comma_list(items: impl IntoIterator<Item = Doc>) -> Doc {
    group(concat([
        text("("),
        indent(concat([
            soft_line(),
            join(concat([text(","), line()]), items),
        ])),
        soft_line(),
        text(")"),
    ]))
}

pub(crate) fn comma_list(items: impl IntoIterator<Item = Doc>) -> Doc {
    group(join(concat([text(","), line()]), items))
}

pub(crate) fn variable_declaration(prefix: impl IntoIterator<Item = Doc>, declarators: Doc) -> Doc {
    group(concat([
        space_separated(prefix),
        text(" "),
        declarators,
        text(";"),
    ]))
}

pub(crate) fn variable_declarator(name: Doc, initializer: Option<Doc>) -> Doc {
    let Some(initializer) = initializer else {
        return name;
    };

    group(concat([
        name,
        text(" ="),
        indent(concat([line(), initializer])),
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

    group(concat([
        text(keyword),
        indent(concat([line(), expression])),
        text(";"),
    ]))
}

pub(crate) fn expression_statement(expression: Doc) -> Doc {
    group(concat([expression, text(";")]))
}

pub(crate) fn parenthesized_expression(expression: Doc) -> Doc {
    group(concat([
        text("("),
        indent(concat([soft_line(), expression])),
        soft_line(),
        text(")"),
    ]))
}

pub(crate) fn assignment_expression(left: Doc, operator: Doc, right: Doc) -> Doc {
    group(concat([
        left,
        text(" "),
        operator,
        indent(concat([line(), right])),
    ]))
}

pub(crate) fn binary_chain(first: Doc, rest: impl IntoIterator<Item = (Doc, Doc)>) -> Doc {
    let continuations = rest
        .into_iter()
        .map(|(operator, operand)| concat([line(), operator, text(" "), operand]));

    group(concat([first, indent(concat(continuations))]))
}

pub(crate) fn dot_chain(base: Doc, selectors: impl IntoIterator<Item = Doc>) -> Doc {
    let selectors = selectors.into_iter().collect::<Vec<_>>();
    if selectors.is_empty() {
        return base;
    }

    group(concat([
        base,
        indent(concat(
            selectors
                .into_iter()
                .map(|selector| concat([soft_line(), text("."), selector])),
        )),
    ]))
}
