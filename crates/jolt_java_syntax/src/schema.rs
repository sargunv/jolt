// The single declarative authority for syntax kinds, typed CST wrappers, and tree shape.
// The recursive-descent parser remains handwritten.
macro_rules! java_syntax_schema {
    ($emit:ident) => {
        $emit! {
            tokens {
                Eof,
                Unknown,
                Identifier,
                IntegerLiteral,
                FloatingPointLiteral,
                BooleanLiteral,
                CharacterLiteral,
                StringLiteral,
                TextBlockLiteral,
                NullLiteral,
                AbstractKw,
                AssertKw,
                BooleanKw,
                BreakKw,
                ByteKw,
                CaseKw,
                CatchKw,
                CharKw,
                ClassKw,
                ConstKw,
                ContinueKw,
                DefaultKw,
                DoKw,
                DoubleKw,
                ElseKw,
                EnumKw,
                ExtendsKw,
                FinalKw,
                FinallyKw,
                FloatKw,
                ForKw,
                GotoKw,
                IfKw,
                ImplementsKw,
                ImportKw,
                InstanceofKw,
                IntKw,
                InterfaceKw,
                LongKw,
                NativeKw,
                NewKw,
                PackageKw,
                PrivateKw,
                ProtectedKw,
                PublicKw,
                ReturnKw,
                ShortKw,
                StaticKw,
                StrictfpKw,
                SuperKw,
                SwitchKw,
                SynchronizedKw,
                ThisKw,
                ThrowKw,
                ThrowsKw,
                TransientKw,
                TryKw,
                VoidKw,
                VolatileKw,
                WhileKw,
                UnderscoreKw,
                LParen,
                RParen,
                LBrace,
                RBrace,
                LBracket,
                RBracket,
                Semicolon,
                Comma,
                Dot,
                Ellipsis,
                At,
                Colon,
                DoubleColon,
                Assign,
                Gt,
                GtEq,
                Lt,
                Bang,
                Tilde,
                Question,
                Arrow,
                EqEq,
                LtEq,
                BangEq,
                AndAnd,
                OrOr,
                PlusPlus,
                MinusMinus,
                Plus,
                Minus,
                Star,
                Slash,
                Amp,
                Bar,
                Caret,
                Percent,
                LShift,
                RShift,
                UnsignedRShift,
                PlusEq,
                MinusEq,
                StarEq,
                SlashEq,
                AmpEq,
                BarEq,
                CaretEq,
                PercentEq,
                LShiftEq,
                RShiftEq,
                UnsignedRShiftEq,
            }
            categories {
                CompilationUnitItem => BogusCompilationUnitItem {
                    PackageDeclaration,
                    ImportDeclaration,
                    ModuleDeclaration,
                    ClassDeclaration,
                    RecordDeclaration,
                    EnumDeclaration,
                    InterfaceDeclaration,
                    AnnotationInterfaceDeclaration,
                    FieldDeclaration,
                    MethodDeclaration,
                    EmptyDeclaration,
                }
                TypeDeclaration => BogusTypeDeclaration {
                    ClassDeclaration,
                    RecordDeclaration,
                    EnumDeclaration,
                    InterfaceDeclaration,
                    AnnotationInterfaceDeclaration,
                }
                Statement => BogusStatement {
                    Block,
                    EmptyStatement,
                    LabeledStatement,
                    ExpressionStatement,
                    IfStatement,
                    AssertStatement,
                    SwitchStatement,
                    WhileStatement,
                    DoStatement,
                    ForStatement,
                    BreakStatement,
                    YieldStatement,
                    ContinueStatement,
                    ReturnStatement,
                    ThrowStatement,
                    SynchronizedStatement,
                    TryStatement,
                    TryWithResourcesStatement,
                }
                EnhancedForVariableSyntax => BogusEnhancedForVariable {
                    EnhancedForVariable,
                }
                ResourceValueSyntax => BogusResourceValue {
                    ResourceVariableDeclaration,
                    VariableAccess,
                }
                SwitchEntrySyntax => BogusSwitchEntry {
                    SwitchBlockStatementGroup,
                    SwitchRule,
                }
                SwitchGuardSyntax => BogusSwitchGuard {
                    Guard,
                }
                Expression => BogusExpression {
                    LiteralExpression,
                    TemplateExpression,
                    NameExpression,
                    ThisExpression,
                    SuperExpression,
                    ParenthesizedExpression,
                    ClassLiteralExpression,
                    FieldAccessExpression,
                    ArrayAccessExpression,
                    MethodInvocationExpression,
                    MethodReferenceExpression,
                    ObjectCreationExpression,
                    ArrayCreationExpression,
                    AssignmentExpression,
                    ConditionalExpression,
                    InstanceofExpression,
                    BinaryExpression,
                    UnaryExpression,
                    PostfixExpression,
                    CastExpression,
                    LambdaExpression,
                    SwitchExpression,
                }
                AssignmentTargetSyntax => BogusAssignmentTarget {
                    NameExpression,
                    FieldAccessExpression,
                    ArrayAccessExpression,
                    BogusExpression,
                }
                MethodInvocationFormSyntax => BogusMethodInvocationForm {
                    QualifiedMethodInvocation,
                    UnqualifiedMethodInvocation,
                }
                ClassLiteralTargetSyntax => BogusClassLiteralTarget {
                    PrimitiveType,
                    VoidType,
                    NameExpression,
                    FieldAccessExpression,
                }
                MethodReferenceReceiverSyntax => BogusMethodReferenceReceiver {
                    LiteralExpression,
                    TemplateExpression,
                    NameExpression,
                    ThisExpression,
                    SuperExpression,
                    ParenthesizedExpression,
                    ClassLiteralExpression,
                    FieldAccessExpression,
                    ArrayAccessExpression,
                    MethodInvocationExpression,
                    ObjectCreationExpression,
                    ArrayCreationExpression,
                    SwitchExpression,
                    ClassType,
                    ArrayType,
                }
                LambdaBodySyntax => BogusLambdaBody {
                    LiteralExpression,
                    TemplateExpression,
                    NameExpression,
                    ThisExpression,
                    SuperExpression,
                    ParenthesizedExpression,
                    ClassLiteralExpression,
                    FieldAccessExpression,
                    ArrayAccessExpression,
                    MethodInvocationExpression,
                    MethodReferenceExpression,
                    ObjectCreationExpression,
                    ArrayCreationExpression,
                    AssignmentExpression,
                    ConditionalExpression,
                    InstanceofExpression,
                    BinaryExpression,
                    UnaryExpression,
                    PostfixExpression,
                    CastExpression,
                    LambdaExpression,
                    SwitchExpression,
                    BogusExpression,
                    Block,
                }
                ArrayCreationTypeSyntax => BogusArrayCreationType {
                    PrimitiveType,
                    ClassType,
                    ArrayType,
                }
                ObjectCreationTypeSyntax => BogusObjectCreationType {
                    ClassType,
                }
                InstanceofTargetSyntax => BogusInstanceofTarget {
                    ClassType,
                    ArrayType,
                    TypePattern,
                    RecordPattern,
                    BogusPattern,
                    BogusType,
                }
                Type => BogusType {
                    PrimitiveType,
                    VoidType,
                    ClassType,
                    ArrayType,
                    IntersectionType,
                    UnionType,
                    WildcardType,
                }
                Pattern => BogusPattern {
                    TypePattern,
                    RecordPattern,
                    MatchAllPattern,
                }
                NameSyntax => BogusName {
                    Name,
                    QualifiedName,
                }
                FormalParameterSyntax => BogusFormalParameter {
                    FormalParameter,
                    ReceiverParameter,
                }
                AnnotationArgumentSyntax => BogusAnnotationArgument {
                    AnnotationElementValue,
                    AnnotationElementValuePair,
                }
                ModuleDirective => BogusModuleDirective {
                    RequiresDirective,
                    ExportsDirective,
                    OpensDirective,
                    UsesDirective,
                    ProvidesDirective,
                }
                BlockItem => BogusBlockItem {
                    BogusStatement,
                    LocalVariableDeclaration,
                    LocalClassOrInterfaceDeclaration,
                    Block,
                    EmptyStatement,
                    LabeledStatement,
                    ExpressionStatement,
                    IfStatement,
                    AssertStatement,
                    SwitchStatement,
                    WhileStatement,
                    DoStatement,
                    ForStatement,
                    BreakStatement,
                    YieldStatement,
                    ContinueStatement,
                    ReturnStatement,
                    ThrowStatement,
                    SynchronizedStatement,
                    TryStatement,
                    TryWithResourcesStatement,
                }
                ClassBodyMember => BogusClassBodyMember {
                    EmptyDeclaration,
                    ClassDeclaration,
                    RecordDeclaration,
                    EnumDeclaration,
                    InterfaceDeclaration,
                    AnnotationInterfaceDeclaration,
                    FieldDeclaration,
                    MethodDeclaration,
                    ConstructorDeclaration,
                    CompactConstructorDeclaration,
                    StaticInitializer,
                    InstanceInitializer,
                }
                InterfaceBodyMember => BogusInterfaceBodyMember {
                    EmptyDeclaration,
                    ClassDeclaration,
                    RecordDeclaration,
                    EnumDeclaration,
                    InterfaceDeclaration,
                    AnnotationInterfaceDeclaration,
                    FieldDeclaration,
                    MethodDeclaration,
                }
                AnnotationInterfaceBodyMember => BogusAnnotationInterfaceBodyMember {
                    EmptyDeclaration,
                    ClassDeclaration,
                    RecordDeclaration,
                    EnumDeclaration,
                    InterfaceDeclaration,
                    AnnotationInterfaceDeclaration,
                    FieldDeclaration,
                    MethodDeclaration,
                    AnnotationElementDeclaration,
                }
                ConstructorBodyEntry => BogusConstructorBodyEntry {
                    ConstructorInvocation,
                    BlockStatement,
                }
                VariableInitializerValue => BogusVariableInitializer {
                    BogusExpression,
                    LiteralExpression,
                    TemplateExpression,
                    NameExpression,
                    ThisExpression,
                    SuperExpression,
                    ParenthesizedExpression,
                    ClassLiteralExpression,
                    FieldAccessExpression,
                    ArrayAccessExpression,
                    MethodInvocationExpression,
                    MethodReferenceExpression,
                    ObjectCreationExpression,
                    ArrayCreationExpression,
                    AssignmentExpression,
                    ConditionalExpression,
                    InstanceofExpression,
                    BinaryExpression,
                    UnaryExpression,
                    PostfixExpression,
                    CastExpression,
                    LambdaExpression,
                    SwitchExpression,
                    ArrayInitializer,
                }
            }
            nodes {
                BogusSwitchLabelItem => BogusSwitchLabelItem [bogus_switch_label_item malformed] {
                    elements: many (any_element) => BogusSwitchLabelItemElement;
                }
                BogusModifier => BogusModifier [bogus_modifier malformed] {
                    elements: many (any_element) => BogusModifierElement;
                }
                CompilationUnit => CompilationUnit [compilation_unit valid] {
                    items: required (list CompilationUnitItemList);
                    eof: required (token Eof);
                }
                PackageDeclaration => PackageDeclaration [package_declaration valid] {
                    annotations: required (list AnnotationList);
                    package_keyword: required (token PackageKw);
                    name: required (category NameSyntax);
                    semicolon: required (token Semicolon);
                }
                ImportDeclaration => ImportDeclaration [import_declaration valid] {
                    import_keyword: required (token ImportKw);
                    module_keyword: optional (contextual "module");
                    static_keyword: optional (token StaticKw);
                    name: required (category NameSyntax);
                    on_demand_dot: optional (token Dot);
                    star: optional (token Star);
                    suffix: optional (node BogusImportSuffix);
                    semicolon: required (token Semicolon);
                }
                ModuleDeclaration => ModuleDeclaration [module_declaration valid] {
                    annotations: required (list AnnotationList);
                    open_keyword: optional (contextual "open");
                    module_keyword: required (contextual "module");
                    name: required (category NameSyntax);
                    open_brace: required (token LBrace);
                    directives: required (list ModuleDirectiveList);
                    close_brace: required (token RBrace);
                }
                BogusImportSuffix => BogusImportSuffix [bogus_import_suffix malformed] {
                    elements: many (any_element) => BogusImportSuffixElement;
                }
                RequiresDirective => RequiresDirective [requires_directive valid] {
                    requires_keyword: required (contextual "requires");
                    modifiers: required (list RequiresModifierList);
                    module: required (category NameSyntax);
                    semicolon: required (token Semicolon);
                }
                ExportsDirective => ExportsDirective [exports_directive valid] {
                    exports_keyword: required (contextual "exports");
                    package: required (category NameSyntax);
                    targets: optional (node ModuleTargetClause);
                    semicolon: required (token Semicolon);
                }
                OpensDirective => OpensDirective [opens_directive valid] {
                    opens_keyword: required (contextual "opens");
                    package: required (category NameSyntax);
                    targets: optional (node ModuleTargetClause);
                    semicolon: required (token Semicolon);
                }
                UsesDirective => UsesDirective [uses_directive valid] {
                    uses_keyword: required (contextual "uses");
                    service: required (category NameSyntax);
                    semicolon: required (token Semicolon);
                }
                ProvidesDirective => ProvidesDirective [provides_directive valid] {
                    provides_keyword: required (contextual "provides");
                    service: required (category NameSyntax);
                    implementation: required (node ModuleImplementationClause);
                    semicolon: required (token Semicolon);
                }
                ModuleTargetClause => ModuleTargetClause [module_target_clause valid] {
                    to_keyword: required (contextual "to");
                    targets: required (list ModuleNameList);
                }
                ModuleImplementationClause => ModuleImplementationClause [module_implementation_clause valid] {
                    with_keyword: required (contextual "with");
                    implementations: required (list ModuleNameList);
                }
                ModifierList => ModifierList [modifier_list list] {
                    modifiers: one_or_more (choice [
                        (node Annotation),
                        (node BogusModifier),
                        (token_set [
                            AbstractKw, FinalKw, NativeKw, PrivateKw, ProtectedKw,
                            PublicKw, StaticKw, StrictfpKw, SynchronizedKw,
                            TransientKw, VolatileKw
                        ]),
                        (token DefaultKw),
                        (contextual "sealed"),
                        (constructed NonSealedModifier)
                    ]) => ModifierElement;
                }
                Annotation => Annotation [annotation valid] {
                    at: required (token At);
                    name: required (category NameSyntax);
                    arguments: optional (node AnnotationArgumentList);
                }
                AnnotationArgumentList => AnnotationArgumentList [annotation_argument_list valid] {
                    open_paren: required (token LParen);
                    elements: optional (node AnnotationElementList);
                    close_paren: required (token RParen);
                }
                AnnotationElementDeclaration => AnnotationElementDeclaration [annotation_element_declaration valid] {
                    modifiers: optional (list ModifierList);
                    r#type: required (category Type);
                    name: required (token Identifier);
                    open_paren: required (token LParen);
                    close_paren: required (token RParen);
                    dimensions: optional (list ArrayDimensions);
                    default: optional (node DefaultValue);
                    semicolon: required (token Semicolon);
                }
                AnnotationElementValue => AnnotationElementValue [annotation_element_value valid] {
                    value: required (choice [(category Expression), (node_set [Annotation, AnnotationArrayInitializer])]) => AnnotationElementValueContent;
                }
                AnnotationElementValuePair => AnnotationElementValuePair [annotation_element_value_pair valid] {
                    name: required (token Identifier);
                    assign: required (token Assign);
                    value: required (node AnnotationElementValue);
                }
                AnnotationElementList => AnnotationElementList [annotation_element_list valid] {
                    arguments: optional (list AnnotationElementArgumentList);
                    declarations: optional (list AnnotationInterfaceBodyMemberList);
                }
                AnnotationArrayInitializer => AnnotationArrayInitializer [annotation_array_initializer valid] {
                    open_brace: required (token LBrace);
                    values: required (list AnnotationElementValueList);
                    close_brace: required (token RBrace);
                }
                DefaultValue => DefaultValue [default_value valid] {
                    default_keyword: required (token DefaultKw);
                    value: required (node AnnotationElementValue);
                }
                ClassDeclaration => ClassDeclaration [class_declaration valid] {
                    modifiers: optional (list ModifierList);
                    class_keyword: required (token ClassKw);
                    name: required (token Identifier);
                    type_parameters: optional (node TypeParameterList);
                    extends: optional (node ExtendsClause);
                    implements: optional (node ImplementsClause);
                    permits: optional (node PermitsClause);
                    body: required (node ClassBody);
                    missing_body_semicolon: optional (token Semicolon);
                }
                RecordDeclaration => RecordDeclaration [record_declaration valid] {
                    modifiers: optional (list ModifierList);
                    record_keyword: required (contextual "record");
                    name: required (token Identifier);
                    type_parameters: optional (node TypeParameterList);
                    open_paren: required (token LParen);
                    components: optional (list RecordComponentList);
                    close_paren: required (token RParen);
                    implements: optional (node ImplementsClause);
                    body: required (node RecordBody);
                    missing_body_semicolon: optional (token Semicolon);
                }
                EnumDeclaration => EnumDeclaration [enum_declaration valid] {
                    modifiers: optional (list ModifierList);
                    enum_keyword: required (token EnumKw);
                    name: required (token Identifier);
                    implements: optional (node ImplementsClause);
                    body: required (node EnumBody);
                    missing_body_semicolon: optional (token Semicolon);
                }
                InterfaceDeclaration => InterfaceDeclaration [interface_declaration valid] {
                    modifiers: optional (list ModifierList);
                    interface_keyword: required (token InterfaceKw);
                    name: required (token Identifier);
                    type_parameters: optional (node TypeParameterList);
                    extends: optional (node ExtendsClause);
                    permits: optional (node PermitsClause);
                    body: required (node InterfaceBody);
                    missing_body_semicolon: optional (token Semicolon);
                }
                AnnotationInterfaceDeclaration => AnnotationInterfaceDeclaration [annotation_interface_declaration valid] {
                    modifiers: optional (list ModifierList);
                    at: required (token At);
                    interface_keyword: required (token InterfaceKw);
                    name: required (token Identifier);
                    body: required (node AnnotationInterfaceBody);
                    missing_body_semicolon: optional (token Semicolon);
                }
                TypeParameterList => TypeParameterList [type_parameter_list valid] {
                    open_angle: required (token Lt);
                    parameters: required (list TypeParameterSeparatedList);
                    close_angle: required (token Gt);
                }
                TypeParameter => TypeParameter [type_parameter valid] {
                    annotations: required (list AnnotationList);
                    name: required (token Identifier);
                    bounds: optional (node TypeBoundList);
                }
                TypeBoundList => TypeBoundList [type_bound_list valid] {
                    extends_keyword: required (token ExtendsKw);
                    bounds: required (node_set [ClassType, IntersectionType, BogusType]) => TypeBound;
                }
                ExtendsClause => ExtendsClause [extends_clause valid] {
                    extends_keyword: required (token ExtendsKw);
                    types: required (list TypeList);
                }
                ImplementsClause => ImplementsClause [implements_clause valid] {
                    implements_keyword: required (token ImplementsKw);
                    types: required (list TypeList);
                }
                PermitsClause => PermitsClause [permits_clause valid] {
                    permits_keyword: required (contextual "permits");
                    names: required (list NameList);
                }
                ClassBody => ClassBody [class_body valid] {
                    open_brace: required (token LBrace);
                    members: required (list ClassBodyMemberList);
                    close_brace: required (token RBrace);
                }
                EmptyDeclaration => EmptyDeclaration [empty_declaration valid] {
                    semicolon: required (token Semicolon);
                }
                RecordBody => RecordBody [record_body valid] {
                    open_brace: required (token LBrace);
                    members: required (list RecordBodyMemberList);
                    close_brace: required (token RBrace);
                }
                InterfaceBody => InterfaceBody [interface_body valid] {
                    open_brace: required (token LBrace);
                    members: required (list InterfaceBodyMemberList);
                    close_brace: required (token RBrace);
                }
                AnnotationInterfaceBody => AnnotationInterfaceBody [annotation_interface_body valid] {
                    open_brace: required (token LBrace);
                    elements: optional (node AnnotationElementList);
                    close_brace: required (token RBrace);
                }
                EnumBody => EnumBody [enum_body valid] {
                    open_brace: required (token LBrace);
                    constants: optional (list EnumConstantList);
                    body_separator: optional (token Semicolon);
                    members: required (list ClassBodyMemberList);
                    close_brace: required (token RBrace);
                }
                EnumConstantList => EnumConstantList [enum_constant_list list] {
                    constants: one_or_more (node EnumConstant) [separated (token Comma), minimum 1, trailing optional, recovery bogus_owner];
                }
                EnumConstant => EnumConstant [enum_constant valid] {
                    annotations: required (list AnnotationList);
                    name: required (token Identifier);
                    arguments: optional (node ArgumentList);
                    body: optional (node ClassBody);
                }
                RecordComponentList => RecordComponentList [record_component_list list] {
                    components: one_or_more (node RecordComponent) [separated (token Comma), minimum 1, trailing forbidden, recovery bogus_owner];
                }
                RecordComponent => RecordComponent [record_component valid] {
                    modifiers: required (list ParameterModifierList);
                    r#type: required (category Type);
                    varargs_annotations: required (list AnnotationList);
                    ellipsis: optional (token Ellipsis);
                    name: required (token_set [Identifier, UnderscoreKw]);
                }
                FieldDeclaration => FieldDeclaration [field_declaration valid] {
                    modifiers: optional (list ModifierList);
                    r#type: required (category Type);
                    declarators: required (list VariableDeclaratorList);
                    semicolon: required (token Semicolon);
                }
                MethodDeclaration => MethodDeclaration [method_declaration valid] {
                    modifiers: optional (list ModifierList);
                    type_parameters: optional (node TypeParameterList);
                    return_annotations: optional (list AnnotationList);
                    return_type: required (category Type);
                    name: required (token Identifier);
                    open_paren: required (token LParen);
                    parameters: optional (list FormalParameterList);
                    close_paren: required (token RParen);
                    dimensions: optional (list ArrayDimensions);
                    throws: optional (node ThrowsClause);
                    body: required (choice [(node Block), (token Semicolon)]) => MethodBody;
                }
                ConstructorDeclaration => ConstructorDeclaration [constructor_declaration valid] {
                    modifiers: optional (list ModifierList);
                    type_parameters: optional (node TypeParameterList);
                    name: required (token Identifier);
                    open_paren: required (token LParen);
                    parameters: optional (list FormalParameterList);
                    close_paren: required (token RParen);
                    throws: optional (node ThrowsClause);
                    body: required (node ConstructorBody);
                }
                ConstructorBody => ConstructorBody [constructor_body valid] {
                    open_brace: required (token LBrace);
                    entries: required (list ConstructorBodyEntryList);
                    close_brace: required (token RBrace);
                }
                ConstructorInvocation => ConstructorInvocation [constructor_invocation valid] {
                    qualifier: optional (choice [(category Expression), (node_set [Name, QualifiedName])]) => ConstructorQualifier;
                    dot: optional (token Dot);
                    type_arguments: optional (node TypeArgumentList);
                    target: required (token_set [ThisKw, SuperKw]);
                    arguments: required (node ArgumentList);
                    semicolon: required (token Semicolon);
                }
                CompactConstructorDeclaration => CompactConstructorDeclaration [compact_constructor_declaration valid] {
                    modifiers: optional (list ModifierList);
                    name: required (token Identifier);
                    body: required (node ConstructorBody);
                }
                StaticInitializer => StaticInitializer [static_initializer valid] {
                    static_keyword: required (token StaticKw);
                    body: required (node Block);
                }
                InstanceInitializer => InstanceInitializer [instance_initializer valid] {
                    body: required (node Block);
                }
                FormalParameterList => FormalParameterList [formal_parameter_list list] {
                    parameters: one_or_more (category FormalParameterSyntax) [separated (token Comma), minimum 1, trailing forbidden, recovery bogus_owner];
                }
                FormalParameter => FormalParameter [formal_parameter valid] {
                    modifiers: required (list ParameterModifierList);
                    r#type: required (category Type);
                    varargs_annotations: required (list AnnotationList);
                    ellipsis: optional (token Ellipsis);
                    name: required (token_set [Identifier, UnderscoreKw]);
                    dimensions: optional (list ArrayDimensions);
                }
                ReceiverParameter => ReceiverParameter [receiver_parameter valid] {
                    annotations: required (list AnnotationList);
                    r#type: required (category Type);
                    qualifier: optional (token Identifier);
                    dot: optional (token Dot);
                    this_keyword: required (token ThisKw);
                }
                ThrowsClause => ThrowsClause [throws_clause valid] {
                    throws_keyword: required (token ThrowsKw);
                    exceptions: required (list TypeList);
                }
                VariableDeclaratorList => VariableDeclaratorList [variable_declarator_list list] {
                    declarators: one_or_more (node VariableDeclarator) [separated (token Comma), minimum 1, trailing forbidden, recovery bogus_owner];
                }
                VariableDeclarator => VariableDeclarator [variable_declarator valid] {
                    name: required (token_set [Identifier, UnderscoreKw]);
                    dimensions: optional (list ArrayDimensions);
                    assign: optional (token Assign);
                    initializer: optional (node VariableInitializer);
                }
                VariableInitializer => VariableInitializer [variable_initializer valid] {
                    value: required (category VariableInitializerValue);
                }
                Block => Block [block valid] {
                    open_brace: required (token LBrace);
                    statements: required (list BlockStatementList);
                    close_brace: required (token RBrace);
                }
                BlockStatement => BlockStatement [block_statement valid] {
                    item: required (category BlockItem);
                    local_declaration_semicolon: optional (token Semicolon);
                }
                LocalVariableDeclaration => LocalVariableDeclaration [local_variable_declaration valid] {
                    modifiers: required (list ParameterModifierList);
                    r#type: required (choice [(category Type), (contextual "var")]) => LocalVariableType;
                    declarators: required (list VariableDeclaratorList);
                }
                LocalClassOrInterfaceDeclaration => LocalClassOrInterfaceDeclaration [local_class_or_interface_declaration valid] {
                    declaration: required (node_set [
                        ClassDeclaration, RecordDeclaration, EnumDeclaration,
                        InterfaceDeclaration, AnnotationInterfaceDeclaration,
                        BogusTypeDeclaration
                    ]) => LocalTypeDeclaration;
                }
                EmptyStatement => EmptyStatement [empty_statement valid] {
                    semicolon: required (token Semicolon);
                }
                LabeledStatement => LabeledStatement [labeled_statement valid] {
                    label: required (token Identifier);
                    colon: required (token Colon);
                    body: required (category Statement);
                }
                ExpressionStatement => ExpressionStatement [expression_statement valid] {
                    expression: required (category Expression);
                    semicolon: required (token Semicolon);
                }
                IfStatement => IfStatement [if_statement valid] {
                    if_keyword: required (token IfKw);
                    open_paren: required (token LParen);
                    condition: required (category Expression);
                    close_paren: required (token RParen);
                    then_branch: required (category Statement);
                    else_keyword: optional (token ElseKw);
                    else_branch: optional (category Statement);
                }
                AssertStatement => AssertStatement [assert_statement valid] {
                    assert_keyword: required (token AssertKw);
                    condition: required (category Expression);
                    colon: optional (token Colon);
                    message: optional (category Expression);
                    semicolon: required (token Semicolon);
                }
                SwitchStatement => SwitchStatement [switch_statement valid] {
                    switch_keyword: required (token SwitchKw);
                    open_paren: required (token LParen);
                    selector: required (category Expression);
                    close_paren: required (token RParen);
                    body: required (node SwitchBlock);
                }
                SwitchBlock => SwitchBlock [switch_block valid] {
                    open_brace: required (token LBrace);
                    entries: required (list SwitchEntryList);
                    close_brace: required (token RBrace);
                }
                SwitchBlockStatementGroup => SwitchBlockStatementGroup [switch_block_statement_group valid] {
                    labels: required (list SwitchLabelColonList);
                    statements: required (list BlockStatementList);
                }
                SwitchRule => SwitchRule [switch_rule valid] {
                    label: required (node SwitchLabel);
                    arrow: required (token Arrow);
                    body: required (choice [(category Expression), (node_set [Block, ThrowStatement])]) => SwitchRuleBody;
                    semicolon: optional (token Semicolon);
                }
                SwitchLabel => SwitchLabel [switch_label valid] {
                    keyword: required (token_set [CaseKw, DefaultKw]);
                    items: required (list SwitchLabelItemList);
                    guard: optional (category SwitchGuardSyntax);
                }
                CaseConstant => CaseConstant [case_constant valid] {
                    expression: required (category Expression);
                }
                CasePattern => CasePattern [case_pattern valid] {
                    pattern: required (category Pattern);
                }
                Guard => Guard [guard valid] {
                    when_keyword: required (contextual "when");
                    open_paren: optional (token LParen);
                    condition: required (category Expression);
                    close_paren: optional (token RParen);
                }
                WhileStatement => WhileStatement [while_statement valid] {
                    while_keyword: required (token WhileKw);
                    open_paren: required (token LParen);
                    condition: required (category Expression);
                    close_paren: required (token RParen);
                    body: required (category Statement);
                }
                DoStatement => DoStatement [do_statement valid] {
                    do_keyword: required (token DoKw);
                    body: required (category Statement);
                    while_keyword: required (token WhileKw);
                    open_paren: required (token LParen);
                    condition: required (category Expression);
                    close_paren: required (token RParen);
                    semicolon: required (token Semicolon);
                }
                ForStatement => ForStatement [for_statement valid] {
                    form: required (node_set [BasicForStatement, EnhancedForStatement]) => ForStatementForm;
                }
                BasicForStatement => BasicForStatement [basic_for_statement valid] {
                    for_keyword: required (token ForKw);
                    open_paren: required (token LParen);
                    initializer: optional (node ForInitializer);
                    first_semicolon: required (token Semicolon);
                    condition: optional (category Expression);
                    second_semicolon: required (token Semicolon);
                    update: optional (node ForUpdate);
                    close_paren: required (token RParen);
                    body: required (category Statement);
                }
                EnhancedForStatement => EnhancedForStatement [enhanced_for_statement valid] {
                    for_keyword: required (token ForKw);
                    open_paren: required (token LParen);
                    variable: required (category EnhancedForVariableSyntax);
                    colon: required (token Colon);
                    iterable: required (category Expression);
                    close_paren: required (token RParen);
                    body: required (category Statement);
                }
                EnhancedForVariable => EnhancedForVariable [enhanced_for_variable valid] {
                    modifiers: required (list ParameterModifierList);
                    r#type: required (choice [(category Type), (contextual "var")]) => EnhancedForVariableType;
                    name: required (token_set [Identifier, UnderscoreKw]);
                    dimensions: optional (list ArrayDimensions);
                }
                ForInitializer => ForInitializer [for_initializer valid] {
                    value: required (choice [(node LocalVariableDeclaration), (list StatementExpressionList)]) => ForInitializerValue;
                }
                ForUpdate => ForUpdate [for_update valid] {
                    expressions: required (list StatementExpressionList);
                }
                StatementExpressionList => StatementExpressionList [statement_expression_list list] {
                    expressions: one_or_more (category Expression) [separated (token Comma), minimum 1, trailing forbidden, recovery bogus_owner];
                }
                BreakStatement => BreakStatement [break_statement valid] {
                    break_keyword: required (token BreakKw);
                    label: optional (token Identifier);
                    semicolon: required (token Semicolon);
                }
                YieldStatement => YieldStatement [yield_statement valid] {
                    yield_keyword: required (contextual "yield");
                    expression: required (category Expression);
                    semicolon: required (token Semicolon);
                }
                ContinueStatement => ContinueStatement [continue_statement valid] {
                    continue_keyword: required (token ContinueKw);
                    label: optional (token Identifier);
                    semicolon: required (token Semicolon);
                }
                ReturnStatement => ReturnStatement [return_statement valid] {
                    return_keyword: required (token ReturnKw);
                    expression: optional (category Expression);
                    semicolon: required (token Semicolon);
                }
                ThrowStatement => ThrowStatement [throw_statement valid] {
                    throw_keyword: required (token ThrowKw);
                    expression: required (category Expression);
                    semicolon: required (token Semicolon);
                }
                SynchronizedStatement => SynchronizedStatement [synchronized_statement valid] {
                    synchronized_keyword: required (token SynchronizedKw);
                    open_paren: required (token LParen);
                    expression: required (category Expression);
                    close_paren: required (token RParen);
                    body: required (node Block);
                }
                TryStatement => TryStatement [try_statement valid] {
                    try_keyword: required (token TryKw);
                    body: required (node Block);
                    catches: required (list CatchClauseList);
                    finally: optional (node FinallyClause);
                }
                TryWithResourcesStatement => TryWithResourcesStatement [try_with_resources_statement valid] {
                    try_keyword: required (token TryKw);
                    resources: required (node ResourceSpecification);
                    body: required (node Block);
                    catches: required (list CatchClauseList);
                    finally: optional (node FinallyClause);
                }
                CatchClause => CatchClause [catch_clause valid] {
                    catch_keyword: required (token CatchKw);
                    open_paren: required (token LParen);
                    parameter: required (node CatchParameter);
                    close_paren: required (token RParen);
                    body: required (node Block);
                }
                CatchParameter => CatchParameter [catch_parameter valid] {
                    modifiers: required (list ParameterModifierList);
                    types: required (node CatchTypeList);
                    name: required (token_set [Identifier, UnderscoreKw]);
                    dimensions: optional (list ArrayDimensions);
                }
                CatchTypeList => CatchTypeList [catch_type_list valid] {
                    types: required (node_set [ClassType, UnionType, BogusType]) => CatchParameterTypes;
                }
                FinallyClause => FinallyClause [finally_clause valid] {
                    finally_keyword: required (token FinallyKw);
                    body: required (node Block);
                }
                ResourceSpecification => ResourceSpecification [resource_specification valid] {
                    open_paren: required (token LParen);
                    resources: required (list ResourceList);
                    trailing_semicolon: optional (token Semicolon);
                    close_paren: required (token RParen);
                }
                ResourceList => ResourceList [resource_list list] {
                    resources: one_or_more (node Resource) [separated (token Semicolon), minimum 1, trailing forbidden, recovery bogus_owner];
                }
                Resource => Resource [resource valid] {
                    value: required (category ResourceValueSyntax);
                }
                ResourceVariableDeclaration => ResourceVariableDeclaration [resource_variable_declaration valid] {
                    modifiers: required (list ParameterModifierList);
                    r#type: required (choice [(category Type), (contextual "var")]) => ResourceVariableType;
                    name: required (token_set [Identifier, UnderscoreKw]);
                    dimensions: optional (list ArrayDimensions);
                    assign: required (token Assign);
                    initializer: required (node VariableInitializer);
                }
                VariableAccess => VariableAccess [variable_access valid] {
                    expression: required (node_set [NameExpression, FieldAccessExpression]) => VariableAccessExpression;
                }
                PrimitiveType => PrimitiveType [primitive_type valid] {
                    annotations: optional (list AnnotationList);
                    keyword: required (token_set [BooleanKw, ByteKw, CharKw, DoubleKw, FloatKw, IntKw, LongKw, ShortKw]);
                }
                VoidType => VoidType [void_type valid] {
                    void_keyword: required (token VoidKw);
                }
                ClassType => ClassType [class_type valid] {
                    segments: required (list ClassTypeSegmentList);
                }
                ArrayType => ArrayType [array_type valid] {
                    element_type: required (node_set [PrimitiveType, ClassType]) => ArrayElementType;
                    dimensions: required (list ArrayDimensions);
                }
                IntersectionType => IntersectionType [intersection_type valid] {
                    first_type: required (category Type);
                    first_amp: required (token Amp);
                    remaining_types: required (list TypeAmpList);
                }
                UnionType => UnionType [union_type valid] {
                    first_type: required (category Type);
                    first_bar: required (token Bar);
                    remaining_types: required (list TypeBarList);
                }
                TypeArgumentList => TypeArgumentList [type_argument_list valid] {
                    open_angle: required (token Lt);
                    arguments: required (list TypeArgumentSeparatedList);
                    close_angle: required (token Gt);
                }
                TypeArgument => TypeArgument [type_argument valid] {
                    annotations: required (list AnnotationList);
                    r#type: required (category Type);
                }
                WildcardType => WildcardType [wildcard_type valid] {
                    question: required (token Question);
                    bound_keyword: optional (token_set [ExtendsKw, SuperKw]);
                    bound: optional (category Type);
                }
                ArrayDimensions => ArrayDimensions [array_dimensions list] {
                    dimensions: one_or_more (node ArrayDimension);
                }
                ArrayDimension => ArrayDimension [array_dimension valid] {
                    annotations: required (list AnnotationList);
                    open_bracket: required (token LBracket);
                    close_bracket: required (token RBracket);
                }
                Name => Name [name valid] {
                    identifier: required (token Identifier);
                }
                QualifiedName => QualifiedName [qualified_name valid] {
                    first_segment: required (constructed QualifiedNameSegmentNode);
                    first_dot: required (token Dot);
                    remaining_segments: required (list NameSegmentDotList);
                }
                LiteralExpression => LiteralExpression [literal_expression valid] {
                    literal: required (token_set [IntegerLiteral, FloatingPointLiteral, BooleanLiteral, CharacterLiteral, StringLiteral, TextBlockLiteral, NullLiteral]);
                }
                TemplateExpression => TemplateExpression [template_expression valid] {
                    processor: required (category Expression);
                    dot: required (token Dot);
                    template: required (node LiteralExpression);
                }
                NameExpression => NameExpression [name_expression valid] {
                    annotations: optional (list AnnotationList);
                    identifier: required (token Identifier);
                }
                ThisExpression => ThisExpression [this_expression valid] {
                    qualifier: optional (category Expression);
                    dot: optional (token Dot);
                    this_keyword: required (token ThisKw);
                }
                SuperExpression => SuperExpression [super_expression valid] {
                    qualifier: optional (category Expression);
                    dot: optional (token Dot);
                    super_keyword: required (token SuperKw);
                }
                ParenthesizedExpression => ParenthesizedExpression [parenthesized_expression valid] {
                    open_paren: required (token LParen);
                    expression: required (category Expression);
                    close_paren: required (token RParen);
                }
                ClassLiteralExpression => ClassLiteralExpression [class_literal_expression valid] {
                    target: required (category ClassLiteralTargetSyntax);
                    dimensions: optional (list ArrayDimensions);
                    dot: required (token Dot);
                    class_keyword: required (token ClassKw);
                }
                FieldAccessExpression => FieldAccessExpression [field_access_expression valid] {
                    receiver: required (category Expression);
                    dot: required (token Dot);
                    name: required (token Identifier);
                    type_arguments: optional (node TypeArgumentList);
                }
                ArrayAccessExpression => ArrayAccessExpression [array_access_expression valid] {
                    array: required (category Expression);
                    open_bracket: required (token LBracket);
                    index: required (category Expression);
                    close_bracket: required (token RBracket);
                }
                MethodInvocationExpression => MethodInvocationExpression [method_invocation_expression valid] {
                    form: required (category MethodInvocationFormSyntax);
                }
                MethodReferenceExpression => MethodReferenceExpression [method_reference_expression valid] {
                    receiver: required (category MethodReferenceReceiverSyntax);
                    receiver_type_arguments: optional (node TypeArgumentList);
                    double_colon: required (token DoubleColon);
                    target_type_arguments: optional (node TypeArgumentList);
                    target: required (token_set [Identifier, NewKw]);
                }
                ObjectCreationExpression => ObjectCreationExpression [object_creation_expression valid] {
                    qualifier: optional (category Expression);
                    dot: optional (token Dot);
                    new_keyword: required (token NewKw);
                    constructor_type_arguments: optional (node TypeArgumentList);
                    r#type: required (category ObjectCreationTypeSyntax);
                    arguments: required (node ArgumentList);
                    body: optional (node ClassBody);
                }
                ArrayCreationExpression => ArrayCreationExpression [array_creation_expression valid] {
                    new_keyword: required (token NewKw);
                    r#type: required (category ArrayCreationTypeSyntax);
                    dimension_expressions: required (list DimExpressionList);
                    dimensions: optional (list ArrayDimensions);
                    initializer: optional (node ArrayInitializer);
                }
                DimExpression => DimExpression [dim_expression valid] {
                    annotations: required (list AnnotationList);
                    open_bracket: required (token LBracket);
                    expression: required (category Expression);
                    close_bracket: required (token RBracket);
                }
                ArrayInitializer => ArrayInitializer [array_initializer valid] {
                    open_brace: required (token LBrace);
                    values: required (list VariableInitializerList);
                    close_brace: required (token RBrace);
                }
                AssignmentExpression => AssignmentExpression [assignment_expression valid] {
                    left: required (category AssignmentTargetSyntax);
                    operator: required (choice [
                        (token_set [
                            Assign, PlusEq, MinusEq, StarEq, SlashEq, AmpEq, BarEq,
                            CaretEq, PercentEq, LShiftEq
                        ]),
                        (constructed RightShiftAssignmentOperator),
                        (constructed UnsignedRightShiftAssignmentOperator)
                    ]) => AssignmentOperatorRole;
                    right: required (category Expression);
                }
                ConditionalExpression => ConditionalExpression [conditional_expression valid] {
                    condition: required (category Expression);
                    question: required (token Question);
                    then_expression: required (category Expression);
                    colon: required (token Colon);
                    else_expression: required (category Expression);
                }
                InstanceofExpression => InstanceofExpression [instanceof_expression valid] {
                    expression: required (category Expression);
                    instanceof_keyword: required (token InstanceofKw);
                    target: required (category InstanceofTargetSyntax);
                }
                BinaryExpression => BinaryExpression [binary_expression valid] {
                    left: required (category Expression);
                    operator: required (choice [
                        (token_set [
                            OrOr, AndAnd, Bar, Caret, Amp, EqEq, BangEq, Lt, Gt,
                            LtEq, LShift, Plus, Minus, Star, Slash, Percent
                        ]),
                        (constructed GreaterThanOrEqualOperator),
                        (constructed RightShiftOperator),
                        (constructed UnsignedRightShiftOperator)
                    ]) => BinaryOperatorRole;
                    right: required (category Expression);
                }
                UnaryExpression => UnaryExpression [unary_expression valid] {
                    operator: required (token_set [PlusPlus, MinusMinus, Plus, Minus, Tilde, Bang]);
                    operand: required (category Expression);
                }
                PostfixExpression => PostfixExpression [postfix_expression valid] {
                    operand: required (category Expression);
                    operator: required (token_set [PlusPlus, MinusMinus]);
                }
                CastExpression => CastExpression [cast_expression valid] {
                    open_paren: required (token LParen);
                    r#type: required (category Type);
                    close_paren: required (token RParen);
                    expression: required (category Expression);
                }
                LambdaExpression => LambdaExpression [lambda_expression valid] {
                    open_paren: optional (token LParen);
                    parameters: required (list LambdaParameterList);
                    close_paren: optional (token RParen);
                    arrow: required (token Arrow);
                    body: required (category LambdaBodySyntax);
                }
                LambdaParameterList => LambdaParameterList [lambda_parameter_list list] {
                    parameters: many (node LambdaParameter) [separated (token Comma), minimum 0, trailing forbidden, recovery bogus_owner];
                }
                LambdaParameter => LambdaParameter [lambda_parameter valid] {
                    modifiers: required (list LambdaModifierList);
                    r#type: optional (category Type);
                    varargs_annotations: required (list AnnotationList);
                    ellipsis: optional (token Ellipsis);
                    name: required (token_set [Identifier, UnderscoreKw]);
                    dimensions: optional (list ArrayDimensions);
                }
                SwitchExpression => SwitchExpression [switch_expression valid] {
                    switch_keyword: required (token SwitchKw);
                    open_paren: required (token LParen);
                    selector: required (category Expression);
                    close_paren: required (token RParen);
                    body: required (node SwitchBlock);
                }
                ArgumentList => ArgumentList [argument_list valid] {
                    open_paren: required (token LParen);
                    arguments: required (list ExpressionList);
                    close_paren: required (token RParen);
                }
                TypePattern => TypePattern [type_pattern valid] {
                    modifiers: required (list ParameterModifierList);
                    r#type: required (node_set [PrimitiveType, ClassType, ArrayType, BogusType]) => TypePatternType;
                    name: required (token_set [Identifier, UnderscoreKw]);
                    dimensions: optional (list ArrayDimensions);
                }
                RecordPattern => RecordPattern [record_pattern valid] {
                    r#type: required (node_set [ClassType, BogusType]) => RecordPatternType;
                    open_paren: required (token LParen);
                    components: required (list ComponentPatternList);
                    close_paren: required (token RParen);
                }
                ComponentPattern => ComponentPattern [component_pattern valid] {
                    pattern: required (category Pattern);
                }
                MatchAllPattern => MatchAllPattern [match_all_pattern valid] {
                    underscore: required (token UnderscoreKw);
                }
                NonSealedModifier => NonSealedModifier [non_sealed_modifier valid] {
                    non_keyword: required (contextual "non");
                    minus: required (token Minus);
                    sealed_keyword: required (contextual "sealed");
                }
                ClassTypeSegmentNode => ClassTypeSegmentNode [class_type_segment_node valid] {
                    annotations: required (list AnnotationList);
                    name: required (category NameSyntax);
                    type_arguments: optional (node TypeArgumentList);
                }
                QualifiedNameSegmentNode => QualifiedNameSegmentNode [qualified_name_segment_node valid] {
                    annotations: required (list AnnotationList);
                    identifier: required (token Identifier);
                }
                QualifiedMethodInvocation => QualifiedMethodInvocation [qualified_method_invocation valid] {
                    receiver: required (category Expression);
                    dot: required (token Dot);
                    type_arguments: optional (node TypeArgumentList);
                    name: required (choice [(node NameExpression), (token Identifier)]) => QualifiedInvocationName;
                    arguments: required (node ArgumentList);
                }
                UnqualifiedMethodInvocation => UnqualifiedMethodInvocation [unqualified_method_invocation valid] {
                    type_arguments: optional (node TypeArgumentList);
                    name: required (choice [(node NameExpression), (token Identifier)]) => UnqualifiedInvocationName;
                    arguments: required (node ArgumentList);
                }
                GreaterThanOrEqualOperator => GreaterThanOrEqualOperator [greater_than_or_equal_operator valid] {
                    greater_than: required (token Gt);
                    assign: required (token Assign);
                }
                RightShiftOperator => RightShiftOperator [right_shift_operator valid] {
                    first_greater_than: required (token Gt);
                    second_greater_than: required (token Gt);
                }
                UnsignedRightShiftOperator => UnsignedRightShiftOperator [unsigned_right_shift_operator valid] {
                    first_greater_than: required (token Gt);
                    second_greater_than: required (token Gt);
                    third_greater_than: required (token Gt);
                }
                RightShiftAssignmentOperator => RightShiftAssignmentOperator [right_shift_assignment_operator valid] {
                    first_greater_than: required (token Gt);
                    second_greater_than: required (token Gt);
                    assign: required (token Assign);
                }
                UnsignedRightShiftAssignmentOperator => UnsignedRightShiftAssignmentOperator [unsigned_right_shift_assignment_operator valid] {
                    first_greater_than: required (token Gt);
                    second_greater_than: required (token Gt);
                    third_greater_than: required (token Gt);
                    assign: required (token Assign);
                }
                CompilationUnitItemList => CompilationUnitItemList [compilation_unit_item_list list] {
                    elements: many (category CompilationUnitItem);
                }
                AnnotationList => AnnotationList [annotation_list list] {
                    elements: many (node Annotation);
                }
                ModuleDirectiveList => ModuleDirectiveList [module_directive_list list] {
                    elements: many (category ModuleDirective);
                }
                RequiresModifierList => RequiresModifierList [requires_modifier_list list] {
                    elements: many (choice [(token StaticKw), (contextual "transitive")]) => RequiresModifier;
                }
                NameList => NameList [name_list list] {
                    elements: many (category NameSyntax) [separated (token Comma), minimum 0, trailing forbidden, recovery bogus_owner];
                }
                ModuleNameList => ModuleNameList [module_name_list list] {
                    elements: one_or_more (category NameSyntax) [separated (token Comma), minimum 1, trailing forbidden, recovery bogus_owner];
                }
                AnnotationElementArgumentList => AnnotationElementArgumentList [annotation_element_argument_list list] {
                    elements: many (category AnnotationArgumentSyntax) [separated (token Comma), minimum 0, trailing forbidden, recovery bogus_owner];
                }
                AnnotationInterfaceBodyMemberList => AnnotationInterfaceBodyMemberList [annotation_interface_body_member_list list] {
                    elements: many (category AnnotationInterfaceBodyMember);
                }
                AnnotationElementValueList => AnnotationElementValueList [annotation_element_value_list list] {
                    elements: many (node AnnotationElementValue) [separated (token Comma), minimum 0, trailing optional, recovery bogus_owner];
                }
                TypeParameterSeparatedList => TypeParameterSeparatedList [type_parameter_separated_list list] {
                    elements: many (node TypeParameter) [separated (token Comma), minimum 0, trailing forbidden, recovery bogus_owner];
                }
                TypeList => TypeList [type_list list] {
                    elements: many (category Type) [separated (token Comma), minimum 0, trailing forbidden, recovery bogus_owner];
                }
                ClassBodyMemberList => ClassBodyMemberList [class_body_member_list list] {
                    elements: many (category ClassBodyMember);
                }
                RecordBodyMemberList => RecordBodyMemberList [record_body_member_list list] {
                    elements: many (category ClassBodyMember);
                }
                InterfaceBodyMemberList => InterfaceBodyMemberList [interface_body_member_list list] {
                    elements: many (category InterfaceBodyMember);
                }
                ParameterModifierList => ParameterModifierList [parameter_modifier_list list] {
                    elements: many (element_set [Annotation, BogusModifier, FinalKw]) => ParameterModifier;
                }
                ConstructorBodyEntryList => ConstructorBodyEntryList [constructor_body_entry_list list] {
                    elements: many (category ConstructorBodyEntry);
                }
                BlockStatementList => BlockStatementList [block_statement_list list] {
                    elements: many (node BlockStatement);
                }
                SwitchEntryList => SwitchEntryList [switch_entry_list list] {
                    elements: many (category SwitchEntrySyntax);
                }
                SwitchLabelColonList => SwitchLabelColonList [switch_label_colon_list list] {
                    elements: many (node SwitchLabel) [separated (token Colon), minimum 0, trailing required, recovery bogus_owner];
                }
                SwitchLabelItemList => SwitchLabelItemList [switch_label_item_list list] {
                    elements: many (element_set [CaseConstant, CasePattern, BogusSwitchLabelItem, DefaultKw]) => SwitchLabelItem [separated (token Comma), minimum 0, trailing forbidden, recovery bogus_owner];
                }
                CatchClauseList => CatchClauseList [catch_clause_list list] {
                    elements: many (node CatchClause);
                }
                ClassTypeSegmentList => ClassTypeSegmentList [class_type_segment_list list] {
                    elements: many (constructed ClassTypeSegmentNode) [separated (token Dot), minimum 0, trailing forbidden, recovery bogus_owner];
                }
                TypeAmpList => TypeAmpList [type_amp_list list] {
                    elements: many (category Type) [separated (token Amp), minimum 0, trailing forbidden, recovery bogus_owner];
                }
                TypeBarList => TypeBarList [type_bar_list list] {
                    elements: many (category Type) [separated (token Bar), minimum 0, trailing forbidden, recovery bogus_owner];
                }
                TypeArgumentSeparatedList => TypeArgumentSeparatedList [type_argument_separated_list list] {
                    elements: many (node TypeArgument) [separated (token Comma), minimum 0, trailing forbidden, recovery bogus_owner];
                }
                NameSegmentDotList => NameSegmentDotList [name_segment_dot_list list] {
                    elements: many (constructed QualifiedNameSegmentNode) [separated (token Dot), minimum 0, trailing forbidden, recovery bogus_owner];
                }
                DimExpressionList => DimExpressionList [dim_expression_list list] {
                    elements: many (node DimExpression);
                }
                VariableInitializerList => VariableInitializerList [variable_initializer_list list] {
                    elements: many (category VariableInitializerValue) [separated (token Comma), minimum 0, trailing optional, recovery bogus_owner];
                }
                LambdaModifierList => LambdaModifierList [lambda_modifier_list list] {
                    elements: many (choice [(node Annotation), (token FinalKw), (contextual "var")]) => LambdaModifier;
                }
                ExpressionList => ExpressionList [expression_list list] {
                    elements: many (category Expression) [separated (token Comma), minimum 0, trailing forbidden, recovery bogus_owner];
                }
                ComponentPatternList => ComponentPatternList [component_pattern_list list] {
                    elements: many (node ComponentPattern) [separated (token Comma), minimum 0, trailing forbidden, recovery bogus_owner];
                }
            }
        }
    };
}
