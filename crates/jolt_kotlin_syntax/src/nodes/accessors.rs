use jolt_syntax::{SyntaxElement, source_gap_is_trivia, tokens_between};

use super::{
    AnnotatedExpression, Annotation, AnnotationArgumentList, AnnotationUseSiteTarget,
    AnonymousFunctionExpression, AnyKotlinNode, AssignmentExpression, BinaryExpression, Block,
    BlockItem, CallExpression, CallableName, CallableReferenceExpression, CatchClause, ClassBody,
    ClassDeclaration, ClassMember, ClassMemberDeclaration, CollectionLiteralExpression,
    CompanionObject, ConstructorDelegationCall, ContextFunctionType, ContextParameter,
    ContextParameterClause, Declaration, DefinitelyNonNullableType, DelegationSpecifier,
    DelegationSpecifierList, DestructuringDeclaration, DestructuringEntry,
    DestructuringPatternEntry, DoWhileStatement, EnumEntry, ErrorNode, ExplicitBackingField,
    Expression, ExpressionParentRole, ExpressionStatement, FinallyClause, ForStatement,
    FunctionDeclaration, FunctionType, FunctionTypeParameter, IfExpression, ImportAlias,
    ImportDirective, ImportList, IndexExpression, InitializerBlock, InterfaceDeclaration,
    JumpExpression, KotlinFile, KotlinFileItem, KotlinNode, KotlinSyntaxKind, KotlinSyntaxNode,
    KotlinSyntaxToken, LambdaExpression, LambdaParameter, LambdaParameterList, LiteralExpression,
    LocalDeclaration, LoopExpression, ModifierList, Name, NameExpression, NavigationExpression,
    NullableType, ObjectDeclaration, ObjectExpression, PackageHeader, ParenthesizedExpression,
    ParenthesizedType, PostfixExpression, PrimaryConstructor, PropertyAccessor,
    PropertyDeclaration, QualifiedName, ReceiverType, RecoveredNode, RecoveredSeparatedListEntry,
    SecondaryConstructor, Statement, StatementSyntax, StringTemplateEntry,
    StringTemplateExpression, StringTemplatePart, SuperExpression, ThisExpression, ThrowExpression,
    TryExpression, TypeAliasDeclaration, TypeArgument, TypeArgumentList, TypeConstraint,
    TypeConstraintList, TypeParameter, TypeParameterList, TypeProjection, TypeProjectionList,
    TypeReference, TypeSyntax, UnaryExpression, UserType, ValueArgument, ValueArgumentList,
    ValueParameter, ValueParameterList, WhenCondition, WhenConditionSyntax, WhenEntry,
    WhenExpression, WhenGuard, WhenSubject, WhileStatement, child, child_family, child_token,
    child_tokens, children, children_family,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NavigationOperatorTokens<'source> {
    Missing,
    Single(KotlinSyntaxToken<'source>),
    QuestionDot {
        question: KotlinSyntaxToken<'source>,
        dot: KotlinSyntaxToken<'source>,
    },
}

impl<'source> NavigationOperatorTokens<'source> {
    #[must_use]
    pub fn is_empty(self) -> bool {
        matches!(self, Self::Missing)
    }

    #[must_use]
    pub fn len(self) -> usize {
        match self {
            Self::Missing => 0,
            Self::Single(_) => 1,
            Self::QuestionDot { .. } => 2,
        }
    }

    #[must_use]
    pub fn first(self) -> Option<KotlinSyntaxToken<'source>> {
        match self {
            Self::Missing => None,
            Self::Single(token) => Some(token),
            Self::QuestionDot { question, .. } => Some(question),
        }
    }

    #[must_use]
    pub fn last(self) -> Option<KotlinSyntaxToken<'source>> {
        match self {
            Self::Missing => None,
            Self::Single(token) => Some(token),
            Self::QuestionDot { dot, .. } => Some(dot),
        }
    }

    #[must_use]
    pub fn iter(self) -> NavigationOperatorTokensIter<'source> {
        match self {
            Self::Missing => NavigationOperatorTokensIter {
                first: None,
                second: None,
            },
            Self::Single(token) => NavigationOperatorTokensIter {
                first: Some(token),
                second: None,
            },
            Self::QuestionDot { question, dot } => NavigationOperatorTokensIter {
                first: Some(question),
                second: Some(dot),
            },
        }
    }
}

pub struct NavigationOperatorTokensIter<'source> {
    first: Option<KotlinSyntaxToken<'source>>,
    second: Option<KotlinSyntaxToken<'source>>,
}

impl<'source> Iterator for NavigationOperatorTokensIter<'source> {
    type Item = KotlinSyntaxToken<'source>;

    fn next(&mut self) -> Option<Self::Item> {
        self.first.take().or_else(|| self.second.take())
    }
}

impl<'source> KotlinFile<'source> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'source>> + use<'source> {
        children(self.syntax())
    }

    #[must_use]
    pub fn package_header(&self) -> Option<PackageHeader<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn import_list(&self) -> Option<ImportList<'source>> {
        child(self.syntax())
    }

    pub fn declarations(&self) -> impl Iterator<Item = Declaration<'source>> + use<'source> {
        children_family(self.syntax())
    }

    pub fn statements(&self) -> impl Iterator<Item = StatementSyntax<'source>> + use<'source> {
        children_family(self.syntax())
    }

    pub fn children(&self) -> impl Iterator<Item = AnyKotlinNode<'source>> + use<'source> {
        self.syntax().children().filter_map(AnyKotlinNode::cast)
    }

    pub fn items(&self) -> impl Iterator<Item = KotlinFileItem<'source>> + use<'source> {
        children_family(self.syntax())
    }
}

impl<'source> PackageHeader<'source> {
    #[must_use]
    pub fn name(&self) -> Option<QualifiedName<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn package_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::PackageKw)
    }
}

impl<'source> ImportList<'source> {
    pub fn directives(&self) -> impl Iterator<Item = ImportDirective<'source>> + use<'source> {
        children(self.syntax())
    }
}

impl<'source> ImportDirective<'source> {
    #[must_use]
    pub fn import_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::ImportKw).or_else(|| self.first_token())
    }

    #[must_use]
    pub fn name(&self) -> Option<QualifiedName<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn star_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Star)
    }

    #[must_use]
    pub fn alias_keyword_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child::<ImportAlias<'source>>(self.syntax()).and_then(|alias| alias.alias_keyword_token())
    }

    #[must_use]
    pub fn alias(&self) -> Option<Name<'source>> {
        child::<ImportAlias<'source>>(self.syntax()).and_then(|alias| alias.name())
    }
}

impl<'source> ImportAlias<'source> {
    #[must_use]
    pub fn alias_keyword_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_tokens(self.syntax()).find(|token| token.text() == "as")
    }

    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }
}

impl<'source> EnumEntry<'source> {
    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }
}

impl<'source> ClassDeclaration<'source> {
    #[must_use]
    pub fn class_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::ClassKw)
    }

    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'source>> {
        child(self.syntax())
    }

    pub fn modifier_lists(&self) -> impl Iterator<Item = ModifierList<'source>> + use<'source, '_> {
        children(self.syntax())
    }

    #[must_use]
    pub fn primary_constructor(&self) -> Option<PrimaryConstructor<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn type_parameter_list(&self) -> Option<TypeParameterList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn type_constraint_list(&self) -> Option<TypeConstraintList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn colon(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Colon)
    }

    #[must_use]
    pub fn delegation_specifier_list(&self) -> Option<DelegationSpecifierList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn body(&self) -> Option<ClassBody<'source>> {
        child(self.syntax())
    }
}

impl<'source> InterfaceDeclaration<'source> {
    #[must_use]
    pub fn interface_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::InterfaceKw)
    }

    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'source>> {
        child(self.syntax())
    }

    pub fn modifier_lists(&self) -> impl Iterator<Item = ModifierList<'source>> + use<'source, '_> {
        children(self.syntax())
    }

    #[must_use]
    pub fn type_parameter_list(&self) -> Option<TypeParameterList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn type_constraint_list(&self) -> Option<TypeConstraintList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn colon(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Colon)
    }

    #[must_use]
    pub fn delegation_specifier_list(&self) -> Option<DelegationSpecifierList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn body(&self) -> Option<ClassBody<'source>> {
        child(self.syntax())
    }
}

impl<'source> ObjectDeclaration<'source> {
    #[must_use]
    pub fn object_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::ObjectKw)
    }

    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn colon(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Colon)
    }

    #[must_use]
    pub fn delegation_specifier_list(&self) -> Option<DelegationSpecifierList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn body(&self) -> Option<ClassBody<'source>> {
        child(self.syntax())
    }
}

impl<'source> CompanionObject<'source> {
    #[must_use]
    pub fn object_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::ObjectKw)
    }

    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn colon(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Colon)
    }

    #[must_use]
    pub fn delegation_specifier_list(&self) -> Option<DelegationSpecifierList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn body(&self) -> Option<ClassBody<'source>> {
        child(self.syntax())
    }
}

impl<'source> FunctionDeclaration<'source> {
    #[must_use]
    pub fn context_parameter_clause(&self) -> Option<ContextParameterClause<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn callable_name(&self) -> Option<CallableName<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'source>> {
        child(self.syntax())
    }

    pub fn modifier_lists(&self) -> impl Iterator<Item = ModifierList<'source>> + use<'source, '_> {
        children(self.syntax())
    }

    #[must_use]
    pub fn fun_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::FunKw)
    }

    #[must_use]
    pub fn name_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn type_parameter_list(&self) -> Option<TypeParameterList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn value_parameter_list(&self) -> Option<ValueParameterList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn colon(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Colon)
    }

    #[must_use]
    pub fn return_type(&self) -> Option<TypeReference<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn assign_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Assign)
    }

    #[must_use]
    pub fn type_constraint_list(&self) -> Option<TypeConstraintList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn block(&self) -> Option<Block<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }

    /// Returns true when this function declaration is a bare `fun` token with no
    /// structured body — i.e. a fun-interface header (`fun interface Foo {}`
    /// form where the `fun` lives on the `FunctionDeclaration` preceding the
    /// `InterfaceDeclaration`).
    #[must_use]
    pub fn is_fun_interface_header(&self) -> bool {
        self.fun_token().is_some()
            && self.name().is_none()
            && self.callable_name().is_none()
            && self.name_token().is_none()
            && self.context_parameter_clause().is_none()
            && self.type_parameter_list().is_none()
            && self.value_parameter_list().is_none()
            && self.colon().is_none()
            && self.return_type().is_none()
            && self.assign_token().is_none()
            && self.type_constraint_list().is_none()
            && self.block().is_none()
            && self.expression().is_none()
    }

    #[must_use]
    pub fn tail_is_trivia_between(&self, start: usize, end: usize) -> bool {
        source_gap_is_trivia(
            self.source_text(),
            self.text_range().start().get(),
            self.token_iter(),
            start,
            end,
        )
    }

    pub fn tail_tokens_between(
        &self,
        start: usize,
        end: usize,
    ) -> impl Iterator<Item = KotlinSyntaxToken<'source>> + use<'source, '_> {
        tokens_between(self.token_iter(), start, end)
    }
}

impl<'source> CallableName<'source> {
    pub fn names(&self) -> impl Iterator<Item = Name<'source>> + use<'source> {
        children(self.syntax())
    }

    #[must_use]
    pub fn receiver_type(&self) -> Option<TypeReference<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        self.names().last()
    }

    #[must_use]
    pub fn receiver_separator(&self) -> Option<KotlinSyntaxToken<'source>> {
        let name_start = self.name()?.text_range().start();
        child_tokens(self.syntax())
            .filter(|token| {
                token.token_text_range().end() <= name_start
                    && matches!(token.kind(), KotlinSyntaxKind::Dot)
            })
            .last()
    }
}

impl<'source> ContextParameterClause<'source> {
    #[must_use]
    pub fn context_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_tokens(self.syntax()).find(|token| token.text() == "context")
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::RParen)
    }

    pub fn entries(
        &self,
    ) -> impl Iterator<Item = ContextParameterClauseEntry<'source>> + use<'source, '_> {
        separated_entries(self.syntax(), ContextParameter::cast, |parameter, comma| {
            ContextParameterClauseEntry { parameter, comma }
        })
    }

    pub fn entries_with_recovered(
        &self,
    ) -> impl Iterator<
        Item = RecoveredSeparatedListEntry<'source, ContextParameterClauseEntry<'source>>,
    > + use<'source, '_> {
        let elements = self
            .syntax()
            .children_with_tokens()
            .skip_while(|element| {
                !matches!(element, SyntaxElement::Token(t) if t.kind() == KotlinSyntaxKind::LParen)
            })
            .skip(1)
            .take_while(|element| {
                !matches!(element, SyntaxElement::Token(t) if t.kind() == KotlinSyntaxKind::RParen)
            });

        recovered_separated_elements(
            elements,
            |kind| kind == KotlinSyntaxKind::ContextParameter,
            |syntax| ContextParameter { syntax },
            |parameter, comma| ContextParameterClauseEntry { parameter, comma },
            |_| false,
        )
    }
}

impl<'source> ContextParameter<'source> {
    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn colon(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Colon)
    }

    #[must_use]
    pub fn ty(&self) -> Option<TypeReference<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn assign_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Assign)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }
}

impl<'source> AnonymousFunctionExpression<'source> {
    #[must_use]
    pub fn fun_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::FunKw)
    }

    #[must_use]
    pub fn value_parameter_list(&self) -> Option<ValueParameterList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn receiver_type(&self) -> Option<TypeReference<'source>> {
        let dot_start = self.dot_token()?.token_text_range().start();
        children(self.syntax())
            .find(|ty: &TypeReference<'source>| ty.text_range().end() <= dot_start)
    }

    #[must_use]
    pub fn dot_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Dot)
    }

    #[must_use]
    pub fn colon(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Colon)
    }

    #[must_use]
    pub fn return_type(&self) -> Option<TypeReference<'source>> {
        let colon_end = self.colon()?.token_text_range().end();
        children(self.syntax())
            .find(|ty: &TypeReference<'source>| ty.text_range().start() >= colon_end)
    }

    #[must_use]
    pub fn assign_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Assign)
    }

    #[must_use]
    pub fn block(&self) -> Option<Block<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }
}

impl<'source> PropertyDeclaration<'source> {
    #[must_use]
    pub fn context_parameter_clause(&self) -> Option<ContextParameterClause<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn callable_name(&self) -> Option<CallableName<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'source>> {
        child(self.syntax())
    }

    pub fn modifier_lists(&self) -> impl Iterator<Item = ModifierList<'source>> + use<'source, '_> {
        children(self.syntax())
    }

    #[must_use]
    pub fn val_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::ValKw)
    }

    #[must_use]
    pub fn var_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::VarKw)
    }

    #[must_use]
    pub fn name_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn destructuring_declaration(&self) -> Option<DestructuringDeclaration<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn colon(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Colon)
    }

    #[must_use]
    pub fn ty(&self) -> Option<TypeReference<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn assign_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Assign)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }

    #[must_use]
    pub fn tail_is_trivia_between(&self, start: usize, end: usize) -> bool {
        source_gap_is_trivia(
            self.source_text(),
            self.text_range().start().get(),
            self.token_iter(),
            start,
            end,
        )
    }

    pub fn tail_tokens_between(
        &self,
        start: usize,
        end: usize,
    ) -> impl Iterator<Item = KotlinSyntaxToken<'source>> + use<'source, '_> {
        tokens_between(self.token_iter(), start, end)
    }

    #[must_use]
    pub fn delegate_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_tokens(self.syntax()).find(|token| token.text() == "by")
    }

    #[must_use]
    pub fn type_parameter_list(&self) -> Option<TypeParameterList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn type_constraint_list(&self) -> Option<TypeConstraintList<'source>> {
        child(self.syntax())
    }

    pub fn explicit_backing_fields(
        &self,
    ) -> impl Iterator<Item = ExplicitBackingField<'source>> + use<'source> {
        children(self.syntax())
    }

    pub fn accessors(&self) -> impl Iterator<Item = PropertyAccessor<'source>> + use<'source> {
        children(self.syntax())
    }
}

impl<'source> ExplicitBackingField<'source> {
    #[must_use]
    pub fn field_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_tokens(self.syntax()).find(|token| token.text() == "field")
    }

    #[must_use]
    pub fn assign_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Assign)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }
}

impl<'source> PropertyAccessor<'source> {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn keyword_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_tokens(self.syntax()).find(|token| token.text() == "get" || token.text() == "set")
    }

    #[must_use]
    pub fn value_parameter_list(&self) -> Option<ValueParameterList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn colon(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Colon)
    }

    #[must_use]
    pub fn return_type(&self) -> Option<TypeReference<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn assign_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Assign)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }

    #[must_use]
    pub fn block(&self) -> Option<Block<'source>> {
        child(self.syntax())
    }
}

impl<'source> InitializerBlock<'source> {
    #[must_use]
    pub fn init_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_tokens(self.syntax()).find(|token| token.text() == "init")
    }

    #[must_use]
    pub fn block(&self) -> Option<Block<'source>> {
        child(self.syntax())
    }
}

impl<'source> SecondaryConstructor<'source> {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn constructor_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_tokens(self.syntax()).find(|token| token.text() == "constructor")
    }

    #[must_use]
    pub fn value_parameter_list(&self) -> Option<ValueParameterList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn colon(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Colon)
    }

    #[must_use]
    pub fn delegation_call(&self) -> Option<ConstructorDelegationCall<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn block(&self) -> Option<Block<'source>> {
        child(self.syntax())
    }
}

impl<'source> ConstructorDelegationCall<'source> {
    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }
}

impl<'source> TypeAliasDeclaration<'source> {
    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn typealias_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::TypeAliasKw)
    }

    #[must_use]
    pub fn type_parameter_list(&self) -> Option<TypeParameterList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn assign_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Assign)
    }

    #[must_use]
    pub fn ty(&self) -> Option<TypeReference<'source>> {
        child(self.syntax())
    }
}

impl<'source> ModifierList<'source> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'source>> + use<'source> {
        children(self.syntax())
    }

    pub fn modifier_tokens(
        &self,
    ) -> impl Iterator<Item = KotlinSyntaxToken<'source>> + use<'source> {
        child_tokens(self.syntax()).filter(|token| token.kind() != KotlinSyntaxKind::At)
    }
}

impl<'source> Annotation<'source> {
    #[must_use]
    pub fn use_site_target(&self) -> Option<AnnotationUseSiteTarget<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn name(&self) -> Option<QualifiedName<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn argument_list(&self) -> Option<AnnotationArgumentList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn at_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::At)
    }
}

impl<'source> AnnotationUseSiteTarget<'source> {
    #[must_use]
    pub fn target_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_tokens(self.syntax()).find(|token| token.kind() != KotlinSyntaxKind::Colon)
    }

    #[must_use]
    pub fn colon_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Colon)
    }
}

impl<'source> AnnotationArgumentList<'source> {
    pub fn entries(&self) -> impl Iterator<Item = ValueArgumentEntry<'source>> + use<'source, '_> {
        separated_entries(self.syntax(), ValueArgument::cast, |argument, comma| {
            ValueArgumentEntry { argument, comma }
        })
    }

    pub fn entries_with_recovered(
        &self,
    ) -> impl Iterator<Item = RecoveredSeparatedListEntry<'source, ValueArgumentEntry<'source>>>
    + use<'source, '_> {
        recovered_separated_entries(
            self.syntax(),
            |kind| kind == KotlinSyntaxKind::ValueArgument,
            |syntax| ValueArgument { syntax },
            |argument, comma| ValueArgumentEntry { argument, comma },
            |token| {
                matches!(
                    token.kind(),
                    KotlinSyntaxKind::LParen | KotlinSyntaxKind::RParen
                )
            },
        )
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::RParen)
    }
}

impl<'source> TypeReference<'source> {
    #[must_use]
    pub fn ty(&self) -> Option<TypeSyntax<'source>> {
        child_family(self.syntax())
    }
}

impl<'source> UserType<'source> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'source>> + use<'source> {
        children(self.syntax())
    }

    pub fn identifier_tokens(
        &self,
    ) -> impl Iterator<Item = KotlinSyntaxToken<'source>> + use<'source> {
        child_tokens(self.syntax()).filter(|token| token.kind() == KotlinSyntaxKind::Identifier)
    }

    pub fn type_argument_lists(
        &self,
    ) -> impl Iterator<Item = TypeArgumentList<'source>> + use<'source> {
        children(self.syntax())
    }

    pub fn dot_tokens(&self) -> impl Iterator<Item = KotlinSyntaxToken<'source>> + use<'source> {
        child_tokens(self.syntax()).filter(|token| token.kind() == KotlinSyntaxKind::Dot)
    }
}

impl<'source> NullableType<'source> {
    #[must_use]
    pub fn inner(&self) -> Option<TypeSyntax<'source>> {
        child_family(self.syntax())
    }

    #[must_use]
    pub fn question_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Question)
    }
}

impl<'source> ParenthesizedType<'source> {
    #[must_use]
    pub fn inner(&self) -> Option<TypeReference<'source>> {
        self.entries().find_map(|entry| entry.parameter.ty())
    }

    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'source>> + use<'source> {
        children(self.syntax())
    }

    pub fn entries(
        &self,
    ) -> impl Iterator<Item = FunctionTypeParameterEntry<'source>> + use<'source, '_> {
        separated_entries(
            self.syntax(),
            FunctionTypeParameter::cast,
            |parameter, comma| FunctionTypeParameterEntry { parameter, comma },
        )
    }

    pub fn entries_with_recovered(
        &self,
    ) -> impl Iterator<
        Item = RecoveredSeparatedListEntry<'source, FunctionTypeParameterEntry<'source>>,
    > + use<'source, '_> {
        let elements = self
            .syntax()
            .children_with_tokens()
            .skip_while(|element| {
                !matches!(element, SyntaxElement::Token(t) if t.kind() == KotlinSyntaxKind::LParen)
            })
            .skip(1)
            .take_while(|element| {
                !matches!(element, SyntaxElement::Token(t) if t.kind() == KotlinSyntaxKind::RParen)
            });

        recovered_separated_elements(
            elements,
            |kind| kind == KotlinSyntaxKind::FunctionTypeParameter,
            |syntax| FunctionTypeParameter { syntax },
            |parameter, comma| FunctionTypeParameterEntry { parameter, comma },
            |_| false,
        )
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::RParen)
    }
}

impl<'source> FunctionTypeParameter<'source> {
    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn colon(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Colon)
    }

    #[must_use]
    pub fn ty(&self) -> Option<TypeReference<'source>> {
        child(self.syntax())
    }
}

impl<'source> ReceiverType<'source> {
    #[must_use]
    pub fn receiver(&self) -> Option<TypeSyntax<'source>> {
        child_family(self.syntax())
    }

    #[must_use]
    pub fn parameter(&self) -> Option<ParenthesizedType<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn dot_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Dot)
    }
}

impl<'source> FunctionType<'source> {
    #[must_use]
    pub fn suspend_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_tokens(self.syntax()).find(|token| token.text() == "suspend")
    }

    #[must_use]
    pub fn receiver(&self) -> Option<ReceiverType<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn nested_function_type(&self) -> Option<FunctionType<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn parameter(&self) -> Option<ParenthesizedType<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn return_type(&self) -> Option<TypeSyntax<'source>> {
        children_family(self.syntax()).last()
    }

    #[must_use]
    pub fn arrow_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Arrow)
    }
}

impl<'source> ContextFunctionType<'source> {
    #[must_use]
    pub fn context_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_tokens(self.syntax()).find(|token| token.text() == "context")
    }

    pub fn context_parameters(
        &self,
    ) -> impl Iterator<Item = TypeReference<'source>> + use<'source> {
        children::<FunctionTypeParameter<'source>>(self.syntax())
            .filter_map(|parameter| parameter.ty())
    }

    pub fn context_parameter_entries(
        &self,
    ) -> impl Iterator<Item = ContextFunctionTypeParameterEntry<'source>> + use<'source, '_> {
        separated_entries(
            self.syntax(),
            FunctionTypeParameter::cast,
            |parameter, comma| ContextFunctionTypeParameterEntry { parameter, comma },
        )
    }

    pub fn context_parameter_entries_with_recovered(
        &self,
    ) -> impl Iterator<
        Item = RecoveredSeparatedListEntry<'source, ContextFunctionTypeParameterEntry<'source>>,
    > + use<'source, '_> {
        let elements = self
            .syntax()
            .children_with_tokens()
            .skip_while(|element| {
                !matches!(element, SyntaxElement::Token(t) if t.kind() == KotlinSyntaxKind::LParen)
            })
            .skip(1)
            .take_while(|element| {
                !matches!(element, SyntaxElement::Token(t) if t.kind() == KotlinSyntaxKind::RParen)
            });

        recovered_separated_elements(
            elements,
            |kind| kind == KotlinSyntaxKind::FunctionTypeParameter,
            |syntax| FunctionTypeParameter { syntax },
            |parameter, comma| ContextFunctionTypeParameterEntry { parameter, comma },
            |_| false,
        )
    }

    #[must_use]
    pub fn function_type(&self) -> Option<FunctionType<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::RParen)
    }
}

impl<'source> DefinitelyNonNullableType<'source> {
    pub fn types(&self) -> impl Iterator<Item = UserType<'source>> + use<'source> {
        children(self.syntax())
    }

    #[must_use]
    pub fn amp_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Amp)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TypeParameterListEntry<'source> {
    pub parameter: TypeParameter<'source>,
    pub comma: Option<KotlinSyntaxToken<'source>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TypeConstraintListEntry<'source> {
    pub constraint: TypeConstraint<'source>,
    pub comma: Option<KotlinSyntaxToken<'source>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TypeProjectionListEntry<'source> {
    pub argument: TypeArgument<'source>,
    pub comma: Option<KotlinSyntaxToken<'source>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FunctionTypeParameterEntry<'source> {
    pub parameter: FunctionTypeParameter<'source>,
    pub comma: Option<KotlinSyntaxToken<'source>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextFunctionTypeParameterEntry<'source> {
    pub parameter: FunctionTypeParameter<'source>,
    pub comma: Option<KotlinSyntaxToken<'source>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LambdaParameterListEntry<'source> {
    pub parameter: LambdaParameter<'source>,
    pub comma: Option<KotlinSyntaxToken<'source>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValueArgumentEntry<'source> {
    pub argument: ValueArgument<'source>,
    pub comma: Option<KotlinSyntaxToken<'source>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DelegationSpecifierListEntry<'source> {
    pub specifier: DelegationSpecifier<'source>,
    pub comma: Option<KotlinSyntaxToken<'source>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DestructuringDeclarationEntry<'source> {
    pub entry: DestructuringEntry<'source>,
    pub comma: Option<KotlinSyntaxToken<'source>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClassMemberDeclarationEntry<'source> {
    pub member: ClassMemberDeclaration<'source>,
    pub comma: Option<KotlinSyntaxToken<'source>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextParameterClauseEntry<'source> {
    pub parameter: ContextParameter<'source>,
    pub comma: Option<KotlinSyntaxToken<'source>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValueParameterListEntry<'source> {
    pub parameter: ValueParameter<'source>,
    pub comma: Option<KotlinSyntaxToken<'source>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WhenConditionEntry<'source> {
    pub condition: WhenCondition<'source>,
    pub comma: Option<KotlinSyntaxToken<'source>>,
}

impl<'source> TypeParameterList<'source> {
    pub fn parameters(&self) -> impl Iterator<Item = TypeParameter<'source>> + use<'source> {
        children(self.syntax())
    }

    pub fn entries(
        &self,
    ) -> impl Iterator<Item = TypeParameterListEntry<'source>> + use<'source, '_> {
        separated_entries(self.syntax(), TypeParameter::cast, |parameter, comma| {
            TypeParameterListEntry { parameter, comma }
        })
    }

    pub fn entries_with_recovered(
        &self,
    ) -> impl Iterator<Item = RecoveredSeparatedListEntry<'source, TypeParameterListEntry<'source>>>
    + use<'source, '_> {
        recovered_separated_entries(
            self.syntax(),
            |kind| kind == KotlinSyntaxKind::TypeParameter,
            |syntax| TypeParameter { syntax },
            |parameter, comma| TypeParameterListEntry { parameter, comma },
            |token| matches!(token.kind(), KotlinSyntaxKind::Lt | KotlinSyntaxKind::Gt),
        )
    }

    #[must_use]
    pub fn open_angle(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Lt)
    }

    #[must_use]
    pub fn close_angle(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Gt)
    }
}

impl<'source> TypeParameter<'source> {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn variance_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::InKw)
    }

    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn colon(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Colon)
    }

    #[must_use]
    pub fn bound(&self) -> Option<TypeReference<'source>> {
        child(self.syntax())
    }
}

impl<'source> TypeConstraintList<'source> {
    pub fn constraints(&self) -> impl Iterator<Item = TypeConstraint<'source>> + use<'source> {
        children(self.syntax())
    }

    pub fn entries(
        &self,
    ) -> impl Iterator<Item = TypeConstraintListEntry<'source>> + use<'source, '_> {
        separated_entries(self.syntax(), TypeConstraint::cast, |constraint, comma| {
            TypeConstraintListEntry { constraint, comma }
        })
    }

    pub fn entries_with_recovered(
        &self,
    ) -> impl Iterator<Item = RecoveredSeparatedListEntry<'source, TypeConstraintListEntry<'source>>>
    + use<'source, '_> {
        recovered_separated_entries(
            self.syntax(),
            |kind| kind == KotlinSyntaxKind::TypeConstraint,
            |syntax| TypeConstraint { syntax },
            |constraint, comma| TypeConstraintListEntry { constraint, comma },
            |token| token.text() == "where",
        )
    }

    #[must_use]
    pub fn where_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_tokens(self.syntax()).find(|token| token.text() == "where")
    }
}

impl<'source> TypeConstraint<'source> {
    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn colon(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Colon)
    }

    #[must_use]
    pub fn bound(&self) -> Option<TypeReference<'source>> {
        child(self.syntax())
    }
}

impl<'source> DelegationSpecifierList<'source> {
    pub fn specifiers(&self) -> impl Iterator<Item = DelegationSpecifier<'source>> + use<'source> {
        children(self.syntax())
    }

    pub fn entries(
        &self,
    ) -> impl Iterator<Item = DelegationSpecifierListEntry<'source>> + use<'source, '_> {
        separated_entries(
            self.syntax(),
            DelegationSpecifier::cast,
            |specifier, comma| DelegationSpecifierListEntry { specifier, comma },
        )
    }

    pub fn entries_with_recovered(
        &self,
    ) -> impl Iterator<
        Item = RecoveredSeparatedListEntry<'source, DelegationSpecifierListEntry<'source>>,
    > + use<'source, '_> {
        recovered_separated_entries(
            self.syntax(),
            |kind| kind == KotlinSyntaxKind::DelegationSpecifier,
            |syntax| DelegationSpecifier { syntax },
            |specifier, comma| DelegationSpecifierListEntry { specifier, comma },
            |_| false,
        )
    }
}

impl<'source> DelegationSpecifier<'source> {
    #[must_use]
    pub fn ty(&self) -> Option<TypeReference<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn value_argument_list(&self) -> Option<ValueArgumentList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn by_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_tokens(self.syntax()).find(|token| token.text() == "by")
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }
}

fn separated_entries<'source, N, T, F>(
    syntax: &KotlinSyntaxNode<'source>,
    cast: fn(KotlinSyntaxNode<'source>) -> Option<N>,
    make_entry: F,
) -> impl Iterator<Item = T> + use<'source, N, T, F>
where
    N: KotlinNode<'source>,
    F: Fn(N, Option<KotlinSyntaxToken<'source>>) -> T + Copy,
{
    let mut elements = syntax.children_with_tokens();
    let mut current = None;
    let mut done = false;

    std::iter::from_fn(move || {
        if done {
            return None;
        }

        for child in elements.by_ref() {
            match child {
                SyntaxElement::Node(node) => {
                    if let Some(node) = cast(node)
                        && let Some(node) = current.replace(node)
                    {
                        return Some(make_entry(node, None));
                    }
                }
                SyntaxElement::Token(token) => {
                    if token.kind() == KotlinSyntaxKind::Comma
                        && let Some(node) = current.take()
                    {
                        return Some(make_entry(node, Some(token)));
                    }
                }
            }
        }

        done = true;
        current.take().map(|node| make_entry(node, None))
    })
}

fn recovered_node_entry<Entry>(
    node: KotlinSyntaxNode<'_>,
) -> RecoveredSeparatedListEntry<'_, Entry> {
    if node.kind() == KotlinSyntaxKind::ErrorNode {
        RecoveredSeparatedListEntry::Error(ErrorNode { syntax: node })
    } else {
        RecoveredSeparatedListEntry::Node(RecoveredNode::new(node))
    }
}

fn recovered_separated_entries<'source, N, T, K, C, F, S>(
    syntax: &KotlinSyntaxNode<'source>,
    is_entry_kind: K,
    make_node: C,
    make_entry: F,
    skip_token: S,
) -> impl Iterator<Item = RecoveredSeparatedListEntry<'source, T>> + use<'source, N, T, K, C, F, S>
where
    K: FnMut(KotlinSyntaxKind) -> bool,
    C: Fn(KotlinSyntaxNode<'source>) -> N + Copy,
    F: Fn(N, Option<KotlinSyntaxToken<'source>>) -> T + Copy,
    S: FnMut(&KotlinSyntaxToken<'source>) -> bool,
{
    recovered_separated_elements(
        syntax.children_with_tokens(),
        is_entry_kind,
        make_node,
        make_entry,
        skip_token,
    )
}

fn recovered_separated_elements<'source, Elements, N, T, K, C, F, S>(
    elements: Elements,
    mut is_entry_kind: K,
    make_node: C,
    make_entry: F,
    mut skip_token: S,
) -> impl Iterator<Item = RecoveredSeparatedListEntry<'source, T>>
+ use<'source, Elements, N, T, K, C, F, S>
where
    Elements: Iterator<Item = SyntaxElement<'source, crate::language::KotlinLanguage>>,
    K: FnMut(KotlinSyntaxKind) -> bool,
    C: Fn(KotlinSyntaxNode<'source>) -> N + Copy,
    F: Fn(N, Option<KotlinSyntaxToken<'source>>) -> T + Copy,
    S: FnMut(&KotlinSyntaxToken<'source>) -> bool,
{
    let mut elements = elements;
    let mut current = None;
    let mut queued = None;
    let mut done = false;

    std::iter::from_fn(move || {
        loop {
            if let Some(entry) = queued.take() {
                return Some(entry);
            }

            if done {
                return None;
            }

            let Some(child) = elements.next() else {
                done = true;
                return current
                    .take()
                    .map(|node| RecoveredSeparatedListEntry::Entry(make_entry(node, None)));
            };

            match child {
                SyntaxElement::Node(node) => {
                    if is_entry_kind(node.kind()) {
                        let item = make_node(node);
                        if let Some(previous) = current.replace(item) {
                            return Some(RecoveredSeparatedListEntry::Entry(make_entry(
                                previous, None,
                            )));
                        }
                    } else {
                        let recovered = recovered_node_entry(node);
                        if let Some(previous) = current.take() {
                            queued = Some(recovered);
                            return Some(RecoveredSeparatedListEntry::Entry(make_entry(
                                previous, None,
                            )));
                        }
                        return Some(recovered);
                    }
                }
                SyntaxElement::Token(token) => {
                    if token.kind() == KotlinSyntaxKind::Comma {
                        if let Some(node) = current.take() {
                            return Some(RecoveredSeparatedListEntry::Entry(make_entry(
                                node,
                                Some(token),
                            )));
                        }
                        return Some(RecoveredSeparatedListEntry::Token(token));
                    }
                    if skip_token(&token) {
                        continue;
                    }
                    if let Some(previous) = current.take() {
                        queued = Some(RecoveredSeparatedListEntry::Token(token));
                        return Some(RecoveredSeparatedListEntry::Entry(make_entry(
                            previous, None,
                        )));
                    }
                    return Some(RecoveredSeparatedListEntry::Token(token));
                }
            }
        }
    })
}

fn recovered_body_entries<'source, Entry, C, S>(
    syntax: &KotlinSyntaxNode<'source>,
    mut classify: C,
    mut skip_token: S,
) -> impl Iterator<Item = RecoveredSeparatedListEntry<'source, Entry>> + use<'source, Entry, C, S>
where
    C: FnMut(KotlinSyntaxNode<'source>) -> Result<Entry, KotlinSyntaxNode<'source>>,
    S: FnMut(KotlinSyntaxKind) -> bool,
{
    syntax
        .children_with_tokens()
        .filter_map(move |element| match element {
            SyntaxElement::Node(node) => Some(match classify(node) {
                Ok(entry) => RecoveredSeparatedListEntry::Entry(entry),
                Err(node) => recovered_node_entry(node),
            }),
            SyntaxElement::Token(token) => {
                (!skip_token(token.kind())).then_some(RecoveredSeparatedListEntry::Token(token))
            }
        })
}

fn classify_block_item(node: KotlinSyntaxNode<'_>) -> Result<BlockItem<'_>, KotlinSyntaxNode<'_>> {
    match node.kind() {
        KotlinSyntaxKind::Statement => Ok(BlockItem::Statement(Statement { syntax: node })),
        KotlinSyntaxKind::ExpressionStatement => {
            Ok(BlockItem::ExpressionStatement(ExpressionStatement {
                syntax: node,
            }))
        }
        KotlinSyntaxKind::LocalDeclaration => Ok(BlockItem::LocalDeclaration(LocalDeclaration {
            syntax: node,
        })),
        KotlinSyntaxKind::Block => Ok(BlockItem::Block(Block { syntax: node })),
        KotlinSyntaxKind::ClassDeclaration => Ok(BlockItem::ClassDeclaration(ClassDeclaration {
            syntax: node,
        })),
        KotlinSyntaxKind::InterfaceDeclaration => {
            Ok(BlockItem::InterfaceDeclaration(InterfaceDeclaration {
                syntax: node,
            }))
        }
        KotlinSyntaxKind::ObjectDeclaration => {
            Ok(BlockItem::ObjectDeclaration(ObjectDeclaration {
                syntax: node,
            }))
        }
        KotlinSyntaxKind::FunctionDeclaration => {
            Ok(BlockItem::FunctionDeclaration(FunctionDeclaration {
                syntax: node,
            }))
        }
        KotlinSyntaxKind::PropertyDeclaration => {
            Ok(BlockItem::PropertyDeclaration(PropertyDeclaration {
                syntax: node,
            }))
        }
        KotlinSyntaxKind::TypeAliasDeclaration => {
            Ok(BlockItem::TypeAliasDeclaration(TypeAliasDeclaration {
                syntax: node,
            }))
        }
        KotlinSyntaxKind::SecondaryConstructor => {
            Ok(BlockItem::SecondaryConstructor(SecondaryConstructor {
                syntax: node,
            }))
        }
        KotlinSyntaxKind::InitializerBlock => Ok(BlockItem::InitializerBlock(InitializerBlock {
            syntax: node,
        })),
        _ => Err(node),
    }
}

fn classify_expression(node: KotlinSyntaxNode<'_>) -> Result<Expression<'_>, KotlinSyntaxNode<'_>> {
    match node.kind() {
        KotlinSyntaxKind::AssignmentExpression => {
            Ok(Expression::AssignmentExpression(AssignmentExpression {
                syntax: node,
            }))
        }
        KotlinSyntaxKind::BinaryExpression => Ok(Expression::BinaryExpression(BinaryExpression {
            syntax: node,
        })),
        KotlinSyntaxKind::UnaryExpression => Ok(Expression::UnaryExpression(UnaryExpression {
            syntax: node,
        })),
        KotlinSyntaxKind::PostfixExpression => {
            Ok(Expression::PostfixExpression(PostfixExpression {
                syntax: node,
            }))
        }
        KotlinSyntaxKind::CallExpression => {
            Ok(Expression::CallExpression(CallExpression { syntax: node }))
        }
        KotlinSyntaxKind::IndexExpression => Ok(Expression::IndexExpression(IndexExpression {
            syntax: node,
        })),
        KotlinSyntaxKind::NavigationExpression => {
            Ok(Expression::NavigationExpression(NavigationExpression {
                syntax: node,
            }))
        }
        KotlinSyntaxKind::CallableReferenceExpression => Ok(
            Expression::CallableReferenceExpression(CallableReferenceExpression { syntax: node }),
        ),
        KotlinSyntaxKind::LiteralExpression => {
            Ok(Expression::LiteralExpression(LiteralExpression {
                syntax: node,
            }))
        }
        KotlinSyntaxKind::StringTemplateExpression => Ok(Expression::StringTemplateExpression(
            StringTemplateExpression { syntax: node },
        )),
        KotlinSyntaxKind::NameExpression => {
            Ok(Expression::NameExpression(NameExpression { syntax: node }))
        }
        KotlinSyntaxKind::ThisExpression => {
            Ok(Expression::ThisExpression(ThisExpression { syntax: node }))
        }
        KotlinSyntaxKind::SuperExpression => Ok(Expression::SuperExpression(SuperExpression {
            syntax: node,
        })),
        KotlinSyntaxKind::ParenthesizedExpression => Ok(Expression::ParenthesizedExpression(
            ParenthesizedExpression { syntax: node },
        )),
        KotlinSyntaxKind::AnnotatedExpression => {
            Ok(Expression::AnnotatedExpression(AnnotatedExpression {
                syntax: node,
            }))
        }
        KotlinSyntaxKind::IfExpression => {
            Ok(Expression::IfExpression(IfExpression { syntax: node }))
        }
        KotlinSyntaxKind::WhenExpression => {
            Ok(Expression::WhenExpression(WhenExpression { syntax: node }))
        }
        KotlinSyntaxKind::TryExpression => {
            Ok(Expression::TryExpression(TryExpression { syntax: node }))
        }
        KotlinSyntaxKind::LoopExpression => {
            Ok(Expression::LoopExpression(LoopExpression { syntax: node }))
        }
        KotlinSyntaxKind::ForStatement => {
            Ok(Expression::ForStatement(ForStatement { syntax: node }))
        }
        KotlinSyntaxKind::WhileStatement => {
            Ok(Expression::WhileStatement(WhileStatement { syntax: node }))
        }
        KotlinSyntaxKind::DoWhileStatement => Ok(Expression::DoWhileStatement(DoWhileStatement {
            syntax: node,
        })),
        KotlinSyntaxKind::JumpExpression => {
            Ok(Expression::JumpExpression(JumpExpression { syntax: node }))
        }
        KotlinSyntaxKind::ThrowExpression => Ok(Expression::ThrowExpression(ThrowExpression {
            syntax: node,
        })),
        KotlinSyntaxKind::LambdaExpression => Ok(Expression::LambdaExpression(LambdaExpression {
            syntax: node,
        })),
        KotlinSyntaxKind::AnonymousFunctionExpression => Ok(
            Expression::AnonymousFunctionExpression(AnonymousFunctionExpression { syntax: node }),
        ),
        KotlinSyntaxKind::ObjectExpression => Ok(Expression::ObjectExpression(ObjectExpression {
            syntax: node,
        })),
        KotlinSyntaxKind::CollectionLiteralExpression => Ok(
            Expression::CollectionLiteralExpression(CollectionLiteralExpression { syntax: node }),
        ),
        _ => Err(node),
    }
}

impl<'source> PrimaryConstructor<'source> {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn constructor_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_tokens(self.syntax()).find(|token| token.text() == "constructor")
    }

    #[must_use]
    pub fn value_parameter_list(&self) -> Option<ValueParameterList<'source>> {
        child(self.syntax())
    }
}

impl<'source> ValueParameterList<'source> {
    pub fn entries(&self) -> impl Iterator<Item = ValueParameter<'source>> + use<'source> {
        children(self.syntax())
    }

    pub fn parameter_entries(
        &self,
    ) -> impl Iterator<Item = ValueParameterListEntry<'source>> + use<'source, '_> {
        separated_entries(self.syntax(), ValueParameter::cast, |parameter, comma| {
            ValueParameterListEntry { parameter, comma }
        })
    }

    pub fn parameter_entries_with_recovered(
        &self,
    ) -> impl Iterator<Item = RecoveredSeparatedListEntry<'source, ValueParameterListEntry<'source>>>
    + use<'source, '_> {
        recovered_separated_entries(
            self.syntax(),
            |kind| kind == KotlinSyntaxKind::ValueParameter,
            |syntax| ValueParameter { syntax },
            |parameter, comma| ValueParameterListEntry { parameter, comma },
            |token| {
                matches!(
                    token.kind(),
                    KotlinSyntaxKind::LParen | KotlinSyntaxKind::RParen
                )
            },
        )
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::RParen)
    }
}

impl<'source> ValueParameter<'source> {
    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn val_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::ValKw)
    }

    #[must_use]
    pub fn var_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::VarKw)
    }

    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn colon(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Colon)
    }

    #[must_use]
    pub fn ty(&self) -> Option<TypeReference<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn assign_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Assign)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }
}

impl<'source> ClassBody<'source> {
    pub fn members(&self) -> impl Iterator<Item = ClassMember<'source>> + use<'source> {
        children_family(self.syntax())
    }

    pub fn member_declaration_entries(
        &self,
    ) -> impl Iterator<Item = ClassMemberDeclarationEntry<'source>> + use<'source, '_> {
        separated_entries(
            self.syntax(),
            ClassMemberDeclaration::cast,
            |member, comma| ClassMemberDeclarationEntry { member, comma },
        )
    }

    pub fn member_declaration_entries_with_recovered(
        &self,
    ) -> impl Iterator<
        Item = RecoveredSeparatedListEntry<'source, ClassMemberDeclarationEntry<'source>>,
    > + use<'source, '_> {
        recovered_separated_entries(
            self.syntax(),
            |kind| kind == KotlinSyntaxKind::ClassMemberDeclaration,
            |syntax| ClassMemberDeclaration { syntax },
            |member, comma| ClassMemberDeclarationEntry { member, comma },
            |token| {
                matches!(
                    token.kind(),
                    KotlinSyntaxKind::LBrace | KotlinSyntaxKind::RBrace
                )
            },
        )
    }

    #[must_use]
    pub fn open_brace(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::RBrace)
    }
}

impl<'source> ClassMemberDeclaration<'source> {
    #[must_use]
    pub fn declaration(&self) -> Option<Declaration<'source>> {
        child_family(self.syntax())
    }

    #[must_use]
    pub fn statement(&self) -> Option<StatementSyntax<'source>> {
        child_family(self.syntax())
    }

    #[must_use]
    pub fn comma(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Comma)
    }
}

impl<'source> Block<'source> {
    #[must_use]
    pub fn open_brace(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::RBrace)
    }

    pub fn statements(&self) -> impl Iterator<Item = StatementSyntax<'source>> + use<'source> {
        children_family(self.syntax())
    }

    pub fn items(&self) -> impl Iterator<Item = BlockItem<'source>> + use<'source> {
        children_family(self.syntax())
    }

    pub fn items_with_recovered(
        &self,
    ) -> impl Iterator<Item = RecoveredSeparatedListEntry<'source, BlockItem<'source>>> + use<'source, '_>
    {
        recovered_body_entries(self.syntax(), classify_block_item, |kind| {
            matches!(kind, KotlinSyntaxKind::LBrace | KotlinSyntaxKind::RBrace)
        })
    }

    /// Returns true when no represented token (apart from the delimiters) is
    /// present between the open and close braces; i.e. the block's interior is
    /// only whitespace and comment trivia.
    #[must_use]
    pub fn inner_is_whitespace(&self) -> bool {
        let Some(open) = self.open_brace() else {
            return false;
        };
        let Some(close) = self.close_brace() else {
            return false;
        };
        let open_end = open.token_text_range().end().get();
        let close_start = close.token_text_range().start().get();
        !self.token_iter().any(|token| {
            let range = token.token_text_range();
            range.start().get() > open_end && range.end().get() < close_start
        })
    }
}

impl<'source> ValueArgumentList<'source> {
    pub fn entries(&self) -> impl Iterator<Item = ValueArgumentEntry<'source>> + use<'source, '_> {
        separated_entries(self.syntax(), ValueArgument::cast, |argument, comma| {
            ValueArgumentEntry { argument, comma }
        })
    }

    pub fn entries_with_recovered(
        &self,
    ) -> impl Iterator<Item = RecoveredSeparatedListEntry<'source, ValueArgumentEntry<'source>>>
    + use<'source, '_> {
        recovered_separated_entries(
            self.syntax(),
            |kind| kind == KotlinSyntaxKind::ValueArgument,
            |syntax| ValueArgument { syntax },
            |argument, comma| ValueArgumentEntry { argument, comma },
            |token| {
                matches!(
                    token.kind(),
                    KotlinSyntaxKind::LParen | KotlinSyntaxKind::RParen
                )
            },
        )
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::RParen)
    }
}

impl<'source> ValueArgument<'source> {
    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn assign_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Assign)
    }

    pub fn prefix_tokens(
        &self,
    ) -> impl Iterator<Item = KotlinSyntaxToken<'source>> + use<'source, '_> {
        let expression_start = self.expression().map_or_else(
            || self.text_range().end(),
            |expression| expression.text_range().start(),
        );
        child_tokens(self.syntax())
            .filter(move |token| token.token_text_range().end() <= expression_start)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }
}

impl<'source> TypeArgumentList<'source> {
    #[must_use]
    pub fn projection_list(&self) -> Option<TypeProjectionList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn open_angle(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Lt)
    }

    #[must_use]
    pub fn close_angle(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Gt)
    }
}

impl<'source> TypeProjectionList<'source> {
    pub fn entries(
        &self,
    ) -> impl Iterator<Item = TypeProjectionListEntry<'source>> + use<'source, '_> {
        separated_entries(self.syntax(), TypeArgument::cast, |argument, comma| {
            TypeProjectionListEntry { argument, comma }
        })
    }

    pub fn entries_with_recovered(
        &self,
    ) -> impl Iterator<Item = RecoveredSeparatedListEntry<'source, TypeProjectionListEntry<'source>>>
    + use<'source, '_> {
        recovered_separated_entries(
            self.syntax(),
            |kind| kind == KotlinSyntaxKind::TypeArgument,
            |syntax| TypeArgument { syntax },
            |argument, comma| TypeProjectionListEntry { argument, comma },
            |_| false,
        )
    }
}

impl<'source> TypeArgument<'source> {
    #[must_use]
    pub fn projection(&self) -> Option<TypeProjection<'source>> {
        child(self.syntax())
    }
}

impl<'source> TypeProjection<'source> {
    #[must_use]
    pub fn variance_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_tokens(self.syntax()).find(|token| token.text() == "in" || token.text() == "out")
    }

    #[must_use]
    pub fn star_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Star)
    }

    #[must_use]
    pub fn ty(&self) -> Option<TypeReference<'source>> {
        child(self.syntax())
    }
}

impl<'source> StringTemplateExpression<'source> {
    pub fn parts(&self) -> impl Iterator<Item = StringTemplatePart<'source>> + use<'source> {
        children_family(self.syntax())
    }
}

impl<'source> StringTemplateEntry<'source> {
    #[must_use]
    pub fn long_entry_start(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::LongTemplateEntryStart)
    }

    #[must_use]
    pub fn long_entry_end(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::LongTemplateEntryEnd)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }
}

impl<'source> StringTemplatePart<'source> {
    #[must_use]
    pub fn long_entry_start(&self) -> Option<KotlinSyntaxToken<'source>> {
        match self {
            Self::StringTemplateEntry(entry) => entry.long_entry_start(),
        }
    }

    #[must_use]
    pub fn long_entry_end(&self) -> Option<KotlinSyntaxToken<'source>> {
        match self {
            Self::StringTemplateEntry(entry) => entry.long_entry_end(),
        }
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        match self {
            Self::StringTemplateEntry(entry) => entry.expression(),
        }
    }
}

impl<'source> LiteralExpression<'source> {
    #[must_use]
    pub fn literal_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        self.first_token()
    }
}

impl<'source> NameExpression<'source> {
    #[must_use]
    pub fn name_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn at_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::At)
    }

    #[must_use]
    pub fn labeled_expression(&self) -> Option<Expression<'source>> {
        let at = self.at_token()?;
        children_family(self.syntax()).find(|expression: &Expression<'source>| {
            expression.text_range().start() >= at.token_text_range().end()
        })
    }
}

impl<'source> ThisExpression<'source> {
    #[must_use]
    pub fn this_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::ThisKw)
    }

    #[must_use]
    pub fn at_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::At)
    }

    #[must_use]
    pub fn label_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        label_token_after_at(self.syntax(), self.at_token())
    }
}

impl<'source> SuperExpression<'source> {
    #[must_use]
    pub fn super_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::SuperKw)
    }

    #[must_use]
    pub fn type_argument_list(&self) -> Option<TypeArgumentList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn at_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::At)
    }

    #[must_use]
    pub fn label_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        label_token_after_at(self.syntax(), self.at_token())
    }
}

fn label_token_after_at<'source>(
    syntax: &KotlinSyntaxNode<'source>,
    at: Option<KotlinSyntaxToken<'source>>,
) -> Option<KotlinSyntaxToken<'source>> {
    let at = at?;
    child_tokens(syntax).find(|token| {
        token.token_text_range().start() >= at.token_text_range().end()
            && matches!(
                token.kind(),
                KotlinSyntaxKind::Identifier | KotlinSyntaxKind::ThisKw | KotlinSyntaxKind::SuperKw
            )
    })
}

impl<'source> ParenthesizedExpression<'source> {
    #[must_use]
    pub fn open_paren(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::LParen)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::RParen)
    }
}

impl<'source> AnnotatedExpression<'source> {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation<'source>> + use<'source> {
        self.modifiers()
            .into_iter()
            .flat_map(|modifiers| modifiers.annotations())
    }

    #[must_use]
    pub fn modifiers(&self) -> Option<ModifierList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }
}

impl<'source> AssignmentExpression<'source> {
    pub fn operands(&self) -> impl Iterator<Item = Expression<'source>> + use<'source> {
        children_family(self.syntax())
    }

    #[must_use]
    pub fn left(&self) -> Option<Expression<'source>> {
        self.operands().next()
    }

    #[must_use]
    pub fn right(&self) -> Option<Expression<'source>> {
        self.operands().nth(1)
    }

    #[must_use]
    pub fn operator_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        let left = self.left()?;
        let right = self.right()?;
        child_tokens(self.syntax()).find(|token| {
            matches!(
                token.kind(),
                KotlinSyntaxKind::Assign
                    | KotlinSyntaxKind::PlusEq
                    | KotlinSyntaxKind::MinusEq
                    | KotlinSyntaxKind::StarEq
                    | KotlinSyntaxKind::SlashEq
                    | KotlinSyntaxKind::PercentEq
            ) && token.token_text_range().start() >= left.text_range().end()
                && token.token_text_range().end() <= right.text_range().start()
        })
    }
}

impl<'source> BinaryExpression<'source> {
    pub fn operands(&self) -> impl Iterator<Item = Expression<'source>> + use<'source> {
        children_family(self.syntax())
    }

    #[must_use]
    pub fn cast_type(&self) -> Option<TypeReference<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn operator_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        let mut operands = self.operands();
        let left = operands.next()?;
        let right_start = operands
            .next()
            .map(|right| right.text_range().start())
            .or_else(|| self.cast_type().map(|ty| ty.text_range().start()))?;
        child_tokens(self.syntax()).find(|token| {
            token.token_text_range().start() >= left.text_range().end()
                && token.token_text_range().end() <= right_start
        })
    }
}

/// Returns true if `a` and `b` are the same Kotlin operator identity.
///
/// Built-in operators are identified by kind alone (e.g. `Plus`, `Star`,
/// `Range`). Identifier-based infix-function operators (`a shl b`,
/// `a shr b`) additionally require text equality because the kind is
/// `Identifier` for both but the function name distinguishes them.
#[must_use]
pub fn operators_equivalent<'source>(
    a: &KotlinSyntaxToken<'source>,
    b: &KotlinSyntaxToken<'source>,
) -> bool {
    if a.kind() != b.kind() {
        return false;
    }
    if a.kind() == KotlinSyntaxKind::Identifier {
        return a.text() == b.text();
    }
    true
}

impl<'source> UnaryExpression<'source> {
    #[must_use]
    pub fn operator_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_tokens(self.syntax()).find(|token| {
            matches!(
                token.kind(),
                KotlinSyntaxKind::Plus
                    | KotlinSyntaxKind::Minus
                    | KotlinSyntaxKind::Bang
                    | KotlinSyntaxKind::PlusPlus
                    | KotlinSyntaxKind::MinusMinus
                    | KotlinSyntaxKind::Star
            )
        })
    }

    #[must_use]
    pub fn operand(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }
}

impl<'source> PostfixExpression<'source> {
    #[must_use]
    pub fn operand(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }

    #[must_use]
    pub fn operator_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_tokens(self.syntax())
            .filter(|token| {
                matches!(
                    token.kind(),
                    KotlinSyntaxKind::PlusPlus
                        | KotlinSyntaxKind::MinusMinus
                        | KotlinSyntaxKind::BangBang
                )
            })
            .last()
    }
}

impl<'source> IndexExpression<'source> {
    #[must_use]
    pub fn receiver(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }

    pub fn entries(&self) -> impl Iterator<Item = ValueArgumentEntry<'source>> + use<'source, '_> {
        separated_entries(self.syntax(), ValueArgument::cast, |argument, comma| {
            ValueArgumentEntry { argument, comma }
        })
    }

    pub fn entries_with_recovered(
        &self,
    ) -> impl Iterator<Item = RecoveredSeparatedListEntry<'source, ValueArgumentEntry<'source>>>
    + use<'source, '_> {
        let elements = self
            .syntax()
            .children_with_tokens()
            .skip_while(|element| {
                !matches!(
                    element,
                    SyntaxElement::Token(t) if t.kind() == KotlinSyntaxKind::LBracket
                )
            })
            .skip(1)
            .take_while(|element| {
                !matches!(
                    element,
                    SyntaxElement::Token(t) if t.kind() == KotlinSyntaxKind::RBracket
                )
            });

        recovered_separated_elements(
            elements,
            |kind| kind == KotlinSyntaxKind::ValueArgument,
            |syntax| ValueArgument { syntax },
            |argument, comma| ValueArgumentEntry { argument, comma },
            |_| false,
        )
    }

    #[must_use]
    pub fn open_bracket(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::LBracket)
    }

    #[must_use]
    pub fn close_bracket(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::RBracket)
    }
}

impl Expression<'_> {
    #[must_use]
    pub fn parent_role(&self) -> Option<ExpressionParentRole> {
        let parent = self.syntax().parent()?;
        let parent = AnyKotlinNode::cast(parent)?;

        access_parent_role(self, parent)
    }
}

fn access_parent_role(
    expression: &Expression,
    parent: AnyKotlinNode,
) -> Option<ExpressionParentRole> {
    match parent {
        AnyKotlinNode::NavigationExpression(parent) => parent
            .receiver()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::NavigationReceiver),
        AnyKotlinNode::CallExpression(parent) => parent
            .callee()
            .is_same_expression(expression)
            .then_some(ExpressionParentRole::CallCallee),
        AnyKotlinNode::IndexExpression(parent) => {
            if parent.receiver().is_same_expression(expression) {
                Some(ExpressionParentRole::IndexReceiver)
            } else {
                parent
                    .entries()
                    .any(|entry| entry.argument.expression().is_same_expression(expression))
                    .then_some(ExpressionParentRole::IndexArgument)
            }
        }
        _ => None,
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

impl<'source> CollectionLiteralExpression<'source> {
    pub fn entries(&self) -> impl Iterator<Item = ValueArgumentEntry<'source>> + use<'source, '_> {
        separated_entries(self.syntax(), ValueArgument::cast, |argument, comma| {
            ValueArgumentEntry { argument, comma }
        })
    }

    pub fn entries_with_recovered(
        &self,
    ) -> impl Iterator<Item = RecoveredSeparatedListEntry<'source, ValueArgumentEntry<'source>>>
    + use<'source, '_> {
        let elements = self
            .syntax()
            .children_with_tokens()
            .skip_while(|element| {
                !matches!(
                    element,
                    SyntaxElement::Token(t) if t.kind() == KotlinSyntaxKind::LBracket
                )
            })
            .skip(1)
            .take_while(|element| {
                !matches!(
                    element,
                    SyntaxElement::Token(t) if t.kind() == KotlinSyntaxKind::RBracket
                )
            });

        recovered_separated_elements(
            elements,
            |kind| kind == KotlinSyntaxKind::ValueArgument,
            |syntax| ValueArgument { syntax },
            |argument, comma| ValueArgumentEntry { argument, comma },
            |_| false,
        )
    }

    #[must_use]
    pub fn open_bracket(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::LBracket)
    }

    #[must_use]
    pub fn close_bracket(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::RBracket)
    }
}

impl<'source> NavigationExpression<'source> {
    #[must_use]
    pub fn receiver(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }

    #[must_use]
    pub fn operator_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        self.operator_tokens().first()
    }

    #[must_use]
    pub fn operator_tokens(&self) -> NavigationOperatorTokens<'source> {
        let mut first_single = None;
        let mut previous_question = None;

        for token in child_tokens(self.syntax()) {
            if token.kind() == KotlinSyntaxKind::Dot
                && let Some(question) = previous_question
                && tokens_are_adjacent(question, token)
            {
                return NavigationOperatorTokens::QuestionDot {
                    question,
                    dot: token,
                };
            }

            if first_single.is_none()
                && matches!(
                    token.kind(),
                    KotlinSyntaxKind::Dot
                        | KotlinSyntaxKind::SafeAccess
                        | KotlinSyntaxKind::ColonColon
                )
            {
                first_single = Some(token);
            }

            previous_question = (token.kind() == KotlinSyntaxKind::Question).then_some(token);
        }

        first_single.map_or(
            NavigationOperatorTokens::Missing,
            NavigationOperatorTokens::Single,
        )
    }

    #[must_use]
    pub fn operator_last_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        self.operator_tokens().last()
    }

    #[must_use]
    pub fn selector_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_tokens(self.syntax())
            .filter(|token| token.kind() == KotlinSyntaxKind::Identifier)
            .last()
    }
}

fn tokens_are_adjacent(left: KotlinSyntaxToken<'_>, right: KotlinSyntaxToken<'_>) -> bool {
    left.token_text_range().end() == right.token_text_range().start()
}

impl<'source> CallableReferenceExpression<'source> {
    #[must_use]
    pub fn receiver(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }

    #[must_use]
    pub fn separator_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::ColonColon)
    }

    #[must_use]
    pub fn target_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        let separator = self.separator_token()?;
        child_tokens(self.syntax()).find(|token| {
            token.token_text_range().start() >= separator.token_text_range().end()
                && matches!(
                    token.kind(),
                    KotlinSyntaxKind::Identifier | KotlinSyntaxKind::ClassKw
                )
        })
    }

    pub fn type_argument_lists(
        &self,
    ) -> impl Iterator<Item = TypeArgumentList<'source>> + use<'source> {
        children(self.syntax())
    }
}

impl<'source> CallExpression<'source> {
    #[must_use]
    pub fn callee(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }

    #[must_use]
    pub fn value_argument_list(&self) -> Option<ValueArgumentList<'source>> {
        child(self.syntax())
    }

    pub fn type_argument_lists(
        &self,
    ) -> impl Iterator<Item = TypeArgumentList<'source>> + use<'source> {
        children(self.syntax())
    }

    pub fn lambdas(&self) -> impl Iterator<Item = LambdaExpression<'source>> + use<'source, '_> {
        let callee_end = self.callee().map(|callee| callee.text_range().end());
        children(self.syntax()).filter(move |lambda: &LambdaExpression<'source>| {
            callee_end.is_none_or(|end| lambda.text_range().start() >= end)
        })
    }
}

impl<'source> LambdaExpression<'source> {
    #[must_use]
    pub fn inner_lambda(&self) -> Option<LambdaExpression<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn label_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_tokens(self.syntax()).find(|token| {
            matches!(
                token.kind(),
                KotlinSyntaxKind::Identifier | KotlinSyntaxKind::ThisKw | KotlinSyntaxKind::SuperKw
            )
        })
    }

    #[must_use]
    pub fn at_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::At)
    }

    #[must_use]
    pub fn open_brace(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::RBrace)
    }

    #[must_use]
    pub fn parameter_list(&self) -> Option<LambdaParameterList<'source>> {
        child(self.syntax())
    }

    pub fn body_items(&self) -> impl Iterator<Item = BlockItem<'source>> + use<'source> {
        children_family(self.syntax())
    }

    pub fn body_items_with_recovered(
        &self,
    ) -> impl Iterator<Item = RecoveredSeparatedListEntry<'source, BlockItem<'source>>> + use<'source, '_>
    {
        self.syntax()
            .children_with_tokens()
            .filter_map(|element| match element {
                SyntaxElement::Node(node)
                    if node.kind() == KotlinSyntaxKind::LambdaParameterList =>
                {
                    None
                }
                SyntaxElement::Node(node) => Some(match classify_block_item(node) {
                    Ok(item) => RecoveredSeparatedListEntry::Entry(item),
                    Err(node) => recovered_node_entry(node),
                }),
                SyntaxElement::Token(token) => (!matches!(
                    token.kind(),
                    KotlinSyntaxKind::LBrace | KotlinSyntaxKind::RBrace | KotlinSyntaxKind::Arrow
                ))
                .then_some(RecoveredSeparatedListEntry::Token(token)),
            })
    }
}

impl<'source> LambdaParameterList<'source> {
    pub fn parameters(&self) -> impl Iterator<Item = LambdaParameter<'source>> + use<'source> {
        children(self.syntax())
    }

    pub fn parameter_entries(
        &self,
    ) -> impl Iterator<Item = LambdaParameterListEntry<'source>> + use<'source, '_> {
        separated_entries(self.syntax(), LambdaParameter::cast, |parameter, comma| {
            LambdaParameterListEntry { parameter, comma }
        })
    }

    pub fn parameter_entries_with_recovered(
        &self,
    ) -> impl Iterator<
        Item = RecoveredSeparatedListEntry<'source, LambdaParameterListEntry<'source>>,
    > + use<'source, '_> {
        recovered_separated_entries(
            self.syntax(),
            |kind| kind == KotlinSyntaxKind::LambdaParameter,
            |syntax| LambdaParameter { syntax },
            |parameter, comma| LambdaParameterListEntry { parameter, comma },
            |token| token.kind() == KotlinSyntaxKind::Arrow,
        )
    }

    #[must_use]
    pub fn arrow_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Arrow)
    }
}

impl<'source> LambdaParameter<'source> {
    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn destructuring_declaration(&self) -> Option<DestructuringDeclaration<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn colon(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Colon)
    }

    #[must_use]
    pub fn ty(&self) -> Option<TypeReference<'source>> {
        child(self.syntax())
    }
}

impl<'source> IfExpression<'source> {
    #[must_use]
    pub fn if_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::IfKw)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::RParen)
    }

    #[must_use]
    pub fn condition(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }

    #[must_use]
    pub fn then_branch(&self) -> Option<Expression<'source>> {
        let condition_end = self.condition().map_or_else(
            || self.text_range().start(),
            |condition| condition.text_range().end(),
        );
        let else_start = self.else_token().map_or_else(
            || self.text_range().end(),
            |token| token.token_text_range().start(),
        );

        self.expressions_after(condition_end)
            .find(|expression| expression.text_range().start() < else_start)
    }

    #[must_use]
    pub fn else_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::ElseKw)
    }

    #[must_use]
    pub fn else_branch(&self) -> Option<Expression<'source>> {
        let else_end = self.else_token()?.token_text_range().end();
        self.expressions_after(else_end).next()
    }

    fn expressions_after(
        &self,
        offset: jolt_text::TextSize,
    ) -> impl Iterator<Item = Expression<'source>> + use<'source> {
        children_family(self.syntax()).filter(move |expression: &Expression<'source>| {
            expression.text_range().start() >= offset
        })
    }
}

impl<'source> WhenEntry<'source> {
    pub fn conditions(&self) -> impl Iterator<Item = WhenConditionSyntax<'source>> + use<'source> {
        children_family(self.syntax())
    }

    pub fn condition_entries(
        &self,
    ) -> impl Iterator<Item = WhenConditionEntry<'source>> + use<'source, '_> {
        separated_entries(self.syntax(), WhenCondition::cast, |condition, comma| {
            WhenConditionEntry { condition, comma }
        })
    }

    pub fn condition_entries_with_recovered(
        &self,
    ) -> impl Iterator<Item = RecoveredSeparatedListEntry<'source, WhenConditionEntry<'source>>>
    + use<'source, '_> {
        let elements = self
            .syntax()
            .children_with_tokens()
            .take_while(|element| match element {
                SyntaxElement::Token(token) => !matches!(
                    token.kind(),
                    KotlinSyntaxKind::Arrow | KotlinSyntaxKind::ElseKw
                ),
                SyntaxElement::Node(node) => node.kind() != KotlinSyntaxKind::WhenGuard,
            });

        recovered_separated_elements(
            elements,
            |kind| kind == KotlinSyntaxKind::WhenCondition,
            |syntax| WhenCondition { syntax },
            |condition, comma| WhenConditionEntry { condition, comma },
            |_| false,
        )
    }

    #[must_use]
    pub fn guard(&self) -> Option<WhenGuard<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn else_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::ElseKw)
    }

    #[must_use]
    pub fn arrow_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Arrow)
    }

    #[must_use]
    pub fn body_expression(&self) -> Option<Expression<'source>> {
        let arrow_end = self.arrow_token()?.token_text_range().end();
        self.expressions()
            .find(|expression| expression.text_range().start() >= arrow_end)
    }

    pub fn expressions(&self) -> impl Iterator<Item = Expression<'source>> + use<'source> {
        children_family(self.syntax())
    }
}

impl<'source> WhenCondition<'source> {
    #[must_use]
    pub fn keyword_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_tokens(self.syntax()).find(|token| {
            matches!(
                token.kind(),
                KotlinSyntaxKind::IsKw
                    | KotlinSyntaxKind::NotIs
                    | KotlinSyntaxKind::InKw
                    | KotlinSyntaxKind::NotIn
            )
        })
    }

    #[must_use]
    pub fn ty(&self) -> Option<TypeReference<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }
}

impl<'source> WhenGuard<'source> {
    #[must_use]
    pub fn if_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::IfKw)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }
}

impl<'source> WhenExpression<'source> {
    #[must_use]
    pub fn when_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::WhenKw)
    }

    #[must_use]
    pub fn subject(&self) -> Option<WhenSubject<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn open_brace(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::LBrace)
    }

    #[must_use]
    pub fn close_brace(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::RBrace)
    }

    pub fn entries(&self) -> impl Iterator<Item = WhenEntry<'source>> + use<'source> {
        children(self.syntax())
    }
}

impl<'source> WhenSubject<'source> {
    #[must_use]
    pub fn open_paren(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::RParen)
    }

    #[must_use]
    pub fn val_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::ValKw)
    }

    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn assign_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Assign)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }
}

impl<'source> TryExpression<'source> {
    #[must_use]
    pub fn try_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::TryKw)
    }

    #[must_use]
    pub fn block(&self) -> Option<Block<'source>> {
        child(self.syntax())
    }

    pub fn catch_clauses(&self) -> impl Iterator<Item = CatchClause<'source>> + use<'source> {
        children(self.syntax())
    }

    #[must_use]
    pub fn finally_clause(&self) -> Option<FinallyClause<'source>> {
        child(self.syntax())
    }
}

impl<'source> ForStatement<'source> {
    #[must_use]
    pub fn for_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::ForKw)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::RParen)
    }

    #[must_use]
    pub fn in_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::InKw)
    }

    #[must_use]
    pub fn destructuring_declaration(&self) -> Option<DestructuringDeclaration<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn variable_expression(&self) -> Option<Expression<'source>> {
        let in_start = self.in_token()?.token_text_range().start();
        children_family(self.syntax())
            .find(|expression: &Expression<'source>| expression.text_range().end() <= in_start)
    }

    #[must_use]
    pub fn iterable_expression(&self) -> Option<Expression<'source>> {
        let in_end = self.in_token()?.token_text_range().end();
        children_family(self.syntax())
            .find(|expression: &Expression<'source>| expression.text_range().start() >= in_end)
    }

    #[must_use]
    pub fn block(&self) -> Option<Block<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn body_expression(&self) -> Option<Expression<'source>> {
        let header_end = self
            .close_paren()
            .or_else(|| {
                self.iterable_expression()
                    .and_then(|expression| expression.last_token())
            })
            .or_else(|| self.in_token())
            .or_else(|| self.for_token())?
            .token_text_range()
            .end();

        children_family(self.syntax())
            .find(|expression: &Expression<'source>| expression.text_range().start() >= header_end)
    }
}

impl<'source> WhileStatement<'source> {
    #[must_use]
    pub fn while_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::WhileKw)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::RParen)
    }

    #[must_use]
    pub fn condition(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }

    #[must_use]
    pub fn block(&self) -> Option<Block<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn body_expression(&self) -> Option<Expression<'source>> {
        let condition_end = match self.condition() {
            Some(condition) => condition.text_range().end(),
            None => self.while_token()?.token_text_range().end(),
        };
        children_family(self.syntax()).find(|expression: &Expression<'source>| {
            expression.text_range().start() >= condition_end
        })
    }
}

impl<'source> DoWhileStatement<'source> {
    #[must_use]
    pub fn do_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::DoKw)
    }

    #[must_use]
    pub fn while_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::WhileKw)
    }

    #[must_use]
    pub fn open_paren(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::LParen)
    }

    #[must_use]
    pub fn close_paren(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::RParen)
    }

    #[must_use]
    pub fn condition(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }

    #[must_use]
    pub fn block(&self) -> Option<Block<'source>> {
        child(self.syntax())
    }
}

impl<'source> ObjectExpression<'source> {
    #[must_use]
    pub fn object_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::ObjectKw)
    }

    #[must_use]
    pub fn colon(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::Colon)
    }

    #[must_use]
    pub fn delegation_specifier_list(&self) -> Option<DelegationSpecifierList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn body(&self) -> Option<ClassBody<'source>> {
        child(self.syntax())
    }
}

impl<'source> CatchClause<'source> {
    #[must_use]
    pub fn catch_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_tokens(self.syntax()).find(|token| token.text() == "catch")
    }

    #[must_use]
    pub fn value_parameter_list(&self) -> Option<ValueParameterList<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn block(&self) -> Option<Block<'source>> {
        child(self.syntax())
    }
}

impl<'source> FinallyClause<'source> {
    #[must_use]
    pub fn finally_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_tokens(self.syntax()).find(|token| token.text() == "finally")
    }

    #[must_use]
    pub fn block(&self) -> Option<Block<'source>> {
        child(self.syntax())
    }
}

impl<'source> DestructuringDeclaration<'source> {
    pub fn entries_with_commas(
        &self,
    ) -> impl Iterator<Item = DestructuringDeclarationEntry<'source>> + use<'source, '_> {
        separated_entries(self.syntax(), DestructuringEntry::cast, |entry, comma| {
            DestructuringDeclarationEntry { entry, comma }
        })
    }

    pub fn entries_with_recovered(
        &self,
    ) -> impl Iterator<
        Item = RecoveredSeparatedListEntry<'source, DestructuringDeclarationEntry<'source>>,
    > + use<'source, '_> {
        recovered_separated_entries(
            self.syntax(),
            |kind| kind == KotlinSyntaxKind::DestructuringEntry,
            |syntax| DestructuringEntry { syntax },
            |entry, comma| DestructuringDeclarationEntry { entry, comma },
            |token| {
                matches!(
                    token.kind(),
                    KotlinSyntaxKind::LParen
                        | KotlinSyntaxKind::RParen
                        | KotlinSyntaxKind::LBracket
                        | KotlinSyntaxKind::RBracket
                )
            },
        )
    }

    #[must_use]
    pub fn open_delimiter(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::LParen)
            .or_else(|| child_token(self.syntax(), KotlinSyntaxKind::LBracket))
    }

    #[must_use]
    pub fn close_delimiter(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::RParen)
            .or_else(|| child_token(self.syntax(), KotlinSyntaxKind::RBracket))
    }
}

impl<'source> DestructuringDeclaration<'source> {
    pub fn entries(
        &self,
    ) -> impl Iterator<Item = DestructuringPatternEntry<'source>> + use<'source> {
        children_family(self.syntax())
    }
}

impl<'source> DestructuringEntry<'source> {
    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }
}

impl TypeSyntax<'_> {
    #[must_use]
    pub fn is_nullable(&self) -> bool {
        matches!(self, Self::NullableType(_))
    }

    #[must_use]
    pub fn is_function_like(&self) -> bool {
        matches!(self, Self::FunctionType(_) | Self::ContextFunctionType(_))
    }
}

impl<'source> Expression<'source> {
    pub fn child_expressions(&self) -> impl Iterator<Item = Expression<'source>> + use<'source> {
        children_family(self.syntax())
    }
}

impl<'source> Statement<'source> {
    #[must_use]
    pub fn statement(&self) -> Option<StatementSyntax<'source>> {
        child_family(self.syntax())
    }
}

impl<'source> ExpressionStatement<'source> {
    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }

    pub fn entries_with_recovered(
        &self,
    ) -> impl Iterator<Item = RecoveredSeparatedListEntry<'source, Expression<'source>>> + use<'source, '_>
    {
        recovered_body_entries(self.syntax(), classify_expression, |_| false)
    }
}

impl<'source> LocalDeclaration<'source> {
    #[must_use]
    pub fn property_declaration(&self) -> Option<PropertyDeclaration<'source>> {
        child(self.syntax())
    }
}

impl<'source> StatementSyntax<'source> {
    pub fn expressions(&self) -> impl Iterator<Item = Expression<'source>> + use<'source> {
        children_family(self.syntax())
    }
}

impl<'source> JumpExpression<'source> {
    #[must_use]
    pub fn keyword_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_tokens(self.syntax()).find(|token| {
            matches!(
                token.kind(),
                KotlinSyntaxKind::ReturnKw
                    | KotlinSyntaxKind::BreakKw
                    | KotlinSyntaxKind::ContinueKw
            )
        })
    }

    #[must_use]
    pub fn at_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::At)
    }

    #[must_use]
    pub fn label_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        let at = self.at_token()?;
        child_tokens(self.syntax()).find(|token| {
            token.kind() == KotlinSyntaxKind::Identifier
                && token.token_text_range().start() > at.token_text_range().start()
        })
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }
}

impl<'source> ThrowExpression<'source> {
    #[must_use]
    pub fn throw_token(&self) -> Option<KotlinSyntaxToken<'source>> {
        child_token(self.syntax(), KotlinSyntaxKind::ThrowKw)
    }

    #[must_use]
    pub fn expression(&self) -> Option<Expression<'source>> {
        child_family(self.syntax())
    }
}
