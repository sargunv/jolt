use jolt_fmt_ir::{Doc, concat, group, indent, line, text};

pub(crate) fn assignment_expression(left: Doc, operator: Doc, right: Doc) -> Doc {
    group(concat([left, text(" "), operator, assignment_rhs(right)]))
}

pub(crate) fn assignment_rhs(right: Doc) -> Doc {
    indent(concat([line(), right]))
}

pub(crate) fn binary_chain(first: Doc, rest: Vec<(Doc, Doc)>) -> Doc {
    if rest.is_empty() {
        return first;
    }

    group(concat([
        first,
        indent(concat(rest.into_iter().map(|(operator, operand)| {
            concat([line(), operator, text(" "), operand])
        }))),
    ]))
}

pub(crate) fn ternary_expression(
    condition: Doc,
    question: Doc,
    consequence: Doc,
    colon: Doc,
    alternative: Doc,
) -> Doc {
    group(concat([
        condition,
        indent(concat([
            line(),
            question,
            text(" "),
            consequence,
            line(),
            colon,
            text(" "),
            alternative,
        ])),
    ]))
}
