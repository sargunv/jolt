use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{
    Annotation, FieldDeclaration, FormalParameter, LocalVariableDeclaration, ReceiverParameter,
    RecordComponent, VariableDeclarator, VariableDeclaratorList, VariableInitializer,
};

use crate::helpers::comments::{
    InlineLeadingTrivia, LeadingTrivia, TrailingTrivia, comment_forces_line,
    format_construct_leading_comments, format_token, format_token_with_comments,
    format_token_with_inline_leading_comments,
};
use crate::helpers::lists::{CommaListItem, comma_list, recovered_comma_list_items};
use crate::helpers::modifiers::inline_modifier_prefix_from_docs;
use crate::rules::annotations::{format_annotation, format_annotation_without_leading_comments};
use crate::rules::expressions::format_variable_initializer_value;
use crate::rules::modifiers::{
    format_typed_modifier_prefix, format_typed_modifier_prefix_from_split_entries,
};
use crate::rules::statements::format_statement_semicolon;
use crate::rules::types::{
    format_array_dimensions, format_type, format_type_without_leading_comments,
};

pub(crate) fn format_field_declaration<'source>(
    field: &FieldDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let modifiers = format_typed_modifier_prefix(field.modifiers(), doc);
    let declaration_prefix = doc_concat!(
        doc,
        [
            format_construct_leading_comments(doc, field.first_token().as_ref()),
            modifiers.declaration_prefix,
        ]
    );
    let ty = field.ty().map_or_else(Doc::nil, |ty| {
        format_type_without_leading_comments(&ty, doc)
    });

    if let Some(declarators) = field.declarators()
        && let Some(declarator) = single_declarator(&declarators)
    {
        let typed_prefix = doc_concat!(doc, [declaration_prefix, modifiers.type_use_prefix, ty]);
        let declaration = format_single_variable_declaration(typed_prefix, &declarator, doc);
        let semicolon = format_statement_semicolon(field.semicolon(), doc);
        return doc_concat!(doc, [declaration, semicolon]);
    }

    let declarators = match field.declarators() {
        Some(declarators) => format_variable_declarator_list(&declarators, doc),
        None => Doc::nil(),
    };
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
    let modifiers = format_typed_modifier_prefix_from_split_entries(
        declaration.declaration_annotations().collect(),
        declaration.type_use_annotations_after_modifiers().collect(),
        declaration.modifier_entries().collect(),
        doc,
    );
    let ty = local_variable_type(declaration, doc);

    if let Some(declarators) = declaration.declarators()
        && let Some(declarator) = single_declarator(&declarators)
    {
        let typed_prefix = doc_concat!(
            doc,
            [modifiers.declaration_prefix, modifiers.type_use_prefix, ty]
        );
        return format_single_variable_declaration(typed_prefix, &declarator, doc);
    }

    let declarators = match declaration.declarators() {
        Some(declarators) => format_variable_declarator_list(&declarators, doc),
        None => Doc::nil(),
    };
    let space = doc.space();

    doc_concat!(
        doc,
        [
            modifiers.declaration_prefix,
            modifiers.type_use_prefix,
            ty,
            space,
            declarators,
        ]
    )
}

pub(crate) fn format_formal_parameter<'source>(
    parameter: &FormalParameter<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let leading_comments = format_construct_leading_comments(doc, parameter.first_token().as_ref());
    let prefix_annotations =
        format_construct_prefix_annotations(parameter.prefix_annotations(), doc);
    let modifier_prefix = inline_modifier_prefix_from_docs(
        doc,
        prefix_annotations,
        parameter.modifier_entries().collect(),
    );
    let modifiers = doc_concat!(doc, [leading_comments, modifier_prefix]);
    let ty = match parameter.ty() {
        Some(ty) => format_type_without_leading_comments(&ty, doc),
        None => Doc::nil(),
    };
    let varargs_annotations = format_annotation_docs(parameter.varargs_annotations(), doc);
    let name = match parameter.name() {
        Some(name) => format_name_after_type_token(doc, &name),
        None => Doc::nil(),
    };
    let dimensions = match parameter.dimensions() {
        Some(dimensions) => format_array_dimensions(&dimensions, doc),
        None => Doc::nil(),
    };
    format_named_typed_declaration(
        modifiers,
        ty,
        varargs_annotations,
        name,
        dimensions,
        parameter.ellipsis_token().as_ref(),
        doc,
    )
}

pub(crate) fn format_record_component<'source>(
    component: &RecordComponent<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let leading_comments = format_construct_leading_comments(doc, component.first_token().as_ref());
    let prefix_annotations =
        format_construct_prefix_annotations(component.prefix_annotations(), doc);
    let modifier_prefix = inline_modifier_prefix_from_docs(
        doc,
        prefix_annotations,
        component.modifier_entries().collect(),
    );
    let modifiers = doc_concat!(doc, [leading_comments, modifier_prefix]);
    let ty = match component.ty() {
        Some(ty) => format_type_without_leading_comments(&ty, doc),
        None => Doc::nil(),
    };
    let varargs_annotations = format_annotation_docs(component.varargs_annotations(), doc);
    let name = match component.name() {
        Some(name) => format_name_after_type_token(doc, &name),
        None => Doc::nil(),
    };
    let dimensions = match component.dimensions() {
        Some(dimensions) => format_array_dimensions(&dimensions, doc),
        None => Doc::nil(),
    };
    format_named_typed_declaration(
        modifiers,
        ty,
        varargs_annotations,
        name,
        dimensions,
        component.ellipsis_token().as_ref(),
        doc,
    )
}

pub(crate) fn format_receiver_parameter<'source>(
    parameter: &ReceiverParameter<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let leading_comments = format_construct_leading_comments(doc, parameter.first_token().as_ref());
    let annotations = format_construct_prefix_annotations(parameter.annotations(), doc);
    let modifiers = inline_modifier_prefix_from_docs(doc, annotations, Vec::new());
    let ty = match parameter.ty() {
        Some(ty) => format_type_without_leading_comments(&ty, doc),
        None => Doc::nil(),
    };
    let space = doc.space();
    let qualifier = match parameter.qualifier() {
        Some(qualifier) => {
            let qualifier = format_token_with_comments(doc, &qualifier);
            let dot = match parameter.dot() {
                Some(dot) => {
                    format_token(doc, &dot, LeadingTrivia::Preserve, TrailingTrivia::Preserve)
                }
                None => Doc::nil(),
            };
            doc_concat!(doc, [qualifier, dot])
        }
        None => Doc::nil(),
    };
    let this_token = match parameter.this_token() {
        Some(this_token) => format_token_with_comments(doc, &this_token),
        None => Doc::nil(),
    };
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

fn format_construct_prefix_annotations<'source>(
    annotations: impl IntoIterator<Item = Annotation<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    let mut docs = doc.list();
    for (index, annotation) in annotations.into_iter().enumerate() {
        if index > 0 {
            let space = doc.space();
            docs.push(space, doc);
        }
        let annotation = if index == 0 {
            format_annotation_without_leading_comments(&annotation, doc)
        } else {
            format_annotation(&annotation, doc)
        };
        docs.push(annotation, doc);
    }

    (!docs.is_empty()).then(|| docs.finish(doc))
}

fn format_annotation_docs<'source>(
    annotations: impl IntoIterator<Item = Annotation<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    let mut docs = doc.list();
    for annotation in annotations {
        if !docs.is_empty() {
            let space = doc.space();
            docs.push(space, doc);
        }
        let annotation = format_annotation(&annotation, doc);
        docs.push(annotation, doc);
    }

    (!docs.is_empty()).then(|| docs.finish(doc))
}

fn format_named_typed_declaration<'source>(
    modifiers: Doc<'source>,
    ty: Doc<'source>,
    varargs_annotations: Option<Doc<'source>>,
    name: Doc<'source>,
    dimensions: Doc<'source>,
    ellipsis: Option<&jolt_java_syntax::JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let has_varargs_annotations = varargs_annotations.is_some();
    let has_ellipsis = ellipsis.is_some();
    let ellipsis = if let Some(ellipsis) = ellipsis {
        if let Some(varargs_annotations) = varargs_annotations {
            let annotations =
                inline_modifier_prefix_from_docs(doc, [varargs_annotations], Vec::new());
            let ellipsis = format_token_with_comments(doc, ellipsis);
            doc_concat!(doc, [annotations, ellipsis])
        } else {
            format_token_with_comments(doc, ellipsis)
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
    match declaration.ty() {
        Some(ty) => format_type(&ty, doc),
        None => match declaration.var_token() {
            Some(token) => format_token_with_comments(doc, &token),
            None => Doc::nil(),
        },
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
    let mut entries = declarators.entries();
    let entry = entries.next()?;
    if entries.next().is_some() || entry.comma.is_some() {
        return None;
    }

    Some(entry.declarator)
}

fn format_single_variable_declaration<'source>(
    typed_prefix: Doc<'source>,
    declarator: &VariableDeclarator<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let name = format_variable_declarator_name_and_dimensions(declarator, doc);
    let Some(initializer) = declarator.initializer() else {
        let space = doc.space();
        return doc_concat!(doc, [typed_prefix, space, name]);
    };

    let space = doc.space();
    let initializer = format_variable_initializer_split(&initializer, doc);
    let declaration = doc_concat!(doc, [name, initializer]);
    let declaration = doc_group!(doc, declaration);
    doc_concat!(doc, [typed_prefix, space, declaration])
}

fn variable_declarator_list_items<'source, 'fmt>(
    declarators: &'fmt VariableDeclaratorList<'source>,
    doc: &'fmt mut DocBuilder<'source>,
) -> Vec<CommaListItem<'source>> {
    recovered_comma_list_items(doc, declarators.entries_with_recovered(), |entry, doc| {
        CommaListItem {
            doc: format_variable_declarator(&entry.declarator, doc),
            comma: entry.comma,
        }
    })
}

fn format_variable_declarator<'source>(
    declarator: &VariableDeclarator<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let name = format_variable_declarator_name_and_dimensions(declarator, doc);
    let initializer = match declarator.initializer() {
        Some(initializer) => format_variable_initializer_split(&initializer, doc),
        None => Doc::nil(),
    };
    let contents = doc_concat!(doc, [name, initializer]);
    doc_group!(doc, contents)
}

fn format_variable_declarator_name_and_dimensions<'source>(
    declarator: &VariableDeclarator<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let name = match declarator.name() {
        Some(name) => format_name_after_type_token(doc, &name),
        None => Doc::nil(),
    };
    let dimensions = match declarator.dimensions() {
        Some(dimensions) => format_array_dimensions(&dimensions, doc),
        None => Doc::nil(),
    };
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
    initializer: &VariableInitializer<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(value) = initializer.value() else {
        let space = doc.space();
        let operator = format_variable_initializer_operator(initializer, doc);
        return doc_concat!(doc, [space, operator]);
    };

    let space = doc.space();
    let operator = format_variable_initializer_operator(initializer, doc);
    let separator = format_variable_initializer_value_separator(initializer, &value, doc);
    let value = format_variable_initializer_value(value, doc);
    let value = doc_concat!(doc, [separator, value]);
    let value = doc_indent!(doc, value);
    doc_concat!(doc, [space, operator, value])
}

fn format_variable_initializer_operator<'source>(
    initializer: &VariableInitializer<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match initializer.operator() {
        Some(operator) => format_token_with_comments(doc, &operator),
        None => Doc::nil(),
    }
}

fn format_variable_initializer_value_separator<'source>(
    initializer: &VariableInitializer<'source>,
    value: &jolt_java_syntax::VariableInitializerValue<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if initializer_operator_trailing_comments_force_line(initializer) {
        Doc::nil()
    } else if initializer_value_has_leading_comments(value) {
        doc.hard_line()
    } else {
        doc.line()
    }
}

fn initializer_operator_trailing_comments_force_line(
    initializer: &VariableInitializer<'_>,
) -> bool {
    initializer.operator().is_some_and(|operator| {
        operator
            .trailing_comments()
            .any(|comment| comment_forces_line(&comment))
    })
}

fn initializer_value_has_leading_comments(
    value: &jolt_java_syntax::VariableInitializerValue<'_>,
) -> bool {
    value
        .first_token()
        .is_some_and(|token| !token.leading_comments().is_empty())
}
