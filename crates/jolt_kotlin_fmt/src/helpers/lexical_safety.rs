use jolt_fmt_ir::{ExceptionalSeparator, LexicalAtom, LexicalAtomKind, LexicalSafety};
use jolt_kotlin_syntax::{KotlinLanguage, KotlinSyntaxKind, KotlinSyntaxToken};

/// Bounded Kotlin lexical-safety decisions at exceptional source boundaries.
pub(crate) struct KotlinLexicalSafety;

impl LexicalSafety<KotlinLanguage> for KotlinLexicalSafety {
    fn classify(&mut self, token: &KotlinSyntaxToken<'_>) -> LexicalAtomKind {
        match token.kind() {
            KotlinSyntaxKind::IntegerLiteral | KotlinSyntaxKind::FloatLiteral => {
                LexicalAtomKind::Number
            }
            KotlinSyntaxKind::CharacterLiteral
            | KotlinSyntaxKind::OpenQuote
            | KotlinSyntaxKind::ClosingQuote
            | KotlinSyntaxKind::RegularStringPart
            | KotlinSyntaxKind::EscapeSequence => LexicalAtomKind::String,
            kind if is_word(kind) => LexicalAtomKind::Identifier,
            _ => LexicalAtomKind::Punctuation,
        }
    }

    fn separator(&mut self, left: LexicalAtom<'_>, right: LexicalAtom<'_>) -> ExceptionalSeparator {
        use LexicalAtomKind::{Comment, Identifier, Number, Punctuation, String};

        let needs_space = match (left.kind(), right.kind()) {
            (Identifier | Number, Identifier | Number) | (String, String) => true,
            (Number, Punctuation) => right.text().starts_with('.'),
            (Punctuation, Number) => left.text().ends_with('.'),
            (Punctuation, Identifier) => {
                left.text().ends_with('!') && matches!(right.text(), "in" | "is")
            }
            (Identifier, Punctuation) => left.text() == "as" && right.text().starts_with('?'),
            (Punctuation | Comment, Punctuation | Comment) => {
                punctuation_join_fuses(left.text(), right.text())
            }
            _ => false,
        };
        if needs_space {
            ExceptionalSeparator::Space
        } else {
            ExceptionalSeparator::None
        }
    }
}

fn punctuation_join_fuses(left: &str, right: &str) -> bool {
    let Some(left) = left.as_bytes().last().copied() else {
        return false;
    };
    let Some(right) = right.as_bytes().first().copied() else {
        return false;
    };
    matches!(
        (left, right),
        (b'/', b'/' | b'*' | b'=')
            | (b'+', b'+' | b'=')
            | (b'-', b'-' | b'=' | b'>')
            | (b'=', b'=' | b'>')
            | (b'*' | b'%' | b'<' | b'>', b'=')
            | (b'!', b'!' | b'=')
            | (b'&', b'&')
            | (b'|', b'|')
            | (b'?', b'.' | b':')
            | (b':', b':')
            | (b';', b';')
            | (b'.', b'.' | b'<')
    )
}

fn is_word(kind: KotlinSyntaxKind) -> bool {
    matches!(
        kind,
        KotlinSyntaxKind::Reserved
            | KotlinSyntaxKind::Identifier
            | KotlinSyntaxKind::FieldIdentifier
            | KotlinSyntaxKind::PackageKw
            | KotlinSyntaxKind::AsKw
            | KotlinSyntaxKind::TypeAliasKw
            | KotlinSyntaxKind::ClassKw
            | KotlinSyntaxKind::ThisKw
            | KotlinSyntaxKind::SuperKw
            | KotlinSyntaxKind::ValKw
            | KotlinSyntaxKind::VarKw
            | KotlinSyntaxKind::FunKw
            | KotlinSyntaxKind::ForKw
            | KotlinSyntaxKind::NullKw
            | KotlinSyntaxKind::TrueKw
            | KotlinSyntaxKind::FalseKw
            | KotlinSyntaxKind::IsKw
            | KotlinSyntaxKind::InKw
            | KotlinSyntaxKind::ThrowKw
            | KotlinSyntaxKind::ReturnKw
            | KotlinSyntaxKind::BreakKw
            | KotlinSyntaxKind::ContinueKw
            | KotlinSyntaxKind::ObjectKw
            | KotlinSyntaxKind::IfKw
            | KotlinSyntaxKind::TryKw
            | KotlinSyntaxKind::ElseKw
            | KotlinSyntaxKind::WhileKw
            | KotlinSyntaxKind::DoKw
            | KotlinSyntaxKind::WhenKw
            | KotlinSyntaxKind::InterfaceKw
            | KotlinSyntaxKind::TypeOfKw
            | KotlinSyntaxKind::AllKw
            | KotlinSyntaxKind::FileKw
            | KotlinSyntaxKind::FieldKw
            | KotlinSyntaxKind::PropertyKw
            | KotlinSyntaxKind::ReceiverKw
            | KotlinSyntaxKind::ParamKw
            | KotlinSyntaxKind::SetParamKw
            | KotlinSyntaxKind::DelegateKw
            | KotlinSyntaxKind::ImportKw
            | KotlinSyntaxKind::WhereKw
            | KotlinSyntaxKind::ByKw
            | KotlinSyntaxKind::GetKw
            | KotlinSyntaxKind::SetKw
            | KotlinSyntaxKind::ConstructorKw
            | KotlinSyntaxKind::InitKw
            | KotlinSyntaxKind::ContextKw
            | KotlinSyntaxKind::CatchKw
            | KotlinSyntaxKind::DynamicKw
            | KotlinSyntaxKind::FinallyKw
            | KotlinSyntaxKind::AbstractKw
            | KotlinSyntaxKind::EnumKw
            | KotlinSyntaxKind::ContractKw
            | KotlinSyntaxKind::OpenKw
            | KotlinSyntaxKind::InnerKw
            | KotlinSyntaxKind::OverrideKw
            | KotlinSyntaxKind::PrivateKw
            | KotlinSyntaxKind::PublicKw
            | KotlinSyntaxKind::InternalKw
            | KotlinSyntaxKind::ProtectedKw
            | KotlinSyntaxKind::OutKw
            | KotlinSyntaxKind::VarargKw
            | KotlinSyntaxKind::ReifiedKw
            | KotlinSyntaxKind::CompanionKw
            | KotlinSyntaxKind::SealedKw
            | KotlinSyntaxKind::FinalKw
            | KotlinSyntaxKind::LateinitKw
            | KotlinSyntaxKind::DataKw
            | KotlinSyntaxKind::ValueKw
            | KotlinSyntaxKind::InlineKw
            | KotlinSyntaxKind::NoinlineKw
            | KotlinSyntaxKind::TailrecKw
            | KotlinSyntaxKind::ExternalKw
            | KotlinSyntaxKind::AnnotationKw
            | KotlinSyntaxKind::CrossinlineKw
            | KotlinSyntaxKind::OperatorKw
            | KotlinSyntaxKind::InfixKw
            | KotlinSyntaxKind::ConstKw
            | KotlinSyntaxKind::SuspendKw
            | KotlinSyntaxKind::ExpectKw
            | KotlinSyntaxKind::ActualKw
    )
}
