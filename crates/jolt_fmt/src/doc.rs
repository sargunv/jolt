//! The document IR: an append-only arena of layout commands whose texts are
//! borrowed from the source or `'static`.

use unicode_width::UnicodeWidthChar;

/// A handle to a document node in a [`DocBuilder`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Doc(u32);

impl Doc {
    /// Nothing.
    pub const NIL: Self = Self(0);
    pub(crate) const SPACE: Self = Self(1);
    const LINE: Self = Self(2);
    const SOFT_LINE: Self = Self(3);
    pub(crate) const HARD_LINE: Self = Self(4);
    pub(crate) const EMPTY_LINE: Self = Self(5);
    pub(crate) const BREAK_PARENT: Self = Self(6);

    #[must_use]
    pub const fn is_nil(self) -> bool {
        self.0 == 0
    }

    const fn index(self) -> usize {
        self.0 as usize
    }
}

/// How the printer rewrites the continuation lines of a multi-line text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reindent<'a> {
    /// A multi-line block comment: star blocks are re-aligned under the
    /// opening delimiter, other lines are trimmed (see [`comment_lines`]).
    Comment,
    /// Verbatim source: `prefix` is removed from each continuation line that
    /// starts with it.
    Strip(&'a str),
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Kind {
    Nil,
    Text(u32),
    Space,
    Line,
    SoftLine,
    HardLine,
    EmptyLine,
    BreakParent,
    Concat(u32, u32),
    Group(Doc),
    Indent(Doc),
    IfBreak(Doc, Doc),
    LineSuffix(Doc),
}

const _: () = assert!(size_of::<Kind>() == 12);

pub(crate) const STICKY: u8 = 1;
pub(crate) const LINE_COMMENT: u8 = 2;
pub(crate) const MULTILINE: u8 = 4;
/// The first text of a verbatim node: the printer records the indentation of
/// the current output line as the base its `Strip` pieces re-indent against.
pub(crate) const VERBATIM_START: u8 = 8;

#[derive(Clone, Copy, Debug)]
#[allow(clippy::struct_field_names)]
pub(crate) struct Text<'a> {
    pub(crate) text: &'a str,
    /// Display width of the first output line.
    pub(crate) width: u32,
    pub(crate) flags: u8,
    pub(crate) reindent: Option<Reindent<'a>>,
}

/// Builds documents. One builder lives for one format run.
pub struct DocBuilder<'a> {
    pub(crate) nodes: Vec<Kind>,
    breaks: Vec<bool>,
    pub(crate) children: Vec<Doc>,
    pub(crate) texts: Vec<Text<'a>>,
}

impl<'a> DocBuilder<'a> {
    /// A builder sized for a tree of `tokens` tokens, so the arena rarely
    /// grows while a file is formatted.
    pub(crate) fn with_capacity(tokens: usize) -> Self {
        let fixed = [
            (Kind::Nil, false),
            (Kind::Space, false),
            (Kind::Line, false),
            (Kind::SoftLine, false),
            (Kind::HardLine, true),
            (Kind::EmptyLine, true),
            (Kind::BreakParent, true),
        ];
        let mut nodes = Vec::with_capacity(fixed.len() + tokens * 3);
        let mut breaks = Vec::with_capacity(nodes.capacity());
        for (kind, forced) in fixed {
            nodes.push(kind);
            breaks.push(forced);
        }
        Self {
            nodes,
            breaks,
            children: Vec::with_capacity(tokens * 3),
            texts: Vec::with_capacity(tokens + tokens / 4),
        }
    }

    pub(crate) fn kind(&self, doc: Doc) -> Kind {
        self.nodes[doc.index()]
    }

    pub(crate) fn breaks(&self, doc: Doc) -> bool {
        self.breaks[doc.index()]
    }

    /// Whether `doc` contains a forced break (a hard line, a blank line, a
    /// break-parent or a multi-line text), so no group around it is flat.
    #[must_use]
    pub fn will_break(&self, doc: Doc) -> bool {
        self.breaks(doc)
    }

    fn push(&mut self, kind: Kind, breaks: bool) -> Doc {
        let id = u32::try_from(self.nodes.len()).expect("document arena fits in u32");
        self.nodes.push(kind);
        self.breaks.push(breaks);
        Doc(id)
    }

    fn push_text(&mut self, text: Text<'a>) -> Doc {
        if text.text.is_empty() {
            return Doc::NIL;
        }
        let index = u32::try_from(self.texts.len()).expect("text table fits in u32");
        let breaks = text.flags & MULTILINE != 0;
        self.texts.push(text);
        self.push(Kind::Text(index), breaks)
    }

    /// Literal text. A text containing a line break is written raw and cannot
    /// be flat.
    pub fn text(&mut self, text: &'a str) -> Doc {
        self.text_with(text, None, 0)
    }

    /// A synthesized separator that prints before a pending line suffix
    /// instead of flushing it.
    pub fn sticky(&mut self, text: &'a str) -> Doc {
        self.push_text(Text {
            text,
            width: measure(text).0,
            flags: STICKY,
            reindent: None,
        })
    }

    /// A multi-line source slice whose continuation lines the printer rewrites
    /// (see [`Reindent`]). Cannot be flat when it spans lines.
    pub(crate) fn reindented(&mut self, text: &'a str, reindent: Reindent<'a>) -> Doc {
        self.text_with(text, Some(reindent), 0)
    }

    pub(crate) fn text_with(
        &mut self,
        text: &'a str,
        reindent: Option<Reindent<'a>>,
        flags: u8,
    ) -> Doc {
        let (width, multiline) = match reindent {
            None | Some(Reindent::Strip(_)) => measure(text),
            Some(Reindent::Comment) => {
                let width = comment_lines(text).next().map_or(0, |parts| {
                    parts.iter().map(|part| display_width(part)).sum()
                });
                (width, has_line_break(text))
            }
        };
        self.push_text(Text {
            text,
            width,
            flags: flags | if multiline { MULTILINE } else { 0 },
            reindent,
        })
    }

    pub(crate) fn comment_text(&mut self, text: &'a str, flags: u8) -> Doc {
        self.push_text(Text {
            text,
            width: measure(text).0,
            flags,
            reindent: None,
        })
    }

    /// One space; dropped at line start and before a newline; consecutive
    /// spaces collapse.
    #[must_use]
    pub const fn space(&self) -> Doc {
        Doc::SPACE
    }

    /// A space when flat, a newline when broken.
    #[must_use]
    pub const fn line(&self) -> Doc {
        Doc::LINE
    }

    /// Nothing when flat, a newline when broken.
    #[must_use]
    pub const fn soft_line(&self) -> Doc {
        Doc::SOFT_LINE
    }

    /// A newline that forces every enclosing group to break.
    #[must_use]
    pub const fn hard_line(&self) -> Doc {
        Doc::HARD_LINE
    }

    /// A blank line; forces enclosing groups to break.
    #[must_use]
    pub const fn empty_line(&self) -> Doc {
        Doc::EMPTY_LINE
    }

    /// A blank line when `blank`, else a hard line.
    #[must_use]
    pub const fn line_break(&self, blank: bool) -> Doc {
        if blank {
            Doc::EMPTY_LINE
        } else {
            Doc::HARD_LINE
        }
    }

    /// Forces every enclosing group to break.
    #[must_use]
    pub const fn break_parent(&self) -> Doc {
        Doc::BREAK_PARENT
    }

    /// A sequence. NIL elements are dropped.
    ///
    /// # Panics
    ///
    /// When the arena outgrows `u32` indices.
    pub fn concat(&mut self, docs: impl IntoIterator<Item = Doc>) -> Doc {
        let start = self.children.len();
        let mut breaks = false;
        for doc in docs {
            if !doc.is_nil() {
                breaks |= self.breaks(doc);
                self.children.push(doc);
            }
        }
        match self.children.len() - start {
            0 => Doc::NIL,
            1 => self.children.pop().expect("one child"),
            len => {
                let start = u32::try_from(start).expect("children fit in u32");
                let len = u32::try_from(len).expect("children fit in u32");
                self.push(Kind::Concat(start, len), breaks)
            }
        }
    }

    /// Flat if the contents fit, else broken.
    pub fn group(&mut self, doc: Doc) -> Doc {
        if doc.is_nil() {
            return doc;
        }
        let breaks = self.breaks(doc);
        self.push(Kind::Group(doc), breaks)
    }

    /// One more indentation level for newlines inside.
    pub fn indent(&mut self, doc: Doc) -> Doc {
        if doc.is_nil() {
            return doc;
        }
        let breaks = self.breaks(doc);
        self.push(Kind::Indent(doc), breaks)
    }

    /// `broken` or `flat` by the mode of the enclosing group.
    pub fn if_break(&mut self, broken: Doc, flat: Doc) -> Doc {
        let breaks = self.breaks(flat);
        self.push(Kind::IfBreak(broken, flat), breaks)
    }

    /// Deferred until the next newline, or the next text that is not sticky.
    pub fn line_suffix(&mut self, doc: Doc) -> Doc {
        if doc.is_nil() {
            return doc;
        }
        self.push(Kind::LineSuffix(doc), false)
    }
}

/// Builds a concatenation, evaluating every argument before borrowing the
/// builder: `cat![f; a, b, c]`.
#[macro_export]
macro_rules! cat {
    ($f:expr; $($doc:expr),* $(,)?) => {{
        let docs = [$($doc),*];
        ($f).concat(docs)
    }};
}

/// Builds a group of a concatenation: `group![f; a, b, c]`.
#[macro_export]
macro_rules! group {
    ($f:expr; $($doc:expr),* $(,)?) => {{
        let docs = [$($doc),*];
        let doc = ($f).concat(docs);
        ($f).group(doc)
    }};
}

/// Builds an indented concatenation: `indent![f; a, b, c]`.
#[macro_export]
macro_rules! indent {
    ($f:expr; $($doc:expr),* $(,)?) => {{
        let docs = [$($doc),*];
        let doc = ($f).concat(docs);
        ($f).indent(doc)
    }};
}

pub(crate) fn has_line_break(text: &str) -> bool {
    text.bytes().any(|byte| matches!(byte, b'\n' | b'\r'))
}

fn first_line(text: &str) -> &str {
    text.bytes()
        .position(|byte| matches!(byte, b'\n' | b'\r'))
        .map_or(text, |end| &text[..end])
}

/// Splits on `\r\n`, `\r` and `\n`.
pub(crate) fn lines(text: &str) -> impl Iterator<Item = &str> + Clone {
    let mut rest = Some(text);
    std::iter::from_fn(move || {
        let text = rest?;
        if let Some(end) = text.find(['\n', '\r']) {
            let skip = if text[end..].starts_with("\r\n") {
                2
            } else {
                1
            };
            rest = Some(&text[end + skip..]);
            Some(&text[..end])
        } else {
            rest = None;
            Some(text)
        }
    })
}

/// The display width of the first line of `text`, and whether `text` spans
/// lines, in one pass over its bytes. ASCII is measured as [`display_width`]
/// measures it; the rest of a text from its first non-ASCII byte goes to
/// [`display_width`].
fn measure(text: &str) -> (u32, bool) {
    let mut width = 0;
    for (index, byte) in text.bytes().enumerate() {
        match byte {
            b'\t' | b' '..=b'~' => width += 1,
            b'\n' | b'\r' => return (width, true),
            0x80.. => {
                // Every byte before `index` is ASCII, so it is a char boundary.
                let rest = &text[index..];
                return (
                    width + display_width(first_line(rest)),
                    has_line_break(rest),
                );
            }
            _ => {}
        }
    }
    (width, false)
}

/// Display width, with a tab counting one column.
pub(crate) fn display_width(text: &str) -> u32 {
    if text.is_ascii() {
        let printable = text
            .bytes()
            .filter(|byte| matches!(byte, b'\t' | b' '..=b'~'))
            .count();
        return u32::try_from(printable).expect("width fits in u32");
    }
    text.chars()
        .map(|ch| {
            if ch == '\t' {
                1
            } else {
                u32::try_from(ch.width().unwrap_or(0)).expect("char width fits in u32")
            }
        })
        .sum()
}

/// One output line of a block comment, as parts to concatenate.
pub(crate) type CommentLine<'t> = [&'t str; 3];

/// The output lines of a block comment: a multi-line doc comment (`/**`) is
/// a star block whose opener text moves to its own line; a multi-line block
/// comment whose continuation starts with `*` is a star block that keeps its
/// opener text; any other comment keeps its continuation lines' indentation
/// relative to each other. A star line keeps its text after the `*` and one
/// space, so `<pre>` code stays indented.
pub(crate) fn comment_lines(text: &str) -> impl Iterator<Item = CommentLine<'_>> {
    let doc = text.starts_with("/**") && !text.starts_with("/**/");
    let content = strip_delimiters(text);
    let multiline = has_line_break(content);
    let star_block = (doc && multiline) || is_star_block(content);
    // Exactly one of the two layouts is `Some`.
    let plain = (!star_block).then(|| {
        let mut lines = lines(text.trim());
        let first = lines.next().unwrap_or("").trim();
        let shared = shared_indentation(lines.clone());
        let rest = lines.map(move |line| [line.trim_end().get(shared..).unwrap_or(""), "", ""]);
        std::iter::once([first, "", ""]).chain(rest)
    });
    let star = star_block.then(|| {
        let mut lines = lines(content);
        let mut first = [if doc { "/**" } else { "/*" }, "", ""];
        let mut has_content = false;
        if !doc && multiline {
            let opener = lines.next().unwrap_or("").trim();
            if !opener.is_empty() {
                has_content = true;
                first[1] = " ";
                first[2] = opener;
            }
        }
        let mut blank_lines = 0;
        let body = lines
            .filter_map(move |line| {
                let line = match line.trim_start().strip_prefix('*') {
                    Some(rest) if multiline => {
                        let rest = rest.trim_end();
                        rest.strip_prefix(' ').unwrap_or(rest)
                    }
                    _ => line.trim(),
                };
                if line.is_empty() {
                    blank_lines += usize::from(has_content);
                    return None;
                }
                has_content = true;
                let blanks = std::iter::repeat_n([" *", "", ""], std::mem::take(&mut blank_lines));
                Some(blanks.chain([[" * ", line, ""]]))
            })
            .flatten();
        std::iter::once(first).chain(body).chain([[" */", "", ""]])
    });
    plain
        .into_iter()
        .flatten()
        .chain(star.into_iter().flatten())
}

/// The byte length of the leading spaces and tabs all non-blank `lines`
/// share.
fn shared_indentation<'t>(lines: impl Iterator<Item = &'t str>) -> usize {
    let mut shared: Option<&str> = None;
    for line in lines.filter(|line| !line.trim().is_empty()) {
        let indentation = &line[..line.len() - line.trim_start_matches([' ', '\t']).len()];
        let common = shared.map_or(indentation.len(), |shared| {
            shared
                .bytes()
                .zip(indentation.bytes())
                .take_while(|(left, right)| left == right)
                .count()
        });
        shared = Some(&indentation[..common]);
    }
    shared.map_or(0, str::len)
}

fn strip_delimiters(text: &str) -> &str {
    let text = text.trim();
    let body = text.strip_suffix("*/").unwrap_or(text);
    body.strip_prefix("/**")
        .or_else(|| body.strip_prefix("/*"))
        .unwrap_or(body)
}

fn is_star_block(content: &str) -> bool {
    let mut lines = lines(content);
    let opener = lines.next().unwrap_or("");
    let continuation = lines.find(|line| !line.trim().is_empty());
    let multiline = has_line_break(content);
    multiline
        && (opener.trim_start().starts_with('*')
            || continuation.is_some_and(|line| line.trim_start().starts_with('*'))
            || content.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::{display_width, first_line, has_line_break, measure};

    /// The one-pass ASCII measure agrees with `display_width` on every ASCII
    /// byte, control bytes included, and around the non-ASCII fallback.
    #[test]
    fn measure_matches_display_width() {
        let mut texts: Vec<String> = (0..0x80u8)
            .map(|byte| format!("a{}b", char::from(byte)))
            .collect();
        texts
            .extend(["", "x\u{7}é\tz", "ab\u{4e2d}\u{301}c\r\nd", "é\n\u{4e2d}"].map(String::from));
        for text in &texts {
            let expected = (display_width(first_line(text)), has_line_break(text));
            assert_eq!(measure(text), expected, "{text:?}");
        }
    }
}
