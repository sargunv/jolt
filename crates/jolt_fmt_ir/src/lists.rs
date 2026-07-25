//! Shared comma-separated list layout.
//!
//! Both languages stage a list as items that each carry the separator
//! following them, then lay the staged items out. Claim-only recovery items
//! occupy no layout, so separators and breaks are placed between *visible*
//! items only.
//!
//! Delimiter handling stays in the language crates: they compose open/close
//! delimiters and their recovery differently.

use jolt_syntax::{Language, SyntaxToken};

use crate::comments::format_separator_with_comments;
use crate::recovery::LayoutDoc;
use crate::{Doc, DocBuilder};

/// One staged list element, with the separator that follows it.
pub struct CommaListItem<'source, L: Language> {
    layout: LayoutDoc<'source>,
    comma: Option<SyntaxToken<'source, L>>,
}

impl<'source, L: Language> CommaListItem<'source, L> {
    /// An element that occupies layout.
    #[must_use]
    pub const fn visible(doc: Doc<'source>) -> Self {
        Self {
            layout: LayoutDoc::Visible(doc),
            comma: None,
        }
    }

    /// An element that already owns its separator.
    #[must_use]
    pub const fn visible_with_comma(doc: Doc<'source>, comma: SyntaxToken<'source, L>) -> Self {
        Self {
            layout: LayoutDoc::Visible(doc),
            comma: Some(comma),
        }
    }

    /// A recovery element that may claim source without occupying layout.
    #[must_use]
    pub const fn recovery(layout: LayoutDoc<'source>) -> Self {
        Self {
            layout,
            comma: None,
        }
    }

    #[must_use]
    pub const fn is_visible(&self) -> bool {
        self.layout.is_visible()
    }

    #[must_use]
    pub const fn doc(&self) -> Doc<'source> {
        self.layout.doc()
    }

    #[must_use]
    pub const fn comma(&self) -> Option<SyntaxToken<'source, L>> {
        self.comma
    }
}

/// Attaches a separator to the last visible element that does not already own
/// one, or stages `orphan` when there is none.
///
/// A separator must never attach to a claim-only recovery element: that element
/// contributes no layout, so a separator held there would never be emitted and
/// its token would be lost. Callers supply `orphan` because they differ in how
/// a separator with no owning element places its trailing trivia.
pub fn attach_comma_separator<'source, L: Language>(
    items: &mut Vec<CommaListItem<'source, L>>,
    separator: SyntaxToken<'source, L>,
    orphan: impl FnOnce(SyntaxToken<'source, L>) -> CommaListItem<'source, L>,
) {
    if let Some(item) = items.iter_mut().rev().find(|item| item.is_visible())
        && item.comma.is_none()
    {
        item.comma = Some(separator);
    } else {
        items.push(orphan(separator));
    }
}

/// Lays out staged elements separated by commas.
pub fn comma_list<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    items: impl IntoIterator<Item = CommaListItem<'source, L>>,
) -> Doc<'source> {
    comma_list_parts(doc, items).0
}

/// Lays out staged elements, reporting whether the source ended with a trailing
/// separator.
///
/// A trailing separator emits no break after itself; the enclosing list decides
/// how to lay out its close delimiter.
pub fn comma_list_parts<'source, L: Language>(
    doc: &mut DocBuilder<'source>,
    items: impl IntoIterator<Item = CommaListItem<'source, L>>,
) -> (Doc<'source>, bool) {
    let items: Vec<_> = items.into_iter().collect();
    let visible_count = items.iter().filter(|item| item.is_visible()).count();
    let mut has_source_trailing_separator = false;
    let docs = doc.concat_list(|docs| {
        let mut visible_index = 0;
        for item in items {
            docs.push(item.doc());
            if !item.is_visible() {
                continue;
            }

            let is_last = visible_index + 1 == visible_count;
            if let Some(comma) = item.comma {
                has_source_trailing_separator |= is_last;
                let unforced_break = if is_last { Doc::nil() } else { docs.line() };
                let separator = format_separator_with_comments(docs, &comma, unforced_break);
                docs.push(separator);
            } else if !is_last {
                let line = docs.line();
                docs.push(line);
            }
            visible_index += 1;
        }
    });

    (docs, has_source_trailing_separator)
}

/// How two adjacent items in a body or file are separated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BodyItemSeparator {
    /// Nothing beyond the line the previous item already ended.
    None,
    /// A single line break.
    Line,
    /// A blank line.
    EmptyLine,
}

impl BodyItemSeparator {
    /// Chooses the separator between two adjacent items.
    ///
    /// `previous_ended_line` means the previous item already ended its own line,
    /// typically because a trailing line comment forced a break. Opening another
    /// break on top of that would leave a spurious blank line, so the separator
    /// steps down: a blank line becomes one break, and one break becomes
    /// nothing.
    #[must_use]
    pub const fn between(source_had_blank_line: bool, previous_ended_line: bool) -> Self {
        match (source_had_blank_line, previous_ended_line) {
            (false, true) => Self::None,
            (true, true) | (false, false) => Self::Line,
            (true, false) => Self::EmptyLine,
        }
    }

    /// Chooses the separator for a body that always spaces its items apart.
    #[must_use]
    pub const fn spaced(previous_ended_line: bool) -> Self {
        Self::between(true, previous_ended_line)
    }

    #[must_use]
    pub fn doc<'source>(self, doc: &mut DocBuilder<'source>) -> Doc<'source> {
        match self {
            // A boundary rather than nothing at all. Whether the previous item
            // ended its line is a prediction made from that item's last source
            // token, and an item may emit more layout after its last token — a
            // synthesized closing parenthesis, say. Emitting nothing on a wrong
            // prediction runs the next item straight onto the previous one.
            Self::None => doc.hard_line_boundary(),
            Self::Line => doc.hard_line(),
            Self::EmptyLine => doc.empty_line(),
        }
    }
}
