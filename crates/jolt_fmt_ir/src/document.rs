use std::borrow::Cow;

use crate::width::{TextWidth, display_width, literal_text_metrics};

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

    const fn is_nil(&self) -> bool {
        matches!(self.0, DocKind::Nil)
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
    final_width: TextWidth,
    line_count: usize,
}

impl LiteralText<'_> {
    pub(crate) const fn final_width(&self) -> TextWidth {
        self.final_width
    }

    pub(crate) const fn is_multiline(&self) -> bool {
        self.line_count > 1
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
pub fn space<'source>() -> Doc<'source> {
    text(" ")
}

#[must_use]
pub fn literal_text<'source>(value: impl Into<Cow<'source, str>>) -> Doc<'source> {
    let text = value.into();
    let metrics = literal_text_metrics(&text);
    Doc(DocKind::LiteralText(LiteralText {
        text,
        final_width: metrics.final_width,
        line_count: metrics.line_count,
    }))
}

#[must_use]
pub fn concat<'source>(docs: impl IntoIterator<Item = Doc<'source>>) -> Doc<'source> {
    let mut docs = docs
        .into_iter()
        .filter(|doc| !doc.is_nil())
        .collect::<Vec<_>>();
    match docs.len() {
        0 => nil(),
        1 => docs.pop().expect("single concat doc exists"),
        _ => Doc(DocKind::Concat(docs)),
    }
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
    concat(joined)
}

#[must_use]
pub fn group(doc: Doc<'_>) -> Doc<'_> {
    if doc.is_nil() {
        return doc;
    }

    Doc(DocKind::Group(Group {
        should_break: false,
        contents: Box::new(doc),
    }))
}

#[must_use]
pub fn force_group(doc: Doc<'_>) -> Doc<'_> {
    if doc.is_nil() {
        return doc;
    }

    Doc(DocKind::Group(Group {
        should_break: true,
        contents: Box::new(doc),
    }))
}

#[must_use]
pub fn indent(doc: Doc<'_>) -> Doc<'_> {
    if doc.is_nil() {
        return doc;
    }

    Doc(DocKind::Indent(Indent {
        levels: 1,
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
