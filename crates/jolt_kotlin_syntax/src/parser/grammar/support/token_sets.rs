use crate::KotlinSyntaxKind as K;

pub(in crate::parser::grammar) const DECLARATION_RECOVERY: &[K] = &[
    K::ClassKw,
    K::InterfaceKw,
    K::ObjectKw,
    K::FunKw,
    K::ValKw,
    K::VarKw,
    K::TypeAliasKw,
    K::ConstructorKw,
    K::InitKw,
    K::RBrace,
    K::Eof,
];
