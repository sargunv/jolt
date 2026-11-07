//! Fields, methods, constructors, initializers, annotation elements, and
//! their parts.

use jolt_fmt::{Doc, ListItem, cat};
use jolt_java_syntax::{
    AnnotationElementDeclaration, BlockItem, CompactConstructorDeclaration, ConstructorBody,
    ConstructorBodyEntry, ConstructorDeclaration, ConstructorInvocation, DefaultValue,
    FieldDeclaration, FormalParameterList, InstanceInitializer, MethodBody, MethodDeclaration,
    RecordComponent, StaticInitializer,
};
use jolt_syntax::{AstNode, SyntaxElement, SyntaxNode, SyntaxToken};

use super::types::header;
use crate::JavaFormatter;
use crate::helpers::bodies::{braced_body, statement_separator};
use crate::helpers::lists::{ListLayout, TrailingComma, delimited};
use crate::rules::types::inline_modifiers;

const PARAMETERS: ListLayout = ListLayout {
    padded: false,
    trailing_comma: TrailingComma::Never,
};

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
    let lead = node
        .type_parameters()
        .map(AstNode::syntax)
        .or_else(|| {
            node.return_annotations()
                .map(AstNode::syntax)
                .filter(|annotations| annotations.first_token().is_some())
        })
        .or_else(|| node.return_type().map(AstNode::syntax))
        .map(SyntaxElement::Node);
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
    let header = header(f, lead, head, tail, throws.map(AstNode::syntax));
    let body = match node.body() {
        Some(MethodBody::Block(block)) => body_after_space(f, Some(block.syntax())),
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
    let lead = node
        .type_parameters()
        .map(|parameters| SyntaxElement::Node(parameters.syntax()))
        .or_else(|| node.name().map(SyntaxElement::Token));
    let throws = node.throws();
    let (close, tail) = split_close(node.close_paren(), throws.is_none());
    head.push(parameters(
        f,
        node.syntax(),
        node.open_paren(),
        node.parameters(),
        close,
    ));
    let header = header(f, lead, head, tail, throws.map(AstNode::syntax));
    let body = body_after_space(f, node.body().map(AstNode::syntax));
    cat![f; modifiers, header, body]
}

/// The `)` of a header: inside the parameter list, or, when it ends the
/// header, the header's tail (see `header`).
fn split_close(
    close: Option<SyntaxToken<'_>>,
    ends_head: bool,
) -> (Option<SyntaxToken<'_>>, Option<SyntaxToken<'_>>) {
    if ends_head {
        (None, close)
    } else {
        (close, None)
    }
}

/// `modifiers Name {`.
pub(crate) fn compact_constructor_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: CompactConstructorDeclaration<'a>,
) -> Doc {
    let modifiers = f.fmt_opt(node.modifiers());
    let name = f.token_opt(node.name());
    let body = body_after_space(f, node.body().map(AstNode::syntax));
    cat![f; modifiers, name, body]
}

/// A block whose first statement may be an explicit constructor invocation.
pub(crate) fn constructor_body<'a>(f: &mut JavaFormatter<'a>, node: ConstructorBody<'a>) -> Doc {
    let entries: Vec<_> = node
        .entries()
        .into_iter()
        .flat_map(jolt_java_syntax::ConstructorBodyEntryList::items)
        .map(|entry| match entry {
            ConstructorBodyEntry::BlockStatement(statement)
                if matches!(statement.item(), Some(BlockItem::EmptyStatement(_))) =>
            {
                ListItem::Elide(statement.syntax())
            }
            _ => ListItem::Format(entry.syntax()),
        })
        .collect();
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
    let body = body_after_space(f, node.body().map(AstNode::syntax));
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
    let default = match node.default() {
        Some(default) => {
            let default = f.fmt(default);
            cat![f; f.space(), default]
        }
        None => Doc::NIL,
    };
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

/// One component, laid out like a formal parameter: modifiers inline.
pub(crate) fn record_component<'a>(f: &mut JavaFormatter<'a>, node: RecordComponent<'a>) -> Doc {
    let modifiers = match node.modifiers() {
        Some(modifiers) => inline_modifiers(f, modifiers),
        None => Doc::NIL,
    };
    let ty = f.fmt_opt(node.r#type());
    // `String @A ... names`: the annotations print with a space after them.
    let varargs_annotations = f.fmt_opt(node.varargs_annotations());
    let varargs_annotations = if varargs_annotations.is_nil() {
        Doc::NIL
    } else {
        cat![f; f.space(), varargs_annotations]
    };
    let ellipsis = f.token_opt(node.ellipsis());
    let name = f.token_opt(node.name());
    cat![f; modifiers, ty, varargs_annotations, ellipsis, f.space(), name]
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
    delimited(f, owner, open, elements, close, PARAMETERS)
}

/// ` {body}`, or nothing when the body is missing.
fn body_after_space<'a>(f: &mut JavaFormatter<'a>, body: Option<SyntaxNode<'a>>) -> Doc {
    match body {
        Some(body) => {
            let body = f.node(body);
            cat![f; f.space(), body]
        }
        None => Doc::NIL,
    }
}
