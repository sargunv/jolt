use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    Annotation, AnnotationArgumentList, AnnotationUseSiteTarget, RecoveredSeparatedListEntry,
};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_token, format_token_sequence, token_has_comments,
};
use crate::helpers::lists::{CommaListItem, compact_parenthesized_list, parenthesized_list};
use crate::rules::expressions::format_value_argument;
use crate::rules::names::format_qualified_name;

pub(crate) fn format_annotation<'source>(
    doc: &mut DocBuilder<'source>,
    annotation: &Annotation<'source>,
) -> Doc<'source> {
    format_annotation_with_leading(doc, annotation, LeadingTrivia::Preserve)
}

pub(crate) fn format_annotation_with_leading<'source>(
    doc: &mut DocBuilder<'source>,
    annotation: &Annotation<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let at = if let Some(token) = annotation.at_token() {
        format_token(
            doc,
            &token,
            leading,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    } else {
        doc.nil()
    };
    let target = if let Some(target) = annotation.use_site_target() {
        format_annotation_use_site_target(doc, &target)
    } else {
        doc.nil()
    };
    let name = if let Some(name) = annotation.name() {
        format_qualified_name(doc, &name)
    } else {
        doc.nil()
    };
    let arguments = if let Some(arguments) = annotation.argument_list() {
        format_annotation_argument_list(doc, &arguments)
    } else {
        doc.nil()
    };
    doc.concat([at, target, name, arguments])
}

fn format_annotation_use_site_target<'source>(
    doc: &mut DocBuilder<'source>,
    target: &AnnotationUseSiteTarget<'source>,
) -> Doc<'source> {
    let target_token = if let Some(token) = target.target_token() {
        format_token(
            doc,
            &token,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    } else {
        doc.nil()
    };
    let colon = if let Some(token) = target.colon_token() {
        format_token(
            doc,
            &token,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    } else {
        doc.nil()
    };
    doc.concat([target_token, colon])
}

fn format_annotation_argument_list<'source>(
    doc: &mut DocBuilder<'source>,
    arguments: &AnnotationArgumentList<'source>,
) -> Doc<'source> {
    let AnnotationArgumentListItems {
        items,
        has_recovered_tokens,
    } = annotation_argument_list_items(doc, arguments);

    if has_recovered_tokens || annotation_argument_list_has_internal_comments(arguments) {
        return parenthesized_list(
            doc,
            arguments.open_paren().as_ref(),
            arguments.close_paren().as_ref(),
            items,
        );
    }

    compact_parenthesized_list(
        doc,
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
    doc: &mut DocBuilder<'source>,
    arguments: &AnnotationArgumentList<'source>,
) -> AnnotationArgumentListItems<'source> {
    let entries = arguments.entries_with_recovered();
    let (lower, _) = entries.size_hint();
    let mut items = Vec::with_capacity(lower);
    let mut has_recovered_tokens = false;

    for entry in entries {
        has_recovered_tokens |= !matches!(entry, RecoveredSeparatedListEntry::Entry(_));
        items.push(match entry {
            RecoveredSeparatedListEntry::Entry(entry) => CommaListItem {
                doc: format_value_argument(doc, &entry.argument),
                comma: entry.comma,
            },
            RecoveredSeparatedListEntry::Token(token) => CommaListItem {
                doc: format_token(
                    doc,
                    &token,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::Preserve,
                ),
                comma: None,
            },
            RecoveredSeparatedListEntry::Error(error) => CommaListItem {
                doc: format_token_sequence(doc, error.token_iter(), LeadingTrivia::Preserve),
                comma: None,
            },
            RecoveredSeparatedListEntry::Node(node) => CommaListItem {
                doc: format_token_sequence(doc, node.token_iter(), LeadingTrivia::Preserve),
                comma: None,
            },
        });
    }

    AnnotationArgumentListItems {
        items,
        has_recovered_tokens,
    }
}

fn annotation_argument_list_has_internal_comments(arguments: &AnnotationArgumentList<'_>) -> bool {
    let close = arguments.close_paren();

    arguments.token_iter().any(|token| {
        if Some(token) == close {
            return !token.leading_comments().is_empty();
        }
        token_has_comments(&token)
    })
}
