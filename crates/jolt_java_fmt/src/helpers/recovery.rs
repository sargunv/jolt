//! Field/list recovery resolution for the Java formatter.
//!
//! Kotlin has a parallel module with the same resolve/format field shapes.
//! Keep them duplicated: Java empty-range malformed boundaries and Kotlin
//! `Invisible` list parts are intentional language-owned recovery policy, not
//! accidental drift that should collapse behind shared generics.

use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{
    JavaMissingSyntax, JavaSyntaxField, JavaSyntaxInvariantError, JavaSyntaxListPart,
    JavaSyntaxToken, JavaSyntaxView,
};

use super::comments::{comment_forces_line, format_comment, format_leading_comment_list};
use super::lexical_safety::JavaLexicalSafety;

/// Formats one syntax-owned malformed boundary and claims its exact source.
pub(crate) fn format_malformed<'source>(
    malformed: &impl JavaSyntaxView<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(core) = malformed.malformed_verbatim_core() else {
        doc.block_on_invariant("malformed Java syntax did not own a verbatim core");
        return Doc::nil();
    };
    let range = core.text_range();
    let has_tokens = core.tokens().next().is_some();
    let (leading, trailing, has_leading_comments, has_trailing_comments) =
        malformed_boundary_comments(&core, doc);
    let (left, right) = if range.start() == range.end() || !has_tokens {
        (None, None)
    } else {
        (
            (!has_leading_comments)
                .then(|| core.previous_token())
                .flatten(),
            (!has_trailing_comments)
                .then(|| core.next_token())
                .flatten(),
        )
    };
    let mut safety = JavaLexicalSafety;
    let fragment = doc.malformed_verbatim_with_safety(&core, &mut safety);
    let fragment = doc.resolve_exceptional(fragment, left.as_ref(), right.as_ref(), &mut safety);
    doc.concat([leading, fragment, trailing])
}

#[derive(Clone, Copy)]
pub(crate) enum JavaFormatField<'source, T> {
    Present(T),
    Malformed(Doc<'source>),
}

pub(crate) enum JavaFormatListPart<'source, T> {
    Item(T),
    Separator(JavaSyntaxToken<'source>),
    Malformed(Doc<'source>),
}

/// A delimiter slot resolved without losing its exact source position.
/// Required missing/malformed slots carry their recovery document; optional
/// absence is represented separately from recovery.
#[derive(Clone, Copy)]
pub(crate) enum JavaFormatDelimiter<'source> {
    Source(JavaSyntaxToken<'source>),
    Recovery(Doc<'source>),
}

impl<'source> JavaFormatDelimiter<'source> {
    pub(crate) fn source(&self) -> Option<&JavaSyntaxToken<'source>> {
        match self {
            Self::Source(token) => Some(token),
            Self::Recovery(_) => None,
        }
    }
}

pub(crate) fn resolve_required_delimiter<'source>(
    field: Result<JavaSyntaxField<'source, JavaSyntaxToken<'source>>, JavaSyntaxInvariantError>,
    doc: &mut DocBuilder<'source>,
) -> JavaFormatDelimiter<'source> {
    match resolve_required_field(field, doc) {
        JavaFormatField::Present(token) => JavaFormatDelimiter::Source(token),
        JavaFormatField::Malformed(recovery) => JavaFormatDelimiter::Recovery(recovery),
    }
}

pub(crate) fn resolve_list_part<'source, T>(
    part: Result<JavaSyntaxListPart<'source, T>, JavaSyntaxInvariantError>,
    doc: &mut DocBuilder<'source>,
) -> JavaFormatListPart<'source, T> {
    match part {
        Ok(JavaSyntaxListPart::Item(item)) => JavaFormatListPart::Item(item),
        Ok(JavaSyntaxListPart::Separator(separator)) => JavaFormatListPart::Separator(separator),
        Ok(JavaSyntaxListPart::Missing(missing)) => {
            JavaFormatListPart::Malformed(format_missing(&missing, doc))
        }
        Ok(JavaSyntaxListPart::Malformed(malformed)) => {
            JavaFormatListPart::Malformed(format_malformed(&malformed, doc))
        }
        Err(error) => {
            doc.block_on_invariant(error.to_string());
            JavaFormatListPart::Malformed(Doc::nil())
        }
    }
}

/// Resolves one generated field without letting missing or malformed syntax
/// leak into a structured layout rule.
pub(crate) fn resolve_required_field<'source, T>(
    field: Result<JavaSyntaxField<'source, T>, JavaSyntaxInvariantError>,
    doc: &mut DocBuilder<'source>,
) -> JavaFormatField<'source, T> {
    match field {
        Ok(JavaSyntaxField::Present(value)) => JavaFormatField::Present(value),
        Ok(JavaSyntaxField::Malformed(malformed)) => {
            JavaFormatField::Malformed(format_malformed(&malformed, doc))
        }
        Ok(JavaSyntaxField::Missing(missing)) => {
            JavaFormatField::Malformed(format_missing(&missing, doc))
        }
        Err(error) => {
            doc.block_on_invariant(error.to_string());
            JavaFormatField::Malformed(Doc::nil())
        }
    }
}

/// Resolves an optional generated field; its empty slot is ordinary absence.
pub(crate) fn resolve_optional_field<'source, T>(
    field: Result<JavaSyntaxField<'source, T>, JavaSyntaxInvariantError>,
    doc: &mut DocBuilder<'source>,
) -> JavaFormatField<'source, Option<T>> {
    match field {
        Ok(JavaSyntaxField::Present(value)) => JavaFormatField::Present(Some(value)),
        Ok(JavaSyntaxField::Missing(_)) => JavaFormatField::Present(None),
        Ok(JavaSyntaxField::Malformed(malformed)) => {
            JavaFormatField::Malformed(format_malformed(&malformed, doc))
        }
        Err(error) => {
            doc.block_on_invariant(error.to_string());
            JavaFormatField::Malformed(Doc::nil())
        }
    }
}

pub(crate) fn format_required_field<'source, T>(
    field: Result<JavaSyntaxField<'source, T>, JavaSyntaxInvariantError>,
    doc: &mut DocBuilder<'source>,
    structured: impl FnOnce(T, &mut DocBuilder<'source>) -> Doc<'source>,
) -> Doc<'source> {
    match resolve_required_field(field, doc) {
        JavaFormatField::Present(value) => structured(value, doc),
        JavaFormatField::Malformed(malformed) => malformed,
    }
}

pub(crate) fn format_optional_field<'source, T>(
    field: Result<JavaSyntaxField<'source, T>, JavaSyntaxInvariantError>,
    doc: &mut DocBuilder<'source>,
    structured: impl FnOnce(T, &mut DocBuilder<'source>) -> Doc<'source>,
) -> Doc<'source> {
    match resolve_optional_field(field, doc) {
        JavaFormatField::Present(Some(value)) => structured(value, doc),
        JavaFormatField::Present(None) => Doc::nil(),
        JavaFormatField::Malformed(malformed) => malformed,
    }
}

pub(crate) fn format_missing<'source>(
    missing: &JavaMissingSyntax<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if missing.verbatim_core().is_err() {
        doc.block_on_invariant("missing Java role did not own an empty verbatim core");
    }
    Doc::nil()
}

fn malformed_boundary_comments<'source>(
    core: &jolt_java_syntax::JavaSyntaxVerbatimCore<'source>,
    doc: &mut DocBuilder<'source>,
) -> (Doc<'source>, Doc<'source>, bool, bool) {
    let leading_comments = core
        .first_token()
        .into_iter()
        .flat_map(|token| token.leading_comments())
        .filter(|comment| !core.contains(comment.text_range()));
    let has_leading_comments = leading_comments.clone().next().is_some();
    let leading = format_leading_comment_list(doc, leading_comments);
    let trailing_comments = core
        .last_token()
        .into_iter()
        .flat_map(|token| token.trailing_comments())
        .filter(|comment| !core.contains(comment.text_range()));
    let has_trailing_comments = trailing_comments.clone().next().is_some();
    let trailing = doc.concat_list(|comments| {
        for comment in trailing_comments {
            let space = comments.space();
            comments.push(space);
            let forces_line = comment_forces_line(&comment);
            let comment = format_comment(comments, &comment);
            comments.push(comment);
            if forces_line {
                let line = comments.hard_line();
                comments.push(line);
            }
        }
    });
    (
        leading,
        trailing,
        has_leading_comments,
        has_trailing_comments,
    )
}
