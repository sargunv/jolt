use super::super::{
    Annotation, AnnotationArgumentList, AnnotationElementList, AnnotationElementValue,
    AnnotationElementValuePair, AnnotationInterfaceBody, AnnotationInterfaceBodyMember,
    AnnotationInterfaceDeclaration, ArgumentList, ArrayDimensions, Block, BlockStatement,
    ClassBody, ClassBodyDeclaration, ClassBodyMember, ClassDeclaration, ConstructorBody,
    ConstructorDeclaration, EmptyDeclaration, EnumBody, EnumConstant, EnumConstantList,
    EnumDeclaration, Expression, ExtendsClause, FieldDeclaration, FormalParameter,
    FormalParameterList, ImplementsClause, InstanceInitializer, InterfaceBody, InterfaceBodyMember,
    InterfaceDeclaration, JavaNode, JavaSyntaxKind, JavaSyntaxToken, LocalVariableDeclaration,
    MethodDeclaration, ModifierList, NameSyntax, PermitsClause, RecordBody, RecordComponent,
    RecordComponentList, RecordDeclaration, StaticInitializer, ThrowsClause, Type, TypeParameter,
    TypeParameterList, VariableDeclarator, VariableDeclaratorList, VariableInitializer,
    VariableInitializerValue, child, child_family, child_token, child_token_in, children,
    children_family, children_tokens_matching, nth_child_token,
};
use super::helpers::{
    has_angle_comma_list_layout_shape, has_braced_block_statement_layout_shape,
    has_comma_list_layout_shape, has_comma_separated_elements,
    has_constructor_declaration_layout_shape, has_method_declaration_layout_shape,
    is_modifier_token,
};

impl ClassDeclaration {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList> {
        child(&self.syntax)
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
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        elements.len() == 2
            && elements[0].kind() == JavaSyntaxKind::ExtendsKw
            && Type::can_cast(elements[1].kind())
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
}

impl EnumDeclaration {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList> {
        child(&self.syntax)
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
}

impl InterfaceDeclaration {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList> {
        child(&self.syntax)
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
        children_tokens_matching(&self.syntax, is_modifier_token)
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
    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn has_simple_layout_shape(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .eq([JavaSyntaxKind::Identifier])
    }
}

impl RecordComponentList {
    pub fn components(&self) -> impl Iterator<Item = RecordComponent> + '_ {
        children(&self.syntax)
    }
}

impl RecordComponent {
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
        children::<ClassBodyDeclaration>(&self.syntax).filter_map(|node| node.member())
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
}

impl EnumConstantList {
    pub fn constants(&self) -> impl Iterator<Item = EnumConstant> + '_ {
        children(&self.syntax)
    }
}

impl EnumConstant {
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
            [
                JavaSyntaxKind::PrimitiveType | JavaSyntaxKind::ClassType,
                JavaSyntaxKind::VariableDeclaratorList,
                JavaSyntaxKind::Semicolon,
            ]
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
        has_braced_block_statement_layout_shape(&self.syntax)
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
    pub fn parameters(&self) -> impl Iterator<Item = FormalParameter> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        has_comma_list_layout_shape(&self.syntax, JavaSyntaxKind::FormalParameter)
    }
}

impl FormalParameter {
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
        let mut kinds = self
            .syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .collect::<Vec<_>>();
        if kinds.first() == Some(&JavaSyntaxKind::FinalKw) {
            kinds.remove(0);
        }
        matches!(
            kinds.as_slice(),
            [
                JavaSyntaxKind::PrimitiveType | JavaSyntaxKind::ClassType,
                JavaSyntaxKind::Identifier,
            ] | [
                JavaSyntaxKind::PrimitiveType | JavaSyntaxKind::ClassType,
                JavaSyntaxKind::Ellipsis,
                JavaSyntaxKind::Identifier,
            ]
        )
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
    pub fn has_identifier_layout_shape(&self) -> bool {
        let kinds = self
            .syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .collect::<Vec<_>>();
        matches!(
            kinds.as_slice(),
            [JavaSyntaxKind::Identifier]
                | [
                    JavaSyntaxKind::Identifier,
                    JavaSyntaxKind::Assign,
                    JavaSyntaxKind::VariableInitializer,
                ]
        )
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
        let mut kinds = self
            .syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .collect::<Vec<_>>();
        if kinds.first() == Some(&JavaSyntaxKind::FinalKw) {
            kinds.remove(0);
        }
        matches!(
            kinds.as_slice(),
            [
                JavaSyntaxKind::PrimitiveType
                    | JavaSyntaxKind::ClassType
                    | JavaSyntaxKind::Identifier,
                JavaSyntaxKind::VariableDeclaratorList,
            ]
        )
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

impl AnnotationElementValue {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
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
