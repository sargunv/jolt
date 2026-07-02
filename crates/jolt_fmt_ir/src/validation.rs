use crate::document::{Doc, DocKind, FlatLine, Line, LineMode, LiteralText};
use crate::render::{Mode, RenderError};
use crate::width::{has_line_terminator, literal_line_count};

pub(crate) fn validate_doc(doc: &Doc) -> Result<(), RenderError> {
    let mut stack = vec![doc];
    while let Some(doc) = stack.pop() {
        match doc.kind() {
            DocKind::Nil | DocKind::LineSuffixBoundary | DocKind::BreakParent => {}
            DocKind::LiteralText(text) => validate_literal_text(text)?,
            DocKind::Text(text) => validate_text(&text.text, "Text")?,
            DocKind::Concat(docs) => {
                for doc in docs {
                    stack.push(doc);
                }
            }
            DocKind::Group(group) => stack.push(&group.contents),
            DocKind::Fill(entries) => {
                for (index, entry) in entries.iter().enumerate() {
                    if index + 1 == entries.len() && entry.separator.is_some() {
                        return Err(RenderError::MalformedFill {
                            index,
                            reason: "last fill entry must not have a separator",
                        });
                    }
                    if index + 1 < entries.len() && entry.separator.is_none() {
                        return Err(RenderError::MalformedFill {
                            index,
                            reason: "non-final fill entry must have a separator",
                        });
                    }
                    stack.push(&entry.content);
                    if let Some(separator) = &entry.separator {
                        stack.push(separator);
                    }
                }
            }
            DocKind::Indent(indent) => stack.push(&indent.contents),
            DocKind::Align(align) => stack.push(&align.contents),
            DocKind::Line(line) => {
                if let FlatLine::Text(text, _) = &line.flat {
                    validate_text(text, "FlatLine::Text")?;
                }
            }
            DocKind::IfBreak(if_break) => {
                stack.push(&if_break.breaks);
                stack.push(&if_break.flat);
            }
            DocKind::IndentIfBreak(indent_if_break) => {
                stack.push(&indent_if_break.contents);
            }
            DocKind::LineSuffix(doc) => {
                stack.push(doc);
                validate_line_suffix_doc(doc, Mode::Flat)?;
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

pub(crate) fn validate_literal_text(text: &LiteralText) -> Result<(), RenderError> {
    let expected = literal_line_count(&text.text);
    let actual = text.line_widths.len();
    if expected == actual {
        Ok(())
    } else {
        Err(RenderError::InvalidLiteralWidths { expected, actual })
    }
}

fn validate_line_suffix_doc(doc: &Doc, mode: Mode) -> Result<(), RenderError> {
    match doc.kind() {
        DocKind::Nil | DocKind::Text(_) | DocKind::LineSuffixBoundary => Ok(()),
        DocKind::BreakParent => Err(RenderError::InvalidLineSuffix {
            reason: "break parent",
        }),
        DocKind::LiteralText(text) => {
            if has_line_terminator(&text.text) {
                Err(RenderError::InvalidLineSuffix {
                    reason: "literal line terminator",
                })
            } else {
                Ok(())
            }
        }
        DocKind::Concat(docs) => {
            for doc in docs {
                validate_line_suffix_doc(doc, mode)?;
            }
            Ok(())
        }
        DocKind::Group(group) => {
            if group.should_break {
                validate_line_suffix_doc(&group.contents, Mode::Break)
            } else {
                validate_line_suffix_doc(&group.contents, Mode::Flat)?;
                validate_line_suffix_doc(&group.contents, Mode::Break)
            }
        }
        DocKind::Fill(entries) => {
            for entry in entries {
                validate_line_suffix_doc(&entry.content, mode)?;
                if let Some(separator) = &entry.separator {
                    validate_line_suffix_doc(separator, Mode::Break)?;
                }
            }
            Ok(())
        }
        DocKind::Indent(indent) => validate_line_suffix_doc(&indent.contents, mode),
        DocKind::Align(align) => validate_line_suffix_doc(&align.contents, mode),
        DocKind::Line(line) => validate_line_suffix_line(line, mode),
        DocKind::IfBreak(if_break) => {
            validate_line_suffix_doc(&if_break.breaks, mode)?;
            validate_line_suffix_doc(&if_break.flat, mode)
        }
        DocKind::IndentIfBreak(indent_if_break) => {
            validate_line_suffix_doc(&indent_if_break.contents, mode)
        }
        DocKind::LineSuffix(doc) => validate_line_suffix_doc(doc, Mode::Flat),
    }
}

fn validate_line_suffix_line(line: &Line, mode: Mode) -> Result<(), RenderError> {
    match (mode, line.mode) {
        (Mode::Flat, LineMode::Soft | LineMode::SoftOrSpace) => Ok(()),
        (Mode::Flat, LineMode::Hard) => Err(RenderError::InvalidLineSuffix {
            reason: "hard line",
        }),
        (Mode::Flat, LineMode::Empty) => Err(RenderError::InvalidLineSuffix {
            reason: "empty line",
        }),
        (Mode::Break, _) => Err(RenderError::InvalidLineSuffix {
            reason: "line break",
        }),
    }
}
