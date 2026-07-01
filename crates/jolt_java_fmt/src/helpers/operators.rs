use jolt_fmt_ir::{Doc, concat, group, indent, line, text};

pub(crate) fn assignment_expression(left: Doc, operator: String, right: Doc) -> Doc {
    group(concat([
        left,
        text(" "),
        text(operator),
        indent(concat([line(), right])),
    ]))
}

pub(crate) fn binary_chain(first: Doc, rest: Vec<(String, Doc)>) -> Doc {
    if rest.is_empty() {
        return first;
    }

    group(concat([
        first,
        concat(
            rest.into_iter()
                .map(|(operator, operand)| concat([line(), text(operator), text(" "), operand])),
        ),
    ]))
}

pub(crate) fn ternary_expression(condition: Doc, consequence: Doc, alternative: Doc) -> Doc {
    group(concat([
        condition,
        line(),
        text("? "),
        consequence,
        line(),
        text(": "),
        alternative,
    ]))
}
