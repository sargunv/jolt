//! Type bodies, enum constants, and stray `;` members.

use jolt_fmt::{Doc, ListItem, cat};
use jolt_java_syntax::{
    AnnotationInterfaceBody, AnnotationInterfaceBodyMemberList, ClassBody, ClassBodyMemberList,
    EmptyDeclaration, EnumBody, EnumConstant, EnumConstantList, InterfaceBody,
    InterfaceBodyMemberList, RecordBody, RecordBodyMemberList,
};
use jolt_syntax::{AstNode, ListElement, SyntaxElement, SyntaxNode, SyntaxToken};

use crate::JavaFormatter;
use crate::helpers::bodies::{body_items, braced, braced_body, member_separator};
use crate::helpers::spaced_node;
use crate::rules::types::annotation_lines;

pub(crate) fn class_body<'a>(f: &mut JavaFormatter<'a>, node: ClassBody<'a>) -> Doc {
    let members = node
        .members()
        .into_iter()
        .flat_map(ClassBodyMemberList::items);
    member_body(
        f,
        node.syntax(),
        node.open_brace(),
        members,
        node.close_brace(),
    )
}

pub(crate) fn interface_body<'a>(f: &mut JavaFormatter<'a>, node: InterfaceBody<'a>) -> Doc {
    let members = node
        .members()
        .into_iter()
        .flat_map(InterfaceBodyMemberList::items);
    member_body(
        f,
        node.syntax(),
        node.open_brace(),
        members,
        node.close_brace(),
    )
}

pub(crate) fn record_body<'a>(f: &mut JavaFormatter<'a>, node: RecordBody<'a>) -> Doc {
    let members = node
        .members()
        .into_iter()
        .flat_map(RecordBodyMemberList::items);
    member_body(
        f,
        node.syntax(),
        node.open_brace(),
        members,
        node.close_brace(),
    )
}

pub(crate) fn annotation_interface_body<'a>(
    f: &mut JavaFormatter<'a>,
    node: AnnotationInterfaceBody<'a>,
) -> Doc {
    let members = node
        .members()
        .into_iter()
        .flat_map(AnnotationInterfaceBodyMemberList::items);
    member_body(
        f,
        node.syntax(),
        node.open_brace(),
        members,
        node.close_brace(),
    )
}

/// `{`, the members one indent deeper, `}`.
fn member_body<'a, M: AstNode<'a>>(
    f: &mut JavaFormatter<'a>,
    body: SyntaxNode<'a>,
    open: Option<SyntaxToken<'a>>,
    members: impl Iterator<Item = M>,
    close: Option<SyntaxToken<'a>>,
) -> Doc {
    let members = body_items(body, members.map(AstNode::syntax));
    braced_body(f, body, open, members, close, member_separator)
}

/// A stray `;` member. Normally removed (N5): its comments stand in its
/// place through `item_list`. Formatted only when the body has recovery,
/// where N5 does not apply.
pub(crate) fn empty_declaration<'a>(f: &mut JavaFormatter<'a>, node: EmptyDeclaration<'a>) -> Doc {
    f.token_opt(node.semicolon())
}

/// Constants one per line; then, after a blank line, the members. Without
/// members, every constant is followed by a comma (a synthesized one after
/// the last), and the `;` is removed, its comments in place. With members,
/// the `;` follows the last constant, and a comma before it is removed (N5,
/// N8); a comment on that comma or on the `;` leaves both as written, the
/// `;` on its own line after a comma.
pub(crate) fn enum_body<'a>(f: &mut JavaFormatter<'a>, node: EnumBody<'a>) -> Doc {
    let open = f.token_opt(node.open_brace());
    let members = node
        .members()
        .into_iter()
        .flat_map(ClassBodyMemberList::items)
        .map(AstNode::syntax);
    let members = body_items(node.syntax(), members);
    let has_members = members
        .iter()
        .any(|member| matches!(member, ListItem::Format(_)));
    let constants = enum_constants(f, node, has_members);
    let members = f.item_list(members, member_separator);
    let content = match (constants.is_nil(), members.is_nil()) {
        (false, false) => cat![f; constants, f.empty_line(), members],
        (true, _) => members,
        (_, true) => constants,
    };
    let close = f.token_opt(node.close_brace());
    braced(f, Some(node.syntax()), open, content, close)
}

/// The constants with their separators and the `;` (see `enum_body`).
fn enum_constants<'a>(f: &mut JavaFormatter<'a>, node: EnumBody<'a>, has_members: bool) -> Doc {
    let separator = node.body_separator();
    let elements: Vec<_> = node
        .constants()
        .into_iter()
        .flat_map(EnumConstantList::elements)
        .filter(|element| !matches!(element, ListElement::Missing))
        .collect();
    let last_constant = elements
        .iter()
        .rposition(|element| matches!(element, ListElement::Item(_) | ListElement::Bogus(_)));
    let trailing_comma = elements[last_constant.map_or(0, |last| last + 1)..]
        .iter()
        .find_map(|element| match element {
            ListElement::Separator(comma) => Some(*comma),
            _ => None,
        });
    let commented = |token: Option<SyntaxToken<'a>>| {
        token.is_some_and(|token| f.has_comments(SyntaxElement::Token(token)))
    };
    // With members, a comment on the trailing comma or the `;` leaves both
    // as written. The body owns the separators: when it is recovered (an
    // unclosed body whose constants run into what follows), a synthesized
    // comma could join the constants to the next tokens.
    let as_written = has_members && (commented(trailing_comma) || commented(separator));
    let normalize = node.syntax().is_recovery_free() && !as_written;
    // With members, the `;` moves up to the last constant, past the comma.
    let move_semicolon = normalize && has_members && last_constant.is_some();
    let mut constants = Vec::new();
    // Without members, the `;` is redundant: it is removed, and a comma is
    // synthesized after the last constant unless one follows it.
    let synthesize_comma = normalize && !has_members;
    let mut needs_comma = false;
    for element in elements {
        let constant = match element {
            ListElement::Item(constant) => constant.syntax(),
            ListElement::Bogus(bogus) => bogus,
            ListElement::Separator(comma) => {
                needs_comma = false;
                if !(move_semicolon && Some(comma) == trailing_comma) {
                    constants.push(f.token(comma));
                }
                continue;
            }
            ListElement::Missing => continue,
        };
        if !constants.is_empty() {
            constants.push(f.hard_line());
        }
        constants.push(f.node(constant));
        needs_comma = synthesize_comma;
    }
    match separator {
        Some(_) if move_semicolon => constants.push(f.sticky(";")),
        Some(semicolon) if has_members || !normalize => {
            if trailing_comma.is_some() && as_written {
                constants.push(f.hard_line());
            }
            constants.push(f.token(semicolon));
        }
        separator => {
            if needs_comma {
                constants.push(f.sticky(","));
            }
            if let Some(semicolon) = separator {
                constants.push(f.removed(SyntaxElement::Token(semicolon)));
            }
        }
    }
    f.concat(constants)
}

/// The annotations one per line, then `NAME(arguments) {body}`.
pub(crate) fn enum_constant<'a>(f: &mut JavaFormatter<'a>, node: EnumConstant<'a>) -> Doc {
    let mut docs = vec![annotation_lines(f, node.annotations())];
    docs.push(f.token_opt(node.name()));
    docs.push(f.fmt_opt(node.arguments()));
    docs.push(spaced_node(f, node.body()));
    f.concat(docs)
}
