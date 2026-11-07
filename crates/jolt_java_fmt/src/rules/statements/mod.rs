//! Area B, statements: blocks, control flow with brace promotion (N4), jump
//! statements, switches, and try statements.

mod blocks;
mod control;
mod jumps;
mod switches;
mod tries;

use jolt_fmt::ListItem;
use jolt_java_syntax::{BlockItem, BlockStatementList};
use jolt_syntax::{AstNode, ListElement, SyntaxNode};

use crate::helpers::bodies;

pub(crate) use blocks::{
    block, block_statement, empty_statement, expression_statement, labeled_statement,
    local_class_or_interface_declaration, local_variable_declaration,
};
pub(crate) use control::promoted_body_comment;
pub(crate) use control::{
    basic_for_statement, do_statement, enhanced_for_statement, enhanced_for_variable,
    for_initializer, for_statement, for_update, if_statement, synchronized_statement,
    while_statement,
};
pub(crate) use jumps::{
    assert_statement, break_statement, continue_statement, return_statement, throw_statement,
    yield_statement,
};
pub(crate) use switches::{
    case_constant, case_pattern, guard, switch_block, switch_block_statement_group,
    switch_expression, switch_label, switch_rule, switch_statement,
};
pub(crate) use tries::{
    catch_clause, catch_parameter, catch_type_list, finally_clause, resource,
    resource_specification, resource_variable_declaration, try_statement,
    try_with_resources_statement, variable_access,
};

pub(crate) use crate::helpers::lists::parenthesized;

/// The statements of a block or switch group as item-list items: every
/// child of the list, including bogus ones, with empty statements elided
/// (N5) when `owner` is recovery-free.
fn statement_items<'a>(
    owner: SyntaxNode<'a>,
    list: Option<BlockStatementList<'a>>,
) -> Vec<ListItem<'a>> {
    let Some(list) = list else {
        return Vec::new();
    };
    let normalize = owner.is_recovery_free();
    list.elements()
        .filter_map(|element| match element {
            ListElement::Item(statement)
                if normalize && matches!(statement.item(), Some(BlockItem::EmptyStatement(_))) =>
            {
                Some(ListItem::Elide(statement.syntax()))
            }
            ListElement::Item(statement) => Some(ListItem::Format(statement.syntax())),
            ListElement::Bogus(bogus) => Some(ListItem::Format(bogus)),
            ListElement::Separator(_) | ListElement::Missing => None,
        })
        .collect()
}

use bodies::statement_separator;
