use jolt_diagnostics::TextRange;

use super::{
    Doc, FormatResult, JavaFormatContext, JavaSyntaxKind, Type, TypeArgumentList, TypeLayoutPart,
    concat, format_annotation, format_token, java_lists, reject_unhandled_comments_before_start,
    text,
};

pub(super) fn format_type(ty: &Type, context: &mut JavaFormatContext<'_>) -> FormatResult<Doc> {
    let parts = ty.layout_parts();
    format_type_layout_parts(&parts, context)
}

pub(super) fn format_type_argument_list(
    arguments: &TypeArgumentList,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let parts = arguments.layout_parts();
    format_type_layout_parts(&parts, context)
}

pub(super) fn format_type_layout_parts(
    parts: &[TypeLayoutPart],
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    format_type_layout_sequence(parts, context)
}

fn format_type_layout_sequence(
    parts: &[TypeLayoutPart],
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let mut docs = Vec::new();
    let mut previous_was_annotation = false;
    let mut previous_was_dot = false;
    let mut index = 0;
    while index < parts.len() {
        let part = &parts[index];
        match part {
            TypeLayoutPart::Text(value) => {
                if previous_was_annotation {
                    docs.push(text(" "));
                }
                previous_was_annotation = false;
                previous_was_dot = false;
                docs.push(text(*value));
            }
            TypeLayoutPart::Annotation(annotation) => {
                if !docs.is_empty() && !previous_was_dot {
                    docs.push(text(" "));
                }
                docs.push(format_annotation(annotation, context, "type-use")?);
                previous_was_annotation = true;
                previous_was_dot = false;
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
                    docs.push(format_type_argument_parts(
                        &parts[index + 1..close_index],
                        list_range,
                        context,
                    )?);
                    previous_was_annotation = false;
                    previous_was_dot = false;
                    index = close_index + 1;
                    continue;
                }
                if previous_was_annotation && token.kind() == JavaSyntaxKind::Identifier {
                    reject_unhandled_comments_before_start(
                        context,
                        token.token_text_range(),
                        "Java formatter does not support comments between type-use annotations and types yet",
                    )?;
                    docs.push(text(" "));
                }
                previous_was_dot = token.kind() == JavaSyntaxKind::Dot;
                previous_was_annotation = false;
                docs.push(format_token(token));
            }
        }
        index += 1;
    }

    Ok(concat(docs))
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
                format_flat_type_layout_sequence(&argument, context)
            }))
        })
        .collect::<FormatResult<Vec<_>>>()?;
    java_lists::type_argument_list(arguments, list_range, context)
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
    for part in parts {
        match part {
            TypeLayoutPart::Text(value) => {
                if previous_was_annotation {
                    docs.push(text(" "));
                }
                previous_was_annotation = false;
                previous_was_dot = false;
                docs.push(text(*value));
            }
            TypeLayoutPart::Annotation(annotation) => {
                if !docs.is_empty() && !previous_was_dot {
                    docs.push(text(" "));
                }
                docs.push(format_annotation(annotation, context, "type-use")?);
                previous_was_annotation = true;
                previous_was_dot = false;
            }
            TypeLayoutPart::Token(token) => {
                if previous_was_annotation && token.kind() == JavaSyntaxKind::Identifier {
                    reject_unhandled_comments_before_start(
                        context,
                        token.token_text_range(),
                        "Java formatter does not support comments between type-use annotations and types yet",
                    )?;
                    docs.push(text(" "));
                }
                previous_was_dot = token.kind() == JavaSyntaxKind::Dot;
                previous_was_annotation = false;
                docs.push(format_token(token));
            }
        }
    }

    Ok(concat(docs))
}
