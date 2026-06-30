use jolt_fmt_ir::{
    Doc, FlatLine, break_, concat, fill, fill_entry, group, hard_line, indent_by, line, text,
};

use crate::analyzers::expressions::ExpressionLayout;
use crate::helpers::literals::TextBlockOpeningIndent;
use crate::policy::JavaFormatPolicy;

pub(crate) struct AssignmentOperator {
    doc: Doc,
    force_break_after: bool,
}

impl AssignmentOperator {
    pub(crate) fn new(doc: Doc) -> Self {
        Self {
            doc,
            force_break_after: false,
        }
    }

    pub(crate) fn with_forced_break_after(doc: Doc) -> Self {
        Self {
            doc,
            force_break_after: true,
        }
    }
}

pub(crate) fn assignment_expression(
    left: Doc,
    operator: AssignmentOperator,
    right: impl Into<AssignmentValue>,
    policy: JavaFormatPolicy,
) -> Doc {
    assignment_expression_with_indent(left, operator, right, policy.continuation_indent_levels())
}

pub(crate) fn simple_assignment_expression(
    left: Doc,
    operator: Doc,
    right: Doc,
    continuation_indent_levels: u16,
) -> Doc {
    assignment_expression_with_indent(
        left,
        AssignmentOperator::new(operator),
        right,
        continuation_indent_levels,
    )
}

pub(crate) struct AssignmentValue {
    doc: Doc,
    starts_absolute_text_block: bool,
}

impl AssignmentValue {
    pub(crate) fn new(doc: Doc) -> Self {
        Self {
            doc,
            starts_absolute_text_block: false,
        }
    }

    pub(crate) fn text_block(doc: Doc, opening_indent: TextBlockOpeningIndent) -> Self {
        Self {
            doc,
            starts_absolute_text_block: opening_indent == TextBlockOpeningIndent::Absolute,
        }
    }

    pub(crate) fn from_expression_layout(doc: Doc, layout: ExpressionLayout) -> Self {
        if let Some(opening_indent) = layout.leading_text_block_indent() {
            Self::text_block(doc, opening_indent)
        } else {
            Self::new(doc)
        }
    }
}

impl From<Doc> for AssignmentValue {
    fn from(doc: Doc) -> Self {
        Self::new(doc)
    }
}

fn assignment_expression_with_indent(
    left: Doc,
    operator: AssignmentOperator,
    right: impl Into<AssignmentValue>,
    continuation_indent_levels: u16,
) -> Doc {
    let right = right.into();
    let break_after_operator = if right.starts_absolute_text_block {
        absolute_line()
    } else if operator.force_break_after {
        hard_line()
    } else {
        line()
    };

    group(concat([
        left,
        text(" "),
        operator.doc,
        indent_by(
            continuation_indent_levels,
            concat([break_after_operator, right.doc]),
        ),
    ]))
}

pub(crate) struct BinaryOperand {
    doc: Doc,
    force_break_after: bool,
    is_text_block: bool,
    starts_absolute_text_block: bool,
}

impl BinaryOperand {
    pub(crate) fn new(doc: Doc) -> Self {
        Self {
            doc,
            force_break_after: false,
            is_text_block: false,
            starts_absolute_text_block: false,
        }
    }

    pub(crate) fn with_forced_break_after(doc: Doc) -> Self {
        Self {
            doc,
            force_break_after: true,
            is_text_block: false,
            starts_absolute_text_block: false,
        }
    }

    pub(crate) fn text_block(doc: Doc, opening_indent: TextBlockOpeningIndent) -> Self {
        Self {
            doc,
            force_break_after: false,
            is_text_block: true,
            starts_absolute_text_block: opening_indent == TextBlockOpeningIndent::Absolute,
        }
    }

    pub(crate) fn from_expression_layout(doc: Doc, layout: ExpressionLayout) -> Self {
        if let Some(opening_indent) = layout.leading_text_block_indent() {
            Self::text_block(doc, opening_indent)
        } else {
            Self::new(doc)
        }
    }
}

pub(crate) struct BinaryOperator {
    doc: Doc,
    force_break_after: bool,
}

impl BinaryOperator {
    pub(crate) fn new(doc: Doc) -> Self {
        Self {
            doc,
            force_break_after: false,
        }
    }

    pub(crate) fn with_forced_break_after(doc: Doc) -> Self {
        Self {
            doc,
            force_break_after: true,
        }
    }
}

pub(crate) fn binary_chain(
    first: BinaryOperand,
    rest: impl IntoIterator<Item = (BinaryOperator, BinaryOperand)>,
    policy: JavaFormatPolicy,
) -> Doc {
    let rest = rest.into_iter().collect::<Vec<_>>();
    let Some((first_operator, _)) = rest.first() else {
        return first.doc;
    };
    if first.is_text_block || rest.iter().any(|(_, operand)| operand.is_text_block) {
        return text_block_binary_chain(first, rest, policy);
    }

    let last = rest
        .last()
        .map(|(_, operand)| operand.doc.clone())
        .expect("non-empty operands checked above");
    let entries = std::iter::once(fill_entry(
        first.doc.clone(),
        binary_separator(
            &first,
            first_operator,
            rest.first().map(|(_, operand)| operand),
        ),
    ))
    .chain(rest.windows(2).map(|window| {
        let (_, operand) = &window[0];
        let (next_operator, _) = &window[1];
        fill_entry(
            operand.doc.clone(),
            binary_separator(operand, next_operator, Some(&window[1].1)),
        )
    }));

    group(indent_by(
        policy.continuation_indent_levels(),
        fill(entries, last),
    ))
}

pub(crate) fn lambda_body_binary_chain(
    first: BinaryOperand,
    rest: impl IntoIterator<Item = (BinaryOperator, BinaryOperand)>,
    policy: JavaFormatPolicy,
) -> Doc {
    let rest = rest.into_iter().collect::<Vec<_>>();
    if !policy.lambda_body_binary_chain_breaks_one_per_line() {
        return binary_chain(first, rest, policy);
    }

    let first_doc = first.doc.clone();
    let mut previous_operand = first;
    let mut tail = Vec::new();
    for (operator, operand) in rest {
        tail.push(concat([
            binary_separator(&previous_operand, &operator, Some(&operand)),
            operand.doc.clone(),
        ]));
        previous_operand = operand;
    }

    group(indent_by(
        policy.continuation_indent_levels(),
        concat(std::iter::once(first_doc).chain(tail)),
    ))
}

fn text_block_binary_chain(
    first: BinaryOperand,
    rest: Vec<(BinaryOperator, BinaryOperand)>,
    policy: JavaFormatPolicy,
) -> Doc {
    let mut previous_operand = first;
    let mut parts = vec![previous_operand.doc.clone()];
    for (operator, operand) in rest {
        parts.push(indent_by(
            policy.continuation_indent_levels(),
            concat([
                binary_separator(&previous_operand, &operator, Some(&operand)),
                operand.doc.clone(),
            ]),
        ));
        previous_operand = operand;
    }
    group(concat(parts))
}

fn binary_separator(
    operand: &BinaryOperand,
    operator: &BinaryOperator,
    next_operand: Option<&BinaryOperand>,
) -> Doc {
    let before_operator = if operand.force_break_after || operand.is_text_block {
        hard_line()
    } else {
        line()
    };
    let after_operator = if next_operand.is_some_and(|operand| operand.starts_absolute_text_block) {
        absolute_line()
    } else if operator.force_break_after {
        hard_line()
    } else {
        text(" ")
    };
    concat([before_operator, operator.doc.clone(), after_operator])
}

fn absolute_line() -> Doc {
    break_(FlatLine::Empty, i16::MIN)
}
