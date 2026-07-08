use std::error::Error;
use std::fmt;

use crate::document::{Doc, DocKind, FlatLine, Group, Line, LineMode, LiteralText};
use crate::validation::validate_doc;
use crate::width::{TextWidth, add_width};

// A flat-fit probe can scan nested docs, the active render stack, and an overlay
// stack for groups/indents. Cap the number of commands each probe can process so
// repeated tiny groups cannot turn rendering into unbounded layout search. When
// the budget is exhausted, the group is treated as not fitting and rendered in
// break mode.
const FLAT_FIT_COMMAND_BUDGET: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndentStyle {
    Space,
    Tab,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderOptions {
    pub line_width: TextWidth,
    pub indent_width: u16,
    pub indent_style: IndentStyle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderOutcome {
    pub halted: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderControl {
    Continue,
    Halt,
}

pub trait RenderSink {
    /// Writes a rendered text chunk and returns whether rendering should continue.
    fn write_str(&mut self, text: &str) -> RenderControl;
}

impl<T: RenderSink + ?Sized> RenderSink for &mut T {
    fn write_str(&mut self, text: &str) -> RenderControl {
        (**self).write_str(text)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderError {
    kind: RenderErrorKind,
}

impl RenderError {
    pub(crate) const fn invalid_text(context: &'static str) -> Self {
        Self {
            kind: RenderErrorKind::InvalidText { context },
        }
    }

    pub(crate) const fn no_current_group() -> Self {
        Self {
            kind: RenderErrorKind::NoCurrentGroup,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum RenderErrorKind {
    InvalidText { context: &'static str },
    NoCurrentGroup,
}

impl fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            RenderErrorKind::InvalidText { context } => {
                write!(formatter, "{context} must not contain line terminators")
            }
            RenderErrorKind::NoCurrentGroup => {
                formatter.write_str("if_break requires a current group")
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
pub fn render_to<S: RenderSink>(
    doc: &Doc<'_>,
    options: RenderOptions,
    sink: S,
) -> Result<RenderOutcome, RenderError> {
    validate_doc(doc)?;
    let mut renderer = Renderer::new(options, sink);
    renderer.render_doc(doc, Mode::Break)?;
    Ok(renderer.finish())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Mode {
    Flat,
    Break,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GroupFrame {
    is_broken: bool,
}

#[derive(Clone, Copy, Debug)]
enum PrintCommand<'doc, 'source> {
    Doc(&'doc Doc<'source>, Mode),
    EndIndent(i16),
    EndGroup,
}

struct Renderer<S> {
    options: RenderOptions,
    sink: S,
    halted: bool,
    column: TextWidth,
    indent_levels: i32,
    group_stack: Vec<GroupFrame>,
    measured_group_fits: bool,
}

impl<S: RenderSink> Renderer<S> {
    fn new(options: RenderOptions, sink: S) -> Self {
        Self {
            options,
            sink,
            halted: false,
            column: TextWidth::ZERO,
            indent_levels: 0,
            group_stack: Vec::new(),
            measured_group_fits: false,
        }
    }

    fn finish(self) -> RenderOutcome {
        RenderOutcome {
            halted: self.halted,
        }
    }

    fn render_doc(&mut self, doc: &Doc<'_>, mode: Mode) -> Result<(), RenderError> {
        let mut stack = vec![PrintCommand::Doc(doc, mode)];
        self.render_commands(&mut stack)
    }

    fn render_commands(
        &mut self,
        stack: &mut Vec<PrintCommand<'_, '_>>,
    ) -> Result<(), RenderError> {
        while let Some(command) = stack.pop() {
            if self.halted {
                break;
            }
            match command {
                PrintCommand::Doc(doc, mode) => self.render_command_doc(doc, mode, stack)?,
                PrintCommand::EndIndent(levels) => {
                    self.indent_levels -= i32::from(levels);
                }
                PrintCommand::EndGroup => {
                    self.group_stack.pop();
                }
            }
        }
        Ok(())
    }

    fn render_command_doc<'doc, 'source>(
        &mut self,
        doc: &'doc Doc<'source>,
        mode: Mode,
        stack: &mut Vec<PrintCommand<'doc, 'source>>,
    ) -> Result<(), RenderError> {
        match doc.kind() {
            DocKind::Nil => Ok(()),
            DocKind::Text(text) => {
                self.write_measured_str(&text.text, text.width);
                Ok(())
            }
            DocKind::LiteralText(text) => {
                self.write_literal(text);
                Ok(())
            }
            DocKind::Concat(docs) => {
                for doc in docs.iter().rev() {
                    stack.push(PrintCommand::Doc(doc, mode));
                }
                Ok(())
            }
            DocKind::Group(group) => {
                self.render_group(group, mode, stack);
                Ok(())
            }
            DocKind::Indent(indent) => {
                self.indent_levels += i32::from(indent.levels);
                stack.push(PrintCommand::EndIndent(indent.levels));
                stack.push(PrintCommand::Doc(&indent.contents, mode));
                Ok(())
            }
            DocKind::Line(line) => {
                self.render_line(line, mode);
                Ok(())
            }
            DocKind::IfBreak(if_break) => {
                let is_broken = self.group_break_state()?;
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
        }
    }

    fn render_group<'doc, 'source>(
        &mut self,
        group: &'doc Group<'source>,
        mode: Mode,
        stack: &mut Vec<PrintCommand<'doc, 'source>>,
    ) {
        let is_broken = if group.should_break {
            true
        } else if mode == Mode::Flat && self.measured_group_fits {
            false
        } else {
            let fits = self.group_fits(group, stack);
            if fits {
                self.measured_group_fits = true;
            }
            !fits
        };
        self.group_stack.push(GroupFrame { is_broken });
        stack.push(PrintCommand::EndGroup);
        stack.push(PrintCommand::Doc(
            &group.contents,
            if is_broken { Mode::Break } else { Mode::Flat },
        ));
    }

    fn group_fits<'doc, 'source>(
        &self,
        group: &'doc Group<'source>,
        stack: &[PrintCommand<'doc, 'source>],
    ) -> bool {
        let mut checker = FitChecker::from_renderer(self);
        checker.fit_group_flat_with_stack(group, stack)
    }

    fn render_line(&mut self, line: &Line, mode: Mode) {
        match (mode, line.mode) {
            (Mode::Flat, LineMode::Soft | LineMode::SoftOrSpace) => {
                self.write_flat_line(&line.flat);
            }
            (_, LineMode::Hard | LineMode::Empty)
            | (Mode::Break, LineMode::Soft | LineMode::SoftOrSpace) => {
                let count = if line.mode == LineMode::Empty { 2 } else { 1 };
                self.write_newline(line.indent_delta, count);
            }
        }
    }

    fn group_break_state(&self) -> Result<bool, RenderError> {
        self.group_stack
            .last()
            .map(|frame| frame.is_broken)
            .ok_or_else(RenderError::no_current_group)
    }

    fn write_measured_str(&mut self, text: &str, width: TextWidth) {
        self.write_str(text);
        self.add_width(width);
    }

    fn write_literal(&mut self, literal: &LiteralText<'_>) {
        self.write_str(&literal.text);
        let final_width = literal.final_width();
        if literal.is_multiline() {
            self.column = final_width;
            self.measured_group_fits = false;
        } else {
            self.add_width(final_width);
        }
    }

    fn write_flat_line(&mut self, flat: &FlatLine) {
        match flat {
            FlatLine::Empty => {}
            FlatLine::Space => self.write_measured_str(" ", TextWidth::new(1)),
        }
    }

    fn write_newline(&mut self, indent_delta: i16, count: u32) {
        for _ in 0..count {
            self.write_str("\n");
            self.column = TextWidth::ZERO;
            self.measured_group_fits = false;
        }
        self.write_indent(indent_delta);
    }

    fn write_indent(&mut self, indent_delta: i16) {
        let effective_levels = (self.indent_levels + i32::from(indent_delta))
            .max(0)
            .cast_unsigned();
        let width = effective_levels * u32::from(self.options.indent_width);
        match self.options.indent_style {
            IndentStyle::Space => {
                self.write_repeated(' ', width);
                self.column = TextWidth::new(width);
            }
            IndentStyle::Tab => {
                self.write_repeated('\t', effective_levels);
                self.column = TextWidth::new(width);
            }
        }
    }

    fn write_str(&mut self, text: &str) {
        if text.is_empty() || self.halted {
            return;
        }
        match self.sink.write_str(text) {
            RenderControl::Continue => {}
            RenderControl::Halt => {
                self.halted = true;
            }
        }
    }

    fn write_repeated(&mut self, ch: char, count: u32) {
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
            self.write_str(&chunk[..usize::try_from(write_len).expect("chunk length fits usize")]);
            remaining -= write_len;
        }
    }

    fn add_width(&mut self, width: TextWidth) {
        self.column = add_width(self.column, width);
    }
}

struct FitChecker<'base> {
    options: RenderOptions,
    column: TextWidth,
    indent_levels: i32,
    base_group_stack: &'base [GroupFrame],
    base_group_len: usize,
    group_stack: Vec<GroupFrame>,
    remaining_commands: usize,
}

impl<'base> FitChecker<'base> {
    fn from_renderer<S>(renderer: &'base Renderer<S>) -> Self {
        Self {
            options: renderer.options,
            column: renderer.column,
            indent_levels: renderer.indent_levels,
            base_group_stack: &renderer.group_stack,
            base_group_len: renderer.group_stack.len(),
            group_stack: Vec::new(),
            remaining_commands: FLAT_FIT_COMMAND_BUDGET,
        }
    }

    fn fits_stack(&mut self, stack: &mut FitStack<'_, '_, '_>) -> bool {
        while let Some(command) = stack.pop() {
            let Some(remaining_commands) = self.remaining_commands.checked_sub(1) else {
                return false;
            };
            self.remaining_commands = remaining_commands;
            match self.fit_command(command, stack) {
                FitResult::Continue => {}
                FitResult::Done => return true,
                FitResult::No => return false,
            }
        }

        self.column <= self.options.line_width
    }

    fn fit_command<'doc, 'source>(
        &mut self,
        command: PrintCommand<'doc, 'source>,
        stack: &mut FitStack<'_, 'doc, 'source>,
    ) -> FitResult {
        match command {
            PrintCommand::Doc(doc, mode) => self.fit_doc(doc, mode, stack),
            PrintCommand::EndIndent(levels) => {
                self.indent_levels -= i32::from(levels);
                FitResult::Continue
            }
            PrintCommand::EndGroup => {
                if self.group_stack.pop().is_none() {
                    self.base_group_len = self.base_group_len.saturating_sub(1);
                }
                FitResult::Continue
            }
        }
    }

    fn fit_doc<'doc, 'source>(
        &mut self,
        doc: &'doc Doc<'source>,
        mode: Mode,
        stack: &mut FitStack<'_, 'doc, 'source>,
    ) -> FitResult {
        match doc.kind() {
            DocKind::Nil => FitResult::Continue,
            DocKind::Text(text) => self.width_result(text.width),
            DocKind::LiteralText(text) => {
                if text.is_multiline() {
                    FitResult::No
                } else {
                    self.width_result(text.final_width())
                }
            }
            DocKind::Concat(docs) => {
                for doc in docs.iter().rev() {
                    stack.push(PrintCommand::Doc(doc, mode));
                }
                FitResult::Continue
            }
            DocKind::Group(group) => self.fit_group(group, mode, stack),
            DocKind::Indent(indent) => {
                self.indent_levels += i32::from(indent.levels);
                stack.push(PrintCommand::EndIndent(indent.levels));
                stack.push(PrintCommand::Doc(&indent.contents, mode));
                FitResult::Continue
            }
            DocKind::Line(line) => self.fit_line(line, mode),
            DocKind::IfBreak(if_break) => {
                let is_broken = self.group_break_state().unwrap_or(false);
                stack.push(PrintCommand::Doc(
                    if is_broken {
                        &if_break.breaks
                    } else {
                        &if_break.flat
                    },
                    mode,
                ));
                FitResult::Continue
            }
        }
    }

    fn fit_group<'doc, 'source>(
        &mut self,
        group: &'doc Group<'source>,
        mode: Mode,
        stack: &mut FitStack<'_, 'doc, 'source>,
    ) -> FitResult {
        if mode == Mode::Flat && group.should_break {
            return FitResult::No;
        }
        let is_broken = mode == Mode::Break || group.should_break;
        self.group_stack.push(GroupFrame { is_broken });
        stack.push(PrintCommand::EndGroup);
        stack.push(PrintCommand::Doc(
            &group.contents,
            if is_broken { Mode::Break } else { Mode::Flat },
        ));
        FitResult::Continue
    }

    fn fit_group_flat_with_stack<'doc, 'source>(
        &mut self,
        group: &'doc Group<'source>,
        stack: &[PrintCommand<'doc, 'source>],
    ) -> bool {
        self.group_stack.push(GroupFrame { is_broken: false });
        let mut fit_stack = FitStack::new(stack);
        fit_stack.push(PrintCommand::EndGroup);
        fit_stack.push(PrintCommand::Doc(&group.contents, Mode::Flat));
        self.fits_stack(&mut fit_stack)
    }

    fn fit_line(&mut self, line: &Line, mode: Mode) -> FitResult {
        match (mode, line.mode) {
            (Mode::Flat, LineMode::Soft | LineMode::SoftOrSpace) => self.fit_flat_line(&line.flat),
            (Mode::Flat, LineMode::Hard | LineMode::Empty) => FitResult::No,
            (Mode::Break, _) => {
                if self.column <= self.options.line_width {
                    FitResult::Done
                } else {
                    FitResult::No
                }
            }
        }
    }

    fn fit_flat_line(&mut self, flat: &FlatLine) -> FitResult {
        match flat {
            FlatLine::Empty => FitResult::Continue,
            FlatLine::Space => self.width_result(TextWidth::new(1)),
        }
    }

    fn group_break_state(&self) -> Result<bool, RenderError> {
        self.group_stack
            .last()
            .or_else(|| self.base_group_stack[..self.base_group_len].last())
            .map(|frame| frame.is_broken)
            .ok_or_else(RenderError::no_current_group)
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

struct FitStack<'stack, 'doc, 'source> {
    base: &'stack [PrintCommand<'doc, 'source>],
    base_next: usize,
    overlay: Vec<PrintCommand<'doc, 'source>>,
}

impl<'stack, 'doc, 'source> FitStack<'stack, 'doc, 'source> {
    fn new(base: &'stack [PrintCommand<'doc, 'source>]) -> Self {
        Self {
            base,
            base_next: base.len(),
            overlay: Vec::new(),
        }
    }

    fn push(&mut self, command: PrintCommand<'doc, 'source>) {
        self.overlay.push(command);
    }

    fn pop(&mut self) -> Option<PrintCommand<'doc, 'source>> {
        self.overlay.pop().or_else(|| {
            self.base_next = self.base_next.checked_sub(1)?;
            Some(self.base[self.base_next])
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FitResult {
    Continue,
    Done,
    No,
}
