//! Language-neutral typed CST accessor generation.
//!
//! These macros centralize the mechanical field accessors, grammar-role
//! wrappers, and variable-length list views shared by every language's typed
//! CST. Language crates own their schema, trait names, and public type names;
//! this module only projects the identical accessor bodies parameterized by
//! those names.
//!
//! The accessor bodies reference items resolved at the expansion site: the free
//! helper functions (`required_token`, `required_node`, `required_family`,
//! `required_role_element`, `list_parts`, and their `optional_*` aliases) that
//! [`crate::define_typed_cst_access`] generates into the language module, and
//! that module's `crate::shape` slot enums.

/// Defines a grammar-role wrapper struct for a heterogeneous declared slot.
///
/// The leading group threads the language type and trait names; the trailing
/// identifier names the generated role type.
#[doc(hidden)]
#[macro_export]
macro_rules! define_typed_cst_role {
    (
        ($syntax_token:ident $role_element:ident $typed_node:ident $family:ident $list_item:ident)
        $role:ident
    ) => {
        #[derive(Clone, Copy, Debug)]
        pub struct $role<'source> {
            element: $role_element<'source>,
        }

        impl<'source> $role<'source> {
            #[must_use]
            pub fn token(self) -> Option<$syntax_token<'source>> {
                match self.element {
                    $role_element::Token(token) => Some(token),
                    $role_element::Node(_) => None,
                }
            }

            #[must_use]
            pub fn first_token(self) -> Option<$syntax_token<'source>> {
                match self.element {
                    $role_element::Node(node) => node.first_token(),
                    $role_element::Token(token) => Some(token),
                }
            }

            #[must_use]
            pub fn last_token(self) -> Option<$syntax_token<'source>> {
                match self.element {
                    $role_element::Node(node) => node.last_token(),
                    $role_element::Token(token) => Some(token),
                }
            }

            #[must_use]
            pub fn cast_node<N: $typed_node<'source>>(self) -> Option<N> {
                N::cast_element(self.element)
            }

            #[must_use]
            pub fn cast_family<F: $family<'source>>(self) -> Option<F> {
                match self.element {
                    $role_element::Node(node) => F::cast(node),
                    $role_element::Token(_) => None,
                }
            }
        }

        impl<'source> $list_item<'source> for $role<'source> {
            fn cast_element(element: $role_element<'source>) -> Option<Self> {
                Some(Self { element })
            }
        }
    };
}

/// Generates a single typed field accessor for one declared slot.
///
/// The leading group threads the language names
/// `(syntax_field syntax_token role_element generic_vis)`.
/// Slot indices resolve through the expansion-site `crate::shape` module.
///
/// `crate::shape` deliberately names the invoking language crate's shape module
/// rather than `jolt_syntax`, so the `crate_in_macro_def` lint is suppressed.
/// Native accessors retain the established inline hint for formatter throughput.
/// WASM leaves them unhinted so the formatter's documented field-resolution
/// boundary can prevent aggregate projection from duplicating every layout rule.
#[doc(hidden)]
#[macro_export]
#[allow(clippy::crate_in_macro_def)]
macro_rules! __typed_cst_field_accessor {
    ($names:tt $module:ident $field:ident required $matcher:tt => $role:ident) => {
        $crate::__typed_cst_field_accessor!(@role $names $module $field $role);
    };
    ($names:tt $module:ident $field:ident optional $matcher:tt => $role:ident) => {
        $crate::__typed_cst_field_accessor!(@role $names $module $field $role);
    };
    (
        @role ($sf:ident $st:ident $re:ident $gv:vis)
        $module:ident $field:ident $role:ident
    ) => {
        #[cfg_attr(not(target_arch = "wasm32"), inline)]
        pub fn $field(&self) -> $sf<'source, $role<'source>> {
            required_role_element(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
                .map(|element| $role { element })
        }
    };

    (
        ($sf:ident $st:ident $re:ident $gv:vis)
        $module:ident $field:ident required (token $kind:ident)
    ) => {
        #[cfg_attr(not(target_arch = "wasm32"), inline)]
        pub fn $field(&self) -> $sf<'source, $st<'source>> {
            required_token(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($names:tt $module:ident $field:ident required (token_set $kinds:tt)) => {
        $crate::__typed_cst_field_accessor!($names $module $field required (token __schema_token_set));
    };
    ($names:tt $module:ident $field:ident required (contextual $text:literal)) => {
        $crate::__typed_cst_field_accessor!($names $module $field required (token __schema_contextual));
    };
    (
        ($sf:ident $st:ident $re:ident $gv:vis)
        $module:ident $field:ident optional (token $kind:ident)
    ) => {
        #[cfg_attr(not(target_arch = "wasm32"), inline)]
        pub fn $field(&self) -> $sf<'source, $st<'source>> {
            optional_token(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    ($names:tt $module:ident $field:ident optional (token_set $kinds:tt)) => {
        $crate::__typed_cst_field_accessor!($names $module $field optional (token __schema_token_set));
    };
    ($names:tt $module:ident $field:ident optional (contextual $text:literal)) => {
        $crate::__typed_cst_field_accessor!($names $module $field optional (token __schema_contextual));
    };

    ($names:tt $module:ident $field:ident required (node $target:ident)) => {
        $crate::__typed_cst_field_accessor!(@node $names required $module $field $target);
    };
    ($names:tt $module:ident $field:ident optional (node $target:ident)) => {
        $crate::__typed_cst_field_accessor!(@node $names optional $module $field $target);
    };
    ($names:tt $module:ident $field:ident required (constructed $target:ident)) => {
        $crate::__typed_cst_field_accessor!(@node $names required $module $field $target);
    };
    ($names:tt $module:ident $field:ident optional (constructed $target:ident)) => {
        $crate::__typed_cst_field_accessor!(@node $names optional $module $field $target);
    };
    ($names:tt $module:ident $field:ident required (list $target:ident)) => {
        $crate::__typed_cst_field_accessor!(@node $names required $module $field $target);
    };
    ($names:tt $module:ident $field:ident optional (list $target:ident)) => {
        $crate::__typed_cst_field_accessor!(@node $names optional $module $field $target);
    };
    (
        @node ($sf:ident $st:ident $re:ident $gv:vis)
        required $module:ident $field:ident $target:ident
    ) => {
        #[cfg_attr(not(target_arch = "wasm32"), inline)]
        pub fn $field(&self) -> $sf<'source, $target<'source>> {
            required_node(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    (
        @node ($sf:ident $st:ident $re:ident $gv:vis)
        optional $module:ident $field:ident $target:ident
    ) => {
        #[cfg_attr(not(target_arch = "wasm32"), inline)]
        pub fn $field(&self) -> $sf<'source, $target<'source>> {
            optional_node(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };

    (
        ($sf:ident $st:ident $re:ident $gv:vis)
        $module:ident $field:ident required (category $target:ident)
    ) => {
        #[cfg_attr(not(target_arch = "wasm32"), inline)]
        pub fn $field(&self) -> $sf<'source, $target<'source>> {
            required_family(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    (
        ($sf:ident $st:ident $re:ident $gv:vis)
        $module:ident $field:ident optional (category $target:ident)
    ) => {
        #[cfg_attr(not(target_arch = "wasm32"), inline)]
        pub fn $field(&self) -> $sf<'source, $target<'source>> {
            optional_family(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };

    // Heterogeneous roles are wrapped by the semantic adapters the language
    // crate hand-writes. This primitive still reads exactly one declared slot
    // and never searches.
    (
        ($sf:ident $st:ident $re:ident $gv:vis)
        $module:ident $field:ident required $matcher:tt
    ) => {
        #[cfg_attr(not(target_arch = "wasm32"), inline)]
        $gv fn $field(&self) -> $sf<'source, $re<'source>> {
            required_role_element(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };
    (
        ($sf:ident $st:ident $re:ident $gv:vis)
        $module:ident $field:ident optional $matcher:tt
    ) => {
        #[cfg_attr(not(target_arch = "wasm32"), inline)]
        $gv fn $field(&self) -> $sf<'source, $re<'source>> {
            required_role_element(self.fixed_syntax(), crate::shape::$module::Slot::$field as usize)
        }
    };

    ($names:tt $module:ident $field:ident many $matcher:tt $(=> $role:ident)?) => {};
    ($names:tt $module:ident $field:ident one_or_more $matcher:tt $(=> $role:ident)?) => {};
}

/// Resolves the Rust item type produced for one variable-list element.
#[doc(hidden)]
#[macro_export]
macro_rules! __typed_cst_list_item_type {
    ($re:ident; $source:lifetime; $matcher:tt => $role:ident) => { $role<$source> };
    ($re:ident; $source:lifetime; (node $target:ident)) => { $target<$source> };
    ($re:ident; $source:lifetime; (constructed $target:ident)) => { $target<$source> };
    ($re:ident; $source:lifetime; (category $target:ident)) => { $target<$source> };
    ($re:ident; $source:lifetime; $matcher:tt) => { $re<$source> };
}

/// Resolves the variable-list element type, honoring an optional role wrapper.
#[doc(hidden)]
#[macro_export]
macro_rules! __typed_cst_list_item_type_optional_role {
    ($re:ident; $source:lifetime; $matcher:tt; $role:ident) => {
        $crate::__typed_cst_list_item_type!($re; $source; $matcher => $role)
    };
    ($re:ident; $source:lifetime; $matcher:tt;) => {
        $crate::__typed_cst_list_item_type!($re; $source; $matcher)
    };
}

/// Reports whether a list slot policy declares separator tokens.
#[doc(hidden)]
#[macro_export]
macro_rules! __typed_cst_list_is_separated {
    ([separated $($policy:tt)*]) => {
        true
    };
    ([$($policy:tt)*]) => {
        false
    };
    () => {
        false
    };
}

/// Generates the `parts()` view for a variable-length syntax-list node.
#[doc(hidden)]
#[macro_export]
macro_rules! __typed_cst_variable_slot_view {
    (
        ($lp:ident $re:ident)
        list;
        $field:ident: $cardinality:ident $matcher:tt $(=> $role:ident)? $([$($policy:tt)*])?;
    ) => {
        /// Returns this list's represented elements and separators in source order.
        pub fn parts(&self) -> impl Iterator<
            Item = $lp<
                'source,
                $crate::__typed_cst_list_item_type_optional_role!($re; 'source; $matcher; $($role)?),
            >,
        > + '_ {
            list_parts::<$crate::__typed_cst_list_item_type_optional_role!($re; 'source; $matcher; $($role)?)>(
                self.syntax,
                $crate::__typed_cst_list_is_separated!($([$($policy)*])?),
            )
        }
    };
    (($lp:ident $re:ident) $class:ident; $($fields:tt)*) => {};
}

/// Generates every typed field accessor, grammar-role wrapper, and list view
/// for one language's syntax schema.
///
/// The language crate keeps ownership of its schema and public type names; this
/// macro only projects the mechanical accessor bodies parameterized by those
/// names. The `generic_vis` name selects the visibility of the raw
/// heterogeneous-role accessors the language wraps with hand-written adapters.
#[doc(hidden)]
#[macro_export]
macro_rules! define_typed_cst_accessors {
    (
        names {
            syntax_token: $syntax_token:ident,
            syntax_field: $syntax_field:ident,
            role_element: $role_element:ident,
            list_part: $list_part:ident,
            typed_node: $typed_node:ident,
            family: $family:ident,
            list_item: $list_item:ident,
            generic_vis: $generic_vis:vis,
        }
        schema {
            tokens { $($token:ident,)* }
            categories { $($fam_name:ident => $bogus:ident { $($member:ident,)* })* }
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
        }
    ) => {
        $($(
            $(
                $crate::define_typed_cst_role!(
                    ($syntax_token $role_element $typed_node $family $list_item)
                    $role
                );
            )?
        )*)*

        $(
            impl<'source> $wrapper<'source> {
                $(
                    $crate::__typed_cst_field_accessor!(
                        ($syntax_field $syntax_token $role_element $generic_vis)
                        $module $field $cardinality $matcher $(=> $role)?
                    );
                )*
                $crate::__typed_cst_variable_slot_view!(
                    ($list_part $role_element)
                    $class;
                    $($field: $cardinality $matcher $(=> $role)? $([$($policy)*])?;)*
                );
            }
        )*
    };
}
