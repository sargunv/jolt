use jolt_diagnostics::TextRange;
use jolt_java_syntax::{JavaLexer, JavaSyntaxKind, Trivia, TriviaKind};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct JavaFormatContext<'source> {
    source: &'source str,
    comments: Vec<JavaCommentTrivia>,
    next_unhandled_comment: usize,
}

impl<'source> JavaFormatContext<'source> {
    pub(crate) fn new(source: &'source str) -> Self {
        let mut lexer = JavaLexer::new(source);
        let mut comments = Vec::new();

        loop {
            let token = lexer.next_token();
            comments.extend(
                token
                    .leading
                    .into_iter()
                    .filter(is_formatter_accounted_trivia)
                    .map(|trivia| JavaCommentTrivia {
                        trivia,
                        attachment: JavaTriviaAttachment::Leading,
                    }),
            );
            comments.extend(
                token
                    .trailing
                    .into_iter()
                    .filter(is_formatter_accounted_trivia)
                    .map(|trivia| JavaCommentTrivia {
                        trivia,
                        attachment: JavaTriviaAttachment::Trailing,
                    }),
            );

            if token.kind == JavaSyntaxKind::Eof {
                break;
            }
        }

        comments.sort_by_key(|comment| comment.trivia.range.start());

        Self {
            source,
            comments,
            next_unhandled_comment: 0,
        }
    }

    pub(crate) fn has_unhandled_comment_trivia(&self) -> bool {
        self.unhandled_comment_trivia().is_some()
    }

    pub(crate) fn unhandled_comment_trivia(&self) -> Option<&JavaCommentTrivia> {
        self.comments.get(self.next_unhandled_comment)
    }

    pub(crate) fn next_unhandled_comment_trivia(&mut self) -> Option<&JavaCommentTrivia> {
        let comment = self.comments.get(self.next_unhandled_comment)?;
        self.next_unhandled_comment += 1;
        Some(comment)
    }

    pub(crate) fn take_leading_comments(
        &mut self,
        code_range: TextRange,
    ) -> Result<Vec<JavaCommentTrivia>, JavaCommentPlacementError> {
        let mut comments = Vec::new();

        while let Some(comment) = self.unhandled_comment_trivia() {
            if comment.trivia.range.start() >= code_range.start() {
                break;
            }
            if comment.trivia.kind == TriviaKind::Ignored {
                break;
            }
            if comment.trivia.range.end() > code_range.start() {
                break;
            }
            if !self.is_own_line_comment(comment) {
                return Err(JavaCommentPlacementError {
                    message: "Java formatter only supports own-line leading comments here"
                        .to_owned(),
                    range: comment.trivia.range,
                });
            }

            let comment = self
                .next_unhandled_comment_trivia()
                .expect("unhandled comment checked above")
                .clone();
            comments.push(comment);
        }

        Ok(comments)
    }

    pub(crate) fn take_dangling_comments(
        &mut self,
        container_range: TextRange,
    ) -> Result<Vec<JavaCommentTrivia>, JavaCommentPlacementError> {
        let mut comments = Vec::new();

        while let Some(comment) = self.unhandled_comment_trivia() {
            if comment.trivia.kind == TriviaKind::Ignored {
                break;
            }
            if comment.trivia.range.start() <= container_range.start() {
                break;
            }
            if comment.trivia.range.start() >= container_range.end() {
                break;
            }
            if comment.trivia.range.end() >= container_range.end() {
                break;
            }
            if !self.is_own_line_comment(comment) {
                return Err(JavaCommentPlacementError {
                    message: "Java formatter only supports own-line dangling comments here"
                        .to_owned(),
                    range: comment.trivia.range,
                });
            }

            let comment = self
                .next_unhandled_comment_trivia()
                .expect("unhandled comment checked above")
                .clone();
            comments.push(comment);
        }

        Ok(comments)
    }

    pub(crate) fn reject_unhandled_comments_before_start(
        &self,
        boundary: TextRange,
        message: &'static str,
    ) -> Result<(), JavaCommentPlacementError> {
        let Some(comment) = self.unhandled_comment_trivia() else {
            return Ok(());
        };
        if comment.trivia.kind != TriviaKind::Ignored
            && comment.trivia.range.start() < boundary.start()
        {
            return Err(JavaCommentPlacementError {
                message: message.to_owned(),
                range: comment.trivia.range,
            });
        }

        Ok(())
    }

    pub(crate) fn reject_unhandled_comments_before_end(
        &self,
        boundary: TextRange,
        message: &'static str,
    ) -> Result<(), JavaCommentPlacementError> {
        let Some(comment) = self.unhandled_comment_trivia() else {
            return Ok(());
        };
        if comment.trivia.kind != TriviaKind::Ignored
            && comment.trivia.range.start() < boundary.end()
        {
            return Err(JavaCommentPlacementError {
                message: message.to_owned(),
                range: comment.trivia.range,
            });
        }

        Ok(())
    }

    pub(crate) fn take_trailing_line_comment(
        &mut self,
        code_range: TextRange,
    ) -> Result<Option<JavaCommentTrivia>, JavaCommentPlacementError> {
        let Some(comment) = self.unhandled_comment_trivia() else {
            return Ok(None);
        };
        if comment.trivia.kind == TriviaKind::Ignored {
            return Ok(None);
        }
        if comment.trivia.range.start() < code_range.end() {
            return Ok(None);
        }
        if !self.is_same_line_span(code_range.end().get(), comment.trivia.range.start().get()) {
            return Ok(None);
        }
        if !self.only_whitespace(code_range.end().get(), comment.trivia.range.start().get()) {
            return Ok(None);
        }
        if comment.trivia.kind != TriviaKind::LineComment {
            return Err(JavaCommentPlacementError {
                message: "Java formatter only supports trailing line comments here".to_owned(),
                range: comment.trivia.range,
            });
        }

        Ok(Some(
            self.next_unhandled_comment_trivia()
                .expect("unhandled comment checked above")
                .clone(),
        ))
    }

    pub(crate) fn raw_text(&self, comment: &JavaCommentTrivia) -> &'source str {
        &self.source[comment.trivia.range.start().get()..comment.trivia.range.end().get()]
    }

    fn is_own_line_comment(&self, comment: &JavaCommentTrivia) -> bool {
        let start = comment.trivia.range.start().get();
        let end = comment.trivia.range.end().get();
        let line_start = self.line_start_before(start);
        let line_end = self.line_end_after(end);

        self.only_whitespace(line_start, start) && self.only_whitespace(end, line_end)
    }

    fn line_start_before(&self, offset: usize) -> usize {
        self.source[..offset]
            .char_indices()
            .filter(|(_, ch)| is_line_terminator(*ch))
            .map(|(index, ch)| index + ch.len_utf8())
            .next_back()
            .unwrap_or(0)
    }

    fn line_end_after(&self, offset: usize) -> usize {
        self.source[offset..]
            .char_indices()
            .find(|(_, ch)| is_line_terminator(*ch))
            .map_or(self.source.len(), |(index, _)| offset + index)
    }

    fn is_same_line_span(&self, start: usize, end: usize) -> bool {
        !self.source[start..end].contains(is_line_terminator)
    }

    fn only_whitespace(&self, start: usize, end: usize) -> bool {
        self.source[start..end].chars().all(char::is_whitespace)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct JavaCommentTrivia {
    pub(crate) trivia: Trivia,
    pub(crate) attachment: JavaTriviaAttachment,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum JavaTriviaAttachment {
    Leading,
    Trailing,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct JavaCommentPlacementError {
    pub(crate) message: String,
    pub(crate) range: TextRange,
}

const fn is_formatter_accounted_trivia(trivia: &Trivia) -> bool {
    matches!(
        trivia.kind,
        TriviaKind::LineComment
            | TriviaKind::BlockComment
            | TriviaKind::JavadocComment
            | TriviaKind::Ignored
    )
}

const fn is_line_terminator(ch: char) -> bool {
    matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source_text<'source>(source: &'source str, trivia: &Trivia) -> &'source str {
        &source[trivia.range.start().get()..trivia.range.end().get()]
    }

    #[test]
    fn context_records_leading_javadoc_comment() {
        let source = "/** docs */ class A {}";
        let mut context = JavaFormatContext::new(source);

        assert!(context.has_unhandled_comment_trivia());
        let comment = context
            .next_unhandled_comment_trivia()
            .expect("leading Javadoc comment");
        assert_eq!(comment.trivia.kind, TriviaKind::JavadocComment);
        assert_eq!(source_text(source, &comment.trivia), "/** docs */");
        assert_eq!(comment.attachment, JavaTriviaAttachment::Leading);
        assert!(!context.has_unhandled_comment_trivia());
    }

    #[test]
    fn context_records_body_block_comment() {
        let source = "class A { /* body */ }";
        let mut context = JavaFormatContext::new(source);

        let comment = context
            .next_unhandled_comment_trivia()
            .expect("body block comment");
        assert_eq!(comment.trivia.kind, TriviaKind::BlockComment);
        assert_eq!(source_text(source, &comment.trivia), "/* body */");
        assert_eq!(comment.attachment, JavaTriviaAttachment::Trailing);
        assert!(!context.has_unhandled_comment_trivia());
    }

    #[test]
    fn context_records_trailing_line_comment() {
        let source = "class A {} // trailing\n";
        let mut context = JavaFormatContext::new(source);

        let comment = context
            .next_unhandled_comment_trivia()
            .expect("trailing line comment");
        assert_eq!(comment.trivia.kind, TriviaKind::LineComment);
        assert_eq!(source_text(source, &comment.trivia), "// trailing");
        assert_eq!(comment.attachment, JavaTriviaAttachment::Trailing);
        assert!(!context.has_unhandled_comment_trivia());
    }

    #[test]
    fn context_records_final_sub_ignored_trivia() {
        let source = "class A {}\u{001A}";
        let mut context = JavaFormatContext::new(source);

        let ignored = context
            .next_unhandled_comment_trivia()
            .expect("final SUB ignored trivia");
        assert_eq!(ignored.trivia.kind, TriviaKind::Ignored);
        assert_eq!(source_text(source, &ignored.trivia), "\u{001A}");
        assert_eq!(ignored.attachment, JavaTriviaAttachment::Leading);
        assert!(!context.has_unhandled_comment_trivia());
    }

    #[test]
    fn whitespace_and_newlines_do_not_create_comment_records() {
        let context = JavaFormatContext::new("\n  class A {}\n");

        assert!(!context.has_unhandled_comment_trivia());
    }

    #[test]
    fn leading_comments_are_consumed_before_code_range() {
        let source = "// file\n/** docs */\nclass A {}";
        let mut context = JavaFormatContext::new(source);
        let comments = context
            .take_leading_comments(TextRange::new(20usize.into(), 30usize.into()))
            .expect("leading comments");

        assert_eq!(comments.len(), 2);
        assert_eq!(source_text(source, &comments[0].trivia), "// file");
        assert_eq!(source_text(source, &comments[1].trivia), "/** docs */");
        assert!(!context.has_unhandled_comment_trivia());
    }

    #[test]
    fn same_line_trailing_line_comment_is_consumed_after_code_range() {
        let source = "class A { int value; // field\n}";
        let mut context = JavaFormatContext::new(source);
        let comment = context
            .take_trailing_line_comment(TextRange::new(10usize.into(), 20usize.into()))
            .expect("trailing comment")
            .expect("line comment");

        assert_eq!(source_text(source, &comment.trivia), "// field");
        assert!(!context.has_unhandled_comment_trivia());
    }

    #[test]
    fn inline_block_comment_is_not_consumed_as_leading_or_trailing() {
        let source = "class A { int /* inline */ value; }";
        let mut context = JavaFormatContext::new(source);

        assert!(
            context
                .take_leading_comments(TextRange::new(10usize.into(), 33usize.into()))
                .expect("no leading comments")
                .is_empty()
        );
        assert!(
            context
                .take_trailing_line_comment(TextRange::new(10usize.into(), 33usize.into()))
                .expect("no trailing line comment")
                .is_none()
        );
        assert!(context.has_unhandled_comment_trivia());
    }
}
