use super::super::{
    Annotation, ArrayDimension, ArrayDimensions, ArrayType, ClassType, JavaFamily, JavaNode,
    JavaSyntaxKind, JavaSyntaxToken, NameSyntax, Type, TypeArgument, TypeArgumentList,
    WildcardType, child, child_family, children,
};
use super::helpers::simple_single_token;

#[derive(Clone)]
pub enum TypeLayoutPart {
    Annotation(Annotation),
    Text(&'static str),
    Token(JavaSyntaxToken),
}

impl ArrayType {
    #[must_use]
    pub fn dimensions(&self) -> Option<ArrayDimensions> {
        child(&self.syntax)
    }
}

impl ArrayDimensions {
    #[must_use]
    pub fn simple_layout_count(&self) -> Option<usize> {
        let mut count = 0;
        for dimension in children::<ArrayDimension>(&self.syntax) {
            let kinds = dimension
                .syntax
                .children_with_tokens()
                .map(|element| element.kind())
                .collect::<Vec<_>>();
            if kinds.as_slice() != [JavaSyntaxKind::LBracket, JavaSyntaxKind::RBracket] {
                return None;
            }
            count += 1;
        }

        (count > 0).then_some(count)
    }

    fn simple_layout_parts(&self) -> Option<Vec<TypeLayoutPart>> {
        let mut parts = Vec::new();
        for dimension in children::<ArrayDimension>(&self.syntax) {
            let elements = dimension.syntax.children_with_tokens().collect::<Vec<_>>();
            let [prefix @ .., left, right] = elements.as_slice() else {
                return None;
            };
            for element in prefix {
                if element.kind() != JavaSyntaxKind::Annotation {
                    return None;
                }
                parts.push(TypeLayoutPart::Annotation(Annotation::cast(
                    element.clone().into_node()?,
                )?));
            }
            if left.kind() != JavaSyntaxKind::LBracket || right.kind() != JavaSyntaxKind::RBracket {
                return None;
            }
            parts.push(TypeLayoutPart::Text("[]"));
        }

        (!parts.is_empty()).then_some(parts)
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
            Self::ArrayType(array) => array.simple_layout_parts(),
            Self::IntersectionType(_) | Self::UnionType(_) | Self::WildcardType(_) => None,
        }
    }
}

impl ArrayType {
    fn simple_layout_parts(&self) -> Option<Vec<TypeLayoutPart>> {
        let base = child_family::<Type>(&self.syntax)?;
        let mut parts = base.simple_layout_parts()?;
        parts.extend(self.dimensions()?.simple_layout_parts()?);
        Some(parts)
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
                JavaSyntaxKind::TypeArgumentList => parts
                    .extend(TypeArgumentList::cast(element.into_node()?)?.simple_layout_parts()?),
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

impl TypeArgumentList {
    #[must_use]
    pub fn simple_layout_parts(&self) -> Option<Vec<TypeLayoutPart>> {
        let mut parts = Vec::new();
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let Some(first) = elements.first() else {
            return None;
        };
        let Some(last) = elements.last() else {
            return None;
        };
        if first.kind() != JavaSyntaxKind::Lt || last.kind() != JavaSyntaxKind::Gt {
            return None;
        }

        parts.push(TypeLayoutPart::Token(JavaSyntaxToken {
            syntax: first.clone().into_token()?,
        }));
        let inner = &elements[1..elements.len().saturating_sub(1)];
        let mut expect_argument = true;
        for element in inner {
            if expect_argument {
                if element.kind() != JavaSyntaxKind::TypeArgument {
                    return None;
                }
                parts.extend(
                    TypeArgument::cast(element.clone().into_node()?)?.simple_layout_parts()?,
                );
            } else {
                if element.kind() != JavaSyntaxKind::Comma {
                    return None;
                }
                parts.push(TypeLayoutPart::Token(JavaSyntaxToken {
                    syntax: element.clone().into_token()?,
                }));
                parts.push(TypeLayoutPart::Text(" "));
            }
            expect_argument = !expect_argument;
        }
        if expect_argument && !inner.is_empty() {
            return None;
        }
        parts.push(TypeLayoutPart::Token(JavaSyntaxToken {
            syntax: last.clone().into_token()?,
        }));

        Some(parts)
    }
}

impl TypeArgument {
    fn simple_layout_parts(&self) -> Option<Vec<TypeLayoutPart>> {
        let mut parts = Vec::new();
        for element in self.syntax.children_with_tokens() {
            match element.kind() {
                JavaSyntaxKind::Annotation => parts.push(TypeLayoutPart::Annotation(
                    Annotation::cast(element.into_node()?)?,
                )),
                JavaSyntaxKind::WildcardType => {
                    parts.extend(WildcardType::cast(element.into_node()?)?.simple_layout_parts()?);
                }
                kind if Type::can_cast(kind) => {
                    parts.extend(Type::cast(element.into_node()?)?.simple_layout_parts()?);
                }
                _ => return None,
            }
        }

        (!parts.is_empty()).then_some(parts)
    }
}

impl WildcardType {
    fn simple_layout_parts(&self) -> Option<Vec<TypeLayoutPart>> {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        match elements.as_slice() {
            [question] if question.kind() == JavaSyntaxKind::Question => {
                Some(vec![TypeLayoutPart::Token(JavaSyntaxToken {
                    syntax: question.clone().into_token()?,
                })])
            }
            [question, bound, ty]
                if question.kind() == JavaSyntaxKind::Question
                    && matches!(
                        bound.kind(),
                        JavaSyntaxKind::ExtendsKw | JavaSyntaxKind::SuperKw
                    )
                    && Type::can_cast(ty.kind()) =>
            {
                let mut parts = vec![
                    TypeLayoutPart::Token(JavaSyntaxToken {
                        syntax: question.clone().into_token()?,
                    }),
                    TypeLayoutPart::Text(" "),
                    TypeLayoutPart::Token(JavaSyntaxToken {
                        syntax: bound.clone().into_token()?,
                    }),
                    TypeLayoutPart::Text(" "),
                ];
                parts.extend(Type::cast(ty.clone().into_node()?)?.simple_layout_parts()?);
                Some(parts)
            }
            _ => None,
        }
    }
}
