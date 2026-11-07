//! The Java `FormatLanguage`: separator and delimiter tables, the
//! formatter-ignore scopes, and comment hooks H1–H5.

use jolt_fmt::{
    CommentContext, CommentKind, CommentLayout, Doc, FormatLanguage, Formatter, Placement,
    is_unclosed, leading_target, trailing_target,
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
    const EXPAND_DOC_COMMENTS: bool = true;

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

    fn place_comment<'a>(context: &CommentContext<'a>) -> Option<Placement<'a>> {
        let prev = context.prev?;
        control_body_comment(context, prev)
            .or_else(|| crate::rules::statements::promoted_body_comment(context))
            .or_else(|| comment_before_body_brace(context, prev))
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

/// H1: a comment between a control header and its body moves to the first
/// line of the body.
fn control_body_comment<'a>(
    context: &CommentContext<'a>,
    prev: SyntaxToken<'a>,
) -> Option<Placement<'a>> {
    let comment = context.comment;
    let applies = match comment.layout {
        CommentLayout::EndOfLine => true,
        CommentLayout::OwnLine | CommentLayout::StartOfLine => comment.kind == CommentKind::Line,
        CommentLayout::Inline => false,
    };
    if !applies {
        return None;
    }
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
        K::ElseKw | K::DoKw | K::TryKw | K::FinallyKw => SyntaxElement::Token(prev),
        _ => return None,
    };
    let container = match header_end {
        SyntaxElement::Node(node) => node.parent()?,
        SyntaxElement::Token(_) => parent,
    };
    let body = next_sibling(container, header_end)?.as_node()?;
    // `try` and `finally` own a block; after `try` a resource specification
    // or (when the block is missing) the catch clauses come next instead.
    let block_only = matches!(kind(prev), K::TryKw | K::FinallyKw);
    if body.is_malformed()
        || body.first_token() != Some(context.next)
        || node_kind(body).is_list()
        || (block_only && node_kind(body) != K::Block)
    {
        return None;
    }
    if kind(prev) == K::ElseKw && node_kind(body) == K::IfStatement {
        return None;
    }
    if !matches!(node_kind(body), K::Block | K::SwitchBlock) {
        return Some(Placement::Leading(
            SyntaxElement::Node(body),
            CommentLayout::OwnLine,
        ));
    }
    let open = body.first_token()?;
    let inner = open
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

/// H3: a comment on its own line before a body's `{` that follows a header
/// moves onto the header line.
fn comment_before_body_brace<'a>(
    context: &CommentContext<'a>,
    prev: SyntaxToken<'a>,
) -> Option<Placement<'a>> {
    let next = context.next;
    if kind(next) != K::LBrace
        || !matches!(
            context.comment.layout,
            CommentLayout::OwnLine | CommentLayout::StartOfLine
        )
    {
        return None;
    }
    let body = next.parent();
    if !matches!(
        node_kind(body),
        K::ClassBody
            | K::InterfaceBody
            | K::EnumBody
            | K::RecordBody
            | K::AnnotationInterfaceBody
            | K::Block
            | K::ConstructorBody
            | K::SwitchBlock
    ) {
        return None;
    }
    let header = body.parent()?;
    // A labeled statement's body starts its own line, so a comment before
    // its `{` already prints where the next pass reads it. A header that
    // ends in an opener is unclosed (recovery); the comment stays on its line.
    if header.first_token() == Some(next)
        || header.parent().is_none()
        || node_kind(header) == K::LabeledStatement
        || JavaFormat::closer_for(prev).is_some()
    {
        return None;
    }
    let layout = if context.comment.kind == CommentKind::Line {
        CommentLayout::EndOfLine
    } else {
        CommentLayout::Inline
    };
    Some(Placement::Trailing(
        trailing_target::<JavaLanguage>(prev),
        layout,
    ))
}

/// H5: a block comment that ends the line after `(`, `[` or a type-argument
/// `<` stays inline before the next token, as in the reference, instead of
/// moving to its own line by G2 (which still applies after `{`). After an
/// assignment operator it leads the value inline too, which is where the next
/// pass (reading it as inline) puts it, so a long declaration breaks the same
/// way on both passes.
fn block_comment_after_opener<'a>(
    context: &CommentContext<'a>,
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
        Some(_) => !is_unclosed::<JavaFormat>(prev),
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
