use jolt_fmt_ir::{
    Doc, concat, fill, fill_entry, force_group, group, indent_by, line, soft_line, text,
};

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
    has_array_value: bool,
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

pub(crate) fn pair_list(
    pairs: impl IntoIterator<Item = AnnotationPair>,
    policy: JavaFormatPolicy,
) -> Doc {
    let pairs = pairs.into_iter().collect::<Vec<_>>();
    let has_array_value = pairs.iter().any(|pair| pair.has_array_value);
    let list = java_lists::formal_parameter_list_docs(
        pairs.into_iter().map(AnnotationPair::into_doc),
        policy,
    );
    if has_array_value {
        force_group(list)
    } else {
        list
    }
}

pub(crate) fn element_value_pair(name: Doc, value: AnnotationValue) -> AnnotationPair {
    let has_array_value = value.is_array();
    if value.is_array() {
        return AnnotationPair {
            doc: group(concat([name, text(" = "), value.into_doc()])),
            has_array_value,
        };
    }

    AnnotationPair {
        doc: crate::layout::assignment_expression(name, text("="), value.into_doc()),
        has_array_value,
    }
}

pub(crate) fn array_initializer(values: impl IntoIterator<Item = AnnotationValue>) -> Doc {
    let mut values = values
        .into_iter()
        .map(AnnotationValue::into_doc)
        .collect::<Vec<_>>();
    if values.is_empty() {
        return text("{}");
    }

    let last = values.pop().expect("non-empty values checked above");
    let entries = values
        .into_iter()
        .map(|value| fill_entry(value, concat([text(","), line()])));

    group(concat([
        text("{"),
        indent_by(1, concat([soft_line(), fill(entries, last)])),
        soft_line(),
        text("}"),
    ]))
}

pub(crate) fn mixed_argument_list(items: impl IntoIterator<Item = Doc>) -> Doc {
    separated::delimited_comma_list("(", ")", 2, items)
}
