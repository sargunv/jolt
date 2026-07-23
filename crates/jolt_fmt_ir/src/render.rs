use std::error::Error;
use std::fmt;

use crate::FormatOptions;
use crate::document::{Doc, DocArena, DocNode, DocumentText, FlatLine, Line, LineMode};
use crate::source_fragment::RenderProof;
use crate::width::{TextWidth, add_width};
use jolt_syntax::{ConservationError, Language, SyntaxNode};

// A flat-fit probe can scan nested docs, the active render stack, and an overlay
// stack for groups/indents. Cap the number of semantic commands each probe can
// process so repeated tiny groups cannot turn rendering into unbounded layout
// search. Concat range cursors are implementation bookkeeping and are bounded by
// the document commands they expose. When the budget is exhausted, the group is
// treated as not fitting and rendered in break mode.
const FLAT_FIT_COMMAND_BUDGET: usize = 4096;

/// A completed source-aware render.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SourceRenderOutcome {
    halted: bool,
    #[cfg(test)]
    used_malformed_verbatim: bool,
}

impl SourceRenderOutcome {
    #[must_use]
    pub(crate) const fn halted(self) -> bool {
        self.halted
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn used_malformed_verbatim(self) -> bool {
        self.used_malformed_verbatim
    }
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
    pub(crate) const fn no_current_group() -> Self {
        Self {
            kind: RenderErrorKind::NoCurrentGroup,
        }
    }

    pub(crate) fn syntax_invariant(message: &str) -> Self {
        Self {
            kind: RenderErrorKind::SyntaxInvariant(message.to_owned()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum RenderErrorKind {
    NoCurrentGroup,
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    Conservation(ConservationError),
    SyntaxInvariant(String),
}

impl fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            RenderErrorKind::NoCurrentGroup => {
                formatter.write_str("if_break requires a current group")
            }
            RenderErrorKind::Conservation(ref error) => write!(formatter, "{error}"),
            RenderErrorKind::SyntaxInvariant(ref message) => {
                write!(formatter, "syntax invariant failed: {message}")
            }
        }
    }
}

impl Error for RenderError {}

/// Renders a source document while proving exact conservation for the selected
/// document branches.
///
/// # Errors
///
/// Returns [`RenderError`] when the document is structurally invalid or a
/// rendered fragment makes a duplicate or foreign source claim.
pub(crate) fn render_source_to<'source, L: Language, S: RenderSink>(
    arena: &DocArena<'source>,
    doc: Doc<'source>,
    options: FormatOptions,
    sink: S,
    root: &SyntaxNode<'source, L>,
) -> Result<SourceRenderOutcome, RenderError> {
    if let Some(error) = arena.invariant_error() {
        return Err(RenderError::syntax_invariant(error));
    }
    #[cfg(debug_assertions)]
    let used_malformed_verbatim = {
        let mut proof = RenderProof::new(root.conservation_tracker());
        let mut renderer = Renderer::new(arena, options, DiscardSink, Some(&mut proof));
        renderer.render_doc(doc, Mode::Break)?;
        proof.finish().map_err(|error| RenderError {
            kind: RenderErrorKind::Conservation(error),
        })?
    };
    #[cfg(debug_assertions)]
    assert!(
        !root.is_recovery_free() || !used_malformed_verbatim,
        "recovery-free syntax rendered a malformed-verbatim fragment"
    );
    #[cfg(not(debug_assertions))]
    let _ = root;
    let mut renderer = Renderer::new(arena, options, sink, None);
    renderer.render_doc(doc, Mode::Break)?;
    if renderer.halted {
        return Ok(SourceRenderOutcome {
            halted: true,
            #[cfg(test)]
            used_malformed_verbatim: false,
        });
    }
    #[cfg(all(test, not(debug_assertions)))]
    let used_malformed_verbatim = false;
    Ok(SourceRenderOutcome {
        halted: false,
        #[cfg(test)]
        used_malformed_verbatim,
    })
}

#[cfg(debug_assertions)]
struct DiscardSink;

#[cfg(debug_assertions)]
impl RenderSink for DiscardSink {
    fn write_str(&mut self, _text: &str) -> RenderControl {
        RenderControl::Continue
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Flat,
    Break,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum HorizontalWhitespace {
    #[default]
    None,
    Pending,
    Emitted,
    LiteralLineEnding,
}

#[derive(Clone, Copy, Debug)]
enum RenderCommand<'source> {
    Doc(Doc<'source>, Mode),
    EndIndent(i16),
    EndGroup,
}

#[derive(Clone, Copy, Debug)]
enum FitCommand<'source> {
    Doc(Doc<'source>, Mode),
    ConcatRange { next: u32, end: u32, mode: Mode },
    EndIndent,
    EndGroup,
    EndMeasuredGroup,
}

impl<'source> From<RenderCommand<'source>> for FitCommand<'source> {
    fn from(command: RenderCommand<'source>) -> Self {
        match command {
            RenderCommand::Doc(doc, mode) => Self::Doc(doc, mode),
            RenderCommand::EndIndent(_) => Self::EndIndent,
            RenderCommand::EndGroup => Self::EndGroup,
        }
    }
}

struct Renderer<'arena, 'proof, 'source, S> {
    arena: &'arena DocArena<'source>,
    options: FormatOptions,
    sink: S,
    halted: bool,
    column: TextWidth,
    indent_levels: i32,
    pending_indent: u32,
    horizontal_whitespace: HorizontalWhitespace,
    group_stack: Vec<Mode>,
    fit_group_stack: Vec<Mode>,
    fit_overlay_stack: Vec<FitCommand<'source>>,
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    proof: Option<&'proof mut RenderProof<'source>>,
}

impl<'arena, 'proof, 'source, S: RenderSink> Renderer<'arena, 'proof, 'source, S> {
    fn new(
        arena: &'arena DocArena<'source>,
        options: FormatOptions,
        sink: S,
        proof: Option<&'proof mut RenderProof<'source>>,
    ) -> Self {
        Self {
            arena,
            options,
            sink,
            halted: false,
            column: TextWidth::ZERO,
            indent_levels: 0,
            pending_indent: 0,
            horizontal_whitespace: HorizontalWhitespace::None,
            group_stack: Vec::new(),
            fit_group_stack: Vec::new(),
            fit_overlay_stack: Vec::new(),
            proof,
        }
    }

    fn render_doc(&mut self, doc: Doc<'source>, mode: Mode) -> Result<(), RenderError> {
        let mut stack = Vec::new();
        stack.push(RenderCommand::Doc(doc, mode));
        while let Some(command) = stack.pop() {
            if self.halted {
                break;
            }
            match command {
                RenderCommand::Doc(doc, mode) => {
                    self.render_command_doc(doc, mode, &mut stack)?;
                }
                RenderCommand::EndIndent(levels) => {
                    self.indent_levels -= i32::from(levels);
                }
                RenderCommand::EndGroup => {
                    self.group_stack.pop();
                }
            }
        }
        Ok(())
    }

    fn render_command_doc(
        &mut self,
        doc: Doc<'source>,
        mode: Mode,
        stack: &mut Vec<RenderCommand<'source>>,
    ) -> Result<(), RenderError> {
        let arena = self.arena;
        match arena.node(doc) {
            None => Ok(()),
            Some(DocNode::Text(text)) => {
                #[cfg(debug_assertions)]
                if let Some(claim) = text.claim
                    && let Some(proof) = self.proof.as_mut()
                {
                    proof.consume(claim).map_err(|error| RenderError {
                        kind: RenderErrorKind::Conservation(error),
                    })?;
                }
                if text.text == " " {
                    if self.horizontal_whitespace == HorizontalWhitespace::None {
                        self.horizontal_whitespace = HorizontalWhitespace::Pending;
                    }
                } else if text.is_multiline() {
                    self.write_multiline_literal(text);
                } else {
                    self.write_measured_str(&text.text, text.final_width());
                }
                Ok(())
            }
            Some(DocNode::InlineConcat { docs, len }) => {
                for child in docs[..usize::from(*len)].iter().rev() {
                    stack.push(RenderCommand::Doc(*child, mode));
                }
                Ok(())
            }
            Some(DocNode::ConcatRange { start, len }) => {
                if let Some(end) = start.checked_add(*len) {
                    self.render_concat_span(*start, end, mode, stack);
                }
                Ok(())
            }
            Some(DocNode::Group {
                contents,
                should_break,
            }) => {
                self.render_group(*contents, *should_break, mode, stack);
                Ok(())
            }
            Some(DocNode::Indent { contents, levels }) => {
                self.indent_levels += i32::from(*levels);
                stack.push(RenderCommand::EndIndent(*levels));
                stack.push(RenderCommand::Doc(*contents, mode));
                Ok(())
            }
            Some(DocNode::Line(line)) => {
                self.render_line(line, mode);
                Ok(())
            }
            Some(DocNode::IfBreak { breaks, flat }) => {
                let is_broken = self.group_break_state()?;
                stack.push(RenderCommand::Doc(
                    if is_broken { *breaks } else { *flat },
                    mode,
                ));
                Ok(())
            }
        }
    }

    fn render_concat_span(
        &mut self,
        start: u32,
        end: u32,
        mode: Mode,
        stack: &mut Vec<RenderCommand<'source>>,
    ) {
        for index in (start..end).rev() {
            stack.push(RenderCommand::Doc(self.arena.child(index), mode));
        }
    }

    fn render_group(
        &mut self,
        contents: Doc<'source>,
        should_break: bool,
        mode: Mode,
        stack: &mut Vec<RenderCommand<'source>>,
    ) {
        let group_mode = if should_break {
            Mode::Break
        } else if mode == Mode::Flat || self.group_fits(contents, stack) {
            Mode::Flat
        } else {
            Mode::Break
        };
        self.group_stack.push(group_mode);
        stack.push(RenderCommand::EndGroup);
        stack.push(RenderCommand::Doc(contents, group_mode));
    }

    fn group_fits(&mut self, contents: Doc<'source>, stack: &[RenderCommand<'source>]) -> bool {
        let mut group_stack = std::mem::take(&mut self.fit_group_stack);
        let mut overlay_stack = std::mem::take(&mut self.fit_overlay_stack);
        group_stack.clear();
        overlay_stack.clear();

        let mut checker = FitChecker::from_renderer(self, &mut group_stack);
        let fits = checker.fit_group_flat_with_stack(contents, stack, &mut overlay_stack);

        group_stack.clear();
        overlay_stack.clear();
        self.fit_group_stack = group_stack;
        self.fit_overlay_stack = overlay_stack;
        fits
    }

    fn render_line(&mut self, line: &Line, mode: Mode) {
        match (mode, line.mode) {
            (Mode::Flat, LineMode::Soft | LineMode::SoftOrSpace) => {
                self.write_flat_line(&line.flat);
            }
            (_, LineMode::Hard | LineMode::Empty)
            | (Mode::Break, LineMode::Soft | LineMode::SoftOrSpace) => {
                let count = if line.mode == LineMode::Empty { 2 } else { 1 };
                self.write_newlines(line.indent_delta, count);
            }
        }
    }

    fn group_break_state(&self) -> Result<bool, RenderError> {
        self.group_stack
            .last()
            .copied()
            .map(|mode| mode == Mode::Break)
            .ok_or_else(RenderError::no_current_group)
    }

    fn write_measured_str(&mut self, text: &str, width: TextWidth) {
        self.write_str(text);
        self.add_width(width);
    }

    fn write_multiline_literal(&mut self, literal: &DocumentText<'_>) {
        self.write_str(&literal.text);
        self.column = literal.final_width();
        if matches!(literal.text.as_bytes().last(), Some(b'\n' | b'\r')) {
            self.horizontal_whitespace = HorizontalWhitespace::LiteralLineEnding;
        }
    }

    fn write_flat_line(&mut self, flat: &FlatLine) {
        match flat {
            FlatLine::Empty => {}
            FlatLine::Space => {
                if self.horizontal_whitespace == HorizontalWhitespace::None {
                    self.horizontal_whitespace = HorizontalWhitespace::Pending;
                }
            }
        }
    }

    fn write_newlines(&mut self, indent_delta: i16, count: u32) {
        self.pending_indent = 0;
        let literal_ended_line =
            self.horizontal_whitespace == HorizontalWhitespace::LiteralLineEnding;
        self.horizontal_whitespace = HorizontalWhitespace::None;
        let count = count - u32::from(literal_ended_line);
        for _ in 0..count {
            self.write_sink_str("\n");
            self.column = TextWidth::ZERO;
        }
        let (indent, width) = self.pending_newline_indent(indent_delta);
        self.pending_indent = indent;
        self.column = width;
    }

    fn pending_newline_indent(&self, indent_delta: i16) -> (u32, TextWidth) {
        let effective_levels = (self.indent_levels + i32::from(indent_delta))
            .max(0)
            .cast_unsigned();
        let width = effective_levels * u32::from(self.options.indent_width);
        let indent_count = if self.options.use_tabs {
            effective_levels
        } else {
            width
        };
        (indent_count, TextWidth::new(width))
    }

    fn write_str(&mut self, text: &str) {
        if text.is_empty() || self.halted {
            return;
        }
        if matches!(text.as_bytes().first(), Some(b'\n' | b'\r')) {
            self.pending_indent = 0;
            self.horizontal_whitespace = HorizontalWhitespace::None;
        } else if matches!(text.as_bytes().first(), Some(b' ' | b'\t')) {
            self.horizontal_whitespace = HorizontalWhitespace::None;
        }
        if self.pending_indent > 0 {
            self.horizontal_whitespace = HorizontalWhitespace::None;
        }
        self.flush_pending_indent();
        self.flush_pending_spaces();
        self.write_sink_str(text);
    }

    fn write_sink_str(&mut self, text: &str) {
        if text.is_empty() || self.halted {
            return;
        }
        match self.sink.write_str(text) {
            RenderControl::Continue => {
                self.horizontal_whitespace = if matches!(text.as_bytes().last(), Some(b' ' | b'\t'))
                {
                    HorizontalWhitespace::Emitted
                } else {
                    HorizontalWhitespace::None
                };
            }
            RenderControl::Halt => {
                self.halted = true;
            }
        }
    }

    fn flush_pending_indent(&mut self) {
        let count = std::mem::take(&mut self.pending_indent);
        if count == 0 {
            return;
        }
        self.write_repeated(if self.options.use_tabs { '\t' } else { ' ' }, count);
    }

    fn flush_pending_spaces(&mut self) {
        if self.horizontal_whitespace == HorizontalWhitespace::Pending {
            self.write_sink_str(" ");
            self.add_width(TextWidth::new(1));
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
        while remaining > 0 && !self.halted {
            let write_len = remaining.min(chunk_len);
            self.write_sink_str(
                &chunk[..usize::try_from(write_len).expect("chunk length fits usize")],
            );
            remaining -= write_len;
        }
    }

    fn add_width(&mut self, width: TextWidth) {
        self.column = add_width(self.column, width);
    }
}

struct FitChecker<'base, 'scratch, 'source> {
    arena: &'base DocArena<'source>,
    line_width: TextWidth,
    column: TextWidth,
    horizontal_whitespace: HorizontalWhitespace,
    pending_indent: bool,
    base_group_stack: &'base [Mode],
    base_group_len: usize,
    group_stack: &'scratch mut Vec<Mode>,
    remaining_commands: usize,
    measured_group_active: bool,
}

impl<'base, 'scratch, 'source> FitChecker<'base, 'scratch, 'source> {
    fn from_renderer<S>(
        renderer: &'base Renderer<'_, '_, 'source, S>,
        group_stack: &'scratch mut Vec<Mode>,
    ) -> Self {
        Self {
            arena: renderer.arena,
            line_width: TextWidth::from(renderer.options.line_width),
            column: renderer.column,
            horizontal_whitespace: renderer.horizontal_whitespace,
            pending_indent: renderer.pending_indent > 0,
            base_group_stack: &renderer.group_stack,
            base_group_len: renderer.group_stack.len(),
            group_stack,
            remaining_commands: FLAT_FIT_COMMAND_BUDGET,
            measured_group_active: true,
        }
    }

    fn fits_stack(&mut self, stack: &mut FitStack<'_, '_, 'source>) -> bool {
        while let Some(command) = stack.pop() {
            if !matches!(command, FitCommand::ConcatRange { .. }) {
                let Some(remaining_commands) = self.remaining_commands.checked_sub(1) else {
                    return false;
                };
                self.remaining_commands = remaining_commands;
            }
            match self.fit_command(command, stack) {
                FitResult::Continue => {}
                FitResult::Done => return true,
                FitResult::No => return false,
            }
        }

        self.column <= self.line_width
    }

    fn fit_command(
        &mut self,
        command: FitCommand<'source>,
        stack: &mut FitStack<'_, '_, 'source>,
    ) -> FitResult {
        match command {
            FitCommand::Doc(doc, mode) => self.fit_doc(doc, mode, stack),
            FitCommand::ConcatRange { next, end, mode } => {
                self.fit_concat_range(next, end, mode, stack)
            }
            FitCommand::EndIndent => FitResult::Continue,
            FitCommand::EndMeasuredGroup => {
                self.group_stack.pop();
                self.measured_group_active = false;
                FitResult::Continue
            }
            FitCommand::EndGroup => {
                if self.group_stack.pop().is_none() {
                    self.base_group_len = self.base_group_len.saturating_sub(1);
                }
                FitResult::Continue
            }
        }
    }

    fn fit_doc(
        &mut self,
        doc: Doc<'source>,
        mode: Mode,
        stack: &mut FitStack<'_, '_, 'source>,
    ) -> FitResult {
        let arena = self.arena;
        match arena.node(doc) {
            None => FitResult::Continue,
            Some(DocNode::Text(text)) if text.text == " " => {
                if self.horizontal_whitespace == HorizontalWhitespace::None {
                    self.horizontal_whitespace = HorizontalWhitespace::Pending;
                }
                FitResult::Continue
            }
            Some(DocNode::Text(text)) if text.is_multiline() => {
                if self.measured_group_active {
                    return FitResult::No;
                }
                if self.text_width_result(&text.text, text.first_width()) == FitResult::No {
                    FitResult::No
                } else {
                    FitResult::Done
                }
            }
            Some(DocNode::Text(text)) => self.text_width_result(&text.text, text.final_width()),
            Some(DocNode::InlineConcat { docs, len }) => {
                for child in docs[..usize::from(*len)].iter().rev() {
                    stack.push(FitCommand::Doc(*child, mode));
                }
                FitResult::Continue
            }
            Some(DocNode::ConcatRange { start, len }) => {
                if let Some(end) = start.checked_add(*len) {
                    stack.push(FitCommand::ConcatRange {
                        next: *start,
                        end,
                        mode,
                    });
                }
                FitResult::Continue
            }
            Some(DocNode::Group {
                contents,
                should_break,
            }) => self.fit_group(*contents, *should_break, mode, stack),
            Some(DocNode::Indent { contents, .. }) => {
                stack.push(FitCommand::EndIndent);
                stack.push(FitCommand::Doc(*contents, mode));
                FitResult::Continue
            }
            Some(DocNode::Line(line)) => self.fit_line(line, mode),
            Some(DocNode::IfBreak { breaks, flat }) => {
                let is_broken = self.group_is_broken();
                stack.push(FitCommand::Doc(
                    if is_broken { *breaks } else { *flat },
                    mode,
                ));
                FitResult::Continue
            }
        }
    }

    fn fit_concat_range(
        &mut self,
        next: u32,
        end: u32,
        mode: Mode,
        stack: &mut FitStack<'_, '_, 'source>,
    ) -> FitResult {
        if next >= end {
            return FitResult::Continue;
        }

        let following = next + 1;
        if following < end {
            stack.push(FitCommand::ConcatRange {
                next: following,
                end,
                mode,
            });
        }
        stack.push(FitCommand::Doc(self.arena.child(next), mode));
        FitResult::Continue
    }

    fn fit_group(
        &mut self,
        contents: Doc<'source>,
        should_break: bool,
        mode: Mode,
        stack: &mut FitStack<'_, '_, 'source>,
    ) -> FitResult {
        if mode == Mode::Flat && should_break {
            return FitResult::No;
        }
        let group_mode = mode;
        self.group_stack.push(group_mode);
        stack.push(FitCommand::EndGroup);
        stack.push(FitCommand::Doc(contents, group_mode));
        FitResult::Continue
    }

    fn fit_group_flat_with_stack(
        &mut self,
        contents: Doc<'source>,
        stack: &[RenderCommand<'source>],
        overlay: &mut Vec<FitCommand<'source>>,
    ) -> bool {
        self.group_stack.push(Mode::Flat);
        let mut fit_stack = FitStack::new(stack, overlay);
        fit_stack.push(FitCommand::EndMeasuredGroup);
        fit_stack.push(FitCommand::Doc(contents, Mode::Flat));
        self.fits_stack(&mut fit_stack)
    }

    fn fit_line(&mut self, line: &Line, mode: Mode) -> FitResult {
        match (mode, line.mode) {
            (Mode::Flat, LineMode::Soft | LineMode::SoftOrSpace) => self.fit_flat_line(&line.flat),
            (Mode::Flat, LineMode::Hard | LineMode::Empty) => FitResult::No,
            (Mode::Break, _) => {
                if self.column <= self.line_width {
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
            FlatLine::Space => {
                if self.horizontal_whitespace == HorizontalWhitespace::None {
                    self.horizontal_whitespace = HorizontalWhitespace::Pending;
                }
                FitResult::Continue
            }
        }
    }

    fn group_is_broken(&self) -> bool {
        self.group_stack
            .last()
            .or_else(|| self.base_group_stack[..self.base_group_len].last())
            .copied()
            .is_some_and(|mode| mode == Mode::Break)
    }

    fn width_result(&mut self, width: TextWidth) -> FitResult {
        self.column = add_width(self.column, width);
        if self.column <= self.line_width {
            FitResult::Continue
        } else {
            FitResult::No
        }
    }

    fn text_width_result(&mut self, text: &str, width: TextWidth) -> FitResult {
        if self.pending_indent {
            self.horizontal_whitespace = HorizontalWhitespace::None;
            self.pending_indent = false;
        }
        if matches!(text.as_bytes().first(), Some(b' ' | b'\t')) {
            self.horizontal_whitespace = HorizontalWhitespace::None;
        }
        if self.horizontal_whitespace == HorizontalWhitespace::Pending
            && self.width_result(TextWidth::new(1)) == FitResult::No
        {
            return FitResult::No;
        }
        let result = self.width_result(width);
        self.horizontal_whitespace = if matches!(text.as_bytes().last(), Some(b' ' | b'\t')) {
            HorizontalWhitespace::Emitted
        } else {
            HorizontalWhitespace::None
        };
        result
    }
}

struct FitStack<'stack, 'scratch, 'source> {
    base: &'stack [RenderCommand<'source>],
    base_next: usize,
    overlay: &'scratch mut Vec<FitCommand<'source>>,
}

impl<'stack, 'scratch, 'source> FitStack<'stack, 'scratch, 'source> {
    fn new(
        base: &'stack [RenderCommand<'source>],
        overlay: &'scratch mut Vec<FitCommand<'source>>,
    ) -> Self {
        overlay.clear();
        Self {
            base,
            base_next: base.len(),
            overlay,
        }
    }

    fn push(&mut self, command: FitCommand<'source>) {
        self.overlay.push(command);
    }

    fn pop(&mut self) -> Option<FitCommand<'source>> {
        self.overlay.pop().or_else(|| {
            self.base_next = self.base_next.checked_sub(1)?;
            Some(self.base[self.base_next].into())
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FitResult {
    Continue,
    Done,
    No,
}

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use jolt_diagnostics::{Diagnostic, DiagnosticCodeId};
    use jolt_java_syntax::{JavaLanguage, JavaSyntaxKind, JavaSyntaxView, parse_compilation_unit};
    use jolt_syntax::{
        BuildSyntaxTreeError, Event, FactoryNode, Language, LanguageLexer, LexedToken,
        NormalizedToken, ParsedChildren, RawSyntaxKind, RemovalClaim, RemovalReason,
        ReplacementClaim, SourceIdentity, SourceTokenId, SyntaxFactory, SyntaxNode,
        SyntaxTokenData, SyntaxTreeSink, SyntaxTrivia, TriviaKind, build_syntax_tree_with_factory,
    };
    #[cfg(debug_assertions)]
    use jolt_syntax::{
        ConservationError, NormalizationOperation, ReorderClaim, ReorderReason, SourceTriviaSide,
        SynthesisClaim,
    };
    use jolt_text::{TextRange, TextSize};

    use crate::document::DocArena;
    #[cfg(debug_assertions)]
    use crate::document::DocNode;
    use crate::formatter_ignore::formatter_ignore_plan_with_safety;
    #[cfg(debug_assertions)]
    use crate::source_fragment::{ExceptionalSeparators, SourceClaim};
    use crate::source_fragment::{FragmentBoundary, exceptional_separators};
    use crate::{
        Doc, DocBuilder, ExceptionalSeparator, FormatOptions, LexicalAtom, LexicalAtomKind,
        LexicalSafety, RenderControl, RenderSink,
    };

    use super::{RenderError, render_source_to};

    #[derive(Default)]
    struct StringSink(String);

    struct ClaimLanguage;
    struct ClaimLexer;
    struct ClaimFactory;

    impl SyntaxFactory for ClaimFactory {
        fn make_syntax(
            &self,
            kind: RawSyntaxKind,
            _children: ParsedChildren<'_>,
            sink: &mut SyntaxTreeSink<'_>,
        ) -> Result<FactoryNode, BuildSyntaxTreeError> {
            if kind == JavaLanguage::kind_to_raw(JavaSyntaxKind::BogusExpression) {
                Ok(sink.raw_malformed(kind))
            } else {
                Ok(sink.raw(kind))
            }
        }
    }

    impl Language for ClaimLanguage {
        type Kind = JavaSyntaxKind;
        type Lexer<'source> = ClaimLexer;
        type NormalizationAuthority = ();

        fn kind_from_raw(raw: RawSyntaxKind) -> Self::Kind {
            JavaLanguage::kind_from_raw(raw)
        }

        fn kind_to_raw(kind: Self::Kind) -> RawSyntaxKind {
            JavaLanguage::kind_to_raw(kind)
        }

        fn eof_kind() -> Self::Kind {
            JavaLanguage::eof_kind()
        }

        fn expected_diagnostic_code() -> DiagnosticCodeId {
            DiagnosticCodeId::new("test.expected")
        }

        fn unexpected_diagnostic_code() -> DiagnosticCodeId {
            DiagnosticCodeId::new("test.unexpected")
        }

        fn split_token(_token: &LexedToken<Self>) -> Option<&'static [Self::Kind]> {
            None
        }
    }

    impl<'source> LanguageLexer<'source> for ClaimLexer {
        type Language = ClaimLanguage;

        fn new(_source: &'source str) -> Self {
            Self
        }

        fn next_token_into(
            &mut self,
            _trivia: &mut Vec<SyntaxTrivia>,
        ) -> LexedToken<Self::Language> {
            panic!("tests construct tokens directly")
        }

        fn finish(self) -> Vec<Diagnostic> {
            Vec::new()
        }
    }

    fn replacement_claim<'tree>(
        owner: &SyntaxNode<'tree, ClaimLanguage>,
        source: SourceTokenId<'tree>,
        token: NormalizedToken,
    ) -> ReplacementClaim<'tree> {
        let owner = jolt_syntax::NormalizationOwner::authorized((), owner)
            .expect("test owner is recovery-free");
        ReplacementClaim::authorized::<ClaimLanguage>(owner, source, token)
    }

    fn removal_claim<'tree>(
        owner: &SyntaxNode<'tree, ClaimLanguage>,
        source: SourceIdentity<'tree>,
        reason: RemovalReason,
    ) -> RemovalClaim<'tree> {
        let owner = jolt_syntax::NormalizationOwner::authorized((), owner)
            .expect("test owner is recovery-free");
        RemovalClaim::authorized::<ClaimLanguage>(owner, source, reason)
    }

    #[cfg(debug_assertions)]
    fn synthesis_claim<'tree>(
        owner: &SyntaxNode<'tree, ClaimLanguage>,
        anchor: SourceTokenId<'tree>,
        token: NormalizedToken,
    ) -> SynthesisClaim<'tree> {
        let owner = jolt_syntax::NormalizationOwner::authorized((), owner)
            .expect("test owner is recovery-free");
        SynthesisClaim::authorized::<ClaimLanguage>(owner, anchor, token)
    }

    #[cfg(debug_assertions)]
    fn reorder_claim<'tree>(
        owner: &SyntaxNode<'tree, ClaimLanguage>,
        anchor: SourceTokenId<'tree>,
        reason: ReorderReason,
    ) -> ReorderClaim<'tree> {
        let owner = jolt_syntax::NormalizationOwner::authorized((), owner)
            .expect("test owner is recovery-free");
        ReorderClaim::authorized::<ClaimLanguage>(owner, anchor, reason)
    }

    impl RenderSink for StringSink {
        fn write_str(&mut self, text: &str) -> RenderControl {
            self.0.push_str(text);
            RenderControl::Continue
        }
    }

    fn options() -> FormatOptions {
        FormatOptions {
            line_width: 80,
            indent_width: 4,
            use_tabs: false,
        }
    }

    #[cfg(debug_assertions)]
    fn conservation_error(error: super::RenderError) -> ConservationError {
        match error.kind {
            super::RenderErrorKind::Conservation(error) => error,
            other => panic!("expected conservation error, got {other:?}"),
        }
    }

    fn empty_boundary() -> FragmentBoundary<'static> {
        FragmentBoundary {
            first: None,
            last: None,
            ends_with_line_comment: false,
        }
    }

    fn syntax_tree_with_root_kind(
        source: &str,
        root_kind: JavaSyntaxKind,
    ) -> jolt_syntax::SyntaxTree {
        let mut offset = 0;
        let tokens = source
            .char_indices()
            .map(|(start, character)| {
                offset = start + character.len_utf8();
                SyntaxTokenData::new(
                    RawSyntaxKind::new(1),
                    TextRange::new(TextSize::new(start), TextSize::new(offset)),
                    TextRange::new(TextSize::new(start), TextSize::new(offset)),
                    Range::default(),
                    Range::default(),
                )
            })
            .collect::<Vec<_>>();
        let mut events = Vec::with_capacity(tokens.len() + 2);
        events.push(Event::Start {
            kind: JavaLanguage::kind_to_raw(root_kind),
            forward_parent: 0,
        });
        events.extend((0..tokens.len()).map(|_| Event::Token));
        events.push(Event::Finish);
        build_syntax_tree_with_factory("", events, tokens, Vec::new(), &ClaimFactory)
            .expect("test syntax tree builds")
    }

    fn syntax_tree(source: &str) -> jolt_syntax::SyntaxTree {
        syntax_tree_with_root_kind(source, JavaSyntaxKind::CompilationUnit)
    }

    fn render_to<S: RenderSink>(
        arena: &DocArena<'_>,
        doc: Doc<'_>,
        options: FormatOptions,
        sink: S,
    ) -> Result<bool, RenderError> {
        let source = "";
        let tree = syntax_tree(source);
        let root = SyntaxNode::<ClaimLanguage>::new_root(source, &tree);
        render_source_to(arena, doc, options, sink, &root).map(super::SourceRenderOutcome::halted)
    }

    fn bogus_syntax_tree(source: &str) -> jolt_syntax::SyntaxTree {
        syntax_tree_with_root_kind(source, JavaSyntaxKind::BogusExpression)
    }

    #[cfg(debug_assertions)]
    fn mixed_syntax_tree() -> jolt_syntax::SyntaxTree {
        let source = "ab";
        let tokens = source
            .char_indices()
            .map(|(start, character)| {
                let end = start + character.len_utf8();
                SyntaxTokenData::new(
                    RawSyntaxKind::new(1),
                    TextRange::new(TextSize::new(start), TextSize::new(end)),
                    TextRange::new(TextSize::new(start), TextSize::new(end)),
                    Range::default(),
                    Range::default(),
                )
            })
            .collect();
        let events = vec![
            Event::Start {
                kind: JavaLanguage::kind_to_raw(JavaSyntaxKind::CompilationUnit),
                forward_parent: 0,
            },
            Event::Token,
            Event::Start {
                kind: JavaLanguage::kind_to_raw(JavaSyntaxKind::BogusExpression),
                forward_parent: 0,
            },
            Event::Token,
            Event::Finish,
            Event::Finish,
        ];
        build_syntax_tree_with_factory("", events, tokens, Vec::new(), &ClaimFactory)
            .expect("mixed test syntax tree builds")
    }

    fn syntax_tree_with_line_comment() -> jolt_syntax::SyntaxTree {
        let token = SyntaxTokenData::new(
            RawSyntaxKind::new(1),
            TextRange::new(TextSize::new(0), TextSize::new(5)),
            TextRange::new(TextSize::new(0), TextSize::new(1)),
            Range::default(),
            0..2,
        );
        let trivia = vec![
            SyntaxTrivia::new(TriviaKind::LineComment, TextSize::new(3)),
            SyntaxTrivia::new(TriviaKind::Newline, TextSize::new(1)),
        ];
        let events = vec![
            Event::Start {
                kind: JavaLanguage::kind_to_raw(JavaSyntaxKind::CompilationUnit),
                forward_parent: 0,
            },
            Event::Token,
            Event::Finish,
        ];
        build_syntax_tree_with_factory("", events, vec![token], trivia, &ClaimFactory)
            .expect("comment test syntax tree builds")
    }

    #[cfg(debug_assertions)]
    #[test]
    fn malformed_verbatim_is_one_tracked_borrowed_core() {
        let source = "a+b";
        let tree = bogus_syntax_tree(source);
        let root = SyntaxNode::<ClaimLanguage>::new_root(source, &tree);
        let core = root
            .malformed_verbatim_core()
            .expect("error node owns a verbatim core");
        let mut builder = DocBuilder::new();
        let fragment = builder.malformed_verbatim(&core, empty_boundary());
        let mut safety = CountingSafety::default();
        let document = builder.resolve_exceptional(fragment, None, None, &mut safety);
        let arena = builder.into_arena();
        let mut sink = StringSink::default();
        let outcome = render_source_to(&arena, document, options(), &mut sink, &root)
            .expect("verbatim document renders");

        assert_eq!(sink.0, source);
        assert!(outcome.used_malformed_verbatim());
    }

    #[test]
    fn valid_node_cannot_construct_a_malformed_verbatim_core() {
        let source = "valid";
        let tree = syntax_tree(source);
        let root = SyntaxNode::<ClaimLanguage>::new_root(source, &tree);
        assert!(root.malformed_verbatim_core().is_none());
    }

    #[cfg(debug_assertions)]
    #[test]
    fn structured_token_and_malformed_core_complete_one_source_render() {
        let source = "ab";
        let tree = mixed_syntax_tree();
        let root = SyntaxNode::<ClaimLanguage>::new_root(source, &tree);
        let first = root.first_token().expect("structured token");
        let malformed = root.children().next().expect("malformed child");
        let core = malformed
            .malformed_verbatim_core()
            .expect("child owns a verbatim core");
        let atom = LexicalAtom::new(LexicalAtomKind::Identifier, "b");
        let mut builder = DocBuilder::new();
        let structured = builder.source_token(&first);
        let malformed = builder.malformed_verbatim(
            &core,
            FragmentBoundary {
                first: Some(atom),
                last: Some(atom),
                ends_with_line_comment: false,
            },
        );
        let mut safety = CountingSafety::default();
        let malformed = builder.resolve_exceptional(malformed, Some(&first), None, &mut safety);
        let document = builder.concat([structured, malformed]);
        let arena = builder.into_arena();
        let mut sink = StringSink::default();

        let outcome = render_source_to(&arena, document, options(), &mut sink, &root)
            .expect("structured and malformed source completes");

        assert_eq!(sink.0, "ab");
        assert_eq!(safety.0, 0);
        assert!(outcome.used_malformed_verbatim());
    }

    #[test]
    fn structured_comment_claims_its_line_terminator() {
        let source = "x//c\n";
        let tree = syntax_tree_with_line_comment();
        let root = SyntaxNode::<ClaimLanguage>::new_root(source, &tree);
        let token = root.first_token().expect("source token");
        let trivia = token.trailing_trivia_with_ids().collect::<Vec<_>>();
        let mut builder = DocBuilder::new();
        let token_doc = builder.source_token(&token);
        let comment_doc = builder.source_trivia(trivia, |docs| docs.literal_text("//c"));
        let line = builder.hard_line();
        let document = builder.concat([token_doc, comment_doc, line]);
        let arena = builder.into_arena();
        let mut sink = StringSink::default();

        render_source_to(&arena, document, options(), &mut sink, &root)
            .expect("structured token and comment complete");

        assert_eq!(sink.0, source);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn source_looking_ordinary_text_cannot_complete_conservation() {
        let source = "x";
        let tree = syntax_tree(source);
        let root = SyntaxNode::<ClaimLanguage>::new_root(source, &tree);
        let mut builder = DocBuilder::new();
        let ordinary = builder.text(source);
        let arena = builder.into_arena();
        let mut sink = StringSink::default();

        let error = render_source_to(&arena, ordinary, options(), &mut sink, &root)
            .expect_err("ordinary text cannot stand in for structured source");

        assert_eq!(
            conservation_error(error),
            ConservationError::MissingToken {
                token: 0,
                range: TextRange::new(TextSize::new(0), TextSize::new(1)),
            }
        );
        assert!(sink.0.is_empty());
    }

    #[cfg(debug_assertions)]
    #[test]
    fn duplicate_claim_is_rejected_before_any_output() {
        let source = "ab";
        let tree = syntax_tree(source);
        let root = SyntaxNode::<ClaimLanguage>::new_root(source, &tree);
        let token = root.first_token().expect("first token");
        let mut builder = DocBuilder::new();
        let first = builder.source_token(&token);
        let duplicate = builder.source_token(&token);
        let document = builder.concat([first, duplicate]);
        let arena = builder.into_arena();
        let mut sink = StringSink::default();

        let error = render_source_to(&arena, document, options(), &mut sink, &root)
            .expect_err("duplicate claim must fail conservation");

        assert_eq!(
            conservation_error(error),
            ConservationError::DuplicateToken {
                token: 0,
                range: TextRange::new(TextSize::new(0), TextSize::new(1)),
            }
        );
        assert!(sink.0.is_empty());
    }

    // A valid formatter cannot produce a duplicate replacement claim for a fixture to trigger.
    #[cfg(debug_assertions)]
    #[test]
    fn replacement_failure_reports_its_operation_and_source() {
        let source = "x";
        let tree = syntax_tree(source);
        let root = SyntaxNode::<ClaimLanguage>::new_root(source, &tree);
        let token = root.first_token().expect("source token");
        let mut builder = DocBuilder::new();
        let source_doc = builder.source_token(&token);
        let replacement = builder.replaced_source(replacement_claim(
            &root,
            token.source_id(),
            NormalizedToken::EnumComma,
        ));
        let document = builder.concat([source_doc, replacement]);
        let arena = builder.into_arena();
        let mut sink = StringSink::default();

        let error = render_source_to(&arena, document, options(), &mut sink, &root)
            .expect_err("duplicate replacement source must fail conservation");

        assert_eq!(
            conservation_error(error),
            ConservationError::Normalization {
                operation: NormalizationOperation::Replacement(NormalizedToken::EnumComma),
                error: Box::new(ConservationError::DuplicateToken {
                    token: 0,
                    range: TextRange::new(TextSize::new(0), TextSize::new(1)),
                }),
            }
        );
        assert!(sink.0.is_empty());
    }

    // A valid formatter cannot produce a duplicate removal claim for a fixture to trigger.
    #[cfg(debug_assertions)]
    #[test]
    fn removal_failure_reports_its_reason_and_trivia() {
        let source = "x//c\n";
        let tree = syntax_tree_with_line_comment();
        let root = SyntaxNode::<ClaimLanguage>::new_root(source, &tree);
        let token = root.first_token().expect("source token");
        let trivia = token.trailing_trivia_with_ids().collect::<Vec<_>>();
        let comment = trivia[0];
        let mut builder = DocBuilder::new();
        let token_doc = builder.source_token(&token);
        let comment_doc = builder.source_trivia(trivia, |docs| docs.literal_text("//c"));
        let line = builder.hard_line();
        let removal = builder.removed_source(removal_claim(
            &root,
            SourceIdentity::Trivia(comment.id()),
            RemovalReason::DuplicateImport,
        ));
        let document = builder.concat([token_doc, comment_doc, line, removal]);
        let arena = builder.into_arena();
        let mut sink = StringSink::default();

        let error = render_source_to(&arena, document, options(), &mut sink, &root)
            .expect_err("duplicate removed trivia must fail conservation");

        assert_eq!(
            conservation_error(error),
            ConservationError::Normalization {
                operation: NormalizationOperation::Removal(RemovalReason::DuplicateImport),
                error: Box::new(ConservationError::DuplicateTrivia {
                    token: 0,
                    side: SourceTriviaSide::Trailing,
                    ordinal: 0,
                    kind: TriviaKind::LineComment,
                    range: TextRange::new(TextSize::new(1), TextSize::new(4)),
                }),
            }
        );
        assert!(sink.0.is_empty());
    }

    #[cfg(debug_assertions)]
    #[test]
    fn zero_token_malformed_core_records_dispatch() {
        let source = "";
        let tree = bogus_syntax_tree(source);
        let root = SyntaxNode::<ClaimLanguage>::new_root(source, &tree);
        let core = root
            .malformed_verbatim_core()
            .expect("empty malformed node owns a core");
        let mut builder = DocBuilder::new();
        let malformed = builder.malformed_verbatim(&core, empty_boundary());
        let mut safety = CountingSafety::default();
        let document = builder.resolve_exceptional(malformed, None, None, &mut safety);
        let arena = builder.into_arena();
        let mut sink = StringSink::default();

        let outcome = render_source_to(&arena, document, options(), &mut sink, &root)
            .expect("empty malformed source completes");

        assert!(outcome.used_malformed_verbatim());
    }

    #[test]
    fn halted_tracked_render_does_not_expose_a_completed_proof() {
        struct HaltOnIndent(String);

        impl RenderSink for HaltOnIndent {
            fn write_str(&mut self, text: &str) -> RenderControl {
                self.0.push_str(text);
                if !text.is_empty() && text.chars().all(|character| character == ' ') {
                    RenderControl::Halt
                } else {
                    RenderControl::Continue
                }
            }
        }

        let source = "x";
        let tree = syntax_tree(source);
        let root = SyntaxNode::<ClaimLanguage>::new_root(source, &tree);
        let token = root.first_token().expect("source token");
        let mut builder = DocBuilder::new();
        let line = builder.hard_line();
        let token = builder.source_token(&token);
        let contents = builder.concat([line, token]);
        let document = builder.indent(contents);
        let arena = builder.into_arena();
        let mut sink = HaltOnIndent(String::new());
        let outcome = render_source_to(&arena, document, options(), &mut sink, &root)
            .expect("an intentional halt is not a conservation error");

        assert!(outcome.halted());
        assert!(!outcome.used_malformed_verbatim());
        assert_eq!(sink.0, "\n    ");
    }

    #[test]
    fn only_rendered_if_break_branch_consumes_claims() {
        let source = "ab";
        let tree = syntax_tree(source);
        let root = SyntaxNode::<ClaimLanguage>::new_root(source, &tree);
        let mut tokens = root.tokens();
        let first = tokens.next().expect("first token").source_id();
        let second = tokens.next().expect("second token").source_id();
        let mut builder = DocBuilder::new();
        let breaks =
            builder.replaced_source(replacement_claim(&root, first, NormalizedToken::EnumComma));
        let flat = builder.replaced_source(replacement_claim(
            &root,
            second,
            NormalizedToken::EnumSemicolon,
        ));
        let conditional = builder.if_break(breaks, flat);
        let conditional = builder.group(conditional);
        let removed = builder.removed_source(removal_claim(
            &root,
            SourceIdentity::Token(first),
            RemovalReason::DuplicateImport,
        ));
        let document = builder.concat([conditional, removed]);
        let arena = builder.into_arena();
        let mut sink = StringSink::default();
        let outcome = render_source_to(&arena, document, options(), &mut sink, &root)
            .expect("selected branch renders without a duplicate claim");

        assert_eq!(sink.0, ";");
        assert!(!outcome.used_malformed_verbatim());
    }

    #[cfg(debug_assertions)]
    #[test]
    fn synthesis_rejects_a_foreign_anchor() {
        let source = "x";
        let tree = syntax_tree(source);
        let root = SyntaxNode::<ClaimLanguage>::new_root(source, &tree);
        let other_tree = syntax_tree("y");
        let other_root = SyntaxNode::<ClaimLanguage>::new_root("y", &other_tree);
        let foreign = other_root.first_token().expect("foreign token").source_id();
        let mut builder = DocBuilder::new();
        let synthesized = builder.synthesized_source(synthesis_claim(
            &other_root,
            foreign,
            NormalizedToken::EnumSemicolon,
        ));
        let arena = builder.into_arena();
        let mut sink = StringSink::default();
        let Err(error) = render_source_to(&arena, synthesized, options(), &mut sink, &root) else {
            panic!("foreign synthesis anchor must be rejected");
        };

        assert_eq!(
            conservation_error(error),
            ConservationError::Normalization {
                operation: NormalizationOperation::Synthesis(NormalizedToken::EnumSemicolon),
                error: Box::new(ConservationError::ForeignToken {
                    token: 0,
                    range: TextRange::new(TextSize::new(0), TextSize::new(1)),
                }),
            }
        );
        assert!(sink.0.is_empty());
    }

    #[test]
    fn unselected_exceptional_branch_does_not_consume_its_claim() {
        let source = "";
        let tree = bogus_syntax_tree(source);
        let root = SyntaxNode::<ClaimLanguage>::new_root(source, &tree);
        let core = root
            .malformed_verbatim_core()
            .expect("error node owns a verbatim core");
        let mut builder = DocBuilder::new();
        let exceptional = builder.malformed_verbatim(&core, empty_boundary());
        let mut safety = CountingSafety::default();
        let exceptional = builder.resolve_exceptional(exceptional, None, None, &mut safety);
        let ordinary = builder.text("ordinary");
        let conditional = builder.if_break(exceptional, ordinary);
        let document = builder.group(conditional);
        let arena = builder.into_arena();
        let mut sink = StringSink::default();

        render_to(&arena, document, options(), &mut sink)
            .expect("unselected exceptional branch is not visited");

        assert_eq!(sink.0, "ordinary");
    }

    #[derive(Default)]
    struct CountingSafety(usize);

    impl LexicalSafety<ClaimLanguage> for CountingSafety {
        fn classify(
            &mut self,
            _token: &jolt_syntax::SyntaxToken<'_, ClaimLanguage>,
        ) -> LexicalAtomKind {
            LexicalAtomKind::Identifier
        }

        fn separator(
            &mut self,
            _left: LexicalAtom<'_>,
            _right: LexicalAtom<'_>,
        ) -> ExceptionalSeparator {
            self.0 += 1;
            ExceptionalSeparator::Space
        }
    }

    #[derive(Default)]
    struct CountingJavaSafety(usize);

    impl LexicalSafety<JavaLanguage> for CountingJavaSafety {
        fn classify(
            &mut self,
            _token: &jolt_syntax::SyntaxToken<'_, JavaLanguage>,
        ) -> LexicalAtomKind {
            self.0 += 1;
            LexicalAtomKind::Identifier
        }

        fn separator(
            &mut self,
            _left: LexicalAtom<'_>,
            _right: LexicalAtom<'_>,
        ) -> ExceptionalSeparator {
            ExceptionalSeparator::Space
        }
    }

    #[test]
    fn formatter_ignore_boundary_scan_is_linearly_bounded() {
        let source = "class C {\n\
            // @formatter:off\n\
            int first=1;\n\
            // @formatter:on\n\
            // @formatter:off\n\
            int second=2;\n\
            // @formatter:on\n\
            }\n";
        let parse = parse_compilation_unit(source);
        let syntax = parse.syntax().expect("test source parses");
        let root = syntax.syntax_node().expect("test syntax has a root");
        let token_count = root.tokens().count();
        let mut safety = CountingJavaSafety::default();

        let plan = formatter_ignore_plan_with_safety(source, root.tokens(), &mut safety);

        assert_eq!(plan.test_range_count(), 2);
        assert!(
            safety.0 <= token_count * 4,
            "{} classifications for {token_count} tokens",
            safety.0
        );
    }

    #[test]
    fn exceptional_lexical_safety_is_bounded_and_line_comment_aware() {
        let source = "x";
        let tree = bogus_syntax_tree(source);
        let root = SyntaxNode::<ClaimLanguage>::new_root(source, &tree);
        let core = root
            .malformed_verbatim_core()
            .expect("error node owns a verbatim core");
        let atom = LexicalAtom::new(LexicalAtomKind::Identifier, "x");
        let mut builder = DocBuilder::new();
        let fragment = builder.malformed_verbatim(
            &core,
            FragmentBoundary {
                first: Some(atom),
                last: Some(atom),
                ends_with_line_comment: true,
            },
        );
        let mut safety = CountingSafety::default();

        let separators =
            exceptional_separators::<ClaimLanguage>(Some(atom), fragment, Some(atom), &mut safety);

        assert_eq!(separators.before, ExceptionalSeparator::Space);
        assert_eq!(separators.after, ExceptionalSeparator::HardLine);
        assert_eq!(
            safety.0, 1,
            "line-comment override avoids a second callback"
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    fn reorder_reason_is_carried_by_the_selected_document() {
        let source = "x";
        let tree = syntax_tree(source);
        let root = SyntaxNode::<ClaimLanguage>::new_root(source, &tree);
        let token = root.first_token().expect("source token");
        let mut builder = DocBuilder::new();
        let token_doc = builder.source_token(&token);
        let document = builder.reordered_source(
            token_doc,
            reorder_claim(&root, token.source_id(), ReorderReason::ModuleDirectives),
        );
        let arena = builder.into_arena();
        let marker = match arena.node(document) {
            Some(DocNode::InlineConcat { docs, .. }) => docs[0],
            other => panic!("reorder wrapper must retain a branch-local marker: {other:?}"),
        };
        let Some(DocNode::Text(text)) = arena.node(marker) else {
            panic!("reorder marker must be a source-aware text node");
        };
        assert!(matches!(
            text.claim,
            Some(SourceClaim::Reordered {
                reason: ReorderReason::ModuleDirectives,
                ..
            })
        ));
        let mut sink = StringSink::default();
        render_source_to(&arena, document, options(), &mut sink, &root)
            .expect("reason-tagged reorder completes");
        assert_eq!(sink.0, source);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn foreign_reorder_owner_is_rejected_before_output() {
        let source = "x";
        let tree = syntax_tree(source);
        let root = SyntaxNode::<ClaimLanguage>::new_root(source, &tree);
        let other_tree = syntax_tree("y");
        let other_root = SyntaxNode::<ClaimLanguage>::new_root("y", &other_tree);
        let foreign = other_root.first_token().expect("foreign token");
        let mut builder = DocBuilder::new();
        let contents = builder.text("generated");
        let document = builder.reordered_source(
            contents,
            reorder_claim(&other_root, foreign.source_id(), ReorderReason::Imports),
        );
        let arena = builder.into_arena();
        let mut sink = StringSink::default();

        let error = render_source_to(&arena, document, options(), &mut sink, &root)
            .expect_err("foreign reorder authority must be rejected");

        assert_eq!(
            conservation_error(error),
            ConservationError::Normalization {
                operation: NormalizationOperation::Reorder(ReorderReason::Imports),
                error: Box::new(ConservationError::ForeignToken {
                    token: 0,
                    range: TextRange::new(TextSize::new(0), TextSize::new(1)),
                }),
            }
        );
        assert!(sink.0.is_empty());
    }

    #[cfg(debug_assertions)]
    #[test]
    fn foreign_formatter_ignore_range_is_rejected_before_separator_output() {
        let source = "x";
        let tree = syntax_tree(source);
        let root = SyntaxNode::<ClaimLanguage>::new_root(source, &tree);
        let other_tree = syntax_tree("y");
        let foreign_root = SyntaxNode::<ClaimLanguage>::new_root("y", &other_tree);
        let foreign = foreign_root.first_token().expect("foreign token");
        let range = foreign.source_range_claim(foreign.token_text_range(), false);
        let mut builder = DocBuilder::new();
        let contents = builder.text("ignored");
        let document = builder.formatter_ignore_source(
            contents,
            range,
            ExceptionalSeparators {
                before: ExceptionalSeparator::Space,
                after: ExceptionalSeparator::None,
            },
        );
        let arena = builder.into_arena();
        let mut sink = StringSink::default();

        let error = render_source_to(&arena, document, options(), &mut sink, &root)
            .expect_err("foreign ignore range must be rejected");

        assert_eq!(
            conservation_error(error),
            ConservationError::ForeignSourceRange {
                range: TextRange::new(TextSize::new(0), TextSize::new(1)),
            }
        );
        assert!(sink.0.is_empty());
    }

    #[cfg(debug_assertions)]
    #[test]
    fn synthesis_reason_is_carried_by_the_selected_document() {
        let source = "x";
        let tree = syntax_tree(source);
        let root = SyntaxNode::<ClaimLanguage>::new_root(source, &tree);
        let token = root.first_token().expect("source token");
        let mut builder = DocBuilder::new();
        let synthesized = builder.synthesized_source(synthesis_claim(
            &root,
            token.source_id(),
            NormalizedToken::EnumSemicolon,
        ));
        let source_doc = builder.source_token(&token);
        let document = builder.concat([source_doc, synthesized]);
        let arena = builder.into_arena();
        let Some(DocNode::Text(text)) = arena.node(synthesized) else {
            panic!("synthesis must be a source-aware text node");
        };
        assert!(matches!(
            text.claim,
            Some(SourceClaim::Synthesized {
                token: NormalizedToken::EnumSemicolon,
                ..
            })
        ));
        let mut sink = StringSink::default();
        render_source_to(&arena, document, options(), &mut sink, &root)
            .expect("reason-tagged synthesis completes");
        assert_eq!(sink.0, "x;");
    }

    #[test]
    fn nested_concat_lists_preserve_order() {
        let mut builder = DocBuilder::new();
        let doc = builder.concat_list(|outer| {
            let a = outer.text("a");
            outer.push(a);
            let inner = outer.concat_list(|inner| {
                let b = inner.text("b");
                inner.push(b);
                let c = inner.text("c");
                inner.push(c);
            });
            outer.push(inner);
            let d = outer.text("d");
            outer.push(d);
        });
        let arena = builder.into_arena();
        let mut sink = StringSink::default();

        render_to(&arena, doc, options(), &mut sink).expect("document renders");

        assert_eq!(sink.0, "abcd");
    }

    #[test]
    fn indentation_is_flushed_only_before_text() {
        let mut builder = DocBuilder::new();
        let contents = builder.concat_list(|contents| {
            let line = contents.hard_line();
            contents.push(line);
            let text = contents.text("indented");
            contents.push(text);
        });
        let doc = builder.indent(contents);
        let arena = builder.into_arena();
        let mut sink = StringSink::default();

        render_to(&arena, doc, options(), &mut sink).expect("document renders");

        assert_eq!(sink.0, "\n    indented");
    }

    #[test]
    fn layout_spaces_are_discarded_before_line_breaks() {
        let mut builder = DocBuilder::new();
        let doc = builder.concat_list(|contents| {
            let text = contents.text("text");
            contents.push(text);
            let space = contents.space();
            contents.push(space);
            let line = contents.hard_line();
            contents.push(line);
            let trailing_space = contents.space();
            contents.push(trailing_space);
        });
        let arena = builder.into_arena();
        let mut sink = StringSink::default();

        render_to(&arena, doc, options(), &mut sink).expect("document renders");

        assert_eq!(sink.0, "text\n");
    }

    #[test]
    fn layout_lines_coalesce_with_a_source_literal_line_ending() {
        for (literal, empty, expected) in [
            ("raw\n", false, "raw\nnext"),
            ("raw\r\n", false, "raw\r\nnext"),
            ("raw\n", true, "raw\n\nnext"),
        ] {
            let mut builder = DocBuilder::new();
            let literal = builder.literal_text(literal);
            let line = if empty {
                builder.empty_line()
            } else {
                builder.hard_line()
            };
            let next = builder.text("next");
            let doc = builder.concat([literal, line, next]);
            let arena = builder.into_arena();
            let mut sink = StringSink::default();

            render_to(&arena, doc, options(), &mut sink).expect("document renders");

            assert_eq!(sink.0, expected);
        }
    }

    #[test]
    fn caught_nested_concat_panic_discards_partial_scratch() {
        let mut builder = DocBuilder::new();
        let doc = builder.concat_list(|outer| {
            let a = outer.text("a");
            outer.push(a);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = outer.concat_list(|inner| {
                    let discarded = inner.text("discarded");
                    inner.push(discarded);
                    panic!("abort nested concat");
                });
            }));
            assert!(result.is_err());
            let b = outer.text("b");
            outer.push(b);
        });
        let arena = builder.into_arena();
        let mut sink = StringSink::default();

        render_to(&arena, doc, options(), &mut sink).expect("document renders");

        assert_eq!(sink.0, "ab");
    }

    #[test]
    fn concat_range_cursors_do_not_consume_the_flat_fit_budget() {
        let mut builder = DocBuilder::new();
        let contents = builder.concat_list(|contents| {
            for _ in 0..2_500 {
                let empty = contents.text("");
                contents.push(empty);
            }
            let line = contents.line();
            contents.push(line);
            let text = contents.text("x");
            contents.push(text);
        });
        let doc = builder.group(contents);
        let arena = builder.into_arena();
        let mut sink = StringSink::default();

        render_to(&arena, doc, options(), &mut sink).expect("document renders");

        assert_eq!(sink.0, " x");
    }
}
