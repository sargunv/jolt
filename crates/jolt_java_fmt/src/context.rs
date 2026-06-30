use jolt_diagnostics::TextRange;
use jolt_java_syntax::{JavaLexer, JavaSyntaxKind, Trivia, TriviaKind};

use crate::options::JavaFormatProfile;
use crate::policy::JavaFormatPolicy;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct JavaFormatContext<'source> {
    source: &'source str,
    policy: JavaFormatPolicy,
    comments: Vec<JavaCommentRecord>,
}

impl<'source> JavaFormatContext<'source> {
    #[cfg(test)]
    pub(crate) fn new(source: &'source str) -> Self {
        Self::with_profile(source, JavaFormatProfile::Google)
    }

    pub(crate) fn with_profile(source: &'source str, profile: JavaFormatProfile) -> Self {
        let mut lexer = JavaLexer::new(source);
        let mut comments = Vec::new();

        loop {
            let token = lexer.next_token();
            comments.extend(
                token
                    .leading
                    .into_iter()
                    .filter(is_formatter_accounted_trivia)
                    .map(|trivia| JavaCommentRecord {
                        trivia: JavaCommentTrivia {
                            trivia,
                            attachment: JavaTriviaAttachment::Leading,
                        },
                        claimed: false,
                    }),
            );
            comments.extend(
                token
                    .trailing
                    .into_iter()
                    .filter(is_formatter_accounted_trivia)
                    .map(|trivia| JavaCommentRecord {
                        trivia: JavaCommentTrivia {
                            trivia,
                            attachment: JavaTriviaAttachment::Trailing,
                        },
                        claimed: false,
                    }),
            );

            if token.kind == JavaSyntaxKind::Eof {
                break;
            }
        }

        comments.sort_by_key(|comment| comment.trivia.trivia.range.start());

        Self {
            source,
            policy: JavaFormatPolicy::new(profile),
            comments,
        }
    }

    pub(crate) fn policy(&self) -> JavaFormatPolicy {
        self.policy
    }

    pub(crate) fn has_unhandled_comment_trivia(&self) -> bool {
        self.unhandled_comment_trivia().is_some()
    }

    pub(crate) fn unhandled_comment_trivia(&self) -> Option<&JavaCommentTrivia> {
        self.comments
            .iter()
            .find(|comment| !comment.claimed)
            .map(|comment| &comment.trivia)
    }

    pub(crate) fn unhandled_comment_trivia_before_start(
        &self,
        boundary: TextRange,
    ) -> Option<&JavaCommentTrivia> {
        self.unhandled_comment_trivia_before_offset(boundary.start().get())
    }

    pub(crate) fn unhandled_comment_trivia_before_end(
        &self,
        boundary: TextRange,
    ) -> Option<&JavaCommentTrivia> {
        self.unhandled_comment_trivia_before_offset(boundary.end().get())
    }

    pub(crate) fn unhandled_comment_trivia_in_range(
        &self,
        boundary: TextRange,
    ) -> Option<&JavaCommentTrivia> {
        self.comments
            .iter()
            .find(|comment| {
                let start = comment.trivia.trivia.range.start();
                !comment.claimed && start >= boundary.start() && start < boundary.end()
            })
            .map(|comment| &comment.trivia)
    }

    pub(crate) fn next_unhandled_comment_trivia(&mut self) -> Option<&JavaCommentTrivia> {
        let comment = self.comments.iter_mut().find(|comment| !comment.claimed)?;
        comment.claimed = true;
        Some(&comment.trivia)
    }

    pub(crate) fn take_leading_comments(
        &mut self,
        code_range: TextRange,
    ) -> Vec<JavaCommentTrivia> {
        self.take_comments_in_bucket(code_range, JavaCommentBucket::Leading)
    }

    pub(crate) fn take_leading_comments_in_range(
        &mut self,
        owner_range: TextRange,
        code_range: TextRange,
    ) -> Vec<JavaCommentTrivia> {
        let indices = self.comment_indices_in_bucket(code_range, JavaCommentBucket::Leading);
        let indices = indices
            .into_iter()
            .filter(|index| {
                let range = self.comments[*index].trivia.trivia.range;
                range.start() >= owner_range.start() && range.end() <= owner_range.end()
            })
            .collect();
        self.claim_comments(indices)
    }

    pub(crate) fn take_dangling_comments(
        &mut self,
        container_range: TextRange,
    ) -> Vec<JavaCommentTrivia> {
        self.take_comments_in_bucket(container_range, JavaCommentBucket::Dangling)
    }

    pub(crate) fn take_trailing_line_comment(
        &mut self,
        code_range: TextRange,
    ) -> Option<JavaCommentTrivia> {
        self.take_first_comment_in_bucket(code_range, JavaCommentBucket::TrailingLine)
    }

    pub(crate) fn take_inline_leading_block_comments(
        &mut self,
        code_range: TextRange,
    ) -> Vec<JavaCommentTrivia> {
        self.take_comments_in_bucket(code_range, JavaCommentBucket::InlineLeadingBlock)
    }

    pub(crate) fn take_inline_trailing_block_comments(
        &mut self,
        code_range: TextRange,
    ) -> Vec<JavaCommentTrivia> {
        self.take_comments_in_bucket(code_range, JavaCommentBucket::InlineTrailingBlock)
    }

    pub(crate) fn take_list_item_trailing_line_comment(
        &mut self,
        item_range: TextRange,
        boundary: TextRange,
    ) -> Option<JavaCommentTrivia> {
        let index = self.comments.iter().position(|comment| {
            !comment.claimed
                && self.is_list_item_trailing_line_comment(&comment.trivia, item_range, boundary)
        })?;

        self.comments[index].claimed = true;
        Some(self.comments[index].trivia.clone())
    }

    pub(crate) fn take_list_item_trailing_block_comments(
        &mut self,
        item_range: TextRange,
        boundary: TextRange,
    ) -> Vec<JavaCommentTrivia> {
        let mut saw_separator = false;
        let indices = self
            .comments
            .iter()
            .enumerate()
            .filter(|(_, comment)| !comment.claimed)
            .filter(|(_, comment)| {
                let matches = self.is_list_item_trailing_block_comment(
                    &comment.trivia,
                    item_range,
                    boundary,
                    saw_separator,
                );
                if matches {
                    saw_separator = true;
                }
                matches
            })
            .map(|(index, _)| index)
            .collect();

        self.claim_comments(indices)
    }

    pub(crate) fn comment_bucket_for_range(
        &self,
        comment: &JavaCommentTrivia,
        code_range: TextRange,
    ) -> JavaCommentBucket {
        if self.is_inline_leading_block_comment(comment, code_range) {
            return JavaCommentBucket::InlineLeadingBlock;
        }
        if self.is_inline_trailing_block_comment(comment, code_range) {
            return JavaCommentBucket::InlineTrailingBlock;
        }
        if self.is_trailing_line_comment(comment, code_range) {
            return JavaCommentBucket::TrailingLine;
        }
        if self.is_leading_comment(comment, code_range) {
            return JavaCommentBucket::Leading;
        }
        if self.is_dangling_comment(comment, code_range) {
            return JavaCommentBucket::Dangling;
        }

        JavaCommentBucket::Remaining
    }

    pub(crate) fn raw_text(&self, comment: &JavaCommentTrivia) -> &'source str {
        &self.source[comment.trivia.range.start().get()..comment.trivia.range.end().get()]
    }

    pub(crate) fn has_blank_line_between(&self, left: TextRange, right: TextRange) -> bool {
        let start = left.end().get();
        let end = right.start().get();
        if start >= end {
            return false;
        }

        let between = &self.source[start..end];
        if !between.chars().all(char::is_whitespace) {
            return false;
        }

        let mut line_terminators = 0;
        let mut chars = between.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '\r' => {
                    if chars.peek().is_some_and(|next| *next == '\n') {
                        chars.next();
                    }
                    line_terminators += 1;
                }
                '\n' | '\u{2028}' | '\u{2029}' => line_terminators += 1,
                _ => {}
            }
        }

        line_terminators >= 2
    }

    fn is_same_line_span(&self, start: usize, end: usize) -> bool {
        !self.source[start..end].contains(is_line_terminator)
    }

    fn only_whitespace(&self, start: usize, end: usize) -> bool {
        self.source[start..end].chars().all(char::is_whitespace)
    }

    fn unhandled_comment_trivia_before_offset(&self, offset: usize) -> Option<&JavaCommentTrivia> {
        self.comments
            .iter()
            .find(|comment| !comment.claimed && comment.trivia.trivia.range.start().get() < offset)
            .map(|comment| &comment.trivia)
    }

    fn take_comments_in_bucket(
        &mut self,
        code_range: TextRange,
        bucket: JavaCommentBucket,
    ) -> Vec<JavaCommentTrivia> {
        let indices = self.comment_indices_in_bucket(code_range, bucket);
        self.claim_comments(indices)
    }

    fn take_first_comment_in_bucket(
        &mut self,
        code_range: TextRange,
        bucket: JavaCommentBucket,
    ) -> Option<JavaCommentTrivia> {
        let index = self.comments.iter().position(|comment| !comment.claimed)?;
        if self.comment_bucket_for_range(&self.comments[index].trivia, code_range) != bucket {
            return None;
        }

        self.comments[index].claimed = true;
        Some(self.comments[index].trivia.clone())
    }

    fn comment_indices_in_bucket(
        &self,
        code_range: TextRange,
        bucket: JavaCommentBucket,
    ) -> Vec<usize> {
        self.comments
            .iter()
            .enumerate()
            .filter(|(_, comment)| !comment.claimed)
            .filter(|(_, comment)| {
                self.comment_bucket_for_range(&comment.trivia, code_range) == bucket
            })
            .map(|(index, _)| index)
            .collect()
    }

    fn claim_comments(&mut self, indices: Vec<usize>) -> Vec<JavaCommentTrivia> {
        indices
            .into_iter()
            .map(|index| {
                self.comments[index].claimed = true;
                self.comments[index].trivia.clone()
            })
            .collect()
    }

    fn is_leading_comment(&self, comment: &JavaCommentTrivia, code_range: TextRange) -> bool {
        if !self.is_comment_trivia(comment) {
            return false;
        }
        if comment.trivia.range.end() > code_range.start() {
            return false;
        }
        if self.is_same_line_span(comment.trivia.range.end().get(), code_range.start().get()) {
            return false;
        }

        self.is_own_line_comment(comment)
    }

    fn is_trailing_line_comment(&self, comment: &JavaCommentTrivia, code_range: TextRange) -> bool {
        comment.trivia.kind == TriviaKind::LineComment
            && comment.trivia.range.start() >= code_range.end()
            && self.is_same_line_span(code_range.end().get(), comment.trivia.range.start().get())
            && self.only_whitespace(code_range.end().get(), comment.trivia.range.start().get())
    }

    fn is_list_item_trailing_line_comment(
        &self,
        comment: &JavaCommentTrivia,
        item_range: TextRange,
        boundary: TextRange,
    ) -> bool {
        if comment.trivia.kind != TriviaKind::LineComment {
            return false;
        }
        if comment.trivia.range.start() < boundary.start()
            || comment.trivia.range.end() > boundary.end()
        {
            return false;
        }
        if !self.is_same_line_span(item_range.end().get(), comment.trivia.range.start().get()) {
            return false;
        }

        let between = &self.source[item_range.end().get()..comment.trivia.range.start().get()];
        between.chars().all(|ch| ch == ',' || ch.is_whitespace())
            && (between.contains(',') || between.chars().all(char::is_whitespace))
    }

    fn is_list_item_trailing_block_comment(
        &self,
        comment: &JavaCommentTrivia,
        item_range: TextRange,
        boundary: TextRange,
        previous_comment_after_separator: bool,
    ) -> bool {
        if !self.is_inline_block_comment(comment) {
            return false;
        }
        if comment.trivia.range.start() < boundary.start()
            || comment.trivia.range.end() > boundary.end()
        {
            return false;
        }
        if !self.is_same_line_span(item_range.end().get(), comment.trivia.range.start().get()) {
            return false;
        }
        if self.is_same_line_span(comment.trivia.range.end().get(), boundary.end().get())
            && self.only_whitespace(comment.trivia.range.end().get(), boundary.end().get())
        {
            return false;
        }
        if previous_comment_after_separator {
            return true;
        }

        let between = &self.source[item_range.end().get()..comment.trivia.range.start().get()];
        between.contains(',') && between.chars().all(|ch| ch == ',' || ch.is_whitespace())
    }

    fn is_dangling_comment(&self, comment: &JavaCommentTrivia, container_range: TextRange) -> bool {
        if !self.is_comment_trivia(comment) {
            return false;
        }
        comment.trivia.range.start() > container_range.start()
            && comment.trivia.range.end() < container_range.end()
            && (self.is_own_line_comment(comment)
                || self.is_delimiter_bounded_dangling_comment(comment, container_range))
    }

    fn is_inline_leading_block_comment(
        &self,
        comment: &JavaCommentTrivia,
        code_range: TextRange,
    ) -> bool {
        self.is_inline_block_comment(comment)
            && comment.trivia.range.end() <= code_range.start()
            && self.is_same_line_span(comment.trivia.range.end().get(), code_range.start().get())
            && self.only_whitespace(comment.trivia.range.end().get(), code_range.start().get())
    }

    fn is_inline_trailing_block_comment(
        &self,
        comment: &JavaCommentTrivia,
        code_range: TextRange,
    ) -> bool {
        self.is_inline_block_comment(comment)
            && comment.trivia.range.start() >= code_range.end()
            && self.is_same_line_span(code_range.end().get(), comment.trivia.range.start().get())
            && self.only_whitespace(code_range.end().get(), comment.trivia.range.start().get())
    }

    fn is_inline_block_comment(&self, comment: &JavaCommentTrivia) -> bool {
        matches!(
            comment.trivia.kind,
            TriviaKind::BlockComment | TriviaKind::JavadocComment
        ) && !self
            .raw_text(comment)
            .contains(['\n', '\r', '\u{2028}', '\u{2029}'])
    }

    fn is_comment_trivia(&self, comment: &JavaCommentTrivia) -> bool {
        matches!(
            comment.trivia.kind,
            TriviaKind::LineComment | TriviaKind::BlockComment | TriviaKind::JavadocComment
        )
    }

    fn is_delimiter_bounded_dangling_comment(
        &self,
        comment: &JavaCommentTrivia,
        container_range: TextRange,
    ) -> bool {
        let before =
            &self.source[container_range.start().get()..comment.trivia.range.start().get()];
        let after = &self.source[comment.trivia.range.end().get()..container_range.end().get()];

        let before = before.trim();
        let after = after.trim();

        matches!(before, "{" | "(" | "[") && matches!(after, "}" | ")" | "]")
    }

    fn is_own_line_comment(&self, comment: &JavaCommentTrivia) -> bool {
        let line_start = self.line_start(comment.trivia.range.start().get());
        let line_end = self.line_end(comment.trivia.range.end().get());

        self.only_whitespace(line_start, comment.trivia.range.start().get())
            && self.only_whitespace(comment.trivia.range.end().get(), line_end)
    }

    fn line_start(&self, offset: usize) -> usize {
        self.source[..offset]
            .rfind(is_line_terminator)
            .map_or(0, |index| {
                index
                    + self.source[index..]
                        .chars()
                        .next()
                        .map_or(1, char::len_utf8)
            })
    }

    fn line_end(&self, offset: usize) -> usize {
        self.source[offset..]
            .find(is_line_terminator)
            .map_or(self.source.len(), |index| offset + index)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct JavaCommentRecord {
    trivia: JavaCommentTrivia,
    claimed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct JavaCommentTrivia {
    pub(crate) trivia: Trivia,
    pub(crate) attachment: JavaTriviaAttachment,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum JavaCommentBucket {
    Leading,
    TrailingLine,
    Dangling,
    InlineLeadingBlock,
    InlineTrailingBlock,
    Remaining,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum JavaTriviaAttachment {
    Leading,
    Trailing,
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

    fn range_of(source: &str, needle: &str) -> TextRange {
        let start = source.find(needle).expect("needle in source");
        TextRange::new(start.into(), (start + needle.len()).into())
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
    fn blank_line_detection_counts_empty_source_lines_between_ranges() {
        let source = "first();\nsecond();\n\nthird();";
        let context = JavaFormatContext::new(source);

        assert!(!context.has_blank_line_between(
            TextRange::new(0usize.into(), 8usize.into()),
            TextRange::new(9usize.into(), 18usize.into())
        ));
        assert!(context.has_blank_line_between(
            TextRange::new(9usize.into(), 18usize.into()),
            TextRange::new(20usize.into(), 28usize.into())
        ));
    }

    #[test]
    fn leading_comments_are_consumed_before_code_range() {
        let source = "// file\n/** docs */\nclass A {}";
        let mut context = JavaFormatContext::new(source);
        let comments =
            context.take_leading_comments(TextRange::new(20usize.into(), 30usize.into()));

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
                .is_empty()
        );
        assert!(
            context
                .take_trailing_line_comment(TextRange::new(10usize.into(), 33usize.into()))
                .is_none()
        );
        assert!(context.has_unhandled_comment_trivia());
    }

    #[test]
    fn leading_comments_do_not_claim_non_leading_buckets() {
        let source = "class A { int first; // trailing\nint second; }";
        let mut context = JavaFormatContext::new(source);

        assert!(
            context
                .take_leading_comments(range_of(source, "int second;"))
                .is_empty()
        );
        assert_eq!(
            source_text(source, &context.unhandled_comment_trivia().unwrap().trivia),
            "// trailing"
        );
    }

    #[test]
    fn dangling_comments_do_not_claim_inline_or_ignored_trivia() {
        let source = "class A { /* inline */ int value;\u{001A} }";
        let mut context = JavaFormatContext::new(source);

        assert!(
            context
                .take_dangling_comments(range_of(source, "{ /* inline */ int value;\u{001A} }"))
                .is_empty()
        );
        assert_eq!(
            source_text(source, &context.unhandled_comment_trivia().unwrap().trivia),
            "/* inline */"
        );
    }

    #[test]
    fn trailing_line_comment_does_not_claim_inline_trailing_block() {
        let source = "class A { int value; /* block */ }";
        let mut context = JavaFormatContext::new(source);

        assert!(
            context
                .take_trailing_line_comment(range_of(source, "int value;"))
                .is_none()
        );
        assert_eq!(
            source_text(source, &context.unhandled_comment_trivia().unwrap().trivia),
            "/* block */"
        );

        let comments = context.take_inline_trailing_block_comments(range_of(source, "int value;"));
        assert_eq!(comments.len(), 1);
        assert_eq!(source_text(source, &comments[0].trivia), "/* block */");
    }

    #[test]
    fn remaining_comments_are_visible_as_unhandled_debt() {
        let source = "class A {}\u{001A}";
        let context = JavaFormatContext::new(source);

        assert!(context.has_unhandled_comment_trivia());
        assert_eq!(
            context.unhandled_comment_trivia().unwrap().trivia.kind,
            TriviaKind::Ignored
        );
    }

    #[test]
    fn comment_bucket_distinguishes_trailing_line_from_next_leading() {
        let source = "class A { int first; // trailing\nint second; }";
        let context = JavaFormatContext::new(source);
        let comment = context
            .unhandled_comment_trivia()
            .expect("line comment after first field");

        assert_eq!(
            context.comment_bucket_for_range(comment, range_of(source, "int first;")),
            JavaCommentBucket::TrailingLine
        );
        assert_eq!(
            context.comment_bucket_for_range(comment, range_of(source, "int second;")),
            JavaCommentBucket::Remaining
        );
    }

    #[test]
    fn comment_bucket_classifies_own_line_comments_by_candidate_owner() {
        let source = "class A {\n// member\nint value;\n}";
        let context = JavaFormatContext::new(source);
        let comment = context
            .unhandled_comment_trivia()
            .expect("own-line member comment");

        assert_eq!(
            context.comment_bucket_for_range(comment, range_of(source, "int value;")),
            JavaCommentBucket::Leading
        );
        assert_eq!(
            context
                .comment_bucket_for_range(comment, range_of(source, "{\n// member\nint value;\n}")),
            JavaCommentBucket::Dangling
        );
    }

    #[test]
    fn comment_bucket_classifies_inline_block_adjacency() {
        let source = "class A { int /* inline */ value; }";
        let context = JavaFormatContext::new(source);
        let comment = context
            .unhandled_comment_trivia()
            .expect("inline block comment");

        assert_eq!(
            context.comment_bucket_for_range(comment, range_of(source, "value")),
            JavaCommentBucket::InlineLeadingBlock
        );
        assert_eq!(
            context.comment_bucket_for_range(comment, range_of(source, "int")),
            JavaCommentBucket::InlineTrailingBlock
        );
    }
}
