use super::{
    Annotation, AnnotationArgument, AnnotationArgumentList, AnnotationArgumentListEntry,
    AnnotationArrayInitializer, AnnotationArrayInitializerEntry, AnnotationElementDeclaration,
    AnnotationElementList, AnnotationElementValue, AnnotationElementValuePair,
    AnnotationInterfaceBody, AnnotationInterfaceBodyMember, AnnotationInterfaceDeclaration,
    AnyJavaNode, ArgumentList, ArgumentListEntry, ArrayAccessExpression, ArrayCreationExpression,
    ArrayDimension, ArrayDimensions, ArrayInitializer, ArrayInitializerEntry, ArrayType,
    AssertStatement, AssignmentExpression, BasicForStatement, BinaryExpression, Block, BlockItem,
    BlockStatement, BreakStatement, CaseConstant, CasePattern, CastExpression, CatchClause,
    CatchParameter, CatchTypeList, ClassBody, ClassBodyDeclaration, ClassBodyMember,
    ClassDeclaration, ClassLiteralExpression, ClassType, ClassTypeSegment,
    CompactConstructorDeclaration, CompilationUnit, CompilationUnitItem, ComponentPattern,
    ConditionalExpression, ConstructorBody, ConstructorDeclaration, ConstructorInvocation,
    ContinueStatement, DefaultValue, DimExpression, DoStatement, EmptyDeclaration,
    EnhancedForStatement, EnumBody, EnumConstant, EnumConstantList, EnumConstantListEntry,
    EnumDeclaration, ExportsDirective, Expression, ExpressionParentRole, ExpressionStatement,
    ExtendsClause, FieldAccessExpression, FieldDeclaration, FinallyClause, ForInitializer,
    ForStatement, ForUpdate, FormalParameter, FormalParameterList, FormalParameterListEntry,
    FormalParameterListItem, Guard, IfStatement, ImplementsClause, ImportDeclaration, ImportKind,
    InstanceInitializer, InstanceofExpression, InterfaceBody, InterfaceBodyMember,
    InterfaceDeclaration, IntersectionType, IntersectionTypeEntry, JavaFamily, JavaNode,
    JavaSyntaxKind, JavaSyntaxToken, LabeledStatement, LambdaExpression, LambdaParameter,
    LambdaParameterList, LiteralExpression, LocalClassOrInterfaceDeclaration,
    LocalVariableDeclaration, MatchAllPattern, MemberChain, MemberChainSuffix, MethodDeclaration,
    MethodInvocationExpression, MethodReferenceExpression, ModifierList, ModuleDeclaration,
    ModuleDirective, ModuleDirectiveNode, ModuleDirectiveRole, ModuleNameListEntry, NameExpression,
    NameSegment, NameSyntax, ObjectCreationExpression, OpensDirective, PackageDeclaration,
    ParenthesizedExpression, Pattern, PermitsClause, PermitsClauseEntry, PostfixExpression,
    PrimitiveType, ProvidesDirective, ReceiverParameter, RecordBody, RecordComponent,
    RecordComponentList, RecordComponentListEntry, RecordDeclaration, RecordPattern,
    RecordPatternComponentEntry, RequiresDirective, Resource, ResourceList, ResourceListEntry,
    ResourceSpecification, ReturnStatement, Statement, StatementBody, StatementExpressionEntry,
    StatementExpressionList, StaticInitializer, SuperExpression, SwitchBlock, SwitchBlockEntry,
    SwitchBlockStatementGroup, SwitchExpression, SwitchLabel, SwitchLabelCaseEntry,
    SwitchLabelCaseItem, SwitchRule, SwitchStatement, SynchronizedStatement, ThisExpression,
    ThrowStatement, ThrowsClause, ThrowsClauseEntry, TryStatement, TryWithResourcesStatement, Type,
    TypeArgument, TypeArgumentList, TypeArgumentListEntry, TypeBoundList, TypeClauseEntry,
    TypeDeclaration, TypeParameter, TypeParameterList, TypeParameterListEntry, TypePattern,
    UnaryExpression, UnionType, UnionTypeEntry, UsesDirective, VariableAccess, VariableDeclarator,
    VariableDeclaratorEntry, VariableDeclaratorList, VariableInitializer, VariableInitializerValue,
    VoidType, WhileStatement, WildcardBound, WildcardType, YieldStatement, child, child_family,
    child_token, child_token_in, children, children_family, children_tokens_matching,
    nth_child_family, nth_child_token, starts_after_blank_line, tokens,
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
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::ExtendsKw)
    }

    pub fn types(&self) -> impl Iterator<Item = Type> + '_ {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = TypeClauseEntry> {
        type_clause_entries(&self.syntax)
    }
}

impl ImplementsClause {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::ImplementsKw)
    }

    pub fn types(&self) -> impl Iterator<Item = Type> + '_ {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = TypeClauseEntry> {
        type_clause_entries(&self.syntax)
    }
}

impl PermitsClause {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        self.syntax
            .first_token()
            .and_then(|syntax| (syntax.text() == "permits").then_some(JavaSyntaxToken { syntax }))
    }

    pub fn names(&self) -> impl Iterator<Item = NameSyntax> + '_ {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = PermitsClauseEntry> {
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

    pub fn declaration_annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        let first_modifier_start = self
            .modifier_tokens()
            .map(|token| token.token_text_range().start())
            .min();

        self.annotations().filter(move |annotation| {
            first_modifier_start.is_none_or(|start| annotation.text_range().start() < start)
        })
    }

    pub fn type_use_annotations_after_modifiers(&self) -> impl Iterator<Item = Annotation> + '_ {
        let first_modifier_start = self
            .modifier_tokens()
            .map(|token| token.token_text_range().start())
            .min();

        self.annotations().filter(move |annotation| {
            first_modifier_start.is_some_and(|start| annotation.text_range().start() > start)
        })
    }

    pub fn modifier_tokens(&self) -> impl Iterator<Item = JavaSyntaxToken> + '_ {
        children_tokens_matching(&self.syntax, is_modifier_token)
    }
}

impl TypeParameterList {
    #[must_use]
    pub fn open_angle(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Lt)
    }

    #[must_use]
    pub fn close_angle(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Gt)
    }

    pub fn parameters(&self) -> impl Iterator<Item = TypeParameter> + '_ {
        children(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = TypeParameterListEntry> {
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

    pub fn entries(&self) -> impl Iterator<Item = IntersectionTypeEntry> {
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
                |intersection| intersection.entries().collect::<Vec<_>>(),
            )
            .into_iter()
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
    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
            .or_else(|| previous_sibling_token(&self.syntax, JavaSyntaxKind::LParen))
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
            .or_else(|| next_sibling_token(&self.syntax, JavaSyntaxKind::RParen))
    }

    pub fn components(&self) -> impl Iterator<Item = RecordComponent> + '_ {
        children(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = RecordComponentListEntry> {
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
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    pub fn members(&self) -> impl Iterator<Item = ClassBodyMember> + '_ {
        self.syntax.children().filter_map(|syntax| {
            if let Some(declaration) = ClassBodyDeclaration::cast(syntax.clone()) {
                return declaration.member();
            }
            ClassBodyMember::cast(syntax)
        })
    }
}

impl ClassBodyMember {
    #[must_use]
    pub fn starts_after_blank_line(&self) -> bool {
        starts_after_blank_line(self.syntax())
    }
}

impl RecordBody {
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    pub fn members(&self) -> impl Iterator<Item = ClassBodyMember> + '_ {
        self.syntax.children().filter_map(|syntax| {
            if let Some(declaration) = ClassBodyDeclaration::cast(syntax.clone()) {
                return declaration.member();
            }
            ClassBodyMember::cast(syntax)
        })
    }
}

impl ClassBodyDeclaration {
    #[must_use]
    pub fn member(&self) -> Option<ClassBodyMember> {
        child_family(&self.syntax)
    }
}

impl InterfaceBody {
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

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
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

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

    pub fn argument_entries(&self) -> impl Iterator<Item = AnnotationArgumentListEntry> {
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
    pub fn semicolon(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
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
    pub fn open_brace(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    #[must_use]
    pub fn constants(&self) -> Option<EnumConstantList> {
        child(&self.syntax)
    }

    pub fn members(&self) -> impl Iterator<Item = ClassBodyMember> + '_ {
        self.syntax.children().filter_map(|syntax| {
            if let Some(declaration) = ClassBodyDeclaration::cast(syntax.clone()) {
                return declaration.member();
            }
            ClassBodyMember::cast(syntax)
        })
    }

    pub fn semicolon_tokens(&self) -> impl Iterator<Item = JavaSyntaxToken> + '_ {
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::Semicolon)
    }
}

impl EnumConstantList {
    pub fn constants(&self) -> impl Iterator<Item = EnumConstant> + '_ {
        children(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = EnumConstantListEntry> {
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

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
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

    pub fn return_type_annotations(&self) -> impl Iterator<Item = Annotation> {
        annotations_before_type(&self.syntax, self.return_type())
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
    pub fn semicolon(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
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
    #[must_use]
    pub fn invocation(&self) -> Option<ConstructorInvocation> {
        child(&self.syntax)
    }

    pub fn block_statements(&self) -> impl Iterator<Item = BlockStatement> + '_ {
        children(&self.syntax)
    }

    pub fn items(&self) -> impl Iterator<Item = BlockItem> + '_ {
        children::<BlockStatement>(&self.syntax).filter_map(|node| node.item())
    }
}

impl ConstructorInvocation {
    #[must_use]
    pub fn starts_after_blank_line(&self) -> bool {
        starts_after_blank_line(&self.syntax)
    }

    #[must_use]
    pub fn qualifier_name(&self) -> Option<NameSyntax> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn qualifier_expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn type_arguments(&self) -> Option<TypeArgumentList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn target(&self) -> Option<JavaSyntaxToken> {
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
    pub fn semicolon(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl ThrowsClause {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::ThrowsKw)
    }

    pub fn exceptions(&self) -> impl Iterator<Item = Type> + '_ {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = ThrowsClauseEntry> {
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
    pub fn open_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
            .or_else(|| previous_sibling_token(&self.syntax, JavaSyntaxKind::LParen))
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
            .or_else(|| next_sibling_token(&self.syntax, JavaSyntaxKind::RParen))
    }

    pub fn parameters(&self) -> impl Iterator<Item = FormalParameter> + '_ {
        children(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = FormalParameterListEntry> {
        let mut entries = Vec::new();
        let mut pending_item = None;

        for element in self.syntax.children_with_tokens() {
            match element {
                SyntaxElement::Node(node) => {
                    let item = ReceiverParameter::cast(node.clone())
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
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::IfKw)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    #[must_use]
    pub fn else_keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::ElseKw)
    }

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
    pub fn dot_token(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Dot)
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
    pub fn dot_token(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Dot)
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

trait OptionalExpressionExt {
    fn is_same_expression(&self, target: &Expression) -> bool;
}

impl OptionalExpressionExt for Option<Expression> {
    fn is_same_expression(&self, target: &Expression) -> bool {
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
    pub fn dot_token(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Dot)
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
    #[must_use]
    pub fn open_angle(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Lt)
    }

    #[must_use]
    pub fn close_angle(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Gt)
    }

    pub fn arguments(&self) -> impl Iterator<Item = TypeArgument> + '_ {
        children(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = TypeArgumentListEntry> {
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

impl FieldAccessExpression {
    #[must_use]
    pub fn receiver(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn dot_token(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Dot)
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
    pub fn double_colon(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::DoubleColon)
    }

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
    pub fn new_token(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::NewKw)
    }

    #[must_use]
    pub fn target_name(&self) -> Option<JavaSyntaxToken> {
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::Identifier).last()
    }
}

impl ArrayAccessExpression {
    #[must_use]
    pub fn open_bracket(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LBracket)
    }

    #[must_use]
    pub fn close_bracket(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RBracket)
    }

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

    pub fn entries(&self) -> impl Iterator<Item = IntersectionTypeEntry> {
        intersection_type_entries(&self.syntax)
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
    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    pub fn arguments(&self) -> impl Iterator<Item = AnnotationArgument> {
        child::<AnnotationElementList>(&self.syntax)
            .map(|list| list.arguments().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
    }

    pub fn entries(&self) -> impl Iterator<Item = AnnotationArgumentListEntry> {
        child::<AnnotationElementList>(&self.syntax)
            .map(|list| list.argument_entries().collect::<Vec<_>>())
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
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    pub fn values(&self) -> impl Iterator<Item = AnnotationElementValue> + '_ {
        children(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = AnnotationArrayInitializerEntry> {
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

impl ParenthesizedExpression {
    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

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
    pub fn question_token(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Question)
    }

    #[must_use]
    pub fn colon_token(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Colon)
    }

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
    pub fn open_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

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
    pub fn instanceof_token(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::InstanceofKw)
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
    pub fn new_token(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::NewKw)
    }

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
    pub fn new_token(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::NewKw)
    }

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
    pub fn open_bracket(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LBracket)
    }

    #[must_use]
    pub fn close_bracket(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RBracket)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }
}

impl ArrayInitializer {
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    pub fn values(&self) -> impl Iterator<Item = VariableInitializerValue> + '_ {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = ArrayInitializerEntry> {
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

impl ReceiverParameter {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        children(&self.syntax)
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
    pub fn dot(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Dot)
    }

    #[must_use]
    pub fn this_token(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::ThisKw)
    }
}

impl LambdaExpression {
    #[must_use]
    pub fn parameters(&self) -> Option<LambdaParameterList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn arrow(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Arrow)
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

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
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
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::AssertKw)
    }

    #[must_use]
    pub fn condition(&self) -> Option<Expression> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn detail(&self) -> Option<Expression> {
        nth_child_family(&self.syntax, 1)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl BreakStatement {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::BreakKw)
    }

    #[must_use]
    pub fn label(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl ContinueStatement {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::ContinueKw)
    }

    #[must_use]
    pub fn label(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl ReturnStatement {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::ReturnKw)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl ThrowStatement {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::ThrowKw)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl YieldStatement {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        children_tokens_matching(&self.syntax, |kind| kind == JavaSyntaxKind::Identifier)
            .find(|token| token.text() == "yield")
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl WhileStatement {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::WhileKw)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

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
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::DoKw)
    }

    #[must_use]
    pub fn while_keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::WhileKw)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

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

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }
}

impl SynchronizedStatement {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::SynchronizedKw)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

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
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::TryKw)
    }

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
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::TryKw)
    }

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
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::CatchKw)
    }

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

    pub fn entries(&self) -> impl Iterator<Item = UnionTypeEntry> {
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
                |union| union.entries().collect::<Vec<_>>(),
            )
            .into_iter()
    }
}

impl UnionType {
    pub fn types(&self) -> impl Iterator<Item = Type> + '_ {
        children_family(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = UnionTypeEntry> {
        let mut entries = Vec::new();
        let mut pending_type = None;

        for element in self.syntax.children_with_tokens() {
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
}

impl FinallyClause {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::FinallyKw)
    }

    #[must_use]
    pub fn body(&self) -> Option<Block> {
        child(&self.syntax)
    }
}

impl ResourceSpecification {
    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn list(&self) -> Option<ResourceList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn trailing_semicolon(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }
}

impl ResourceList {
    pub fn resources(&self) -> impl Iterator<Item = Resource> + '_ {
        children(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = ResourceListEntry> {
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
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::ForKw)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

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
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::ForKw)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

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

    pub fn entries(&self) -> impl Iterator<Item = StatementExpressionEntry> {
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

impl SwitchStatement {
    #[must_use]
    pub fn keyword(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::SwitchKw)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

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
    pub fn block_statements(&self) -> impl Iterator<Item = BlockStatement> + '_ {
        children(&self.syntax)
    }

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
        self.case_entries()
            .map(|entry| entry.item)
            .collect::<Vec<_>>()
            .into_iter()
    }

    pub fn case_entries(&self) -> impl Iterator<Item = SwitchLabelCaseEntry> {
        let mut entries = Vec::new();
        let mut pending_item = None;

        for element in self.syntax.children_with_tokens() {
            match element {
                SyntaxElement::Node(node) => {
                    if let Some(item) = CaseConstant::cast(node.clone())
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

    #[must_use]
    pub fn open_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RParen)
    }

    pub fn components(&self) -> impl Iterator<Item = ComponentPattern> + '_ {
        children(&self.syntax)
    }

    pub fn entries(&self) -> impl Iterator<Item = RecordPatternComponentEntry> {
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

    pub fn target_entries(&self) -> impl Iterator<Item = ModuleNameListEntry> {
        module_name_entries_after_contextual_keyword(&self.syntax, "to")
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

    pub fn target_entries(&self) -> impl Iterator<Item = ModuleNameListEntry> {
        module_name_entries_after_contextual_keyword(&self.syntax, "to")
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

    pub fn implementation_entries(&self) -> impl Iterator<Item = ModuleNameListEntry> {
        module_name_entries_after_contextual_keyword(&self.syntax, "with")
    }

    fn names(&self) -> impl Iterator<Item = NameSyntax> + '_ {
        children_family(&self.syntax)
    }
}

impl Block {
    #[must_use]
    pub fn open_brace(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::RBrace)
    }

    pub fn block_statements(&self) -> impl Iterator<Item = BlockStatement> + '_ {
        children(&self.syntax)
    }

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
    pub fn starts_after_blank_line(&self) -> bool {
        starts_after_blank_line(&self.syntax)
    }

    #[must_use]
    pub fn statement(&self) -> Option<Statement> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn semicolon(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Semicolon)
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

fn previous_sibling_token(
    syntax: &super::JavaSyntaxNode,
    kind: JavaSyntaxKind,
) -> Option<JavaSyntaxToken> {
    match syntax.prev_sibling_or_token()? {
        SyntaxElement::Token(syntax) if syntax.kind() == kind => Some(JavaSyntaxToken { syntax }),
        _ => None,
    }
}

fn next_sibling_token(
    syntax: &super::JavaSyntaxNode,
    kind: JavaSyntaxKind,
) -> Option<JavaSyntaxToken> {
    match syntax.next_sibling_or_token()? {
        SyntaxElement::Token(syntax) if syntax.kind() == kind => Some(JavaSyntaxToken { syntax }),
        _ => None,
    }
}

fn type_clause_entries(syntax: &super::JavaSyntaxNode) -> std::vec::IntoIter<TypeClauseEntry> {
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

fn intersection_type_entries(
    syntax: &super::JavaSyntaxNode,
) -> std::vec::IntoIter<IntersectionTypeEntry> {
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

fn module_name_entries_after_contextual_keyword(
    syntax: &super::JavaSyntaxNode,
    keyword_text: &str,
) -> std::vec::IntoIter<ModuleNameListEntry> {
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
