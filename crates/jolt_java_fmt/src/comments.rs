use std::collections::HashMap;

use jolt_java_syntax::{CompilationUnit, JavaComment, JavaSyntaxToken};

#[derive(Default)]
pub(crate) struct CommentMap {
    leading: HashMap<CommentAnchor, Vec<JavaComment>>,
    trailing: HashMap<CommentAnchor, Vec<JavaComment>>,
}

impl CommentMap {
    pub(crate) fn from_compilation_unit(unit: &CompilationUnit) -> Self {
        let mut map = Self::default();

        for token in unit.tokens() {
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

    pub(crate) fn leading_comments_for_tokens(&self, tokens: &[JavaSyntaxToken]) -> &[JavaComment] {
        tokens
            .first()
            .and_then(|token| self.leading.get(&comment_anchor(token)))
            .map_or(&[], Vec::as_slice)
    }

    pub(crate) fn trailing_comments_for_tokens(
        &self,
        tokens: &[JavaSyntaxToken],
    ) -> &[JavaComment] {
        tokens
            .last()
            .and_then(|token| self.trailing.get(&comment_anchor(token)))
            .map_or(&[], Vec::as_slice)
    }

    pub(crate) fn has_leading_comment_for_tokens(&self, tokens: &[JavaSyntaxToken]) -> bool {
        !self.leading_comments_for_tokens(tokens).is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct CommentAnchor {
    start: usize,
    end: usize,
}

fn comment_anchor(token: &JavaSyntaxToken) -> CommentAnchor {
    let range = token.token_text_range();
    CommentAnchor {
        start: range.start().get(),
        end: range.end().get(),
    }
}

#[cfg(test)]
mod tests {
    use jolt_java_syntax::{SyntaxOutcome, parse_compilation_unit};

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
        let first_tokens = imports[0].tokens();
        let second_tokens = imports[1].tokens();

        assert_eq!(
            map.trailing_comments_for_tokens(&first_tokens)[0].text(),
            "// trailing"
        );
        assert_eq!(
            map.leading_comments_for_tokens(&second_tokens)[0].text(),
            "// leading"
        );
        assert!(map.has_leading_comment_for_tokens(&second_tokens));
    }
}
