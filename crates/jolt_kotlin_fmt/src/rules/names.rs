use std::cmp::Ordering;

use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    KotlinComment, KotlinSyntaxField, KotlinSyntaxListPart, KotlinSyntaxToken, KotlinSyntaxView,
    Name, QualifiedName, QualifiedNameSegment,
};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, comment_forces_line, format_comment, format_token,
};
use crate::helpers::recovery::{
    KotlinFormatField, KotlinFormatListPart, format_malformed, format_required_field,
    resolve_list_part, resolve_required_field,
};

pub(crate) fn format_name<'source>(
    doc: &mut DocBuilder<'source>,
    name: &Name<'source>,
) -> Doc<'source> {
    format_name_with_leading(doc, name, LeadingTrivia::Preserve)
}

pub(crate) fn format_name_with_leading<'source>(
    doc: &mut DocBuilder<'source>,
    name: &Name<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_required_field(name.identifier(), doc, |token, doc| {
        format_token(doc, &token, leading, TrailingTrivia::Preserve)
    })
}

pub(crate) fn format_qualified_name<'source>(
    doc: &mut DocBuilder<'source>,
    name: &QualifiedName<'source>,
) -> Doc<'source> {
    let multiline = qualified_name_has_line_comments(name);
    let contents = format_qualified_name_parts(doc, name, multiline);
    if multiline {
        doc.indent(contents)
    } else {
        contents
    }
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
                    KotlinFormatListPart::Item(QualifiedNameSegment::Name(name)) => {
                        let formatted = format_name(docs, &name);
                        docs.push(formatted);
                    }
                    KotlinFormatListPart::Item(
                        QualifiedNameSegment::BogusQualifiedNameSegment(malformed),
                    ) => {
                        let malformed = format_malformed(&malformed, docs);
                        docs.push(malformed);
                    }
                    KotlinFormatListPart::Separator(separator) => {
                        if multiline {
                            let line = docs.hard_line();
                            docs.push(line);
                        }
                        let dot = format_name_dot(docs, &separator);
                        docs.push(dot);
                    }
                    KotlinFormatListPart::Recovery(recovery) => docs.push(recovery.doc()),
                }
            }
        }),
        KotlinFormatField::Malformed(recovery) => recovery,
    }
}

fn qualified_name_has_line_comments(name: &QualifiedName<'_>) -> bool {
    let KotlinSyntaxField::Present(segments) = name.segments() else {
        return false;
    };
    segments.parts().any(|part| match part {
        KotlinSyntaxListPart::Item(QualifiedNameSegment::Name(name)) => {
            name.first_token()
                .is_some_and(|token| token_has_line_comments(&token))
                || name
                    .last_token()
                    .is_some_and(|token| token_has_line_comments(&token))
        }
        KotlinSyntaxListPart::Item(QualifiedNameSegment::BogusQualifiedNameSegment(malformed)) => {
            malformed
                .first_token()
                .is_some_and(|token| token_has_line_comments(&token))
                || malformed
                    .last_token()
                    .is_some_and(|token| token_has_line_comments(&token))
        }
        KotlinSyntaxListPart::Separator(token) => token_has_line_comments(&token),
        KotlinSyntaxListPart::Missing(_) | KotlinSyntaxListPart::Malformed(_) => false,
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
    pub(crate) fn new(name: &QualifiedName<'source>, on_demand: bool) -> Option<Self> {
        if !name.is_recovery_free() {
            return None;
        }
        let mut identifiers = Vec::new();
        let KotlinSyntaxField::Present(segments) = name.segments() else {
            return None;
        };
        for part in segments.parts() {
            match part {
                KotlinSyntaxListPart::Item(QualifiedNameSegment::Name(name)) => {
                    let KotlinSyntaxField::Present(identifier) = name.identifier() else {
                        return None;
                    };
                    identifiers.push(identifier.text());
                }
                KotlinSyntaxListPart::Separator(_) => {}
                KotlinSyntaxListPart::Item(QualifiedNameSegment::BogusQualifiedNameSegment(_))
                | KotlinSyntaxListPart::Missing(_)
                | KotlinSyntaxListPart::Malformed(_) => return None,
            }
        }
        Some(Self {
            segments: identifiers,
            on_demand,
        })
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
