use super::super::{
    Annotation, AnyJavaNode, CompilationUnit, EmptyDeclaration, ExportsDirective, FieldDeclaration,
    ImportDeclaration, JavaFamily, JavaNode, JavaSyntaxKind, JavaSyntaxToken, MethodDeclaration,
    ModuleDeclaration, ModuleDirective, ModuleDirectiveNode, NameSyntax, OpensDirective,
    PackageDeclaration, ProvidesDirective, RequiresDirective, TypeDeclaration, UsesDirective,
    child, child_family, child_token, children, children_family, children_tokens_matching,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompilationUnitMember {
    EmptyDeclaration(EmptyDeclaration),
    FieldDeclaration(FieldDeclaration),
    MethodDeclaration(MethodDeclaration),
    TypeDeclaration(TypeDeclaration),
}

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

    pub fn compact_members(&self) -> impl Iterator<Item = CompilationUnitMember> + '_ {
        self.syntax.children().filter_map(|node| {
            EmptyDeclaration::cast(node.clone())
                .map(CompilationUnitMember::EmptyDeclaration)
                .or_else(|| {
                    FieldDeclaration::cast(node.clone())
                        .map(CompilationUnitMember::FieldDeclaration)
                })
                .or_else(|| {
                    MethodDeclaration::cast(node.clone())
                        .map(CompilationUnitMember::MethodDeclaration)
                })
                .or_else(|| TypeDeclaration::cast(node).map(CompilationUnitMember::TypeDeclaration))
        })
    }

    pub fn unsupported_layout_child(&self) -> Option<AnyJavaNode> {
        self.syntax
            .children()
            .filter_map(AnyJavaNode::cast)
            .find(|node| {
                !matches!(
                    node.kind(),
                    JavaSyntaxKind::PackageDeclaration
                        | JavaSyntaxKind::ImportDeclaration
                        | JavaSyntaxKind::ModuleDeclaration
                        | JavaSyntaxKind::ClassDeclaration
                        | JavaSyntaxKind::RecordDeclaration
                        | JavaSyntaxKind::EnumDeclaration
                        | JavaSyntaxKind::InterfaceDeclaration
                        | JavaSyntaxKind::AnnotationInterfaceDeclaration
                        | JavaSyntaxKind::EmptyDeclaration
                        | JavaSyntaxKind::FieldDeclaration
                        | JavaSyntaxKind::MethodDeclaration
                )
            })
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
    pub fn is_static(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::StaticKw).is_some()
    }

    #[must_use]
    pub fn is_module(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .filter_map(jolt_syntax::SyntaxElement::into_token)
            .nth(1)
            .is_some_and(|token| {
                token.kind() == JavaSyntaxKind::Identifier && token.text() == "module"
            })
    }

    #[must_use]
    pub fn is_on_demand(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::Star).is_some()
    }

    #[must_use]
    pub fn name(&self) -> Option<NameSyntax> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let mut elements = self.syntax.children_with_tokens();
        let Some(import_kw) = elements
            .next()
            .and_then(jolt_syntax::SyntaxElement::into_token)
        else {
            return false;
        };
        if import_kw.kind() != JavaSyntaxKind::ImportKw {
            return false;
        }

        let mut next = elements.next();
        if self.is_module() {
            let Some(module) = next.and_then(jolt_syntax::SyntaxElement::into_token) else {
                return false;
            };
            if module.kind() != JavaSyntaxKind::Identifier || module.text() != "module" {
                return false;
            }
            next = elements.next();
        } else if self.is_static() {
            let Some(static_kw) = next.and_then(jolt_syntax::SyntaxElement::into_token) else {
                return false;
            };
            if static_kw.kind() != JavaSyntaxKind::StaticKw {
                return false;
            }
            next = elements.next();
        }

        let Some(name) = next.and_then(jolt_syntax::SyntaxElement::into_node) else {
            return false;
        };
        if !NameSyntax::can_cast(name.kind()) {
            return false;
        }

        next = elements.next();
        if self.is_on_demand() {
            let Some(dot) = next.and_then(jolt_syntax::SyntaxElement::into_token) else {
                return false;
            };
            if dot.kind() != JavaSyntaxKind::Dot {
                return false;
            }
            let Some(star) = elements
                .next()
                .and_then(jolt_syntax::SyntaxElement::into_token)
            else {
                return false;
            };
            if star.kind() != JavaSyntaxKind::Star {
                return false;
            }
            next = elements.next();
        }

        let Some(semicolon) = next.and_then(jolt_syntax::SyntaxElement::into_token) else {
            return false;
        };
        semicolon.kind() == JavaSyntaxKind::Semicolon && elements.next().is_none()
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
}
impl ModuleDeclaration {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn is_open(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .filter_map(jolt_syntax::SyntaxElement::into_token)
            .any(|token| token.kind() == JavaSyntaxKind::Identifier && token.text() == "open")
    }

    #[must_use]
    pub fn name(&self) -> Option<NameSyntax> {
        child_family(&self.syntax)
    }

    pub fn directives(&self) -> impl Iterator<Item = ModuleDirective> + '_ {
        children::<ModuleDirectiveNode>(&self.syntax).map(|node| {
            node.directive()
                .expect("parser-clean module directive node should have a directive child")
        })
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let mut index = 0;

        while elements
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::Annotation)
        {
            index += 1;
        }

        if elements
            .get(index)
            .and_then(|element| element.clone().into_token())
            .is_some_and(|token| {
                token.kind() == JavaSyntaxKind::Identifier && token.text() == "open"
            })
        {
            index += 1;
        }

        let Some(module_kw) = elements
            .get(index)
            .and_then(|element| element.clone().into_token())
        else {
            return false;
        };
        if module_kw.kind() != JavaSyntaxKind::Identifier || module_kw.text() != "module" {
            return false;
        }
        index += 1;

        if !elements
            .get(index)
            .is_some_and(|element| NameSyntax::can_cast(element.kind()))
        {
            return false;
        }
        index += 1;

        if !elements
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::LBrace)
        {
            return false;
        }
        index += 1;

        while elements
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::ModuleDirective)
        {
            index += 1;
        }

        elements
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::RBrace)
            && index + 1 == elements.len()
    }
}

impl ModuleDirectiveNode {
    #[must_use]
    pub fn directive(&self) -> Option<ModuleDirective> {
        child_family(&self.syntax)
    }
}

impl RequiresDirective {
    #[must_use]
    pub fn is_transitive(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .filter_map(jolt_syntax::SyntaxElement::into_token)
            .any(|token| token.kind() == JavaSyntaxKind::Identifier && token.text() == "transitive")
    }

    #[must_use]
    pub fn is_static(&self) -> bool {
        child_token(&self.syntax, JavaSyntaxKind::StaticKw).is_some()
    }

    #[must_use]
    pub fn name(&self) -> Option<NameSyntax> {
        child_family(&self.syntax)
    }
}

impl ExportsDirective {
    #[must_use]
    pub fn package_name(&self) -> Option<NameSyntax> {
        child_family(&self.syntax)
    }

    pub fn target_modules(&self) -> impl Iterator<Item = NameSyntax> + '_ {
        children_family(&self.syntax).skip(1)
    }
}

impl OpensDirective {
    #[must_use]
    pub fn package_name(&self) -> Option<NameSyntax> {
        child_family(&self.syntax)
    }

    pub fn target_modules(&self) -> impl Iterator<Item = NameSyntax> + '_ {
        children_family(&self.syntax).skip(1)
    }
}

impl UsesDirective {
    #[must_use]
    pub fn service_name(&self) -> Option<NameSyntax> {
        child_family(&self.syntax)
    }
}

impl ProvidesDirective {
    #[must_use]
    pub fn service_name(&self) -> Option<NameSyntax> {
        child_family(&self.syntax)
    }

    pub fn implementation_names(&self) -> impl Iterator<Item = NameSyntax> + '_ {
        children_family(&self.syntax).skip(1)
    }
}
