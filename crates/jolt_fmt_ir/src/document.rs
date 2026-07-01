use std::sync::Arc;

use crate::render::RenderError;
use crate::validation::{contains_marker, validate_literal_text};
use crate::width::{TextWidth, display_width, literal_line_widths};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GroupId(pub u32);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BreakMarkerId(pub u32);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LevelBreakTag(pub u32);

/// Opaque formatter document node.
///
/// Build documents with the constructor functions in this crate rather than
/// assembling IR variants directly.
///
/// ```compile_fail
/// let _ = jolt_fmt_ir::Doc::BestFitting(Vec::new());
/// ```
#[derive(Clone, Debug)]
pub struct Doc(Arc<DocKind>);

impl Doc {
    pub(crate) fn kind(&self) -> &DocKind {
        self.0.as_ref()
    }

    pub(crate) fn cache_ptr(&self) -> *const DocKind {
        Arc::as_ptr(&self.0)
    }
}

impl PartialEq for Doc {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_ref() == other.0.as_ref()
    }
}

impl Eq for Doc {}

fn make_doc(kind: DocKind) -> Doc {
    Doc(Arc::new(kind))
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
    IndentIfLevelBreak(IndentIfLevelBreak),
    TrailingFlatWidth(TrailingFlatWidth),
    LineSuffix(Box<Doc>),
    LineSuffixBoundary,
    BestFitting(Vec<Doc>),
    BreakLevel(BreakLevel),
    BreakParent,
}

/// Controls how a break point behaves when its enclosing [`break_level`] does not
/// fit on one line.
///
/// Semantics mirror google-java-format `FillMode`:
/// - [`Unified`](Self::Unified) corresponds to `breakOp` / unified breaks.
/// - [`Independent`](Self::Independent) corresponds to `breakToFill` / fill breaks.
/// - [`Forced`](Self::Forced) corresponds to forced breaks and makes the level
///   unable to fit on one line.
///
/// When a level fits on one line, all breaks use their flat spellings regardless
/// of mode. Nested levels still perform their own one-line fit check at the
/// current column even if an outer level is broken.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LevelBreakMode {
    /// When the level breaks, this break breaks too.
    Unified,
    /// Break only when the following segment does not fit on the current line.
    Independent,
    /// Always break.
    Forced,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LevelBreak {
    pub mode: LevelBreakMode,
    pub flat: FlatLine,
    /// Text emitted on the broken side of the break, after newline/indent.
    ///
    /// GJF encodes this token in the following segment; this field models that
    /// spelling without duplicating segment content in flat layout.
    pub broken_prefix: Doc,
    pub indent_delta: i16,
    pub tag: Option<LevelBreakTag>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BreakLevel {
    /// Extra indent applied to the whole level when it does not fit on one line.
    pub plus_indent: i16,
    pub segments: Vec<Doc>,
    pub breaks: Vec<LevelBreak>,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TrailingFlatWidth {
    pub(crate) width: TextWidth,
    pub(crate) contents: Box<Doc>,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IndentIfLevelBreak {
    pub(crate) tag: LevelBreakTag,
    pub(crate) if_broken_levels: i16,
    pub(crate) if_flat_levels: i16,
    pub(crate) contents: Box<Doc>,
}

#[must_use]
pub fn nil() -> Doc {
    make_doc(DocKind::Nil)
}

#[must_use]
pub fn text(value: impl Into<Box<str>>) -> Doc {
    let text = value.into();
    let width = display_width(&text);
    make_doc(DocKind::Text(Text { text, width }))
}

#[must_use]
pub fn text_with_width(value: impl Into<Box<str>>, width: TextWidth) -> Doc {
    make_doc(DocKind::Text(Text {
        text: value.into(),
        width,
    }))
}

#[must_use]
pub fn with_trailing_flat_width(width: TextWidth, doc: Doc) -> Doc {
    make_doc(DocKind::TrailingFlatWidth(TrailingFlatWidth {
        width,
        contents: Box::new(doc),
    }))
}

#[must_use]
pub fn literal_text(value: impl Into<Box<str>>) -> Doc {
    let text = value.into();
    let line_widths = literal_line_widths(&text);
    make_doc(DocKind::LiteralText(LiteralText { text, line_widths }))
}

#[must_use]
pub fn literal_text_with_width(value: impl Into<Box<str>>, width: TextWidth) -> Doc {
    let text = value.into();
    let mut line_widths = literal_line_widths(&text).into_vec();
    if let Some(final_width) = line_widths.last_mut() {
        *final_width = width;
    }
    make_doc(DocKind::LiteralText(LiteralText {
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
    Ok(make_doc(DocKind::LiteralText(literal)))
}

#[must_use]
pub fn concat(docs: impl IntoIterator<Item = Doc>) -> Doc {
    make_doc(DocKind::Concat(docs.into_iter().collect()))
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
    make_doc(DocKind::Concat(joined))
}

#[must_use]
pub fn group(doc: Doc) -> Doc {
    make_doc(DocKind::Group(Group {
        id: None,
        should_break: false,
        fit: GroupFit::LineWidth,
        contents: Box::new(doc),
    }))
}

#[must_use]
pub fn group_id(id: GroupId, doc: Doc) -> Doc {
    make_doc(DocKind::Group(Group {
        id: Some(id),
        should_break: false,
        fit: GroupFit::LineWidth,
        contents: Box::new(doc),
    }))
}

#[must_use]
pub fn force_group(doc: Doc) -> Doc {
    make_doc(DocKind::Group(Group {
        id: None,
        should_break: true,
        fit: GroupFit::LineWidth,
        contents: Box::new(doc),
    }))
}

#[must_use]
pub fn force_group_id(id: GroupId, doc: Doc) -> Doc {
    make_doc(DocKind::Group(Group {
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
    Ok(make_doc(DocKind::Group(Group {
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
    make_doc(DocKind::Fill(entries))
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
    make_doc(DocKind::Indent(Indent {
        levels,
        contents: Box::new(doc),
    }))
}

#[must_use]
pub fn align(spaces: u16, doc: Doc) -> Doc {
    make_doc(DocKind::Align(Align {
        spaces,
        contents: Box::new(doc),
    }))
}

#[must_use]
pub fn line() -> Doc {
    make_doc(DocKind::Line(Line {
        mode: LineMode::SoftOrSpace,
        flat: FlatLine::Space,
        indent_delta: 0,
        propagate_break: false,
        marker: None,
    }))
}

#[must_use]
pub fn soft_line() -> Doc {
    make_doc(DocKind::Line(Line {
        mode: LineMode::Soft,
        flat: FlatLine::Empty,
        indent_delta: 0,
        propagate_break: false,
        marker: None,
    }))
}

#[must_use]
pub fn hard_line() -> Doc {
    make_doc(DocKind::Line(Line {
        mode: LineMode::Hard,
        flat: FlatLine::Empty,
        indent_delta: 0,
        propagate_break: true,
        marker: None,
    }))
}

#[must_use]
pub fn empty_line() -> Doc {
    make_doc(DocKind::Line(Line {
        mode: LineMode::Empty,
        flat: FlatLine::Empty,
        indent_delta: 0,
        propagate_break: true,
        marker: None,
    }))
}

#[must_use]
pub fn break_(flat: FlatLine, indent_delta: i16) -> Doc {
    make_doc(DocKind::Line(Line {
        mode: LineMode::Soft,
        flat,
        indent_delta,
        propagate_break: false,
        marker: None,
    }))
}

#[must_use]
pub fn hard_line_without_break_parent() -> Doc {
    make_doc(DocKind::Line(Line {
        mode: LineMode::Hard,
        flat: FlatLine::Empty,
        indent_delta: 0,
        propagate_break: false,
        marker: None,
    }))
}

#[must_use]
pub fn marked_break(marker: BreakMarkerId, flat: FlatLine, indent_delta: i16) -> Doc {
    make_doc(DocKind::Line(Line {
        mode: LineMode::Soft,
        flat,
        indent_delta,
        propagate_break: false,
        marker: Some(marker),
    }))
}

#[must_use]
pub fn if_break(breaks: Doc, flat: Doc) -> Doc {
    make_doc(DocKind::IfBreak(IfBreak {
        group_id: None,
        breaks: Box::new(breaks),
        flat: Box::new(flat),
    }))
}

#[must_use]
pub fn if_group_breaks(id: GroupId, breaks: Doc, flat: Doc) -> Doc {
    make_doc(DocKind::IfBreak(IfBreak {
        group_id: Some(id),
        breaks: Box::new(breaks),
        flat: Box::new(flat),
    }))
}

#[must_use]
pub fn indent_if_break(id: GroupId, doc: Doc) -> Doc {
    make_doc(DocKind::IndentIfBreak(IndentIfBreak {
        group_id: id,
        contents: Box::new(doc),
        negate: false,
    }))
}

#[must_use]
pub fn indent_if_level_breaks(
    tag: LevelBreakTag,
    if_broken_levels: i16,
    if_flat_levels: i16,
    doc: Doc,
) -> Doc {
    make_doc(DocKind::IndentIfLevelBreak(IndentIfLevelBreak {
        tag,
        if_broken_levels,
        if_flat_levels,
        contents: Box::new(doc),
    }))
}

#[must_use]
pub fn line_suffix(doc: Doc) -> Doc {
    make_doc(DocKind::LineSuffix(Box::new(doc)))
}

#[must_use]
pub fn line_suffix_boundary() -> Doc {
    make_doc(DocKind::LineSuffixBoundary)
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
    make_doc(DocKind::BestFitting(docs))
}

#[must_use]
pub fn break_parent() -> Doc {
    make_doc(DocKind::BreakParent)
}

/// Creates a break point for use inside [`break_level`].
#[must_use]
pub fn level_break(mode: LevelBreakMode, flat: FlatLine, indent_delta: i16) -> LevelBreak {
    level_break_with_prefix(mode, flat, nil(), indent_delta)
}

/// Creates a break point with distinct flat and broken-side spellings.
#[must_use]
pub fn level_break_with_prefix(
    mode: LevelBreakMode,
    flat: FlatLine,
    broken_prefix: Doc,
    indent_delta: i16,
) -> LevelBreak {
    LevelBreak {
        mode,
        flat,
        broken_prefix,
        indent_delta,
        tag: None,
    }
}

#[must_use]
pub fn tagged_level_break_with_prefix(
    tag: LevelBreakTag,
    mode: LevelBreakMode,
    flat: FlatLine,
    broken_prefix: Doc,
    indent_delta: i16,
) -> LevelBreak {
    LevelBreak {
        mode,
        flat,
        broken_prefix,
        indent_delta,
        tag: Some(tag),
    }
}

/// Creates a level whose segments are separated by optional break points.
///
/// When the level does not fit on one line, `plus_indent` is added to the
/// current indent for break layout. Per-break [`LevelBreak::indent_delta`] is
/// applied on top when that break is taken.
///
/// `breaks.len()` must equal `segments.len().saturating_sub(1)`.
///
/// # Errors
///
/// Returns [`RenderError::MalformedBreakLevel`] when the segment and break
/// counts are inconsistent.
pub fn break_level_with_indent(
    plus_indent: i16,
    segments: impl IntoIterator<Item = Doc>,
    breaks: impl IntoIterator<Item = LevelBreak>,
) -> Result<Doc, RenderError> {
    let segments: Vec<_> = segments.into_iter().collect();
    let breaks: Vec<_> = breaks.into_iter().collect();
    validate_break_level_shape(&segments, &breaks)?;
    Ok(make_doc(DocKind::BreakLevel(BreakLevel {
        plus_indent,
        segments,
        breaks,
    })))
}

/// Creates a level with no extra broken-side indent.
///
/// Shorthand for [`break_level_with_indent`] with `plus_indent` set to zero.
///
/// # Errors
///
/// Returns [`RenderError::MalformedBreakLevel`] when the segment and break
/// counts are inconsistent.
pub fn break_level(
    segments: impl IntoIterator<Item = Doc>,
    breaks: impl IntoIterator<Item = LevelBreak>,
) -> Result<Doc, RenderError> {
    break_level_with_indent(0, segments, breaks)
}

fn validate_break_level_shape(segments: &[Doc], breaks: &[LevelBreak]) -> Result<(), RenderError> {
    if segments.is_empty() {
        return Err(RenderError::MalformedBreakLevel {
            reason: "level must contain at least one segment",
        });
    }
    if breaks.len() + 1 != segments.len() {
        return Err(RenderError::MalformedBreakLevel {
            reason: "break count must be one less than segment count",
        });
    }
    Ok(())
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
