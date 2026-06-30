use super::super::{
    Annotation, AnnotationArgumentList, AnnotationArrayInitializer, AnnotationElementDeclaration,
    AnnotationElementList, AnnotationElementValue, AnnotationElementValuePair,
    AnnotationInterfaceBody, AnnotationInterfaceBodyMember, AnnotationInterfaceDeclaration,
    ArgumentList, ArrayDimensions, Block, BlockStatement, ClassBody, ClassBodyDeclaration,
    ClassBodyMember, ClassDeclaration, CompactConstructorDeclaration, ConstructorBody,
    ConstructorDeclaration, ConstructorInvocation, DefaultValue, EmptyDeclaration, EnumBody,
    EnumConstant, EnumConstantList, EnumDeclaration, Expression, ExtendsClause, FieldDeclaration,
    FormalParameter, FormalParameterList, ImplementsClause, InstanceInitializer, InterfaceBody,
    InterfaceBodyMember, InterfaceDeclaration, JavaFamily, JavaNode, JavaSyntaxKind,
    JavaSyntaxToken, LocalVariableDeclaration, MethodDeclaration, ModifierList, NameSyntax,
    PermitsClause, ReceiverParameter, RecordBody, RecordComponent, RecordComponentList,
    RecordDeclaration, StaticInitializer, ThrowsClause, Type, TypeArgumentList, TypeBoundList,
    TypeParameter, TypeParameterList, VariableDeclarator, VariableDeclaratorList,
    VariableInitializer, VariableInitializerValue, child, child_family, child_token,
    child_token_in, children, children_family, children_tokens_matching, nth_child_token,
};
use super::helpers::{
    has_angle_comma_list_layout_shape, has_comma_list_layout_shape, has_comma_separated_elements,
    has_constructor_declaration_layout_shape, has_method_declaration_layout_shape,
    is_modifier_token,
};

pub enum FormalParameterModifier {
    Annotation(Annotation),
    Final(JavaSyntaxToken),
}

pub enum AnnotationElementListItem {
    Value(AnnotationElementValue),
    Pair(AnnotationElementValuePair),
}

impl ClassDeclaration {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::ClassKw)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn type_parameters(&self) -> Option<TypeParameterList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn extends_clause(&self) -> Option<ExtendsClause> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn implements_clause(&self) -> Option<ImplementsClause> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn permits_clause(&self) -> Option<PermitsClause> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<ClassBody> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let kinds = self
            .syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .collect::<Vec<_>>();
        let mut index = 0;
        if kinds.get(index) == Some(&JavaSyntaxKind::ModifierList) {
            index += 1;
        }
        if kinds.get(index) != Some(&JavaSyntaxKind::ClassKw) {
            return false;
        }
        index += 1;
        if kinds.get(index) != Some(&JavaSyntaxKind::Identifier) {
            return false;
        }
        index += 1;
        if kinds.get(index) == Some(&JavaSyntaxKind::TypeParameterList) {
            index += 1;
        }
        if kinds.get(index) == Some(&JavaSyntaxKind::ExtendsClause) {
            index += 1;
        }
        if kinds.get(index) == Some(&JavaSyntaxKind::ImplementsClause) {
            index += 1;
        }
        if kinds.get(index) == Some(&JavaSyntaxKind::PermitsClause) {
            index += 1;
        }

        kinds.get(index) == Some(&JavaSyntaxKind::ClassBody) && index + 1 == kinds.len()
    }
}

impl ExtendsClause {
    pub fn types(&self) -> impl Iterator<Item = Type> + '_ {
        children_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        self.has_single_type_layout_shape()
    }

    #[must_use]
    pub fn has_single_type_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        elements.len() == 2
            && elements[0].kind() == JavaSyntaxKind::ExtendsKw
            && Type::can_cast(elements[1].kind())
    }

    #[must_use]
    pub fn has_type_list_layout_shape(&self) -> bool {
        has_keyword_type_list_shape(&self.syntax, JavaSyntaxKind::ExtendsKw)
    }
}

impl ImplementsClause {
    pub fn types(&self) -> impl Iterator<Item = Type> + '_ {
        children_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        has_keyword_type_list_shape(&self.syntax, JavaSyntaxKind::ImplementsKw)
    }
}

impl PermitsClause {
    pub fn names(&self) -> impl Iterator<Item = NameSyntax> + '_ {
        children_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let Some(keyword) = elements
            .first()
            .and_then(|element| element.clone().into_token())
        else {
            return false;
        };
        keyword.kind() == JavaSyntaxKind::Identifier
            && keyword.text() == "permits"
            && has_comma_separated_elements(&elements[1..], NameSyntax::can_cast)
    }
}

fn has_keyword_type_list_shape(
    syntax: &super::super::JavaSyntaxNode,
    keyword: JavaSyntaxKind,
) -> bool {
    let elements = syntax.children_with_tokens().collect::<Vec<_>>();
    let Some(first) = elements.first() else {
        return false;
    };
    first.kind() == keyword && has_comma_separated_elements(&elements[1..], Type::can_cast)
}

impl RecordDeclaration {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        nth_child_token(&self.syntax, JavaSyntaxKind::Identifier, 0)
            .filter(|token| token.text() == "record")
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        nth_child_token(&self.syntax, JavaSyntaxKind::Identifier, 1)
    }

    #[must_use]
    pub fn type_parameters(&self) -> Option<TypeParameterList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn components(&self) -> Option<RecordComponentList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn implements_clause(&self) -> Option<ImplementsClause> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<RecordBody> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let kinds = self
            .syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .collect::<Vec<_>>();
        let mut index = 0;
        if kinds.get(index) == Some(&JavaSyntaxKind::ModifierList) {
            index += 1;
        }
        if kinds.get(index) != Some(&JavaSyntaxKind::Identifier) {
            return false;
        }
        index += 1;
        if kinds.get(index) != Some(&JavaSyntaxKind::Identifier) {
            return false;
        }
        index += 1;
        if kinds.get(index) == Some(&JavaSyntaxKind::TypeParameterList) {
            index += 1;
        }
        if kinds.get(index) != Some(&JavaSyntaxKind::LParen) {
            return false;
        }
        index += 1;
        if kinds.get(index) == Some(&JavaSyntaxKind::RecordComponentList) {
            index += 1;
        }
        if kinds.get(index) != Some(&JavaSyntaxKind::RParen) {
            return false;
        }
        index += 1;
        if kinds.get(index) == Some(&JavaSyntaxKind::ImplementsClause) {
            index += 1;
        }
        kinds.get(index) == Some(&JavaSyntaxKind::RecordBody) && index + 1 == kinds.len()
    }
}

impl EnumDeclaration {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::EnumKw)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn implements_clause(&self) -> Option<ImplementsClause> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<EnumBody> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let mut index = 0;
        if elements
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::ModifierList)
        {
            index += 1;
        }

        if elements
            .get(index)
            .is_none_or(|element| element.kind() != JavaSyntaxKind::EnumKw)
            || elements
                .get(index + 1)
                .is_none_or(|element| element.kind() != JavaSyntaxKind::Identifier)
        {
            return false;
        }
        index += 2;

        if elements
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::ImplementsClause)
        {
            index += 1;
        }

        elements
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::EnumBody)
            && index + 1 == elements.len()
    }
}

impl InterfaceDeclaration {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::InterfaceKw)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn type_parameters(&self) -> Option<TypeParameterList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn extends_clause(&self) -> Option<ExtendsClause> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn permits_clause(&self) -> Option<PermitsClause> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<InterfaceBody> {
        child(&self.syntax)
    }
}

impl AnnotationInterfaceDeclaration {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::InterfaceKw)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn body(&self) -> Option<AnnotationInterfaceBody> {
        child(&self.syntax)
    }
}

impl ModifierList {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        children(&self.syntax)
    }

    pub fn tokens(&self) -> impl Iterator<Item = JavaSyntaxToken> + '_ {
        self.syntax
            .children_with_tokens()
            .filter_map(jolt_syntax::SyntaxElement::into_token)
            .map(|syntax| JavaSyntaxToken { syntax })
    }

    pub fn modifier_tokens(&self) -> impl Iterator<Item = JavaSyntaxToken> + '_ {
        self.tokens().filter(|token| {
            is_modifier_token(token.kind())
                || token.kind() == JavaSyntaxKind::Identifier
                || token.kind() == JavaSyntaxKind::Minus
        })
    }
}

impl TypeParameterList {
    pub fn parameters(&self) -> impl Iterator<Item = TypeParameter> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        has_angle_comma_list_layout_shape(&self.syntax, JavaSyntaxKind::TypeParameter)
    }
}

impl TypeParameter {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn bounds(&self) -> Option<TypeBoundList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let kinds = self
            .syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .collect::<Vec<_>>();
        let mut index = 0;
        while kinds
            .get(index)
            .is_some_and(|kind| *kind == JavaSyntaxKind::Annotation)
        {
            index += 1;
        }
        if !kinds
            .get(index)
            .is_some_and(|kind| *kind == JavaSyntaxKind::Identifier)
        {
            return false;
        }
        index += 1;
        if kinds
            .get(index)
            .is_some_and(|kind| *kind == JavaSyntaxKind::TypeBoundList)
        {
            index += 1;
        }
        index == kinds.len()
    }
}

impl TypeBoundList {
    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [extends, ty]
                if extends.kind() == JavaSyntaxKind::ExtendsKw && Type::can_cast(ty.kind())
        )
    }
}

impl RecordComponentList {
    pub fn components(&self) -> impl Iterator<Item = RecordComponent> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        has_comma_list_layout_shape(&self.syntax, JavaSyntaxKind::RecordComponent)
    }
}

impl RecordComponent {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        self.syntax
            .children()
            .take_while(|node| node.kind() == JavaSyntaxKind::Annotation)
            .filter_map(Annotation::cast)
    }

    pub fn varargs_annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        let mut seen_type = false;
        self.syntax
            .children_with_tokens()
            .skip_while(move |element| {
                if seen_type {
                    return false;
                }
                seen_type = Type::can_cast(element.kind());
                true
            })
            .take_while(|element| element.kind() == JavaSyntaxKind::Annotation)
            .filter_map(jolt_syntax::SyntaxElement::into_node)
            .filter_map(Annotation::cast)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn dimensions(&self) -> Option<ArrayDimensions> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn ellipsis(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Ellipsis)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let kinds = self
            .syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .collect::<Vec<_>>();
        let mut index = 0;
        while kinds.get(index) == Some(&JavaSyntaxKind::Annotation) {
            index += 1;
        }
        if !kinds.get(index).is_some_and(|kind| Type::can_cast(*kind)) {
            return false;
        }
        index += 1;
        if kinds.get(index) == Some(&JavaSyntaxKind::Ellipsis) {
            index += 1;
        }
        if kinds.get(index) != Some(&JavaSyntaxKind::Identifier) {
            return false;
        }
        index += 1;
        if kinds.get(index) == Some(&JavaSyntaxKind::ArrayDimensions) {
            index += 1;
        }
        index == kinds.len()
    }
}

impl ClassBody {
    pub fn declarations(&self) -> impl Iterator<Item = ClassBodyDeclaration> + '_ {
        children(&self.syntax)
    }

    pub fn members(&self) -> impl Iterator<Item = ClassBodyMember> + '_ {
        self.syntax.children().filter_map(|node| {
            ClassBodyDeclaration::cast(node.clone())
                .and_then(|declaration| declaration.member())
                .or_else(|| EmptyDeclaration::cast(node).map(ClassBodyMember::EmptyDeclaration))
        })
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let kinds = self
            .syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .collect::<Vec<_>>();
        matches!(kinds.first(), Some(JavaSyntaxKind::LBrace))
            && matches!(kinds.last(), Some(JavaSyntaxKind::RBrace))
            && kinds[1..kinds.len().saturating_sub(1)].iter().all(|kind| {
                matches!(
                    kind,
                    JavaSyntaxKind::ClassBodyDeclaration | JavaSyntaxKind::EmptyDeclaration
                )
            })
    }
}

impl RecordBody {
    pub fn members(&self) -> impl Iterator<Item = ClassBodyMember> + '_ {
        self.syntax.children().filter_map(|node| {
            ClassBodyDeclaration::cast(node.clone())
                .and_then(|declaration| declaration.member())
                .or_else(|| EmptyDeclaration::cast(node).map(ClassBodyMember::EmptyDeclaration))
        })
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let kinds = self
            .syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .collect::<Vec<_>>();
        matches!(kinds.first(), Some(JavaSyntaxKind::LBrace))
            && matches!(kinds.last(), Some(JavaSyntaxKind::RBrace))
            && kinds[1..kinds.len().saturating_sub(1)]
                .iter()
                .all(|kind| *kind == JavaSyntaxKind::ClassBodyDeclaration)
    }
}

impl ClassBodyDeclaration {
    #[must_use]
    pub fn member(&self) -> Option<ClassBodyMember> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let kinds = self
            .syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .collect::<Vec<_>>();
        kinds.len() == 1
            && kinds
                .first()
                .is_some_and(|kind| ClassBodyMember::can_cast(*kind))
    }
}

impl InterfaceBody {
    pub fn members(&self) -> impl Iterator<Item = InterfaceBodyMember> + '_ {
        children_family(&self.syntax)
    }
}

impl AnnotationInterfaceBody {
    pub fn members(&self) -> impl Iterator<Item = AnnotationInterfaceBodyMember> {
        child::<AnnotationElementList>(&self.syntax)
            .map(|list| list.members().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
    }
}

impl AnnotationElementList {
    pub fn members(&self) -> impl Iterator<Item = AnnotationInterfaceBodyMember> + '_ {
        children_family(&self.syntax)
    }
}

impl EnumBody {
    #[must_use]
    pub fn constants(&self) -> Option<EnumConstantList> {
        child(&self.syntax)
    }

    pub fn members(&self) -> impl Iterator<Item = ClassBodyMember> + '_ {
        children::<ClassBodyDeclaration>(&self.syntax).filter_map(|node| node.member())
    }

    #[must_use]
    pub fn has_semicolon(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon).is_some()
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let Some(first) = elements.first() else {
            return false;
        };
        let Some(last) = elements.last() else {
            return false;
        };
        if first.kind() != JavaSyntaxKind::LBrace || last.kind() != JavaSyntaxKind::RBrace {
            return false;
        }

        let mut index = 1;
        if elements
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::EnumConstantList)
        {
            index += 1;
        }
        if elements
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::Semicolon)
        {
            index += 1;
        }

        elements[index..elements.len().saturating_sub(1)]
            .iter()
            .all(|element| {
                matches!(
                    element.kind(),
                    JavaSyntaxKind::ClassBodyDeclaration | JavaSyntaxKind::EmptyDeclaration
                )
            })
    }
}

impl EnumConstantList {
    pub fn constants(&self) -> impl Iterator<Item = EnumConstant> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn has_trailing_comma(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .last()
            .is_some_and(|element| element.kind() == JavaSyntaxKind::Comma)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        if elements.is_empty() {
            return false;
        }

        let mut expect_constant = true;
        let mut saw_constant = false;
        for element in elements {
            if expect_constant {
                if element.kind() != JavaSyntaxKind::EnumConstant {
                    return false;
                }
                saw_constant = true;
            } else if element.kind() != JavaSyntaxKind::Comma {
                return false;
            }
            expect_constant = !expect_constant;
        }

        saw_constant
    }
}

impl EnumConstant {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn arguments(&self) -> Option<ArgumentList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<ClassBody> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let mut index = 0;
        while elements
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::Annotation)
        {
            index += 1;
        }
        if elements
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::ModifierList)
        {
            index += 1;
        }

        if elements
            .get(index)
            .is_none_or(|element| element.kind() != JavaSyntaxKind::Identifier)
        {
            return false;
        }
        index += 1;

        if elements
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::ArgumentList)
        {
            index += 1;
        }
        if elements
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::ClassBody)
        {
            index += 1;
        }

        index == elements.len()
    }
}

impl FieldDeclaration {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn declarators(&self) -> Option<VariableDeclaratorList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let mut kinds = self
            .syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .collect::<Vec<_>>();
        if kinds.first() == Some(&JavaSyntaxKind::ModifierList) {
            kinds.remove(0);
        }
        matches!(
            kinds.as_slice(),
            [ty, JavaSyntaxKind::VariableDeclaratorList, JavaSyntaxKind::Semicolon]
                if Type::can_cast(*ty)
        )
    }
}

impl MethodDeclaration {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn type_parameters(&self) -> Option<TypeParameterList> {
        child(&self.syntax)
    }

    pub fn result_annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        let mut seen_type_parameters = false;
        self.syntax
            .children_with_tokens()
            .skip_while(move |element| {
                if element.kind() == JavaSyntaxKind::TypeParameterList {
                    seen_type_parameters = true;
                    return true;
                }
                !seen_type_parameters
                    && matches!(
                        element.kind(),
                        JavaSyntaxKind::ModifierList | JavaSyntaxKind::TypeParameterList
                    )
            })
            .take_while(|element| element.kind() == JavaSyntaxKind::Annotation)
            .filter_map(jolt_syntax::SyntaxElement::into_node)
            .filter_map(Annotation::cast)
    }

    #[must_use]
    pub fn return_type(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn parameters(&self) -> Option<FormalParameterList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn l_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn r_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    #[must_use]
    pub fn dimensions(&self) -> Option<ArrayDimensions> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn throws_clause(&self) -> Option<ThrowsClause> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Block> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_semicolon_body(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon).is_some()
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        has_method_declaration_layout_shape(&self.syntax)
    }
}

impl ConstructorDeclaration {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn type_parameters(&self) -> Option<TypeParameterList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn parameters(&self) -> Option<FormalParameterList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn l_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn r_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    #[must_use]
    pub fn throws_clause(&self) -> Option<ThrowsClause> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<ConstructorBody> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        has_constructor_declaration_layout_shape(&self.syntax)
    }
}

impl ConstructorBody {
    #[must_use]
    pub fn constructor_invocation(&self) -> Option<ConstructorInvocation> {
        child(&self.syntax)
    }

    pub fn block_statements(&self) -> impl Iterator<Item = BlockStatement> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn has_empty_layout_shape(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .eq([JavaSyntaxKind::LBrace, JavaSyntaxKind::RBrace])
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let Some(first) = elements.first() else {
            return false;
        };
        let Some(last) = elements.last() else {
            return false;
        };
        if first.kind() != JavaSyntaxKind::LBrace || last.kind() != JavaSyntaxKind::RBrace {
            return false;
        }

        let mut index = 1;
        if elements
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::ConstructorInvocation)
        {
            index += 1;
        }
        elements[index..elements.len() - 1]
            .iter()
            .all(|element| element.kind() == JavaSyntaxKind::BlockStatement)
    }
}

impl CompactConstructorDeclaration {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn body(&self) -> Option<ConstructorBody> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let kinds = self
            .syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .collect::<Vec<_>>();
        let mut index = 0;
        if kinds.get(index) == Some(&JavaSyntaxKind::ModifierList) {
            index += 1;
        }
        kinds.get(index) == Some(&JavaSyntaxKind::Identifier)
            && kinds.get(index + 1) == Some(&JavaSyntaxKind::ConstructorBody)
            && index + 2 == kinds.len()
    }
}

impl ConstructorInvocation {
    #[must_use]
    pub fn qualifier_expression(&self) -> Option<Expression> {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        if elements
            .get(1)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::Dot)
        {
            return elements
                .first()
                .and_then(|element| element.clone().into_node())
                .and_then(Expression::cast);
        }
        None
    }

    #[must_use]
    pub fn qualifier_name(&self) -> Option<NameSyntax> {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        if elements
            .get(1)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::Dot)
        {
            return elements
                .first()
                .and_then(|element| element.clone().into_node())
                .and_then(NameSyntax::cast);
        }
        None
    }

    #[must_use]
    pub fn type_arguments(&self) -> Option<TypeArgumentList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token_in(
            &self.syntax,
            &[JavaSyntaxKind::ThisKw, JavaSyntaxKind::SuperKw],
        )
    }

    #[must_use]
    pub fn arguments(&self) -> Option<ArgumentList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let mut index = 0;

        if elements
            .get(index + 1)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::Dot)
        {
            let Some(qualifier) = elements.get(index) else {
                return false;
            };
            if !(Expression::can_cast(qualifier.kind()) || qualifier.kind() == JavaSyntaxKind::Name)
            {
                return false;
            }
            index += 2;
        }

        if elements
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::TypeArgumentList)
        {
            index += 1;
        }

        matches!(
            elements.get(index).map(jolt_syntax::SyntaxElement::kind),
            Some(JavaSyntaxKind::ThisKw | JavaSyntaxKind::SuperKw)
        ) && elements
            .get(index + 1)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::ArgumentList)
            && elements
                .get(index + 2)
                .is_some_and(|element| element.kind() == JavaSyntaxKind::Semicolon)
            && index + 3 == elements.len()
    }
}

impl StaticInitializer {
    #[must_use]
    pub fn body(&self) -> Option<Block> {
        child(&self.syntax)
    }
}

impl InstanceInitializer {
    #[must_use]
    pub fn body(&self) -> Option<Block> {
        child(&self.syntax)
    }
}

impl FormalParameterList {
    #[must_use]
    pub fn receiver(&self) -> Option<ReceiverParameter> {
        child(&self.syntax)
    }

    pub fn parameters(&self) -> impl Iterator<Item = FormalParameter> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let mut expect_item = true;
        let mut saw_item = false;
        let mut allow_receiver = true;

        for element in self.syntax.children_with_tokens() {
            match (expect_item, element.kind()) {
                (true, JavaSyntaxKind::ReceiverParameter) if allow_receiver => {
                    expect_item = false;
                    saw_item = true;
                    allow_receiver = false;
                }
                (true, JavaSyntaxKind::FormalParameter) => {
                    expect_item = false;
                    saw_item = true;
                    allow_receiver = false;
                }
                (false, JavaSyntaxKind::Comma) => expect_item = true,
                _ => return false,
            }
        }

        saw_item && !expect_item
    }
}

impl ReceiverParameter {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        self.syntax
            .children_with_tokens()
            .take_while(|element| element.kind() == JavaSyntaxKind::Annotation)
            .filter_map(jolt_syntax::SyntaxElement::into_node)
            .filter_map(Annotation::cast)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn qualifier(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn this_token(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::ThisKw)
    }
}

impl FormalParameter {
    pub fn modifiers(&self) -> impl Iterator<Item = FormalParameterModifier> + '_ {
        self.syntax
            .children_with_tokens()
            .take_while(|element| {
                matches!(
                    element.kind(),
                    JavaSyntaxKind::Annotation | JavaSyntaxKind::FinalKw
                )
            })
            .filter_map(|element| match element.kind() {
                JavaSyntaxKind::Annotation => Some(FormalParameterModifier::Annotation(
                    Annotation::cast(element.into_node()?)?,
                )),
                JavaSyntaxKind::FinalKw => Some(FormalParameterModifier::Final(JavaSyntaxToken {
                    syntax: element.into_token()?,
                })),
                _ => None,
            })
    }

    pub fn varargs_annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        let mut seen_type = false;
        self.syntax
            .children_with_tokens()
            .skip_while(move |element| {
                if seen_type {
                    return false;
                }
                seen_type = Type::can_cast(element.kind());
                true
            })
            .take_while(|element| element.kind() == JavaSyntaxKind::Annotation)
            .filter_map(jolt_syntax::SyntaxElement::into_node)
            .filter_map(Annotation::cast)
    }

    #[must_use]
    pub fn ellipsis(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Ellipsis)
    }

    #[must_use]
    pub fn final_token(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::FinalKw)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn dimensions(&self) -> Option<ArrayDimensions> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn is_varargs(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::Ellipsis).is_some()
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let kinds = self
            .syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .collect::<Vec<_>>();
        let mut index = 0;
        while kinds.get(index).is_some_and(|kind| {
            matches!(kind, JavaSyntaxKind::Annotation | JavaSyntaxKind::FinalKw)
        }) {
            index += 1;
        }
        if !kinds.get(index).is_some_and(|kind| Type::can_cast(*kind)) {
            return false;
        }
        index += 1;
        while kinds
            .get(index)
            .is_some_and(|kind| *kind == JavaSyntaxKind::Annotation)
        {
            index += 1;
        }
        if kinds
            .get(index)
            .is_some_and(|kind| *kind == JavaSyntaxKind::Ellipsis)
        {
            index += 1;
        }
        if !kinds
            .get(index)
            .is_some_and(|kind| *kind == JavaSyntaxKind::Identifier)
        {
            return false;
        }
        index += 1;
        if kinds
            .get(index)
            .is_some_and(|kind| *kind == JavaSyntaxKind::ArrayDimensions)
        {
            index += 1;
        }
        index == kinds.len()
    }
}

impl ThrowsClause {
    pub fn types(&self) -> impl Iterator<Item = Type> + '_ {
        children_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let Some(first) = elements.first() else {
            return false;
        };
        if first.kind() != JavaSyntaxKind::ThrowsKw {
            return false;
        }

        has_comma_separated_elements(&elements[1..], Type::can_cast)
    }
}

impl VariableDeclaratorList {
    pub fn declarators(&self) -> impl Iterator<Item = VariableDeclarator> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn has_single_declarator_layout_shape(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .eq([JavaSyntaxKind::VariableDeclarator])
    }
}

impl VariableDeclarator {
    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token_in(
            &self.syntax,
            &[JavaSyntaxKind::Identifier, JavaSyntaxKind::UnderscoreKw],
        )
    }

    #[must_use]
    pub fn dimensions(&self) -> Option<ArrayDimensions> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn initializer(&self) -> Option<VariableInitializer> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let kinds = self
            .syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .collect::<Vec<_>>();
        let mut index = 0;
        if !kinds.get(index).is_some_and(|kind| {
            matches!(
                kind,
                JavaSyntaxKind::Identifier | JavaSyntaxKind::UnderscoreKw
            )
        }) {
            return false;
        }
        index += 1;
        if kinds
            .get(index)
            .is_some_and(|kind| *kind == JavaSyntaxKind::ArrayDimensions)
        {
            index += 1;
        }
        if kinds
            .get(index)
            .is_some_and(|kind| *kind == JavaSyntaxKind::Assign)
        {
            index += 1;
            if !kinds
                .get(index)
                .is_some_and(|kind| *kind == JavaSyntaxKind::VariableInitializer)
            {
                return false;
            }
            index += 1;
        }
        index == kinds.len()
    }
}

impl VariableInitializer {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn value(&self) -> Option<VariableInitializerValue> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn has_expression_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let [expression] = elements.as_slice() else {
            return false;
        };
        Expression::can_cast(expression.kind())
    }
}

impl LocalVariableDeclaration {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn final_token(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::FinalKw)
    }

    #[must_use]
    pub fn var_type_token(&self) -> Option<JavaSyntaxToken> {
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::Identifier)
            .find(|token| token.text() == "var")
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn declarators(&self) -> Option<VariableDeclaratorList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let kinds = self
            .syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .collect::<Vec<_>>();
        let mut index = 0;
        while kinds.get(index).is_some_and(|kind| {
            matches!(
                kind,
                JavaSyntaxKind::Annotation | JavaSyntaxKind::FinalKw | JavaSyntaxKind::ModifierList
            )
        }) {
            index += 1;
        }
        if !kinds
            .get(index)
            .is_some_and(|kind| Type::can_cast(*kind) || *kind == JavaSyntaxKind::Identifier)
        {
            return false;
        }
        index += 1;
        if !kinds
            .get(index)
            .is_some_and(|kind| *kind == JavaSyntaxKind::VariableDeclaratorList)
        {
            return false;
        }
        index + 1 == kinds.len()
    }
}
impl Annotation {
    #[must_use]
    pub fn name(&self) -> Option<NameSyntax> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn arguments(&self) -> Option<AnnotationArgumentList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [at, name]
                if at.kind() == JavaSyntaxKind::At && NameSyntax::can_cast(name.kind())
        ) || matches!(
            elements.as_slice(),
            [at, name, arguments]
                if at.kind() == JavaSyntaxKind::At
                    && NameSyntax::can_cast(name.kind())
                    && arguments.kind() == JavaSyntaxKind::AnnotationArgumentList
        )
    }
}

impl AnnotationArgumentList {
    #[must_use]
    pub fn elements(&self) -> Option<AnnotationElementList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [left, right]
                if left.kind() == JavaSyntaxKind::LParen && right.kind() == JavaSyntaxKind::RParen
        ) || matches!(
            elements.as_slice(),
            [left, list, right]
                if left.kind() == JavaSyntaxKind::LParen
                    && list.kind() == JavaSyntaxKind::AnnotationElementList
                    && right.kind() == JavaSyntaxKind::RParen
        )
    }
}

impl AnnotationElementList {
    pub fn items(&self) -> impl Iterator<Item = AnnotationElementListItem> + '_ {
        self.syntax.children().filter_map(|node| {
            AnnotationElementValue::cast(node.clone())
                .map(AnnotationElementListItem::Value)
                .or_else(|| {
                    AnnotationElementValuePair::cast(node).map(AnnotationElementListItem::Pair)
                })
        })
    }

    pub fn values(&self) -> impl Iterator<Item = AnnotationElementValue> + '_ {
        children(&self.syntax)
    }

    pub fn pairs(&self) -> impl Iterator<Item = AnnotationElementValuePair> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn has_value_list_layout_shape(&self) -> bool {
        has_comma_list_layout_shape(&self.syntax, JavaSyntaxKind::AnnotationElementValue)
    }

    #[must_use]
    pub fn has_pair_list_layout_shape(&self) -> bool {
        has_comma_list_layout_shape(&self.syntax, JavaSyntaxKind::AnnotationElementValuePair)
    }
}

impl AnnotationElementDeclaration {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn dimensions(&self) -> Option<ArrayDimensions> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn default_value(&self) -> Option<DefaultValue> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let mut index = usize::from(
            elements
                .first()
                .is_some_and(|element| element.kind() == JavaSyntaxKind::ModifierList),
        );
        if !elements
            .get(index)
            .is_some_and(|element| Type::can_cast(element.kind()))
        {
            return false;
        }
        index += 1;
        if !elements
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::Identifier)
        {
            return false;
        }
        index += 1;
        if !matches!(
            (elements.get(index), elements.get(index + 1)),
            (Some(left), Some(right))
                if left.kind() == JavaSyntaxKind::LParen
                    && right.kind() == JavaSyntaxKind::RParen
        ) {
            return false;
        }
        index += 2;
        if elements
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::ArrayDimensions)
        {
            index += 1;
        }
        if elements
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::DefaultValue)
        {
            index += 1;
        }
        matches!(elements.get(index), Some(element) if element.kind() == JavaSyntaxKind::Semicolon)
            && index + 1 == elements.len()
    }
}

impl DefaultValue {
    #[must_use]
    pub fn value(&self) -> Option<AnnotationElementValue> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [default_kw, value]
                if default_kw.kind() == JavaSyntaxKind::DefaultKw
                    && value.kind() == JavaSyntaxKind::AnnotationElementValue
        )
    }
}

impl AnnotationElementValue {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn annotation(&self) -> Option<Annotation> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn array_initializer(&self) -> Option<AnnotationArrayInitializer> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_expression_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let [expression] = elements.as_slice() else {
            return false;
        };
        Expression::can_cast(expression.kind())
    }
}

impl AnnotationArrayInitializer {
    pub fn values(&self) -> impl Iterator<Item = AnnotationElementValue> + '_ {
        children(&self.syntax)
    }
}

impl AnnotationElementValuePair {
    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn value(&self) -> Option<AnnotationElementValue> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [name, assign, value]
                if name.kind() == JavaSyntaxKind::Identifier
                    && assign.kind() == JavaSyntaxKind::Assign
                    && value.kind() == JavaSyntaxKind::AnnotationElementValue
        )
    }
}
