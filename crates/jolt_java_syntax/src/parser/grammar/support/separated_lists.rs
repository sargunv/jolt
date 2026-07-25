// Drives the comma-separated list bodies shared by Java's delimited grammars.
use jolt_syntax::NodeAnchor;

use super::{JavaSyntaxKind, Parser};

/// What parsing one element did to the separator that follows it.
///
/// Most elements leave the separator alone, so a callback returning `()`
/// converts to [`SeparatedElement::Parsed`] and reads as an ordinary loop body.
pub(in crate::parser::grammar) enum SeparatedElement {
    /// The driver should consume the separator.
    Parsed,
    /// The element parse consumed the separator itself.
    ConsumedSeparator,
    /// The list ends here, with no separator consumed.
    Stop,
}

impl From<()> for SeparatedElement {
    fn from((): ()) -> Self {
        Self::Parsed
    }
}

impl Parser<'_> {
    /// Parses one comma-separated list body, reporting the element slot that a
    /// trailing separator declares but no source fills.
    ///
    /// A separated list alternates element and separator slots, so a list that
    /// ends with a separator still declares one more element. Without this the
    /// slot is left empty and unowned: the tree records a missing element that
    /// no diagnostic explains, and the parser accepts source Java rejects.
    ///
    /// Every delimited list drives its body through here. Elements that must
    /// consume their own separator, such as a varargs parameter deciding
    /// whether anything may follow it, say so with [`SeparatedElement`] rather
    /// than running their own loop, so no list can forget the trailing slot.
    ///
    /// The caller owns the list marker and completes it after this returns.
    ///
    /// `parse_element` receives the physical slot index of the element it is
    /// about to parse, so lists that report per-element recovery can own their
    /// own slot.
    pub(in crate::parser::grammar) fn parse_comma_separated<E>(
        &mut self,
        list_owner: NodeAnchor,
        expected: &'static str,
        at_close: impl Fn(&mut Self) -> bool,
        mut parse_element: impl FnMut(&mut Self, u16) -> E,
    ) where
        E: Into<SeparatedElement>,
    {
        let mut next_item_slot = 0;
        let mut trailing_separator = false;
        while !self.at_eof() && !at_close(self) {
            match parse_element(self, next_item_slot).into() {
                SeparatedElement::Parsed => {
                    if !self.eat(JavaSyntaxKind::Comma) {
                        trailing_separator = false;
                        break;
                    }
                }
                SeparatedElement::ConsumedSeparator => {}
                SeparatedElement::Stop => {
                    trailing_separator = false;
                    break;
                }
            }
            next_item_slot += 2;
            trailing_separator = true;
        }

        if trailing_separator {
            let diagnostic = self.pending_expected(expected);
            self.missing_required_slot(list_owner, next_item_slot, [diagnostic]);
        }
    }
}
