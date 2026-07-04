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
    LocalClassOrInterfaceDeclaration, LocalVariableDeclaration, MatchAllPattern, MethodDeclaration,
    MethodInvocationExpression, MethodReferenceExpression, ModifierEntry, ModifierList,
    ModuleDeclaration, ModuleDirective, ModuleDirectiveNode, ModuleDirectiveRole,
    ModuleNameListEntry, NameExpression, NameSegment, NameSyntax, ObjectCreationExpression,
    OpensDirective, PackageDeclaration, ParenthesizedExpression, Pattern, PermitsClause,
    PermitsClauseEntry, PostfixExpression, PrimitiveType, ProvidesDirective, ReceiverParameter,
    RecordBody, RecordComponent, RecordComponentList, RecordComponentListEntry, RecordDeclaration,
    RecordPattern, RecordPatternComponentEntry, RequiresDirective, Resource, ResourceList,
    ResourceListEntry, ResourceSpecification, ReturnStatement, Statement, StatementBody,
    StatementExpressionEntry, StatementExpressionList, StaticInitializer, SuperExpression,
    SwitchBlock, SwitchBlockEntry, SwitchBlockStatementGroup, SwitchBlockStatementGroupLabel,
    SwitchExpression, SwitchLabel, SwitchLabelCaseEntry, SwitchLabelCaseItem, SwitchRule,
    SwitchStatement, SynchronizedStatement, TemplateExpression, ThisExpression, ThrowStatement,
    ThrowsClause, ThrowsClauseEntry, TryStatement, TryWithResourcesStatement, Type, TypeArgument,
    TypeArgumentList, TypeArgumentListEntry, TypeBoundList, TypeClauseEntry, TypeDeclaration,
    TypeParameter, TypeParameterList, TypeParameterListEntry, TypePattern, UnaryExpression,
    UnionType, UnionTypeEntry, UsesDirective, VariableAccess, VariableDeclarator,
    VariableDeclaratorEntry, VariableDeclaratorList, VariableInitializer, VariableInitializerValue,
    VoidType, WhileStatement, WildcardBound, WildcardType, YieldStatement,
    assignment_operator_kind, binary_operator_kind, child, child_family, child_token,
    child_token_in, children, children_family, children_tokens_matching, nth_child_family,
    nth_child_token, starts_after_blank_line,
};
use crate::{JavaSyntaxNode, language::JavaLanguage};
use jolt_syntax::SyntaxElement;

impl<'source> CompilationUnit<'source> {
    pub fn items(&self) -> impl Iterator<Item = CompilationUnitItem<'source>> + use<'source> {
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
    pub fn package_declaration(&self) -> Option<PackageDeclaration<'source>> {
        child(&self.syntax)
    }

    pub fn imports(&self) -> impl Iterator<Item = ImportDeclaration<'source>> + use<'source> {
        children(&self.syntax)
    }

    #[must_use]
    pub fn module_declaration(&self) -> Option<ModuleDeclaration<'source>> {
        child(&self.syntax)
    }

    pub fn type_declarations(
        &self,
    ) -> impl Iterator<Item = TypeDeclaration<'source>> + use<'source> {
        children_family(&self.syntax)
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

    fn is_static(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::StaticKw).is_some()
    }

    fn is_star(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::Star).is_some()
    }

    fn is_module(&self) -> bool {
        let name_start = self.name().map(|name| name.text_range().start());
        contextual_keyword_in(&self.syntax, "module").is_some_and(|token| {
            name_start.is_some_and(|name_start| token.token_text_range().end() <= name_start)
        })
    }
}

impl<'source> PackageDeclaration<'source> {
    #[must_use]
    pub fn package_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::PackageKw)
    }

    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'source>> + use<'source> {
        children(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<NameSyntax<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl<'source> NameSyntax<'source> {
    pub fn segments(&self) -> impl Iterator<Item = JavaSyntaxToken<'source>> + use<'source> {
        children_tokens_matching(self.syntax(), |kind| kind == JavaSyntaxKind::Identifier)
    }

    pub fn segments_with_annotations(&self) -> impl Iterator<Item = NameSegment<'source>> {
        let mut elements = self.syntax().children_with_tokens();
        let mut annotations = Vec::new();
        let mut dot_before = None;

        std::iter::from_fn(move || {
            loop {
                match elements.next()? {
                    SyntaxElement::Node(node) => {
                        if let Some(annotation) = Annotation::cast(node) {
                            annotations.push(annotation);
                        }
                    }
                    SyntaxElement::Token(syntax) if syntax.kind() == JavaSyntaxKind::Dot => {
                        dot_before = Some(JavaSyntaxToken { syntax });
                    }
                    SyntaxElement::Token(syntax) if syntax.kind() == JavaSyntaxKind::Identifier => {
                        return Some(NameSegment {
                            annotations: std::mem::take(&mut annotations),
                            dot_before: dot_before.take(),
                            identifier: JavaSyntaxToken { syntax },
                        });
                    }
                    SyntaxElement::Token(_) => {}
                }
            }
        })
    }
}

impl<'source> ClassDeclaration<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::ClassKw)
    }

    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn type_parameters(&self) -> Option<TypeParameterList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn extends_clause(&self) -> Option<ExtendsClause<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn implements_clause(&self) -> Option<ImplementsClause<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn permits_clause(&self) -> Option<PermitsClause<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<ClassBody<'source>> {
        child(&self.syntax)
    }
}

impl<'source> RecordDeclaration<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::Identifier)
            .find(|token| token.text() == "record")
    }

    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'source>> {
        nth_child_token(&self.syntax, JavaSyntaxKind::Identifier, 1)
    }

    #[must_use]
    pub fn type_parameters(&self) -> Option<TypeParameterList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
            .or_else(|| previous_sibling_token(&self.syntax, JavaSyntaxKind::LParen))
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
            .or_else(|| next_sibling_token(&self.syntax, JavaSyntaxKind::RParen))
    }

    #[must_use]
    pub fn components(&self) -> Option<RecordComponentList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn implements_clause(&self) -> Option<ImplementsClause<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<RecordBody<'source>> {
        child(&self.syntax)
    }
}

impl<'source> EnumDeclaration<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::EnumKw)
    }

    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn implements_clause(&self) -> Option<ImplementsClause<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<EnumBody<'source>> {
        child(&self.syntax)
    }
}

impl<'source> InterfaceDeclaration<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::InterfaceKw)
    }

    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn type_parameters(&self) -> Option<TypeParameterList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn extends_clause(&self) -> Option<ExtendsClause<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn permits_clause(&self) -> Option<PermitsClause<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<InterfaceBody<'source>> {
        child(&self.syntax)
    }
}

impl<'source> ExtendsClause<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::ExtendsKw)
    }

    pub fn types(&self) -> impl Iterator<Item = Type<'source>> + use<'source, '_> {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = TypeClauseEntry<'source>> {
        type_clause_entries(&self.syntax)
    }
}

impl<'source> ImplementsClause<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::ImplementsKw)
    }

    pub fn types(&self) -> impl Iterator<Item = Type<'source>> + use<'source, '_> {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = TypeClauseEntry<'source>> {
        type_clause_entries(&self.syntax)
    }
}

impl<'source> PermitsClause<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        self.syntax
            .first_token()
            .and_then(|syntax| (syntax.text() == "permits").then_some(JavaSyntaxToken { syntax }))
    }

    pub fn names(&self) -> impl Iterator<Item = NameSyntax<'source>> + use<'source> {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = PermitsClauseEntry<'source>> + use<'source, '_> {
        separated_entries(
            self.syntax.children_with_tokens(),
            JavaSyntaxKind::Comma,
            NameSyntax::cast,
            |name, comma| PermitsClauseEntry { name, comma },
        )
    }
}

impl<'source> AnnotationInterfaceDeclaration<'source> {
    #[must_use]
    pub fn at_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::At)
    }

    #[must_use]
    pub fn interface_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::InterfaceKw)
    }

    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn body(&self) -> Option<AnnotationInterfaceBody<'source>> {
        child(&self.syntax)
    }
}

impl<'source> ModifierList<'source> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'source>> + use<'source> {
        children(&self.syntax)
    }

    pub fn declaration_annotations(
        &self,
    ) -> impl Iterator<Item = Annotation<'source>> + use<'source> {
        let first_modifier_start = self
            .modifier_entries()
            .filter_map(|entry| {
                entry
                    .tokens()
                    .next()
                    .map(|token| token.token_text_range().start())
            })
            .min();

        self.annotations().filter(move |annotation| {
            first_modifier_start.is_none_or(|start| annotation.text_range().start() < start)
        })
    }

    pub fn type_use_annotations_after_modifiers(
        &self,
    ) -> impl Iterator<Item = Annotation<'source>> + use<'source> {
        let first_modifier_start = self
            .modifier_entries()
            .filter_map(|entry| {
                entry
                    .tokens()
                    .next()
                    .map(|token| token.token_text_range().start())
            })
            .min();

        self.annotations().filter(move |annotation| {
            first_modifier_start.is_some_and(|start| annotation.text_range().start() > start)
        })
    }

    pub fn modifier_tokens(&self) -> impl Iterator<Item = JavaSyntaxToken<'source>> + use<'source> {
        self.modifier_entries().flat_map(ModifierEntry::into_tokens)
    }

    pub fn modifier_entries(&self) -> impl Iterator<Item = ModifierEntry<'source>> + use<'source> {
        modifier_entries(&self.syntax)
    }
}

impl<'source> TypeParameterList<'source> {
    #[must_use]
    pub fn open_angle(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Lt)
    }

    #[must_use]
    pub fn close_angle(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Gt)
    }

    pub fn parameters(&self) -> impl Iterator<Item = TypeParameter<'source>> + use<'source> {
        children(&self.syntax)
    }

    pub fn entries(
        &self,
    ) -> impl Iterator<Item = TypeParameterListEntry<'source>> + use<'source, '_> {
        separated_entries(
            self.syntax.children_with_tokens(),
            JavaSyntaxKind::Comma,
            TypeParameter::cast,
            |parameter, comma| TypeParameterListEntry { parameter, comma },
        )
    }
}

impl<'source> TypeParameter<'source> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'source>> + use<'source> {
        children(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn bounds(&self) -> Option<TypeBoundList<'source>> {
        child(&self.syntax)
    }
}

impl<'source> TypeBoundList<'source> {
    #[must_use]
    pub fn extends_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::ExtendsKw)
    }

    pub fn bounds(&self) -> impl Iterator<Item = Type<'source>> + use<'source> {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = IntersectionTypeEntry<'source>> {
        let intersection = child::<IntersectionType>(&self.syntax);
        let has_intersection = intersection.is_some();
        (!has_intersection)
            .then_some(())
            .into_iter()
            .flat_map(|()| {
                children_family(&self.syntax).map(|ty| IntersectionTypeEntry {
                    ty,
                    separator: None,
                })
            })
            .chain(
                intersection
                    .into_iter()
                    .flat_map(|intersection| intersection_type_entries(&intersection.syntax)),
            )
    }
}

impl<'source> PrimitiveType<'source> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'source>> + use<'source> {
        children(&self.syntax)
    }

    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
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

impl<'source> VoidType<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::VoidKw)
    }
}

impl<'source> ClassType<'source> {
    pub fn segments(&self) -> impl Iterator<Item = ClassTypeSegment<'source>> {
        let mut elements = self.syntax.children_with_tokens();
        let mut annotations = Vec::new();
        let mut dot_before = None;
        let mut current: Option<ClassTypeSegment> = None;
        let mut finished = false;

        std::iter::from_fn(move || {
            if finished {
                return None;
            }

            loop {
                let Some(element) = elements.next() else {
                    finished = true;
                    return current.take();
                };

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
                    let next = ClassTypeSegment {
                        annotations: std::mem::take(&mut annotations),
                        dot_before: dot_before.take(),
                        name,
                        type_arguments: None,
                    };
                    if let Some(segment) = current.replace(next) {
                        return Some(segment);
                    }
                    continue;
                }

                if let Some(type_arguments) = TypeArgumentList::cast(node)
                    && let Some(segment) = current.as_mut()
                {
                    segment.type_arguments = Some(type_arguments);
                }
            }
        })
    }
}

impl<'source> TypeArgument<'source> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'source>> + use<'source> {
        children(&self.syntax)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type<'source>> {
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

impl<'source> RecordComponentList<'source> {
    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
            .or_else(|| previous_sibling_token(&self.syntax, JavaSyntaxKind::LParen))
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
            .or_else(|| next_sibling_token(&self.syntax, JavaSyntaxKind::RParen))
    }

    pub fn components(&self) -> impl Iterator<Item = RecordComponent<'source>> + use<'source> {
        children(&self.syntax)
    }

    pub fn entries(
        &self,
    ) -> impl Iterator<Item = RecordComponentListEntry<'source>> + use<'source, '_> {
        separated_entries(
            self.syntax.children_with_tokens(),
            JavaSyntaxKind::Comma,
            RecordComponent::cast,
            |component, comma| RecordComponentListEntry { component, comma },
        )
    }
}

impl<'source> RecordComponent<'source> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'source>> + use<'source> {
        children(&self.syntax)
    }

    pub fn prefix_annotations(&self) -> impl Iterator<Item = Annotation<'source>> {
        annotations_before_type(&self.syntax, self.ty())
    }

    pub fn varargs_annotations(&self) -> impl Iterator<Item = Annotation<'source>> {
        annotations_between_type_and_token(&self.syntax, self.ty(), JavaSyntaxKind::Ellipsis)
    }

    pub fn modifier_tokens(&self) -> impl Iterator<Item = JavaSyntaxToken<'source>> + use<'source> {
        children_tokens_matching(&self.syntax, is_modifier_token)
    }

    #[must_use]
    pub fn is_variable_arity(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::Ellipsis).is_some()
    }

    #[must_use]
    pub fn ellipsis_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Ellipsis)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn dimensions(&self) -> Option<ArrayDimensions<'source>> {
        child(&self.syntax)
    }
}

impl<'source> ClassBody<'source> {
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    pub fn members(&self) -> impl Iterator<Item = ClassBodyMember<'source>> + use<'source> {
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

impl<'source> RecordBody<'source> {
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    pub fn members(&self) -> impl Iterator<Item = ClassBodyMember<'source>> + use<'source> {
        self.syntax.children().filter_map(|syntax| {
            if let Some(declaration) = ClassBodyDeclaration::cast(syntax) {
                return child_family(&declaration.syntax);
            }
            ClassBodyMember::cast(syntax)
        })
    }
}

impl<'source> InterfaceBody<'source> {
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    pub fn members(&self) -> impl Iterator<Item = InterfaceBodyMember<'source>> + use<'source> {
        children_family(&self.syntax)
    }
}

impl InterfaceBodyMember<'_> {
    #[must_use]
    pub fn starts_after_blank_line(&self) -> bool {
        starts_after_blank_line(self.syntax())
    }
}

impl<'source> AnnotationInterfaceBody<'source> {
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    pub fn members(&self) -> impl Iterator<Item = AnnotationInterfaceBodyMember<'source>> {
        child::<AnnotationElementList>(&self.syntax)
            .into_iter()
            .flat_map(|list| {
                list.syntax
                    .children()
                    .filter_map(AnnotationInterfaceBodyMember::cast)
            })
    }
}

impl AnnotationInterfaceBodyMember<'_> {
    #[must_use]
    pub fn starts_after_blank_line(&self) -> bool {
        starts_after_blank_line(self.syntax())
    }
}

impl<'source> AnnotationElementList<'source> {
    pub fn members(
        &self,
    ) -> impl Iterator<Item = AnnotationInterfaceBodyMember<'source>> + use<'source> {
        children_family(&self.syntax)
    }

    pub fn argument_entries(
        &self,
    ) -> impl Iterator<Item = AnnotationArgumentListEntry<'source>> + use<'source> {
        separated_entries(
            self.syntax.children_with_tokens(),
            JavaSyntaxKind::Comma,
            AnnotationArgument::cast,
            |argument, comma| AnnotationArgumentListEntry { argument, comma },
        )
    }
}

impl<'source> AnnotationElementDeclaration<'source> {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
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
    pub fn dimensions(&self) -> Option<ArrayDimensions<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn default_value(&self) -> Option<DefaultValue<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl<'source> DefaultValue<'source> {
    #[must_use]
    pub fn default_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::DefaultKw)
    }

    #[must_use]
    pub fn value(&self) -> Option<AnnotationElementValue<'source>> {
        child(&self.syntax)
    }
}

impl<'source> EnumBody<'source> {
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    #[must_use]
    pub fn constants(&self) -> Option<EnumConstantList<'source>> {
        child(&self.syntax)
    }

    pub fn members(&self) -> impl Iterator<Item = ClassBodyMember<'source>> + use<'source> {
        self.syntax.children().filter_map(|syntax| {
            if let Some(declaration) = ClassBodyDeclaration::cast(syntax) {
                return child_family(&declaration.syntax);
            }
            ClassBodyMember::cast(syntax)
        })
    }

    pub fn semicolon_tokens(
        &self,
    ) -> impl Iterator<Item = JavaSyntaxToken<'source>> + use<'source> {
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::Semicolon)
    }
}

impl<'source> EnumConstantList<'source> {
    pub fn constants(&self) -> impl Iterator<Item = EnumConstant<'source>> + use<'source> {
        children(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = EnumConstantListEntry<'source>> + use<'source> {
        separated_entries(
            self.syntax.children_with_tokens(),
            JavaSyntaxKind::Comma,
            EnumConstant::cast,
            |constant, comma| EnumConstantListEntry { constant, comma },
        )
    }
}

impl<'source> EnumConstant<'source> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'source>> + use<'source> {
        children(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn arguments(&self) -> Option<ArgumentList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<ClassBody<'source>> {
        child(&self.syntax)
    }
}

impl BlockItem<'_> {
    #[must_use]
    pub fn starts_after_blank_line(&self) -> bool {
        starts_after_blank_line(self.syntax())
    }
}

impl<'source> LocalClassOrInterfaceDeclaration<'source> {
    #[must_use]
    pub fn declaration(&self) -> Option<TypeDeclaration<'source>> {
        child_family(&self.syntax)
    }
}

impl<'source> FieldDeclaration<'source> {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn declarators(&self) -> Option<VariableDeclaratorList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl<'source> MethodDeclaration<'source> {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn type_parameters(&self) -> Option<TypeParameterList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn return_type(&self) -> Option<Type<'source>> {
        child_family(&self.syntax)
    }

    pub fn return_type_annotations(&self) -> impl Iterator<Item = Annotation<'source>> {
        annotations_before_type(&self.syntax, self.return_type())
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn parameters(&self) -> Option<FormalParameterList<'source>> {
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
    pub fn throws_clause(&self) -> Option<ThrowsClause<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Block<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl<'source> ConstructorDeclaration<'source> {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn type_parameters(&self) -> Option<TypeParameterList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn parameters(&self) -> Option<FormalParameterList<'source>> {
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
    pub fn throws_clause(&self) -> Option<ThrowsClause<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<ConstructorBody<'source>> {
        child(&self.syntax)
    }
}

impl<'source> CompactConstructorDeclaration<'source> {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn body(&self) -> Option<ConstructorBody<'source>> {
        child(&self.syntax)
    }
}

impl<'source> ConstructorBody<'source> {
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    #[must_use]
    pub fn invocation(&self) -> Option<ConstructorInvocation<'source>> {
        child(&self.syntax)
    }

    pub fn block_statements(&self) -> impl Iterator<Item = BlockStatement<'source>> + use<'source> {
        children(&self.syntax)
    }

    pub fn items(&self) -> impl Iterator<Item = BlockItem<'source>> + use<'source> {
        children::<BlockStatement>(&self.syntax).filter_map(|node| child_family(&node.syntax))
    }
}

impl<'source> ConstructorInvocation<'source> {
    #[must_use]
    pub fn starts_after_blank_line(&self) -> bool {
        starts_after_blank_line(&self.syntax)
    }

    #[must_use]
    pub fn qualifier_name(&self) -> Option<NameSyntax<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn qualifier_expression(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn dot_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Dot)
    }

    #[must_use]
    pub fn type_arguments(&self) -> Option<TypeArgumentList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn target(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token_in(
            &self.syntax,
            &[JavaSyntaxKind::ThisKw, JavaSyntaxKind::SuperKw],
        )
    }

    #[must_use]
    pub fn arguments(&self) -> Option<ArgumentList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl<'source> ThrowsClause<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::ThrowsKw)
    }

    pub fn exceptions(&self) -> impl Iterator<Item = Type<'source>> + use<'source> {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = ThrowsClauseEntry<'source>> + use<'source, '_> {
        separated_entries(
            self.syntax.children_with_tokens(),
            JavaSyntaxKind::Comma,
            Type::cast,
            |exception, comma| ThrowsClauseEntry { exception, comma },
        )
    }
}

impl<'source> StaticInitializer<'source> {
    #[must_use]
    pub fn static_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::StaticKw)
    }

    #[must_use]
    pub fn body(&self) -> Option<Block<'source>> {
        child(&self.syntax)
    }
}

impl<'source> InstanceInitializer<'source> {
    #[must_use]
    pub fn body(&self) -> Option<Block<'source>> {
        child(&self.syntax)
    }
}

impl<'source> FormalParameterList<'source> {
    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
            .or_else(|| previous_sibling_token(&self.syntax, JavaSyntaxKind::LParen))
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
            .or_else(|| next_sibling_token(&self.syntax, JavaSyntaxKind::RParen))
    }

    pub fn parameters(&self) -> impl Iterator<Item = FormalParameter<'source>> + use<'source> {
        children(&self.syntax)
    }

    pub fn entries(
        &self,
    ) -> impl Iterator<Item = FormalParameterListEntry<'source>> + use<'source, '_> {
        separated_entries(
            self.syntax.children_with_tokens(),
            JavaSyntaxKind::Comma,
            |node| {
                ReceiverParameter::cast(node)
                    .map(FormalParameterListItem::ReceiverParameter)
                    .or_else(|| {
                        FormalParameter::cast(node).map(FormalParameterListItem::FormalParameter)
                    })
            },
            |item, comma| FormalParameterListEntry { item, comma },
        )
    }
}

impl<'source> FormalParameter<'source> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'source>> + use<'source> {
        children(&self.syntax)
    }

    pub fn prefix_annotations(&self) -> impl Iterator<Item = Annotation<'source>> {
        annotations_before_type(&self.syntax, self.ty())
    }

    pub fn varargs_annotations(&self) -> impl Iterator<Item = Annotation<'source>> {
        annotations_between_type_and_token(&self.syntax, self.ty(), JavaSyntaxKind::Ellipsis)
    }

    pub fn modifier_tokens(&self) -> impl Iterator<Item = JavaSyntaxToken<'source>> + use<'source> {
        children_tokens_matching(&self.syntax, is_modifier_token)
    }

    #[must_use]
    pub fn is_variable_arity(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::Ellipsis).is_some()
    }

    #[must_use]
    pub fn ellipsis_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Ellipsis)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token_in(
            &self.syntax,
            &[JavaSyntaxKind::Identifier, JavaSyntaxKind::UnderscoreKw],
        )
    }

    #[must_use]
    pub fn dimensions(&self) -> Option<ArrayDimensions<'source>> {
        child(&self.syntax)
    }
}

impl<'source> VariableDeclaratorList<'source> {
    pub fn declarators(&self) -> impl Iterator<Item = VariableDeclarator<'source>> + use<'source> {
        children(&self.syntax)
    }

    pub fn entries(
        &self,
    ) -> impl Iterator<Item = VariableDeclaratorEntry<'source>> + use<'source, '_> {
        separated_entries(
            self.syntax.children_with_tokens(),
            JavaSyntaxKind::Comma,
            VariableDeclarator::cast,
            |declarator, comma| VariableDeclaratorEntry { declarator, comma },
        )
    }
}

impl<'source> VariableDeclarator<'source> {
    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token_in(
            &self.syntax,
            &[JavaSyntaxKind::Identifier, JavaSyntaxKind::UnderscoreKw],
        )
    }

    #[must_use]
    pub fn dimensions(&self) -> Option<ArrayDimensions<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn initializer(&self) -> Option<VariableInitializer<'source>> {
        child(&self.syntax)
    }
}

impl<'source> VariableInitializer<'source> {
    #[must_use]
    pub fn operator(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Assign)
            .or_else(|| previous_sibling_token(&self.syntax, JavaSyntaxKind::Assign))
    }

    #[must_use]
    pub fn value(&self) -> Option<VariableInitializerValue<'source>> {
        child_family(&self.syntax)
    }
}

impl<'source> LocalVariableDeclaration<'source> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'source>> + use<'source> {
        children(&self.syntax)
    }

    pub fn declaration_annotations(
        &self,
    ) -> impl Iterator<Item = Annotation<'source>> + use<'source> {
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
    ) -> impl Iterator<Item = Annotation<'source>> + use<'source> {
        let first_modifier_start = self
            .modifier_tokens()
            .map(|token| token.token_text_range().start())
            .min();

        self.annotations().filter(move |annotation| {
            first_modifier_start.is_some_and(|start| annotation.text_range().start() > start)
        })
    }

    pub fn modifier_tokens(&self) -> impl Iterator<Item = JavaSyntaxToken<'source>> + use<'source> {
        children_tokens_matching(&self.syntax, is_modifier_token)
    }

    #[must_use]
    pub fn var_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier).filter(|token| token.text() == "var")
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn declarators(&self) -> Option<VariableDeclaratorList<'source>> {
        child(&self.syntax)
    }
}

impl<'source> IfStatement<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::IfKw)
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
    pub fn else_keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::ElseKw)
    }

    #[must_use]
    pub fn condition(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }

    fn then_statement(&self) -> Option<Statement<'source>> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn then_body(&self) -> Option<StatementBody<'source>> {
        self.then_statement().map(StatementBody::from)
    }

    fn else_statement(&self) -> Option<Statement<'source>> {
        nth_child_family(&self.syntax, 1)
    }

    #[must_use]
    pub fn else_body(&self) -> Option<StatementBody<'source>> {
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

impl<'source> LiteralExpression<'source> {
    #[must_use]
    pub fn literal_token(&self) -> Option<JavaSyntaxToken<'source>> {
        self.syntax
            .first_token()
            .map(|syntax| JavaSyntaxToken { syntax })
    }
}

impl<'source> NameExpression<'source> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'source>> + use<'source> {
        children(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }
}

impl<'source> ThisExpression<'source> {
    #[must_use]
    pub fn qualifier(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn dot_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Dot)
    }

    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::ThisKw)
    }
}

impl<'source> SuperExpression<'source> {
    #[must_use]
    pub fn qualifier(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn dot_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Dot)
    }

    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::SuperKw)
    }
}

impl<'source> ClassLiteralExpression<'source> {
    #[must_use]
    pub fn target_expression(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn void_type(&self) -> Option<VoidType<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn primitive_keyword(&self) -> Option<JavaSyntaxToken<'source>> {
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
    pub fn dimensions(&self) -> Option<ArrayDimensions<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn dot_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Dot)
    }

    #[must_use]
    pub fn class_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::ClassKw)
    }
}

impl Expression<'_> {
    #[must_use]
    pub fn parent_role(&self) -> Option<ExpressionParentRole> {
        let parent = self.syntax().parent()?;
        let parent = AnyJavaNode::cast(parent)?;

        expression_parent_role(self, parent).or_else(|| statement_parent_role(self, parent))
    }
}

fn expression_parent_role(
    expression: &Expression,
    parent: AnyJavaNode,
) -> Option<ExpressionParentRole> {
    operator_parent_role(expression, parent).or_else(|| access_parent_role(expression, parent))
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

impl<'source> ArgumentList<'source> {
    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    pub fn arguments(&self) -> impl Iterator<Item = Expression<'source>> + use<'source> {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = ArgumentListEntry<'source>> + use<'source, '_> {
        separated_entries(
            self.syntax.children_with_tokens(),
            JavaSyntaxKind::Comma,
            Expression::cast,
            |argument, comma| ArgumentListEntry { argument, comma },
        )
    }
}

impl<'source> TypeArgumentList<'source> {
    #[must_use]
    pub fn open_angle(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Lt)
    }

    #[must_use]
    pub fn close_angle(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Gt)
    }

    pub fn arguments(&self) -> impl Iterator<Item = TypeArgument<'source>> + use<'source> {
        children(&self.syntax)
    }

    pub fn entries(
        &self,
    ) -> impl Iterator<Item = TypeArgumentListEntry<'source>> + use<'source, '_> {
        separated_entries(
            self.syntax.children_with_tokens(),
            JavaSyntaxKind::Comma,
            TypeArgument::cast,
            |argument, comma| TypeArgumentListEntry { argument, comma },
        )
    }
}

impl<'source> TemplateExpression<'source> {
    #[must_use]
    pub fn processor(&self) -> Option<Expression<'source>> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn dot_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Dot)
    }

    #[must_use]
    pub fn template(&self) -> Option<LiteralExpression<'source>> {
        child(&self.syntax)
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
    pub fn field_name(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn type_arguments(&self) -> Option<TypeArgumentList<'source>> {
        child(&self.syntax)
    }
}

impl<'source> MethodReferenceExpression<'source> {
    #[must_use]
    pub fn double_colon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::DoubleColon)
    }

    #[must_use]
    pub fn receiver_expression(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn receiver_type(&self) -> Option<Type<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn receiver_dimensions(&self) -> Option<ArrayDimensions<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn type_arguments(&self) -> Option<TypeArgumentList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn is_constructor_reference(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::NewKw).is_some()
    }

    #[must_use]
    pub fn new_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::NewKw)
    }

    #[must_use]
    pub fn target_name(&self) -> Option<JavaSyntaxToken<'source>> {
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::Identifier).last()
    }
}

impl<'source> ArrayAccessExpression<'source> {
    #[must_use]
    pub fn open_bracket(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LBracket)
    }

    #[must_use]
    pub fn close_bracket(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RBracket)
    }

    #[must_use]
    pub fn array(&self) -> Option<Expression<'source>> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn index(&self) -> Option<Expression<'source>> {
        nth_child_family(&self.syntax, 1)
    }
}

impl<'source> ArrayType<'source> {
    #[must_use]
    pub fn element_type(&self) -> Option<Type<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn dimensions(&self) -> Option<ArrayDimensions<'source>> {
        child(&self.syntax)
    }
}

impl<'source> ArrayDimensions<'source> {
    pub fn dimensions(&self) -> impl Iterator<Item = ArrayDimension<'source>> + use<'source> {
        children(&self.syntax)
    }
}

impl<'source> ArrayDimension<'source> {
    #[must_use]
    pub fn open_bracket(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LBracket)
    }

    #[must_use]
    pub fn close_bracket(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RBracket)
    }

    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'source>> + use<'source> {
        children(&self.syntax)
    }
}

impl<'source> IntersectionType<'source> {
    pub fn types(&self) -> impl Iterator<Item = Type<'source>> + use<'source> {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = IntersectionTypeEntry<'source>> {
        intersection_type_entries(&self.syntax)
    }
}

impl<'source> Annotation<'source> {
    #[must_use]
    pub fn at_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::At)
    }

    #[must_use]
    pub fn name(&self) -> Option<NameSyntax<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn arguments(&self) -> Option<AnnotationArgumentList<'source>> {
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

    pub fn entries(&self) -> impl Iterator<Item = AnnotationArgumentListEntry<'source>> {
        child::<AnnotationElementList>(&self.syntax)
            .into_iter()
            .flat_map(|list| list.argument_entries())
    }
}

impl<'source> AnnotationElementValuePair<'source> {
    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn equals_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Assign)
    }

    #[must_use]
    pub fn value(&self) -> Option<AnnotationElementValue<'source>> {
        child(&self.syntax)
    }
}

impl<'source> AnnotationElementValue<'source> {
    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn annotation(&self) -> Option<Annotation<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn array_initializer(&self) -> Option<AnnotationArrayInitializer<'source>> {
        child(&self.syntax)
    }
}

impl<'source> AnnotationArrayInitializer<'source> {
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    pub fn entries(
        &self,
    ) -> impl Iterator<Item = AnnotationArrayInitializerEntry<'source>> + use<'source, '_> {
        separated_entries(
            self.syntax.children_with_tokens(),
            JavaSyntaxKind::Comma,
            AnnotationElementValue::cast,
            |value, comma| AnnotationArrayInitializerEntry { value, comma },
        )
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

impl<'source> ConditionalExpression<'source> {
    #[must_use]
    pub fn question_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Question)
    }

    #[must_use]
    pub fn colon_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Colon)
    }

    #[must_use]
    pub fn condition(&self) -> Option<Expression<'source>> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn true_expression(&self) -> Option<Expression<'source>> {
        nth_child_family(&self.syntax, 1)
    }

    #[must_use]
    pub fn false_expression(&self) -> Option<Expression<'source>> {
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

impl<'source> UnaryExpression<'source> {
    #[must_use]
    pub fn operator(&self) -> Option<JavaSyntaxToken<'source>> {
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
    pub fn operand(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }
}

impl<'source> PostfixExpression<'source> {
    #[must_use]
    pub fn operand(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn operator(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token_in(
            &self.syntax,
            &[JavaSyntaxKind::PlusPlus, JavaSyntaxKind::MinusMinus],
        )
    }
}

impl<'source> CastExpression<'source> {
    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }
}

impl<'source> InstanceofExpression<'source> {
    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn instanceof_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::InstanceofKw)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn pattern(&self) -> Option<Pattern<'source>> {
        child_family(&self.syntax)
    }
}

impl<'source> ObjectCreationExpression<'source> {
    #[must_use]
    pub fn new_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::NewKw)
    }

    #[must_use]
    pub fn qualifier(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn dot_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Dot)
    }

    #[must_use]
    pub fn constructor_type_arguments(&self) -> Option<TypeArgumentList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn arguments(&self) -> Option<ArgumentList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<ClassBody<'source>> {
        child(&self.syntax)
    }
}

impl<'source> ArrayCreationExpression<'source> {
    #[must_use]
    pub fn new_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::NewKw)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type<'source>> {
        child_family(&self.syntax)
    }

    pub fn dimensions(&self) -> impl Iterator<Item = DimExpression<'source>> + use<'source> {
        children(&self.syntax)
    }

    #[must_use]
    pub fn initializer(&self) -> Option<ArrayInitializer<'source>> {
        child(&self.syntax)
    }
}

impl<'source> DimExpression<'source> {
    #[must_use]
    pub fn open_bracket(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LBracket)
    }

    #[must_use]
    pub fn close_bracket(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RBracket)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }
}

impl<'source> ArrayInitializer<'source> {
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    pub fn entries(
        &self,
    ) -> impl Iterator<Item = ArrayInitializerEntry<'source>> + use<'source, '_> {
        separated_entries(
            self.syntax.children_with_tokens(),
            JavaSyntaxKind::Comma,
            VariableInitializerValue::cast,
            |value, comma| ArrayInitializerEntry { value, comma },
        )
    }
}

impl<'source> ReceiverParameter<'source> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'source>> + use<'source> {
        children(&self.syntax)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn qualifier(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn dot(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Dot)
    }

    #[must_use]
    pub fn this_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::ThisKw)
    }
}

impl<'source> LambdaExpression<'source> {
    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'source>> {
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::LParen).next()
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'source>> {
        let arrow_start = self.arrow().map(|token| token.token_text_range().start());
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::RParen)
            .filter(|token| {
                arrow_start.is_none_or(|arrow_start| token.token_text_range().end() <= arrow_start)
            })
            .last()
    }

    #[must_use]
    pub fn parameters(&self) -> Option<LambdaParameterList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn arrow(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Arrow)
    }

    #[must_use]
    pub fn concise_parameter(&self) -> Option<LambdaParameter<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn expression_body(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn block_body(&self) -> Option<Block<'source>> {
        child(&self.syntax)
    }
}

impl<'source> LambdaParameterList<'source> {
    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    pub fn parameters(&self) -> impl Iterator<Item = LambdaParameter<'source>> + use<'source> {
        children(&self.syntax)
    }
}

impl<'source> LambdaParameter<'source> {
    pub fn prefix_annotations(&self) -> impl Iterator<Item = Annotation<'source>> {
        annotations_before_type(&self.syntax, self.ty())
    }

    pub fn varargs_annotations(&self) -> impl Iterator<Item = Annotation<'source>> {
        annotations_between_type_and_token(&self.syntax, self.ty(), JavaSyntaxKind::Ellipsis)
    }

    pub fn modifier_tokens(&self) -> impl Iterator<Item = JavaSyntaxToken<'source>> + use<'source> {
        children_tokens_matching(&self.syntax, is_modifier_token)
    }

    #[must_use]
    pub fn var_token(&self) -> Option<JavaSyntaxToken<'source>> {
        let mut identifiers =
            children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::Identifier);
        let first = identifiers.next()?;
        (first.text() == "var" && identifiers.next().is_some()).then_some(first)
    }

    #[must_use]
    pub fn is_variable_arity(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::Ellipsis).is_some()
    }

    #[must_use]
    pub fn ellipsis_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Ellipsis)
    }

    #[must_use]
    pub fn ty(&self) -> Option<Type<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'source>> {
        children_tokens_matching(&self.syntax, |kind| {
            matches!(
                kind,
                JavaSyntaxKind::Identifier | JavaSyntaxKind::UnderscoreKw
            )
        })
        .last()
    }
}

impl<'source> ExpressionStatement<'source> {
    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl<'source> LabeledStatement<'source> {
    #[must_use]
    pub fn label(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn colon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Colon)
    }

    #[must_use]
    pub fn body(&self) -> Option<Statement<'source>> {
        child_family(&self.syntax)
    }
}

impl<'source> AssertStatement<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::AssertKw)
    }

    #[must_use]
    pub fn condition(&self) -> Option<Expression<'source>> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn detail(&self) -> Option<Expression<'source>> {
        nth_child_family(&self.syntax, 1)
    }

    #[must_use]
    pub fn colon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Colon)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl<'source> BreakStatement<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::BreakKw)
    }

    #[must_use]
    pub fn label(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl<'source> ContinueStatement<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::ContinueKw)
    }

    #[must_use]
    pub fn label(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl<'source> ReturnStatement<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::ReturnKw)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl<'source> ThrowStatement<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::ThrowKw)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl<'source> YieldStatement<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::Identifier)
            .find(|token| token.text() == "yield")
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl<'source> WhileStatement<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::WhileKw)
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
    pub fn condition(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Statement<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn statement_body(&self) -> Option<StatementBody<'source>> {
        self.body().map(StatementBody::from)
    }
}

impl<'source> DoStatement<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::DoKw)
    }

    #[must_use]
    pub fn while_keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::WhileKw)
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
    pub fn body(&self) -> Option<Statement<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn statement_body(&self) -> Option<StatementBody<'source>> {
        self.body().map(StatementBody::from)
    }

    #[must_use]
    pub fn condition(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl<'source> SynchronizedStatement<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::SynchronizedKw)
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
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Block<'source>> {
        child(&self.syntax)
    }
}

impl<'source> TryStatement<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::TryKw)
    }

    #[must_use]
    pub fn resources_statement(&self) -> Option<TryWithResourcesStatement<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Block<'source>> {
        child(&self.syntax)
    }

    pub fn catch_clauses(&self) -> impl Iterator<Item = CatchClause<'source>> + use<'source> {
        children(&self.syntax)
    }

    #[must_use]
    pub fn finally_clause(&self) -> Option<FinallyClause<'source>> {
        child(&self.syntax)
    }
}

impl<'source> TryWithResourcesStatement<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::TryKw)
    }

    #[must_use]
    pub fn resources(&self) -> Option<ResourceSpecification<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Block<'source>> {
        child(&self.syntax)
    }

    pub fn catch_clauses(&self) -> impl Iterator<Item = CatchClause<'source>> + use<'source> {
        children(&self.syntax)
    }

    #[must_use]
    pub fn finally_clause(&self) -> Option<FinallyClause<'source>> {
        child(&self.syntax)
    }
}

impl<'source> CatchClause<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::CatchKw)
    }

    #[must_use]
    pub fn parameter(&self) -> Option<CatchParameter<'source>> {
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
    pub fn body(&self) -> Option<Block<'source>> {
        child(&self.syntax)
    }
}

impl<'source> CatchParameter<'source> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'source>> + use<'source> {
        children(&self.syntax)
    }

    pub fn modifier_tokens(&self) -> impl Iterator<Item = JavaSyntaxToken<'source>> + use<'source> {
        children_tokens_matching(&self.syntax, is_modifier_token)
    }

    #[must_use]
    pub fn types(&self) -> Option<CatchTypeList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token_in(
            &self.syntax,
            &[JavaSyntaxKind::Identifier, JavaSyntaxKind::UnderscoreKw],
        )
    }
}

impl<'source> CatchTypeList<'source> {
    pub fn types(&self) -> impl Iterator<Item = Type<'source>> + use<'source, '_> {
        let union = child::<UnionType>(&self.syntax);
        let has_union = union.is_some();
        (!has_union)
            .then_some(())
            .into_iter()
            .flat_map(|()| children_family(&self.syntax))
            .chain(
                union
                    .into_iter()
                    .flat_map(|union| union.syntax.children().filter_map(Type::cast)),
            )
    }

    pub fn entries(&self) -> impl Iterator<Item = UnionTypeEntry<'source>> {
        let union = child::<UnionType>(&self.syntax);
        let has_union = union.is_some();
        (!has_union)
            .then_some(())
            .into_iter()
            .flat_map(|()| {
                children_family(&self.syntax).map(|ty| UnionTypeEntry {
                    ty,
                    separator: None,
                })
            })
            .chain(
                union
                    .into_iter()
                    .flat_map(|union| union_type_entries(&union.syntax)),
            )
    }
}

impl<'source> UnionType<'source> {
    pub fn types(&self) -> impl Iterator<Item = Type<'source>> + use<'source> {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = UnionTypeEntry<'source>> {
        union_type_entries(&self.syntax)
    }
}

impl<'source> FinallyClause<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::FinallyKw)
    }

    #[must_use]
    pub fn body(&self) -> Option<Block<'source>> {
        child(&self.syntax)
    }
}

impl<'source> ResourceSpecification<'source> {
    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn list(&self) -> Option<ResourceList<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn trailing_semicolon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }
}

impl<'source> ResourceList<'source> {
    pub fn resources(&self) -> impl Iterator<Item = Resource<'source>> + use<'source> {
        children(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = ResourceListEntry<'source>> + use<'source, '_> {
        separated_entries(
            self.syntax.children_with_tokens(),
            JavaSyntaxKind::Semicolon,
            Resource::cast,
            |resource, separator| ResourceListEntry {
                resource,
                separator,
            },
        )
    }
}

impl<'source> Resource<'source> {
    #[must_use]
    pub fn declaration(&self) -> Option<LocalVariableDeclaration<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn variable_access(&self) -> Option<VariableAccess<'source>> {
        child(&self.syntax)
    }
}

impl<'source> VariableAccess<'source> {
    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
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

impl<'source> BasicForStatement<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::ForKw)
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
    pub fn first_semicolon(&self) -> Option<JavaSyntaxToken<'source>> {
        nth_child_token(&self.syntax, JavaSyntaxKind::Semicolon, 0)
    }

    #[must_use]
    pub fn second_semicolon(&self) -> Option<JavaSyntaxToken<'source>> {
        nth_child_token(&self.syntax, JavaSyntaxKind::Semicolon, 1)
    }

    #[must_use]
    pub fn initializer(&self) -> Option<ForInitializer<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn condition(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn update(&self) -> Option<ForUpdate<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Statement<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn statement_body(&self) -> Option<StatementBody<'source>> {
        self.body().map(StatementBody::from)
    }
}

impl<'source> EnhancedForStatement<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::ForKw)
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
    pub fn variable(&self) -> Option<LocalVariableDeclaration<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn iterable(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn colon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Colon)
    }

    #[must_use]
    pub fn body(&self) -> Option<Statement<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn statement_body(&self) -> Option<StatementBody<'source>> {
        self.body().map(StatementBody::from)
    }
}

impl<'source> ForInitializer<'source> {
    #[must_use]
    pub fn local_variable_declaration(&self) -> Option<LocalVariableDeclaration<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn expressions(&self) -> Option<StatementExpressionList<'source>> {
        child(&self.syntax)
    }
}

impl<'source> ForUpdate<'source> {
    #[must_use]
    pub fn expressions(&self) -> Option<StatementExpressionList<'source>> {
        child(&self.syntax)
    }
}

impl<'source> StatementExpressionList<'source> {
    pub fn expressions(&self) -> impl Iterator<Item = Expression<'source>> + use<'source> {
        children_family(&self.syntax)
    }

    pub fn entries(
        &self,
    ) -> impl Iterator<Item = StatementExpressionEntry<'source>> + use<'source, '_> {
        separated_entries(
            self.syntax.children_with_tokens(),
            JavaSyntaxKind::Comma,
            Expression::cast,
            |expression, comma| StatementExpressionEntry { expression, comma },
        )
    }
}

impl<'source> SwitchStatement<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::SwitchKw)
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
    pub fn selector(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn block(&self) -> Option<SwitchBlock<'source>> {
        child(&self.syntax)
    }
}

impl<'source> SwitchExpression<'source> {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::SwitchKw)
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
    pub fn selector(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn block(&self) -> Option<SwitchBlock<'source>> {
        child(&self.syntax)
    }
}

impl<'source> SwitchBlock<'source> {
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    pub fn entries(&self) -> impl Iterator<Item = SwitchBlockEntry<'source>> + use<'source> {
        self.syntax.children().filter_map(|syntax| {
            if let Some(group) = SwitchBlockStatementGroup::cast(syntax) {
                return Some(SwitchBlockEntry::StatementGroup(group));
            }
            SwitchRule::cast(syntax).map(SwitchBlockEntry::Rule)
        })
    }
}

impl<'source> SwitchBlockStatementGroup<'source> {
    pub fn block_statements(&self) -> impl Iterator<Item = BlockStatement<'source>> + use<'source> {
        children(&self.syntax)
    }

    pub fn label_entries(
        &self,
    ) -> impl Iterator<Item = SwitchBlockStatementGroupLabel<'source>> + use<'source, '_> {
        separated_entries(
            self.syntax.children_with_tokens(),
            JavaSyntaxKind::Colon,
            SwitchLabel::cast,
            |label, colon| SwitchBlockStatementGroupLabel { label, colon },
        )
    }

    pub fn items(&self) -> impl Iterator<Item = BlockItem<'source>> + use<'source> {
        children::<BlockStatement>(&self.syntax)
            .filter_map(|statement| child_family(&statement.syntax))
    }
}

impl<'source> SwitchRule<'source> {
    #[must_use]
    pub fn label(&self) -> Option<SwitchLabel<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn arrow(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Arrow)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }

    #[must_use]
    pub fn block(&self) -> Option<Block<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn throw_statement(&self) -> Option<ThrowStatement<'source>> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }
}

impl<'source> SwitchLabel<'source> {
    #[must_use]
    pub fn case_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::CaseKw)
    }

    #[must_use]
    pub fn default_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::DefaultKw)
    }

    #[must_use]
    pub fn is_default_label(&self) -> bool {
        self.syntax
            .first_token()
            .is_some_and(|token| token.kind() == JavaSyntaxKind::DefaultKw)
    }

    pub fn case_entries(&self) -> impl Iterator<Item = SwitchLabelCaseEntry<'source>> {
        let mut elements = self.syntax.children_with_tokens();
        let mut pending_item = None;
        let is_default_label = self.is_default_label();
        let mut done = false;

        std::iter::from_fn(move || {
            if done {
                return None;
            }

            for element in elements.by_ref() {
                match element {
                    SyntaxElement::Node(node) => {
                        if let Some(item) = CaseConstant::cast(node)
                            .map(SwitchLabelCaseItem::Constant)
                            .or_else(|| CasePattern::cast(node).map(SwitchLabelCaseItem::Pattern))
                            && let Some(previous) = pending_item.replace(item)
                        {
                            return Some(SwitchLabelCaseEntry {
                                item: previous,
                                comma: None,
                            });
                        }
                    }
                    SyntaxElement::Token(syntax) if syntax.kind() == JavaSyntaxKind::DefaultKw => {
                        if !is_default_label
                            && let Some(previous) = pending_item
                                .replace(SwitchLabelCaseItem::Default(JavaSyntaxToken { syntax }))
                        {
                            return Some(SwitchLabelCaseEntry {
                                item: previous,
                                comma: None,
                            });
                        }
                    }
                    SyntaxElement::Token(token) if token.kind() == JavaSyntaxKind::Comma => {
                        if let Some(item) = pending_item.take() {
                            return Some(SwitchLabelCaseEntry {
                                item,
                                comma: Some(JavaSyntaxToken { syntax: token }),
                            });
                        }
                    }
                    SyntaxElement::Token(_) => {}
                }
            }

            done = true;
            pending_item
                .take()
                .map(|item| SwitchLabelCaseEntry { item, comma: None })
        })
    }

    #[must_use]
    pub fn guard(&self) -> Option<Guard<'source>> {
        child(&self.syntax)
    }
}

impl<'source> CaseConstant<'source> {
    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }
}

impl<'source> CasePattern<'source> {
    #[must_use]
    pub fn pattern(&self) -> Option<Pattern<'source>> {
        child_family(&self.syntax)
    }
}

impl<'source> Guard<'source> {
    #[must_use]
    pub fn when_token(&self) -> Option<JavaSyntaxToken<'source>> {
        contextual_keyword_in(&self.syntax, "when")
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(&self.syntax)
    }
}

impl<'source> TypePattern<'source> {
    #[must_use]
    pub fn variable(&self) -> Option<LocalVariableDeclaration<'source>> {
        child(&self.syntax)
    }
}

impl<'source> RecordPattern<'source> {
    #[must_use]
    pub fn ty(&self) -> Option<Type<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    pub fn components(&self) -> impl Iterator<Item = ComponentPattern<'source>> + use<'source> {
        children(&self.syntax)
    }

    pub fn entries(
        &self,
    ) -> impl Iterator<Item = RecordPatternComponentEntry<'source>> + use<'source, '_> {
        separated_entries(
            self.syntax.children_with_tokens(),
            JavaSyntaxKind::Comma,
            ComponentPattern::cast,
            |component, comma| RecordPatternComponentEntry { component, comma },
        )
    }
}

impl<'source> ComponentPattern<'source> {
    #[must_use]
    pub fn pattern(&self) -> Option<Pattern<'source>> {
        child_family(&self.syntax)
    }
}

impl<'source> MatchAllPattern<'source> {
    #[must_use]
    pub fn underscore(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::UnderscoreKw)
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

    let mut pending = None;

    std::iter::from_fn(move || {
        loop {
            let token = pending.take().or_else(|| tokens.next())?;

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
                    return Some(ModifierEntry::non_sealed(token, minus, sealed));
                }

                pending = Some(sealed);
                continue;
            }

            if is_modifier_token(token.kind()) || token.text() == "sealed" {
                return Some(ModifierEntry::single(token));
            }
        }
    })
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

impl<'source> ModuleDeclaration<'source> {
    #[must_use]
    pub fn open_token(&self) -> Option<JavaSyntaxToken<'source>> {
        contextual_keyword_in(&self.syntax, "open")
    }

    #[must_use]
    pub fn module_token(&self) -> Option<JavaSyntaxToken<'source>> {
        let name_start = self.name().map(|name| name.text_range().start());
        contextual_keyword_in(&self.syntax, "module").filter(|token| {
            name_start.is_some_and(|name_start| token.token_text_range().end() <= name_start)
        })
    }

    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    #[must_use]
    pub fn name(&self) -> Option<NameSyntax<'source>> {
        child_family(&self.syntax)
    }

    pub fn directives(&self) -> impl Iterator<Item = ModuleDirective<'source>> + use<'source> {
        children::<ModuleDirectiveNode>(&self.syntax).filter_map(|node| child_family(&node.syntax))
    }
}

impl<'source> ModuleDirectiveNode<'source> {
    #[must_use]
    pub fn directive(&self) -> Option<ModuleDirective<'source>> {
        child_family(&self.syntax)
    }
}

impl<'source> ModuleDirective<'source> {
    #[must_use]
    pub fn directive_role(&self) -> Option<ModuleDirectiveRole<'source>> {
        match self {
            Self::RequiresDirective(directive) => directive.directive_role(),
            Self::ExportsDirective(directive) => directive.directive_role(),
            Self::OpensDirective(directive) => directive.directive_role(),
            Self::UsesDirective(directive) => directive.directive_role(),
            Self::ProvidesDirective(directive) => directive.directive_role(),
        }
    }

    pub fn names(&self) -> impl Iterator<Item = NameSyntax<'source>> + use<'source> {
        children_family(self.syntax())
    }
}

impl<'source> RequiresDirective<'source> {
    #[must_use]
    pub fn directive_role(&self) -> Option<ModuleDirectiveRole<'source>> {
        Some(ModuleDirectiveRole::Requires {
            module: self.module_name()?,
            is_static: self.static_token().is_some(),
            is_transitive: self.transitive_token().is_some(),
        })
    }

    #[must_use]
    pub fn module_name(&self) -> Option<NameSyntax<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn requires_token(&self) -> Option<JavaSyntaxToken<'source>> {
        contextual_keyword_in(&self.syntax, "requires")
    }

    #[must_use]
    pub fn static_token(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::StaticKw)
    }

    #[must_use]
    pub fn transitive_token(&self) -> Option<JavaSyntaxToken<'source>> {
        contextual_keyword_in(&self.syntax, "transitive")
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl<'source> ExportsDirective<'source> {
    #[must_use]
    pub fn directive_role(&self) -> Option<ModuleDirectiveRole<'source>> {
        let mut names = self.names();
        Some(ModuleDirectiveRole::Exports {
            package: names.next()?,
        })
    }

    pub fn names(&self) -> impl Iterator<Item = NameSyntax<'source>> + use<'source> {
        children_family(&self.syntax)
    }

    #[must_use]
    pub fn exports_token(&self) -> Option<JavaSyntaxToken<'source>> {
        contextual_keyword_in(&self.syntax, "exports")
    }

    #[must_use]
    pub fn to_token(&self) -> Option<JavaSyntaxToken<'source>> {
        contextual_keyword_in(&self.syntax, "to")
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }

    pub fn target_entries(&self) -> impl Iterator<Item = ModuleNameListEntry<'source>> {
        module_name_entries_after_contextual_keyword(&self.syntax, "to")
    }
}

impl<'source> OpensDirective<'source> {
    #[must_use]
    pub fn directive_role(&self) -> Option<ModuleDirectiveRole<'source>> {
        let mut names = self.names();
        Some(ModuleDirectiveRole::Opens {
            package: names.next()?,
        })
    }

    pub fn names(&self) -> impl Iterator<Item = NameSyntax<'source>> + use<'source> {
        children_family(&self.syntax)
    }

    #[must_use]
    pub fn opens_token(&self) -> Option<JavaSyntaxToken<'source>> {
        contextual_keyword_in(&self.syntax, "opens")
    }

    #[must_use]
    pub fn to_token(&self) -> Option<JavaSyntaxToken<'source>> {
        contextual_keyword_in(&self.syntax, "to")
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }

    pub fn target_entries(&self) -> impl Iterator<Item = ModuleNameListEntry<'source>> {
        module_name_entries_after_contextual_keyword(&self.syntax, "to")
    }
}

impl<'source> UsesDirective<'source> {
    #[must_use]
    pub fn directive_role(&self) -> Option<ModuleDirectiveRole<'source>> {
        Some(ModuleDirectiveRole::Uses {
            service: self.service_name()?,
        })
    }

    #[must_use]
    pub fn service_name(&self) -> Option<NameSyntax<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn uses_token(&self) -> Option<JavaSyntaxToken<'source>> {
        contextual_keyword_in(&self.syntax, "uses")
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl<'source> ProvidesDirective<'source> {
    #[must_use]
    pub fn directive_role(&self) -> Option<ModuleDirectiveRole<'source>> {
        Some(ModuleDirectiveRole::Provides {
            service: self.service_name()?,
        })
    }

    #[must_use]
    pub fn service_name(&self) -> Option<NameSyntax<'source>> {
        self.names().next()
    }

    pub fn implementation_entries(&self) -> impl Iterator<Item = ModuleNameListEntry<'source>> {
        module_name_entries_after_contextual_keyword(&self.syntax, "with")
    }

    #[must_use]
    pub fn provides_token(&self) -> Option<JavaSyntaxToken<'source>> {
        contextual_keyword_in(&self.syntax, "provides")
    }

    #[must_use]
    pub fn with_token(&self) -> Option<JavaSyntaxToken<'source>> {
        contextual_keyword_in(&self.syntax, "with")
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }

    fn names(&self) -> impl Iterator<Item = NameSyntax<'source>> + use<'source> {
        children_family(&self.syntax)
    }
}

impl<'source> Block<'source> {
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    pub fn block_statements(&self) -> impl Iterator<Item = BlockStatement<'source>> + use<'source> {
        children(&self.syntax)
    }

    pub fn items(&self) -> impl Iterator<Item = BlockItem<'source>> + use<'source> {
        children::<BlockStatement>(&self.syntax).filter_map(|node| child_family(&node.syntax))
    }

    pub fn statements(&self) -> impl Iterator<Item = Statement<'source>> + use<'source> {
        children::<BlockStatement>(&self.syntax).filter_map(|node| child_family(&node.syntax))
    }
}

impl<'source> BlockStatement<'source> {
    #[must_use]
    pub fn item(&self) -> Option<BlockItem<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn starts_after_blank_line(&self) -> bool {
        starts_after_blank_line(&self.syntax)
    }

    #[must_use]
    pub fn statement(&self) -> Option<Statement<'source>> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken<'source>> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
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
) -> impl Iterator<Item = TypeClauseEntry<'source>> + use<'source> {
    separated_entries(
        syntax.children_with_tokens(),
        JavaSyntaxKind::Comma,
        Type::cast,
        |ty, comma| TypeClauseEntry { ty, comma },
    )
}

fn intersection_type_entries<'source>(
    syntax: &super::JavaSyntaxNode<'source>,
) -> impl Iterator<Item = IntersectionTypeEntry<'source>> + use<'source> {
    separated_entries(
        syntax.children_with_tokens(),
        JavaSyntaxKind::Amp,
        Type::cast,
        |ty, separator| IntersectionTypeEntry { ty, separator },
    )
}

fn union_type_entries<'source>(
    syntax: &super::JavaSyntaxNode<'source>,
) -> impl Iterator<Item = UnionTypeEntry<'source>> + use<'source> {
    separated_entries(
        syntax.children_with_tokens(),
        JavaSyntaxKind::Bar,
        Type::cast,
        |ty, separator| UnionTypeEntry { ty, separator },
    )
}

fn module_name_entries_after_contextual_keyword<'source, 'keyword>(
    syntax: &super::JavaSyntaxNode<'source>,
    keyword_text: &'keyword str,
) -> impl Iterator<Item = ModuleNameListEntry<'source>> + use<'source, 'keyword> {
    let mut elements = syntax.children_with_tokens();
    let mut after_keyword = false;
    let mut pending_name = None;
    let mut done = false;

    std::iter::from_fn(move || {
        if done {
            return None;
        }

        for element in elements.by_ref() {
            match element {
                SyntaxElement::Token(token)
                    if token.kind() == JavaSyntaxKind::Identifier
                        && token.text() == keyword_text =>
                {
                    after_keyword = true;
                    pending_name = None;
                }
                _ if !after_keyword => {}
                SyntaxElement::Node(node) => {
                    if let Some(name) = NameSyntax::cast(node)
                        && let Some(previous) = pending_name.replace(name)
                    {
                        return Some(ModuleNameListEntry {
                            name: previous,
                            comma: None,
                        });
                    }
                }
                SyntaxElement::Token(token) if token.kind() == JavaSyntaxKind::Comma => {
                    if let Some(name) = pending_name.take() {
                        return Some(ModuleNameListEntry {
                            name,
                            comma: Some(JavaSyntaxToken { syntax: token }),
                        });
                    }
                }
                SyntaxElement::Token(_) => {}
            }
        }

        done = true;
        pending_name
            .take()
            .map(|name| ModuleNameListEntry { name, comma: None })
    })
}

fn separated_entries<'source, Elements, Item, Entry, Cast, Make>(
    mut elements: Elements,
    separator_kind: JavaSyntaxKind,
    mut cast: Cast,
    mut make: Make,
) -> impl Iterator<Item = Entry> + use<'source, Elements, Item, Entry, Cast, Make>
where
    Elements: Iterator<Item = SyntaxElement<'source, JavaLanguage>>,
    Cast: FnMut(JavaSyntaxNode<'source>) -> Option<Item>,
    Make: FnMut(Item, Option<JavaSyntaxToken<'source>>) -> Entry,
{
    let mut pending_item = None;
    let mut done = false;

    std::iter::from_fn(move || {
        if done {
            return None;
        }

        for element in elements.by_ref() {
            match element {
                SyntaxElement::Node(node) => {
                    if let Some(item) = cast(node)
                        && let Some(previous) = pending_item.replace(item)
                    {
                        return Some(make(previous, None));
                    }
                }
                SyntaxElement::Token(token) if token.kind() == separator_kind => {
                    if let Some(item) = pending_item.take() {
                        return Some(make(item, Some(JavaSyntaxToken { syntax: token })));
                    }
                }
                SyntaxElement::Token(_) => {}
            }
        }

        done = true;
        pending_item.take().map(|item| make(item, None))
    })
}

fn annotations_before_type<'source>(
    syntax: &super::JavaSyntaxNode<'source>,
    ty: Option<Type<'source>>,
) -> impl Iterator<Item = Annotation<'source>> + use<'source> {
    let type_start = ty.map(|ty| ty.text_range().start());
    syntax
        .children()
        .filter_map(Annotation::cast)
        .filter(move |annotation| {
            type_start.is_none_or(|type_start| annotation.text_range().start() < type_start)
        })
}

fn annotations_between_type_and_token<'source>(
    syntax: &super::JavaSyntaxNode<'source>,
    ty: Option<Type<'source>>,
    token_kind: JavaSyntaxKind,
) -> impl Iterator<Item = Annotation<'source>> + use<'source> {
    let range = ty
        .zip(child_token(syntax, token_kind))
        .map(|(ty, token)| ty.text_range().end()..token.token_text_range().start());
    syntax
        .children()
        .filter_map(Annotation::cast)
        .filter(move |annotation| {
            let start = annotation.text_range().start();
            range
                .as_ref()
                .is_some_and(|range| start >= range.start && start < range.end)
        })
}
