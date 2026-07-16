use jolt_syntax::{
    NormalizedToken, RemovalClaim, RemovalReason, ReplacementClaim, SourceIdentity, SynthesisClaim,
};

use crate::language::{JavaLanguage, NORMALIZATION_AUTHORITY};
use crate::{
    AnnotationArrayInitializer, ArrayInitializer, ClassBodyMember, EmptyDeclaration,
    EmptyStatement, EnumBody, Expression, Guard, JavaSyntaxField, JavaSyntaxListPart,
    JavaSyntaxToken, JavaSyntaxView, LambdaExpression, ParenthesizedExpression,
    ResourceSpecification, Statement,
};

/// Paired source-free delimiters authorized by one valid Java syntax owner.
pub struct JavaDelimiterSynthesis<'source> {
    pub open: SynthesisClaim<'source>,
    pub close: SynthesisClaim<'source>,
}

/// Up to two represented delimiters intentionally omitted by canonical layout.
pub struct JavaDelimiterRemoval<'source> {
    pub open: Option<RemovalClaim<'source>>,
    pub close: Option<RemovalClaim<'source>>,
}

fn delimiter_synthesis<'source>(
    owner: &impl JavaSyntaxView<'source>,
    open: NormalizedToken,
    close: NormalizedToken,
) -> Option<JavaDelimiterSynthesis<'source>> {
    if !owner.is_recovery_free() {
        return None;
    }
    let syntax = owner.syntax_node()?;
    let anchor = syntax
        .first_token()
        .or_else(|| syntax.last_token())?
        .source_id();
    Some(JavaDelimiterSynthesis {
        open: SynthesisClaim::authorized::<JavaLanguage>(NORMALIZATION_AUTHORITY, anchor, open),
        close: SynthesisClaim::authorized::<JavaLanguage>(NORMALIZATION_AUTHORITY, anchor, close),
    })
}

fn present<T>(field: Result<JavaSyntaxField<'_, T>, crate::JavaSyntaxInvariantError>) -> Option<T> {
    match field.ok()? {
        JavaSyntaxField::Present(value) => Some(value),
        JavaSyntaxField::Missing(_) | JavaSyntaxField::Malformed(_) => None,
    }
}

impl<'source> Statement<'source> {
    /// Authorizes canonical braces around a valid unbraced statement body.
    #[must_use]
    pub fn block_brace_claims(&self) -> Option<JavaDelimiterSynthesis<'source>> {
        delimiter_synthesis(
            self,
            NormalizedToken::OpenBlockBrace,
            NormalizedToken::CloseBlockBrace,
        )
    }
}

impl<'source> EmptyStatement<'source> {
    /// Authorizes canonical braces around a represented valid empty body.
    #[must_use]
    pub fn block_brace_claims(&self) -> Option<JavaDelimiterSynthesis<'source>> {
        present(self.semicolon())?;
        delimiter_synthesis(
            self,
            NormalizedToken::OpenBlockBrace,
            NormalizedToken::CloseBlockBrace,
        )
    }

    /// Authorizes omission of this redundant empty-statement separator.
    #[must_use]
    pub fn separator_removal_claim(&self) -> Option<RemovalClaim<'source>> {
        let token = present(self.semicolon())?;
        Some(RemovalClaim::authorized::<JavaLanguage>(
            NORMALIZATION_AUTHORITY,
            SourceIdentity::Token(token.source_id()),
            RemovalReason::RedundantSeparator,
        ))
    }
}

impl<'source> ResourceSpecification<'source> {
    /// Authorizes canonical omission of Java's optional trailing resource
    /// separator while retaining its represented comments.
    #[must_use]
    pub fn trailing_separator_removal_claim(&self) -> Option<RemovalClaim<'source>> {
        if !self.is_recovery_free() {
            return None;
        }
        let token = present(self.trailing_semicolon())?;
        Some(RemovalClaim::authorized::<JavaLanguage>(
            NORMALIZATION_AUTHORITY,
            SourceIdentity::Token(token.source_id()),
            RemovalReason::RedundantSeparator,
        ))
    }
}

impl<'source> Guard<'source> {
    /// Authorizes canonical omission of optional parentheses around a valid
    /// switch-pattern guard condition.
    #[must_use]
    pub fn redundant_parenthesis_removal_claims(&self) -> JavaDelimiterRemoval<'source> {
        if !self.is_recovery_free() || present(self.condition()).is_none() {
            return JavaDelimiterRemoval {
                open: None,
                close: None,
            };
        }
        JavaDelimiterRemoval {
            open: present(self.open_paren()).map(|token| {
                RemovalClaim::authorized::<JavaLanguage>(
                    NORMALIZATION_AUTHORITY,
                    SourceIdentity::Token(token.source_id()),
                    RemovalReason::RedundantDelimiter,
                )
            }),
            close: present(self.close_paren()).map(|token| {
                RemovalClaim::authorized::<JavaLanguage>(
                    NORMALIZATION_AUTHORITY,
                    SourceIdentity::Token(token.source_id()),
                    RemovalReason::RedundantDelimiter,
                )
            }),
        }
    }
}

impl<'source> EmptyDeclaration<'source> {
    /// Authorizes omission of this redundant empty-declaration separator.
    #[must_use]
    pub fn separator_removal_claim(&self) -> Option<RemovalClaim<'source>> {
        let token = present(self.semicolon())?;
        Some(RemovalClaim::authorized::<JavaLanguage>(
            NORMALIZATION_AUTHORITY,
            SourceIdentity::Token(token.source_id()),
            RemovalReason::RedundantSeparator,
        ))
    }
}

impl<'source> Expression<'source> {
    /// Authorizes readability parentheses around this valid expression.
    #[must_use]
    pub fn precedence_parenthesis_claims(&self) -> Option<JavaDelimiterSynthesis<'source>> {
        delimiter_synthesis(
            self,
            NormalizedToken::OpenPrecedenceParenthesis,
            NormalizedToken::ClosePrecedenceParenthesis,
        )
    }
}

fn trailing_comma<'source>(
    owner: &impl JavaSyntaxView<'source>,
) -> Option<SynthesisClaim<'source>> {
    if !owner.is_recovery_free() {
        return None;
    }
    let syntax = owner.syntax_node()?;
    let anchor = syntax
        .last_token()
        .or_else(|| syntax.first_token())?
        .source_id();
    Some(SynthesisClaim::authorized::<JavaLanguage>(
        NORMALIZATION_AUTHORITY,
        anchor,
        NormalizedToken::TrailingComma,
    ))
}

impl<'source> ArrayInitializer<'source> {
    #[must_use]
    pub fn trailing_comma_claim(&self) -> Option<SynthesisClaim<'source>> {
        present(self.open_brace())?;
        let values = present(self.values())?;
        if !values.is_recovery_free() {
            return None;
        }
        present(self.close_brace())?;
        trailing_comma(self)
    }
}

impl<'source> AnnotationArrayInitializer<'source> {
    #[must_use]
    pub fn trailing_comma_claim(&self) -> Option<SynthesisClaim<'source>> {
        present(self.open_brace())?;
        let values = present(self.values())?;
        if !values.is_recovery_free() {
            return None;
        }
        present(self.close_brace())?;
        trailing_comma(self)
    }
}

impl<'source> EnumBody<'source> {
    #[must_use]
    pub fn trailing_comma_claim(&self) -> Option<SynthesisClaim<'source>> {
        present(self.open_brace())?;
        let constants = match self.constants().ok()? {
            JavaSyntaxField::Present(constants) => constants,
            JavaSyntaxField::Missing(_) | JavaSyntaxField::Malformed(_) => return None,
        };
        if !constants.is_recovery_free() {
            return None;
        }
        let members = present(self.members())?;
        if !members.is_recovery_free() {
            return None;
        }
        present(self.close_brace())?;
        trailing_comma(self)
    }

    /// Authorizes normalizing the represented enum body separator.
    #[must_use]
    pub fn separator_replacement_claim(
        &self,
        source: &JavaSyntaxToken<'source>,
        normalized: NormalizedToken,
    ) -> Option<ReplacementClaim<'source>> {
        let recovery_free_shape = present(self.open_brace()).is_some()
            && self
                .constants()
                .is_ok_and(|field| !matches!(field, JavaSyntaxField::Malformed(_)))
            && self
                .body_separator()
                .is_ok_and(|field| !matches!(field, JavaSyntaxField::Malformed(_)))
            && present(self.members()).is_some_and(|members| members.is_recovery_free())
            && present(self.close_brace()).is_some();
        if !self.is_recovery_free()
            || !recovery_free_shape
            || !matches!(
                normalized,
                NormalizedToken::EnumComma | NormalizedToken::EnumSemicolon
            )
            || !self.owns_separator(source)
        {
            return None;
        }
        Some(ReplacementClaim::authorized::<JavaLanguage>(
            NORMALIZATION_AUTHORITY,
            source.source_id(),
            normalized,
        ))
    }

    /// Authorizes removing the last constant comma when the represented body
    /// separator supplies the canonical semicolon at the same boundary.
    #[must_use]
    pub fn redundant_constant_separator_removal_claim(
        &self,
        source: &JavaSyntaxToken<'source>,
    ) -> Option<RemovalClaim<'source>> {
        let body_separator = present(self.body_separator())?;
        if !self.is_recovery_free()
            || body_separator.source_id() == source.source_id()
            || !self.owns_separator(source)
        {
            return None;
        }
        Some(RemovalClaim::authorized::<JavaLanguage>(
            NORMALIZATION_AUTHORITY,
            SourceIdentity::Token(source.source_id()),
            RemovalReason::RedundantSeparator,
        ))
    }

    /// Authorizes removing a body separator when the enum has no constants or
    /// retained body declarations.
    #[must_use]
    pub fn redundant_body_separator_removal_claim(&self) -> Option<RemovalClaim<'source>> {
        let separator = present(self.body_separator())?;
        let constants_are_empty = match self.constants().ok()? {
            JavaSyntaxField::Missing(_) => true,
            JavaSyntaxField::Present(constants) => constants.parts().next().is_none(),
            JavaSyntaxField::Malformed(_) => false,
        };
        let members = present(self.members())?;
        let members_are_empty = members.parts().all(|part| {
            matches!(
                part,
                Ok(JavaSyntaxListPart::Item(ClassBodyMember::EmptyDeclaration(
                    _
                )))
            )
        });
        if !self.is_recovery_free() || !constants_are_empty || !members_are_empty {
            return None;
        }
        Some(RemovalClaim::authorized::<JavaLanguage>(
            NORMALIZATION_AUTHORITY,
            SourceIdentity::Token(separator.source_id()),
            RemovalReason::RedundantSeparator,
        ))
    }

    fn owns_separator(&self, source: &JavaSyntaxToken<'_>) -> bool {
        if matches!(
            self.body_separator(),
            Ok(JavaSyntaxField::Present(token)) if token.source_id() == source.source_id()
        ) {
            return true;
        }
        let Ok(JavaSyntaxField::Present(constants)) = self.constants() else {
            return false;
        };
        constants.parts().any(|part| {
            matches!(
                part,
                Ok(JavaSyntaxListPart::Separator(token))
                    if token.source_id() == source.source_id()
            )
        })
    }
}

impl<'source> LambdaExpression<'source> {
    /// Authorizes omission of represented parentheses around a canonical
    /// single untyped parameter. Policy still decides whether to use it.
    #[must_use]
    pub fn parameter_parenthesis_removal_claims(&self) -> JavaDelimiterRemoval<'source> {
        let recovery_free_shape = present(self.parameters())
            .is_some_and(|list| list.is_recovery_free())
            && present(self.arrow()).is_some()
            && present(self.body()).is_some();
        if !self.is_recovery_free() || !recovery_free_shape {
            return JavaDelimiterRemoval {
                open: None,
                close: None,
            };
        }
        let open = match self.open_paren() {
            Ok(JavaSyntaxField::Present(token)) => Some(token),
            _ => None,
        };
        let close = match self.close_paren() {
            Ok(JavaSyntaxField::Present(token)) => Some(token),
            _ => None,
        };
        JavaDelimiterRemoval {
            open: open.map(|token| {
                RemovalClaim::authorized::<JavaLanguage>(
                    NORMALIZATION_AUTHORITY,
                    SourceIdentity::Token(token.source_id()),
                    RemovalReason::RedundantDelimiter,
                )
            }),
            close: close.map(|token| {
                RemovalClaim::authorized::<JavaLanguage>(
                    NORMALIZATION_AUTHORITY,
                    SourceIdentity::Token(token.source_id()),
                    RemovalReason::RedundantDelimiter,
                )
            }),
        }
    }
}

impl<'source> ParenthesizedExpression<'source> {
    /// Authorizes omission of represented readability parentheses while a
    /// binary chain is flattened. Formatter policy still decides whether the
    /// delimiters are comment-free and safe to omit.
    #[must_use]
    pub fn redundant_parenthesis_removal_claims(&self) -> JavaDelimiterRemoval<'source> {
        if !self.is_recovery_free() || present(self.expression()).is_none() {
            return JavaDelimiterRemoval {
                open: None,
                close: None,
            };
        }
        JavaDelimiterRemoval {
            open: present(self.open_paren()).map(|token| {
                RemovalClaim::authorized::<JavaLanguage>(
                    NORMALIZATION_AUTHORITY,
                    SourceIdentity::Token(token.source_id()),
                    RemovalReason::RedundantDelimiter,
                )
            }),
            close: present(self.close_paren()).map(|token| {
                RemovalClaim::authorized::<JavaLanguage>(
                    NORMALIZATION_AUTHORITY,
                    SourceIdentity::Token(token.source_id()),
                    RemovalReason::RedundantDelimiter,
                )
            }),
        }
    }
}
