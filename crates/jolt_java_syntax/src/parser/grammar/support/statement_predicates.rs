// Answers statement-level grammar questions before the parser commits to a branch.
use super::{JavaSyntaxKind, Parser, gt_rest, is_assignment_operator, is_gt_family};

/// The number of leading `>` characters of a `>`-family token.
pub(in crate::parser::grammar) fn gt_count(mut kind: JavaSyntaxKind) -> usize {
    let mut count = 0;
    while is_gt_family(kind) {
        count += 1;
        match gt_rest(kind) {
            Some(rest) => kind = rest,
            None => break,
        }
    }
    count
}

impl Parser<'_> {
    pub(in crate::parser::grammar) fn starts_local_variable_declaration(&mut self) -> bool {
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

    pub(in crate::parser::grammar) fn starts_labeled_statement(&mut self) -> bool {
        self.current() == JavaSyntaxKind::Identifier && self.nth(1) == JavaSyntaxKind::Colon
    }

    pub(in crate::parser::grammar) fn starts_yield_statement(&mut self) -> bool {
        let position = self.position();
        self.at_contextual("yield")
            && !is_assignment_operator(self.nth(1))
            && !matches!(
                self.nth(1),
                JavaSyntaxKind::LBracket
                    | JavaSyntaxKind::Dot
                    | JavaSyntaxKind::Assign
                    | JavaSyntaxKind::PlusPlus
                    | JavaSyntaxKind::MinusMinus
                    | JavaSyntaxKind::PlusEq
                    | JavaSyntaxKind::MinusEq
                    | JavaSyntaxKind::StarEq
                    | JavaSyntaxKind::SlashEq
                    | JavaSyntaxKind::AmpEq
                    | JavaSyntaxKind::BarEq
                    | JavaSyntaxKind::CaretEq
                    | JavaSyntaxKind::PercentEq
                    | JavaSyntaxKind::LShiftEq
                    | JavaSyntaxKind::Semicolon
            )
            && (self.nth(1) != JavaSyntaxKind::LParen || !self.tokens_touch(position))
    }

    pub(in crate::parser::grammar) fn for_header_has_top_level_colon(&mut self) -> bool {
        let mut index = self.position();
        while self.kind_at(index) != JavaSyntaxKind::Eof
            && self.kind_at(index) != JavaSyntaxKind::LParen
        {
            index += 1;
        }

        if self.kind_at(index) != JavaSyntaxKind::LParen {
            return false;
        }

        index += 1;
        let mut conditional_depth = 0usize;
        let mut angle_depth = 0usize;
        while self.kind_at(index) != JavaSyntaxKind::Eof {
            if let Some(next) = self.skip_balanced_delimiter_at(index) {
                index = next;
                continue;
            }

            match self.kind_at(index) {
                JavaSyntaxKind::Lt => angle_depth += 1,
                kind if is_gt_family(kind) && angle_depth > 0 => {
                    angle_depth = angle_depth.saturating_sub(gt_count(kind));
                }
                JavaSyntaxKind::Question if angle_depth == 0 => conditional_depth += 1,
                JavaSyntaxKind::Colon if angle_depth == 0 && conditional_depth > 0 => {
                    conditional_depth -= 1;
                }
                JavaSyntaxKind::Colon if angle_depth == 0 => return true,
                JavaSyntaxKind::Semicolon | JavaSyntaxKind::RParen | JavaSyntaxKind::RBracket => {
                    return false;
                }
                _ => {}
            }
            index += 1;
        }

        false
    }

    pub(in crate::parser::grammar) fn starts_switch_label(&mut self) -> bool {
        matches!(
            self.current(),
            JavaSyntaxKind::CaseKw | JavaSyntaxKind::DefaultKw
        )
    }

    pub(in crate::parser::grammar) fn switch_label_is_rule(&mut self) -> bool {
        let mut index = self.position();
        let mut conditional_depth = 0usize;
        let mut angle_depth = 0usize;
        while self.kind_at(index) != JavaSyntaxKind::Eof {
            if let Some(next) = self.skip_balanced_delimiter_at(index) {
                index = next;
                continue;
            }

            match self.kind_at(index) {
                JavaSyntaxKind::Arrow => return true,
                JavaSyntaxKind::Lt => angle_depth += 1,
                kind if is_gt_family(kind) && angle_depth > 0 => {
                    angle_depth = angle_depth.saturating_sub(gt_count(kind));
                }
                JavaSyntaxKind::Question if angle_depth == 0 => conditional_depth += 1,
                JavaSyntaxKind::Colon if angle_depth == 0 && conditional_depth > 0 => {
                    conditional_depth -= 1;
                }
                JavaSyntaxKind::Colon if angle_depth == 0 => return false,
                JavaSyntaxKind::RBrace | JavaSyntaxKind::RParen | JavaSyntaxKind::RBracket => {
                    return false;
                }
                _ => {}
            }
            index += 1;
        }

        false
    }
}
