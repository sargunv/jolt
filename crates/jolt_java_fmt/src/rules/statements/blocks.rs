use super::{
    Block, BlockItem, BlockStatement, BodyItem, Doc, FormatterIgnoreItemRange,
    FormatterIgnoreSplice, LeadingTrivia, TrailingTrivia, comments_from_tokens,
    for_each_formatter_ignore_splice, format_dangling_comments, format_local_variable_declaration,
    format_statement, format_statement_semicolon, format_token, format_type_declaration,
    formatter_ignore_content_range, formatter_ignore_run_doc, join_body_items,
};
use crate::helpers::blocks::BodyContent;
use crate::helpers::comments::{
    format_line_start_construct, format_token_after_relocated_leading_comments,
    format_token_removal, has_removed_comments,
};
use crate::helpers::recovery::{
    JavaFormatField, field_is_claim_only, format_malformed, present_token, resolve_required_field,
};
use jolt_fmt_ir::DocBuilder;
use jolt_fmt_ir::formatter_ignore::{
    FormatterIgnoreRun, formatter_ignore_runs_claim_boundary_comment,
};
use jolt_java_syntax::{JavaSyntaxListPart, JavaSyntaxView, LocalTypeDeclarationSyntax};

pub(crate) fn format_block<'source>(
    block: &Block<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    // A comment leading the open brace follows line-start knowledge: a block
    // that begins its own line (a nested block statement or an instance
    // initializer) is registered by the enclosing join and keeps the comment
    // on a line of its own, while a block behind a header inlines it between
    // the header and the brace, where the reparse reads it back identically.
    let open = match resolve_required_field(block.open_brace(), doc) {
        JavaFormatField::Present(open) => format_token(
            doc,
            &open,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        ),
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

fn format_block_statements_body<'source>(
    block: &Block<'source>,
    doc: &mut DocBuilder<'source>,
) -> BodyContent<'source> {
    let statements_field = block.statements();
    let statements = match resolve_required_field(statements_field, doc) {
        JavaFormatField::Present(statements) => statements,
        JavaFormatField::Malformed(malformed) => {
            // Salvaged open-brace comments that are the tail of a block that
            // never closes take the unterminated-tail placement.
            let unterminated_tail = present_token(block.close_brace()).is_none()
                && field_is_claim_only(&statements_field);
            let mut items = Vec::new();
            items.extend(format_block_open_dangling_comments(
                block,
                unterminated_tail,
                doc,
            ));
            items.push(BodyItem::new(malformed, false));
            items.extend(format_block_close_dangling_comments(block, &[], doc));
            return BodyContent::new(join_body_items(doc, items), true, true);
        }
    };
    let entries = statements.parts().collect::<Vec<_>>();
    let open = present_token(block.open_brace());
    let close = present_token(block.close_brace());
    let container = formatter_ignore_content_range(statements.text_range(), open, close);
    let runs = doc.formatter_ignore_runs(
        container,
        entries.iter().map(block_statement_part_ignore_range),
    );
    let mut items = Vec::with_capacity(entries.len().saturating_add(2));
    let entry_items: Vec<BodyItem<'source>> = if runs.is_empty() {
        entries
            .iter()
            .map(|entry| format_block_statement_part(entry, doc))
            .collect()
    } else {
        format_block_statement_items_with_ignored(&entries, &runs, doc)
    };
    let close_item = format_block_close_dangling_comments(block, &runs, doc);
    // Salvaged open-brace comments that are the only visible content of a
    // block that never closes take the unterminated-tail placement.
    let unterminated_tail = close.is_none() && entry_items.iter().all(|item| !item.visible);
    items.extend(format_block_open_dangling_comments(
        block,
        unterminated_tail,
        doc,
    ));
    items.extend(entry_items);
    items.extend(close_item);
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
    entries: &[JavaSyntaxListPart<'source, BlockStatement<'source>>],
    runs: &[FormatterIgnoreRun<'source>],
    doc: &mut DocBuilder<'source>,
) -> Vec<BodyItem<'source>> {
    let mut items = Vec::with_capacity(entries.len().saturating_add(runs.len()));
    for_each_formatter_ignore_splice(entries.len(), runs, |event| match event {
        FormatterIgnoreSplice::Ignore(run) => {
            items.push(BodyItem::new(formatter_ignore_run_doc(run, doc), false));
        }
        FormatterIgnoreSplice::Item { index, .. } => {
            items.push(format_block_statement_part(&entries[index], doc));
        }
        FormatterIgnoreSplice::End { .. } => {}
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
    at_root_margin: bool,
    doc: &mut DocBuilder<'source>,
) -> Option<BodyItem<'source>> {
    let jolt_java_syntax::JavaSyntaxField::Present(open) = block.open_brace() else {
        return None;
    };
    let comments = open.trailing_comments();
    (!comments.is_empty()).then(|| {
        let comments = format_dangling_comments(doc, comments);
        let comments = if at_root_margin {
            doc.root_margin(comments)
        } else {
            comments
        };
        BodyItem::new(comments, false)
    })
}

fn format_block_close_dangling_comments<'source>(
    block: &Block<'source>,
    runs: &[FormatterIgnoreRun<'source>],
    doc: &mut DocBuilder<'source>,
) -> Option<BodyItem<'source>> {
    let jolt_java_syntax::JavaSyntaxField::Present(close) = block.close_brace() else {
        return None;
    };
    let comments = close
        .leading_comments()
        .filter(|comment| !formatter_ignore_runs_claim_boundary_comment(runs, comment))
        .collect::<Vec<_>>();
    // The gap that opens the close brace's leading trivia belongs to that
    // token, so the separator in front of this run reads it from there.
    (!comments.is_empty()).then(|| {
        BodyItem::new(
            format_dangling_comments(doc, comments),
            close.has_leading_blank_line(),
        )
    })
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
    // A statement in a body begins its own line, so its first token's leading
    // comments keep lines of their own.
    format_line_start_construct(doc, statement.first_token(), |doc| {
        format_block_statement_item_at_line_start(statement, doc)
    })
}

#[allow(clippy::map_unwrap_or)]
fn format_block_statement_item_at_line_start<'source>(
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
            let (removed, visible, ends_before_blank_line) =
                format_removed_empty_statement(&empty, doc);
            return if visible {
                BodyItem::removed_comments(removed, starts_after_blank_line, ends_before_blank_line)
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
) -> (Doc<'source>, bool, bool) {
    let comments = comments_from_tokens(statement.token_iter()).collect::<Vec<_>>();
    let has_comments = has_removed_comments(comments.iter().copied());
    let ends_before_blank_line = comments
        .last()
        .is_some_and(jolt_java_syntax::JavaComment::is_followed_by_blank_line);
    let jolt_java_syntax::JavaSyntaxField::Present(semicolon) = statement.semicolon() else {
        return (
            format_statement_semicolon(statement.semicolon(), doc),
            true,
            false,
        );
    };
    let (normalized, removed) =
        format_token_removal(doc, &semicolon, statement.separator_removal_claim());
    (normalized, has_comments || !removed, ends_before_blank_line)
}

fn block_statement_ignore_range(
    statement: &BlockStatement<'_>,
) -> Option<FormatterIgnoreItemRange> {
    Some(FormatterIgnoreItemRange::between(
        &statement.first_token()?,
        &statement.last_token()?,
    ))
}
