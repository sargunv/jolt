//! Method invocations, member chains, and argument lists.

use jolt_fmt::{Doc, cat};
use jolt_java_syntax::{
    ArgumentList, Expression, FieldAccessExpression, MethodInvocationExpression,
    MethodInvocationFormSyntax, QualifiedInvocationName, QualifiedMethodInvocation,
    UnqualifiedInvocationName, UnqualifiedMethodInvocation,
};
use jolt_syntax::{AstNode, SyntaxNode};

use crate::JavaFormatter;
use crate::helpers::lists::{ListLayout, TrailingComma, delimited};

/// One link of a receiver chain, from the outermost node inward.
#[derive(Clone, Copy)]
enum Link<'a> {
    Call(
        MethodInvocationExpression<'a>,
        QualifiedMethodInvocation<'a>,
    ),
    Field(FieldAccessExpression<'a>),
}

impl<'a> Link<'a> {
    fn receiver(self) -> Option<Expression<'a>> {
        match self {
            Self::Call(_, qualified) => qualified.receiver(),
            Self::Field(access) => access.receiver(),
        }
    }

    /// The spine nodes this link owns, outermost first.
    fn nodes(self) -> impl Iterator<Item = SyntaxNode<'a>> {
        let (outer, inner) = match self {
            Self::Call(call, qualified) => (call.syntax(), Some(qualified.syntax())),
            Self::Field(access) => (access.syntax(), None),
        };
        std::iter::once(outer).chain(inner)
    }
}

fn link(expression: Expression<'_>) -> Option<Link<'_>> {
    match expression {
        Expression::MethodInvocationExpression(call) if !call.syntax().is_malformed() => {
            match call.form()? {
                MethodInvocationFormSyntax::QualifiedMethodInvocation(qualified)
                    if !qualified.syntax().is_malformed() =>
                {
                    Some(Link::Call(call, qualified))
                }
                _ => None,
            }
        }
        Expression::FieldAccessExpression(access) if !access.syntax().is_malformed() => {
            Some(Link::Field(access))
        }
        _ => None,
    }
}

/// A call, or a member chain walked iteratively: the head (everything before
/// the first link) then `.name(…)` and `.field` segments. Flat if it fits;
/// otherwise every call after the head starts its own line one indent
/// deeper. After a simple head (a name, `this`, `super`, a class literal) the
/// first segment stays on the head's line; consecutive field accesses stay
/// together.
pub(crate) fn method_invocation_expression<'a>(
    f: &mut JavaFormatter<'a>,
    node: MethodInvocationExpression<'a>,
) -> Doc {
    let Some(first) = link(Expression::MethodInvocationExpression(node)) else {
        return f.fmt_opt(node.form());
    };
    let mut links = vec![first];
    let mut head = first.receiver();
    while let Some(next) = head.and_then(link) {
        links.push(next);
        head = next.receiver();
    }
    // Interior spine nodes print their leading comments before the head and
    // their trailing comments after the segment they end with; the
    // outermost node's comments are printed by `node()`.
    let mut docs = Vec::new();
    for (index, link) in links.iter().enumerate() {
        for (position, node) in link.nodes().enumerate() {
            if index > 0 || position > 0 {
                docs.push(f.leading_comments(node));
            }
        }
    }
    let simple_head = matches!(
        head,
        Some(
            Expression::NameExpression(_)
                | Expression::ThisExpression(_)
                | Expression::SuperExpression(_)
                | Expression::ClassLiteralExpression(_)
        )
    );
    docs.push(f.fmt_opt(head));
    let head = f.concat(docs);

    // Segments innermost first; runs of field accesses merge into one.
    let mut segments: Vec<Doc> = Vec::new();
    let mut field_run: Vec<Doc> = Vec::new();
    for (index, &link) in links.iter().enumerate().rev() {
        let segment = match link {
            Link::Call(_, qualified) => {
                let dot = f.token_opt(qualified.dot());
                let type_arguments = f.fmt_opt(qualified.type_arguments());
                let name = match qualified.name() {
                    Some(QualifiedInvocationName::Token(token)) => f.token(token),
                    Some(QualifiedInvocationName::NameExpression(name)) => f.fmt(name),
                    None => Doc::NIL,
                };
                let arguments = f.fmt_opt(qualified.arguments());
                cat![f; dot, type_arguments, name, arguments]
            }
            Link::Field(access) => {
                let dot = f.token_opt(access.dot());
                let name = f.token_opt(access.name());
                let type_arguments = f.fmt_opt(access.type_arguments());
                cat![f; dot, name, type_arguments]
            }
        };
        let mut trailing = Vec::new();
        for (position, node) in link
            .nodes()
            .enumerate()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            if index > 0 || position > 0 {
                trailing.push(f.trailing_comments(node));
            }
        }
        let trailing = f.concat(trailing);
        let segment = cat![f; segment, trailing];
        match link {
            Link::Field(_) => field_run.push(segment),
            Link::Call(..) => {
                if !field_run.is_empty() {
                    segments.push(f.concat(std::mem::take(&mut field_run)));
                }
                segments.push(segment);
            }
        }
    }
    if !field_run.is_empty() {
        segments.push(f.concat(field_run));
    }

    let mut segments = segments.into_iter();
    let (head, rest_start) = if simple_head {
        let first = segments.next().unwrap_or(Doc::NIL);
        (cat![f; head, first], Vec::new())
    } else {
        (head, Vec::new())
    };
    let mut rest = rest_start;
    for segment in segments {
        rest.push(f.soft_line());
        rest.push(segment);
    }
    if rest.is_empty() {
        return f.group(head);
    }
    let rest = f.concat(rest);
    let rest = f.indent(rest);
    let chain = cat![f; head, rest];
    f.group(chain)
}

pub(crate) fn unqualified_method_invocation<'a>(
    f: &mut JavaFormatter<'a>,
    node: UnqualifiedMethodInvocation<'a>,
) -> Doc {
    let type_arguments = f.fmt_opt(node.type_arguments());
    let name = match node.name() {
        Some(UnqualifiedInvocationName::Token(token)) => f.token(token),
        Some(UnqualifiedInvocationName::NameExpression(name)) => f.fmt(name),
        None => Doc::NIL,
    };
    let arguments = f.fmt_opt(node.arguments());
    cat![f; type_arguments, name, arguments]
}

/// Reached only for a qualified invocation formatted on its own (the member
/// chain rule formats the ones under a call).
pub(crate) fn qualified_method_invocation<'a>(
    f: &mut JavaFormatter<'a>,
    node: QualifiedMethodInvocation<'a>,
) -> Doc {
    let receiver = f.fmt_opt(node.receiver());
    let dot = f.token_opt(node.dot());
    let type_arguments = f.fmt_opt(node.type_arguments());
    let name = match node.name() {
        Some(QualifiedInvocationName::Token(token)) => f.token(token),
        Some(QualifiedInvocationName::NameExpression(name)) => f.fmt(name),
        None => Doc::NIL,
    };
    let arguments = f.fmt_opt(node.arguments());
    cat![f; receiver, dot, type_arguments, name, arguments]
}

pub(crate) fn argument_list<'a>(f: &mut JavaFormatter<'a>, node: ArgumentList<'a>) -> Doc {
    delimited(
        f,
        node.syntax(),
        node.open_paren(),
        node.arguments()
            .into_iter()
            .flat_map(jolt_java_syntax::ExpressionList::elements),
        node.close_paren(),
        ListLayout {
            padded: false,
            trailing_comma: TrailingComma::Never,
        },
    )
}
