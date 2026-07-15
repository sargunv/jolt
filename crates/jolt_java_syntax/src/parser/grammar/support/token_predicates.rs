// Contains shallow token checks that do not need full grammar lookahead.
use super::{JavaSyntaxKind, Parser, type_modifier_len};

impl Parser<'_> {
    pub(in crate::parser::grammar) fn at_type_modifier(&mut self) -> bool {
        self.is_type_modifier_at(self.position())
    }

    pub(in crate::parser::grammar) fn is_type_modifier_at(&mut self, index: usize) -> bool {
        self.type_modifier_len_at(index).is_some()
    }

    pub(in crate::parser::grammar) fn bump_type_modifier(&mut self) {
        let modifier_len = self.type_modifier_len_at(self.position()).unwrap_or(1);
        let modifier = (modifier_len == 3).then(|| self.start());
        let next = self.position() + modifier_len;
        while self.position() < next {
            self.bump();
        }
        if let Some(modifier) = modifier {
            self.complete(modifier, JavaSyntaxKind::NonSealedModifier);
        }
    }

    fn type_modifier_len_at(&mut self, index: usize) -> Option<usize> {
        type_modifier_len(
            self.kind_at(index),
            self.text_at(index),
            self.kind_at(index + 1),
            self.text_at(index + 2),
        )
    }

    pub(in crate::parser::grammar) fn at_name_segment(&mut self) -> bool {
        self.is_name_segment_at(self.position())
    }

    pub(in crate::parser::grammar) fn nth_is_name_segment(&mut self, n: usize) -> bool {
        self.is_name_segment_at(self.position() + n)
    }

    pub(in crate::parser::grammar) fn is_name_segment_at(&mut self, index: usize) -> bool {
        self.kind_at(index) == JavaSyntaxKind::Identifier
    }

    pub(in crate::parser::grammar) fn at_primitive_type(&mut self) -> bool {
        self.is_primitive_type_start_at(self.position())
    }

    pub(in crate::parser::grammar) fn starts_array_dimensions(&mut self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_annotations();
        lookahead.at(JavaSyntaxKind::LBracket) && lookahead.nth_kind(1) == JavaSyntaxKind::RBracket
    }

    pub(in crate::parser::grammar) fn starts_dim_expression(&mut self) -> bool {
        let mut lookahead = self.lookahead();
        lookahead.skip_annotations();
        lookahead.at(JavaSyntaxKind::LBracket) && lookahead.nth_kind(1) != JavaSyntaxKind::RBracket
    }

    pub(in crate::parser::grammar) fn is_primitive_type_start_at(&mut self, index: usize) -> bool {
        matches!(
            self.kind_at(index),
            JavaSyntaxKind::BooleanKw
                | JavaSyntaxKind::ByteKw
                | JavaSyntaxKind::CharKw
                | JavaSyntaxKind::DoubleKw
                | JavaSyntaxKind::FloatKw
                | JavaSyntaxKind::IntKw
                | JavaSyntaxKind::LongKw
                | JavaSyntaxKind::ShortKw
        )
    }
}
