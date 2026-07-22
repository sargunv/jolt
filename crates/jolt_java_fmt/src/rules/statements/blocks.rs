use super::{
    Block, BlockItem, BlockStatement, BodyItem, Doc, FormatterIgnoreItemRange,
    FormatterIgnoreSplice, JavaSyntaxToken, TrailingTrivia, comments_from_tokens,
    for_each_formatter_ignore_splice, format_dangling_comments, format_local_variable_declaration,
    format_statement, format_statement_semicolon, format_type_declaration,
    formatter_ignore_content_range, formatter_ignore_run_doc, join_body_items,
};
use crate::helpers::blocks::BodyContent;
use crate::helpers::comments::{
    InlineLeadingTrivia, format_token_after_relocated_leading_comments, format_token_removal,
    format_token_with_inline_leading_comments, has_removed_comments,
};
use crate::helpers::recovery::{JavaFormatField, format_malformed, resolve_required_field};
use jolt_fmt_ir::DocBuilder;
use jolt_fmt_ir::formatter_ignore::FormatterIgnoreRun;
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
    let entries = statements.parts().collect::<Vec<_>>();
    let open = present_block_token(block.open_brace());
    let close = present_block_token(block.close_brace());
    let container = formatter_ignore_content_range(statements.text_range(), open, close);
    let runs = doc.formatter_ignore_runs(
        container,
        entries.iter().map(block_statement_part_ignore_range),
    );
    let mut items = Vec::with_capacity(entries.len().saturating_add(2));
    items.extend(format_block_open_dangling_comments(block, doc));
    if runs.is_empty() {
        items.extend(
            entries
                .iter()
                .map(|entry| format_block_statement_part(entry, doc)),
        );
    } else {
        items.extend(format_block_statement_items_with_ignored(
            &entries, &runs, doc,
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

fn present_block_token<'source>(
    field: jolt_java_syntax::JavaSyntaxField<'source, JavaSyntaxToken<'source>>,
) -> Option<JavaSyntaxToken<'source>> {
    match field {
        jolt_java_syntax::JavaSyntaxField::Present(token) => Some(token),
        _ => None,
    }
}

fn format_block_statement_items_with_ignored<'source>(
    entries: &[JavaSyntaxListPart<'source, BlockStatement<'source>>],
    runs: &[FormatterIgnoreRun<'source>],
    doc: &mut DocBuilder<'source>,
) -> Vec<BodyItem<'source>> {
    let mut items = Vec::with_capacity(entries.len().saturating_add(runs.len()));
    for_each_formatter_ignore_splice(entries.len(), runs, |event| match event {
        FormatterIgnoreSplice::Ignore(run) => {
            items.push(BodyItem::new(formatter_ignore_run_doc(run, doc), false));
        }
        FormatterIgnoreSplice::Item {
            index,
            clear_blank_line_before,
        } => {
            let mut item = format_block_statement_part(&entries[index], doc);
            if clear_blank_line_before {
                item = item.without_blank_line_before();
            }
            items.push(item);
        }
    });
    items
}

fn format_block_statement_part<'source>(
    entry: &JavaSyntaxListPart<'source, BlockStatement<'source>>,
    doc: &mut DocBuilder<'source>,
) -> BodyItem<'source> {
    match entry {
        JavaSyntaxListPart::Item(statement) => format_block_statement_item(statement, doc),
        JavaSyntaxListPart::Malformed(malformed) => {
            BodyItem::new(format_malformed(malformed, doc), false)
        }
        JavaSyntaxListPart::Missing(missing) => BodyItem::new(
            crate::helpers::recovery::format_missing(missing, doc),
            false,
        ),
        JavaSyntaxListPart::Separator(token) => {
            doc.block_on_invariant("unseparated block statement list contained a separator");
            BodyItem::new(
                crate::helpers::comments::format_token_with_comments(doc, token),
                false,
            )
        }
    }
}

fn format_block_open_dangling_comments<'source>(
    block: &Block<'source>,
    doc: &mut DocBuilder<'source>,
) -> Option<BodyItem<'source>> {
    let jolt_java_syntax::JavaSyntaxField::Present(open) = block.open_brace() else {
        return None;
    };
    let comments = open.trailing_comments();
    (!comments.is_empty()).then(|| BodyItem::new(format_dangling_comments(doc, comments), false))
}

fn format_block_close_dangling_comments<'source>(
    block: &Block<'source>,
    doc: &mut DocBuilder<'source>,
) -> Option<BodyItem<'source>> {
    let jolt_java_syntax::JavaSyntaxField::Present(close) = block.close_brace() else {
        return None;
    };
    let comments = close.leading_comments();
    (!comments.is_empty()).then(|| BodyItem::new(format_dangling_comments(doc, comments), false))
}

fn block_statement_part_ignore_range(
    entry: &JavaSyntaxListPart<'_, BlockStatement<'_>>,
) -> Option<FormatterIgnoreItemRange> {
    match entry {
        JavaSyntaxListPart::Item(statement) => block_statement_ignore_range(statement),
        JavaSyntaxListPart::Separator(token) => {
            Some(FormatterIgnoreItemRange::between(token, token))
        }
        JavaSyntaxListPart::Malformed(malformed) => {
            let syntax = malformed.syntax_node()?;
            Some(FormatterIgnoreItemRange::between(
                &syntax.first_token()?,
                &syntax.last_token()?,
            ))
        }
        JavaSyntaxListPart::Missing(_) => None,
    }
}

#[allow(clippy::map_unwrap_or)]
pub(crate) fn format_block_statement_item<'source>(
    statement: &BlockStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> BodyItem<'source> {
    // Recovery trivia can be repartitioned when an adjacent line comment is
    // relocated by structured formatting. Only recovery-free statements may
    // use it to request a blank separator; recovered statements receive the
    // block's canonical one-line boundary around their smallest verbatim core.
    let starts_after_blank_line =
        statement.is_recovery_free() && statement.starts_after_blank_line();
    let item = match resolve_required_field(statement.item(), doc) {
        JavaFormatField::Present(item) => item,
        JavaFormatField::Malformed(malformed) => {
            return BodyItem::new(malformed, starts_after_blank_line);
        }
    };
    let formatted = match item {
        BlockItem::EmptyStatement(empty) => {
            let (removed, visible) = format_removed_empty_statement(&empty, doc);
            return if visible {
                BodyItem::new(removed, starts_after_blank_line)
            } else {
                BodyItem::invisible(removed)
            };
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
    BodyItem::new(formatted, starts_after_blank_line)
}

fn format_removed_empty_statement<'source>(
    statement: &jolt_java_syntax::EmptyStatement<'source>,
    doc: &mut DocBuilder<'source>,
) -> (Doc<'source>, bool) {
    let has_comments = has_removed_comments(comments_from_tokens(statement.token_iter()));
    let jolt_java_syntax::JavaSyntaxField::Present(semicolon) = statement.semicolon() else {
        return (format_statement_semicolon(statement.semicolon(), doc), true);
    };
    let (normalized, removed) =
        format_token_removal(doc, &semicolon, statement.separator_removal_claim());
    (normalized, has_comments || !removed)
}

fn block_statement_ignore_range(
    statement: &BlockStatement<'_>,
) -> Option<FormatterIgnoreItemRange> {
    Some(FormatterIgnoreItemRange::between(
        &statement.first_token()?,
        &statement.last_token()?,
    ))
}
