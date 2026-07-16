use super::{
    Block, BlockItem, BlockStatement, BodyItem, Doc, FormatterIgnoreRange, JavaSyntaxToken, Range,
    TrailingTrivia, comments_from_tokens, format_dangling_comments,
    format_local_variable_declaration, format_removed_comments, format_statement,
    format_statement_semicolon, format_type_declaration, formatter_ignore_ranges,
    formatter_ignore_run_doc, formatter_ignore_runs, join_body_items, relative_token_range_between,
};
use crate::helpers::blocks::BodyContent;
use crate::helpers::comments::{
    InlineLeadingTrivia, format_token_after_relocated_leading_comments,
    format_token_with_inline_leading_comments, has_removed_comments,
};
use crate::helpers::recovery::{JavaFormatField, format_malformed, resolve_required_field};
use jolt_fmt_ir::DocBuilder;
use jolt_java_syntax::{JavaSyntaxListPart, JavaSyntaxView, LocalTypeDeclarationSyntax};

pub(crate) fn format_block<'source>(
    block: &Block<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let open = match resolve_required_field(block.open_brace(), doc) {
        JavaFormatField::Present(open) => format_block_open_brace(&open, doc),
        JavaFormatField::Malformed(malformed) => malformed,
    };
    let body = match format_block_statements_body(block, doc) {
        BodyContent {
            doc: body,
            visible: true,
            ..
        } => {
            let body = doc_concat!(doc, [doc.hard_line(), body]);
            doc_concat!(doc, [doc_indent!(doc, body), doc.hard_line()])
        }
        BodyContent { doc: claims, .. } => doc_concat!(doc, [claims, doc.hard_line()]),
    };
    let close = match resolve_required_field(block.close_brace(), doc) {
        JavaFormatField::Present(close) => {
            format_token_after_relocated_leading_comments(doc, &close, TrailingTrivia::Preserve)
        }
        JavaFormatField::Malformed(malformed) => malformed,
    };
    doc_concat!(doc, [open, body, close])
}

fn format_block_open_brace<'source>(
    open: &JavaSyntaxToken<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_token_with_inline_leading_comments(
        doc,
        open,
        InlineLeadingTrivia::BeforeToken,
        TrailingTrivia::RelocatedToEnclosingContext,
    )
}

fn format_block_statements_body<'source>(
    block: &Block<'source>,
    doc: &mut DocBuilder<'source>,
) -> BodyContent<'source> {
    let statements = match resolve_required_field(block.statements(), doc) {
        JavaFormatField::Present(statements) => statements,
        JavaFormatField::Malformed(malformed) => {
            let mut items = Vec::new();
            items.extend(format_block_open_dangling_comments(block, doc));
            items.push(BodyItem::new(malformed, false));
            items.extend(format_block_close_dangling_comments(block, doc));
            return BodyContent::new(join_body_items(doc, items), true, true);
        }
    };
    let block_start = block.text_range().start().get();
    let ignored_ranges =
        formatter_ignore_ranges(block.source_text(), block_start, block.token_iter());
    let entries = statements.parts().collect::<Vec<_>>();
    let mut items = Vec::with_capacity(entries.len().saturating_add(2));
    items.extend(format_block_open_dangling_comments(block, doc));
    if ignored_ranges.is_empty() {
        items.extend(
            entries
                .iter()
                .filter_map(|entry| format_block_statement_part(entry, doc)),
        );
    } else {
        items.extend(format_block_statement_items_with_ignored(
            &entries,
            block_start,
            &ignored_ranges,
            doc,
        ));
    }
    items.extend(format_block_close_dangling_comments(block, doc));
    let present = !items.is_empty();
    let visible = items.iter().any(|item| item.visible);
    let contents = if present {
        join_body_items(doc, items)
    } else {
        Doc::nil()
    };
    BodyContent::new(contents, present, visible)
}

fn format_block_statement_items_with_ignored<'source>(
    entries: &[Result<
        JavaSyntaxListPart<'source, BlockStatement<'source>>,
        jolt_java_syntax::JavaSyntaxInvariantError,
    >],
    block_start: usize,
    ignored_ranges: &[FormatterIgnoreRange<'source>],
    doc: &mut DocBuilder<'source>,
) -> Vec<BodyItem<'source>> {
    let ranges = entries
        .iter()
        .map(|entry| block_statement_part_token_range(entry, block_start))
        .collect::<Vec<_>>();
    let runs = formatter_ignore_runs(ignored_ranges, &ranges);
    let mut items = Vec::with_capacity(entries.len().saturating_add(runs.len()));
    let mut ignored_index = 0;
    let mut skip_index = 0;
    for (index, entry) in entries.iter().enumerate() {
        while ignored_index < runs.len() && runs[ignored_index].insert_index == index {
            items.push(BodyItem::new(
                formatter_ignore_run_doc(&runs[ignored_index], doc),
                false,
            ));
            ignored_index += 1;
        }
        while skip_index < runs.len() && runs[skip_index].skip_end <= index {
            skip_index += 1;
        }
        if skip_index < runs.len() && runs[skip_index].skips(index) {
            continue;
        }
        if let Some(mut item) = format_block_statement_part(entry, doc) {
            if skip_index > 0 && runs[skip_index - 1].skip_end == index {
                item = item.without_blank_line_before();
            }
            items.push(item);
        }
    }
    while ignored_index < runs.len() {
        items.push(BodyItem::new(
            formatter_ignore_run_doc(&runs[ignored_index], doc),
            false,
        ));
        ignored_index += 1;
    }
    items
}

fn format_block_statement_part<'source>(
    entry: &Result<
        JavaSyntaxListPart<'source, BlockStatement<'source>>,
        jolt_java_syntax::JavaSyntaxInvariantError,
    >,
    doc: &mut DocBuilder<'source>,
) -> Option<BodyItem<'source>> {
    match entry {
        Ok(JavaSyntaxListPart::Item(statement)) => format_block_statement_item(statement, doc),
        Ok(JavaSyntaxListPart::Malformed(malformed)) => {
            Some(BodyItem::new(format_malformed(malformed, doc), false))
        }
        Ok(JavaSyntaxListPart::Missing(missing)) => Some(BodyItem::new(
            crate::helpers::recovery::format_missing(missing, doc),
            false,
        )),
        Ok(JavaSyntaxListPart::Separator(token)) => {
            doc.block_on_invariant("unseparated block statement list contained a separator");
            Some(BodyItem::new(
                crate::helpers::comments::format_token_with_comments(doc, token),
                false,
            ))
        }
        Err(error) => {
            doc.block_on_invariant(error.to_string());
            None
        }
    }
}

fn format_block_open_dangling_comments<'source>(
    block: &Block<'source>,
    doc: &mut DocBuilder<'source>,
) -> Option<BodyItem<'source>> {
    let jolt_java_syntax::JavaSyntaxField::Present(open) = block.open_brace().ok()? else {
        return None;
    };
    let comments = open.trailing_comments();
    (!comments.is_empty()).then(|| BodyItem::new(format_dangling_comments(doc, comments), false))
}

fn format_block_close_dangling_comments<'source>(
    block: &Block<'source>,
    doc: &mut DocBuilder<'source>,
) -> Option<BodyItem<'source>> {
    let jolt_java_syntax::JavaSyntaxField::Present(close) = block.close_brace().ok()? else {
        return None;
    };
    let comments = close.leading_comments();
    (!comments.is_empty()).then(|| BodyItem::new(format_dangling_comments(doc, comments), false))
}

fn block_statement_part_token_range(
    entry: &Result<
        JavaSyntaxListPart<'_, BlockStatement<'_>>,
        jolt_java_syntax::JavaSyntaxInvariantError,
    >,
    block_start: usize,
) -> Option<Range<usize>> {
    match entry {
        Ok(JavaSyntaxListPart::Item(statement)) => {
            block_statement_token_range(statement, block_start)
        }
        Ok(JavaSyntaxListPart::Separator(token)) => {
            Some(relative_token_range_between(token, token, block_start))
        }
        Ok(JavaSyntaxListPart::Malformed(malformed)) => {
            let syntax = malformed.syntax_node()?;
            Some(relative_token_range_between(
                &syntax.first_token()?,
                &syntax.last_token()?,
                block_start,
            ))
        }
        Ok(JavaSyntaxListPart::Missing(_)) | Err(_) => None,
    }
}

#[allow(clippy::map_unwrap_or, clippy::unnecessary_wraps)]
pub(crate) fn format_block_statement_item<'source>(
    statement: &BlockStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Option<BodyItem<'source>> {
    let starts_after_blank_line = statement.starts_after_blank_line();
    let item = match resolve_required_field(statement.item(), doc) {
        JavaFormatField::Present(item) => item,
        JavaFormatField::Malformed(malformed) => {
            return Some(BodyItem::new(malformed, starts_after_blank_line));
        }
    };
    let formatted = match item {
        BlockItem::EmptyStatement(empty) => {
            let (removed, visible) = format_removed_empty_statement(&empty, doc);
            return Some(if visible {
                BodyItem::new(removed, starts_after_blank_line)
            } else {
                BodyItem::invisible(removed)
            });
        }
        BlockItem::LocalVariableDeclaration(declaration) => doc_concat!(
            doc,
            [
                format_local_variable_declaration(&declaration, doc),
                format_statement_semicolon(statement.local_declaration_semicolon(), doc)
            ]
        ),
        BlockItem::LocalClassOrInterfaceDeclaration(declaration) => {
            match resolve_required_field(declaration.declaration(), doc) {
                JavaFormatField::Present(declaration) => match declaration.classify() {
                    Ok(LocalTypeDeclarationSyntax::ClassDeclaration(declaration)) => {
                        format_type_declaration(&declaration.into(), doc)
                    }
                    Ok(LocalTypeDeclarationSyntax::RecordDeclaration(declaration)) => {
                        format_type_declaration(&declaration.into(), doc)
                    }
                    Ok(LocalTypeDeclarationSyntax::EnumDeclaration(declaration)) => {
                        format_type_declaration(&declaration.into(), doc)
                    }
                    Ok(LocalTypeDeclarationSyntax::InterfaceDeclaration(declaration)) => {
                        format_type_declaration(&declaration.into(), doc)
                    }
                    Ok(LocalTypeDeclarationSyntax::AnnotationInterfaceDeclaration(declaration)) => {
                        format_type_declaration(&declaration.into(), doc)
                    }
                    Ok(LocalTypeDeclarationSyntax::BogusTypeDeclaration(declaration)) => {
                        format_type_declaration(&declaration.into(), doc)
                    }
                    Err(error) => {
                        doc.block_on_invariant(error.to_string());
                        Doc::nil()
                    }
                },
                JavaFormatField::Malformed(malformed) => malformed,
            }
        }
        BlockItem::Block(block) => format_block(&block, doc),
        BlockItem::BogusBlockItem(value) => format_malformed(&value, doc),
        BlockItem::BogusStatement(value) => format_malformed(&value, doc),
        BlockItem::LabeledStatement(statement) => format_statement(&statement.into(), doc),
        BlockItem::ExpressionStatement(statement) => format_statement(&statement.into(), doc),
        BlockItem::IfStatement(statement) => format_statement(&statement.into(), doc),
        BlockItem::AssertStatement(statement) => format_statement(&statement.into(), doc),
        BlockItem::SwitchStatement(statement) => format_statement(&statement.into(), doc),
        BlockItem::WhileStatement(statement) => format_statement(&statement.into(), doc),
        BlockItem::DoStatement(statement) => format_statement(&statement.into(), doc),
        BlockItem::ForStatement(statement) => format_statement(&statement.into(), doc),
        BlockItem::BreakStatement(statement) => format_statement(&statement.into(), doc),
        BlockItem::YieldStatement(statement) => format_statement(&statement.into(), doc),
        BlockItem::ContinueStatement(statement) => format_statement(&statement.into(), doc),
        BlockItem::ReturnStatement(statement) => format_statement(&statement.into(), doc),
        BlockItem::ThrowStatement(statement) => format_statement(&statement.into(), doc),
        BlockItem::SynchronizedStatement(statement) => format_statement(&statement.into(), doc),
        BlockItem::TryStatement(statement) => format_statement(&statement.into(), doc),
        BlockItem::TryWithResourcesStatement(statement) => format_statement(&statement.into(), doc),
    };
    Some(BodyItem::new(formatted, starts_after_blank_line))
}

fn format_removed_empty_statement<'source>(
    statement: &jolt_java_syntax::EmptyStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> (Doc<'source>, bool) {
    let visible = has_removed_comments(comments_from_tokens(statement.token_iter()));
    let removed = statement
        .separator_removal_claim()
        .map_or_else(Doc::nil, |claim| doc.removed_source(claim));
    let comments = format_removed_comments(doc, comments_from_tokens(statement.token_iter()))
        .unwrap_or_else(Doc::nil);
    (doc_concat!(doc, [removed, comments]), visible)
}

fn block_statement_token_range(
    statement: &BlockStatement<'_>,
    block_start: usize,
) -> Option<Range<usize>> {
    Some(relative_token_range_between(
        &statement.first_token()?,
        &statement.last_token()?,
        block_start,
    ))
}
