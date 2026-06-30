use jolt_java_syntax::{BinaryExpression, Expression, JavaSyntaxKind, JavaSyntaxToken};

#[derive(Clone, Debug)]
pub(crate) struct BinaryChain {
    precedence: u8,
    operands: Vec<Expression>,
    operators: Vec<JavaSyntaxToken>,
}

impl BinaryChain {
    pub(crate) fn for_expression(binary: &BinaryExpression) -> Self {
        let operator = binary
            .operator()
            .expect("parser-clean binary expression should have an operator");
        let precedence = precedence(operator.kind())
            .expect("parser-clean binary expression should have a binary operator");
        let left = binary
            .left()
            .expect("parser-clean binary expression should have a left side");
        let right = binary
            .right()
            .expect("parser-clean binary expression should have a right side");

        let mut operands = Vec::new();
        let mut operators = Vec::new();
        collect_left_chain(&left, precedence, &mut operands, &mut operators);
        operands.push(right);
        operators.push(operator);

        Self {
            precedence,
            operands,
            operators,
        }
    }

    pub(crate) const fn precedence(&self) -> u8 {
        self.precedence
    }

    pub(crate) fn operands(&self) -> &[Expression] {
        &self.operands
    }

    pub(crate) fn operators(&self) -> &[JavaSyntaxToken] {
        &self.operators
    }
}

#[derive(Clone, Copy)]
pub(crate) enum BinarySide {
    Left,
    Right,
}

pub(crate) fn operand_needs_parentheses(
    operand: &Expression,
    parent_precedence: u8,
    side: BinarySide,
) -> bool {
    let Expression::BinaryExpression(binary) = operand else {
        return false;
    };
    let operator = binary
        .operator()
        .expect("parser-clean binary expression should have an operator");
    let child_precedence = precedence(operator.kind())
        .expect("parser-clean binary expression should have a binary operator");
    child_precedence < parent_precedence
        || (child_precedence == parent_precedence && matches!(side, BinarySide::Right))
}

pub(crate) fn precedence(kind: JavaSyntaxKind) -> Option<u8> {
    match kind {
        JavaSyntaxKind::OrOr => Some(3),
        JavaSyntaxKind::AndAnd => Some(4),
        JavaSyntaxKind::Bar => Some(5),
        JavaSyntaxKind::Caret => Some(6),
        JavaSyntaxKind::Amp => Some(7),
        JavaSyntaxKind::EqEq | JavaSyntaxKind::BangEq => Some(8),
        JavaSyntaxKind::Lt | JavaSyntaxKind::Gt | JavaSyntaxKind::LtEq | JavaSyntaxKind::GtEq => {
            Some(9)
        }
        JavaSyntaxKind::LShift | JavaSyntaxKind::RShift | JavaSyntaxKind::UnsignedRShift => {
            Some(10)
        }
        JavaSyntaxKind::Plus | JavaSyntaxKind::Minus => Some(11),
        JavaSyntaxKind::Star | JavaSyntaxKind::Slash | JavaSyntaxKind::Percent => Some(12),
        _ => None,
    }
}

fn collect_left_chain(
    expression: &Expression,
    parent_precedence: u8,
    operands: &mut Vec<Expression>,
    operators: &mut Vec<JavaSyntaxToken>,
) {
    if let Expression::BinaryExpression(binary) = expression {
        let operator = binary
            .operator()
            .expect("parser-clean binary expression should have an operator");
        let child_precedence = precedence(operator.kind())
            .expect("parser-clean binary expression should have a binary operator");
        if child_precedence == parent_precedence {
            let left = binary
                .left()
                .expect("parser-clean binary expression should have a left side");
            let right = binary
                .right()
                .expect("parser-clean binary expression should have a right side");

            collect_left_chain(&left, parent_precedence, operands, operators);
            operands.push(right);
            operators.push(operator);
            return;
        }
    }

    operands.push(expression.clone());
}
