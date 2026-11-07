//! Area B, statements: every rule entry point of the area. Unported kinds call
//! the temporary `placeholder` (grep `placeholder(`).

mod blocks;

use jolt_fmt::Doc;
use jolt_java_syntax::{
    AssertStatement, BasicForStatement, BreakStatement, CaseConstant, CasePattern, CatchClause,
    CatchParameter, CatchTypeList, ContinueStatement, DoStatement, EmptyStatement,
    EnhancedForStatement, EnhancedForVariable, FinallyClause, ForInitializer, ForStatement,
    ForUpdate, Guard, IfStatement, LabeledStatement, LocalClassOrInterfaceDeclaration, Resource,
    ResourceSpecification, ResourceVariableDeclaration, SwitchBlock, SwitchBlockStatementGroup,
    SwitchExpression, SwitchLabel, SwitchRule, SwitchStatement, SynchronizedStatement,
    ThrowStatement, TryStatement, TryWithResourcesStatement, VariableAccess, WhileStatement,
    YieldStatement,
};
use jolt_syntax::AstNode;

use crate::JavaFormatter;
use crate::helpers::placeholder::placeholder;

pub(crate) use blocks::{
    block, block_statement, expression_statement, local_variable_declaration, return_statement,
};

pub(crate) fn local_class_or_interface_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: LocalClassOrInterfaceDeclaration<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn empty_statement<'a>(f: &mut JavaFormatter<'a>, node: EmptyStatement<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn labeled_statement<'a>(f: &mut JavaFormatter<'a>, node: LabeledStatement<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn if_statement<'a>(f: &mut JavaFormatter<'a>, node: IfStatement<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn assert_statement<'a>(f: &mut JavaFormatter<'a>, node: AssertStatement<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn while_statement<'a>(f: &mut JavaFormatter<'a>, node: WhileStatement<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn do_statement<'a>(f: &mut JavaFormatter<'a>, node: DoStatement<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn for_statement<'a>(f: &mut JavaFormatter<'a>, node: ForStatement<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn basic_for_statement<'a>(
    f: &mut JavaFormatter<'a>,
    node: BasicForStatement<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn enhanced_for_statement<'a>(
    f: &mut JavaFormatter<'a>,
    node: EnhancedForStatement<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn enhanced_for_variable<'a>(
    f: &mut JavaFormatter<'a>,
    node: EnhancedForVariable<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn for_initializer<'a>(f: &mut JavaFormatter<'a>, node: ForInitializer<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn for_update<'a>(f: &mut JavaFormatter<'a>, node: ForUpdate<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn break_statement<'a>(f: &mut JavaFormatter<'a>, node: BreakStatement<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn yield_statement<'a>(f: &mut JavaFormatter<'a>, node: YieldStatement<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn continue_statement<'a>(
    f: &mut JavaFormatter<'a>,
    node: ContinueStatement<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn throw_statement<'a>(f: &mut JavaFormatter<'a>, node: ThrowStatement<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn synchronized_statement<'a>(
    f: &mut JavaFormatter<'a>,
    node: SynchronizedStatement<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn switch_statement<'a>(f: &mut JavaFormatter<'a>, node: SwitchStatement<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn switch_expression<'a>(f: &mut JavaFormatter<'a>, node: SwitchExpression<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn switch_block<'a>(f: &mut JavaFormatter<'a>, node: SwitchBlock<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn switch_block_statement_group<'a>(
    f: &mut JavaFormatter<'a>,
    node: SwitchBlockStatementGroup<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn switch_rule<'a>(f: &mut JavaFormatter<'a>, node: SwitchRule<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn switch_label<'a>(f: &mut JavaFormatter<'a>, node: SwitchLabel<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn case_constant<'a>(f: &mut JavaFormatter<'a>, node: CaseConstant<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn case_pattern<'a>(f: &mut JavaFormatter<'a>, node: CasePattern<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn guard<'a>(f: &mut JavaFormatter<'a>, node: Guard<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn try_statement<'a>(f: &mut JavaFormatter<'a>, node: TryStatement<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn try_with_resources_statement<'a>(
    f: &mut JavaFormatter<'a>,
    node: TryWithResourcesStatement<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn catch_clause<'a>(f: &mut JavaFormatter<'a>, node: CatchClause<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn catch_parameter<'a>(f: &mut JavaFormatter<'a>, node: CatchParameter<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn catch_type_list<'a>(f: &mut JavaFormatter<'a>, node: CatchTypeList<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn finally_clause<'a>(f: &mut JavaFormatter<'a>, node: FinallyClause<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn resource_specification<'a>(
    f: &mut JavaFormatter<'a>,
    node: ResourceSpecification<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn resource<'a>(f: &mut JavaFormatter<'a>, node: Resource<'a>) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn resource_variable_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: ResourceVariableDeclaration<'a>,
) -> Doc {
    placeholder(f, node.syntax())
}

pub(crate) fn variable_access<'a>(f: &mut JavaFormatter<'a>, node: VariableAccess<'a>) -> Doc {
    placeholder(f, node.syntax())
}
