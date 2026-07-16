use jolt_syntax::{
    NormalizedToken, RemovalClaim, RemovalReason, ReorderClaim, ReorderReason, SourceIdentity,
    SynthesisClaim,
};

use crate::language::{KotlinLanguage, NORMALIZATION_AUTHORITY};
use crate::{
    BlockItemList, Expression, ImportDirective, KotlinFileItemList, KotlinSyntaxKind,
    KotlinSyntaxListPart, KotlinSyntaxToken, KotlinSyntaxView, PropertyBodyMemberList,
    TerminatorList,
};

/// Paired source-free delimiters authorized by one valid Kotlin syntax owner.
pub struct KotlinDelimiterSynthesis<'source> {
    pub open: SynthesisClaim<'source>,
    pub close: SynthesisClaim<'source>,
}

impl<'source> Expression<'source> {
    /// Authorizes readability parentheses around this valid expression.
    #[must_use]
    pub fn precedence_parenthesis_claims(&self) -> Option<KotlinDelimiterSynthesis<'source>> {
        if !self.is_recovery_free() {
            return None;
        }
        let syntax = self.syntax_node()?;
        let anchor = syntax
            .first_token()
            .or_else(|| syntax.last_token())?
            .source_id();
        Some(KotlinDelimiterSynthesis {
            open: SynthesisClaim::authorized::<KotlinLanguage>(
                NORMALIZATION_AUTHORITY,
                anchor,
                NormalizedToken::OpenPrecedenceParenthesis,
            ),
            close: SynthesisClaim::authorized::<KotlinLanguage>(
                NORMALIZATION_AUTHORITY,
                anchor,
                NormalizedToken::ClosePrecedenceParenthesis,
            ),
        })
    }
}

fn separator_removal_claim<'source>(
    token: KotlinSyntaxToken<'source>,
    allowed: &[KotlinSyntaxKind],
) -> Option<RemovalClaim<'source>> {
    allowed.contains(&token.kind()).then(|| {
        RemovalClaim::authorized::<KotlinLanguage>(
            NORMALIZATION_AUTHORITY,
            SourceIdentity::Token(token.source_id()),
            RemovalReason::RedundantSeparator,
        )
    })
}

macro_rules! impl_separator_removal_claim {
    ($owner:ident, [$($kind:ident),+ $(,)?]) => {
        impl<'source> $owner<'source> {
            /// Authorizes canonical omission of a represented statement
            /// boundary separator owned by this physical list.
            #[must_use]
            pub fn separator_removal_claim(
                &self,
                token: KotlinSyntaxToken<'source>,
            ) -> Option<RemovalClaim<'source>> {
                if !self.is_recovery_free()
                    || !self.parts().any(|part| match part {
                        Ok(KotlinSyntaxListPart::Item(element)) => element
                            .token()
                            .is_some_and(|owned| owned.source_id() == token.source_id()),
                        Ok(KotlinSyntaxListPart::Separator(owned)) => {
                            owned.source_id() == token.source_id()
                        }
                        Ok(KotlinSyntaxListPart::Missing(_) | KotlinSyntaxListPart::Malformed(_))
                        | Err(_) => false,
                    })
                {
                    return None;
                }
                separator_removal_claim(token, &[$(KotlinSyntaxKind::$kind),+])
            }
        }
    };
}

impl_separator_removal_claim!(TerminatorList, [EolOrSemicolon, Semicolon]);
impl_separator_removal_claim!(KotlinFileItemList, [EolOrSemicolon, Semicolon]);
impl_separator_removal_claim!(BlockItemList, [EolOrSemicolon, Semicolon]);
impl_separator_removal_claim!(PropertyBodyMemberList, [EolOrSemicolon, Semicolon]);

impl<'source> ImportDirective<'source> {
    /// Authorizes canonical ordering of this recovery-free import.
    #[must_use]
    pub fn canonical_reorder_claim(&self) -> Option<ReorderClaim<'source>> {
        if !self.is_recovery_free() {
            return None;
        }
        let syntax = self.syntax_node()?;
        let anchor = syntax
            .first_token()
            .or_else(|| syntax.last_token())?
            .source_id();
        Some(ReorderClaim::authorized::<KotlinLanguage>(
            NORMALIZATION_AUTHORITY,
            anchor,
            ReorderReason::Imports,
        ))
    }
}
