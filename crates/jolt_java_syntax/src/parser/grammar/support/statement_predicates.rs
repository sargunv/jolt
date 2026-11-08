//! Statement-level grammar questions, answered before the parser commits to
//! a branch.

use jolt_syntax::TokenSet;

use super::{JavaSyntaxKind, Parser, gt_count, is_assignment_operator};

impl Parser<'_> {
    pub(in crate::parser::grammar) fn starts_local_variable_declaration(&self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_variable_modifiers();

        if lookahead.at_contextual("yield") && lookahead.nth(1) != JavaSyntaxKind::Dot {
            return false;
        }

        if lookahead.at_contextual("var") && lookahead.nth(1) != JavaSyntaxKind::Dot {
            return matches!(
                lookahead.nth(1),
                JavaSyntaxKind::Identifier | JavaSyntaxKind::UnderscoreKw
            ) && !matches!(
                lookahead.nth(2),
                JavaSyntaxKind::LParen | JavaSyntaxKind::Dot
            );
        }

        if !lookahead.at_non_void_type_start() {
            return false;
        }

        lookahead.skip_type();
        lookahead.at_variable_identifier() && !matches!(lookahead.nth(1), JavaSyntaxKind::LParen)
    }

    pub(in crate::parser::grammar) fn starts_labeled_statement(&self) -> bool {
        self.current() == JavaSyntaxKind::Identifier && self.nth(1) == JavaSyntaxKind::Colon
    }

    pub(in crate::parser::grammar) fn starts_yield_statement(&self) -> bool {
        let position = self.position();
        self.at_contextual("yield")
            && !is_assignment_operator(self.nth(1))
            && !matches!(
                self.nth(1),
                JavaSyntaxKind::LBracket
                    | JavaSyntaxKind::Dot
                    | JavaSyntaxKind::PlusPlus
                    | JavaSyntaxKind::MinusMinus
                    | JavaSyntaxKind::Semicolon
            )
            && (self.nth(1) != JavaSyntaxKind::LParen || !self.tokens_touch(position))
    }

    /// Whether the statement at the current token is an explicit constructor
    /// invocation: a `this(` or `super(` before its `;`.
    pub(in crate::parser::grammar) fn starts_constructor_invocation_statement(&self) -> bool {
        if matches!(
            self.current(),
            JavaSyntaxKind::LBrace | JavaSyntaxKind::RParen | JavaSyntaxKind::RBracket
        ) {
            return false;
        }
        let (end, saw_invocation) = self.table.statement_end(self.position());
        saw_invocation && self.kind_at(end) == JavaSyntaxKind::Semicolon
    }

    /// Whether the `for` header at the current token has a `:` outside its
    /// brackets, type arguments, and conditional expressions.
    pub(in crate::parser::grammar) fn for_header_has_top_level_colon(&self) -> bool {
        let mut index = self.position();
        while !matches!(
            self.kind_at(index),
            JavaSyntaxKind::Eof | JavaSyntaxKind::LParen
        ) {
            index += 1;
        }
        self.kind_at(index) == JavaSyntaxKind::LParen
            && self.first_top_level_colon_or(index + 1, token_set![Semicolon, RParen, RBracket])
                == JavaSyntaxKind::Colon
    }

    pub(in crate::parser::grammar) fn starts_switch_label(&self) -> bool {
        matches!(
            self.current(),
            JavaSyntaxKind::CaseKw | JavaSyntaxKind::DefaultKw
        )
    }

    /// Whether the switch label at the current token ends with `->` rather
    /// than `:`.
    pub(in crate::parser::grammar) fn switch_label_is_rule(&self) -> bool {
        self.first_top_level_colon_or(self.position(), token_set![Arrow, RBrace, RParen, RBracket])
            == JavaSyntaxKind::Arrow
    }

    /// The first token from `index` on, outside brackets and type arguments,
    /// that is in `ends` or is a `:` that closes no conditional `?`; EOF if
    /// there is none. Bracketed groups are skipped through the lookahead
    /// table, so a scan is linear in the tokens outside them.
    fn first_top_level_colon_or(&self, mut index: usize, ends: TokenSet) -> JavaSyntaxKind {
        let (mut conditionals, mut angles) = (0usize, 0usize);
        loop {
            let kind = self.kind_at(index);
            match kind {
                JavaSyntaxKind::Eof => return kind,
                JavaSyntaxKind::LParen | JavaSyntaxKind::LBracket => {
                    index = self.table.bracket_end(index);
                    continue;
                }
                _ if ends.contains(kind.to_raw()) => return kind,
                JavaSyntaxKind::Lt => angles += 1,
                _ if angles > 0 => angles = angles.saturating_sub(gt_count(kind)),
                JavaSyntaxKind::Question => conditionals += 1,
                JavaSyntaxKind::Colon if conditionals > 0 => conditionals -= 1,
                JavaSyntaxKind::Colon => return kind,
                _ => {}
            }
            index += 1;
        }
    }
}
