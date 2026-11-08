//! The comma-separated list bodies shared by Java's delimited grammars.

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
    /// Parses one comma-separated list body, reporting `expected` when a
    /// trailing separator declares an element no source fills.
    ///
    /// Elements that must consume their own separator, such as a varargs
    /// parameter deciding whether anything may follow it, say so with
    /// [`SeparatedElement`] rather than running their own loop.
    ///
    /// The caller owns the list marker and completes it after this returns.
    pub(in crate::parser::grammar) fn parse_comma_separated<E>(
        &mut self,
        expected: &'static str,
        at_close: impl Fn(&Self) -> bool,
        mut parse_element: impl FnMut(&mut Self) -> E,
    ) where
        E: Into<SeparatedElement>,
    {
        let mut trailing_separator = false;
        let mut guard = self.loop_guard();
        while !at_close(self) && self.check(&mut guard) {
            match parse_element(self).into() {
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
            trailing_separator = true;
        }

        if trailing_separator {
            self.error_expected(expected);
        }
    }
}
