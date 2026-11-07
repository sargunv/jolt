//! Fields, methods, and throws clauses.

use jolt_fmt::{Doc, cat, indent};
use jolt_java_syntax::{FieldDeclaration, MethodBody, MethodDeclaration, ThrowsClause};
use jolt_syntax::{AstNode, ListElement};

use crate::JavaFormatter;
use crate::helpers::lists::{ListLayout, TrailingComma, delimited};

pub(crate) fn field_declaration<'a>(f: &mut JavaFormatter<'a>, node: FieldDeclaration<'a>) -> Doc {
    let modifiers = f.fmt_opt(node.modifiers());
    let ty = f.fmt_opt(node.r#type());
    let declarators = f.fmt_opt(node.declarators());
    let semicolon = f.token_opt(node.semicolon());
    cat![f; modifiers, ty, f.space(), declarators, semicolon]
}

/// `modifiers` then the header group (`<T> Type name(params) throws X`),
/// then ` {` body or `;`. A header that does not fit breaks its parameters,
/// and its throws clause goes to the next line one indent deeper.
pub(crate) fn method_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: MethodDeclaration<'a>,
) -> Doc {
    let modifiers = f.fmt_opt(node.modifiers());
    let mut header = Vec::new();
    if let Some(parameters) = node.type_parameters() {
        header.extend([f.fmt(parameters), f.space()]);
    }
    if let Some(annotations) = node.return_annotations() {
        header.push(f.fmt(annotations));
    }
    header.extend([
        f.fmt_opt(node.return_type()),
        f.space(),
        f.token_opt(node.name()),
    ]);
    let parameters = delimited(
        f,
        node.syntax(),
        node.open_paren(),
        node.parameters()
            .into_iter()
            .flat_map(jolt_java_syntax::FormalParameterList::elements),
        node.close_paren(),
        ListLayout {
            padded: false,
            trailing_comma: TrailingComma::Never,
        },
    );
    header.push(parameters);
    header.push(f.fmt_opt(node.dimensions()));
    if let Some(throws) = node.throws() {
        let throws = f.fmt(throws);
        header.push(indent![f; f.line(), throws]);
    }
    let header = f.concat(header);
    let header = f.group(header);
    let body = match node.body() {
        Some(MethodBody::Block(block)) => {
            let block = f.fmt(block);
            cat![f; f.space(), block]
        }
        Some(MethodBody::Token(semicolon)) => f.token(semicolon),
        None => Doc::NIL,
    };
    cat![f; modifiers, header, body]
}

/// `throws A, B` with the types breaking one indent further.
pub(crate) fn throws_clause<'a>(f: &mut JavaFormatter<'a>, node: ThrowsClause<'a>) -> Doc {
    let keyword = f.token_opt(node.throws_keyword());
    let mut types = Vec::new();
    for element in node
        .exceptions()
        .into_iter()
        .flat_map(jolt_java_syntax::TypeList::elements)
    {
        match element {
            ListElement::Item(ty) => types.push(f.fmt(ty)),
            ListElement::Bogus(bogus) => types.push(f.node(bogus)),
            ListElement::Separator(comma) => types.extend([f.token(comma), f.line()]),
            ListElement::Missing => {}
        }
    }
    let types = f.concat(types);
    let types = f.indent(types);
    cat![f; keyword, f.space(), types]
}
