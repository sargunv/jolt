use super::{
    Annotation, AnnotationArgumentList, AnnotationArrayInitializer, AnnotationElementListItem,
    AnnotationElementValue, AnnotationElementValuePair, Doc, FormatResult, JavaFormatContext,
    JavaSyntaxToken, ModifierList, concat, format_expression, format_name, format_token, hard_line,
    join, reject_unhandled_comments_before_start, text,
};
use crate::comments::{
    reject_unhandled_comments_in_range, take_adjacent_leading_javadoc_comment_docs_in_range,
    take_inline_leading_block_comment_docs_in_range,
};
use crate::helpers::{annotations as java_annotations, lists as java_lists};
use jolt_diagnostics::TextRange;

pub(super) fn format_modifier_list(
    modifiers: Option<ModifierList>,
    _declaration_kind: &str,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<ModifierDocs> {
    let Some(modifiers) = modifiers else {
        return Ok(ModifierDocs::default());
    };

    let annotations = format_annotation_list(modifiers.annotations(), context, "declaration")?;
    let tokens = modifiers.tokens().collect::<Vec<_>>();
    let keyword_tokens = modifiers.modifier_tokens().collect::<Vec<_>>();
    let (leading_comments, inline_leading_comments) =
        if let Some(first_modifier) = keyword_tokens.first() {
            let owner_range = TextRange::new(
                modifiers.text_range().start(),
                first_modifier.token_text_range().start(),
            );
            (
                take_adjacent_leading_javadoc_comment_docs_in_range(
                    context,
                    owner_range,
                    first_modifier.token_text_range(),
                ),
                take_inline_leading_block_comment_docs_in_range(
                    context,
                    owner_range,
                    first_modifier.token_text_range(),
                ),
            )
        } else {
            (Vec::new(), Vec::new())
        };
    if tokens.len() != keyword_tokens.len() {
        return Ok(ModifierDocs {
            leading_comments,
            annotations,
            inline_leading_comments,
            modifier_tokens: tokens,
        });
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
        leading_comments,
        annotations,
        inline_leading_comments,
        modifier_tokens: keyword_tokens,
    })
}

#[derive(Default)]
pub(super) struct ModifierDocs {
    pub(super) leading_comments: Vec<Doc>,
    pub(super) annotations: Vec<Doc>,
    pub(super) inline_leading_comments: Vec<Doc>,
    pub(super) modifier_tokens: Vec<JavaSyntaxToken>,
}

impl ModifierDocs {
    pub(super) fn has_annotations(&self) -> bool {
        !self.annotations.is_empty()
    }

    pub(super) fn modifier_docs(&self) -> Vec<Doc> {
        let mut docs = Vec::new();
        let mut index = 0;
        while let Some(token) = self.modifier_tokens.get(index) {
            if token.text() == "non"
                && self
                    .modifier_tokens
                    .get(index + 1)
                    .is_some_and(|token| token.text() == "-")
                && self
                    .modifier_tokens
                    .get(index + 2)
                    .is_some_and(|token| token.text() == "sealed")
            {
                docs.push(concat([text("non"), text("-"), text("sealed")]));
                index += 3;
            } else {
                docs.push(format_token(token));
                index += 1;
            }
        }
        if !self.inline_leading_comments.is_empty()
            && let Some(first) = docs.first_mut()
        {
            *first = concat([
                join(text(" "), self.inline_leading_comments.clone()),
                text(" "),
                first.clone(),
            ]);
        }

        docs
    }

    pub(super) fn with_annotations(self, declaration: Doc) -> Doc {
        let doc = with_vertical_annotations(self.annotations, declaration);
        if self.leading_comments.is_empty() {
            doc
        } else {
            concat([join(hard_line(), self.leading_comments), hard_line(), doc])
        }
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
        .unwrap_or_else(|| annotation.text_range());
    reject_unhandled_comments_before_start(context, code_range, messages.between)?;

    let name = annotation
        .name()
        .expect("parser-clean annotation should have a name");
    let Some(arguments) = annotation.arguments() else {
        reject_unhandled_comments_in_range(context, code_range, messages.inside)?;
        return Ok(concat([text("@"), format_name(&name)]));
    };

    let doc = concat([
        text("@"),
        format_name(&name),
        format_annotation_argument_list(&arguments, context)?,
    ]);
    reject_unhandled_comments_in_range(context, code_range, messages.inside)?;
    Ok(doc)
}

struct AnnotationMessages {
    between: &'static str,
    inside: &'static str,
}

fn annotation_messages(annotation_kind: &'static str) -> AnnotationMessages {
    match annotation_kind {
        "type-use" => AnnotationMessages {
            between: "Java formatter does not support comments between type-use annotations yet",
            inside: "Java formatter does not support comments inside type-use annotations yet",
        },
        "declaration" => AnnotationMessages {
            between: "Java formatter does not support comments between declaration annotations yet",
            inside: "Java formatter does not support comments inside declaration annotations yet",
        },
        _ => unreachable!("unknown annotation kind"),
    }
}

pub(super) fn format_annotation_argument_list(
    arguments: &AnnotationArgumentList,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let list_range = arguments.text_range();
    let Some(elements) = arguments.elements() else {
        return Ok(java_annotations::argument_list(
            std::iter::empty(),
            context.policy(),
        ));
    };

    if elements.has_pair_list_layout_shape() {
        let pairs = elements
            .pairs()
            .map(|pair| {
                let range = pair
                    .code_text_range()
                    .expect("parser-clean annotation pair should have a source range");
                let pair = pair.clone();
                Ok(java_lists::ListItem::new(range, move |context| {
                    format_annotation_element_value_pair(&pair, context)
                        .map(java_annotations::AnnotationPair::into_doc)
                }))
            })
            .collect::<FormatResult<Vec<_>>>()?;
        return java_lists::formal_parameter_list(pairs, list_range, None, context);
    }

    if elements.has_value_list_layout_shape() {
        let values = elements.values().collect::<Vec<_>>();
        if values.len() != 1 {
            let values = values
                .into_iter()
                .map(|value| {
                    let range = value
                        .code_text_range()
                        .expect("parser-clean annotation value should have a source range");
                    Ok(java_lists::ListItem::new(range, move |context| {
                        format_annotation_element_value(&value, context)
                            .map(java_annotations::AnnotationValue::into_doc)
                    }))
                })
                .collect::<FormatResult<Vec<_>>>()?;
            return java_lists::argument_list(values, list_range, false, context);
        }
        return Ok(java_annotations::single_argument(
            format_annotation_element_value(&values[0], context)?,
            context.policy(),
        ));
    }

    Ok(java_annotations::mixed_argument_list(
        elements
            .items()
            .map(|item| match item {
                AnnotationElementListItem::Value(value) => {
                    format_annotation_element_value(&value, context)
                        .map(java_annotations::AnnotationValue::into_doc)
                }
                AnnotationElementListItem::Pair(pair) => {
                    format_annotation_element_value_pair(&pair, context)
                        .map(java_annotations::AnnotationPair::into_doc)
                }
            })
            .collect::<FormatResult<Vec<_>>>()?,
    ))
}

pub(super) fn format_annotation_element_value_pair(
    pair: &AnnotationElementValuePair,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<java_annotations::AnnotationPair> {
    let name = pair
        .name()
        .expect("parser-clean annotation element value pair should have a name");
    let value = pair
        .value()
        .expect("parser-clean annotation element value pair should have a value");

    Ok(java_annotations::element_value_pair(
        format_token(&name),
        format_annotation_element_value(&value, context)?,
    ))
}

pub(super) fn format_annotation_element_value(
    value: &AnnotationElementValue,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<java_annotations::AnnotationValue> {
    if let Some(expression) = value.expression() {
        return format_expression(&expression, context)
            .map(java_annotations::AnnotationValue::expression);
    }
    if let Some(annotation) = value.annotation() {
        return format_annotation(&annotation, context, "declaration")
            .map(java_annotations::AnnotationValue::annotation);
    }
    if let Some(initializer) = value.array_initializer() {
        return format_annotation_array_initializer(&initializer, context)
            .map(java_annotations::AnnotationValue::array);
    }

    unreachable!("parser-clean annotation element value should have a formatted child")
}

fn format_annotation_array_initializer(
    initializer: &AnnotationArrayInitializer,
    context: &mut JavaFormatContext<'_>,
) -> FormatResult<Doc> {
    let list_range = initializer
        .code_text_range()
        .expect("parser-clean annotation array initializer should have a source range");
    let values = initializer
        .values()
        .map(|value| {
            let range = value
                .code_text_range()
                .expect("parser-clean annotation array value should have a source range");
            Ok(java_lists::ListItem::new(range, move |context| {
                format_annotation_element_value(&value, context)
                    .map(java_annotations::AnnotationValue::into_doc)
            }))
        })
        .collect::<FormatResult<Vec<_>>>()?;

    java_lists::braced_comma_list(values, list_range, false, false, context)
}
