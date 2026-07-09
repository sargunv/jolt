use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::CompilationUnit;

use crate::helpers::comments::format_comment;

pub(crate) fn format_comment_only_compilation_unit<'source>(
    unit: &CompilationUnit<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut comments = doc.list();
    if let Some(token) = unit.last_token() {
        for comment in token.leading_comments() {
            if !comments.is_empty() {
                let line = doc.hard_line();
                comments.push(line, doc);
            }
            let comment = format_comment(doc, &comment);
            comments.push(comment, doc);
        }
    }
    comments.finish(doc)
}
