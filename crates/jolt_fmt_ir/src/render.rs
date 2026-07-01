use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::document::{
    Doc, DocKind, FillEntry, FlatLine, Group, GroupId, IfBreak, IndentIfBreak, Line, LineMode,
    LiteralText,
};
use crate::validation::validate_doc;
use crate::width::{TextWidth, add_width, has_line_terminator, push_repeated};

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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RenderError {
    InvalidText { context: &'static str },
    InvalidLiteralWidths { expected: usize, actual: usize },
    InvalidLineSuffix { reason: &'static str },
    MalformedFill { index: usize, reason: &'static str },
    UnknownGroupId(GroupId),
    NoCurrentGroup,
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
            Self::MalformedFill { index, reason } => {
                write!(formatter, "malformed fill entry at index {index}: {reason}")
            }
            Self::UnknownGroupId(id) => write!(formatter, "unknown group id {}", id.0),
            Self::NoCurrentGroup => formatter.write_str("if_break requires a current group"),
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Mode {
    Flat,
    Break,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
    align_spaces: i32,
    group_modes: BTreeMap<GroupId, bool>,
    group_stack: Vec<GroupFrame>,
    line_suffixes: Vec<Doc>,
    flushing_suffixes: bool,
    stats: RenderStats,
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
            line_suffixes: Vec::new(),
            flushing_suffixes: false,
            stats: RenderStats::default(),
        }
    }

    fn finish(mut self) -> Rendered {
        self.stats.line_count = self.line;
        self.stats.max_column = self.max_column.max(self.column);
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
                self.align_spaces += i32::from(align.spaces);
                let result = self.render_doc(&align.contents, mode);
                self.align_spaces -= i32::from(align.spaces);
                result
            }
            DocKind::Line(line) => self.render_line(line, mode),
            DocKind::IfBreak(if_break) => self.render_if_break(if_break, mode),
            DocKind::IndentIfBreak(indent_if_break) => {
                self.render_indent_if_break(indent_if_break, mode)
            }
            DocKind::LineSuffix(doc) => {
                self.stats.line_suffix_count += 1;
                self.line_suffixes.push((**doc).clone());
                Ok(())
            }
            DocKind::LineSuffixBoundary => self.flush_line_suffixes(),
        }
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
        let mut checker = FitChecker::from_renderer(self)?;
        checker.fits_doc(&group.contents, Mode::Flat)
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
            if !checker.fits_doc(doc, Mode::Flat)? {
                return Ok(false);
            }
        }
        Ok(checker.column <= self.options.line_width)
    }

    fn render_line(&mut self, line: &Line, mode: Mode) -> Result<(), RenderError> {
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
        let base_width = effective_levels * u32::from(self.options.indent_width);
        match self.options.indent_style {
            IndentStyle::Space => {
                let width = (i64::from(base_width) + i64::from(self.align_spaces)).max(0) as u32;
                push_repeated(&mut self.output, ' ', width);
                self.column = TextWidth::new(width);
            }
            IndentStyle::Tab => {
                if self.align_spaces >= 0 {
                    push_repeated(&mut self.output, '\t', effective_levels);
                    let spaces = self.align_spaces.cast_unsigned();
                    push_repeated(&mut self.output, ' ', spaces);
                    self.column = TextWidth::new(base_width + spaces);
                } else {
                    let width =
                        (i64::from(base_width) + i64::from(self.align_spaces)).max(0) as u32;
                    let tab_width = u32::from(self.options.indent_width);
                    let tabs = width / tab_width;
                    let spaces = width % tab_width;
                    push_repeated(&mut self.output, '\t', tabs);
                    push_repeated(&mut self.output, ' ', spaces);
                    self.column = TextWidth::new(width);
                }
            }
        }
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

#[derive(Clone, Debug)]
struct FitChecker {
    options: RenderOptions,
    column: TextWidth,
    indent_levels: i32,
    align_spaces: i32,
    group_modes: BTreeMap<GroupId, bool>,
    group_stack: Vec<GroupFrame>,
}

impl FitChecker {
    fn from_renderer(renderer: &Renderer) -> Result<Self, RenderError> {
        let mut checker = Self {
            options: renderer.options,
            column: renderer.column,
            indent_levels: renderer.indent_levels,
            align_spaces: renderer.align_spaces,
            group_modes: renderer.group_modes.clone(),
            group_stack: renderer.group_stack.clone(),
        };
        for suffix in &renderer.line_suffixes {
            if !checker.fit_doc(suffix, Mode::Flat)? {
                checker.column = add_width(checker.options.line_width, TextWidth::new(1));
                break;
            }
        }
        Ok(checker)
    }

    fn fits_doc(&mut self, doc: &Doc, mode: Mode) -> Result<bool, RenderError> {
        Ok(self.fit_doc(doc, mode)? && self.column <= self.options.line_width)
    }

    fn fit_doc(&mut self, doc: &Doc, mode: Mode) -> Result<bool, RenderError> {
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
                self.align_spaces += i32::from(align.spaces);
                let result = self.fit_doc(&align.contents, mode);
                self.align_spaces -= i32::from(align.spaces);
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
            DocKind::LineSuffix(doc) => self.fit_doc(doc, Mode::Flat),
        }
    }

    fn fit_group(&mut self, group: &Group) -> Result<bool, RenderError> {
        if group.should_break {
            return Ok(false);
        }
        let mut nested = self.clone();
        if !nested.fits_doc(&group.contents, Mode::Flat)? {
            return Ok(false);
        }
        *self = nested;
        if let Some(id) = group.id {
            self.group_modes.insert(id, false);
        }
        Ok(true)
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
