use jolt_fmt_ir::{Doc, concat, group, text};

use crate::helpers::{lists as java_lists, separated};
use crate::policy::JavaFormatPolicy;

#[derive(Clone)]
pub(crate) struct AnnotationValue {
    doc: Doc,
    kind: AnnotationValueKind,
}

impl AnnotationValue {
    pub(crate) const fn expression(doc: Doc) -> Self {
        Self {
            doc,
            kind: AnnotationValueKind::Expression,
        }
    }

    pub(crate) const fn annotation(doc: Doc) -> Self {
        Self {
            doc,
            kind: AnnotationValueKind::Annotation,
        }
    }

    pub(crate) const fn array(doc: Doc) -> Self {
        Self {
            doc,
            kind: AnnotationValueKind::Array,
        }
    }

    pub(crate) fn into_doc(self) -> Doc {
        self.doc
    }

    fn is_array(&self) -> bool {
        matches!(self.kind, AnnotationValueKind::Array)
    }
}

#[derive(Clone, Copy)]
enum AnnotationValueKind {
    Expression,
    Annotation,
    Array,
}

pub(crate) struct AnnotationPair {
    doc: Doc,
}

impl AnnotationPair {
    pub(crate) fn into_doc(self) -> Doc {
        self.doc
    }
}

pub(crate) fn argument_list(
    values: impl IntoIterator<Item = AnnotationValue>,
    policy: JavaFormatPolicy,
) -> Doc {
    java_lists::argument_list_docs(values.into_iter().map(AnnotationValue::into_doc), policy)
}

pub(crate) fn single_argument(value: AnnotationValue, policy: JavaFormatPolicy) -> Doc {
    if value.is_array() {
        return concat([text("("), value.into_doc(), text(")")]);
    }

    argument_list([value], policy)
}

pub(crate) fn element_value_pair(name: Doc, value: AnnotationValue) -> AnnotationPair {
    if value.is_array() {
        return AnnotationPair {
            doc: group(concat([name, text(" = "), value.into_doc()])),
        };
    }

    AnnotationPair {
        doc: crate::layout::assignment_expression(name, text("="), value.into_doc()),
    }
}

pub(crate) fn mixed_argument_list(items: impl IntoIterator<Item = Doc>) -> Doc {
    separated::delimited_comma_list("(", ")", 2, items)
}
