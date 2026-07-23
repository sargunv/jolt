use super::member_bodies::{
    combine_comment_members, format_body_close_dangling_comments,
    format_body_open_dangling_comments, format_class_member_body,
    format_empty_enum_constant_list_comments,
};
use super::{
    Doc, EnumConstant, FormattedMember, JavaSyntaxToken, comment_forces_line,
    comment_is_star_block, comments_from_tokens, format_argument_list, format_class_body,
    format_comment, format_dangling_comments, format_removed_comments, format_token_with_comments,
    formatter_ignore_content_range, is_formatter_control_marker, source_braced_body,
};
use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_leading_comments, format_token, format_token_doc,
};
use crate::helpers::recovery::{
    JavaFormatField, JavaFormatListPart, format_optional_field, format_required_field,
    resolve_list_part, resolve_optional_field, resolve_required_delimiter, resolve_required_field,
};
use jolt_fmt_ir::{ConcatBuilder, DocBuilder};
use jolt_java_syntax::NormalizedToken;

struct FormattedEnumConstant<'source> {
    doc: Doc<'source>,
    comma: Option<JavaSyntaxToken<'source>>,
    is_malformed: bool,
}

#[allow(clippy::too_many_lines)]
pub(super) fn format_enum_body_contents<'source>(
    body: &jolt_java_syntax::EnumBody<'source>,
    doc: &mut DocBuilder<'source>,
) -> crate::helpers::blocks::BodyContent<'source> {
    let open = present_token(body.open_brace());
    let close = present_token(body.close_brace());
    let mut constants = Vec::new();
    let constant_list = match resolve_optional_field(body.constants(), doc) {
        JavaFormatField::Present(value) => value,
        JavaFormatField::Malformed(recovery) => {
            constants.push(FormattedEnumConstant {
                doc: recovery,
                comma: None,
                is_malformed: true,
            });
            None
        }
    };
    constants.extend(
        constant_list
            .into_iter()
            .flat_map(|constants| enum_constant_entries(constants, doc)),
    );
    let (body_declaration_separator, separator_recovery) =
        match resolve_optional_field(body.body_separator(), doc) {
            JavaFormatField::Present(value) => (value, None),
            JavaFormatField::Malformed(recovery) => (None, Some(recovery)),
        };
    let has_constants = constants.iter().any(|constant| !constant.is_malformed);
    let has_constant_entries = !constants.is_empty();
    let resolved_members = resolve_required_field(body.members(), doc);
    let resolved_has_body_declarations = match &resolved_members {
        JavaFormatField::Present(members) => members.parts().any(|part| match part {
            jolt_java_syntax::JavaSyntaxListPart::Item(
                jolt_java_syntax::ClassBodyMember::EmptyDeclaration(_),
            ) => false,
            jolt_java_syntax::JavaSyntaxListPart::Item(_)
            | jolt_java_syntax::JavaSyntaxListPart::Malformed(_) => true,
            _ => false,
        }),
        JavaFormatField::Malformed(_) => false,
    };
    let separator_has_structured_destination =
        has_constant_entries || resolved_has_body_declarations;
    let open_dangling_comments = format_body_open_dangling_comments(open, doc);
    let semicolon_comments = body_declaration_separator.map(|separator| {
        let token = match body.redundant_body_separator_removal_claim() {
            Some(claim) => doc.removed_source(claim),
            None if separator_has_structured_destination => Doc::nil(),
            None => format_token_with_comments(doc, &separator),
        };
        match format_removed_comments(doc, comments_from_tokens([separator])) {
            Some(comments) => FormattedMember::comment(doc_concat!(doc, [token, comments])),
            None => FormattedMember::invisible(token),
        }
    });
    let open_comments = combine_comment_members(doc, open_dangling_comments, semicolon_comments);
    let empty_constant_comments = format_empty_enum_constant_list_comments(constant_list, doc);
    let open_comments = combine_comment_members(doc, open_comments, empty_constant_comments);
    let close_comments = format_body_close_dangling_comments(close, doc);
    let (members_doc, has_body_declarations) = match resolved_members {
        JavaFormatField::Present(members) => {
            let member_start = body_declaration_separator.map_or_else(
                || {
                    if has_constant_entries {
                        members.text_range().start()
                    } else {
                        open.map_or(members.text_range().start(), |token| {
                            token.token_text_range().end()
                        })
                    }
                },
                |token| token.token_text_range().end(),
            );
            let container = formatter_ignore_content_range(
                jolt_text::TextRange::new(member_start, body.text_range().end()),
                None,
                close,
            );
            (
                format_class_member_body(
                    container,
                    members.parts(),
                    open_comments,
                    close_comments,
                    doc,
                ),
                resolved_has_body_declarations,
            )
        }
        JavaFormatField::Malformed(recovery) => {
            let comments = combine_comment_members(doc, open_comments, close_comments)
                .map(|comments| comments.doc);
            let recovery = comments.map_or(recovery, |comments| {
                doc_concat!(doc, [comments, doc.hard_line(), recovery])
            });
            (
                crate::helpers::blocks::BodyContent::new(recovery, true, true),
                true,
            )
        }
    };
    let members_visible = members_doc.visible;
    let members_doc = members_doc.present.then_some(members_doc.doc);
    if !has_constant_entries && members_doc.is_none() && separator_recovery.is_none() {
        return crate::helpers::blocks::BodyContent::new(Doc::nil(), false, false);
    }

    let mut moved_member_comments = Vec::new();
    let constants_doc = has_constant_entries.then(|| {
        format_enum_constants_doc(
            doc,
            body,
            &constants,
            has_body_declarations,
            body_declaration_separator.as_ref(),
            separator_recovery.is_some(),
            &mut moved_member_comments,
        )
    });

    let moved_member_comments = (!moved_member_comments.is_empty())
        .then(|| format_dangling_comments(doc, moved_member_comments));
    let members_doc = match (moved_member_comments, members_doc) {
        (Some(comments), Some(members)) => {
            Some(doc_concat!(doc, [comments, doc.hard_line(), members]))
        }
        (Some(comments), None) => Some(comments),
        (None, members) => members,
    };

    let separator_recovery = separator_recovery.map(|recovery| {
        if has_constants {
            doc_concat!(doc, [doc.hard_line(), recovery])
        } else {
            recovery
        }
    });
    let constants_doc = match (constants_doc, separator_recovery) {
        (Some(constants), Some(recovery)) => Some(doc_concat!(doc, [constants, recovery])),
        (constants, None) => constants,
        (None, recovery) => recovery,
    };

    let contents = match (constants_doc, members_doc) {
        (Some(constants), Some(members)) if members_visible => {
            Some(doc_concat!(doc, [constants, doc.empty_line(), members]))
        }
        (Some(constants), Some(members)) => Some(doc_concat!(doc, [constants, members])),
        (Some(constants), None) => Some(constants),
        (None, Some(members)) if has_body_declarations => Some(doc_concat!(
            doc,
            [
                format_enum_body_declaration_separator(doc, body_declaration_separator.as_ref()),
                doc.empty_line(),
                members,
            ]
        )),
        (None, Some(members)) => Some(members),
        (None, None) => None,
    };
    crate::helpers::blocks::BodyContent::new(
        contents.unwrap_or_else(Doc::nil),
        contents.is_some(),
        has_constant_entries || members_visible,
    )
}

fn format_enum_constants_doc<'source>(
    doc: &mut DocBuilder<'source>,
    body: &jolt_java_syntax::EnumBody<'source>,
    constants: &[FormattedEnumConstant<'source>],
    has_body_declarations: bool,
    body_declaration_separator: Option<&JavaSyntaxToken<'source>>,
    has_body_separator_recovery: bool,
    moved_member_comments: &mut Vec<jolt_java_syntax::JavaComment<'source>>,
) -> Doc<'source> {
    let mut pending_constant_comments = Vec::new();
    let mut has_constant_line = false;
    let last_constant_index = constants
        .iter()
        .rposition(|constant| !constant.is_malformed);
    doc.concat_list(|constant_lines| {
        for (index, entry) in constants.iter().enumerate() {
            if !pending_constant_comments.is_empty() {
                let comments = format_dangling_comments(
                    constant_lines,
                    std::mem::take(&mut pending_constant_comments),
                );
                push_hard_line_separated(constant_lines, &mut has_constant_line, comments);
            }

            if entry.is_malformed {
                push_hard_line_separated(constant_lines, &mut has_constant_line, entry.doc);
                continue;
            }

            let is_last_constant = Some(index) == last_constant_index;
            let separator = if is_last_constant
                && !has_body_separator_recovery
                && (has_body_declarations || body_declaration_separator.is_some())
            {
                ";"
            } else {
                ","
            };
            if let Some(comma) = entry.comma.as_ref() {
                let moved_comments = enum_separator_moved_comments(
                    *comma,
                    has_body_declarations && is_last_constant,
                );
                if has_body_declarations && is_last_constant {
                    moved_member_comments.extend(moved_comments);
                } else {
                    pending_constant_comments.extend(moved_comments);
                }
            }

            let constant_doc = doc_concat!(
                constant_lines,
                [
                    entry.doc,
                    format_enum_constant_separator(
                        constant_lines,
                        body,
                        entry.comma.as_ref(),
                        is_last_constant
                            .then_some(body_declaration_separator)
                            .flatten(),
                        separator,
                        !has_body_declarations || !is_last_constant,
                    ),
                ]
            );
            push_hard_line_separated(constant_lines, &mut has_constant_line, constant_doc);
        }

        if !pending_constant_comments.is_empty() {
            let comments = format_dangling_comments(constant_lines, pending_constant_comments);
            push_hard_line_separated(constant_lines, &mut has_constant_line, comments);
        }
    })
}

fn push_hard_line_separated<'source>(
    docs: &mut ConcatBuilder<'_, 'source>,
    has_doc: &mut bool,
    next: Doc<'source>,
) {
    if *has_doc {
        let hard_line = docs.hard_line();
        docs.push(hard_line);
    }
    docs.push(next);
    *has_doc = true;
}

fn enum_constant_entries<'source>(
    constants: jolt_java_syntax::EnumConstantList<'source>,
    doc: &mut DocBuilder<'source>,
) -> Vec<FormattedEnumConstant<'source>> {
    let parts = constants.parts();
    let (lower, _) = parts.size_hint();
    let mut entries = Vec::with_capacity(lower);
    for part in parts {
        match resolve_list_part(part, doc) {
            JavaFormatListPart::Item(constant) => entries.push(FormattedEnumConstant {
                doc: format_enum_constant(&constant, doc),
                comma: None,
                is_malformed: false,
            }),
            JavaFormatListPart::Separator(comma) => {
                if let Some(entry) = entries.last_mut() {
                    entry.comma = Some(comma);
                } else {
                    doc.block_on_invariant("enum separator had no preceding constant");
                }
            }
            JavaFormatListPart::Malformed(malformed) => entries.push(FormattedEnumConstant {
                doc: malformed,
                comma: None,
                is_malformed: true,
            }),
        }
    }
    entries
}

fn format_enum_constant_separator<'source>(
    doc: &mut DocBuilder<'source>,
    body: &jolt_java_syntax::EnumBody<'source>,
    comma: Option<&JavaSyntaxToken<'source>>,
    body_declaration_separator: Option<&JavaSyntaxToken<'source>>,
    separator: &'static str,
    include_trailing_comments: bool,
) -> Doc<'source> {
    if let Some(body_declaration_separator) = body_declaration_separator
        && separator == ";"
    {
        let comma = comma.map_or_else(Doc::nil, |comma| {
            match body.redundant_constant_separator_removal_claim(comma) {
                Some(claim) => doc.removed_source(claim),
                None => format_enum_constant_separator(doc, body, Some(comma), None, ",", false),
            }
        });
        return doc_concat!(
            doc,
            [
                comma,
                format_enum_body_declaration_separator(doc, Some(body_declaration_separator)),
            ]
        );
    }

    let Some(separator_token) = body_declaration_separator.or(comma) else {
        return if separator == "," {
            // Intentional synthesized token: multiline enum constants use a
            // doc-owned trailing comma even when the source omitted one.
            body.trailing_comma_claim()
                .map_or_else(Doc::nil, |claim| doc.synthesized_source(claim))
        } else {
            Doc::nil()
        };
    };

    doc_concat!(
        doc,
        [
            format_leading_comments(doc, separator_token),
            if separator_token.text() == separator {
                format_token(
                    doc,
                    separator_token,
                    LeadingTrivia::SuppressAlreadyHandled,
                    TrailingTrivia::RelocatedToEnclosingContext,
                )
            } else {
                let normalized = if separator == "," {
                    NormalizedToken::EnumComma
                } else {
                    NormalizedToken::EnumSemicolon
                };
                match body.separator_replacement_claim(separator_token, normalized) {
                    Some(claim) => {
                        let token_doc = doc.replaced_source(claim);
                        format_token_doc(
                            doc,
                            separator_token,
                            token_doc,
                            LeadingTrivia::SuppressAlreadyHandled,
                            TrailingTrivia::RelocatedToEnclosingContext,
                        )
                    }
                    None => format_token(
                        doc,
                        separator_token,
                        LeadingTrivia::SuppressAlreadyHandled,
                        TrailingTrivia::RelocatedToEnclosingContext,
                    ),
                }
            },
            if include_trailing_comments {
                format_enum_separator_inline_trailing_comments(doc, separator_token)
            } else {
                Doc::nil()
            },
        ]
    )
}

fn format_enum_body_declaration_separator<'source>(
    doc: &mut DocBuilder<'source>,
    separator: Option<&JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    separator.map_or_else(Doc::nil, |separator| {
        format_token(
            doc,
            separator,
            LeadingTrivia::SuppressAlreadyHandled,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    })
}

fn format_enum_separator_inline_trailing_comments<'source>(
    doc: &mut DocBuilder<'source>,
    comma: &JavaSyntaxToken<'source>,
) -> Doc<'source> {
    let comments = comma
        .trailing_comments()
        .filter(|comment| !enum_separator_comment_moves(comment));
    doc.concat_list(|docs| {
        for comment in comments {
            let space = docs.space();
            docs.push(space);
            let comment_doc = format_comment(docs, &comment);
            docs.push(comment_doc);
        }
    })
}

fn enum_separator_moved_comments(
    comma: JavaSyntaxToken<'_>,
    move_all_trailing_comments: bool,
) -> impl Iterator<Item = jolt_java_syntax::JavaComment<'_>> + use<'_> {
    comma.trailing_comments().filter(move |comment| {
        !is_formatter_control_marker(comment.text())
            && (move_all_trailing_comments || enum_separator_comment_moves(comment))
    })
}

fn enum_separator_comment_moves(comment: &jolt_java_syntax::JavaComment<'_>) -> bool {
    comment.kind() != jolt_java_syntax::JavaCommentKind::Line
        && (comment_forces_line(comment) || comment_is_star_block(comment))
}

fn format_enum_constant<'source>(
    constant: &EnumConstant<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let annotations = format_required_field(constant.annotations(), doc, |annotations, doc| {
        format_enum_constant_annotations(annotations, doc)
    });
    let name = format_required_field(constant.name(), doc, |name, doc| {
        format_token_with_comments(doc, &name)
    });
    let arguments = format_optional_field(constant.arguments(), doc, |arguments, doc| {
        format_argument_list(arguments, doc)
    });
    let body = format_optional_field(constant.body(), doc, |body, doc| {
        let open = resolve_required_delimiter(body.open_brace(), doc);
        let close = resolve_required_delimiter(body.close_brace(), doc);
        let body_doc = format_class_body(&body, doc);
        doc_concat!(
            doc,
            [doc.space(), source_braced_body(doc, open, close, body_doc),]
        )
    });
    doc_concat!(doc, [annotations, name, arguments, body])
}

fn format_enum_constant_annotations<'source>(
    annotations: jolt_java_syntax::AnnotationList<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for part in annotations.parts() {
            let annotation = match resolve_list_part(part, docs) {
                JavaFormatListPart::Item(annotation) => {
                    crate::rules::annotations::format_annotation(&annotation, docs)
                }
                JavaFormatListPart::Malformed(malformed) => malformed,
                JavaFormatListPart::Separator(separator) => {
                    docs.block_on_invariant("annotation list contained a separator");
                    format_token_with_comments(docs, &separator)
                }
            };
            docs.push(annotation);
            let hard_line = docs.hard_line();
            docs.push(hard_line);
        }
    })
}

fn present_token<'source>(
    field: jolt_java_syntax::JavaSyntaxField<'source, JavaSyntaxToken<'source>>,
) -> Option<JavaSyntaxToken<'source>> {
    match field {
        jolt_java_syntax::JavaSyntaxField::Present(token) => Some(token),
        jolt_java_syntax::JavaSyntaxField::Missing(_)
        | jolt_java_syntax::JavaSyntaxField::Malformed(_) => None,
    }
}
