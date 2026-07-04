use std::borrow::Cow;

use crate::width::{TextWidth, display_width, literal_line_widths};

/// Opaque formatter document node.
///
/// Build documents with the constructor functions in this crate rather than
/// assembling IR variants directly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Doc<'source>(DocKind<'source>);

impl<'source> Doc<'source> {
    pub(crate) const fn kind(&self) -> &DocKind<'source> {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DocKind<'source> {
    Nil,
    Text(Text<'source>),
    LiteralText(LiteralText<'source>),
    Concat(Vec<Doc<'source>>),
    Group(Group<'source>),
    Indent(Indent<'source>),
    Line(Line),
    IfBreak(IfBreak<'source>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Text<'source> {
    pub(crate) text: Cow<'source, str>,
    pub(crate) width: TextWidth,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LiteralText<'source> {
    pub(crate) text: Cow<'source, str>,
    pub(crate) line_widths: Box<[TextWidth]>,
}

impl LiteralText<'_> {
    pub(crate) fn final_width(&self) -> TextWidth {
        self.line_widths.last().copied().unwrap_or(TextWidth::ZERO)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Group<'source> {
    pub(crate) should_break: bool,
    pub(crate) contents: Box<Doc<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Indent<'source> {
    pub(crate) levels: i16,
    pub(crate) contents: Box<Doc<'source>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Line {
    pub(crate) mode: LineMode,
    pub(crate) flat: FlatLine,
    pub(crate) indent_delta: i16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LineMode {
    Soft,
    SoftOrSpace,
    Hard,
    Empty,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FlatLine {
    Empty,
    Space,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IfBreak<'source> {
    pub(crate) breaks: Box<Doc<'source>>,
    pub(crate) flat: Box<Doc<'source>>,
}

#[must_use]
pub const fn nil<'source>() -> Doc<'source> {
    Doc(DocKind::Nil)
}

#[must_use]
pub fn text<'source>(value: impl Into<Cow<'source, str>>) -> Doc<'source> {
    let text = value.into();
    let width = display_width(&text);
    Doc(DocKind::Text(Text { text, width }))
}

#[must_use]
pub fn literal_text<'source>(value: impl Into<Cow<'source, str>>) -> Doc<'source> {
    let text = value.into();
    let line_widths = literal_line_widths(&text);
    Doc(DocKind::LiteralText(LiteralText { text, line_widths }))
}

#[must_use]
pub fn concat<'source>(docs: impl IntoIterator<Item = Doc<'source>>) -> Doc<'source> {
    Doc(DocKind::Concat(docs.into_iter().collect()))
}

#[must_use]
pub fn join<'source>(
    separator: &Doc<'source>,
    docs: impl IntoIterator<Item = Doc<'source>>,
) -> Doc<'source> {
    let mut joined = Vec::new();
    for doc in docs {
        if !joined.is_empty() {
            joined.push(separator.clone());
        }
        joined.push(doc);
    }
    Doc(DocKind::Concat(joined))
}

#[must_use]
pub fn group(doc: Doc<'_>) -> Doc<'_> {
    Doc(DocKind::Group(Group {
        should_break: false,
        contents: Box::new(doc),
    }))
}

#[must_use]
pub fn force_group(doc: Doc<'_>) -> Doc<'_> {
    Doc(DocKind::Group(Group {
        should_break: true,
        contents: Box::new(doc),
    }))
}

#[must_use]
pub fn indent(doc: Doc<'_>) -> Doc<'_> {
    indent_by(1, doc)
}

/// Creates a document indented by `levels` indentation levels.
///
/// # Panics
///
/// Panics if `levels` does not fit in the renderer's signed indentation delta.
#[must_use]
pub fn indent_by(levels: u16, doc: Doc<'_>) -> Doc<'_> {
    Doc(DocKind::Indent(Indent {
        levels: i16::try_from(levels).expect("indent level count fits i16"),
        contents: Box::new(doc),
    }))
}

#[must_use]
pub const fn line<'source>() -> Doc<'source> {
    Doc(DocKind::Line(Line {
        mode: LineMode::SoftOrSpace,
        flat: FlatLine::Space,
        indent_delta: 0,
    }))
}

#[must_use]
pub const fn soft_line<'source>() -> Doc<'source> {
    Doc(DocKind::Line(Line {
        mode: LineMode::Soft,
        flat: FlatLine::Empty,
        indent_delta: 0,
    }))
}

#[must_use]
pub const fn hard_line<'source>() -> Doc<'source> {
    Doc(DocKind::Line(Line {
        mode: LineMode::Hard,
        flat: FlatLine::Empty,
        indent_delta: 0,
    }))
}

#[must_use]
pub const fn empty_line<'source>() -> Doc<'source> {
    Doc(DocKind::Line(Line {
        mode: LineMode::Empty,
        flat: FlatLine::Empty,
        indent_delta: 0,
    }))
}

#[must_use]
pub fn if_break<'source>(breaks: Doc<'source>, flat: Doc<'source>) -> Doc<'source> {
    Doc(DocKind::IfBreak(IfBreak {
        breaks: Box::new(breaks),
        flat: Box::new(flat),
    }))
}
