use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    Annotation, AnnotationArgumentList, AnnotationUseSiteTarget, KotlinRoleElement,
    KotlinSyntaxView, Name, QualifiedName, ValueArgumentListEntry,
};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_token, trailing_comments_force_line,
};
use crate::helpers::lists::{
    CommaListItem, annotation_parenthesized_list, delimited_comma_list, push_recovery_item,
};
use crate::helpers::recovery::{
    KotlinFormatField, KotlinFormatListPart, format_optional_field, format_required_field,
    join_delimited_recovery, resolve_list_part, resolve_required_delimiter, resolve_required_field,
};
use crate::rules::expressions::format_value_argument;
use crate::rules::names::{format_name, format_qualified_name};

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
    let sigil = format_required_field(annotation.sigil(), doc, |token, doc| {
        format_token(
            doc,
            &token,
            leading,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    });
    let target = format_optional_field(annotation.use_site_target(), doc, |target, doc| {
        format_annotation_use_site_target(doc, &target)
    });
    let has_arguments = matches!(
        annotation.argument_list(),
        Ok(jolt_kotlin_syntax::KotlinSyntaxField::Present(_))
    );
    let name = format_required_field(annotation.name(), doc, |element, doc| {
        let last_token = match element {
            KotlinRoleElement::Node(node) => node.last_token(),
            KotlinRoleElement::Token(token) => Some(token),
        };
        let needs_comment_space = has_arguments
            && last_token.is_some_and(|token| {
                !token.trailing_comments().is_empty() && !trailing_comments_force_line(&token)
            });
        let name = format_annotation_name(doc, element);
        if needs_comment_space {
            let space = doc.space();
            doc.concat([name, space])
        } else {
            name
        }
    });
    let arguments = format_optional_field(annotation.argument_list(), doc, |arguments, doc| {
        format_annotation_argument_list(doc, &arguments)
    });
    doc.concat([sigil, target, name, arguments])
}

fn format_annotation_name<'source>(
    doc: &mut DocBuilder<'source>,
    element: KotlinRoleElement<'source>,
) -> Doc<'source> {
    if let Some(name) = element.cast_node::<Name<'source>>() {
        format_name(doc, &name)
    } else if let Some(name) = element.cast_node::<QualifiedName<'source>>() {
        format_qualified_name(doc, &name)
    } else {
        doc.block_on_invariant("invalid annotation name role");
        Doc::nil()
    }
}

fn format_annotation_use_site_target<'source>(
    doc: &mut DocBuilder<'source>,
    target: &AnnotationUseSiteTarget<'source>,
) -> Doc<'source> {
    let target_token = format_required_field(target.target(), doc, |element, doc| {
        let Some(token) = element.token() else {
            doc.block_on_invariant("annotation use-site target is not a token");
            return Doc::nil();
        };
        format_token(
            doc,
            &token,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    });
    let colon = format_required_field(target.colon(), doc, |token, doc| {
        format_token(
            doc,
            &token,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    });
    doc.concat([target_token, colon])
}

fn format_annotation_argument_list<'source>(
    doc: &mut DocBuilder<'source>,
    arguments: &AnnotationArgumentList<'source>,
) -> Doc<'source> {
    let open = resolve_required_delimiter(arguments.open_paren(), doc);
    let close = resolve_required_delimiter(arguments.close_paren(), doc);
    let items = annotation_argument_list_items(doc, arguments);
    let list = if arguments.is_recovery_free() {
        annotation_parenthesized_list(doc, open.source(), close.source(), items)
    } else {
        delimited_comma_list(doc, open.source(), close.source(), items)
    };
    join_delimited_recovery(doc, &open, list, &close)
}

fn annotation_argument_list_items<'source>(
    doc: &mut DocBuilder<'source>,
    arguments: &AnnotationArgumentList<'source>,
) -> Vec<CommaListItem<'source>> {
    let entries = match resolve_required_field(arguments.entries(), doc) {
        KotlinFormatField::Present(entries) => entries,
        KotlinFormatField::Malformed(recovery) => {
            return vec![CommaListItem {
                doc: recovery,
                comma: None,
                layout_visible: true,
            }];
        }
    };

    let mut items = Vec::new();
    for part in entries.parts() {
        match resolve_list_part(part, doc) {
            KotlinFormatListPart::Item(argument) => {
                let formatted = match argument {
                    ValueArgumentListEntry::ValueArgument(argument) => {
                        format_value_argument(doc, &argument)
                    }
                    ValueArgumentListEntry::BogusValueArgument(bogus) => {
                        crate::helpers::recovery::format_malformed(&bogus, doc)
                    }
                };
                items.push(CommaListItem {
                    doc: formatted,
                    comma: None,
                    layout_visible: true,
                });
            }
            KotlinFormatListPart::Separator(comma) => {
                if let Some(item) = items.iter_mut().rev().find(|item| item.layout_visible)
                    && item.comma.is_none()
                {
                    item.comma = Some(comma);
                } else {
                    items.push(CommaListItem {
                        doc: format_token(
                            doc,
                            &comma,
                            LeadingTrivia::Preserve,
                            TrailingTrivia::Preserve,
                        ),
                        comma: None,
                        layout_visible: true,
                    });
                }
            }
            KotlinFormatListPart::Malformed(recovery) => {
                push_recovery_item(&mut items, recovery, true);
            }
            KotlinFormatListPart::Invisible(recovery) => {
                push_recovery_item(&mut items, recovery, false);
            }
        }
    }
    items
}
