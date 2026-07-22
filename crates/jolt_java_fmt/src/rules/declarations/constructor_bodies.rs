use super::{
    BodyItem, ConstructorInvocation, Doc, FormatterIgnoreItemRange, FormatterIgnoreSplice,
    JavaSyntaxToken, for_each_formatter_ignore_splice, format_argument_list,
    format_block_statement_item, format_construct_leading_comments, format_dangling_comments,
    format_expression, format_name, format_removed_comments, format_statement_semicolon,
    format_token_after_construct_leading_comments, format_token_with_comments,
    format_type_argument_list, formatter_ignore_content_range, formatter_ignore_run_doc,
    join_body_items,
};
use jolt_fmt_ir::DocBuilder;
use jolt_java_syntax::{ConstructorBodyEntry, JavaSyntaxField, JavaSyntaxListPart, JavaSyntaxView};
use jolt_text::TextRange;

use crate::helpers::recovery::{
    JavaFormatField, format_malformed, format_optional_field, format_required_field,
    resolve_optional_field,
};

pub(super) fn format_constructor_body<'source>(
    body: &jolt_java_syntax::ConstructorBody<'source>,
    open: Option<JavaSyntaxToken<'source>>,
    close: Option<JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    let start = body.text_range().start();
    let empty_fallback = TextRange::new(start, start);
    let (elements, fallback) = match body.entries() {
        JavaSyntaxField::Present(entries) => (
            constructor_body_elements(&entries, doc),
            entries.text_range(),
        ),
        JavaSyntaxField::Malformed(malformed) => (
            vec![constructor_body_recovery(&malformed, doc)],
            syntax_view_range(&malformed, empty_fallback),
        ),
        JavaSyntaxField::Missing(missing) => (
            vec![ConstructorBodyElement::Recovery {
                doc: crate::helpers::recovery::format_missing(&missing, doc),
                first: None,
                last: None,
            }],
            empty_fallback,
        ),
    };
    let container = formatter_ignore_content_range(fallback, open, close);
    let ignored_runs = doc.formatter_ignore_runs(
        container,
        elements.iter().map(constructor_body_element_ignore_range),
    );
    if ignored_runs.is_empty() {
        let mut items = Vec::with_capacity(elements.len().saturating_add(2));
        items.extend(format_constructor_body_open_dangling_comments(doc, open));
        items.extend(
            elements
                .iter()
                .filter_map(|element| format_constructor_body_element(element, doc)),
        );
        items.extend(format_constructor_body_close_dangling_comments(doc, close));
        return (!items.is_empty()).then(|| join_body_items(doc, items));
    }
    let mut items = Vec::with_capacity(
        elements
            .len()
            .saturating_add(ignored_runs.len())
            .saturating_add(2),
    );
    items.extend(format_constructor_body_open_dangling_comments(doc, open));
    for_each_formatter_ignore_splice(elements.len(), &ignored_runs, |event| match event {
        FormatterIgnoreSplice::Ignore(run) => {
            items.push(BodyItem::new(formatter_ignore_run_doc(run, doc), false));
        }
        FormatterIgnoreSplice::Item {
            index,
            clear_blank_line_before,
        } => {
            let Some(mut item) = format_constructor_body_element(&elements[index], doc) else {
                return;
            };
            if clear_blank_line_before {
                item = item.without_blank_line_before();
            }
            items.push(item);
        }
    });
    items.extend(format_constructor_body_close_dangling_comments(doc, close));

    (!items.is_empty()).then(|| join_body_items(doc, items))
}

fn syntax_view_range<'source>(
    view: &impl JavaSyntaxView<'source>,
    fallback: TextRange,
) -> TextRange {
    view.syntax_node()
        .map_or(fallback, |syntax| syntax.text_range())
}

fn format_constructor_body_open_dangling_comments<'source>(
    doc: &mut jolt_fmt_ir::DocBuilder<'source>,
    open: Option<JavaSyntaxToken<'source>>,
) -> Option<BodyItem<'source>> {
    format_removed_comments(doc, open?.trailing_comments())
        .map(|comments| BodyItem::new(comments, false))
}

fn format_constructor_body_close_dangling_comments<'source>(
    doc: &mut jolt_fmt_ir::DocBuilder<'source>,
    close: Option<JavaSyntaxToken<'source>>,
) -> Option<BodyItem<'source>> {
    let comments = close?.leading_comments();
    (!comments.is_empty()).then(|| BodyItem::new(format_dangling_comments(doc, comments), false))
}

#[derive(Clone, Copy)]
enum ConstructorBodyElement<'source> {
    Invocation(jolt_java_syntax::ConstructorInvocation<'source>),
    Statement(jolt_java_syntax::BlockStatement<'source>),
    Recovery {
        doc: Doc<'source>,
        first: Option<JavaSyntaxToken<'source>>,
        last: Option<JavaSyntaxToken<'source>>,
    },
}

fn constructor_body_elements<'source>(
    entries: &jolt_java_syntax::ConstructorBodyEntryList<'source>,
    doc: &mut DocBuilder<'source>,
) -> Vec<ConstructorBodyElement<'source>> {
    entries
        .parts()
        .filter_map(|part| match part {
            JavaSyntaxListPart::Item(ConstructorBodyEntry::ConstructorInvocation(item)) => {
                Some(ConstructorBodyElement::Invocation(item))
            }
            JavaSyntaxListPart::Item(ConstructorBodyEntry::BlockStatement(item)) => {
                Some(ConstructorBodyElement::Statement(item))
            }
            JavaSyntaxListPart::Item(ConstructorBodyEntry::BogusConstructorBodyEntry(item)) => {
                Some(constructor_body_recovery(&item, doc))
            }
            JavaSyntaxListPart::Malformed(malformed) => {
                Some(constructor_body_recovery(&malformed, doc))
            }
            JavaSyntaxListPart::Missing(missing) => Some(ConstructorBodyElement::Recovery {
                doc: crate::helpers::recovery::format_missing(&missing, doc),
                first: None,
                last: None,
            }),
            JavaSyntaxListPart::Separator(_) => {
                doc.block_on_invariant("constructor body had an unexpected separator");
                None
            }
        })
        .collect()
}

fn constructor_body_recovery<'source>(
    view: &impl JavaSyntaxView<'source>,
    doc: &mut DocBuilder<'source>,
) -> ConstructorBodyElement<'source> {
    ConstructorBodyElement::Recovery {
        doc: format_malformed(view, doc),
        first: view.first_token(),
        last: view.syntax_node().and_then(|syntax| syntax.last_token()),
    }
}

fn constructor_body_element_ignore_range(
    element: &ConstructorBodyElement<'_>,
) -> Option<FormatterIgnoreItemRange> {
    match element {
        ConstructorBodyElement::Invocation(node) => Some(FormatterIgnoreItemRange::between(
            &node.first_token()?,
            &node.last_token()?,
        )),
        ConstructorBodyElement::Statement(node) => Some(FormatterIgnoreItemRange::between(
            &node.first_token()?,
            &node.last_token()?,
        )),
        ConstructorBodyElement::Recovery { first, last, .. } => Some(
            FormatterIgnoreItemRange::between(first.as_ref()?, last.as_ref()?),
        ),
    }
}

fn format_constructor_body_element<'source>(
    element: &ConstructorBodyElement<'source>,
    doc: &mut DocBuilder<'source>,
) -> Option<BodyItem<'source>> {
    match element {
        ConstructorBodyElement::Invocation(invocation) => Some(BodyItem::new(
            format_constructor_invocation(invocation, doc),
            invocation
                .first_token()
                .is_some_and(|token| token.has_leading_blank_line()),
        )),
        ConstructorBodyElement::Statement(statement) => format_block_statement_item(statement, doc),
        ConstructorBodyElement::Recovery { doc, .. } => Some(BodyItem::new(*doc, false)),
    }
}

fn format_constructor_invocation<'source>(
    invocation: &ConstructorInvocation<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let type_arguments =
        format_optional_field(invocation.type_arguments(), doc, |arguments, doc| {
            format_type_argument_list(&arguments, doc)
        });
    let target = format_required_field(invocation.target(), doc, |target, doc| {
        format_token_after_construct_leading_comments(
            doc,
            &target,
            invocation.first_token().as_ref(),
        )
    });
    let arguments = format_required_field(invocation.arguments(), doc, |arguments, doc| {
        format_argument_list(arguments, doc)
    });
    let semicolon = invocation.semicolon();
    let invocation_first_token = invocation.first_token();
    doc_concat!(
        doc,
        [
            format_construct_leading_comments(doc, invocation_first_token.as_ref()),
            format_constructor_invocation_qualifier(invocation, doc),
            type_arguments,
            target,
            arguments,
            format_statement_semicolon(semicolon, doc),
        ]
    )
}

fn format_constructor_invocation_qualifier<'source>(
    invocation: &ConstructorInvocation<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let qualifier = match resolve_optional_field(invocation.qualifier(), doc) {
        JavaFormatField::Present(Some(qualifier)) => {
            if let Some(name) = qualifier.cast_family::<jolt_java_syntax::NameSyntax<'source>>() {
                format_name(&name, doc)
            } else if let Some(expression) =
                qualifier.cast_family::<jolt_java_syntax::Expression<'source>>()
            {
                format_expression(&expression, doc)
            } else {
                doc.block_on_invariant("constructor invocation qualifier had an undeclared kind");
                Doc::nil()
            }
        }
        JavaFormatField::Present(None) => Doc::nil(),
        JavaFormatField::Malformed(malformed) => malformed,
    };
    let dot = format_optional_field(invocation.dot(), doc, |dot, doc| {
        format_token_with_comments(doc, &dot)
    });
    doc_concat!(doc, [qualifier, dot])
}
