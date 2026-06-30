use jolt_java_syntax::{Expression, JavaSyntaxKind, VariableInitializerValue};

use crate::helpers::literals::{self as java_literals, TextBlockOpeningIndent};
use crate::policy::JavaFormatPolicy;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ExpressionLayout {
    leading_text_block_indent: Option<TextBlockOpeningIndent>,
}

impl ExpressionLayout {
    pub(crate) fn for_expression(expression: &Expression, policy: JavaFormatPolicy) -> Self {
        Self {
            leading_text_block_indent: leading_text_block_indent(expression, policy),
        }
    }

    pub(crate) fn for_variable_initializer(
        value: &VariableInitializerValue,
        policy: JavaFormatPolicy,
    ) -> Self {
        match value {
            VariableInitializerValue::LiteralExpression(literal) => {
                Self::for_expression(&Expression::LiteralExpression(literal.clone()), policy)
            }
            VariableInitializerValue::BinaryExpression(binary) => {
                Self::for_expression(&Expression::BinaryExpression(binary.clone()), policy)
            }
            _ => Self::default(),
        }
    }

    pub(crate) const fn leading_text_block_indent(self) -> Option<TextBlockOpeningIndent> {
        self.leading_text_block_indent
    }
}

fn leading_text_block_indent(
    expression: &Expression,
    policy: JavaFormatPolicy,
) -> Option<TextBlockOpeningIndent> {
    if !policy.normalizes_text_block_indentation() {
        return None;
    }

    match expression {
        Expression::LiteralExpression(literal) => literal
            .token()
            .filter(|token| token.kind() == JavaSyntaxKind::TextBlockLiteral)
            .map(|token| java_literals::text_block_opening_indent(token.text())),
        Expression::BinaryExpression(binary) => {
            let left = binary
                .left()
                .expect("parser-clean binary expression should have a left side");
            leading_text_block_indent(&left, policy)
        }
        _ => None,
    }
}
