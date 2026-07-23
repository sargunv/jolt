use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{
    Annotation, EnhancedForVariable, FieldDeclaration, FormalParameter, JavaSyntaxField,
    JavaSyntaxInvariantError, JavaSyntaxToken, JavaSyntaxView, LocalVariableDeclaration,
    ParameterModifierList, ReceiverParameter, RecordComponent, ResourceVariableDeclaration,
    VariableDeclarator, VariableDeclaratorList, VariableTypeSyntax,
};

use crate::helpers::comments::{
    InlineLeadingTrivia, LeadingTrivia, TrailingTrivia, comment_forces_line,
    format_construct_leading_comments, format_token, format_token_with_comments,
    format_token_with_inline_leading_comments,
};
use crate::helpers::lists::{CommaListItem, comma_list, syntax_comma_list_items};
use crate::helpers::modifiers::{VisibleDoc, inline_modifier_prefix_from_docs};
use crate::helpers::recovery::{
    JavaFormatField, JavaFormatListPart, format_malformed, format_missing, format_optional_field,
    format_required_field, resolve_list_part_with_visibility, resolve_optional_field,
    resolve_required_field,
};
use crate::rules::annotations::{format_annotation, format_annotation_without_leading_comments};
use crate::rules::expressions::format_variable_initializer_value;
use crate::rules::modifiers::{
    TypedModifierPrefix, format_inline_typed_parameter_modifier_prefix,
    format_typed_modifier_prefix, format_typed_parameter_modifier_prefix,
};
use crate::rules::statements::format_statement_semicolon;
use crate::rules::types::{
    format_array_dimensions, format_type, format_type_without_leading_comments,
};

pub(crate) fn format_field_declaration<'source>(
    field: &FieldDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let modifiers = match resolve_optional_field(field.modifiers(), doc) {
        JavaFormatField::Present(modifiers) => format_typed_modifier_prefix(modifiers, doc),
        JavaFormatField::Malformed(recovery) => TypedModifierPrefix {
            declaration_prefix: recovery,
            type_use_prefix: Doc::nil(),
        },
    };
    let declaration_prefix = modifiers.declaration_prefix;
    let ty = format_required_field(field.r#type(), doc, |ty, doc| format_type(&ty, doc));

    if let Some(declarators) = present_required(field.declarators())
        && let Some(declarator) = single_declarator(&declarators)
    {
        let typed_prefix = doc_concat!(doc, [declaration_prefix, modifiers.type_use_prefix, ty]);
        let declaration = format_single_variable_declaration(typed_prefix, &declarator, doc);
        let semicolon = format_statement_semicolon(field.semicolon(), doc);
        return doc_concat!(doc, [declaration, semicolon]);
    }

    let declarators = format_required_field(field.declarators(), doc, |declarators, doc| {
        format_variable_declarator_list(&declarators, doc)
    });
    let space = doc.space();
    let semicolon = format_statement_semicolon(field.semicolon(), doc);

    doc_concat!(
        doc,
        [
            declaration_prefix,
            modifiers.type_use_prefix,
            ty,
            space,
            declarators,
            semicolon,
        ]
    )
}

pub(crate) fn format_local_variable_declaration<'source>(
    declaration: &LocalVariableDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let modifiers = format_required_parameter_modifiers(declaration.modifiers(), doc);
    let declaration_prefix = modifiers.declaration_prefix;
    let ty = local_variable_type(declaration, doc);

    if let Some(declarators) = present_required(declaration.declarators())
        && let Some(declarator) = single_declarator(&declarators)
    {
        let typed_prefix = doc_concat!(doc, [declaration_prefix, modifiers.type_use_prefix, ty]);
        return format_single_variable_declaration(typed_prefix, &declarator, doc);
    }

    let declarators = format_required_field(declaration.declarators(), doc, |declarators, doc| {
        format_variable_declarator_list(&declarators, doc)
    });
    let space = doc.space();

    doc_concat!(
        doc,
        [
            declaration_prefix,
            modifiers.type_use_prefix,
            ty,
            space,
            declarators,
        ]
    )
}

pub(crate) fn format_enhanced_for_variable<'source>(
    variable: &EnhancedForVariable<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let modifiers = format_required_parameter_modifiers(variable.modifiers(), doc);
    let declaration_prefix = modifiers.declaration_prefix;
    let ty = format_required_field(variable.r#type(), doc, |ty, doc| {
        format_variable_type(ty.classify(), doc)
    });
    let name = format_required_field(variable.name(), doc, |name, doc| {
        format_name_after_type_token(doc, &name)
    });
    let dimensions = format_optional_field(variable.dimensions(), doc, |dimensions, doc| {
        format_array_dimensions(&dimensions, doc)
    });
    doc_concat!(
        doc,
        [
            declaration_prefix,
            modifiers.type_use_prefix,
            ty,
            doc.space(),
            name,
            dimensions,
        ]
    )
}

pub(crate) fn format_resource_variable_declaration<'source>(
    declaration: &ResourceVariableDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let modifiers = format_required_parameter_modifiers(declaration.modifiers(), doc);
    let declaration_prefix = modifiers.declaration_prefix;
    let ty = format_required_field(declaration.r#type(), doc, |ty, doc| {
        format_variable_type(ty.classify(), doc)
    });
    let name = format_required_field(declaration.name(), doc, |name, doc| {
        format_name_after_type_token(doc, &name)
    });
    let dimensions = format_optional_field(declaration.dimensions(), doc, |dimensions, doc| {
        format_array_dimensions(&dimensions, doc)
    });
    let assign = format_required_field(declaration.assign(), doc, |assign, doc| {
        format_token_with_comments(doc, &assign)
    });
    let initializer = format_required_field(declaration.initializer(), doc, |initializer, doc| {
        format_required_field(initializer.value(), doc, |value, doc| {
            format_variable_initializer_value(value, doc)
        })
    });
    doc_concat!(
        doc,
        [
            declaration_prefix,
            modifiers.type_use_prefix,
            ty,
            doc.space(),
            name,
            dimensions,
            doc.space(),
            assign,
            doc.space(),
            initializer,
        ]
    )
}

pub(crate) fn format_formal_parameter<'source>(
    parameter: &FormalParameter<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let modifiers = match resolve_required_field(parameter.modifiers(), doc) {
        JavaFormatField::Present(modifiers) => {
            format_inline_typed_parameter_modifier_prefix(&modifiers, doc)
        }
        JavaFormatField::Malformed(recovery) => TypedModifierPrefix {
            declaration_prefix: recovery,
            type_use_prefix: Doc::nil(),
        },
    };
    let declaration_prefix = modifiers.declaration_prefix;
    let ty = format_required_field(parameter.r#type(), doc, |ty, doc| format_type(&ty, doc));
    let ty = doc_concat!(doc, [modifiers.type_use_prefix, ty]);
    let varargs_annotations = resolve_annotation_list_docs(parameter.varargs_annotations(), doc);
    let name = format_required_field(parameter.name(), doc, |name, doc| {
        format_name_after_type_token(doc, &name)
    });
    let dimensions = format_optional_field(parameter.dimensions(), doc, |dimensions, doc| {
        format_array_dimensions(&dimensions, doc)
    });
    let ellipsis = resolve_ellipsis_doc(parameter.ellipsis(), doc);
    format_named_typed_declaration(
        declaration_prefix,
        ty,
        varargs_annotations,
        name,
        dimensions,
        ellipsis,
        doc,
    )
}

pub(crate) fn format_record_component<'source>(
    component: &RecordComponent<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let modifiers = match resolve_required_field(component.modifiers(), doc) {
        JavaFormatField::Present(modifiers) => {
            format_inline_typed_parameter_modifier_prefix(&modifiers, doc)
        }
        JavaFormatField::Malformed(recovery) => TypedModifierPrefix {
            declaration_prefix: recovery,
            type_use_prefix: Doc::nil(),
        },
    };
    let declaration_prefix = modifiers.declaration_prefix;
    let ty = format_required_field(component.r#type(), doc, |ty, doc| format_type(&ty, doc));
    let ty = doc_concat!(doc, [modifiers.type_use_prefix, ty]);
    let varargs_annotations = resolve_annotation_list_docs(component.varargs_annotations(), doc);
    let name = format_required_field(component.name(), doc, |name, doc| {
        format_name_after_type_token(doc, &name)
    });
    let ellipsis = resolve_ellipsis_doc(component.ellipsis(), doc);
    format_named_typed_declaration(
        declaration_prefix,
        ty,
        varargs_annotations,
        name,
        Doc::nil(),
        ellipsis,
        doc,
    )
}

pub(crate) fn format_receiver_parameter<'source>(
    parameter: &ReceiverParameter<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let parameter_first = parameter.first_token();
    let leading_comments = format_construct_leading_comments(doc, parameter_first.as_ref());
    let modifiers = format_required_field(parameter.annotations(), doc, |annotations, doc| {
        format_annotation_parts(annotations.parts(), true, doc).map_or_else(
            Doc::nil,
            |annotations| {
                if annotations.visible {
                    doc_concat!(doc, [annotations.doc, doc.space()])
                } else {
                    annotations.doc
                }
            },
        )
    });
    let ty = format_required_field(parameter.r#type(), doc, |ty, doc| {
        if ty.first_token() == parameter_first {
            format_type_without_leading_comments(&ty, doc)
        } else {
            format_type(&ty, doc)
        }
    });
    let space = doc.space();
    let qualifier = format_optional_field(parameter.qualifier(), doc, |qualifier, doc| {
        let qualifier = format_token_with_comments(doc, &qualifier);
        let dot = format_optional_field(parameter.dot(), doc, |dot, doc| {
            format_token(doc, &dot, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
        });
        doc_concat!(doc, [qualifier, dot])
    });
    let this_token = format_required_field(parameter.this_keyword(), doc, |token, doc| {
        format_token_with_comments(doc, &token)
    });
    doc_concat!(
        doc,
        [
            leading_comments,
            modifiers,
            ty,
            space,
            qualifier,
            this_token,
        ]
    )
}

fn format_annotation_parts<'source>(
    parts: impl IntoIterator<Item = jolt_java_syntax::JavaSyntaxListPart<'source, Annotation<'source>>>,
    suppress_first_leading: bool,
    doc: &mut DocBuilder<'source>,
) -> Option<VisibleDoc<'source>> {
    let mut has_parts = false;
    let mut visible = false;
    let docs = doc.concat_list(|docs| {
        for part in parts {
            let (part, part_is_visible) =
                resolve_list_part_with_visibility(part, docs, |annotation| {
                    annotation.first_token().is_some()
                });
            if visible && part_is_visible {
                let space = docs.space();
                docs.push(space);
            }
            let first_visible = !visible && part_is_visible;
            has_parts = true;
            let part = match part {
                JavaFormatListPart::Item(annotation) => {
                    if first_visible && suppress_first_leading {
                        format_annotation_without_leading_comments(&annotation, docs)
                    } else {
                        format_annotation(&annotation, docs)
                    }
                }
                JavaFormatListPart::Separator(separator) => {
                    format_token_with_comments(docs, &separator)
                }
                JavaFormatListPart::Malformed(recovery) => recovery,
            };
            docs.push(part);
            visible |= part_is_visible;
        }
    });

    has_parts.then_some(VisibleDoc { doc: docs, visible })
}

fn resolve_annotation_list_docs<'source>(
    field: jolt_java_syntax::JavaSyntaxField<'source, jolt_java_syntax::AnnotationList<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Option<VisibleDoc<'source>> {
    match field {
        JavaSyntaxField::Present(annotations) => {
            format_annotation_parts(annotations.parts(), false, doc)
        }
        JavaSyntaxField::Malformed(malformed) => Some(VisibleDoc {
            visible: malformed.first_token().is_some(),
            doc: format_malformed(&malformed, doc),
        }),
        JavaSyntaxField::Missing(missing) => Some(VisibleDoc {
            doc: format_missing(&missing, doc),
            visible: false,
        }),
    }
}

fn resolve_ellipsis_doc<'source>(
    field: JavaSyntaxField<'source, JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Option<VisibleDoc<'source>> {
    match field {
        JavaSyntaxField::Present(ellipsis) => Some(VisibleDoc {
            doc: format_token_with_comments(doc, &ellipsis),
            visible: true,
        }),
        JavaSyntaxField::Malformed(malformed) => Some(VisibleDoc {
            visible: malformed.first_token().is_some(),
            doc: format_malformed(&malformed, doc),
        }),
        JavaSyntaxField::Missing(_) => None,
    }
}

fn format_named_typed_declaration<'source>(
    modifiers: Doc<'source>,
    ty: Doc<'source>,
    varargs_annotations: Option<VisibleDoc<'source>>,
    name: Doc<'source>,
    dimensions: Doc<'source>,
    ellipsis: Option<VisibleDoc<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let has_varargs_annotations =
        varargs_annotations.is_some_and(|annotations| annotations.visible);
    let has_ellipsis = ellipsis.is_some_and(|ellipsis| ellipsis.visible);
    let ellipsis = if let Some(ellipsis) = ellipsis {
        if let Some(varargs_annotations) = varargs_annotations {
            let annotations = inline_modifier_prefix_from_docs(
                doc,
                Some(varargs_annotations),
                Vec::new(),
                false,
                false,
                false,
            );
            doc_concat!(doc, [annotations, ellipsis.doc])
        } else {
            ellipsis.doc
        }
    } else {
        Doc::nil()
    };
    let ellipsis_name_separator = if has_ellipsis {
        doc.space()
    } else {
        Doc::nil()
    };
    let name = doc_concat!(doc, [ellipsis, ellipsis_name_separator, name, dimensions]);
    let type_name_separator = if has_ellipsis && !has_varargs_annotations {
        Doc::nil()
    } else {
        doc.space()
    };

    doc_concat!(doc, [modifiers, ty, type_name_separator, name])
}

fn local_variable_type<'source>(
    declaration: &LocalVariableDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_required_field(declaration.r#type(), doc, |ty, doc| {
        format_variable_type(ty.classify(), doc)
    })
}

fn format_required_parameter_modifiers<'source>(
    field: JavaSyntaxField<'source, ParameterModifierList<'source>>,
    doc: &mut DocBuilder<'source>,
) -> TypedModifierPrefix<'source> {
    match resolve_required_field(field, doc) {
        JavaFormatField::Present(modifiers) => {
            format_typed_parameter_modifier_prefix(&modifiers, doc)
        }
        JavaFormatField::Malformed(recovery) => TypedModifierPrefix {
            declaration_prefix: recovery,
            type_use_prefix: Doc::nil(),
        },
    }
}

fn format_variable_type<'source>(
    ty: Result<VariableTypeSyntax<'source>, JavaSyntaxInvariantError>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match ty {
        Ok(VariableTypeSyntax::Type(ty)) => format_type(&ty, doc),
        Ok(VariableTypeSyntax::Var(token)) => format_token_with_comments(doc, &token),
        Err(error) => {
            doc.block_on_invariant(error.to_string());
            Doc::nil()
        }
    }
}

fn format_variable_declarator_list<'source>(
    declarators: &VariableDeclaratorList<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let items = variable_declarator_list_items(declarators, doc);
    let list = comma_list(doc, items);
    doc_group!(doc, list)
}

fn single_declarator<'source>(
    declarators: &VariableDeclaratorList<'source>,
) -> Option<VariableDeclarator<'source>> {
    let mut parts = declarators.parts();
    let declarator = match parts.next()? {
        jolt_java_syntax::JavaSyntaxListPart::Item(declarator) => declarator,
        jolt_java_syntax::JavaSyntaxListPart::Separator(_)
        | jolt_java_syntax::JavaSyntaxListPart::Missing(_)
        | jolt_java_syntax::JavaSyntaxListPart::Malformed(_) => return None,
    };
    parts.next().is_none().then_some(declarator)
}

fn format_single_variable_declaration<'source>(
    typed_prefix: Doc<'source>,
    declarator: &VariableDeclarator<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let name = format_variable_declarator_name_and_dimensions(declarator, doc);
    if optional_is_absent(declarator.initializer()) && optional_is_absent(declarator.assign()) {
        let space = doc.space();
        return doc_concat!(doc, [typed_prefix, space, name]);
    }

    let space = doc.space();
    let initializer = format_variable_initializer_split(declarator, doc);
    let declaration = doc_concat!(doc, [name, initializer]);
    let declaration = doc_group!(doc, declaration);
    doc_concat!(doc, [typed_prefix, space, declaration])
}

fn variable_declarator_list_items<'source, 'fmt>(
    declarators: &'fmt VariableDeclaratorList<'source>,
    doc: &'fmt mut DocBuilder<'source>,
) -> Vec<CommaListItem<'source>> {
    syntax_comma_list_items(doc, declarators.parts(), |declarator, doc| {
        format_variable_declarator(&declarator, doc)
    })
}

fn format_variable_declarator<'source>(
    declarator: &VariableDeclarator<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let name = format_variable_declarator_name_and_dimensions(declarator, doc);
    let initializer = format_variable_initializer_split(declarator, doc);
    let contents = doc_concat!(doc, [name, initializer]);
    doc_group!(doc, contents)
}

fn format_variable_declarator_name_and_dimensions<'source>(
    declarator: &VariableDeclarator<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let name = format_required_field(declarator.name(), doc, |name, doc| {
        format_name_after_type_token(doc, &name)
    });
    let dimensions = format_optional_field(declarator.dimensions(), doc, |dimensions, doc| {
        format_array_dimensions(&dimensions, doc)
    });
    doc_concat!(doc, [name, dimensions])
}

fn format_name_after_type_token<'source>(
    doc: &mut DocBuilder<'source>,
    name: &jolt_java_syntax::JavaSyntaxToken<'source>,
) -> Doc<'source> {
    format_token_with_inline_leading_comments(
        doc,
        name,
        InlineLeadingTrivia::BeforeToken,
        TrailingTrivia::Preserve,
    )
}

fn format_variable_initializer_split<'source>(
    declarator: &VariableDeclarator<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let (operator, operator_forces_line) = match resolve_optional_field(declarator.assign(), doc) {
        JavaFormatField::Present(Some(operator)) => {
            let forces_line = operator
                .trailing_comments()
                .any(|comment| comment_forces_line(&comment));
            (format_token_with_comments(doc, &operator), forces_line)
        }
        JavaFormatField::Present(None) => (Doc::nil(), false),
        JavaFormatField::Malformed(recovery) => (recovery, false),
    };
    let (value, value_has_leading_comments, has_value) =
        match resolve_optional_field(declarator.initializer(), doc) {
            JavaFormatField::Present(Some(initializer)) => {
                match resolve_required_field(initializer.value(), doc) {
                    JavaFormatField::Present(value) => {
                        let has_comments = value
                            .first_token()
                            .is_some_and(|token| !token.leading_comments().is_empty());
                        (
                            format_variable_initializer_value(value, doc),
                            has_comments,
                            true,
                        )
                    }
                    JavaFormatField::Malformed(recovery) => (recovery, false, true),
                }
            }
            JavaFormatField::Present(None) => (Doc::nil(), false, false),
            JavaFormatField::Malformed(recovery) => (recovery, false, true),
        };

    if !has_value {
        if optional_is_absent(declarator.assign()) {
            return Doc::nil();
        }
        let space = doc.space();
        return doc_concat!(doc, [space, operator]);
    }

    let space = doc.space();
    let separator = if operator_forces_line {
        Doc::nil()
    } else if value_has_leading_comments {
        doc.hard_line()
    } else {
        doc.line()
    };
    let value = doc_concat!(doc, [separator, value]);
    let value = doc_indent!(doc, value);
    doc_concat!(doc, [space, operator, value])
}

fn present_required<T>(field: jolt_java_syntax::JavaSyntaxField<'_, T>) -> Option<T> {
    match field {
        jolt_java_syntax::JavaSyntaxField::Present(value) => Some(value),
        jolt_java_syntax::JavaSyntaxField::Missing(_)
        | jolt_java_syntax::JavaSyntaxField::Malformed(_) => None,
    }
}

#[allow(clippy::needless_pass_by_value)]
fn optional_is_absent<T>(field: jolt_java_syntax::JavaSyntaxField<'_, T>) -> bool {
    matches!(field, jolt_java_syntax::JavaSyntaxField::Missing(_))
}
