//! Type bodies, enum constants, and stray `;` members.

use jolt_fmt::{Doc, ListItem};
use jolt_java_syntax::{
    AnnotationInterfaceBody, ClassBody, EmptyDeclaration, EnumBody, EnumConstant, InterfaceBody,
    JavaSyntaxKind as K, RecordBody,
};
use jolt_syntax::{AstNode, ListElement, SyntaxElement, SyntaxNode};

use crate::JavaFormatter;
use crate::helpers::bodies::{braced, braced_body, member_separator};
use crate::rules::types::annotation_lines;

pub(crate) fn class_body<'a>(f: &mut JavaFormatter<'a>, node: ClassBody<'a>) -> Doc {
    let members = member_items(
        node.syntax(),
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
        node.syntax(),
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
        node.syntax(),
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

pub(crate) fn annotation_interface_body<'a>(
    f: &mut JavaFormatter<'a>,
    node: AnnotationInterfaceBody<'a>,
) -> Doc {
    let members = member_items(
        node.syntax(),
        node.members()
            .into_iter()
            .flat_map(jolt_java_syntax::AnnotationInterfaceBodyMemberList::items),
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

/// Removed (N5): its comments stand in its place through `item_list`. Only
/// reached when a list formats it as an item, which none does.
pub(crate) fn empty_declaration<'a>(f: &mut JavaFormatter<'a>, node: EmptyDeclaration<'a>) -> Doc {
    f.token_opt(node.semicolon())
}

/// Members as list items, stray `;` elided (N5) when the body is
/// recovery-free.
fn member_items<'a, M: AstNode<'a>>(
    body: SyntaxNode<'a>,
    members: impl Iterator<Item = M>,
) -> Vec<ListItem<'a>> {
    let normalize = body.is_recovery_free();
    members
        .map(|member| {
            let syntax = member.syntax();
            if normalize && K::from_raw(syntax.kind()) == K::EmptyDeclaration {
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
        node.syntax(),
        node.members()
            .into_iter()
            .flat_map(jolt_java_syntax::ClassBodyMemberList::items),
    );
    let has_members = members
        .iter()
        .any(|member| matches!(member, ListItem::Format(_)));
    let (constants, semicolon_comments) = enum_constants(f, node, has_members);
    let members = f.item_list(members, member_separator);
    let mut content = vec![constants];
    if !semicolon_comments.is_nil() {
        content.extend([f.empty_line(), semicolon_comments]);
    }
    if !members.is_nil() {
        if content.iter().any(|doc| !doc.is_nil()) {
            content.push(f.empty_line());
        }
        content.push(members);
    }
    let content = f.concat(content);
    let close = f.token_opt(node.close_brace());
    braced(f, Some(node.syntax()), open, content, close)
}

/// The constants with their separators and the `;`, and the comments that
/// start the section after them. The `;` that ends the constants moves up to
/// the last constant, past a removed trailing comma, whose comments follow it
/// on its line (`B, // c` / `;` prints `B; // c`). When the comma or the `;`
/// has a comment that starts a line before it, or the `;` has comments after
/// the comma's line comment, the comments of both start the section after
/// the constants, after a blank line, where the next pass finds them (`B,` /
/// `// c` / `;` prints `B;`, a blank line, `// c`).
fn enum_constants<'a>(
    f: &mut JavaFormatter<'a>,
    node: EnumBody<'a>,
    has_members: bool,
) -> (Doc, Doc) {
    // The body owns the separators: when it is recovered (an unclosed body
    // whose constants run into what follows), a synthesized comma could
    // join the constants to the next tokens.
    let normalize = node.syntax().is_recovery_free();
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
    // A comma is synthesized after the last constant unless one follows it,
    // or, as in the reference, it ends its line with a line comment.
    let synthesize_comma = normalize && separator.is_none() && !has_members;
    let mut needs_comma = false;
    for (index, element) in elements.into_iter().enumerate() {
        let constant = match element {
            ListElement::Item(constant) => constant.syntax(),
            ListElement::Bogus(bogus) => bogus,
            ListElement::Separator(comma) => {
                needs_comma = false;
                if last_constant.is_some_and(|last| index > last)
                    && normalize
                    && separator.is_some()
                {
                    removed_comma = Some(comma);
                } else {
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
        needs_comma =
            synthesize_comma && !f.has_trailing_line_comment(SyntaxElement::Node(constant));
    }
    let mut semicolon_comments = Doc::NIL;
    match separator {
        Some(semicolon) if normalize && last_constant.is_some() => {
            constants.push(f.sticky(";"));
            let comma = removed_comma.map(SyntaxElement::Token);
            let semicolon = SyntaxElement::Token(semicolon);
            // Nothing follows a line comment on its line.
            let after_line_comment = comma.is_some_and(|comma| f.has_trailing_line_comment(comma))
                && f.has_trailing_comments(semicolon);
            if comma.is_some_and(|comma| f.leading_comments_break(comma))
                || f.has_leading_comments(semicolon)
                || after_line_comment
            {
                let moved: Vec<_> = comma.into_iter().chain([semicolon]).collect();
                semicolon_comments = f.removed_on_own_lines(&moved);
            } else {
                if let Some(comma) = comma {
                    constants.push(f.removed(comma));
                }
                constants.push(f.removed(semicolon));
            }
        }
        Some(semicolon) if has_members || !normalize => {
            constants.push(f.token(semicolon));
        }
        Some(semicolon) => constants.push(f.removed(SyntaxElement::Token(semicolon))),
        None if needs_comma => constants.push(f.sticky(",")),
        None => {}
    }
    (f.concat(constants), semicolon_comments)
}

/// The annotations one per line, then `NAME(arguments) {body}`.
pub(crate) fn enum_constant<'a>(f: &mut JavaFormatter<'a>, node: EnumConstant<'a>) -> Doc {
    let mut docs = vec![annotation_lines(f, node.annotations())];
    docs.push(f.token_opt(node.name()));
    docs.push(f.fmt_opt(node.arguments()));
    if let Some(body) = node.body() {
        docs.extend([f.space(), f.fmt(body)]);
    }
    f.concat(docs)
}
