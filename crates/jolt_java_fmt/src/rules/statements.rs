use jolt_fmt_ir::{Doc, concat, hard_line, literal_text, text};
use jolt_java_syntax::{Block, BlockItem, IfStatement, Statement, WhileStatement};

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
        Statement::IfStatement(statement) => format_if_statement(statement),
        Statement::WhileStatement(statement) => format_while_statement(statement),
        _ => source_doc(&statement.source_text()),
    }
}

fn format_if_statement(statement: &IfStatement) -> Doc {
    let condition = statement.condition().map_or_else(String::new, |condition| {
        condition.source_text().trim().to_owned()
    });
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

fn format_while_statement(statement: &WhileStatement) -> Doc {
    let condition = statement.condition().map_or_else(String::new, |condition| {
        condition.source_text().trim().to_owned()
    });
    concat([
        text("while ("),
        text(condition),
        text(") "),
        statement_body_as_block(statement.body()),
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
