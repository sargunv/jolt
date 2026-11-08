//! Type declarations, declaration headers and their clauses.

use jolt_fmt::{Doc, cat, indent};
use jolt_java_syntax::{
    AnnotationInterfaceDeclaration, ClassDeclaration, EnumDeclaration, ExtendsClause,
    ImplementsClause, InterfaceDeclaration, JavaSyntaxKind as K, PermitsClause, RecordDeclaration,
    ThrowsClause,
};
use jolt_syntax::{AstNode, ListElement, SyntaxElement, SyntaxNode, SyntaxToken};

use crate::JavaFormatter;
use crate::helpers::lists::{ListLayout, TrailingComma, delimited, joined};

pub(crate) fn class_declaration<'a>(f: &mut JavaFormatter<'a>, node: ClassDeclaration<'a>) -> Doc {
    let modifiers = f.fmt_opt(node.modifiers());
    let head = vec![
        f.token_opt(node.class_keyword()),
        f.space(),
        f.token_opt(node.name()),
        f.fmt_opt(node.type_parameters()),
    ];
    let clauses = [
        node.extends().map(AstNode::syntax),
        node.implements().map(AstNode::syntax),
        node.permits().map(AstNode::syntax),
    ];
    let lead = node.class_keyword().map(SyntaxElement::Token);
    let end = HeaderEnd::Clauses(clauses.into_iter().flatten().collect());
    let header = header(f, lead, head, end);
    with_body(
        f,
        modifiers,
        header,
        node.body().map(AstNode::syntax),
        node.missing_body_semicolon(),
    )
}

pub(crate) fn interface_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: InterfaceDeclaration<'a>,
) -> Doc {
    let modifiers = f.fmt_opt(node.modifiers());
    let head = vec![
        f.token_opt(node.interface_keyword()),
        f.space(),
        f.token_opt(node.name()),
        f.fmt_opt(node.type_parameters()),
    ];
    let clauses = [
        node.extends().map(AstNode::syntax),
        node.permits().map(AstNode::syntax),
    ];
    let lead = node.interface_keyword().map(SyntaxElement::Token);
    let end = HeaderEnd::Clauses(clauses.into_iter().flatten().collect());
    let header = header(f, lead, head, end);
    with_body(
        f,
        modifiers,
        header,
        node.body().map(AstNode::syntax),
        node.missing_body_semicolon(),
    )
}

/// `record Name<T>(components) implements X {`: the components are a
/// break-all list inside the header.
pub(crate) fn record_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: RecordDeclaration<'a>,
) -> Doc {
    let modifiers = f.fmt_opt(node.modifiers());
    let mut head = vec![
        f.token_opt(node.record_keyword()),
        f.space(),
        f.token_opt(node.name()),
        f.fmt_opt(node.type_parameters()),
    ];
    let implements = node.implements().map(AstNode::syntax);
    let (close, tail) = match implements {
        Some(_) => (node.close_paren(), None),
        None => (None, node.close_paren()),
    };
    head.push(delimited(
        f,
        node.syntax(),
        node.open_paren(),
        node.components()
            .into_iter()
            .flat_map(jolt_java_syntax::RecordComponentList::elements),
        close,
        ListLayout {
            padded: false,
            trailing_comma: TrailingComma::Never,
        },
    ));
    let lead = node.record_keyword().map(SyntaxElement::Token);
    let end = implements.map_or(HeaderEnd::Tail(tail), |clause| {
        HeaderEnd::Clauses(vec![clause])
    });
    let header = header(f, lead, head, end);
    with_body(
        f,
        modifiers,
        header,
        node.body().map(AstNode::syntax),
        node.missing_body_semicolon(),
    )
}

pub(crate) fn enum_declaration<'a>(f: &mut JavaFormatter<'a>, node: EnumDeclaration<'a>) -> Doc {
    let modifiers = f.fmt_opt(node.modifiers());
    let head = vec![
        f.token_opt(node.enum_keyword()),
        f.space(),
        f.token_opt(node.name()),
    ];
    let implements = node.implements().map(AstNode::syntax);
    let lead = node.enum_keyword().map(SyntaxElement::Token);
    let end = HeaderEnd::Clauses(implements.into_iter().collect());
    let header = header(f, lead, head, end);
    with_body(
        f,
        modifiers,
        header,
        node.body().map(AstNode::syntax),
        node.missing_body_semicolon(),
    )
}

/// `@interface Name {`.
pub(crate) fn annotation_interface_declaration<'a>(
    f: &mut JavaFormatter<'a>,
    node: AnnotationInterfaceDeclaration<'a>,
) -> Doc {
    let modifiers = f.fmt_opt(node.modifiers());
    let head = vec![
        f.token_opt(node.at()),
        f.token_opt(node.interface_keyword()),
        f.space(),
        f.token_opt(node.name()),
    ];
    let lead = node.at().map(SyntaxElement::Token);
    let header = header(f, lead, head, HeaderEnd::Tail(None));
    with_body(
        f,
        modifiers,
        header,
        node.body().map(AstNode::syntax),
        node.missing_body_semicolon(),
    )
}

/// `modifiers header {body}`, or the `;` that stands in for a missing body.
fn with_body<'a>(
    f: &mut JavaFormatter<'a>,
    modifiers: Doc,
    header: Doc,
    body: Option<SyntaxNode<'a>>,
    missing_body_semicolon: Option<SyntaxToken<'a>>,
) -> Doc {
    let body = match body {
        Some(body) => {
            let body = f.node(body);
            cat![f; f.space(), body]
        }
        None => Doc::NIL,
    };
    let semicolon = f.token_opt(missing_body_semicolon);
    cat![f; modifiers, header, body, semicolon]
}

/// What ends a declaration header: a closer that ends the head (the `)` of
/// the parameters or components, when no clause follows), or the clauses.
pub(super) enum HeaderEnd<'a> {
    Tail(Option<SyntaxToken<'a>>),
    Clauses(Vec<SyntaxNode<'a>>),
}

/// The header group of a declaration: `head`, then each clause, every
/// clause on its own line one indent deeper when the header does not fit.
///
/// When `lead`, the element the header's first part (the first non-NIL doc
/// of `head`) begins with, has a leading comment that breaks the line, such
/// as a comment on its own line before the header (after declaration
/// annotations), or a trailing line comment, that part prints outside the
/// group. The trailing comments of the last part
/// (`tail` or the last clause) print outside it too, so a multi-line comment
/// before a `;` (a JSNI body, `native void m() /*-{ … }-*/;`) does not break
/// the header. (Breaks inside the first part, such as a broken return type,
/// keep it in the group.)
pub(super) fn header<'a>(
    f: &mut JavaFormatter<'a>,
    lead: Option<SyntaxElement<'a>>,
    mut head: Vec<Doc>,
    end: HeaderEnd<'a>,
) -> Doc {
    let apart = lead
        .is_some_and(|lead| f.leading_comments_break(lead) || f.has_trailing_line_comment(lead));
    let first = head
        .iter()
        .position(|doc| !doc.is_nil())
        .filter(|_| apart)
        .map_or(0, |i| i + 1);
    let mut rest = head.split_off(first);
    let first = f.concat(head);
    let (tail, clauses) = match end {
        HeaderEnd::Tail(tail) => (tail, Vec::new()),
        HeaderEnd::Clauses(clauses) => (None, clauses),
    };
    let mut after = f.token_opt(tail);
    for (index, &clause) in clauses.iter().enumerate() {
        let clause = if index + 1 < clauses.len() || clause.is_malformed() {
            f.node(clause)
        } else {
            let leading = f.leading_comments(clause);
            let body = clause_body(f, clause);
            after = f.trailing_comments(clause);
            cat![f; leading, body]
        };
        rest.push(indent![f; f.line(), clause]);
    }
    let rest = f.concat(rest);
    let rest = f.group(rest);
    cat![f; first, rest, after]
}

/// The rule of a header clause, without the clause's own comments.
fn clause_body<'a>(f: &mut JavaFormatter<'a>, clause: SyntaxNode<'a>) -> Doc {
    match K::from_raw(clause.kind()) {
        K::ExtendsClause => ExtendsClause::cast(clause).map(|node| extends_clause(f, node)),
        K::ImplementsClause => {
            ImplementsClause::cast(clause).map(|node| implements_clause(f, node))
        }
        K::PermitsClause => PermitsClause::cast(clause).map(|node| permits_clause(f, node)),
        K::ThrowsClause => ThrowsClause::cast(clause).map(|node| throws_clause(f, node)),
        _ => None,
    }
    .expect("a header clause")
}

pub(crate) fn extends_clause<'a>(f: &mut JavaFormatter<'a>, node: ExtendsClause<'a>) -> Doc {
    let elements = node
        .types()
        .into_iter()
        .flat_map(jolt_java_syntax::TypeList::elements);
    header_clause(f, node.extends_keyword(), elements)
}

pub(crate) fn implements_clause<'a>(f: &mut JavaFormatter<'a>, node: ImplementsClause<'a>) -> Doc {
    let elements = node
        .types()
        .into_iter()
        .flat_map(jolt_java_syntax::TypeList::elements);
    header_clause(f, node.implements_keyword(), elements)
}

pub(crate) fn permits_clause<'a>(f: &mut JavaFormatter<'a>, node: PermitsClause<'a>) -> Doc {
    let elements = node
        .names()
        .into_iter()
        .flat_map(jolt_java_syntax::NameList::elements);
    header_clause(f, node.permits_keyword(), elements)
}

/// `throws A, B`. Unlike the type clauses, the types break with the
/// enclosing header, one indent further.
pub(crate) fn throws_clause<'a>(f: &mut JavaFormatter<'a>, node: ThrowsClause<'a>) -> Doc {
    let keyword = f.token_opt(node.throws_keyword());
    let types = comma_items(
        f,
        node.exceptions()
            .into_iter()
            .flat_map(jolt_java_syntax::TypeList::elements),
    );
    let types = f.indent(types);
    cat![f; keyword, f.space(), types]
}

/// `keyword A, B`: the names stay on the keyword's line if they fit,
/// otherwise each goes on its own line one indent deeper.
pub(super) fn header_clause<'a, T: AstNode<'a>>(
    f: &mut JavaFormatter<'a>,
    keyword: Option<SyntaxToken<'a>>,
    elements: impl Iterator<Item = ListElement<'a, T>>,
) -> Doc {
    let keyword = f.token_opt(keyword);
    let items = comma_items(f, elements);
    let items = cat![f; f.line(), items];
    let items = f.group(items);
    let items = f.indent(items);
    cat![f; keyword, items]
}

/// The elements of a comma list with a `line` after each comma.
fn comma_items<'a, T: AstNode<'a>>(
    f: &mut JavaFormatter<'a>,
    elements: impl Iterator<Item = ListElement<'a, T>>,
) -> Doc {
    joined(f, elements, |f| f.line())
}
