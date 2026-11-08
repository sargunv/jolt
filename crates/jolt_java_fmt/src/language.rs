//! The Java `FormatLanguage`: separator and delimiter tables, the
//! formatter-ignore scopes, and comment hooks H1–H5.

use jolt_fmt::{
    CommentContext, CommentKind, CommentLayout, Doc, FormatLanguage, Formatter, Placement,
    leading_target,
};
use jolt_java_syntax::{JavaLanguage, JavaSyntaxKind as K, is_assignment_operator};
use jolt_syntax::{RawSyntaxKind, SyntaxElement, SyntaxNode, SyntaxToken};

/// The Java formatter language.
pub struct JavaFormat;

fn kind(token: SyntaxToken<'_>) -> K {
    K::from_raw(token.kind())
}

fn node_kind(node: SyntaxNode<'_>) -> K {
    K::from_raw(node.kind())
}

/// `<`/`>` delimit only type argument and type parameter lists (H2).
fn is_angle_list(token: SyntaxToken<'_>) -> bool {
    matches!(
        node_kind(token.parent()),
        K::TypeArgumentList | K::TypeParameterList
    )
}

impl FormatLanguage for JavaFormat {
    type Syntax = JavaLanguage;

    fn format_node<'a>(f: &mut Formatter<'a, Self>, node: SyntaxNode<'a>) -> Doc {
        crate::dispatch::format_node(f, node)
    }

    fn is_separator(token: SyntaxToken<'_>) -> bool {
        matches!(kind(token), K::Comma | K::Semicolon)
    }

    fn closer_for(token: SyntaxToken<'_>) -> Option<RawSyntaxKind> {
        let closer = match kind(token) {
            K::LParen => K::RParen,
            K::LBracket => K::RBracket,
            K::LBrace => K::RBrace,
            K::Lt if is_angle_list(token) => K::Gt,
            _ => return None,
        };
        Some(closer.to_raw())
    }

    fn is_closer(token: SyntaxToken<'_>) -> bool {
        match kind(token) {
            K::RParen | K::RBracket | K::RBrace => true,
            K::Gt => is_angle_list(token),
            _ => false,
        }
    }

    /// Identifier characters, the doubled operators (`++`, `--`, `&&`, `||`,
    /// `<<`, `::`), `->`, comment openers, and an operator before `=`. Never
    /// `>` `>`, which closes nested type arguments.
    fn joins(left: char, right: char) -> bool {
        let word = |ch: char| ch.is_alphanumeric() || ch == '_' || ch == '$' || ch == '\\';
        (word(left) && word(right))
            || matches!(
                (left, right),
                ('+', '+')
                    | ('-', '-' | '>')
                    | ('&', '&')
                    | ('|', '|')
                    | ('<', '<')
                    | (':', ':')
                    | ('/', '/' | '*')
            )
            || (right == '=' && "+-*/%&|^<>!=".contains(left))
    }

    fn is_open_literal(token: SyntaxToken<'_>) -> bool {
        let text = token.text();
        kind(token) == K::TextBlockLiteral && !(text.len() >= 6 && text.ends_with("\"\"\""))
    }

    fn hugs_next(token: SyntaxToken<'_>) -> bool {
        Self::closer_for(token).is_some() || matches!(kind(token), K::Dot | K::DoubleColon | K::At)
    }

    fn place_comment<'a>(context: &CommentContext<'a, '_>) -> Option<Placement<'a>> {
        let prev = context.prev?;
        body_comment(context, prev)
            .or_else(|| crate::rules::statements::promoted_body_comment(context))
            .or_else(|| block_comment_after_opener(context, prev))
    }

    fn ignore_scope(enclosing: SyntaxNode<'_>) -> Option<SyntaxNode<'_>> {
        match node_kind(enclosing) {
            K::CompilationUnit
            | K::ClassBody
            | K::InterfaceBody
            | K::EnumBody
            | K::RecordBody
            | K::AnnotationInterfaceBody
            | K::Block
            | K::ConstructorBody
            | K::SwitchBlockStatementGroup
            | K::ModuleDeclaration => Some(enclosing),
            K::CompilationUnitItemList
            | K::ClassBodyMemberList
            | K::InterfaceBodyMemberList
            | K::RecordBodyMemberList
            | K::AnnotationInterfaceBodyMemberList
            | K::BlockStatementList
            | K::ConstructorBodyEntryList
            | K::ModuleDirectiveList => enclosing.parent().and_then(Self::ignore_scope),
            _ => None,
        }
    }
}

/// The element after `element` among its parent's children.
fn next_sibling<'a>(
    parent: SyntaxNode<'a>,
    element: SyntaxElement<'a>,
) -> Option<SyntaxElement<'a>> {
    let mut children = parent.children();
    children.find(|&child| child == element)?;
    children.next()
}

/// H1: a comment between a header and its body moves into the body. Before
/// a body's `{` (of a declaration, control statement, lambda, switch rule or
/// initializer) that is every comment but an inline one; before the
/// non-block body of a control statement, an end-of-line comment or an
/// own-line line comment.
fn body_comment<'a>(
    context: &CommentContext<'a, '_>,
    prev: SyntaxToken<'a>,
) -> Option<Placement<'a>> {
    let comment = context.comment;
    let mut next = context.next;
    // S1 would move an end-of-line line comment past a separator that the
    // `{` follows (a trailing separator, recovery), so it goes in as well.
    if comment.kind == CommentKind::Line
        && comment.layout == CommentLayout::EndOfLine
        && JavaFormat::is_separator(next)
        && !JavaFormat::is_separator(prev)
        && let Some(open) = next.next().filter(|&open| kind(open) == K::LBrace)
    {
        next = open;
    }
    let body = if kind(next) == K::LBrace {
        if comment.layout == CommentLayout::Inline {
            return None;
        }
        braced_body(context, prev, next)?
    } else {
        let applies = match comment.layout {
            CommentLayout::EndOfLine => true,
            CommentLayout::OwnLine | CommentLayout::StartOfLine => {
                comment.kind == CommentKind::Line
            }
            CommentLayout::Inline => false,
        };
        if !applies {
            return None;
        }
        let body = control_body(prev)?;
        if body.first_token() != Some(next) || node_kind(body).is_list() {
            return None;
        }
        return Some(Placement::Leading(
            SyntaxElement::Node(body),
            CommentLayout::OwnLine,
        ));
    };
    let inner = next
        .next()
        .filter(|&inner| !(inner.parent() == body && JavaFormat::is_closer(inner)));
    Some(match inner {
        Some(inner) => Placement::Leading(
            leading_target::<JavaLanguage>(inner),
            CommentLayout::OwnLine,
        ),
        None => Placement::Dangling(body),
    })
}

/// The body that `open` begins, when it follows a header in the same parent.
/// A labeled statement's body and a nested block start their own line, and a
/// header with an unclosed opener before `prev` (recovery) does not end at
/// the body.
fn braced_body<'a>(
    context: &CommentContext<'a, '_>,
    prev: SyntaxToken<'a>,
    open: SyntaxToken<'a>,
) -> Option<SyntaxNode<'a>> {
    let body = open.parent();
    if body.is_malformed()
        || !matches!(
            node_kind(body),
            K::ClassBody
                | K::InterfaceBody
                | K::EnumBody
                | K::RecordBody
                | K::AnnotationInterfaceBody
                | K::Block
                | K::ConstructorBody
                | K::SwitchBlock
        )
    {
        return None;
    }
    let header = body.parent()?;
    if header.first_token() == Some(open)
        || header.parent().is_none()
        || node_kind(header) == K::LabeledStatement
    {
        return None;
    }
    let mut node = prev.parent();
    loop {
        if context.has_unclosed_child_until(node, prev) {
            return None;
        }
        if node == header {
            return Some(body);
        }
        node = node.parent()?;
    }
}

/// The non-block body of the control statement whose header `prev` ends
/// (the `)` of `if`, `while`, `for`, `switch`, `synchronized` or `catch`, a
/// resource specification, or `else`, `do`), not an `else if`.
fn control_body(prev: SyntaxToken<'_>) -> Option<SyntaxNode<'_>> {
    let parent = prev.parent();
    let header_end = match kind(prev) {
        K::RParen => match node_kind(parent) {
            K::IfStatement
            | K::WhileStatement
            | K::BasicForStatement
            | K::EnhancedForStatement
            | K::SwitchStatement
            | K::SwitchExpression
            | K::SynchronizedStatement
            | K::CatchClause => SyntaxElement::Token(prev),
            K::ResourceSpecification => SyntaxElement::Node(parent),
            _ => return None,
        },
        K::ElseKw | K::DoKw => SyntaxElement::Token(prev),
        _ => return None,
    };
    let container = match header_end {
        SyntaxElement::Node(node) => node.parent()?,
        SyntaxElement::Token(_) => parent,
    };
    let body = next_sibling(container, header_end)?.as_node()?;
    if body.is_malformed() || (kind(prev) == K::ElseKw && node_kind(body) == K::IfStatement) {
        return None;
    }
    Some(body)
}

/// H5: a block comment that ends the line after `(`, `[` or a type-argument
/// `<` stays inline before the next token instead of
/// moving to its own line by G2 (which still applies after `{`). After an
/// assignment operator it leads the value inline too, which is where the next
/// pass (reading it as inline) puts it, so a long declaration breaks the same
/// way on both passes.
fn block_comment_after_opener<'a>(
    context: &CommentContext<'a, '_>,
    prev: SyntaxToken<'a>,
) -> Option<Placement<'a>> {
    let comment = context.comment;
    let next = context.next;
    let opener = match kind(prev) {
        K::LParen | K::LBracket => true,
        K::Lt => is_angle_list(prev),
        kind => is_assignment_operator(kind),
    };
    let empty = JavaFormat::is_closer(next) && next.parent() == prev.parent();
    // Without the closer or the value (recovery), what follows belongs to an
    // enclosing node, and the comment stays where the generic rules put it.
    let inside = || match JavaFormat::closer_for(prev) {
        Some(_) => !context.is_unclosed(prev),
        None => prev
            .parent()
            .last_token()
            .is_some_and(|last| last.id() >= next.id()),
    };
    if comment.kind == CommentKind::Line
        || comment.layout != CommentLayout::EndOfLine
        || !opener
        || empty
        || !inside()
    {
        return None;
    }
    Some(Placement::Leading(
        leading_target::<JavaLanguage>(next),
        CommentLayout::Inline,
    ))
}
