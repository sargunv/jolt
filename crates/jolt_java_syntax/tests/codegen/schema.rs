//! The Java syntax schema: the single source of truth for kinds and tree
//! shape. `main.rs` renders it to `src/generated.rs`.

use super::model::*;

#[allow(clippy::too_many_lines)]
pub fn java() -> Schema {
    Schema {
        tokens: vec![
            "Eof",
            "Unknown",
            "Identifier",
            "IntegerLiteral",
            "FloatingPointLiteral",
            "BooleanLiteral",
            "CharacterLiteral",
            "StringLiteral",
            "TextBlockLiteral",
            "NullLiteral",
            "AbstractKw",
            "AssertKw",
            "BooleanKw",
            "BreakKw",
            "ByteKw",
            "CaseKw",
            "CatchKw",
            "CharKw",
            "ClassKw",
            "ConstKw",
            "ContinueKw",
            "DefaultKw",
            "DoKw",
            "DoubleKw",
            "ElseKw",
            "EnumKw",
            "ExtendsKw",
            "FinalKw",
            "FinallyKw",
            "FloatKw",
            "ForKw",
            "GotoKw",
            "IfKw",
            "ImplementsKw",
            "ImportKw",
            "InstanceofKw",
            "IntKw",
            "InterfaceKw",
            "LongKw",
            "NativeKw",
            "NewKw",
            "PackageKw",
            "PrivateKw",
            "ProtectedKw",
            "PublicKw",
            "ReturnKw",
            "ShortKw",
            "StaticKw",
            "StrictfpKw",
            "SuperKw",
            "SwitchKw",
            "SynchronizedKw",
            "ThisKw",
            "ThrowKw",
            "ThrowsKw",
            "TransientKw",
            "TryKw",
            "VoidKw",
            "VolatileKw",
            "WhileKw",
            "UnderscoreKw",
            "LParen",
            "RParen",
            "LBrace",
            "RBrace",
            "LBracket",
            "RBracket",
            "Semicolon",
            "Comma",
            "Dot",
            "Ellipsis",
            "At",
            "Colon",
            "DoubleColon",
            "Assign",
            "Gt",
            "GtEq",
            "Lt",
            "Bang",
            "Tilde",
            "Question",
            "Arrow",
            "EqEq",
            "LtEq",
            "BangEq",
            "AndAnd",
            "OrOr",
            "PlusPlus",
            "MinusMinus",
            "Plus",
            "Minus",
            "Star",
            "Slash",
            "Amp",
            "Bar",
            "Caret",
            "Percent",
            "LShift",
            "RShift",
            "UnsignedRShift",
            "PlusEq",
            "MinusEq",
            "StarEq",
            "SlashEq",
            "AmpEq",
            "BarEq",
            "CaretEq",
            "PercentEq",
            "LShiftEq",
            "RShiftEq",
            "UnsignedRShiftEq",
        ],
        categories: vec![
            category(
                "CompilationUnitItem",
                &[
                    "PackageDeclaration",
                    "ImportDeclaration",
                    "ModuleDeclaration",
                    "ClassDeclaration",
                    "RecordDeclaration",
                    "EnumDeclaration",
                    "InterfaceDeclaration",
                    "AnnotationInterfaceDeclaration",
                    "FieldDeclaration",
                    "MethodDeclaration",
                    "EmptyDeclaration",
                ],
            ),
            category(
                "Statement",
                &[
                    "Block",
                    "EmptyStatement",
                    "LabeledStatement",
                    "ExpressionStatement",
                    "IfStatement",
                    "AssertStatement",
                    "SwitchStatement",
                    "WhileStatement",
                    "DoStatement",
                    "ForStatement",
                    "BreakStatement",
                    "YieldStatement",
                    "ContinueStatement",
                    "ReturnStatement",
                    "ThrowStatement",
                    "SynchronizedStatement",
                    "TryStatement",
                    "TryWithResourcesStatement",
                ],
            ),
            category("EnhancedForVariableSyntax", &["EnhancedForVariable"]),
            category(
                "ResourceValueSyntax",
                &["ResourceVariableDeclaration", "VariableAccess"],
            ),
            category(
                "SwitchEntrySyntax",
                &["SwitchBlockStatementGroup", "SwitchRule"],
            ),
            category("SwitchGuardSyntax", &["Guard"]),
            category(
                "Expression",
                &[
                    "LiteralExpression",
                    "NameExpression",
                    "ThisExpression",
                    "SuperExpression",
                    "ParenthesizedExpression",
                    "ClassLiteralExpression",
                    "FieldAccessExpression",
                    "ArrayAccessExpression",
                    "MethodInvocationExpression",
                    "MethodReferenceExpression",
                    "ObjectCreationExpression",
                    "ArrayCreationExpression",
                    "AssignmentExpression",
                    "ConditionalExpression",
                    "InstanceofExpression",
                    "BinaryExpression",
                    "UnaryExpression",
                    "PostfixExpression",
                    "CastExpression",
                    "LambdaExpression",
                    "SwitchExpression",
                ],
            ),
            category(
                "AssignmentTargetSyntax",
                &[
                    "NameExpression",
                    "FieldAccessExpression",
                    "ArrayAccessExpression",
                ],
            ),
            category(
                "MethodInvocationFormSyntax",
                &["QualifiedMethodInvocation", "UnqualifiedMethodInvocation"],
            ),
            category(
                "ClassLiteralTargetSyntax",
                &[
                    "PrimitiveType",
                    "VoidType",
                    "NameExpression",
                    "FieldAccessExpression",
                ],
            ),
            category(
                "MethodReferenceReceiverSyntax",
                &[
                    "LiteralExpression",
                    "NameExpression",
                    "ThisExpression",
                    "SuperExpression",
                    "ParenthesizedExpression",
                    "ClassLiteralExpression",
                    "FieldAccessExpression",
                    "ArrayAccessExpression",
                    "MethodInvocationExpression",
                    "ObjectCreationExpression",
                    "ArrayCreationExpression",
                    "SwitchExpression",
                    "ClassType",
                    "ArrayType",
                ],
            ),
            category(
                "LambdaBodySyntax",
                &[
                    "LiteralExpression",
                    "NameExpression",
                    "ThisExpression",
                    "SuperExpression",
                    "ParenthesizedExpression",
                    "ClassLiteralExpression",
                    "FieldAccessExpression",
                    "ArrayAccessExpression",
                    "MethodInvocationExpression",
                    "MethodReferenceExpression",
                    "ObjectCreationExpression",
                    "ArrayCreationExpression",
                    "AssignmentExpression",
                    "ConditionalExpression",
                    "InstanceofExpression",
                    "BinaryExpression",
                    "UnaryExpression",
                    "PostfixExpression",
                    "CastExpression",
                    "LambdaExpression",
                    "SwitchExpression",
                    "Block",
                ],
            ),
            category(
                "ArrayCreationTypeSyntax",
                &["PrimitiveType", "ClassType", "ArrayType"],
            ),
            category("ObjectCreationTypeSyntax", &["ClassType"]),
            category(
                "InstanceofTargetSyntax",
                &["ClassType", "ArrayType", "TypePattern", "RecordPattern"],
            ),
            category(
                "Type",
                &[
                    "PrimitiveType",
                    "VoidType",
                    "ClassType",
                    "ArrayType",
                    "IntersectionType",
                    "UnionType",
                    "WildcardType",
                ],
            ),
            category(
                "Pattern",
                &["TypePattern", "RecordPattern", "MatchAllPattern"],
            ),
            category("NameSyntax", &["Name", "QualifiedName"]),
            category(
                "FormalParameterSyntax",
                &["FormalParameter", "ReceiverParameter"],
            ),
            category(
                "AnnotationArgumentSyntax",
                &["AnnotationElementValue", "AnnotationElementValuePair"],
            ),
            category(
                "ModuleDirective",
                &[
                    "RequiresDirective",
                    "ExportsDirective",
                    "OpensDirective",
                    "UsesDirective",
                    "ProvidesDirective",
                ],
            ),
            category(
                "BlockItem",
                &[
                    "LocalVariableDeclaration",
                    "LocalClassOrInterfaceDeclaration",
                    "Block",
                    "EmptyStatement",
                    "LabeledStatement",
                    "ExpressionStatement",
                    "IfStatement",
                    "AssertStatement",
                    "SwitchStatement",
                    "WhileStatement",
                    "DoStatement",
                    "ForStatement",
                    "BreakStatement",
                    "YieldStatement",
                    "ContinueStatement",
                    "ReturnStatement",
                    "ThrowStatement",
                    "SynchronizedStatement",
                    "TryStatement",
                    "TryWithResourcesStatement",
                ],
            ),
            category(
                "ClassBodyMember",
                &[
                    "EmptyDeclaration",
                    "ClassDeclaration",
                    "RecordDeclaration",
                    "EnumDeclaration",
                    "InterfaceDeclaration",
                    "AnnotationInterfaceDeclaration",
                    "FieldDeclaration",
                    "MethodDeclaration",
                    "ConstructorDeclaration",
                    "CompactConstructorDeclaration",
                    "StaticInitializer",
                    "InstanceInitializer",
                ],
            ),
            category(
                "InterfaceBodyMember",
                &[
                    "EmptyDeclaration",
                    "ClassDeclaration",
                    "RecordDeclaration",
                    "EnumDeclaration",
                    "InterfaceDeclaration",
                    "AnnotationInterfaceDeclaration",
                    "FieldDeclaration",
                    "MethodDeclaration",
                ],
            ),
            category(
                "AnnotationInterfaceBodyMember",
                &[
                    "EmptyDeclaration",
                    "ClassDeclaration",
                    "RecordDeclaration",
                    "EnumDeclaration",
                    "InterfaceDeclaration",
                    "AnnotationInterfaceDeclaration",
                    "FieldDeclaration",
                    "MethodDeclaration",
                    "AnnotationElementDeclaration",
                ],
            ),
            category(
                "ConstructorBodyEntry",
                &["ConstructorInvocation", "BlockStatement"],
            ),
            category(
                "VariableInitializerValue",
                &[
                    "LiteralExpression",
                    "NameExpression",
                    "ThisExpression",
                    "SuperExpression",
                    "ParenthesizedExpression",
                    "ClassLiteralExpression",
                    "FieldAccessExpression",
                    "ArrayAccessExpression",
                    "MethodInvocationExpression",
                    "MethodReferenceExpression",
                    "ObjectCreationExpression",
                    "ArrayCreationExpression",
                    "AssignmentExpression",
                    "ConditionalExpression",
                    "InstanceofExpression",
                    "BinaryExpression",
                    "UnaryExpression",
                    "PostfixExpression",
                    "CastExpression",
                    "LambdaExpression",
                    "SwitchExpression",
                    "ArrayInitializer",
                ],
            ),
        ],
        nodes: vec![
            node(
                "CompilationUnit",
                vec![
                    req("items", n("CompilationUnitItemList")),
                    req("eof", t("Eof")),
                ],
            ),
            node(
                "PackageDeclaration",
                vec![
                    req("annotations", n("AnnotationList")),
                    req("package_keyword", t("PackageKw")),
                    req("name", cat("NameSyntax")),
                    req("semicolon", t("Semicolon")),
                ],
            ),
            node(
                "ImportDeclaration",
                vec![
                    req("import_keyword", t("ImportKw")),
                    opt("module_keyword", kw("module")),
                    opt("static_keyword", t("StaticKw")),
                    req("name", cat("NameSyntax")),
                    opt("on_demand_dot", t("Dot")),
                    opt("star", t("Star")),
                    opt("suffix", n("Bogus")),
                    req("semicolon", t("Semicolon")),
                ],
            ),
            node(
                "ModuleDeclaration",
                vec![
                    req("annotations", n("AnnotationList")),
                    opt("open_keyword", kw("open")),
                    req("module_keyword", kw("module")),
                    req("name", cat("NameSyntax")),
                    req("open_brace", t("LBrace")),
                    req("directives", n("ModuleDirectiveList")),
                    req("close_brace", t("RBrace")),
                ],
            ),
            node(
                "RequiresDirective",
                vec![
                    req("requires_keyword", kw("requires")),
                    req("modifiers", n("RequiresModifierList")),
                    req("module", cat("NameSyntax")),
                    req("semicolon", t("Semicolon")),
                ],
            ),
            node(
                "ExportsDirective",
                vec![
                    req("exports_keyword", kw("exports")),
                    req("package", cat("NameSyntax")),
                    opt("targets", n("ModuleTargetClause")),
                    req("semicolon", t("Semicolon")),
                ],
            ),
            node(
                "OpensDirective",
                vec![
                    req("opens_keyword", kw("opens")),
                    req("package", cat("NameSyntax")),
                    opt("targets", n("ModuleTargetClause")),
                    req("semicolon", t("Semicolon")),
                ],
            ),
            node(
                "UsesDirective",
                vec![
                    req("uses_keyword", kw("uses")),
                    req("service", cat("NameSyntax")),
                    req("semicolon", t("Semicolon")),
                ],
            ),
            node(
                "ProvidesDirective",
                vec![
                    req("provides_keyword", kw("provides")),
                    req("service", cat("NameSyntax")),
                    req("implementation", n("ModuleImplementationClause")),
                    req("semicolon", t("Semicolon")),
                ],
            ),
            node(
                "ModuleTargetClause",
                vec![
                    req("to_keyword", kw("to")),
                    req("targets", n("ModuleNameList")),
                ],
            ),
            node(
                "ModuleImplementationClause",
                vec![
                    req("with_keyword", kw("with")),
                    req("implementations", n("ModuleNameList")),
                ],
            ),
            list(
                "ModifierList",
                1,
                choice(vec![
                    n("Annotation"),
                    n("Bogus"),
                    tokens(&[
                        "AbstractKw",
                        "FinalKw",
                        "NativeKw",
                        "PrivateKw",
                        "ProtectedKw",
                        "PublicKw",
                        "StaticKw",
                        "StrictfpKw",
                        "SynchronizedKw",
                        "TransientKw",
                        "VolatileKw",
                    ]),
                    t("DefaultKw"),
                    kw("sealed"),
                    n("NonSealedModifier"),
                ]),
            )
            .role("ModifierElement"),
            node(
                "Annotation",
                vec![
                    req("at", t("At")),
                    req("name", cat("NameSyntax")),
                    opt("arguments", n("AnnotationArgumentList")),
                ],
            ),
            node(
                "AnnotationArgumentList",
                vec![
                    req("open_paren", t("LParen")),
                    req("elements", n("AnnotationElementArgumentList")),
                    req("close_paren", t("RParen")),
                ],
            ),
            node(
                "AnnotationElementDeclaration",
                vec![
                    opt("modifiers", n("ModifierList")),
                    req("type", cat("Type")),
                    req("name", t("Identifier")),
                    req("open_paren", t("LParen")),
                    req("close_paren", t("RParen")),
                    opt("dimensions", n("ArrayDimensions")),
                    opt("default", n("DefaultValue")),
                    req("semicolon", t("Semicolon")),
                ],
            ),
            node(
                "AnnotationElementValue",
                vec![
                    req(
                        "value",
                        choice(vec![
                            cat("Expression"),
                            nodes(&["Annotation", "AnnotationArrayInitializer"]),
                        ]),
                    )
                    .role("AnnotationElementValueContent"),
                ],
            ),
            node(
                "AnnotationElementValuePair",
                vec![
                    req("name", t("Identifier")),
                    req("assign", t("Assign")),
                    req("value", n("AnnotationElementValue")),
                ],
            ),
            node(
                "AnnotationArrayInitializer",
                vec![
                    req("open_brace", t("LBrace")),
                    req("values", n("AnnotationElementValueList")),
                    req("close_brace", t("RBrace")),
                ],
            ),
            node(
                "DefaultValue",
                vec![
                    req("default_keyword", t("DefaultKw")),
                    req("value", n("AnnotationElementValue")),
                ],
            ),
            node(
                "ClassDeclaration",
                vec![
                    opt("modifiers", n("ModifierList")),
                    req("class_keyword", t("ClassKw")),
                    req("name", t("Identifier")),
                    opt("type_parameters", n("TypeParameterList")),
                    opt("extends", n("ExtendsClause")),
                    opt("implements", n("ImplementsClause")),
                    opt("permits", n("PermitsClause")),
                    req("body", n("ClassBody")),
                    opt("missing_body_semicolon", t("Semicolon")),
                ],
            ),
            node(
                "RecordDeclaration",
                vec![
                    opt("modifiers", n("ModifierList")),
                    req("record_keyword", kw("record")),
                    req("name", t("Identifier")),
                    opt("type_parameters", n("TypeParameterList")),
                    req("open_paren", t("LParen")),
                    opt("components", n("RecordComponentList")),
                    req("close_paren", t("RParen")),
                    opt("implements", n("ImplementsClause")),
                    req("body", n("RecordBody")),
                    opt("missing_body_semicolon", t("Semicolon")),
                ],
            ),
            node(
                "EnumDeclaration",
                vec![
                    opt("modifiers", n("ModifierList")),
                    req("enum_keyword", t("EnumKw")),
                    req("name", t("Identifier")),
                    opt("implements", n("ImplementsClause")),
                    req("body", n("EnumBody")),
                    opt("missing_body_semicolon", t("Semicolon")),
                ],
            ),
            node(
                "InterfaceDeclaration",
                vec![
                    opt("modifiers", n("ModifierList")),
                    req("interface_keyword", t("InterfaceKw")),
                    req("name", t("Identifier")),
                    opt("type_parameters", n("TypeParameterList")),
                    opt("extends", n("ExtendsClause")),
                    opt("permits", n("PermitsClause")),
                    req("body", n("InterfaceBody")),
                    opt("missing_body_semicolon", t("Semicolon")),
                ],
            ),
            node(
                "AnnotationInterfaceDeclaration",
                vec![
                    opt("modifiers", n("ModifierList")),
                    req("at", t("At")),
                    req("interface_keyword", t("InterfaceKw")),
                    req("name", t("Identifier")),
                    req("body", n("AnnotationInterfaceBody")),
                    opt("missing_body_semicolon", t("Semicolon")),
                ],
            ),
            node(
                "TypeParameterList",
                vec![
                    req("open_angle", t("Lt")),
                    req("parameters", n("TypeParameterSeparatedList")),
                    req("close_angle", t("Gt")),
                ],
            ),
            node(
                "TypeParameter",
                vec![
                    req("annotations", n("AnnotationList")),
                    req("name", t("Identifier")),
                    opt("bounds", n("TypeBoundList")),
                ],
            ),
            node(
                "TypeBoundList",
                vec![
                    req("extends_keyword", t("ExtendsKw")),
                    req("bounds", nodes(&["ClassType", "IntersectionType", "Bogus"]))
                        .role("TypeBound"),
                ],
            ),
            node(
                "ExtendsClause",
                vec![
                    req("extends_keyword", t("ExtendsKw")),
                    req("types", n("TypeList")),
                ],
            ),
            node(
                "ImplementsClause",
                vec![
                    req("implements_keyword", t("ImplementsKw")),
                    req("types", n("TypeList")),
                ],
            ),
            node(
                "PermitsClause",
                vec![
                    req("permits_keyword", kw("permits")),
                    req("names", n("NameList")),
                ],
            ),
            node(
                "ClassBody",
                vec![
                    req("open_brace", t("LBrace")),
                    req("members", n("ClassBodyMemberList")),
                    req("close_brace", t("RBrace")),
                ],
            ),
            node("EmptyDeclaration", vec![req("semicolon", t("Semicolon"))]),
            node(
                "RecordBody",
                vec![
                    req("open_brace", t("LBrace")),
                    req("members", n("RecordBodyMemberList")),
                    req("close_brace", t("RBrace")),
                ],
            ),
            node(
                "InterfaceBody",
                vec![
                    req("open_brace", t("LBrace")),
                    req("members", n("InterfaceBodyMemberList")),
                    req("close_brace", t("RBrace")),
                ],
            ),
            node(
                "AnnotationInterfaceBody",
                vec![
                    req("open_brace", t("LBrace")),
                    req("members", n("AnnotationInterfaceBodyMemberList")),
                    req("close_brace", t("RBrace")),
                ],
            ),
            node(
                "EnumBody",
                vec![
                    req("open_brace", t("LBrace")),
                    opt("constants", n("EnumConstantList")),
                    opt("body_separator", t("Semicolon")),
                    req("members", n("ClassBodyMemberList")),
                    req("close_brace", t("RBrace")),
                ],
            ),
            list("EnumConstantList", 1, n("EnumConstant")).separated("Comma", Trailing::Optional),
            node(
                "EnumConstant",
                vec![
                    req("annotations", n("AnnotationList")),
                    req("name", t("Identifier")),
                    opt("arguments", n("ArgumentList")),
                    opt("body", n("ClassBody")),
                ],
            ),
            list("RecordComponentList", 1, n("RecordComponent"))
                .separated("Comma", Trailing::Forbidden),
            node(
                "RecordComponent",
                vec![
                    req("modifiers", n("ParameterModifierList")),
                    req("type", cat("Type")),
                    req("varargs_annotations", n("AnnotationList")),
                    opt("ellipsis", t("Ellipsis")),
                    req("name", tokens(&["Identifier", "UnderscoreKw"])),
                ],
            ),
            node(
                "FieldDeclaration",
                vec![
                    opt("modifiers", n("ModifierList")),
                    req("type", cat("Type")),
                    req("declarators", n("VariableDeclaratorList")),
                    req("semicolon", t("Semicolon")),
                ],
            ),
            node(
                "MethodDeclaration",
                vec![
                    opt("modifiers", n("ModifierList")),
                    opt("type_parameters", n("TypeParameterList")),
                    opt("return_annotations", n("AnnotationList")),
                    req("return_type", cat("Type")),
                    req("name", t("Identifier")),
                    req("open_paren", t("LParen")),
                    opt("parameters", n("FormalParameterList")),
                    req("close_paren", t("RParen")),
                    opt("dimensions", n("ArrayDimensions")),
                    opt("throws", n("ThrowsClause")),
                    req("body", choice(vec![n("Block"), t("Semicolon")])).role("MethodBody"),
                ],
            ),
            node(
                "ConstructorDeclaration",
                vec![
                    opt("modifiers", n("ModifierList")),
                    opt("type_parameters", n("TypeParameterList")),
                    req("name", t("Identifier")),
                    req("open_paren", t("LParen")),
                    opt("parameters", n("FormalParameterList")),
                    req("close_paren", t("RParen")),
                    opt("throws", n("ThrowsClause")),
                    req("body", n("ConstructorBody")),
                ],
            ),
            node(
                "ConstructorBody",
                vec![
                    req("open_brace", t("LBrace")),
                    req("entries", n("ConstructorBodyEntryList")),
                    req("close_brace", t("RBrace")),
                ],
            ),
            node(
                "ConstructorInvocation",
                vec![
                    opt(
                        "qualifier",
                        choice(vec![cat("Expression"), nodes(&["Name", "QualifiedName"])]),
                    )
                    .role("ConstructorQualifier"),
                    opt("dot", t("Dot")),
                    opt("type_arguments", n("TypeArgumentList")),
                    req("target", tokens(&["ThisKw", "SuperKw"])),
                    req("arguments", n("ArgumentList")),
                    req("semicolon", t("Semicolon")),
                ],
            ),
            node(
                "CompactConstructorDeclaration",
                vec![
                    opt("modifiers", n("ModifierList")),
                    req("name", t("Identifier")),
                    req("body", n("ConstructorBody")),
                ],
            ),
            node(
                "StaticInitializer",
                vec![
                    req("static_keyword", t("StaticKw")),
                    req("body", n("Block")),
                ],
            ),
            node("InstanceInitializer", vec![req("body", n("Block"))]),
            list("FormalParameterList", 1, cat("FormalParameterSyntax"))
                .separated("Comma", Trailing::Forbidden),
            node(
                "FormalParameter",
                vec![
                    req("modifiers", n("ParameterModifierList")),
                    req("type", cat("Type")),
                    req("varargs_annotations", n("AnnotationList")),
                    opt("ellipsis", t("Ellipsis")),
                    req("name", tokens(&["Identifier", "UnderscoreKw"])),
                    opt("dimensions", n("ArrayDimensions")),
                ],
            ),
            node(
                "ReceiverParameter",
                vec![
                    req("annotations", n("AnnotationList")),
                    req("type", cat("Type")),
                    opt("qualifier", t("Identifier")),
                    opt("dot", t("Dot")),
                    req("this_keyword", t("ThisKw")),
                ],
            ),
            node(
                "ThrowsClause",
                vec![
                    req("throws_keyword", t("ThrowsKw")),
                    req("exceptions", n("TypeList")),
                ],
            ),
            list("VariableDeclaratorList", 1, n("VariableDeclarator"))
                .separated("Comma", Trailing::Forbidden),
            node(
                "VariableDeclarator",
                vec![
                    req("name", tokens(&["Identifier", "UnderscoreKw"])),
                    opt("dimensions", n("ArrayDimensions")),
                    opt("assign", t("Assign")),
                    opt("initializer", n("VariableInitializer")),
                ],
            ),
            node(
                "VariableInitializer",
                vec![req("value", cat("VariableInitializerValue"))],
            ),
            node(
                "Block",
                vec![
                    req("open_brace", t("LBrace")),
                    req("statements", n("BlockStatementList")),
                    req("close_brace", t("RBrace")),
                ],
            ),
            node(
                "BlockStatement",
                vec![
                    req("item", cat("BlockItem")),
                    opt("local_declaration_semicolon", t("Semicolon")),
                ],
            ),
            node(
                "LocalVariableDeclaration",
                vec![
                    req("modifiers", n("ParameterModifierList")),
                    req("type", choice(vec![cat("Type"), kw("var")])).role("LocalVariableType"),
                    req("declarators", n("VariableDeclaratorList")),
                ],
            ),
            node(
                "LocalClassOrInterfaceDeclaration",
                vec![
                    req(
                        "declaration",
                        nodes(&[
                            "ClassDeclaration",
                            "RecordDeclaration",
                            "EnumDeclaration",
                            "InterfaceDeclaration",
                            "AnnotationInterfaceDeclaration",
                            "Bogus",
                        ]),
                    )
                    .role("LocalTypeDeclaration"),
                ],
            ),
            node("EmptyStatement", vec![req("semicolon", t("Semicolon"))]),
            node(
                "LabeledStatement",
                vec![
                    req("label", t("Identifier")),
                    req("colon", t("Colon")),
                    req("body", cat("Statement")),
                ],
            ),
            node(
                "ExpressionStatement",
                vec![
                    req("expression", cat("Expression")),
                    req("semicolon", t("Semicolon")),
                ],
            ),
            node(
                "IfStatement",
                vec![
                    req("if_keyword", t("IfKw")),
                    req("open_paren", t("LParen")),
                    req("condition", cat("Expression")),
                    req("close_paren", t("RParen")),
                    req("then_branch", cat("Statement")),
                    opt("else_keyword", t("ElseKw")),
                    opt("else_branch", cat("Statement")),
                ],
            ),
            node(
                "AssertStatement",
                vec![
                    req("assert_keyword", t("AssertKw")),
                    req("condition", cat("Expression")),
                    opt("colon", t("Colon")),
                    opt("message", cat("Expression")),
                    req("semicolon", t("Semicolon")),
                ],
            ),
            node(
                "SwitchStatement",
                vec![
                    req("switch_keyword", t("SwitchKw")),
                    req("open_paren", t("LParen")),
                    req("selector", cat("Expression")),
                    req("close_paren", t("RParen")),
                    req("body", n("SwitchBlock")),
                ],
            ),
            node(
                "SwitchBlock",
                vec![
                    req("open_brace", t("LBrace")),
                    req("entries", n("SwitchEntryList")),
                    req("close_brace", t("RBrace")),
                ],
            ),
            node(
                "SwitchBlockStatementGroup",
                vec![
                    req("labels", n("SwitchLabelColonList")),
                    opt("statements", n("BlockStatementList")),
                ],
            ),
            node(
                "SwitchRule",
                vec![
                    req("label", n("SwitchLabel")),
                    req("arrow", t("Arrow")),
                    req(
                        "body",
                        choice(vec![cat("Expression"), nodes(&["Block", "ThrowStatement"])]),
                    )
                    .role("SwitchRuleBody"),
                    opt("semicolon", t("Semicolon")),
                ],
            ),
            node(
                "SwitchLabel",
                vec![
                    req("keyword", tokens(&["CaseKw", "DefaultKw"])),
                    req("items", n("SwitchLabelItemList")),
                    opt("guard", cat("SwitchGuardSyntax")),
                ],
            ),
            node("CaseConstant", vec![req("expression", cat("Expression"))]),
            node("CasePattern", vec![req("pattern", cat("Pattern"))]),
            node(
                "Guard",
                vec![
                    req("when_keyword", kw("when")),
                    opt("open_paren", t("LParen")),
                    req("condition", cat("Expression")),
                    opt("close_paren", t("RParen")),
                ],
            ),
            node(
                "WhileStatement",
                vec![
                    req("while_keyword", t("WhileKw")),
                    req("open_paren", t("LParen")),
                    req("condition", cat("Expression")),
                    req("close_paren", t("RParen")),
                    req("body", cat("Statement")),
                ],
            ),
            node(
                "DoStatement",
                vec![
                    req("do_keyword", t("DoKw")),
                    req("body", cat("Statement")),
                    req("while_keyword", t("WhileKw")),
                    req("open_paren", t("LParen")),
                    req("condition", cat("Expression")),
                    req("close_paren", t("RParen")),
                    req("semicolon", t("Semicolon")),
                ],
            ),
            node(
                "ForStatement",
                vec![
                    req(
                        "form",
                        nodes(&["BasicForStatement", "EnhancedForStatement"]),
                    )
                    .role("ForStatementForm"),
                ],
            ),
            node(
                "BasicForStatement",
                vec![
                    req("for_keyword", t("ForKw")),
                    req("open_paren", t("LParen")),
                    opt("initializer", n("ForInitializer")),
                    req("first_semicolon", t("Semicolon")),
                    opt("condition", cat("Expression")),
                    req("second_semicolon", t("Semicolon")),
                    opt("update", n("ForUpdate")),
                    req("close_paren", t("RParen")),
                    req("body", cat("Statement")),
                ],
            ),
            node(
                "EnhancedForStatement",
                vec![
                    req("for_keyword", t("ForKw")),
                    req("open_paren", t("LParen")),
                    req("variable", cat("EnhancedForVariableSyntax")),
                    req("colon", t("Colon")),
                    req("iterable", cat("Expression")),
                    req("close_paren", t("RParen")),
                    req("body", cat("Statement")),
                ],
            ),
            node(
                "EnhancedForVariable",
                vec![
                    req("modifiers", n("ParameterModifierList")),
                    req("type", choice(vec![cat("Type"), kw("var")]))
                        .role("EnhancedForVariableType"),
                    req("name", tokens(&["Identifier", "UnderscoreKw"])),
                    opt("dimensions", n("ArrayDimensions")),
                ],
            ),
            node(
                "ForInitializer",
                vec![
                    req(
                        "value",
                        choice(vec![
                            n("LocalVariableDeclaration"),
                            n("StatementExpressionList"),
                        ]),
                    )
                    .role("ForInitializerValue"),
                ],
            ),
            node(
                "ForUpdate",
                vec![req("expressions", n("StatementExpressionList"))],
            ),
            list("StatementExpressionList", 1, cat("Expression"))
                .separated("Comma", Trailing::Forbidden),
            node(
                "BreakStatement",
                vec![
                    req("break_keyword", t("BreakKw")),
                    opt("label", t("Identifier")),
                    req("semicolon", t("Semicolon")),
                ],
            ),
            node(
                "YieldStatement",
                vec![
                    req("yield_keyword", kw("yield")),
                    req("expression", cat("Expression")),
                    req("semicolon", t("Semicolon")),
                ],
            ),
            node(
                "ContinueStatement",
                vec![
                    req("continue_keyword", t("ContinueKw")),
                    opt("label", t("Identifier")),
                    req("semicolon", t("Semicolon")),
                ],
            ),
            node(
                "ReturnStatement",
                vec![
                    req("return_keyword", t("ReturnKw")),
                    opt("expression", cat("Expression")),
                    req("semicolon", t("Semicolon")),
                ],
            ),
            node(
                "ThrowStatement",
                vec![
                    req("throw_keyword", t("ThrowKw")),
                    req("expression", cat("Expression")),
                    req("semicolon", t("Semicolon")),
                ],
            ),
            node(
                "SynchronizedStatement",
                vec![
                    req("synchronized_keyword", t("SynchronizedKw")),
                    req("open_paren", t("LParen")),
                    req("expression", cat("Expression")),
                    req("close_paren", t("RParen")),
                    req("body", n("Block")),
                ],
            ),
            node(
                "TryStatement",
                vec![
                    req("try_keyword", t("TryKw")),
                    req("body", n("Block")),
                    req("catches", n("CatchClauseList")),
                    opt("finally", n("FinallyClause")),
                ],
            ),
            node(
                "TryWithResourcesStatement",
                vec![
                    req("try_keyword", t("TryKw")),
                    req("resources", n("ResourceSpecification")),
                    req("body", n("Block")),
                    req("catches", n("CatchClauseList")),
                    opt("finally", n("FinallyClause")),
                ],
            ),
            node(
                "CatchClause",
                vec![
                    req("catch_keyword", t("CatchKw")),
                    req("open_paren", t("LParen")),
                    req("parameter", n("CatchParameter")),
                    req("close_paren", t("RParen")),
                    req("body", n("Block")),
                ],
            ),
            node(
                "CatchParameter",
                vec![
                    req("modifiers", n("ParameterModifierList")),
                    req("types", n("CatchTypeList")),
                    req("name", tokens(&["Identifier", "UnderscoreKw"])),
                    opt("dimensions", n("ArrayDimensions")),
                ],
            ),
            node(
                "CatchTypeList",
                vec![
                    req("types", nodes(&["ClassType", "UnionType", "Bogus"]))
                        .role("CatchParameterTypes"),
                ],
            ),
            node(
                "FinallyClause",
                vec![
                    req("finally_keyword", t("FinallyKw")),
                    req("body", n("Block")),
                ],
            ),
            node(
                "ResourceSpecification",
                vec![
                    req("open_paren", t("LParen")),
                    req("resources", n("ResourceList")),
                    opt("trailing_semicolon", t("Semicolon")),
                    req("close_paren", t("RParen")),
                ],
            ),
            list("ResourceList", 1, n("Resource")).separated("Semicolon", Trailing::Forbidden),
            node("Resource", vec![req("value", cat("ResourceValueSyntax"))]),
            node(
                "ResourceVariableDeclaration",
                vec![
                    req("modifiers", n("ParameterModifierList")),
                    req("type", choice(vec![cat("Type"), kw("var")])).role("ResourceVariableType"),
                    req("name", tokens(&["Identifier", "UnderscoreKw"])),
                    opt("dimensions", n("ArrayDimensions")),
                    req("assign", t("Assign")),
                    req("initializer", n("VariableInitializer")),
                ],
            ),
            node(
                "VariableAccess",
                vec![
                    req(
                        "expression",
                        nodes(&["NameExpression", "FieldAccessExpression"]),
                    )
                    .role("VariableAccessExpression"),
                ],
            ),
            node(
                "PrimitiveType",
                vec![
                    opt("annotations", n("AnnotationList")),
                    req(
                        "keyword",
                        tokens(&[
                            "BooleanKw",
                            "ByteKw",
                            "CharKw",
                            "DoubleKw",
                            "FloatKw",
                            "IntKw",
                            "LongKw",
                            "ShortKw",
                        ]),
                    ),
                ],
            ),
            node("VoidType", vec![req("void_keyword", t("VoidKw"))]),
            node(
                "ClassType",
                vec![req("segments", n("ClassTypeSegmentList"))],
            ),
            node(
                "ArrayType",
                vec![
                    req("element_type", nodes(&["PrimitiveType", "ClassType"]))
                        .role("ArrayElementType"),
                    req("dimensions", n("ArrayDimensions")),
                ],
            ),
            node(
                "IntersectionType",
                vec![
                    req("first_type", cat("Type")),
                    req("first_amp", t("Amp")),
                    req("remaining_types", n("TypeAmpList")),
                ],
            ),
            node(
                "UnionType",
                vec![
                    req("first_type", cat("Type")),
                    req("first_bar", t("Bar")),
                    req("remaining_types", n("TypeBarList")),
                ],
            ),
            node(
                "TypeArgumentList",
                vec![
                    req("open_angle", t("Lt")),
                    req("arguments", n("TypeArgumentSeparatedList")),
                    req("close_angle", t("Gt")),
                ],
            ),
            node(
                "TypeArgument",
                vec![
                    req("annotations", n("AnnotationList")),
                    req("type", cat("Type")),
                ],
            ),
            node(
                "WildcardType",
                vec![
                    req("question", t("Question")),
                    opt("bound_keyword", tokens(&["ExtendsKw", "SuperKw"])),
                    opt("bound", cat("Type")),
                ],
            ),
            list("ArrayDimensions", 1, n("ArrayDimension")),
            node(
                "ArrayDimension",
                vec![
                    req("annotations", n("AnnotationList")),
                    req("open_bracket", t("LBracket")),
                    req("close_bracket", t("RBracket")),
                ],
            ),
            node("Name", vec![req("identifier", t("Identifier"))]),
            node(
                "QualifiedName",
                vec![
                    req("first_segment", n("QualifiedNameSegmentNode")),
                    req("first_dot", t("Dot")),
                    req("remaining_segments", n("NameSegmentDotList")),
                ],
            ),
            node(
                "LiteralExpression",
                vec![req(
                    "literal",
                    tokens(&[
                        "IntegerLiteral",
                        "FloatingPointLiteral",
                        "BooleanLiteral",
                        "CharacterLiteral",
                        "StringLiteral",
                        "TextBlockLiteral",
                        "NullLiteral",
                    ]),
                )],
            ),
            node(
                "NameExpression",
                vec![
                    opt("annotations", n("AnnotationList")),
                    req("identifier", t("Identifier")),
                ],
            ),
            node(
                "ThisExpression",
                vec![
                    opt("qualifier", cat("Expression")),
                    opt("dot", t("Dot")),
                    req("this_keyword", t("ThisKw")),
                ],
            ),
            node(
                "SuperExpression",
                vec![
                    opt("qualifier", cat("Expression")),
                    opt("dot", t("Dot")),
                    req("super_keyword", t("SuperKw")),
                ],
            ),
            node(
                "ParenthesizedExpression",
                vec![
                    req("open_paren", t("LParen")),
                    req("expression", cat("Expression")),
                    req("close_paren", t("RParen")),
                ],
            ),
            node(
                "ClassLiteralExpression",
                vec![
                    req("target", cat("ClassLiteralTargetSyntax")),
                    opt("dimensions", n("ArrayDimensions")),
                    req("dot", t("Dot")),
                    req("class_keyword", t("ClassKw")),
                ],
            ),
            node(
                "FieldAccessExpression",
                vec![
                    req("receiver", cat("Expression")),
                    req("dot", t("Dot")),
                    req("name", t("Identifier")),
                    opt("type_arguments", n("TypeArgumentList")),
                ],
            ),
            node(
                "ArrayAccessExpression",
                vec![
                    req("array", cat("Expression")),
                    req("open_bracket", t("LBracket")),
                    req("index", cat("Expression")),
                    req("close_bracket", t("RBracket")),
                ],
            ),
            node(
                "MethodInvocationExpression",
                vec![req("form", cat("MethodInvocationFormSyntax"))],
            ),
            node(
                "MethodReferenceExpression",
                vec![
                    req("receiver", cat("MethodReferenceReceiverSyntax")),
                    opt("receiver_type_arguments", n("TypeArgumentList")),
                    req("double_colon", t("DoubleColon")),
                    opt("target_type_arguments", n("TypeArgumentList")),
                    req("target", tokens(&["Identifier", "NewKw"])),
                ],
            ),
            node(
                "ObjectCreationExpression",
                vec![
                    opt("qualifier", cat("Expression")),
                    opt("dot", t("Dot")),
                    req("new_keyword", t("NewKw")),
                    opt("constructor_type_arguments", n("TypeArgumentList")),
                    req("type", cat("ObjectCreationTypeSyntax")),
                    req("arguments", n("ArgumentList")),
                    opt("body", n("ClassBody")),
                ],
            ),
            node(
                "ArrayCreationExpression",
                vec![
                    req("new_keyword", t("NewKw")),
                    req("type", cat("ArrayCreationTypeSyntax")),
                    req("dimension_expressions", n("DimExpressionList")),
                    opt("dimensions", n("ArrayDimensions")),
                    opt("initializer", n("ArrayInitializer")),
                ],
            ),
            node(
                "DimExpression",
                vec![
                    req("annotations", n("AnnotationList")),
                    req("open_bracket", t("LBracket")),
                    req("expression", cat("Expression")),
                    req("close_bracket", t("RBracket")),
                ],
            ),
            node(
                "ArrayInitializer",
                vec![
                    req("open_brace", t("LBrace")),
                    req("values", n("VariableInitializerList")),
                    req("close_brace", t("RBrace")),
                ],
            ),
            node(
                "AssignmentExpression",
                vec![
                    req("left", cat("AssignmentTargetSyntax")),
                    req(
                        "operator",
                        tokens(&[
                            "Assign",
                            "PlusEq",
                            "MinusEq",
                            "StarEq",
                            "SlashEq",
                            "AmpEq",
                            "BarEq",
                            "CaretEq",
                            "PercentEq",
                            "LShiftEq",
                            "RShiftEq",
                            "UnsignedRShiftEq",
                        ]),
                    ),
                    req("right", cat("Expression")),
                ],
            ),
            node(
                "ConditionalExpression",
                vec![
                    req("condition", cat("Expression")),
                    req("question", t("Question")),
                    req("then_expression", cat("Expression")),
                    req("colon", t("Colon")),
                    req("else_expression", cat("Expression")),
                ],
            ),
            node(
                "InstanceofExpression",
                vec![
                    req("expression", cat("Expression")),
                    req("instanceof_keyword", t("InstanceofKw")),
                    req("target", cat("InstanceofTargetSyntax")),
                ],
            ),
            node(
                "BinaryExpression",
                vec![
                    req("left", cat("Expression")),
                    req(
                        "operator",
                        tokens(&[
                            "OrOr",
                            "AndAnd",
                            "Bar",
                            "Caret",
                            "Amp",
                            "EqEq",
                            "BangEq",
                            "Lt",
                            "Gt",
                            "LtEq",
                            "GtEq",
                            "LShift",
                            "RShift",
                            "UnsignedRShift",
                            "Plus",
                            "Minus",
                            "Star",
                            "Slash",
                            "Percent",
                        ]),
                    ),
                    req("right", cat("Expression")),
                ],
            ),
            node(
                "UnaryExpression",
                vec![
                    req(
                        "operator",
                        tokens(&["PlusPlus", "MinusMinus", "Plus", "Minus", "Tilde", "Bang"]),
                    ),
                    req("operand", cat("Expression")),
                ],
            ),
            node(
                "PostfixExpression",
                vec![
                    req("operand", cat("Expression")),
                    req("operator", tokens(&["PlusPlus", "MinusMinus"])),
                ],
            ),
            node(
                "CastExpression",
                vec![
                    req("open_paren", t("LParen")),
                    req("type", cat("Type")),
                    req("close_paren", t("RParen")),
                    req("expression", cat("Expression")),
                ],
            ),
            node(
                "LambdaExpression",
                vec![
                    opt("open_paren", t("LParen")),
                    req("parameters", n("LambdaParameterList")),
                    opt("close_paren", t("RParen")),
                    req("arrow", t("Arrow")),
                    req("body", cat("LambdaBodySyntax")),
                ],
            ),
            list("LambdaParameterList", 0, n("LambdaParameter"))
                .separated("Comma", Trailing::Forbidden),
            node(
                "LambdaParameter",
                vec![
                    req("modifiers", n("LambdaModifierList")),
                    opt("type", cat("Type")),
                    req("varargs_annotations", n("AnnotationList")),
                    opt("ellipsis", t("Ellipsis")),
                    req("name", tokens(&["Identifier", "UnderscoreKw"])),
                    opt("dimensions", n("ArrayDimensions")),
                ],
            ),
            node(
                "SwitchExpression",
                vec![
                    req("switch_keyword", t("SwitchKw")),
                    req("open_paren", t("LParen")),
                    req("selector", cat("Expression")),
                    req("close_paren", t("RParen")),
                    req("body", n("SwitchBlock")),
                ],
            ),
            node(
                "ArgumentList",
                vec![
                    req("open_paren", t("LParen")),
                    req("arguments", n("ExpressionList")),
                    req("close_paren", t("RParen")),
                ],
            ),
            node(
                "TypePattern",
                vec![
                    req("modifiers", n("ParameterModifierList")),
                    req(
                        "type",
                        nodes(&["PrimitiveType", "ClassType", "ArrayType", "Bogus"]),
                    )
                    .role("TypePatternType"),
                    req("name", tokens(&["Identifier", "UnderscoreKw"])),
                    opt("dimensions", n("ArrayDimensions")),
                ],
            ),
            node(
                "RecordPattern",
                vec![
                    req("type", nodes(&["ClassType", "Bogus"])).role("RecordPatternType"),
                    req("open_paren", t("LParen")),
                    req("components", n("ComponentPatternList")),
                    req("close_paren", t("RParen")),
                ],
            ),
            node("ComponentPattern", vec![req("pattern", cat("Pattern"))]),
            node(
                "MatchAllPattern",
                vec![req("underscore", t("UnderscoreKw"))],
            ),
            node(
                "NonSealedModifier",
                vec![
                    req("non_keyword", kw("non")),
                    req("minus", t("Minus")),
                    req("sealed_keyword", kw("sealed")),
                ],
            ),
            node(
                "ClassTypeSegmentNode",
                vec![
                    req("annotations", n("AnnotationList")),
                    req("name", cat("NameSyntax")),
                    opt("type_arguments", n("TypeArgumentList")),
                ],
            ),
            node(
                "QualifiedNameSegmentNode",
                vec![
                    req("annotations", n("AnnotationList")),
                    req("identifier", t("Identifier")),
                ],
            ),
            node(
                "QualifiedMethodInvocation",
                vec![
                    req("receiver", cat("Expression")),
                    req("dot", t("Dot")),
                    opt("type_arguments", n("TypeArgumentList")),
                    req("name", choice(vec![n("NameExpression"), t("Identifier")]))
                        .role("QualifiedInvocationName"),
                    req("arguments", n("ArgumentList")),
                ],
            ),
            node(
                "UnqualifiedMethodInvocation",
                vec![
                    opt("type_arguments", n("TypeArgumentList")),
                    req("name", choice(vec![n("NameExpression"), t("Identifier")]))
                        .role("UnqualifiedInvocationName"),
                    req("arguments", n("ArgumentList")),
                ],
            ),
            list("CompilationUnitItemList", 0, cat("CompilationUnitItem")),
            list("AnnotationList", 0, n("Annotation")),
            list("ModuleDirectiveList", 0, cat("ModuleDirective")),
            list(
                "RequiresModifierList",
                0,
                choice(vec![t("StaticKw"), kw("transitive")]),
            ),
            list("NameList", 0, cat("NameSyntax")).separated("Comma", Trailing::Forbidden),
            list("ModuleNameList", 1, cat("NameSyntax")).separated("Comma", Trailing::Forbidden),
            list(
                "AnnotationElementArgumentList",
                0,
                cat("AnnotationArgumentSyntax"),
            )
            .separated("Comma", Trailing::Forbidden),
            list(
                "AnnotationInterfaceBodyMemberList",
                0,
                cat("AnnotationInterfaceBodyMember"),
            ),
            list("AnnotationElementValueList", 0, n("AnnotationElementValue"))
                .separated("Comma", Trailing::Optional),
            list("TypeParameterSeparatedList", 0, n("TypeParameter"))
                .separated("Comma", Trailing::Forbidden),
            list("TypeList", 0, cat("Type")).separated("Comma", Trailing::Forbidden),
            list("ClassBodyMemberList", 0, cat("ClassBodyMember")),
            list("RecordBodyMemberList", 0, cat("ClassBodyMember")),
            list("InterfaceBodyMemberList", 0, cat("InterfaceBodyMember")),
            list(
                "ParameterModifierList",
                0,
                elements(&["Annotation", "Bogus", "FinalKw"]),
            )
            .role("ParameterModifier"),
            list("ConstructorBodyEntryList", 0, cat("ConstructorBodyEntry")),
            list("BlockStatementList", 0, n("BlockStatement")),
            list("SwitchEntryList", 0, cat("SwitchEntrySyntax")),
            list("SwitchLabelColonList", 0, n("SwitchLabel"))
                .separated("Colon", Trailing::Required),
            list(
                "SwitchLabelItemList",
                0,
                elements(&["CaseConstant", "CasePattern", "Bogus", "DefaultKw"]),
            )
            .role("SwitchLabelItem")
            .separated("Comma", Trailing::Forbidden),
            list("CatchClauseList", 0, n("CatchClause")),
            list("ClassTypeSegmentList", 0, n("ClassTypeSegmentNode"))
                .separated("Dot", Trailing::Forbidden),
            list("TypeAmpList", 0, cat("Type")).separated("Amp", Trailing::Forbidden),
            list("TypeBarList", 0, cat("Type")).separated("Bar", Trailing::Forbidden),
            list("TypeArgumentSeparatedList", 0, n("TypeArgument"))
                .separated("Comma", Trailing::Forbidden),
            list("NameSegmentDotList", 0, n("QualifiedNameSegmentNode"))
                .separated("Dot", Trailing::Forbidden),
            list("DimExpressionList", 0, n("DimExpression")),
            list(
                "VariableInitializerList",
                0,
                cat("VariableInitializerValue"),
            )
            .separated("Comma", Trailing::Optional),
            list(
                "LambdaModifierList",
                0,
                choice(vec![n("Annotation"), t("FinalKw"), kw("var")]),
            )
            .role("LambdaModifier"),
            list("ExpressionList", 0, cat("Expression")).separated("Comma", Trailing::Forbidden),
            list("ComponentPatternList", 0, n("ComponentPattern"))
                .separated("Comma", Trailing::Forbidden),
        ],
    }
}
