// Handles generic type argument closes, consuming one `>` atom at a time.
use super::{JavaSyntaxKind, Parser};
use crate::parser::source::{TokenBuffer, TokenCursor};

// Bounds native and WASM parser stacks without limiting token consumption.
// The next nested argument becomes one lossless bogus type.
pub(in crate::parser::grammar) const MAX_GENERIC_TYPE_DEPTH: usize = 64;

pub(in crate::parser::grammar) fn over_depth_type_end(
    buffer: &mut TokenBuffer<'_>,
    mut cursor: TokenCursor,
) -> usize {
    let (mut angles, mut parens, mut braces, mut brackets) = (0usize, 0usize, 0usize, 0usize);
    loop {
        let kind = cursor.kind(buffer);
        let outside_delimiters = parens == 0 && braces == 0 && brackets == 0;
        if matches!(kind, JavaSyntaxKind::Eof | JavaSyntaxKind::Semicolon)
            || matches!(kind, JavaSyntaxKind::RParen) && parens == 0
            || matches!(kind, JavaSyntaxKind::RBrace) && braces == 0
            || matches!(kind, JavaSyntaxKind::RBracket) && brackets == 0
            || (outside_delimiters
                && (matches!(
                    kind,
                    JavaSyntaxKind::Assign
                        | JavaSyntaxKind::LBrace
                        | JavaSyntaxKind::Colon
                        | JavaSyntaxKind::Arrow
                ) || (angles == 0
                    && matches!(
                        kind,
                        JavaSyntaxKind::Comma
                            | JavaSyntaxKind::Gt
                            | JavaSyntaxKind::Amp
                            | JavaSyntaxKind::Bar
                    ))))
        {
            return cursor.position();
        }

        match kind {
            JavaSyntaxKind::Lt if outside_delimiters => angles += 1,
            JavaSyntaxKind::Gt if outside_delimiters => angles = angles.saturating_sub(1),
            JavaSyntaxKind::LParen => parens += 1,
            JavaSyntaxKind::RParen => parens = parens.saturating_sub(1),
            JavaSyntaxKind::LBrace => braces += 1,
            JavaSyntaxKind::RBrace => braces = braces.saturating_sub(1),
            JavaSyntaxKind::LBracket => brackets += 1,
            JavaSyntaxKind::RBracket => brackets = brackets.saturating_sub(1),
            _ => {}
        }
        cursor.bump(buffer);
    }
}

impl Parser<'_> {
    pub(in crate::parser::grammar) fn at_type_argument_close(&mut self) -> bool {
        self.current_kind() == JavaSyntaxKind::Gt
    }

    pub(in crate::parser::grammar) fn eat_type_argument_close(&mut self) -> bool {
        if self.at_type_argument_close() {
            self.bump();
            true
        } else {
            false
        }
    }

    pub(in crate::parser::grammar) fn type_arguments_are_followed_by_double_colon(
        &mut self,
    ) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_type_arguments();
        lookahead.at(JavaSyntaxKind::DoubleColon)
    }

    pub(in crate::parser::grammar) fn type_arguments_are_followed_by_dot(&mut self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_type_arguments();
        lookahead.at(JavaSyntaxKind::Dot)
    }

    pub(in crate::parser::grammar) fn dot_is_followed_by_annotated_name(&mut self) -> bool {
        if !self.at(JavaSyntaxKind::Dot) {
            return false;
        }

        let mut lookahead = self.lookahead();
        lookahead.bump();
        lookahead.skip_annotations();
        lookahead.at_name_segment()
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;
    use crate::{JavaSyntaxKind, parse_compilation_unit, parser::JavaParseDiagnosticCode};
    use jolt_test_support::assert_exact_diagnostic_owner;

    fn nested_type(depth: usize, leaf: &str) -> String {
        let mut ty = String::new();
        for index in 0..depth {
            ty.push_str(&format!("T{index}<"));
        }
        ty.push_str(leaf);
        ty.push_str(&">".repeat(depth));
        ty
    }

    fn alternating_annotation_type(depth: usize) -> String {
        let mut ty = String::with_capacity(depth * 24 + 4);
        for index in (0..depth).rev() {
            write!(&mut ty, "T{index}<@A((").expect("writing to String cannot fail");
        }
        ty.push_str("Leaf");
        for _ in 0..depth {
            ty.push_str(") value) Leaf>");
        }
        ty
    }

    fn count_nodes(source: &str, kind: JavaSyntaxKind) -> usize {
        let parse = parse_compilation_unit(source);
        let root = parse.syntax().expect("represented compilation unit");
        assert_eq!(root.source_text(), source);
        let mut nodes = vec![*root.syntax()];
        let mut count = 0;
        while let Some(node) = nodes.pop() {
            nodes.extend(node.children());
            count += usize::from(node.kind() == kind);
        }
        count
    }

    #[test]
    fn generic_depth_limit_preserves_the_last_structured_level() {
        let depth_64 = format!("class C {{ {} value; }}", nested_type(64, "Leaf"));
        assert!(parse_compilation_unit(&depth_64).diagnostics().is_empty());
        assert_eq!(count_nodes(&depth_64, JavaSyntaxKind::BogusType), 0);

        let depth_65 = format!(
            "class C {{ {} value; int following; }} class D {{}}",
            nested_type(65, "Leaf")
        );
        let parse = parse_compilation_unit(&depth_65);
        assert_eq!(parse.diagnostics().len(), 1);
        assert_eq!(
            parse.diagnostics()[0].code,
            JavaParseDiagnosticCode::ExcessiveTypeNesting.id()
        );
        assert_eq!(
            parse.diagnostics()[0].message,
            "generic type nesting exceeds 64 levels"
        );
        let root = parse.syntax().expect("represented compilation unit");
        assert_exact_diagnostic_owner(
            *root.syntax(),
            parse.diagnostics(),
            parse.structural_diagnostic_owners(),
            JavaParseDiagnosticCode::ExcessiveTypeNesting.id(),
            "generic type nesting exceeds 64 levels",
            JavaSyntaxKind::BogusType,
            None,
        );
        assert_eq!(count_nodes(&depth_65, JavaSyntaxKind::BogusType), 1);
        assert_eq!(count_nodes(&depth_65, JavaSyntaxKind::FieldDeclaration), 2);
        assert_eq!(count_nodes(&depth_65, JavaSyntaxKind::ClassDeclaration), 2);
    }

    #[test]
    fn very_deep_generic_recovery_is_lossless_and_bounded() {
        let source = format!(
            "class C {{ {} value; int following; }} class D {{}}",
            nested_type(4096, "Leaf")
        );
        assert_eq!(count_nodes(&source, JavaSyntaxKind::BogusType), 1);
        assert_eq!(count_nodes(&source, JavaSyntaxKind::FieldDeclaration), 2);
        assert_eq!(count_nodes(&source, JavaSyntaxKind::ClassDeclaration), 2);
    }

    #[test]
    fn annotation_cast_reentry_cannot_reset_the_active_depth() {
        let source = format!(
            "class C {{ {} value; int following; }} class D {{}}",
            alternating_annotation_type(4096)
        );
        let parse = parse_compilation_unit(&source);
        assert_eq!(
            parse.syntax().expect("represented tree").source_text(),
            source
        );
        assert_eq!(
            parse
                .diagnostics()
                .iter()
                .filter(|diagnostic| {
                    diagnostic.code == JavaParseDiagnosticCode::ExcessiveTypeNesting.id()
                })
                .count(),
            1
        );
        assert_eq!(count_nodes(&source, JavaSyntaxKind::FieldDeclaration), 2);
        assert_eq!(count_nodes(&source, JavaSyntaxKind::ClassDeclaration), 2);

        let malformed_leaf = format!("+ @A(({}) value) Leaf", nested_type(4096, "Leaf"));
        let malformed = format!(
            "class C {{ {} value; int following; }} class D {{}}",
            nested_type(65, &malformed_leaf)
        );
        assert_eq!(count_nodes(&malformed, JavaSyntaxKind::FieldDeclaration), 2);
        assert_eq!(count_nodes(&malformed, JavaSyntaxKind::ClassDeclaration), 2);
    }

    #[test]
    fn over_depth_wildcard_is_normalized_to_one_bogus_type() {
        let source = format!("class C {{ {} value; }}", nested_type(65, "? extends Leaf"));
        let parse = parse_compilation_unit(&source);
        assert_eq!(parse.diagnostics().len(), 1);
        assert_eq!(
            parse.diagnostics()[0].code,
            JavaParseDiagnosticCode::ExcessiveTypeNesting.id()
        );
        assert_eq!(count_nodes(&source, JavaSyntaxKind::WildcardType), 0);
        assert_eq!(count_nodes(&source, JavaSyntaxKind::BogusType), 1);
    }

    #[test]
    fn over_depth_scan_stops_at_outer_and_hard_recovery_boundaries() {
        for (fragment, end) in [
            ("Type<Nested>, tail", 4),
            ("Type @A(value = {1, 2}), tail", 12),
            ("Broken(value; int following", 3),
            ("Broken[value; int following", 3),
            ("Broken(value } class Following", 3),
            ("Broken[value ) tail", 3),
            ("Type & Other", 1),
            ("Type | Other", 1),
        ] {
            let mut parser = Parser::new(fragment);
            let cursor = parser.inner.fork_cursor();
            assert_eq!(
                over_depth_type_end(&mut parser.inner.buffer, cursor),
                end,
                "fragment: {fragment}"
            );
        }
    }

    #[test]
    fn over_depth_invalid_argument_uses_the_same_endpoint_in_both_grammars() {
        let shallow = format!("class C {{ {} value; }}", nested_type(1, "+"));
        assert_eq!(
            parse_compilation_unit(&shallow).diagnostics()[0].code,
            JavaParseDiagnosticCode::ExpectedSyntax.id()
        );

        let deep = format!(
            "class C {{ {} value; int following; }}",
            nested_type(65, "+")
        );
        let parse = parse_compilation_unit(&deep);
        assert_eq!(parse.diagnostics().len(), 1);
        assert_eq!(
            parse.diagnostics()[0].code,
            JavaParseDiagnosticCode::ExcessiveTypeNesting.id()
        );
        assert_eq!(count_nodes(&deep, JavaSyntaxKind::FieldDeclaration), 2);
    }

    #[test]
    fn unmatched_over_depth_delimiters_preserve_following_declarations() {
        for leaf in ["Broken(value", "Broken[value"] {
            let source = format!(
                "class C {{ {} ; int following; }} class D {{}}",
                nested_type(65, leaf).trim_end_matches('>')
            );
            assert_eq!(count_nodes(&source, JavaSyntaxKind::BogusType), 1);
            assert_eq!(count_nodes(&source, JavaSyntaxKind::FieldDeclaration), 2);
            assert_eq!(count_nodes(&source, JavaSyntaxKind::ClassDeclaration), 2);
        }
    }
}
