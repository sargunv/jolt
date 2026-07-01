use super::expressions::{PatternLayout, format_pattern_with_layout};
use super::types::format_type_layout_parts;
use super::{
    BasicForStatement, Block, BlockItem, BlockStatement, BreakStatement, CatchClause,
    CatchParameter, ConstructorBody, ConstructorInvocation, ContinueStatement, DoStatement, Doc,
    EmptyStatement, EnhancedForStatement, FinallyClause, ForInitializer, ForStatement, ForUpdate,
    FormatResult, IfStatement, JavaFormatContext, JavaSyntaxToken, LabeledStatement,
    LocalVariableDeclaration, Resource, ResourceSpecification, ReturnStatement, Statement,
    StatementExpressionList, SwitchBlock, SwitchBlockItem, SwitchBlockStatementGroup, SwitchLabel,
    SwitchLabelItem, SwitchRule, SwitchRuleBody, SwitchStatement, ThrowStatement, TryStatement,
    TryWithResourcesStatement, Type, TypeLayoutPart, VariableAccess, VariableDeclarator,
    VariableInitializerValue, WhileStatement, YieldStatement, concat, format_annotation_doc_list,
    format_argument_list, format_array_dimensions, format_expression, format_modifier_list,
    format_name, format_token, format_type, format_type_argument_list, format_type_declaration,
    format_variable_initializer_value, hard_line, join, reject_unhandled_comments_before_start,
    take_block_comment_docs_in_range_as_inline, take_inline_leading_block_comment_docs_in_range,
    take_leading_comment_docs, take_leading_comment_docs_in_range,
    take_own_line_comment_docs_in_range, take_same_line_trailing_block_comment_docs_in_range,
    take_trailing_line_comment_docs_in_range_as_own_line,
    take_trailing_line_comment_docs_in_range_as_suffix, text, with_leading_and_trailing_comments,
    wrap,
};
use crate::analyzers::expressions::ExpressionLayout;
use crate::helpers::annotations as java_annotations;
use crate::helpers::bodies::{
    self, BlockLayoutOptions, StatementBodyKind, statement_block,
    statement_block_with_opening_comments,
};
use crate::helpers::callables;
use crate::helpers::expressions as java_expressions;
use crate::helpers::lists as java_lists;
use crate::helpers::statements as java_statements;
use crate::helpers::switches as java_switches;
use jolt_diagnostics::TextRange;
use jolt_fmt_ir::{TextWidth, group, indent_by, line};

pub(super) fn format_block(
    block: &Block,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    format_block_with_options(block, context, BlockLayoutOptions::default())
}

pub(super) fn format_block_with_options(
    block: &Block,
    context: &mut JavaFormatContext<'_>,
    options: BlockLayoutOptions,
) -> FormatResult<Doc> {
    let code_range = block
        .code_text_range()
        .unwrap_or_else(|| block.text_range());
    format_block_statements(code_range, block.block_statements(), context, options)
}

pub(super) fn format_block_with_opening_comments(
    block: &Block,
    opening_comments: Vec<Doc>,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    format_block_with_options_and_opening_comments(
        block,
        opening_comments,
        context,
        BlockLayoutOptions::default(),
    )
}

pub(super) fn format_block_with_options_and_opening_comments(
    block: &Block,
    opening_comments: Vec<Doc>,
    context: &mut JavaFormatContext<'_>,
    options: BlockLayoutOptions,
) -> FormatResult<Doc> {
    let code_range = block
        .code_text_range()
        .unwrap_or_else(|| block.text_range());
    format_block_statements_with_opening_comments(
        code_range,
        block.block_statements(),
        opening_comments,
        context,
        options,
    )
}

pub(super) fn format_constructor_body_with_opening_comments(
    body: &ConstructorBody,
    opening_comments: Vec<Doc>,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = body.code_text_range().unwrap_or_else(|| body.text_range());

    let invocation = body
        .constructor_invocation()
        .map(|invocation| format_constructor_invocation(&invocation, context))
        .transpose()?;
    let statements = body
        .block_statements()
        .map(|statement| format_block_statement(&statement, context))
        .collect::<FormatResult<Vec<_>>>()?;

    bodies::constructor_body_with_opening_comments(
        code_range,
        invocation,
        statements,
        opening_comments,
        context,
    )
}

fn format_constructor_invocation(
    invocation: &ConstructorInvocation,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let qualifier = if let Some(expression) = invocation.qualifier_expression() {
        Some(format_expression(&expression, context)?)
    } else {
        invocation.qualifier_name().map(|name| format_name(&name))
    };
    let type_arguments = invocation
        .type_arguments()
        .map(|arguments| format_type_argument_list(&arguments, context))
        .transpose()?;
    let keyword = invocation
        .keyword()
        .map_or_else(|| text(""), |keyword| format_token(&keyword));
    let arguments = invocation
        .arguments()
        .map(|arguments| format_argument_list(&arguments, context))
        .transpose()?
        .unwrap_or_else(|| text("()"));

    let mut parts = Vec::new();
    if let Some(qualifier) = qualifier {
        parts.push(qualifier);
        parts.push(text("."));
    }
    parts.push(type_arguments.unwrap_or_else(|| text("")));
    parts.push(keyword);
    parts.push(arguments);
    parts.push(text(";"));

    Ok(concat(parts))
}

pub(super) fn format_block_statements(
    container_range: jolt_diagnostics::TextRange,
    statements: impl Iterator<Item = BlockStatement>,
    context: &mut JavaFormatContext<'_>,
    options: BlockLayoutOptions,
) -> FormatResult<Doc> {
    let statements = statements.collect::<Vec<_>>();
    statement_block(
        container_range,
        &statements,
        context,
        options,
        BlockStatement::code_text_range,
        format_block_statement,
    )
}

pub(super) fn format_block_statements_with_opening_comments(
    container_range: jolt_diagnostics::TextRange,
    statements: impl Iterator<Item = BlockStatement>,
    opening_comments: Vec<Doc>,
    context: &mut JavaFormatContext<'_>,
    options: BlockLayoutOptions,
) -> FormatResult<Doc> {
    let statements = statements.collect::<Vec<_>>();
    statement_block_with_opening_comments(
        container_range,
        &statements,
        opening_comments,
        context,
        options,
        BlockStatement::code_text_range,
        format_block_statement,
    )
}

pub(super) fn format_block_statement(
    statement: &BlockStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let item = statement
        .item()
        .expect("parser-clean block statement should have an item");
    let code_range = statement
        .code_text_range()
        .unwrap_or_else(|| statement.text_range());
    let leading_comments = take_leading_comment_docs(context, code_range)?;
    let doc = format_block_item(&item, context)?;
    with_leading_and_trailing_comments(context, code_range, leading_comments, doc)
}

pub(super) fn format_block_item(
    item: &BlockItem,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    match item {
        BlockItem::LocalVariableDeclaration(declaration) => {
            format_local_variable_declaration(declaration, context)
        }
        BlockItem::LocalClassOrInterfaceDeclaration(declaration) => {
            let type_declaration = declaration.declaration().expect(
                "parser-clean local class or interface declaration should have a declaration",
            );
            format_type_declaration(&type_declaration, context)
        }
        BlockItem::Block(block) => format_statement_rule(StatementRule::Block(block), context),
        BlockItem::EmptyStatement(empty) => {
            format_statement_rule(StatementRule::Empty(empty), context)
        }
        BlockItem::ExpressionStatement(expression) => {
            format_statement_rule(StatementRule::Expression(expression), context)
        }
        BlockItem::IfStatement(if_statement) => {
            format_statement_rule(StatementRule::If(if_statement), context)
        }
        BlockItem::BreakStatement(break_statement) => {
            format_statement_rule(StatementRule::Break(break_statement), context)
        }
        BlockItem::ContinueStatement(continue_statement) => {
            format_statement_rule(StatementRule::Continue(continue_statement), context)
        }
        BlockItem::ReturnStatement(return_statement) => {
            format_statement_rule(StatementRule::Return(return_statement), context)
        }
        BlockItem::ThrowStatement(throw_statement) => {
            format_statement_rule(StatementRule::Throw(throw_statement), context)
        }
        BlockItem::YieldStatement(yield_statement) => {
            format_statement_rule(StatementRule::Yield(yield_statement), context)
        }
        BlockItem::LabeledStatement(labeled) => {
            format_statement_rule(StatementRule::Labeled(labeled), context)
        }
        BlockItem::AssertStatement(assert_statement) => {
            format_statement_rule(StatementRule::Assert(assert_statement), context)
        }
        BlockItem::SwitchStatement(switch_statement) => {
            format_statement_rule(StatementRule::Switch(switch_statement), context)
        }
        BlockItem::WhileStatement(while_statement) => {
            format_statement_rule(StatementRule::While(while_statement), context)
        }
        BlockItem::DoStatement(do_statement) => {
            format_statement_rule(StatementRule::Do(do_statement), context)
        }
        BlockItem::ForStatement(for_statement) => {
            format_statement_rule(StatementRule::For(for_statement), context)
        }
        BlockItem::SynchronizedStatement(synchronized) => {
            format_statement_rule(StatementRule::Synchronized(synchronized), context)
        }
        BlockItem::TryStatement(try_statement) => {
            format_statement_rule(StatementRule::Try(try_statement), context)
        }
        BlockItem::TryWithResourcesStatement(try_statement) => {
            format_statement_rule(StatementRule::TryWithResources(try_statement), context)
        }
    }
}

pub(super) fn format_unbraced_statement(
    statement: &Statement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    format_unbraced_statement_with_block_options(statement, context, BlockLayoutOptions::default())
}

fn format_unbraced_statement_with_block_options(
    statement: &Statement,
    context: &mut JavaFormatContext<'_>,
    block_options: BlockLayoutOptions,
) -> FormatResult<Doc> {
    if let Statement::Block(block) = statement {
        return format_block_with_options(block, context, block_options);
    }
    if let Statement::IfStatement(if_statement) = statement {
        return format_if_statement_with_then_options(if_statement, context, Some(block_options));
    }

    format_statement_rule(statement_rule(statement)?, context)
}

fn statement_body_kind(statement: &Statement) -> StatementBodyKind {
    match statement {
        Statement::Block(_) => StatementBodyKind::Block,
        Statement::IfStatement(_) => StatementBodyKind::If,
        _ => StatementBodyKind::Other,
    }
}

fn statement_rule(statement: &Statement) -> FormatResult<StatementRule<'_>> {
    match statement {
        Statement::Block(block) => Ok(StatementRule::Block(block)),
        Statement::EmptyStatement(empty) => Ok(StatementRule::Empty(empty)),
        Statement::ExpressionStatement(expression) => Ok(StatementRule::Expression(expression)),
        Statement::IfStatement(if_statement) => Ok(StatementRule::If(if_statement)),
        Statement::BreakStatement(break_statement) => Ok(StatementRule::Break(break_statement)),
        Statement::ContinueStatement(continue_statement) => {
            Ok(StatementRule::Continue(continue_statement))
        }
        Statement::ReturnStatement(return_statement) => Ok(StatementRule::Return(return_statement)),
        Statement::ThrowStatement(throw_statement) => Ok(StatementRule::Throw(throw_statement)),
        Statement::YieldStatement(yield_statement) => Ok(StatementRule::Yield(yield_statement)),
        Statement::LabeledStatement(labeled) => Ok(StatementRule::Labeled(labeled)),
        Statement::AssertStatement(assert_statement) => Ok(StatementRule::Assert(assert_statement)),
        Statement::SwitchStatement(switch_statement) => Ok(StatementRule::Switch(switch_statement)),
        Statement::WhileStatement(while_statement) => Ok(StatementRule::While(while_statement)),
        Statement::DoStatement(do_statement) => Ok(StatementRule::Do(do_statement)),
        Statement::ForStatement(for_statement) => Ok(StatementRule::For(for_statement)),
        Statement::SynchronizedStatement(synchronized) => {
            Ok(StatementRule::Synchronized(synchronized))
        }
        Statement::TryStatement(try_statement) => Ok(StatementRule::Try(try_statement)),
        Statement::TryWithResourcesStatement(try_statement) => {
            Ok(StatementRule::TryWithResources(try_statement))
        }
    }
}

enum StatementRule<'a> {
    Block(&'a Block),
    Empty(&'a EmptyStatement),
    Expression(&'a jolt_java_syntax::ExpressionStatement),
    If(&'a IfStatement),
    While(&'a WhileStatement),
    Do(&'a DoStatement),
    For(&'a ForStatement),
    Synchronized(&'a jolt_java_syntax::SynchronizedStatement),
    Try(&'a TryStatement),
    TryWithResources(&'a TryWithResourcesStatement),
    Break(&'a BreakStatement),
    Continue(&'a ContinueStatement),
    Return(&'a ReturnStatement),
    Throw(&'a ThrowStatement),
    Yield(&'a YieldStatement),
    Labeled(&'a LabeledStatement),
    Assert(&'a jolt_java_syntax::AssertStatement),
    Switch(&'a SwitchStatement),
}

fn format_statement_rule(
    rule: StatementRule<'_>,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    match rule {
        StatementRule::Block(block) => format_block(block, context),
        StatementRule::Empty(empty) => format_empty_statement(empty),
        StatementRule::Expression(expression) => format_expression_statement(expression, context),
        StatementRule::If(if_statement) => format_if_statement(if_statement, context),
        StatementRule::While(while_statement) => format_while_statement(while_statement, context),
        StatementRule::Do(do_statement) => format_do_statement(do_statement, context),
        StatementRule::For(for_statement) => format_for_statement(for_statement, context),
        StatementRule::Synchronized(synchronized) => {
            format_synchronized_statement(synchronized, context)
        }
        StatementRule::Try(try_statement) => format_try_statement(try_statement, context),
        StatementRule::TryWithResources(try_statement) => {
            format_try_with_resources_statement(try_statement, context)
        }
        StatementRule::Break(break_statement) => format_break_statement(break_statement),
        StatementRule::Continue(continue_statement) => {
            format_continue_statement(continue_statement)
        }
        StatementRule::Return(return_statement) => {
            format_return_statement(return_statement, context)
        }
        StatementRule::Throw(throw_statement) => format_throw_statement(throw_statement, context),
        StatementRule::Yield(yield_statement) => format_yield_statement(yield_statement, context),
        StatementRule::Labeled(labeled) => format_labeled_statement(labeled, context),
        StatementRule::Assert(assert_statement) => {
            format_assert_statement(assert_statement, context)
        }
        StatementRule::Switch(switch_statement) => {
            format_switch_statement(switch_statement, context)
        }
    }
}

pub(super) fn format_empty_statement(statement: &EmptyStatement) -> FormatResult<Doc> {
    let _ = statement;
    Ok(text(";"))
}

pub(super) fn format_if_statement(
    statement: &IfStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    format_if_statement_with_then_options(statement, context, None)
}

fn format_if_statement_with_then_options(
    statement: &IfStatement,
    context: &mut JavaFormatContext<'_>,
    then_options: Option<BlockLayoutOptions>,
) -> FormatResult<Doc> {
    let condition = statement
        .condition()
        .expect("parser-clean if statement should have a condition");
    let then_statement = statement
        .then_statement()
        .expect("parser-clean if statement should have a then body");
    if let Some(then_range) = then_statement.code_text_range() {
        reject_unhandled_comments_before_start(
            context,
            then_range,
            "Java formatter does not support comments before if statement bodies yet",
        )?;
    }
    let then_statement = bodies::if_then_body(
        statement_body_kind(&then_statement),
        statement.else_statement().is_some(),
        then_options,
        |block_options| {
            format_unbraced_statement_with_block_options(&then_statement, context, block_options)
        },
    )?;
    let else_statement = statement
        .else_statement()
        .map(|else_statement| {
            if let Some(else_range) = else_statement.code_text_range() {
                reject_unhandled_comments_before_start(
                    context,
                    else_range,
                    "Java formatter does not support comments before else statement bodies yet",
                )?;
            }
            let else_body =
                bodies::if_else_body(statement_body_kind(&else_statement), |block_options| {
                    format_unbraced_statement_with_block_options(
                        &else_statement,
                        context,
                        block_options,
                    )
                })?;
            Ok((else_body.doc, else_body.follows_keyword))
        })
        .transpose()?;

    Ok(java_statements::if_statement(
        format_expression(&condition, context)?,
        then_statement.doc,
        then_statement.is_block,
        else_statement,
    ))
}

pub(super) fn format_while_statement(
    statement: &WhileStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let condition = statement
        .condition()
        .expect("parser-clean while statement should have a condition");
    let body = statement
        .body()
        .expect("parser-clean while statement should have a body");
    if let Some(body_range) = body.code_text_range() {
        reject_unhandled_comments_before_start(
            context,
            body_range,
            "Java formatter does not support comments before while statement bodies yet",
        )?;
    }
    let body = bodies::loop_body(statement_body_kind(&body), |block_options| {
        format_unbraced_statement_with_block_options(&body, context, block_options)
    })?;

    Ok(java_statements::while_statement(
        format_expression(&condition, context)?,
        body.doc,
        body.is_block,
    ))
}

pub(super) fn format_do_statement(
    statement: &DoStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let body = statement
        .body()
        .expect("parser-clean do statement should have a body");
    if let Some(body_range) = body.code_text_range() {
        reject_unhandled_comments_before_start(
            context,
            body_range,
            "Java formatter does not support comments before do statement bodies yet",
        )?;
    }
    let condition = statement
        .condition()
        .expect("parser-clean do statement should have a condition");
    let body = bodies::do_body(statement_body_kind(&body), |block_options| {
        format_unbraced_statement_with_block_options(&body, context, block_options)
    })?;

    Ok(java_statements::do_statement(
        body.doc,
        body.is_block,
        format_expression(&condition, context)?,
    ))
}

pub(super) fn format_for_statement(
    statement: &ForStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if let Some(basic) = statement.basic() {
        return format_basic_for_statement(&basic, context);
    }
    if let Some(enhanced) = statement.enhanced() {
        return format_enhanced_for_statement(&enhanced, context);
    }

    unreachable!("parser-clean for statement should be basic or enhanced")
}

pub(super) fn format_basic_for_statement(
    statement: &BasicForStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let initializer = statement
        .initializer()
        .map(|initializer| format_for_initializer(&initializer, context))
        .transpose()?;
    let condition = statement
        .condition()
        .map(|condition| format_expression(&condition, context))
        .transpose()?;
    let update = statement
        .update()
        .map(|update| format_for_update(&update, context))
        .transpose()?;
    let body = statement
        .body()
        .expect("parser-clean basic for statement should have a body");
    if let Some(body_range) = body.code_text_range() {
        reject_unhandled_comments_before_start(
            context,
            body_range,
            "Java formatter does not support comments before for statement bodies yet",
        )?;
    }
    let body = bodies::loop_body(statement_body_kind(&body), |block_options| {
        format_unbraced_statement_with_block_options(&body, context, block_options)
    })?;

    Ok(java_statements::for_statement(
        java_statements::basic_for_header(initializer, condition, update, context.policy()),
        body.doc,
        body.is_block,
    ))
}

pub(super) fn format_enhanced_for_statement(
    statement: &EnhancedForStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let variable = statement
        .variable()
        .expect("parser-clean enhanced for statement should have a variable");
    let iterable = statement
        .iterable()
        .expect("parser-clean enhanced for statement should have an iterable");
    let body = statement
        .body()
        .expect("parser-clean enhanced for statement should have a body");
    if let Some(body_range) = body.code_text_range() {
        reject_unhandled_comments_before_start(
            context,
            body_range,
            "Java formatter does not support comments before for statement bodies yet",
        )?;
    }
    let body = bodies::loop_body(statement_body_kind(&body), |block_options| {
        format_unbraced_statement_with_block_options(&body, context, block_options)
    })?;

    Ok(java_statements::for_statement(
        java_statements::enhanced_for_header(
            format_local_variable_declaration_header(&variable, context)?,
            format_expression(&iterable, context)?,
            context.policy(),
        ),
        body.doc,
        body.is_block,
    ))
}

pub(super) fn format_for_initializer(
    initializer: &ForInitializer,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if let Some(declaration) = initializer.local_variable_declaration() {
        return format_local_variable_declaration_header(&declaration, context);
    }
    let expressions = initializer
        .expressions()
        .expect("parser-clean for initializer should have expressions or declaration");
    format_statement_expression_list(&expressions, context)
}

pub(super) fn format_for_update(
    update: &ForUpdate,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let expressions = update
        .expressions()
        .expect("parser-clean for update should have expressions");
    format_statement_expression_list(&expressions, context)
}

pub(super) fn format_statement_expression_list(
    list: &StatementExpressionList,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let list_range = list
        .code_text_range()
        .expect("parser-clean statement expression list should have a code range");
    let expressions = list
        .expressions()
        .map(|expression| {
            let range = expression
                .code_text_range()
                .expect("parser-clean statement expression should have a code range");
            let expression = expression.clone();
            Ok(java_lists::ListItem::new(range, move |context| {
                format_expression(&expression, context)
            }))
        })
        .collect::<FormatResult<Vec<_>>>()?;
    java_lists::statement_expression_list(expressions, list_range, context)
}

pub(super) fn format_synchronized_statement(
    statement: &jolt_java_syntax::SynchronizedStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let expression = statement
        .expression()
        .expect("parser-clean synchronized statement should have an expression");
    let body = statement
        .body()
        .expect("parser-clean synchronized statement should have a body");
    if let Some(body_range) = body.code_text_range() {
        reject_unhandled_comments_before_start(
            context,
            body_range,
            "Java formatter does not support comments inside synchronized statement headers yet",
        )?;
    }

    Ok(concat([
        text("synchronized "),
        java_expressions::parenthesized_expression(
            format_expression(&expression, context)?,
            context.policy(),
        ),
        text(" "),
        format_block(&body, context)?,
    ]))
}

pub(super) fn format_try_statement(
    statement: &TryStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if let Some(try_with_resources) = statement.try_with_resources() {
        return format_try_with_resources_statement(&try_with_resources, context);
    }

    let body = statement
        .body()
        .expect("parser-clean try statement should have a body");
    if let Some(body_range) = body.code_text_range() {
        reject_unhandled_comments_before_start(
            context,
            body_range,
            "Java formatter does not support comments before try statement bodies yet",
        )?;
    }

    let catches: Vec<_> = statement.catches().collect();
    let has_finally = statement.finally_clause().is_some();
    let body_options = bodies::try_body_options(!catches.is_empty() || has_finally);

    let catch_count = catches.len();
    let catches = catches
        .iter()
        .enumerate()
        .map(|(index, catch)| {
            let has_trailing_clause = has_finally || index + 1 < catch_count;
            let body_options = bodies::catch_body_options(has_trailing_clause);
            format_catch_clause(catch, context, body_options)
        })
        .collect::<FormatResult<Vec<_>>>()?;
    let finally_clause = statement
        .finally_clause()
        .map(|finally_clause| format_finally_clause(&finally_clause, context))
        .transpose()?;

    Ok(java_statements::try_statement(
        format_block_with_options(&body, context, body_options)?,
        catches,
        finally_clause,
    ))
}

pub(super) fn format_try_with_resources_statement(
    statement: &TryWithResourcesStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let resources = statement
        .resources()
        .expect("parser-clean try-with-resources statement should have resources");
    let body = statement
        .body()
        .expect("parser-clean try-with-resources statement should have a body");
    let resource_specification = format_resource_specification(&resources, context)?;
    if let Some(body_range) = body.code_text_range() {
        reject_unhandled_comments_before_start(
            context,
            body_range,
            "Java formatter does not support comments before try statement bodies yet",
        )?;
    }

    let catches: Vec<_> = statement.catches().collect();
    let has_finally = statement.finally_clause().is_some();
    let body_options = bodies::try_body_options(!catches.is_empty() || has_finally);

    let catch_count = catches.len();
    let catches = catches
        .iter()
        .enumerate()
        .map(|(index, catch)| {
            let has_trailing_clause = has_finally || index + 1 < catch_count;
            let body_options = bodies::catch_body_options(has_trailing_clause);
            format_catch_clause(catch, context, body_options)
        })
        .collect::<FormatResult<Vec<_>>>()?;
    let finally_clause = statement
        .finally_clause()
        .map(|finally_clause| format_finally_clause(&finally_clause, context))
        .transpose()?;

    Ok(java_statements::try_statement_with_header(
        concat([
            text("try "),
            resource_specification,
            text(" "),
            format_block_with_options(&body, context, body_options)?,
        ]),
        catches,
        finally_clause,
    ))
}

fn format_resource_specification(
    specification: &ResourceSpecification,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let resources = specification
        .resources()
        .expect("parser-clean resource specification should have a resource list");

    let resources = resources
        .resources()
        .map(|resource| {
            let range = resource
                .code_text_range()
                .unwrap_or_else(|| resource.text_range());
            java_lists::ListItem::new(range, move |context| format_resource(&resource, context))
        })
        .collect::<Vec<_>>();

    java_lists::resource_specification(
        resources,
        specification.text_range(),
        specification.has_trailing_semicolon(),
        context,
    )
}

fn format_resource(resource: &Resource, context: &mut JavaFormatContext<'_>) -> FormatResult<Doc> {
    if let Some(declaration) = resource.local_variable_declaration() {
        return format_local_variable_declaration_header_with_options(
            &declaration,
            context,
            LocalVariableDeclarationHeaderOptions {
                indent_vertical_annotations: true,
            },
        );
    }

    let access = resource
        .variable_access()
        .expect("parser-clean resource should be a declaration or variable access");
    format_variable_access(&access, context)
}

fn format_variable_access(
    access: &VariableAccess,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let expression = access
        .expression()
        .expect("parser-clean variable access should have an expression");
    format_expression(&expression, context)
}

pub(super) fn format_catch_clause(
    clause: &CatchClause,
    context: &mut JavaFormatContext<'_>,
    body_options: BlockLayoutOptions,
) -> FormatResult<Doc> {
    let parameter = clause
        .parameter()
        .expect("parser-clean catch clause should have a parameter");
    if let Some(parameter_range) = parameter.code_text_range() {
        reject_unhandled_comments_before_start(
            context,
            parameter_range,
            "Java formatter does not support comments inside catch statement headers yet",
        )?;
    }
    let body = clause
        .body()
        .expect("parser-clean catch clause should have a body");
    if let Some(body_range) = body.code_text_range() {
        reject_unhandled_comments_before_start(
            context,
            body_range,
            "Java formatter does not support comments inside catch statement headers yet",
        )?;
    }

    Ok(concat([
        text("catch "),
        format_catch_parameter_header(&parameter, context)?,
        text(" "),
        format_block_with_options(&body, context, body_options)?,
    ]))
}

fn format_catch_parameter_header(
    parameter: &CatchParameter,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let parameter = format_catch_parameter(parameter, context)?;
    if parameter.is_union {
        Ok(concat([
            text("("),
            indent_by(context.policy().continuation_indent_levels(), parameter.doc),
            text(")"),
        ]))
    } else {
        Ok(java_expressions::parenthesized_expression(
            parameter.doc,
            context.policy(),
        ))
    }
}

struct CatchParameterDoc {
    doc: Doc,
    is_union: bool,
}

fn format_catch_parameter(
    parameter: &CatchParameter,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<CatchParameterDoc> {
    let ty = parameter
        .ty()
        .expect("parser-clean catch parameter should have a type");
    let name = parameter
        .name()
        .expect("parser-clean catch parameter should have a name");

    let annotations = format_annotation_doc_list(parameter.annotations(), context, "declaration")?;
    let final_ranges = parameter
        .final_token()
        .into_iter()
        .map(|token| token.token_text_range());
    let split =
        java_annotations::split_type_bearing_declaration_annotations(annotations, final_ranges);
    let mut parts = split
        .declaration_annotations
        .into_iter()
        .map(java_annotations::AnnotationDoc::into_doc)
        .collect::<Vec<_>>();
    if let Some(final_token) = parameter.final_token() {
        parts.push(format_token(&final_token));
    }
    let is_union = is_catch_union_type_list(&ty);
    let doc = if is_union {
        format_catch_union_parameter(
            parts,
            split.type_use_annotations,
            &ty,
            format_token(&name),
            context,
        )?
    } else {
        parts.push(java_annotations::type_use_prefix(
            split.type_use_annotations,
            format_catch_type_list(&ty, context)?,
        ));
        parts.push(format_token(&name));
        wrap::space_separated(parts)
    };
    Ok(CatchParameterDoc { doc, is_union })
}

fn is_catch_union_type_list(types: &jolt_java_syntax::CatchTypeList) -> bool {
    catch_union_type_alternatives(types).is_some()
}

fn format_catch_union_parameter(
    mut prefix_parts: Vec<Doc>,
    type_use_annotations: Vec<java_annotations::AnnotationDoc>,
    types: &jolt_java_syntax::CatchTypeList,
    name: Doc,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let mut types = catch_union_type_alternatives(types)
        .expect("parser-clean catch union parameter should have union alternatives")
        .into_iter()
        .map(|parts| format_type_layout_parts(&parts, context))
        .collect::<FormatResult<Vec<_>>>()?;
    let last = types
        .pop()
        .expect("parser-clean catch union parameter should have at least one type");
    let mut alternatives = Vec::new();
    let first = types
        .first()
        .cloned()
        .expect("union parameter should keep at least one type before the last alternative");
    let first = java_annotations::type_use_prefix(type_use_annotations, first);
    if prefix_parts.is_empty() {
        alternatives.push(first);
    } else {
        prefix_parts.push(first);
        alternatives.push(wrap::space_separated(prefix_parts));
    }
    for ty in types.into_iter().skip(1) {
        alternatives.push(concat([text("| "), ty]));
    }
    alternatives.push(concat([text("| "), wrap::space_separated([last, name])]));
    Ok(group(join(line(), alternatives)))
}

fn catch_union_type_alternatives(
    types: &jolt_java_syntax::CatchTypeList,
) -> Option<Vec<Vec<TypeLayoutPart>>> {
    let ty = types.ty()?;
    if !matches!(ty, Type::UnionType(_)) {
        return None;
    }

    let mut alternatives = Vec::new();
    let mut current = Vec::new();
    for part in ty.layout_parts() {
        match &part {
            TypeLayoutPart::Token(token)
                if token.kind() == jolt_java_syntax::JavaSyntaxKind::Bar =>
            {
                trim_trailing_space_parts(&mut current);
                alternatives.push(std::mem::take(&mut current));
            }
            TypeLayoutPart::Text(value) if current.is_empty() && value.trim().is_empty() => {}
            _ => current.push(part),
        }
    }
    trim_trailing_space_parts(&mut current);
    alternatives.push(current);

    (alternatives.len() > 1).then_some(alternatives)
}

fn trim_trailing_space_parts(parts: &mut Vec<TypeLayoutPart>) {
    while matches!(parts.last(), Some(TypeLayoutPart::Text(value)) if value.trim().is_empty()) {
        parts.pop();
    }
}

fn format_catch_type_list(
    types: &jolt_java_syntax::CatchTypeList,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let types = types
        .types()
        .map(|ty| format_type(&ty, context))
        .collect::<FormatResult<Vec<_>>>()?;

    Ok(wrap::space_separated(
        types
            .into_iter()
            .enumerate()
            .flat_map(|(index, ty)| {
                if index == 0 {
                    vec![ty]
                } else {
                    vec![text("|"), ty]
                }
            })
            .collect::<Vec<_>>(),
    ))
}

pub(super) fn format_finally_clause(
    clause: &FinallyClause,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let body = clause
        .body()
        .expect("parser-clean finally clause should have a body");
    if let Some(body_range) = body.code_text_range() {
        reject_unhandled_comments_before_start(
            context,
            body_range,
            "Java formatter does not support comments before finally clause bodies yet",
        )?;
    }

    Ok(concat([
        text("finally "),
        format_block_with_options(&body, context, bodies::finally_body_options())?,
    ]))
}

pub(super) fn format_break_statement(statement: &BreakStatement) -> FormatResult<Doc> {
    Ok(java_statements::keyword_label_statement(
        "break",
        statement.label().map(|label| format_token(&label)),
    ))
}

pub(super) fn format_continue_statement(statement: &ContinueStatement) -> FormatResult<Doc> {
    Ok(java_statements::keyword_label_statement(
        "continue",
        statement.label().map(|label| format_token(&label)),
    ))
}

pub(super) fn format_labeled_statement(
    statement: &LabeledStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let label = statement
        .label()
        .expect("parser-clean labeled statement should have a label");
    let body = statement
        .statement()
        .expect("parser-clean labeled statement should have a body");
    if let Some(body_range) = body.code_text_range() {
        reject_unhandled_comments_before_start(
            context,
            body_range,
            "Java formatter does not support comments before labeled statement bodies yet",
        )?;
    }

    Ok(concat([
        format_token(&label),
        text(":"),
        hard_line(),
        format_unbraced_statement(&body, context)?,
    ]))
}

pub(super) fn format_local_variable_declaration(
    declaration: &LocalVariableDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    Ok(concat([
        format_local_variable_declaration_header(declaration, context)?,
        text(";"),
    ]))
}

pub(super) fn format_local_variable_declaration_header(
    declaration: &LocalVariableDeclaration,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    format_local_variable_declaration_header_with_options(
        declaration,
        context,
        LocalVariableDeclarationHeaderOptions::default(),
    )
}

#[derive(Default)]
struct LocalVariableDeclarationHeaderOptions {
    indent_vertical_annotations: bool,
}

fn format_local_variable_declaration_header_with_options(
    declaration: &LocalVariableDeclaration,
    context: &mut JavaFormatContext<'_>,
    options: LocalVariableDeclarationHeaderOptions,
) -> FormatResult<Doc> {
    let ty_source_width = declaration.ty().map(|ty| {
        let ty_range = ty
            .code_text_range()
            .expect("parser-clean local variable type should have a code range");
        text_range_width(ty_range)
    });
    let mut ty = if let Some(ty) = declaration.ty() {
        format_type(&ty, context)?
    } else {
        let token = declaration
            .var_type_token()
            .expect("parser-clean local variable declaration should have a type");
        format_token(&token)
    };
    let declarators = declaration
        .declarators()
        .expect("parser-clean local variable declaration should have declarators");
    let first_declarator_name_width = declarators
        .declarators()
        .next()
        .and_then(|declarator| variable_declarator_name_source_width(&declarator))
        .expect("parser-clean local variable declaration should have a declarator name");
    let declarators = format_variable_declarator_list(&declarators, "local variable", context)?;

    let mut modifiers = format_modifier_list(declaration.modifiers(), "local variable", context)?;
    let mut annotations =
        format_annotation_doc_list(declaration.annotations(), context, "declaration")?;
    annotations.extend(modifiers.annotations);
    let split_ranges = modifiers
        .modifier_tokens
        .iter()
        .map(JavaSyntaxToken::token_text_range)
        .chain(
            declaration
                .final_token()
                .into_iter()
                .map(|token| token.token_text_range()),
        );
    let split =
        java_annotations::split_type_bearing_declaration_annotations(annotations, split_ranges);
    modifiers.annotations = split.declaration_annotations;
    ty = java_annotations::type_use_prefix(split.type_use_annotations, ty);
    let mut prefix = modifiers
        .modifier_tokens
        .iter()
        .map(format_token)
        .collect::<Vec<_>>();
    if modifiers.modifier_tokens.is_empty()
        && let Some(final_token) = declaration.final_token()
    {
        prefix.push(format_token(&final_token));
    }
    prefix.push(ty);

    let leading_type_policy = ty_source_width.map(|width| {
        let rendered_declaration_head_source_width =
            local_declaration_head_source_width(declaration, width, first_declarator_name_width);
        callables::DeclarationLeadingTypePolicy {
            has_type_arguments: declaration
                .ty()
                .is_some_and(|ty| type_contains_type_arguments(&ty)),
            rendered_leading_type_source_width: width,
            rendered_declaration_head_source_width,
        }
    });

    let declaration = callables::variable_declaration_header(
        prefix,
        declarators,
        leading_type_policy,
        context.policy(),
    );
    let layout = java_annotations::local_annotation_layout(&modifiers.annotations);
    if options.indent_vertical_annotations {
        let leading_comments = modifiers.leading_comments;
        let doc = java_annotations::with_resource_declaration_annotations(
            modifiers.annotations,
            declaration,
            layout,
            context.policy().continuation_indent_levels(),
        );
        return Ok(if leading_comments.is_empty() {
            doc
        } else {
            concat([join(hard_line(), leading_comments), hard_line(), doc])
        });
    }
    Ok(modifiers.with_annotations_layout(declaration, layout))
}

fn type_contains_type_arguments(ty: &Type) -> bool {
    ty.layout_parts()
        .iter()
        .any(|part| matches!(part, TypeLayoutPart::Token(token) if token.text() == "<"))
}

fn text_range_width(range: TextRange) -> usize {
    range.end().get().saturating_sub(range.start().get())
}

fn local_declaration_head_source_width(
    declaration: &LocalVariableDeclaration,
    leading_type_width: usize,
    name_width: usize,
) -> usize {
    let modifier_width = local_modifier_source_width(declaration);
    let mut width = leading_type_width + 1 + name_width;
    if modifier_width > 0 {
        width += modifier_width + 1;
    }
    width
}

fn local_modifier_source_width(declaration: &LocalVariableDeclaration) -> usize {
    let mut modifier_tokens = declaration
        .modifiers()
        .map(|modifiers| modifiers.modifier_tokens().collect::<Vec<_>>())
        .unwrap_or_default();
    modifier_tokens.extend(declaration.final_token());
    if modifier_tokens.is_empty() {
        return 0;
    }

    modifier_tokens
        .iter()
        .map(|token| token.text().len())
        .sum::<usize>()
        + modifier_tokens.len()
        - 1
}

fn variable_declarator_name_source_width(declarator: &VariableDeclarator) -> Option<usize> {
    let name = declarator.name()?;
    let mut width = name.text().len();
    if let Some(dimensions) = declarator.dimensions()
        && let Some(range) = dimensions.code_text_range()
    {
        width += text_range_width(range);
    }
    Some(width)
}

pub(super) fn format_variable_declarator_list(
    declarators: &jolt_java_syntax::VariableDeclaratorList,
    declaration_kind: &str,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let declarator_docs = declarators
        .declarators()
        .map(|declarator| format_variable_declarator(&declarator, context))
        .collect::<FormatResult<Vec<_>>>()?;

    if declarator_docs.is_empty() {
        let _ = declaration_kind;
        unreachable!("parser-clean variable declarator list should not be empty");
    }

    Ok(wrap::comma_list(declarator_docs))
}

pub(super) fn format_variable_declarator(
    declarator: &VariableDeclarator,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let name = declarator
        .name()
        .expect("parser-clean variable declarator should have a name");
    let name = if let Some(dimensions) = declarator.dimensions() {
        concat([
            format_token(&name),
            format_array_dimensions(&dimensions, context)?,
        ])
    } else {
        format_token(&name)
    };
    let Some(initializer) = declarator.initializer() else {
        return Ok(name);
    };
    let value = initializer
        .value()
        .expect("parser-clean variable initializer should have a value");
    let value_range = value
        .code_text_range()
        .expect("parser-clean variable initializer value should have a code range");
    let mut leading_comments = Vec::new();
    let mut assignment_operator = java_expressions::AssignmentOperator::new(text("="));
    if let Some(assign) = declarator.assign() {
        let assign_range = assign.token_text_range();
        let owner_range = TextRange::new(assign_range.end(), value_range.start());
        let trailing_line = take_trailing_line_comment_docs_in_range_as_own_line(
            context,
            assign_range,
            owner_range,
        );
        if trailing_line.is_empty() {
            let trailing_block = take_same_line_trailing_block_comment_docs_in_range(
                context,
                assign_range,
                owner_range,
            );
            if !trailing_block.is_empty() {
                assignment_operator =
                    java_expressions::AssignmentOperator::with_forced_break_after(concat([
                        format_token(&assign),
                        text(" "),
                        join(text(" "), trailing_block),
                    ]));
            }
        } else {
            assignment_operator =
                java_expressions::AssignmentOperator::with_forced_break_after(concat([
                    format_token(&assign),
                    text(" "),
                    join(hard_line(), trailing_line),
                ]));
        }
        leading_comments.extend(take_leading_comment_docs_in_range(
            context,
            owner_range,
            value_range,
        )?);
        leading_comments.extend(take_inline_leading_block_comment_docs_in_range(
            context,
            owner_range,
            value_range,
        ));
        leading_comments.extend(take_block_comment_docs_in_range_as_inline(
            context,
            owner_range,
        ));
    }
    let initializer_layout = ExpressionLayout::for_variable_initializer(&value, context.policy());
    let mut initializer = format_variable_initializer_value(&value, context)?;
    let has_leading_comments = !leading_comments.is_empty();
    if has_leading_comments {
        initializer = concat([
            join(hard_line(), leading_comments),
            hard_line(),
            initializer,
        ]);
    }
    if matches!(value, VariableInitializerValue::ArrayInitializer(_)) {
        return Ok(java_expressions::variable_declarator_block_initializer(
            name,
            initializer,
        ));
    }
    let initializer = if has_leading_comments {
        java_expressions::AssignmentValue::new(initializer)
    } else {
        java_expressions::AssignmentValue::from_expression_layout(initializer, initializer_layout)
    };

    Ok(java_expressions::assignment_expression(
        name,
        assignment_operator,
        initializer,
        context.policy(),
    ))
}

pub(super) fn format_return_statement(
    statement: &ReturnStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let expression = statement
        .expression()
        .map(|expression| format_expression(&expression, context))
        .transpose()?;
    Ok(java_statements::keyword_expression_statement(
        "return", expression,
    ))
}

pub(super) fn format_throw_statement(
    statement: &ThrowStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let expression = statement
        .expression()
        .expect("parser-clean throw statement should have an expression");
    Ok(java_statements::keyword_expression_statement(
        "throw",
        Some(format_expression(&expression, context)?),
    ))
}

pub(super) fn format_yield_statement(
    statement: &YieldStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let expression = statement
        .expression()
        .expect("parser-clean yield statement should have an expression");
    Ok(java_statements::keyword_expression_statement(
        "yield",
        Some(format_expression(&expression, context)?),
    ))
}

pub(super) fn format_assert_statement(
    statement: &jolt_java_syntax::AssertStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let expressions = statement.expressions().collect::<Vec<_>>();
    match expressions.as_slice() {
        [condition] => Ok(java_statements::keyword_expression_statement(
            "assert",
            Some(format_expression(condition, context)?),
        )),
        [condition, detail] => Ok(concat([
            text("assert "),
            format_expression(condition, context)?,
            text(" : "),
            format_expression(detail, context)?,
            text(";"),
        ])),
        _ => unreachable!("parser-clean assert statement should have one or two expressions"),
    }
}

pub(super) fn format_switch_statement(
    statement: &SwitchStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let selector = statement
        .selector()
        .expect("parser-clean switch statement should have a selector");
    let block = statement
        .block()
        .expect("parser-clean switch statement should have a block");

    Ok(java_switches::switch_construct(
        format_expression(&selector, context)?,
        format_switch_block(&block, context)?,
        context.policy(),
    ))
}

pub(super) fn format_switch_expression(
    switch_expression: &jolt_java_syntax::SwitchExpression,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let selector = switch_expression
        .selector()
        .expect("parser-clean switch expression should have a selector");
    let block = switch_expression
        .block()
        .expect("parser-clean switch expression should have a block");

    Ok(java_switches::switch_construct(
        format_expression(&selector, context)?,
        format_switch_block(&block, context)?,
        context.policy(),
    ))
}

pub(super) fn format_switch_block(
    block: &SwitchBlock,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let items = block.items().collect::<Vec<_>>();
    let ranges = items
        .iter()
        .map(switch_block_item_range)
        .collect::<Vec<_>>();
    let separators = ranges
        .windows(2)
        .map(|window| {
            let [Some(left), Some(right)] = window else {
                return Ok(hard_line());
            };
            let boundary = TextRange::new(left.end(), right.start());
            let comments = take_leading_comment_docs_in_range(context, boundary, *right)?;
            Ok(java_switches::switch_block_item_separator(comments))
        })
        .collect::<FormatResult<Vec<_>>>()?;

    let docs = items
        .iter()
        .map(|item| format_switch_block_item(item, context))
        .collect::<FormatResult<Vec<_>>>()?;

    let leading = if let (Some(first_range), Some(block_range)) =
        (ranges.first().copied().flatten(), block.code_text_range())
    {
        take_leading_comment_docs_in_range(
            context,
            TextRange::new(block_range.start(), first_range.start()),
            first_range,
        )?
    } else {
        Vec::new()
    };
    let trailing = if let (Some(last_range), Some(block_range)) =
        (ranges.last().copied().flatten(), block.code_text_range())
    {
        take_own_line_comment_docs_in_range(
            context,
            TextRange::new(last_range.end(), block_range.end()),
        )?
    } else {
        Vec::new()
    };

    Ok(java_switches::switch_block(
        docs, separators, leading, trailing,
    ))
}

fn switch_block_item_range(item: &SwitchBlockItem) -> Option<TextRange> {
    match item {
        SwitchBlockItem::StatementGroup(group) => group.code_text_range(),
        SwitchBlockItem::Rule(rule) => rule.code_text_range(),
        SwitchBlockItem::BlockStatement(statement) => statement.code_text_range(),
    }
}

pub(super) fn format_switch_block_item(
    item: &SwitchBlockItem,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    match item {
        SwitchBlockItem::StatementGroup(group) => format_switch_statement_group(group, context),
        SwitchBlockItem::Rule(rule) => format_switch_rule(rule, context),
        SwitchBlockItem::BlockStatement(statement) => format_block_statement(statement, context),
    }
}

pub(super) fn format_switch_statement_group(
    group: &SwitchBlockStatementGroup,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = group
        .code_text_range()
        .unwrap_or_else(|| group.text_range());
    let leading_comments = take_leading_comment_docs(context, code_range)?;

    let labels = group
        .labels()
        .zip(group.colons())
        .map(|(label, colon)| {
            let colon_range = colon.token_text_range();
            Ok(concat([
                format_switch_label(&label, context)?,
                with_leading_and_trailing_comments(context, colon_range, Vec::new(), text(":"))?,
            ]))
        })
        .collect::<FormatResult<Vec<_>>>()?;
    let statement_nodes = group.block_statements().collect::<Vec<_>>();
    let mut body_comments = Vec::new();
    if let (Some(colon), Some(first_statement)) = (group.colons().last(), statement_nodes.first())
        && let Some(statement_range) = first_statement.code_text_range()
    {
        let colon_range = colon.token_text_range();
        let owner_range = TextRange::new(colon_range.end(), statement_range.start());
        body_comments.extend(take_leading_comment_docs_in_range(
            context,
            owner_range,
            statement_range,
        )?);
    }
    let statements = statement_nodes
        .iter()
        .map(|statement| format_block_statement(statement, context))
        .collect::<FormatResult<Vec<_>>>()?;

    let doc = java_switches::switch_statement_group(labels, body_comments, statements);
    with_leading_and_trailing_comments(context, code_range, leading_comments, doc)
}

pub(super) fn format_switch_rule(
    rule: &SwitchRule,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = rule.code_text_range().unwrap_or_else(|| rule.text_range());
    let leading_comments = take_leading_comment_docs(context, code_range)?;
    let label = rule
        .label()
        .expect("parser-clean switch rule should have a label");
    let body = rule
        .body()
        .expect("parser-clean switch rule should have a body");
    let arrow = rule
        .arrow()
        .expect("parser-clean switch rule should have an arrow");
    let arrow_range = arrow.token_text_range();
    let body_range = switch_rule_body_range(&body)
        .expect("parser-clean switch rule body should have a code range");
    let body_is_block = matches!(&body, SwitchRuleBody::Block(_));
    let arrow_to_body_range = TextRange::new(arrow_range.end(), body_range.start());
    let arrow_trailing_comments = take_trailing_line_comment_docs_in_range_as_suffix(
        context,
        arrow_range,
        arrow_to_body_range,
    );
    let arrow_has_trailing_comment = !arrow_trailing_comments.is_empty();
    let arrow = concat([text(" ->"), concat(arrow_trailing_comments)]);
    let body_comments =
        take_leading_comment_docs_in_range(context, arrow_to_body_range, body_range)?;

    let body = match body {
        SwitchRuleBody::Block(block) => format_block(&block, context)?,
        SwitchRuleBody::Expression(expression) => {
            java_statements::expression_statement(format_expression(&expression, context)?)
        }
        SwitchRuleBody::Throw(statement) => format_throw_statement(&statement, context)?,
    };
    let label = format_switch_label(&label, context)?;
    let doc = java_switches::switch_rule(
        label,
        arrow,
        body,
        body_comments,
        body_is_block,
        arrow_has_trailing_comment,
        context.policy(),
    );
    with_leading_and_trailing_comments(context, code_range, leading_comments, doc)
}

fn switch_rule_body_range(body: &SwitchRuleBody) -> Option<TextRange> {
    match body {
        SwitchRuleBody::Block(block) => block.code_text_range(),
        SwitchRuleBody::Expression(expression) => expression.code_text_range(),
        SwitchRuleBody::Throw(statement) => statement.code_text_range(),
    }
}

pub(super) fn format_switch_label(
    label: &SwitchLabel,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if label.has_default_only_layout_shape() {
        return Ok(text("default"));
    }

    let items = label
        .items()
        .map(|item| format_switch_label_item(item, context))
        .collect::<FormatResult<Vec<_>>>()?;

    Ok(java_switches::case_label(items, context.policy()))
}

fn format_switch_label_item(
    item: SwitchLabelItem,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    match item {
        SwitchLabelItem::Constant(constant) => {
            let expression = constant
                .expression()
                .expect("parser-clean case constant should have an expression");
            format_expression(&expression, context)
        }
        SwitchLabelItem::Pattern(pattern, guard) => {
            let base = pattern
                .pattern()
                .map(|pattern| {
                    format_pattern_with_layout(&pattern, context, PatternLayout::SwitchLabel)
                })
                .transpose()?
                .expect("parser-clean case pattern should have a pattern");
            let Some(guard) = guard.and_then(|guard| guard.expression()) else {
                return Ok(base);
            };
            Ok(java_switches::guarded_pattern(
                base,
                format_expression(&guard, context)?,
                context.policy(),
            ))
        }
        SwitchLabelItem::Default(_) => Ok(text("default")),
    }
}

pub(super) fn format_expression_statement(
    statement: &jolt_java_syntax::ExpressionStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let expression = statement
        .expression()
        .expect("parser-clean expression statement should have an expression");

    let expression_doc = format_expression(&expression, context)?;
    if matches!(
        &expression,
        jolt_java_syntax::Expression::MethodInvocationExpression(invocation)
            if invocation.receiver().is_none()
    ) {
        return Ok(
            java_statements::expression_statement_with_trailing_fit_width(
                expression_doc,
                TextWidth::new(1),
            ),
        );
    }

    Ok(java_statements::expression_statement(expression_doc))
}
