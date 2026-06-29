use super::super::{
    BasicForStatement, Block, BlockItem, BlockStatement, BreakStatement, ContinueStatement,
    DoStatement, EmptyStatement, EnhancedForStatement, Expression, ExpressionStatement,
    ForInitializer, ForStatement, ForUpdate, IfStatement, JavaSyntaxKind, JavaSyntaxToken,
    LocalVariableDeclaration, ReturnStatement, Statement, StatementExpressionList, SwitchBlock,
    SwitchBlockStatementGroup, SwitchRule, SwitchStatement, SynchronizedStatement, ThrowStatement,
    UnaryExpression, WhileStatement, YieldStatement, child, child_family, child_token, children,
    children_family, nth_child_family,
};
use super::helpers::{
    has_braced_block_statement_layout_shape, has_keyword_optional_expression_semicolon_shape,
    has_keyword_optional_label_semicolon_shape, has_keyword_required_expression_semicolon_shape,
};

impl IfStatement {
    #[must_use]
    pub fn condition(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn then_statement(&self) -> Option<Statement> {
        nth_child_family(&self.syntax, 0)
    }

    #[must_use]
    pub fn else_statement(&self) -> Option<Statement> {
        nth_child_family(&self.syntax, 1)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        match elements.as_slice() {
            [if_kw, left, condition, right, then_statement] => {
                if_kw.kind() == JavaSyntaxKind::IfKw
                    && left.kind() == JavaSyntaxKind::LParen
                    && Expression::can_cast(condition.kind())
                    && right.kind() == JavaSyntaxKind::RParen
                    && Statement::can_cast(then_statement.kind())
            }
            [
                if_kw,
                left,
                condition,
                right,
                then_statement,
                else_kw,
                else_statement,
            ] => {
                if_kw.kind() == JavaSyntaxKind::IfKw
                    && left.kind() == JavaSyntaxKind::LParen
                    && Expression::can_cast(condition.kind())
                    && right.kind() == JavaSyntaxKind::RParen
                    && Statement::can_cast(then_statement.kind())
                    && else_kw.kind() == JavaSyntaxKind::ElseKw
                    && Statement::can_cast(else_statement.kind())
            }
            _ => false,
        }
    }
}

impl EmptyStatement {
    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .eq([JavaSyntaxKind::Semicolon])
    }
}

impl BreakStatement {
    #[must_use]
    pub fn label(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        has_keyword_optional_label_semicolon_shape(&self.syntax, JavaSyntaxKind::BreakKw)
    }
}

impl ContinueStatement {
    #[must_use]
    pub fn label(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        has_keyword_optional_label_semicolon_shape(&self.syntax, JavaSyntaxKind::ContinueKw)
    }
}
impl ExpressionStatement {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [expression, semicolon]
                if match expression.kind() {
                    JavaSyntaxKind::AssignmentExpression
                        | JavaSyntaxKind::MethodInvocationExpression
                        | JavaSyntaxKind::PostfixExpression => true,
                    JavaSyntaxKind::UnaryExpression => child::<UnaryExpression>(&self.syntax)
                        .and_then(|unary| unary.operator())
                        .is_some_and(|operator| {
                            matches!(
                                operator.kind(),
                                JavaSyntaxKind::PlusPlus | JavaSyntaxKind::MinusMinus
                            )
                        }),
                    _ => false,
                } && semicolon.kind() == JavaSyntaxKind::Semicolon
        )
    }
}

impl ReturnStatement {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        has_keyword_optional_expression_semicolon_shape(
            &self.syntax,
            JavaSyntaxKind::ReturnKw,
            None,
        )
    }
}

impl ThrowStatement {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        has_keyword_required_expression_semicolon_shape(&self.syntax, JavaSyntaxKind::ThrowKw, None)
    }
}

impl YieldStatement {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        has_keyword_required_expression_semicolon_shape(
            &self.syntax,
            JavaSyntaxKind::Identifier,
            Some("yield"),
        )
    }
}

impl WhileStatement {
    #[must_use]
    pub fn condition(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Statement> {
        child_family(&self.syntax)
    }
}

impl DoStatement {
    #[must_use]
    pub fn body(&self) -> Option<Statement> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn condition(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }
}

impl SynchronizedStatement {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Block> {
        child(&self.syntax)
    }
}

impl ForStatement {
    #[must_use]
    pub fn basic(&self) -> Option<BasicForStatement> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn enhanced(&self) -> Option<EnhancedForStatement> {
        child(&self.syntax)
    }
}

impl BasicForStatement {
    #[must_use]
    pub fn initializer(&self) -> Option<ForInitializer> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn condition(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn update(&self) -> Option<ForUpdate> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Statement> {
        child_family(&self.syntax)
    }
}

impl EnhancedForStatement {
    #[must_use]
    pub fn variable(&self) -> Option<LocalVariableDeclaration> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn iterable(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Statement> {
        child_family(&self.syntax)
    }
}

impl ForInitializer {
    #[must_use]
    pub fn local_variable_declaration(&self) -> Option<LocalVariableDeclaration> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn expressions(&self) -> Option<StatementExpressionList> {
        child(&self.syntax)
    }
}

impl ForUpdate {
    #[must_use]
    pub fn expressions(&self) -> Option<StatementExpressionList> {
        child(&self.syntax)
    }
}

impl StatementExpressionList {
    pub fn expressions(&self) -> impl Iterator<Item = Expression> + '_ {
        children_family(&self.syntax)
    }
}

impl SwitchStatement {
    #[must_use]
    pub fn selector(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn block(&self) -> Option<SwitchBlock> {
        child(&self.syntax)
    }
}
impl SwitchBlock {
    pub fn statement_groups(&self) -> impl Iterator<Item = SwitchBlockStatementGroup> + '_ {
        children(&self.syntax)
    }

    pub fn rules(&self) -> impl Iterator<Item = SwitchRule> + '_ {
        children(&self.syntax)
    }
}

impl Block {
    pub fn block_statements(&self) -> impl Iterator<Item = BlockStatement> + '_ {
        children(&self.syntax)
    }

    pub fn items(&self) -> impl Iterator<Item = BlockItem> + '_ {
        children::<BlockStatement>(&self.syntax).filter_map(|node| node.item())
    }

    pub fn statements(&self) -> impl Iterator<Item = Statement> + '_ {
        children::<BlockStatement>(&self.syntax).filter_map(|node| node.statement())
    }

    #[must_use]
    pub fn has_empty_layout_shape(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .eq([JavaSyntaxKind::LBrace, JavaSyntaxKind::RBrace])
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        has_braced_block_statement_layout_shape(&self.syntax)
    }
}

impl BlockStatement {
    #[must_use]
    pub fn item(&self) -> Option<BlockItem> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn statement(&self) -> Option<Statement> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        matches!(
            self.syntax
                .children_with_tokens()
                .map(|element| element.kind())
                .collect::<Vec<_>>()
                .as_slice(),
            [
                JavaSyntaxKind::LocalVariableDeclaration,
                JavaSyntaxKind::Semicolon
            ] | [JavaSyntaxKind::Block
                | JavaSyntaxKind::EmptyStatement
                | JavaSyntaxKind::ReturnStatement
                | JavaSyntaxKind::ThrowStatement
                | JavaSyntaxKind::YieldStatement
                | JavaSyntaxKind::ExpressionStatement
                | JavaSyntaxKind::IfStatement
                | JavaSyntaxKind::BreakStatement
                | JavaSyntaxKind::ContinueStatement]
        )
    }
}
