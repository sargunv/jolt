//! Shared lowering for language syntax schemas.
//!
//! The production language crates consume their declarative schema directly.
//! This macro emits only the category discriminants and physical slot indices
//! shared by their generated factory and typed accessors.

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
    };
}
