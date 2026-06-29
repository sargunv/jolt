use super::{
    BasicForStatement, Block, BlockItem, BlockStatement, BreakStatement, CatchClause,
    CatchParameter, ConstructorBody, ContinueStatement, DoStatement, Doc, EmptyStatement,
    EnhancedForStatement, FinallyClause, ForInitializer, ForStatement, ForUpdate, FormatResult,
    IfStatement, JavaFormatContext, LabeledStatement, LocalVariableDeclaration, ReturnStatement,
    Statement, StatementExpressionList, SwitchBlock, SwitchBlockItem, SwitchBlockStatementGroup,
    SwitchLabel, SwitchRule, SwitchRuleBody, SwitchStatement, ThrowStatement, TryStatement,
    VariableDeclarator, WhileStatement, YieldStatement, concat, format_expression, format_token,
    format_type, hard_line, join, missing_layout, reject_unhandled_comments_before_start,
    take_dangling_comment_docs, take_leading_comment_docs, text,
    with_leading_and_trailing_comments, wrap,
};

pub(super) fn format_block(
    block: &Block,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !block.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this block shape yet",
            block.text_range(),
        ));
    }
    let code_range = block
        .code_text_range()
        .ok_or_else(|| missing_layout("Java formatter found an empty block", block.text_range()))?;
    format_block_statements(code_range, block.block_statements(), context)
}

pub(super) fn format_constructor_body(
    body: &ConstructorBody,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !body.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support constructor invocations or this constructor body shape yet",
            body.text_range(),
        ));
    }
    let code_range = body.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty constructor body",
            body.text_range(),
        )
    })?;
    format_block_statements(code_range, body.block_statements(), context)
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

    let statements = statements
        .into_iter()
        .map(|statement| format_block_statement(&statement, context))
        .collect::<FormatResult<Vec<_>>>()?;

    Ok(wrap::braced_block(statements))
}

pub(super) fn format_block_statement(
    statement: &BlockStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this block statement shape yet",
            statement.text_range(),
        ));
    }

    let item = statement.item().ok_or_else(|| {
        missing_layout(
            "Java formatter found a block statement without an item",
            statement.text_range(),
        )
    })?;
    let code_range = statement.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty block statement",
            statement.text_range(),
        )
    })?;
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
        BlockItem::LocalClassOrInterfaceDeclaration(declaration) => Err(missing_layout(
            "Java formatter does not support local class or interface declarations yet",
            declaration.text_range(),
        )),
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
        BlockItem::TryWithResourcesStatement(try_statement) => Err(missing_layout(
            "Java formatter does not support try-with-resources statements yet",
            try_statement.text_range(),
        )),
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
        Statement::TryWithResourcesStatement(try_statement) => Err(missing_layout(
            "Java formatter does not support try-with-resources statements yet",
            try_statement.text_range(),
        )),
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
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this empty statement shape yet",
            statement.text_range(),
        ));
    }

    Ok(text(";"))
}

pub(super) fn format_if_statement(
    statement: &IfStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this if statement shape yet",
            statement.text_range(),
        ));
    }

    let condition = statement.condition().ok_or_else(|| {
        missing_layout(
            "Java formatter found an if statement without a condition",
            statement.text_range(),
        )
    })?;
    let then_statement = statement.then_statement().ok_or_else(|| {
        missing_layout(
            "Java formatter found an if statement without a then statement",
            statement.text_range(),
        )
    })?;
    let then_range = then_statement.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty if statement body",
            then_statement.text_range(),
        )
    })?;
    reject_unhandled_comments_before_start(
        context,
        then_range,
        "Java formatter does not support comments before if statement bodies yet",
    )?;
    let then_is_block = matches!(then_statement, Statement::Block(_));
    let then_statement = format_unbraced_statement(&then_statement, context)?;
    let else_statement = statement
        .else_statement()
        .map(|else_statement| {
            let else_range = else_statement.code_text_range().ok_or_else(|| {
                missing_layout(
                    "Java formatter found an empty else statement body",
                    else_statement.text_range(),
                )
            })?;
            reject_unhandled_comments_before_start(
                context,
                else_range,
                "Java formatter does not support comments before else statement bodies yet",
            )?;
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
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this while statement shape yet",
            statement.text_range(),
        ));
    }

    let condition = statement.condition().ok_or_else(|| {
        missing_layout(
            "Java formatter found a while statement without a condition",
            statement.text_range(),
        )
    })?;
    let body = statement.body().ok_or_else(|| {
        missing_layout(
            "Java formatter found a while statement without a body",
            statement.text_range(),
        )
    })?;
    let body_range = body.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty while statement body",
            body.text_range(),
        )
    })?;
    reject_unhandled_comments_before_start(
        context,
        body_range,
        "Java formatter does not support comments before while statement bodies yet",
    )?;
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
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this do statement shape yet",
            statement.text_range(),
        ));
    }

    let body = statement.body().ok_or_else(|| {
        missing_layout(
            "Java formatter found a do statement without a body",
            statement.text_range(),
        )
    })?;
    let body_range = body.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty do statement body",
            body.text_range(),
        )
    })?;
    reject_unhandled_comments_before_start(
        context,
        body_range,
        "Java formatter does not support comments before do statement bodies yet",
    )?;
    let condition = statement.condition().ok_or_else(|| {
        missing_layout(
            "Java formatter found a do statement without a condition",
            statement.text_range(),
        )
    })?;
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
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this for statement shape yet",
            statement.text_range(),
        ));
    }

    if let Some(basic) = statement.basic() {
        return format_basic_for_statement(&basic, context);
    }
    if let Some(enhanced) = statement.enhanced() {
        return format_enhanced_for_statement(&enhanced, context);
    }

    Err(missing_layout(
        "Java formatter found a for statement without a basic or enhanced form",
        statement.text_range(),
    ))
}

pub(super) fn format_basic_for_statement(
    statement: &BasicForStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this basic for statement shape yet",
            statement.text_range(),
        ));
    }

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
    let body = statement.body().ok_or_else(|| {
        missing_layout(
            "Java formatter found a basic for statement without a body",
            statement.text_range(),
        )
    })?;
    let body_range = body.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty for statement body",
            body.text_range(),
        )
    })?;
    reject_unhandled_comments_before_start(
        context,
        body_range,
        "Java formatter does not support comments before for statement bodies yet",
    )?;
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
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this enhanced for statement shape yet",
            statement.text_range(),
        ));
    }

    let variable = statement.variable().ok_or_else(|| {
        missing_layout(
            "Java formatter found an enhanced for statement without a variable",
            statement.text_range(),
        )
    })?;
    let iterable = statement.iterable().ok_or_else(|| {
        missing_layout(
            "Java formatter found an enhanced for statement without an iterable expression",
            statement.text_range(),
        )
    })?;
    let body = statement.body().ok_or_else(|| {
        missing_layout(
            "Java formatter found an enhanced for statement without a body",
            statement.text_range(),
        )
    })?;
    let body_range = body.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty for statement body",
            body.text_range(),
        )
    })?;
    reject_unhandled_comments_before_start(
        context,
        body_range,
        "Java formatter does not support comments before for statement bodies yet",
    )?;
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
    if !initializer.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this for initializer shape yet",
            initializer.text_range(),
        ));
    }
    if let Some(declaration) = initializer.local_variable_declaration() {
        return format_local_variable_declaration_header(&declaration, context);
    }
    let expressions = initializer.expressions().ok_or_else(|| {
        missing_layout(
            "Java formatter found a for initializer without expressions",
            initializer.text_range(),
        )
    })?;
    format_statement_expression_list(&expressions, context)
}

pub(super) fn format_for_update(
    update: &ForUpdate,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !update.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this for update shape yet",
            update.text_range(),
        ));
    }
    let expressions = update.expressions().ok_or_else(|| {
        missing_layout(
            "Java formatter found a for update without expressions",
            update.text_range(),
        )
    })?;
    format_statement_expression_list(&expressions, context)
}

pub(super) fn format_statement_expression_list(
    list: &StatementExpressionList,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !list.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this statement expression list shape yet",
            list.text_range(),
        ));
    }
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
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this synchronized statement shape yet",
            statement.text_range(),
        ));
    }

    let expression = statement.expression().ok_or_else(|| {
        missing_layout(
            "Java formatter found a synchronized statement without an expression",
            statement.text_range(),
        )
    })?;
    let body = statement.body().ok_or_else(|| {
        missing_layout(
            "Java formatter found a synchronized statement without a body",
            statement.text_range(),
        )
    })?;
    let body_range = body.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty synchronized statement body",
            body.text_range(),
        )
    })?;
    reject_unhandled_comments_before_start(
        context,
        body_range,
        "Java formatter does not support comments inside synchronized statement headers yet",
    )?;

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
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this try statement shape yet",
            statement.text_range(),
        ));
    }

    let body = statement.body().ok_or_else(|| {
        missing_layout(
            "Java formatter found a try statement without a body",
            statement.text_range(),
        )
    })?;
    let body_range = body.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty try statement body",
            body.text_range(),
        )
    })?;
    reject_unhandled_comments_before_start(
        context,
        body_range,
        "Java formatter does not support comments before try statement bodies yet",
    )?;

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

pub(super) fn format_catch_clause(
    clause: &CatchClause,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !clause.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this catch clause shape yet",
            clause.text_range(),
        ));
    }

    let parameter = clause.parameter().ok_or_else(|| {
        missing_layout(
            "Java formatter found a catch clause without a parameter",
            clause.text_range(),
        )
    })?;
    let parameter_range = parameter.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty catch parameter",
            parameter.text_range(),
        )
    })?;
    reject_unhandled_comments_before_start(
        context,
        parameter_range,
        "Java formatter does not support comments inside catch statement headers yet",
    )?;
    let body = clause.body().ok_or_else(|| {
        missing_layout(
            "Java formatter found a catch clause without a body",
            clause.text_range(),
        )
    })?;
    let body_range = body.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty catch clause body",
            body.text_range(),
        )
    })?;
    reject_unhandled_comments_before_start(
        context,
        body_range,
        "Java formatter does not support comments inside catch statement headers yet",
    )?;

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
    if !parameter.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter only supports simple catch parameters yet",
            parameter.text_range(),
        ));
    }

    let ty = parameter.ty().ok_or_else(|| {
        missing_layout(
            "Java formatter found a catch parameter without a type",
            parameter.text_range(),
        )
    })?;
    if !ty.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter only supports single catch parameter types yet",
            ty.text_range(),
        ));
    }
    let ty = ty.ty().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty catch parameter type",
            ty.text_range(),
        )
    })?;
    let name = parameter.name().ok_or_else(|| {
        missing_layout(
            "Java formatter found a catch parameter without a name",
            parameter.text_range(),
        )
    })?;

    let mut parts = Vec::new();
    if let Some(final_token) = parameter.final_token() {
        parts.push(format_token(&final_token));
    }
    parts.push(format_type(&ty, context)?);
    parts.push(format_token(&name));
    Ok(wrap::space_separated(parts))
}

pub(super) fn format_finally_clause(
    clause: &FinallyClause,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !clause.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this finally clause shape yet",
            clause.text_range(),
        ));
    }

    let body = clause.body().ok_or_else(|| {
        missing_layout(
            "Java formatter found a finally clause without a body",
            clause.text_range(),
        )
    })?;
    let body_range = body.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty finally clause body",
            body.text_range(),
        )
    })?;
    reject_unhandled_comments_before_start(
        context,
        body_range,
        "Java formatter does not support comments before finally clause bodies yet",
    )?;

    Ok(concat([text("finally "), format_block(&body, context)?]))
}

pub(super) fn format_break_statement(statement: &BreakStatement) -> FormatResult<Doc> {
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this break statement shape yet",
            statement.text_range(),
        ));
    }

    Ok(wrap::keyword_label_statement(
        "break",
        statement.label().map(|label| format_token(&label)),
    ))
}

pub(super) fn format_continue_statement(statement: &ContinueStatement) -> FormatResult<Doc> {
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this continue statement shape yet",
            statement.text_range(),
        ));
    }

    Ok(wrap::keyword_label_statement(
        "continue",
        statement.label().map(|label| format_token(&label)),
    ))
}

pub(super) fn format_labeled_statement(
    statement: &LabeledStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this labeled statement shape yet",
            statement.text_range(),
        ));
    }

    let label = statement.label().ok_or_else(|| {
        missing_layout(
            "Java formatter found a labeled statement without a label",
            statement.text_range(),
        )
    })?;
    let body = statement.statement().ok_or_else(|| {
        missing_layout(
            "Java formatter found a labeled statement without a body",
            statement.text_range(),
        )
    })?;
    let body_range = body.code_text_range().ok_or_else(|| {
        missing_layout(
            "Java formatter found an empty labeled statement body",
            body.text_range(),
        )
    })?;
    reject_unhandled_comments_before_start(
        context,
        body_range,
        "Java formatter does not support comments before labeled statement bodies yet",
    )?;

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
    if !declaration.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this local variable declaration shape yet",
            declaration.text_range(),
        ));
    }

    let ty = if let Some(ty) = declaration.ty() {
        format_type(&ty, context)?
    } else {
        let token = declaration.var_type_token().ok_or_else(|| {
            missing_layout(
                "Java formatter found a local variable declaration without a type",
                declaration.text_range(),
            )
        })?;
        format_token(&token)
    };
    let declarators = declaration.declarators().ok_or_else(|| {
        missing_layout(
            "Java formatter found a local variable declaration without declarators",
            declaration.text_range(),
        )
    })?;
    let declarators = format_variable_declarator_list(&declarators, "local variable", context)?;

    let mut prefix = Vec::new();
    if let Some(final_token) = declaration.final_token() {
        prefix.push(format_token(&final_token));
    }
    prefix.push(ty);

    Ok(wrap::variable_declaration_header(prefix, declarators))
}

pub(super) fn format_variable_declarator_list(
    declarators: &jolt_java_syntax::VariableDeclaratorList,
    declaration_kind: &str,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let declarator_docs = declarators
        .declarators()
        .map(|declarator| {
            if !declarator.has_identifier_layout_shape() {
                return Err(missing_layout(
                    format!(
                        "Java formatter only supports identifier {declaration_kind} declarators without array dimensions"
                    ),
                    declarator.text_range(),
                ));
            }
            format_variable_declarator(&declarator, context)
        })
        .collect::<FormatResult<Vec<_>>>()?;

    if declarator_docs.is_empty() {
        return Err(missing_layout(
            format!("Java formatter found an empty {declaration_kind} declarator list"),
            declarators.text_range(),
        ));
    }

    Ok(wrap::comma_list(declarator_docs))
}

pub(super) fn format_variable_declarator(
    declarator: &VariableDeclarator,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let name = declarator.name().ok_or_else(|| {
        missing_layout(
            "Java formatter found a variable declarator without a name",
            declarator.text_range(),
        )
    })?;
    let Some(initializer) = declarator.initializer() else {
        return Ok(wrap::variable_declarator(text(name.text()), None));
    };
    if !initializer.has_expression_layout_shape() {
        return Err(missing_layout(
            "Java formatter only supports expression variable initializers",
            initializer.text_range(),
        ));
    }
    let expression = initializer.expression().ok_or_else(|| {
        missing_layout(
            "Java formatter found a variable initializer without an expression",
            initializer.text_range(),
        )
    })?;

    Ok(wrap::variable_declarator(
        text(name.text()),
        Some(format_expression(&expression, context)?),
    ))
}

pub(super) fn format_return_statement(
    statement: &ReturnStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this return statement shape yet",
            statement.text_range(),
        ));
    }

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
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this throw statement shape yet",
            statement.text_range(),
        ));
    }
    let expression = statement.expression().ok_or_else(|| {
        missing_layout(
            "Java formatter found a throw statement without an expression",
            statement.text_range(),
        )
    })?;
    Ok(wrap::keyword_expression_statement(
        "throw",
        Some(format_expression(&expression, context)?),
    ))
}

pub(super) fn format_yield_statement(
    statement: &YieldStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this yield statement shape yet",
            statement.text_range(),
        ));
    }
    let expression = statement.expression().ok_or_else(|| {
        missing_layout(
            "Java formatter found a yield statement without an expression",
            statement.text_range(),
        )
    })?;
    Ok(wrap::keyword_expression_statement(
        "yield",
        Some(format_expression(&expression, context)?),
    ))
}

pub(super) fn format_assert_statement(
    statement: &jolt_java_syntax::AssertStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this assert statement shape yet",
            statement.text_range(),
        ));
    }

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
        _ => Err(missing_layout(
            "Java formatter found an assert statement without one or two expressions",
            statement.text_range(),
        )),
    }
}

pub(super) fn format_switch_statement(
    statement: &SwitchStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this switch statement shape yet",
            statement.text_range(),
        ));
    }

    let selector = statement
        .selector()
        .expect("validated switch statement should have a selector");
    let block = statement
        .block()
        .expect("validated switch statement should have a block");

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
    let selector = switch_expression.selector().ok_or_else(|| {
        missing_layout(
            "Java formatter found a switch expression without a selector",
            switch_expression.text_range(),
        )
    })?;
    let block = switch_expression.block().ok_or_else(|| {
        missing_layout(
            "Java formatter found a switch expression without a block",
            switch_expression.text_range(),
        )
    })?;

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
    if !block.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this switch block shape yet",
            block.text_range(),
        ));
    }

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
    }
}

pub(super) fn format_switch_statement_group(
    group: &SwitchBlockStatementGroup,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !group.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this switch statement group shape yet",
            group.text_range(),
        ));
    }

    let code_range = group
        .code_text_range()
        .expect("validated switch statement group should have code");
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
    if !rule.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this switch rule shape yet",
            rule.text_range(),
        ));
    }

    let code_range = rule
        .code_text_range()
        .expect("validated switch rule should have code");
    let leading_comments = take_leading_comment_docs(context, code_range)?;
    let label = rule
        .label()
        .expect("validated switch rule should have a label");
    let body = rule
        .body()
        .expect("validated switch rule should have a body");

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
    if !label.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this switch label shape yet",
            label.text_range(),
        ));
    }

    if label.has_default_only_layout_shape() {
        return Ok(text("default"));
    }

    let items = label
        .constants()
        .map(|constant| {
            if !constant.has_supported_layout_shape() {
                return Err(missing_layout(
                    "Java formatter does not support this switch case constant shape yet",
                    constant.text_range(),
                ));
            }
            let expression = constant
                .expression()
                .expect("validated switch case constant should have an expression");
            format_expression(&expression, context)
        })
        .collect::<FormatResult<Vec<_>>>()?;

    let items = if label.default_token().is_some() {
        items
            .into_iter()
            .chain([text("default")])
            .collect::<Vec<_>>()
    } else {
        items
    };

    Ok(concat([text("case "), wrap::comma_list(items)]))
}

pub(super) fn format_expression_statement(
    statement: &jolt_java_syntax::ExpressionStatement,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !statement.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this expression statement shape yet",
            statement.text_range(),
        ));
    }
    let expression = statement.expression().ok_or_else(|| {
        missing_layout(
            "Java formatter found an expression statement without an expression",
            statement.text_range(),
        )
    })?;

    Ok(wrap::expression_statement(format_expression(
        &expression,
        context,
    )?))
}
