use super::{
    Doc, FormatResult, JavaFormatContext, JavaSyntaxKind, Type, TypeLayoutPart, concat,
    format_annotation, format_token, missing_layout, reject_unhandled_comments_before_start, text,
};

pub(super) fn format_type(ty: &Type, context: &mut JavaFormatContext<'_>) -> FormatResult<Doc> {
    let parts = ty.simple_layout_parts().ok_or_else(|| {
        missing_layout(
            "Java formatter does not support this type shape yet",
            ty.text_range(),
        )
    })?;

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
                docs.push(text(value));
            }
            TypeLayoutPart::Annotation(annotation) => {
                if !docs.is_empty() && !previous_was_dot {
                    docs.push(text(" "));
                }
                docs.push(format_annotation(&annotation, context, "type-use")?);
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
                docs.push(format_token(&token));
            }
        }
    }

    Ok(concat(docs))
}
