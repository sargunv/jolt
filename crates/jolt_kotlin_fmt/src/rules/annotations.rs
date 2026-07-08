use jolt_fmt_ir::{Doc, concat};
use jolt_kotlin_syntax::{
    Annotation, AnnotationArgumentList, AnnotationUseSiteTarget, RecoveredSeparatedListEntry,
};

use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token, token_has_comments};
use crate::helpers::lists::{
    CommaListItem, compact_parenthesized_list, parenthesized_list, recovered_comma_list_items,
};
use crate::rules::expressions::format_value_argument;
use crate::rules::names::format_qualified_name;

pub(crate) fn format_annotation<'source>(annotation: &Annotation<'source>) -> Doc<'source> {
    format_annotation_with_leading(annotation, LeadingTrivia::Preserve)
}

pub(crate) fn format_annotation_with_leading<'source>(
    annotation: &Annotation<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    concat([
        annotation
            .at_token()
            .map_or_else(jolt_fmt_ir::nil, |token| {
                format_token(&token, leading, TrailingTrivia::RelocatedToEnclosingContext)
            }),
        annotation
            .use_site_target()
            .map_or_else(jolt_fmt_ir::nil, |target| {
                format_annotation_use_site_target(&target)
            }),
        annotation
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_qualified_name(&name)),
        annotation
            .argument_list()
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                format_annotation_argument_list(&arguments)
            }),
    ])
}

fn format_annotation_use_site_target<'source>(
    target: &AnnotationUseSiteTarget<'source>,
) -> Doc<'source> {
    concat([
        target
            .target_token()
            .map_or_else(jolt_fmt_ir::nil, |token| {
                format_token(
                    &token,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::RelocatedToEnclosingContext,
                )
            }),
        target.colon_token().map_or_else(jolt_fmt_ir::nil, |token| {
            format_token(
                &token,
                LeadingTrivia::Preserve,
                TrailingTrivia::RelocatedToEnclosingContext,
            )
        }),
    ])
}

fn format_annotation_argument_list<'source>(
    arguments: &AnnotationArgumentList<'source>,
) -> Doc<'source> {
    let AnnotationArgumentListItems {
        items,
        has_recovered_tokens,
    } = annotation_argument_list_items(arguments);

    if has_recovered_tokens || annotation_argument_list_has_internal_comments(arguments) {
        return parenthesized_list(
            arguments.open_paren().as_ref(),
            arguments.close_paren().as_ref(),
            items,
        );
    }

    compact_parenthesized_list(
        arguments.open_paren().as_ref(),
        arguments.close_paren().as_ref(),
        items,
    )
}

struct AnnotationArgumentListItems<'source> {
    items: Vec<CommaListItem<'source>>,
    has_recovered_tokens: bool,
}

fn annotation_argument_list_items<'source>(
    arguments: &AnnotationArgumentList<'source>,
) -> AnnotationArgumentListItems<'source> {
    let entries = arguments.entries_with_recovered().collect::<Vec<_>>();
    let has_recovered_tokens = entries
        .iter()
        .any(|entry| !matches!(entry, RecoveredSeparatedListEntry::Entry(_)));
    let items = recovered_comma_list_items(entries, |entry| CommaListItem {
        doc: format_value_argument(&entry.argument),
        comma: entry.comma,
    });

    AnnotationArgumentListItems {
        items,
        has_recovered_tokens,
    }
}

fn annotation_argument_list_has_internal_comments(arguments: &AnnotationArgumentList<'_>) -> bool {
    let close = arguments.close_paren();
    let close_range = close
        .as_ref()
        .map(jolt_kotlin_syntax::KotlinSyntaxToken::token_text_range);

    arguments.token_iter().any(|token| {
        if Some(token.token_text_range()) == close_range {
            return !token.leading_comments().is_empty();
        }
        token_has_comments(&token)
    })
}
