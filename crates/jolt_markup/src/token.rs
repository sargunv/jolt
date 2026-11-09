//! Tokens: what a tokenizer reads in a doc comment, for the layout.
//!
//! The tokenizers (`javadoc`, `markdown`) are the only code that reads a
//! comment's characters. Each turns a comment into a flat list of tokens
//! that borrow its text, or into `None` when the comment has a construct
//! outside the subset that Jolt lays out. The layouts (`html`, `markdown`)
//! read the tokens only.

/// How the source separates a token from the token before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Sep {
    /// Nothing: the token touches the one before it (`word<b>`).
    None,
    /// Spaces on the same line.
    Space,
    /// A line break.
    Line,
}

/// A block-level HTML element that the Javadoc layout gives a place of its
/// own. Names match without regard to case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Name {
    P,
    /// `<ul>`, `<ol>`.
    List,
    /// `<li>`.
    Item,
    /// `</pre>` outside a `<pre>` region (an opening `<pre>` starts a
    /// [`Kind::Pre`] region).
    Pre,
    /// `<h1>` to `<h6>`.
    Heading,
    Br,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A word: text, an entity, an HTML tag written on one line, a piece of
    /// an inline tag, a Markdown code span or link.
    Word,
    /// A block tag starts here; its name (`@param`) is the next word.
    BlockTag,
    /// A block-level HTML element's tag (Javadoc).
    Element { name: Name, closing: bool },
    /// The first line of a `<pre>` region, from `<pre` to the end of its
    /// line, or to the end of `</pre>` when the region is on one line
    /// (Javadoc). Its later lines are [`Kind::Verbatim`], and
    /// [`Kind::PreEnd`] ends it.
    Pre,
    /// The end of a `<pre>` region.
    PreEnd,
    /// A line printed as written, from its frame (the first `*` or `///`)
    /// on: a later line of a `<pre>` region (the last one up to `</pre>`), or
    /// a line of a Markdown fenced code block.
    Verbatim,
    /// A blank line. In Markdown, its text is the line from its `///` on.
    Blank,
    /// The source's line break before the next token stays (Javadoc: before
    /// a line that starts with a list marker, and around a ruler).
    Break,
}

/// One token of a doc comment.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Token<'a> {
    pub(crate) kind: Kind,
    pub(crate) text: &'a str,
    pub(crate) sep: Sep,
    /// A word that stays on the line of the word before it, after a space:
    /// a block tag's name and reference, an inline tag's reference, the
    /// words of `{@code …}`.
    pub(crate) glue: bool,
    /// A word that would start new structure at the start of a line
    /// ([`crate::tags::starts_structure`]), or (Markdown) one after a word
    /// that ends in `\`: a wrap never puts it at a line start.
    pub(crate) sticky: bool,
}

impl<'a> Token<'a> {
    pub(crate) const fn new(kind: Kind, text: &'a str, sep: Sep) -> Self {
        Self {
            kind,
            text,
            sep,
            glue: false,
            sticky: false,
        }
    }
}
