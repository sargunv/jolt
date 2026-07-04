use crate::document::{Doc, DocKind};
use crate::render::RenderError;
use crate::width::has_line_terminator;

pub(crate) fn validate_doc(doc: &Doc<'_>) -> Result<(), RenderError> {
    let mut stack = vec![doc];
    while let Some(doc) = stack.pop() {
        match doc.kind() {
            DocKind::Nil | DocKind::LiteralText(_) | DocKind::Line(_) => {}
            DocKind::Text(text) => validate_text(&text.text, "Text")?,
            DocKind::Concat(docs) => {
                for doc in docs {
                    stack.push(doc);
                }
            }
            DocKind::Group(group) => stack.push(&group.contents),
            DocKind::Indent(indent) => stack.push(&indent.contents),
            DocKind::IfBreak(if_break) => {
                stack.push(&if_break.breaks);
                stack.push(&if_break.flat);
            }
        }
    }
    Ok(())
}

fn validate_text(text: &str, context: &'static str) -> Result<(), RenderError> {
    if has_line_terminator(text) {
        Err(RenderError::invalid_text(context))
    } else {
        Ok(())
    }
}
