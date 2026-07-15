use std::error::Error;
use std::fmt;

use crate::document::{Doc, DocArena, DocNode, FlatLine, Line, LineMode, LiteralText};
#[cfg(debug_assertions)]
use crate::source_fragment::SourceFragment;
use crate::source_fragment::{CompletedRenderProof, RenderProof};
use crate::width::{TextWidth, add_width};
use jolt_syntax::ConservationError;

// A flat-fit probe can scan nested docs, the active render stack, and an overlay
// stack for groups/indents. Cap the number of semantic commands each probe can
// process so repeated tiny groups cannot turn rendering into unbounded layout
// search. Concat range cursors are implementation bookkeeping and are bounded by
// the document commands they expose. When the budget is exhausted, the group is
// treated as not fitting and rendered in break mode.
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

/// The ordinary render result plus a completed source-conservation proof.
pub struct TrackedRenderOutcome<'tree> {
    halted: bool,
    proof: Option<CompletedRenderProof<'tree>>,
}

impl<'tree> TrackedRenderOutcome<'tree> {
    #[must_use]
    pub const fn halted(&self) -> bool {
        self.halted
    }

    #[must_use]
    pub const fn completed_proof(&self) -> Option<&CompletedRenderProof<'tree>> {
        self.proof.as_ref()
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

    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    pub(crate) const fn missing_conservation_proof() -> Self {
        Self {
            kind: RenderErrorKind::MissingConservationProof,
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
    MissingConservationProof,
    Conservation(ConservationError),
    SyntaxInvariant(String),
}

impl fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            RenderErrorKind::NoCurrentGroup => {
                formatter.write_str("if_break requires a current group")
            }
            RenderErrorKind::MissingConservationProof => {
                formatter.write_str("exceptional source fragment requires a conservation proof")
            }
            RenderErrorKind::Conservation(ref error) => write!(formatter, "{error}"),
            RenderErrorKind::SyntaxInvariant(ref message) => {
                write!(formatter, "syntax invariant failed: {message}")
            }
        }
    }
}

impl Error for RenderError {}

/// Renders a document using the provided options.
///
/// # Errors
///
/// Returns [`RenderError`] when the document is structurally invalid.
pub fn render_to<S: RenderSink>(
    arena: &DocArena<'_>,
    doc: Doc<'_>,
    options: RenderOptions,
    sink: S,
) -> Result<RenderOutcome, RenderError> {
    if let Some(error) = arena.invariant_error() {
        return Err(RenderError::syntax_invariant(error));
    }
    let mut renderer = Renderer::new(arena, options, sink, None);
    renderer.render_doc(doc, Mode::Break)?;
    Ok(renderer.finish())
}

/// Renders a document while proving source conservation for the exceptional
/// fragments that are actually emitted.
///
/// # Errors
///
/// Returns [`RenderError`] when the document is structurally invalid or a
/// rendered fragment makes a duplicate or foreign source claim.
pub fn render_to_tracked<'source, S: RenderSink>(
    arena: &DocArena<'source>,
    doc: Doc<'source>,
    options: RenderOptions,
    sink: S,
    mut proof: RenderProof<'source>,
) -> Result<TrackedRenderOutcome<'source>, RenderError> {
    if let Some(error) = arena.invariant_error() {
        return Err(RenderError::syntax_invariant(error));
    }
    let mut renderer = Renderer::new(arena, options, sink, Some(&mut proof));
    renderer.render_doc(doc, Mode::Break)?;
    let render = renderer.finish();
    if render.halted {
        return Ok(TrackedRenderOutcome {
            halted: true,
            proof: None,
        });
    }
    let proof = proof.finish().map_err(|error| RenderError {
        kind: RenderErrorKind::Conservation(error),
    })?;
    Ok(TrackedRenderOutcome {
        halted: false,
        proof: Some(proof),
    })
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
struct PendingIndent {
    character: char,
    count: u32,
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
    EndIndent(i16),
    EndGroup,
}

impl<'source> From<RenderCommand<'source>> for FitCommand<'source> {
    fn from(command: RenderCommand<'source>) -> Self {
        match command {
            RenderCommand::Doc(doc, mode) => Self::Doc(doc, mode),
            RenderCommand::EndIndent(levels) => Self::EndIndent(levels),
            RenderCommand::EndGroup => Self::EndGroup,
        }
    }
}

struct Renderer<'arena, 'proof, 'source, S> {
    arena: &'arena DocArena<'source>,
    options: RenderOptions,
    sink: S,
    halted: bool,
    column: TextWidth,
    indent_levels: i32,
    pending_indent: Option<PendingIndent>,
    group_stack: Vec<GroupFrame>,
    command_stack: Vec<RenderCommand<'source>>,
    fit_group_stack: Vec<GroupFrame>,
    fit_overlay_stack: Vec<FitCommand<'source>>,
    measured_group_fits: bool,
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    proof: Option<&'proof mut RenderProof<'source>>,
}

impl<'arena, 'proof, 'source, S: RenderSink> Renderer<'arena, 'proof, 'source, S> {
    fn new(
        arena: &'arena DocArena<'source>,
        options: RenderOptions,
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
            pending_indent: None,
            group_stack: Vec::new(),
            command_stack: Vec::new(),
            fit_group_stack: Vec::new(),
            fit_overlay_stack: Vec::new(),
            measured_group_fits: false,
            proof,
        }
    }

    fn finish(self) -> RenderOutcome {
        RenderOutcome {
            halted: self.halted,
        }
    }

    fn render_doc(&mut self, doc: Doc<'source>, mode: Mode) -> Result<(), RenderError> {
        let mut stack = std::mem::take(&mut self.command_stack);
        stack.clear();
        stack.push(RenderCommand::Doc(doc, mode));
        let result = self.render_commands(&mut stack);
        stack.clear();
        self.command_stack = stack;
        result
    }

    fn render_commands(
        &mut self,
        stack: &mut Vec<RenderCommand<'source>>,
    ) -> Result<(), RenderError> {
        while let Some(command) = stack.pop() {
            if self.halted {
                break;
            }
            match command {
                RenderCommand::Doc(doc, mode) => self.render_command_doc(doc, mode, stack)?,
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
                self.write_measured_str(&text.text, text.width);
                Ok(())
            }
            Some(DocNode::LiteralText(text)) => {
                self.write_literal(text);
                Ok(())
            }
            #[cfg(debug_assertions)]
            Some(DocNode::SourceFragment(fragment)) => {
                if let Some(proof) = self.proof.as_mut() {
                    proof
                        .render_fragment(fragment, arena.source_claims(fragment))
                        .map_err(|error| RenderError {
                            kind: RenderErrorKind::Conservation(error),
                        })?;
                } else if !fragment.is_claim_marker() {
                    return Err(RenderError::missing_conservation_proof());
                }
                self.write_source_fragment(fragment);
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
        let is_broken = if should_break {
            true
        } else if mode == Mode::Flat && self.measured_group_fits {
            false
        } else {
            let fits = self.group_fits(contents, stack);
            if fits {
                self.measured_group_fits = true;
            }
            !fits
        };
        self.group_stack.push(GroupFrame { is_broken });
        stack.push(RenderCommand::EndGroup);
        stack.push(RenderCommand::Doc(
            contents,
            if is_broken { Mode::Break } else { Mode::Flat },
        ));
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

    #[cfg(debug_assertions)]
    fn write_source_fragment(&mut self, fragment: &SourceFragment<'_, '_>) {
        self.write_str(&fragment.text);
        let final_width = fragment.final_width();
        if fragment.is_multiline() {
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
        self.pending_indent = None;
        for _ in 0..count {
            self.write_sink_str("\n");
            self.column = TextWidth::ZERO;
            self.measured_group_fits = false;
        }
        let effective_levels = (self.indent_levels + i32::from(indent_delta))
            .max(0)
            .cast_unsigned();
        let width = effective_levels * u32::from(self.options.indent_width);
        let (character, count) = match self.options.indent_style {
            IndentStyle::Space => (' ', width),
            IndentStyle::Tab => ('\t', effective_levels),
        };
        self.pending_indent = (count > 0).then_some(PendingIndent { character, count });
        self.column = TextWidth::new(width);
    }

    fn write_str(&mut self, text: &str) {
        if text.is_empty() || self.halted {
            return;
        }
        self.flush_pending_indent();
        self.write_sink_str(text);
    }

    fn write_sink_str(&mut self, text: &str) {
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

    fn flush_pending_indent(&mut self) {
        let Some(indent) = self.pending_indent.take() else {
            return;
        };
        self.write_repeated(indent.character, indent.count);
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
    options: RenderOptions,
    column: TextWidth,
    indent_levels: i32,
    base_group_stack: &'base [GroupFrame],
    base_group_len: usize,
    group_stack: &'scratch mut Vec<GroupFrame>,
    remaining_commands: usize,
}

impl<'base, 'scratch, 'source> FitChecker<'base, 'scratch, 'source> {
    fn from_renderer<S>(
        renderer: &'base Renderer<'_, '_, 'source, S>,
        group_stack: &'scratch mut Vec<GroupFrame>,
    ) -> Self {
        Self {
            arena: renderer.arena,
            options: renderer.options,
            column: renderer.column,
            indent_levels: renderer.indent_levels,
            base_group_stack: &renderer.group_stack,
            base_group_len: renderer.group_stack.len(),
            group_stack,
            remaining_commands: FLAT_FIT_COMMAND_BUDGET,
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

        self.column <= self.options.line_width
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
            FitCommand::EndIndent(levels) => {
                self.indent_levels -= i32::from(levels);
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
            Some(DocNode::Text(text)) => self.width_result(text.width),
            Some(DocNode::LiteralText(text)) => {
                if text.is_multiline() {
                    FitResult::No
                } else {
                    self.width_result(text.final_width())
                }
            }
            #[cfg(debug_assertions)]
            Some(DocNode::SourceFragment(fragment)) => {
                if fragment.is_multiline() {
                    FitResult::No
                } else {
                    self.width_result(fragment.final_width())
                }
            }
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
            Some(DocNode::Indent { contents, levels }) => {
                self.indent_levels += i32::from(*levels);
                stack.push(FitCommand::EndIndent(*levels));
                stack.push(FitCommand::Doc(*contents, mode));
                FitResult::Continue
            }
            Some(DocNode::Line(line)) => self.fit_line(line, mode),
            Some(DocNode::IfBreak { breaks, flat }) => {
                let is_broken = self.group_break_state().unwrap_or(false);
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
        let is_broken = mode == Mode::Break || should_break;
        self.group_stack.push(GroupFrame { is_broken });
        stack.push(FitCommand::EndGroup);
        stack.push(FitCommand::Doc(
            contents,
            if is_broken { Mode::Break } else { Mode::Flat },
        ));
        FitResult::Continue
    }

    fn fit_group_flat_with_stack(
        &mut self,
        contents: Doc<'source>,
        stack: &[RenderCommand<'source>],
        overlay: &mut Vec<FitCommand<'source>>,
    ) -> bool {
        self.group_stack.push(GroupFrame { is_broken: false });
        let mut fit_stack = FitStack::new(stack, overlay);
        fit_stack.push(FitCommand::EndGroup);
        fit_stack.push(FitCommand::Doc(contents, Mode::Flat));
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
    use jolt_java_syntax::{JavaLanguage, JavaSyntaxKind};
    use jolt_syntax::{
        BuildSyntaxTreeError, Event, FactoryNode, Language, LanguageLexer, LexedToken,
        ParsedChildren, RawSyntaxKind, SourceIdentity, SourceTokenId, SyntaxFactory, SyntaxNode,
        SyntaxTokenData, SyntaxTreeSink, SyntaxTrivia, TriviaKind, build_syntax_tree_with_factory,
    };
    use jolt_text::{TextRange, TextSize};

    use crate::source_fragment::exceptional_separators;
    use crate::{
        DocBuilder, ExceptionalSeparator, FragmentBoundary, LexicalAtom, LexicalAtomKind,
        LexicalSafety, NormalizedToken, RemovalClaim, RemovalReason, RenderControl, RenderProof,
        RenderSink, ReplacementClaim, SourceFragmentKind, SynthesisClaim,
    };

    use super::{IndentStyle, RenderOptions, TextWidth, render_to, render_to_tracked};

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
            if kind == JavaLanguage::kind_to_raw(JavaLanguage::error_node_kind()) {
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

        fn error_node_kind() -> Self::Kind {
            JavaLanguage::error_node_kind()
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

    fn replacement_claim(
        source: SourceTokenId<'_>,
        token: NormalizedToken,
    ) -> ReplacementClaim<'_> {
        ReplacementClaim::authorized::<ClaimLanguage>((), source, token)
    }

    fn removal_claim(source: SourceIdentity<'_>, reason: RemovalReason) -> RemovalClaim<'_> {
        RemovalClaim::authorized::<ClaimLanguage>((), source, reason)
    }

    fn synthesis_claim(anchor: SourceTokenId<'_>, token: NormalizedToken) -> SynthesisClaim<'_> {
        SynthesisClaim::authorized::<ClaimLanguage>((), anchor, token)
    }

    impl RenderSink for StringSink {
        fn write_str(&mut self, text: &str) -> RenderControl {
            self.0.push_str(text);
            RenderControl::Continue
        }
    }

    fn options() -> RenderOptions {
        RenderOptions {
            line_width: TextWidth::new(80),
            indent_width: 4,
            indent_style: IndentStyle::Space,
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

    fn bogus_syntax_tree(source: &str) -> jolt_syntax::SyntaxTree {
        syntax_tree_with_root_kind(source, JavaLanguage::error_node_kind())
    }

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
                kind: JavaLanguage::kind_to_raw(JavaLanguage::error_node_kind()),
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

    #[test]
    fn malformed_verbatim_is_one_tracked_borrowed_core() {
        let source = "a+b";
        let tree = bogus_syntax_tree(source);
        let root = SyntaxNode::<JavaLanguage>::new_root(source, &tree);
        let core = root
            .malformed_verbatim_core()
            .expect("error node owns a verbatim core");
        let mut builder = DocBuilder::new();
        let fragment = builder.malformed_verbatim(&core, empty_boundary());
        let mut safety = CountingSafety::default();
        let document = builder.resolve_exceptional(fragment, None, None, &mut safety);
        let arena = builder.into_arena();
        let mut sink = StringSink::default();
        let proof = RenderProof::new(root.conservation_tracker());

        let outcome = render_to_tracked(&arena, document, options(), &mut sink, proof)
            .expect("verbatim document renders");

        assert_eq!(sink.0, source);
        assert!(matches!(
            outcome
                .completed_proof()
                .expect("render completed")
                .rendered_fragments(),
            [rendered]
                if matches!(rendered.kind, SourceFragmentKind::MalformedVerbatim { .. })
        ));
    }

    #[test]
    fn valid_node_cannot_construct_a_malformed_verbatim_core() {
        let source = "valid";
        let tree = syntax_tree(source);
        let root = SyntaxNode::<JavaLanguage>::new_root(source, &tree);
        assert!(root.malformed_verbatim_core().is_none());
    }

    #[test]
    fn structured_token_and_bogus_core_complete_one_root_proof() {
        let source = "ab";
        let tree = mixed_syntax_tree();
        let root = SyntaxNode::<JavaLanguage>::new_root(source, &tree);
        let first = root.first_token().expect("structured token");
        let bogus = root.children().next().expect("bogus child");
        let core = bogus
            .malformed_verbatim_core()
            .expect("error child owns a verbatim core");
        let atom_b = LexicalAtom::new(LexicalAtomKind::Identifier, "b");
        let mut builder = DocBuilder::new();
        let structured = builder.source_token(&first);
        let malformed = builder.malformed_verbatim(
            &core,
            FragmentBoundary {
                first: Some(atom_b),
                last: Some(atom_b),
                ends_with_line_comment: false,
            },
        );
        let mut safety = CountingSafety::default();
        let malformed = builder.resolve_exceptional(malformed, Some(&first), None, &mut safety);
        let document = builder.concat([structured, malformed]);
        let arena = builder.into_arena();
        let mut sink = StringSink::default();
        let proof = RenderProof::new(root.conservation_tracker());

        let outcome = render_to_tracked(&arena, document, options(), &mut sink, proof)
            .expect("mixed structured and bogus output completes");

        assert_eq!(sink.0, "a b");
        assert_eq!(safety.0, 1);
        assert!(matches!(
            outcome
                .completed_proof()
                .expect("render completed")
                .rendered_fragments(),
            [rendered]
                if matches!(rendered.kind, SourceFragmentKind::MalformedVerbatim { .. })
        ));
    }

    #[test]
    fn structured_comment_claims_its_line_terminator() {
        let source = "x//c\n";
        let tree = syntax_tree_with_line_comment();
        let root = SyntaxNode::<JavaLanguage>::new_root(source, &tree);
        let token = root.first_token().expect("source token");
        let trivia = token.trailing_trivia_with_ids().collect::<Vec<_>>();
        let mut builder = DocBuilder::new();
        let token_doc = builder.source_token(&token);
        let comment_doc = builder.source_trivia("//c", trivia);
        let line = builder.hard_line();
        let document = builder.concat([token_doc, comment_doc, line]);
        let arena = builder.into_arena();
        let mut sink = StringSink::default();
        let proof = RenderProof::new(root.conservation_tracker());

        let outcome = render_to_tracked(&arena, document, options(), &mut sink, proof)
            .expect("structured token and comment complete");

        assert_eq!(sink.0, source);
        assert!(
            outcome
                .completed_proof()
                .expect("render completed")
                .rendered_fragments()
                .is_empty()
        );
    }

    #[test]
    fn tracked_render_cannot_return_before_completion() {
        let source = "x";
        let tree = syntax_tree(source);
        let root = SyntaxNode::<JavaLanguage>::new_root(source, &tree);
        let mut builder = DocBuilder::new();
        let untracked = builder.text(source);
        let arena = builder.into_arena();
        let mut sink = StringSink::default();
        let proof = RenderProof::new(root.conservation_tracker());

        let Err(error) = render_to_tracked(&arena, untracked, options(), &mut sink, proof) else {
            panic!("tracked render must reject a missing structured token claim");
        };

        assert!(error.to_string().starts_with("MissingToken"));
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
        let root = SyntaxNode::<JavaLanguage>::new_root(source, &tree);
        let token = root.first_token().expect("source token");
        let mut builder = DocBuilder::new();
        let line = builder.hard_line();
        let token = builder.source_token(&token);
        let contents = builder.concat([line, token]);
        let document = builder.indent(contents);
        let arena = builder.into_arena();
        let mut sink = HaltOnIndent(String::new());
        let proof = RenderProof::new(root.conservation_tracker());

        let outcome = render_to_tracked(&arena, document, options(), &mut sink, proof)
            .expect("an intentional halt is not a conservation error");

        assert!(outcome.halted());
        assert!(outcome.completed_proof().is_none());
        assert_eq!(sink.0, "\n    ");
    }

    #[test]
    fn only_rendered_if_break_branch_consumes_claims() {
        let source = "ab";
        let tree = syntax_tree(source);
        let root = SyntaxNode::<JavaLanguage>::new_root(source, &tree);
        let mut tokens = root.tokens();
        let first = tokens.next().expect("first token").source_id();
        let second = tokens.next().expect("second token").source_id();
        let mut builder = DocBuilder::new();
        let breaks =
            builder.replaced_source(replacement_claim(first, NormalizedToken::ImportKeyword));
        let flat = builder.replaced_source(replacement_claim(
            second,
            NormalizedToken::ImportAliasKeyword,
        ));
        let mut safety = CountingSafety::default();
        let breaks = builder.resolve_exceptional(breaks, None, None, &mut safety);
        let flat = builder.resolve_exceptional(flat, None, None, &mut safety);
        let conditional = builder.if_break(breaks, flat);
        let conditional = builder.group(conditional);
        let removed = builder.removed_source(removal_claim(
            SourceIdentity::Token(first),
            RemovalReason::DuplicateImport,
        ));
        let document = builder.concat([conditional, removed]);
        let arena = builder.into_arena();
        let mut sink = StringSink::default();
        let proof = RenderProof::new(root.conservation_tracker());

        let outcome = render_to_tracked(&arena, document, options(), &mut sink, proof)
            .expect("selected branch renders without a duplicate claim");

        assert_eq!(sink.0, "as");
        let fragments = outcome
            .completed_proof()
            .expect("render completed")
            .rendered_fragments();
        assert_eq!(fragments.len(), 2);
        assert!(matches!(
            fragments[0].kind,
            SourceFragmentKind::Replaced {
                token: NormalizedToken::ImportAliasKeyword
            }
        ));
    }

    #[test]
    fn zero_token_malformed_core_still_records_dispatch() {
        let source = "";
        let tree = bogus_syntax_tree(source);
        let root = SyntaxNode::<JavaLanguage>::new_root(source, &tree);
        let core = root
            .malformed_verbatim_core()
            .expect("empty error node owns a verbatim core");
        let mut builder = DocBuilder::new();
        let fragment = builder.malformed_verbatim(&core, empty_boundary());
        let mut safety = CountingSafety::default();
        let document = builder.resolve_exceptional(fragment, None, None, &mut safety);
        let arena = builder.into_arena();
        let mut sink = StringSink::default();
        let proof = RenderProof::new(root.conservation_tracker());

        let outcome = render_to_tracked(&arena, document, options(), &mut sink, proof)
            .expect("empty malformed fragment renders");

        assert_eq!(
            outcome
                .completed_proof()
                .expect("render completed")
                .rendered_fragments()
                .len(),
            1
        );
    }

    #[test]
    fn synthesis_has_closed_provenance_and_a_source_anchor() {
        let source = "x";
        let tree = syntax_tree(source);
        let root = SyntaxNode::<JavaLanguage>::new_root(source, &tree);
        let token = root.first_token().expect("source token");
        let mut builder = DocBuilder::new();
        let source_doc = builder.source_token(&token);
        let synthesized = builder.synthesized_source(synthesis_claim(
            token.source_id(),
            NormalizedToken::EnumSemicolon,
        ));
        let mut safety = CountingSafety::default();
        let synthesized = builder.resolve_exceptional(synthesized, None, None, &mut safety);
        let document = builder.concat([source_doc, synthesized]);
        let arena = builder.into_arena();
        let mut sink = StringSink::default();
        let proof = RenderProof::new(root.conservation_tracker());

        let outcome = render_to_tracked(&arena, document, options(), &mut sink, proof)
            .expect("source-backed and synthesized fragments render");

        assert_eq!(sink.0, "x;");
        assert!(matches!(
            outcome
                .completed_proof()
                .expect("render completed")
                .rendered_fragments(),
            [synthesized]
                if matches!(synthesized.kind, SourceFragmentKind::Synthesized {
                    token: NormalizedToken::EnumSemicolon,
                    ..
                })
        ));
    }

    #[test]
    fn synthesis_rejects_a_foreign_anchor() {
        let source = "x";
        let tree = syntax_tree(source);
        let root = SyntaxNode::<JavaLanguage>::new_root(source, &tree);
        let other_tree = syntax_tree("y");
        let other_root = SyntaxNode::<JavaLanguage>::new_root("y", &other_tree);
        let foreign = other_root.first_token().expect("foreign token").source_id();
        let mut builder = DocBuilder::new();
        let synthesized =
            builder.synthesized_source(synthesis_claim(foreign, NormalizedToken::EnumSemicolon));
        let mut safety = CountingSafety::default();
        let document = builder.resolve_exceptional(synthesized, None, None, &mut safety);
        let arena = builder.into_arena();
        let mut sink = StringSink::default();
        let proof = RenderProof::new(root.conservation_tracker());

        let Err(error) = render_to_tracked(&arena, document, options(), &mut sink, proof) else {
            panic!("foreign synthesis anchor must be rejected");
        };

        assert_eq!(error.to_string(), "ForeignToken");
        assert!(sink.0.is_empty());
    }

    #[test]
    fn exceptional_fragment_cannot_render_without_proof() {
        let source = "";
        let tree = bogus_syntax_tree(source);
        let root = SyntaxNode::<JavaLanguage>::new_root(source, &tree);
        let core = root
            .malformed_verbatim_core()
            .expect("error node owns a verbatim core");
        let mut builder = DocBuilder::new();
        let fragment = builder.malformed_verbatim(&core, empty_boundary());
        let arena = builder.into_arena();
        let mut sink = StringSink::default();

        let error = render_to(&arena, fragment.doc(), options(), &mut sink)
            .expect_err("untracked exceptional output must be rejected");

        assert_eq!(
            error.to_string(),
            "exceptional source fragment requires a conservation proof"
        );
        assert!(sink.0.is_empty());
    }

    #[test]
    fn unselected_exceptional_branch_does_not_require_proof() {
        let source = "";
        let tree = bogus_syntax_tree(source);
        let root = SyntaxNode::<JavaLanguage>::new_root(source, &tree);
        let core = root
            .malformed_verbatim_core()
            .expect("error node owns a verbatim core");
        let mut builder = DocBuilder::new();
        let exceptional = builder.malformed_verbatim(&core, empty_boundary());
        let ordinary = builder.text("ordinary");
        let conditional = builder.if_break(exceptional.doc(), ordinary);
        let document = builder.group(conditional);
        let arena = builder.into_arena();
        let mut sink = StringSink::default();

        render_to(&arena, document, options(), &mut sink)
            .expect("unselected exceptional branch is not visited");

        assert_eq!(sink.0, "ordinary");
    }

    #[derive(Default)]
    struct CountingSafety(usize);

    impl LexicalSafety<JavaLanguage> for CountingSafety {
        fn classify(
            &mut self,
            _token: &jolt_syntax::SyntaxToken<'_, JavaLanguage>,
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

    #[test]
    fn exceptional_lexical_safety_is_bounded_and_line_comment_aware() {
        let source = "x";
        let tree = bogus_syntax_tree(source);
        let root = SyntaxNode::<JavaLanguage>::new_root(source, &tree);
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
            exceptional_separators::<JavaLanguage>(Some(atom), fragment, Some(atom), &mut safety);

        assert_eq!(separators.before, ExceptionalSeparator::Space);
        assert_eq!(separators.after, ExceptionalSeparator::HardLine);
        assert_eq!(
            safety.0, 1,
            "line-comment override avoids a second callback"
        );
    }

    #[test]
    fn adjacent_exceptional_fragments_share_one_safe_join() {
        let source = "ab";
        let tree = syntax_tree(source);
        let root = SyntaxNode::<JavaLanguage>::new_root(source, &tree);
        let mut tokens = root.tokens();
        let first = tokens.next().expect("first token").source_id();
        let second = tokens.next().expect("second token").source_id();
        let mut builder = DocBuilder::new();
        let import =
            builder.replaced_source(replacement_claim(first, NormalizedToken::ImportKeyword));
        let alias = builder.replaced_source(replacement_claim(
            second,
            NormalizedToken::ImportAliasKeyword,
        ));
        let mut safety = CountingSafety::default();
        let joined = builder.join_exceptional(import, alias, &mut safety);
        let document = builder.resolve_exceptional(joined, None, None, &mut safety);
        let arena = builder.into_arena();
        let mut sink = StringSink::default();
        let proof = RenderProof::new(root.conservation_tracker());

        render_to_tracked(&arena, document, options(), &mut sink, proof)
            .expect("joined exceptional fragments render");

        assert_eq!(sink.0, "import as");
        assert_eq!(safety.0, 1);
    }

    #[test]
    fn exceptional_join_after_line_comment_forces_a_hard_line() {
        let source = "x";
        let tree = bogus_syntax_tree(source);
        let root = SyntaxNode::<JavaLanguage>::new_root(source, &tree);
        let token = root.first_token().expect("source token");
        let core = root
            .malformed_verbatim_core()
            .expect("error node owns a verbatim core");
        let atom = LexicalAtom::new(LexicalAtomKind::Comment, source);
        let mut builder = DocBuilder::new();
        let malformed = builder.malformed_verbatim(
            &core,
            FragmentBoundary {
                first: Some(atom),
                last: Some(atom),
                ends_with_line_comment: true,
            },
        );
        let synthesized = builder.synthesized_source(synthesis_claim(
            token.source_id(),
            NormalizedToken::EnumSemicolon,
        ));
        let mut safety = CountingSafety::default();
        let joined = builder.join_exceptional(malformed, synthesized, &mut safety);
        let document = builder.resolve_exceptional(joined, None, None, &mut safety);
        let arena = builder.into_arena();
        let mut sink = StringSink::default();
        let proof = RenderProof::new(root.conservation_tracker());

        render_to_tracked(&arena, document, options(), &mut sink, proof)
            .expect("line-comment boundary remains safe");

        assert_eq!(sink.0, "x\n;");
        assert_eq!(safety.0, 0);
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
    fn consecutive_indented_lines_do_not_emit_trailing_whitespace() {
        let mut builder = DocBuilder::new();
        let contents = builder.concat_list(|contents| {
            let first = contents.text("first");
            contents.push(first);
            let hard = contents.hard_line();
            contents.push(hard);
            let second_hard = contents.hard_line();
            contents.push(second_hard);
            let second = contents.text("second");
            contents.push(second);
            let empty = contents.empty_line();
            contents.push(empty);
            let third = contents.text("third");
            contents.push(third);
            let trailing = contents.hard_line();
            contents.push(trailing);
        });
        let doc = builder.indent(contents);
        let arena = builder.into_arena();
        let mut sink = StringSink::default();

        render_to(&arena, doc, options(), &mut sink).expect("document renders");

        assert_eq!(sink.0, "first\n\n    second\n\n    third\n");
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
