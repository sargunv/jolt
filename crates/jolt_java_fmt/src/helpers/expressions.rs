use jolt_fmt_ir::{
    Doc, FlatLine, break_, concat, fill, fill_entry, group, hard_line, indent_by, join, line, text,
};

use crate::analyzers::binary::{BinaryChain, BinarySide};
use crate::analyzers::expressions::ExpressionLayout;
use crate::diagnostics::FormatResult;
use crate::helpers::literals::TextBlockOpeningIndent;
use crate::policy::JavaFormatPolicy;
use jolt_diagnostics::TextRange;
use jolt_java_syntax::{BinaryExpression, Expression, JavaSyntaxToken};

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

pub(crate) fn assignment_operator_with_trailing_comments(
    operator: Doc,
    trailing_line: Vec<Doc>,
    trailing_block: Vec<Doc>,
) -> AssignmentOperator {
    if !trailing_line.is_empty() {
        return AssignmentOperator::with_forced_break_after(concat([
            operator,
            text(" "),
            join(hard_line(), trailing_line),
        ]));
    }

    if !trailing_block.is_empty() {
        return AssignmentOperator::with_forced_break_after(concat([
            operator,
            text(" "),
            join(text(" "), trailing_block),
        ]));
    }

    AssignmentOperator::new(operator)
}

pub(crate) fn assignment_expression(
    left: Doc,
    operator: AssignmentOperator,
    right: impl Into<AssignmentValue>,
    policy: JavaFormatPolicy,
) -> Doc {
    assignment_expression_with_indent(left, operator, right, policy.continuation_indent_levels())
}

pub(crate) fn assignment_expression_from_parts(
    left: Doc,
    operator: AssignmentOperator,
    right: Doc,
    right_layout: ExpressionLayout,
    leading_comments: Vec<Doc>,
    policy: JavaFormatPolicy,
) -> Doc {
    let has_leading_comments = !leading_comments.is_empty();
    let right = if has_leading_comments {
        concat([join(hard_line(), leading_comments), hard_line(), right])
    } else {
        right
    };
    let right = if has_leading_comments {
        AssignmentValue::new(right)
    } else {
        AssignmentValue::from_expression_layout(right, right_layout)
    };

    assignment_expression(left, operator, right, policy)
}

pub(crate) fn variable_declarator_block_initializer(name: Doc, initializer: Doc) -> Doc {
    concat([name, text(" = "), initializer])
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

pub(crate) fn conditional_expression(
    condition: Doc,
    true_expression: Doc,
    false_expression: Doc,
    policy: JavaFormatPolicy,
) -> Doc {
    group(indent_by(
        policy.continuation_indent_levels(),
        concat([
            condition,
            line(),
            text("? "),
            true_expression,
            line(),
            text(": "),
            false_expression,
        ]),
    ))
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

pub(crate) fn binary_operator_with_trailing_comments(
    operator: Doc,
    trailing_comments: Vec<Doc>,
) -> BinaryOperator {
    if trailing_comments.is_empty() {
        BinaryOperator::new(operator)
    } else {
        BinaryOperator::with_forced_break_after(concat([
            operator,
            text(" "),
            join(hard_line(), trailing_comments),
        ]))
    }
}

pub(crate) fn binary_operand_from_parts(
    operand: Doc,
    layout: ExpressionLayout,
    leading_comments: Vec<Doc>,
    trailing_comments: Vec<Doc>,
) -> BinaryOperand {
    let mut operand = operand;
    let has_leading_comments = !leading_comments.is_empty();
    let has_trailing_comments = !trailing_comments.is_empty();

    if has_trailing_comments {
        operand = concat([operand, text(" "), join(hard_line(), trailing_comments)]);
    }
    if has_leading_comments {
        operand = concat([join(hard_line(), leading_comments), hard_line(), operand]);
    }

    if has_trailing_comments {
        BinaryOperand::with_forced_break_after(operand)
    } else if has_leading_comments {
        BinaryOperand::new(operand)
    } else {
        BinaryOperand::from_expression_layout(operand, layout)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BinaryExpressionLayout {
    Default,
    LambdaBody,
}

pub(crate) struct BinaryOperandSlot<'a> {
    pub(crate) operand: &'a Expression,
    pub(crate) parent_precedence: u8,
    pub(crate) side: BinarySide,
    pub(crate) previous_range: TextRange,
    pub(crate) next_operator_range: Option<TextRange>,
}

pub(crate) struct BinaryOperatorSlot<'a> {
    pub(crate) operator: &'a JavaSyntaxToken,
    pub(crate) next_operand_range: TextRange,
}

pub(crate) trait BinaryExpressionFormatter {
    fn format_operand(&mut self, slot: BinaryOperandSlot<'_>) -> FormatResult<BinaryOperand>;

    fn format_operator(&mut self, slot: BinaryOperatorSlot<'_>) -> BinaryOperator;
}

pub(crate) fn binary_expression(
    binary: &BinaryExpression,
    layout: BinaryExpressionLayout,
    policy: JavaFormatPolicy,
    formatter: &mut impl BinaryExpressionFormatter,
) -> FormatResult<Doc> {
    let chain = BinaryChain::for_expression(binary);
    let precedence = chain.precedence();
    let operands = chain.operands();
    let operators = chain.operators();

    let first_operand = operands
        .first()
        .expect("parser-clean binary chain should have a first operand");
    let first_operator = operators
        .first()
        .expect("parser-clean binary chain should have an operator");
    let first = formatter.format_operand(BinaryOperandSlot {
        operand: first_operand,
        parent_precedence: precedence,
        side: BinarySide::Left,
        previous_range: first_operand
            .code_text_range()
            .expect("parser-clean binary operand should have a code range"),
        next_operator_range: Some(first_operator.token_text_range()),
    })?;

    let mut rest = Vec::new();
    for (index, operator) in operators.iter().enumerate() {
        let operand = operands
            .get(index + 1)
            .expect("binary operator should have a following operand");
        let operand_range = operand
            .code_text_range()
            .expect("parser-clean binary operand should have a code range");
        let next_operator_range = operators
            .get(index + 1)
            .map(JavaSyntaxToken::token_text_range);
        rest.push((
            formatter.format_operator(BinaryOperatorSlot {
                operator,
                next_operand_range: operand_range,
            }),
            formatter.format_operand(BinaryOperandSlot {
                operand,
                parent_precedence: precedence,
                side: BinarySide::Right,
                previous_range: operator.token_text_range(),
                next_operator_range,
            })?,
        ));
    }

    Ok(match layout {
        BinaryExpressionLayout::Default => binary_chain(first, rest, policy),
        BinaryExpressionLayout::LambdaBody => lambda_body_binary_chain(first, rest, policy),
    })
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
