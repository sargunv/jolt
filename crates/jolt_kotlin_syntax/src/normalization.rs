pub use jolt_syntax::ReorderClaim;
use jolt_syntax::{
    NormalizationOwner, NormalizedToken, RemovalClaim, RemovalReason, ReorderReason,
    SourceIdentity, SynthesisClaim,
};

use crate::language::{KotlinLanguage, NORMALIZATION_AUTHORITY};
use crate::{
    BinaryExpression, Expression, ImportDirective, KotlinSyntaxKind, KotlinSyntaxToken,
    KotlinSyntaxView, TerminatorList,
};

/// Paired source-free delimiters authorized by one valid Kotlin syntax owner.
pub struct KotlinDelimiterSynthesis<'source> {
    pub open: SynthesisClaim<'source>,
    pub close: SynthesisClaim<'source>,
}

fn normalization_owner<'source>(
    owner: &impl KotlinSyntaxView<'source>,
) -> Option<NormalizationOwner<'source, KotlinLanguage>> {
    NormalizationOwner::authorized(NORMALIZATION_AUTHORITY, &owner.syntax_node()?)
}

impl<'source> BinaryExpression<'source> {
    /// Authorizes readability parentheses around this valid expression.
    #[must_use]
    pub fn precedence_parenthesis_claims(
        &self,
        operand: &Expression<'source>,
    ) -> Option<KotlinDelimiterSynthesis<'source>> {
        let syntax = operand.syntax_node()?;
        let owner = normalization_owner(self)?;
        let anchor = syntax
            .first_token()
            .or_else(|| syntax.last_token())?
            .source_id();
        Some(KotlinDelimiterSynthesis {
            open: SynthesisClaim::authorized(
                owner,
                anchor,
                NormalizedToken::OpenPrecedenceParenthesis,
            ),
            close: SynthesisClaim::authorized(
                owner,
                anchor,
                NormalizedToken::ClosePrecedenceParenthesis,
            ),
        })
    }
}

fn separator_removal_claim<'source>(
    owner: NormalizationOwner<'source, KotlinLanguage>,
    token: KotlinSyntaxToken<'source>,
    allowed: &[KotlinSyntaxKind],
) -> Option<RemovalClaim<'source>> {
    allowed.contains(&token.kind()).then(|| {
        RemovalClaim::authorized_boundary(
            owner,
            SourceIdentity::Token(token.source_id()),
            RemovalReason::RedundantSeparator,
        )
    })
}

/// Authorizes omission of one represented boundary separator against the
/// already-tracked preceding complete item. Formatter list walks call this in
/// one pass rather than rescanning the physical list for every separator.
#[must_use]
pub fn boundary_separator_removal_claim<'source>(
    owner: &impl KotlinSyntaxView<'source>,
    token: KotlinSyntaxToken<'source>,
) -> Option<RemovalClaim<'source>> {
    let owner = normalization_owner(owner)?;
    if !owner.owns_boundary_token(token.source_id()) {
        return None;
    }
    separator_removal_claim(
        owner,
        token,
        &[
            KotlinSyntaxKind::EolOrSemicolon,
            KotlinSyntaxKind::Semicolon,
        ],
    )
}

impl<'source> TerminatorList<'source> {
    /// Authorizes omission of a terminator only when its complete enclosing
    /// package, import, or statement owner is recovery-free.
    #[must_use]
    pub fn separator_removal_claim(
        &self,
        token: KotlinSyntaxToken<'source>,
    ) -> Option<RemovalClaim<'source>> {
        let syntax = self.syntax_node()?;
        let owner = NormalizationOwner::authorized(NORMALIZATION_AUTHORITY, &syntax.parent()?)?;
        if !owner.owns_token(token.source_id()) {
            return None;
        }
        separator_removal_claim(
            owner,
            token,
            &[
                KotlinSyntaxKind::EolOrSemicolon,
                KotlinSyntaxKind::Semicolon,
            ],
        )
    }
}

impl<'source> ImportDirective<'source> {
    /// Authorizes canonical ordering of this recovery-free import.
    #[must_use]
    pub fn canonical_reorder_claim(&self) -> Option<ReorderClaim<'source>> {
        let syntax = self.syntax_node()?;
        let owner = normalization_owner(self)?;
        let anchor = syntax
            .first_token()
            .or_else(|| syntax.last_token())?
            .source_id();
        Some(ReorderClaim::authorized(
            owner,
            anchor,
            ReorderReason::Imports,
        ))
    }
}
