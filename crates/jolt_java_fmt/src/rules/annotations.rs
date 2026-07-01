use jolt_fmt_ir::{
    Doc, concat, force_group, group, hard_line, if_break, indent, line, soft_line, text,
};
use jolt_java_syntax::{
    Annotation, AnnotationArgument, AnnotationArgumentList, AnnotationArrayInitializer,
    AnnotationArrayInitializerEntry, AnnotationElementValue, AnnotationElementValuePair,
    JavaComment, JavaSyntaxToken,
};

use crate::helpers::comments::{
    comment_forces_line, format_comment, format_dangling_comments, format_leading_comments,
    format_trailing_comments, format_trailing_comments_before_line_break,
};
use crate::helpers::lists::parenthesized_list;
use crate::rules::expressions::format_expression;
use crate::rules::names::format_name;

pub(crate) fn format_annotation(annotation: &Annotation) -> Doc {
    concat([
        text("@"),
        annotation
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_name(&name)),
        annotation
            .arguments()
            .map_or_else(jolt_fmt_ir::nil, |arguments| {
                format_annotation_argument_list(&arguments)
            }),
    ])
}

pub(crate) fn format_annotation_element_value(value: &AnnotationElementValue) -> Doc {
    if let Some(expression) = value.expression() {
        return format_expression(&expression);
    }
    if let Some(annotation) = value.annotation() {
        return format_annotation(&annotation);
    }
    value
        .array_initializer()
        .map_or_else(jolt_fmt_ir::nil, |array| {
            format_annotation_array_initializer(&array)
        })
}

fn format_annotation_argument_list(arguments: &AnnotationArgumentList) -> Doc {
    parenthesized_list(
        arguments
            .arguments()
            .map(format_annotation_argument)
            .collect(),
    )
}

fn format_annotation_argument(argument: AnnotationArgument) -> Doc {
    match argument {
        AnnotationArgument::Value(value) => format_annotation_element_value(&value),
        AnnotationArgument::Pair(pair) => format_annotation_element_value_pair(&pair),
    }
}

fn format_annotation_element_value_pair(pair: &AnnotationElementValuePair) -> Doc {
    concat([
        pair.name()
            .map_or_else(jolt_fmt_ir::nil, |name| text(name.text().to_owned())),
        text(" = "),
        pair.value().map_or_else(jolt_fmt_ir::nil, |value| {
            format_annotation_element_value(&value)
        }),
    ])
}

fn format_annotation_array_initializer(initializer: &AnnotationArrayInitializer) -> Doc {
    let entries = initializer.entries().collect::<Vec<_>>();
    if entries.is_empty() {
        return format_empty_annotation_array_initializer(initializer);
    }

    let has_dangling_comments = annotation_array_initializer_has_dangling_comments(initializer);
    let doc = group(concat([
        format_annotation_array_initializer_open(initializer),
        indent(concat([
            format_open_annotation_array_initializer_spacing(initializer),
            format_annotation_array_initializer_entries(entries),
        ])),
        format_annotation_array_initializer_close_with_spacing(initializer),
    ]));

    if has_dangling_comments {
        force_group(doc)
    } else {
        doc
    }
}

fn format_empty_annotation_array_initializer(initializer: &AnnotationArrayInitializer) -> Doc {
    if !annotation_array_initializer_has_dangling_comments(initializer) {
        return concat([
            format_annotation_array_initializer_open(initializer),
            format_annotation_array_initializer_close_delimiter(initializer),
        ]);
    }

    force_group(concat([
        format_annotation_array_initializer_open(initializer),
        indent(concat([
            hard_line(),
            format_annotation_array_initializer_dangling_comments(initializer),
        ])),
        hard_line(),
        format_annotation_array_initializer_close_delimiter_without_leading(initializer),
    ]))
}

fn annotation_array_initializer_has_dangling_comments(
    initializer: &AnnotationArrayInitializer,
) -> bool {
    initializer
        .open_brace()
        .is_some_and(|token| !token.trailing_comments().is_empty())
        || initializer
            .close_brace()
            .is_some_and(|token| !token.leading_comments().is_empty())
}

fn format_annotation_array_initializer_open(initializer: &AnnotationArrayInitializer) -> Doc {
    initializer.open_brace().map_or_else(
        || text("{"),
        |open| concat([format_leading_comments(&open), text("{")]),
    )
}

fn format_open_annotation_array_initializer_spacing(
    initializer: &AnnotationArrayInitializer,
) -> Doc {
    let Some(open) = initializer.open_brace() else {
        return soft_line();
    };

    let comments = open.trailing_comments();
    if comments.is_empty() {
        return soft_line();
    }

    concat([hard_line(), format_dangling_comments(comments), hard_line()])
}

fn format_annotation_array_initializer_entries(
    entries: Vec<AnnotationArrayInitializerEntry>,
) -> Doc {
    let mut docs = Vec::new();
    let entries_len = entries.len();

    for (index, entry) in entries.into_iter().enumerate() {
        docs.push(format_annotation_element_value(&entry.value));
        if let Some(comma) = entry.comma {
            docs.push(format_annotation_array_initializer_separator(
                &comma,
                index + 1 == entries_len,
            ));
        } else if index + 1 < entries_len {
            docs.push(line());
        } else {
            docs.push(if_break(text(","), jolt_fmt_ir::nil()));
        }
    }

    concat(docs)
}

fn format_annotation_array_initializer_separator(comma: &JavaSyntaxToken, is_last: bool) -> Doc {
    let trailing_comments = comma.trailing_comments();
    let has_trailing_comments = !trailing_comments.is_empty();
    let force_line = trailing_comments.iter().any(comment_forces_line);

    concat([
        format_leading_comments(comma),
        text(","),
        format_trailing_comments_before_line_break(comma),
        if is_last {
            if has_trailing_comments && !force_line {
                text(" ")
            } else {
                jolt_fmt_ir::nil()
            }
        } else if force_line {
            hard_line()
        } else if has_trailing_comments {
            text(" ")
        } else {
            line()
        },
    ])
}

fn format_annotation_array_initializer_close_with_spacing(
    initializer: &AnnotationArrayInitializer,
) -> Doc {
    let close_has_leading_comments = initializer
        .close_brace()
        .as_ref()
        .is_some_and(|token| !token.leading_comments().is_empty());

    concat([
        if close_has_leading_comments {
            line()
        } else {
            soft_line()
        },
        format_annotation_array_initializer_close_delimiter(initializer),
    ])
}

fn format_annotation_array_initializer_close_delimiter(
    initializer: &AnnotationArrayInitializer,
) -> Doc {
    let close = initializer.close_brace();
    let close_has_leading_comments = close
        .as_ref()
        .is_some_and(|token| !token.leading_comments().is_empty());
    close.map_or_else(
        || text("}"),
        |close| {
            concat([
                if close_has_leading_comments {
                    format_leading_comments(&close)
                } else {
                    jolt_fmt_ir::nil()
                },
                text("}"),
                format_trailing_comments(&close),
            ])
        },
    )
}

fn format_annotation_array_initializer_close_delimiter_without_leading(
    initializer: &AnnotationArrayInitializer,
) -> Doc {
    initializer.close_brace().map_or_else(
        || text("}"),
        |close| concat([text("}"), format_trailing_comments(&close)]),
    )
}

fn format_annotation_array_initializer_dangling_comments(
    initializer: &AnnotationArrayInitializer,
) -> Doc {
    let mut docs = Vec::new();

    if let Some(open) = initializer.open_brace() {
        push_dangling_comments(&mut docs, open.trailing_comments());
    }
    if let Some(close) = initializer.close_brace() {
        push_dangling_comments(&mut docs, close.leading_comments());
    }

    concat(docs)
}

fn push_dangling_comments(docs: &mut Vec<Doc>, comments: Vec<JavaComment>) {
    for comment in comments {
        if !docs.is_empty() {
            docs.push(hard_line());
        }
        docs.push(format_comment(&comment));
    }
}
