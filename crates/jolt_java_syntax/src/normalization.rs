use jolt_syntax::{
    NormalizedToken, RemovalReason, ReorderReason, ReplacementClaim, SourceIdentity, SynthesisClaim,
};
pub use jolt_syntax::{RemovalClaim, ReorderClaim};

use crate::language::{JavaLanguage, NORMALIZATION_AUTHORITY};
use crate::{
    AnnotationArrayInitializer, ArrayInitializer, BasicForStatement, ClassBodyMember, DoStatement,
    EmptyDeclaration, EmptyStatement, EnhancedForStatement, EnumBody, Expression, Guard,
    IfStatement, ImportDeclaration, JavaSyntaxField, JavaSyntaxListPart, JavaSyntaxToken,
    JavaSyntaxView, LambdaExpression, LambdaParameter, ModifierList, ModuleDeclaration,
    ParameterModifierList, ParenthesizedExpression, RequiresDirective, ResourceSpecification,
    Statement, WhileStatement,
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

impl<'source> EmptyStatement<'source> {
    /// Authorizes omission of this redundant empty-statement separator.
    #[must_use]
    pub fn separator_removal_claim(&self) -> Option<RemovalClaim<'source>> {
        if !self.is_recovery_free() {
            return None;
        }
        let token = present(self.semicolon())?;
        Some(RemovalClaim::authorized::<JavaLanguage>(
            NORMALIZATION_AUTHORITY,
            SourceIdentity::Token(token.source_id()),
            RemovalReason::RedundantSeparator,
        ))
    }
}

fn control_body_brace_claims<'source>(
    owner: &impl JavaSyntaxView<'source>,
    body: Statement<'source>,
) -> Option<JavaDelimiterSynthesis<'source>> {
    if matches!(body, Statement::Block(_)) {
        return None;
    }
    delimiter_synthesis(
        owner,
        NormalizedToken::OpenBlockBrace,
        NormalizedToken::CloseBlockBrace,
    )
}

impl<'source> IfStatement<'source> {
    #[must_use]
    pub fn then_block_brace_claims(&self) -> Option<JavaDelimiterSynthesis<'source>> {
        control_body_brace_claims(self, present(self.then_branch())?)
    }

    #[must_use]
    pub fn else_block_brace_claims(&self) -> Option<JavaDelimiterSynthesis<'source>> {
        control_body_brace_claims(self, present(self.else_branch())?)
    }
}

macro_rules! impl_control_body_brace_claims {
    ($($owner:ident),+ $(,)?) => {
        $(
            impl<'source> $owner<'source> {
                #[must_use]
                pub fn body_block_brace_claims(
                    &self,
                ) -> Option<JavaDelimiterSynthesis<'source>> {
                    control_body_brace_claims(self, present(self.body())?)
                }
            }
        )+
    };
}

impl_control_body_brace_claims!(
    WhileStatement,
    DoStatement,
    BasicForStatement,
    EnhancedForStatement,
);

fn reorder_claim<'source>(
    owner: &impl JavaSyntaxView<'source>,
    reason: ReorderReason,
) -> Option<ReorderClaim<'source>> {
    if !owner.is_recovery_free() {
        return None;
    }
    let syntax = owner.syntax_node()?;
    let anchor = syntax
        .first_token()
        .or_else(|| syntax.last_token())?
        .source_id();
    Some(ReorderClaim::authorized::<JavaLanguage>(
        NORMALIZATION_AUTHORITY,
        anchor,
        reason,
    ))
}

macro_rules! impl_reorder_claim {
    ($owner:ident, $reason:ident) => {
        impl<'source> $owner<'source> {
            #[must_use]
            pub fn canonical_reorder_claim(&self) -> Option<ReorderClaim<'source>> {
                reorder_claim(self, ReorderReason::$reason)
            }
        }
    };
}

impl_reorder_claim!(ImportDeclaration, Imports);
impl_reorder_claim!(ModifierList, Modifiers);
impl_reorder_claim!(ParameterModifierList, Modifiers);

impl<'source> ModuleDeclaration<'source> {
    #[must_use]
    pub fn directive_reorder_claim(&self) -> Option<ReorderClaim<'source>> {
        reorder_claim(self, ReorderReason::ModuleDirectives)
    }
}

impl<'source> RequiresDirective<'source> {
    #[must_use]
    pub fn modifier_reorder_claim(&self) -> Option<ReorderClaim<'source>> {
        reorder_claim(self, ReorderReason::RequiresModifiers)
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
        let (Some(open), Some(close)) = (present(self.open_paren()), present(self.close_paren()))
        else {
            return JavaDelimiterRemoval {
                open: None,
                close: None,
            };
        };
        JavaDelimiterRemoval {
            open: Some(RemovalClaim::authorized::<JavaLanguage>(
                NORMALIZATION_AUTHORITY,
                SourceIdentity::Token(open.source_id()),
                RemovalReason::RedundantDelimiter,
            )),
            close: Some(RemovalClaim::authorized::<JavaLanguage>(
                NORMALIZATION_AUTHORITY,
                SourceIdentity::Token(close.source_id()),
                RemovalReason::RedundantDelimiter,
            )),
        }
    }
}

impl<'source> EmptyDeclaration<'source> {
    /// Authorizes omission of this redundant empty-declaration separator.
    #[must_use]
    pub fn separator_removal_claim(&self) -> Option<RemovalClaim<'source>> {
        if !self.is_recovery_free() {
            return None;
        }
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
        if !values.is_recovery_free()
            || !values
                .parts()
                .any(|part| matches!(part, Ok(JavaSyntaxListPart::Item(_))))
        {
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
        if !values.is_recovery_free()
            || !values
                .parts()
                .any(|part| matches!(part, Ok(JavaSyntaxListPart::Item(_))))
        {
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
    /// single untyped, unmodified parameter.
    #[must_use]
    pub fn simple_parameter_parenthesis_removal(
        &self,
    ) -> Option<JavaLambdaParameterParenthesisRemoval<'source>> {
        if !self.is_recovery_free()
            || present(self.arrow()).is_none()
            || present(self.body()).is_none()
        {
            return None;
        }
        let parameters = present(self.parameters())?;
        let mut parts = parameters.parts();
        let parameter = match parts.next()?.ok()? {
            JavaSyntaxListPart::Item(parameter) => parameter,
            JavaSyntaxListPart::Separator(_)
            | JavaSyntaxListPart::Missing(_)
            | JavaSyntaxListPart::Malformed(_) => return None,
        };
        let modifiers = present(parameter.modifiers())?;
        let varargs_annotations = present(parameter.varargs_annotations())?;
        if parts.next().is_some()
            || !matches!(parameter.r#type(), Ok(JavaSyntaxField::Missing(_)))
            || !matches!(parameter.ellipsis(), Ok(JavaSyntaxField::Missing(_)))
            || !modifiers.is_recovery_free()
            || modifiers.first_token().is_some()
            || !varargs_annotations.is_recovery_free()
            || varargs_annotations.first_token().is_some()
        {
            return None;
        }
        let open = present(self.open_paren())?;
        let close = present(self.close_paren())?;
        Some(JavaLambdaParameterParenthesisRemoval {
            parameter,
            open: RemovalClaim::authorized::<JavaLanguage>(
                NORMALIZATION_AUTHORITY,
                SourceIdentity::Token(open.source_id()),
                RemovalReason::RedundantDelimiter,
            ),
            close: RemovalClaim::authorized::<JavaLanguage>(
                NORMALIZATION_AUTHORITY,
                SourceIdentity::Token(close.source_id()),
                RemovalReason::RedundantDelimiter,
            ),
        })
    }
}

pub struct JavaLambdaParameterParenthesisRemoval<'source> {
    pub parameter: LambdaParameter<'source>,
    pub open: RemovalClaim<'source>,
    pub close: RemovalClaim<'source>,
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
