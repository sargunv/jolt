use super::{
    Annotation, AnnotationArgument, AnnotationArgumentList, AnnotationArrayInitializer,
    AnnotationElementDeclaration, AnnotationElementList, AnnotationElementValue,
    AnnotationElementValuePair, AnnotationInterfaceBody, AnnotationInterfaceBodyMember,
    AnnotationInterfaceDeclaration, AnyJavaNode, ArgumentList, ArgumentListEntry,
    ArrayAccessExpression, ArrayCreationExpression, ArrayDimension, ArrayDimensions,
    ArrayInitializer, ArrayType, AssertStatement, AssignmentExpression, BasicForStatement,
    BinaryExpression, Block, BlockItem, BlockStatement, BreakStatement, CaseConstant, CasePattern,
    CastExpression, CatchClause, CatchParameter, CatchTypeList, ClassBody, ClassBodyDeclaration,
    ClassBodyMember, ClassDeclaration, ClassLiteralExpression, ClassType, ClassTypeSegment,
    CompactConstructorDeclaration, CompilationUnit, CompilationUnitItem, ComponentPattern,
    ConditionalExpression, ConstructorBody, ConstructorDeclaration, ContinueStatement,
    DefaultValue, DimExpression, DoStatement, EmptyDeclaration, EnhancedForStatement, EnumBody,
    EnumConstant, EnumConstantList, EnumDeclaration, ExportsDirective, Expression,
    ExpressionStatement, ExtendsClause, FieldAccessExpression, FieldDeclaration, FinallyClause,
    ForInitializer, ForStatement, ForUpdate, FormalParameter, FormalParameterList, Guard,
    IfStatement, ImplementsClause, ImportDeclaration, ImportKind, InstanceInitializer,
    InstanceofExpression, InterfaceBody, InterfaceBodyMember, InterfaceDeclaration,
    IntersectionType, JavaFamily, JavaNode, JavaSyntaxKind, JavaSyntaxToken, LabeledStatement,
    LambdaExpression, LambdaParameter, LambdaParameterList, LiteralExpression,
    LocalClassOrInterfaceDeclaration, LocalVariableDeclaration, MatchAllPattern, MemberChain,
    MemberChainSuffix, MethodDeclaration, MethodInvocationExpression, MethodReferenceExpression,
    ModifierList, ModuleDeclaration, ModuleDirective, ModuleDirectiveNode, ModuleDirectiveRole,
    NameExpression, NameSegment, NameSyntax, ObjectCreationExpression, OpensDirective,
    PackageDeclaration, ParenthesizedExpression, Pattern, PermitsClause, PostfixExpression,
    PrimitiveType, ProvidesDirective, RecordBody, RecordComponent, RecordComponentList,
    RecordDeclaration, RecordPattern, RequiresDirective, Resource, ResourceList,
    ResourceSpecification, ReturnStatement, Statement, StatementBody, StatementExpressionList,
    StaticInitializer, SuperExpression, SwitchBlock, SwitchBlockEntry, SwitchBlockStatementGroup,
    SwitchExpression, SwitchLabel, SwitchLabelCaseItem, SwitchRule, SwitchStatement,
    SynchronizedStatement, ThisExpression, ThrowStatement, ThrowsClause, TryStatement,
    TryWithResourcesStatement, Type, TypeArgument, TypeArgumentList, TypeBoundList,
    TypeDeclaration, TypeParameter, TypeParameterList, TypePattern, UnaryExpression, UnionType,
    UsesDirective, VariableAccess, VariableDeclarator, VariableDeclaratorEntry,
    VariableDeclaratorList, VariableInitializer, VariableInitializerValue, VoidType,
    WhileStatement, WildcardBound, WildcardType, YieldStatement, child, child_family, child_token,
    child_token_in, children, children_family, children_tokens_matching, nth_child_family,
    nth_child_token, starts_after_blank_line, tokens,
};
use jolt_syntax::{SyntaxElement, TriviaKind};

impl CompilationUnit {
    pub fn items(&self) -> impl Iterator<Item = CompilationUnitItem> + '_ {
        self.syntax.children().filter_map(|syntax| {
            if let Some(package) = PackageDeclaration::cast(syntax.clone()) {
                return Some(CompilationUnitItem::Package(package));
            }
            if let Some(import) = ImportDeclaration::cast(syntax.clone()) {
                return Some(CompilationUnitItem::Import(import));
            }
            if let Some(module) = ModuleDeclaration::cast(syntax.clone()) {
                return Some(CompilationUnitItem::Module(module));
            }
            if let Some(declaration) = TypeDeclaration::cast(syntax.clone()) {
                return Some(CompilationUnitItem::Type(declaration));
            }
            EmptyDeclaration::cast(syntax).map(CompilationUnitItem::EmptyDeclaration)
        })
    }

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
    pub fn import_kind(&self) -> Option<ImportKind> {
        let name = self.name()?;
        match (self.is_module(), self.is_static(), self.is_star()) {
            (true, _, _) => Some(ImportKind::SingleModule(name)),
            (false, true, true) => Some(ImportKind::StaticOnDemand(name)),
            (false, true, false) => Some(ImportKind::SingleStatic(name)),
            (false, false, true) => Some(ImportKind::TypeOnDemand(name)),
            (false, false, false) => Some(ImportKind::SingleType(name)),
        }
    }

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
        let name_start = self.name().map(|name| name.text_range().start());
        self.contextual_keyword("module").is_some_and(|token| {
            name_start.is_some_and(|name_start| token.token_text_range().end() <= name_start)
        })
    }

    #[must_use]
    pub fn import_path(&self) -> Option<String> {
        self.import_kind().map(|kind| match kind {
            ImportKind::SingleType(name)
            | ImportKind::SingleStatic(name)
            | ImportKind::SingleModule(name) => name.compact_text(),
            ImportKind::TypeOnDemand(name) | ImportKind::StaticOnDemand(name) => {
                format!("{}.*", name.compact_text())
            }
        })
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

    pub fn segments_with_annotations(&self) -> impl Iterator<Item = NameSegment> {
        let mut segments = Vec::new();
        let mut annotations = Vec::new();

        for element in self.syntax().children_with_tokens() {
            match element {
                SyntaxElement::Node(node) => {
                    if let Some(annotation) = Annotation::cast(node) {
                        annotations.push(annotation);
                    }
                }
                SyntaxElement::Token(syntax) if syntax.kind() == JavaSyntaxKind::Identifier => {
                    segments.push(NameSegment {
                        annotations: std::mem::take(&mut annotations),
                        identifier: JavaSyntaxToken { syntax },
                    });
                }
                SyntaxElement::Token(_) => {}
            }
        }

        segments.into_iter()
    }

    #[must_use]
    pub fn compact_text(&self) -> String {
        self.segments()
            .map(|token| token.text().to_owned())
            .collect::<Vec<_>>()
            .join(".")
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

impl ExtendsClause {
    pub fn types(&self) -> impl Iterator<Item = Type> + '_ {
        children_family(&self.syntax)
    }
}

impl ImplementsClause {
    pub fn types(&self) -> impl Iterator<Item = Type> + '_ {
        children_family(&self.syntax)
    }
}

impl PermitsClause {
    pub fn names(&self) -> impl Iterator<Item = NameSyntax> + '_ {
        children_family(&self.syntax)
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
}

impl TypeBoundList {
    pub fn bounds(&self) -> impl Iterator<Item = Type> + '_ {
        children_family(&self.syntax)
    }
}

impl PrimitiveType {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token_in(
            &self.syntax,
            &[
                JavaSyntaxKind::BooleanKw,
                JavaSyntaxKind::ByteKw,
                JavaSyntaxKind::CharKw,
                JavaSyntaxKind::DoubleKw,
                JavaSyntaxKind::FloatKw,
                JavaSyntaxKind::IntKw,
                JavaSyntaxKind::LongKw,
                JavaSyntaxKind::ShortKw,
            ],
        )
    }
}

impl VoidType {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::VoidKw)
    }
}

impl ClassType {
    pub fn segments(&self) -> impl Iterator<Item = ClassTypeSegment> {
        let mut segments = Vec::new();
        let mut annotations = Vec::new();
        let mut current: Option<ClassTypeSegment> = None;

        for element in self.syntax.children_with_tokens() {
            let SyntaxElement::Node(node) = element else {
                continue;
            };

            if let Some(annotation) = Annotation::cast(node.clone()) {
                annotations.push(annotation);
                continue;
            }

            if let Some(name) = NameSyntax::cast(node.clone()) {
                if let Some(segment) = current.take() {
                    segments.push(segment);
                }
                current = Some(ClassTypeSegment {
                    annotations: std::mem::take(&mut annotations),
                    name,
                    type_arguments: None,
                });
                continue;
            }

            if let Some(type_arguments) = TypeArgumentList::cast(node)
                && let Some(segment) = current.as_mut()
            {
                segment.type_arguments = Some(type_arguments);
            }
        }

        if let Some(segment) = current {
            segments.push(segment);
        }

        segments.into_iter()
    }
}

impl TypeArgument {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
    }
}

impl WildcardType {
    #[must_use]
    pub fn bound_clause(&self) -> Option<WildcardBound> {
        let keyword = self.bound_keyword()?;
        let bound = self.bound()?;
        match keyword.kind() {
            JavaSyntaxKind::ExtendsKw => Some(WildcardBound::Extends(bound)),
            JavaSyntaxKind::SuperKw => Some(WildcardBound::Super(bound)),
            _ => None,
        }
    }

    #[must_use]
    pub fn bound_keyword(&self) -> Option<JavaSyntaxToken> {
        child_token_in(
            &self.syntax,
            &[JavaSyntaxKind::ExtendsKw, JavaSyntaxKind::SuperKw],
        )
    }

    #[must_use]
    pub fn bound(&self) -> Option<Type> {
        child_family(&self.syntax)
    }
}

impl RecordComponentList {
    pub fn components(&self) -> impl Iterator<Item = RecordComponent> + '_ {
        children(&self.syntax)
    }
}

impl RecordComponent {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        children(&self.syntax)
    }

    pub fn prefix_annotations(&self) -> impl Iterator<Item = Annotation> {
        annotations_before_type(&self.syntax, self.ty())
    }

    pub fn varargs_annotations(&self) -> impl Iterator<Item = Annotation> {
        annotations_between_type_and_token(&self.syntax, self.ty(), JavaSyntaxKind::Ellipsis)
    }

    pub fn modifier_tokens(&self) -> impl Iterator<Item = JavaSyntaxToken> + '_ {
        children_tokens_matching(&self.syntax, is_modifier_token)
    }

    #[must_use]
    pub fn is_variable_arity(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::Ellipsis).is_some()
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
}

impl ClassBody {
    pub fn members(&self) -> impl Iterator<Item = ClassBodyMember> + '_ {
        children::<ClassBodyDeclaration>(&self.syntax).filter_map(|node| node.member())
    }
}

impl ClassBodyMember {
    #[must_use]
    pub fn starts_after_blank_line(&self) -> bool {
        starts_after_blank_line(self.syntax())
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

impl InterfaceBodyMember {
    #[must_use]
    pub fn starts_after_blank_line(&self) -> bool {
        starts_after_blank_line(self.syntax())
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

impl AnnotationInterfaceBodyMember {
    #[must_use]
    pub fn starts_after_blank_line(&self) -> bool {
        starts_after_blank_line(self.syntax())
    }
}

impl AnnotationElementList {
    pub fn members(&self) -> impl Iterator<Item = AnnotationInterfaceBodyMember> + '_ {
        children_family(&self.syntax)
    }

    pub fn arguments(&self) -> impl Iterator<Item = AnnotationArgument> + '_ {
        self.syntax.children().filter_map(AnnotationArgument::cast)
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
}

impl DefaultValue {
    #[must_use]
    pub fn value(&self) -> Option<AnnotationElementValue> {
        child(&self.syntax)
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
    pub fn annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        children(&self.syntax)
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
}

impl BlockItem {
    #[must_use]
    pub fn starts_after_blank_line(&self) -> bool {
        starts_after_blank_line(self.syntax())
    }
}

impl LocalClassOrInterfaceDeclaration {
    #[must_use]
    pub fn declaration(&self) -> Option<TypeDeclaration> {
        child_family(&self.syntax)
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

    #[must_use]
    pub fn header_tokens(&self) -> Vec<JavaSyntaxToken> {
        let header_end = self
            .body()
            .map_or_else(|| self.text_range().end(), |body| body.text_range().start());
        let mut header_tokens = tokens(&self.syntax)
            .into_iter()
            .filter(|token| token.token_text_range().start() < header_end)
            .collect::<Vec<_>>();
        if self.body().is_none()
            && header_tokens
                .last()
                .is_some_and(|token| token.kind() == JavaSyntaxKind::Semicolon)
        {
            header_tokens.pop();
        }
        header_tokens
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
    pub fn header_tokens(&self) -> Vec<JavaSyntaxToken> {
        let header_end = self
            .body()
            .map_or_else(|| self.text_range().end(), |body| body.text_range().start());
        tokens(&self.syntax)
            .into_iter()
            .filter(|token| token.token_text_range().start() < header_end)
            .collect()
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
}

impl ConstructorBody {
    pub fn items(&self) -> impl Iterator<Item = BlockItem> + '_ {
        children::<BlockStatement>(&self.syntax).filter_map(|node| node.item())
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
    pub fn annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        children(&self.syntax)
    }

    pub fn prefix_annotations(&self) -> impl Iterator<Item = Annotation> {
        annotations_before_type(&self.syntax, self.ty())
    }

    pub fn varargs_annotations(&self) -> impl Iterator<Item = Annotation> {
        annotations_between_type_and_token(&self.syntax, self.ty(), JavaSyntaxKind::Ellipsis)
    }

    pub fn modifier_tokens(&self) -> impl Iterator<Item = JavaSyntaxToken> + '_ {
        children_tokens_matching(&self.syntax, is_modifier_token)
    }

    #[must_use]
    pub fn is_variable_arity(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::Ellipsis).is_some()
    }

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

    #[must_use]
    pub fn is_unnamed(&self) -> bool {
        self.name()
            .is_some_and(|name| name.kind() == JavaSyntaxKind::UnderscoreKw)
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

    pub fn entries(&self) -> impl Iterator<Item = VariableDeclaratorEntry> {
        let mut entries = Vec::new();
        let mut pending_declarator = None;

        for element in self.syntax.children_with_tokens() {
            match element {
                SyntaxElement::Node(node) => {
                    if let Some(declarator) = VariableDeclarator::cast(node)
                        && let Some(previous) = pending_declarator.replace(declarator)
                    {
                        entries.push(VariableDeclaratorEntry {
                            declarator: previous,
                            comma: None,
                        });
                    }
                }
                SyntaxElement::Token(token) if token.kind() == JavaSyntaxKind::Comma => {
                    if let Some(declarator) = pending_declarator.take() {
                        entries.push(VariableDeclaratorEntry {
                            declarator,
                            comma: Some(JavaSyntaxToken { syntax: token }),
                        });
                    }
                }
                SyntaxElement::Token(_) => {}
            }
        }

        if let Some(declarator) = pending_declarator {
            entries.push(VariableDeclaratorEntry {
                declarator,
                comma: None,
            });
        }

        entries.into_iter()
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
    pub fn is_unnamed(&self) -> bool {
        self.name()
            .is_some_and(|name| name.kind() == JavaSyntaxKind::UnderscoreKw)
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
    pub fn annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        children(&self.syntax)
    }

    pub fn modifier_tokens(&self) -> impl Iterator<Item = JavaSyntaxToken> + '_ {
        children_tokens_matching(&self.syntax, is_modifier_token)
    }

    #[must_use]
    pub fn var_token(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier).filter(|token| token.text() == "var")
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
    pub fn then_body(&self) -> Option<StatementBody> {
        self.then_statement().map(StatementBody::from)
    }

    #[must_use]
    pub fn else_statement(&self) -> Option<Statement> {
        nth_child_family(&self.syntax, 1)
    }

    #[must_use]
    pub fn else_body(&self) -> Option<StatementBody> {
        self.else_statement().map(StatementBody::from)
    }
}

impl From<Statement> for StatementBody {
    fn from(statement: Statement) -> Self {
        match statement {
            Statement::Block(block) => Self::Block(block),
            Statement::EmptyStatement(empty) => Self::Empty(empty),
            statement => Self::Unbraced(statement),
        }
    }
}

impl LiteralExpression {
    #[must_use]
    pub fn literal_token(&self) -> Option<JavaSyntaxToken> {
        self.syntax
            .first_token()
            .map(|syntax| JavaSyntaxToken { syntax })
    }
}

impl NameExpression {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }
}

impl ThisExpression {
    #[must_use]
    pub fn qualifier(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::ThisKw)
    }
}

impl SuperExpression {
    #[must_use]
    pub fn qualifier(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::SuperKw)
    }
}

impl ClassLiteralExpression {
    #[must_use]
    pub fn target_expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn void_type(&self) -> Option<VoidType> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn primitive_keyword(&self) -> Option<JavaSyntaxToken> {
        child_token_in(
            &self.syntax,
            &[
                JavaSyntaxKind::BooleanKw,
                JavaSyntaxKind::ByteKw,
                JavaSyntaxKind::CharKw,
                JavaSyntaxKind::DoubleKw,
                JavaSyntaxKind::FloatKw,
                JavaSyntaxKind::IntKw,
                JavaSyntaxKind::LongKw,
                JavaSyntaxKind::ShortKw,
            ],
        )
    }

    #[must_use]
    pub fn dimensions(&self) -> Option<ArrayDimensions> {
        child(&self.syntax)
    }
}

impl Expression {
    #[must_use]
    pub fn member_chain(&self) -> Option<MemberChain> {
        collect_member_chain(self)
    }
}

fn collect_member_chain(expression: &Expression) -> Option<MemberChain> {
    match expression {
        Expression::FieldAccessExpression(access) => {
            let receiver = access.receiver()?;
            Some(append_member_chain_suffix(
                receiver,
                MemberChainSuffix::FieldAccess(access.clone()),
            ))
        }
        Expression::MethodInvocationExpression(invocation) => {
            invocation.direct_method_name()?;
            let qualifier = invocation.qualifier()?;
            Some(append_member_chain_suffix(
                qualifier,
                MemberChainSuffix::MethodInvocation(invocation.clone()),
            ))
        }
        _ => None,
    }
}

fn append_member_chain_suffix(receiver: Expression, suffix: MemberChainSuffix) -> MemberChain {
    if let Some(mut chain) = collect_member_chain(&receiver) {
        chain.suffixes.push(suffix);
        return chain;
    }

    MemberChain {
        root: receiver,
        suffixes: vec![suffix],
    }
}

impl MethodInvocationExpression {
    #[must_use]
    pub fn qualifier(&self) -> Option<Expression> {
        self.direct_method_name()
            .and_then(|_| child_family(&self.syntax))
    }

    #[must_use]
    pub fn direct_method_name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn simple_name_expression(&self) -> Option<Expression> {
        self.direct_method_name()
            .is_none()
            .then(|| child_family(&self.syntax))
            .flatten()
    }

    #[must_use]
    pub fn type_arguments(&self) -> Option<TypeArgumentList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn arguments(&self) -> Option<ArgumentList> {
        child(&self.syntax)
    }
}

impl ArgumentList {
    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    pub fn arguments(&self) -> impl Iterator<Item = Expression> + '_ {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = ArgumentListEntry> {
        let mut entries = Vec::new();
        let mut pending_argument = None;

        for element in self.syntax.children_with_tokens() {
            match element {
                SyntaxElement::Node(node) => {
                    if let Some(argument) = Expression::cast(node)
                        && let Some(previous) = pending_argument.replace(argument)
                    {
                        entries.push(ArgumentListEntry {
                            argument: previous,
                            comma: None,
                        });
                    }
                }
                SyntaxElement::Token(token) if token.kind() == JavaSyntaxKind::Comma => {
                    if let Some(argument) = pending_argument.take() {
                        entries.push(ArgumentListEntry {
                            argument,
                            comma: Some(JavaSyntaxToken { syntax: token }),
                        });
                    }
                }
                SyntaxElement::Token(_) => {}
            }
        }

        if let Some(argument) = pending_argument {
            entries.push(ArgumentListEntry {
                argument,
                comma: None,
            });
        }

        entries.into_iter()
    }
}

impl TypeArgumentList {
    pub fn arguments(&self) -> impl Iterator<Item = TypeArgument> + '_ {
        children(&self.syntax)
    }
}

impl FieldAccessExpression {
    #[must_use]
    pub fn receiver(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn field_name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn type_arguments(&self) -> Option<TypeArgumentList> {
        child(&self.syntax)
    }
}

impl MethodReferenceExpression {
    #[must_use]
    pub fn receiver_expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn receiver_type(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn receiver_dimensions(&self) -> Option<ArrayDimensions> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn type_arguments(&self) -> Option<TypeArgumentList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn is_constructor_reference(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::NewKw).is_some()
    }

    #[must_use]
    pub fn target_name(&self) -> Option<JavaSyntaxToken> {
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::Identifier).last()
    }
}

impl ArrayAccessExpression {
    #[must_use]
    pub fn array(&self) -> Option<Expression> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn index(&self) -> Option<Expression> {
        nth_child_family(&self.syntax, 1)
    }
}

impl ArrayType {
    #[must_use]
    pub fn element_type(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn dimensions(&self) -> Option<ArrayDimensions> {
        child(&self.syntax)
    }
}

impl ArrayDimensions {
    pub fn dimensions(&self) -> impl Iterator<Item = ArrayDimension> + '_ {
        children(&self.syntax)
    }
}

impl ArrayDimension {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        children(&self.syntax)
    }
}

impl IntersectionType {
    pub fn types(&self) -> impl Iterator<Item = Type> + '_ {
        children_family(&self.syntax)
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
}

impl AnnotationArgumentList {
    pub fn arguments(&self) -> impl Iterator<Item = AnnotationArgument> {
        child::<AnnotationElementList>(&self.syntax)
            .map(|list| list.arguments().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
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
}

impl AnnotationArrayInitializer {
    pub fn values(&self) -> impl Iterator<Item = AnnotationElementValue> + '_ {
        children(&self.syntax)
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

impl InstanceofExpression {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn pattern(&self) -> Option<Pattern> {
        child_family(&self.syntax)
    }
}

impl ObjectCreationExpression {
    #[must_use]
    pub fn qualifier(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn constructor_type_arguments(&self) -> Option<TypeArgumentList> {
        child(&self.syntax)
    }

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

impl DimExpression {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }
}

impl ArrayInitializer {
    pub fn values(&self) -> impl Iterator<Item = VariableInitializerValue> + '_ {
        children_family(&self.syntax)
    }
}

impl LambdaExpression {
    #[must_use]
    pub fn parameters(&self) -> Option<LambdaParameterList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn concise_parameter(&self) -> Option<LambdaParameter> {
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
    pub fn prefix_annotations(&self) -> impl Iterator<Item = Annotation> {
        annotations_before_type(&self.syntax, self.ty())
    }

    pub fn varargs_annotations(&self) -> impl Iterator<Item = Annotation> {
        annotations_between_type_and_token(&self.syntax, self.ty(), JavaSyntaxKind::Ellipsis)
    }

    pub fn modifier_tokens(&self) -> impl Iterator<Item = JavaSyntaxToken> + '_ {
        children_tokens_matching(&self.syntax, is_modifier_token)
    }

    #[must_use]
    pub fn var_token(&self) -> Option<JavaSyntaxToken> {
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::Identifier)
            .find(|token| token.text() == "var")
    }

    #[must_use]
    pub fn is_variable_arity(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::Ellipsis).is_some()
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        children_tokens_matching(&self.syntax, |kind| {
            matches!(
                kind,
                JavaSyntaxKind::Identifier | JavaSyntaxKind::UnderscoreKw
            )
        })
        .last()
    }

    #[must_use]
    pub fn is_unnamed(&self) -> bool {
        self.name()
            .is_some_and(|name| name.kind() == JavaSyntaxKind::UnderscoreKw)
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

    #[must_use]
    pub fn statement_body(&self) -> Option<StatementBody> {
        self.body().map(StatementBody::from)
    }
}

impl DoStatement {
    #[must_use]
    pub fn body(&self) -> Option<Statement> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn statement_body(&self) -> Option<StatementBody> {
        self.body().map(StatementBody::from)
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
    pub fn annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        children(&self.syntax)
    }

    pub fn modifier_tokens(&self) -> impl Iterator<Item = JavaSyntaxToken> + '_ {
        children_tokens_matching(&self.syntax, is_modifier_token)
    }

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

    #[must_use]
    pub fn is_unnamed(&self) -> bool {
        self.name()
            .is_some_and(|name| name.kind() == JavaSyntaxKind::UnderscoreKw)
    }
}

impl CatchTypeList {
    pub fn types(&self) -> impl Iterator<Item = Type> + '_ {
        child::<UnionType>(&self.syntax)
            .map_or_else(
                || children_family(&self.syntax).collect(),
                |union| union.types().collect::<Vec<_>>(),
            )
            .into_iter()
    }
}

impl UnionType {
    pub fn types(&self) -> impl Iterator<Item = Type> + '_ {
        children_family(&self.syntax)
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

    #[must_use]
    pub fn statement_body(&self) -> Option<StatementBody> {
        self.body().map(StatementBody::from)
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

    #[must_use]
    pub fn statement_body(&self) -> Option<StatementBody> {
        self.body().map(StatementBody::from)
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
    pub fn arrow(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Arrow)
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

impl SwitchLabel {
    #[must_use]
    pub fn is_default_label(&self) -> bool {
        self.syntax
            .first_token()
            .is_some_and(|token| token.kind() == JavaSyntaxKind::DefaultKw)
    }

    pub fn case_items(&self) -> impl Iterator<Item = SwitchLabelCaseItem> {
        self.syntax
            .children_with_tokens()
            .filter_map(|element| match element {
                SyntaxElement::Node(node) => CaseConstant::cast(node.clone())
                    .map(SwitchLabelCaseItem::Constant)
                    .or_else(|| CasePattern::cast(node).map(SwitchLabelCaseItem::Pattern)),
                SyntaxElement::Token(syntax) if syntax.kind() == JavaSyntaxKind::DefaultKw => {
                    Some(SwitchLabelCaseItem::Default(JavaSyntaxToken { syntax }))
                }
                SyntaxElement::Token(_) => None,
            })
            .filter(|item| {
                !matches!(item, SwitchLabelCaseItem::Default(_)) || !self.is_default_label()
            })
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[must_use]
    pub fn guard(&self) -> Option<Guard> {
        child(&self.syntax)
    }
}

impl CaseConstant {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }
}

impl CasePattern {
    #[must_use]
    pub fn pattern(&self) -> Option<Pattern> {
        child_family(&self.syntax)
    }
}

impl Guard {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }
}

impl TypePattern {
    #[must_use]
    pub fn variable(&self) -> Option<LocalVariableDeclaration> {
        child(&self.syntax)
    }
}

impl RecordPattern {
    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    pub fn components(&self) -> impl Iterator<Item = ComponentPattern> + '_ {
        children(&self.syntax)
    }
}

impl ComponentPattern {
    #[must_use]
    pub fn pattern(&self) -> Option<Pattern> {
        child_family(&self.syntax)
    }
}

impl MatchAllPattern {
    #[must_use]
    pub fn underscore(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::UnderscoreKw)
    }

    #[must_use]
    pub fn is_unnamed(&self) -> bool {
        self.underscore().is_some()
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
    #[must_use]
    pub fn directive_role(&self) -> Option<ModuleDirectiveRole> {
        match self {
            Self::RequiresDirective(directive) => directive.directive_role(),
            Self::ExportsDirective(directive) => directive.directive_role(),
            Self::OpensDirective(directive) => directive.directive_role(),
            Self::UsesDirective(directive) => directive.directive_role(),
            Self::ProvidesDirective(directive) => directive.directive_role(),
        }
    }

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
    pub fn directive_role(&self) -> Option<ModuleDirectiveRole> {
        Some(ModuleDirectiveRole::Requires {
            module: self.module_name()?,
            is_static: self.has_static_modifier(),
            is_transitive: self.has_transitive_modifier(),
        })
    }

    #[must_use]
    pub fn module_name(&self) -> Option<NameSyntax> {
        child_family(&self.syntax)
    }

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

impl ExportsDirective {
    #[must_use]
    pub fn directive_role(&self) -> Option<ModuleDirectiveRole> {
        let mut names = self.names();
        Some(ModuleDirectiveRole::Exports {
            package: names.next()?,
            targets: names.collect(),
        })
    }

    pub fn names(&self) -> impl Iterator<Item = NameSyntax> + '_ {
        children_family(&self.syntax)
    }
}

impl OpensDirective {
    #[must_use]
    pub fn directive_role(&self) -> Option<ModuleDirectiveRole> {
        let mut names = self.names();
        Some(ModuleDirectiveRole::Opens {
            package: names.next()?,
            targets: names.collect(),
        })
    }

    pub fn names(&self) -> impl Iterator<Item = NameSyntax> + '_ {
        children_family(&self.syntax)
    }
}

impl UsesDirective {
    #[must_use]
    pub fn directive_role(&self) -> Option<ModuleDirectiveRole> {
        Some(ModuleDirectiveRole::Uses {
            service: self.service_name()?,
        })
    }

    #[must_use]
    pub fn service_name(&self) -> Option<NameSyntax> {
        child_family(&self.syntax)
    }
}

impl ProvidesDirective {
    #[must_use]
    pub fn directive_role(&self) -> Option<ModuleDirectiveRole> {
        Some(ModuleDirectiveRole::Provides {
            service: self.service_name()?,
            implementations: self.implementation_names().collect(),
        })
    }

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

fn annotations_before_type(
    syntax: &super::JavaSyntaxNode,
    ty: Option<Type>,
) -> std::vec::IntoIter<Annotation> {
    let Some(ty) = ty else {
        return children::<Annotation>(syntax)
            .collect::<Vec<_>>()
            .into_iter();
    };
    let type_start = ty.text_range().start();
    children::<Annotation>(syntax)
        .filter(|annotation| annotation.text_range().start() < type_start)
        .collect::<Vec<_>>()
        .into_iter()
}

fn annotations_between_type_and_token(
    syntax: &super::JavaSyntaxNode,
    ty: Option<Type>,
    token_kind: JavaSyntaxKind,
) -> std::vec::IntoIter<Annotation> {
    let (Some(ty), Some(token)) = (ty, child_token(syntax, token_kind)) else {
        return Vec::new().into_iter();
    };
    let type_end = ty.text_range().end();
    let token_start = token.token_text_range().start();
    children::<Annotation>(syntax)
        .filter(|annotation| {
            let start = annotation.text_range().start();
            start >= type_end && start < token_start
        })
        .collect::<Vec<_>>()
        .into_iter()
}
