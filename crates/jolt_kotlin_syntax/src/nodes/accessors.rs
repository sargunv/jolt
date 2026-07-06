use super::{
    AnyKotlinNode, ClassBody, ClassDeclaration, ClassMember, CompanionObject, Declaration,
    DestructuringDeclaration, DestructuringEntry, DestructuringPatternEntry, Expression,
    FunctionDeclaration, ImportDirective, ImportList, InterfaceDeclaration, KotlinFile,
    KotlinFileItem, KotlinSyntaxKind, KotlinSyntaxToken, Name, ObjectDeclaration, PackageHeader,
    PropertyDeclaration, QualifiedName, StatementSyntax, StringTemplateExpression,
    StringTemplatePart, TypeAliasDeclaration, TypeArgumentList, TypeArgumentListEntry,
    TypeReference, TypeSyntax, ValueArgumentList, ValueArgumentListEntry, WhenConditionSyntax,
    WhenEntry, child, child_family, child_token, children, children_family,
};

impl<'source> KotlinFile<'source> {
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
    pub fn name(&self) -> Option<QualifiedName<'source>> {
        child(self.syntax())
    }

    #[must_use]
    pub fn alias(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }
}

impl<'source> ClassDeclaration<'source> {
    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }
}

impl<'source> InterfaceDeclaration<'source> {
    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }
}

impl<'source> ObjectDeclaration<'source> {
    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }
}

impl<'source> CompanionObject<'source> {
    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }
}

impl<'source> FunctionDeclaration<'source> {
    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }
}

impl<'source> PropertyDeclaration<'source> {
    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }
}

impl<'source> TypeAliasDeclaration<'source> {
    #[must_use]
    pub fn name(&self) -> Option<Name<'source>> {
        child(self.syntax())
    }
}

impl<'source> TypeReference<'source> {
    #[must_use]
    pub fn ty(&self) -> Option<TypeSyntax<'source>> {
        child_family(self.syntax())
    }
}

impl<'source> ClassBody<'source> {
    pub fn members(&self) -> impl Iterator<Item = ClassMember<'source>> + use<'source> {
        children_family(self.syntax())
    }
}

impl<'source> ValueArgumentList<'source> {
    pub fn entries(&self) -> impl Iterator<Item = ValueArgumentListEntry<'source>> + use<'source> {
        children_family(self.syntax())
    }
}

impl<'source> TypeArgumentList<'source> {
    pub fn entries(&self) -> impl Iterator<Item = TypeArgumentListEntry<'source>> + use<'source> {
        children_family(self.syntax())
    }
}

impl<'source> StringTemplateExpression<'source> {
    pub fn parts(&self) -> impl Iterator<Item = StringTemplatePart<'source>> + use<'source> {
        children_family(self.syntax())
    }
}

impl<'source> WhenEntry<'source> {
    pub fn conditions(&self) -> impl Iterator<Item = WhenConditionSyntax<'source>> + use<'source> {
        children_family(self.syntax())
    }

    pub fn expressions(&self) -> impl Iterator<Item = Expression<'source>> + use<'source> {
        children_family(self.syntax())
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

impl<'source> StatementSyntax<'source> {
    pub fn expressions(&self) -> impl Iterator<Item = Expression<'source>> + use<'source> {
        children_family(self.syntax())
    }
}
