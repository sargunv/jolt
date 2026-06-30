use jolt_diagnostics::TextRange;

use super::{
    Doc, FormatResult, JavaFormatContext, JavaSyntaxKind, Type, TypeArgumentList, TypeLayoutPart,
    concat, format_annotation, format_token, java_lists, reject_unhandled_comments_before_start,
    take_inline_leading_block_comment_docs_in_range, text,
};
use jolt_fmt_ir::{best_fitting, group, indent_by, join, line};

pub(super) fn format_type(ty: &Type, context: &mut JavaFormatContext<'_>) -> FormatResult<Doc> {
    let parts = ty.layout_parts();
    format_type_layout_parts(&parts, context)
}

pub(super) fn format_type_clause_type(
    ty: &Type,
    has_multiple_clause_types: bool,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let parts = ty.layout_parts();
    format_type_layout_sequence(
        &parts,
        TypeArgumentListSlot::TypeClause {
            has_multiple_clause_types,
        },
        context,
    )
}

pub(super) fn format_type_argument_list(
    arguments: &TypeArgumentList,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let parts = arguments.layout_parts();
    format_type_layout_parts(&parts, context)
}

pub(super) fn format_selector_type_argument_list_variants(
    arguments: &TypeArgumentList,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<(Doc, Doc)> {
    let parts = arguments.layout_parts();
    let Some((open_index, close_index)) = outer_type_argument_bounds(&parts) else {
        let doc = format_type_layout_parts(&parts, context)?;
        return Ok((doc.clone(), doc));
    };
    let open = match &parts[open_index] {
        TypeLayoutPart::Token(token) => token.token_text_range(),
        _ => unreachable!("type argument open should be a token"),
    };
    let close = match &parts[close_index] {
        TypeLayoutPart::Token(token) => token.token_text_range(),
        _ => unreachable!("type argument close should be a token"),
    };
    let list_range = TextRange::new(open.start(), close.end());
    format_selector_type_argument_part_variants(
        &parts[open_index + 1..close_index],
        list_range,
        context,
    )
}

pub(super) fn format_type_layout_parts(
    parts: &[TypeLayoutPart],
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    format_type_layout_sequence(parts, TypeArgumentListSlot::Default, context)
}

#[derive(Clone, Copy)]
enum TypeArgumentListSlot {
    Default,
    NestedGeneric,
    TypeClause { has_multiple_clause_types: bool },
}

fn format_type_layout_sequence(
    parts: &[TypeLayoutPart],
    type_argument_slot: TypeArgumentListSlot,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let mut docs = Vec::new();
    let mut previous_was_annotation = false;
    let mut previous_was_dot = false;
    let mut previous_was_space_text = false;
    let mut previous_dot_range = None;
    let mut index = 0;
    while index < parts.len() {
        let part = &parts[index];
        match part {
            TypeLayoutPart::Text(value) => {
                if previous_was_annotation {
                    docs.push(text(" "));
                }
                previous_was_space_text = value.chars().all(char::is_whitespace);
                previous_was_annotation = false;
                previous_was_dot = false;
                previous_dot_range = None;
                docs.push(text(*value));
            }
            TypeLayoutPart::Annotation(annotation) => {
                let inserted_qualified_comments = previous_was_dot
                    && annotation.code_text_range().is_some_and(|range| {
                        append_qualified_segment_comments(
                            &mut docs,
                            previous_dot_range,
                            range,
                            context,
                        )
                    });
                if !docs.is_empty()
                    && !previous_was_dot
                    && !previous_was_space_text
                    && !inserted_qualified_comments
                {
                    docs.push(text(" "));
                }
                docs.push(format_annotation(annotation, context, "type-use")?);
                previous_was_annotation = true;
                previous_was_dot = false;
                previous_was_space_text = false;
                previous_dot_range = None;
            }
            TypeLayoutPart::Token(token) => {
                if token.kind() == JavaSyntaxKind::Lt
                    && let Some(close_index) = matching_type_argument_close(parts, index)
                {
                    let open = token.token_text_range();
                    let close = match &parts[close_index] {
                        TypeLayoutPart::Token(token) => token.token_text_range(),
                        _ => unreachable!("matching type argument close should be a token"),
                    };
                    let list_range = TextRange::new(open.start(), close.end());
                    let arguments = &parts[index + 1..close_index];
                    let type_arguments = match type_argument_slot {
                        TypeArgumentListSlot::Default => {
                            format_type_argument_parts(arguments, list_range, context)?
                        }
                        TypeArgumentListSlot::NestedGeneric => {
                            format_nested_type_argument_parts(arguments, list_range, context)?
                        }
                        TypeArgumentListSlot::TypeClause {
                            has_multiple_clause_types,
                        } => format_type_clause_type_argument_parts(
                            arguments,
                            list_range,
                            has_multiple_clause_types,
                            context,
                        )?,
                    };
                    docs.push(type_arguments);
                    previous_was_annotation = false;
                    previous_was_dot = false;
                    previous_dot_range = None;
                    index = close_index + 1;
                    continue;
                }
                if previous_was_dot && token.kind() == JavaSyntaxKind::Identifier {
                    append_qualified_segment_comments(
                        &mut docs,
                        previous_dot_range,
                        token.token_text_range(),
                        context,
                    );
                }
                if previous_was_annotation && token_needs_space_after_annotation(token.kind()) {
                    reject_unhandled_comments_before_start(
                        context,
                        token.token_text_range(),
                        "Java formatter does not support comments between type-use annotations and types yet",
                    )?;
                    docs.push(text(" "));
                }
                previous_was_dot = token.kind() == JavaSyntaxKind::Dot;
                previous_dot_range = previous_was_dot.then_some(token.token_text_range());
                previous_was_annotation = false;
                previous_was_space_text = false;
                docs.push(format_token(token));
            }
        }
        index += 1;
    }

    Ok(concat(docs))
}

fn append_qualified_segment_comments(
    docs: &mut Vec<Doc>,
    previous_dot_range: Option<TextRange>,
    segment_range: TextRange,
    context: &mut JavaFormatContext<'_>,
) -> bool {
    let Some(dot_range) = previous_dot_range else {
        return false;
    };
    let owner_range = TextRange::new(dot_range.end(), segment_range.start());
    let comments =
        take_inline_leading_block_comment_docs_in_range(context, owner_range, segment_range);
    if comments.is_empty() {
        return false;
    }

    docs.extend(comments);
    docs.push(text(" "));
    true
}

fn matching_type_argument_close(parts: &[TypeLayoutPart], open_index: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, part) in parts.iter().enumerate().skip(open_index) {
        let TypeLayoutPart::Token(token) = part else {
            continue;
        };
        match token.kind() {
            JavaSyntaxKind::Lt => depth += 1,
            JavaSyntaxKind::Gt => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn outer_type_argument_bounds(parts: &[TypeLayoutPart]) -> Option<(usize, usize)> {
    let open_index = parts.iter().position(|part| {
        matches!(
            part,
            TypeLayoutPart::Token(token) if token.kind() == JavaSyntaxKind::Lt
        )
    })?;
    Some((open_index, matching_type_argument_close(parts, open_index)?))
}

fn format_type_argument_parts(
    parts: &[TypeLayoutPart],
    list_range: TextRange,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let arguments = split_type_argument_parts(parts)
        .into_iter()
        .map(|argument| {
            let range = type_layout_part_range(argument)
                .expect("parser-clean type argument should have a source range");
            let argument = argument.to_vec();
            Ok(java_lists::ListItem::new(range, move |context| {
                format_type_argument_layout_sequence(&argument, context)
            }))
        })
        .collect::<FormatResult<Vec<_>>>()?;
    java_lists::type_argument_list(arguments, list_range, context)
}

fn format_nested_type_argument_parts(
    parts: &[TypeLayoutPart],
    list_range: TextRange,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let arguments = split_type_argument_parts(parts)
        .into_iter()
        .map(|argument| {
            let range = type_layout_part_range(argument)
                .expect("parser-clean type argument should have a source range");
            let argument = argument.to_vec();
            Ok(java_lists::ListItem::new(range, move |context| {
                format_type_argument_layout_sequence(&argument, context)
            }))
        })
        .collect::<FormatResult<Vec<_>>>()?;
    java_lists::nested_type_argument_list(arguments, list_range, context)
}

fn format_type_clause_type_argument_parts(
    parts: &[TypeLayoutPart],
    list_range: TextRange,
    has_multiple_clause_types: bool,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let arguments = split_type_argument_parts(parts)
        .into_iter()
        .map(|argument| {
            let range = type_layout_part_range(argument)
                .expect("parser-clean type argument should have a source range");
            let argument = argument.to_vec();
            Ok(java_lists::ListItem::new(range, move |context| {
                format_type_argument_layout_sequence(&argument, context)
            }))
        })
        .collect::<FormatResult<Vec<_>>>()?;
    java_lists::type_clause_type_argument_list(
        arguments,
        list_range,
        has_multiple_clause_types,
        context,
    )
}

fn format_selector_type_argument_part_variants(
    parts: &[TypeLayoutPart],
    list_range: TextRange,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<(Doc, Doc)> {
    let arguments = split_type_argument_parts(parts)
        .into_iter()
        .map(|argument| {
            let range = type_layout_part_range(argument)
                .expect("parser-clean type argument should have a source range");
            let argument = argument.to_vec();
            Ok(java_lists::ListItem::new(range, move |context| {
                format_type_argument_layout_sequence(&argument, context)
            }))
        })
        .collect::<FormatResult<Vec<_>>>()?;
    java_lists::selector_type_argument_list_variants(arguments, list_range, context)
}

fn type_layout_part_range(parts: &[TypeLayoutPart]) -> Option<TextRange> {
    let start = parts
        .iter()
        .find_map(type_layout_part_source_range)?
        .start();
    let end = parts
        .iter()
        .rev()
        .find_map(type_layout_part_source_range)?
        .end();

    Some(TextRange::new(start, end))
}

fn type_layout_part_source_range(part: &TypeLayoutPart) -> Option<TextRange> {
    match part {
        TypeLayoutPart::Annotation(annotation) => annotation.code_text_range(),
        TypeLayoutPart::Text(_) => None,
        TypeLayoutPart::Token(token) => Some(token.token_text_range()),
    }
}

fn split_type_argument_parts(parts: &[TypeLayoutPart]) -> Vec<&[TypeLayoutPart]> {
    if parts.is_empty() {
        return Vec::new();
    }

    let mut arguments = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (index, part) in parts.iter().enumerate() {
        let TypeLayoutPart::Token(token) = part else {
            continue;
        };
        match token.kind() {
            JavaSyntaxKind::Lt => depth += 1,
            JavaSyntaxKind::Gt => depth = depth.saturating_sub(1),
            JavaSyntaxKind::Comma if depth == 0 => {
                arguments.push(trim_type_argument_part(&parts[start..index]));
                start = index + 1;
            }
            _ => {}
        }
    }
    arguments.push(trim_type_argument_part(&parts[start..]));
    arguments
}

fn trim_type_argument_part(mut parts: &[TypeLayoutPart]) -> &[TypeLayoutPart] {
    while matches!(parts.first(), Some(TypeLayoutPart::Text(value)) if value.trim().is_empty()) {
        parts = &parts[1..];
    }
    while matches!(parts.last(), Some(TypeLayoutPart::Text(value)) if value.trim().is_empty()) {
        parts = &parts[..parts.len() - 1];
    }
    parts
}

fn format_flat_type_layout_sequence(
    parts: &[TypeLayoutPart],
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let mut docs = Vec::new();
    let mut previous_was_annotation = false;
    let mut previous_was_dot = false;
    let mut previous_was_space_text = false;
    let mut previous_dot_range = None;
    for part in parts {
        match part {
            TypeLayoutPart::Text(value) => {
                if previous_was_annotation {
                    docs.push(text(" "));
                }
                previous_was_space_text = value.chars().all(char::is_whitespace);
                previous_was_annotation = false;
                previous_was_dot = false;
                previous_dot_range = None;
                docs.push(text(*value));
            }
            TypeLayoutPart::Annotation(annotation) => {
                let inserted_qualified_comments = previous_was_dot
                    && annotation.code_text_range().is_some_and(|range| {
                        append_qualified_segment_comments(
                            &mut docs,
                            previous_dot_range,
                            range,
                            context,
                        )
                    });
                if !docs.is_empty()
                    && !previous_was_dot
                    && !previous_was_space_text
                    && !inserted_qualified_comments
                {
                    docs.push(text(" "));
                }
                docs.push(format_annotation(annotation, context, "type-use")?);
                previous_was_annotation = true;
                previous_was_dot = false;
                previous_was_space_text = false;
                previous_dot_range = None;
            }
            TypeLayoutPart::Token(token) => {
                if previous_was_dot && token.kind() == JavaSyntaxKind::Identifier {
                    append_qualified_segment_comments(
                        &mut docs,
                        previous_dot_range,
                        token.token_text_range(),
                        context,
                    );
                }
                if previous_was_annotation && token_needs_space_after_annotation(token.kind()) {
                    reject_unhandled_comments_before_start(
                        context,
                        token.token_text_range(),
                        "Java formatter does not support comments between type-use annotations and types yet",
                    )?;
                    docs.push(text(" "));
                }
                previous_was_dot = token.kind() == JavaSyntaxKind::Dot;
                previous_dot_range = previous_was_dot.then_some(token.token_text_range());
                previous_was_annotation = false;
                previous_was_space_text = false;
                docs.push(format_token(token));
            }
        }
    }

    Ok(concat(docs))
}

fn format_type_argument_layout_sequence(
    parts: &[TypeLayoutPart],
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let Some(first_annotation) = wildcard_bound_annotation_index(parts) else {
        if !contains_type_argument_list(parts)
            || !context.policy().type_arguments_break_nested_generic_items()
        {
            return format_flat_type_layout_sequence(parts, context);
        }

        if type_layout_part_range(parts)
            .is_some_and(|range| context.unhandled_comment_trivia_in_range(range).is_some())
        {
            return format_type_layout_sequence(
                parts,
                TypeArgumentListSlot::NestedGeneric,
                context,
            );
        }

        let flat = format_flat_type_layout_sequence(parts, context)?;
        let broken =
            format_type_layout_sequence(parts, TypeArgumentListSlot::NestedGeneric, context)?;
        return Ok(best_fitting(flat, [broken]));
    };

    let prefix = format_flat_type_layout_sequence(
        &parts[..trim_trailing_space_text(parts, first_annotation)],
        context,
    )?;
    let (annotations, rest_start) =
        format_consecutive_type_annotations(parts, first_annotation, context)?;
    let rest = format_flat_type_layout_sequence(&parts[rest_start..], context)?;

    Ok(group(concat([
        prefix,
        indent_by(
            context.policy().continuation_indent_levels(),
            concat([line(), annotations, line(), rest]),
        ),
    ])))
}

fn contains_type_argument_list(parts: &[TypeLayoutPart]) -> bool {
    parts.iter().any(
        |part| matches!(part, TypeLayoutPart::Token(token) if token.kind() == JavaSyntaxKind::Lt),
    )
}

fn wildcard_bound_annotation_index(parts: &[TypeLayoutPart]) -> Option<usize> {
    if !matches!(
        parts.first(),
        Some(TypeLayoutPart::Token(token)) if token.kind() == JavaSyntaxKind::Question
    ) {
        return None;
    }

    let mut index = skip_space_text(parts, 1);
    if !matches!(
        parts.get(index),
        Some(TypeLayoutPart::Token(token))
            if matches!(token.kind(), JavaSyntaxKind::ExtendsKw | JavaSyntaxKind::SuperKw)
    ) {
        return None;
    }

    index = skip_space_text(parts, index + 1);
    matches!(parts.get(index), Some(TypeLayoutPart::Annotation(_))).then_some(index)
}

fn format_consecutive_type_annotations(
    parts: &[TypeLayoutPart],
    mut index: usize,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<(Doc, usize)> {
    let mut annotations = Vec::new();
    while let Some(TypeLayoutPart::Annotation(annotation)) = parts.get(index) {
        annotations.push(format_annotation(annotation, context, "type-use")?);
        index = skip_space_text(parts, index + 1);
    }

    Ok((join(line(), annotations), index))
}

fn skip_space_text(parts: &[TypeLayoutPart], mut index: usize) -> usize {
    while matches!(
        parts.get(index),
        Some(TypeLayoutPart::Text(value)) if value.chars().all(char::is_whitespace)
    ) {
        index += 1;
    }
    index
}

fn trim_trailing_space_text(parts: &[TypeLayoutPart], mut end: usize) -> usize {
    while end > 0
        && matches!(
            parts.get(end - 1),
            Some(TypeLayoutPart::Text(value)) if value.chars().all(char::is_whitespace)
        )
    {
        end -= 1;
    }
    end
}

fn token_needs_space_after_annotation(kind: JavaSyntaxKind) -> bool {
    !matches!(
        kind,
        JavaSyntaxKind::Dot
            | JavaSyntaxKind::Comma
            | JavaSyntaxKind::Gt
            | JavaSyntaxKind::LBracket
            | JavaSyntaxKind::RBracket
    )
}
