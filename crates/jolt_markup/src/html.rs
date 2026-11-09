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
//! indented differently (ASCII art, a grammar, aligned text). The lines of a
//! `<pre>` element keep their trailing whitespace, which is its text; the
//! other lines lose theirs outside atoms, since HTML collapses it.

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
    /// as text with its whitespace, up to its own end tag, so the element,
    /// to that end tag, is an atom.
    Raw(Raw),
}

/// The raw text elements of HTML, and its escapable raw text elements.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Raw {
    Script,
    Style,
    Textarea,
    Title,
}

impl Raw {
    /// The element's name, in lower case.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Script => "script",
            Self::Style => "style",
            Self::Textarea => "textarea",
            Self::Title => "title",
        }
    }

    /// The element named `name`, without regard to case.
    pub(crate) fn of(name: &str) -> Option<Self> {
        let is = |candidate: &str| name.eq_ignore_ascii_case(candidate);
        Some(if is("script") {
            Self::Script
        } else if is("style") {
            Self::Style
        } else if is("textarea") {
            Self::Textarea
        } else if is("title") {
            Self::Title
        } else {
            return None;
        })
    }
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
        } else if let Some(raw) = Raw::of(name) {
            Self::Raw(raw)
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
#[allow(
    clippy::struct_excessive_bools,
    reason = "independent properties of a token"
)]
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
    /// Follows a tag's name or argument after whitespace, which stays: a
    /// reference ends at whitespace.
    after_head: bool,
    /// Follows an erroneous node on its line, after a space or nothing,
    /// which stays: what follows the node decided that javac reads it as an
    /// error, and a line break there could make it valid.
    pinned: bool,
}

/// A piece of a section (the description, a block tag), with the nodes of
/// inline tags' rich text in line.
#[derive(Clone, Copy, Debug)]
struct Piece {
    kind: PieceKind,
    span: Span,
    glue: bool,
    /// An inline tag's name or head (its reference, its index term): the
    /// separator after it stays.
    head: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PieceKind {
    Text,
    Atom,
    Erroneous,
    Element {
        name: Option<Name>,
        closing: bool,
    },
    /// A `<pre>` or `<table>` region; `error` when an erroneous node ends
    /// it, so the separator after it keeps its kind.
    Region {
        error: bool,
    },
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
        head_next: false,
        head_last: false,
        kept: Vec::new(),
        indents: lines
            .iter()
            .map(|line| {
                let text = line.text;
                let tabs = text.len() - text.trim_start_matches([' ', '\t']).len();
                let feeds = text.len() - text.trim_start_matches([' ', '\t', '\u{c}']).len();
                (to_u32(tabs), to_u32(feeds))
            })
            .collect(),
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
            head: false,
        });
    }
    for &span in &tokens.kept {
        layout.keep_ends(span);
    }
    // An erroneous node that ends the comment keeps the line break after it
    // (an unfinished `{@snippet :`), which the one-line form would drop.
    if tokens.after_error
        && tokens
            .last_end
            .is_some_and(|end| end.line as usize + 1 < lines.len())
    {
        layout.multiline();
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
        ends_sentence: false,
        lone_first: None,
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
                head: false,
            }),
            Node::Atom(span) => pieces.push(Piece {
                kind: PieceKind::Atom,
                span: *span,
                glue: false,
                head: false,
            }),
            Node::Erroneous(span) => pieces.push(Piece {
                kind: PieceKind::Erroneous,
                span: *span,
                glue: false,
                head: false,
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
                    head: false,
                });
            }
            Node::Inline(inline) => match &inline.rich {
                None => pieces.push(Piece {
                    kind: PieceKind::Atom,
                    span: inline.span,
                    glue: false,
                    head: false,
                }),
                Some(rich) => {
                    pieces.push(Piece {
                        kind: PieceKind::Atom,
                        span: inline.name,
                        glue: false,
                        head: true,
                    });
                    if let Some(head) = rich.head {
                        let reference = !lines[inline.name.start.line as usize].text
                            [inline.name.start.off as usize..]
                            .starts_with("{@index");
                        let joined = reference
                            .then(|| words::joined_pieces(lines, head))
                            .flatten();
                        // An unfinished reference can be empty; an empty word
                        // would start a line of its own.
                        for span in joined
                            .unwrap_or_else(|| vec![head])
                            .into_iter()
                            .filter(|span| span.start != span.end)
                        {
                            pieces.push(Piece {
                                kind: PieceKind::Atom,
                                span,
                                glue: true,
                                head: true,
                            });
                        }
                    }
                    flatten(&rich.content, lines, pieces);
                    if let Some(close) = rich.close {
                        // After a head with no text, javac reads the `}` at
                        // a line start and keeps reading the line as just
                        // started, so an `@` right after it would start a
                        // block tag: the `}` stays on the head's line.
                        pieces.push(Piece {
                            kind: PieceKind::Atom,
                            span: close,
                            glue: rich.content.is_empty(),
                            head: false,
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

/// Replaces each `<pre>` element (to the `</pre>` that closes it, or the
/// section's end) and each `<table>` element (likewise) with a region, and
/// each raw text element (to its own end tag, or the section's end) with an
/// atom. Adds to `kept` the spans whose line ends keep their whitespace:
/// each `<pre>` element, and in a `<table>` the raw text elements, the
/// `<pre>` elements and the pieces whose whitespace javadoc reads.
fn regions(pieces: &mut Vec<Piece>, kept: &mut Vec<Span>) {
    let mut out = Vec::with_capacity(pieces.len());
    let mut index = 0;
    while index < pieces.len() {
        let piece = pieces[index];
        let Some(name) = opens_region(piece) else {
            out.push(piece);
            index += 1;
            continue;
        };
        let mut end = element_end(pieces, index, name, pieces.len() - 1);
        let kind = match name {
            Name::Raw(_) => PieceKind::Atom,
            Name::Pre => PieceKind::Region { error: false },
            _ => {
                // Without its end tag, the table ends at the last piece,
                // which it holds.
                let mut stop = if closes(pieces[end], name) {
                    end
                } else {
                    end + 1
                };
                let mut inner = index + 1;
                while inner < stop {
                    let piece = pieces[inner];
                    match opens_region(piece) {
                        Some(name @ (Name::Pre | Name::Raw(_))) => {
                            // A `<pre>` that the table doesn't close runs on
                            // past its end tag, as javadoc reads it, and the
                            // region with it. (A raw text element can't: the
                            // table's end tag was looked for past it.)
                            let mut last = element_end(pieces, inner, name, end);
                            if !closes(pieces[last], name) && last < pieces.len() - 1 {
                                last = element_end(pieces, inner, name, pieces.len() - 1);
                                if last > end {
                                    end = last;
                                    stop = last + 1;
                                }
                            }
                            kept.push(Span {
                                start: piece.span.start,
                                end: pieces[last].span.end,
                            });
                            inner = last;
                        }
                        _ if piece.kind != PieceKind::Text => kept.push(piece.span),
                        _ => {}
                    }
                    inner += 1;
                }
                PieceKind::Region { error: false }
            }
        };
        // An empty piece (an unfinished tag at the comment's end) doesn't
        // move the region's end past the whitespace before it, which the
        // frame prints.
        let last = pieces[index..=end]
            .iter()
            .rev()
            .find(|piece| piece.span.start != piece.span.end)
            .unwrap_or(&piece);
        let span = Span {
            start: piece.span.start,
            end: last.span.end,
        };
        if name == Name::Pre {
            kept.push(span);
        }
        // An erroneous node that ends the region (an unclosed `<pre>`) still
        // keeps the separator after it.
        let error = [pieces[end].kind, last.kind].contains(&PieceKind::Erroneous);
        let kind = match kind {
            PieceKind::Atom if error => PieceKind::Erroneous,
            PieceKind::Region { .. } => PieceKind::Region { error },
            kind => kind,
        };
        out.push(Piece {
            kind,
            span,
            glue: false,
            head: false,
        });
        index = end + 1;
    }
    *pieces = out;
}

/// Whether `piece` is the end tag of the element `name`.
fn closes(piece: Piece, name: Name) -> bool {
    matches!(
        piece.kind,
        PieceKind::Element { name: Some(found), closing: true } if found == name
    )
}

/// The element (`<pre>`, `<table>` or a raw text element) that `piece`
/// opens, if any.
const fn opens_region(piece: Piece) -> Option<Name> {
    match piece.kind {
        PieceKind::Element {
            name: Some(name @ (Name::Pre | Name::Table | Name::Raw(_))),
            closing: false,
        } => Some(name),
        _ => None,
    }
}

/// The index of the piece that ends the element `name` opened at
/// `pieces[open]`: the end tag that closes it (counting nested elements of
/// the same name, except for a raw text element, which holds no elements),
/// or the last piece. A raw text element inside it is skipped to its own end
/// tag, since HTML reads what it holds as text. The search goes no further
/// than `pieces[last]`, which it returns when no end tag comes first: an
/// element inside a table ends with the table at the latest, so its search
/// stays inside the table.
fn element_end(pieces: &[Piece], open: usize, name: Name, last: usize) -> usize {
    let raw = matches!(name, Name::Raw(_));
    let mut depth = 1usize;
    let mut at = open + 1;
    while at <= last {
        if let PieceKind::Element {
            name: Some(found),
            closing,
        } = pieces[at].kind
        {
            if found == name {
                if closing {
                    depth -= 1;
                    if depth == 0 {
                        return at;
                    }
                } else if !raw {
                    depth += 1;
                }
            } else if !raw && !closing && matches!(found, Name::Raw(_)) {
                at = element_end(pieces, at, found, last);
            }
        }
        at += 1;
    }
    last
}

#[allow(
    clippy::struct_excessive_bools,
    reason = "the independent states of the tokenizer"
)]
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
    /// The next token is a tag's name or argument, and the last one was.
    head_next: bool,
    head_last: bool,
    /// The atoms that span lines, whose line ends keep their whitespace.
    kept: Vec<Span>,
    /// Per line, the length of its leading spaces and tabs, and of its
    /// leading spaces, tabs and form feeds: found once, so a line's many
    /// text nodes don't each rescan its indentation.
    indents: Vec<(u32, u32)>,
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
        // javac reads a tag written right after an erroneous one (`@code`
        // used as a block tag) as a tag; after a line break, it reads no
        // further.
        let pinned = std::mem::take(&mut self.after_error) && self.last_end == Some(tag.span.start);
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
            after_head: false,
            pinned,
        });
        self.last_end = Some(tag.span.start);
        self.head_last = false;
        let Some(body) = &tag.body else {
            self.push(Kind::Word, tag.span, false, false);
            self.after_error = true;
            return;
        };
        for (index, &head) in tag.head.iter().enumerate() {
            for (part, span) in head_parts(self.lines, head).into_iter().enumerate() {
                self.head_next = true;
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
        self.head_next = piece.head;
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
            } if !matches!(name, Name::Raw(_))
                && (!matches!(name, Name::Pre | Name::Table) || closing) =>
            {
                let name = if matches!(name, Name::Pre | Name::Table) {
                    Name::Block
                } else {
                    name
                };
                self.push(Kind::Element { name, closing }, piece.span, false, false);
            }
            PieceKind::Element { .. } => self.push(Kind::Word, piece.span, false, false),
            PieceKind::Region { error } => {
                self.push(Kind::Region, piece.span, false, false);
                self.after_error = error;
            }
        }
    }

    /// Whether only spaces and tabs come before `pos` on its line.
    fn at_line_start(&self, pos: Pos) -> bool {
        pos.off <= self.indents[pos.line as usize].0
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
            // The parser's text ends before the line's trailing whitespace,
            // which the layout drops.
            let ruler = from == 0
                && text[to..]
                    .bytes()
                    .all(|byte| matches!(byte, b' ' | b'\t' | 0x0c))
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
            let mut first_on_line = to_u32(from) <= self.indents[line as usize].1;
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

    /// Asks for a line break before the next token, unless it starts the
    /// comment or a block tag, which start their line already.
    fn push_break(&mut self) {
        if self
            .list
            .last()
            .is_some_and(|token| token.kind != Kind::BlockTag)
        {
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
                after_head: false,
                pinned: false,
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
                after_head: false,
                pinned: false,
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
        let after_error = std::mem::take(&mut self.after_error);
        let pinned = after_error && sep != Sep::Line;
        if after_error {
            match sep {
                Sep::Line => self.push_break(),
                Sep::Space => glue = true,
                Sep::None => {}
            }
        }
        self.push_gap_blank(span.start);
        let after_head = std::mem::replace(&mut self.head_last, false) && sep != Sep::None;
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
                after_head: false,
                pinned: false,
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
            after_head,
            pinned,
        });
        self.head_last = std::mem::take(&mut self.head_next);
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
            after_head: false,
            pinned: false,
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

/// Whether the HTML tag at the start of `text` ends javadoc's first
/// sentence by itself, as `<p>`, `<pre>` and the headings and their end tags
/// do (javadoc's `DocTreeMaker` sentence break tags).
fn breaks_sentence(text: &str) -> bool {
    let name = text.strip_prefix('<').unwrap_or("");
    let name = name
        .strip_prefix('/')
        .unwrap_or(name)
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .next()
        .unwrap_or("");
    ["p", "pre", "h1", "h2", "h3", "h4", "h5", "h6"]
        .iter()
        .any(|tag| name.eq_ignore_ascii_case(tag))
}

/// The deepest indentation nested lists get, in columns after the frame:
/// deeper lists keep it, so output stays linear in the input however deeply
/// lists nest (HTML ignores the indentation).
const MAX_INDENT: u16 = 24;

#[allow(
    clippy::struct_excessive_bools,
    reason = "the independent states of the layout"
)]
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
    /// The last word ends a sentence (`.`, `!`, `?`): javadoc's first
    /// sentence ends there if whitespace follows.
    ends_sentence: bool,
    /// The start of the last word, when it is the comment's first and one
    /// character long (see [`offset_one`](Self::offset_one)).
    lone_first: Option<Pos>,
}

impl Html<'_, '_, '_, '_> {
    fn token(&mut self, token: &Token) {
        match token.kind {
            Kind::Blank => {
                self.request(Why::Text);
                self.gap = Gap::Blank;
            }
            Kind::BlockTag => self.start_tag(token.pinned),
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

    /// Whether a line break before `token`, a tag that would start a line,
    /// would change javadoc's reading; `emit` then keeps it on the line. That
    /// is a `pinned` token, after an erroneous node; and a tag written right
    /// after the word before it, where the break would move the end of the
    /// first sentence: after a sentence's end, whitespace would end the
    /// sentence there, unless the tag ends it itself; and a tag that ends it
    /// (`<p>`) doesn't when it follows a lone first character, at offset 1,
    /// where whitespace would move it.
    fn keeps_line(&self, token: &Token) -> bool {
        token.pinned
            || token.sep == Sep::None
                && !self.layout.at_start()
                && self
                    .offset_one(token)
                    .unwrap_or_else(|| self.ends_sentence && !self.breaks_sentence(token))
    }

    /// Whether `token` is a tag that ends javadoc's first sentence.
    fn breaks_sentence(&self, token: &Token) -> bool {
        breaks_sentence(self.layout.text_at(token.span.start))
    }

    /// For a tag that ends the first sentence (`<p>`, `</p>`) right after
    /// the comment's lone first character: whether it is at offset 1 of
    /// what javadoc reads, where `DocTreeMaker.isSentenceBreak` reads it as
    /// no break. The output puts the character at offset 0, so the tag must
    /// follow it with nothing between exactly when this holds. javadoc
    /// counts each line before the first word (line 0 is the first it
    /// reads), the indentation it keeps before it, and the separator.
    fn offset_one(&self, token: &Token) -> Option<bool> {
        let start = self.lone_first.filter(|_| self.breaks_sentence(token))?;
        Some(start == Pos { line: 0, off: 0 } && token.sep == Sep::None)
    }

    /// Adds a word. `glue` keeps it on the line of the word before.
    fn emit(&mut self, token: &Token, glue: bool) {
        let first = self.layout.at_start();
        // A pinned token keeps the separator before it: no line break, and
        // no wrap point.
        let glue = glue || token.pinned;
        if token.pinned && self.next_line.take().is_some() {
            self.layout.hang(self.column());
        }
        let text = self
            .layout
            .segment(token.span.start.line, token.span.start.off, None);
        if let Some(why) = self.next_line.take() {
            // A line break asked for and not taken still moves the wraps to
            // the innermost open block's column.
            if why == Why::Block && self.keeps_line(token) {
                self.layout.hang(self.column());
                self.layout.word(token.span, Join::None, token.sticky);
            } else if !self.layout.at_start()
                && token.sticky
                && (token.sep != Sep::Line || text.starts_with('@'))
            {
                // A word that a line break would turn into structure stays on
                // the line of the word before: always for `@`, which would
                // become a block tag; for the other words, unless the source
                // starts a line with it. It joins that word as it would on
                // the line, without a wrap point.
                let join = self.join(token, true);
                self.layout.hang(self.column());
                self.layout.word(token.span, join, true);
            } else {
                let column = self.column();
                self.start(why, column, column);
                self.layout.word(token.span, Join::Wrap, token.sticky);
            }
        } else {
            let join = self.join(token, glue);
            self.layout.word(token.span, join, token.sticky);
        }
        let last = if token.span.end.line == token.span.start.line {
            &text[..(token.span.end.off - token.span.start.off) as usize]
        } else {
            self.layout
                .segment(token.span.end.line, 0, Some(token.span.end.off))
        };
        self.ends_sentence = last.ends_with(['.', '!', '?']);
        self.lone_first = (first
            && token.span.end.line == token.span.start.line
            && last.encode_utf16().count() == 1)
            .then_some(token.span.start);
        self.attach = false;
        self.gap = Gap::None;
    }

    /// How `token` joins the word before it on its line; `glue` leaves no
    /// wrap point.
    fn join(&self, token: &Token, glue: bool) -> Join {
        match token.sep {
            // A tag that ends the sentence after a lone first character keeps
            // it ending there (or not).
            _ if self.offset_one(token) == Some(false) => Join::Space,
            // After an erroneous node, the separator stays.
            Sep::Space | Sep::Line if token.pinned => Join::Space,
            // A space after a sentence's end stays: it ends javadoc's first
            // sentence there.
            Sep::Space | Sep::Line if self.attach && self.ends_sentence => Join::Space,
            _ if self.attach && !token.after_head => Join::None,
            Sep::None => Join::None,
            Sep::Space | Sep::Line if glue => Join::Space,
            Sep::Space | Sep::Line => Join::Wrap,
        }
    }

    /// A block tag starts on this line, unless it is `pinned` to the
    /// erroneous tag before it: it closes the open blocks, and a blank line
    /// goes before the first.
    fn start_tag(&mut self, pinned: bool) {
        self.frames.clear();
        let blank = !self.in_tags;
        self.in_tags = true;
        self.layout.multiline();
        self.next_line = None;
        if !pinned {
            self.layout.line(blank, 0, Frame::Tag.column());
        }
        self.frames.push(Frame::Tag);
        self.attach = false;
        self.gap = Gap::None;
    }

    /// Lays out a list item's start tag (`<li>`, `<dt>`, `<dd>`), which
    /// closes the item before it.
    fn open_item(&mut self, token: &Token) {
        if matches!(self.frames.last(), Some(Frame::Item { .. })) {
            self.frames.pop();
        }
        let line = self.column();
        let column = (line + 4).min(MAX_INDENT);
        let kept = self.keeps_line(token);
        if kept {
            self.next_line = Some(Why::Block);
        } else {
            self.next_line = None;
            self.start(Why::Block, line, column);
        }
        self.emit(token, false);
        self.frames.push(Frame::Item { line, column });
        if kept {
            // The item's wraps go to its column, as when it starts a line.
            self.layout.hang(column);
        }
        self.attach = token.span.end.line == token.span.start.line;
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
                    column: (open + 2).min(MAX_INDENT),
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
                if self.keeps_line(token) {
                    self.next_line = Some(Why::Block);
                } else {
                    self.next_line = None;
                    self.layout.line(self.gap == Gap::Blank, open, open);
                }
                self.emit(token, false);
                self.gap = self.gap.max(Gap::Block);
                self.request(Why::Text);
            }
            (Name::Item, false) => self.open_item(token),
            (Name::Item, true) => {
                if let Some(&Frame::Item { line, .. }) = self.frames.last() {
                    if self.next_line.take().is_some() {
                        self.layout.line(self.gap == Gap::Blank, line, line);
                    } else {
                        self.attach = true;
                    }
                    self.emit(token, true);
                    self.frames.pop();
                    // Wraps after the item go to the column around it.
                    self.layout.hang(self.column());
                } else {
                    self.emit(token, true);
                }
            }
            // A raw text element is an atom, and a stray end tag a word.
            (Name::Raw(_), _) => self.emit(token, false),
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
                if self.keeps_line(token) {
                    self.next_line = Some(Why::Block);
                } else {
                    self.request(Why::Break);
                }
                self.emit(token, false);
                self.gap = self.gap.max(Gap::Block);
                self.request(Why::Text);
            }
        }
    }
}
