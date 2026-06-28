use super::{
    Annotation, AnnotationArgumentList, AnnotationElementList, AnnotationInterfaceBody,
    AnnotationInterfaceBodyMember, AnnotationInterfaceDeclaration, AnyJavaNode, ArgumentList,
    ArrayCreationExpression, ArrayDimensions, ArrayInitializer, ArrayType, AssignmentExpression,
    BasicForStatement, BinaryExpression, Block, BlockItem, BlockStatement, CastExpression,
    ClassBody, ClassBodyDeclaration, ClassBodyMember, ClassDeclaration, ClassType, CompilationUnit,
    ConditionalExpression, ConstructorBody, ConstructorDeclaration, DimExpression, DoStatement,
    EmptyDeclaration, EnhancedForStatement, EnumBody, EnumConstant, EnumConstantList,
    EnumDeclaration, Expression, ExpressionStatement, ExtendsClause, FieldAccessExpression,
    FieldDeclaration, ForInitializer, ForStatement, ForUpdate, FormalParameter,
    FormalParameterList, IfStatement, ImplementsClause, ImportDeclaration, InstanceInitializer,
    InterfaceBody, InterfaceBodyMember, InterfaceDeclaration, JavaNode, JavaSyntaxKind,
    JavaSyntaxToken, LambdaExpression, LambdaParameter, LambdaParameterList, LiteralExpression,
    LocalVariableDeclaration, MethodDeclaration, MethodInvocationExpression, ModifierList,
    ModuleDeclaration, ModuleDirective, ModuleDirectiveNode, NameExpression, NameSyntax,
    ObjectCreationExpression, PackageDeclaration, ParenthesizedExpression, PermitsClause,
    PostfixExpression, RecordBody, RecordComponent, RecordComponentList, RecordDeclaration,
    ReturnStatement, Statement, StatementExpressionList, StaticInitializer, SuperExpression,
    SwitchBlock, SwitchBlockStatementGroup, SwitchExpression, SwitchRule, SwitchStatement,
    SynchronizedStatement, ThisExpression, ThrowStatement, ThrowsClause, Type, TypeDeclaration,
    TypeParameter, TypeParameterList, UnaryExpression, VariableDeclarator, VariableDeclaratorList,
    VariableInitializer, VariableInitializerValue, WhileStatement, YieldStatement, child,
    child_family, child_token, child_token_in, children, children_family, children_tokens_matching,
    nth_child_family, nth_child_token,
};

impl CompilationUnit {
    #[must_use]
    pub fn package_declaration(&self) -> Option<PackageDeclaration> {
        child(&self.syntax)
    }

    pub fn imports(&self) -> impl Iterator<Item = ImportDeclaration> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn module_declaration(&self) -> Option<ModuleDeclaration> {
        child(&self.syntax)
    }

    pub fn type_declarations(&self) -> impl Iterator<Item = TypeDeclaration> + '_ {
        children_family(&self.syntax)
    }

    pub fn unsupported_layout_child(&self) -> Option<AnyJavaNode> {
        self.syntax
            .children()
            .filter_map(AnyJavaNode::cast)
            .find(|node| {
                !matches!(
                    node.kind(),
                    JavaSyntaxKind::PackageDeclaration
                        | JavaSyntaxKind::ImportDeclaration
                        | JavaSyntaxKind::ModuleDeclaration
                        | JavaSyntaxKind::ClassDeclaration
                        | JavaSyntaxKind::RecordDeclaration
                        | JavaSyntaxKind::EnumDeclaration
                        | JavaSyntaxKind::InterfaceDeclaration
                        | JavaSyntaxKind::AnnotationInterfaceDeclaration
                )
            })
    }

    /// Returns descendant nodes as typed Java wrappers.
    ///
    /// Prefer grammar-specific accessors for formatter layout. This traversal is
    /// intended for corpus summaries, diagnostics, and generic syntax tooling.
    pub fn descendants(&self) -> impl Iterator<Item = AnyJavaNode> + '_ {
        self.syntax.descendants().filter_map(AnyJavaNode::cast)
    }

    /// Returns this compilation unit and its descendants as typed Java wrappers.
    ///
    /// Prefer grammar-specific accessors for formatter layout. This traversal is
    /// intended for corpus summaries, diagnostics, and generic syntax tooling.
    pub fn self_and_descendants(&self) -> impl Iterator<Item = AnyJavaNode> + '_ {
        std::iter::once(AnyJavaNode::from(self.clone())).chain(self.descendants())
    }
}

impl ImportDeclaration {
    #[must_use]
    pub fn is_static(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::StaticKw).is_some()
    }

    #[must_use]
    pub fn is_module(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .filter_map(jolt_syntax::SyntaxElement::into_token)
            .nth(1)
            .is_some_and(|token| {
                token.kind() == JavaSyntaxKind::Identifier && token.text() == "module"
            })
    }

    #[must_use]
    pub fn is_on_demand(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::Star).is_some()
    }

    #[must_use]
    pub fn name(&self) -> Option<NameSyntax> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let mut elements = self.syntax.children_with_tokens();
        let Some(import_kw) = elements
            .next()
            .and_then(jolt_syntax::SyntaxElement::into_token)
        else {
            return false;
        };
        if import_kw.kind() != JavaSyntaxKind::ImportKw {
            return false;
        }

        let mut next = elements.next();
        if self.is_module() {
            let Some(module) = next.and_then(jolt_syntax::SyntaxElement::into_token) else {
                return false;
            };
            if module.kind() != JavaSyntaxKind::Identifier || module.text() != "module" {
                return false;
            }
            next = elements.next();
        } else if self.is_static() {
            let Some(static_kw) = next.and_then(jolt_syntax::SyntaxElement::into_token) else {
                return false;
            };
            if static_kw.kind() != JavaSyntaxKind::StaticKw {
                return false;
            }
            next = elements.next();
        }

        let Some(name) = next.and_then(jolt_syntax::SyntaxElement::into_node) else {
            return false;
        };
        if !NameSyntax::can_cast(name.kind()) {
            return false;
        }

        next = elements.next();
        if self.is_on_demand() {
            let Some(dot) = next.and_then(jolt_syntax::SyntaxElement::into_token) else {
                return false;
            };
            if dot.kind() != JavaSyntaxKind::Dot {
                return false;
            }
            let Some(star) = elements
                .next()
                .and_then(jolt_syntax::SyntaxElement::into_token)
            else {
                return false;
            };
            if star.kind() != JavaSyntaxKind::Star {
                return false;
            }
            next = elements.next();
        }

        let Some(semicolon) = next.and_then(jolt_syntax::SyntaxElement::into_token) else {
            return false;
        };
        semicolon.kind() == JavaSyntaxKind::Semicolon && elements.next().is_none()
    }
}

impl PackageDeclaration {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<NameSyntax> {
        child_family(&self.syntax)
    }
}

impl NameSyntax {
    pub fn segments(&self) -> impl Iterator<Item = JavaSyntaxToken> + '_ {
        children_tokens_matching(self.syntax(), |kind| kind == JavaSyntaxKind::Identifier)
    }
}

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
        let mut kinds = self
            .syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .collect::<Vec<_>>();
        if kinds.first() == Some(&JavaSyntaxKind::ModifierList) {
            kinds.remove(0);
        }
        kinds
            == [
                JavaSyntaxKind::ClassKw,
                JavaSyntaxKind::Identifier,
                JavaSyntaxKind::ClassBody,
            ]
    }
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
                JavaSyntaxKind::PrimitiveType
                    | JavaSyntaxKind::VoidType
                    | JavaSyntaxKind::ClassType,
                JavaSyntaxKind::Identifier,
                JavaSyntaxKind::LParen,
                JavaSyntaxKind::RParen,
                JavaSyntaxKind::Block,
            ]
        )
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
                JavaSyntaxKind::Identifier,
                JavaSyntaxKind::LParen,
                JavaSyntaxKind::RParen,
                JavaSyntaxKind::ConstructorBody,
            ]
        )
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
}

impl FormalParameter {
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

impl IfStatement {
    #[must_use]
    pub fn condition(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn then_statement(&self) -> Option<Statement> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn else_statement(&self) -> Option<Statement> {
        nth_child_family(&self.syntax, 1)
    }
}

impl MethodInvocationExpression {
    #[must_use]
    pub fn receiver(&self) -> Option<Expression> {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let [receiver, dot, _, _] = elements.as_slice() else {
            return None;
        };
        if !Expression::can_cast(receiver.kind()) || dot.kind() != JavaSyntaxKind::Dot {
            return None;
        }
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        self.syntax
            .children_with_tokens()
            .filter_map(jolt_syntax::SyntaxElement::into_token)
            .find(|token| token.kind() == JavaSyntaxKind::Identifier)
            .map(|syntax| JavaSyntaxToken { syntax })
    }

    #[must_use]
    pub fn simple_name(&self) -> Option<JavaSyntaxToken> {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let [callee, arguments] = elements.as_slice() else {
            return None;
        };
        if callee.kind() != JavaSyntaxKind::NameExpression
            || arguments.kind() != JavaSyntaxKind::ArgumentList
        {
            return None;
        }
        child::<NameExpression>(&self.syntax)?.identifier()
    }

    #[must_use]
    pub fn arguments(&self) -> Option<ArgumentList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        match elements.as_slice() {
            [callee, arguments] => {
                callee.kind() == JavaSyntaxKind::NameExpression
                    && arguments.kind() == JavaSyntaxKind::ArgumentList
            }
            [receiver, dot, name, arguments] => {
                Expression::can_cast(receiver.kind())
                    && dot.kind() == JavaSyntaxKind::Dot
                    && name.kind() == JavaSyntaxKind::Identifier
                    && arguments.kind() == JavaSyntaxKind::ArgumentList
            }
            _ => false,
        }
    }
}

impl ArgumentList {
    pub fn arguments(&self) -> impl Iterator<Item = Expression> + '_ {
        children_family(&self.syntax)
    }

    #[must_use]
    pub fn has_empty_layout_shape(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .eq([JavaSyntaxKind::LParen, JavaSyntaxKind::RParen])
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
        if first.kind() != JavaSyntaxKind::LParen || last.kind() != JavaSyntaxKind::RParen {
            return false;
        }

        let inner = &elements[1..elements.len().saturating_sub(1)];
        if inner.is_empty() {
            return true;
        }
        if inner.len() % 2 == 0 {
            return false;
        }
        inner.iter().enumerate().all(|(index, element)| {
            if index % 2 == 0 {
                Expression::can_cast(element.kind())
            } else {
                element.kind() == JavaSyntaxKind::Comma
            }
        })
    }
}

impl LiteralExpression {
    #[must_use]
    pub fn token(&self) -> Option<JavaSyntaxToken> {
        let tokens = self
            .syntax
            .children_with_tokens()
            .map(jolt_syntax::SyntaxElement::into_token)
            .collect::<Option<Vec<_>>>()?;
        let [token] = tokens.as_slice() else {
            return None;
        };
        is_literal_token(token.kind()).then(|| JavaSyntaxToken {
            syntax: token.clone(),
        })
    }

    #[must_use]
    pub fn has_simple_layout_shape(&self) -> bool {
        self.token().is_some()
    }
}

impl NameExpression {
    #[must_use]
    pub fn identifier(&self) -> Option<JavaSyntaxToken> {
        let tokens = self
            .syntax
            .children_with_tokens()
            .map(jolt_syntax::SyntaxElement::into_token)
            .collect::<Option<Vec<_>>>()?;
        let [token] = tokens.as_slice() else {
            return None;
        };
        (token.kind() == JavaSyntaxKind::Identifier).then(|| JavaSyntaxToken {
            syntax: token.clone(),
        })
    }

    #[must_use]
    pub fn has_simple_layout_shape(&self) -> bool {
        self.identifier().is_some()
    }
}

impl ThisExpression {
    #[must_use]
    pub fn token(&self) -> Option<JavaSyntaxToken> {
        simple_keyword_token(&self.syntax, JavaSyntaxKind::ThisKw)
    }

    #[must_use]
    pub fn has_simple_layout_shape(&self) -> bool {
        self.token().is_some()
    }
}

impl SuperExpression {
    #[must_use]
    pub fn token(&self) -> Option<JavaSyntaxToken> {
        simple_keyword_token(&self.syntax, JavaSyntaxKind::SuperKw)
    }

    #[must_use]
    pub fn has_simple_layout_shape(&self) -> bool {
        self.token().is_some()
    }
}

impl FieldAccessExpression {
    #[must_use]
    pub fn receiver(&self) -> Option<Expression> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [receiver, dot, name]
                if Expression::can_cast(receiver.kind())
                    && dot.kind() == JavaSyntaxKind::Dot
                    && name.kind() == JavaSyntaxKind::Identifier
        )
    }
}

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

impl Annotation {
    #[must_use]
    pub fn arguments(&self) -> Option<AnnotationArgumentList> {
        child(&self.syntax)
    }
}

impl ParenthesizedExpression {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [left, expression, right]
                if left.kind() == JavaSyntaxKind::LParen
                    && Expression::can_cast(expression.kind())
                    && right.kind() == JavaSyntaxKind::RParen
        )
    }
}

impl AssignmentExpression {
    #[must_use]
    pub fn left(&self) -> Option<Expression> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn operator(&self) -> Option<JavaSyntaxToken> {
        child_token_in(&self.syntax, ASSIGNMENT_OPERATORS)
    }

    #[must_use]
    pub fn right(&self) -> Option<Expression> {
        nth_child_family(&self.syntax, 1)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [left, operator, right]
                if Expression::can_cast(left.kind())
                    && ASSIGNMENT_OPERATORS.contains(&operator.kind())
                    && Expression::can_cast(right.kind())
        )
    }
}

impl ConditionalExpression {
    #[must_use]
    pub fn condition(&self) -> Option<Expression> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn true_expression(&self) -> Option<Expression> {
        nth_child_family(&self.syntax, 1)
    }

    #[must_use]
    pub fn false_expression(&self) -> Option<Expression> {
        nth_child_family(&self.syntax, 2)
    }
}

impl BinaryExpression {
    #[must_use]
    pub fn left(&self) -> Option<Expression> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn operator(&self) -> Option<JavaSyntaxToken> {
        child_token_in(&self.syntax, BINARY_OPERATORS)
    }

    #[must_use]
    pub fn right(&self) -> Option<Expression> {
        nth_child_family(&self.syntax, 1)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [left, operator, right]
                if Expression::can_cast(left.kind())
                    && BINARY_OPERATORS.contains(&operator.kind())
                    && Expression::can_cast(right.kind())
        )
    }
}

impl UnaryExpression {
    #[must_use]
    pub fn operator(&self) -> Option<JavaSyntaxToken> {
        child_token_in(
            &self.syntax,
            &[
                JavaSyntaxKind::PlusPlus,
                JavaSyntaxKind::MinusMinus,
                JavaSyntaxKind::Plus,
                JavaSyntaxKind::Minus,
                JavaSyntaxKind::Bang,
                JavaSyntaxKind::Tilde,
            ],
        )
    }

    #[must_use]
    pub fn operand(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [operator, operand]
                if [
                    JavaSyntaxKind::PlusPlus,
                    JavaSyntaxKind::MinusMinus,
                    JavaSyntaxKind::Plus,
                    JavaSyntaxKind::Minus,
                    JavaSyntaxKind::Bang,
                    JavaSyntaxKind::Tilde,
                ]
                .contains(&operator.kind())
                    && Expression::can_cast(operand.kind())
        )
    }
}

impl PostfixExpression {
    #[must_use]
    pub fn operand(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn operator(&self) -> Option<JavaSyntaxToken> {
        child_token_in(
            &self.syntax,
            &[JavaSyntaxKind::PlusPlus, JavaSyntaxKind::MinusMinus],
        )
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [operand, operator]
                if Expression::can_cast(operand.kind())
                    && [JavaSyntaxKind::PlusPlus, JavaSyntaxKind::MinusMinus]
                        .contains(&operator.kind())
        )
    }
}

impl CastExpression {
    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }
}

impl ObjectCreationExpression {
    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
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

impl ArrayCreationExpression {
    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    pub fn dimensions(&self) -> impl Iterator<Item = DimExpression> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn initializer(&self) -> Option<ArrayInitializer> {
        child(&self.syntax)
    }
}

impl LambdaExpression {
    #[must_use]
    pub fn parameters(&self) -> Option<LambdaParameterList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn expression_body(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn block_body(&self) -> Option<Block> {
        child(&self.syntax)
    }
}

impl LambdaParameterList {
    pub fn parameters(&self) -> impl Iterator<Item = LambdaParameter> + '_ {
        children(&self.syntax)
    }
}

impl LambdaParameter {
    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token_in(
            &self.syntax,
            &[JavaSyntaxKind::Identifier, JavaSyntaxKind::UnderscoreKw],
        )
    }
}

impl ExpressionStatement {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [expression, semicolon]
                if match expression.kind() {
                    JavaSyntaxKind::AssignmentExpression
                        | JavaSyntaxKind::MethodInvocationExpression
                        | JavaSyntaxKind::PostfixExpression => true,
                    JavaSyntaxKind::UnaryExpression => child::<UnaryExpression>(&self.syntax)
                        .and_then(|unary| unary.operator())
                        .is_some_and(|operator| {
                            matches!(
                                operator.kind(),
                                JavaSyntaxKind::PlusPlus | JavaSyntaxKind::MinusMinus
                            )
                        }),
                    _ => false,
                } && semicolon.kind() == JavaSyntaxKind::Semicolon
        )
    }
}

impl ReturnStatement {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        has_keyword_optional_expression_semicolon_shape(
            &self.syntax,
            JavaSyntaxKind::ReturnKw,
            None,
        )
    }
}

impl ThrowStatement {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        has_keyword_required_expression_semicolon_shape(&self.syntax, JavaSyntaxKind::ThrowKw, None)
    }
}

impl YieldStatement {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        has_keyword_required_expression_semicolon_shape(
            &self.syntax,
            JavaSyntaxKind::Identifier,
            Some("yield"),
        )
    }
}

impl WhileStatement {
    #[must_use]
    pub fn condition(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Statement> {
        child_family(&self.syntax)
    }
}

impl DoStatement {
    #[must_use]
    pub fn body(&self) -> Option<Statement> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn condition(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }
}

impl SynchronizedStatement {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Block> {
        child(&self.syntax)
    }
}

impl ForStatement {
    #[must_use]
    pub fn basic(&self) -> Option<BasicForStatement> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn enhanced(&self) -> Option<EnhancedForStatement> {
        child(&self.syntax)
    }
}

impl BasicForStatement {
    #[must_use]
    pub fn initializer(&self) -> Option<ForInitializer> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn condition(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn update(&self) -> Option<ForUpdate> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Statement> {
        child_family(&self.syntax)
    }
}

impl EnhancedForStatement {
    #[must_use]
    pub fn variable(&self) -> Option<LocalVariableDeclaration> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn iterable(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Statement> {
        child_family(&self.syntax)
    }
}

impl ForInitializer {
    #[must_use]
    pub fn local_variable_declaration(&self) -> Option<LocalVariableDeclaration> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn expressions(&self) -> Option<StatementExpressionList> {
        child(&self.syntax)
    }
}

impl ForUpdate {
    #[must_use]
    pub fn expressions(&self) -> Option<StatementExpressionList> {
        child(&self.syntax)
    }
}

impl StatementExpressionList {
    pub fn expressions(&self) -> impl Iterator<Item = Expression> + '_ {
        children_family(&self.syntax)
    }
}

impl SwitchStatement {
    #[must_use]
    pub fn selector(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn block(&self) -> Option<SwitchBlock> {
        child(&self.syntax)
    }
}

impl SwitchExpression {
    #[must_use]
    pub fn selector(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn block(&self) -> Option<SwitchBlock> {
        child(&self.syntax)
    }
}

impl SwitchBlock {
    pub fn statement_groups(&self) -> impl Iterator<Item = SwitchBlockStatementGroup> + '_ {
        children(&self.syntax)
    }

    pub fn rules(&self) -> impl Iterator<Item = SwitchRule> + '_ {
        children(&self.syntax)
    }
}

fn is_modifier_token(kind: JavaSyntaxKind) -> bool {
    matches!(
        kind,
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
    )
}

const ASSIGNMENT_OPERATORS: &[JavaSyntaxKind] = &[
    JavaSyntaxKind::Assign,
    JavaSyntaxKind::PlusEq,
    JavaSyntaxKind::MinusEq,
    JavaSyntaxKind::StarEq,
    JavaSyntaxKind::SlashEq,
    JavaSyntaxKind::AmpEq,
    JavaSyntaxKind::BarEq,
    JavaSyntaxKind::CaretEq,
    JavaSyntaxKind::PercentEq,
    JavaSyntaxKind::LShiftEq,
    JavaSyntaxKind::RShiftEq,
    JavaSyntaxKind::UnsignedRShiftEq,
];

const BINARY_OPERATORS: &[JavaSyntaxKind] = &[
    JavaSyntaxKind::InstanceofKw,
    JavaSyntaxKind::OrOr,
    JavaSyntaxKind::AndAnd,
    JavaSyntaxKind::Bar,
    JavaSyntaxKind::Caret,
    JavaSyntaxKind::Amp,
    JavaSyntaxKind::EqEq,
    JavaSyntaxKind::BangEq,
    JavaSyntaxKind::Lt,
    JavaSyntaxKind::Gt,
    JavaSyntaxKind::LtEq,
    JavaSyntaxKind::GtEq,
    JavaSyntaxKind::LShift,
    JavaSyntaxKind::RShift,
    JavaSyntaxKind::UnsignedRShift,
    JavaSyntaxKind::Plus,
    JavaSyntaxKind::Minus,
    JavaSyntaxKind::Star,
    JavaSyntaxKind::Slash,
    JavaSyntaxKind::Percent,
];

fn simple_single_token(syntax: &super::JavaSyntaxNode) -> Option<Vec<JavaSyntaxToken>> {
    let tokens = syntax
        .children_with_tokens()
        .map(jolt_syntax::SyntaxElement::into_token)
        .collect::<Option<Vec<_>>>()?;
    let [token] = tokens.as_slice() else {
        return None;
    };
    Some(vec![JavaSyntaxToken {
        syntax: token.clone(),
    }])
}

fn simple_keyword_token(
    syntax: &super::JavaSyntaxNode,
    expected: JavaSyntaxKind,
) -> Option<JavaSyntaxToken> {
    let tokens = syntax
        .children_with_tokens()
        .map(jolt_syntax::SyntaxElement::into_token)
        .collect::<Option<Vec<_>>>()?;
    let [token] = tokens.as_slice() else {
        return None;
    };
    (token.kind() == expected).then(|| JavaSyntaxToken {
        syntax: token.clone(),
    })
}

fn has_keyword_optional_expression_semicolon_shape(
    syntax: &super::JavaSyntaxNode,
    keyword_kind: JavaSyntaxKind,
    keyword_text: Option<&str>,
) -> bool {
    let elements = syntax.children_with_tokens().collect::<Vec<_>>();
    let [keyword, semicolon] = elements.as_slice() else {
        let [keyword, expression, semicolon] = elements.as_slice() else {
            return false;
        };
        return keyword_matches(keyword, keyword_kind, keyword_text)
            && Expression::can_cast(expression.kind())
            && semicolon.kind() == JavaSyntaxKind::Semicolon;
    };

    keyword_matches(keyword, keyword_kind, keyword_text)
        && semicolon.kind() == JavaSyntaxKind::Semicolon
}

fn has_keyword_required_expression_semicolon_shape(
    syntax: &super::JavaSyntaxNode,
    keyword_kind: JavaSyntaxKind,
    keyword_text: Option<&str>,
) -> bool {
    let elements = syntax.children_with_tokens().collect::<Vec<_>>();
    let [keyword, expression, semicolon] = elements.as_slice() else {
        return false;
    };

    keyword_matches(keyword, keyword_kind, keyword_text)
        && Expression::can_cast(expression.kind())
        && semicolon.kind() == JavaSyntaxKind::Semicolon
}

fn keyword_matches(
    element: &jolt_syntax::SyntaxElement<crate::language::JavaLanguage>,
    expected: JavaSyntaxKind,
    expected_text: Option<&str>,
) -> bool {
    let Some(token) = element.clone().into_token() else {
        return false;
    };
    token.kind() == expected && expected_text.is_none_or(|text| token.text() == text)
}

fn has_braced_block_statement_layout_shape(syntax: &super::JavaSyntaxNode) -> bool {
    let kinds = syntax
        .children_with_tokens()
        .map(|element| element.kind())
        .collect::<Vec<_>>();
    matches!(kinds.first(), Some(JavaSyntaxKind::LBrace))
        && matches!(kinds.last(), Some(JavaSyntaxKind::RBrace))
        && kinds[1..kinds.len().saturating_sub(1)]
            .iter()
            .all(|kind| *kind == JavaSyntaxKind::BlockStatement)
}

fn is_literal_token(kind: JavaSyntaxKind) -> bool {
    matches!(
        kind,
        JavaSyntaxKind::IntegerLiteral
            | JavaSyntaxKind::FloatingPointLiteral
            | JavaSyntaxKind::BooleanLiteral
            | JavaSyntaxKind::CharacterLiteral
            | JavaSyntaxKind::StringLiteral
            | JavaSyntaxKind::TextBlockLiteral
            | JavaSyntaxKind::NullLiteral
    )
}

impl ModuleDeclaration {
    pub fn directives(&self) -> impl Iterator<Item = ModuleDirective> + '_ {
        children::<ModuleDirectiveNode>(&self.syntax).filter_map(|node| node.directive())
    }
}

impl ModuleDirectiveNode {
    #[must_use]
    pub fn directive(&self) -> Option<ModuleDirective> {
        child_family(&self.syntax)
    }
}

impl Block {
    pub fn block_statements(&self) -> impl Iterator<Item = BlockStatement> + '_ {
        children(&self.syntax)
    }

    pub fn items(&self) -> impl Iterator<Item = BlockItem> + '_ {
        children::<BlockStatement>(&self.syntax).filter_map(|node| node.item())
    }

    pub fn statements(&self) -> impl Iterator<Item = Statement> + '_ {
        children::<BlockStatement>(&self.syntax).filter_map(|node| node.statement())
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

impl BlockStatement {
    #[must_use]
    pub fn item(&self) -> Option<BlockItem> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn statement(&self) -> Option<Statement> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        matches!(
            self.syntax
                .children_with_tokens()
                .map(|element| element.kind())
                .collect::<Vec<_>>()
                .as_slice(),
            [
                JavaSyntaxKind::LocalVariableDeclaration,
                JavaSyntaxKind::Semicolon
            ] | [JavaSyntaxKind::Block
                | JavaSyntaxKind::ReturnStatement
                | JavaSyntaxKind::ThrowStatement
                | JavaSyntaxKind::YieldStatement
                | JavaSyntaxKind::ExpressionStatement]
        )
    }
}
