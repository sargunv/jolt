use std::cmp::Ordering;

use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    KotlinComment, KotlinNode, KotlinRoleElement, KotlinSyntaxField, KotlinSyntaxListPart,
    KotlinSyntaxToken, Name, QualifiedName,
};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, comment_forces_line, format_comment, format_token,
};
use crate::helpers::recovery::{
    KotlinFormatField, KotlinFormatListPart, format_or_verbatim, format_required_field,
    resolve_list_part, resolve_required_field,
};

pub(crate) fn format_name<'source>(
    doc: &mut DocBuilder<'source>,
    name: &Name<'source>,
) -> Doc<'source> {
    format_or_verbatim(name, doc, |doc| {
        format_required_field(name.identifier(), doc, |token, doc| {
            format_token(
                doc,
                &token,
                LeadingTrivia::Preserve,
                TrailingTrivia::Preserve,
            )
        })
    })
}

pub(crate) fn format_qualified_name<'source>(
    doc: &mut DocBuilder<'source>,
    name: &QualifiedName<'source>,
) -> Doc<'source> {
    format_or_verbatim(name, doc, |doc| {
        let multiline = qualified_name_has_line_comments(name);
        let contents = format_qualified_name_parts(doc, name, multiline);
        if multiline {
            doc.indent(contents)
        } else {
            contents
        }
    })
}

fn format_qualified_name_parts<'source>(
    doc: &mut DocBuilder<'source>,
    name: &QualifiedName<'source>,
    multiline: bool,
) -> Doc<'source> {
    match resolve_required_field(name.segments(), doc) {
        KotlinFormatField::Present(segments) => doc.concat_list(|docs| {
            for part in segments.parts() {
                match resolve_list_part(part, docs) {
                    KotlinFormatListPart::Item(element) => match element {
                        KotlinRoleElement::Node(node) => {
                            if let Some(name) = Name::cast(node) {
                                let formatted = format_name(docs, &name);
                                docs.push(formatted);
                            } else {
                                docs.block_on_invariant("invalid qualified-name segment node");
                            }
                        }
                        KotlinRoleElement::Token(token) => {
                            if multiline {
                                let line = docs.hard_line();
                                docs.push(line);
                            }
                            let dot = format_name_dot(docs, &token);
                            docs.push(dot);
                        }
                    },
                    KotlinFormatListPart::Separator(separator) => {
                        docs.block_on_invariant(format!(
                            "unexpected qualified-name separator slot: {:?}",
                            separator.kind()
                        ));
                    }
                    KotlinFormatListPart::Malformed(recovery) => docs.push(recovery),
                }
            }
        }),
        KotlinFormatField::Malformed(recovery) => recovery,
    }
}

fn qualified_name_has_line_comments(name: &QualifiedName<'_>) -> bool {
    let Ok(KotlinSyntaxField::Present(segments)) = name.segments() else {
        return false;
    };
    segments.parts().any(|part| match part {
        Ok(KotlinSyntaxListPart::Item(KotlinRoleElement::Token(token))) => {
            token_has_line_comments(&token)
        }
        Ok(KotlinSyntaxListPart::Item(KotlinRoleElement::Node(node))) => {
            node.first_token()
                .is_some_and(|token| token_has_line_comments(&token))
                || node
                    .last_token()
                    .is_some_and(|token| token_has_line_comments(&token))
        }
        Ok(
            KotlinSyntaxListPart::Separator(_)
            | KotlinSyntaxListPart::Missing(_)
            | KotlinSyntaxListPart::Malformed(_),
        )
        | Err(_) => false,
    })
}

fn token_has_line_comments(token: &KotlinSyntaxToken<'_>) -> bool {
    token
        .leading_comments()
        .chain(token.trailing_comments())
        .any(|comment| comment_forces_line(&comment))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NameSortKey<'source> {
    segments: Vec<&'source str>,
    on_demand: bool,
}

impl<'source> NameSortKey<'source> {
    pub(crate) fn empty() -> Self {
        Self {
            segments: Vec::new(),
            on_demand: false,
        }
    }

    pub(crate) fn new(name: &QualifiedName<'source>, on_demand: bool) -> Self {
        let mut identifiers = Vec::new();
        if let Ok(KotlinSyntaxField::Present(segments)) = name.segments() {
            for part in segments.parts() {
                let Ok(KotlinSyntaxListPart::Item(KotlinRoleElement::Node(node))) = part else {
                    continue;
                };
                let Some(name) = Name::cast(node) else {
                    continue;
                };
                if let Ok(KotlinSyntaxField::Present(identifier)) = name.identifier() {
                    identifiers.push(identifier.text());
                }
            }
        }
        Self {
            segments: identifiers,
            on_demand,
        }
    }

    fn chars(&self) -> impl Iterator<Item = char> + '_ {
        self.segments
            .iter()
            .enumerate()
            .flat_map(|(index, segment)| {
                (index > 0)
                    .then_some(".")
                    .into_iter()
                    .chain(std::iter::once(*segment))
            })
            .chain(self.on_demand.then_some(".*"))
            .flat_map(str::chars)
    }
}

impl Ord for NameSortKey<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.chars().cmp(other.chars())
    }
}

impl PartialOrd for NameSortKey<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn format_name_dot<'source>(
    doc: &mut DocBuilder<'source>,
    dot: &KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    let leading = format_leading_dot_comments(doc, dot.leading_comments());
    let text = doc.source_token(dot);
    let trailing = format_inline_comments(doc, dot.trailing_comments());
    doc.concat([leading, text, trailing])
}

fn format_leading_dot_comments<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = KotlinComment<'source>>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for comment in comments {
            let space = docs.space();
            docs.push(space);
            let comment_doc = format_comment(docs, &comment);
            docs.push(comment_doc);
            if comment_forces_line(&comment) {
                let hard_line = docs.hard_line();
                docs.push(hard_line);
            }
        }
    })
}

fn format_inline_comments<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = KotlinComment<'source>>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for comment in comments {
            let space = docs.space();
            docs.push(space);
            let comment_doc = format_comment(docs, &comment);
            docs.push(comment_doc);
            if comment_forces_line(&comment) {
                let hard_line = docs.hard_line();
                docs.push(hard_line);
            } else {
                let space = docs.space();
                docs.push(space);
            }
        }
    })
}
