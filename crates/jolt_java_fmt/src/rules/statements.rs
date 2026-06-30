use super::{
    BasicForStatement, Block, BlockItem, BlockStatement, BreakStatement, CatchClause,
    CatchParameter, ConstructorBody, ConstructorInvocation, ContinueStatement, DoStatement, Doc,
    EmptyStatement, EnhancedForStatement, FinallyClause, ForInitializer, ForStatement, ForUpdate,
    FormatResult, IfStatement, JavaFormatContext, LabeledStatement, LocalVariableDeclaration,
    Resource, ResourceSpecification, ReturnStatement, Statement, StatementExpressionList,
    SwitchBlock, SwitchBlockItem, SwitchBlockStatementGroup, SwitchLabel, SwitchLabelItem,
    SwitchRule, SwitchRuleBody, SwitchStatement, ThrowStatement, TryStatement,
    TryWithResourcesStatement, VariableAccess, VariableDeclarator, VariableInitializerValue,
    WhileStatement, YieldStatement, concat, format_annotation_list, format_argument_list,
    format_array_dimensions, format_expression, format_modifier_list, format_name, format_pattern,
    format_token, format_type, format_type_argument_list, format_type_declaration,
    format_variable_initializer_value, hard_line, join, reject_unhandled_comments_before_start,
    take_dangling_comment_docs, take_leading_comment_docs, text,
    with_leading_and_trailing_comments, with_vertical_annotations, wrap,
};

pub(super) fn format_block(
    block: &Block,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = block
        .code_text_range()
        .unwrap_or_else(|| block.text_range());
    format_block_statements(code_range, block.block_statements(), context)
}

pub(super) fn format_constructor_body(
    body: &ConstructorBody,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let code_range = body.code_text_range().unwrap_or_else(|| body.text_range());

    let mut statements = Vec::new();
    if let Some(invocation) = body.constructor_invocation() {
        statements.push(format_constructor_invocation(&invocation, context)?);
    }
    statements.extend(
        body.block_statements()
            .map(|statement| format_block_statement(&statement, context))
            .collect::<FormatResult<Vec<_>>>()?,
    );

    if statements.is_empty() {
        return Ok(wrap::braced_block(take_dangling_comment_docs(
            context, code_range,
        )?));
    }

    Ok(wrap::braced_block(statements))
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
) -> FormatResult<Doc> {
    let statements = statements.collect::<Vec<_>>();
    if statements.is_empty() {
        return Ok(wrap::braced_block(take_dangling_comment_docs(
            context,
            container_range,
        )?));
    }

    let separators = statements
        .windows(2)
        .map(|window| {
            let left = window[0].code_text_range();
            let right = window[1].code_text_range();
            if let (Some(left), Some(right)) = (left, right)
                && context.has_blank_line_between(left, right)
            {
                return jolt_fmt_ir::empty_line();
            }
            hard_line()
        })
        .collect::<Vec<_>>();

    let statements = statements
        .iter()
        .map(|statement| format_block_statement(statement, context))
        .collect::<FormatResult<Vec<_>>>()?;

    Ok(wrap::braced_block_with_separators(statements, separators))
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
    format_statement_rule(statement_rule(statement)?, context)
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
    let then_is_block = matches!(then_statement, Statement::Block(_));
    let then_statement = format_unbraced_statement(&then_statement, context)?;
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
            let follows_keyword = matches!(
                else_statement,
                Statement::Block(_) | Statement::IfStatement(_)
            );
            Ok((
                format_unbraced_statement(&else_statement, context)?,
                follows_keyword,
            ))
        })
        .transpose()?;

    Ok(wrap::if_statement(
        format_expression(&condition, context)?,
        then_statement,
        then_is_block,
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
    let body_is_block = matches!(body, Statement::Block(_));
    let body = format_unbraced_statement(&body, context)?;

    Ok(wrap::while_statement(
        format_expression(&condition, context)?,
        body,
        body_is_block,
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
    let body_is_block = matches!(body, Statement::Block(_));
    let body = format_unbraced_statement(&body, context)?;

    Ok(wrap::do_statement(
        body,
        body_is_block,
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
    let body_is_block = matches!(body, Statement::Block(_));
    let body = format_unbraced_statement(&body, context)?;

    Ok(wrap::for_statement(
        format_basic_for_header(initializer, condition, update),
        body,
        body_is_block,
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
    let body_is_block = matches!(body, Statement::Block(_));
    let body = format_unbraced_statement(&body, context)?;

    Ok(wrap::for_statement(
        concat([
            text("for ("),
            format_local_variable_declaration_header(&variable, context)?,
            text(" : "),
            format_expression(&iterable, context)?,
            text(")"),
        ]),
        body,
        body_is_block,
    ))
}

pub(super) fn format_basic_for_header(
    initializer: Option<Doc>,
    condition: Option<Doc>,
    update: Option<Doc>,
) -> Doc {
    let mut parts = vec![text("for (")];
    if let Some(initializer) = initializer {
        parts.push(initializer);
    }
    parts.push(text(";"));
    if let Some(condition) = condition {
        parts.push(text(" "));
        parts.push(condition);
    }
    parts.push(text(";"));
    if let Some(update) = update {
        parts.push(text(" "));
        parts.push(update);
    }
    parts.push(text(")"));
    concat(parts)
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
    let expressions = list
        .expressions()
        .map(|expression| format_expression(&expression, context))
        .collect::<FormatResult<Vec<_>>>()?;
    Ok(wrap::comma_list(expressions))
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
        wrap::parenthesized_expression(format_expression(&expression, context)?),
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

    let catches = statement
        .catches()
        .map(|catch| format_catch_clause(&catch, context))
        .collect::<FormatResult<Vec<_>>>()?;
    let finally_clause = statement
        .finally_clause()
        .map(|finally_clause| format_finally_clause(&finally_clause, context))
        .transpose()?;

    Ok(wrap::try_statement(
        format_block(&body, context)?,
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
    if let Some(body_range) = body.code_text_range() {
        reject_unhandled_comments_before_start(
            context,
            body_range,
            "Java formatter does not support comments before try statement bodies yet",
        )?;
    }

    let catches = statement
        .catches()
        .map(|catch| format_catch_clause(&catch, context))
        .collect::<FormatResult<Vec<_>>>()?;
    let finally_clause = statement
        .finally_clause()
        .map(|finally_clause| format_finally_clause(&finally_clause, context))
        .transpose()?;

    Ok(wrap::try_statement_with_header(
        concat([
            text("try "),
            format_resource_specification(&resources, context)?,
            text(" "),
            format_block(&body, context)?,
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
        .map(|resource| format_resource(&resource, context))
        .collect::<FormatResult<Vec<_>>>()?;

    Ok(wrap::parenthesized_semicolon_list(resources))
}

fn format_resource(resource: &Resource, context: &mut JavaFormatContext<'_>) -> FormatResult<Doc> {
    if let Some(declaration) = resource.local_variable_declaration() {
        return format_local_variable_declaration_header(&declaration, context);
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
        wrap::parenthesized_expression(format_catch_parameter(&parameter, context)?),
        text(" "),
        format_block(&body, context)?,
    ]))
}

pub(super) fn format_catch_parameter(
    parameter: &CatchParameter,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let ty = parameter
        .ty()
        .expect("parser-clean catch parameter should have a type");
    let name = parameter
        .name()
        .expect("parser-clean catch parameter should have a name");

    let mut parts = format_annotation_list(parameter.annotations(), context, "declaration")?;
    if let Some(final_token) = parameter.final_token() {
        parts.push(format_token(&final_token));
    }
    parts.push(format_catch_type_list(&ty, context)?);
    parts.push(format_token(&name));
    Ok(wrap::space_separated(parts))
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

    Ok(concat([text("finally "), format_block(&body, context)?]))
}

pub(super) fn format_break_statement(statement: &BreakStatement) -> FormatResult<Doc> {
    Ok(wrap::keyword_label_statement(
        "break",
        statement.label().map(|label| format_token(&label)),
    ))
}

pub(super) fn format_continue_statement(statement: &ContinueStatement) -> FormatResult<Doc> {
    Ok(wrap::keyword_label_statement(
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
    let ty = if let Some(ty) = declaration.ty() {
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
    let declarators = format_variable_declarator_list(&declarators, "local variable", context)?;

    let modifiers = format_modifier_list(declaration.modifiers(), "local variable", context)?;
    let direct_annotations =
        format_annotation_list(declaration.annotations(), context, "declaration")?;
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

    let declaration = wrap::variable_declaration_header(prefix, declarators);
    Ok(with_vertical_annotations(
        direct_annotations,
        modifiers.with_annotations(declaration),
    ))
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
        return Ok(wrap::variable_declarator(name, None));
    };
    let value = initializer
        .value()
        .expect("parser-clean variable initializer should have a value");
    let initializer = format_variable_initializer_value(&value, context)?;
    if matches!(value, VariableInitializerValue::ArrayInitializer(_)) {
        return Ok(wrap::variable_declarator_block_initializer(
            name,
            initializer,
        ));
    }

    Ok(wrap::variable_declarator(name, Some(initializer)))
}

pub(super) fn format_return_statement(
    statement: &ReturnStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let expression = statement
        .expression()
        .map(|expression| format_expression(&expression, context))
        .transpose()?;
    Ok(wrap::keyword_expression_statement("return", expression))
}

pub(super) fn format_throw_statement(
    statement: &ThrowStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let expression = statement
        .expression()
        .expect("parser-clean throw statement should have an expression");
    Ok(wrap::keyword_expression_statement(
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
    Ok(wrap::keyword_expression_statement(
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
        [condition] => Ok(wrap::keyword_expression_statement(
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

    Ok(concat([
        text("switch "),
        wrap::parenthesized_expression(format_expression(&selector, context)?),
        text(" "),
        format_switch_block(&block, context)?,
    ]))
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

    Ok(concat([
        text("switch "),
        wrap::parenthesized_expression(format_expression(&selector, context)?),
        text(" "),
        format_switch_block(&block, context)?,
    ]))
}

pub(super) fn format_switch_block(
    block: &SwitchBlock,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let items = block
        .items()
        .map(|item| format_switch_block_item(&item, context))
        .collect::<FormatResult<Vec<_>>>()?;
    Ok(wrap::braced_block(items))
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
        .map(|label| Ok(concat([format_switch_label(&label, context)?, text(":")])))
        .collect::<FormatResult<Vec<_>>>()?;
    let statements = group
        .block_statements()
        .map(|statement| format_block_statement(&statement, context))
        .collect::<FormatResult<Vec<_>>>()?;

    let doc = if statements.is_empty() {
        join(hard_line(), labels)
    } else {
        concat([
            join(hard_line(), labels),
            jolt_fmt_ir::indent(concat([hard_line(), join(hard_line(), statements)])),
        ])
    };
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

    let body = match body {
        SwitchRuleBody::Block(block) => format_block(&block, context)?,
        SwitchRuleBody::Expression(expression) => {
            wrap::expression_statement(format_expression(&expression, context)?)
        }
        SwitchRuleBody::Throw(statement) => format_throw_statement(&statement, context)?,
    };
    let doc = concat([format_switch_label(&label, context)?, text(" -> "), body]);
    with_leading_and_trailing_comments(context, code_range, leading_comments, doc)
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

    Ok(concat([text("case "), wrap::comma_list(items)]))
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
                .map(|pattern| format_pattern(&pattern, context))
                .transpose()?
                .expect("parser-clean case pattern should have a pattern");
            let Some(guard) = guard.and_then(|guard| guard.expression()) else {
                return Ok(base);
            };
            Ok(concat([
                base,
                text(" when "),
                format_expression(&guard, context)?,
            ]))
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

    Ok(wrap::expression_statement(format_expression(
        &expression,
        context,
    )?))
}
