use jolt_fmt_ir::{Doc, concat, group, hard_line, indent, soft_line, text};
use jolt_java_syntax::{
    AssertStatement, BasicForStatement, Block, BlockItem, CatchClause, CatchParameter,
    CatchTypeList, DoStatement, EnhancedForStatement, Expression, ExpressionStatement,
    FinallyClause, ForInitializer, ForStatement, ForUpdate, IfStatement, LabeledStatement,
    Resource, ReturnStatement, Statement, StatementExpressionList, SwitchBlock, SwitchBlockEntry,
    SwitchBlockStatementGroup, SwitchRule, SwitchStatement, SynchronizedStatement, ThrowStatement,
    TryStatement, TryWithResourcesStatement, Type, WhileStatement, YieldStatement,
};

use crate::helpers::blocks::{
    BodyItem, braced_block, braced_body, braced_body_items, empty_block, join_body_items,
    join_hard_lines,
};
use crate::helpers::comments::{
    comment_forces_line, format_comment, format_token_sequence, tokens_have_comments,
};
use crate::helpers::lists::semicolon_list;
use crate::helpers::modifiers::modifier_prefix_from_parts;
use crate::helpers::operators::binary_chain;
use crate::rules::declarations::format_type_declaration;
use crate::rules::expressions::format_expression;
use crate::rules::variables::format_local_variable_declaration;

pub(crate) fn format_block(block: &Block) -> Doc {
    let items = block
        .items()
        .filter_map(format_block_item)
        .collect::<Vec<_>>();
    braced_body_items(items)
}

fn format_block_item(item: BlockItem) -> Option<BodyItem> {
    let starts_after_blank_line = item.starts_after_blank_line();
    let doc = match item {
        BlockItem::EmptyStatement(_) => None,
        BlockItem::LocalVariableDeclaration(declaration) => Some(concat([
            format_local_variable_declaration(&declaration),
            text(";"),
        ])),
        BlockItem::LocalClassOrInterfaceDeclaration(declaration) => declaration
            .declaration()
            .map(|declaration| format_type_declaration(&declaration)),
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
    }?;
    Some(BodyItem::new(doc, starts_after_blank_line))
}

fn format_statement(statement: &Statement) -> Doc {
    match statement {
        Statement::Block(block) => format_block(block),
        Statement::EmptyStatement(_) => empty_block(),
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
        statement
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression)
            }),
        text(";"),
    ])
}

fn format_if_statement(statement: &IfStatement) -> Doc {
    let condition = statement
        .condition()
        .map_or_else(jolt_fmt_ir::nil, |condition| format_expression(&condition));
    let then_body = statement_body_as_block(statement.then_statement());

    concat([
        text("if ("),
        condition,
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
        statement
            .condition()
            .map_or_else(jolt_fmt_ir::nil, |condition| format_expression(&condition)),
        statement.detail().map_or_else(jolt_fmt_ir::nil, |detail| {
            concat([text(" : "), format_expression(&detail)])
        }),
        text(";"),
    ])
}

fn format_while_statement(statement: &WhileStatement) -> Doc {
    let condition = statement
        .condition()
        .map_or_else(jolt_fmt_ir::nil, |condition| format_expression(&condition));
    concat([
        text("while ("),
        condition,
        text(") "),
        statement_body_as_block(statement.body()),
    ])
}

fn format_do_statement(statement: &DoStatement) -> Doc {
    concat([
        text("do "),
        statement_body_as_block(statement.body()),
        text(" while ("),
        statement
            .condition()
            .map_or_else(jolt_fmt_ir::nil, |condition| format_expression(&condition)),
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

    format_token_sequence(&statement.tokens())
}

fn format_basic_for_statement(statement: &BasicForStatement) -> Doc {
    let initializer = statement
        .initializer()
        .map(|initializer| format_for_initializer(&initializer));
    let condition = statement
        .condition()
        .map(|condition| format_expression(&condition));
    let update = statement.update().map(|update| format_for_update(&update));
    let is_empty_header = initializer.is_none() && condition.is_none() && update.is_none();
    let header = if is_empty_header {
        concat([text("for ("), text(";;"), text(")")])
    } else {
        group(concat([
            text("for ("),
            indent(concat([
                soft_line(),
                semicolon_list(vec![
                    initializer.unwrap_or_else(jolt_fmt_ir::nil),
                    condition.unwrap_or_else(jolt_fmt_ir::nil),
                    update.unwrap_or_else(jolt_fmt_ir::nil),
                ]),
            ])),
            soft_line(),
            text(")"),
        ]))
    };

    concat([header, text(" "), statement_body_as_block(statement.body())])
}

fn format_enhanced_for_statement(statement: &EnhancedForStatement) -> Doc {
    concat([
        text("for ("),
        statement
            .variable()
            .map_or_else(jolt_fmt_ir::nil, |variable| {
                format_local_variable_declaration(&variable)
            }),
        text(" : "),
        statement
            .iterable()
            .map_or_else(jolt_fmt_ir::nil, |iterable| format_expression(&iterable)),
        text(") "),
        statement_body_as_block(statement.body()),
    ])
}

fn format_for_initializer(initializer: &ForInitializer) -> Doc {
    if let Some(declaration) = initializer.local_variable_declaration() {
        return format_local_variable_declaration(&declaration);
    }
    initializer
        .expressions()
        .map_or_else(jolt_fmt_ir::nil, |expressions| {
            format_statement_expression_list(&expressions)
        })
}

fn format_for_update(update: &ForUpdate) -> Doc {
    update
        .expressions()
        .map_or_else(jolt_fmt_ir::nil, |expressions| {
            format_statement_expression_list(&expressions)
        })
}

fn format_statement_expression_list(expressions: &StatementExpressionList) -> Doc {
    let tokens = expressions.tokens();
    if tokens_have_comments(&tokens) {
        return format_token_sequence(&tokens);
    }
    jolt_fmt_ir::join(
        text(", "),
        expressions
            .expressions()
            .map(|expression| format_expression(&expression)),
    )
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
            let expression_doc = concat([text(" "), format_expression(&expression)]);
            if matches!(expression, Expression::SwitchExpression(_)) {
                expression_doc
            } else {
                indent(expression_doc)
            }
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
        statement
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression)
            }),
        text(") "),
        statement
            .body()
            .map_or_else(empty_block, |body| format_block(&body)),
    ])
}

fn format_switch_statement(statement: &SwitchStatement) -> Doc {
    concat([
        text("switch ("),
        statement
            .selector()
            .map_or_else(jolt_fmt_ir::nil, |selector| format_expression(&selector)),
        text(") "),
        statement
            .block()
            .map_or_else(empty_block, |block| format_switch_block(&block)),
    ])
}

pub(crate) fn format_switch_block(block: &SwitchBlock) -> Doc {
    let entries = block
        .entries()
        .map(|entry| match entry {
            SwitchBlockEntry::StatementGroup(group) => format_switch_statement_group(&group),
            SwitchBlockEntry::Rule(rule) => format_switch_rule(&rule),
        })
        .collect::<Vec<_>>();

    braced_block(entries)
}

fn format_switch_statement_group(group: &SwitchBlockStatementGroup) -> Doc {
    let labels = group
        .labels()
        .map(|label| concat([format_token_sequence(&label.tokens()), text(":")]))
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
            jolt_fmt_ir::indent(concat([hard_line(), join_body_items(items)]))
        },
    ])
}

fn format_switch_rule(rule: &SwitchRule) -> Doc {
    let label = rule.label().map_or_else(jolt_fmt_ir::nil, |label| {
        format_token_sequence(&label.tokens())
    });

    concat([
        label,
        format_switch_rule_arrow(rule),
        format_switch_rule_body(rule),
    ])
}

fn format_switch_rule_arrow(rule: &SwitchRule) -> Doc {
    let Some(arrow) = rule.arrow() else {
        return text(" -> ");
    };

    let trailing_comments = arrow.trailing_comments();
    if trailing_comments.is_empty() {
        return text(" -> ");
    }

    let mut docs = vec![text(" ->")];
    let mut forced_line = false;
    for comment in trailing_comments {
        docs.push(text(" "));
        forced_line |= comment_forces_line(&comment);
        docs.push(format_comment(&comment));
    }
    docs.push(if forced_line { hard_line() } else { text(" ") });
    concat(docs)
}

fn format_switch_rule_body(rule: &SwitchRule) -> Doc {
    if let Some(block) = rule.block() {
        return format_block(&block);
    }
    if let Some(statement) = rule.throw_statement() {
        return format_throw_statement(&statement);
    }
    if let Some(expression) = rule.expression() {
        return concat([format_expression(&expression), text(";")]);
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
            .map_or_else(empty_block, |body| format_block(&body)),
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
            .map_or_else(empty_block, |body| format_block(&body)),
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
        return format_local_variable_declaration(&declaration);
    }
    if let Some(access) = resource.variable_access() {
        return access
            .expression()
            .map_or_else(jolt_fmt_ir::nil, |expression| {
                format_expression(&expression)
            });
    }

    format_token_sequence(&resource.tokens())
}

fn format_catch_clauses<'a>(clauses: impl Iterator<Item = CatchClause> + 'a) -> Doc {
    concat(clauses.map(|clause| format_catch_clause(&clause)))
}

fn format_catch_clause(clause: &CatchClause) -> Doc {
    concat([
        text(" catch "),
        clause
            .parameter()
            .map_or_else(jolt_fmt_ir::nil, |parameter| {
                format_parenthesized_catch_parameter(&parameter)
            }),
        text(" "),
        clause
            .body()
            .map_or_else(empty_block, |body| format_block(&body)),
    ])
}

fn format_parenthesized_catch_parameter(parameter: &CatchParameter) -> Doc {
    group(concat([
        text("("),
        indent(concat([soft_line(), format_catch_parameter(parameter)])),
        soft_line(),
        text(")"),
    ]))
}

fn format_catch_parameter(parameter: &CatchParameter) -> Doc {
    let tokens = parameter.tokens();
    let annotations = parameter.annotations().collect::<Vec<_>>();
    if tokens_have_comments(&tokens) || !annotations.is_empty() {
        return format_token_sequence(&tokens);
    }

    concat([
        modifier_prefix_from_parts(annotations, parameter.modifier_tokens().collect()),
        parameter.types().map_or_else(jolt_fmt_ir::nil, |types| {
            format_catch_type_list(&types, parameter.name())
        }),
    ])
}

fn format_catch_type_list(
    types: &CatchTypeList,
    name: Option<jolt_java_syntax::JavaSyntaxToken>,
) -> Doc {
    let mut types = types
        .types()
        .map(|ty| format_catch_type(&ty))
        .collect::<Vec<_>>();
    let name = name.map_or_else(jolt_fmt_ir::nil, |name| text(name.text().to_owned()));

    let Some(last_type) = types.pop() else {
        return name;
    };
    let last = concat([last_type, text(" "), name]);
    if types.is_empty() {
        return last;
    }

    let first = types.remove(0);
    let rest = types
        .into_iter()
        .chain(std::iter::once(last))
        .map(|ty| ("|".to_owned(), ty))
        .collect();
    binary_chain(first, rest)
}

fn format_catch_type(ty: &Type) -> Doc {
    format_token_sequence(&ty.tokens())
}

fn format_finally_clause(clause: &FinallyClause) -> Doc {
    concat([
        text(" finally "),
        clause
            .body()
            .map_or_else(empty_block, |body| format_block(&body)),
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
        Some(Statement::EmptyStatement(_)) | None => empty_block(),
        Some(statement) => braced_body(Some(format_statement(&statement))),
    }
}
