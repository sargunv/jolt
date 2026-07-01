use jolt_fmt_ir::{
    Doc, FlatLine, LevelBreakMode, break_level_with_indent, concat, group, hard_line, indent_by,
    join, level_break, line, soft_line, text,
};

use crate::helpers::{expressions as java_expressions, lists as java_lists, separated};
use crate::policy::JavaFormatPolicy;
use jolt_diagnostics::TextRange;

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
    is_array_initializer: bool,
}

impl AnnotationPair {
    pub(crate) fn into_doc(self) -> Doc {
        self.doc
    }

    fn is_array_initializer(&self) -> bool {
        self.is_array_initializer
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
    let is_array_initializer = value.is_array();
    if is_array_initializer {
        return AnnotationPair {
            doc: group(concat([name, text(" = "), value.into_doc()])),
            is_array_initializer,
        };
    }

    AnnotationPair {
        doc: java_expressions::simple_assignment_expression(name, text("="), value.into_doc(), 2),
        is_array_initializer,
    }
}

pub(crate) fn pair_argument_list(
    pairs: impl IntoIterator<Item = AnnotationPair>,
    policy: JavaFormatPolicy,
) -> Doc {
    let pairs = pairs.into_iter().collect::<Vec<_>>();
    debug_assert!(pairs.iter().any(AnnotationPair::is_array_initializer));
    let docs = pairs
        .into_iter()
        .map(AnnotationPair::into_doc)
        .collect::<Vec<_>>();
    delimited_annotation_argument_list_one_per_line(docs, policy.continuation_indent_levels())
}

fn delimited_annotation_argument_list_one_per_line(items: Vec<Doc>, indent_levels: u16) -> Doc {
    if items.is_empty() {
        return text("()");
    }

    group(concat([
        text("("),
        indent_by(
            indent_levels,
            concat([soft_line(), join(concat([text(","), hard_line()]), items)]),
        ),
        text(")"),
    ]))
}

pub(crate) fn mixed_argument_list(items: impl IntoIterator<Item = Doc>) -> Doc {
    separated::delimited_comma_list("(", ")", 2, items)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AnnotationLayout {
    Horizontal,
    Vertical,
}

#[derive(Clone)]
pub(crate) struct AnnotationDoc {
    doc: Doc,
    range: TextRange,
    has_arguments: bool,
    is_type_use: bool,
}

impl AnnotationDoc {
    pub(crate) const fn new(
        doc: Doc,
        range: TextRange,
        has_arguments: bool,
        is_type_use: bool,
    ) -> Self {
        Self {
            doc,
            range,
            has_arguments,
            is_type_use,
        }
    }

    pub(crate) fn into_doc(self) -> Doc {
        self.doc
    }

    pub(crate) const fn has_arguments(&self) -> bool {
        self.has_arguments
    }

    pub(crate) const fn is_type_use(&self) -> bool {
        self.is_type_use
    }

    pub(crate) const fn range(&self) -> TextRange {
        self.range
    }
}

#[derive(Clone, Default)]
pub(crate) struct DeclarationAnnotationSplit {
    pub(crate) declaration_annotations: Vec<AnnotationDoc>,
    pub(crate) type_use_annotations: Vec<AnnotationDoc>,
}

pub(crate) fn split_declaration_and_type_use_annotations(
    annotations: Vec<AnnotationDoc>,
) -> DeclarationAnnotationSplit {
    let mut declaration_annotations = annotations;
    let split_at = declaration_annotations
        .iter()
        .rposition(|annotation| !annotation.is_type_use())
        .map_or(0, |index| index + 1);
    let type_use_annotations = declaration_annotations.split_off(split_at);

    DeclarationAnnotationSplit {
        declaration_annotations,
        type_use_annotations,
    }
}

pub(crate) fn split_type_bearing_declaration_annotations(
    annotations: Vec<AnnotationDoc>,
    modifier_ranges: impl IntoIterator<Item = TextRange>,
) -> DeclarationAnnotationSplit {
    let modifier_ranges = modifier_ranges.into_iter().collect::<Vec<_>>();
    if modifier_ranges.is_empty() {
        return split_declaration_and_type_use_annotations(annotations);
    }

    let first_modifier_start = modifier_ranges
        .iter()
        .map(|range| range.start())
        .min()
        .expect("non-empty modifier range list should have a first range");
    let mut split = DeclarationAnnotationSplit::default();
    for annotation in annotations {
        if annotation.range().start() > first_modifier_start {
            split.type_use_annotations.push(annotation);
        } else {
            split.declaration_annotations.push(annotation);
        }
    }
    split
}

pub(crate) fn is_known_type_use_annotation_name(name: &str) -> bool {
    matches!(name, "NonNull" | "Nullable")
}

pub(crate) fn declaration_annotation_layout(annotations: &[AnnotationDoc]) -> AnnotationLayout {
    if annotations
        .iter()
        .all(|annotation| !annotation.has_arguments())
    {
        AnnotationLayout::Horizontal
    } else {
        AnnotationLayout::Vertical
    }
}

pub(crate) fn local_annotation_layout(annotations: &[AnnotationDoc]) -> AnnotationLayout {
    let parameterless = annotations
        .iter()
        .filter(|annotation| !annotation.has_arguments())
        .count();
    if parameterless <= 1 && parameterless == annotations.len() {
        AnnotationLayout::Horizontal
    } else {
        AnnotationLayout::Vertical
    }
}

pub(crate) fn annotation_cluster(
    annotations: impl IntoIterator<Item = AnnotationDoc>,
    layout: AnnotationLayout,
) -> Option<Doc> {
    let annotations = annotations
        .into_iter()
        .map(AnnotationDoc::into_doc)
        .collect::<Vec<_>>();
    if annotations.is_empty() {
        return None;
    }

    Some(match layout {
        AnnotationLayout::Horizontal => join(text(" "), annotations),
        AnnotationLayout::Vertical => join(hard_line(), annotations),
    })
}

pub(crate) fn with_declaration_annotations(
    annotations: Vec<AnnotationDoc>,
    declaration: Doc,
    layout: AnnotationLayout,
) -> Doc {
    if annotations.is_empty() {
        return declaration;
    }

    match layout {
        AnnotationLayout::Horizontal => {
            let mut parts = annotations
                .into_iter()
                .map(AnnotationDoc::into_doc)
                .collect::<Vec<_>>();
            parts.push(declaration);
            group(join(line(), parts))
        }
        AnnotationLayout::Vertical => {
            let annotations = annotation_cluster(annotations, layout)
                .expect("non-empty annotations checked above");
            concat([annotations, hard_line(), declaration])
        }
    }
}

pub(crate) fn with_resource_declaration_annotations(
    annotations: Vec<AnnotationDoc>,
    declaration: Doc,
    layout: AnnotationLayout,
    indent_levels: u16,
) -> Doc {
    if annotations.is_empty() {
        return declaration;
    }

    match layout {
        AnnotationLayout::Horizontal => {
            with_declaration_annotations(annotations, declaration, layout)
        }
        AnnotationLayout::Vertical => {
            let annotations = annotation_cluster(annotations, layout)
                .expect("non-empty annotations checked above");
            concat([
                annotations,
                indent_by(indent_levels, concat([hard_line(), declaration])),
            ])
        }
    }
}

pub(crate) fn type_use_prefix(
    annotations: impl IntoIterator<Item = AnnotationDoc>,
    ty: Doc,
) -> Doc {
    let mut parts = annotations
        .into_iter()
        .map(AnnotationDoc::into_doc)
        .collect::<Vec<_>>();
    parts.push(ty);
    join(text(" "), parts)
}

pub(crate) fn annotated_parameter(
    declaration_annotations: Vec<AnnotationDoc>,
    modifiers: Vec<Doc>,
    ty: Doc,
    name: Doc,
    policy: JavaFormatPolicy,
) -> Doc {
    let annotation_docs = declaration_annotations
        .into_iter()
        .map(AnnotationDoc::into_doc)
        .collect::<Vec<_>>();

    if annotation_docs.is_empty() {
        if policy.annotated_parameter_groups_type_and_name() {
            let mut head_parts = modifiers;
            head_parts.push(ty);
            let head = join(text(" "), head_parts);
            return group(concat([head, indent_by(2, concat([line(), name]))]));
        }
        let mut inline_parts = modifiers;
        inline_parts.push(ty);
        inline_parts.push(name);
        return join(text(" "), inline_parts);
    }

    let mut head_parts = annotation_docs;
    head_parts.extend(modifiers);
    let head = join(text(" "), head_parts);
    let type_and_name = if policy.annotated_parameter_groups_type_and_name() {
        group(concat([ty, indent_by(2, concat([line(), name]))]))
    } else {
        concat([ty, text(" "), name])
    };
    break_level_with_indent(
        2,
        [head, type_and_name],
        [level_break(LevelBreakMode::Unified, FlatLine::Space, 0)],
    )
    .expect("valid annotated parameter break level")
}

pub(crate) fn annotated_parameter_with_name_continuation(
    declaration_annotations: Vec<AnnotationDoc>,
    modifiers: Vec<Doc>,
    ty: Doc,
    name: Doc,
) -> Doc {
    let mut head_parts = declaration_annotations
        .into_iter()
        .map(AnnotationDoc::into_doc)
        .collect::<Vec<_>>();
    head_parts.extend(modifiers);
    let type_and_name = concat([ty, indent_by(2, concat([hard_line(), name]))]);
    if head_parts.is_empty() {
        return type_and_name;
    }

    concat([
        join(text(" "), head_parts),
        indent_by(2, concat([hard_line(), type_and_name])),
    ])
}
