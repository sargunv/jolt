use super::{JavaSyntaxKind, Parser};

impl Parser<'_> {
    pub(in crate::parser::grammar) fn at_type_modifier(&self) -> bool {
        self.is_type_modifier_at(self.position())
    }

    pub(in crate::parser::grammar) fn is_type_modifier_at(&self, index: usize) -> bool {
        matches!(
            self.kind_at(index),
            JavaSyntaxKind::PublicKw
                | JavaSyntaxKind::ProtectedKw
                | JavaSyntaxKind::PrivateKw
                | JavaSyntaxKind::AbstractKw
                | JavaSyntaxKind::StaticKw
                | JavaSyntaxKind::FinalKw
                | JavaSyntaxKind::TransientKw
                | JavaSyntaxKind::VolatileKw
                | JavaSyntaxKind::SynchronizedKw
                | JavaSyntaxKind::NativeKw
                | JavaSyntaxKind::StrictfpKw
                | JavaSyntaxKind::DefaultKw
        ) || self.text_at(index) == Some("sealed")
            || (self.text_at(index) == Some("non")
                && self.kind_at(index + 1) == JavaSyntaxKind::Minus
                && self.text_at(index + 2) == Some("sealed"))
    }

    pub(in crate::parser::grammar) fn skip_type_modifier_at(&self, index: usize) -> usize {
        if self.text_at(index) == Some("non")
            && self.kind_at(index + 1) == JavaSyntaxKind::Minus
            && self.text_at(index + 2) == Some("sealed")
        {
            index + 3
        } else {
            index + 1
        }
    }

    pub(in crate::parser::grammar) fn bump_type_modifier(&mut self) {
        let next = self.skip_type_modifier_at(self.position());
        while self.position() < next {
            self.bump();
        }
    }

    pub(in crate::parser::grammar) fn at_name_segment(&self) -> bool {
        self.is_name_segment_at(self.position())
    }

    pub(in crate::parser::grammar) fn nth_is_name_segment(&self, n: usize) -> bool {
        self.is_name_segment_at(self.position() + n)
    }

    pub(in crate::parser::grammar) fn is_name_segment_at(&self, index: usize) -> bool {
        self.kind_at(index) == JavaSyntaxKind::Identifier
    }

    pub(in crate::parser::grammar) fn at_primitive_type(&self) -> bool {
        self.is_primitive_type_start_at(self.position())
    }

    pub(in crate::parser::grammar) fn starts_array_dimensions(&self) -> bool {
        let index = self.skip_annotations_from(self.position());
        self.kind_at(index) == JavaSyntaxKind::LBracket
            && self.kind_at(index + 1) == JavaSyntaxKind::RBracket
    }

    pub(in crate::parser::grammar) fn starts_dim_expression(&self) -> bool {
        let index = self.skip_annotations_from(self.position());
        self.kind_at(index) == JavaSyntaxKind::LBracket
            && self.kind_at(index + 1) != JavaSyntaxKind::RBracket
    }
}
