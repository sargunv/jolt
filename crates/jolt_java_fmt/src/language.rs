//! The Java `FormatLanguage`: separator and delimiter tables, the
//! formatter-ignore scopes, and comment hooks H1 to H5.

use jolt_fmt::{
    CommentContext, CommentKind, CommentLayout, Doc, FormatLanguage, Formatter, Placement,
    leading_target, trailing_target,
};
use jolt_java_syntax::{
    IfStatement, JavaLanguage, JavaSyntaxKind as K, Statement, is_assignment_operator,
};
use jolt_syntax::{AstNode, RawSyntaxKind, SyntaxElement, SyntaxNode, SyntaxToken};

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

    /// `,` and `;`.
    fn is_separator(token: SyntaxToken<'_>) -> bool {
        matches!(kind(token), K::Comma | K::Semicolon)
    }

    /// `(`, `[`, `{`, and the `<` of type arguments and type parameters
    /// (H2).
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

    /// `)`, `]`, `}`, and the `>` of type arguments and type parameters
    /// (H2).
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

    /// An unterminated text block.
    fn is_open_literal(token: SyntaxToken<'_>) -> bool {
        let text = token.text();
        kind(token) == K::TextBlockLiteral && !(text.len() >= 6 && text.ends_with("\"\"\""))
    }

    /// The openers, `.`, `::` and `@`.
    fn hugs_next(token: SyntaxToken<'_>) -> bool {
        Self::closer_for(token).is_some() || matches!(kind(token), K::Dot | K::DoubleColon | K::At)
    }

    /// Hooks H1 to H5 (see `docs/internals/formatter.md`).
    fn place_comment<'a>(context: &CommentContext<'a, '_>) -> Option<Placement<'a>> {
        let prev = context.prev?;
        body_comment(context, prev)
            .or_else(|| promoted_body_comment(context, prev))
            .or_else(|| block_comment_after_opener(context, prev))
            .or_else(|| comment_before_stray_semicolon(context, prev))
    }

    /// The compilation unit, type bodies, blocks, constructor bodies, switch
    /// block statement groups and the module declaration. Their item list
    /// nodes map to the owner.
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

/// H1: a comment between a header and its body moves into the body: every
/// comment but an inline one before a body's `{` (of a declaration, control
/// statement, lambda, switch rule or initializer), and an end-of-line comment
/// or an own-line line comment before the non-block body of a control
/// statement.
fn body_comment<'a>(
    context: &CommentContext<'a, '_>,
    prev: SyntaxToken<'a>,
) -> Option<Placement<'a>> {
    let comment = context.comment;
    let next = context.next;
    // S1 would move an end-of-line line comment past a separator that the
    // `{` follows (a trailing separator, recovery), so it goes in as well.
    let open = if comment.kind == CommentKind::Line
        && comment.source_layout == CommentLayout::EndOfLine
        && JavaFormat::is_separator(next)
        && !JavaFormat::is_separator(prev)
    {
        next.next().filter(|&open| kind(open) == K::LBrace)
    } else {
        Some(next).filter(|&next| kind(next) == K::LBrace)
    };
    match open {
        Some(open) => braced_body_comment(context, prev, open),
        None => unbraced_body_comment(context, prev),
    }
}

/// H1 before the `{` of a body: the comment leads what follows the `{`, or
/// dangles in an empty body.
fn braced_body_comment<'a>(
    context: &CommentContext<'a, '_>,
    prev: SyntaxToken<'a>,
    open: SyntaxToken<'a>,
) -> Option<Placement<'a>> {
    if context.comment.source_layout == CommentLayout::Inline {
        return None;
    }
    let body = braced_body(context, prev, open)?;
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

/// H1 before a control statement's non-block body: the comment leads the
/// body on its own line.
fn unbraced_body_comment<'a>(
    context: &CommentContext<'a, '_>,
    prev: SyntaxToken<'a>,
) -> Option<Placement<'a>> {
    let comment = context.comment;
    let applies = match comment.source_layout {
        CommentLayout::EndOfLine => true,
        CommentLayout::OwnLine | CommentLayout::StartOfLine => comment.kind == CommentKind::Line,
        CommentLayout::Inline => false,
    };
    if !applies {
        return None;
    }
    let body = control_body(prev)?;
    if body.first_token() != Some(context.next) || node_kind(body).is_list() {
        return None;
    }
    Some(Placement::Leading(
        SyntaxElement::Node(body),
        CommentLayout::OwnLine,
    ))
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

/// H4, tried after H1: an end-of-line comment after the last token of a
/// control statement's final non-block body (`if (a) return; // c`) would
/// trail the whole statement by the generic rules, so it would print after
/// the synthesized `}` of brace promotion. It trails the body instead and
/// stays inside the block.
fn promoted_body_comment<'a>(
    context: &CommentContext<'a, '_>,
    prev: SyntaxToken<'a>,
) -> Option<Placement<'a>> {
    if context.comment.source_layout != CommentLayout::EndOfLine {
        return None;
    }
    let mut node = prev.parent();
    while node.last_token() == Some(prev) {
        let parent = node.parent()?;
        let kind = node_kind(node);
        let body = match node_kind(parent) {
            K::IfStatement => IfStatement::cast(parent).is_some_and(|statement| {
                let is_else = statement.else_branch().map(AstNode::syntax) == Some(node);
                !(is_else && kind == K::IfStatement)
            }),
            K::WhileStatement | K::BasicForStatement | K::EnhancedForStatement => true,
            _ => false,
        };
        if body && kind != K::Block && Statement::can_cast(node.kind()) {
            return Some(Placement::Trailing(
                SyntaxElement::Node(node),
                CommentLayout::EndOfLine,
            ));
        }
        node = parent;
    }
    None
}

/// H3: an end-of-line comment before a stray `;` (an empty statement or
/// declaration, which N5 removes) trails the code before it, where S1 would
/// move it onto the `;`. So `} // c` followed by `;` keeps the comment on
/// the `}` line whether the `;` is removed or stays (`case 1: // c` then a
/// group's only `;`). After an opener G2 still applies.
fn comment_before_stray_semicolon<'a>(
    context: &CommentContext<'a, '_>,
    prev: SyntaxToken<'a>,
) -> Option<Placement<'a>> {
    let next = context.next;
    let stray = kind(next) == K::Semicolon
        && matches!(
            node_kind(next.parent()),
            K::EmptyStatement | K::EmptyDeclaration
        );
    if context.comment.source_layout != CommentLayout::EndOfLine
        || !stray
        || JavaFormat::closer_for(prev).is_some()
    {
        return None;
    }
    Some(Placement::Trailing(
        trailing_target::<JavaLanguage>(prev),
        CommentLayout::EndOfLine,
    ))
}

fn is_header_clause(node: SyntaxNode<'_>) -> bool {
    matches!(
        node_kind(node),
        K::ExtendsClause | K::ImplementsClause | K::PermitsClause | K::ThrowsClause
    )
}

/// H5: a block comment that ends the line after `(`, `[` or a type-argument
/// `<` stays inline before the next token instead of moving to its own line
/// by G2 (which still applies after `{`). After an assignment operator, and
/// before the first clause of a declaration header (`extends`, `implements`,
/// `permits`, `throws`), it leads the value or the clause inline too. That is
/// where the next pass, reading it as inline, puts it, so the line breaks the
/// same way on both passes. (Between two clauses, which break together, the
/// generic rules are already stable.)
fn block_comment_after_opener<'a>(
    context: &CommentContext<'a, '_>,
    prev: SyntaxToken<'a>,
) -> Option<Placement<'a>> {
    let comment = context.comment;
    let next = context.next;
    if comment.kind == CommentKind::Line || comment.source_layout != CommentLayout::EndOfLine {
        return None;
    }
    let clause = next.parent();
    let before_first_clause = is_header_clause(clause)
        && clause.first_token() == Some(next)
        && !std::iter::successors(Some(prev.parent()), |node| node.parent()).any(is_header_clause);
    if before_first_clause {
        return Some(Placement::Leading(
            SyntaxElement::Node(clause),
            CommentLayout::Inline,
        ));
    }
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
    if !opener || empty || !inside() {
        return None;
    }
    Some(Placement::Leading(
        leading_target::<JavaLanguage>(next),
        CommentLayout::Inline,
    ))
}
