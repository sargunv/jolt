//! The iterative printer: a command stack, newline coalescing, line suffixes,
//! and a bounded fit check.

use std::ops::ControlFlow;

use crate::doc::{
    Kind, LINE_COMMENT, MULTILINE, Reindent, STICKY, Text, VERBATIM_START, comment_lines,
    display_width, lines,
};
use crate::{Doc, DocBuilder, FormatOptions, Sink};

/// The most commands one fit check visits before answering "does not fit".
pub(crate) const FIT_BUDGET: usize = 4096;

/// The whitespace verbatim lines lose at their ends: what separates tokens.
/// (Other Unicode whitespace is a token of its own.)
const BLANK: [char; 3] = [' ', '\t', '\x0c'];

/// Output is handed to the sink in chunks of about this size, so a sink that
/// compares (check mode) can stop the printer soon after a difference.
const CHUNK: usize = 1024;

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
struct Printer<'p, 'a, S: Sink + ?Sized, J: Fn(char, char) -> bool> {
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
    /// The last character written on the current line, if it is not a space.
    last: Option<char>,
    joins: J,
    suffixes: Vec<Command>,
    /// The fit check's stack, kept to reuse its allocation.
    fit_stack: Vec<Fit>,
    stack: Vec<Command>,
}

pub(crate) fn print<S: Sink + ?Sized, J: Fn(char, char) -> bool>(
    docs: &DocBuilder<'_>,
    root: Doc,
    options: FormatOptions,
    joins: J,
    final_newline: bool,
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
        last: None,
        joins,
        suffixes: Vec::new(),
        fit_stack: Vec::new(),
        stack: vec![Command {
            indent: 0,
            mode: Mode::Break,
            doc: root,
        }],
    };
    printer.run()?;
    if final_newline {
        printer.out.push('\n');
    }
    printer.sink.write(&printer.out)
}

impl<S: Sink + ?Sized, J: Fn(char, char) -> bool> Printer<'_, '_, S, J> {
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
                Kind::Group(child) => {
                    let mode = if self.docs.breaks(doc) {
                        Mode::Break
                    } else if mode == Mode::Flat || self.fits(child, indent) {
                        Mode::Flat
                    } else {
                        Mode::Break
                    };
                    self.stack.push(Command {
                        indent,
                        mode,
                        doc: child,
                    });
                }
                Kind::IfBreak(broken, flat) => self.stack.push(Command {
                    indent,
                    mode,
                    doc: if mode == Mode::Break { broken } else { flat },
                }),
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
        let bytes = text.text.as_bytes();
        if let Some(last) = self.last {
            // Texts start with an ASCII byte in all but rare cases.
            let first = match bytes.first() {
                Some(&byte) if byte.is_ascii() => Some(char::from(byte)),
                _ => text.text.chars().next(),
            };
            if first.is_some_and(|first| (self.joins)(last, first)) {
                self.out.push(' ');
                self.column += 1;
            }
        }
        self.last = match bytes.last() {
            Some(b' ') | None => None,
            Some(&byte) if byte.is_ascii() => Some(char::from(byte)),
            _ => text.text.chars().next_back(),
        };
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
            self.last = None;
        }
        if self.pending_space {
            if !self.at_line_start {
                self.out.push(' ');
                self.column += 1;
                self.last = None;
            }
            self.pending_space = false;
        }
    }

    fn indent_width(&self, indent: u32) -> u32 {
        let per_level = u32::from(self.options.indent_width);
        indent * per_level
    }

    fn write(&mut self, text: &Text<'_>) {
        if text.flags & MULTILINE == 0 {
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
                    first.trim_end_matches(BLANK)
                } else {
                    first
                };
                self.out.push_str(first);
                self.column += display_width(first);
                while let Some(line) = lines.next() {
                    let line = if lines.peek().is_some() {
                        line.trim_end_matches(BLANK)
                    } else {
                        line
                    };
                    self.out.push('\n');
                    // A blank line gets no indentation; the last line is never
                    // blank, since a text block follows it on the same line.
                    if line.trim_matches(BLANK).is_empty() && lines.peek().is_some() {
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
            Some(Reindent::Comment) => {
                for (index, parts) in comment_lines(text.text).enumerate() {
                    if index > 0 {
                        self.out.push('\n');
                        self.column = 0;
                        if !parts.iter().all(|part| part.is_empty()) {
                            self.out.push_str(&self.line_ws);
                            self.column = display_width(&self.line_ws);
                        }
                    }
                    for part in parts {
                        self.out.push_str(part);
                        self.column += display_width(part);
                    }
                }
            }
        }
    }

    /// Whether `doc` fits flat on the rest of the current line, followed by
    /// the rest of the stack in its recorded modes. `indent` is the group's.
    fn fits(&mut self, doc: Doc, indent: u32) -> bool {
        let mut stack = std::mem::take(&mut self.fit_stack);
        stack.clear();
        let fits = self.measure(doc, indent, &mut stack);
        self.fit_stack = stack;
        fits
    }

    #[allow(clippy::too_many_lines)]
    fn measure(&self, doc: Doc, indent: u32, stack: &mut Vec<Fit>) -> bool {
        let (start, mut space) = if self.pending_newlines > 0 {
            (self.indent_width(self.pending_indent), false)
        } else {
            (self.column, self.pending_space && !self.at_line_start)
        };
        // Spaces at the start of a line are dropped, as when printing.
        let mut line_start = self.pending_newlines > 0 || self.at_line_start;
        let mut width = i64::from(self.options.line_width) - i64::from(start);
        let mut suffix_pending = !self.suffixes.is_empty();
        // A flat line was measured inside the group, so breaking the group
        // would change where its text starts.
        let mut saw_line = false;
        let mut rest = self.stack.len();
        stack.push(Fit {
            mode: Mode::Flat,
            doc,
            indent,
            in_rest: false,
            in_suffix: false,
        });
        let mut budget = FIT_BUDGET;
        loop {
            let entry = if let Some(entry) = stack.pop() {
                entry
            } else {
                if rest == 0 {
                    return true;
                }
                rest -= 1;
                let command = self.stack[rest];
                Fit {
                    mode: command.mode,
                    doc: command.doc,
                    indent: command.indent,
                    in_rest: true,
                    in_suffix: false,
                }
            };
            if budget == 0 {
                return false;
            }
            budget -= 1;
            let Fit {
                mode, doc, in_rest, ..
            } = entry;
            match self.docs.kind(doc) {
                Kind::Nil => {}
                Kind::Text(index) => {
                    let text = &self.docs.texts[index as usize];
                    if suffix_pending && !entry.in_suffix && text.flags & STICKY == 0 {
                        // The text starts a new line after the pending
                        // comment. If breaking the group cannot move it,
                        // measure the group on from that line.
                        if in_rest || saw_line {
                            return in_rest;
                        }
                        suffix_pending = false;
                        space = false;
                        width = i64::from(self.options.line_width)
                            - i64::from(self.indent_width(entry.indent));
                    }
                    width -= i64::from(text.width) + i64::from(space);
                    space = false;
                    line_start = false;
                    if width < 0 {
                        return false;
                    }
                    if text.flags & MULTILINE != 0 {
                        return in_rest;
                    }
                }
                Kind::Space => space = !line_start,
                Kind::Line if mode == Mode::Flat => {
                    space = !line_start;
                    saw_line |= !in_rest;
                }
                Kind::SoftLine if mode == Mode::Flat => saw_line |= !in_rest,
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
                    stack.extend(children.iter().rev().map(|&doc| Fit { doc, ..entry }));
                }
                Kind::Indent(child) => stack.push(Fit {
                    doc: child,
                    indent: entry.indent + 1,
                    ..entry
                }),
                Kind::Group(child) => {
                    let mode = if self.docs.breaks(doc) {
                        Mode::Break
                    } else {
                        mode
                    };
                    stack.push(Fit {
                        mode,
                        doc: child,
                        ..entry
                    });
                }
                Kind::IfBreak(broken, flat) => stack.push(Fit {
                    doc: if mode == Mode::Break { broken } else { flat },
                    ..entry
                }),
                // The suffix (an end-of-line comment) counts toward the width
                // of the line it ends.
                Kind::LineSuffix(child) => {
                    suffix_pending = true;
                    stack.push(Fit {
                        mode: Mode::Flat,
                        doc: child,
                        in_suffix: true,
                        ..entry
                    });
                }
            }
        }
    }
}

/// One entry of the fit check's stack.
#[derive(Clone, Copy)]
struct Fit {
    mode: Mode,
    doc: Doc,
    indent: u32,
    in_rest: bool,
    in_suffix: bool,
}

fn leading_ws(line: &str) -> &str {
    let end = line
        .find(|ch: char| ch != ' ' && ch != '\t')
        .unwrap_or(line.len());
    &line[..end]
}

#[cfg(test)]
mod tests {
    use super::{FIT_BUDGET, print};
    use crate::{DocBuilder, FormatOptions};

    /// The fit check is bounded: a group whose flat contents take more than
    /// `FIT_BUDGET` commands breaks, even though it would fit by width.
    #[test]
    fn fit_check_gives_up_after_its_budget() {
        let render = |lines: usize| {
            let mut docs = DocBuilder::with_capacity(0);
            let mut parts = vec![docs.text("a")];
            parts.extend((0..lines).map(|_| docs.soft_line()));
            parts.push(docs.text("b"));
            let contents = docs.concat(parts);
            let root = docs.group(contents);
            let mut out = String::new();
            let _ = print(
                &docs,
                root,
                FormatOptions::default(),
                |_, _| false,
                true,
                &mut out,
            );
            out
        };
        assert_eq!(render(8), "ab\n");
        assert!(render(FIT_BUDGET).contains("a\n"));
    }
}
