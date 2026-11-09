//! Javadoc comments (`/**`): javadoc's reading of a comment (`parser`),
//! turned into words, blocks and verbatim regions, and their layout.
//!
//! HTML collapses whitespace outside `<pre>`, so the line breaks and blank
//! lines the layout chooses between words don't change the rendered page.
//! Every node whose whitespace javadoc keeps is an atom printed as written:
//! inline tags without rich text (`{@code}`, `{@literal}`, `{@value}`,
//! `{@snippet}`, unknown tags), references, HTML tags with their attribute
//! values, comments, erroneous nodes, and the raw text elements of HTML
//! (`<script>`, `<style>`, `<textarea>`, `<title>`), whose content HTML reads
//! with its whitespace. An atom that spans lines keeps its line breaks, and
//! the whitespace at the ends of its lines. `<pre>` and `<table>` elements
//! are verbatim regions, and so is a paragraph of plain text whose lines are
//! indented differently (ASCII art, a grammar, aligned text); their lines
//! lose their trailing whitespace outside atoms, which a browser doesn't
//! show even in `<pre>`.

use crate::frame::Line;
use crate::layout::{Join, Layout};
use crate::parser::{BlockTag, Comment, Node, Pos, Span, to_u32};
use crate::words;

/// A block-level HTML element that the layout gives a place of its own.
/// Names match without regard to case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Name {
    P,
    /// `<ul>`, `<ol>`, `<dl>`.
    List,
    /// `<li>`, `<dt>`, `<dd>`.
    Item,
    /// `<h1>` to `<h6>`.
    Heading,
    Br,
    /// Another block (`<blockquote>`, `<div>`, `<hr>`, a stray `</pre>`, …):
    /// its tags start lines, and so does the text after them.
    Block,
    /// `<pre>`, which starts a verbatim region.
    Pre,
    /// `<table>`, which starts a verbatim region.
    Table,
    /// A raw text element of HTML (`<script>`, `<style>`) or an escapable
    /// raw text element (`<textarea>`, `<title>`): HTML reads its content
    /// as text with its whitespace, so the element, to its end tag, is an
    /// atom.
    Raw,
}

impl Name {
    fn of(name: &str) -> Option<Self> {
        let is = |candidate: &str| name.eq_ignore_ascii_case(candidate);
        Some(if is("p") {
            Self::P
        } else if is("ul") || is("ol") || is("dl") {
            Self::List
        } else if is("li") || is("dt") || is("dd") {
            Self::Item
        } else if is("br") {
            Self::Br
        } else if is("pre") {
            Self::Pre
        } else if is("table") {
            Self::Table
        } else if is("script") || is("style") || is("textarea") || is("title") {
            Self::Raw
        } else if name.len() == 2
            && name.as_bytes()[0].eq_ignore_ascii_case(&b'h')
            && matches!(name.as_bytes()[1], b'1'..=b'6')
        {
            Self::Heading
        } else if [
            "blockquote",
            "div",
            "hr",
            "center",
            "section",
            "article",
            "aside",
            "nav",
            "header",
            "footer",
            "main",
            "figure",
            "figcaption",
            "details",
            "summary",
            "address",
            "fieldset",
            "form",
            "menu",
            "dir",
        ]
        .iter()
        .any(|candidate| is(candidate))
        {
            Self::Block
        } else {
            return None;
        })
    }
}

/// How the source separates a token from the token before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sep {
    /// Nothing: the token touches the one before it (`word<b>`).
    None,
    /// Spaces on the same line.
    Space,
    /// A line break.
    Line,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// A word or an atom, which may span lines.
    Word,
    /// A block tag starts here; its head words follow.
    BlockTag,
    /// A block-level element's tag.
    Element { name: Name, closing: bool },
    /// A verbatim region from its first character (`<pre>`, `<table>`): a
    /// block whose later lines are printed as written.
    Region,
    /// Whole lines printed as written (`span` from the first line's start to
    /// the last line's end).
    Lines,
    /// A blank line of the source.
    Blank,
    /// The source's line break before the next token stays.
    Break,
}

#[derive(Clone, Copy, Debug)]
struct Token {
    kind: Kind,
    span: Span,
    sep: Sep,
    /// Stays on the line of the word before it, after a space: a block
    /// tag's arguments, an inline tag's reference.
    glue: bool,
    /// Would start new structure at the start of a line: a wrap never puts
    /// it at a line start.
    sticky: bool,
    /// A word of text (not an atom).
    plain: bool,
}

/// A piece of a section (the description, a block tag), with the nodes of
/// inline tags' rich text in line.
#[derive(Clone, Copy, Debug)]
struct Piece {
    kind: PieceKind,
    span: Span,
    glue: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PieceKind {
    Text,
    Atom,
    Erroneous,
    Element { name: Option<Name>, closing: bool },
    Region,
}

/// Lays out a Javadoc comment read by `parser`.
pub(crate) fn lay_out<'a>(
    comment: &Comment,
    lines: &[Line<'a>],
    opener_text: bool,
    layout: &mut Layout<'_, '_, 'a>,
) {
    let mut tokens = Tokens {
        lines,
        list: Vec::with_capacity(lines.len() * 8),
        last_end: None,
        break_next: false,
        after_error: false,
        glue_next: false,
        kept: Vec::new(),
    };
    let mut pieces = Vec::new();
    flatten(&comment.body, lines, &mut pieces);
    tokens.section(&pieces);
    for tag in &comment.tags {
        tokens.block_tag(tag, &mut pieces);
    }
    if let Some(tail) = comment.tail {
        tokens.piece(Piece {
            kind: PieceKind::Atom,
            span: tail,
            glue: false,
        });
    }
    for &span in &tokens.kept {
        layout.keep_ends(span);
    }
    let mut tokens = tokens.list;
    verbatim_paragraphs(&mut tokens, lines, opener_text);
    if tokens.first().is_some_and(|first| {
        first.kind == Kind::Word
            && layout
                .text_at(first.span.start)
                .starts_with(|ch: char| ch == '*' || crate::java::is_whitespace(ch))
    }) {
        // On the opener's line, javadoc would drop a leading `*`.
        layout.multiline();
    }
    let mut html = Html {
        layout,
        frames: Vec::new(),
        in_tags: false,
        next_line: None,
        gap: Gap::None,
        attach: false,
    };
    for token in &tokens {
        html.token(token);
    }
}

/// Appends the pieces of `nodes`: inline tags' parts and rich text in line.
fn flatten(nodes: &[Node], lines: &[Line<'_>], pieces: &mut Vec<Piece>) {
    for node in nodes {
        match node {
            Node::Text(span) => pieces.push(Piece {
                kind: PieceKind::Text,
                span: *span,
                glue: false,
            }),
            Node::Atom(span) => pieces.push(Piece {
                kind: PieceKind::Atom,
                span: *span,
                glue: false,
            }),
            Node::Erroneous(span) => pieces.push(Piece {
                kind: PieceKind::Erroneous,
                span: *span,
                glue: false,
            }),
            Node::Element {
                span,
                name,
                closing,
            } => {
                let text = &lines[name.start.line as usize].text
                    [name.start.off as usize..name.end.off as usize];
                pieces.push(Piece {
                    kind: PieceKind::Element {
                        name: Name::of(text),
                        closing: *closing,
                    },
                    span: *span,
                    glue: false,
                });
            }
            Node::Inline(inline) => match &inline.rich {
                None => pieces.push(Piece {
                    kind: PieceKind::Atom,
                    span: inline.span,
                    glue: false,
                }),
                Some(rich) => {
                    pieces.push(Piece {
                        kind: PieceKind::Atom,
                        span: inline.name,
                        glue: false,
                    });
                    if let Some(head) = rich.head {
                        let reference = !lines[inline.name.start.line as usize].text
                            [inline.name.start.off as usize..]
                            .starts_with("{@index");
                        let joined = reference
                            .then(|| words::joined_pieces(lines, head))
                            .flatten();
                        for span in joined.unwrap_or_else(|| vec![head]) {
                            pieces.push(Piece {
                                kind: PieceKind::Atom,
                                span,
                                glue: true,
                            });
                        }
                    }
                    flatten(&rich.content, lines, pieces);
                    if let Some(close) = rich.close {
                        pieces.push(Piece {
                            kind: PieceKind::Atom,
                            span: close,
                            glue: false,
                        });
                    }
                }
            },
        }
    }
}

/// A block tag's head piece (its name, or an argument), in pieces joined
/// on one line (see [`words::joined_pieces`]); a quoted string stays whole.
pub(crate) fn head_parts(lines: &[Line<'_>], head: Span) -> Vec<Span> {
    let quoted = lines[head.start.line as usize].text[head.start.off as usize..].starts_with('"');
    let joined = (!quoted)
        .then(|| words::joined_pieces(lines, head))
        .flatten();
    joined.unwrap_or_else(|| vec![head])
}

/// Replaces each `<pre>` element (to its `</pre>`, or the section's end) and
/// each `<table>` element (to the `</table>` that closes it) with a region,
/// and each raw text element (to its end tag, or the section's end) with an
/// atom. Adds to `kept` the pieces in a region whose whitespace javadoc
/// reads.
fn regions(pieces: &mut Vec<Piece>, kept: &mut Vec<Span>) {
    let mut out = Vec::with_capacity(pieces.len());
    let mut index = 0;
    while index < pieces.len() {
        let piece = pieces[index];
        let PieceKind::Element {
            name: Some(name @ (Name::Pre | Name::Table | Name::Raw)),
            closing: false,
        } = piece.kind
        else {
            out.push(piece);
            index += 1;
            continue;
        };
        let mut depth = 0usize;
        let mut end = pieces.len() - 1;
        for (at, later) in pieces.iter().enumerate().skip(index) {
            if let PieceKind::Element {
                name: Some(found),
                closing,
            } = later.kind
                && found == name
            {
                if closing {
                    depth = depth.saturating_sub(1);
                    if depth == 0 || name != Name::Table {
                        end = at;
                        break;
                    }
                } else if name == Name::Table || depth == 0 {
                    depth += 1;
                }
            }
        }
        if name != Name::Raw {
            kept.extend(
                pieces[index..=end]
                    .iter()
                    .filter(|inner| inner.kind != PieceKind::Text)
                    .map(|inner| inner.span),
            );
        }
        out.push(Piece {
            kind: if name == Name::Raw {
                PieceKind::Atom
            } else {
                PieceKind::Region
            },
            span: Span {
                start: piece.span.start,
                end: pieces[end].span.end,
            },
            glue: false,
        });
        index = end + 1;
    }
    *pieces = out;
}

struct Tokens<'l, 'a> {
    lines: &'l [Line<'a>],
    list: Vec<Token>,
    /// The end of the last token.
    last_end: Option<Pos>,
    /// The next token starts a line: an atom on lines of its own ended.
    break_next: bool,
    /// The last token is an erroneous node: the separator after it keeps its
    /// kind.
    after_error: bool,
    /// The next token is the first of a block tag's text.
    glue_next: bool,
    /// The atoms that span lines, whose line ends keep their whitespace.
    kept: Vec<Span>,
}

impl Tokens<'_, '_> {
    fn section(&mut self, pieces: &[Piece]) {
        let mut pieces = pieces.to_vec();
        regions(&mut pieces, &mut self.kept);
        for piece in pieces {
            self.piece(piece);
        }
    }

    fn block_tag(&mut self, tag: &BlockTag, pieces: &mut Vec<Piece>) {
        self.list.push(Token {
            kind: Kind::BlockTag,
            span: Span {
                start: tag.span.start,
                end: tag.span.start,
            },
            sep: Sep::Line,
            glue: false,
            sticky: false,
            plain: false,
        });
        self.last_end = Some(tag.span.start);
        let Some(body) = &tag.body else {
            self.push(Kind::Word, tag.span, false, false);
            return;
        };
        for (index, &head) in tag.head.iter().enumerate() {
            for (part, span) in head_parts(self.lines, head).into_iter().enumerate() {
                self.push(Kind::Word, span, index > 0 || part > 0, false);
            }
        }
        // The first word of the tag's text stays on the tag's line.
        self.glue_next = true;
        pieces.clear();
        flatten(body, self.lines, pieces);
        let pieces = std::mem::take(pieces);
        self.section(&pieces);
    }

    fn piece(&mut self, piece: Piece) {
        match piece.kind {
            PieceKind::Text => self.text(piece.span),
            PieceKind::Atom => self.word(piece.span, piece.glue, false),
            PieceKind::Erroneous => {
                self.word(piece.span, piece.glue, false);
                self.after_error = true;
            }
            PieceKind::Element {
                name: Some(name),
                closing,
            } if name != Name::Raw && (!matches!(name, Name::Pre | Name::Table) || closing) => {
                let name = if matches!(name, Name::Pre | Name::Table) {
                    Name::Block
                } else {
                    name
                };
                self.push(Kind::Element { name, closing }, piece.span, false, false);
            }
            PieceKind::Element { .. } => self.push(Kind::Word, piece.span, false, false),
            PieceKind::Region => self.push(Kind::Region, piece.span, false, false),
        }
    }

    /// The text of line `line` from byte `from` to byte `to`.
    fn slice(&self, line: u32, from: u32, to: u32) -> &str {
        &self.lines[line as usize].text[from as usize..to as usize]
    }

    /// Whether only spaces and tabs come before `pos` on its line.
    fn at_line_start(&self, pos: Pos) -> bool {
        self.slice(pos.line, 0, pos.off)
            .bytes()
            .all(|byte| byte == b' ' || byte == b'\t')
    }

    /// Splits a text node into words at HTML whitespace and line breaks.
    fn text(&mut self, span: Span) {
        for line in span.start.line..=span.end.line {
            let text = self.lines[line as usize].text;
            let from = if line == span.start.line {
                span.start.off as usize
            } else {
                0
            };
            let to = if line == span.end.line {
                span.end.off as usize
            } else {
                text.len()
            };
            let segment = &text[from..to];
            let ruler = from == 0
                && to == text.len()
                && line > 0
                && words::is_ruler(segment)
                && span.start <= (Pos { line, off: 0 });
            if ruler {
                let spaces = [' ', '\t', '\u{c}'];
                let trimmed = segment.trim_matches(spaces);
                let start = to_u32(segment.len() - segment.trim_start_matches(spaces).len());
                let word = Span {
                    start: Pos { line, off: start },
                    end: Pos {
                        line,
                        off: start + to_u32(trimmed.len()),
                    },
                };
                self.push_break();
                self.push(Kind::Word, word, false, false);
                self.push_break();
                continue;
            }
            let mut at = 0;
            let bytes = segment.as_bytes();
            // Whether the next word is the first on its source line.
            let mut first_on_line = text[..from]
                .bytes()
                .all(|byte| matches!(byte, b' ' | b'\t' | 0x0c));
            while at < bytes.len() {
                while at < bytes.len() && matches!(bytes[at], b' ' | b'\t' | 0x0c) {
                    at += 1;
                }
                if at == bytes.len() {
                    break;
                }
                let start = at;
                while at < bytes.len() && !matches!(bytes[at], b' ' | b'\t' | 0x0c) {
                    at += 1;
                }
                let word = Span {
                    start: Pos {
                        line,
                        off: to_u32(from + start),
                    },
                    end: Pos {
                        line,
                        off: to_u32(from + at),
                    },
                };
                if std::mem::take(&mut first_on_line)
                    && line > 0
                    && words::is_list_marker(&segment[start..at])
                {
                    self.push_break();
                }
                self.word(word, false, true);
            }
        }
    }

    /// Adds a word (`plain`: of text) or an atom. A token that starts with
    /// `@` keeps whether it starts a line: javadoc reads an `@` after only
    /// spaces and tabs on its line as a block tag or an escape.
    fn word(&mut self, span: Span, glue: bool, plain: bool) {
        let text = &self.lines[span.start.line as usize].text[span.start.off as usize..];
        if text.starts_with('@') {
            if span.start.line > 0 && self.at_line_start(span.start) {
                self.push_break();
                self.push_token(Kind::Word, span, glue, false, plain);
            } else {
                self.push_token(Kind::Word, span, glue, true, plain);
            }
            return;
        }
        let end = if span.end.line == span.start.line {
            (span.end.off - span.start.off) as usize
        } else {
            text.len()
        };
        let sticky = words::starts_structure(&text[..end], false);
        self.push_token(Kind::Word, span, glue, sticky, plain);
    }

    fn push_break(&mut self) {
        if !self.list.is_empty() {
            self.list.push(Token {
                kind: Kind::Break,
                span: Span {
                    start: Pos { line: 0, off: 0 },
                    end: Pos { line: 0, off: 0 },
                },
                sep: Sep::Line,
                glue: false,
                sticky: false,
                plain: false,
            });
        }
    }

    /// Adds a blank line token when the source has a blank line between the
    /// last token and `start`.
    fn push_gap_blank(&mut self, start: Pos) {
        if let Some(end) = self.last_end
            && start.line > end.line + 1
        {
            self.list.push(Token {
                kind: Kind::Blank,
                span: Span { start, end: start },
                sep: Sep::Line,
                glue: false,
                sticky: false,
                plain: false,
            });
        }
    }

    fn push(&mut self, kind: Kind, span: Span, glue: bool, sticky: bool) {
        self.push_token(kind, span, glue, sticky, false);
    }

    fn push_token(&mut self, kind: Kind, span: Span, glue: bool, sticky: bool, plain: bool) {
        // An atom on lines of its own (a `{@snippet}`, an HTML comment) keeps
        // them: the line breaks around it stay.
        let own_lines = kind == Kind::Word && !plain && span.end.line > span.start.line;
        if std::mem::take(&mut self.break_next) || own_lines && self.at_line_start(span.start) {
            self.push_break();
        }
        if own_lines && self.at_line_start(span.start) {
            let line = self.lines[span.end.line as usize].text;
            self.break_next = line[span.end.off as usize..]
                .bytes()
                .all(|byte| matches!(byte, b' ' | b'\t' | 0x0c));
        }
        let sep = match self.last_end {
            None => Sep::Line,
            Some(end) if end == span.start => Sep::None,
            Some(end) if end.line < span.start.line => Sep::Line,
            Some(_) => Sep::Space,
        };
        let mut glue = glue || std::mem::take(&mut self.glue_next) && kind == Kind::Word;
        if std::mem::take(&mut self.after_error) {
            match sep {
                Sep::Line => self.push_break(),
                Sep::Space => glue = true,
                Sep::None => {}
            }
        }
        self.push_gap_blank(span.start);
        // Text between two tokens is whitespace; anything else is printed as
        // a word, so that no character is lost.
        if let Some(end) = self.last_end
            && end < span.start
            && let Some(gap) = self.gap_word(end, span.start)
        {
            self.list.push(Token {
                kind: Kind::Word,
                span: gap,
                sep: Sep::None,
                glue: false,
                sticky: true,
                plain: false,
            });
        }
        if matches!(kind, Kind::Word | Kind::Element { .. })
            && !plain
            && span.end.line > span.start.line
        {
            self.kept.push(span);
        }
        self.list.push(Token {
            kind,
            span,
            sep,
            glue,
            sticky,
            plain,
        });
        self.last_end = Some(span.end);
    }

    /// The span from the first to the last character between `from` and
    /// `to` that is not whitespace, if any.
    fn gap_word(&self, from: Pos, to: Pos) -> Option<Span> {
        let mut first = None;
        let mut last = None;
        for line in from.line..=to.line {
            let text = self.lines[line as usize].text;
            let start = if line == from.line {
                from.off as usize
            } else {
                0
            };
            let end = if line == to.line {
                to.off as usize
            } else {
                text.len()
            };
            for (at, ch) in text[start..end].char_indices() {
                if !crate::java::is_whitespace(ch) {
                    let pos = Pos {
                        line,
                        off: to_u32(start + at),
                    };
                    first.get_or_insert(pos);
                    last = Some(Pos {
                        line,
                        off: to_u32(start + at + ch.len_utf8()),
                    });
                }
            }
        }
        Some(Span {
            start: first?,
            end: last?,
        })
    }
}

/// Replaces the words of each paragraph of plain text whose lines are
/// indented differently, by two columns or more, with its lines printed as
/// written. A paragraph is a run of two or more lines that hold only words
/// of text, after a line that doesn't (a blank line, a block tag, a block
/// element, an inline tag). The comment's first line doesn't count when it
/// is on the line of `/**`.
fn verbatim_paragraphs(tokens: &mut Vec<Token>, lines: &[Line<'_>], opener_text: bool) {
    let mut plain = vec![true; lines.len()];
    for (line, flag) in lines.iter().zip(plain.iter_mut()) {
        if line.is_blank() {
            *flag = false;
        }
    }
    if opener_text {
        plain[0] = false;
    }
    let mut word_line = vec![false; lines.len()];
    for token in tokens.iter() {
        let (start, end) = (token.span.start.line as usize, token.span.end.line as usize);
        match token.kind {
            Kind::Word if token.plain => word_line[start] = true,
            Kind::Blank | Kind::Break => {}
            _ => plain[start..=end].iter_mut().for_each(|flag| *flag = false),
        }
    }
    for (flag, has_word) in plain.iter_mut().zip(&word_line) {
        *flag &= has_word;
    }
    let indent = |line: &Line<'_>| -> usize {
        let mut column = 0;
        for byte in line.text.bytes() {
            match byte {
                b' ' => column += 1,
                b'\t' => column += 4 - column % 4,
                _ => break,
            }
        }
        column
    };
    let mut kept = vec![false; lines.len()];
    let mut line = 0;
    while line < lines.len() {
        if !plain[line] {
            line += 1;
            continue;
        }
        let first = line;
        while line < lines.len() && plain[line] {
            line += 1;
        }
        if line - first >= 2 {
            let (least, most) = lines[first..line]
                .iter()
                .map(indent)
                .fold((usize::MAX, 0), |(least, most), column| {
                    (least.min(column), most.max(column))
                });
            if most >= least + 2 {
                kept[first..line].iter_mut().for_each(|flag| *flag = true);
            }
        }
    }
    if kept.contains(&true) {
        keep_lines(tokens, lines, &kept);
    }
}

/// Replaces the words on the `kept` lines with `Lines` tokens.
fn keep_lines(tokens: &mut Vec<Token>, lines: &[Line<'_>], kept: &[bool]) {
    let mut out = Vec::with_capacity(tokens.len());
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index];
        let line = token.span.start.line as usize;
        if token.kind != Kind::Word || !kept[line] {
            out.push(token);
            index += 1;
            continue;
        }
        let first = line;
        let mut last = line;
        let kept_word =
            |token: &Token| token.kind == Kind::Word && kept[token.span.start.line as usize];
        loop {
            match tokens.get(index) {
                Some(token) if kept_word(token) => {
                    last = token.span.start.line as usize;
                    index += 1;
                }
                Some(token)
                    if token.kind == Kind::Break
                        && tokens.get(index + 1).is_some_and(kept_word) =>
                {
                    index += 1;
                }
                _ => break,
            }
        }
        let first_pos = Pos {
            line: to_u32(first),
            off: 0,
        };
        out.push(Token {
            kind: Kind::Lines,
            span: Span {
                start: first_pos,
                end: Pos {
                    line: to_u32(last),
                    off: to_u32(lines[last].text.len()),
                },
            },
            sep: token.sep,
            glue: false,
            sticky: false,
            plain: false,
        });
    }
    *tokens = out;
}

/// Why the next word starts a line, from weakest to strongest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Why {
    /// A line break inside a paragraph (`<br>`, a list-like line, a ruler).
    Break,
    /// A new paragraph of text (after a blank line or a block).
    Text,
    /// A block element.
    Block,
}

/// What lies between the last word and the next, from weakest to strongest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Gap {
    None,
    /// A block ended: a list, a heading or a verbatim region.
    Block,
    /// A blank line of the source.
    Blank,
}

/// An open block that indents the lines inside it.
#[derive(Clone, Copy, Debug)]
enum Frame {
    /// Block tag text: its lines are indented 4.
    Tag,
    /// A list: its `<ul>` line at `open`, its items at `column`.
    List { open: u16, column: u16 },
    /// A list item: its tag at `line`, its other lines at `column`.
    Item { line: u16, column: u16 },
}

impl Frame {
    const fn column(self) -> u16 {
        match self {
            Self::Tag => 4,
            Self::List { column, .. } | Self::Item { column, .. } => column,
        }
    }
}

struct Html<'p, 'd, 'l, 'a> {
    layout: &'p mut Layout<'d, 'l, 'a>,
    frames: Vec<Frame>,
    /// Whether a block tag has started.
    in_tags: bool,
    /// The next word starts a line.
    next_line: Option<Why>,
    /// What lies between the last word and the next.
    gap: Gap,
    /// The next word follows the last with nothing between (after `<p>` or
    /// `<li>`).
    attach: bool,
}

impl Html<'_, '_, '_, '_> {
    fn token(&mut self, token: &Token) {
        match token.kind {
            Kind::Blank => {
                self.request(Why::Text);
                self.gap = Gap::Blank;
            }
            Kind::BlockTag => self.start_tag(),
            Kind::Break => self.request(Why::Break),
            Kind::Word => self.emit(token, token.glue),
            Kind::Element { name, closing } => self.element(token, name, closing),
            Kind::Region => {
                self.request(Why::Block);
                self.emit(token, false);
                self.gap = self.gap.max(Gap::Block);
                self.request(Why::Text);
            }
            Kind::Lines => {
                self.request(Why::Text);
                if let Some(why) = self.next_line.take() {
                    let column = self.column();
                    self.start(why, column, column);
                }
                self.layout
                    .verbatim_lines(token.span.start.line, token.span.end.line);
                self.attach = false;
                self.gap = Gap::Block;
                self.request(Why::Text);
            }
        }
    }

    /// The column of a new line in the innermost open block.
    fn column(&self) -> u16 {
        self.frames.last().map_or(0, |frame| frame.column())
    }

    /// Asks for a new line before the next word; the strongest reason wins.
    fn request(&mut self, why: Why) {
        self.next_line = Some(self.next_line.map_or(why, |next| next.max(why)));
    }

    /// Starts a line at `indent`, with wraps at `hang`. A blank line goes
    /// before it in the description, outside lists, before a block or after
    /// a blank line or a block; elsewhere (in a list or in tag text) only a
    /// blank line of the source is kept.
    fn start(&mut self, why: Why, indent: u16, hang: u16) {
        let blank = match why {
            Why::Break => false,
            _ if self.frames.is_empty() => why == Why::Block || self.gap != Gap::None,
            _ => self.gap == Gap::Blank,
        };
        self.layout.line(blank, indent, hang);
    }

    /// Adds a word. `glue` keeps it on the line of the word before.
    fn emit(&mut self, token: &Token, glue: bool) {
        if let Some(why) = self.next_line.take() {
            // A word that a line break would turn into structure stays on
            // the line where the source has it.
            if token.sep != Sep::Line && !self.layout.at_start() && token.sticky {
                self.layout.word(token.span, Join::Space, true);
            } else {
                let column = self.column();
                self.start(why, column, column);
                self.layout.word(token.span, Join::Wrap, token.sticky);
            }
        } else {
            let join = match token.sep {
                _ if self.attach => Join::None,
                Sep::None => Join::None,
                Sep::Space | Sep::Line if glue => Join::Space,
                Sep::Space | Sep::Line => Join::Wrap,
            };
            self.layout.word(token.span, join, token.sticky);
        }
        self.attach = false;
        self.gap = Gap::None;
    }

    /// A block tag starts on this line: it closes the open blocks, and a
    /// blank line goes before the first.
    fn start_tag(&mut self) {
        self.frames.clear();
        let blank = !self.in_tags;
        self.in_tags = true;
        self.layout.multiline();
        self.next_line = None;
        self.layout.line(blank, 0, Frame::Tag.column());
        self.frames.push(Frame::Tag);
        self.attach = false;
        self.gap = Gap::None;
    }

    /// Lays out a block-level element's tag.
    fn element(&mut self, token: &Token, name: Name, closing: bool) {
        match (name, closing) {
            (Name::P | Name::Heading, false) => {
                // `<li><p>` stays together.
                if !(self.attach && name == Name::P) {
                    self.request(Why::Block);
                }
                self.emit(token, false);
                self.attach = token.span.end.line == token.span.start.line;
            }
            (Name::P | Name::Heading, true) => {
                self.attach |= self.next_line.is_none();
                self.emit(token, true);
                if name == Name::Heading {
                    self.gap = self.gap.max(Gap::Block);
                    self.request(Why::Text);
                }
            }
            (Name::List, false) => {
                self.request(Why::Block);
                let open = self.column();
                self.emit(token, false);
                self.frames.push(Frame::List {
                    open,
                    column: open + 2,
                });
                self.request(Why::Text);
            }
            (Name::List, true) => {
                let Some(list) = self
                    .frames
                    .iter()
                    .rposition(|frame| matches!(frame, Frame::List { .. }))
                else {
                    self.emit(token, false);
                    return;
                };
                let Frame::List { open, .. } = self.frames[list] else {
                    unreachable!("a list frame");
                };
                self.frames.truncate(list);
                self.next_line = None;
                self.layout.line(self.gap == Gap::Blank, open, open);
                self.emit(token, false);
                self.gap = self.gap.max(Gap::Block);
                self.request(Why::Text);
            }
            (Name::Item, false) => {
                if matches!(self.frames.last(), Some(Frame::Item { .. })) {
                    self.frames.pop();
                }
                let line = self.column();
                self.next_line = None;
                self.start(Why::Block, line, line + 4);
                self.emit(token, false);
                self.frames.push(Frame::Item {
                    line,
                    column: line + 4,
                });
                self.attach = token.span.end.line == token.span.start.line;
            }
            (Name::Item, true) => {
                if let Some(&Frame::Item { line, .. }) = self.frames.last() {
                    if self.next_line.take().is_some() {
                        self.layout.line(self.gap == Gap::Blank, line, line);
                    } else {
                        self.attach = true;
                    }
                    self.emit(token, true);
                    self.frames.pop();
                } else {
                    self.emit(token, true);
                }
            }
            // A raw text element is an atom, and a stray end tag a word.
            (Name::Raw, _) => self.emit(token, false),
            (Name::Br, _) => {
                self.emit(token, true);
                self.request(Why::Break);
            }
            (Name::Block | Name::Pre | Name::Table, false) => {
                self.request(Why::Block);
                self.emit(token, false);
                self.request(Why::Text);
            }
            (Name::Block | Name::Pre | Name::Table, true) => {
                self.request(Why::Break);
                self.emit(token, false);
                self.gap = self.gap.max(Gap::Block);
                self.request(Why::Text);
            }
        }
    }
}
