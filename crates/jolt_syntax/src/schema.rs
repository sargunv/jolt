//! Shared declarative syntax-shape metadata.
//!
//! This module is public only so language syntax crates can lower their
//! declarative schemas into one production-consumable representation. It is
//! not a public API for formatter consumers.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cardinality {
    Required,
    Optional,
    Many,
    OneOrMore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disambiguation {
    None,
    LongestThenFirst,
    LeftmostLongest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrailingSeparator {
    Forbidden,
    Optional,
    Required,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Recovery {
    BogusOwner,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Matcher<K: 'static, C: 'static> {
    Token(K),
    TokenSet(&'static [K]),
    ElementSet(&'static [K]),
    Contextual {
        kind: K,
        text: &'static str,
    },
    Node(K),
    /// One fixed parent slot containing a node constructed from the declared
    /// compact child shape during the flat-tree migration.
    Constructed(K),
    /// One fixed parent slot containing a syntax-list node. During the compact
    /// tree migration, audits expand the list node's declared child shape in
    /// place; the production factory stores the constructed node in one slot.
    List(K),
    NodeSet(&'static [K]),
    Category(C),
    AnyNode,
    AnyElement,
    Choice(&'static [Matcher<K, C>]),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Repeat<K: 'static, C: 'static> {
    None,
    Separated {
        separator: Matcher<K, C>,
        minimum: u16,
        trailing: TrailingSeparator,
        recovery: Recovery,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SlotShape<K: 'static, C: 'static> {
    pub cardinality: Cardinality,
    pub matcher: Matcher<K, C>,
    pub disambiguation: Disambiguation,
    pub repeat: Repeat<K, C>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeShape<K: 'static, C: 'static> {
    pub kind: K,
    pub class: NodeClass,
    pub slots: &'static [SlotShape<K, C>],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeClass {
    Valid,
    /// A variable-length syntax-list node. List entries occupy its slot range;
    /// a parent still refers to the entire list through one fixed node slot.
    List,
    Malformed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CategoryShape<K: 'static, C: 'static> {
    pub category: C,
    pub kinds: &'static [K],
    pub bogus: K,
}

#[derive(Clone, Copy, Debug)]
pub struct SyntaxSchema<K: 'static, C: 'static> {
    pub nodes: &'static [NodeShape<K, C>],
    pub categories: &'static [CategoryShape<K, C>],
}

/// Lowers a language's declarative schema into static syntax-shape metadata.
///
/// This is exported only because the language syntax crates are separate
/// crates. It is an implementation detail of their crate-private schema
/// macros, not a syntax or formatter API.
#[doc(hidden)]
#[macro_export]
macro_rules! __lower_syntax_schema {
    (
        $syntax_kind:ident;
        tokens { $($token:ident,)* }
        categories { $($family:ident => $bogus:ident { $($member:ident,)* })* }
        nodes {
            $(
                $kind:ident => $wrapper:ident [$module:ident $class:ident] {
                    $(
                        $field:ident: $cardinality:ident $matcher:tt
                        $(=> $role:ident)?
                        $([$($policy:tt)*])?;
                    )*
                }
            )*
        }
    ) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub(crate) enum Category {
            $($family,)*
        }

        pub(crate) const FIRST_NODE_KIND: usize = [$(stringify!($token)),*].len();

        pub(crate) const CATEGORIES: &[$crate::schema::CategoryShape<$syntax_kind, Category>] = &[
            $($crate::schema::CategoryShape {
                category: Category::$family,
                kinds: &[$($syntax_kind::$member),*],
                bogus: $syntax_kind::$bogus,
            },)*
        ];

        pub(crate) const NODES: &[$crate::schema::NodeShape<$syntax_kind, Category>] = &[
            $($crate::schema::NodeShape {
                kind: $syntax_kind::$kind,
                class: $crate::__lower_syntax_schema!(@class $class),
                slots: &[$($crate::schema::SlotShape {
                    cardinality: $crate::__lower_syntax_schema!(@cardinality $cardinality),
                    matcher: $crate::__lower_syntax_schema!(@matcher $syntax_kind $matcher),
                    disambiguation: $crate::__lower_syntax_schema!(
                        @disambiguation $([$($policy)*])?
                    ),
                    repeat: $crate::__lower_syntax_schema!(
                        @repeat $syntax_kind $([$($policy)*])?
                    ),
                },)*],
            },)*
            $($crate::schema::NodeShape {
                kind: $syntax_kind::$bogus,
                class: $crate::schema::NodeClass::Malformed,
                slots: &[$crate::schema::SlotShape {
                    cardinality: $crate::schema::Cardinality::Many,
                    matcher: $crate::schema::Matcher::AnyElement,
                    disambiguation: $crate::schema::Disambiguation::None,
                    repeat: $crate::schema::Repeat::None,
                }],
            },)*
        ];

        $(
            #[allow(non_camel_case_types)]
            pub(crate) mod $module {
                #[repr(usize)]
                pub(crate) enum Slot {
                    $($field,)*
                    __Len,
                }
            }
        )*

        pub(crate) const SCHEMA: $crate::schema::SyntaxSchema<$syntax_kind, Category> =
            $crate::schema::SyntaxSchema {
                nodes: NODES,
                categories: CATEGORIES,
            };

        pub(crate) fn raw_node_for_kind(
            kind: $syntax_kind,
        ) -> Option<&'static $crate::schema::NodeShape<$syntax_kind, Category>> {
            let index = usize::from(u16::from(kind)).checked_sub(FIRST_NODE_KIND)?;
            NODES.get(index)
        }
    };

    (@cardinality required) => { $crate::schema::Cardinality::Required };
    (@cardinality optional) => { $crate::schema::Cardinality::Optional };
    (@cardinality many) => { $crate::schema::Cardinality::Many };
    (@cardinality one_or_more) => { $crate::schema::Cardinality::OneOrMore };

    (@class valid) => { $crate::schema::NodeClass::Valid };
    (@class list) => { $crate::schema::NodeClass::List };
    (@class malformed) => { $crate::schema::NodeClass::Malformed };

    (@disambiguation) => { $crate::schema::Disambiguation::None };
    (@disambiguation [disambiguate longest_then_first]) => {
        $crate::schema::Disambiguation::LongestThenFirst
    };
    (@disambiguation [disambiguate leftmost_longest]) => {
        $crate::schema::Disambiguation::LeftmostLongest
    };
    (@disambiguation [separated $($policy:tt)*]) => {
        $crate::schema::Disambiguation::None
    };

    (@repeat $syntax_kind:ident) => { $crate::schema::Repeat::None };
    (@repeat $syntax_kind:ident [disambiguate $disambiguation:ident]) => {
        $crate::schema::Repeat::None
    };
    (@repeat $syntax_kind:ident [
        separated $separator:tt,
        minimum $minimum:literal,
        trailing $trailing:ident,
        recovery bogus_owner
    ]) => {
        $crate::schema::Repeat::Separated {
            separator: $crate::__lower_syntax_schema!(@matcher $syntax_kind $separator),
            minimum: $minimum,
            trailing: $crate::__lower_syntax_schema!(@trailing $trailing),
            recovery: $crate::schema::Recovery::BogusOwner,
        }
    };

    (@trailing forbidden) => { $crate::schema::TrailingSeparator::Forbidden };
    (@trailing optional) => { $crate::schema::TrailingSeparator::Optional };
    (@trailing required) => { $crate::schema::TrailingSeparator::Required };

    (@matcher $syntax_kind:ident (token $kind:ident)) => {
        $crate::schema::Matcher::Token($syntax_kind::$kind)
    };
    (@matcher $syntax_kind:ident (token_set [$($kind:ident),*])) => {
        $crate::schema::Matcher::TokenSet(&[$($syntax_kind::$kind),*])
    };
    (@matcher $syntax_kind:ident (element_set [$($kind:ident),*])) => {
        $crate::schema::Matcher::ElementSet(&[$($syntax_kind::$kind),*])
    };
    (@matcher $syntax_kind:ident (contextual $text:literal)) => {
        $crate::schema::Matcher::Contextual {
            kind: $syntax_kind::Identifier,
            text: $text,
        }
    };
    (@matcher $syntax_kind:ident (node $kind:ident)) => {
        $crate::schema::Matcher::Node($syntax_kind::$kind)
    };
    (@matcher $syntax_kind:ident (constructed $kind:ident)) => {
        $crate::schema::Matcher::Constructed($syntax_kind::$kind)
    };
    (@matcher $syntax_kind:ident (list $kind:ident)) => {
        $crate::schema::Matcher::List($syntax_kind::$kind)
    };
    (@matcher $syntax_kind:ident (node_set [$($kind:ident),*])) => {
        $crate::schema::Matcher::NodeSet(&[$($syntax_kind::$kind),*])
    };
    (@matcher $syntax_kind:ident (category $category:ident)) => {
        $crate::schema::Matcher::Category(Category::$category)
    };
    (@matcher $syntax_kind:ident (any_node)) => { $crate::schema::Matcher::AnyNode };
    (@matcher $syntax_kind:ident (any_element)) => { $crate::schema::Matcher::AnyElement };
    (@matcher $syntax_kind:ident (choice [$($matcher:tt),*])) => {
        $crate::schema::Matcher::Choice(&[
            $($crate::__lower_syntax_schema!(@matcher $syntax_kind $matcher)),*
        ])
    };
}
