use crate::KotlinSyntaxKind as K;

use super::super::Parser;

impl Parser<'_> {
    pub(in crate::parser::grammar) fn parse_qualified_name(&mut self) {
        let marker = self.start();
        let segments = self.start();
        self.parse_name();
        while self.eat(K::Dot) {
            if self.at(K::Star) {
                break;
            }
            self.parse_name();
        }
        self.complete(segments, K::QualifiedNameSegmentList);
        self.complete(marker, K::QualifiedName);
    }

    pub(in crate::parser::grammar) fn parse_name(&mut self) {
        let marker = self.start();
        if self.at_identifier_like() {
            self.bump();
        } else {
            self.expected_here("expected name");
        }
        self.complete(marker, K::Name);
    }

    pub(in crate::parser::grammar) fn at_identifier_like(&mut self) -> bool {
        is_identifier_like_kind(self.current_kind())
    }
}

pub(in crate::parser::grammar) fn is_identifier_like_kind(kind: K) -> bool {
    matches!(
        kind,
        K::Identifier
            | K::FieldIdentifier
            | K::AllKw
            | K::FileKw
            | K::FieldKw
            | K::PropertyKw
            | K::ReceiverKw
            | K::ParamKw
            | K::SetParamKw
            | K::DelegateKw
            | K::ImportKw
            | K::WhereKw
            | K::ByKw
            | K::GetKw
            | K::SetKw
            | K::ConstructorKw
            | K::InitKw
            | K::ContextKw
            | K::CatchKw
            | K::DynamicKw
            | K::FinallyKw
            | K::AbstractKw
            | K::EnumKw
            | K::ContractKw
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
