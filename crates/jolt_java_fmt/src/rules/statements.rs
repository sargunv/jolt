use jolt_fmt_ir::{Doc, concat, hard_line, literal_text, text};
use jolt_java_syntax::{
    AssertStatement, BasicForStatement, Block, BlockItem, CatchClause, DoStatement,
    EnhancedForStatement, Expression, ExpressionStatement, FinallyClause, ForInitializer,
    ForStatement, ForUpdate, IfStatement, LabeledStatement, Resource, ReturnStatement, Statement,
    StatementExpressionList, SwitchBlock, SwitchBlockEntry, SwitchBlockStatementGroup, SwitchRule,
    SwitchStatement, SynchronizedStatement, ThrowStatement, TryStatement,
    TryWithResourcesStatement, WhileStatement, YieldStatement,
};

pub(crate) fn format_block(block: &Block) -> Doc {
    let items = block
        .items()
        .filter_map(format_block_item)
        .collect::<Vec<_>>();
    if items.is_empty() {
        return concat([text("{"), hard_line(), text("}")]);
    }

    concat([
        text("{"),
        jolt_fmt_ir::indent(concat([hard_line(), join_hard_lines(items)])),
        hard_line(),
        text("}"),
    ])
}

fn format_block_item(item: BlockItem) -> Option<Doc> {
    match item {
        BlockItem::EmptyStatement(_) => None,
        BlockItem::LocalVariableDeclaration(declaration) => Some(concat([
            text(declaration.source_text().trim().to_owned()),
            text(";"),
        ])),
        BlockItem::LocalClassOrInterfaceDeclaration(declaration) => {
            Some(source_doc(&declaration.source_text()))
        }
        BlockItem::Block(block) => Some(format_block(&block)),
        BlockItem::LabeledStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::ExpressionStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::IfStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::AssertStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::SwitchStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::WhileStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::DoStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::ForStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::BreakStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::YieldStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::ContinueStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::ReturnStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::ThrowStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::SynchronizedStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::TryStatement(statement) => Some(format_statement(&statement.into())),
        BlockItem::TryWithResourcesStatement(statement) => {
            Some(format_statement(&statement.into()))
        }
    }
}

fn format_statement(statement: &Statement) -> Doc {
    match statement {
        Statement::Block(block) => format_block(block),
        Statement::EmptyStatement(_) => concat([text("{"), hard_line(), text("}")]),
        Statement::LabeledStatement(statement) => format_labeled_statement(statement),
        Statement::ExpressionStatement(statement) => format_expression_statement(statement),
        Statement::IfStatement(statement) => format_if_statement(statement),
        Statement::AssertStatement(statement) => format_assert_statement(statement),
        Statement::SwitchStatement(statement) => format_switch_statement(statement),
        Statement::WhileStatement(statement) => format_while_statement(statement),
        Statement::DoStatement(statement) => format_do_statement(statement),
        Statement::ForStatement(statement) => format_for_statement(statement),
        Statement::BreakStatement(statement) => format_jump_statement("break", statement.label()),
        Statement::YieldStatement(statement) => format_yield_statement(statement),
        Statement::ContinueStatement(statement) => {
            format_jump_statement("continue", statement.label())
        }
        Statement::ReturnStatement(statement) => format_return_statement(statement),
        Statement::ThrowStatement(statement) => format_throw_statement(statement),
        Statement::SynchronizedStatement(statement) => format_synchronized_statement(statement),
        Statement::TryStatement(statement) => format_try_statement(statement),
        Statement::TryWithResourcesStatement(statement) => {
            format_try_with_resources_statement(statement)
        }
    }
}

fn format_labeled_statement(statement: &LabeledStatement) -> Doc {
    let label = statement
        .label()
        .map_or_else(String::new, |label| label.text().to_owned());

    concat([
        text(label),
        text(":"),
        hard_line(),
        statement
            .body()
            .map_or_else(jolt_fmt_ir::nil, |body| format_statement(&body)),
    ])
}

fn format_expression_statement(statement: &ExpressionStatement) -> Doc {
    concat([
        text(
            statement
                .expression()
                .map_or_else(String::new, |expression| expression_text(&expression)),
        ),
        text(";"),
    ])
}

fn format_if_statement(statement: &IfStatement) -> Doc {
    let condition = statement
        .condition()
        .map_or_else(String::new, |condition| expression_text(&condition));
    let then_body = statement_body_as_block(statement.then_statement());

    concat([
        text("if ("),
        text(condition),
        text(") "),
        then_body,
        statement
            .else_statement()
            .map_or_else(jolt_fmt_ir::nil, |else_statement| {
                concat([
                    text(" else "),
                    match else_statement {
                        Statement::IfStatement(else_if) => format_if_statement(&else_if),
                        _ => statement_body_as_block(Some(else_statement)),
                    },
                ])
            }),
    ])
}

fn format_assert_statement(statement: &AssertStatement) -> Doc {
    concat([
        text("assert "),
        text(
            statement
                .condition()
                .map_or_else(String::new, |condition| expression_text(&condition)),
        ),
        statement.detail().map_or_else(jolt_fmt_ir::nil, |detail| {
            concat([text(" : "), text(expression_text(&detail))])
        }),
        text(";"),
    ])
}

fn format_while_statement(statement: &WhileStatement) -> Doc {
    let condition = statement
        .condition()
        .map_or_else(String::new, |condition| expression_text(&condition));
    concat([
        text("while ("),
        text(condition),
        text(") "),
        statement_body_as_block(statement.body()),
    ])
}

fn format_do_statement(statement: &DoStatement) -> Doc {
    concat([
        text("do "),
        statement_body_as_block(statement.body()),
        text(" while ("),
        text(
            statement
                .condition()
                .map_or_else(String::new, |condition| expression_text(&condition)),
        ),
        text(");"),
    ])
}

fn format_for_statement(statement: &ForStatement) -> Doc {
    if let Some(basic) = statement.basic() {
        return format_basic_for_statement(&basic);
    }
    if let Some(enhanced) = statement.enhanced() {
        return format_enhanced_for_statement(&enhanced);
    }

    source_doc(&statement.source_text())
}

fn format_basic_for_statement(statement: &BasicForStatement) -> Doc {
    let initializer = statement
        .initializer()
        .map_or_else(String::new, |initializer| {
            format_for_initializer_text(&initializer)
        });
    let condition = statement
        .condition()
        .map_or_else(String::new, |condition| expression_text(&condition));
    let update = statement
        .update()
        .map_or_else(String::new, |update| format_for_update_text(&update));

    concat([
        text("for ("),
        text(initializer),
        text(";"),
        format_for_segment_after_semicolon(&condition),
        text(";"),
        format_for_segment_after_semicolon(&update),
        text(") "),
        statement_body_as_block(statement.body()),
    ])
}

fn format_enhanced_for_statement(statement: &EnhancedForStatement) -> Doc {
    let variable = statement.variable().map_or_else(String::new, |variable| {
        variable.source_text().trim().to_owned()
    });
    let iterable = statement
        .iterable()
        .map_or_else(String::new, |iterable| expression_text(&iterable));

    concat([
        text("for ("),
        text(variable),
        text(" : "),
        text(iterable),
        text(") "),
        statement_body_as_block(statement.body()),
    ])
}

fn format_for_initializer_text(initializer: &ForInitializer) -> String {
    if let Some(declaration) = initializer.local_variable_declaration() {
        return declaration.source_text().trim().to_owned();
    }
    initializer
        .expressions()
        .map_or_else(String::new, |expressions| {
            format_statement_expression_list_text(&expressions)
        })
}

fn format_for_update_text(update: &ForUpdate) -> String {
    update
        .expressions()
        .map_or_else(String::new, |expressions| {
            format_statement_expression_list_text(&expressions)
        })
}

fn format_statement_expression_list_text(expressions: &StatementExpressionList) -> String {
    expressions
        .expressions()
        .map(|expression| expression_text(&expression))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_for_segment_after_semicolon(segment: &str) -> Doc {
    if segment.is_empty() {
        jolt_fmt_ir::nil()
    } else {
        concat([text(" "), text(segment.to_owned())])
    }
}

fn format_return_statement(statement: &ReturnStatement) -> Doc {
    format_keyword_expression_statement("return", statement.expression())
}

fn format_throw_statement(statement: &ThrowStatement) -> Doc {
    format_keyword_expression_statement("throw", statement.expression())
}

fn format_yield_statement(statement: &YieldStatement) -> Doc {
    format_keyword_expression_statement("yield", statement.expression())
}

fn format_keyword_expression_statement(keyword: &str, expression: Option<Expression>) -> Doc {
    concat([
        text(keyword.to_owned()),
        expression.map_or_else(jolt_fmt_ir::nil, |expression| {
            concat([text(" "), text(expression_text(&expression))])
        }),
        text(";"),
    ])
}

fn format_jump_statement(keyword: &str, label: Option<jolt_java_syntax::JavaSyntaxToken>) -> Doc {
    concat([
        text(keyword.to_owned()),
        label.map_or_else(jolt_fmt_ir::nil, |label| {
            concat([text(" "), text(label.text().to_owned())])
        }),
        text(";"),
    ])
}

fn format_synchronized_statement(statement: &SynchronizedStatement) -> Doc {
    concat([
        text("synchronized ("),
        text(
            statement
                .expression()
                .map_or_else(String::new, |expression| expression_text(&expression)),
        ),
        text(") "),
        statement.body().map_or_else(
            || concat([text("{"), hard_line(), text("}")]),
            |body| format_block(&body),
        ),
    ])
}

fn format_switch_statement(statement: &SwitchStatement) -> Doc {
    concat([
        text("switch ("),
        text(
            statement
                .selector()
                .map_or_else(String::new, |selector| expression_text(&selector)),
        ),
        text(") "),
        statement
            .block()
            .map_or_else(empty_block_doc, |block| format_switch_block(&block)),
    ])
}

fn format_switch_block(block: &SwitchBlock) -> Doc {
    let entries = block
        .entries()
        .map(|entry| match entry {
            SwitchBlockEntry::StatementGroup(group) => format_switch_statement_group(&group),
            SwitchBlockEntry::Rule(rule) => format_switch_rule(&rule),
        })
        .collect::<Vec<_>>();

    if entries.is_empty() {
        return empty_block_doc();
    }

    concat([
        text("{"),
        jolt_fmt_ir::indent(concat([hard_line(), join_hard_lines(entries)])),
        hard_line(),
        text("}"),
    ])
}

fn format_switch_statement_group(group: &SwitchBlockStatementGroup) -> Doc {
    let labels = group
        .labels()
        .map(|label| concat([text(label.source_text().trim().to_owned()), text(":")]))
        .collect::<Vec<_>>();
    let items = group
        .items()
        .filter_map(format_block_item)
        .collect::<Vec<_>>();

    concat([
        join_hard_lines(labels),
        if items.is_empty() {
            jolt_fmt_ir::nil()
        } else {
            jolt_fmt_ir::indent(concat([hard_line(), join_hard_lines(items)]))
        },
    ])
}

fn format_switch_rule(rule: &SwitchRule) -> Doc {
    let label = rule
        .label()
        .map_or_else(String::new, |label| label.source_text().trim().to_owned());

    concat([text(label), text(" -> "), format_switch_rule_body(rule)])
}

fn format_switch_rule_body(rule: &SwitchRule) -> Doc {
    if let Some(block) = rule.block() {
        return format_block(&block);
    }
    if let Some(statement) = rule.throw_statement() {
        return format_throw_statement(&statement);
    }
    if let Some(expression) = rule.expression() {
        return concat([text(expression_text(&expression)), text(";")]);
    }

    jolt_fmt_ir::nil()
}

fn format_try_statement(statement: &TryStatement) -> Doc {
    if let Some(resources_statement) = statement.resources_statement() {
        return format_try_with_resources_statement(&resources_statement);
    }

    concat([
        text("try "),
        statement
            .body()
            .map_or_else(empty_block_doc, |body| format_block(&body)),
        format_catch_clauses(statement.catch_clauses()),
        statement
            .finally_clause()
            .map_or_else(jolt_fmt_ir::nil, |finally_clause| {
                format_finally_clause(&finally_clause)
            }),
    ])
}

fn format_try_with_resources_statement(statement: &TryWithResourcesStatement) -> Doc {
    concat([
        text("try ("),
        format_resource_specification(statement),
        text(") "),
        statement
            .body()
            .map_or_else(empty_block_doc, |body| format_block(&body)),
        format_catch_clauses(statement.catch_clauses()),
        statement
            .finally_clause()
            .map_or_else(jolt_fmt_ir::nil, |finally_clause| {
                format_finally_clause(&finally_clause)
            }),
    ])
}

fn format_resource_specification(statement: &TryWithResourcesStatement) -> Doc {
    let resources = statement
        .resources()
        .and_then(|specification| specification.list())
        .map(|list| {
            list.resources()
                .map(|resource| format_resource(&resource))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if resources.is_empty() {
        return jolt_fmt_ir::nil();
    }

    concat([
        jolt_fmt_ir::indent(concat([hard_line(), join_resource_lines(resources)])),
        hard_line(),
    ])
}

fn format_resource(resource: &Resource) -> Doc {
    if let Some(declaration) = resource.declaration() {
        return text(declaration.source_text().trim().to_owned());
    }
    if let Some(access) = resource.variable_access() {
        return text(
            access
                .expression()
                .map_or_else(String::new, |expression| expression_text(&expression)),
        );
    }

    source_doc(&resource.source_text())
}

fn format_catch_clauses<'a>(clauses: impl Iterator<Item = CatchClause> + 'a) -> Doc {
    concat(clauses.map(|clause| format_catch_clause(&clause)))
}

fn format_catch_clause(clause: &CatchClause) -> Doc {
    concat([
        text(" catch ("),
        text(clause.parameter().map_or_else(String::new, |parameter| {
            parameter.source_text().trim().to_owned()
        })),
        text(") "),
        clause
            .body()
            .map_or_else(empty_block_doc, |body| format_block(&body)),
    ])
}

fn format_finally_clause(clause: &FinallyClause) -> Doc {
    concat([
        text(" finally "),
        clause
            .body()
            .map_or_else(empty_block_doc, |body| format_block(&body)),
    ])
}

fn join_resource_lines(docs: Vec<Doc>) -> Doc {
    let mut joined = Vec::new();
    for (index, doc) in docs.into_iter().enumerate() {
        if index > 0 {
            joined.push(text(";"));
            joined.push(hard_line());
        }
        joined.push(doc);
    }
    concat(joined)
}

fn statement_body_as_block(statement: Option<Statement>) -> Doc {
    match statement {
        Some(Statement::Block(block)) => format_block(&block),
        Some(Statement::EmptyStatement(_)) | None => empty_block_doc(),
        Some(statement) => concat([
            text("{"),
            jolt_fmt_ir::indent(concat([hard_line(), format_statement(&statement)])),
            hard_line(),
            text("}"),
        ]),
    }
}

fn join_hard_lines(docs: Vec<Doc>) -> Doc {
    let mut joined = Vec::new();
    for doc in docs {
        if !joined.is_empty() {
            joined.push(hard_line());
        }
        joined.push(doc);
    }
    concat(joined)
}

fn source_doc(source: &str) -> Doc {
    literal_text(source.trim().to_owned())
}

fn empty_block_doc() -> Doc {
    concat([text("{"), hard_line(), text("}")])
}

fn expression_text(expression: &Expression) -> String {
    expression.source_text().trim().to_owned()
}
