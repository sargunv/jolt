use crate::render::RenderError;
use crate::validation::{contains_marker, validate_literal_text};
use crate::width::{TextWidth, display_width, literal_line_widths};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct GroupId(pub u32);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BreakMarkerId(pub u32);

/// Opaque formatter document node.
///
/// Build documents with the constructor functions in this crate rather than
/// assembling IR variants directly.
///
/// ```compile_fail
/// let _ = jolt_fmt_ir::Doc::BestFitting(Vec::new());
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Doc(DocKind);

impl Doc {
    pub(crate) const fn kind(&self) -> &DocKind {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DocKind {
    Nil,
    Text(Text),
    LiteralText(LiteralText),
    Concat(Vec<Doc>),
    Group(Group),
    Fill(Vec<FillEntry>),
    Indent(Indent),
    Align(Align),
    Line(Line),
    IfBreak(IfBreak),
    IndentIfBreak(IndentIfBreak),
    LineSuffix(Box<Doc>),
    LineSuffixBoundary,
    BestFitting(Vec<Doc>),
    BreakParent,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Text {
    pub(crate) text: Box<str>,
    pub(crate) width: TextWidth,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LiteralText {
    pub(crate) text: Box<str>,
    pub(crate) line_widths: Box<[TextWidth]>,
}

impl LiteralText {
    pub(crate) fn final_width(&self) -> TextWidth {
        self.line_widths.last().copied().unwrap_or(TextWidth::ZERO)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Group {
    pub(crate) id: Option<GroupId>,
    pub(crate) should_break: bool,
    pub(crate) fit: GroupFit,
    pub(crate) contents: Box<Doc>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GroupFit {
    LineWidth,
    MarkedBreak {
        marker: BreakMarkerId,
        max_column_before_last_marked_break: TextWidth,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FillEntry {
    pub(crate) content: Doc,
    pub(crate) separator: Option<Doc>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Indent {
    pub(crate) levels: u16,
    pub(crate) contents: Box<Doc>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Align {
    pub(crate) spaces: u16,
    pub(crate) contents: Box<Doc>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Line {
    pub(crate) mode: LineMode,
    pub(crate) flat: FlatLine,
    pub(crate) indent_delta: i16,
    pub(crate) propagate_break: bool,
    pub(crate) marker: Option<BreakMarkerId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LineMode {
    Soft,
    SoftOrSpace,
    Hard,
    Empty,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FlatLine {
    Empty,
    Space,
    Text(Box<str>, TextWidth),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IfBreak {
    pub(crate) group_id: Option<GroupId>,
    pub(crate) breaks: Box<Doc>,
    pub(crate) flat: Box<Doc>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IndentIfBreak {
    pub(crate) group_id: GroupId,
    pub(crate) contents: Box<Doc>,
    pub(crate) negate: bool,
}

#[must_use]
pub const fn nil() -> Doc {
    Doc(DocKind::Nil)
}

#[must_use]
pub fn text(value: impl Into<Box<str>>) -> Doc {
    let text = value.into();
    let width = display_width(&text);
    Doc(DocKind::Text(Text { text, width }))
}

#[must_use]
pub fn text_with_width(value: impl Into<Box<str>>, width: TextWidth) -> Doc {
    Doc(DocKind::Text(Text {
        text: value.into(),
        width,
    }))
}

#[must_use]
pub fn literal_text(value: impl Into<Box<str>>) -> Doc {
    let text = value.into();
    let line_widths = literal_line_widths(&text);
    Doc(DocKind::LiteralText(LiteralText { text, line_widths }))
}

#[must_use]
pub fn literal_text_with_width(value: impl Into<Box<str>>, width: TextWidth) -> Doc {
    let text = value.into();
    let mut line_widths = literal_line_widths(&text).into_vec();
    if let Some(final_width) = line_widths.last_mut() {
        *final_width = width;
    }
    Doc(DocKind::LiteralText(LiteralText {
        text,
        line_widths: line_widths.into_boxed_slice(),
    }))
}

/// Creates literal text with explicit widths for every literal line.
///
/// # Errors
///
/// Returns [`RenderError::InvalidLiteralWidths`] when the width count does not
/// match the number of lines in the literal text.
pub fn literal_text_with_line_widths(
    value: impl Into<Box<str>>,
    line_widths: impl IntoIterator<Item = TextWidth>,
) -> Result<Doc, RenderError> {
    let literal = LiteralText {
        text: value.into(),
        line_widths: line_widths.into_iter().collect(),
    };
    validate_literal_text(&literal)?;
    Ok(Doc(DocKind::LiteralText(literal)))
}

#[must_use]
pub fn concat(docs: impl IntoIterator<Item = Doc>) -> Doc {
    Doc(DocKind::Concat(docs.into_iter().collect()))
}

#[allow(clippy::needless_pass_by_value)]
#[must_use]
pub fn join(separator: Doc, docs: impl IntoIterator<Item = Doc>) -> Doc {
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
pub fn group(doc: Doc) -> Doc {
    Doc(DocKind::Group(Group {
        id: None,
        should_break: false,
        fit: GroupFit::LineWidth,
        contents: Box::new(doc),
    }))
}

#[must_use]
pub fn group_id(id: GroupId, doc: Doc) -> Doc {
    Doc(DocKind::Group(Group {
        id: Some(id),
        should_break: false,
        fit: GroupFit::LineWidth,
        contents: Box::new(doc),
    }))
}

#[must_use]
pub fn force_group(doc: Doc) -> Doc {
    Doc(DocKind::Group(Group {
        id: None,
        should_break: true,
        fit: GroupFit::LineWidth,
        contents: Box::new(doc),
    }))
}

#[must_use]
pub fn force_group_id(id: GroupId, doc: Doc) -> Doc {
    Doc(DocKind::Group(Group {
        id: Some(id),
        should_break: true,
        fit: GroupFit::LineWidth,
        contents: Box::new(doc),
    }))
}

/// Creates a group with a custom fit constraint.
///
/// # Errors
///
/// Returns [`RenderError::MissingBreakMarker`] when a marked-break fit
/// references a marker that does not appear inside the group contents.
pub fn group_with_fit(fit: GroupFit, doc: Doc) -> Result<Doc, RenderError> {
    if let GroupFit::MarkedBreak { marker, .. } = fit
        && !contains_marker(&doc, marker)
    {
        return Err(RenderError::MissingBreakMarker(marker));
    }
    Ok(Doc(DocKind::Group(Group {
        id: None,
        should_break: false,
        fit,
        contents: Box::new(doc),
    })))
}

#[must_use]
pub fn fill(entries: impl IntoIterator<Item = FillEntry>, final_content: Doc) -> Doc {
    let mut entries: Vec<_> = entries.into_iter().collect();
    entries.push(FillEntry {
        content: final_content,
        separator: None,
    });
    Doc(DocKind::Fill(entries))
}

#[must_use]
pub fn fill_entry(content: Doc, separator: Doc) -> FillEntry {
    FillEntry {
        content,
        separator: Some(separator),
    }
}

#[must_use]
pub fn indent(doc: Doc) -> Doc {
    indent_by(1, doc)
}

#[must_use]
pub fn indent_by(levels: u16, doc: Doc) -> Doc {
    Doc(DocKind::Indent(Indent {
        levels,
        contents: Box::new(doc),
    }))
}

#[must_use]
pub fn align(spaces: u16, doc: Doc) -> Doc {
    Doc(DocKind::Align(Align {
        spaces,
        contents: Box::new(doc),
    }))
}

#[must_use]
pub const fn line() -> Doc {
    Doc(DocKind::Line(Line {
        mode: LineMode::SoftOrSpace,
        flat: FlatLine::Space,
        indent_delta: 0,
        propagate_break: false,
        marker: None,
    }))
}

#[must_use]
pub const fn soft_line() -> Doc {
    Doc(DocKind::Line(Line {
        mode: LineMode::Soft,
        flat: FlatLine::Empty,
        indent_delta: 0,
        propagate_break: false,
        marker: None,
    }))
}

#[must_use]
pub const fn hard_line() -> Doc {
    Doc(DocKind::Line(Line {
        mode: LineMode::Hard,
        flat: FlatLine::Empty,
        indent_delta: 0,
        propagate_break: true,
        marker: None,
    }))
}

#[must_use]
pub const fn empty_line() -> Doc {
    Doc(DocKind::Line(Line {
        mode: LineMode::Empty,
        flat: FlatLine::Empty,
        indent_delta: 0,
        propagate_break: true,
        marker: None,
    }))
}

#[must_use]
pub fn break_(flat: FlatLine, indent_delta: i16) -> Doc {
    Doc(DocKind::Line(Line {
        mode: LineMode::Soft,
        flat,
        indent_delta,
        propagate_break: false,
        marker: None,
    }))
}

#[must_use]
pub fn hard_line_without_break_parent() -> Doc {
    Doc(DocKind::Line(Line {
        mode: LineMode::Hard,
        flat: FlatLine::Empty,
        indent_delta: 0,
        propagate_break: false,
        marker: None,
    }))
}

#[must_use]
pub fn marked_break(marker: BreakMarkerId, flat: FlatLine, indent_delta: i16) -> Doc {
    Doc(DocKind::Line(Line {
        mode: LineMode::Soft,
        flat,
        indent_delta,
        propagate_break: false,
        marker: Some(marker),
    }))
}

#[must_use]
pub fn if_break(breaks: Doc, flat: Doc) -> Doc {
    Doc(DocKind::IfBreak(IfBreak {
        group_id: None,
        breaks: Box::new(breaks),
        flat: Box::new(flat),
    }))
}

#[must_use]
pub fn if_group_breaks(id: GroupId, breaks: Doc, flat: Doc) -> Doc {
    Doc(DocKind::IfBreak(IfBreak {
        group_id: Some(id),
        breaks: Box::new(breaks),
        flat: Box::new(flat),
    }))
}

#[must_use]
pub fn indent_if_break(id: GroupId, doc: Doc) -> Doc {
    Doc(DocKind::IndentIfBreak(IndentIfBreak {
        group_id: id,
        contents: Box::new(doc),
        negate: false,
    }))
}

#[must_use]
pub fn line_suffix(doc: Doc) -> Doc {
    Doc(DocKind::LineSuffix(Box::new(doc)))
}

#[must_use]
pub const fn line_suffix_boundary() -> Doc {
    Doc(DocKind::LineSuffixBoundary)
}

/// Chooses the first variant that fits, falling back to the final variant in
/// break mode.
///
/// At least one variant is required.
///
/// ```compile_fail
/// use jolt_fmt_ir::best_fitting;
///
/// let _ = best_fitting([]);
/// ```
#[must_use]
pub fn best_fitting(first: Doc, rest: impl IntoIterator<Item = Doc>) -> Doc {
    let mut docs = vec![first];
    docs.extend(rest);
    Doc(DocKind::BestFitting(docs))
}

#[must_use]
pub const fn break_parent() -> Doc {
    Doc(DocKind::BreakParent)
}

#[must_use]
pub fn flat_text(value: impl Into<Box<str>>) -> FlatLine {
    let text = value.into();
    let width = display_width(&text);
    FlatLine::Text(text, width)
}

#[must_use]
pub fn flat_text_with_width(value: impl Into<Box<str>>, width: TextWidth) -> FlatLine {
    FlatLine::Text(value.into(), width)
}
