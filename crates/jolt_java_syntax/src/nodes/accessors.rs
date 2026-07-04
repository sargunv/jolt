use super::{
    Annotation, AnnotationArgument, AnnotationArgumentList, AnnotationArgumentListEntry,
    AnnotationArrayInitializer, AnnotationArrayInitializerEntry, AnnotationElementDeclaration,
    AnnotationElementList, AnnotationElementValue, AnnotationElementValuePair,
    AnnotationInterfaceBody, AnnotationInterfaceBodyMember, AnnotationInterfaceDeclaration,
    AnyJavaNode, ArgumentList, ArgumentListEntry, ArrayAccessExpression, ArrayCreationExpression,
    ArrayDimension, ArrayDimensions, ArrayInitializer, ArrayInitializerEntry, ArrayType,
    AssertStatement, AssignmentExpression, BasicForStatement, BinaryExpression, Block, BlockItem,
    BlockStatement, BreakStatement, COMPOSITE_ASSIGNMENT_OPERATORS, COMPOSITE_BINARY_OPERATORS,
    CaseConstant, CasePattern, CastExpression, CatchClause, CatchParameter, CatchTypeList,
    ClassBody, ClassBodyDeclaration, ClassBodyMember, ClassDeclaration, ClassLiteralExpression,
    ClassType, ClassTypeSegment, CompactConstructorDeclaration, CompilationUnit,
    CompilationUnitItem, ComponentPattern, ConditionalExpression, ConstructorBody,
    ConstructorDeclaration, ConstructorInvocation, ContinueStatement, DefaultValue, DimExpression,
    DoStatement, EmptyDeclaration, EnhancedForStatement, EnumBody, EnumConstant, EnumConstantList,
    EnumConstantListEntry, EnumDeclaration, ExportsDirective, Expression, ExpressionParentRole,
    ExpressionStatement, ExtendsClause, FieldAccessExpression, FieldDeclaration, FinallyClause,
    ForInitializer, ForStatement, ForUpdate, FormalParameter, FormalParameterList,
    FormalParameterListEntry, FormalParameterListItem, Guard, IfStatement, ImplementsClause,
    ImportDeclaration, ImportKind, InstanceInitializer, InstanceofExpression, InterfaceBody,
    InterfaceBodyMember, InterfaceDeclaration, IntersectionType, IntersectionTypeEntry, JavaFamily,
    JavaNode, JavaOperator, JavaOperatorKind, JavaOperatorPattern, JavaSyntaxKind, JavaSyntaxToken,
    LabeledStatement, LambdaExpression, LambdaParameter, LambdaParameterList, LiteralExpression,
    LocalClassOrInterfaceDeclaration, LocalVariableDeclaration, MatchAllPattern, MemberChain,
    MemberChainSuffix, MethodDeclaration, MethodInvocationExpression, MethodReferenceExpression,
    ModifierEntry, ModifierList, ModuleDeclaration, ModuleDirective, ModuleDirectiveNode,
    ModuleDirectiveRole, ModuleNameListEntry, NameExpression, NameSegment, NameSyntax,
    ObjectCreationExpression, OpensDirective, PackageDeclaration, ParenthesizedExpression, Pattern,
    PermitsClause, PermitsClauseEntry, PostfixExpression, PrimitiveType, ProvidesDirective,
    ReceiverParameter, RecordBody, RecordComponent, RecordComponentList, RecordComponentListEntry,
    RecordDeclaration, RecordPattern, RecordPatternComponentEntry, RequiresDirective, Resource,
    ResourceList, ResourceListEntry, ResourceSpecification, ReturnStatement, Statement,
    StatementBody, StatementExpressionEntry, StatementExpressionList, StaticInitializer,
    SuperExpression, SwitchBlock, SwitchBlockEntry, SwitchBlockStatementGroup,
    SwitchBlockStatementGroupLabel, SwitchExpression, SwitchLabel, SwitchLabelCaseEntry,
    SwitchLabelCaseItem, SwitchRule, SwitchStatement, SynchronizedStatement, ThisExpression,
    ThrowStatement, ThrowsClause, ThrowsClauseEntry, TryStatement, TryWithResourcesStatement, Type,
    TypeArgument, TypeArgumentList, TypeArgumentListEntry, TypeBoundList, TypeClauseEntry,
    TypeDeclaration, TypeParameter, TypeParameterList, TypeParameterListEntry, TypePattern,
    UnaryExpression, UnionType, UnionTypeEntry, UsesDirective, VariableAccess, VariableDeclarator,
    VariableDeclaratorEntry, VariableDeclaratorList, VariableInitializer, VariableInitializerValue,
    VoidType, WhileStatement, WildcardBound, WildcardType, YieldStatement,
    assignment_operator_kind, binary_operator_kind, child, child_family, child_token,
    child_token_in, children, children_family, children_tokens_matching, nth_child_family,
    nth_child_token, starts_after_blank_line,
};
use crate::JavaSyntaxNode;
use jolt_syntax::{SyntaxElement, TriviaKind};

impl CompilationUnit<'_> {
    pub fn items(&self) -> impl Iterator<Item = CompilationUnitItem<'_>> + '_ {
        self.syntax.children().filter_map(|syntax| {
            if let Some(package) = PackageDeclaration::cast(syntax) {
                return Some(CompilationUnitItem::Package(package));
            }
            if let Some(import) = ImportDeclaration::cast(syntax) {
                return Some(CompilationUnitItem::Import(import));
            }
            if let Some(module) = ModuleDeclaration::cast(syntax) {
                return Some(CompilationUnitItem::Module(module));
            }
            if let Some(declaration) = TypeDeclaration::cast(syntax) {
                return Some(CompilationUnitItem::Type(declaration));
            }
            EmptyDeclaration::cast(syntax).map(CompilationUnitItem::EmptyDeclaration)
        })
    }

    #[must_use]
    pub fn package_declaration(&self) -> Option<PackageDeclaration<'_>> {
        child(&self.syntax)
    }

    pub fn imports(&self) -> impl Iterator<Item = ImportDeclaration<'_>> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn module_declaration(&self) -> Option<ModuleDeclaration<'_>> {
        child(&self.syntax)
    }

    pub fn type_declarations(&self) -> impl Iterator<Item = TypeDeclaration<'_>> + '_ {
        children_family(&self.syntax)
    }

    /// Returns descendant nodes as typed Java wrappers.
    ///
    /// Prefer grammar-specific accessors for formatter layout. This traversal is
    /// intended for corpus summaries, diagnostics, and generic syntax tooling.
    pub fn descendants(&self) -> impl Iterator<Item = AnyJavaNode<'_>> + '_ {
        self.syntax.descendants().filter_map(AnyJavaNode::cast)
    }

    /// Returns this compilation unit and its descendants as typed Java wrappers.
    ///
    /// Prefer grammar-specific accessors for formatter layout. This traversal is
    /// intended for corpus summaries, diagnostics, and generic syntax tooling.
    pub fn self_and_descendants(&self) -> impl Iterator<Item = AnyJavaNode<'_>> + '_ {
        std::iter::once(AnyJavaNode::from(self.clone())).chain(self.descendants())
    }
}

impl<'source> ImportDeclaration<'source> {
    #[must_use]
    pub fn import_kind(&self) -> Option<ImportKind<'source>> {
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
    pub fn name(&self) -> Option<NameSyntax<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn import_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::ImportKw)
    }

    #[must_use]
    pub fn module_token(&self) -> Option<JavaSyntaxToken<'source>> {
        let name_start = self.name().map(|name| name.text_range().start());
        self.syntax
            .child_tokens()
            .map(|syntax| JavaSyntaxToken { syntax })
            .find(|token| {
                token.kind() == JavaSyntaxKind::Identifier
                    && token.text() == "module"
                    && name_start
                        .is_some_and(|name_start| token.token_text_range().end() <= name_start)
            })
    }

    #[must_use]
    pub fn static_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::StaticKw)
    }

    #[must_use]
    pub fn star_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Star)
    }

    #[must_use]
    pub fn on_demand_dot_token(&self) -> Option<JavaSyntaxToken<'source>> {
        let star_start = self
            .star_token()
            .map(|token| token.token_text_range().start())?;
        self.syntax
            .child_tokens()
            .map(|syntax| JavaSyntaxToken { syntax })
            .filter(|token| token.kind() == JavaSyntaxKind::Dot)
            .filter(|token| token.token_text_range().start() < star_start)
            .last()
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
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

    fn contextual_keyword(&self, text: &str) -> Option<JavaSyntaxToken<'_>> {
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::Identifier)
            .find(|token| token.text() == text)
    }
}

impl PackageDeclaration<'_> {
    #[must_use]
    pub fn package_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::PackageKw)
    }

    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'_>> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<NameSyntax<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl NameSyntax<'_> {
    pub fn segments(&self) -> impl Iterator<Item = JavaSyntaxToken<'_>> + '_ {
        children_tokens_matching(self.syntax(), |kind| kind == JavaSyntaxKind::Identifier)
    }

    pub fn segments_with_annotations(&self) -> impl Iterator<Item = NameSegment<'_>> {
        let mut segments = Vec::new();
        let mut annotations = Vec::new();
        let mut dot_before = None;

        for element in self.syntax().children_with_tokens() {
            match element {
                SyntaxElement::Node(node) => {
                    if let Some(annotation) = Annotation::cast(node) {
                        annotations.push(annotation);
                    }
                }
                SyntaxElement::Token(syntax) if syntax.kind() == JavaSyntaxKind::Dot => {
                    dot_before = Some(JavaSyntaxToken { syntax });
                }
                SyntaxElement::Token(syntax) if syntax.kind() == JavaSyntaxKind::Identifier => {
                    segments.push(NameSegment {
                        annotations: std::mem::take(&mut annotations),
                        dot_before: dot_before.take(),
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

impl ClassDeclaration<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::ClassKw)
    }

    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn type_parameters(&self) -> Option<TypeParameterList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn extends_clause(&self) -> Option<ExtendsClause<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn implements_clause(&self) -> Option<ImplementsClause<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn permits_clause(&self) -> Option<PermitsClause<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<ClassBody<'_>> {
        child(&self.syntax)
    }
}

impl<'source> RecordDeclaration<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::Identifier)
            .find(|token| token.text() == "record")
    }

    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'_>> {
        nth_child_token(&self.syntax, JavaSyntaxKind::Identifier, 1)
    }

    #[must_use]
    pub fn type_parameters(&self) -> Option<TypeParameterList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    #[must_use]
    pub fn components(&self) -> Option<RecordComponentList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn implements_clause(&self) -> Option<ImplementsClause<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<RecordBody<'_>> {
        child(&self.syntax)
    }
}

impl EnumDeclaration<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::EnumKw)
    }

    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn implements_clause(&self) -> Option<ImplementsClause<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<EnumBody<'_>> {
        child(&self.syntax)
    }
}

impl InterfaceDeclaration<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::InterfaceKw)
    }

    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn type_parameters(&self) -> Option<TypeParameterList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn extends_clause(&self) -> Option<ExtendsClause<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn permits_clause(&self) -> Option<PermitsClause<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<InterfaceBody<'_>> {
        child(&self.syntax)
    }
}

impl ExtendsClause<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::ExtendsKw)
    }

    pub fn types(&self) -> impl Iterator<Item = Type<'_>> + '_ {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = TypeClauseEntry<'_>> {
        type_clause_entries(&self.syntax)
    }
}

impl ImplementsClause<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::ImplementsKw)
    }

    pub fn types(&self) -> impl Iterator<Item = Type<'_>> + '_ {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = TypeClauseEntry<'_>> {
        type_clause_entries(&self.syntax)
    }
}

impl PermitsClause<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        self.syntax
            .first_token()
            .and_then(|syntax| (syntax.text() == "permits").then_some(JavaSyntaxToken { syntax }))
    }

    pub fn names(&self) -> impl Iterator<Item = NameSyntax<'_>> + '_ {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = PermitsClauseEntry<'_>> {
        let mut entries = Vec::new();
        let mut pending_name = None;

        for element in self.syntax.children_with_tokens() {
            match element {
                SyntaxElement::Node(node) => {
                    if let Some(name) = NameSyntax::cast(node)
                        && let Some(previous) = pending_name.replace(name)
                    {
                        entries.push(PermitsClauseEntry {
                            name: previous,
                            comma: None,
                        });
                    }
                }
                SyntaxElement::Token(token) if token.kind() == JavaSyntaxKind::Comma => {
                    if let Some(name) = pending_name.take() {
                        entries.push(PermitsClauseEntry {
                            name,
                            comma: Some(JavaSyntaxToken { syntax: token }),
                        });
                    }
                }
                SyntaxElement::Token(_) => {}
            }
        }

        if let Some(name) = pending_name {
            entries.push(PermitsClauseEntry { name, comma: None });
        }

        entries.into_iter()
    }
}

impl AnnotationInterfaceDeclaration<'_> {
    #[must_use]
    pub fn at_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::At)
    }

    #[must_use]
    pub fn interface_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::InterfaceKw)
    }

    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn body(&self) -> Option<AnnotationInterfaceBody<'_>> {
        child(&self.syntax)
    }
}

impl ModifierList<'_> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'_>> + '_ {
        children(&self.syntax)
    }

    pub fn declaration_annotations(&self) -> impl Iterator<Item = Annotation<'_>> + '_ {
        let first_modifier_start = self
            .modifier_entries()
            .filter_map(|entry| {
                entry
                    .first_token()
                    .map(|token| token.token_text_range().start())
            })
            .min();

        self.annotations().filter(move |annotation| {
            first_modifier_start.is_none_or(|start| annotation.text_range().start() < start)
        })
    }

    pub fn type_use_annotations_after_modifiers(
        &self,
    ) -> impl Iterator<Item = Annotation<'_>> + '_ {
        let first_modifier_start = self
            .modifier_entries()
            .filter_map(|entry| {
                entry
                    .first_token()
                    .map(|token| token.token_text_range().start())
            })
            .min();

        self.annotations().filter(move |annotation| {
            first_modifier_start.is_some_and(|start| annotation.text_range().start() > start)
        })
    }

    pub fn modifier_tokens(&self) -> impl Iterator<Item = JavaSyntaxToken<'_>> + '_ {
        self.modifier_entries().flat_map(ModifierEntry::into_tokens)
    }

    pub fn modifier_entries(&self) -> impl Iterator<Item = ModifierEntry<'_>> + '_ {
        modifier_entries(&self.syntax)
    }
}

impl TypeParameterList<'_> {
    #[must_use]
    pub fn open_angle(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Lt)
    }

    #[must_use]
    pub fn close_angle(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Gt)
    }

    pub fn parameters(&self) -> impl Iterator<Item = TypeParameter<'_>> + '_ {
        children(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = TypeParameterListEntry<'_>> {
        let mut entries = Vec::new();
        let mut pending_parameter = None;

        for element in self.syntax.children_with_tokens() {
            match element {
                SyntaxElement::Node(node) => {
                    if let Some(parameter) = TypeParameter::cast(node)
                        && let Some(previous) = pending_parameter.replace(parameter)
                    {
                        entries.push(TypeParameterListEntry {
                            parameter: previous,
                            comma: None,
                        });
                    }
                }
                SyntaxElement::Token(token) if token.kind() == JavaSyntaxKind::Comma => {
                    if let Some(parameter) = pending_parameter.take() {
                        entries.push(TypeParameterListEntry {
                            parameter,
                            comma: Some(JavaSyntaxToken { syntax: token }),
                        });
                    }
                }
                SyntaxElement::Token(_) => {}
            }
        }

        if let Some(parameter) = pending_parameter {
            entries.push(TypeParameterListEntry {
                parameter,
                comma: None,
            });
        }

        entries.into_iter()
    }
}

impl TypeParameter<'_> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'_>> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn bounds(&self) -> Option<TypeBoundList<'_>> {
        child(&self.syntax)
    }
}

impl TypeBoundList<'_> {
    #[must_use]
    pub fn extends_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::ExtendsKw)
    }

    pub fn bounds(&self) -> impl Iterator<Item = Type<'_>> + '_ {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = IntersectionTypeEntry<'_>> {
        child::<IntersectionType>(&self.syntax)
            .map_or_else(
                || {
                    children_family(&self.syntax)
                        .map(|ty| IntersectionTypeEntry {
                            ty,
                            separator: None,
                        })
                        .collect()
                },
                |intersection| intersection_type_entries(&intersection.syntax).collect::<Vec<_>>(),
            )
            .into_iter()
    }
}

impl PrimitiveType<'_> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'_>> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
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

impl VoidType<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::VoidKw)
    }
}

impl ClassType<'_> {
    pub fn segments(&self) -> impl Iterator<Item = ClassTypeSegment<'_>> {
        let mut segments = Vec::new();
        let mut annotations = Vec::new();
        let mut dot_before = None;
        let mut current: Option<ClassTypeSegment> = None;

        for element in self.syntax.children_with_tokens() {
            let node = match element {
                SyntaxElement::Token(token) if token.kind() == JavaSyntaxKind::Dot => {
                    dot_before = Some(JavaSyntaxToken { syntax: token });
                    continue;
                }
                SyntaxElement::Node(node) => node,
                SyntaxElement::Token(_) => continue,
            };

            if let Some(annotation) = Annotation::cast(node) {
                annotations.push(annotation);
                continue;
            }

            if let Some(name) = NameSyntax::cast(node) {
                if let Some(segment) = current.take() {
                    segments.push(segment);
                }
                current = Some(ClassTypeSegment {
                    annotations: std::mem::take(&mut annotations),
                    dot_before: dot_before.take(),
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

impl TypeArgument<'_> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'_>> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type<'_>> {
        child_family(&self.syntax)
    }
}

impl<'source> WildcardType<'source> {
    #[must_use]
    pub fn question_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Question)
    }

    #[must_use]
    pub fn bound_clause(&self) -> Option<WildcardBound<'source>> {
        let keyword = self.bound_keyword()?;
        let bound = self.bound()?;
        match keyword.kind() {
            JavaSyntaxKind::ExtendsKw => Some(WildcardBound::Extends(bound)),
            JavaSyntaxKind::SuperKw => Some(WildcardBound::Super(bound)),
            _ => None,
        }
    }

    #[must_use]
    pub fn bound_keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token_in(
            &self.syntax,
            &[JavaSyntaxKind::ExtendsKw, JavaSyntaxKind::SuperKw],
        )
    }

    #[must_use]
    pub fn bound(&self) -> Option<Type<'source>> {
        child_family(&self.syntax)
    }
}

impl RecordComponentList<'_> {
    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
            .or_else(|| previous_sibling_token(&self.syntax, JavaSyntaxKind::LParen))
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
            .or_else(|| next_sibling_token(&self.syntax, JavaSyntaxKind::RParen))
    }

    pub fn components(&self) -> impl Iterator<Item = RecordComponent<'_>> + '_ {
        children(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = RecordComponentListEntry<'_>> {
        let mut entries = Vec::new();
        let mut pending_component = None;

        for element in self.syntax.children_with_tokens() {
            match element {
                SyntaxElement::Node(node) => {
                    if let Some(component) = RecordComponent::cast(node)
                        && let Some(previous) = pending_component.replace(component)
                    {
                        entries.push(RecordComponentListEntry {
                            component: previous,
                            comma: None,
                        });
                    }
                }
                SyntaxElement::Token(token) if token.kind() == JavaSyntaxKind::Comma => {
                    if let Some(component) = pending_component.take() {
                        entries.push(RecordComponentListEntry {
                            component,
                            comma: Some(JavaSyntaxToken { syntax: token }),
                        });
                    }
                }
                SyntaxElement::Token(_) => {}
            }
        }

        if let Some(component) = pending_component {
            entries.push(RecordComponentListEntry {
                component,
                comma: None,
            });
        }

        entries.into_iter()
    }
}

impl RecordComponent<'_> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'_>> + '_ {
        children(&self.syntax)
    }

    pub fn prefix_annotations(&self) -> impl Iterator<Item = Annotation<'_>> {
        annotations_before_type(&self.syntax, self.ty())
    }

    pub fn varargs_annotations(&self) -> impl Iterator<Item = Annotation<'_>> {
        annotations_between_type_and_token(&self.syntax, self.ty(), JavaSyntaxKind::Ellipsis)
    }

    pub fn modifier_tokens(&self) -> impl Iterator<Item = JavaSyntaxToken<'_>> + '_ {
        children_tokens_matching(&self.syntax, is_modifier_token)
    }

    #[must_use]
    pub fn is_variable_arity(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::Ellipsis).is_some()
    }

    #[must_use]
    pub fn ellipsis_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Ellipsis)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn dimensions(&self) -> Option<ArrayDimensions<'_>> {
        child(&self.syntax)
    }
}

impl ClassBody<'_> {
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    pub fn members(&self) -> impl Iterator<Item = ClassBodyMember<'_>> + '_ {
        self.syntax.children().filter_map(|syntax| {
            if let Some(declaration) = ClassBodyDeclaration::cast(syntax) {
                return child_family(&declaration.syntax);
            }
            ClassBodyMember::cast(syntax)
        })
    }
}

impl ClassBodyMember<'_> {
    #[must_use]
    pub fn starts_after_blank_line(&self) -> bool {
        starts_after_blank_line(self.syntax())
    }
}

impl RecordBody<'_> {
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    pub fn members(&self) -> impl Iterator<Item = ClassBodyMember<'_>> + '_ {
        self.syntax.children().filter_map(|syntax| {
            if let Some(declaration) = ClassBodyDeclaration::cast(syntax) {
                return child_family(&declaration.syntax);
            }
            ClassBodyMember::cast(syntax)
        })
    }
}

impl ClassBodyDeclaration<'_> {
    #[must_use]
    pub fn member(&self) -> Option<ClassBodyMember<'_>> {
        child_family(&self.syntax)
    }
}

impl InterfaceBody<'_> {
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    pub fn members(&self) -> impl Iterator<Item = InterfaceBodyMember<'_>> + '_ {
        children_family(&self.syntax)
    }
}

impl InterfaceBodyMember<'_> {
    #[must_use]
    pub fn starts_after_blank_line(&self) -> bool {
        starts_after_blank_line(self.syntax())
    }
}

impl AnnotationInterfaceBody<'_> {
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    pub fn members(&self) -> impl Iterator<Item = AnnotationInterfaceBodyMember<'_>> {
        child::<AnnotationElementList>(&self.syntax)
            .map(|list| {
                list.syntax
                    .children()
                    .filter_map(AnnotationInterfaceBodyMember::cast)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
            .into_iter()
    }
}

impl AnnotationInterfaceBodyMember<'_> {
    #[must_use]
    pub fn starts_after_blank_line(&self) -> bool {
        starts_after_blank_line(self.syntax())
    }
}

impl<'source> AnnotationElementList<'source> {
    pub fn members(&self) -> impl Iterator<Item = AnnotationInterfaceBodyMember<'_>> + '_ {
        children_family(&self.syntax)
    }

    pub fn arguments(&self) -> std::vec::IntoIter<AnnotationArgument<'source>> {
        self.syntax
            .children()
            .filter_map(AnnotationArgument::cast)
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[must_use]
    pub fn argument_entries(&self) -> std::vec::IntoIter<AnnotationArgumentListEntry<'source>> {
        let mut entries = Vec::new();
        let mut pending_argument = None;

        for element in self.syntax.children_with_tokens() {
            match element {
                SyntaxElement::Node(node) => {
                    if let Some(argument) = AnnotationArgument::cast(node)
                        && let Some(previous) = pending_argument.replace(argument)
                    {
                        entries.push(AnnotationArgumentListEntry {
                            argument: previous,
                            comma: None,
                        });
                    }
                }
                SyntaxElement::Token(token) if token.kind() == JavaSyntaxKind::Comma => {
                    if let Some(argument) = pending_argument.take() {
                        entries.push(AnnotationArgumentListEntry {
                            argument,
                            comma: Some(JavaSyntaxToken { syntax: token }),
                        });
                    }
                }
                SyntaxElement::Token(_) => {}
            }
        }

        if let Some(argument) = pending_argument {
            entries.push(AnnotationArgumentListEntry {
                argument,
                comma: None,
            });
        }

        entries.into_iter()
    }
}

impl AnnotationElementDeclaration<'_> {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    #[must_use]
    pub fn dimensions(&self) -> Option<ArrayDimensions<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn default_value(&self) -> Option<DefaultValue<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl DefaultValue<'_> {
    #[must_use]
    pub fn default_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::DefaultKw)
    }

    #[must_use]
    pub fn value(&self) -> Option<AnnotationElementValue<'_>> {
        child(&self.syntax)
    }
}

impl EnumBody<'_> {
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    #[must_use]
    pub fn constants(&self) -> Option<EnumConstantList<'_>> {
        child(&self.syntax)
    }

    pub fn members(&self) -> impl Iterator<Item = ClassBodyMember<'_>> + '_ {
        self.syntax.children().filter_map(|syntax| {
            if let Some(declaration) = ClassBodyDeclaration::cast(syntax) {
                return child_family(&declaration.syntax);
            }
            ClassBodyMember::cast(syntax)
        })
    }

    pub fn semicolon_tokens(&self) -> impl Iterator<Item = JavaSyntaxToken<'_>> + '_ {
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::Semicolon)
    }
}

impl<'source> EnumConstantList<'source> {
    pub fn constants(&self) -> impl Iterator<Item = EnumConstant<'_>> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn entries(&self) -> std::vec::IntoIter<EnumConstantListEntry<'source>> {
        let mut entries = Vec::new();
        let mut pending_constant = None;

        for element in self.syntax.children_with_tokens() {
            match element {
                SyntaxElement::Node(node) => {
                    if let Some(constant) = EnumConstant::cast(node)
                        && let Some(previous) = pending_constant.replace(constant)
                    {
                        entries.push(EnumConstantListEntry {
                            constant: previous,
                            comma: None,
                        });
                    }
                }
                SyntaxElement::Token(token) if token.kind() == JavaSyntaxKind::Comma => {
                    if let Some(constant) = pending_constant.take() {
                        entries.push(EnumConstantListEntry {
                            constant,
                            comma: Some(JavaSyntaxToken { syntax: token }),
                        });
                    }
                }
                SyntaxElement::Token(_) => {}
            }
        }

        if let Some(constant) = pending_constant {
            entries.push(EnumConstantListEntry {
                constant,
                comma: None,
            });
        }

        entries.into_iter()
    }
}

impl EnumConstant<'_> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'_>> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn arguments(&self) -> Option<ArgumentList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<ClassBody<'_>> {
        child(&self.syntax)
    }
}

impl BlockItem<'_> {
    #[must_use]
    pub fn starts_after_blank_line(&self) -> bool {
        starts_after_blank_line(self.syntax())
    }
}

impl LocalClassOrInterfaceDeclaration<'_> {
    #[must_use]
    pub fn declaration(&self) -> Option<TypeDeclaration<'_>> {
        child_family(&self.syntax)
    }
}

impl FieldDeclaration<'_> {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn declarators(&self) -> Option<VariableDeclaratorList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl MethodDeclaration<'_> {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn type_parameters(&self) -> Option<TypeParameterList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn return_type(&self) -> Option<Type<'_>> {
        child_family(&self.syntax)
    }

    pub fn return_type_annotations(&self) -> impl Iterator<Item = Annotation<'_>> {
        annotations_before_type(&self.syntax, self.return_type())
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn parameters(&self) -> Option<FormalParameterList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn throws_clause(&self) -> Option<ThrowsClause<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Block<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl ConstructorDeclaration<'_> {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn type_parameters(&self) -> Option<TypeParameterList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn parameters(&self) -> Option<FormalParameterList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn throws_clause(&self) -> Option<ThrowsClause<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<ConstructorBody<'_>> {
        child(&self.syntax)
    }
}

impl CompactConstructorDeclaration<'_> {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn body(&self) -> Option<ConstructorBody<'_>> {
        child(&self.syntax)
    }
}

impl ConstructorBody<'_> {
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    #[must_use]
    pub fn invocation(&self) -> Option<ConstructorInvocation<'_>> {
        child(&self.syntax)
    }

    pub fn block_statements(&self) -> impl Iterator<Item = BlockStatement<'_>> + '_ {
        children(&self.syntax)
    }

    pub fn items(&self) -> impl Iterator<Item = BlockItem<'_>> + '_ {
        children::<BlockStatement>(&self.syntax).filter_map(|node| child_family(&node.syntax))
    }
}

impl ConstructorInvocation<'_> {
    #[must_use]
    pub fn starts_after_blank_line(&self) -> bool {
        starts_after_blank_line(&self.syntax)
    }

    #[must_use]
    pub fn qualifier_name(&self) -> Option<NameSyntax<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn qualifier_expression(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn dot_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Dot)
    }

    #[must_use]
    pub fn type_arguments(&self) -> Option<TypeArgumentList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn target(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token_in(
            &self.syntax,
            &[JavaSyntaxKind::ThisKw, JavaSyntaxKind::SuperKw],
        )
    }

    #[must_use]
    pub fn arguments(&self) -> Option<ArgumentList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl ThrowsClause<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::ThrowsKw)
    }

    pub fn exceptions(&self) -> impl Iterator<Item = Type<'_>> + '_ {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = ThrowsClauseEntry<'_>> {
        let mut entries = Vec::new();
        let mut pending_exception = None;

        for element in self.syntax.children_with_tokens() {
            match element {
                SyntaxElement::Node(node) => {
                    if let Some(exception) = Type::cast(node)
                        && let Some(previous) = pending_exception.replace(exception)
                    {
                        entries.push(ThrowsClauseEntry {
                            exception: previous,
                            comma: None,
                        });
                    }
                }
                SyntaxElement::Token(token) if token.kind() == JavaSyntaxKind::Comma => {
                    if let Some(exception) = pending_exception.take() {
                        entries.push(ThrowsClauseEntry {
                            exception,
                            comma: Some(JavaSyntaxToken { syntax: token }),
                        });
                    }
                }
                SyntaxElement::Token(_) => {}
            }
        }

        if let Some(exception) = pending_exception {
            entries.push(ThrowsClauseEntry {
                exception,
                comma: None,
            });
        }

        entries.into_iter()
    }
}

impl StaticInitializer<'_> {
    #[must_use]
    pub fn static_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::StaticKw)
    }

    #[must_use]
    pub fn body(&self) -> Option<Block<'_>> {
        child(&self.syntax)
    }
}

impl InstanceInitializer<'_> {
    #[must_use]
    pub fn body(&self) -> Option<Block<'_>> {
        child(&self.syntax)
    }
}

impl FormalParameterList<'_> {
    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
            .or_else(|| previous_sibling_token(&self.syntax, JavaSyntaxKind::LParen))
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
            .or_else(|| next_sibling_token(&self.syntax, JavaSyntaxKind::RParen))
    }

    pub fn parameters(&self) -> impl Iterator<Item = FormalParameter<'_>> + '_ {
        children(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = FormalParameterListEntry<'_>> {
        let mut entries = Vec::new();
        let mut pending_item = None;

        for element in self.syntax.children_with_tokens() {
            match element {
                SyntaxElement::Node(node) => {
                    let item = ReceiverParameter::cast(node)
                        .map(FormalParameterListItem::ReceiverParameter)
                        .or_else(|| {
                            FormalParameter::cast(node)
                                .map(FormalParameterListItem::FormalParameter)
                        });

                    if let Some(item) = item
                        && let Some(previous) = pending_item.replace(item)
                    {
                        entries.push(FormalParameterListEntry {
                            item: previous,
                            comma: None,
                        });
                    }
                }
                SyntaxElement::Token(token) if token.kind() == JavaSyntaxKind::Comma => {
                    if let Some(item) = pending_item.take() {
                        entries.push(FormalParameterListEntry {
                            item,
                            comma: Some(JavaSyntaxToken { syntax: token }),
                        });
                    }
                }
                SyntaxElement::Token(_) => {}
            }
        }

        if let Some(item) = pending_item {
            entries.push(FormalParameterListEntry { item, comma: None });
        }

        entries.into_iter()
    }
}

impl FormalParameter<'_> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'_>> + '_ {
        children(&self.syntax)
    }

    pub fn prefix_annotations(&self) -> impl Iterator<Item = Annotation<'_>> {
        annotations_before_type(&self.syntax, self.ty())
    }

    pub fn varargs_annotations(&self) -> impl Iterator<Item = Annotation<'_>> {
        annotations_between_type_and_token(&self.syntax, self.ty(), JavaSyntaxKind::Ellipsis)
    }

    pub fn modifier_tokens(&self) -> impl Iterator<Item = JavaSyntaxToken<'_>> + '_ {
        children_tokens_matching(&self.syntax, is_modifier_token)
    }

    #[must_use]
    pub fn is_variable_arity(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::Ellipsis).is_some()
    }

    #[must_use]
    pub fn ellipsis_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Ellipsis)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'_>> {
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
    pub fn dimensions(&self) -> Option<ArrayDimensions<'_>> {
        child(&self.syntax)
    }
}

impl VariableDeclaratorList<'_> {
    pub fn declarators(&self) -> impl Iterator<Item = VariableDeclarator<'_>> + '_ {
        children(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = VariableDeclaratorEntry<'_>> {
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

impl VariableDeclarator<'_> {
    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'_>> {
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
    pub fn dimensions(&self) -> Option<ArrayDimensions<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn initializer(&self) -> Option<VariableInitializer<'_>> {
        child(&self.syntax)
    }
}

impl VariableInitializer<'_> {
    #[must_use]
    pub fn operator(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Assign)
    }

    #[must_use]
    pub fn value(&self) -> Option<VariableInitializerValue<'_>> {
        child_family(&self.syntax)
    }
}

impl LocalVariableDeclaration<'_> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'_>> + '_ {
        children(&self.syntax)
    }

    pub fn declaration_annotations(&self) -> impl Iterator<Item = Annotation<'_>> + '_ {
        let first_modifier_start = self
            .modifier_tokens()
            .map(|token| token.token_text_range().start())
            .min();

        self.annotations().filter(move |annotation| {
            first_modifier_start.is_none_or(|start| annotation.text_range().start() < start)
        })
    }

    pub fn type_use_annotations_after_modifiers(
        &self,
    ) -> impl Iterator<Item = Annotation<'_>> + '_ {
        let first_modifier_start = self
            .modifier_tokens()
            .map(|token| token.token_text_range().start())
            .min();

        self.annotations().filter(move |annotation| {
            first_modifier_start.is_some_and(|start| annotation.text_range().start() > start)
        })
    }

    pub fn modifier_tokens(&self) -> impl Iterator<Item = JavaSyntaxToken<'_>> + '_ {
        children_tokens_matching(&self.syntax, is_modifier_token)
    }

    #[must_use]
    pub fn var_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier).filter(|token| token.text() == "var")
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn declarators(&self) -> Option<VariableDeclaratorList<'_>> {
        child(&self.syntax)
    }
}

impl IfStatement<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::IfKw)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    #[must_use]
    pub fn else_keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::ElseKw)
    }

    #[must_use]
    pub fn condition(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn then_statement(&self) -> Option<Statement<'_>> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn then_body(&self) -> Option<StatementBody<'_>> {
        self.then_statement().map(StatementBody::from)
    }

    #[must_use]
    pub fn else_statement(&self) -> Option<Statement<'_>> {
        nth_child_family(&self.syntax, 1)
    }

    #[must_use]
    pub fn else_body(&self) -> Option<StatementBody<'_>> {
        self.else_statement().map(StatementBody::from)
    }
}

impl<'source> From<Statement<'source>> for StatementBody<'source> {
    fn from(statement: Statement<'source>) -> Self {
        match statement {
            Statement::Block(block) => Self::Block(block),
            Statement::EmptyStatement(empty) => Self::Empty(empty),
            statement => Self::Unbraced(statement),
        }
    }
}

impl LiteralExpression<'_> {
    #[must_use]
    pub fn literal_token(&self) -> Option<JavaSyntaxToken<'_>> {
        self.syntax
            .first_token()
            .map(|syntax| JavaSyntaxToken { syntax })
    }
}

impl NameExpression<'_> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'_>> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }
}

impl ThisExpression<'_> {
    #[must_use]
    pub fn qualifier(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn dot_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Dot)
    }

    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::ThisKw)
    }
}

impl SuperExpression<'_> {
    #[must_use]
    pub fn qualifier(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn dot_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Dot)
    }

    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::SuperKw)
    }
}

impl ClassLiteralExpression<'_> {
    #[must_use]
    pub fn target_expression(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn void_type(&self) -> Option<VoidType<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn primitive_keyword(&self) -> Option<JavaSyntaxToken<'_>> {
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
    pub fn dimensions(&self) -> Option<ArrayDimensions<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn dot_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Dot)
    }

    #[must_use]
    pub fn class_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::ClassKw)
    }
}

impl<'source> Expression<'source> {
    #[must_use]
    pub fn member_chain(&self) -> Option<MemberChain<'source>> {
        collect_member_chain(self.clone())
    }

    #[must_use]
    pub fn parent_role(&self) -> Option<ExpressionParentRole> {
        let parent = self.syntax().parent()?;
        let parent = AnyJavaNode::cast(parent)?;

        expression_parent_role(self, parent.clone()).or_else(|| statement_parent_role(self, parent))
    }
}

fn expression_parent_role(
    expression: &Expression,
    parent: AnyJavaNode,
) -> Option<ExpressionParentRole> {
    operator_parent_role(expression, parent.clone())
        .or_else(|| access_parent_role(expression, parent))
}

fn operator_parent_role(
    expression: &Expression,
    parent: AnyJavaNode,
) -> Option<ExpressionParentRole> {
    match parent {
        AnyJavaNode::ParenthesizedExpression(parent) => parent
            .expression()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::ParenthesizedExpression),
        AnyJavaNode::AssignmentExpression(parent) => role_for_binary_children(
            expression,
            parent.left(),
            ExpressionParentRole::AssignmentLeft,
            parent.right(),
            ExpressionParentRole::AssignmentRight,
        ),
        AnyJavaNode::ConditionalExpression(parent) => {
            if parent.condition().is_same_expression(expression) {
                Some(ExpressionParentRole::ConditionalCondition)
            } else if parent.true_expression().is_same_expression(expression) {
                Some(ExpressionParentRole::ConditionalTrueExpression)
            } else {
                parent
                    .false_expression()
                    .is_same_expression(expression)
                    .then_some(ExpressionParentRole::ConditionalFalseExpression)
            }
        }
        AnyJavaNode::BinaryExpression(parent) => role_for_binary_children(
            expression,
            parent.left(),
            ExpressionParentRole::BinaryLeft,
            parent.right(),
            ExpressionParentRole::BinaryRight,
        ),
        AnyJavaNode::UnaryExpression(parent) => parent
            .operand()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::UnaryOperand),
        AnyJavaNode::PostfixExpression(parent) => parent
            .operand()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::PostfixOperand),
        AnyJavaNode::CastExpression(parent) => parent
            .expression()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::CastOperand),
        AnyJavaNode::InstanceofExpression(parent) => parent
            .expression()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::InstanceofOperand),
        _ => None,
    }
}

fn access_parent_role(
    expression: &Expression,
    parent: AnyJavaNode,
) -> Option<ExpressionParentRole> {
    match parent {
        AnyJavaNode::FieldAccessExpression(parent) => parent
            .receiver()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::FieldAccessReceiver),
        AnyJavaNode::MethodInvocationExpression(parent) => method_invocation_parent_role(
            expression,
            parent.qualifier(),
            parent.simple_name_expression(),
        ),
        AnyJavaNode::MethodReferenceExpression(parent) => parent
            .receiver_expression()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::MethodReferenceReceiver),
        AnyJavaNode::ArrayAccessExpression(parent) => role_for_binary_children(
            expression,
            parent.array(),
            ExpressionParentRole::ArrayAccessArray,
            parent.index(),
            ExpressionParentRole::ArrayAccessIndex,
        ),
        AnyJavaNode::ObjectCreationExpression(parent) => parent
            .qualifier()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::ObjectCreationQualifier),
        AnyJavaNode::DimExpression(parent) => parent
            .expression()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::ArrayCreationDimension),
        AnyJavaNode::ClassLiteralExpression(parent) => parent
            .target_expression()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::ClassLiteralTarget),
        AnyJavaNode::LambdaExpression(parent) => parent
            .expression_body()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::LambdaBody),
        AnyJavaNode::SwitchExpression(parent) => parent
            .selector()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::SwitchExpressionSelector),
        AnyJavaNode::ArgumentList(parent) => parent
            .arguments()
            .any(|argument| expression_is_same(&argument, expression))
            .then_some(ExpressionParentRole::Argument),
        AnyJavaNode::AnnotationElementValue(parent) => parent
            .expression()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::AnnotationElementValue),
        AnyJavaNode::VariableInitializer(parent) => parent
            .value()
            .and_then(|_| child_family(&parent.syntax))
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::VariableInitializer),
        _ => None,
    }
}

fn statement_parent_role(
    expression: &Expression,
    parent: AnyJavaNode,
) -> Option<ExpressionParentRole> {
    match parent {
        AnyJavaNode::ExpressionStatement(parent) => parent
            .expression()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::ExpressionStatement),
        AnyJavaNode::IfStatement(parent) => parent
            .condition()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::IfCondition),
        AnyJavaNode::WhileStatement(parent) => parent
            .condition()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::WhileCondition),
        AnyJavaNode::DoStatement(parent) => parent
            .condition()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::DoCondition),
        AnyJavaNode::BasicForStatement(parent) => parent
            .condition()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::BasicForCondition),
        AnyJavaNode::EnhancedForStatement(parent) => parent
            .iterable()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::EnhancedForIterable),
        AnyJavaNode::SynchronizedStatement(parent) => parent
            .expression()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::SynchronizedExpression),
        AnyJavaNode::AssertStatement(parent) => {
            if parent.condition().is_same_expression(expression) {
                Some(ExpressionParentRole::AssertCondition)
            } else {
                parent
                    .detail()
                    .is_same_expression(expression)
                    .then_some(ExpressionParentRole::AssertDetail)
            }
        }
        AnyJavaNode::ReturnStatement(parent) => parent
            .expression()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::ReturnValue),
        AnyJavaNode::ThrowStatement(parent) => parent
            .expression()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::ThrowValue),
        AnyJavaNode::YieldStatement(parent) => parent
            .expression()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::YieldValue),
        AnyJavaNode::SwitchStatement(parent) => parent
            .selector()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::SwitchStatementSelector),
        _ => None,
    }
}

fn method_invocation_parent_role(
    expression: &Expression,
    qualifier: Option<Expression>,
    callee: Option<Expression>,
) -> Option<ExpressionParentRole> {
    if qualifier
        .into_iter()
        .any(|qualifier| expression_is_same(&qualifier, expression))
    {
        Some(ExpressionParentRole::MethodInvocationQualifier)
    } else {
        callee
            .into_iter()
            .any(|callee| expression_is_same(&callee, expression))
            .then_some(ExpressionParentRole::MethodInvocationCallee)
    }
}

trait OptionalExpressionExt<'source> {
    fn is_same_expression(&self, target: &Expression<'source>) -> bool;
}

impl<'source> OptionalExpressionExt<'source> for Option<Expression<'source>> {
    fn is_same_expression(&self, target: &Expression<'source>) -> bool {
        self.as_ref()
            .is_some_and(|expression| expression_is_same(expression, target))
    }
}

fn expression_is_same(expression: &Expression, target: &Expression) -> bool {
    expression.kind() == target.kind() && expression.text_range() == target.text_range()
}

fn role_for_binary_children(
    target: &Expression,
    left: Option<Expression>,
    left_role: ExpressionParentRole,
    right: Option<Expression>,
    right_role: ExpressionParentRole,
) -> Option<ExpressionParentRole> {
    if left
        .into_iter()
        .any(|expression| expression_is_same(&expression, target))
    {
        Some(left_role)
    } else {
        right
            .into_iter()
            .any(|expression| expression_is_same(&expression, target))
            .then_some(right_role)
    }
}

fn collect_member_chain(expression: Expression<'_>) -> Option<MemberChain<'_>> {
    match expression {
        Expression::FieldAccessExpression(access) => {
            let receiver = access.receiver()?;
            Some(append_member_chain_suffix(
                receiver,
                MemberChainSuffix::FieldAccess(access),
            ))
        }
        Expression::MethodInvocationExpression(invocation) => {
            invocation.direct_method_name()?;
            let qualifier = invocation.qualifier()?;
            Some(append_member_chain_suffix(
                qualifier,
                MemberChainSuffix::MethodInvocation(invocation),
            ))
        }
        _ => None,
    }
}

fn append_member_chain_suffix<'source>(
    receiver: Expression<'source>,
    suffix: MemberChainSuffix<'source>,
) -> MemberChain<'source> {
    if let Some(mut chain) = collect_member_chain(receiver.clone()) {
        chain.suffixes.push(suffix);
        return chain;
    }

    MemberChain {
        root: receiver,
        suffixes: vec![suffix],
    }
}

impl<'source> MethodInvocationExpression<'source> {
    #[must_use]
    pub fn qualifier(&self) -> Option<Expression<'source>> {
        self.direct_method_name()
            .and_then(|_| child_family(&self.syntax))
    }

    #[must_use]
    pub fn dot_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Dot)
    }

    #[must_use]
    pub fn direct_method_name(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn simple_name_expression(&self) -> Option<Expression<'source>> {
        self.direct_method_name()
            .is_none()
            .then(|| child_family(&self.syntax))
            .flatten()
    }

    #[must_use]
    pub fn type_arguments(&self) -> Option<TypeArgumentList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn arguments(&self) -> Option<ArgumentList<'source>> {
        child(&self.syntax)
    }
}

impl ArgumentList<'_> {
    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    pub fn arguments(&self) -> impl Iterator<Item = Expression<'_>> + '_ {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = ArgumentListEntry<'_>> {
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

impl TypeArgumentList<'_> {
    #[must_use]
    pub fn open_angle(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Lt)
    }

    #[must_use]
    pub fn close_angle(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Gt)
    }

    pub fn arguments(&self) -> impl Iterator<Item = TypeArgument<'_>> + '_ {
        children(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = TypeArgumentListEntry<'_>> {
        let mut entries = Vec::new();
        let mut pending_argument = None;

        for element in self.syntax.children_with_tokens() {
            match element {
                SyntaxElement::Node(node) => {
                    if let Some(argument) = TypeArgument::cast(node)
                        && let Some(previous) = pending_argument.replace(argument)
                    {
                        entries.push(TypeArgumentListEntry {
                            argument: previous,
                            comma: None,
                        });
                    }
                }
                SyntaxElement::Token(token) if token.kind() == JavaSyntaxKind::Comma => {
                    if let Some(argument) = pending_argument.take() {
                        entries.push(TypeArgumentListEntry {
                            argument,
                            comma: Some(JavaSyntaxToken { syntax: token }),
                        });
                    }
                }
                SyntaxElement::Token(_) => {}
            }
        }

        if let Some(argument) = pending_argument {
            entries.push(TypeArgumentListEntry {
                argument,
                comma: None,
            });
        }

        entries.into_iter()
    }
}

impl<'source> FieldAccessExpression<'source> {
    #[must_use]
    pub fn receiver(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn dot_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Dot)
    }

    #[must_use]
    pub fn field_name(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn type_arguments(&self) -> Option<TypeArgumentList<'_>> {
        child(&self.syntax)
    }
}

impl MethodReferenceExpression<'_> {
    #[must_use]
    pub fn double_colon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::DoubleColon)
    }

    #[must_use]
    pub fn receiver_expression(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn receiver_type(&self) -> Option<Type<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn receiver_dimensions(&self) -> Option<ArrayDimensions<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn type_arguments(&self) -> Option<TypeArgumentList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn is_constructor_reference(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::NewKw).is_some()
    }

    #[must_use]
    pub fn new_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::NewKw)
    }

    #[must_use]
    pub fn target_name(&self) -> Option<JavaSyntaxToken<'_>> {
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::Identifier).last()
    }
}

impl ArrayAccessExpression<'_> {
    #[must_use]
    pub fn open_bracket(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LBracket)
    }

    #[must_use]
    pub fn close_bracket(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RBracket)
    }

    #[must_use]
    pub fn array(&self) -> Option<Expression<'_>> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn index(&self) -> Option<Expression<'_>> {
        nth_child_family(&self.syntax, 1)
    }
}

impl ArrayType<'_> {
    #[must_use]
    pub fn element_type(&self) -> Option<Type<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn dimensions(&self) -> Option<ArrayDimensions<'_>> {
        child(&self.syntax)
    }
}

impl ArrayDimensions<'_> {
    pub fn dimensions(&self) -> impl Iterator<Item = ArrayDimension<'_>> + '_ {
        children(&self.syntax)
    }
}

impl ArrayDimension<'_> {
    #[must_use]
    pub fn open_bracket(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LBracket)
    }

    #[must_use]
    pub fn close_bracket(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RBracket)
    }

    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'_>> + '_ {
        children(&self.syntax)
    }
}

impl IntersectionType<'_> {
    pub fn types(&self) -> impl Iterator<Item = Type<'_>> + '_ {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = IntersectionTypeEntry<'_>> {
        intersection_type_entries(&self.syntax)
    }
}

impl Annotation<'_> {
    #[must_use]
    pub fn at_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::At)
    }

    #[must_use]
    pub fn name(&self) -> Option<NameSyntax<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn arguments(&self) -> Option<AnnotationArgumentList<'_>> {
        child(&self.syntax)
    }
}

impl<'source> AnnotationArgumentList<'source> {
    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    pub fn arguments(&self) -> impl Iterator<Item = AnnotationArgument<'source>> {
        child::<AnnotationElementList>(&self.syntax)
            .map(|list| list.arguments().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
    }

    pub fn entries(&self) -> impl Iterator<Item = AnnotationArgumentListEntry<'source>> {
        child::<AnnotationElementList>(&self.syntax)
            .map(|list| list.argument_entries().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
    }
}

impl AnnotationElementValuePair<'_> {
    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn equals_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Assign)
    }

    #[must_use]
    pub fn value(&self) -> Option<AnnotationElementValue<'_>> {
        child(&self.syntax)
    }
}

impl<'source> AnnotationElementValue<'source> {
    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn annotation(&self) -> Option<Annotation<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn array_initializer(&self) -> Option<AnnotationArrayInitializer<'_>> {
        child(&self.syntax)
    }
}

impl AnnotationArrayInitializer<'_> {
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    pub fn values(&self) -> impl Iterator<Item = AnnotationElementValue<'_>> + '_ {
        children(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = AnnotationArrayInitializerEntry<'_>> {
        let mut entries = Vec::new();
        let mut pending_value = None;

        for element in self.syntax.children_with_tokens() {
            match element {
                SyntaxElement::Node(node) => {
                    if let Some(value) = AnnotationElementValue::cast(node)
                        && let Some(previous) = pending_value.replace(value)
                    {
                        entries.push(AnnotationArrayInitializerEntry {
                            value: previous,
                            comma: None,
                        });
                    }
                }
                SyntaxElement::Token(token) if token.kind() == JavaSyntaxKind::Comma => {
                    if let Some(value) = pending_value.take() {
                        entries.push(AnnotationArrayInitializerEntry {
                            value,
                            comma: Some(JavaSyntaxToken { syntax: token }),
                        });
                    }
                }
                SyntaxElement::Token(_) => {}
            }
        }

        if let Some(value) = pending_value {
            entries.push(AnnotationArrayInitializerEntry { value, comma: None });
        }

        entries.into_iter()
    }
}

impl<'source> ParenthesizedExpression<'source> {
    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }
}

impl<'source> AssignmentExpression<'source> {
    #[must_use]
    pub fn left(&self) -> Option<Expression<'source>> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn operator(&self) -> Option<JavaOperator<'source>> {
        assignment_operator(&self.syntax)
    }

    #[must_use]
    pub fn right(&self) -> Option<Expression<'source>> {
        nth_child_family(&self.syntax, 1)
    }
}

impl ConditionalExpression<'_> {
    #[must_use]
    pub fn question_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Question)
    }

    #[must_use]
    pub fn colon_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Colon)
    }

    #[must_use]
    pub fn condition(&self) -> Option<Expression<'_>> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn true_expression(&self) -> Option<Expression<'_>> {
        nth_child_family(&self.syntax, 1)
    }

    #[must_use]
    pub fn false_expression(&self) -> Option<Expression<'_>> {
        nth_child_family(&self.syntax, 2)
    }
}

impl<'source> BinaryExpression<'source> {
    #[must_use]
    pub fn left(&self) -> Option<Expression<'source>> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn operator(&self) -> Option<JavaOperator<'source>> {
        binary_operator(&self.syntax)
    }

    #[must_use]
    pub fn right(&self) -> Option<Expression<'source>> {
        nth_child_family(&self.syntax, 1)
    }
}

impl UnaryExpression<'_> {
    #[must_use]
    pub fn operator(&self) -> Option<JavaSyntaxToken<'_>> {
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
    pub fn operand(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }
}

impl PostfixExpression<'_> {
    #[must_use]
    pub fn operand(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn operator(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token_in(
            &self.syntax,
            &[JavaSyntaxKind::PlusPlus, JavaSyntaxKind::MinusMinus],
        )
    }
}

impl CastExpression<'_> {
    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }
}

impl InstanceofExpression<'_> {
    #[must_use]
    pub fn expression(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn instanceof_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::InstanceofKw)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn pattern(&self) -> Option<Pattern<'_>> {
        child_family(&self.syntax)
    }
}

impl ObjectCreationExpression<'_> {
    #[must_use]
    pub fn new_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::NewKw)
    }

    #[must_use]
    pub fn qualifier(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn constructor_type_arguments(&self) -> Option<TypeArgumentList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn arguments(&self) -> Option<ArgumentList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<ClassBody<'_>> {
        child(&self.syntax)
    }
}

impl ArrayCreationExpression<'_> {
    #[must_use]
    pub fn new_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::NewKw)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type<'_>> {
        child_family(&self.syntax)
    }

    pub fn dimensions(&self) -> impl Iterator<Item = DimExpression<'_>> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn initializer(&self) -> Option<ArrayInitializer<'_>> {
        child(&self.syntax)
    }
}

impl DimExpression<'_> {
    #[must_use]
    pub fn open_bracket(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LBracket)
    }

    #[must_use]
    pub fn close_bracket(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RBracket)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }
}

impl ArrayInitializer<'_> {
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    pub fn values(&self) -> impl Iterator<Item = VariableInitializerValue<'_>> + '_ {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = ArrayInitializerEntry<'_>> {
        let mut entries = Vec::new();
        let mut pending_value = None;

        for element in self.syntax.children_with_tokens() {
            match element {
                SyntaxElement::Node(node) => {
                    if let Some(value) = VariableInitializerValue::cast(node)
                        && let Some(previous) = pending_value.replace(value)
                    {
                        entries.push(ArrayInitializerEntry {
                            value: previous,
                            comma: None,
                        });
                    }
                }
                SyntaxElement::Token(token) if token.kind() == JavaSyntaxKind::Comma => {
                    if let Some(value) = pending_value.take() {
                        entries.push(ArrayInitializerEntry {
                            value,
                            comma: Some(JavaSyntaxToken { syntax: token }),
                        });
                    }
                }
                SyntaxElement::Token(_) => {}
            }
        }

        if let Some(value) = pending_value {
            entries.push(ArrayInitializerEntry { value, comma: None });
        }

        entries.into_iter()
    }
}

impl ReceiverParameter<'_> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'_>> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn qualifier(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn dot(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Dot)
    }

    #[must_use]
    pub fn this_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::ThisKw)
    }
}

impl LambdaExpression<'_> {
    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::LParen).next()
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        let arrow_start = self.arrow().map(|token| token.token_text_range().start());
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::RParen)
            .filter(|token| {
                arrow_start.is_none_or(|arrow_start| token.token_text_range().end() <= arrow_start)
            })
            .last()
    }

    #[must_use]
    pub fn parameters(&self) -> Option<LambdaParameterList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn arrow(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Arrow)
    }

    #[must_use]
    pub fn concise_parameter(&self) -> Option<LambdaParameter<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn expression_body(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn block_body(&self) -> Option<Block<'_>> {
        child(&self.syntax)
    }
}

impl LambdaParameterList<'_> {
    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    pub fn parameters(&self) -> impl Iterator<Item = LambdaParameter<'_>> + '_ {
        children(&self.syntax)
    }
}

impl LambdaParameter<'_> {
    pub fn prefix_annotations(&self) -> impl Iterator<Item = Annotation<'_>> {
        annotations_before_type(&self.syntax, self.ty())
    }

    pub fn varargs_annotations(&self) -> impl Iterator<Item = Annotation<'_>> {
        annotations_between_type_and_token(&self.syntax, self.ty(), JavaSyntaxKind::Ellipsis)
    }

    pub fn modifier_tokens(&self) -> impl Iterator<Item = JavaSyntaxToken<'_>> + '_ {
        children_tokens_matching(&self.syntax, is_modifier_token)
    }

    #[must_use]
    pub fn var_token(&self) -> Option<JavaSyntaxToken<'_>> {
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::Identifier)
            .find(|token| token.text() == "var")
    }

    #[must_use]
    pub fn is_variable_arity(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::Ellipsis).is_some()
    }

    #[must_use]
    pub fn ellipsis_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Ellipsis)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'_>> {
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

impl ExpressionStatement<'_> {
    #[must_use]
    pub fn expression(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl LabeledStatement<'_> {
    #[must_use]
    pub fn label(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn colon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Colon)
    }

    #[must_use]
    pub fn body(&self) -> Option<Statement<'_>> {
        child_family(&self.syntax)
    }
}

impl AssertStatement<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::AssertKw)
    }

    #[must_use]
    pub fn condition(&self) -> Option<Expression<'_>> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn detail(&self) -> Option<Expression<'_>> {
        nth_child_family(&self.syntax, 1)
    }

    #[must_use]
    pub fn colon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Colon)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl BreakStatement<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::BreakKw)
    }

    #[must_use]
    pub fn label(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl ContinueStatement<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::ContinueKw)
    }

    #[must_use]
    pub fn label(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl ReturnStatement<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::ReturnKw)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl ThrowStatement<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::ThrowKw)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl YieldStatement<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::Identifier)
            .find(|token| token.text() == "yield")
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl WhileStatement<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::WhileKw)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    #[must_use]
    pub fn condition(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Statement<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn statement_body(&self) -> Option<StatementBody<'_>> {
        self.body().map(StatementBody::from)
    }
}

impl DoStatement<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::DoKw)
    }

    #[must_use]
    pub fn while_keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::WhileKw)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    #[must_use]
    pub fn body(&self) -> Option<Statement<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn statement_body(&self) -> Option<StatementBody<'_>> {
        self.body().map(StatementBody::from)
    }

    #[must_use]
    pub fn condition(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl SynchronizedStatement<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::SynchronizedKw)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Block<'_>> {
        child(&self.syntax)
    }
}

impl TryStatement<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::TryKw)
    }

    #[must_use]
    pub fn resources_statement(&self) -> Option<TryWithResourcesStatement<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Block<'_>> {
        child(&self.syntax)
    }

    pub fn catch_clauses(&self) -> impl Iterator<Item = CatchClause<'_>> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn finally_clause(&self) -> Option<FinallyClause<'_>> {
        child(&self.syntax)
    }
}

impl TryWithResourcesStatement<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::TryKw)
    }

    #[must_use]
    pub fn resources(&self) -> Option<ResourceSpecification<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Block<'_>> {
        child(&self.syntax)
    }

    pub fn catch_clauses(&self) -> impl Iterator<Item = CatchClause<'_>> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn finally_clause(&self) -> Option<FinallyClause<'_>> {
        child(&self.syntax)
    }
}

impl CatchClause<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::CatchKw)
    }

    #[must_use]
    pub fn parameter(&self) -> Option<CatchParameter<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    #[must_use]
    pub fn body(&self) -> Option<Block<'_>> {
        child(&self.syntax)
    }
}

impl CatchParameter<'_> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'_>> + '_ {
        children(&self.syntax)
    }

    pub fn modifier_tokens(&self) -> impl Iterator<Item = JavaSyntaxToken<'_>> + '_ {
        children_tokens_matching(&self.syntax, is_modifier_token)
    }

    #[must_use]
    pub fn types(&self) -> Option<CatchTypeList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'_>> {
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

impl CatchTypeList<'_> {
    pub fn types(&self) -> impl Iterator<Item = Type<'_>> + '_ {
        child::<UnionType>(&self.syntax)
            .map_or_else(
                || children_family(&self.syntax).collect(),
                |union| {
                    union
                        .syntax
                        .children()
                        .filter_map(Type::cast)
                        .collect::<Vec<_>>()
                },
            )
            .into_iter()
    }

    pub fn entries(&self) -> impl Iterator<Item = UnionTypeEntry<'_>> {
        child::<UnionType>(&self.syntax)
            .map_or_else(
                || {
                    children_family(&self.syntax)
                        .map(|ty| UnionTypeEntry {
                            ty,
                            separator: None,
                        })
                        .collect()
                },
                |union| union_type_entries(&union.syntax).collect::<Vec<_>>(),
            )
            .into_iter()
    }
}

impl UnionType<'_> {
    pub fn types(&self) -> impl Iterator<Item = Type<'_>> + '_ {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = UnionTypeEntry<'_>> {
        union_type_entries(&self.syntax)
    }
}

impl FinallyClause<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::FinallyKw)
    }

    #[must_use]
    pub fn body(&self) -> Option<Block<'_>> {
        child(&self.syntax)
    }
}

impl ResourceSpecification<'_> {
    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn list(&self) -> Option<ResourceList<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn trailing_semicolon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }
}

impl<'source> ResourceList<'source> {
    pub fn resources(&self) -> impl Iterator<Item = Resource<'_>> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn entries(&self) -> std::vec::IntoIter<ResourceListEntry<'source>> {
        let mut entries = Vec::new();
        let mut pending_resource = None;

        for element in self.syntax.children_with_tokens() {
            match element {
                SyntaxElement::Node(node) => {
                    if let Some(resource) = Resource::cast(node)
                        && let Some(previous) = pending_resource.replace(resource)
                    {
                        entries.push(ResourceListEntry {
                            resource: previous,
                            separator: None,
                        });
                    }
                }
                SyntaxElement::Token(token) if token.kind() == JavaSyntaxKind::Semicolon => {
                    if let Some(resource) = pending_resource.take() {
                        entries.push(ResourceListEntry {
                            resource,
                            separator: Some(JavaSyntaxToken { syntax: token }),
                        });
                    }
                }
                SyntaxElement::Token(_) => {}
            }
        }

        if let Some(resource) = pending_resource {
            entries.push(ResourceListEntry {
                resource,
                separator: None,
            });
        }

        entries.into_iter()
    }
}

impl Resource<'_> {
    #[must_use]
    pub fn declaration(&self) -> Option<LocalVariableDeclaration<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn variable_access(&self) -> Option<VariableAccess<'_>> {
        child(&self.syntax)
    }
}

impl VariableAccess<'_> {
    #[must_use]
    pub fn expression(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }
}

impl<'source> ForStatement<'source> {
    #[must_use]
    pub fn basic(&self) -> Option<BasicForStatement<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn enhanced(&self) -> Option<EnhancedForStatement<'source>> {
        child(&self.syntax)
    }
}

impl BasicForStatement<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::ForKw)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    #[must_use]
    pub fn first_semicolon(&self) -> Option<JavaSyntaxToken<'_>> {
        nth_child_token(&self.syntax, JavaSyntaxKind::Semicolon, 0)
    }

    #[must_use]
    pub fn second_semicolon(&self) -> Option<JavaSyntaxToken<'_>> {
        nth_child_token(&self.syntax, JavaSyntaxKind::Semicolon, 1)
    }

    #[must_use]
    pub fn initializer(&self) -> Option<ForInitializer<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn condition(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn update(&self) -> Option<ForUpdate<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Statement<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn statement_body(&self) -> Option<StatementBody<'_>> {
        self.body().map(StatementBody::from)
    }
}

impl EnhancedForStatement<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::ForKw)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    #[must_use]
    pub fn variable(&self) -> Option<LocalVariableDeclaration<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn iterable(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn colon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Colon)
    }

    #[must_use]
    pub fn body(&self) -> Option<Statement<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn statement_body(&self) -> Option<StatementBody<'_>> {
        self.body().map(StatementBody::from)
    }
}

impl ForInitializer<'_> {
    #[must_use]
    pub fn local_variable_declaration(&self) -> Option<LocalVariableDeclaration<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn expressions(&self) -> Option<StatementExpressionList<'_>> {
        child(&self.syntax)
    }
}

impl ForUpdate<'_> {
    #[must_use]
    pub fn expressions(&self) -> Option<StatementExpressionList<'_>> {
        child(&self.syntax)
    }
}

impl StatementExpressionList<'_> {
    pub fn expressions(&self) -> impl Iterator<Item = Expression<'_>> + '_ {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = StatementExpressionEntry<'_>> {
        let mut entries = Vec::new();
        let mut pending_expression = None;

        for element in self.syntax.children_with_tokens() {
            match element {
                SyntaxElement::Node(node) => {
                    if let Some(expression) = Expression::cast(node)
                        && let Some(previous) = pending_expression.replace(expression)
                    {
                        entries.push(StatementExpressionEntry {
                            expression: previous,
                            comma: None,
                        });
                    }
                }
                SyntaxElement::Token(token) if token.kind() == JavaSyntaxKind::Comma => {
                    if let Some(expression) = pending_expression.take() {
                        entries.push(StatementExpressionEntry {
                            expression,
                            comma: Some(JavaSyntaxToken { syntax: token }),
                        });
                    }
                }
                SyntaxElement::Token(_) => {}
            }
        }

        if let Some(expression) = pending_expression {
            entries.push(StatementExpressionEntry {
                expression,
                comma: None,
            });
        }

        entries.into_iter()
    }
}

impl SwitchStatement<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::SwitchKw)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    #[must_use]
    pub fn selector(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn block(&self) -> Option<SwitchBlock<'_>> {
        child(&self.syntax)
    }
}

impl SwitchExpression<'_> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::SwitchKw)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    #[must_use]
    pub fn selector(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn block(&self) -> Option<SwitchBlock<'_>> {
        child(&self.syntax)
    }
}

impl SwitchBlock<'_> {
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    pub fn entries(&self) -> impl Iterator<Item = SwitchBlockEntry<'_>> + '_ {
        self.syntax.children().filter_map(|syntax| {
            if let Some(group) = SwitchBlockStatementGroup::cast(syntax) {
                return Some(SwitchBlockEntry::StatementGroup(group));
            }
            SwitchRule::cast(syntax).map(SwitchBlockEntry::Rule)
        })
    }

    pub fn statement_groups(&self) -> impl Iterator<Item = SwitchBlockStatementGroup<'_>> + '_ {
        children(&self.syntax)
    }

    pub fn rules(&self) -> impl Iterator<Item = SwitchRule<'_>> + '_ {
        children(&self.syntax)
    }
}

impl SwitchBlockStatementGroup<'_> {
    pub fn block_statements(&self) -> impl Iterator<Item = BlockStatement<'_>> + '_ {
        children(&self.syntax)
    }

    pub fn labels(&self) -> impl Iterator<Item = SwitchLabel<'_>> + '_ {
        children(&self.syntax)
    }

    pub fn label_entries(&self) -> impl Iterator<Item = SwitchBlockStatementGroupLabel<'_>> + '_ {
        let mut labels = Vec::new();
        let mut pending_label = None;

        for element in self.syntax.children_with_tokens() {
            match element {
                SyntaxElement::Node(node) => {
                    if let Some(label) = SwitchLabel::cast(node)
                        && let Some(label) = pending_label.replace(label)
                    {
                        labels.push(SwitchBlockStatementGroupLabel { label, colon: None });
                    }
                }
                SyntaxElement::Token(token) if token.kind() == JavaSyntaxKind::Colon => {
                    if let Some(label) = pending_label.take() {
                        labels.push(SwitchBlockStatementGroupLabel {
                            label,
                            colon: Some(JavaSyntaxToken { syntax: token }),
                        });
                    }
                }
                SyntaxElement::Token(_) => {}
            }
        }

        if let Some(label) = pending_label {
            labels.push(SwitchBlockStatementGroupLabel { label, colon: None });
        }

        labels.into_iter()
    }

    pub fn items(&self) -> impl Iterator<Item = BlockItem<'_>> + '_ {
        children::<BlockStatement>(&self.syntax)
            .filter_map(|statement| child_family(&statement.syntax))
    }
}

impl SwitchRule<'_> {
    #[must_use]
    pub fn label(&self) -> Option<SwitchLabel<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn arrow(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Arrow)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }

    #[must_use]
    pub fn block(&self) -> Option<Block<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn throw_statement(&self) -> Option<ThrowStatement<'_>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }
}

impl SwitchLabel<'_> {
    #[must_use]
    pub fn case_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::CaseKw)
    }

    #[must_use]
    pub fn default_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::DefaultKw)
    }

    #[must_use]
    pub fn is_default_label(&self) -> bool {
        self.syntax
            .first_token()
            .is_some_and(|token| token.kind() == JavaSyntaxKind::DefaultKw)
    }

    pub fn case_items(&self) -> impl Iterator<Item = SwitchLabelCaseItem<'_>> {
        self.case_entries()
            .map(|entry| entry.item)
            .collect::<Vec<_>>()
            .into_iter()
    }

    pub fn case_entries(&self) -> impl Iterator<Item = SwitchLabelCaseEntry<'_>> {
        let mut entries = Vec::new();
        let mut pending_item = None;

        for element in self.syntax.children_with_tokens() {
            match element {
                SyntaxElement::Node(node) => {
                    if let Some(item) = CaseConstant::cast(node)
                        .map(SwitchLabelCaseItem::Constant)
                        .or_else(|| CasePattern::cast(node).map(SwitchLabelCaseItem::Pattern))
                        && let Some(previous) = pending_item.replace(item)
                    {
                        entries.push(SwitchLabelCaseEntry {
                            item: previous,
                            comma: None,
                        });
                    }
                }
                SyntaxElement::Token(syntax) if syntax.kind() == JavaSyntaxKind::DefaultKw => {
                    if !self.is_default_label()
                        && let Some(previous) = pending_item
                            .replace(SwitchLabelCaseItem::Default(JavaSyntaxToken { syntax }))
                    {
                        entries.push(SwitchLabelCaseEntry {
                            item: previous,
                            comma: None,
                        });
                    }
                }
                SyntaxElement::Token(token) if token.kind() == JavaSyntaxKind::Comma => {
                    if let Some(item) = pending_item.take() {
                        entries.push(SwitchLabelCaseEntry {
                            item,
                            comma: Some(JavaSyntaxToken { syntax: token }),
                        });
                    }
                }
                SyntaxElement::Token(_) => {}
            }
        }

        if let Some(item) = pending_item {
            entries.push(SwitchLabelCaseEntry { item, comma: None });
        }

        entries.into_iter()
    }

    #[must_use]
    pub fn guard(&self) -> Option<Guard<'_>> {
        child(&self.syntax)
    }
}

impl CaseConstant<'_> {
    #[must_use]
    pub fn expression(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }
}

impl CasePattern<'_> {
    #[must_use]
    pub fn pattern(&self) -> Option<Pattern<'_>> {
        child_family(&self.syntax)
    }
}

impl Guard<'_> {
    #[must_use]
    pub fn when_token(&self) -> Option<JavaSyntaxToken<'_>> {
        contextual_keyword_in(&self.syntax, "when")
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'_>> {
        child_family(&self.syntax)
    }
}

impl TypePattern<'_> {
    #[must_use]
    pub fn variable(&self) -> Option<LocalVariableDeclaration<'_>> {
        child(&self.syntax)
    }
}

impl RecordPattern<'_> {
    #[must_use]
    pub fn ty(&self) -> Option<Type<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    pub fn components(&self) -> impl Iterator<Item = ComponentPattern<'_>> + '_ {
        children(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = RecordPatternComponentEntry<'_>> {
        let mut entries = Vec::new();
        let mut pending_component = None;

        for element in self.syntax.children_with_tokens() {
            match element {
                SyntaxElement::Node(node) => {
                    if let Some(component) = ComponentPattern::cast(node)
                        && let Some(previous) = pending_component.replace(component)
                    {
                        entries.push(RecordPatternComponentEntry {
                            component: previous,
                            comma: None,
                        });
                    }
                }
                SyntaxElement::Token(token) if token.kind() == JavaSyntaxKind::Comma => {
                    if let Some(component) = pending_component.take() {
                        entries.push(RecordPatternComponentEntry {
                            component,
                            comma: Some(JavaSyntaxToken { syntax: token }),
                        });
                    }
                }
                SyntaxElement::Token(_) => {}
            }
        }

        if let Some(component) = pending_component {
            entries.push(RecordPatternComponentEntry {
                component,
                comma: None,
            });
        }

        entries.into_iter()
    }
}

impl ComponentPattern<'_> {
    #[must_use]
    pub fn pattern(&self) -> Option<Pattern<'_>> {
        child_family(&self.syntax)
    }
}

impl MatchAllPattern<'_> {
    #[must_use]
    pub fn underscore(&self) -> Option<JavaSyntaxToken<'_>> {
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

fn modifier_entries<'source>(
    syntax: &JavaSyntaxNode<'source>,
) -> impl Iterator<Item = ModifierEntry<'source>> + use<'source> {
    let mut tokens = syntax
        .children_with_tokens()
        .filter_map(|element| match element {
            SyntaxElement::Token(token) => Some(JavaSyntaxToken { syntax: token }),
            SyntaxElement::Node(_) => None,
        });

    let mut entries = Vec::new();
    let mut pending = None;
    while let Some(token) = pending.take().or_else(|| tokens.next()) {
        if token.text() == "non" {
            let Some(minus) = tokens.next() else {
                continue;
            };

            if minus.kind() != JavaSyntaxKind::Minus {
                pending = Some(minus);
                continue;
            }

            let Some(sealed) = tokens.next() else {
                continue;
            };

            if sealed.text() == "sealed" {
                entries.push(ModifierEntry::non_sealed(token, minus, sealed));
            } else {
                pending = Some(sealed);
            }
            continue;
        }

        if is_modifier_token(token.kind()) || token.text() == "sealed" {
            entries.push(ModifierEntry::single(token));
        }
    }

    entries.into_iter()
}

fn assignment_operator<'source>(syntax: &JavaSyntaxNode<'source>) -> Option<JavaOperator<'source>> {
    operator_from_direct_child_tokens(
        syntax,
        COMPOSITE_ASSIGNMENT_OPERATORS,
        assignment_operator_kind,
    )
}

fn binary_operator<'source>(syntax: &JavaSyntaxNode<'source>) -> Option<JavaOperator<'source>> {
    operator_from_direct_child_tokens(syntax, COMPOSITE_BINARY_OPERATORS, binary_operator_kind)
}

fn operator_from_direct_child_tokens<'source>(
    syntax: &JavaSyntaxNode<'source>,
    composite_patterns: &[JavaOperatorPattern],
    single_kind: fn(JavaSyntaxKind) -> Option<JavaOperatorKind>,
) -> Option<JavaOperator<'source>> {
    let tokens = direct_child_token_prefix(syntax);

    let (kind, len) = composite_patterns
        .iter()
        .find(|pattern| token_sequence_matches(&tokens, pattern.tokens))
        .map(|pattern| (pattern.kind, pattern.tokens.len()))
        .or_else(|| {
            let first = tokens[0].as_ref()?;
            Some((single_kind(first.kind())?, 1))
        })?;

    java_operator(kind, tokens, len)
}

fn direct_child_token_prefix<'source>(
    syntax: &JavaSyntaxNode<'source>,
) -> [Option<JavaSyntaxToken<'source>>; 4] {
    let mut tokens = std::array::from_fn(|_| None);
    for (index, syntax) in syntax.child_tokens().take(4).enumerate() {
        tokens[index] = Some(JavaSyntaxToken { syntax });
    }

    tokens
}

fn token_sequence_matches(
    tokens: &[Option<JavaSyntaxToken<'_>>; 4],
    kinds: &[JavaSyntaxKind],
) -> bool {
    kinds.iter().enumerate().all(|(index, kind)| {
        tokens[index]
            .as_ref()
            .is_some_and(|token| token.kind() == *kind)
    }) && (1..kinds.len()).all(|index| {
        let Some(left) = tokens[index - 1].as_ref() else {
            return false;
        };
        let Some(right) = tokens[index].as_ref() else {
            return false;
        };

        left.token_text_range().end() == right.token_text_range().start()
            && left.syntax.trailing().is_empty()
            && right.syntax.leading().is_empty()
    })
}

fn java_operator(
    kind: JavaOperatorKind,
    mut tokens: [Option<JavaSyntaxToken<'_>>; 4],
    len: usize,
) -> Option<JavaOperator<'_>> {
    let first = tokens[0].take()?;
    if len == 1 {
        return Some(JavaOperator::single(kind, first));
    }

    let last = tokens.get_mut(len.checked_sub(1)?)?.take()?;
    Some(JavaOperator::composite(kind, first, last))
}

impl ModuleDeclaration<'_> {
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.open_token().is_some()
    }

    #[must_use]
    pub fn open_token(&self) -> Option<JavaSyntaxToken<'_>> {
        self.contextual_keyword("open")
    }

    #[must_use]
    pub fn module_token(&self) -> Option<JavaSyntaxToken<'_>> {
        let name_start = self.name().map(|name| name.text_range().start());
        self.contextual_keyword("module").filter(|token| {
            name_start.is_some_and(|name_start| token.token_text_range().end() <= name_start)
        })
    }

    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    #[must_use]
    pub fn name(&self) -> Option<NameSyntax<'_>> {
        child_family(&self.syntax)
    }

    pub fn directives(&self) -> impl Iterator<Item = ModuleDirective<'_>> + '_ {
        children::<ModuleDirectiveNode>(&self.syntax).filter_map(|node| child_family(&node.syntax))
    }

    fn contextual_keyword(&self, text: &str) -> Option<JavaSyntaxToken<'_>> {
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::Identifier)
            .find(|token| token.text() == text)
    }
}

impl ModuleDirectiveNode<'_> {
    #[must_use]
    pub fn directive(&self) -> Option<ModuleDirective<'_>> {
        child_family(&self.syntax)
    }
}

impl ModuleDirective<'_> {
    #[must_use]
    pub fn directive_role(&self) -> Option<ModuleDirectiveRole<'_>> {
        match self {
            Self::RequiresDirective(directive) => directive.directive_role(),
            Self::ExportsDirective(directive) => directive.directive_role(),
            Self::OpensDirective(directive) => directive.directive_role(),
            Self::UsesDirective(directive) => directive.directive_role(),
            Self::ProvidesDirective(directive) => directive.directive_role(),
        }
    }

    pub fn names(&self) -> impl Iterator<Item = NameSyntax<'_>> + '_ {
        children_family(self.syntax())
    }

    #[must_use]
    pub fn primary_name(&self) -> Option<NameSyntax<'_>> {
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

impl RequiresDirective<'_> {
    #[must_use]
    pub fn directive_role(&self) -> Option<ModuleDirectiveRole<'_>> {
        Some(ModuleDirectiveRole::Requires {
            module: self.module_name()?,
            is_static: self.has_static_modifier(),
            is_transitive: self.has_transitive_modifier(),
        })
    }

    #[must_use]
    pub fn module_name(&self) -> Option<NameSyntax<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn requires_token(&self) -> Option<JavaSyntaxToken<'_>> {
        self.contextual_keyword("requires")
    }

    #[must_use]
    pub fn static_token(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::StaticKw)
    }

    #[must_use]
    pub fn transitive_token(&self) -> Option<JavaSyntaxToken<'_>> {
        self.contextual_keyword("transitive")
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }

    #[must_use]
    pub fn has_static_modifier(&self) -> bool {
        self.static_token().is_some()
    }

    #[must_use]
    pub fn has_transitive_modifier(&self) -> bool {
        self.transitive_token().is_some()
    }

    fn contextual_keyword(&self, text: &str) -> Option<JavaSyntaxToken<'_>> {
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::Identifier)
            .find(|token| token.text() == text)
    }
}

impl ExportsDirective<'_> {
    #[must_use]
    pub fn directive_role(&self) -> Option<ModuleDirectiveRole<'_>> {
        let mut names = self.names();
        Some(ModuleDirectiveRole::Exports {
            package: names.next()?,
            targets: names.collect(),
        })
    }

    pub fn names(&self) -> impl Iterator<Item = NameSyntax<'_>> + '_ {
        children_family(&self.syntax)
    }

    #[must_use]
    pub fn exports_token(&self) -> Option<JavaSyntaxToken<'_>> {
        contextual_keyword_in(&self.syntax, "exports")
    }

    #[must_use]
    pub fn to_token(&self) -> Option<JavaSyntaxToken<'_>> {
        contextual_keyword_in(&self.syntax, "to")
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }

    pub fn target_entries(&self) -> impl Iterator<Item = ModuleNameListEntry<'_>> {
        module_name_entries_after_contextual_keyword(&self.syntax, "to")
    }
}

impl OpensDirective<'_> {
    #[must_use]
    pub fn directive_role(&self) -> Option<ModuleDirectiveRole<'_>> {
        let mut names = self.names();
        Some(ModuleDirectiveRole::Opens {
            package: names.next()?,
            targets: names.collect(),
        })
    }

    pub fn names(&self) -> impl Iterator<Item = NameSyntax<'_>> + '_ {
        children_family(&self.syntax)
    }

    #[must_use]
    pub fn opens_token(&self) -> Option<JavaSyntaxToken<'_>> {
        contextual_keyword_in(&self.syntax, "opens")
    }

    #[must_use]
    pub fn to_token(&self) -> Option<JavaSyntaxToken<'_>> {
        contextual_keyword_in(&self.syntax, "to")
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }

    pub fn target_entries(&self) -> impl Iterator<Item = ModuleNameListEntry<'_>> {
        module_name_entries_after_contextual_keyword(&self.syntax, "to")
    }
}

impl UsesDirective<'_> {
    #[must_use]
    pub fn directive_role(&self) -> Option<ModuleDirectiveRole<'_>> {
        Some(ModuleDirectiveRole::Uses {
            service: self.service_name()?,
        })
    }

    #[must_use]
    pub fn service_name(&self) -> Option<NameSyntax<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn uses_token(&self) -> Option<JavaSyntaxToken<'_>> {
        contextual_keyword_in(&self.syntax, "uses")
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl ProvidesDirective<'_> {
    #[must_use]
    pub fn directive_role(&self) -> Option<ModuleDirectiveRole<'_>> {
        Some(ModuleDirectiveRole::Provides {
            service: self.service_name()?,
            implementations: self.implementation_names().collect(),
        })
    }

    #[must_use]
    pub fn service_name(&self) -> Option<NameSyntax<'_>> {
        self.names().next()
    }

    pub fn implementation_names(&self) -> impl Iterator<Item = NameSyntax<'_>> + '_ {
        self.names().skip(1)
    }

    pub fn implementation_entries(&self) -> impl Iterator<Item = ModuleNameListEntry<'_>> {
        module_name_entries_after_contextual_keyword(&self.syntax, "with")
    }

    #[must_use]
    pub fn provides_token(&self) -> Option<JavaSyntaxToken<'_>> {
        contextual_keyword_in(&self.syntax, "provides")
    }

    #[must_use]
    pub fn with_token(&self) -> Option<JavaSyntaxToken<'_>> {
        contextual_keyword_in(&self.syntax, "with")
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }

    fn names(&self) -> impl Iterator<Item = NameSyntax<'_>> + '_ {
        children_family(&self.syntax)
    }
}

impl Block<'_> {
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    pub fn block_statements(&self) -> impl Iterator<Item = BlockStatement<'_>> + '_ {
        children(&self.syntax)
    }

    pub fn items(&self) -> impl Iterator<Item = BlockItem<'_>> + '_ {
        children::<BlockStatement>(&self.syntax).filter_map(|node| child_family(&node.syntax))
    }

    pub fn statements(&self) -> impl Iterator<Item = Statement<'_>> + '_ {
        children::<BlockStatement>(&self.syntax).filter_map(|node| child_family(&node.syntax))
    }
}

impl BlockStatement<'_> {
    #[must_use]
    pub fn item(&self) -> Option<BlockItem<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn starts_after_blank_line(&self) -> bool {
        starts_after_blank_line(&self.syntax)
    }

    #[must_use]
    pub fn statement(&self) -> Option<Statement<'_>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'_>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

fn node_has_leading_comment(syntax: &super::JavaSyntaxNode<'_>) -> bool {
    syntax.first_token().is_some_and(|token| {
        token.leading().iter().any(|trivia| {
            matches!(
                trivia.kind(),
                TriviaKind::LineComment | TriviaKind::BlockComment | TriviaKind::DocComment
            )
        })
    })
}

fn previous_sibling_token<'source>(
    syntax: &super::JavaSyntaxNode<'source>,
    kind: JavaSyntaxKind,
) -> Option<JavaSyntaxToken<'source>> {
    match syntax.prev_sibling_or_token()? {
        SyntaxElement::Token(syntax) if syntax.kind() == kind => Some(JavaSyntaxToken { syntax }),
        _ => None,
    }
}

fn next_sibling_token<'source>(
    syntax: &super::JavaSyntaxNode<'source>,
    kind: JavaSyntaxKind,
) -> Option<JavaSyntaxToken<'source>> {
    match syntax.next_sibling_or_token()? {
        SyntaxElement::Token(syntax) if syntax.kind() == kind => Some(JavaSyntaxToken { syntax }),
        _ => None,
    }
}

fn contextual_keyword_in<'source>(
    syntax: &super::JavaSyntaxNode<'source>,
    text: &str,
) -> Option<JavaSyntaxToken<'source>> {
    syntax
        .child_tokens()
        .find(|token| token.kind() == JavaSyntaxKind::Identifier && token.text() == text)
        .map(|syntax| JavaSyntaxToken { syntax })
}

fn type_clause_entries<'source>(
    syntax: &super::JavaSyntaxNode<'source>,
) -> std::vec::IntoIter<TypeClauseEntry<'source>> {
    let mut entries = Vec::new();
    let mut pending_type = None;

    for element in syntax.children_with_tokens() {
        match element {
            SyntaxElement::Node(node) => {
                if let Some(ty) = Type::cast(node)
                    && let Some(previous) = pending_type.replace(ty)
                {
                    entries.push(TypeClauseEntry {
                        ty: previous,
                        comma: None,
                    });
                }
            }
            SyntaxElement::Token(token) if token.kind() == JavaSyntaxKind::Comma => {
                if let Some(ty) = pending_type.take() {
                    entries.push(TypeClauseEntry {
                        ty,
                        comma: Some(JavaSyntaxToken { syntax: token }),
                    });
                }
            }
            SyntaxElement::Token(_) => {}
        }
    }

    if let Some(ty) = pending_type {
        entries.push(TypeClauseEntry { ty, comma: None });
    }

    entries.into_iter()
}

fn intersection_type_entries<'source>(
    syntax: &super::JavaSyntaxNode<'source>,
) -> std::vec::IntoIter<IntersectionTypeEntry<'source>> {
    let mut entries = Vec::new();
    let mut pending_type = None;

    for element in syntax.children_with_tokens() {
        match element {
            SyntaxElement::Node(node) => {
                if let Some(ty) = Type::cast(node)
                    && let Some(previous) = pending_type.replace(ty)
                {
                    entries.push(IntersectionTypeEntry {
                        ty: previous,
                        separator: None,
                    });
                }
            }
            SyntaxElement::Token(token) if token.kind() == JavaSyntaxKind::Amp => {
                if let Some(ty) = pending_type.take() {
                    entries.push(IntersectionTypeEntry {
                        ty,
                        separator: Some(JavaSyntaxToken { syntax: token }),
                    });
                }
            }
            SyntaxElement::Token(_) => {}
        }
    }

    if let Some(ty) = pending_type {
        entries.push(IntersectionTypeEntry {
            ty,
            separator: None,
        });
    }

    entries.into_iter()
}

fn union_type_entries<'source>(
    syntax: &super::JavaSyntaxNode<'source>,
) -> std::vec::IntoIter<UnionTypeEntry<'source>> {
    let mut entries = Vec::new();
    let mut pending_type = None;

    for element in syntax.children_with_tokens() {
        match element {
            SyntaxElement::Node(node) => {
                if let Some(ty) = Type::cast(node)
                    && let Some(previous) = pending_type.replace(ty)
                {
                    entries.push(UnionTypeEntry {
                        ty: previous,
                        separator: None,
                    });
                }
            }
            SyntaxElement::Token(token) if token.kind() == JavaSyntaxKind::Bar => {
                if let Some(ty) = pending_type.take() {
                    entries.push(UnionTypeEntry {
                        ty,
                        separator: Some(JavaSyntaxToken { syntax: token }),
                    });
                }
            }
            SyntaxElement::Token(_) => {}
        }
    }

    if let Some(ty) = pending_type {
        entries.push(UnionTypeEntry {
            ty,
            separator: None,
        });
    }

    entries.into_iter()
}

fn module_name_entries_after_contextual_keyword<'source>(
    syntax: &super::JavaSyntaxNode<'source>,
    keyword_text: &str,
) -> std::vec::IntoIter<ModuleNameListEntry<'source>> {
    let mut entries = Vec::new();
    let mut after_keyword = false;
    let mut pending_name = None;

    for element in syntax.children_with_tokens() {
        match element {
            SyntaxElement::Token(token)
                if token.kind() == JavaSyntaxKind::Identifier && token.text() == keyword_text =>
            {
                after_keyword = true;
                pending_name = None;
            }
            _ if !after_keyword => {}
            SyntaxElement::Node(node) => {
                if let Some(name) = NameSyntax::cast(node)
                    && let Some(previous) = pending_name.replace(name)
                {
                    entries.push(ModuleNameListEntry {
                        name: previous,
                        comma: None,
                    });
                }
            }
            SyntaxElement::Token(token) if token.kind() == JavaSyntaxKind::Comma => {
                if let Some(name) = pending_name.take() {
                    entries.push(ModuleNameListEntry {
                        name,
                        comma: Some(JavaSyntaxToken { syntax: token }),
                    });
                }
            }
            SyntaxElement::Token(_) => {}
        }
    }

    if let Some(name) = pending_name {
        entries.push(ModuleNameListEntry { name, comma: None });
    }

    entries.into_iter()
}

fn node_leading_comment_texts(syntax: &super::JavaSyntaxNode<'_>) -> Vec<String> {
    syntax
        .first_token()
        .map(|token| {
            token
                .leading()
                .iter()
                .scan(token.offset(), |offset, trivia| {
                    let range = jolt_text::TextRange::new(*offset, *offset + trivia.text_len());
                    *offset = range.end();
                    Some((trivia, range))
                })
                .filter(|(trivia, _)| {
                    matches!(
                        trivia.kind(),
                        TriviaKind::LineComment | TriviaKind::BlockComment | TriviaKind::DocComment
                    )
                })
                .map(|(_, range)| {
                    token.source()[range.start().get()..range.end().get()]
                        .trim()
                        .to_owned()
                })
                .collect()
        })
        .unwrap_or_default()
}

fn annotations_before_type<'source>(
    syntax: &super::JavaSyntaxNode<'source>,
    ty: Option<Type<'source>>,
) -> std::vec::IntoIter<Annotation<'source>> {
    let Some(ty) = ty else {
        return syntax
            .children()
            .filter_map(Annotation::cast)
            .collect::<Vec<_>>()
            .into_iter();
    };
    let type_start = ty.text_range().start();
    syntax
        .children()
        .filter_map(Annotation::cast)
        .filter(|annotation| annotation.text_range().start() < type_start)
        .collect::<Vec<_>>()
        .into_iter()
}

fn annotations_between_type_and_token<'source>(
    syntax: &super::JavaSyntaxNode<'source>,
    ty: Option<Type<'source>>,
    token_kind: JavaSyntaxKind,
) -> std::vec::IntoIter<Annotation<'source>> {
    let (Some(ty), Some(token)) = (ty, child_token(syntax, token_kind)) else {
        return Vec::new().into_iter();
    };
    let type_end = ty.text_range().end();
    let token_start = token.token_text_range().start();
    syntax
        .children()
        .filter_map(Annotation::cast)
        .filter(|annotation| {
            let start = annotation.text_range().start();
            start >= type_end && start < token_start
        })
        .collect::<Vec<_>>()
        .into_iter()
}
