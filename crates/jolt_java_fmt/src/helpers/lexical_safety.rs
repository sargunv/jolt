use jolt_fmt_ir::{ExceptionalSeparator, LexicalAtom, LexicalAtomKind, LexicalSafety};
use jolt_java_syntax::{JavaLanguage, JavaSyntaxKind, JavaSyntaxToken};

/// Bounded Java lexical-safety decisions at exceptional source boundaries.
pub(crate) struct JavaLexicalSafety;

impl LexicalSafety<JavaLanguage> for JavaLexicalSafety {
    fn classify(&mut self, token: &JavaSyntaxToken<'_>) -> LexicalAtomKind {
        match token.kind() {
            JavaSyntaxKind::IntegerLiteral | JavaSyntaxKind::FloatingPointLiteral => {
                LexicalAtomKind::Number
            }
            JavaSyntaxKind::CharacterLiteral
            | JavaSyntaxKind::StringLiteral
            | JavaSyntaxKind::TextBlockLiteral => LexicalAtomKind::String,
            kind if is_word(kind) => LexicalAtomKind::Identifier,
            _ => LexicalAtomKind::Punctuation,
        }
    }

    fn separator(&mut self, left: LexicalAtom<'_>, right: LexicalAtom<'_>) -> ExceptionalSeparator {
        if lexical_join_needs_space(left.kind(), left.text(), right.kind(), right.text()) {
            ExceptionalSeparator::Space
        } else {
            ExceptionalSeparator::None
        }
    }
}

/// Returns whether two structured source tokens need an explicit space to
/// retain their lexical identities at a formatter-created boundary.
pub(crate) fn structured_tokens_need_space(
    left: &JavaSyntaxToken<'_>,
    right: &JavaSyntaxToken<'_>,
) -> bool {
    let mut safety = JavaLexicalSafety;
    lexical_join_needs_space(
        safety.classify(left),
        left.text(),
        safety.classify(right),
        right.text(),
    )
}

fn lexical_join_needs_space(
    left_kind: LexicalAtomKind,
    left_text: &str,
    right_kind: LexicalAtomKind,
    right_text: &str,
) -> bool {
    use LexicalAtomKind::{Comment, Identifier, Number, Punctuation, String};

    match (left_kind, right_kind) {
        (Identifier | Number, Identifier | Number) | (String, String) => true,
        (Number, Punctuation) => right_text.starts_with('.'),
        (Punctuation, Number) => left_text.ends_with('.'),
        (Punctuation | Comment, Punctuation | Comment) => {
            punctuation_join_fuses(left_text, right_text)
        }
        _ => false,
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
            | (b'=' | b'!' | b'*' | b'%' | b'^', b'=')
            | (b'<', b'<' | b'=')
            | (b'>', b'>' | b'=')
            | (b'&', b'&' | b'=')
            | (b'|', b'|' | b'=')
            | (b':', b':')
            | (b'.', b'.')
    )
}

fn is_word(kind: JavaSyntaxKind) -> bool {
    matches!(
        kind,
        JavaSyntaxKind::Identifier
            | JavaSyntaxKind::BooleanLiteral
            | JavaSyntaxKind::NullLiteral
            | JavaSyntaxKind::AbstractKw
            | JavaSyntaxKind::AssertKw
            | JavaSyntaxKind::BooleanKw
            | JavaSyntaxKind::BreakKw
            | JavaSyntaxKind::ByteKw
            | JavaSyntaxKind::CaseKw
            | JavaSyntaxKind::CatchKw
            | JavaSyntaxKind::CharKw
            | JavaSyntaxKind::ClassKw
            | JavaSyntaxKind::ConstKw
            | JavaSyntaxKind::ContinueKw
            | JavaSyntaxKind::DefaultKw
            | JavaSyntaxKind::DoKw
            | JavaSyntaxKind::DoubleKw
            | JavaSyntaxKind::ElseKw
            | JavaSyntaxKind::EnumKw
            | JavaSyntaxKind::ExtendsKw
            | JavaSyntaxKind::FinalKw
            | JavaSyntaxKind::FinallyKw
            | JavaSyntaxKind::FloatKw
            | JavaSyntaxKind::ForKw
            | JavaSyntaxKind::GotoKw
            | JavaSyntaxKind::IfKw
            | JavaSyntaxKind::ImplementsKw
            | JavaSyntaxKind::ImportKw
            | JavaSyntaxKind::InstanceofKw
            | JavaSyntaxKind::IntKw
            | JavaSyntaxKind::InterfaceKw
            | JavaSyntaxKind::LongKw
            | JavaSyntaxKind::NativeKw
            | JavaSyntaxKind::NewKw
            | JavaSyntaxKind::PackageKw
            | JavaSyntaxKind::PrivateKw
            | JavaSyntaxKind::ProtectedKw
            | JavaSyntaxKind::PublicKw
            | JavaSyntaxKind::ReturnKw
            | JavaSyntaxKind::ShortKw
            | JavaSyntaxKind::StaticKw
            | JavaSyntaxKind::StrictfpKw
            | JavaSyntaxKind::SuperKw
            | JavaSyntaxKind::SwitchKw
            | JavaSyntaxKind::SynchronizedKw
            | JavaSyntaxKind::ThisKw
            | JavaSyntaxKind::ThrowKw
            | JavaSyntaxKind::ThrowsKw
            | JavaSyntaxKind::TransientKw
            | JavaSyntaxKind::TryKw
            | JavaSyntaxKind::VoidKw
            | JavaSyntaxKind::VolatileKw
            | JavaSyntaxKind::WhileKw
            | JavaSyntaxKind::UnderscoreKw
    )
}
