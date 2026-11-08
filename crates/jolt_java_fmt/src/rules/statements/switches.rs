//! Switch statements and expressions, their blocks, groups, rules, labels
//! and guards.

use jolt_fmt::{Doc, ListItem, cat, indent};
use jolt_java_syntax::{
    BlockItem, BlockStatement, CaseConstant, CasePattern, Expression, Guard, SwitchBlock,
    SwitchBlockStatementGroup, SwitchExpression, SwitchLabel, SwitchLabelColonList,
    SwitchLabelItemList, SwitchRule, SwitchRuleBody, SwitchStatement,
};
use jolt_syntax::{AstNode, ListElement, SyntaxElement, SyntaxNode, SyntaxToken};

use super::statement_items;
use crate::JavaFormatter;
use crate::helpers::bodies::{braced_body, statement_separator};
use crate::helpers::lists::parenthesized;
use crate::helpers::{after_token, spaced_node};

pub(crate) fn switch_statement<'a>(f: &mut JavaFormatter<'a>, node: SwitchStatement<'a>) -> Doc {
    switch(
        f,
        node.syntax(),
        node.switch_keyword(),
        node.selector(),
        [node.open_paren(), node.close_paren()],
        node.body(),
    )
}

pub(crate) fn switch_expression<'a>(f: &mut JavaFormatter<'a>, node: SwitchExpression<'a>) -> Doc {
    switch(
        f,
        node.syntax(),
        node.switch_keyword(),
        node.selector(),
        [node.open_paren(), node.close_paren()],
        node.body(),
    )
}

/// `switch (selector) {` … `}`.
fn switch<'a>(
    f: &mut JavaFormatter<'a>,
    owner: SyntaxNode<'a>,
    keyword: Option<SyntaxToken<'a>>,
    selector: Option<Expression<'a>>,
    [open, close]: [Option<SyntaxToken<'a>>; 2],
    body: Option<SwitchBlock<'a>>,
) -> Doc {
    let keyword = f.token_opt(keyword);
    let selector = f.fmt_opt(selector);
    let header = parenthesized(f, owner, open, selector, close);
    let body = f.fmt_opt(body);
    cat![f; keyword, f.space(), header, f.space(), body]
}

/// `{`, one entry per line one indent deeper (no blank lines), `}`.
pub(crate) fn switch_block<'a>(f: &mut JavaFormatter<'a>, node: SwitchBlock<'a>) -> Doc {
    let entries: Vec<_> = node
        .entries()
        .map(|entries| {
            entries
                .items()
                .map(|entry| ListItem::Format(entry.syntax()))
                .collect()
        })
        .unwrap_or_default();
    braced_body(
        f,
        node.syntax(),
        node.open_brace(),
        entries,
        node.close_brace(),
        |f, _| f.hard_line(),
    )
}

/// Each `case …:` label on its own line, then the statements one indent
/// deeper. A single label whose single statement is a block keeps the block
/// on the label line: `case 1: {`.
pub(crate) fn switch_block_statement_group<'a>(
    f: &mut JavaFormatter<'a>,
    node: SwitchBlockStatementGroup<'a>,
) -> Doc {
    let mut labels = Vec::new();
    let mut label_count = 0;
    for element in node
        .labels()
        .into_iter()
        .flat_map(SwitchLabelColonList::elements)
    {
        let label = match element {
            ListElement::Item(label) => f.fmt(label),
            ListElement::Bogus(bogus) => f.node(bogus),
            ListElement::Separator(colon) => {
                labels.push(f.token(colon));
                continue;
            }
            ListElement::Missing => continue,
        };
        if label_count > 0 {
            labels.push(f.hard_line());
        }
        labels.push(label);
        label_count += 1;
    }
    let labels = f.concat(labels);
    let mut statements = statement_items(f, node.syntax(), node.statements());
    // N5 keeps one `;` of a group that has nothing else when another group
    // follows: removing it would merge the labels into that group.
    let last = node
        .syntax()
        .parent()
        .and_then(|entries| entries.children().last())
        .is_none_or(|entry| entry == SyntaxElement::Node(node.syntax()));
    if !last
        && statements
            .iter()
            .all(|item| matches!(item, ListItem::Elide(_)))
        && let Some(ListItem::Elide(first)) = statements.first().copied()
    {
        statements[0] = ListItem::Format(first);
    }
    // The statements that print: a removed `;` prints nothing, except
    // verbatim in a formatter-ignore region.
    let mut printed = statements.iter().filter_map(|&item| match item {
        ListItem::Elide(node) if !f.in_ignore_region(node) => None,
        ListItem::Format(node) | ListItem::Elide(node) => Some(node),
    });
    if label_count == 1
        && let (Some(statement), None) = (printed.next(), printed.next())
        && is_inline_block(f, statement)
    {
        let block = f.node(statement);
        return cat![f; labels, f.space(), block];
    }
    let statements = f.item_list(statements, statement_separator);
    if statements.is_nil() {
        return labels;
    }
    let statements = indent![f; f.hard_line(), statements];
    cat![f; labels, statements]
}

/// A block statement that may share the label line: a plain block outside
/// formatter-ignore with no leading comment that starts a line. (A blank
/// line before it in the source does not count: the other layout would drop
/// it, so the next pass would choose this one.)
fn is_inline_block<'a>(f: &JavaFormatter<'a>, statement: SyntaxNode<'a>) -> bool {
    let element = SyntaxElement::Node(statement);
    BlockStatement::cast(statement)
        .and_then(BlockStatement::item)
        .is_some_and(|item| matches!(item, BlockItem::Block(_)))
        && !f.leading_comments_break(element)
        && !f.in_ignore_region(statement)
}

/// `label -> body`. An expression body that does not fit continues on the
/// next line one indent deeper.
pub(crate) fn switch_rule<'a>(f: &mut JavaFormatter<'a>, node: SwitchRule<'a>) -> Doc {
    let label = f.fmt_opt(node.label());
    let body = match node.body() {
        Some(SwitchRuleBody::Expression(expression)) => {
            let arrow = f.token_opt(node.arrow());
            let expression = f.fmt(expression);
            let semicolon = f.token_opt(node.semicolon());
            let body = indent![f; f.line(), expression, semicolon];
            let body = f.group(body);
            cat![f; arrow, body]
        }
        // A `throw` or a block (see `after_token`). H1 moves a comment
        // before a block's `{` into it, so only inline comments lead it.
        body => {
            let body = body.map(|body| (SyntaxElement::Node(body.syntax()), f.fmt(body)));
            let semicolon = f.token_opt(node.semicolon());
            after_token(f, node.arrow(), body, semicolon)
        }
    };
    cat![f; label, f.space(), body]
}

/// `case a, b when guard` or `default`. Items that do not fit break one per
/// line, one indent deeper.
pub(crate) fn switch_label<'a>(f: &mut JavaFormatter<'a>, node: SwitchLabel<'a>) -> Doc {
    let keyword = f.token_opt(node.keyword());
    let mut items = Vec::new();
    for element in node
        .items()
        .into_iter()
        .flat_map(SwitchLabelItemList::elements)
    {
        match element {
            ListElement::Item(item) => items.push(f.element(item.element())),
            ListElement::Bogus(bogus) => items.push(f.node(bogus)),
            ListElement::Separator(comma) => items.extend([f.token(comma), f.line()]),
            ListElement::Missing => {}
        }
    }
    let items = if items.is_empty() {
        Doc::NIL
    } else {
        let items = f.concat(items);
        let items = f.indent(items);
        let items = f.group(items);
        cat![f; f.space(), items]
    };
    let guard = spaced_node(f, node.guard());
    cat![f; keyword, items, guard]
}

pub(crate) fn case_constant<'a>(f: &mut JavaFormatter<'a>, node: CaseConstant<'a>) -> Doc {
    f.fmt_opt(node.expression())
}

pub(crate) fn case_pattern<'a>(f: &mut JavaFormatter<'a>, node: CasePattern<'a>) -> Doc {
    f.fmt_opt(node.pattern())
}

/// `when condition`. Parentheses around the condition are removed (N6)
/// unless a comment is inside them.
pub(crate) fn guard<'a>(f: &mut JavaFormatter<'a>, node: Guard<'a>) -> Doc {
    let when = f.token_opt(node.when_keyword());
    let condition = f.fmt_opt(node.condition());
    let (open, close) = (node.open_paren(), node.close_paren());
    let condition = match (open, close) {
        (None, None) => condition,
        (Some(open), Some(close)) if removable_parentheses(f, node, open, close) => condition,
        _ => parenthesized(f, node.syntax(), open, condition, close),
    };
    cat![f; when, f.space(), condition]
}

/// N6 for a guard: the guard is recovery-free and no comment is within the
/// parentheses (on them, dangling between them, or within the condition).
/// The guard node also spans `when` and the comments around it, so this
/// checks the parenthesized span rather than the whole guard.
fn removable_parentheses<'a>(
    f: &JavaFormatter<'a>,
    node: Guard<'a>,
    open: SyntaxToken<'a>,
    close: SyntaxToken<'a>,
) -> bool {
    node.syntax().is_recovery_free()
        && !f.has_comments(SyntaxElement::Token(open))
        && !f.has_comments(SyntaxElement::Token(close))
        && !f.has_dangling_comments(node.syntax())
        && node
            .condition()
            .is_none_or(|condition| !f.has_comments_within(condition.syntax()))
}
