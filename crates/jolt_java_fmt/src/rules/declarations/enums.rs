use super::member_bodies::{
    combine_comment_members, format_body_close_dangling_comments,
    format_body_open_dangling_comments, format_class_member_body,
    format_empty_enum_constant_list_comments,
};
use super::{
    ClassBodyMember, Doc, EnumConstant, FormattedMember, JavaSyntaxToken, comment_forces_line,
    comment_is_star_block, comments_from_tokens, format_argument_list, format_class_body,
    format_comment, format_dangling_comments, format_modifier_prefix_from_parts,
    format_removed_comments, format_token_with_comments, formatter_ignore_ranges,
    is_formatter_control_marker, source_braced_body,
};
use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_leading_comments, format_token, format_token_sequence,
};
use crate::helpers::syntax_tokens::{
    FormatterInsertedToken, format_token_with_normalized_text, inserted_syntax_token,
};
use jolt_fmt_ir::{DocBuilder, DocList};
struct FormattedEnumConstant<'source> {
    doc: Doc<'source>,
    comma: Option<JavaSyntaxToken<'source>>,
    is_recovered: bool,
}

pub(super) fn format_enum_body_contents<'source>(
    body: &jolt_java_syntax::EnumBody<'source>,
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    let mut constants = Vec::new();
    if let Some(constant_list) = body.constants() {
        constants.extend(enum_constant_entries(constant_list, doc));
    }
    let has_constants = constants.iter().any(|constant| !constant.is_recovered);
    let has_body_declarations = body
        .members()
        .any(|member| !matches!(member, ClassBodyMember::EmptyDeclaration(_)));
    let body_declaration_separator = body.body_declaration_separator();
    let open_dangling_comments = format_body_open_dangling_comments(body.open_brace(), doc);
    let semicolon_comments =
        format_removed_comments(doc, comments_from_tokens(body.semicolon_tokens()))
            .map(FormattedMember::comment);
    let open_comments = combine_comment_members(doc, open_dangling_comments, semicolon_comments);
    let empty_constant_comments = format_empty_enum_constant_list_comments(body.constants(), doc);
    let open_comments = combine_comment_members(doc, open_comments, empty_constant_comments);
    let close_comments = format_body_close_dangling_comments(body.close_brace(), doc);
    let ignored_ranges = formatter_ignore_ranges(
        body.source_text(),
        body.text_range().start().get(),
        body.token_iter(),
    );
    let members_doc = format_class_member_body(
        body.text_range().start().get(),
        &ignored_ranges,
        body.members_with_recovered(),
        open_comments,
        close_comments,
        doc,
    );
    if !has_constants && members_doc.is_none() {
        return None;
    }

    let mut moved_member_comments = Vec::new();
    let constants_doc = has_constants.then(|| {
        format_enum_constants_doc(
            doc,
            &constants,
            has_body_declarations,
            body_declaration_separator.as_ref(),
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

    match (constants_doc, members_doc) {
        (Some(constants), Some(members)) => {
            Some(doc_concat!(doc, [constants, doc.empty_line(), members]))
        }
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
    }
}

fn format_enum_constants_doc<'source>(
    doc: &mut DocBuilder<'source>,
    constants: &[FormattedEnumConstant<'source>],
    has_body_declarations: bool,
    body_declaration_separator: Option<&JavaSyntaxToken<'source>>,
    moved_member_comments: &mut Vec<jolt_java_syntax::JavaComment<'source>>,
) -> Doc<'source> {
    let mut pending_constant_comments = Vec::new();
    let mut constant_lines = doc.list();
    let mut has_constant_line = false;
    for (index, entry) in constants.iter().enumerate() {
        if !pending_constant_comments.is_empty() {
            let comments =
                format_dangling_comments(doc, std::mem::take(&mut pending_constant_comments));
            push_hard_line_separated(&mut constant_lines, &mut has_constant_line, comments, doc);
        }

        if entry.is_recovered {
            push_hard_line_separated(&mut constant_lines, &mut has_constant_line, entry.doc, doc);
            continue;
        }

        let is_last_constant = !constants[index + 1..]
            .iter()
            .any(|constant| !constant.is_recovered);
        let separator = if !has_body_declarations || !is_last_constant {
            ","
        } else {
            ";"
        };
        if let Some(comma) = entry.comma.as_ref() {
            let moved_comments =
                enum_separator_moved_comments(*comma, has_body_declarations && is_last_constant);
            if has_body_declarations && is_last_constant {
                moved_member_comments.extend(moved_comments);
            } else {
                pending_constant_comments.extend(moved_comments);
            }
        }

        let constant_doc = doc_concat!(
            doc,
            [
                entry.doc,
                format_enum_constant_separator(
                    doc,
                    entry.comma.as_ref(),
                    is_last_constant
                        .then_some(body_declaration_separator)
                        .flatten(),
                    separator,
                    !has_body_declarations || !is_last_constant,
                ),
            ]
        );
        push_hard_line_separated(
            &mut constant_lines,
            &mut has_constant_line,
            constant_doc,
            doc,
        );
    }

    if !pending_constant_comments.is_empty() {
        let comments = format_dangling_comments(doc, pending_constant_comments);
        push_hard_line_separated(&mut constant_lines, &mut has_constant_line, comments, doc);
    }

    constant_lines.finish(doc)
}

fn push_hard_line_separated<'source>(
    docs: &mut DocList<'source>,
    has_doc: &mut bool,
    next: Doc<'source>,
    doc: &mut DocBuilder<'source>,
) {
    if *has_doc {
        let separator = doc.hard_line();
        docs.push(separator, doc);
    }
    docs.push(next, doc);
    *has_doc = true;
}

fn enum_constant_entries<'source>(
    constants: jolt_java_syntax::EnumConstantList<'source>,
    doc: &mut DocBuilder<'source>,
) -> Vec<FormattedEnumConstant<'source>> {
    let mut entries = Vec::new();
    for entry in constants.entries_with_recovered() {
        entries.push(match entry {
            jolt_java_syntax::RecoveredSeparatedListEntry::Entry(entry) => {
                format_enum_constant_entry(&entry, doc)
            }
            jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => FormattedEnumConstant {
                doc: format_token(
                    doc,
                    &token,
                    LeadingTrivia::Preserve,
                    TrailingTrivia::Preserve,
                ),
                comma: None,
                is_recovered: true,
            },
            jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => FormattedEnumConstant {
                doc: format_token_sequence(doc, error.token_iter(), LeadingTrivia::Preserve),
                comma: None,
                is_recovered: true,
            },
            jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => FormattedEnumConstant {
                doc: format_token_sequence(doc, node.token_iter(), LeadingTrivia::Preserve),
                comma: None,
                is_recovered: true,
            },
        });
    }
    entries
}

fn format_enum_constant_entry<'source>(
    entry: &jolt_java_syntax::EnumConstantListEntry<'source>,
    doc: &mut DocBuilder<'source>,
) -> FormattedEnumConstant<'source> {
    FormattedEnumConstant {
        doc: format_enum_constant(&entry.constant, doc),
        comma: entry.comma,
        is_recovered: false,
    }
}

fn format_enum_constant_separator<'source>(
    doc: &mut DocBuilder<'source>,
    comma: Option<&JavaSyntaxToken<'source>>,
    body_declaration_separator: Option<&JavaSyntaxToken<'source>>,
    separator: &'static str,
    include_trailing_comments: bool,
) -> Doc<'source> {
    if let Some(body_declaration_separator) = body_declaration_separator
        && separator == ";"
    {
        return format_enum_body_declaration_separator(doc, Some(body_declaration_separator));
    }

    let Some(separator_token) = body_declaration_separator.or(comma) else {
        return if separator == "," {
            // Intentional synthesized token: multiline enum constants use a
            // doc-owned trailing comma even when the source omitted one.
            inserted_syntax_token(doc, ",", FormatterInsertedToken::TrailingComma)
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
                format_token_with_normalized_text(
                    doc,
                    separator_token,
                    separator,
                    FormatterInsertedToken::EnumSeparator,
                    LeadingTrivia::SuppressAlreadyHandled,
                    TrailingTrivia::RelocatedToEnclosingContext,
                )
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
    let mut docs = doc.list();
    for comment in comments {
        docs.push(doc.space(), doc);
        docs.push(format_comment(doc, &comment), doc);
    }
    docs.finish(doc)
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
    doc_concat!(
        doc,
        [
            format_modifier_prefix_from_parts(constant.annotations(), Vec::new(), doc),
            constant
                .name()
                .map_or_else(Doc::nil, |name| format_token_with_comments(doc, &name)),
            constant
                .arguments()
                .map_or_else(Doc::nil, |arguments| format_argument_list(
                    Some(arguments),
                    doc
                ),),
            constant.body().map_or_else(Doc::nil, |body| {
                let open = body.open_brace();
                let close = body.close_brace();
                let body_doc = format_class_body(&body, doc);
                doc_concat!(
                    doc,
                    [
                        doc.space(),
                        source_braced_body(doc, open.as_ref(), close.as_ref(), body_doc),
                    ]
                )
            },),
        ]
    )
}
