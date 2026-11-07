//! Class declarations and class bodies.

use jolt_fmt::{Doc, ListItem, cat, indent};
use jolt_java_syntax::{ClassBody, ClassBodyMember, ClassDeclaration};
use jolt_syntax::AstNode;

use crate::JavaFormatter;
use crate::helpers::bodies::{braced_body, member_separator};

pub(crate) fn class_declaration<'a>(f: &mut JavaFormatter<'a>, node: ClassDeclaration<'a>) -> Doc {
    let modifiers = f.fmt_opt(node.modifiers());
    let mut header = vec![
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
    for clause in clauses.into_iter().flatten() {
        let clause = f.node(clause);
        header.push(indent![f; f.line(), clause]);
    }
    let header = f.concat(header);
    let header = f.group(header);
    let body = match node.body() {
        Some(body) => {
            let body = f.fmt(body);
            cat![f; f.space(), body]
        }
        None => Doc::NIL,
    };
    let semicolon = f.token_opt(node.missing_body_semicolon());
    cat![f; modifiers, header, body, semicolon]
}

pub(crate) fn class_body<'a>(f: &mut JavaFormatter<'a>, node: ClassBody<'a>) -> Doc {
    let members = node
        .members()
        .into_iter()
        .flat_map(jolt_java_syntax::ClassBodyMemberList::items)
        .map(|member| {
            let syntax = member.syntax();
            if let ClassBodyMember::EmptyDeclaration(_) = member {
                ListItem::Elide(syntax)
            } else {
                ListItem::Format(syntax)
            }
        });
    let members: Vec<_> = members.collect();
    braced_body(
        f,
        node.syntax(),
        node.open_brace(),
        members,
        node.close_brace(),
        member_separator,
    )
}
