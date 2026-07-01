use jolt_fmt_ir::{Doc, concat, hard_line, literal_text, text};
use jolt_java_syntax::{
    AssertStatement, BasicForStatement, Block, BlockItem, DoStatement, EnhancedForStatement,
    Expression, ExpressionStatement, ForInitializer, ForStatement, ForUpdate, IfStatement,
    LabeledStatement, ReturnStatement, Statement, StatementExpressionList, SynchronizedStatement,
    ThrowStatement, WhileStatement, YieldStatement,
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
        _ => source_doc(&statement.source_text()),
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

fn statement_body_as_block(statement: Option<Statement>) -> Doc {
    match statement {
        Some(Statement::Block(block)) => format_block(&block),
        Some(Statement::EmptyStatement(_)) | None => concat([text("{"), hard_line(), text("}")]),
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

fn expression_text(expression: &Expression) -> String {
    expression.source_text().trim().to_owned()
}
