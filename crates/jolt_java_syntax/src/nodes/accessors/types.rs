use super::super::{
    Annotation, ArrayDimensions, ArrayType, ClassType, JavaFamily, JavaNode, JavaSyntaxKind,
    JavaSyntaxToken, NameSyntax, Type, child,
};
use super::helpers::simple_single_token;

#[derive(Clone)]
pub enum TypeLayoutPart {
    Annotation(Annotation),
    Token(JavaSyntaxToken),
}

impl ArrayType {
    #[must_use]
    pub fn dimensions(&self) -> Option<ArrayDimensions> {
        child(&self.syntax)
    }
}

impl Type {
    #[must_use]
    pub fn simple_layout_parts(&self) -> Option<Vec<TypeLayoutPart>> {
        match self {
            Self::PrimitiveType(primitive) => simple_single_token(&primitive.syntax)
                .map(|tokens| tokens.into_iter().map(TypeLayoutPart::Token).collect()),
            Self::VoidType(void) => simple_single_token(&void.syntax)
                .map(|tokens| tokens.into_iter().map(TypeLayoutPart::Token).collect()),
            Self::ClassType(class) => class.simple_layout_name_tokens(),
            Self::ArrayType(_)
            | Self::IntersectionType(_)
            | Self::UnionType(_)
            | Self::WildcardType(_) => None,
        }
    }
}

impl ClassType {
    fn simple_layout_name_tokens(&self) -> Option<Vec<TypeLayoutPart>> {
        let mut parts = Vec::new();
        for element in self.syntax.children_with_tokens() {
            match element.kind() {
                JavaSyntaxKind::Annotation => parts.push(TypeLayoutPart::Annotation(
                    Annotation::cast(element.into_node()?)?,
                )),
                JavaSyntaxKind::Name | JavaSyntaxKind::QualifiedName => {
                    parts.extend(simple_name_layout_parts(&NameSyntax::cast(
                        element.into_node()?,
                    )?)?);
                }
                JavaSyntaxKind::Dot => parts.push(TypeLayoutPart::Token(JavaSyntaxToken {
                    syntax: element.into_token()?,
                })),
                _ => return None,
            }
        }

        (!parts.is_empty()).then_some(parts)
    }
}

fn simple_name_layout_parts(name: &NameSyntax) -> Option<Vec<TypeLayoutPart>> {
    let mut parts = Vec::new();
    for element in name.syntax().children_with_tokens() {
        match element.kind() {
            JavaSyntaxKind::Annotation => parts.push(TypeLayoutPart::Annotation(Annotation::cast(
                element.into_node()?,
            )?)),
            JavaSyntaxKind::Dot | JavaSyntaxKind::Identifier => {
                parts.push(TypeLayoutPart::Token(JavaSyntaxToken {
                    syntax: element.into_token()?,
                }));
            }
            _ => return None,
        }
    }
    Some(parts)
}
