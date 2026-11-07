//! The iterative printer: a command stack, newline coalescing, line suffixes,
//! and a bounded fit check.

use std::ops::ControlFlow;

use crate::doc::{
    Kind, LINE_COMMENT, MULTILINE, STICKY, TRAILING_COMMENT, Text, VERBATIM_START, comment_lines,
    display_width, lines,
};
use crate::{Doc, DocBuilder, FormatOptions, Reindent, Sink};

/// The most commands one fit check visits before answering "does not fit".
pub const FIT_BUDGET: usize = 4096;

const CHUNK: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Flat,
    Break,
}

#[derive(Clone, Copy, Debug)]
struct Command {
    indent: u32,
    mode: Mode,
    doc: Doc,
}

#[allow(clippy::struct_excessive_bools)]
struct Printer<'p, 'a, S: Sink + ?Sized> {
    docs: &'p DocBuilder<'a>,
    options: FormatOptions,
    sink: &'p mut S,
    out: String,
    /// Display column of the end of the output.
    column: u32,
    /// Leading whitespace of the current output line.
    line_ws: String,
    /// The base indentation of the verbatim node being printed.
    verbatim_base: String,
    pending_newlines: u8,
    pending_indent: u32,
    pending_space: bool,
    /// Only indentation has been written on the current line.
    at_line_start: bool,
    wrote_any: bool,
    after_line_comment: bool,
    suffixes: Vec<Command>,
    group_modes: Vec<Option<Mode>>,
    stack: Vec<Command>,
}

pub(crate) fn print<S: Sink + ?Sized>(
    docs: &DocBuilder<'_>,
    root: Doc,
    options: FormatOptions,
    sink: &mut S,
) -> ControlFlow<()> {
    let mut printer = Printer {
        docs,
        options,
        sink,
        out: String::new(),
        column: 0,
        line_ws: String::new(),
        verbatim_base: String::new(),
        pending_newlines: 0,
        pending_indent: 0,
        pending_space: false,
        at_line_start: true,
        wrote_any: false,
        after_line_comment: false,
        suffixes: Vec::new(),
        group_modes: vec![None; docs.group_count() + 1],
        stack: vec![Command {
            indent: 0,
            mode: Mode::Break,
            doc: root,
        }],
    };
    printer.run()?;
    printer.out.push('\n');
    printer.sink.write(&printer.out)
}

impl<S: Sink + ?Sized> Printer<'_, '_, S> {
    fn run(&mut self) -> ControlFlow<()> {
        loop {
            let Some(command) = self.stack.pop() else {
                if self.suffixes.is_empty() {
                    return ControlFlow::Continue(());
                }
                self.flush_suffixes();
                continue;
            };
            let Command { indent, mode, doc } = command;
            match self.docs.kind(doc) {
                Kind::Nil | Kind::BreakParent => {}
                Kind::Text(index) => self.text(index, command)?,
                Kind::Space => self.pending_space = true,
                Kind::Line if mode == Mode::Flat => self.pending_space = true,
                Kind::SoftLine if mode == Mode::Flat => {}
                Kind::Line | Kind::SoftLine | Kind::HardLine => self.newline(command, 1),
                Kind::EmptyLine => self.newline(command, 2),
                Kind::Concat(start, len) => {
                    let children = &self.docs.children[start as usize..(start + len) as usize];
                    self.stack.extend(children.iter().rev().map(|&doc| Command {
                        indent,
                        mode,
                        doc,
                    }));
                }
                Kind::Indent(child) => self.stack.push(Command {
                    indent: indent + 1,
                    mode,
                    doc: child,
                }),
                Kind::Group(child, id) => {
                    let mode = if self.docs.breaks(doc) {
                        Mode::Break
                    } else if mode == Mode::Flat || self.fits(child) {
                        Mode::Flat
                    } else {
                        Mode::Break
                    };
                    if id != 0 {
                        self.group_modes[id as usize] = Some(mode);
                    }
                    self.stack.push(Command {
                        indent,
                        mode,
                        doc: child,
                    });
                }
                Kind::IfBreak(broken, flat, id) => {
                    let chosen = if id == 0 {
                        mode
                    } else {
                        self.group_modes[id as usize].unwrap_or(Mode::Flat)
                    };
                    self.stack.push(Command {
                        indent,
                        mode,
                        doc: if chosen == Mode::Break { broken } else { flat },
                    });
                }
                Kind::LineSuffix(child) => self.suffixes.push(Command {
                    indent,
                    mode,
                    doc: child,
                }),
            }
            if self.out.len() >= CHUNK {
                self.sink.write(&self.out)?;
                self.out.clear();
            }
        }
    }

    /// Prints the pending line suffixes before whatever is on the stack.
    fn flush_suffixes(&mut self) {
        let suffixes = std::mem::take(&mut self.suffixes);
        self.stack.extend(suffixes.into_iter().rev());
    }

    fn newline(&mut self, command: Command, count: u8) {
        if !self.suffixes.is_empty() {
            self.stack.push(command);
            self.flush_suffixes();
            return;
        }
        self.pending_newlines = self.pending_newlines.max(count);
        self.pending_indent = command.indent;
        self.pending_space = false;
    }

    fn text(&mut self, index: u32, command: Command) -> ControlFlow<()> {
        let text = self.docs.texts[index as usize];
        if text.flags & STICKY == 0 && !self.suffixes.is_empty() {
            // A suffix is never carried past a token: print it, then start
            // this text's line.
            self.stack.push(command);
            self.stack.push(Command {
                doc: Doc::HARD_LINE,
                ..command
            });
            self.flush_suffixes();
            return ControlFlow::Continue(());
        }
        if self.after_line_comment && self.pending_newlines == 0 {
            self.pending_newlines = 1;
            self.pending_indent = command.indent;
        }
        self.flush_pending();
        if text.flags & VERBATIM_START != 0 {
            self.verbatim_base.clone_from(&self.line_ws);
        }
        self.write(&text);
        self.wrote_any = true;
        self.at_line_start = false;
        self.after_line_comment = text.flags & LINE_COMMENT != 0;
        ControlFlow::Continue(())
    }

    fn flush_pending(&mut self) {
        if self.pending_newlines > 0 {
            if self.wrote_any {
                for _ in 0..self.pending_newlines {
                    self.out.push('\n');
                }
            }
            self.line_ws.clear();
            for _ in 0..self.pending_indent {
                if self.options.use_tabs {
                    self.line_ws.push('\t');
                } else {
                    for _ in 0..self.options.indent_width {
                        self.line_ws.push(' ');
                    }
                }
            }
            self.out.push_str(&self.line_ws);
            self.column = self.indent_width(self.pending_indent);
            self.pending_newlines = 0;
            self.pending_space = false;
            self.at_line_start = true;
            self.after_line_comment = false;
        }
        if self.pending_space {
            if !self.at_line_start {
                self.out.push(' ');
                self.column += 1;
            }
            self.pending_space = false;
        }
    }

    fn indent_width(&self, indent: u32) -> u32 {
        let per_level = u32::from(self.options.indent_width);
        indent * per_level
    }

    fn write(&mut self, text: &Text<'_>) {
        if text.flags & MULTILINE == 0 && text.reindent != Some(Reindent::ExpandedDoc) {
            self.out.push_str(text.text);
            self.column += text.width;
            return;
        }
        match text.reindent {
            None => {
                let mut lines = lines(text.text);
                let first = lines.next().unwrap_or("");
                self.out.push_str(first);
                self.column += display_width(first);
                for line in lines {
                    self.out.push('\n');
                    self.out.push_str(line);
                    self.line_ws.clear();
                    self.line_ws.push_str(leading_ws(line));
                    self.column = display_width(line);
                }
            }
            Some(Reindent::Strip(prefix)) => {
                let base = std::mem::take(&mut self.verbatim_base);
                let mut lines = lines(text.text).peekable();
                let first = lines.next().unwrap_or("");
                let first = if lines.peek().is_some() {
                    first.trim_end()
                } else {
                    first
                };
                self.out.push_str(first);
                self.column += display_width(first);
                while let Some(line) = lines.next() {
                    let line = if lines.peek().is_some() {
                        line.trim_end()
                    } else {
                        line
                    };
                    self.out.push('\n');
                    if line.trim().is_empty() {
                        self.line_ws.clear();
                        self.column = 0;
                        continue;
                    }
                    let rest = line.strip_prefix(prefix).unwrap_or(line);
                    self.out.push_str(&base);
                    self.out.push_str(rest);
                    self.line_ws.clear();
                    self.line_ws.push_str(&base);
                    self.line_ws.push_str(leading_ws(rest));
                    self.column = display_width(&base) + display_width(rest);
                }
                self.verbatim_base = base;
            }
            Some(reindent @ (Reindent::Comment | Reindent::ExpandedDoc)) => {
                let base = std::mem::take(&mut self.line_ws);
                let mut first = true;
                let out = &mut self.out;
                let column = &mut self.column;
                comment_lines(text.text, reindent == Reindent::ExpandedDoc, |parts| {
                    if first {
                        first = false;
                    } else {
                        out.push('\n');
                        *column = 0;
                        if !parts.iter().all(|part| part.is_empty()) {
                            out.push_str(&base);
                            *column = display_width(&base);
                        }
                    }
                    for part in parts {
                        out.push_str(part);
                        *column += display_width(part);
                    }
                });
                self.line_ws = base;
            }
        }
    }

    /// Whether `doc` fits flat on the rest of the current line, followed by
    /// the rest of the stack in its recorded modes.
    fn fits(&self, doc: Doc) -> bool {
        let (start, mut space) = if self.pending_newlines > 0 {
            (self.indent_width(self.pending_indent), false)
        } else {
            (self.column, self.pending_space && !self.at_line_start)
        };
        let mut width = i64::from(self.options.line_width) - i64::from(start);
        // The width of trailing comments that only counts if text follows.
        let mut deferred = 0;
        let mut suffix_pending = !self.suffixes.is_empty();
        let mut rest = self.stack.len();
        let mut stack: Vec<(Mode, Doc, bool)> = vec![(Mode::Flat, doc, false)];
        let mut budget = FIT_BUDGET;
        loop {
            let (mode, doc, in_rest) = if let Some(entry) = stack.pop() {
                entry
            } else {
                if rest == 0 {
                    return true;
                }
                rest -= 1;
                let command = self.stack[rest];
                (command.mode, command.doc, true)
            };
            if budget == 0 {
                return false;
            }
            budget -= 1;
            match self.docs.kind(doc) {
                Kind::Nil => {}
                Kind::Text(index) => {
                    let text = &self.docs.texts[index as usize];
                    if suffix_pending && text.flags & STICKY == 0 {
                        return in_rest;
                    }
                    if text.flags & TRAILING_COMMENT != 0 {
                        deferred += i64::from(text.width) + i64::from(space);
                        space = false;
                        continue;
                    }
                    width -= i64::from(text.width) + i64::from(space) + deferred;
                    space = false;
                    deferred = 0;
                    if width < 0 {
                        return false;
                    }
                    if text.flags & MULTILINE != 0 {
                        return in_rest;
                    }
                }
                Kind::Space => space = true,
                Kind::Line if mode == Mode::Flat => space = true,
                Kind::SoftLine if mode == Mode::Flat => {}
                Kind::Line | Kind::SoftLine | Kind::HardLine | Kind::EmptyLine => {
                    return in_rest;
                }
                Kind::BreakParent => {
                    if !in_rest {
                        return false;
                    }
                }
                Kind::Concat(start, len) => {
                    let children = &self.docs.children[start as usize..(start + len) as usize];
                    stack.extend(children.iter().rev().map(|&child| (mode, child, in_rest)));
                }
                Kind::Indent(child) => stack.push((mode, child, in_rest)),
                Kind::Group(child, _) => {
                    let mode = if self.docs.breaks(doc) {
                        Mode::Break
                    } else {
                        mode
                    };
                    stack.push((mode, child, in_rest));
                }
                Kind::IfBreak(broken, flat, id) => {
                    let chosen = if id == 0 {
                        mode
                    } else {
                        self.group_modes[id as usize].unwrap_or(Mode::Flat)
                    };
                    stack.push((
                        mode,
                        if chosen == Mode::Break { broken } else { flat },
                        in_rest,
                    ));
                }
                Kind::LineSuffix(_) => suffix_pending = true,
            }
        }
    }
}

fn leading_ws(line: &str) -> &str {
    let end = line
        .find(|ch: char| ch != ' ' && ch != '\t')
        .unwrap_or(line.len());
    &line[..end]
}
