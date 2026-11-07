//! Area C, expressions: every `Expression` member except `SwitchExpression`,
//! plus lambda parameters, array initializers, dimension expressions and
//! argument lists.
//!
//! Two spines are walked iteratively because the parser builds them without
//! a depth limit: the receiver spine (`spine.rs`: member chains, field and
//! array access, postfix operators, method references, class literals and
//! the qualified forms of `new`, `this` and `super`) and the left operand
//! spine of binary and `instanceof` chains (`binary.rs`).

mod binary;
mod calls;
mod creation;
mod lambdas;
mod leaves;
mod operators;
mod spine;

pub(crate) use binary::{binary_expression, instanceof_expression};
pub(crate) use calls::{argument_list, qualified_method_invocation, unqualified_method_invocation};
pub(crate) use creation::{array_creation_expression, array_initializer, dim_expression};
pub(crate) use lambdas::{lambda_expression, lambda_parameter};
pub(crate) use leaves::{literal_expression, name_expression};
pub(crate) use operators::{
    assignment_expression, cast_expression, conditional_expression, parenthesized_expression,
    unary_expression,
};
pub(crate) use spine::{
    array_access_expression, class_literal_expression, field_access_expression,
    method_invocation_expression, method_reference_expression, object_creation_expression,
    postfix_expression, super_expression, this_expression,
};
