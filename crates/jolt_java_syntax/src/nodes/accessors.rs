use super::{
    Annotation, AnnotationArgumentList, AnnotationInterfaceBody, AnnotationInterfaceDeclaration,
    AnyJavaNode, ArgumentList, ArrayDimensions, ArrayType, Block, BlockItem, BlockStatement,
    ClassBody, ClassDeclaration, CompilationUnit, EnumBody, EnumDeclaration, Expression,
    FormalParameterList, IfStatement, ImportDeclaration, InterfaceBody, InterfaceDeclaration,
    JavaSyntaxKind, JavaSyntaxToken, MethodDeclaration, MethodInvocationExpression,
    ModuleDeclaration, ModuleDirective, ModuleDirectiveNode, NameSyntax, PackageDeclaration,
    RecordBody, RecordDeclaration, Statement, TypeDeclaration, child, child_family, child_token,
    children, children_family, nth_child_family, nth_child_token,
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
}

impl ClassDeclaration {
    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn body(&self) -> Option<ClassBody> {
        child(&self.syntax)
    }
}

impl RecordDeclaration {
    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        nth_child_token(&self.syntax, JavaSyntaxKind::Identifier, 1)
    }

    #[must_use]
    pub fn body(&self) -> Option<RecordBody> {
        child(&self.syntax)
    }
}

impl EnumDeclaration {
    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn body(&self) -> Option<EnumBody> {
        child(&self.syntax)
    }
}

impl InterfaceDeclaration {
    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn body(&self) -> Option<InterfaceBody> {
        child(&self.syntax)
    }
}

impl AnnotationInterfaceDeclaration {
    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn body(&self) -> Option<AnnotationInterfaceBody> {
        child(&self.syntax)
    }
}

impl MethodDeclaration {
    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn parameters(&self) -> Option<FormalParameterList> {
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
