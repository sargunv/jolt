use jolt_java_syntax::{Expression, JavaSyntaxKind, LiteralExpression};

/// Matches google-java-format `isFormatMethod`: the first argument is a string
/// concatenation (literals and `+` only) containing a format specifier (`%` or
/// `{0}`-style).
pub(crate) fn is_format_string_expression(expression: &Expression) -> bool {
    let mut only_string_concat = true;
    let mut has_format_specifier = false;
    visit_format_string_expression(
        expression,
        &mut only_string_concat,
        &mut has_format_specifier,
    );
    only_string_concat && has_format_specifier
}

fn visit_format_string_expression(
    expression: &Expression,
    only_string_concat: &mut bool,
    has_format_specifier: &mut bool,
) {
    if !*only_string_concat {
        return;
    }

    match expression {
        Expression::LiteralExpression(literal) => {
            if literal_has_format_specifier(literal) {
                *has_format_specifier = true;
            }
        }
        Expression::BinaryExpression(binary) => {
            let Some(operator) = binary.operator() else {
                *only_string_concat = false;
                return;
            };
            if operator.kind() != JavaSyntaxKind::Plus {
                *only_string_concat = false;
                return;
            }
            if let Some(left) = binary.left() {
                visit_format_string_expression(&left, only_string_concat, has_format_specifier);
            }
            if let Some(right) = binary.right() {
                visit_format_string_expression(&right, only_string_concat, has_format_specifier);
            }
        }
        Expression::ParenthesizedExpression(parenthesized) => {
            if let Some(inner) = parenthesized.expression() {
                visit_format_string_expression(&inner, only_string_concat, has_format_specifier);
            } else {
                *only_string_concat = false;
            }
        }
        _ => *only_string_concat = false,
    }
}

fn literal_has_format_specifier(literal: &LiteralExpression) -> bool {
    let Some(token) = literal.token() else {
        return false;
    };
    if token.kind() != JavaSyntaxKind::StringLiteral {
        return false;
    }

    contains_format_specifier(token.text())
}

fn contains_format_specifier(text: &str) -> bool {
    if text.contains('%') {
        return true;
    }

    text.as_bytes()
        .windows(2)
        .any(|window| window[0] == b'{' && window[1].is_ascii_digit())
}
