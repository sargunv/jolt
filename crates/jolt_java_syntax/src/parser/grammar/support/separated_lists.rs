// Drives the comma-separated list bodies shared by Java's delimited grammars.
use jolt_syntax::NodeAnchor;

use super::{JavaSyntaxKind, Parser};

impl Parser<'_> {
    /// Parses one comma-separated list body, reporting the element slot that a
    /// trailing separator declares but no source fills.
    ///
    /// A separated list alternates element and separator slots, so a list that
    /// ends with a separator still declares one more element. Without this the
    /// slot is left empty and unowned: the tree records a missing element that
    /// no diagnostic explains, and the parser accepts source Java rejects.
    ///
    /// The caller owns the list marker and completes it after this returns.
    /// Lists that carry extra per-element state, such as formal parameters
    /// tracking receiver and varargs position, drive their own loop instead.
    ///
    /// `parse_element` receives the physical slot index of the element it is
    /// about to parse, so lists that report per-element recovery can own their
    /// own slot.
    pub(in crate::parser::grammar) fn parse_comma_separated(
        &mut self,
        list_owner: NodeAnchor,
        expected: &'static str,
        at_close: impl Fn(&mut Self) -> bool,
        mut parse_element: impl FnMut(&mut Self, u16),
    ) {
        let mut next_item_slot = 0;
        let mut trailing_separator = false;
        while !self.at_eof() && !at_close(self) {
            parse_element(self, next_item_slot);
            if !self.eat(JavaSyntaxKind::Comma) {
                trailing_separator = false;
                break;
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
