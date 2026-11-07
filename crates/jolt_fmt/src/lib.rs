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
    Comment, CommentContext, CommentKind, CommentLayout, Placement, is_unclosed, leading_target,
    trailing_target,
};
pub use doc::{Doc, DocBuilder, Reindent};
pub use formatter::{Formatter, ListItem, Separator};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct FormatOptions {
    pub line_width: u16,
    pub indent_width: u8,
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

/// Receives rendered output in chunks; `Break` stops rendering early.
pub trait Sink {
    fn write(&mut self, chunk: &str) -> ControlFlow<()>;
}

impl Sink for String {
    fn write(&mut self, chunk: &str) -> ControlFlow<()> {
        self.push_str(chunk);
        ControlFlow::Continue(())
    }
}

/// What a language provides to the shared formatter.
pub trait FormatLanguage: Sized + 'static {
    type Syntax: Language;
    /// Formats a node that is not malformed. Called only by
    /// [`Formatter::node`].
    fn format_node<'a>(f: &mut Formatter<'a, Self>, node: SyntaxNode<'a>) -> Doc;
    /// Separators and terminators that no layout puts on a new line.
    fn is_separator(token: SyntaxToken<'_>) -> bool;
    /// If `token` opens a delimited region of its parent, the kind of its
    /// closer.
    fn closer_for(token: SyntaxToken<'_>) -> Option<RawSyntaxKind>;
    fn is_closer(token: SyntaxToken<'_>) -> bool;
    /// Whether text ending in `left` followed directly by text starting with
    /// `right` would lex differently; the printer separates such texts with a
    /// space.
    fn joins(left: char, right: char) -> bool;
    /// A literal that runs to the end of the file unterminated and would take
    /// in a final newline printed after it (Java: an unterminated text
    /// block), so the output ends without one.
    fn is_open_literal(token: SyntaxToken<'_>) -> bool;
    /// Tokens after which an inline comment is printed with no space.
    fn hugs_next(token: SyntaxToken<'_>) -> bool;
    /// Language exceptions to comment placement, tried before the generic
    /// rules.
    fn place_comment<'a>(context: &CommentContext<'a, '_>) -> Option<Placement<'a>>;
    /// The item-list owner a formatter-ignore marker belongs to, given the
    /// deepest node containing the tokens around it: the owner when the
    /// marker sits between, before or after the items of the owner's list,
    /// `None` when it is inside an item.
    fn ignore_scope(enclosing: SyntaxNode<'_>) -> Option<SyntaxNode<'_>>;
    /// Expand an own-line `/** text */` into a multi-line star block.
    const EXPAND_DOC_COMMENTS: bool;
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
    printer::print(&builder, doc, *options, L::joins, !open_end, sink)?;
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
