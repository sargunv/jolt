use std::collections::BTreeMap;
use std::convert::Infallible;
use std::error::Error;
use std::fmt;

use crate::document::{
    Doc, DocKind, FillEntry, FlatLine, Group, GroupId, Line, LineMode, LiteralText,
};
use crate::validation::validate_doc;
use crate::width::{TextWidth, add_width, has_line_terminator};

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
pub struct RenderOutcome {
    pub stats: RenderStats,
    pub halted: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderControl {
    Continue,
    Halt,
}

pub trait RenderSink {
    type Error;

    /// Writes a rendered text chunk and returns whether rendering should continue.
    ///
    /// # Errors
    ///
    /// Returns a sink-specific error when the chunk cannot be accepted.
    fn write_str(&mut self, text: &str) -> Result<RenderControl, Self::Error>;
}

impl<T: RenderSink + ?Sized> RenderSink for &mut T {
    type Error = T::Error;

    fn write_str(&mut self, text: &str) -> Result<RenderControl, Self::Error> {
        (**self).write_str(text)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct StringSink {
    text: String,
}

impl StringSink {
    fn into_string(self) -> String {
        self.text
    }
}

impl RenderSink for StringSink {
    type Error = Infallible;

    fn write_str(&mut self, text: &str) -> Result<RenderControl, Self::Error> {
        self.text.push_str(text);
        Ok(RenderControl::Continue)
    }
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RenderToError<E> {
    Render(RenderError),
    Sink(E),
}

impl<E: fmt::Display> fmt::Display for RenderToError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Render(error) => error.fmt(formatter),
            Self::Sink(error) => error.fmt(formatter),
        }
    }
}

impl<E: fmt::Debug + fmt::Display> Error for RenderToError<E> {}

/// Renders a document using the provided options.
///
/// # Errors
///
/// Returns [`RenderError`] when the document is structurally invalid or contains
/// invalid non-literal text.
pub fn render_to<S: RenderSink>(
    doc: &Doc,
    options: RenderOptions,
    sink: S,
) -> Result<RenderOutcome, RenderToError<S::Error>> {
    validate_doc(doc).map_err(RenderToError::Render)?;
    let mut renderer = Renderer::new(options, sink);
    renderer.render_doc(doc, Mode::Break)?;
    if !renderer.halted {
        renderer.flush_line_suffixes()?;
    }
    Ok(renderer.finish())
}

/// Renders a document into an owned [`String`].
///
/// # Errors
///
/// Returns [`RenderError`] when the document is structurally invalid or contains
/// invalid non-literal text.
pub fn render(doc: &Doc, options: RenderOptions) -> Result<Rendered, RenderError> {
    let mut sink = StringSink::default();
    let outcome = render_to(doc, options, &mut sink).map_err(infallible_render_to_error)?;
    Ok(Rendered {
        text: sink.into_string(),
        stats: outcome.stats,
    })
}

fn infallible_render_to_error(error: RenderToError<Infallible>) -> RenderError {
    match error {
        RenderToError::Render(error) => error,
        RenderToError::Sink(error) => match error {},
    }
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

#[derive(Clone, Copy, Debug)]
enum PrintCommand<'a> {
    Doc(&'a Doc, Mode),
    EndIndent(i16),
    EndAlign(i16),
    EndGroup,
}

struct Renderer<S> {
    options: RenderOptions,
    sink: S,
    halted: bool,
    line: u32,
    column: TextWidth,
    max_column: TextWidth,
    indent_levels: i32,
    align_spaces: i32,
    group_modes: BTreeMap<GroupId, bool>,
    group_stack: Vec<GroupFrame>,
    line_suffixes: Vec<Doc>,
    flushing_suffixes: bool,
    measured_group_fits: bool,
    stats: RenderStats,
}

impl<S: RenderSink> Renderer<S> {
    fn new(options: RenderOptions, sink: S) -> Self {
        Self {
            options,
            sink,
            halted: false,
            line: 1,
            column: TextWidth::ZERO,
            max_column: TextWidth::ZERO,
            indent_levels: 0,
            align_spaces: 0,
            group_modes: BTreeMap::new(),
            group_stack: Vec::new(),
            line_suffixes: Vec::new(),
            flushing_suffixes: false,
            measured_group_fits: false,
            stats: RenderStats::default(),
        }
    }

    fn finish(mut self) -> RenderOutcome {
        self.stats.line_count = self.line;
        self.stats.max_column = self.max_column.max(self.column);
        RenderOutcome {
            stats: self.stats,
            halted: self.halted,
        }
    }

    fn render_doc(&mut self, doc: &Doc, mode: Mode) -> Result<(), RenderToError<S::Error>> {
        let mut stack = vec![PrintCommand::Doc(doc, mode)];
        self.render_commands(&mut stack)
    }

    fn render_commands(
        &mut self,
        stack: &mut Vec<PrintCommand<'_>>,
    ) -> Result<(), RenderToError<S::Error>> {
        while let Some(command) = stack.pop() {
            if self.halted {
                break;
            }
            match command {
                PrintCommand::Doc(doc, mode) => self.render_command_doc(doc, mode, stack)?,
                PrintCommand::EndIndent(levels) => {
                    self.indent_levels -= i32::from(levels);
                }
                PrintCommand::EndAlign(spaces) => {
                    self.align_spaces -= i32::from(spaces);
                }
                PrintCommand::EndGroup => {
                    self.group_stack.pop();
                }
            }
        }
        Ok(())
    }

    fn render_command_doc<'a>(
        &mut self,
        doc: &'a Doc,
        mode: Mode,
        stack: &mut Vec<PrintCommand<'a>>,
    ) -> Result<(), RenderToError<S::Error>> {
        match doc.kind() {
            DocKind::Nil | DocKind::BreakParent => Ok(()),
            DocKind::Text(text) => {
                self.write_text(&text.text, text.width)?;
                Ok(())
            }
            DocKind::LiteralText(text) => {
                self.write_literal(text)?;
                Ok(())
            }
            DocKind::Concat(docs) => {
                for doc in docs.iter().rev() {
                    stack.push(PrintCommand::Doc(doc, mode));
                }
                Ok(())
            }
            DocKind::Group(group) => self.render_group(group, mode, stack),
            DocKind::Fill(entries) => self.render_fill(entries, mode),
            DocKind::Indent(indent) => {
                self.indent_levels += i32::from(indent.levels);
                stack.push(PrintCommand::EndIndent(indent.levels));
                stack.push(PrintCommand::Doc(&indent.contents, mode));
                Ok(())
            }
            DocKind::Align(align) => {
                self.align_spaces += i32::from(align.spaces);
                stack.push(PrintCommand::EndAlign(align.spaces));
                stack.push(PrintCommand::Doc(&align.contents, mode));
                Ok(())
            }
            DocKind::Line(line) => self.render_line(line, mode),
            DocKind::IfBreak(if_break) => {
                let is_broken = self
                    .group_break_state(if_break.group_id)
                    .map_err(RenderToError::Render)?;
                stack.push(PrintCommand::Doc(
                    if is_broken {
                        &if_break.breaks
                    } else {
                        &if_break.flat
                    },
                    mode,
                ));
                Ok(())
            }
            DocKind::IndentIfBreak(indent_if_break) => {
                let is_broken = self
                    .group_break_state(Some(indent_if_break.group_id))
                    .map_err(RenderToError::Render)?;
                let should_indent = is_broken != indent_if_break.negate;
                if should_indent {
                    self.indent_levels += 1;
                    stack.push(PrintCommand::EndIndent(1));
                }
                stack.push(PrintCommand::Doc(&indent_if_break.contents, mode));
                Ok(())
            }
            DocKind::LineSuffix(doc) => {
                self.stats.line_suffix_count += 1;
                self.line_suffixes.push((**doc).clone());
                Ok(())
            }
            DocKind::LineSuffixBoundary => self.flush_line_suffixes(),
        }
    }

    fn render_group<'a>(
        &mut self,
        group: &'a Group,
        mode: Mode,
        stack: &mut Vec<PrintCommand<'a>>,
    ) -> Result<(), RenderToError<S::Error>> {
        let is_broken = if group.should_break {
            true
        } else if mode == Mode::Flat && self.measured_group_fits {
            false
        } else {
            let fits = self
                .group_fits(group, stack)
                .map_err(RenderToError::Render)?;
            if fits {
                self.measured_group_fits = true;
            }
            !fits
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
        stack.push(PrintCommand::EndGroup);
        stack.push(PrintCommand::Doc(
            &group.contents,
            if is_broken { Mode::Break } else { Mode::Flat },
        ));
        Ok(())
    }

    fn group_fits<'a>(
        &self,
        group: &'a Group,
        stack: &[PrintCommand<'a>],
    ) -> Result<bool, RenderError> {
        let mut checker = FitChecker::from_renderer(self);
        checker.fit_group_flat_with_stack(group, stack)
    }

    fn render_fill(
        &mut self,
        entries: &[FillEntry],
        mode: Mode,
    ) -> Result<(), RenderToError<S::Error>> {
        for (index, entry) in entries.iter().enumerate() {
            self.render_doc(&entry.content, mode)?;
            let Some(separator) = &entry.separator else {
                continue;
            };
            let separator_mode = self
                .fill_pair_separator_mode(entries, index, separator)
                .map_err(RenderToError::Render)?;
            self.render_doc(separator, separator_mode)?;
        }
        Ok(())
    }

    fn fill_pair_fits(&self, separator: &Doc, next_content: &Doc) -> Result<bool, RenderError> {
        let mut checker = FitChecker::from_renderer(self);
        let docs = [separator, next_content];
        for doc in docs {
            if !checker.fits_doc(doc, Mode::Flat)? {
                return Ok(false);
            }
        }
        Ok(checker.column <= self.options.line_width)
    }

    fn render_line(&mut self, line: &Line, mode: Mode) -> Result<(), RenderToError<S::Error>> {
        match (mode, line.mode) {
            (_, LineMode::Hard) => self.write_newline(line.indent_delta, 1),
            (_, LineMode::Empty) => self.write_newline(line.indent_delta, 2),
            (Mode::Flat, LineMode::Soft | LineMode::SoftOrSpace) => {
                self.write_flat_line(&line.flat)?;
                Ok(())
            }
            (Mode::Break, LineMode::Soft | LineMode::SoftOrSpace) => {
                self.write_newline(line.indent_delta, 1)
            }
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

    fn write_text(&mut self, text: &str, width: TextWidth) -> Result<(), RenderToError<S::Error>> {
        self.write_str(text)?;
        self.add_width(width);
        Ok(())
    }

    fn write_literal(&mut self, literal: &LiteralText) -> Result<(), RenderToError<S::Error>> {
        self.write_str(&literal.text)?;
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
            self.measured_group_fits = false;
        }
        Ok(())
    }

    fn write_flat_line(&mut self, flat: &FlatLine) -> Result<(), RenderToError<S::Error>> {
        match flat {
            FlatLine::Empty => Ok(()),
            FlatLine::Space => self.write_text(" ", TextWidth::new(1)),
            FlatLine::Text(text, width) => self.write_text(text, *width),
        }
    }

    fn write_newline(
        &mut self,
        indent_delta: i16,
        count: u32,
    ) -> Result<(), RenderToError<S::Error>> {
        self.flush_line_suffixes()?;
        for _ in 0..count {
            self.max_column = self.max_column.max(self.column);
            self.write_str(self.options.line_ending.as_str())?;
            self.line += 1;
            self.column = TextWidth::ZERO;
            self.measured_group_fits = false;
        }
        self.write_indent(indent_delta)
    }

    fn write_indent(&mut self, indent_delta: i16) -> Result<(), RenderToError<S::Error>> {
        let effective_levels = (self.indent_levels + i32::from(indent_delta))
            .max(0)
            .cast_unsigned();
        let base_width = effective_levels * u32::from(self.options.indent_width);
        match self.options.indent_style {
            IndentStyle::Space => {
                let width = saturating_nonnegative_u32(
                    i64::from(base_width) + i64::from(self.align_spaces),
                );
                self.write_repeated(' ', width)?;
                self.column = TextWidth::new(width);
            }
            IndentStyle::Tab => {
                if self.align_spaces >= 0 {
                    self.write_repeated('\t', effective_levels)?;
                    let spaces = self.align_spaces.cast_unsigned();
                    self.write_repeated(' ', spaces)?;
                    self.column = TextWidth::new(base_width + spaces);
                } else {
                    let width = saturating_nonnegative_u32(
                        i64::from(base_width) + i64::from(self.align_spaces),
                    );
                    let tab_width = u32::from(self.options.indent_width);
                    let tabs = width / tab_width;
                    let spaces = width % tab_width;
                    self.write_repeated('\t', tabs)?;
                    self.write_repeated(' ', spaces)?;
                    self.column = TextWidth::new(width);
                }
            }
        }
        self.max_column = self.max_column.max(self.column);
        Ok(())
    }

    fn flush_line_suffixes(&mut self) -> Result<(), RenderToError<S::Error>> {
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

    fn write_str(&mut self, text: &str) -> Result<(), RenderToError<S::Error>> {
        if text.is_empty() || self.halted {
            return Ok(());
        }
        match self.sink.write_str(text).map_err(RenderToError::Sink)? {
            RenderControl::Continue => {}
            RenderControl::Halt => {
                self.halted = true;
            }
        }
        Ok(())
    }

    fn write_repeated(&mut self, ch: char, count: u32) -> Result<(), RenderToError<S::Error>> {
        const SPACES: &str = "                                ";
        const TABS: &str = "\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t";

        let chunk = match ch {
            ' ' => SPACES,
            '\t' => TABS,
            _ => unreachable!("renderer only repeats indentation whitespace"),
        };
        let chunk_len = u32::try_from(chunk.len()).expect("indent chunk length fits u32");
        let mut remaining = count;
        while remaining > 0 {
            let write_len = remaining.min(chunk_len);
            self.write_str(&chunk[..usize::try_from(write_len).expect("chunk length fits usize")])?;
            remaining -= write_len;
        }
        Ok(())
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

fn saturating_nonnegative_u32(value: i64) -> u32 {
    u32::try_from(value.max(0)).unwrap_or(u32::MAX)
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
    line_suffixes: Vec<Doc>,
}

impl FitChecker {
    fn from_renderer<S>(renderer: &Renderer<S>) -> Self {
        Self {
            options: renderer.options,
            column: renderer.column,
            indent_levels: renderer.indent_levels,
            align_spaces: renderer.align_spaces,
            group_modes: renderer.group_modes.clone(),
            group_stack: renderer.group_stack.clone(),
            line_suffixes: renderer.line_suffixes.clone(),
        }
    }

    fn fits_doc(&mut self, doc: &Doc, mode: Mode) -> Result<bool, RenderError> {
        let mut stack = vec![PrintCommand::Doc(doc, mode)];
        self.fits_stack(&mut stack)
    }

    fn fits_stack(&mut self, stack: &mut Vec<PrintCommand<'_>>) -> Result<bool, RenderError> {
        while let Some(command) = stack.pop() {
            match self.fit_command(command, stack)? {
                FitResult::Continue => {}
                FitResult::Done => return Ok(true),
                FitResult::No => return Ok(false),
            }
        }

        Ok(self.flush_line_suffixes()? && self.column <= self.options.line_width)
    }

    fn fit_command<'a>(
        &mut self,
        command: PrintCommand<'a>,
        stack: &mut Vec<PrintCommand<'a>>,
    ) -> Result<FitResult, RenderError> {
        match command {
            PrintCommand::Doc(doc, mode) => self.fit_doc(doc, mode, stack),
            PrintCommand::EndIndent(levels) => {
                self.indent_levels -= i32::from(levels);
                Ok(FitResult::Continue)
            }
            PrintCommand::EndAlign(spaces) => {
                self.align_spaces -= i32::from(spaces);
                Ok(FitResult::Continue)
            }
            PrintCommand::EndGroup => {
                self.group_stack.pop();
                Ok(FitResult::Continue)
            }
        }
    }

    fn fit_doc<'a>(
        &mut self,
        doc: &'a Doc,
        mode: Mode,
        stack: &mut Vec<PrintCommand<'a>>,
    ) -> Result<FitResult, RenderError> {
        match doc.kind() {
            DocKind::Nil => Ok(FitResult::Continue),
            DocKind::BreakParent => Ok(FitResult::No),
            DocKind::LineSuffixBoundary => {
                if self.flush_line_suffixes()? {
                    Ok(FitResult::Continue)
                } else {
                    Ok(FitResult::No)
                }
            }
            DocKind::Text(text) => Ok(self.width_result(text.width)),
            DocKind::LiteralText(text) => {
                if has_line_terminator(&text.text) {
                    Ok(FitResult::No)
                } else {
                    Ok(self.width_result(text.final_width()))
                }
            }
            DocKind::Concat(docs) => {
                for doc in docs.iter().rev() {
                    stack.push(PrintCommand::Doc(doc, mode));
                }
                Ok(FitResult::Continue)
            }
            DocKind::Group(group) => Ok(self.fit_group(group, mode, stack)),
            DocKind::Fill(entries) => self.fit_fill(entries, mode, stack),
            DocKind::Indent(indent) => {
                self.indent_levels += i32::from(indent.levels);
                stack.push(PrintCommand::EndIndent(indent.levels));
                stack.push(PrintCommand::Doc(&indent.contents, mode));
                Ok(FitResult::Continue)
            }
            DocKind::Align(align) => {
                self.align_spaces += i32::from(align.spaces);
                stack.push(PrintCommand::EndAlign(align.spaces));
                stack.push(PrintCommand::Doc(&align.contents, mode));
                Ok(FitResult::Continue)
            }
            DocKind::Line(line) => self.fit_line(line, mode),
            DocKind::IfBreak(if_break) => {
                let is_broken = self.group_break_state(if_break.group_id).unwrap_or(false);
                stack.push(PrintCommand::Doc(
                    if is_broken {
                        &if_break.breaks
                    } else {
                        &if_break.flat
                    },
                    mode,
                ));
                Ok(FitResult::Continue)
            }
            DocKind::IndentIfBreak(indent_if_break) => {
                let is_broken = self
                    .group_break_state(Some(indent_if_break.group_id))
                    .unwrap_or(false);
                let should_indent = is_broken != indent_if_break.negate;
                if should_indent {
                    self.indent_levels += 1;
                    stack.push(PrintCommand::EndIndent(1));
                }
                stack.push(PrintCommand::Doc(&indent_if_break.contents, mode));
                Ok(FitResult::Continue)
            }
            DocKind::LineSuffix(doc) => {
                self.line_suffixes.push((**doc).clone());
                Ok(FitResult::Continue)
            }
        }
    }

    fn fit_group<'a>(
        &mut self,
        group: &'a Group,
        mode: Mode,
        stack: &mut Vec<PrintCommand<'a>>,
    ) -> FitResult {
        if mode == Mode::Flat && group.should_break {
            return FitResult::No;
        }
        let is_broken = mode == Mode::Break || group.should_break;
        if let Some(id) = group.id {
            self.group_modes.insert(id, is_broken);
        }
        self.group_stack.push(GroupFrame {
            id: group.id,
            is_broken,
        });
        stack.push(PrintCommand::EndGroup);
        stack.push(PrintCommand::Doc(
            &group.contents,
            if is_broken { Mode::Break } else { Mode::Flat },
        ));
        FitResult::Continue
    }

    fn fit_group_flat_with_stack<'a>(
        &mut self,
        group: &'a Group,
        stack: &[PrintCommand<'a>],
    ) -> Result<bool, RenderError> {
        if let Some(id) = group.id {
            self.group_modes.insert(id, false);
        }
        self.group_stack.push(GroupFrame {
            id: group.id,
            is_broken: false,
        });
        let mut fit_stack = stack.to_vec();
        fit_stack.push(PrintCommand::EndGroup);
        fit_stack.push(PrintCommand::Doc(&group.contents, Mode::Flat));
        self.fits_stack(&mut fit_stack)
    }

    fn fit_fill<'a>(
        &mut self,
        entries: &'a [FillEntry],
        mode: Mode,
        stack: &mut Vec<PrintCommand<'a>>,
    ) -> Result<FitResult, RenderError> {
        if entries.is_empty() {
            return Ok(FitResult::Continue);
        }

        for (index, entry) in entries.iter().enumerate().rev() {
            if let Some(separator) = &entry.separator {
                let separator_mode = if let Some(next) = entries.get(index + 1) {
                    let mut pair = self.clone();
                    if pair.fits_doc(separator, Mode::Flat)?
                        && pair.fits_doc(&next.content, Mode::Flat)?
                    {
                        Mode::Flat
                    } else {
                        Mode::Break
                    }
                } else {
                    Mode::Break
                };
                stack.push(PrintCommand::Doc(separator, separator_mode));
            }
            stack.push(PrintCommand::Doc(&entry.content, mode));
        }
        Ok(FitResult::Continue)
    }

    fn fit_line(&mut self, line: &Line, mode: Mode) -> Result<FitResult, RenderError> {
        match (mode, line.mode) {
            (Mode::Flat, LineMode::Soft | LineMode::SoftOrSpace) => {
                Ok(self.fit_flat_line(&line.flat))
            }
            (Mode::Flat, LineMode::Hard | LineMode::Empty) if line.propagate_break => {
                Ok(FitResult::No)
            }
            (Mode::Flat, LineMode::Hard | LineMode::Empty) | (Mode::Break, _) => {
                if self.flush_line_suffixes()? && self.column <= self.options.line_width {
                    Ok(FitResult::Done)
                } else {
                    Ok(FitResult::No)
                }
            }
        }
    }

    fn fit_flat_line(&mut self, flat: &FlatLine) -> FitResult {
        match flat {
            FlatLine::Empty => FitResult::Continue,
            FlatLine::Space => self.width_result(TextWidth::new(1)),
            FlatLine::Text(_, width) => self.width_result(*width),
        }
    }

    fn flush_line_suffixes(&mut self) -> Result<bool, RenderError> {
        while !self.line_suffixes.is_empty() {
            let suffixes = std::mem::take(&mut self.line_suffixes);
            for suffix in &suffixes {
                let mut stack = vec![PrintCommand::Doc(suffix, Mode::Flat)];
                if !self.fits_stack(&mut stack)? {
                    return Ok(false);
                }
            }
        }

        Ok(self.column <= self.options.line_width)
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

    fn width_result(&mut self, width: TextWidth) -> FitResult {
        self.column = add_width(self.column, width);
        if self.column <= self.options.line_width {
            FitResult::Continue
        } else {
            FitResult::No
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FitResult {
    Continue,
    Done,
    No,
}
