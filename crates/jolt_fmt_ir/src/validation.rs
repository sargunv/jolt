use crate::document::{Doc, DocKind, LiteralText};
use crate::render::RenderError;
use crate::width::{has_line_terminator, literal_line_count};

pub(crate) fn validate_doc(doc: &Doc<'_>) -> Result<(), RenderError> {
    let mut stack = vec![doc];
    while let Some(doc) = stack.pop() {
        match doc.kind() {
            DocKind::Nil | DocKind::Line(_) => {}
            DocKind::LiteralText(text) => validate_literal_text(text)?,
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
        Err(RenderError::InvalidText { context })
    } else {
        Ok(())
    }
}

fn validate_literal_text(text: &LiteralText<'_>) -> Result<(), RenderError> {
    let expected = literal_line_count(&text.text);
    let actual = text.line_widths.len();
    if expected == actual {
        Ok(())
    } else {
        Err(RenderError::InvalidLiteralWidths { expected, actual })
    }
}
