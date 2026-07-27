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
use crate::formatter_ignore::FormatterIgnoreItemRange;
use crate::recovery::LayoutDoc;
use crate::{Doc, DocBuilder};

/// One staged list element, with the separator that follows it.
pub struct CommaListItem<'source, L: Language> {
    layout: LayoutDoc<'source>,
    comma: Option<SyntaxToken<'source, L>>,
    comma_starts_after_line: bool,
    physical_separator: Option<SyntaxToken<'source, L>>,
    ignore_range: Option<FormatterIgnoreItemRange>,
    starts_after_line: bool,
}

impl<'source, L: Language> CommaListItem<'source, L> {
    /// An element that occupies layout.
    #[must_use]
    pub const fn visible(doc: Doc<'source>) -> Self {
        Self {
            layout: LayoutDoc::Visible(doc),
            comma: None,
            comma_starts_after_line: false,
            physical_separator: None,
            ignore_range: None,
            starts_after_line: false,
        }
    }

    /// An element that already owns its separator.
    #[must_use]
    pub fn visible_with_comma(doc: Doc<'source>, comma: SyntaxToken<'source, L>) -> Self {
        Self {
            layout: LayoutDoc::Visible(doc),
            comma: Some(comma),
            comma_starts_after_line: false,
            physical_separator: None,
            ignore_range: None,
            starts_after_line: false,
        }
    }

    /// A recovery element that may claim source without occupying layout.
    #[must_use]
    pub const fn recovery(layout: LayoutDoc<'source>) -> Self {
        Self {
            layout,
            comma: None,
            comma_starts_after_line: false,
            physical_separator: None,
            ignore_range: None,
            starts_after_line: false,
        }
    }

    /// A physical separator staged separately until formatter-ignore regions
    /// have claimed their exact item and separator boundaries.
    #[must_use]
    pub fn physical_separator(separator: SyntaxToken<'source, L>) -> Self {
        Self {
            layout: LayoutDoc::ClaimOnly(Doc::nil()),
            comma: None,
            comma_starts_after_line: false,
            physical_separator: Some(separator),
            ignore_range: Some(FormatterIgnoreItemRange::between(&separator, &separator)),
            starts_after_line: false,
        }
    }

    /// Attaches the physical syntax range used to splice formatter-ignore
    /// regions through this staged item.
    #[must_use]
    pub const fn with_ignore_range(mut self, range: Option<FormatterIgnoreItemRange>) -> Self {
        self.ignore_range = range;
        self
    }

    #[must_use]
    pub const fn with_line_before(mut self) -> Self {
        self.starts_after_line = true;
        self
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

    #[must_use]
    pub const fn comma_starts_after_line(&self) -> bool {
        self.comma_starts_after_line
    }

    #[must_use]
    pub const fn staged_separator(&self) -> Option<SyntaxToken<'source, L>> {
        self.physical_separator
    }

    #[must_use]
    pub const fn ignore_range(&self) -> Option<FormatterIgnoreItemRange> {
        self.ignore_range
    }

    #[must_use]
    pub const fn starts_after_line(&self) -> bool {
        self.starts_after_line
    }
}

/// Attaches a separator to the last visible element that does not already own
/// one, or stages `orphan` when there is none.
///
/// A separator must never attach to a claim-only recovery element: that element
/// contributes no layout, so a separator held there would never be emitted and
/// its token would be lost. Callers supply `orphan` because they differ in how
/// a separator with no owning element places its trailing trivia. The explicit
/// `starts_after_line` state comes from an enclosing structural splice; ordinary
/// source trivia does not force the surrounding layout group.
pub fn attach_comma_separator<'source, L: Language>(
    items: &mut Vec<CommaListItem<'source, L>>,
    separator: SyntaxToken<'source, L>,
    starts_after_line: bool,
    orphan: impl FnOnce(SyntaxToken<'source, L>) -> CommaListItem<'source, L>,
) {
    if let Some(item) = items.iter_mut().rev().find(|item| item.is_visible())
        && item.comma.is_none()
    {
        item.comma = Some(separator);
        item.comma_starts_after_line = starts_after_line;
    } else {
        let mut item = orphan(separator);
        item.comma_starts_after_line = starts_after_line;
        items.push(item);
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
                if item.comma_starts_after_line {
                    let boundary = docs.hard_line_boundary();
                    docs.push(boundary);
                }
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
///
/// Both variants are line *boundaries*: they name the line state the gap must
/// reach, not the breaks to append. Whether the previous item already ended its
/// own line is then the renderer's business, so no caller has to predict it
/// from that item's last source token — a prediction an item defeats whenever
/// it emits layout after its last token, such as a synthesized closing
/// parenthesis.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BodyItemSeparator {
    /// The next item starts on the next line.
    Line,
    /// The next item starts after a blank line.
    EmptyLine,
}

impl BodyItemSeparator {
    /// Chooses the separator between two adjacent items.
    #[must_use]
    pub const fn between(source_had_blank_line: bool) -> Self {
        if source_had_blank_line {
            Self::EmptyLine
        } else {
            Self::Line
        }
    }

    #[must_use]
    pub fn doc<'source>(self, doc: &mut DocBuilder<'source>) -> Doc<'source> {
        match self {
            Self::Line => doc.hard_line_boundary(),
            Self::EmptyLine => doc.empty_line_boundary(),
        }
    }
}
