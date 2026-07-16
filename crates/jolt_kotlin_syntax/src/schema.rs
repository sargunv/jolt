// The single declarative authority for syntax kinds, typed CST wrappers, and tree shape.
// The recursive-descent parser remains handwritten.
macro_rules! kotlin_syntax_schema {
    ($emit:ident) => {
        $emit! {
            tokens {
                Eof,
                Unknown,
                Reserved,
                Identifier,
                FieldIdentifier,
                IntegerLiteral,
                FloatLiteral,
                CharacterLiteral,
                InterpolationPrefix,
                OpenQuote,
                ClosingQuote,
                RegularStringPart,
                EscapeSequence,
                ShortTemplateEntryStart,
                LongTemplateEntryStart,
                LongTemplateEntryEnd,
                DanglingNewline,
                PackageKw,
                AsKw,
                TypeAliasKw,
                ClassKw,
                ThisKw,
                SuperKw,
                ValKw,
                VarKw,
                FunKw,
                ForKw,
                NullKw,
                TrueKw,
                FalseKw,
                IsKw,
                InKw,
                ThrowKw,
                ReturnKw,
                BreakKw,
                ContinueKw,
                ObjectKw,
                IfKw,
                TryKw,
                ElseKw,
                WhileKw,
                DoKw,
                WhenKw,
                InterfaceKw,
                TypeOfKw,
                AsSafe,
                AllKw,
                FileKw,
                FieldKw,
                PropertyKw,
                ReceiverKw,
                ParamKw,
                SetParamKw,
                DelegateKw,
                ImportKw,
                WhereKw,
                ByKw,
                GetKw,
                SetKw,
                ConstructorKw,
                InitKw,
                ContextKw,
                CatchKw,
                DynamicKw,
                FinallyKw,
                AbstractKw,
                EnumKw,
                ContractKw,
                OpenKw,
                InnerKw,
                OverrideKw,
                PrivateKw,
                PublicKw,
                InternalKw,
                ProtectedKw,
                OutKw,
                VarargKw,
                ReifiedKw,
                CompanionKw,
                SealedKw,
                FinalKw,
                LateinitKw,
                DataKw,
                ValueKw,
                InlineKw,
                NoinlineKw,
                TailrecKw,
                ExternalKw,
                AnnotationKw,
                CrossinlineKw,
                OperatorKw,
                InfixKw,
                ConstKw,
                SuspendKw,
                ExpectKw,
                ActualKw,
                LBracket,
                RBracket,
                LBrace,
                RBrace,
                LParen,
                RParen,
                Dot,
                Question,
                ColonColon,
                Colon,
                Semicolon,
                DoubleSemicolon,
                Range,
                RangeUntil,
                Assign,
                Hash,
                At,
                Comma,
                EolOrSemicolon,
                PlusPlus,
                MinusMinus,
                Star,
                Plus,
                Minus,
                Bang,
                Slash,
                Percent,
                Lt,
                Gt,
                LtEq,
                GtEq,
                EqEqEq,
                Arrow,
                DoubleArrow,
                BangEqEqEq,
                EqEq,
                BangEq,
                BangBang,
                AndAnd,
                Amp,
                OrOr,
                SafeAccess,
                Elvis,
                StarEq,
                SlashEq,
                PercentEq,
                PlusEq,
                MinusEq,
                NotIn,
                NotIs,
            }
            categories {
                KotlinFileItem => BogusKotlinFileItem {
                    PackageHeader,
                    ImportDirectiveList,
                    ClassDeclaration,
                    InterfaceDeclaration,
                    ObjectDeclaration,
                    CompanionObject,
                    EnumEntry,
                    FunctionDeclaration,
                    PropertyDeclaration,
                    TypeAliasDeclaration,
                    SecondaryConstructor,
                    InitializerBlock,
                    Statement,
                }
                Declaration => BogusDeclaration {
                    ClassDeclaration,
                    InterfaceDeclaration,
                    ObjectDeclaration,
                    CompanionObject,
                    EnumEntry,
                    FunctionDeclaration,
                    PropertyDeclaration,
                    TypeAliasDeclaration,
                    SecondaryConstructor,
                    InitializerBlock,
                }
                ClassMember => BogusClassMember {
                    ClassMemberDeclaration,
                    ClassDeclaration,
                    InterfaceDeclaration,
                    ObjectDeclaration,
                    CompanionObject,
                    EnumEntry,
                    FunctionDeclaration,
                    PropertyDeclaration,
                    TypeAliasDeclaration,
                    SecondaryConstructor,
                    InitializerBlock,
                    PropertyAccessor,
                    ExplicitBackingField,
                    Statement,
                }
                TypeSyntax => BogusType {
                    UserType,
                    NullableType,
                    FunctionType,
                    ContextFunctionType,
                    ReceiverType,
                    ParenthesizedType,
                    DefinitelyNonNullableType,
                }
                FunctionTypeForm => BogusFunctionTypeForm {
                    SuspendedFunctionType,
                    ArrowFunctionType,
                }
                DefinitelyNonNullableTypeForm => BogusDefinitelyNonNullableTypeForm {
                    IntersectionDefinitelyNonNullableType,
                    BangDefinitelyNonNullableType,
                }
                Expression => BogusExpression {
                    AssignmentExpression,
                    BinaryExpression,
                    UnaryExpression,
                    PostfixExpression,
                    CallExpression,
                    IndexExpression,
                    NavigationExpression,
                    CallableReferenceExpression,
                    LiteralExpression,
                    StringTemplateExpression,
                    NameExpression,
                    ThisExpression,
                    SuperExpression,
                    ParenthesizedExpression,
                    AnnotatedExpression,
                    IfExpression,
                    WhenExpression,
                    TryExpression,
                    ForStatement,
                    WhileStatement,
                    DoWhileStatement,
                    JumpExpression,
                    ThrowExpression,
                    LambdaExpression,
                    AnonymousFunctionExpression,
                    ObjectExpression,
                    CollectionLiteralExpression,
                }
                WhenConditionSyntax => BogusWhenCondition {
                    WhenCondition,
                    WhenGuard,
                }
                TryClause => BogusTryClause {
                    CatchClause,
                    FinallyClause,
                }
                StringTemplatePart => BogusStringTemplatePart {
                    StringTemplateEntry,
                }
                NavigationSelector => BogusNavigationSelector {
                    ThisExpression,
                    SuperExpression,
                }
                LambdaForm => BogusLambdaForm {
                    LabeledLambdaExpression,
                    LambdaBody,
                }
                LambdaParameterListEntry => BogusLambdaParameter {
                    LambdaParameter,
                }
                ValueArgumentListEntry => BogusValueArgument {
                    ValueArgument,
                }
                TypeArgumentListEntry => BogusTypeArgument {
                    TypeReference,
                    TypeProjection,
                    StarProjection,
                }
                TypeParameterListEntry => BogusTypeParameter {
                    TypeParameter,
                }
                TypeConstraintListEntry => BogusTypeConstraint {
                    TypeConstraint,
                }
                FunctionTypeParameterListEntry => BogusFunctionTypeParameter {
                    FunctionTypeParameter,
                }
                ContextParameterListEntry => BogusContextParameter {
                    ContextParameter,
                }
                ValueParameterListEntry => BogusValueParameter {
                    ValueParameter,
                }
                ValueParameterName => BogusValueParameterName {
                    Name,
                    DestructuringDeclaration,
                }
                CallableDeclarationName => BogusCallableDeclarationName {
                    Name,
                    CallableName,
                }
                PropertyBinding => BogusPropertyBinding {
                    Name,
                    CallableName,
                    DestructuringDeclaration,
                }
                DeclarationBody => BogusDeclarationBody {
                    BlockBody,
                    ExpressionBody,
                }
                PropertyBodyMember => BogusPropertyBodyMember {
                    ExplicitBackingField,
                    PropertyAccessor,
                }
                DelegationSpecifierEntry => BogusDelegationSpecifier {
                    DelegationSpecifier,
                }
                UserTypeSegmentSyntax => BogusUserTypeSegment {
                    UserTypeSegment,
                }
                DestructuringPatternEntry => BogusDestructuringEntry {
                    DestructuringEntry,
                }
                StatementSyntax => BogusStatement {
                    Statement,
                    ExpressionStatement,
                    LocalDeclaration,
                    EmptyStatement,
                    Block,
                }
                BlockItem => BogusBlockItem {
                    Statement,
                    ExpressionStatement,
                    LocalDeclaration,
                    EmptyStatement,
                    Block,
                    ClassDeclaration,
                    InterfaceDeclaration,
                    ObjectDeclaration,
                    FunctionDeclaration,
                    PropertyDeclaration,
                    TypeAliasDeclaration,
                    SecondaryConstructor,
                    InitializerBlock,
                }
                QualifiedNameSegment => BogusQualifiedNameSegment {
                    Name,
                }
            }
            nodes {
                KotlinFile => KotlinFile [kotlin_file valid] {
                    annotations: required (list AnnotationList);
                    items: required (list KotlinFileItemList);
                    eof: required (token Eof);
                }
                PackageHeader => PackageHeader [package_header valid] {
                    package_token: required (token PackageKw);
                    name: required (node QualifiedName);
                    suffix: optional (node BogusPackageSuffix);
                    terminators: required (list TerminatorList);
                }
                ImportDirective => ImportDirective [import_directive valid] {
                    import_token: required (contextual "import");
                    name: required (node QualifiedName);
                    on_demand: optional (node ImportOnDemandSuffix);
                    alias: optional (node ImportAlias);
                    suffix: optional (node BogusImportSuffix);
                    terminators: required (list TerminatorList);
                }
                ImportOnDemandSuffix => ImportOnDemandSuffix [import_on_demand_suffix valid] {
                    dot: required (token Dot);
                    star: required (token Star);
                }
                ImportAlias => ImportAlias [import_alias valid] {
                    alias_keyword: required (token_set [AsKw, Identifier]);
                    name: required (node Name);
                }
                BogusPackageSuffix => BogusPackageSuffix [bogus_package_suffix malformed] {
                    elements: many (any_element) => BogusPackageSuffixElement;
                }
                BogusImportSuffix => BogusImportSuffix [bogus_import_suffix malformed] {
                    elements: many (any_element) => BogusImportSuffixElement;
                }
                Annotation => Annotation [annotation valid] {
                    sigil: required (token_set [At, Hash]);
                    use_site_target: optional (node AnnotationUseSiteTarget);
                    name: required (node_set [QualifiedName, Name]);
                    argument_list: optional (node AnnotationArgumentList);
                }
                AnnotationUseSiteTarget => AnnotationUseSiteTarget [annotation_use_site_target valid] {
                    target: required (choice [
                        (contextual "all"), (contextual "file"), (contextual "field"),
                        (contextual "property"), (contextual "receiver"),
                        (contextual "param"), (contextual "setparam"),
                        (contextual "delegate"), (contextual "get"), (contextual "set")
                    ]);
                    colon: required (token Colon);
                }
                AnnotationArgumentList => AnnotationArgumentList [annotation_argument_list valid] {
                    open_paren: required (token LParen);
                    entries: required (list ValueArgumentSeparatedList);
                    close_paren: required (token RParen);
                }
                ValueArgumentList => ValueArgumentList [value_argument_list valid] {
                    open_paren: required (token LParen);
                    entries: required (list ValueArgumentEntryList);
                    close_paren: required (token RParen);
                }
                ValueArgument => ValueArgument [value_argument valid] {
                    prefix: required (list ValueArgumentPrefixList);
                    name: optional (node Name);
                    assign: optional (token Assign);
                    expression: required (category Expression);
                }
                Name => Name [name valid] {
                    identifier: required (token_set [Identifier, FieldIdentifier]);
                }
                QualifiedName => QualifiedName [qualified_name valid] {
                    segments: required (list QualifiedNameSegmentList);
                }
                CallableName => CallableName [callable_name valid] {
                    receiver: required (node TypeReference);
                    dot: required (token Dot);
                    name: required (node Name);
                }
                TypeArgumentList => TypeArgumentList [type_argument_list valid] {
                    open_angle: required (token Lt);
                    entries: required (list TypeProjectionSeparatedList);
                    close_angle: required (token Gt);
                }
                ClassDeclaration => ClassDeclaration [class_declaration valid] {
                    leading_modifiers: required (list ModifierList) [disambiguate leftmost_longest];
                    context: optional (node ContextParameterClause);
                    post_context_modifiers: required (list ModifierList);
                    class_token: required (token ClassKw);
                    name: required (node Name);
                    type_parameters: optional (node TypeParameterList);
                    primary_constructor: optional (node PrimaryConstructor);
                    delegation: optional (node DelegationClause);
                    constraints: optional (node TypeConstraintList);
                    body: optional (node ClassBody);
                }
                InterfaceDeclaration => InterfaceDeclaration [interface_declaration valid] {
                    leading_modifiers: required (list ModifierList) [disambiguate leftmost_longest];
                    context: optional (node ContextParameterClause);
                    post_context_modifiers: required (list ModifierList);
                    fun_token: optional (token FunKw);
                    interface_token: required (token InterfaceKw);
                    name: required (node Name);
                    type_parameters: optional (node TypeParameterList);
                    delegation: optional (node DelegationClause);
                    constraints: optional (node TypeConstraintList);
                    body: optional (node ClassBody);
                }
                ObjectDeclaration => ObjectDeclaration [object_declaration valid] {
                    leading_modifiers: required (list ModifierList) [disambiguate leftmost_longest];
                    context: optional (node ContextParameterClause);
                    post_context_modifiers: required (list ModifierList);
                    object_token: required (token ObjectKw);
                    name: optional (node Name);
                    delegation: optional (node DelegationClause);
                    body: optional (node ClassBody);
                }
                CompanionObject => CompanionObject [companion_object valid] {
                    leading_modifiers: required (list ModifierList) [disambiguate leftmost_longest];
                    context: optional (node ContextParameterClause);
                    post_context_modifiers: required (list ModifierList);
                    companion_token: required (contextual "companion");
                    object_token: optional (token ObjectKw);
                    name: optional (node Name);
                    delegation: optional (node DelegationClause);
                    body: optional (node ClassBody);
                }
                EnumEntry => EnumEntry [enum_entry valid] {
                    modifiers: required (list ModifierList);
                    name: required (node Name);
                    arguments: optional (node ValueArgumentList);
                    body: optional (node ClassBody);
                    comma: optional (token Comma);
                }
                ClassBody => ClassBody [class_body valid] {
                    open_brace: required (token LBrace);
                    members: required (list ClassMemberList);
                    close_brace: required (token RBrace);
                }
                ClassMemberDeclaration => ClassMemberDeclaration [class_member_declaration valid] {
                    member: required (choice [(category Declaration), (node Statement)]);
                    comma: optional (token Comma);
                }
                PrimaryConstructor => PrimaryConstructor [primary_constructor valid] {
                    modifiers: required (list ModifierList);
                    constructor_token: optional (contextual "constructor");
                    parameters: required (node ValueParameterList);
                }
                SecondaryConstructor => SecondaryConstructor [secondary_constructor valid] {
                    leading_modifiers: required (list ModifierList) [disambiguate leftmost_longest];
                    context: optional (node ContextParameterClause);
                    post_context_modifiers: required (list ModifierList);
                    constructor_token: required (contextual "constructor");
                    parameters: required (node ValueParameterList);
                    delegation: optional (node ConstructorDelegation);
                    body: optional (node Block);
                }
                ConstructorDelegation => ConstructorDelegation [constructor_delegation valid] {
                    colon: required (token Colon);
                    call: required (node ConstructorDelegationCall);
                }
                ConstructorDelegationCall => ConstructorDelegationCall [constructor_delegation_call valid] {
                    expression: required (category Expression);
                }
                InitializerBlock => InitializerBlock [initializer_block valid] {
                    init_token: required (contextual "init");
                    block: required (node Block);
                }
                FunctionDeclaration => FunctionDeclaration [function_declaration valid] {
                    leading_modifiers: required (list ModifierList) [disambiguate leftmost_longest];
                    context: optional (node ContextParameterClause);
                    post_context_modifiers: required (list ModifierList);
                    fun_token: required (token FunKw);
                    type_parameters: optional (node TypeParameterList);
                    receiver_modifiers: required (list ModifierList);
                    name: required (category CallableDeclarationName);
                    parameters: required (node ValueParameterList);
                    return_colon: optional (token Colon);
                    return_type: optional (node TypeReference);
                    constraints: optional (node TypeConstraintList);
                    body: optional (category DeclarationBody);
                }
                PropertyDeclaration => PropertyDeclaration [property_declaration valid] {
                    leading_modifiers: required (list ModifierList) [disambiguate leftmost_longest];
                    context: optional (node ContextParameterClause);
                    post_context_modifiers: required (list ModifierList);
                    binding_keyword: required (token_set [ValKw, VarKw]);
                    type_parameters: optional (node TypeParameterList);
                    binding: required (category PropertyBinding);
                    type_colon: optional (token Colon);
                    r#type: optional (node TypeReference);
                    constraints: optional (node TypeConstraintList);
                    initializer: optional (node PropertyInitializer);
                    body_members: required (list PropertyBodyMemberList);
                }
                PropertyAccessor => PropertyAccessor [property_accessor valid] {
                    modifiers: required (list ModifierList);
                    keyword: required (choice [(contextual "get"), (contextual "set")]);
                    parameters: optional (node ValueParameterList);
                    return_colon: optional (token Colon);
                    return_type: optional (node TypeReference);
                    body: optional (category DeclarationBody);
                }
                BlockBody => BlockBody [block_body valid] {
                    block: required (node Block);
                }
                ExpressionBody => ExpressionBody [expression_body valid] {
                    assign: required (token Assign);
                    expression: required (category Expression);
                }
                PropertyInitializer => PropertyInitializer [property_initializer valid] {
                    operator: required (choice [(token Assign), (contextual "by")]);
                    expression: required (category Expression);
                }
                ExplicitBackingField => ExplicitBackingField [explicit_backing_field valid] {
                    field_token: required (choice [(contextual "field"), (token FieldIdentifier)]);
                    assign: required (token Assign);
                    expression: required (category Expression);
                }
                TypeAliasDeclaration => TypeAliasDeclaration [type_alias_declaration valid] {
                    leading_modifiers: required (list ModifierList) [disambiguate leftmost_longest];
                    context: optional (node ContextParameterClause);
                    post_context_modifiers: required (list ModifierList);
                    typealias_token: required (token TypeAliasKw);
                    name: required (node Name);
                    type_parameters: optional (node TypeParameterList);
                    assign: required (token Assign);
                    r#type: required (node TypeReference);
                }
                TypeParameterList => TypeParameterList [type_parameter_list valid] {
                    open_angle: required (token Lt);
                    entries: required (list TypeParameterSeparatedList);
                    close_angle: required (token Gt);
                }
                TypeParameter => TypeParameter [type_parameter valid] {
                    modifiers: required (list ModifierList);
                    variance: optional (choice [(token InKw), (contextual "out")]);
                    name: required (node Name);
                    colon: optional (token Colon);
                    bound: optional (node TypeReference);
                }
                TypeConstraintList => TypeConstraintList [type_constraint_list valid] {
                    where_token: required (contextual "where");
                    entries: required (list TypeConstraintSeparatedList);
                }
                TypeConstraint => TypeConstraint [type_constraint valid] {
                    name: required (node Name);
                    colon: required (token Colon);
                    bound: required (node TypeReference);
                }
                ContextParameterClause => ContextParameterClause [context_parameter_clause valid] {
                    context_token: required (contextual "context");
                    open_paren: required (token LParen);
                    entries: required (list ContextParameterSeparatedList);
                    close_paren: required (token RParen);
                }
                ContextParameter => ContextParameter [context_parameter valid] {
                    name: optional (node Name);
                    colon: optional (token Colon);
                    r#type: required (node TypeReference);
                    assign: optional (token Assign);
                    default: optional (category Expression);
                }
                DelegationClause => DelegationClause [delegation_clause valid] {
                    colon: required (token Colon);
                    specifiers: required (list DelegationSpecifierSeparatedList);
                }
                DelegationSpecifier => DelegationSpecifier [delegation_specifier valid] {
                    r#type: required (node TypeReference);
                    arguments: optional (node ValueArgumentList);
                    by_clause: optional (node DelegationByClause);
                }
                DelegationByClause => DelegationByClause [delegation_by_clause valid] {
                    by_token: required (contextual "by");
                    delegate: required (category Expression);
                }
                UserType => UserType [user_type valid] {
                    segments: required (list UserTypeSegmentList);
                }
                UserTypeSegment => UserTypeSegment [user_type_segment valid] {
                    annotations: optional (list AnnotationList);
                    name: required (token_set [Identifier, FieldIdentifier]);
                    arguments: optional (node TypeArgumentList);
                }
                NullableType => NullableType [nullable_type valid] {
                    inner: required (category TypeSyntax);
                    question: required (token Question);
                }
                FunctionType => FunctionType [function_type valid] {
                    form: required (category FunctionTypeForm);
                }
                ContextFunctionType => ContextFunctionType [context_function_type valid] {
                    context_token: required (contextual "context");
                    open_paren: required (token LParen);
                    context_parameters: required (list FunctionTypeParameterSeparatedList);
                    close_paren: required (token RParen);
                    function_type: required (node FunctionType);
                }
                ReceiverType => ReceiverType [receiver_type valid] {
                    receiver: required (category TypeSyntax);
                    dot: required (token Dot);
                    parameter: required (category TypeSyntax);
                }
                ParenthesizedType => ParenthesizedType [parenthesized_type valid] {
                    annotations: required (list AnnotationList);
                    open_paren: required (token LParen);
                    entries: required (list ParenthesizedTypeEntryList);
                    close_paren: required (token RParen);
                }
                FunctionTypeParameter => FunctionTypeParameter [function_type_parameter valid] {
                    name: optional (node Name);
                    colon: optional (token Colon);
                    r#type: required (node TypeReference);
                }
                DefinitelyNonNullableType => DefinitelyNonNullableType [definitely_non_nullable_type valid] {
                    form: required (category DefinitelyNonNullableTypeForm);
                }
                TypeProjection => TypeProjection [type_projection valid] {
                    variance: required (choice [(token InKw), (contextual "out")]);
                    r#type: required (node TypeReference);
                }
                StarProjection => StarProjection [star_projection valid] {
                    star: required (token Star);
                }
                Block => Block [block valid] {
                    open_brace: required (token LBrace);
                    items: required (list BlockItemList);
                    close_brace: required (token RBrace);
                }
                Statement => Statement [statement valid] {
                    statement: required (choice [(category StatementSyntax), (category Expression), (category Declaration)]) => StatementContentValue;
                    tail: required (list TerminatorList);
                }
                ExpressionStatement => ExpressionStatement [expression_statement valid] {
                    expression: required (category Expression);
                }
                LocalDeclaration => LocalDeclaration [local_declaration valid] {
                    declaration: required (node PropertyDeclaration);
                }
                EmptyStatement => EmptyStatement [empty_statement valid] {
                    terminator: required (token_set [Semicolon, DoubleSemicolon]);
                }
                AssignmentExpression => AssignmentExpression [assignment_expression valid] {
                    left: required (category Expression);
                    operator: required (token_set [Assign, StarEq, SlashEq, PercentEq, PlusEq, MinusEq]);
                    right: required (category Expression);
                }
                BinaryExpression => BinaryExpression [binary_expression valid] {
                    left: required (category Expression);
                    operator: required (choice [
                        (token_set [
                            Star, Slash, Percent, Plus, Minus, Range, RangeUntil,
                            Lt, Gt, LtEq, GtEq, InKw, IsKw, NotIn, NotIs, EqEq,
                            BangEq, EqEqEq, BangEqEqEq, Amp, AndAnd, OrOr, Elvis,
                            AsKw, AsSafe
                        ]),
                        (token Identifier)
                    ]) => BinaryOperatorValue;
                    right: required (choice [(category Expression), (node TypeReference)]) => BinaryExpressionRightValue;
                }
                UnaryExpression => UnaryExpression [unary_expression valid] {
                    operator: required (token_set [Plus, Minus, Bang, PlusPlus, MinusMinus, Star]);
                    operand: required (category Expression);
                }
                PostfixExpression => PostfixExpression [postfix_expression valid] {
                    operand: required (category Expression);
                    operator: required (token_set [PlusPlus, MinusMinus, BangBang]);
                }
                CallExpression => CallExpression [call_expression valid] {
                    callee: required (category Expression);
                    type_arguments: required (list TypeArgumentListList);
                    arguments: optional (node ValueArgumentList);
                    lambdas: required (list LambdaExpressionList);
                }
                IndexExpression => IndexExpression [index_expression valid] {
                    receiver: required (category Expression);
                    open_bracket: required (token LBracket);
                    entries: required (list ValueArgumentEntryList);
                    close_bracket: required (token RBracket);
                }
                NavigationExpression => NavigationExpression [navigation_expression valid] {
                    receiver: required (category Expression);
                    operator: required (choice [(token_set [Dot, SafeAccess]), (node SplitSafeNavigationOperator)]) => NavigationOperatorValue;
                    selector: required (choice [
                        (token_set [Identifier, FieldIdentifier]),
                        (category NavigationSelector)
                    ]) => NavigationSelectorValue;
                }
                CallableReferenceExpression => CallableReferenceExpression [callable_reference_expression valid] {
                    receiver: optional (node CallableReferenceReceiver);
                    separator: required (token ColonColon);
                    target: required (node CallableReferenceTarget);
                    type_arguments: required (list TypeArgumentListList);
                }
                LiteralExpression => LiteralExpression [literal_expression valid] {
                    literal: required (token_set [IntegerLiteral, FloatLiteral, CharacterLiteral, NullKw, TrueKw, FalseKw]);
                }
                StringTemplateExpression => StringTemplateExpression [string_template_expression valid] {
                    parts: required (list StringTemplateEntryList);
                    close_quote: required (token ClosingQuote);
                }
                StringTemplateEntry => StringTemplateEntry [string_template_entry valid] {
                    content: required (choice [
                        (token_set [
                            InterpolationPrefix, OpenQuote, RegularStringPart,
                            EscapeSequence, ShortTemplateEntryStart,
                            LongTemplateEntryStart, LongTemplateEntryEnd,
                            DanglingNewline, Identifier, ThisKw
                        ]),
                        (category Expression),
                        (constructed LongStringTemplateEntry)
                    ]) => StringTemplateContentValue [disambiguate longest_then_first];
                }
                NameExpression => NameExpression [name_expression valid] {
                    name: required (token_set [Identifier, FieldIdentifier]);
                    at: optional (token At);
                    labeled_expression: optional (category Expression);
                }
                ThisExpression => ThisExpression [this_expression valid] {
                    this_token: required (token ThisKw);
                    label: optional (node LabelReference);
                }
                SuperExpression => SuperExpression [super_expression valid] {
                    super_token: required (token SuperKw);
                    type_arguments: optional (node TypeArgumentList);
                    label: optional (node LabelReference);
                }
                LabelReference => LabelReference [label_reference valid] {
                    at: required (token At);
                    label: required (token Identifier);
                }
                ParenthesizedExpression => ParenthesizedExpression [parenthesized_expression valid] {
                    open_paren: required (token LParen);
                    expression: required (category Expression);
                    close_paren: required (token RParen);
                }
                AnnotatedExpression => AnnotatedExpression [annotated_expression valid] {
                    prefix: required (list ModifierList);
                    expression: required (category Expression);
                }
                IfExpression => IfExpression [if_expression valid] {
                    if_token: required (token IfKw);
                    condition: required (node ParenthesizedExpression);
                    then_branch: required (choice [(category Expression), (node_set [Block, EmptyStatement])]) => IfThenBranchValue;
                    else_token: optional (token ElseKw);
                    else_branch: optional (choice [(category Expression), (node_set [Block, EmptyStatement])]) => IfElseBranchValue;
                }
                WhenExpression => WhenExpression [when_expression valid] {
                    when_token: required (token WhenKw);
                    subject: optional (node WhenSubject);
                    open_brace: required (token LBrace);
                    entries: required (list WhenEntryList);
                    close_brace: required (token RBrace);
                }
                WhenSubject => WhenSubject [when_subject valid] {
                    open_paren: required (token LParen);
                    val_token: optional (token_set [ValKw, VarKw]);
                    name: optional (node Name);
                    colon: optional (token Colon);
                    r#type: optional (node TypeReference);
                    assign: optional (token Assign);
                    expression: required (category Expression);
                    close_paren: required (token RParen);
                }
                WhenEntry => WhenEntry [when_entry valid] {
                    else_token: optional (token ElseKw);
                    conditions: required (list WhenConditionSeparatedList);
                    guard: optional (node WhenGuard);
                    arrow: required (token Arrow);
                    body: required (node WhenEntryBody);
                }
                WhenEntryBody => WhenEntryBody [when_entry_body valid] {
                    value: required (choice [(category Expression), (node Block)]) => WhenEntryBodyValue;
                }
                WhenCondition => WhenCondition [when_condition valid] {
                    keyword: optional (token_set [InKw, NotIn, IsKw, NotIs]);
                    value: required (choice [(node TypeReference), (category Expression)]) => WhenConditionValue;
                }
                WhenGuard => WhenGuard [when_guard valid] {
                    if_token: required (token IfKw);
                    expression: required (category Expression);
                }
                TryExpression => TryExpression [try_expression valid] {
                    try_token: required (token TryKw);
                    block: required (node Block);
                    clauses: required (list TryClauseList);
                }
                CatchClause => CatchClause [catch_clause valid] {
                    catch_token: required (contextual "catch");
                    parameter: required (node CatchParameter);
                    block: required (node Block);
                }
                CatchParameter => CatchParameter [catch_parameter valid] {
                    open_paren: required (token LParen);
                    modifiers: required (list ModifierList);
                    name: required (node Name);
                    colon: required (token Colon);
                    r#type: required (node TypeReference);
                    close_paren: required (token RParen);
                }
                FinallyClause => FinallyClause [finally_clause valid] {
                    finally_token: required (contextual "finally");
                    block: required (node Block);
                }
                ForStatement => ForStatement [for_statement valid] {
                    for_token: required (token ForKw);
                    open_paren: required (token LParen);
                    variable: required (node ForVariable);
                    in_token: required (token InKw);
                    iterable: required (category Expression);
                    close_paren: required (token RParen);
                    body: required (choice [(node_set [Block, EmptyStatement]), (category Expression)]) => ForBodyValue;
                }
                ForVariable => ForVariable [for_variable valid] {
                    modifiers: required (list ModifierList);
                    binding: required (node_set [Name, DestructuringDeclaration]) => ForVariableBindingValue;
                    colon: optional (token Colon);
                    r#type: optional (node TypeReference);
                }
                WhileStatement => WhileStatement [while_statement valid] {
                    while_token: required (token WhileKw);
                    condition: required (node ParenthesizedExpression);
                    body: required (choice [(node_set [Block, EmptyStatement]), (category Expression)]) => WhileBodyValue;
                }
                DoWhileStatement => DoWhileStatement [do_while_statement valid] {
                    do_token: required (token DoKw);
                    body: required (choice [(node_set [Block, EmptyStatement]), (category Expression)]) => DoWhileBodyValue;
                    while_token: required (token WhileKw);
                    condition: required (node ParenthesizedExpression);
                }
                JumpExpression => JumpExpression [jump_expression valid] {
                    keyword: required (token_set [ReturnKw, BreakKw, ContinueKw]);
                    label: optional (node LabelReference);
                    expression: optional (category Expression);
                }
                ThrowExpression => ThrowExpression [throw_expression valid] {
                    throw_token: required (token ThrowKw);
                    expression: required (category Expression);
                }
                LambdaExpression => LambdaExpression [lambda_expression valid] {
                    form: required (category LambdaForm);
                }
                LambdaParameterList => LambdaParameterList [lambda_parameter_list valid] {
                    parameters: required (list LambdaParameterSeparatedList);
                    arrow: required (token Arrow);
                }
                LambdaParameter => LambdaParameter [lambda_parameter valid] {
                    binding: required (node LambdaParameterBinding);
                    colon: optional (token Colon);
                    r#type: optional (node TypeReference);
                }
                AnonymousFunctionExpression => AnonymousFunctionExpression [anonymous_function_expression valid] {
                    fun_token: required (token FunKw);
                    receiver: optional (node TypeReference);
                    dot: optional (token Dot);
                    parameters: required (node ValueParameterList);
                    return_colon: optional (token Colon);
                    return_type: optional (node TypeReference);
                    body: required (category DeclarationBody);
                }
                ObjectExpression => ObjectExpression [object_expression valid] {
                    object_token: required (token ObjectKw);
                    delegation: optional (node DelegationClause);
                    body: required (node ClassBody);
                }
                CollectionLiteralExpression => CollectionLiteralExpression [collection_literal_expression valid] {
                    open_bracket: required (token LBracket);
                    entries: required (list ValueArgumentEntryList);
                    close_bracket: required (token RBracket);
                }
                DestructuringDeclaration => DestructuringDeclaration [destructuring_declaration valid] {
                    open_delimiter: required (token_set [LParen, LBracket]);
                    entries: required (list DestructuringEntrySeparatedList);
                    close_delimiter: required (token_set [RParen, RBracket]);
                }
                DestructuringEntry => DestructuringEntry [destructuring_entry valid] {
                    modifier: optional (token_set [ValKw, VarKw]);
                    name: required (node Name);
                    assign: optional (token Assign);
                    default: optional (category Expression);
                }
                ValueParameterList => ValueParameterList [value_parameter_list valid] {
                    open_paren: required (token LParen);
                    entries: required (list ValueParameterSeparatedList);
                    close_paren: required (token RParen);
                }
                ValueParameter => ValueParameter [value_parameter valid] {
                    modifiers: required (list ModifierList);
                    parameter_keyword: optional (token_set [ValKw, VarKw, VarargKw]);
                    name: required (category ValueParameterName);
                    colon: optional (token Colon);
                    r#type: optional (node TypeReference);
                    assign: optional (token Assign);
                    default: optional (category Expression);
                }
                TypeReference => TypeReference [type_reference valid] {
                    r#type: required (category TypeSyntax);
                }
                SuspendedFunctionType => SuspendedFunctionType [suspended_function_type valid] {
                    suspend_token: required (contextual "suspend");
                    function_type: required (node FunctionType);
                }
                ArrowFunctionType => ArrowFunctionType [arrow_function_type valid] {
                    parameter_type: required (category TypeSyntax);
                    arrow: required (token Arrow);
                    return_type: required (category TypeSyntax);
                }
                SplitSafeNavigationOperator => SplitSafeNavigationOperator [split_safe_navigation_operator valid] {
                    question: required (token Question);
                    dot: required (token Dot);
                }
                LongStringTemplateEntry => LongStringTemplateEntry [long_string_template_entry valid] {
                    open: required (token LongTemplateEntryStart);
                    expression: required (category Expression);
                    close: required (token LongTemplateEntryEnd);
                }
                LabeledLambdaExpression => LabeledLambdaExpression [labeled_lambda_expression valid] {
                    label: required (token Identifier);
                    at: required (token At);
                    lambda: required (node LambdaExpression);
                }
                LambdaBody => LambdaBody [lambda_body valid] {
                    open_brace: required (token LBrace);
                    parameters: optional (node LambdaParameterList);
                    items: required (list LambdaBodyItemList);
                    close_brace: required (token RBrace);
                }
                CallableReferenceTarget => CallableReferenceTarget [callable_reference_target valid] {
                    target: required (token_set [Identifier, ClassKw]);
                }
                CallableReferenceReceiver => CallableReferenceReceiver [callable_reference_receiver valid] {
                    receiver: required (choice [(category Expression), (node TypeReference)]) => CallableReferenceReceiverValue;
                }
                LambdaParameterBinding => LambdaParameterBinding [lambda_parameter_binding valid] {
                    binding: required (node_set [Name, DestructuringDeclaration]) => LambdaParameterBindingValue;
                }
                ValueArgumentPrefix => ValueArgumentPrefix [value_argument_prefix valid] {
                    prefix: required (choice [(token Star), (node Annotation)]) => ValueArgumentPrefixValue;
                }
                IntersectionDefinitelyNonNullableType => IntersectionDefinitelyNonNullableType [intersection_definitely_non_nullable_type valid] {
                    left: required (category TypeSyntax);
                    amp: required (token Amp);
                    right: required (category TypeSyntax);
                }
                BangDefinitelyNonNullableType => BangDefinitelyNonNullableType [bang_definitely_non_nullable_type valid] {
                    inner: required (category TypeSyntax);
                    bang_bang: required (token BangBang);
                }
                AnnotationList => AnnotationList [annotation_list list] {
                    annotations: many (node Annotation);
                }
                KotlinFileItemList => KotlinFileItemList [kotlin_file_item_list list] {
                    items: many (choice [(category KotlinFileItem), (token_set [EolOrSemicolon, Semicolon])]);
                }
                TerminatorList => TerminatorList [terminator_list list] {
                    terminators: many (token_set [EolOrSemicolon, Semicolon, DoubleSemicolon]);
                }
                ImportDirectiveList => ImportDirectiveList [import_directive_list list] {
                    directives: one_or_more (node ImportDirective);
                }
                ModifierList => ModifierList [modifier_list list] {
                    modifiers: many (choice [
                        (node Annotation),
                        (contextual "abstract"), (contextual "enum"),
                        (contextual "contract"), (contextual "open"),
                        (contextual "inner"), (contextual "override"),
                        (contextual "private"), (contextual "public"),
                        (contextual "internal"), (contextual "protected"),
                        (contextual "out"), (contextual "vararg"),
                        (contextual "reified"), (contextual "companion"),
                        (contextual "sealed"), (contextual "final"),
                        (contextual "lateinit"), (contextual "data"),
                        (contextual "value"), (contextual "inline"),
                        (contextual "noinline"), (contextual "tailrec"),
                        (contextual "external"), (contextual "annotation"),
                        (contextual "crossinline"), (contextual "operator"),
                        (contextual "infix"), (contextual "const"),
                        (contextual "suspend"), (contextual "expect"),
                        (contextual "actual")
                    ]);
                }
                ValueArgumentSeparatedList => ValueArgumentSeparatedList [value_argument_separated_list list] {
                    entries: many (category ValueArgumentListEntry) [separated (token Comma), minimum 0, trailing optional, recovery bogus_owner];
                }
                ValueArgumentEntryList => ValueArgumentEntryList [value_argument_entry_list list] {
                    entries: many (category ValueArgumentListEntry) [separated (token Comma), minimum 0, trailing optional, recovery bogus_owner];
                }
                ValueArgumentPrefixList => ValueArgumentPrefixList [value_argument_prefix_list list] {
                    entries: many (node ValueArgumentPrefix);
                }
                QualifiedNameSegmentList => QualifiedNameSegmentList [qualified_name_segment_list list] {
                    segments: one_or_more (category QualifiedNameSegment) [separated (token Dot), minimum 1, trailing forbidden, recovery bogus_owner];
                }
                ClassMemberList => ClassMemberList [class_member_list list] {
                    members: many (choice [(category ClassMember), (token_set [EolOrSemicolon, Semicolon, DoubleSemicolon])]);
                }
                PropertyBodyMemberList => PropertyBodyMemberList [property_body_member_list list] {
                    members: many (choice [(category PropertyBodyMember), (token_set [EolOrSemicolon, Semicolon])]);
                }
                TypeParameterSeparatedList => TypeParameterSeparatedList [type_parameter_separated_list list] {
                    entries: one_or_more (category TypeParameterListEntry) [separated (token Comma), minimum 1, trailing optional, recovery bogus_owner];
                }
                TypeConstraintSeparatedList => TypeConstraintSeparatedList [type_constraint_separated_list list] {
                    entries: one_or_more (category TypeConstraintListEntry) [separated (token Comma), minimum 1, trailing forbidden, recovery bogus_owner];
                }
                ContextParameterSeparatedList => ContextParameterSeparatedList [context_parameter_separated_list list] {
                    entries: one_or_more (category ContextParameterListEntry) [separated (token Comma), minimum 1, trailing optional, recovery bogus_owner];
                }
                DelegationSpecifierSeparatedList => DelegationSpecifierSeparatedList [delegation_specifier_separated_list list] {
                    entries: one_or_more (category DelegationSpecifierEntry) [separated (token Comma), minimum 1, trailing optional, recovery bogus_owner];
                }
                UserTypeSegmentList => UserTypeSegmentList [user_type_segment_list list] {
                    segments: one_or_more (category UserTypeSegmentSyntax) [separated (token_set [Dot, Range]), minimum 1, trailing forbidden, recovery bogus_owner];
                }
                FunctionTypeParameterSeparatedList => FunctionTypeParameterSeparatedList [function_type_parameter_separated_list list] {
                    entries: one_or_more (category FunctionTypeParameterListEntry) [separated (token Comma), minimum 1, trailing optional, recovery bogus_owner];
                }
                ParenthesizedTypeEntryList => ParenthesizedTypeEntryList [parenthesized_type_entry_list list] {
                    entries: many (category FunctionTypeParameterListEntry) [separated (token Comma), minimum 0, trailing optional, recovery bogus_owner];
                }
                TypeProjectionSeparatedList => TypeProjectionSeparatedList [type_projection_separated_list list] {
                    entries: one_or_more (category TypeArgumentListEntry) [separated (token Comma), minimum 1, trailing optional, recovery bogus_owner];
                }
                BlockItemList => BlockItemList [block_item_list list] {
                    items: many (choice [(category BlockItem), (token_set [EolOrSemicolon, Semicolon])]) => BlockItemListElement;
                }
                TypeArgumentListList => TypeArgumentListList [type_argument_list_list list] {
                    lists: many (node TypeArgumentList);
                }
                LambdaExpressionList => LambdaExpressionList [lambda_expression_list list] {
                    lambdas: many (node LambdaExpression);
                }
                StringTemplateEntryList => StringTemplateEntryList [string_template_entry_list list] {
                    entries: one_or_more (category StringTemplatePart);
                }
                WhenEntryList => WhenEntryList [when_entry_list list] {
                    entries: many (choice [(node WhenEntry), (token_set [EolOrSemicolon, Semicolon, DoubleSemicolon])]) => WhenEntryListElement;
                }
                WhenConditionSeparatedList => WhenConditionSeparatedList [when_condition_separated_list list] {
                    conditions: many (category WhenConditionSyntax) [separated (token Comma), minimum 0, trailing optional, recovery bogus_owner];
                }
                TryClauseList => TryClauseList [try_clause_list list] {
                    clauses: many (category TryClause);
                }
                LambdaBodyItemList => LambdaBodyItemList [lambda_body_item_list list] {
                    items: many (choice [(category BlockItem), (token_set [EolOrSemicolon, Semicolon, DoubleSemicolon])]) => LambdaBodyItem;
                }
                LambdaParameterSeparatedList => LambdaParameterSeparatedList [lambda_parameter_separated_list list] {
                    parameters: many (category LambdaParameterListEntry) [separated (token Comma), minimum 0, trailing optional, recovery bogus_owner];
                }
                DestructuringEntrySeparatedList => DestructuringEntrySeparatedList [destructuring_entry_separated_list list] {
                    entries: many (category DestructuringPatternEntry) [separated (token Comma), minimum 0, trailing optional, recovery bogus_owner];
                }
                ValueParameterSeparatedList => ValueParameterSeparatedList [value_parameter_separated_list list] {
                    entries: many (category ValueParameterListEntry) [separated (token Comma), minimum 0, trailing optional, recovery bogus_owner];
                }
            }
        }
    };
}
