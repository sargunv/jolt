//! Field/list recovery resolution for the Kotlin formatter.
//!
//! Java has a parallel module with the same resolve/format field shapes.
//! Keep them duplicated: Kotlin `Invisible` list parts and Java empty-range
//! malformed boundaries are intentional language-owned recovery policy, not
//! accidental drift that should collapse behind shared generics.

use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    KotlinMissingSyntax, KotlinSyntaxField, KotlinSyntaxInvariantError, KotlinSyntaxListPart,
    KotlinSyntaxToken, KotlinSyntaxView,
};

use super::comments::{comment_forces_line, format_comment, format_leading_comment_list};
use super::lexical_safety::KotlinLexicalSafety;

/// Formats one syntax-owned malformed boundary and claims its exact source.
pub(crate) fn format_malformed<'source>(
    malformed: &impl KotlinSyntaxView<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(core) = malformed.malformed_verbatim_core() else {
        doc.block_on_invariant("malformed Kotlin syntax did not own a verbatim core");
        return Doc::nil();
    };
    let has_tokens = core.tokens().next().is_some();
    let (leading, trailing, has_leading_comments, has_trailing_comments) =
        malformed_boundary_comments(&core, doc);
    let (left, right) = if has_tokens {
        (
            (!has_leading_comments)
                .then(|| core.previous_token())
                .flatten(),
            (!has_trailing_comments)
                .then(|| core.next_token())
                .flatten(),
        )
    } else {
        (None, None)
    };
    let mut safety = KotlinLexicalSafety;
    let fragment = doc.malformed_verbatim_with_safety(&core, &mut safety);
    let fragment = doc.resolve_exceptional(fragment, left.as_ref(), right.as_ref(), &mut safety);
    doc.concat([leading, fragment, trailing])
}

#[derive(Clone, Copy)]
pub(crate) enum KotlinFormatField<'source, T> {
    Present(T),
    Malformed(Doc<'source>),
}

pub(crate) enum KotlinFormatListPart<'source, T> {
    Item(T),
    Separator(KotlinSyntaxToken<'source>),
    Malformed(Doc<'source>),
    Invisible(Doc<'source>),
}

/// A delimiter slot resolved without losing its exact source position.
#[derive(Clone, Copy)]
pub(crate) enum KotlinFormatDelimiter<'source> {
    Source(KotlinSyntaxToken<'source>),
    Recovery(Doc<'source>),
}

impl<'source> KotlinFormatDelimiter<'source> {
    pub(crate) fn source(&self) -> Option<&KotlinSyntaxToken<'source>> {
        match self {
            Self::Source(token) => Some(token),
            Self::Recovery(_) => None,
        }
    }

    const fn recovery(&self) -> Doc<'source> {
        match self {
            Self::Source(_) => Doc::nil(),
            Self::Recovery(recovery) => *recovery,
        }
    }
}

pub(crate) fn join_delimited_recovery<'source>(
    doc: &mut DocBuilder<'source>,
    open: &KotlinFormatDelimiter<'source>,
    contents: Doc<'source>,
    close: &KotlinFormatDelimiter<'source>,
) -> Doc<'source> {
    doc.concat([open.recovery(), contents, close.recovery()])
}

pub(crate) fn resolve_required_delimiter<'source>(
    field: Result<
        KotlinSyntaxField<'source, KotlinSyntaxToken<'source>>,
        KotlinSyntaxInvariantError,
    >,
    doc: &mut DocBuilder<'source>,
) -> KotlinFormatDelimiter<'source> {
    match resolve_required_field(field, doc) {
        KotlinFormatField::Present(token) => KotlinFormatDelimiter::Source(token),
        KotlinFormatField::Malformed(recovery) => KotlinFormatDelimiter::Recovery(recovery),
    }
}

pub(crate) fn resolve_list_part<'source, T>(
    part: Result<KotlinSyntaxListPart<'source, T>, KotlinSyntaxInvariantError>,
    doc: &mut DocBuilder<'source>,
) -> KotlinFormatListPart<'source, T> {
    match part {
        Ok(KotlinSyntaxListPart::Item(item)) => KotlinFormatListPart::Item(item),
        Ok(KotlinSyntaxListPart::Separator(separator)) => {
            KotlinFormatListPart::Separator(separator)
        }
        Ok(KotlinSyntaxListPart::Missing(missing)) => {
            KotlinFormatListPart::Invisible(format_missing(&missing, doc))
        }
        Ok(KotlinSyntaxListPart::Malformed(malformed)) => {
            let recovery = format_malformed(&malformed, doc);
            if malformed.first_token().is_some() {
                KotlinFormatListPart::Malformed(recovery)
            } else {
                KotlinFormatListPart::Invisible(recovery)
            }
        }
        Err(error) => {
            doc.block_on_invariant(error.to_string());
            KotlinFormatListPart::Invisible(Doc::nil())
        }
    }
}

pub(crate) fn resolve_required_field<'source, T>(
    field: Result<KotlinSyntaxField<'source, T>, KotlinSyntaxInvariantError>,
    doc: &mut DocBuilder<'source>,
) -> KotlinFormatField<'source, T> {
    match field {
        Ok(KotlinSyntaxField::Present(value)) => KotlinFormatField::Present(value),
        Ok(KotlinSyntaxField::Malformed(malformed)) => {
            KotlinFormatField::Malformed(format_malformed(&malformed, doc))
        }
        Ok(KotlinSyntaxField::Missing(missing)) => {
            KotlinFormatField::Malformed(format_missing(&missing, doc))
        }
        Err(error) => {
            doc.block_on_invariant(error.to_string());
            KotlinFormatField::Malformed(Doc::nil())
        }
    }
}

pub(crate) fn resolve_optional_field<'source, T>(
    field: Result<KotlinSyntaxField<'source, T>, KotlinSyntaxInvariantError>,
    doc: &mut DocBuilder<'source>,
) -> KotlinFormatField<'source, Option<T>> {
    match field {
        Ok(KotlinSyntaxField::Present(value)) => KotlinFormatField::Present(Some(value)),
        Ok(KotlinSyntaxField::Missing(_)) => KotlinFormatField::Present(None),
        Ok(KotlinSyntaxField::Malformed(malformed)) => {
            KotlinFormatField::Malformed(format_malformed(&malformed, doc))
        }
        Err(error) => {
            doc.block_on_invariant(error.to_string());
            KotlinFormatField::Malformed(Doc::nil())
        }
    }
}

pub(crate) fn format_required_field<'source, T>(
    field: Result<KotlinSyntaxField<'source, T>, KotlinSyntaxInvariantError>,
    doc: &mut DocBuilder<'source>,
    structured: impl FnOnce(T, &mut DocBuilder<'source>) -> Doc<'source>,
) -> Doc<'source> {
    match resolve_required_field(field, doc) {
        KotlinFormatField::Present(value) => structured(value, doc),
        KotlinFormatField::Malformed(malformed) => malformed,
    }
}

pub(crate) fn format_optional_field<'source, T>(
    field: Result<KotlinSyntaxField<'source, T>, KotlinSyntaxInvariantError>,
    doc: &mut DocBuilder<'source>,
    structured: impl FnOnce(T, &mut DocBuilder<'source>) -> Doc<'source>,
) -> Doc<'source> {
    match resolve_optional_field(field, doc) {
        KotlinFormatField::Present(Some(value)) => structured(value, doc),
        KotlinFormatField::Present(None) => Doc::nil(),
        KotlinFormatField::Malformed(malformed) => malformed,
    }
}

pub(crate) fn format_missing<'source>(
    missing: &KotlinMissingSyntax<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    if missing.verbatim_core().is_err() {
        doc.block_on_invariant("missing Kotlin role did not own an empty verbatim core");
    }
    Doc::nil()
}

fn malformed_boundary_comments<'source>(
    core: &jolt_kotlin_syntax::KotlinSyntaxVerbatimCore<'source>,
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
