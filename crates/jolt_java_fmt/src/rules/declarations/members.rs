//! Fields, methods, constructors, initializers, annotation elements, and
//! their parts.

use jolt_fmt::{Doc, cat};
use jolt_java_syntax::{
    AnnotationElementDeclaration, CompactConstructorDeclaration, ConstructorBody,
    ConstructorBodyEntryList, ConstructorDeclaration, ConstructorInvocation, DefaultValue,
    FieldDeclaration, FormalParameterList, InstanceInitializer, MethodBody, MethodDeclaration,
    StaticInitializer,
};
use jolt_syntax::{AstNode, SyntaxNode, SyntaxToken};

use super::types::{HeaderEnd, header, split_close};
use crate::JavaFormatter;
use crate::helpers::bodies::{body_items, braced_body, statement_separator};
use crate::helpers::lists::{ListLayout, delimited};
use crate::helpers::spaced_node;

pub(crate) fn field_declaration<'a>(f: &mut JavaFormatter<'a>, node: FieldDeclaration<'a>) -> Doc {
    let modifiers = f.fmt_opt(node.modifiers());
    let ty = f.fmt_opt(node.r#type());
    let declarators = f.fmt_opt(node.declarators());
    let semicolon = f.token_opt(node.semicolon());
    cat![f; modifiers, ty, f.space(), declarators, semicolon]
}

/// `modifiers`, the header (`<T> Type name(params) throws X`, see
/// `types::header`), then ` {body}` or `;`. Parameters that do not fit break
/// as a list, and a throws clause that does not fit goes on its own line one
/// indent deeper.
pub(crate) fn method_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: MethodDeclaration<'a>,
) -> Doc {
    let modifiers = f.fmt_opt(node.modifiers());
    let mut head = Vec::new();
    if let Some(parameters) = node.type_parameters() {
        head.extend([f.fmt(parameters), f.space()]);
    }
    head.push(f.fmt_opt(node.return_annotations()));
    head.extend([
        f.fmt_opt(node.return_type()),
        f.space(),
        f.token_opt(node.name()),
    ]);
    let throws = node.throws();
    let ends_head = throws.is_none() && node.dimensions().is_none();
    let (close, tail) = split_close(node.close_paren(), ends_head);
    head.push(parameters(
        f,
        node.syntax(),
        node.open_paren(),
        node.parameters(),
        close,
    ));
    head.push(f.fmt_opt(node.dimensions()));
    let end = throws.map_or(HeaderEnd::Tail(tail), |throws| {
        HeaderEnd::Clauses(vec![throws.syntax()])
    });
    let header = header(f, head, end);
    let body = match node.body() {
        Some(MethodBody::Block(block)) => spaced_node(f, Some(block)),
        Some(MethodBody::Token(semicolon)) => f.token(semicolon),
        None => Doc::NIL,
    };
    cat![f; modifiers, header, body]
}

/// Like a method without a return type.
pub(crate) fn constructor_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: ConstructorDeclaration<'a>,
) -> Doc {
    let modifiers = f.fmt_opt(node.modifiers());
    let mut head = Vec::new();
    if let Some(parameters) = node.type_parameters() {
        head.extend([f.fmt(parameters), f.space()]);
    }
    head.push(f.token_opt(node.name()));
    let throws = node.throws();
    let (close, tail) = split_close(node.close_paren(), throws.is_none());
    head.push(parameters(
        f,
        node.syntax(),
        node.open_paren(),
        node.parameters(),
        close,
    ));
    let end = throws.map_or(HeaderEnd::Tail(tail), |throws| {
        HeaderEnd::Clauses(vec![throws.syntax()])
    });
    let header = header(f, head, end);
    let body = spaced_node(f, node.body());
    cat![f; modifiers, header, body]
}

/// `modifiers Name {`.
pub(crate) fn compact_constructor_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: CompactConstructorDeclaration<'a>,
) -> Doc {
    let modifiers = f.fmt_opt(node.modifiers());
    let name = f.token_opt(node.name());
    let body = spaced_node(f, node.body());
    cat![f; modifiers, name, body]
}

/// A block whose first statement may be an explicit constructor invocation.
pub(crate) fn constructor_body<'a>(f: &mut JavaFormatter<'a>, node: ConstructorBody<'a>) -> Doc {
    let entries = node
        .entries()
        .into_iter()
        .flat_map(ConstructorBodyEntryList::items)
        .map(AstNode::syntax);
    let entries = body_items(node.syntax(), entries);
    braced_body(
        f,
        node.syntax(),
        node.open_brace(),
        entries,
        node.close_brace(),
        statement_separator,
    )
}

/// `super(…);`, `this(…);`, `outer.<T>super(…);`, formatted like a call.
pub(crate) fn constructor_invocation<'a>(
    f: &mut JavaFormatter<'a>,
    node: ConstructorInvocation<'a>,
) -> Doc {
    let qualifier = f.fmt_opt(node.qualifier());
    let dot = f.token_opt(node.dot());
    let type_arguments = f.fmt_opt(node.type_arguments());
    let target = f.token_opt(node.target());
    let arguments = f.fmt_opt(node.arguments());
    let semicolon = f.token_opt(node.semicolon());
    cat![f; qualifier, dot, type_arguments, target, arguments, semicolon]
}

pub(crate) fn static_initializer<'a>(
    f: &mut JavaFormatter<'a>,
    node: StaticInitializer<'a>,
) -> Doc {
    let keyword = f.token_opt(node.static_keyword());
    let body = spaced_node(f, node.body());
    cat![f; keyword, body]
}

pub(crate) fn instance_initializer<'a>(
    f: &mut JavaFormatter<'a>,
    node: InstanceInitializer<'a>,
) -> Doc {
    f.fmt_opt(node.body())
}

/// `modifiers Type name()[] default value;`.
pub(crate) fn annotation_element_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: AnnotationElementDeclaration<'a>,
) -> Doc {
    let modifiers = f.fmt_opt(node.modifiers());
    let ty = f.fmt_opt(node.r#type());
    let name = f.token_opt(node.name());
    let parentheses = parameters(
        f,
        node.syntax(),
        node.open_paren(),
        None,
        node.close_paren(),
    );
    let dimensions = f.fmt_opt(node.dimensions());
    let default = spaced_node(f, node.default());
    let semicolon = f.token_opt(node.semicolon());
    let declaration = cat![f; modifiers, ty, f.space(), name, parentheses, dimensions, default];
    let declaration = f.group(declaration);
    cat![f; declaration, semicolon]
}

/// `default value`.
pub(crate) fn default_value<'a>(f: &mut JavaFormatter<'a>, node: DefaultValue<'a>) -> Doc {
    let keyword = f.token_opt(node.default_keyword());
    let value = f.fmt_opt(node.value());
    cat![f; keyword, f.space(), value]
}

/// `(params)` as a break-all list owned by `owner`, the declaration that
/// holds the parentheses.
fn parameters<'a>(
    f: &mut JavaFormatter<'a>,
    owner: SyntaxNode<'a>,
    open: Option<SyntaxToken<'a>>,
    list: Option<FormalParameterList<'a>>,
    close: Option<SyntaxToken<'a>>,
) -> Doc {
    let elements = list.into_iter().flat_map(FormalParameterList::elements);
    delimited(f, owner, open, elements, close, ListLayout::Plain)
}
