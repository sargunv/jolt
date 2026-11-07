//! The `left = right` layout.

use jolt_fmt::{Doc, cat, indent};
use jolt_syntax::SyntaxToken;

use crate::JavaFormatter;

/// `left space op` then `group(indent(line, right))`. With a trailing comment
/// on `op` (`x = // c`), the pending-suffix fit rule breaks the group, so the
/// value starts the next line one indent deeper.
pub(crate) fn assignment_like<'a>(
    f: &mut JavaFormatter<'a>,
    left: Doc,
    op: Option<SyntaxToken<'a>>,
    right: Doc,
) -> Doc {
    let op = f.token_opt(op);
    let value = indent![f; f.line(), right];
    let value = f.group(value);
    cat![f; left, f.space(), op, value]
}
