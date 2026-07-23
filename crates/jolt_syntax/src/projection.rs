//! Language-neutral typed CST projection declarations.

/// Defines the shared syntax-field representation used by a language's typed
/// CST projection.
#[doc(hidden)]
#[macro_export]
macro_rules! define_typed_cst_fields {
    (
        language: $language:ty,
        syntax_kind: $syntax_kind:ident,
        syntax_node: $syntax_node:ident,
        invariant_error: $invariant_error:ident,
        syntax_result: $syntax_result:ident,
        syntax_field: $syntax_field:ident,
        malformed_syntax: $malformed_syntax:ident,
        missing_syntax: $missing_syntax:ident,
        fixed_syntax: $fixed_syntax:ident,
    ) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub struct $invariant_error {
            pub node: $syntax_kind,
            pub slot: usize,
        }

        impl fmt::Display for $invariant_error {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    formatter,
                    "{:?} has an invalid element in slot {}",
                    self.node, self.slot
                )
            }
        }

        impl std::error::Error for $invariant_error {}

        type $syntax_result<T> = Result<T, $invariant_error>;

        /// A declared grammar role, including represented malformed alternatives.
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum $syntax_field<'source, T> {
            Present(T),
            Missing($missing_syntax<'source>),
            Malformed($malformed_syntax<'source>),
        }

        impl<'source, T> $syntax_field<'source, T> {
            pub fn as_ref(&self) -> $syntax_field<'source, &T> {
                match self {
                    Self::Present(value) => $syntax_field::Present(value),
                    Self::Missing(missing) => $syntax_field::Missing(*missing),
                    Self::Malformed(node) => $syntax_field::Malformed(*node),
                }
            }

            pub fn map<U>(self, map: impl FnOnce(T) -> U) -> $syntax_field<'source, U> {
                match self {
                    Self::Present(value) => $syntax_field::Present(map(value)),
                    Self::Missing(missing) => $syntax_field::Missing(missing),
                    Self::Malformed(node) => $syntax_field::Malformed(node),
                }
            }
        }

        /// A syntax-owned malformed node occupying a declared role.
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub struct $malformed_syntax<'source> {
            syntax: $syntax_node<'source>,
        }

        /// Syntax-owned evidence for one represented empty required or optional slot.
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub struct $missing_syntax<'source> {
            owner: $syntax_node<'source>,
            slot: usize,
        }

        impl<'source> $missing_syntax<'source> {
            /// Returns the exact zero-width source boundary represented by this missing slot.
            pub fn verbatim_core(
                self,
            ) -> Result<SyntaxVerbatimCore<'source, $language>, $invariant_error> {
                self.owner
                    .missing_verbatim_core(self.slot)
                    .ok_or($invariant_error {
                        node: self.owner.kind(),
                        slot: self.slot,
                    })
            }
        }

        #[derive(Clone, Copy)]
        struct $fixed_syntax<'source>($syntax_node<'source>);

        impl<'source> $fixed_syntax<'source> {
            #[inline]
            fn kind(self) -> $syntax_kind {
                self.0.kind()
            }

            #[inline]
            fn slot_at(self, slot: usize) -> Option<SyntaxSlot<'source, $language>> {
                self.0.slot_at(slot)
            }

            #[inline]
            fn missing_owner(self) -> $syntax_node<'source> {
                self.0
            }

            #[inline]
            fn text_range(self) -> TextRange {
                self.0.text_range()
            }

            #[inline]
            fn source(self) -> &'source str {
                self.0.source()
            }
        }

        #[inline]
        fn required_slot(
            syntax: $fixed_syntax<'_>,
            slot: usize,
        ) -> $syntax_field<'_, SyntaxElement<'_, $language>> {
            match syntax.slot_at(slot) {
                Some(SyntaxSlot::Node(node)) if node.is_directly_malformed() => {
                    $syntax_field::Malformed($malformed_syntax { syntax: node })
                }
                Some(SyntaxSlot::Node(node)) => {
                    $syntax_field::Present(SyntaxElement::Node(node))
                }
                Some(SyntaxSlot::Token(token)) => {
                    $syntax_field::Present(SyntaxElement::Token(token))
                }
                Some(SyntaxSlot::Empty) => $syntax_field::Missing($missing_syntax {
                    owner: syntax.missing_owner(),
                    slot,
                }),
                None => invalid_physical_projection(syntax, slot),
            }
        }

        #[inline]
        fn optional_slot(
            syntax: $fixed_syntax<'_>,
            slot: usize,
        ) -> $syntax_field<'_, SyntaxElement<'_, $language>> {
            required_slot(syntax, slot)
        }

        /// Generated accessors trust nodes produced by the schema-derived syntax factory.
        /// A mismatch is a lowering bug or a value from a doc-hidden custom factory/tree sink.
        #[cold]
        #[track_caller]
        fn invalid_physical_projection<T>(syntax: $fixed_syntax<'_>, slot: usize) -> T {
            panic!(
                "trusted syntax factory produced an invalid physical projection for {:?} slot {slot}",
                syntax.kind(),
            )
        }
    };
}

/// Defines traits and constant-time access helpers shared by typed CST views.
#[doc(hidden)]
#[macro_export]
macro_rules! define_typed_cst_access {
    (
        language: $language:ty,
        syntax_kind: $syntax_kind:ident,
        syntax_node: $syntax_node:ident,
        syntax_token: $syntax_token:ident,
        invariant_error: $invariant_error:ident,
        syntax_result: $syntax_result:ident,
        syntax_field: $syntax_field:ident,
        malformed_syntax: $malformed_syntax:ident,
        missing_syntax: $missing_syntax:ident,
        fixed_syntax: $fixed_syntax:ident,
        syntax_view: $syntax_view:ident,
        typed_node: $typed_node:ident,
        node: $node:ident,
        family: $family:ident,
        role_element: $role_element:ident,
        list_item: $list_item:ident,
        list_part: $list_part:ident,
        category_bogus: $category_bogus:ident,
    ) => {
        mod private {
            pub trait Sealed {}
        }

        /// Sealed access to behavior shared by every typed syntax view.
        pub trait $syntax_view<'source>: private::Sealed {
            fn syntax_node(&self) -> Option<$syntax_node<'source>>;

            fn first_token(&self) -> Option<$syntax_token<'source>> {
                self.syntax_node().and_then(|syntax| syntax.first_token())
            }

            #[must_use]
            fn is_recovery_free(&self) -> bool {
                self.syntax_node()
                    .is_none_or(|syntax| syntax.is_recovery_free())
            }

            #[must_use]
            fn malformed_verbatim_core(&self) -> Option<SyntaxVerbatimCore<'source, $language>> {
                self.syntax_node()
                    .and_then(|syntax| syntax.malformed_verbatim_core())
            }

            #[must_use]
            fn starts_after_blank_line(&self) -> bool {
                self.first_token()
                    .is_some_and(|token| token.has_leading_blank_line())
            }
        }

        impl private::Sealed for $malformed_syntax<'_> {}

        impl<'source> $syntax_view<'source> for $malformed_syntax<'source> {
            fn syntax_node(&self) -> Option<$syntax_node<'source>> {
                Some(self.syntax)
            }
        }

        pub trait $typed_node<'source>: Clone + private::Sealed {
            #[doc(hidden)]
            fn cast_element(element: $role_element<'source>) -> Option<Self>;
        }

        pub trait $node<'source>: $typed_node<'source> {
            fn cast(syntax: $syntax_node<'source>) -> Option<Self>;
        }

        pub trait $family<'source>: Clone + private::Sealed {
            fn cast(syntax: $syntax_node<'source>) -> Option<Self>;
        }

        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum $role_element<'source> {
            Node($syntax_node<'source>),
            Token($syntax_token<'source>),
        }

        impl<'source> $role_element<'source> {
            #[inline]
            fn kind(self) -> $syntax_kind {
                match self {
                    Self::Node(node) => node.kind(),
                    Self::Token(token) => token.kind(),
                }
            }

            #[must_use]
            pub fn token(self) -> Option<$syntax_token<'source>> {
                match self {
                    Self::Token(token) => Some(token),
                    Self::Node(_) => None,
                }
            }

            #[must_use]
            pub fn cast_node<N: $typed_node<'source>>(self) -> Option<N> {
                N::cast_element(self)
            }

            #[must_use]
            pub fn cast_family<F: $family<'source>>(self) -> Option<F> {
                match self {
                    Self::Node(node) => F::cast(node),
                    Self::Token(_) => None,
                }
            }
        }

        trait $list_item<'source>: Sized {
            fn cast_element(element: $role_element<'source>) -> Option<Self>;
        }

        impl<'source> $list_item<'source> for $role_element<'source> {
            fn cast_element(element: $role_element<'source>) -> Option<Self> {
                Some(element)
            }
        }

        /// One represented part of a variable-length syntax-list node.
        #[derive(Clone, Copy, Debug)]
        pub enum $list_part<'source, T> {
            Item(T),
            Separator($syntax_token<'source>),
            Missing($missing_syntax<'source>),
            Malformed($malformed_syntax<'source>),
        }

        fn list_parts<'source, T: $list_item<'source>>(
            syntax: $syntax_node<'source>,
            separated: bool,
        ) -> impl Iterator<Item = $list_part<'source, T>> + use<'source, T> {
            (0..syntax.slot_count()).map(move |index| {
                let Some(slot) = syntax.slot_at(index) else {
                    return invalid_physical_projection($fixed_syntax(syntax), index);
                };
                match slot {
                    SyntaxSlot::Token(token) if separated && index % 2 == 1 => {
                        $list_part::Separator(token)
                    }
                    SyntaxSlot::Node(node) => T::cast_element($role_element::Node(node))
                        .filter(|_| !node.is_directly_malformed() || $category_bogus(node.kind()))
                        .map($list_part::Item)
                        .unwrap_or_else(|| {
                            if node.is_directly_malformed() {
                                $list_part::Malformed($malformed_syntax { syntax: node })
                            } else {
                                invalid_physical_projection($fixed_syntax(syntax), index)
                            }
                        }),
                    SyntaxSlot::Token(token) => T::cast_element($role_element::Token(token))
                        .map($list_part::Item)
                        .unwrap_or_else(|| {
                            invalid_physical_projection($fixed_syntax(syntax), index)
                        }),
                    SyntaxSlot::Empty => $list_part::Missing($missing_syntax {
                        owner: syntax,
                        slot: index,
                    }),
                }
            })
        }

        #[inline]
        fn required_token(
            syntax: $fixed_syntax<'_>,
            slot: usize,
        ) -> $syntax_field<'_, $syntax_token<'_>> {
            match required_slot(syntax, slot) {
                $syntax_field::Present(SyntaxElement::Token(token)) => {
                    $syntax_field::Present(token)
                }
                $syntax_field::Missing(missing) => $syntax_field::Missing(missing),
                $syntax_field::Malformed(node) => $syntax_field::Malformed(node),
                $syntax_field::Present(SyntaxElement::Node(_)) => {
                    invalid_physical_projection(syntax, slot)
                }
            }
        }

        #[inline]
        fn optional_token(
            syntax: $fixed_syntax<'_>,
            slot: usize,
        ) -> $syntax_field<'_, $syntax_token<'_>> {
            required_token(syntax, slot)
        }

        #[inline]
        fn required_node<'source, N: $node<'source>>(
            syntax: $fixed_syntax<'source>,
            slot: usize,
        ) -> $syntax_field<'source, N> {
            match required_slot(syntax, slot) {
                $syntax_field::Present(SyntaxElement::Node(node)) => N::cast(node)
                    .map($syntax_field::Present)
                    .unwrap_or_else(|| invalid_physical_projection(syntax, slot)),
                $syntax_field::Missing(missing) => $syntax_field::Missing(missing),
                $syntax_field::Malformed(node) => $syntax_field::Malformed(node),
                $syntax_field::Present(SyntaxElement::Token(_)) => {
                    invalid_physical_projection(syntax, slot)
                }
            }
        }

        #[inline]
        fn optional_node<'source, N: $node<'source>>(
            syntax: $fixed_syntax<'source>,
            slot: usize,
        ) -> $syntax_field<'source, N> {
            required_node(syntax, slot)
        }

        #[inline]
        fn required_role_element(
            syntax: $fixed_syntax<'_>,
            slot: usize,
        ) -> $syntax_field<'_, $role_element<'_>> {
            match required_slot(syntax, slot) {
                $syntax_field::Present(SyntaxElement::Node(node)) => {
                    $syntax_field::Present($role_element::Node(node))
                }
                $syntax_field::Present(SyntaxElement::Token(token)) => {
                    $syntax_field::Present($role_element::Token(token))
                }
                $syntax_field::Missing(missing) => $syntax_field::Missing(missing),
                $syntax_field::Malformed(node) => $syntax_field::Malformed(node),
            }
        }

        #[inline]
        fn required_family<'source, F: $family<'source>>(
            syntax: $fixed_syntax<'source>,
            slot: usize,
        ) -> $syntax_field<'source, F> {
            match syntax.slot_at(slot) {
                Some(SyntaxSlot::Node(node)) => match F::cast(node) {
                    Some(value)
                        if !node.is_directly_malformed() || $category_bogus(node.kind()) =>
                    {
                        $syntax_field::Present(value)
                    }
                    Some(_) => $syntax_field::Malformed($malformed_syntax { syntax: node }),
                    None if node.is_directly_malformed() => {
                        $syntax_field::Malformed($malformed_syntax { syntax: node })
                    }
                    None => invalid_physical_projection(syntax, slot),
                },
                Some(SyntaxSlot::Empty) => $syntax_field::Missing($missing_syntax {
                    owner: syntax.missing_owner(),
                    slot,
                }),
                Some(SyntaxSlot::Token(_)) | None => invalid_physical_projection(syntax, slot),
            }
        }

        #[inline]
        fn optional_family<'source, F: $family<'source>>(
            syntax: $fixed_syntax<'source>,
            slot: usize,
        ) -> $syntax_field<'source, F> {
            required_family(syntax, slot)
        }
    };
}

/// Defines the typed node wrappers and category enums projected from one
/// language's syntax schema.
///
/// The language crate retains ownership of its schema, traits, and public type
/// names. This macro only centralizes the mechanical projection shared by all
/// languages.
#[doc(hidden)]
#[macro_export]
macro_rules! define_typed_cst_projection {
    (
        syntax_node: $syntax_node:ident,
        syntax_token: $syntax_token:ident,
        syntax_kind: $syntax_kind:ident,
        fixed_syntax: $fixed_syntax:ident,
        role_element: $role_element:ident,
        node_trait: $node_trait:ident,
        typed_node_trait: $typed_node_trait:ident,
        family_trait: $family_trait:ident,
        list_item_trait: $list_item_trait:ident,
        syntax_view_trait: $syntax_view_trait:ident,
        any_node: { $($any_node:ident)? },
        nodes {
            $($node:ident => $kind:ident [$class:ident],)*
        }
        enums {
            $(
                $family:ident = $($variant:ident)|+;
            )*
        }
    ) => {
        $(
            #[derive(Clone, Copy, Eq, PartialEq)]
            pub struct $node<'source> {
                syntax: $syntax_node<'source>,
            }

            impl<'source> $node<'source> {
                #[inline]
                fn fixed_syntax(&self) -> $fixed_syntax<'source> {
                    $fixed_syntax(self.syntax)
                }

                #[must_use]
                pub fn kind(&self) -> $syntax_kind {
                    self.syntax.kind()
                }

                #[must_use]
                pub fn text_range(&self) -> TextRange {
                    self.syntax.text_range()
                }

                #[must_use]
                pub fn source_text(&self) -> &'source str {
                    let syntax = self.fixed_syntax();
                    let range = syntax.text_range();
                    &syntax.source()[range.start().get()..range.end().get()]
                }

                pub fn token_iter(&self) -> impl Iterator<Item = $syntax_token<'source>> + '_ {
                    self.syntax.tokens()
                }

                #[must_use]
                pub fn first_token(&self) -> Option<$syntax_token<'source>> {
                    self.syntax.first_token()
                }

                #[must_use]
                pub fn last_token(&self) -> Option<$syntax_token<'source>> {
                    self.syntax.last_token()
                }
            }

            impl private::Sealed for $node<'_> {}

            impl<'source> $syntax_view_trait<'source> for $node<'source> {
                fn syntax_node(&self) -> Option<$syntax_node<'source>> {
                    Some(self.syntax)
                }
            }

            impl<'source> $node_trait<'source> for $node<'source> {
                fn cast(syntax: $syntax_node<'source>) -> Option<Self> {
                    (matches!(syntax.kind(), $syntax_kind::$kind)
                        && $crate::__typed_cst_class_accepts!($class, syntax))
                    .then_some(Self { syntax })
                }
            }

            impl<'source> $typed_node_trait<'source> for $node<'source> {
                fn cast_element(element: $role_element<'source>) -> Option<Self> {
                    match element {
                        $role_element::Node(node) => Self::cast(node),
                        $role_element::Token(_) => None,
                    }
                }
            }

            impl<'source> $list_item_trait<'source> for $node<'source> {
                fn cast_element(element: $role_element<'source>) -> Option<Self> {
                    <Self as $typed_node_trait<'source>>::cast_element(element)
                }
            }

            impl fmt::Debug for $node<'_> {
                fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    self.syntax.fmt(formatter)
                }
            }
        )*

        $crate::__define_any_typed_node! {
            [$($any_node)?]
            syntax_node: $syntax_node,
            syntax_kind: $syntax_kind,
            node_trait: $node_trait,
            nodes { $($node => $kind,)* }
        }

        $(
            #[derive(Clone, Copy, Debug, Eq, PartialEq)]
            pub enum $family<'source> {
                $($variant($variant<'source>),)+
            }

            impl<'source> $family<'source> {
                #[must_use]
                pub fn kind(&self) -> $syntax_kind {
                    self.syntax().kind()
                }

                #[must_use]
                pub fn text_range(&self) -> TextRange {
                    self.syntax().text_range()
                }

                #[must_use]
                pub fn source_text(&self) -> &'source str {
                    let syntax = $fixed_syntax(*self.syntax());
                    let range = syntax.text_range();
                    &syntax.source()[range.start().get()..range.end().get()]
                }

                pub fn token_iter(&self) -> impl Iterator<Item = $syntax_token<'source>> + '_ {
                    self.syntax().tokens()
                }

                #[must_use]
                pub fn first_token(&self) -> Option<$syntax_token<'source>> {
                    self.syntax().first_token()
                }

                #[must_use]
                pub fn last_token(&self) -> Option<$syntax_token<'source>> {
                    self.syntax().last_token()
                }

                pub(crate) fn syntax(&self) -> &$syntax_node<'source> {
                    match self {
                        $(Self::$variant(node) => &node.syntax,)+
                    }
                }
            }

            impl<'source> $family_trait<'source> for $family<'source> {
                fn cast(syntax: $syntax_node<'source>) -> Option<Self> {
                    match syntax.kind() {
                        $(
                            $syntax_kind::$variant => {
                                <$variant<'source> as $node_trait<'source>>::cast(syntax)
                                    .map(Self::$variant)
                            }
                        )+
                        _ => None,
                    }
                }
            }

            impl<'source> $list_item_trait<'source> for $family<'source> {
                fn cast_element(element: $role_element<'source>) -> Option<Self> {
                    match element {
                        $role_element::Node(node) => Self::cast(node),
                        $role_element::Token(_) => None,
                    }
                }
            }

            impl private::Sealed for $family<'_> {}

            impl<'source> $syntax_view_trait<'source> for $family<'source> {
                fn syntax_node(&self) -> Option<$syntax_node<'source>> {
                    Some(*self.syntax())
                }
            }

            $(
                impl<'source> From<$variant<'source>> for $family<'source> {
                    fn from(node: $variant<'source>) -> Self {
                        Self::$variant(node)
                    }
                }
            )+
        )*
    };
}

/// Applies the schema node class to typed-wrapper ownership.
#[doc(hidden)]
#[macro_export]
macro_rules! __typed_cst_class_accepts {
    (valid, $syntax:ident) => {
        !$syntax.is_directly_malformed()
    };
    (list, $syntax:ident) => {
        !$syntax.is_directly_malformed()
    };
    (malformed, $syntax:ident) => {
        $syntax.is_directly_malformed()
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __define_any_typed_node {
    (
        []
        syntax_node: $syntax_node:ident,
        syntax_kind: $syntax_kind:ident,
        node_trait: $node_trait:ident,
        nodes { $($nodes:tt)* }
    ) => {};
    (
        [$any_node:ident]
        syntax_node: $syntax_node:ident,
        syntax_kind: $syntax_kind:ident,
        node_trait: $node_trait:ident,
        nodes { $($node:ident => $kind:ident,)* }
    ) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum $any_node<'source> {
            $($node($node<'source>),)*
        }

        impl<'source> $any_node<'source> {
            #[must_use]
            pub fn cast(syntax: $syntax_node<'source>) -> Option<Self> {
                match syntax.kind() {
                    $(
                        $syntax_kind::$kind => {
                            <$node<'source> as $node_trait<'source>>::cast(syntax).map(Self::$node)
                        }
                    )*
                    _ => None,
                }
            }
        }

        $(
            impl<'source> From<$node<'source>> for $any_node<'source> {
                fn from(node: $node<'source>) -> Self {
                    Self::$node(node)
                }
            }
        )*
    };
}
