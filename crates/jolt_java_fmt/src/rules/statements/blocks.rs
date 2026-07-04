use super::{
    Block, BlockItem, BlockStatement, BodyItem, Doc, FormatterIgnoreRange, JavaFormatter,
    JavaSyntaxToken, Range, TrailingTrivia, comments_from_tokens, concat, format_dangling_comments,
    format_local_variable_declaration, format_removed_comments, format_statement,
    format_statement_semicolon, format_type_declaration, formatter_ignore_ranges,
    formatter_ignore_run_doc, formatter_ignore_runs, hard_line, join_body_items,
    relative_token_range_between,
};
use crate::helpers::comments::{
    InlineLeadingTrivia, format_token_after_relocated_leading_comments,
    format_token_with_inline_leading_comments,
};

pub(crate) fn format_block(block: &Block, formatter: &JavaFormatter<'_>) -> Doc {
    concat([
        block
            .open_brace()
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, format_block_open_brace),
        format_block_body(block, formatter).map_or_else(hard_line, |body| {
            concat([
                jolt_fmt_ir::indent(concat([hard_line(), body])),
                hard_line(),
            ])
        }),
        block
            .close_brace()
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, |close| {
                format_token_after_relocated_leading_comments(close, TrailingTrivia::Preserve)
            }),
    ])
}

fn format_block_open_brace(open: &JavaSyntaxToken) -> Doc {
    format_token_with_inline_leading_comments(
        open,
        InlineLeadingTrivia::BeforeToken,
        TrailingTrivia::RelocatedToEnclosingContext,
    )
}

pub(crate) fn format_block_body(block: &Block, formatter: &JavaFormatter<'_>) -> Option<Doc> {
    format_block_statements_body(block, formatter)
}

fn format_block_statements_body(block: &Block, formatter: &JavaFormatter<'_>) -> Option<Doc> {
    let statements = block.block_statements().collect::<Vec<_>>();
    let block_start = block.text_range().start().get();
    let ignored_ranges = block_formatter_ignore_ranges(block);
    let mut items = Vec::new();
    items.extend(format_block_open_dangling_comments(block));
    items.extend(format_block_statement_items(
        &statements,
        block_start,
        &ignored_ranges,
        formatter,
    ));
    items.extend(format_block_close_dangling_comments(block));
    (!items.is_empty()).then(|| join_body_items(items))
}

fn format_block_open_dangling_comments(block: &Block) -> Option<BodyItem> {
    let comments = block.open_brace()?.trailing_comments();
    (!comments.is_empty()).then(|| BodyItem::new(format_dangling_comments(comments), false))
}

fn format_block_close_dangling_comments(block: &Block) -> Option<BodyItem> {
    let comments = block.close_brace()?.leading_comments();
    (!comments.is_empty()).then(|| BodyItem::new(format_dangling_comments(comments), false))
}

fn format_block_statement_items(
    statements: &[BlockStatement],
    block_start: usize,
    ignored_ranges: &[FormatterIgnoreRange],
    formatter: &JavaFormatter<'_>,
) -> Vec<BodyItem> {
    let statement_ranges = statements
        .iter()
        .map(|statement| block_statement_token_range(statement, block_start))
        .collect::<Vec<_>>();
    let ignored_runs = formatter_ignore_runs(ignored_ranges, &statement_ranges);

    let mut items = Vec::new();
    let mut ignored_index = 0;
    let mut skip_index = 0;
    for (statement_index, statement) in statements.iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == statement_index
        {
            let run = &ignored_runs[ignored_index];
            items.push(BodyItem::new(formatter_ignore_run_doc(run), false));
            ignored_index += 1;
        }

        while skip_index < ignored_runs.len()
            && ignored_runs[skip_index].skip_end <= statement_index
        {
            skip_index += 1;
        }

        if skip_index < ignored_runs.len() && ignored_runs[skip_index].skips(statement_index) {
            continue;
        }

        if let Some(mut item) = format_block_statement_item(statement, formatter) {
            if skip_index > 0 && ignored_runs[skip_index - 1].skip_end == statement_index {
                item = item.without_blank_line_before();
            }
            items.push(item);
        }
    }

    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        items.push(BodyItem::new(formatter_ignore_run_doc(run), false));
        ignored_index += 1;
    }

    items
}

pub(crate) fn format_block_statement_item(
    statement: &BlockStatement,
    formatter: &JavaFormatter<'_>,
) -> Option<BodyItem> {
    let starts_after_blank_line = statement.starts_after_blank_line();
    let doc = format_block_item_doc(statement.item()?, statement.semicolon(), formatter)?;
    Some(BodyItem::new(doc, starts_after_blank_line))
}

fn format_block_item_doc(
    item: BlockItem,
    semicolon: Option<JavaSyntaxToken>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc> {
    let doc = match item {
        BlockItem::EmptyStatement(statement) => format_removed_empty_statement(&statement),
        BlockItem::LocalVariableDeclaration(declaration) => Some(concat([
            format_local_variable_declaration(&declaration, formatter),
            format_statement_semicolon(semicolon),
        ])),
        BlockItem::LocalClassOrInterfaceDeclaration(declaration) => declaration
            .declaration()
            .map(|declaration| format_type_declaration(&declaration, formatter)),
        BlockItem::Block(block) => Some(format_block(&block, formatter)),
        BlockItem::LabeledStatement(statement) => {
            Some(format_statement(&statement.into(), formatter))
        }
        BlockItem::ExpressionStatement(statement) => {
            Some(format_statement(&statement.into(), formatter))
        }
        BlockItem::IfStatement(statement) => Some(format_statement(&statement.into(), formatter)),
        BlockItem::AssertStatement(statement) => {
            Some(format_statement(&statement.into(), formatter))
        }
        BlockItem::SwitchStatement(statement) => {
            Some(format_statement(&statement.into(), formatter))
        }
        BlockItem::WhileStatement(statement) => {
            Some(format_statement(&statement.into(), formatter))
        }
        BlockItem::DoStatement(statement) => Some(format_statement(&statement.into(), formatter)),
        BlockItem::ForStatement(statement) => Some(format_statement(&statement.into(), formatter)),
        BlockItem::BreakStatement(statement) => {
            Some(format_statement(&statement.into(), formatter))
        }
        BlockItem::YieldStatement(statement) => {
            Some(format_statement(&statement.into(), formatter))
        }
        BlockItem::ContinueStatement(statement) => {
            Some(format_statement(&statement.into(), formatter))
        }
        BlockItem::ReturnStatement(statement) => {
            Some(format_statement(&statement.into(), formatter))
        }
        BlockItem::ThrowStatement(statement) => {
            Some(format_statement(&statement.into(), formatter))
        }
        BlockItem::SynchronizedStatement(statement) => {
            Some(format_statement(&statement.into(), formatter))
        }
        BlockItem::TryStatement(statement) => Some(format_statement(&statement.into(), formatter)),
        BlockItem::TryWithResourcesStatement(statement) => {
            Some(format_statement(&statement.into(), formatter))
        }
    }?;
    Some(doc)
}

fn format_removed_empty_statement(statement: &jolt_java_syntax::EmptyStatement) -> Option<Doc> {
    format_removed_comments(comments_from_tokens(statement.token_iter()))
}

fn block_formatter_ignore_ranges(block: &Block) -> Vec<FormatterIgnoreRange> {
    formatter_ignore_ranges(&block.source_text())
}

fn block_statement_token_range(
    statement: &BlockStatement,
    block_start: usize,
) -> Option<Range<usize>> {
    Some(relative_token_range_between(
        &statement.first_token()?,
        &statement.last_token()?,
        block_start,
    ))
}
