use super::super::{
    ArrayDimensions, ArrayType, ClassType, JavaSyntaxToken, NameSyntax, Type, child, child_family,
};
use super::helpers::simple_single_token;

impl ArrayType {
    #[must_use]
    pub fn dimensions(&self) -> Option<ArrayDimensions> {
        child(&self.syntax)
    }
}

impl Type {
    #[must_use]
    pub fn simple_layout_tokens(&self) -> Option<Vec<JavaSyntaxToken>> {
        match self {
            Self::PrimitiveType(primitive) => simple_single_token(&primitive.syntax),
            Self::VoidType(void) => simple_single_token(&void.syntax),
            Self::ClassType(class) => class.simple_layout_name_tokens(),
            Self::ArrayType(_)
            | Self::IntersectionType(_)
            | Self::UnionType(_)
            | Self::WildcardType(_) => None,
        }
    }
}

impl ClassType {
    fn simple_layout_name_tokens(&self) -> Option<Vec<JavaSyntaxToken>> {
        let kinds = self
            .syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .collect::<Vec<_>>();
        let [kind] = kinds.as_slice() else {
            return None;
        };
        if !NameSyntax::can_cast(*kind) {
            return None;
        }

        let name: NameSyntax = child_family(&self.syntax)?;
        Some(name.segments().collect())
    }
}
