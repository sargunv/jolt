use super::{
    Annotation, AnnotationArgumentList, AnnotationElementValue, AnnotationElementValuePair, Doc,
    FormatResult, JavaFormatContext, JavaSyntaxToken, ModifierList, concat, format_expression,
    format_name, format_token, hard_line, join, missing_layout,
    reject_unhandled_comments_before_end, reject_unhandled_comments_before_start, text, wrap,
};

pub(super) fn format_modifier_list(
    modifiers: Option<ModifierList>,
    declaration_kind: &str,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<ModifierDocs> {
    let Some(modifiers) = modifiers else {
        return Ok(ModifierDocs::default());
    };

    let annotations = format_annotation_list(modifiers.annotations(), context, "declaration")?;
    let tokens = modifiers.tokens().collect::<Vec<_>>();
    let keyword_tokens = modifiers.modifier_tokens().collect::<Vec<_>>();
    if tokens.len() != keyword_tokens.len() {
        return Err(missing_layout(
            format!("Java formatter does not support contextual {declaration_kind} modifiers yet"),
            modifiers.text_range(),
        ));
    }
    if !annotations.is_empty()
        && let Some(first_modifier) = keyword_tokens.first()
    {
        reject_unhandled_comments_before_start(
            context,
            first_modifier.token_text_range(),
            "Java formatter does not support comments between declaration annotations and modifiers yet",
        )?;
    }

    Ok(ModifierDocs {
        annotations,
        modifier_tokens: keyword_tokens,
    })
}

#[derive(Default)]
pub(super) struct ModifierDocs {
    pub(super) annotations: Vec<Doc>,
    pub(super) modifier_tokens: Vec<JavaSyntaxToken>,
}

impl ModifierDocs {
    pub(super) fn has_annotations(&self) -> bool {
        !self.annotations.is_empty()
    }

    pub(super) fn with_annotations(self, declaration: Doc) -> Doc {
        with_vertical_annotations(self.annotations, declaration)
    }
}

pub(super) fn with_vertical_annotations(annotations: Vec<Doc>, declaration: Doc) -> Doc {
    if annotations.is_empty() {
        return declaration;
    }

    concat([join(hard_line(), annotations), hard_line(), declaration])
}

pub(super) fn format_annotation_list(
    annotations: impl Iterator<Item = Annotation>,
    context: &mut JavaFormatContext<'_>,
    annotation_kind: &'static str,
) -> FormatResult<Vec<Doc>> {
    annotations
        .map(|annotation| format_annotation(&annotation, context, annotation_kind))
        .collect()
}

pub(super) fn format_annotation(
    annotation: &Annotation,
    context: &mut JavaFormatContext<'_>,
    annotation_kind: &'static str,
) -> FormatResult<Doc> {
    let messages = annotation_messages(annotation_kind);
    let code_range = annotation
        .code_text_range()
        .ok_or_else(|| missing_layout(messages.empty, annotation.text_range()))?;
    reject_unhandled_comments_before_start(context, code_range, messages.between)?;
    reject_unhandled_comments_before_end(context, code_range, messages.inside)?;
    if !annotation.has_supported_layout_shape() {
        return Err(missing_layout(messages.shape, annotation.text_range()));
    }

    let name = annotation
        .name()
        .ok_or_else(|| missing_layout(messages.missing_name, annotation.text_range()))?;
    let Some(arguments) = annotation.arguments() else {
        return Ok(concat([text("@"), format_name(&name)]));
    };

    Ok(concat([
        text("@"),
        format_name(&name),
        format_annotation_argument_list(&arguments, context)?,
    ]))
}

struct AnnotationMessages {
    empty: &'static str,
    between: &'static str,
    inside: &'static str,
    shape: &'static str,
    missing_name: &'static str,
}

fn annotation_messages(annotation_kind: &'static str) -> AnnotationMessages {
    match annotation_kind {
        "type-use" => AnnotationMessages {
            empty: "Java formatter found an empty type-use annotation",
            between: "Java formatter does not support comments between type-use annotations yet",
            inside: "Java formatter does not support comments inside type-use annotations yet",
            shape: "Java formatter does not support this type-use annotation shape yet",
            missing_name: "Java formatter found a type-use annotation without a name",
        },
        "declaration" => AnnotationMessages {
            empty: "Java formatter found an empty declaration annotation",
            between: "Java formatter does not support comments between declaration annotations yet",
            inside: "Java formatter does not support comments inside declaration annotations yet",
            shape: "Java formatter does not support this declaration annotation shape yet",
            missing_name: "Java formatter found a declaration annotation without a name",
        },
        _ => unreachable!("unknown annotation kind"),
    }
}

pub(super) fn format_annotation_argument_list(
    arguments: &AnnotationArgumentList,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !arguments.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this annotation argument list shape yet",
            arguments.text_range(),
        ));
    }
    let Some(elements) = arguments.elements() else {
        return Ok(wrap::parenthesized_comma_list(std::iter::empty()));
    };

    if elements.has_pair_list_layout_shape() {
        return Ok(wrap::parenthesized_comma_list(
            elements
                .pairs()
                .map(|pair| format_annotation_element_value_pair(&pair, context))
                .collect::<FormatResult<Vec<_>>>()?,
        ));
    }

    if elements.has_value_list_layout_shape() {
        let values = elements.values().collect::<Vec<_>>();
        if values.len() != 1 {
            return Err(missing_layout(
                "Java formatter only supports single-member annotation values yet",
                elements.text_range(),
            ));
        }
        return Ok(wrap::parenthesized_comma_list([
            format_annotation_element_value(&values[0], context)?,
        ]));
    }

    Err(missing_layout(
        "Java formatter does not support mixed annotation argument lists yet",
        elements.text_range(),
    ))
}

pub(super) fn format_annotation_element_value_pair(
    pair: &AnnotationElementValuePair,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !pair.has_supported_layout_shape() {
        return Err(missing_layout(
            "Java formatter does not support this annotation element pair shape yet",
            pair.text_range(),
        ));
    }
    let name = pair.name().ok_or_else(|| {
        missing_layout(
            "Java formatter found an annotation element pair without a name",
            pair.text_range(),
        )
    })?;
    let value = pair.value().ok_or_else(|| {
        missing_layout(
            "Java formatter found an annotation element pair without a value",
            pair.text_range(),
        )
    })?;

    Ok(wrap::assignment_expression(
        format_token(&name),
        text("="),
        format_annotation_element_value(&value, context)?,
    ))
}

pub(super) fn format_annotation_element_value(
    value: &AnnotationElementValue,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    if !value.has_expression_layout_shape() {
        return Err(missing_layout(
            "Java formatter only supports expression annotation values yet",
            value.text_range(),
        ));
    }
    let expression = value.expression().ok_or_else(|| {
        missing_layout(
            "Java formatter found an annotation element value without an expression",
            value.text_range(),
        )
    })?;

    format_expression(&expression, context)
}
