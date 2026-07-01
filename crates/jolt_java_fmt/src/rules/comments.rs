use jolt_fmt_ir::Doc;
use jolt_java_syntax::{CompilationUnit, JavaLexer, JavaSyntaxKind, TriviaKind};

use crate::helpers::blocks::join_hard_lines;
use crate::helpers::comments::format_raw_comment;

pub(crate) fn format_comment_only_compilation_unit(unit: &CompilationUnit) -> Doc {
    let source = unit.source_text();
    let mut lexer = JavaLexer::new(&source);
    let token = lexer.next_token();
    if token.kind != JavaSyntaxKind::Eof {
        return jolt_fmt_ir::nil();
    }

    join_hard_lines(
        token
            .leading
            .into_iter()
            .filter(|trivia| {
                matches!(
                    trivia.kind,
                    TriviaKind::LineComment | TriviaKind::BlockComment | TriviaKind::JavadocComment
                )
            })
            .map(|trivia| {
                let range = trivia.range;
                let text = &source[range.start().get()..range.end().get()];
                format_raw_comment(trivia.kind, text)
            })
            .collect(),
    )
}
