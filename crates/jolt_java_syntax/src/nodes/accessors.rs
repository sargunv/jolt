use super::{
    Annotation, AnnotationArgumentList, AnnotationElementList, AnnotationInterfaceBody,
    AnnotationInterfaceBodyMember, AnnotationInterfaceDeclaration, AnyJavaNode, ArgumentList,
    ArrayCreationExpression, ArrayDimensions, ArrayInitializer, ArrayType, AssertStatement,
    AssignmentExpression, BasicForStatement, BinaryExpression, Block, BlockItem, BlockStatement,
    BreakStatement, CastExpression, CatchClause, CatchParameter, CatchTypeList, ClassBody,
    ClassBodyDeclaration, ClassBodyMember, ClassDeclaration, CompilationUnit,
    ConditionalExpression, ConstructorBody, ConstructorDeclaration, ContinueStatement,
    DimExpression, DoStatement, EnhancedForStatement, EnumBody, EnumConstant, EnumConstantList,
    EnumDeclaration, Expression, ExpressionStatement, ExtendsClause, FieldDeclaration,
    FinallyClause, ForInitializer, ForStatement, ForUpdate, FormalParameter, FormalParameterList,
    IfStatement, ImplementsClause, ImportDeclaration, InstanceInitializer, InterfaceBody,
    InterfaceBodyMember, InterfaceDeclaration, JavaNode, JavaSyntaxKind, JavaSyntaxToken,
    LabeledStatement, LambdaExpression, LambdaParameter, LambdaParameterList,
    LocalVariableDeclaration, MethodDeclaration, MethodInvocationExpression, ModifierList,
    ModuleDeclaration, ModuleDirective, ModuleDirectiveNode, NameSyntax, ObjectCreationExpression,
    PackageDeclaration, ParenthesizedExpression, PermitsClause, PostfixExpression,
    ProvidesDirective, RecordBody, RecordComponent, RecordComponentList, RecordDeclaration,
    RequiresDirective, Resource, ResourceList, ResourceSpecification, ReturnStatement, Statement,
    StatementExpressionList, StaticInitializer, SwitchBlock, SwitchBlockEntry,
    SwitchBlockStatementGroup, SwitchExpression, SwitchLabel, SwitchRule, SwitchStatement,
    SynchronizedStatement, ThrowStatement, ThrowsClause, TryStatement, TryWithResourcesStatement,
    Type, TypeDeclaration, TypeParameter, TypeParameterList, UnaryExpression, VariableAccess,
    VariableDeclarator, VariableDeclaratorList, VariableInitializer, VariableInitializerValue,
    WhileStatement, YieldStatement, child, child_family, child_token, child_token_in, children,
    children_family, children_tokens_matching, nth_child_family, nth_child_token,
};
use jolt_syntax::TriviaKind;

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
    pub fn name(&self) -> Option<NameSyntax> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn is_static(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::StaticKw).is_some()
    }

    #[must_use]
    pub fn is_star(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::Star).is_some()
    }

    #[must_use]
    pub fn is_module(&self) -> bool {
        self.contextual_keyword("module").is_some()
    }

    #[must_use]
    pub fn import_path(&self) -> Option<String> {
        let mut path = self.name()?.source_text();
        if self.is_star() {
            path.push_str(".*");
        }
        Some(path)
    }

    #[must_use]
    pub fn has_leading_comment(&self) -> bool {
        node_has_leading_comment(&self.syntax)
    }

    #[must_use]
    pub fn leading_comment_texts(&self) -> Vec<String> {
        node_leading_comment_texts(&self.syntax)
    }

    fn contextual_keyword(&self, text: &str) -> Option<JavaSyntaxToken> {
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::Identifier)
            .find(|token| token.text() == text)
    }
}

impl PackageDeclaration {
    #[must_use]
    pub fn name(&self) -> Option<NameSyntax> {
        child_family(&self.syntax)
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
    pub fn members(&self) -> impl Iterator<Item = ClassBodyMember> + '_ {
        children::<ClassBodyDeclaration>(&self.syntax).filter_map(|node| node.member())
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
}

impl ThrowsClause {
    pub fn exceptions(&self) -> impl Iterator<Item = Type> + '_ {
        children_family(&self.syntax)
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
}

impl VariableInitializer {
    #[must_use]
    pub fn value(&self) -> Option<VariableInitializerValue> {
        child_family(&self.syntax)
    }
}

impl LocalVariableDeclaration {
    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn declarators(&self) -> Option<VariableDeclaratorList> {
        child(&self.syntax)
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
    pub fn arguments(&self) -> Option<ArgumentList> {
        child(&self.syntax)
    }
}

impl ArgumentList {
    pub fn arguments(&self) -> impl Iterator<Item = Expression> + '_ {
        children_family(&self.syntax)
    }
}

impl ArrayType {
    #[must_use]
    pub fn dimensions(&self) -> Option<ArrayDimensions> {
        child(&self.syntax)
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
}

impl LabeledStatement {
    #[must_use]
    pub fn label(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn body(&self) -> Option<Statement> {
        child_family(&self.syntax)
    }
}

impl AssertStatement {
    #[must_use]
    pub fn condition(&self) -> Option<Expression> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn detail(&self) -> Option<Expression> {
        nth_child_family(&self.syntax, 1)
    }
}

impl BreakStatement {
    #[must_use]
    pub fn label(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }
}

impl ContinueStatement {
    #[must_use]
    pub fn label(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }
}

impl ReturnStatement {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }
}

impl ThrowStatement {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }
}

impl YieldStatement {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
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

impl TryStatement {
    #[must_use]
    pub fn resources_statement(&self) -> Option<TryWithResourcesStatement> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Block> {
        child(&self.syntax)
    }

    pub fn catch_clauses(&self) -> impl Iterator<Item = CatchClause> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn finally_clause(&self) -> Option<FinallyClause> {
        child(&self.syntax)
    }
}

impl TryWithResourcesStatement {
    #[must_use]
    pub fn resources(&self) -> Option<ResourceSpecification> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Block> {
        child(&self.syntax)
    }

    pub fn catch_clauses(&self) -> impl Iterator<Item = CatchClause> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn finally_clause(&self) -> Option<FinallyClause> {
        child(&self.syntax)
    }
}

impl CatchClause {
    #[must_use]
    pub fn parameter(&self) -> Option<CatchParameter> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Block> {
        child(&self.syntax)
    }
}

impl CatchParameter {
    #[must_use]
    pub fn types(&self) -> Option<CatchTypeList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token_in(
            &self.syntax,
            &[JavaSyntaxKind::Identifier, JavaSyntaxKind::UnderscoreKw],
        )
    }
}

impl FinallyClause {
    #[must_use]
    pub fn body(&self) -> Option<Block> {
        child(&self.syntax)
    }
}

impl ResourceSpecification {
    #[must_use]
    pub fn list(&self) -> Option<ResourceList> {
        child(&self.syntax)
    }
}

impl ResourceList {
    pub fn resources(&self) -> impl Iterator<Item = Resource> + '_ {
        children(&self.syntax)
    }
}

impl Resource {
    #[must_use]
    pub fn declaration(&self) -> Option<LocalVariableDeclaration> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn variable_access(&self) -> Option<VariableAccess> {
        child(&self.syntax)
    }
}

impl VariableAccess {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
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
    pub fn entries(&self) -> impl Iterator<Item = SwitchBlockEntry> + '_ {
        self.syntax.children().filter_map(|syntax| {
            if let Some(group) = SwitchBlockStatementGroup::cast(syntax.clone()) {
                return Some(SwitchBlockEntry::StatementGroup(group));
            }
            SwitchRule::cast(syntax).map(SwitchBlockEntry::Rule)
        })
    }

    pub fn statement_groups(&self) -> impl Iterator<Item = SwitchBlockStatementGroup> + '_ {
        children(&self.syntax)
    }

    pub fn rules(&self) -> impl Iterator<Item = SwitchRule> + '_ {
        children(&self.syntax)
    }
}

impl SwitchBlockStatementGroup {
    pub fn labels(&self) -> impl Iterator<Item = SwitchLabel> + '_ {
        children(&self.syntax)
    }

    pub fn items(&self) -> impl Iterator<Item = BlockItem> + '_ {
        children::<BlockStatement>(&self.syntax).filter_map(|statement| statement.item())
    }
}

impl SwitchRule {
    #[must_use]
    pub fn label(&self) -> Option<SwitchLabel> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn block(&self) -> Option<Block> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn throw_statement(&self) -> Option<ThrowStatement> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
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

impl ModuleDeclaration {
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.contextual_keyword("open").is_some()
    }

    #[must_use]
    pub fn name(&self) -> Option<NameSyntax> {
        child_family(&self.syntax)
    }

    pub fn directives(&self) -> impl Iterator<Item = ModuleDirective> + '_ {
        children::<ModuleDirectiveNode>(&self.syntax).filter_map(|node| node.directive())
    }

    fn contextual_keyword(&self, text: &str) -> Option<JavaSyntaxToken> {
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::Identifier)
            .find(|token| token.text() == text)
    }
}

impl ModuleDirectiveNode {
    #[must_use]
    pub fn directive(&self) -> Option<ModuleDirective> {
        child_family(&self.syntax)
    }
}

impl ModuleDirective {
    pub fn names(&self) -> impl Iterator<Item = NameSyntax> + '_ {
        children_family(self.syntax())
    }

    #[must_use]
    pub fn primary_name(&self) -> Option<NameSyntax> {
        self.names().next()
    }

    #[must_use]
    pub fn has_leading_comment(&self) -> bool {
        node_has_leading_comment(self.syntax())
    }

    #[must_use]
    pub fn leading_comment_texts(&self) -> Vec<String> {
        node_leading_comment_texts(self.syntax())
    }
}

impl RequiresDirective {
    #[must_use]
    pub fn has_static_modifier(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::StaticKw).is_some()
    }

    #[must_use]
    pub fn has_transitive_modifier(&self) -> bool {
        self.contextual_keyword("transitive").is_some()
    }

    fn contextual_keyword(&self, text: &str) -> Option<JavaSyntaxToken> {
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::Identifier)
            .find(|token| token.text() == text)
    }
}

impl ProvidesDirective {
    #[must_use]
    pub fn service_name(&self) -> Option<NameSyntax> {
        self.names().next()
    }

    pub fn implementation_names(&self) -> impl Iterator<Item = NameSyntax> + '_ {
        self.names().skip(1)
    }

    fn names(&self) -> impl Iterator<Item = NameSyntax> + '_ {
        children_family(&self.syntax)
    }
}

impl Block {
    pub fn items(&self) -> impl Iterator<Item = BlockItem> + '_ {
        children::<BlockStatement>(&self.syntax).filter_map(|node| node.item())
    }

    pub fn statements(&self) -> impl Iterator<Item = Statement> + '_ {
        children::<BlockStatement>(&self.syntax).filter_map(|node| node.statement())
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
}

fn node_has_leading_comment(syntax: &super::JavaSyntaxNode) -> bool {
    syntax.first_token().is_some_and(|token| {
        token.leading().iter().any(|trivia| {
            matches!(
                trivia.kind(),
                TriviaKind::LineComment | TriviaKind::BlockComment | TriviaKind::DocComment
            )
        })
    })
}

fn node_leading_comment_texts(syntax: &super::JavaSyntaxNode) -> Vec<String> {
    syntax
        .first_token()
        .map(|token| {
            token
                .leading()
                .iter()
                .filter(|trivia| {
                    matches!(
                        trivia.kind(),
                        TriviaKind::LineComment | TriviaKind::BlockComment | TriviaKind::DocComment
                    )
                })
                .map(|trivia| trivia.text().trim().to_owned())
                .collect()
        })
        .unwrap_or_default()
}
