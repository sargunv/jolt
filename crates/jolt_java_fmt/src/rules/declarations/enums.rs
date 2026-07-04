use super::member_bodies::{
    combine_comment_members, format_body_close_dangling_comments,
    format_body_open_dangling_comments, format_class_member_body,
    format_empty_enum_constant_list_comments,
};
use super::{
    ClassBodyMember, Doc, EnumConstant, FormattedMember, JavaFormatter, JavaSyntaxToken,
    braced_body, comment_forces_line, comment_is_star_block, comments_from_tokens, concat,
    format_argument_list, format_class_body, format_comment, format_dangling_comments,
    format_leading_comments, format_modifier_prefix_from_parts, format_removed_comments,
    format_token_with_comments, formatter_ignore_ranges, hard_line, is_formatter_control_marker,
    text,
};

struct FormattedEnumConstant<'source> {
    doc: Doc<'source>,
    comma: Option<JavaSyntaxToken<'source>>,
}

pub(super) fn format_enum_body_contents<'source>(
    body: &jolt_java_syntax::EnumBody<'source>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc<'source>> {
    let mut constants = body
        .constants()
        .into_iter()
        .flat_map(|constants| constants.entries())
        .map(|entry| format_enum_constant_entry(&entry, formatter))
        .peekable();
    let has_constants = constants.peek().is_some();
    let has_body_declarations = body
        .members()
        .any(|member| !matches!(member, ClassBodyMember::EmptyDeclaration(_)));
    let open_comments = combine_comment_members(
        combine_comment_members(
            format_body_open_dangling_comments(body.open_brace()),
            format_removed_comments(comments_from_tokens(body.semicolon_tokens()))
                .map(FormattedMember::comment),
        ),
        format_empty_enum_constant_list_comments(body.constants()),
    );
    let close_comments = format_body_close_dangling_comments(body.close_brace());
    let ignored_ranges = formatter_ignore_ranges(
        body.source_text(),
        body.text_range().start().get(),
        body.token_iter(),
    );
    let members_doc = format_class_member_body(
        body.text_range().start().get(),
        &ignored_ranges,
        body.members(),
        open_comments,
        close_comments,
        formatter,
    );
    if !has_constants && members_doc.is_none() {
        return None;
    }

    let mut moved_member_comments = Vec::new();
    let constants_doc = has_constants.then(|| {
        let mut pending_constant_comments = Vec::new();
        let mut constant_lines = Vec::new();
        while let Some(entry) = constants.next() {
            if !pending_constant_comments.is_empty() {
                constant_lines.push(format_dangling_comments(std::mem::take(
                    &mut pending_constant_comments,
                )));
            }

            let is_last_constant = constants.peek().is_none();
            let separator = if !has_body_declarations || !is_last_constant {
                ","
            } else {
                ";"
            };
            if let Some(comma) = entry.comma {
                let moved_comments =
                    enum_separator_moved_comments(comma, has_body_declarations && is_last_constant);
                if has_body_declarations && is_last_constant {
                    moved_member_comments.extend(moved_comments);
                } else {
                    pending_constant_comments.extend(moved_comments);
                }
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

        jolt_fmt_ir::join(&hard_line(), constant_lines)
    });

    let moved_member_comments = (!moved_member_comments.is_empty())
        .then(|| format_dangling_comments(moved_member_comments));
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

fn format_enum_constant_entry<'source>(
    entry: &jolt_java_syntax::EnumConstantListEntry<'source>,
    formatter: &JavaFormatter<'_>,
) -> FormattedEnumConstant<'source> {
    FormattedEnumConstant {
        doc: format_enum_constant(&entry.constant, formatter),
        comma: entry.comma,
    }
}

fn format_enum_constant_separator<'source>(
    comma: Option<&JavaSyntaxToken<'source>>,
    separator: &'static str,
    include_trailing_comments: bool,
) -> Doc<'source> {
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

fn format_enum_separator_inline_trailing_comments<'source>(
    comma: &JavaSyntaxToken<'source>,
) -> Doc<'source> {
    let mut docs = Vec::new();
    for comment in comma
        .trailing_comments()
        .filter(|comment| !enum_separator_comment_moves(comment))
    {
        docs.push(text(" "));
        docs.push(format_comment(&comment));
    }
    concat(docs)
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
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    concat([
        format_modifier_prefix_from_parts(constant.annotations().collect(), Vec::new(), formatter),
        constant
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_token_with_comments(&name)),
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
