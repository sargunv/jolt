use super::member_bodies::{
    combine_comment_members, effective_members, format_body_close_dangling_comments,
    format_body_open_dangling_comments, format_class_member_body,
    format_empty_enum_constant_list_comments, format_enum_body_semicolon_comments, join_docs,
};
use super::{
    ClassBodyMember, Doc, EnumConstant, EnumConstantListEntry, JavaFormatter, JavaSyntaxToken,
    braced_body, comment_forces_line, comment_is_star_block, concat, format_argument_list,
    format_class_body, format_comment, format_dangling_comments, format_leading_comments,
    format_modifier_prefix_from_parts, format_token_sequence, format_token_text,
    format_trailing_comments, hard_line, is_formatter_control_marker, text,
};

pub(super) struct FormattedEnumConstant {
    doc: Doc,
    comma: Option<JavaSyntaxToken>,
}

pub(super) fn format_enum_body_contents(
    constants: Vec<FormattedEnumConstant>,
    body: &jolt_java_syntax::EnumBody,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc> {
    let members = body.members().collect::<Vec<_>>();
    let enum_semicolons = body.semicolon_tokens().collect::<Vec<_>>();
    let effective_members = effective_members(&members);
    let has_body_declarations = effective_members
        .iter()
        .any(|member| !matches!(member, ClassBodyMember::EmptyDeclaration(_)));
    let open_comments = combine_comment_members(
        combine_comment_members(
            format_body_open_dangling_comments(body.open_brace()),
            format_enum_body_semicolon_comments(&enum_semicolons),
        ),
        format_empty_enum_constant_list_comments(body.constants()),
    );
    let close_comments = format_body_close_dangling_comments(body.close_brace());
    if constants.is_empty()
        && effective_members.is_empty()
        && open_comments.is_none()
        && close_comments.is_none()
    {
        return None;
    }

    let mut moved_member_comments = Vec::new();
    let constants_doc = (!constants.is_empty()).then(|| {
        let constants_len = constants.len();
        let mut pending_constant_comments = Vec::new();
        let mut constant_lines = Vec::new();
        for (index, entry) in constants.into_iter().enumerate() {
            if !pending_constant_comments.is_empty() {
                constant_lines.push(format_dangling_comments(std::mem::take(
                    &mut pending_constant_comments,
                )));
            }

            let is_last_constant = index + 1 == constants_len;
            let separator = if !has_body_declarations || !is_last_constant {
                ","
            } else {
                ";"
            };
            let moved_comments = entry.comma.as_ref().map_or_else(Vec::new, |comma| {
                enum_separator_moved_comments(comma, has_body_declarations && is_last_constant)
            });
            if has_body_declarations && is_last_constant {
                moved_member_comments.extend(moved_comments);
            } else {
                pending_constant_comments.extend(moved_comments);
            }

            constant_lines.push(concat([
                entry.doc,
                format_enum_constant_separator(
                    entry.comma.as_ref(),
                    separator,
                    !has_body_declarations || !is_last_constant,
                ),
            ]));
        }

        if !pending_constant_comments.is_empty() {
            constant_lines.push(format_dangling_comments(pending_constant_comments));
        }

        join_docs(constant_lines, &hard_line())
    });

    let moved_member_comments = (!moved_member_comments.is_empty())
        .then(|| format_dangling_comments(moved_member_comments));
    let members_doc = format_class_member_body(
        &body.source_text(),
        body.text_range().start().get(),
        &members,
        open_comments,
        close_comments,
        formatter,
    );
    let members_doc = match (moved_member_comments, members_doc) {
        (Some(comments), Some(members)) => Some(concat([comments, hard_line(), members])),
        (Some(comments), None) => Some(comments),
        (None, members) => members,
    };

    match (constants_doc, members_doc) {
        (Some(constants), Some(members)) => {
            Some(concat([constants, jolt_fmt_ir::empty_line(), members]))
        }
        (Some(constants), None) => Some(constants),
        (None, Some(members)) if has_body_declarations => {
            Some(concat([text(";"), jolt_fmt_ir::empty_line(), members]))
        }
        (None, Some(members)) => Some(members),
        (None, None) => None,
    }
}

pub(super) fn format_enum_constant_entry(
    entry: EnumConstantListEntry,
    formatter: &JavaFormatter<'_>,
) -> FormattedEnumConstant {
    FormattedEnumConstant {
        doc: format_enum_constant(&entry.constant, formatter),
        comma: entry.comma,
    }
}

fn format_enum_constant_separator(
    comma: Option<&JavaSyntaxToken>,
    separator: &'static str,
    include_trailing_comments: bool,
) -> Doc {
    comma.map_or_else(
        || text(separator),
        |comma| {
            concat([
                format_leading_comments(comma),
                text(separator),
                if include_trailing_comments {
                    format_enum_separator_inline_trailing_comments(comma)
                } else {
                    jolt_fmt_ir::nil()
                },
            ])
        },
    )
}

fn format_enum_separator_inline_trailing_comments(comma: &JavaSyntaxToken) -> Doc {
    let comments = comma
        .trailing_comments()
        .into_iter()
        .filter(|comment| !enum_separator_comment_moves(comment))
        .collect::<Vec<_>>();

    let mut docs = Vec::new();
    for comment in comments {
        docs.push(text(" "));
        docs.push(format_comment(&comment));
    }
    concat(docs)
}

fn enum_separator_moved_comments(
    comma: &JavaSyntaxToken,
    move_all_trailing_comments: bool,
) -> Vec<jolt_java_syntax::JavaComment> {
    comma
        .trailing_comments()
        .into_iter()
        .filter(|comment| {
            !is_formatter_control_marker(comment.text())
                && (move_all_trailing_comments || enum_separator_comment_moves(comment))
        })
        .collect()
}

fn enum_separator_comment_moves(comment: &jolt_java_syntax::JavaComment) -> bool {
    comment.kind() != jolt_java_syntax::JavaCommentKind::Line
        && (comment_forces_line(comment) || comment_is_star_block(comment))
}

fn format_enum_constant(constant: &EnumConstant, formatter: &JavaFormatter<'_>) -> Doc {
    let tokens = constant.tokens();
    let Some(name) = constant.name() else {
        return format_token_sequence(&tokens);
    };

    concat([
        format_modifier_prefix_from_parts(constant.annotations().collect(), Vec::new(), formatter),
        format_leading_comments(&name),
        format_token_text(name.text()),
        format_trailing_comments(&name),
        constant
            .arguments()
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                format_argument_list(Some(arguments), formatter)
            }),
        constant.body().map_or_else(jolt_fmt_ir::nil, |body| {
            concat([text(" "), braced_body(format_class_body(&body, formatter))])
        }),
    ])
}
