use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    CatchClause, DoWhileBodySyntax, DoWhileStatement, EmptyStatement, Expression, FinallyClause,
    ForBodySyntax, ForStatement, ForVariableSyntax, IfElseBranchSyntax, IfExpression,
    IfThenBranchSyntax, JumpExpression, KotlinSyntaxToken, NameExpression, ParenthesizedExpression,
    ThrowExpression, TryClause, TryExpression, WhenCondition, WhenConditionSyntax,
    WhenConditionValueSyntax, WhenEntry, WhenEntryBodySyntax, WhenEntryListElement,
    WhenEntryListElementSyntax, WhenExpression, WhenGuard, WhenSubject, WhileBodySyntax,
    WhileStatement,
};

use crate::helpers::blocks::join_hard_lines;
use crate::helpers::comments::{LeadingTrivia, TrailingTrivia, format_token};
use crate::helpers::lists::{CommaListItem, physical_comma_list_items};
use crate::helpers::recovery::{
    KotlinFormatDelimiter, KotlinFormatField, KotlinFormatListPart, format_optional_field,
    format_required_field, resolve_list_part, resolve_optional_field, resolve_required_delimiter,
    resolve_required_field,
};
use crate::rules::declarations::{
    format_destructuring_declaration, format_inline_modifier_prefix, format_modifier_prefix,
    format_type_annotation,
};
use crate::rules::names::format_name;
use crate::rules::types::format_type_reference;

use super::{format_expression, format_expression_with_leading};

pub(super) fn format_if_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &IfExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let has_condition = matches!(
        expression.condition(),
        Ok(jolt_kotlin_syntax::KotlinSyntaxField::Present(ref condition))
            if condition.first_token().is_some()
    );
    let keyword = format_required_token(expression.if_token(), doc, leading);
    let condition = format_required_field(expression.condition(), doc, |condition, doc| {
        format_control_flow_condition(doc, &condition)
    });
    let then_branch = resolve_required_field(expression.then_branch(), doc);
    let then_branch_is_nested_if = matches!(
        &then_branch,
        KotlinFormatField::Present(branch)
            if matches!(
                branch.classify(),
                Ok(IfThenBranchSyntax::Expression(Expression::IfExpression(_)))
            )
    );
    let then_branch_is_empty = matches!(
        &then_branch,
        KotlinFormatField::Present(branch)
            if matches!(branch.classify(), Ok(IfThenBranchSyntax::EmptyStatement(_)))
    );
    let then_branch = match then_branch {
        KotlinFormatField::Present(branch) => {
            let branch = format_if_then_branch(doc, branch);
            if then_branch_is_nested_if {
                let line = doc.hard_line();
                let branch = doc.concat([line, branch]);
                doc.indent(branch)
            } else if then_branch_is_empty {
                branch
            } else {
                let space = doc.space();
                doc.concat([space, branch])
            }
        }
        KotlinFormatField::Malformed(recovery) => recovery,
    };
    let else_branch = format_else_branch(doc, expression, then_branch_is_nested_if);
    let before_condition = if has_condition {
        doc.space()
    } else {
        Doc::nil()
    };
    doc.concat([
        keyword,
        before_condition,
        condition,
        then_branch,
        else_branch,
    ])
}

pub(super) fn format_when_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &WhenExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let keyword = format_required_token(expression.when_token(), doc, leading);
    let subject = match resolve_optional_field(expression.subject(), doc) {
        KotlinFormatField::Present(Some(subject)) => {
            let space = doc.space();
            let subject = format_when_subject(doc, &subject);
            doc.concat([space, subject])
        }
        KotlinFormatField::Present(None) => Doc::nil(),
        KotlinFormatField::Malformed(recovery) => recovery,
    };
    let open = resolve_required_delimiter(expression.open_brace(), doc);
    let close = resolve_required_delimiter(expression.close_brace(), doc);
    let entries = match resolve_required_field(expression.entries(), doc) {
        KotlinFormatField::Present(entries) => {
            let mut parts = Vec::new();
            for part in entries.parts() {
                let part = match resolve_list_part(part, doc) {
                    KotlinFormatListPart::Item(element) => format_when_entry_element(doc, element),
                    KotlinFormatListPart::Separator(token) => format_plain_token(doc, token),
                    KotlinFormatListPart::Malformed(recovery) => recovery,
                };
                parts.push(part);
            }
            parts
        }
        KotlinFormatField::Malformed(recovery) => vec![recovery],
    };
    let has_close = close.source().is_some();
    let entries = if entries.is_empty() {
        doc.hard_line()
    } else {
        let line = doc.hard_line();
        let entries = join_hard_lines(doc, entries);
        let entries = doc.concat([line, entries]);
        let entries = doc.indent(entries);
        if has_close {
            let trailing = doc.hard_line();
            doc.concat([entries, trailing])
        } else {
            entries
        }
    };
    let space = if open.source().is_some() {
        doc.space()
    } else {
        Doc::nil()
    };
    let open = format_delimiter(doc, open, LeadingTrivia::Preserve);
    let close = format_delimiter(doc, close, LeadingTrivia::Preserve);
    doc.concat([keyword, subject, space, open, entries, close])
}

pub(super) fn format_try_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &TryExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let has_block = matches!(
        expression.block(),
        Ok(jolt_kotlin_syntax::KotlinSyntaxField::Present(ref block))
            if block.first_token().is_some()
    );
    let keyword = format_required_token(expression.try_token(), doc, leading);
    let block = format_required_field(expression.block(), doc, |block, doc| {
        crate::rules::statements::format_block(doc, &block)
    });
    let clauses = match resolve_required_field(expression.clauses(), doc) {
        KotlinFormatField::Present(clauses) => doc.concat_list(|docs| {
            for part in clauses.parts() {
                let part = match resolve_list_part(part, docs) {
                    KotlinFormatListPart::Item(TryClause::CatchClause(clause)) => {
                        format_catch_clause(docs, &clause)
                    }
                    KotlinFormatListPart::Item(TryClause::FinallyClause(clause)) => {
                        format_finally_clause(docs, &clause)
                    }
                    KotlinFormatListPart::Item(TryClause::BogusTryClause(bogus)) => {
                        crate::helpers::recovery::format_malformed(&bogus, docs)
                    }
                    KotlinFormatListPart::Separator(token) => format_plain_token(docs, token),
                    KotlinFormatListPart::Malformed(recovery) => recovery,
                };
                if part != Doc::nil() {
                    let space = docs.space();
                    docs.push(space);
                }
                docs.push(part);
            }
        }),
        KotlinFormatField::Malformed(recovery) => recovery,
    };
    let before_block = if has_block { doc.space() } else { Doc::nil() };
    doc.concat([keyword, before_block, block, clauses])
}

pub(super) fn format_labeled_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &NameExpression<'source>,
    leading: LeadingTrivia,
) -> Option<Doc<'source>> {
    let at = match resolve_optional_field(expression.at(), doc) {
        KotlinFormatField::Present(Some(at)) => at,
        KotlinFormatField::Present(None) => return None,
        KotlinFormatField::Malformed(recovery) => return Some(recovery),
    };
    let label = format_required_token(expression.name(), doc, leading);
    let at = format_token(
        doc,
        &at,
        LeadingTrivia::Preserve,
        TrailingTrivia::RelocatedToEnclosingContext,
    );
    let labeled = match resolve_optional_field(expression.labeled_expression(), doc) {
        KotlinFormatField::Present(Some(labeled)) => {
            let space = doc.space();
            let labeled = format_expression_with_leading(
                doc,
                &labeled,
                LeadingTrivia::SuppressAlreadyHandled,
            );
            doc.concat([space, labeled])
        }
        KotlinFormatField::Present(None) => Doc::nil(),
        KotlinFormatField::Malformed(recovery) => recovery,
    };
    Some(doc.concat([label, at, labeled]))
}

pub(super) fn format_for_statement<'source>(
    doc: &mut DocBuilder<'source>,
    statement: &ForStatement<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let has_body = matches!(
        statement.body(),
        Ok(jolt_kotlin_syntax::KotlinSyntaxField::Present(ref body))
            if body.first_token().is_some()
    );
    let keyword = format_required_token(statement.for_token(), doc, leading);
    let open = resolve_required_delimiter(statement.open_paren(), doc);
    let close = resolve_required_delimiter(statement.close_paren(), doc);
    let variable = format_required_field(statement.variable(), doc, |variable, doc| {
        format_for_variable(doc, variable)
    });
    let in_token = format_required_token(statement.in_token(), doc, LeadingTrivia::Preserve);
    let iterable = format_required_field(statement.iterable(), doc, |iterable, doc| {
        format_expression(doc, &iterable)
    });
    let open = format_delimiter(doc, open, LeadingTrivia::Preserve);
    let close = format_delimiter(doc, close, LeadingTrivia::Preserve);
    let space = doc.space();
    let header = doc.concat([open, variable, space, in_token, space, iterable, close]);
    let body = format_required_field(statement.body(), doc, |body, doc| {
        format_for_body(doc, body)
    });
    let body_is_empty = matches!(
        statement.body(),
        Ok(jolt_kotlin_syntax::KotlinSyntaxField::Present(ref body))
            if matches!(body.classify(), Ok(ForBodySyntax::EmptyStatement(_)))
    );
    let before_header = doc.space();
    let before_body = if has_body && !body_is_empty {
        doc.space()
    } else {
        Doc::nil()
    };
    doc.concat([keyword, before_header, header, before_body, body])
}

fn format_for_variable<'source>(
    doc: &mut DocBuilder<'source>,
    variable: jolt_kotlin_syntax::ForVariable<'source>,
) -> Doc<'source> {
    let modifiers = format_inline_modifier_prefix(doc, variable.modifiers());
    let binding = format_required_field(variable.binding(), doc, |binding, doc| {
        match binding.classify() {
            Ok(ForVariableSyntax::Name(name)) => format_name(doc, &name),
            Ok(ForVariableSyntax::Destructuring(declaration)) => {
                format_destructuring_declaration(doc, &declaration)
            }
            Err(error) => {
                doc.block_on_invariant(error.to_string());
                Doc::nil()
            }
        }
    });
    let ty = format_type_annotation(doc, variable.colon(), variable.r#type());
    doc.concat([modifiers, binding, ty])
}

pub(super) fn format_while_statement<'source>(
    doc: &mut DocBuilder<'source>,
    statement: &WhileStatement<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let has_condition = matches!(
        statement.condition(),
        Ok(jolt_kotlin_syntax::KotlinSyntaxField::Present(ref condition))
            if condition.first_token().is_some()
    );
    let has_body = matches!(
        statement.body(),
        Ok(jolt_kotlin_syntax::KotlinSyntaxField::Present(ref body))
            if body.first_token().is_some()
    );
    let keyword = format_required_token(statement.while_token(), doc, leading);
    let condition = format_required_field(statement.condition(), doc, |condition, doc| {
        format_control_flow_condition(doc, &condition)
    });
    let body = format_required_field(statement.body(), doc, |body, doc| {
        format_while_body(doc, body)
    });
    let body_is_empty = matches!(
        statement.body(),
        Ok(jolt_kotlin_syntax::KotlinSyntaxField::Present(ref body))
            if matches!(body.classify(), Ok(WhileBodySyntax::EmptyStatement(_)))
    );
    let space = doc.space();
    let before_condition = if has_condition { space } else { Doc::nil() };
    let before_body = if has_body && !body_is_empty {
        space
    } else {
        Doc::nil()
    };
    doc.concat([keyword, before_condition, condition, before_body, body])
}

pub(super) fn format_do_while_statement<'source>(
    doc: &mut DocBuilder<'source>,
    statement: &DoWhileStatement<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let has_body = matches!(
        statement.body(),
        Ok(jolt_kotlin_syntax::KotlinSyntaxField::Present(ref body))
            if body.first_token().is_some()
    );
    let has_while = matches!(
        statement.while_token(),
        Ok(jolt_kotlin_syntax::KotlinSyntaxField::Present(_))
    );
    let has_condition = matches!(
        statement.condition(),
        Ok(jolt_kotlin_syntax::KotlinSyntaxField::Present(ref condition))
            if condition.first_token().is_some()
    );
    let do_token = format_required_token(statement.do_token(), doc, leading);
    let body = format_required_field(statement.body(), doc, |body, doc| {
        format_do_while_body(doc, body)
    });
    let body_is_empty = matches!(
        statement.body(),
        Ok(jolt_kotlin_syntax::KotlinSyntaxField::Present(ref body))
            if matches!(body.classify(), Ok(DoWhileBodySyntax::EmptyStatement(_)))
    );
    let while_token = format_required_token(statement.while_token(), doc, LeadingTrivia::Preserve);
    let condition = format_required_field(statement.condition(), doc, |condition, doc| {
        format_control_flow_condition(doc, &condition)
    });
    let after_do = if has_body && !body_is_empty {
        doc.space()
    } else {
        Doc::nil()
    };
    let before_while = if has_while { doc.space() } else { Doc::nil() };
    let before_condition = if has_condition {
        doc.space()
    } else {
        Doc::nil()
    };
    doc.concat([
        do_token,
        after_do,
        body,
        before_while,
        while_token,
        before_condition,
        condition,
    ])
}

pub(super) fn format_jump_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &JumpExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let keyword = format_required_token(expression.keyword(), doc, leading);
    let label = format_optional_field(expression.label(), doc, |label, doc| {
        let at = format_required_field(label.at(), doc, |at, doc| format_plain_token(doc, at));
        let label = format_required_field(label.label(), doc, |label, doc| {
            format_plain_token(doc, label)
        });
        doc.concat([at, label])
    });
    let value = match resolve_optional_field(expression.expression(), doc) {
        KotlinFormatField::Present(Some(value)) => {
            let space = doc.space();
            let value = format_expression(doc, &value);
            doc.concat([space, value])
        }
        KotlinFormatField::Present(None) => Doc::nil(),
        KotlinFormatField::Malformed(recovery) => recovery,
    };
    doc.concat([keyword, label, value])
}

pub(super) fn format_throw_expression<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &ThrowExpression<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    let keyword = format_required_token(expression.throw_token(), doc, leading);
    let value = format_required_field(expression.expression(), doc, |value, doc| {
        let space = doc.space();
        let value = format_expression(doc, &value);
        doc.concat([space, value])
    });
    doc.concat([keyword, value])
}

fn format_control_flow_condition<'source>(
    doc: &mut DocBuilder<'source>,
    condition: &ParenthesizedExpression<'source>,
) -> Doc<'source> {
    let open = resolve_required_delimiter(condition.open_paren(), doc);
    let close = resolve_required_delimiter(condition.close_paren(), doc);
    let expression = format_required_field(condition.expression(), doc, |expression, doc| {
        format_expression(doc, &expression)
    });
    let open = format_delimiter(doc, open, LeadingTrivia::Preserve);
    let close = format_delimiter(doc, close, LeadingTrivia::Preserve);
    let soft_line = doc.soft_line();
    let inner = doc.concat([soft_line, expression]);
    let inner = doc.indent(inner);
    let trailing = doc.soft_line();
    let contents = doc.concat([open, inner, trailing, close]);
    doc.group(contents)
}

fn format_else_branch<'source>(
    doc: &mut DocBuilder<'source>,
    expression: &IfExpression<'source>,
    starts_after_broken_then: bool,
) -> Doc<'source> {
    let else_token = match resolve_optional_field(expression.else_token(), doc) {
        KotlinFormatField::Present(Some(token)) => token,
        KotlinFormatField::Present(None) => return Doc::nil(),
        KotlinFormatField::Malformed(recovery) => return recovery,
    };
    let token = format_plain_token(doc, else_token);
    let branch = match resolve_optional_field(expression.else_branch(), doc) {
        KotlinFormatField::Present(Some(branch)) => {
            let branch_is_empty =
                matches!(branch.classify(), Ok(IfElseBranchSyntax::EmptyStatement(_)));
            let branch = format_if_else_branch(doc, branch);
            if branch_is_empty {
                branch
            } else {
                let space = doc.space();
                doc.concat([space, branch])
            }
        }
        KotlinFormatField::Present(None) => Doc::nil(),
        KotlinFormatField::Malformed(recovery) => recovery,
    };
    let separator = if starts_after_broken_then {
        doc.hard_line()
    } else {
        doc.space()
    };
    doc.concat([separator, token, branch])
}

fn format_when_subject<'source>(
    doc: &mut DocBuilder<'source>,
    subject: &WhenSubject<'source>,
) -> Doc<'source> {
    let open = resolve_required_delimiter(subject.open_paren(), doc);
    let close = resolve_required_delimiter(subject.close_paren(), doc);
    let val_token = format_optional_field(subject.val_token(), doc, |token, doc| {
        let token = format_plain_token(doc, token);
        let space = doc.space();
        doc.concat([token, space])
    });
    let name = format_optional_field(subject.name(), doc, |name, doc| format_name(doc, &name));
    let assign = format_optional_field(subject.assign(), doc, |assign, doc| {
        let before = doc.space();
        let assign = format_plain_token(doc, assign);
        let after = doc.space();
        doc.concat([before, assign, after])
    });
    let expression = format_required_field(subject.expression(), doc, |expression, doc| {
        format_expression(doc, &expression)
    });
    let open = format_delimiter(doc, open, LeadingTrivia::Preserve);
    let close = format_delimiter(doc, close, LeadingTrivia::Preserve);
    doc.concat([open, val_token, name, assign, expression, close])
}

fn format_when_entry<'source>(
    doc: &mut DocBuilder<'source>,
    entry: &WhenEntry<'source>,
) -> Doc<'source> {
    let has_body = matches!(
        entry.body(),
        Ok(jolt_kotlin_syntax::KotlinSyntaxField::Present(ref body))
            if body.first_token().is_some()
    );
    let else_token = resolve_optional_field(entry.else_token(), doc);
    let label = match else_token {
        KotlinFormatField::Present(Some(token)) => format_plain_token(doc, token),
        KotlinFormatField::Present(None) => format_when_conditions(doc, entry),
        KotlinFormatField::Malformed(recovery) => recovery,
    };
    let guard = format_optional_field(entry.guard(), doc, |guard, doc| {
        let space = doc.space();
        let guard = format_when_guard(doc, &guard);
        doc.concat([space, guard])
    });
    let arrow = format_required_field(entry.arrow(), doc, |arrow, doc| {
        let space = doc.space();
        let arrow = format_plain_token(doc, arrow);
        doc.concat([space, arrow])
    });
    let body = format_required_field(entry.body(), doc, |body, doc| {
        format_when_entry_body(doc, body)
    });
    let body = if has_body {
        let line = doc.line();
        let body = doc.concat([line, body]);
        doc.indent(body)
    } else {
        body
    };
    let contents = doc.concat([label, guard, arrow, body]);
    doc.group(contents)
}

fn format_when_entry_element<'source>(
    doc: &mut DocBuilder<'source>,
    element: WhenEntryListElement<'source>,
) -> Doc<'source> {
    match element.classify() {
        Ok(WhenEntryListElementSyntax::Entry(entry)) => format_when_entry(doc, &entry),
        Ok(WhenEntryListElementSyntax::Terminator(token)) => format_plain_token(doc, token),
        Err(error) => {
            doc.block_on_invariant(error.to_string());
            Doc::nil()
        }
    }
}

fn format_when_conditions<'source>(
    doc: &mut DocBuilder<'source>,
    entry: &WhenEntry<'source>,
) -> Doc<'source> {
    match resolve_required_field(entry.conditions(), doc) {
        KotlinFormatField::Present(conditions) => {
            let items = physical_comma_list_items(doc, conditions.parts(), |doc, condition| {
                CommaListItem {
                    doc: match condition {
                        WhenConditionSyntax::WhenCondition(condition) => {
                            format_when_condition(doc, &condition)
                        }
                        WhenConditionSyntax::WhenGuard(guard) => format_when_guard(doc, &guard),
                        WhenConditionSyntax::BogusWhenCondition(bogus) => {
                            crate::helpers::recovery::format_malformed(&bogus, doc)
                        }
                    },
                    comma: None,
                }
            });
            let mut items = items.into_iter().peekable();
            doc.concat_list(|docs| {
                while let Some(item) = items.next() {
                    docs.push(item.doc);
                    if let Some(comma) = item.comma {
                        let comma = format_token(
                            docs,
                            &comma,
                            LeadingTrivia::Preserve,
                            TrailingTrivia::BeforeSpaceIfComments,
                        );
                        docs.push(comma);
                    }
                    if items.peek().is_some() {
                        let space = docs.space();
                        docs.push(space);
                    }
                }
            })
        }
        KotlinFormatField::Malformed(recovery) => recovery,
    }
}

fn format_when_condition<'source>(
    doc: &mut DocBuilder<'source>,
    condition: &WhenCondition<'source>,
) -> Doc<'source> {
    let keyword = resolve_optional_field(condition.keyword(), doc);
    let value = match resolve_required_field(condition.value(), doc) {
        KotlinFormatField::Present(value) => format_when_condition_value(doc, value),
        KotlinFormatField::Malformed(recovery) => recovery,
    };
    match keyword {
        KotlinFormatField::Present(Some(keyword)) => {
            let keyword = format_plain_token(doc, keyword);
            let space = doc.space();
            doc.concat([keyword, space, value])
        }
        KotlinFormatField::Present(None) => value,
        KotlinFormatField::Malformed(recovery) => doc.concat([recovery, value]),
    }
}

fn format_when_condition_value<'source>(
    doc: &mut DocBuilder<'source>,
    value: jolt_kotlin_syntax::WhenConditionValue<'source>,
) -> Doc<'source> {
    match value.classify() {
        Ok(WhenConditionValueSyntax::TypeReference(ty)) => {
            crate::rules::types::format_type_reference(doc, &ty)
        }
        Ok(WhenConditionValueSyntax::Expression(expression)) => format_expression(doc, &expression),
        Err(error) => {
            doc.block_on_invariant(error.to_string());
            Doc::nil()
        }
    }
}

fn format_when_guard<'source>(
    doc: &mut DocBuilder<'source>,
    guard: &WhenGuard<'source>,
) -> Doc<'source> {
    let if_token = format_required_token(guard.if_token(), doc, LeadingTrivia::Preserve);
    let expression = format_required_field(guard.expression(), doc, |expression, doc| {
        let space = doc.space();
        let expression = format_expression(doc, &expression);
        doc.concat([space, expression])
    });
    doc.concat([if_token, expression])
}

fn format_catch_clause<'source>(
    doc: &mut DocBuilder<'source>,
    clause: &CatchClause<'source>,
) -> Doc<'source> {
    let has_parameter = matches!(
        clause.parameter(),
        Ok(jolt_kotlin_syntax::KotlinSyntaxField::Present(ref parameter))
            if parameter.first_token().is_some()
    );
    let has_block = matches!(
        clause.block(),
        Ok(jolt_kotlin_syntax::KotlinSyntaxField::Present(ref block))
            if block.first_token().is_some()
    );
    let keyword = format_required_token(clause.catch_token(), doc, LeadingTrivia::Preserve);
    let parameter = format_required_field(clause.parameter(), doc, |parameter, doc| {
        format_catch_parameter(doc, &parameter)
    });
    let block = format_required_field(clause.block(), doc, |block, doc| {
        crate::rules::statements::format_block(doc, &block)
    });
    let before_parameter = if has_parameter {
        doc.space()
    } else {
        Doc::nil()
    };
    let before_block = if has_block { doc.space() } else { Doc::nil() };
    doc.concat([keyword, before_parameter, parameter, before_block, block])
}

fn format_catch_parameter<'source>(
    doc: &mut DocBuilder<'source>,
    parameter: &jolt_kotlin_syntax::CatchParameter<'source>,
) -> Doc<'source> {
    let open = format_required_token(parameter.open_paren(), doc, LeadingTrivia::Preserve);
    let modifiers = format_modifier_prefix(doc, parameter.modifiers());
    let name = format_required_field(parameter.name(), doc, |name, doc| format_name(doc, &name));
    let colon = format_required_token(parameter.colon(), doc, LeadingTrivia::Preserve);
    let ty = format_required_field(parameter.r#type(), doc, |ty, doc| {
        let space = doc.space();
        let ty = format_type_reference(doc, &ty);
        doc.concat([space, ty])
    });
    let close = format_required_token(parameter.close_paren(), doc, LeadingTrivia::Preserve);
    doc.concat([open, modifiers, name, colon, ty, close])
}

fn format_finally_clause<'source>(
    doc: &mut DocBuilder<'source>,
    clause: &FinallyClause<'source>,
) -> Doc<'source> {
    let has_block = matches!(
        clause.block(),
        Ok(jolt_kotlin_syntax::KotlinSyntaxField::Present(ref block))
            if block.first_token().is_some()
    );
    let keyword = format_required_token(clause.finally_token(), doc, LeadingTrivia::Preserve);
    let block = format_required_field(clause.block(), doc, |block, doc| {
        crate::rules::statements::format_block(doc, &block)
    });
    let before_block = if has_block { doc.space() } else { Doc::nil() };
    doc.concat([keyword, before_block, block])
}

fn format_if_then_branch<'source>(
    doc: &mut DocBuilder<'source>,
    branch: jolt_kotlin_syntax::IfThenBranchValue<'source>,
) -> Doc<'source> {
    match branch.classify() {
        Ok(IfThenBranchSyntax::Expression(expression)) => format_expression(doc, &expression),
        Ok(IfThenBranchSyntax::Block(block)) => crate::rules::statements::format_block(doc, &block),
        Ok(IfThenBranchSyntax::EmptyStatement(statement)) => {
            format_empty_statement(doc, &statement)
        }
        Err(error) => {
            doc.block_on_invariant(error.to_string());
            Doc::nil()
        }
    }
}

fn format_if_else_branch<'source>(
    doc: &mut DocBuilder<'source>,
    branch: jolt_kotlin_syntax::IfElseBranchValue<'source>,
) -> Doc<'source> {
    match branch.classify() {
        Ok(IfElseBranchSyntax::Expression(expression)) => format_expression(doc, &expression),
        Ok(IfElseBranchSyntax::Block(block)) => crate::rules::statements::format_block(doc, &block),
        Ok(IfElseBranchSyntax::EmptyStatement(statement)) => {
            format_empty_statement(doc, &statement)
        }
        Err(error) => {
            doc.block_on_invariant(error.to_string());
            Doc::nil()
        }
    }
}

fn format_for_body<'source>(
    doc: &mut DocBuilder<'source>,
    body: jolt_kotlin_syntax::ForBodyValue<'source>,
) -> Doc<'source> {
    match body.classify() {
        Ok(ForBodySyntax::Expression(expression)) => format_expression(doc, &expression),
        Ok(ForBodySyntax::Block(block)) => crate::rules::statements::format_block(doc, &block),
        Ok(ForBodySyntax::EmptyStatement(statement)) => format_empty_statement(doc, &statement),
        Err(error) => {
            doc.block_on_invariant(error.to_string());
            Doc::nil()
        }
    }
}

fn format_while_body<'source>(
    doc: &mut DocBuilder<'source>,
    body: jolt_kotlin_syntax::WhileBodyValue<'source>,
) -> Doc<'source> {
    match body.classify() {
        Ok(WhileBodySyntax::Expression(expression)) => format_expression(doc, &expression),
        Ok(WhileBodySyntax::Block(block)) => crate::rules::statements::format_block(doc, &block),
        Ok(WhileBodySyntax::EmptyStatement(statement)) => format_empty_statement(doc, &statement),
        Err(error) => {
            doc.block_on_invariant(error.to_string());
            Doc::nil()
        }
    }
}

fn format_do_while_body<'source>(
    doc: &mut DocBuilder<'source>,
    body: jolt_kotlin_syntax::DoWhileBodyValue<'source>,
) -> Doc<'source> {
    match body.classify() {
        Ok(DoWhileBodySyntax::Expression(expression)) => format_expression(doc, &expression),
        Ok(DoWhileBodySyntax::Block(block)) => crate::rules::statements::format_block(doc, &block),
        Ok(DoWhileBodySyntax::EmptyStatement(statement)) => format_empty_statement(doc, &statement),
        Err(error) => {
            doc.block_on_invariant(error.to_string());
            Doc::nil()
        }
    }
}

fn format_empty_statement<'source>(
    doc: &mut DocBuilder<'source>,
    statement: &EmptyStatement<'source>,
) -> Doc<'source> {
    format_required_field(statement.terminator(), doc, |terminator, doc| {
        format_plain_token(doc, terminator)
    })
}

fn format_when_entry_body<'source>(
    doc: &mut DocBuilder<'source>,
    body: jolt_kotlin_syntax::WhenEntryBody<'source>,
) -> Doc<'source> {
    format_required_field(body.value(), doc, |value, doc| match value.classify() {
        Ok(WhenEntryBodySyntax::Expression(expression)) => format_expression(doc, &expression),
        Ok(WhenEntryBodySyntax::Block(block)) => {
            crate::rules::statements::format_block(doc, &block)
        }
        Err(error) => {
            doc.block_on_invariant(error.to_string());
            Doc::nil()
        }
    })
}

fn format_required_token<'source>(
    field: Result<
        jolt_kotlin_syntax::KotlinSyntaxField<'source, KotlinSyntaxToken<'source>>,
        jolt_kotlin_syntax::KotlinSyntaxInvariantError,
    >,
    doc: &mut DocBuilder<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    format_required_field(field, doc, |token, doc| {
        format_token(doc, &token, leading, TrailingTrivia::Preserve)
    })
}

fn format_plain_token<'source>(
    doc: &mut DocBuilder<'source>,
    token: KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    format_token(
        doc,
        &token,
        LeadingTrivia::Preserve,
        TrailingTrivia::Preserve,
    )
}

fn format_delimiter<'source>(
    doc: &mut DocBuilder<'source>,
    delimiter: KotlinFormatDelimiter<'source>,
    leading: LeadingTrivia,
) -> Doc<'source> {
    match delimiter {
        KotlinFormatDelimiter::Source(token) => {
            format_token(doc, &token, leading, TrailingTrivia::Preserve)
        }
        KotlinFormatDelimiter::Recovery(recovery) => recovery,
    }
}
