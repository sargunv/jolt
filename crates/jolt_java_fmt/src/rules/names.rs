use std::cmp::Ordering;

use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{
    JavaComment, JavaSyntaxField, JavaSyntaxListPart, JavaSyntaxToken, NameSyntax,
};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, comment_forces_line, format_comment, format_token,
    trailing_comments_force_line,
};
use crate::helpers::recovery::{
    JavaFormatListPart, format_malformed, format_missing, resolve_list_part,
};

fn name_identifier_texts<'source>(name: &NameSyntax<'source>) -> Option<Vec<&'source str>> {
    fn identifier<'source>(
        field: JavaSyntaxField<'source, JavaSyntaxToken<'source>>,
    ) -> Option<&'source str> {
        match field {
            JavaSyntaxField::Present(token) => Some(token.text()),
            JavaSyntaxField::Missing(_) | JavaSyntaxField::Malformed(_) => None,
        }
    }

    match name {
        NameSyntax::Name(name) => Some(vec![identifier(name.identifier())?]),
        NameSyntax::QualifiedName(name) => {
            let first = match name.first_segment() {
                JavaSyntaxField::Present(segment) => identifier(segment.identifier())?,
                JavaSyntaxField::Missing(_) | JavaSyntaxField::Malformed(_) => return None,
            };
            let mut identifiers = vec![first];
            let segments = match name.remaining_segments() {
                JavaSyntaxField::Present(segments) => segments,
                JavaSyntaxField::Missing(_) | JavaSyntaxField::Malformed(_) => return None,
            };
            for part in segments.parts() {
                match part {
                    JavaSyntaxListPart::Item(segment) => {
                        identifiers.push(identifier(segment.identifier())?);
                    }
                    JavaSyntaxListPart::Separator(_) => {}
                    JavaSyntaxListPart::Missing(_) | JavaSyntaxListPart::Malformed(_) => {
                        return None;
                    }
                }
            }
            Some(identifiers)
        }
        NameSyntax::BogusName(_) => None,
    }
}

pub(crate) fn format_name<'source>(
    name: &NameSyntax<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_name_with_leading_trivia(name, LeadingTrivia::Preserve, doc)
}

/// Formats a name whose first token's leading comments an enclosing construct
/// already emitted.
pub(crate) fn format_name_without_leading_comments<'source>(
    name: &NameSyntax<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_name_with_leading_trivia(name, LeadingTrivia::SuppressAlreadyHandled, doc)
}

fn format_name_with_leading_trivia<'source>(
    name: &NameSyntax<'source>,
    leading: LeadingTrivia,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let multiline = name_has_line_comments(name);
    let contents = doc.concat_list(|docs| format_name_parts(name, multiline, leading, docs));
    if multiline {
        doc_indent!(doc, contents)
    } else {
        contents
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NameSortKey<'source> {
    segments: Vec<&'source str>,
    on_demand: bool,
}

impl<'source> NameSortKey<'source> {
    pub(crate) fn new(name: &NameSyntax<'source>, on_demand: bool) -> Option<Self> {
        Some(Self {
            segments: name_identifier_texts(name)?,
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

/// Only a line comment that lands *between* two segments can split a name.
///
/// A comment leading the first segment sits before the whole name, and one
/// trailing the last segment sits after it; neither has a segment on the far
/// side to push onto another line, so the enclosing construct places them.
fn name_has_line_comments(name: &NameSyntax<'_>) -> bool {
    let leading_forces_line = |field: JavaSyntaxField<'_, JavaSyntaxToken<'_>>| {
        matches!(field, JavaSyntaxField::Present(token) if token
            .leading_comments()
            .any(|comment| comment_forces_line(&comment)))
    };
    let trailing_forces_line = |field: JavaSyntaxField<'_, JavaSyntaxToken<'_>>| {
        matches!(field, JavaSyntaxField::Present(token) if token
            .trailing_comments()
            .any(|comment| comment_forces_line(&comment)))
    };
    // A lone identifier has no interior boundary to break at.
    let NameSyntax::QualifiedName(name) = name else {
        return false;
    };
    if matches!(name.first_segment(), JavaSyntaxField::Present(segment) if trailing_forces_line(segment.identifier()))
    {
        return true;
    }
    if matches!(name.first_dot(), JavaSyntaxField::Present(dot) if token_has_line_comments(&dot)) {
        return true;
    }
    let JavaSyntaxField::Present(segments) = name.remaining_segments() else {
        return false;
    };
    let mut parts = segments.parts().peekable();
    while let Some(part) = parts.next() {
        let followed_by_part = parts.peek().is_some();
        let splits = match part {
            JavaSyntaxListPart::Item(segment) => {
                leading_forces_line(segment.identifier())
                    || (followed_by_part && trailing_forces_line(segment.identifier()))
            }
            JavaSyntaxListPart::Separator(token) => token_has_line_comments(&token),
            JavaSyntaxListPart::Missing(_) | JavaSyntaxListPart::Malformed(_) => false,
        };
        if splits {
            return true;
        }
    }
    false
}

fn token_has_line_comments(token: &JavaSyntaxToken<'_>) -> bool {
    token
        .leading_comments()
        .chain(token.trailing_comments())
        .any(|comment| comment_forces_line(&comment))
}

fn format_name_parts<'source>(
    name: &NameSyntax<'source>,
    multiline: bool,
    leading: LeadingTrivia,
    docs: &mut jolt_fmt_ir::ConcatBuilder<'_, 'source>,
) {
    match name {
        NameSyntax::Name(name) => {
            push_identifier_doc(name.identifier(), false, multiline, leading, docs);
        }
        NameSyntax::QualifiedName(name) => {
            let first_has_dot = matches!(name.first_dot(), JavaSyntaxField::Present(_));
            match name.first_segment() {
                JavaSyntaxField::Present(segment) => {
                    push_identifier_doc(
                        segment.identifier(),
                        first_has_dot,
                        multiline,
                        leading,
                        docs,
                    );
                }
                JavaSyntaxField::Missing(missing) => {
                    let recovery = format_missing(&missing, docs);
                    docs.push(recovery);
                }
                JavaSyntaxField::Malformed(malformed) => {
                    let recovery = format_malformed(&malformed, docs);
                    docs.push(recovery);
                }
            }
            push_dot_doc(name.first_dot(), multiline, docs);
            match name.remaining_segments() {
                JavaSyntaxField::Present(segments) => {
                    let mut parts = segments.parts().peekable();
                    while let Some(part) = parts.next() {
                        match resolve_list_part(part, docs) {
                            JavaFormatListPart::Item(segment) => {
                                let followed_by_dot =
                                    matches!(parts.peek(), Some(JavaSyntaxListPart::Separator(_)));
                                push_identifier_doc(
                                    segment.identifier(),
                                    followed_by_dot,
                                    multiline,
                                    LeadingTrivia::Preserve,
                                    docs,
                                );
                            }
                            JavaFormatListPart::Separator(dot) => {
                                push_dot_token_doc(&dot, multiline, docs);
                            }
                            JavaFormatListPart::Recovery(recovery) => docs.push(recovery.doc()),
                        }
                    }
                }
                JavaSyntaxField::Missing(missing) => {
                    let recovery = format_missing(&missing, docs);
                    docs.push(recovery);
                }
                JavaSyntaxField::Malformed(malformed) => {
                    let recovery = format_malformed(&malformed, docs);
                    docs.push(recovery);
                }
            }
        }
        NameSyntax::BogusName(name) => {
            let recovery = format_malformed(name, docs);
            docs.push(recovery);
        }
    }
}

fn push_identifier_doc<'source>(
    field: JavaSyntaxField<'source, JavaSyntaxToken<'source>>,
    followed_by_dot: bool,
    multiline: bool,
    leading: LeadingTrivia,
    docs: &mut jolt_fmt_ir::ConcatBuilder<'_, 'source>,
) {
    let formatted = match field {
        JavaSyntaxField::Present(identifier) if multiline => {
            format_name_segment_identifier(docs, &identifier, followed_by_dot, leading)
        }
        JavaSyntaxField::Present(identifier) => {
            format_inline_name_segment_identifier(docs, &identifier, followed_by_dot, leading)
        }
        JavaSyntaxField::Missing(missing) => format_missing(&missing, docs),
        JavaSyntaxField::Malformed(malformed) => format_malformed(&malformed, docs),
    };
    docs.push(formatted);
}

fn push_dot_doc<'source>(
    field: JavaSyntaxField<'source, JavaSyntaxToken<'source>>,
    multiline: bool,
    docs: &mut jolt_fmt_ir::ConcatBuilder<'_, 'source>,
) {
    match field {
        JavaSyntaxField::Present(dot) => push_dot_token_doc(&dot, multiline, docs),
        JavaSyntaxField::Missing(missing) => {
            let recovery = format_missing(&missing, docs);
            docs.push(recovery);
        }
        JavaSyntaxField::Malformed(malformed) => {
            let recovery = format_malformed(&malformed, docs);
            docs.push(recovery);
        }
    }
}

fn push_dot_token_doc<'source>(
    dot: &JavaSyntaxToken<'source>,
    multiline: bool,
    docs: &mut jolt_fmt_ir::ConcatBuilder<'_, 'source>,
) {
    if multiline {
        let line = docs.hard_line();
        docs.push(line);
    }
    let dot = format_name_dot(docs, dot);
    docs.push(dot);
}

fn format_name_dot<'source>(
    doc: &mut DocBuilder<'source>,
    dot: &JavaSyntaxToken<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            format_leading_dot_comments(doc, dot.leading_comments()),
            format_token(
                doc,
                dot,
                LeadingTrivia::SuppressAlreadyHandled,
                TrailingTrivia::RelocatedToEnclosingContext,
            ),
            format_inline_comments(doc, dot.trailing_comments()),
        ]
    )
}

fn format_name_segment_identifier<'source>(
    doc: &mut DocBuilder<'source>,
    identifier: &JavaSyntaxToken<'source>,
    followed_by_dot: bool,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let token = format_token(doc, identifier, leading, TrailingTrivia::BeforeLineBreak);
    // A following dot brings its own break. The last segment has nothing after
    // it inside the name, so a line comment there would swallow whatever the
    // enclosing construct renders next -- typically a `;`. The boundary
    // collapses into the enclosing construct's own break when it has one.
    if !followed_by_dot && trailing_comments_force_line(identifier) {
        let line = doc.hard_line_boundary();
        doc.concat([token, line])
    } else {
        token
    }
}

fn format_inline_name_segment_identifier<'source>(
    doc: &mut DocBuilder<'source>,
    identifier: &JavaSyntaxToken<'source>,
    followed_by_dot: bool,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let leading_comments = match leading {
        LeadingTrivia::Preserve => format_inline_comments(doc, identifier.leading_comments()),
        LeadingTrivia::SuppressAlreadyHandled => Doc::nil(),
    };
    doc_concat!(
        doc,
        [
            leading_comments,
            format_token(
                doc,
                identifier,
                LeadingTrivia::SuppressAlreadyHandled,
                TrailingTrivia::RelocatedToEnclosingContext,
            ),
            if followed_by_dot {
                format_leading_dot_comments(doc, identifier.trailing_comments())
            } else {
                format_inline_comments(doc, identifier.trailing_comments())
            },
        ]
    )
}

fn format_leading_dot_comments<'source>(
    doc: &mut DocBuilder<'source>,
    comments: impl IntoIterator<Item = JavaComment<'source>>,
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
    comments: impl IntoIterator<Item = JavaComment<'source>>,
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
