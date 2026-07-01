use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::document::{
    BreakLevel, BreakMarkerId, Doc, DocKind, FillEntry, FlatLine, Group, GroupFit, GroupId,
    IfBreak, IndentIfBreak, IndentIfLevelBreak, LevelBreak, LevelBreakMode, LevelBreakTag, Line,
    LineMode, LiteralText,
};
use crate::validation::validate_doc;
use crate::width::{TextWidth, add_width, fits_at_column, has_line_terminator, push_repeated};

const MAX_FLAT_WIDTH: TextWidth = TextWidth::new(1000);

fn level_break_should_break(
    break_: &LevelBreak,
    must_break: bool,
    column: TextWidth,
    segment_width: TextWidth,
    line_width: TextWidth,
) -> bool {
    match break_.mode {
        LevelBreakMode::Forced | LevelBreakMode::Unified => true,
        LevelBreakMode::Independent => {
            let unbroken_width = add_width(flat_line_width(&break_.flat), segment_width);
            must_break || !fits_at_column(column, unbroken_width, line_width)
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndentStyle {
    Space,
    Tab,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineEnding {
    Lf,
    CrLf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderOptions {
    pub line_width: TextWidth,
    pub indent_width: u16,
    pub indent_style: IndentStyle,
    pub line_ending: LineEnding,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            line_width: TextWidth::new(100),
            indent_width: 2,
            indent_style: IndentStyle::Space,
            line_ending: LineEnding::Lf,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Rendered {
    pub text: String,
    pub stats: RenderStats,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RenderStats {
    pub line_count: u32,
    pub max_column: TextWidth,
    pub group_count: u32,
    pub expanded_group_count: u32,
    pub line_suffix_count: u32,
    pub break_level_count: u32,
    pub flat_width_cache_hits: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RenderError {
    InvalidText { context: &'static str },
    InvalidLiteralWidths { expected: usize, actual: usize },
    InvalidLineSuffix { reason: &'static str },
    EmptyBestFitting,
    MalformedFill { index: usize, reason: &'static str },
    UnknownGroupId(GroupId),
    NoCurrentGroup,
    MissingBreakMarker(BreakMarkerId),
    MalformedBreakLevel { reason: &'static str },
}

impl fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidText { context } => {
                write!(formatter, "{context} must not contain line terminators")
            }
            Self::InvalidLiteralWidths { expected, actual } => write!(
                formatter,
                "literal text line width count {actual} does not match line count {expected}"
            ),
            Self::InvalidLineSuffix { reason } => {
                write!(
                    formatter,
                    "line suffix must stay on the current line: {reason}"
                )
            }
            Self::EmptyBestFitting => {
                formatter.write_str("best-fitting document must not be empty")
            }
            Self::MalformedFill { index, reason } => {
                write!(formatter, "malformed fill entry at index {index}: {reason}")
            }
            Self::UnknownGroupId(id) => write!(formatter, "unknown group id {}", id.0),
            Self::NoCurrentGroup => formatter.write_str("if_break requires a current group"),
            Self::MissingBreakMarker(id) => write!(formatter, "missing break marker {}", id.0),
            Self::MalformedBreakLevel { reason } => {
                write!(formatter, "malformed break level: {reason}")
            }
        }
    }
}

impl Error for RenderError {}

/// Renders a document using the provided options.
///
/// # Errors
///
/// Returns [`RenderError`] when the document is structurally invalid or contains
/// invalid non-literal text.
pub fn render(doc: &Doc, options: RenderOptions) -> Result<Rendered, RenderError> {
    validate_doc(doc)?;
    let mut renderer = Renderer::new(options);
    renderer.render_doc(doc, Mode::Break)?;
    renderer.flush_line_suffixes()?;
    Ok(renderer.finish())
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum Mode {
    Flat,
    Break,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct GroupFrame {
    id: Option<GroupId>,
    is_broken: bool,
}

struct Renderer {
    options: RenderOptions,
    output: String,
    line: u32,
    column: TextWidth,
    max_column: TextWidth,
    indent_levels: i32,
    align_spaces: u32,
    group_modes: BTreeMap<GroupId, bool>,
    group_stack: Vec<GroupFrame>,
    level_break_tags: BTreeMap<LevelBreakTag, bool>,
    trailing_flat_width: TextWidth,
    line_suffixes: Vec<Doc>,
    flushing_suffixes: bool,
    stats: RenderStats,
    fit_cache: Rc<RefCell<HashMap<FitCacheKey, CachedFit>>>,
    flat_width_cache: Rc<RefCell<HashMap<*const DocKind, FlatWidthSummary>>>,
    flat_width_cache_hits: Rc<RefCell<u32>>,
    level_break_indent_cache: Rc<RefCell<HashMap<*const DocKind, bool>>>,
}

impl Renderer {
    fn new(options: RenderOptions) -> Self {
        Self {
            options,
            output: String::new(),
            line: 1,
            column: TextWidth::ZERO,
            max_column: TextWidth::ZERO,
            indent_levels: 0,
            align_spaces: 0,
            group_modes: BTreeMap::new(),
            group_stack: Vec::new(),
            level_break_tags: BTreeMap::new(),
            trailing_flat_width: TextWidth::ZERO,
            line_suffixes: Vec::new(),
            flushing_suffixes: false,
            stats: RenderStats::default(),
            fit_cache: Rc::new(RefCell::new(HashMap::new())),
            flat_width_cache: Rc::new(RefCell::new(HashMap::new())),
            flat_width_cache_hits: Rc::new(RefCell::new(0)),
            level_break_indent_cache: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    fn flat_width_computer(&self) -> FlatWidthComputer {
        FlatWidthComputer::new(
            Rc::clone(&self.flat_width_cache),
            Rc::clone(&self.flat_width_cache_hits),
        )
    }

    fn finish(mut self) -> Rendered {
        self.stats.line_count = self.line;
        self.stats.max_column = self.max_column.max(self.column);
        self.stats.flat_width_cache_hits = *self.flat_width_cache_hits.borrow();
        Rendered {
            text: self.output,
            stats: self.stats,
        }
    }

    fn render_doc(&mut self, doc: &Doc, mode: Mode) -> Result<(), RenderError> {
        match doc.kind() {
            DocKind::Nil | DocKind::BreakParent => Ok(()),
            DocKind::Text(text) => {
                self.write_text(&text.text, text.width);
                Ok(())
            }
            DocKind::LiteralText(text) => {
                self.write_literal(text);
                Ok(())
            }
            DocKind::Concat(docs) => {
                for doc in docs {
                    self.render_doc(doc, mode)?;
                }
                Ok(())
            }
            DocKind::Group(group) => self.render_group(group),
            DocKind::Fill(entries) => self.render_fill(entries, mode),
            DocKind::Indent(indent) => {
                self.indent_levels += i32::from(indent.levels);
                let result = self.render_doc(&indent.contents, mode);
                self.indent_levels -= i32::from(indent.levels);
                result
            }
            DocKind::Align(align) => {
                self.align_spaces += u32::from(align.spaces);
                let result = self.render_doc(&align.contents, mode);
                self.align_spaces -= u32::from(align.spaces);
                result
            }
            DocKind::Line(line) => self.render_line(line, mode),
            DocKind::IfBreak(if_break) => self.render_if_break(if_break, mode),
            DocKind::IndentIfBreak(indent_if_break) => {
                self.render_indent_if_break(indent_if_break, mode)
            }
            DocKind::IndentIfLevelBreak(indent_if_level_break) => {
                self.render_indent_if_level_break(indent_if_level_break, mode)
            }
            DocKind::TrailingFlatWidth(trailing) => {
                let saved = self.trailing_flat_width;
                self.trailing_flat_width = add_width(self.trailing_flat_width, trailing.width);
                let result = self.render_doc(&trailing.contents, mode);
                self.trailing_flat_width = saved;
                result
            }
            DocKind::LineSuffix(doc) => {
                self.stats.line_suffix_count += 1;
                self.line_suffixes.push((**doc).clone());
                Ok(())
            }
            DocKind::LineSuffixBoundary => self.flush_line_suffixes(),
            DocKind::BestFitting(docs) => self.render_best_fitting(docs),
            DocKind::BreakLevel(level) => self.render_break_level(level),
        }
    }

    fn render_break_level(&mut self, level: &BreakLevel) -> Result<(), RenderError> {
        self.stats.break_level_count += 1;
        let computer = self.flat_width_computer();
        let level_width = computer.break_level_flat_width(level)?;
        let saved_tags = self.level_break_tags.clone();
        // Each level decides flat vs broken from the current column, independent of
        // ancestor break mode (GJF `Level.computeBreaks` oneLine path).
        let result = if level_width.fits_on_line_from(
            self.column,
            self.trailing_flat_width,
            self.options.line_width,
        ) {
            self.render_break_level_flat(level)
        } else {
            self.render_break_level_broken(level, &computer)
        };
        self.level_break_tags = saved_tags;
        result
    }

    fn render_break_level_flat(&mut self, level: &BreakLevel) -> Result<(), RenderError> {
        let Some(first) = level.segments.first() else {
            return Ok(());
        };
        self.render_doc(first, Mode::Flat)?;
        for (break_, segment) in level.breaks.iter().zip(level.segments.iter().skip(1)) {
            self.record_level_break_tag(break_, false);
            self.write_flat_line(&break_.flat);
            self.render_doc(segment, Mode::Flat)?;
        }
        Ok(())
    }

    fn render_break_level_broken(
        &mut self,
        level: &BreakLevel,
        computer: &FlatWidthComputer,
    ) -> Result<(), RenderError> {
        self.indent_levels += i32::from(level.plus_indent);
        let result = self.render_break_level_broken_with_indent(level, computer);
        self.indent_levels -= i32::from(level.plus_indent);
        result
    }

    fn render_break_level_broken_with_indent(
        &mut self,
        level: &BreakLevel,
        computer: &FlatWidthComputer,
    ) -> Result<(), RenderError> {
        let Some(first) = level.segments.first() else {
            return Ok(());
        };
        self.render_doc(first, Mode::Break)?;
        let mut must_break = false;
        for (break_, segment) in level.breaks.iter().zip(level.segments.iter().skip(1)) {
            let segment_width = computer.flat_width(segment)?.width;
            let should_break = level_break_should_break(
                break_,
                must_break,
                self.column,
                add_width(segment_width, self.trailing_flat_width),
                self.options.line_width,
            );
            self.record_level_break_tag(break_, should_break);
            if should_break {
                self.write_newline(break_.indent_delta, 1)?;
                self.render_doc(&break_.broken_prefix, Mode::Break)?;
            } else {
                self.write_flat_line(&break_.flat);
            }
            let enough_room = fits_at_column(
                self.column,
                add_width(segment_width, self.trailing_flat_width),
                self.options.line_width,
            );
            self.render_doc(segment, Mode::Break)?;
            if !enough_room {
                must_break = true;
            }
        }
        Ok(())
    }

    fn render_group(&mut self, group: &Group) -> Result<(), RenderError> {
        let is_broken = if group.should_break {
            true
        } else {
            !self.group_fits(group)?
        };
        self.stats.group_count += 1;
        if is_broken {
            self.stats.expanded_group_count += 1;
        }
        if let Some(id) = group.id {
            self.group_modes.insert(id, is_broken);
        }
        self.group_stack.push(GroupFrame {
            id: group.id,
            is_broken,
        });
        let result = self.render_doc(
            &group.contents,
            if is_broken { Mode::Break } else { Mode::Flat },
        );
        self.group_stack.pop();
        result
    }

    fn group_fits(&self, group: &Group) -> Result<bool, RenderError> {
        if matches!(group.fit, GroupFit::LineWidth) && self.line_suffixes.is_empty() {
            let width = self.flat_width_computer().flat_width(&group.contents)?;
            if !width.fits_on_line_from(
                self.column,
                self.trailing_flat_width,
                self.options.line_width,
            ) {
                return Ok(false);
            }
        }

        let mut checker = FitChecker::from_renderer(self)?;
        let result = checker.fits_doc(&group.contents, Mode::Flat)?;
        if !result.fits {
            return Ok(false);
        }
        match group.fit {
            GroupFit::LineWidth => Ok(true),
            GroupFit::MarkedBreak {
                marker,
                max_column_before_last_marked_break,
            } => result
                .marker_columns
                .get(&marker)
                .and_then(|columns| columns.last().copied())
                .map_or(Err(RenderError::MissingBreakMarker(marker)), |column| {
                    Ok(column <= max_column_before_last_marked_break)
                }),
        }
    }

    fn render_fill(&mut self, entries: &[FillEntry], mode: Mode) -> Result<(), RenderError> {
        for (index, entry) in entries.iter().enumerate() {
            self.render_doc(&entry.content, mode)?;
            let Some(separator) = &entry.separator else {
                continue;
            };
            let separator_mode = self.fill_pair_separator_mode(entries, index, separator)?;
            self.render_doc(separator, separator_mode)?;
        }
        Ok(())
    }

    fn fill_pair_fits(&self, separator: &Doc, next_content: &Doc) -> Result<bool, RenderError> {
        let mut checker = FitChecker::from_renderer(self)?;
        let docs = [separator, next_content];
        for doc in docs {
            if !checker.fits_doc(doc, Mode::Flat)?.fits {
                return Ok(false);
            }
        }
        Ok(checker.column <= self.options.line_width)
    }

    fn render_line(&mut self, line: &Line, mode: Mode) -> Result<(), RenderError> {
        if line.marker.is_some() {
            // Markers affect only fitting. Rendering still follows the line mode.
        }
        match (mode, line.mode) {
            (_, LineMode::Hard) => self.write_newline(line.indent_delta, 1),
            (_, LineMode::Empty) => self.write_newline(line.indent_delta, 2),
            (Mode::Flat, LineMode::Soft | LineMode::SoftOrSpace) => {
                self.write_flat_line(&line.flat);
                Ok(())
            }
            (Mode::Break, LineMode::Soft | LineMode::SoftOrSpace) => {
                self.write_newline(line.indent_delta, 1)
            }
        }
    }

    fn render_if_break(&mut self, if_break: &IfBreak, mode: Mode) -> Result<(), RenderError> {
        let is_broken = self.group_break_state(if_break.group_id)?;
        if is_broken {
            self.render_doc(&if_break.breaks, mode)
        } else {
            self.render_doc(&if_break.flat, mode)
        }
    }

    fn render_indent_if_break(
        &mut self,
        indent_if_break: &IndentIfBreak,
        mode: Mode,
    ) -> Result<(), RenderError> {
        let is_broken = self.group_break_state(Some(indent_if_break.group_id))?;
        let should_indent = is_broken != indent_if_break.negate;
        if should_indent {
            self.indent_levels += 1;
        }
        let result = self.render_doc(&indent_if_break.contents, mode);
        if should_indent {
            self.indent_levels -= 1;
        }
        result
    }

    fn render_indent_if_level_break(
        &mut self,
        indent_if_level_break: &IndentIfLevelBreak,
        mode: Mode,
    ) -> Result<(), RenderError> {
        let is_broken = self
            .level_break_tags
            .get(&indent_if_level_break.tag)
            .copied()
            .unwrap_or(false);
        let levels = if is_broken {
            indent_if_level_break.if_broken_levels
        } else {
            indent_if_level_break.if_flat_levels
        };
        self.indent_levels += i32::from(levels);
        let result = self.render_doc(&indent_if_level_break.contents, mode);
        self.indent_levels -= i32::from(levels);
        result
    }

    fn record_level_break_tag(&mut self, break_: &LevelBreak, is_broken: bool) {
        if let Some(tag) = break_.tag {
            self.level_break_tags.insert(tag, is_broken);
        }
    }

    fn render_best_fitting(&mut self, docs: &[Doc]) -> Result<(), RenderError> {
        for doc in docs.iter().take(docs.len().saturating_sub(1)) {
            if self.doc_fits(doc)? {
                return self.render_doc(doc, Mode::Flat);
            }
        }
        let fallback = docs.last().ok_or(RenderError::EmptyBestFitting)?;
        self.render_doc(fallback, Mode::Break)
    }

    fn doc_fits(&self, doc: &Doc) -> Result<bool, RenderError> {
        let mut checker = FitChecker::from_renderer(self)?;
        checker.fits_doc(doc, Mode::Flat).map(|result| result.fits)
    }

    fn group_break_state(&self, group_id: Option<GroupId>) -> Result<bool, RenderError> {
        if let Some(group_id) = group_id {
            self.group_stack
                .iter()
                .rev()
                .find(|frame| frame.id == Some(group_id))
                .map(|frame| frame.is_broken)
                .or_else(|| self.group_modes.get(&group_id).copied())
                .ok_or(RenderError::UnknownGroupId(group_id))
        } else {
            self.group_stack
                .last()
                .map(|frame| frame.is_broken)
                .ok_or(RenderError::NoCurrentGroup)
        }
    }

    fn write_text(&mut self, text: &str, width: TextWidth) {
        self.output.push_str(text);
        self.add_width(width);
    }

    fn write_literal(&mut self, literal: &LiteralText) {
        self.output.push_str(&literal.text);
        let mut line_base_column = self.column;
        let mut line_index = 0;
        let mut chars = literal.text.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '\r' => {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    let width = literal.line_widths[line_index];
                    self.max_column = self.max_column.max(add_width(line_base_column, width));
                    line_base_column = TextWidth::ZERO;
                    line_index += 1;
                }
                '\n' => {
                    let width = literal.line_widths[line_index];
                    self.max_column = self.max_column.max(add_width(line_base_column, width));
                    line_base_column = TextWidth::ZERO;
                    line_index += 1;
                }
                _ => {}
            }
        }
        let final_width = literal.line_widths[line_index];
        if line_index == 0 {
            self.add_width(final_width);
        } else {
            self.line += u32::try_from(line_index).expect("literal line count fits u32");
            self.column = final_width;
            self.max_column = self.max_column.max(self.column);
        }
    }

    fn write_flat_line(&mut self, flat: &FlatLine) {
        match flat {
            FlatLine::Empty => {}
            FlatLine::Space => self.write_text(" ", TextWidth::new(1)),
            FlatLine::Text(text, width) => self.write_text(text, *width),
        }
    }

    fn write_newline(&mut self, indent_delta: i16, count: u32) -> Result<(), RenderError> {
        self.flush_line_suffixes()?;
        for _ in 0..count {
            self.max_column = self.max_column.max(self.column);
            self.output.push_str(self.options.line_ending.as_str());
            self.line += 1;
            self.column = TextWidth::ZERO;
        }
        self.write_indent(indent_delta);
        Ok(())
    }

    fn write_indent(&mut self, indent_delta: i16) {
        let effective_levels = (self.indent_levels + i32::from(indent_delta))
            .max(0)
            .cast_unsigned();
        match self.options.indent_style {
            IndentStyle::Space => {
                let width = effective_levels * u32::from(self.options.indent_width);
                push_repeated(&mut self.output, ' ', width);
                self.column = TextWidth::new(width);
            }
            IndentStyle::Tab => {
                push_repeated(&mut self.output, '\t', effective_levels);
                self.column =
                    TextWidth::new(effective_levels * u32::from(self.options.indent_width));
            }
        }
        push_repeated(&mut self.output, ' ', self.align_spaces);
        self.column = add_width(self.column, TextWidth::new(self.align_spaces));
        self.max_column = self.max_column.max(self.column);
    }

    fn flush_line_suffixes(&mut self) -> Result<(), RenderError> {
        if self.flushing_suffixes || self.line_suffixes.is_empty() {
            return Ok(());
        }
        self.flushing_suffixes = true;
        let result = (|| {
            while !self.line_suffixes.is_empty() {
                let suffixes = std::mem::take(&mut self.line_suffixes);
                for suffix in suffixes {
                    self.render_doc(&suffix, Mode::Flat)?;
                }
            }
            Ok(())
        })();
        self.flushing_suffixes = false;
        result
    }

    fn fill_pair_separator_mode(
        &self,
        entries: &[FillEntry],
        index: usize,
        separator: &Doc,
    ) -> Result<Mode, RenderError> {
        let Some(next) = entries.get(index + 1) else {
            return Ok(Mode::Break);
        };
        if self.fill_pair_fits(separator, &next.content)? {
            Ok(Mode::Flat)
        } else {
            Ok(Mode::Break)
        }
    }

    fn add_width(&mut self, width: TextWidth) {
        self.column = add_width(self.column, width);
        self.max_column = self.max_column.max(self.column);
    }
}

impl LineEnding {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FlatWidthSummary {
    width: TextWidth,
    fits: bool,
}

impl FlatWidthSummary {
    const fn finite(width: TextWidth) -> Self {
        Self { width, fits: true }
    }

    const fn infinite() -> Self {
        Self {
            width: TextWidth::ZERO,
            fits: false,
        }
    }

    fn fits_on_line_from(
        self,
        column: TextWidth,
        trailing_width: TextWidth,
        line_width: TextWidth,
    ) -> bool {
        self.fits && fits_at_column(column, add_width(self.width, trailing_width), line_width)
    }
}

fn add_flat_width(lhs: TextWidth, rhs: TextWidth) -> TextWidth {
    let width = add_width(lhs, rhs);
    if width > MAX_FLAT_WIDTH {
        MAX_FLAT_WIDTH
    } else {
        width
    }
}

fn flat_line_width(flat: &FlatLine) -> TextWidth {
    match flat {
        FlatLine::Empty => TextWidth::ZERO,
        FlatLine::Space => TextWidth::new(1),
        FlatLine::Text(_, width) => *width,
    }
}

struct FlatWidthComputer {
    cache: Rc<RefCell<HashMap<*const DocKind, FlatWidthSummary>>>,
    cache_hits: Rc<RefCell<u32>>,
}

impl FlatWidthComputer {
    fn new(
        cache: Rc<RefCell<HashMap<*const DocKind, FlatWidthSummary>>>,
        cache_hits: Rc<RefCell<u32>>,
    ) -> Self {
        Self { cache, cache_hits }
    }

    fn flat_width(&self, doc: &Doc) -> Result<FlatWidthSummary, RenderError> {
        let ptr = doc.cache_ptr();
        if let Some(cached) = self.cache.borrow().get(&ptr).copied() {
            *self.cache_hits.borrow_mut() += 1;
            return Ok(cached);
        }
        let summary = self.compute_flat_width(doc)?;
        self.cache.borrow_mut().insert(ptr, summary);
        Ok(summary)
    }

    fn break_level_flat_width(&self, level: &BreakLevel) -> Result<FlatWidthSummary, RenderError> {
        if level
            .breaks
            .iter()
            .any(|break_| matches!(break_.mode, LevelBreakMode::Forced))
        {
            return Ok(FlatWidthSummary::infinite());
        }
        let mut width = TextWidth::ZERO;
        for (index, segment) in level.segments.iter().enumerate() {
            if index > 0 {
                width = add_flat_width(width, flat_line_width(&level.breaks[index - 1].flat));
            }
            let segment_width = self.flat_width(segment)?;
            if !segment_width.fits {
                return Ok(FlatWidthSummary::infinite());
            }
            width = add_flat_width(width, segment_width.width);
            if width >= MAX_FLAT_WIDTH {
                return Ok(FlatWidthSummary::finite(MAX_FLAT_WIDTH));
            }
        }
        Ok(FlatWidthSummary::finite(width))
    }

    fn compute_flat_width(&self, doc: &Doc) -> Result<FlatWidthSummary, RenderError> {
        match doc.kind() {
            DocKind::Nil | DocKind::LineSuffixBoundary => {
                Ok(FlatWidthSummary::finite(TextWidth::ZERO))
            }
            DocKind::BreakParent => Ok(FlatWidthSummary::infinite()),
            DocKind::Text(text) => Ok(FlatWidthSummary::finite(text.width)),
            DocKind::LiteralText(text) => {
                if has_line_terminator(&text.text) {
                    Ok(FlatWidthSummary::infinite())
                } else {
                    Ok(FlatWidthSummary::finite(text.final_width()))
                }
            }
            DocKind::Concat(docs) => self.sum_flat_widths(docs),
            DocKind::Group(group) => {
                if group.should_break {
                    Ok(FlatWidthSummary::infinite())
                } else {
                    self.flat_width(&group.contents)
                }
            }
            DocKind::Fill(entries) => {
                let mut width = TextWidth::ZERO;
                for entry in entries {
                    let content = self.flat_width(&entry.content)?;
                    if !content.fits {
                        return Ok(FlatWidthSummary::infinite());
                    }
                    width = add_flat_width(width, content.width);
                    if width >= MAX_FLAT_WIDTH {
                        return Ok(FlatWidthSummary::finite(MAX_FLAT_WIDTH));
                    }
                    if let Some(separator) = &entry.separator {
                        let separator_width = self.flat_width(separator)?;
                        if !separator_width.fits {
                            return Ok(FlatWidthSummary::infinite());
                        }
                        width = add_flat_width(width, separator_width.width);
                        if width >= MAX_FLAT_WIDTH {
                            return Ok(FlatWidthSummary::finite(MAX_FLAT_WIDTH));
                        }
                    }
                }
                Ok(FlatWidthSummary::finite(width))
            }
            DocKind::Indent(indent) => self.flat_width(&indent.contents),
            DocKind::Align(align) => {
                let inner = self.flat_width(&align.contents)?;
                if !inner.fits {
                    return Ok(FlatWidthSummary::infinite());
                }
                Ok(FlatWidthSummary::finite(add_flat_width(
                    inner.width,
                    TextWidth::new(u32::from(align.spaces)),
                )))
            }
            DocKind::Line(line) => Ok(FlatWidthSummary::finite(flat_line_width(&line.flat))),
            DocKind::IfBreak(if_break) => self.flat_width(&if_break.flat),
            DocKind::IndentIfBreak(indent_if_break) => self.flat_width(&indent_if_break.contents),
            DocKind::IndentIfLevelBreak(indent_if_level_break) => {
                self.flat_width(&indent_if_level_break.contents)
            }
            DocKind::TrailingFlatWidth(trailing) => self.flat_width(&trailing.contents),
            DocKind::LineSuffix(doc) => self.flat_width(doc),
            DocKind::BestFitting(docs) => {
                let Some((fallback, candidates)) = docs.split_last() else {
                    return Err(RenderError::EmptyBestFitting);
                };
                for candidate in candidates {
                    let width = self.flat_width(candidate)?;
                    if width.fits {
                        return Ok(width);
                    }
                }
                self.flat_width(fallback)
            }
            DocKind::BreakLevel(level) => self.break_level_flat_width(level),
        }
    }

    fn sum_flat_widths(&self, docs: &[Doc]) -> Result<FlatWidthSummary, RenderError> {
        let mut width = TextWidth::ZERO;
        for doc in docs {
            let doc_width = self.flat_width(doc)?;
            if !doc_width.fits {
                return Ok(FlatWidthSummary::infinite());
            }
            width = add_flat_width(width, doc_width.width);
            if width >= MAX_FLAT_WIDTH {
                return Ok(FlatWidthSummary::finite(MAX_FLAT_WIDTH));
            }
        }
        Ok(FlatWidthSummary::finite(width))
    }
}

#[derive(Clone, Debug)]
struct FitResult {
    fits: bool,
    marker_columns: BTreeMap<BreakMarkerId, Vec<TextWidth>>,
}

impl FitResult {
    fn yes(marker_columns: BTreeMap<BreakMarkerId, Vec<TextWidth>>) -> Self {
        Self {
            fits: true,
            marker_columns,
        }
    }

    fn no(marker_columns: BTreeMap<BreakMarkerId, Vec<TextWidth>>) -> Self {
        Self {
            fits: false,
            marker_columns,
        }
    }
}

#[derive(Clone, Debug)]
struct CachedFit {
    fits: bool,
    column: TextWidth,
    indent_levels: i32,
    align_spaces: u32,
    group_modes: BTreeMap<GroupId, bool>,
    group_stack: Vec<GroupFrame>,
    trailing_flat_width: TextWidth,
    marker_additions: BTreeMap<BreakMarkerId, Vec<TextWidth>>,
}

#[derive(Clone, Debug)]
struct FitCacheKey {
    doc_ptr: *const DocKind,
    mode: Mode,
    column: TextWidth,
    indent_levels: i32,
    align_spaces: u32,
    group_modes: BTreeMap<GroupId, bool>,
    group_stack: Vec<GroupFrame>,
    trailing_flat_width: TextWidth,
}

impl PartialEq for FitCacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.doc_ptr == other.doc_ptr
            && self.mode == other.mode
            && self.column == other.column
            && self.indent_levels == other.indent_levels
            && self.align_spaces == other.align_spaces
            && self.group_modes == other.group_modes
            && self.group_stack == other.group_stack
            && self.trailing_flat_width == other.trailing_flat_width
    }
}

impl Eq for FitCacheKey {}

impl Hash for FitCacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.doc_ptr.hash(state);
        self.mode.hash(state);
        self.column.get().hash(state);
        self.indent_levels.hash(state);
        self.align_spaces.hash(state);
        self.group_modes.hash(state);
        self.group_stack.hash(state);
        self.trailing_flat_width.get().hash(state);
    }
}

impl FitCacheKey {
    fn new(doc: &Doc, mode: Mode, checker: &FitChecker) -> Self {
        Self {
            doc_ptr: doc.cache_ptr(),
            mode,
            column: checker.column,
            indent_levels: checker.indent_levels,
            align_spaces: checker.align_spaces,
            group_modes: checker.group_modes.clone(),
            group_stack: checker.group_stack.clone(),
            trailing_flat_width: checker.trailing_flat_width,
        }
    }
}

fn marker_additions(
    before: &BTreeMap<BreakMarkerId, Vec<TextWidth>>,
    after: &BTreeMap<BreakMarkerId, Vec<TextWidth>>,
) -> BTreeMap<BreakMarkerId, Vec<TextWidth>> {
    let mut additions = BTreeMap::new();
    for (id, after_columns) in after {
        let before_len = before.get(id).map_or(0, Vec::len);
        if after_columns.len() > before_len {
            additions.insert(*id, after_columns[before_len..].to_vec());
        }
    }
    additions
}

fn apply_marker_additions(
    marker_columns: &mut BTreeMap<BreakMarkerId, Vec<TextWidth>>,
    additions: &BTreeMap<BreakMarkerId, Vec<TextWidth>>,
) {
    for (id, columns) in additions {
        marker_columns
            .entry(*id)
            .or_default()
            .extend(columns.iter().copied());
    }
}

#[derive(Clone, Debug)]
struct FitChecker {
    options: RenderOptions,
    column: TextWidth,
    indent_levels: i32,
    align_spaces: u32,
    group_modes: BTreeMap<GroupId, bool>,
    group_stack: Vec<GroupFrame>,
    level_break_tags: BTreeMap<LevelBreakTag, bool>,
    trailing_flat_width: TextWidth,
    marker_columns: BTreeMap<BreakMarkerId, Vec<TextWidth>>,
    fit_cache: Rc<RefCell<HashMap<FitCacheKey, CachedFit>>>,
    flat_width_cache: Rc<RefCell<HashMap<*const DocKind, FlatWidthSummary>>>,
    flat_width_cache_hits: Rc<RefCell<u32>>,
    level_break_indent_cache: Rc<RefCell<HashMap<*const DocKind, bool>>>,
}

impl FitChecker {
    fn flat_width_computer(&self) -> FlatWidthComputer {
        FlatWidthComputer::new(
            Rc::clone(&self.flat_width_cache),
            Rc::clone(&self.flat_width_cache_hits),
        )
    }

    fn from_renderer(renderer: &Renderer) -> Result<Self, RenderError> {
        let mut checker = Self {
            options: renderer.options,
            column: renderer.column,
            indent_levels: renderer.indent_levels,
            align_spaces: renderer.align_spaces,
            group_modes: renderer.group_modes.clone(),
            group_stack: renderer.group_stack.clone(),
            level_break_tags: renderer.level_break_tags.clone(),
            trailing_flat_width: renderer.trailing_flat_width,
            marker_columns: BTreeMap::new(),
            fit_cache: Rc::clone(&renderer.fit_cache),
            flat_width_cache: Rc::clone(&renderer.flat_width_cache),
            flat_width_cache_hits: Rc::clone(&renderer.flat_width_cache_hits),
            level_break_indent_cache: Rc::clone(&renderer.level_break_indent_cache),
        };
        for suffix in &renderer.line_suffixes {
            if !checker.fit_doc(suffix, Mode::Flat)? {
                checker.column = add_width(checker.options.line_width, TextWidth::new(1));
                break;
            }
        }
        Ok(checker)
    }

    fn fits_doc(&mut self, doc: &Doc, mode: Mode) -> Result<FitResult, RenderError> {
        let fits = self.fit_doc(doc, mode)? && self.column <= self.options.line_width;
        let marker_columns = self.marker_columns.clone();
        Ok(if fits {
            FitResult::yes(marker_columns)
        } else {
            FitResult::no(marker_columns)
        })
    }

    fn fit_doc(&mut self, doc: &Doc, mode: Mode) -> Result<bool, RenderError> {
        if self.contains_indent_if_level_break(doc) {
            return self.fit_doc_uncached(doc, mode);
        }

        let cache_key = FitCacheKey::new(doc, mode, self);
        let cached = { self.fit_cache.borrow().get(&cache_key).cloned() };
        if let Some(cached) = cached {
            self.restore_cached_fit(&cached);
            return Ok(cached.fits);
        }

        let markers_before = self.marker_columns.clone();
        let fits = self.fit_doc_uncached(doc, mode)?;
        self.fit_cache.borrow_mut().insert(
            cache_key,
            CachedFit {
                fits,
                column: self.column,
                indent_levels: self.indent_levels,
                align_spaces: self.align_spaces,
                group_modes: self.group_modes.clone(),
                group_stack: self.group_stack.clone(),
                trailing_flat_width: self.trailing_flat_width,
                marker_additions: marker_additions(&markers_before, &self.marker_columns),
            },
        );
        Ok(fits)
    }

    fn restore_cached_fit(&mut self, cached: &CachedFit) {
        self.column = cached.column;
        self.indent_levels = cached.indent_levels;
        self.align_spaces = cached.align_spaces;
        self.group_modes = cached.group_modes.clone();
        self.group_stack = cached.group_stack.clone();
        self.trailing_flat_width = cached.trailing_flat_width;
        apply_marker_additions(&mut self.marker_columns, &cached.marker_additions);
    }

    fn contains_indent_if_level_break(&self, doc: &Doc) -> bool {
        contains_indent_if_level_break(doc, &self.level_break_indent_cache)
    }

    fn fit_doc_uncached(&mut self, doc: &Doc, mode: Mode) -> Result<bool, RenderError> {
        match doc.kind() {
            DocKind::Nil | DocKind::LineSuffixBoundary | DocKind::BreakParent => {
                Ok(!matches!(doc.kind(), DocKind::BreakParent))
            }
            DocKind::Text(text) => Ok(self.add_width(text.width)),
            DocKind::LiteralText(text) => {
                if has_line_terminator(&text.text) {
                    Ok(false)
                } else {
                    Ok(self.add_width(text.final_width()))
                }
            }
            DocKind::Concat(docs) => {
                for doc in docs {
                    if !self.fit_doc(doc, mode)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            DocKind::Group(group) => self.fit_group(group),
            DocKind::Fill(entries) => self.fit_fill(entries, mode),
            DocKind::Indent(indent) => {
                self.indent_levels += i32::from(indent.levels);
                let result = self.fit_doc(&indent.contents, mode);
                self.indent_levels -= i32::from(indent.levels);
                result
            }
            DocKind::Align(align) => {
                self.align_spaces += u32::from(align.spaces);
                let result = self.fit_doc(&align.contents, mode);
                self.align_spaces -= u32::from(align.spaces);
                result
            }
            DocKind::Line(line) => Ok(self.fit_line(line, mode)),
            DocKind::IfBreak(if_break) => {
                let is_broken = self.group_break_state(if_break.group_id).unwrap_or(false);
                self.fit_doc(
                    if is_broken {
                        &if_break.breaks
                    } else {
                        &if_break.flat
                    },
                    mode,
                )
            }
            DocKind::IndentIfBreak(indent_if_break) => {
                let is_broken = self
                    .group_break_state(Some(indent_if_break.group_id))
                    .unwrap_or(false);
                let should_indent = is_broken != indent_if_break.negate;
                if should_indent {
                    self.indent_levels += 1;
                }
                let result = self.fit_doc(&indent_if_break.contents, mode);
                if should_indent {
                    self.indent_levels -= 1;
                }
                result
            }
            DocKind::IndentIfLevelBreak(indent_if_level_break) => {
                let is_broken = self
                    .level_break_tags
                    .get(&indent_if_level_break.tag)
                    .copied()
                    .unwrap_or(false);
                let levels = if is_broken {
                    indent_if_level_break.if_broken_levels
                } else {
                    indent_if_level_break.if_flat_levels
                };
                self.indent_levels += i32::from(levels);
                let result = self.fit_doc(&indent_if_level_break.contents, mode);
                self.indent_levels -= i32::from(levels);
                result
            }
            DocKind::TrailingFlatWidth(trailing) => {
                let saved = self.trailing_flat_width;
                self.trailing_flat_width = add_width(self.trailing_flat_width, trailing.width);
                let result = self.fit_doc(&trailing.contents, mode);
                self.trailing_flat_width = saved;
                result
            }
            DocKind::LineSuffix(doc) => self.fit_doc(doc, Mode::Flat),
            DocKind::BestFitting(docs) => self.fit_best_fitting(docs),
            DocKind::BreakLevel(level) => self.fit_break_level(level, mode),
        }
    }

    fn fit_break_level(&mut self, level: &BreakLevel, mode: Mode) -> Result<bool, RenderError> {
        let computer = self.flat_width_computer();
        let level_width = computer.break_level_flat_width(level)?;
        let saved_tags = self.level_break_tags.clone();
        // Match `render_break_level`: every level makes its own one-line fit decision at
        // the current column, even when an ancestor is already in break mode.
        let result = if level_width.fits_on_line_from(
            self.column,
            self.trailing_flat_width,
            self.options.line_width,
        ) {
            self.fit_break_level_flat(level)
        } else {
            match mode {
                Mode::Flat => Ok(false),
                Mode::Break => self.fit_break_level_broken(level, &computer),
            }
        };
        self.level_break_tags = saved_tags;
        result
    }

    fn fit_break_level_flat(&mut self, level: &BreakLevel) -> Result<bool, RenderError> {
        let Some(first) = level.segments.first() else {
            return Ok(true);
        };
        if !self.fit_doc(first, Mode::Flat)? {
            return Ok(false);
        }
        for (break_, segment) in level.breaks.iter().zip(level.segments.iter().skip(1)) {
            self.record_level_break_tag(break_, false);
            if !self.fit_flat_line(&break_.flat) {
                return Ok(false);
            }
            if !self.fit_doc(segment, Mode::Flat)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn fit_break_level_broken(
        &mut self,
        level: &BreakLevel,
        computer: &FlatWidthComputer,
    ) -> Result<bool, RenderError> {
        self.indent_levels += i32::from(level.plus_indent);
        let result = self.fit_break_level_broken_with_indent(level, computer);
        self.indent_levels -= i32::from(level.plus_indent);
        result
    }

    fn fit_break_level_broken_with_indent(
        &mut self,
        level: &BreakLevel,
        computer: &FlatWidthComputer,
    ) -> Result<bool, RenderError> {
        let Some(first) = level.segments.first() else {
            return Ok(true);
        };
        if !self.fit_doc(first, Mode::Break)? {
            return Ok(false);
        }
        let mut must_break = false;
        for (break_, segment) in level.breaks.iter().zip(level.segments.iter().skip(1)) {
            let segment_width = computer.flat_width(segment)?.width;
            let should_break = level_break_should_break(
                break_,
                must_break,
                self.column,
                add_width(segment_width, self.trailing_flat_width),
                self.options.line_width,
            );
            self.record_level_break_tag(break_, should_break);
            if should_break {
                if !self.fit_newline(break_.indent_delta) {
                    return Ok(false);
                }
                if !self.fit_doc(&break_.broken_prefix, Mode::Break)? {
                    return Ok(false);
                }
            } else if !self.fit_flat_line(&break_.flat) {
                return Ok(false);
            }
            let enough_room = fits_at_column(
                self.column,
                add_width(segment_width, self.trailing_flat_width),
                self.options.line_width,
            );
            if !self.fit_doc(segment, Mode::Break)? {
                return Ok(false);
            }
            if !enough_room {
                must_break = true;
            }
        }
        Ok(true)
    }

    fn record_level_break_tag(&mut self, break_: &LevelBreak, is_broken: bool) {
        if let Some(tag) = break_.tag {
            self.level_break_tags.insert(tag, is_broken);
        }
    }

    fn fit_newline(&mut self, indent_delta: i16) -> bool {
        self.column = TextWidth::ZERO;
        let effective_levels = (self.indent_levels + i32::from(indent_delta))
            .max(0)
            .cast_unsigned();
        self.column = TextWidth::new(
            effective_levels * u32::from(self.options.indent_width) + self.align_spaces,
        );
        self.column <= self.options.line_width
    }

    fn fit_group(&mut self, group: &Group) -> Result<bool, RenderError> {
        if group.should_break {
            return Ok(false);
        }
        if matches!(group.fit, GroupFit::LineWidth) && self.marker_columns.is_empty() {
            let width = self.flat_width_computer().flat_width(&group.contents)?;
            if !width.fits_on_line_from(
                self.column,
                self.trailing_flat_width,
                self.options.line_width,
            ) {
                return Ok(false);
            }
        }
        let mut nested = self.clone();
        let nested_result = nested.fits_doc(&group.contents, Mode::Flat)?;
        if !nested_result.fits {
            return Ok(false);
        }
        match group.fit {
            GroupFit::LineWidth => {
                *self = nested;
                if let Some(id) = group.id {
                    self.group_modes.insert(id, false);
                }
                Ok(true)
            }
            GroupFit::MarkedBreak {
                marker,
                max_column_before_last_marked_break,
            } => {
                let last_marker_column = nested_result
                    .marker_columns
                    .get(&marker)
                    .and_then(|columns| columns.last().copied())
                    .ok_or(RenderError::MissingBreakMarker(marker))?;
                if last_marker_column <= max_column_before_last_marked_break {
                    *self = nested;
                    if let Some(id) = group.id {
                        self.group_modes.insert(id, false);
                    }
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }
    }

    fn fit_fill(&mut self, entries: &[FillEntry], mode: Mode) -> Result<bool, RenderError> {
        for (index, entry) in entries.iter().enumerate() {
            if !self.fit_doc(&entry.content, mode)? {
                return Ok(false);
            }
            let Some(separator) = &entry.separator else {
                continue;
            };
            let mut pair = self.clone();
            let next_fits = if let Some(next) = entries.get(index + 1) {
                pair.fit_doc(separator, Mode::Flat)?
                    && pair.fit_doc(&next.content, Mode::Flat)?
                    && pair.column <= pair.options.line_width
            } else {
                false
            };
            if next_fits {
                *self = pair;
            } else if !self.fit_doc(separator, Mode::Break)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn fit_line(&mut self, line: &Line, mode: Mode) -> bool {
        if let Some(marker) = line.marker {
            self.marker_columns
                .entry(marker)
                .or_default()
                .push(self.column);
        }
        match (mode, line.mode) {
            (Mode::Flat, LineMode::Soft | LineMode::SoftOrSpace) => self.fit_flat_line(&line.flat),
            (_, LineMode::Hard | LineMode::Empty) if !line.propagate_break => true,
            (_, LineMode::Hard | LineMode::Empty)
            | (Mode::Break, LineMode::Soft | LineMode::SoftOrSpace) => false,
        }
    }

    fn fit_flat_line(&mut self, flat: &FlatLine) -> bool {
        match flat {
            FlatLine::Empty => true,
            FlatLine::Space => self.add_width(TextWidth::new(1)),
            FlatLine::Text(_, width) => self.add_width(*width),
        }
    }

    fn fit_best_fitting(&mut self, docs: &[Doc]) -> Result<bool, RenderError> {
        let Some((fallback, candidates)) = docs.split_last() else {
            return Err(RenderError::EmptyBestFitting);
        };
        for doc in candidates {
            let mut candidate = self.clone();
            if candidate.fit_doc(doc, Mode::Flat)?
                && candidate.column <= candidate.options.line_width
            {
                *self = candidate;
                return Ok(true);
            }
        }
        self.fit_doc(fallback, Mode::Break)
    }

    fn group_break_state(&self, group_id: Option<GroupId>) -> Result<bool, RenderError> {
        if let Some(group_id) = group_id {
            self.group_stack
                .iter()
                .rev()
                .find(|frame| frame.id == Some(group_id))
                .map(|frame| frame.is_broken)
                .or_else(|| self.group_modes.get(&group_id).copied())
                .ok_or(RenderError::UnknownGroupId(group_id))
        } else {
            self.group_stack
                .last()
                .map(|frame| frame.is_broken)
                .ok_or(RenderError::NoCurrentGroup)
        }
    }

    fn add_width(&mut self, width: TextWidth) -> bool {
        self.column = add_width(self.column, width);
        self.column <= self.options.line_width
    }
}

fn contains_indent_if_level_break(
    doc: &Doc,
    cache: &Rc<RefCell<HashMap<*const DocKind, bool>>>,
) -> bool {
    let ptr = doc.cache_ptr();
    if let Some(cached) = cache.borrow().get(&ptr).copied() {
        return cached;
    }

    let contains = match doc.kind() {
        DocKind::IndentIfLevelBreak(_) => true,
        DocKind::Concat(docs) => docs
            .iter()
            .any(|doc| contains_indent_if_level_break(doc, cache)),
        DocKind::Group(group) => contains_indent_if_level_break(&group.contents, cache),
        DocKind::Fill(entries) => entries
            .iter()
            .any(|entry| contains_indent_if_level_break(&entry.content, cache)),
        DocKind::Indent(indent) => contains_indent_if_level_break(&indent.contents, cache),
        DocKind::Align(align) => contains_indent_if_level_break(&align.contents, cache),
        DocKind::IfBreak(if_break) => {
            contains_indent_if_level_break(&if_break.breaks, cache)
                || contains_indent_if_level_break(&if_break.flat, cache)
        }
        DocKind::IndentIfBreak(indent_if_break) => {
            contains_indent_if_level_break(&indent_if_break.contents, cache)
        }
        DocKind::TrailingFlatWidth(trailing) => {
            contains_indent_if_level_break(&trailing.contents, cache)
        }
        DocKind::LineSuffix(doc) => contains_indent_if_level_break(doc, cache),
        DocKind::BestFitting(docs) => docs
            .iter()
            .any(|doc| contains_indent_if_level_break(doc, cache)),
        DocKind::BreakLevel(level) => {
            level
                .segments
                .iter()
                .any(|doc| contains_indent_if_level_break(doc, cache))
                || level
                    .breaks
                    .iter()
                    .any(|break_| contains_indent_if_level_break(&break_.broken_prefix, cache))
        }
        DocKind::Nil
        | DocKind::Text(_)
        | DocKind::LiteralText(_)
        | DocKind::Line(_)
        | DocKind::LineSuffixBoundary
        | DocKind::BreakParent => false,
    };
    cache.borrow_mut().insert(ptr, contains);
    contains
}
