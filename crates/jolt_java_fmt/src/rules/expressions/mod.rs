//! Area C, expressions: every rule entry point of the area. Unported kinds call
//! the temporary `placeholder` (grep `placeholder(`).

mod binary;
mod calls;
mod leaves;

use jolt_fmt::Doc;
use jolt_java_syntax::{
    ArrayAccessExpression, ArrayCreationExpression, ArrayInitializer, AssignmentExpression,
    CastExpression, ClassLiteralExpression, ConditionalExpression, DimExpression,
    FieldAccessExpression, InstanceofExpression, LambdaExpression, LambdaParameter,
    MethodReferenceExpression, ObjectCreationExpression, ParenthesizedExpression,
    PostfixExpression, SuperExpression, ThisExpression, UnaryExpression,
};
use jolt_syntax::AstNode;

use crate::JavaFormatter;
use crate::helpers::placeholder::placeholder;

pub(crate) use binary::binary_expression;
pub(crate) use calls::{
    argument_list, method_invocation_expression, qualified_method_invocation,
    unqualified_method_invocation,
};
pub(crate) use leaves::{literal_expression, name_expression};

pub(crate) fn this_expression<'a>(f: &mut JavaFormatter<'a>, node: ThisExpression<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn super_expression<'a>(f: &mut JavaFormatter<'a>, node: SuperExpression<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn parenthesized_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: ParenthesizedExpression<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn class_literal_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: ClassLiteralExpression<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn field_access_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: FieldAccessExpression<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn array_access_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: ArrayAccessExpression<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn method_reference_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: MethodReferenceExpression<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn object_creation_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: ObjectCreationExpression<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn array_creation_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: ArrayCreationExpression<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn dim_expression<'a>(f: &mut JavaFormatter<'a>, node: DimExpression<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn array_initializer<'a>(f: &mut JavaFormatter<'a>, node: ArrayInitializer<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn assignment_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: AssignmentExpression<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn conditional_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: ConditionalExpression<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn instanceof_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: InstanceofExpression<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn unary_expression<'a>(f: &mut JavaFormatter<'a>, node: UnaryExpression<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn postfix_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: PostfixExpression<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn cast_expression<'a>(f: &mut JavaFormatter<'a>, node: CastExpression<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn lambda_expression<'a>(f: &mut JavaFormatter<'a>, node: LambdaExpression<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn lambda_parameter<'a>(f: &mut JavaFormatter<'a>, node: LambdaParameter<'a>) -> Doc {
    placeholder(f, node.syntax())
}
