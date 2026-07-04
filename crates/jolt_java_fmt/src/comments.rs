use std::collections::HashMap;

use jolt_java_syntax::{CompilationUnit, JavaComment, JavaSyntaxToken};

#[derive(Default)]
pub(crate) struct CommentMap<'source> {
    leading: HashMap<CommentAnchor, Vec<JavaComment<'source>>>,
    trailing: HashMap<CommentAnchor, Vec<JavaComment<'source>>>,
}

impl<'source> CommentMap<'source> {
    pub(crate) fn from_compilation_unit(unit: &CompilationUnit<'source>) -> Self {
        let mut map = Self::default();

        for token in unit.token_iter() {
            let anchor = comment_anchor(&token);
            for comment in token.leading_comments() {
                map.leading.entry(anchor).or_default().push(comment);
            }
            for comment in token.trailing_comments() {
                map.trailing.entry(anchor).or_default().push(comment);
            }
        }

        map
    }

    pub(crate) fn leading_comments_for_token(
        &self,
        token: &JavaSyntaxToken<'source>,
    ) -> &[JavaComment<'source>] {
        self.leading
            .get(&comment_anchor(token))
            .map_or(&[], Vec::as_slice)
    }

    pub(crate) fn leading_comments_for_token_option(
        &self,
        token: Option<&JavaSyntaxToken<'source>>,
    ) -> &[JavaComment<'source>] {
        token.map_or(&[], |token| self.leading_comments_for_token(token))
    }

    pub(crate) fn trailing_comments_for_token(
        &self,
        token: &JavaSyntaxToken<'source>,
    ) -> &[JavaComment<'source>] {
        self.trailing
            .get(&comment_anchor(token))
            .map_or(&[], Vec::as_slice)
    }

    pub(crate) fn trailing_comments_for_token_option(
        &self,
        token: Option<&JavaSyntaxToken<'source>>,
    ) -> &[JavaComment<'source>] {
        token.map_or(&[], |token| self.trailing_comments_for_token(token))
    }

    pub(crate) fn has_leading_comment_for_token(&self, token: &JavaSyntaxToken<'source>) -> bool {
        !self.leading_comments_for_token(token).is_empty()
    }

    pub(crate) fn has_delimiter_dangling_comments(
        open: Option<&JavaSyntaxToken<'source>>,
        close: Option<&JavaSyntaxToken<'source>>,
    ) -> bool {
        open.is_some_and(|token| !token.trailing_comments().is_empty())
            || close.is_some_and(|token| !token.leading_comments().is_empty())
    }

    pub(crate) fn delimiter_dangling_comments(
        open: Option<&JavaSyntaxToken<'source>>,
        close: Option<&JavaSyntaxToken<'source>>,
    ) -> Vec<JavaComment<'source>> {
        let mut comments = Vec::new();

        if let Some(open) = open {
            comments.extend(open.trailing_comments());
        }
        if let Some(close) = close {
            comments.extend(close.leading_comments());
        }

        comments
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct CommentAnchor {
    start: usize,
    end: usize,
}

fn comment_anchor(token: &JavaSyntaxToken<'_>) -> CommentAnchor {
    let range = token.token_text_range();
    CommentAnchor {
        start: range.start().get(),
        end: range.end().get(),
    }
}

#[cfg(test)]
mod tests {
    use jolt_diagnostics::SyntaxOutcome;
    use jolt_java_syntax::{JavaSyntaxKind, parse_compilation_unit};

    use super::CommentMap;

    #[test]
    fn classifies_import_comments_by_anchor_token() {
        let parse =
            parse_compilation_unit("import b.Beta; // trailing\n// leading\nimport a.Alpha;\n");
        let syntax = parse.syntax().expect("clean parse").clone();

        assert_eq!(parse.outcome(), SyntaxOutcome::Clean);
        assert!(parse.diagnostics().is_empty());

        let map = CommentMap::from_compilation_unit(&syntax);
        let imports = syntax.imports().collect::<Vec<_>>();
        let first_last_token = imports[0].last_token().expect("first import last token");
        let second_first_token = imports[1].first_token().expect("second import first token");

        assert_eq!(
            map.trailing_comments_for_token(&first_last_token)[0].text(),
            "// trailing"
        );
        assert_eq!(
            map.leading_comments_for_token(&second_first_token)[0].text(),
            "// leading"
        );
        assert!(map.has_leading_comment_for_token(&second_first_token));
    }

    #[test]
    fn classifies_delimiter_dangling_comments() {
        let parse =
            parse_compilation_unit("class A { void f() { call( /* open */\n/* close */ ); } }\n");
        let syntax = parse.syntax().expect("clean parse").clone();

        assert_eq!(parse.outcome(), SyntaxOutcome::Clean);
        assert!(parse.diagnostics().is_empty());

        let mut tokens = syntax.token_iter();
        let open = tokens
            .find(|token| {
                token.kind() == JavaSyntaxKind::LParen && token.trailing_comments().len() == 1
            })
            .expect("commented open paren");
        let close = syntax
            .token_iter()
            .find(|token| {
                token.kind() == JavaSyntaxKind::RParen && !token.leading_comments().is_empty()
            })
            .expect("commented close paren");

        assert!(CommentMap::has_delimiter_dangling_comments(
            Some(&open),
            Some(&close)
        ));
        let comments = CommentMap::delimiter_dangling_comments(Some(&open), Some(&close));
        assert_eq!(comments[0].text(), "/* open */");
        assert_eq!(comments[1].text(), "/* close */");
    }
}
