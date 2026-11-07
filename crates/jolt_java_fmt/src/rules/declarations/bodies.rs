//! Type bodies, enum constants, and stray `;` members.

use jolt_fmt::{Doc, ListItem, cat};
use jolt_java_syntax::{
    AnnotationInterfaceBody, ClassBody, EmptyDeclaration, EnumBody, EnumConstant, InterfaceBody,
    JavaSyntaxKind as K, RecordBody,
};
use jolt_syntax::{AstNode, ListElement, SyntaxElement, SyntaxNode};

use crate::JavaFormatter;
use crate::helpers::bodies::{braced_body, member_separator};

pub(crate) fn class_body<'a>(f: &mut JavaFormatter<'a>, node: ClassBody<'a>) -> Doc {
    let members = member_items(
        node.members()
            .into_iter()
            .flat_map(jolt_java_syntax::ClassBodyMemberList::items),
    );
    braced_body(
        f,
        node.syntax(),
        node.open_brace(),
        members,
        node.close_brace(),
        member_separator,
    )
}

pub(crate) fn interface_body<'a>(f: &mut JavaFormatter<'a>, node: InterfaceBody<'a>) -> Doc {
    let members = member_items(
        node.members()
            .into_iter()
            .flat_map(jolt_java_syntax::InterfaceBodyMemberList::items),
    );
    braced_body(
        f,
        node.syntax(),
        node.open_brace(),
        members,
        node.close_brace(),
        member_separator,
    )
}

pub(crate) fn record_body<'a>(f: &mut JavaFormatter<'a>, node: RecordBody<'a>) -> Doc {
    let members = member_items(
        node.members()
            .into_iter()
            .flat_map(jolt_java_syntax::RecordBodyMemberList::items),
    );
    braced_body(
        f,
        node.syntax(),
        node.open_brace(),
        members,
        node.close_brace(),
        member_separator,
    )
}

/// The members sit in an `AnnotationElementList` node, which is not a list
/// kind, so comments before the first member and after the last attach to
/// it; they print around the members.
pub(crate) fn annotation_interface_body<'a>(
    f: &mut JavaFormatter<'a>,
    node: AnnotationInterfaceBody<'a>,
) -> Doc {
    let open = f.token_opt(node.open_brace());
    let content = match node.elements() {
        Some(elements) if !elements.syntax().is_malformed() => {
            let members = member_items(
                elements
                    .declarations()
                    .into_iter()
                    .flat_map(jolt_java_syntax::AnnotationInterfaceBodyMemberList::items),
            );
            // The wrapper's leading comments lead its first member; when that
            // member is a removed `;`, they stand in its place, so the next
            // member starts its own line, as `item_list` does for an
            // elided item.
            let first_elided = matches!(members.first(), Some(ListItem::Elide(_)));
            let leading = f.leading_comments(elements.syntax());
            let leading = if first_elided && !leading.is_nil() {
                cat![f; leading, f.hard_line()]
            } else {
                leading
            };
            let members = f.item_list(members, member_separator);
            let trailing = f.trailing_comments(elements.syntax());
            cat![f; leading, members, trailing]
        }
        elements => f.fmt_opt(elements),
    };
    let close = f.token_opt(node.close_brace());
    braced(f, node.syntax(), open, content, close)
}

/// Removed (N5): its comments stand in its place through `item_list`. Only
/// reached when a list formats it as an item, which none does.
pub(crate) fn empty_declaration<'a>(f: &mut JavaFormatter<'a>, node: EmptyDeclaration<'a>) -> Doc {
    f.token_opt(node.semicolon())
}

/// Members as list items, stray `;` elided (N5).
fn member_items<'a, M: AstNode<'a>>(members: impl Iterator<Item = M>) -> Vec<ListItem<'a>> {
    members
        .map(|member| {
            let syntax = member.syntax();
            if K::from_raw(syntax.kind()) == K::EmptyDeclaration {
                ListItem::Elide(syntax)
            } else {
                ListItem::Format(syntax)
            }
        })
        .collect()
}

/// Constants one per line; then, after a blank line, the members. Without
/// members, every constant is followed by a comma (a synthesized one after
/// the last) unless the source ends the constants with `;`, which is kept.
/// With members, the last constant is followed by the `;`. A comma before
/// the `;` is removed, and so is a `;` with neither constants nor members
/// (N5, N8).
pub(crate) fn enum_body<'a>(f: &mut JavaFormatter<'a>, node: EnumBody<'a>) -> Doc {
    let open = f.token_opt(node.open_brace());
    let members = member_items(
        node.members()
            .into_iter()
            .flat_map(jolt_java_syntax::ClassBodyMemberList::items),
    );
    let has_members = members
        .iter()
        .any(|member| matches!(member, ListItem::Format(_)));
    let normalize = node
        .constants()
        .is_none_or(|constants| constants.syntax().is_recovery_free());
    let separator = node.body_separator();

    let elements: Vec<_> = node
        .constants()
        .into_iter()
        .flat_map(jolt_java_syntax::EnumConstantList::elements)
        .filter(|element| !matches!(element, ListElement::Missing))
        .collect();
    let last_constant = elements
        .iter()
        .rposition(|element| matches!(element, ListElement::Item(_) | ListElement::Bogus(_)));
    let mut constants = Vec::new();
    let mut removed_comma = None;
    let mut trailing_comma = false;
    for (index, element) in elements.into_iter().enumerate() {
        match element {
            ListElement::Item(constant) => {
                if !constants.is_empty() {
                    constants.push(f.hard_line());
                }
                constants.push(f.fmt(constant));
            }
            ListElement::Bogus(bogus) => {
                if !constants.is_empty() {
                    constants.push(f.hard_line());
                }
                constants.push(f.node(bogus));
            }
            ListElement::Separator(comma) => {
                let trailing = last_constant.is_some_and(|last| index > last);
                if trailing && normalize && separator.is_some() {
                    removed_comma = Some(comma);
                } else {
                    trailing_comma |= trailing;
                    constants.push(f.token(comma));
                }
            }
            ListElement::Missing => {}
        }
    }
    // With members, the comments of a removed trailing comma start the
    // section after the constants, after a blank line, where the next pass
    // finds them leading the first member (`B, // c` `;` members prints `B;`,
    // a blank line, `// c`, the members), as in the reference. Comments on
    // their own lines before the `;` stay there too, and the `;` moves up to
    // the last constant: it is re-emitted there and its comments print with
    // the comma's. Without members, the `;` stays in place and the comma's
    // comments follow it (`B; // c`): there, after a blank line, they would
    // be dangling comments, which print one per line.
    let move_down = normalize && has_members && last_constant.is_some();
    let mut moved_semicolon = None;
    match separator {
        Some(semicolon) if move_down && f.has_leading_comments(SyntaxElement::Token(semicolon)) => {
            constants.push(f.sticky(";"));
            moved_semicolon = Some(semicolon);
        }
        Some(semicolon) if last_constant.is_some() || has_members || !normalize => {
            constants.push(f.token(semicolon));
        }
        Some(semicolon) => constants.push(f.removed(SyntaxElement::Token(semicolon))),
        None if last_constant.is_some() && !trailing_comma && !has_members && normalize => {
            constants.push(f.sticky(","));
        }
        None => {}
    }
    let mut removed = Vec::new();
    for token in [removed_comma, moved_semicolon].into_iter().flatten() {
        removed.push(f.removed(SyntaxElement::Token(token)));
    }
    let removed = f.concat(removed);
    if !move_down {
        constants.push(removed);
    }
    let constants = f.concat(constants);
    let members = f.item_list(members, member_separator);
    let mut content = vec![constants];
    let mut before_members = f.empty_line();
    if move_down && !removed.is_nil() {
        content.extend([f.empty_line(), removed]);
        before_members = f.hard_line();
    }
    if !members.is_nil() {
        if !constants.is_nil() {
            content.push(before_members);
        }
        content.push(members);
    }
    let content = f.concat(content);
    let close = f.token_opt(node.close_brace());
    braced(f, node.syntax(), open, content, close)
}

/// The annotations one per line, then `NAME(arguments) {body}`.
pub(crate) fn enum_constant<'a>(f: &mut JavaFormatter<'a>, node: EnumConstant<'a>) -> Doc {
    let mut docs = Vec::new();
    for annotation in node
        .annotations()
        .into_iter()
        .flat_map(jolt_java_syntax::AnnotationList::items)
    {
        docs.extend([f.fmt(annotation), f.hard_line()]);
    }
    docs.push(f.token_opt(node.name()));
    docs.push(f.fmt_opt(node.arguments()));
    if let Some(body) = node.body() {
        docs.extend([f.space(), f.fmt(body)]);
    }
    f.concat(docs)
}

/// REQUEST(shared): `braced_body` with already formatted contents, for the
/// bodies whose contents are not one item list (enum bodies) or sit inside a
/// wrapper node (annotation interface bodies). Must stay in step with
/// `helpers::bodies::braced_body`, including D1.
fn braced<'a>(
    f: &mut JavaFormatter<'a>,
    owner: SyntaxNode<'a>,
    open: Doc,
    content: Doc,
    close: Doc,
) -> Doc {
    let dangling = f.dangling(owner);
    let mut inner = Vec::with_capacity(4);
    if !content.is_nil() {
        inner.extend([f.hard_line(), content]);
    }
    if !dangling.is_nil() {
        let blank = !content.is_nil() && f.blank_line_before_dangling(owner);
        inner.extend([if blank { f.empty_line() } else { f.hard_line() }, dangling]);
    }
    let inner = f.concat(inner);
    let inner = f.indent(inner);
    cat![f; open, inner, f.hard_line(), close]
}
