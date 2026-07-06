use crate::KotlinSyntaxKind as K;

use super::super::Parser;

impl Parser<'_> {
    pub(in crate::parser::grammar) fn at_modifier_or_annotation(&mut self) -> bool {
        self.is_modifier_or_annotation_start_at(self.position())
    }

    pub(in crate::parser::grammar) fn at_annotation_use_site_target(&mut self) -> bool {
        matches!(
            self.current_kind(),
            K::FileKw
                | K::FieldKw
                | K::PropertyKw
                | K::ReceiverKw
                | K::ParamKw
                | K::SetParamKw
                | K::DelegateKw
                | K::GetKw
                | K::SetKw
                | K::AllKw
        ) || self
            .current_text()
            .is_some_and(is_annotation_use_site_target)
    }

    pub(in crate::parser::grammar) fn expect_soft_keyword(&mut self, text: &str, message: &str) {
        if !self.eat_soft_keyword(text) {
            self.expected_here(message);
        }
    }

    pub(in crate::parser::grammar) fn eat_soft_keyword(&mut self, text: &str) -> bool {
        if self.at_soft_keyword(text) {
            self.bump();
            true
        } else {
            false
        }
    }

    pub(in crate::parser::grammar) fn at_soft_keyword(&mut self, text: &str) -> bool {
        let kind = self.current_kind();
        self.is_soft_kind(kind, text)
    }

    pub(in crate::parser::grammar) fn nth_non_modifier_is_soft_keyword(
        &mut self,
        text: &str,
    ) -> bool {
        let mut index = self.position();
        while self.is_modifier_or_annotation_start_at(index) {
            index += 1;
        }
        let kind = self.kind_at(index);
        let token_text = self.text_at(index);
        kind_or_text_is_soft_keyword(kind, token_text, text)
    }

    pub(in crate::parser::grammar) fn is_soft_kind(&mut self, kind: K, text: &str) -> bool {
        let token_text = self.current_text();
        kind_or_text_is_soft_keyword(kind, token_text, text)
    }

    fn is_modifier_or_annotation_start_at(&mut self, index: usize) -> bool {
        let kind = self.kind_at(index);
        is_modifier_or_annotation_start(kind)
            || self.text_at(index).is_some_and(is_modifier_keyword_text)
                && !matches!(
                    self.kind_at(index + 1),
                    K::Colon
                        | K::Comma
                        | K::RParen
                        | K::Assign
                        | K::Dot
                        | K::SafeAccess
                        | K::Lt
                        | K::LtEq
                        | K::Gt
                        | K::GtEq
                        | K::EqEq
                        | K::BangEq
                        | K::EqEqEq
                        | K::BangEqEqEq
                )
    }
}

fn kind_or_text_is_soft_keyword(kind: K, token_text: Option<&str>, text: &str) -> bool {
    soft_keyword_kind_text(kind) == Some(text) || token_text == Some(text)
}

fn soft_keyword_kind_text(kind: K) -> Option<&'static str> {
    match kind {
        K::AllKw => Some("all"),
        K::FileKw => Some("file"),
        K::FieldKw => Some("field"),
        K::PropertyKw => Some("property"),
        K::ReceiverKw => Some("receiver"),
        K::ParamKw => Some("param"),
        K::SetParamKw => Some("setparam"),
        K::DelegateKw => Some("delegate"),
        K::ImportKw => Some("import"),
        K::WhereKw => Some("where"),
        K::ByKw => Some("by"),
        K::GetKw => Some("get"),
        K::SetKw => Some("set"),
        K::ConstructorKw => Some("constructor"),
        K::InitKw => Some("init"),
        K::ContextKw => Some("context"),
        K::CatchKw => Some("catch"),
        K::DynamicKw => Some("dynamic"),
        K::FinallyKw => Some("finally"),
        K::AbstractKw => Some("abstract"),
        K::EnumKw => Some("enum"),
        K::ContractKw => Some("contract"),
        K::OpenKw => Some("open"),
        K::InnerKw => Some("inner"),
        K::OverrideKw => Some("override"),
        K::PrivateKw => Some("private"),
        K::PublicKw => Some("public"),
        K::InternalKw => Some("internal"),
        K::ProtectedKw => Some("protected"),
        K::OutKw => Some("out"),
        K::VarargKw => Some("vararg"),
        K::ReifiedKw => Some("reified"),
        K::CompanionKw => Some("companion"),
        K::SealedKw => Some("sealed"),
        K::FinalKw => Some("final"),
        K::LateinitKw => Some("lateinit"),
        K::DataKw => Some("data"),
        K::ValueKw => Some("value"),
        K::InlineKw => Some("inline"),
        K::NoinlineKw => Some("noinline"),
        K::TailrecKw => Some("tailrec"),
        K::ExternalKw => Some("external"),
        K::AnnotationKw => Some("annotation"),
        K::CrossinlineKw => Some("crossinline"),
        K::OperatorKw => Some("operator"),
        K::InfixKw => Some("infix"),
        K::ConstKw => Some("const"),
        K::SuspendKw => Some("suspend"),
        K::ExpectKw => Some("expect"),
        K::ActualKw => Some("actual"),
        _ => None,
    }
}

fn is_modifier_keyword_text(text: &str) -> bool {
    matches!(
        text,
        "abstract"
            | "enum"
            | "contract"
            | "open"
            | "inner"
            | "override"
            | "private"
            | "public"
            | "internal"
            | "protected"
            | "out"
            | "vararg"
            | "reified"
            | "companion"
            | "sealed"
            | "final"
            | "lateinit"
            | "data"
            | "value"
            | "inline"
            | "noinline"
            | "tailrec"
            | "external"
            | "annotation"
            | "crossinline"
            | "operator"
            | "infix"
            | "const"
            | "suspend"
            | "expect"
            | "actual"
    )
}

fn is_annotation_use_site_target(text: &str) -> bool {
    matches!(
        text,
        "file"
            | "field"
            | "property"
            | "receiver"
            | "param"
            | "setparam"
            | "delegate"
            | "get"
            | "set"
            | "all"
    )
}

fn is_modifier_or_annotation_start(kind: K) -> bool {
    matches!(
        kind,
        K::At
            | K::Hash
            | K::AbstractKw
            | K::EnumKw
            | K::OpenKw
            | K::InnerKw
            | K::OverrideKw
            | K::PrivateKw
            | K::PublicKw
            | K::InternalKw
            | K::ProtectedKw
            | K::OutKw
            | K::VarargKw
            | K::ReifiedKw
            | K::CompanionKw
            | K::SealedKw
            | K::FinalKw
            | K::LateinitKw
            | K::DataKw
            | K::ValueKw
            | K::InlineKw
            | K::NoinlineKw
            | K::TailrecKw
            | K::ExternalKw
            | K::AnnotationKw
            | K::CrossinlineKw
            | K::OperatorKw
            | K::InfixKw
            | K::ConstKw
            | K::SuspendKw
            | K::ExpectKw
            | K::ActualKw
    )
}
