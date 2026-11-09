//! The language-agnostic formatter core: options, the document IR, the
//! printer, comment placement and printing, verbatim emission, item lists with
//! formatter-ignore, and the [`Formatter`] context. See
//! `docs/internals/formatter.md`.

mod comments;
mod doc;
mod formatter;
mod printer;

use std::ops::ControlFlow;

use jolt_syntax::{Language, RawSyntaxKind, SyntaxNode, SyntaxToken, TriviaKind};

pub use comments::{
    Comment, CommentContext, CommentKind, CommentLayout, DocComment, DocCommentSource, Placement,
    is_unclosed, leading_target, trailing_target,
};
pub use doc::{Doc, DocBuilder};
pub use formatter::{Formatter, ListItem, Separator};

/// The options of one format run. The user-facing options, their names and
/// their validation live in `jolt_config`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct FormatOptions {
    /// The width the printer fits lines into, in display columns. Default
    /// 80.
    pub line_width: u16,
    /// Columns per indentation level. With `use_tabs`, the width a tab
    /// counts for fitting. Default 2.
    pub indent_width: u8,
    /// Indent with one tab per level instead of `indent_width` spaces.
    /// Default false.
    pub use_tabs: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            line_width: 80,
            indent_width: 2,
            use_tabs: false,
        }
    }
}

/// Receives rendered output in chunks of about 1 KiB; `Break` stops
/// rendering early, so a sink that compares the output with the source (check
/// mode) stops the printer soon after the first difference.
pub trait Sink {
    /// Takes the next chunk of output.
    fn write(&mut self, chunk: &str) -> ControlFlow<()>;
}

impl Sink for String {
    fn write(&mut self, chunk: &str) -> ControlFlow<()> {
        self.push_str(chunk);
        ControlFlow::Continue(())
    }
}

/// What a language provides to the shared formatter. Implemented by a unit
/// type in the language's formatter crate (`jolt_java_fmt::JavaFormat`).
///
/// The token tables take a token, not a kind, because the same kind can play
/// two roles (Java's `<` and `>` delimit type arguments or are operators).
pub trait FormatLanguage: Sized + 'static {
    /// The language's tree.
    type Syntax: Language;
    /// Formats a node that is not malformed. Called only by
    /// [`Formatter::node`].
    fn format_node<'a>(f: &mut Formatter<'a, Self>, node: SyntaxNode<'a>) -> Doc;
    /// Separators and terminators that no layout ever puts on a new line. An
    /// end-of-line line comment directly before one moves after it
    /// (placement rule S1), and an inline comment before one trails the
    /// previous element.
    fn is_separator(token: SyntaxToken<'_>) -> bool;
    /// If `token` opens a delimited region of its parent, the kind of its
    /// closer. Placement rules G1 and G2 and [`is_unclosed`] use it.
    fn closer_for(token: SyntaxToken<'_>) -> Option<RawSyntaxKind>;
    /// Tokens that close a delimited region: the closers of
    /// [`closer_for`](Self::closer_for). Placement rule G3 uses it.
    fn is_closer(token: SyntaxToken<'_>) -> bool;
    /// Whether text ending in `left` followed directly by text starting with
    /// `right` would lex differently (`- -x`, `a b`, `/ /*c*/`). The printer
    /// writes a space between such texts, so no rule has to.
    fn joins(left: char, right: char) -> bool;
    /// A literal that runs to the end of the file unterminated and would take
    /// in a final newline printed after it (Java: an unterminated text
    /// block), so the output ends without one.
    fn is_open_literal(token: SyntaxToken<'_>) -> bool;
    /// Tokens after which an inline comment is printed with no space, such
    /// as the openers of [`closer_for`](Self::closer_for).
    fn hugs_next(token: SyntaxToken<'_>) -> bool;
    /// Language exceptions to comment placement, tried before the generic
    /// rules; `None` falls through to them. A hook may only target elements
    /// inside the comment's enclosing node; a `Leading` layout is not
    /// `EndOfLine`, and a `Trailing` layout is `EndOfLine` or `Inline`
    /// (checked by debug assertions).
    fn place_comment<'a>(context: &CommentContext<'a, '_>) -> Option<Placement<'a>>;
    /// The opener of a doc line comment (`"///"` for Java), if the language
    /// has one. Placement groups consecutive doc line comments into runs that
    /// print as one (see [`DocCommentSource::Lines`]), and the printer keeps
    /// two runs apart with a blank line when a layout puts one right after
    /// the other, so they never merge.
    const DOC_LINE_COMMENT: Option<&'static str>;
    /// What [`doc_comment`](Self::doc_comment) needs to know about the whole
    /// file, found once per file by [`doc_file`](Self::doc_file), so that a
    /// file's many doc comments don't each rescan it.
    type DocFile: Copy;
    /// The [`DocFile`](Self::DocFile) of the file whose tree is `root`.
    fn doc_file(root: SyntaxNode<'_>) -> Self::DocFile;
    /// Lays out a doc comment (a doc block comment, or a run of doc line
    /// comments) that starts its output line; `None` prints it as written.
    /// Called only by the comment chokepoints, for a comment outside every
    /// formatter-ignore region whose target has no recovery. The layout
    /// starts at the comment's own column, after a line break. A doc line
    /// run's layout ends with [`DocBuilder::line_comment`], so that its line
    /// ends after it.
    fn doc_comment<'a>(
        docs: &mut DocBuilder<'a>,
        comment: &DocComment<'a, '_>,
        file: Self::DocFile,
    ) -> Option<Doc>;
    /// The item-list owner a formatter-ignore marker belongs to, given the
    /// deepest node containing the tokens around it: the owner when the
    /// marker sits between, before or after the items of the owner's list,
    /// `None` when it is inside an item.
    fn ignore_scope(enclosing: SyntaxNode<'_>) -> Option<SyntaxNode<'_>>;
}

/// Formats a tree: placement, ignore regions, doc construction, the debug
/// emission check, and printing. A byte order mark at the start of the source
/// is written first, and a trailing substitute character after the final
/// newline (which is left out after an open literal at the end of the file).
pub fn format<L: FormatLanguage, S: Sink + ?Sized>(
    root: SyntaxNode<'_>,
    options: &FormatOptions,
    sink: &mut S,
) -> ControlFlow<()> {
    let mut formatter = Formatter::<L>::new(root);
    let doc = formatter.node(root);
    if cfg!(debug_assertions) {
        formatter.assert_emitted();
    }
    let builder = formatter.into_builder();
    if let Some(bom) = special_trivia(root.first_token(), TriviaKind::ByteOrderMark) {
        sink.write(bom)?;
    }
    let open_end = root
        .last_token()
        .and_then(SyntaxToken::prev)
        .is_some_and(|last| {
            last.text_range().end().get() == root.source().len() && L::is_open_literal(last)
        });
    printer::print(
        &builder,
        doc,
        *options,
        printer::Language {
            joins: L::joins,
            doc_line: L::DOC_LINE_COMMENT,
        },
        !open_end,
        sink,
    )?;
    if let Some(substitute) = special_trivia(root.last_token(), TriviaKind::TrailingSubstitute) {
        sink.write(substitute)?;
    }
    ControlFlow::Continue(())
}

fn special_trivia(token: Option<SyntaxToken<'_>>, kind: TriviaKind) -> Option<&str> {
    let token = token?;
    token
        .leading()
        .chain(token.trailing())
        .find(|piece| piece.kind == kind)
        .map(|piece| piece.text)
}
