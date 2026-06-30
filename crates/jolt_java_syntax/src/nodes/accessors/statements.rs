use super::super::{
    Annotation, AssertStatement, BasicForStatement, Block, BlockItem, BlockStatement,
    BreakStatement, CaseConstant, CasePattern, CatchClause, CatchParameter, CatchTypeList,
    ContinueStatement, DoStatement, EmptyStatement, EnhancedForStatement, Expression,
    ExpressionStatement, FinallyClause, ForInitializer, ForStatement, ForUpdate, Guard,
    IfStatement, JavaFamily, JavaNode, JavaSyntaxKind, JavaSyntaxToken, LabeledStatement,
    LocalClassOrInterfaceDeclaration, LocalVariableDeclaration, Pattern, Resource, ResourceList,
    ResourceSpecification, ReturnStatement, Statement, StatementExpressionList, SwitchBlock,
    SwitchBlockStatementGroup, SwitchLabel, SwitchRule, SwitchStatement, SynchronizedStatement,
    ThrowStatement, TryStatement, TryWithResourcesStatement, Type, TypeDeclaration,
    UnaryExpression, VariableAccess, WhileStatement, YieldStatement, child, child_family,
    child_token, child_token_in, children, children_family, nth_child_family,
};
use super::helpers::{
    has_braced_block_statement_layout_shape, has_keyword_optional_expression_semicolon_shape,
    has_keyword_optional_label_semicolon_shape, has_keyword_required_expression_semicolon_shape,
};

pub enum SwitchLabelItem {
    Constant(CaseConstant),
    Pattern(CasePattern, Option<Guard>),
    Default(JavaSyntaxToken),
}

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

impl LabeledStatement {
    #[must_use]
    pub fn label(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Identifier)
    }

    #[must_use]
    pub fn statement(&self) -> Option<Statement> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [label, colon, statement]
                if label.kind() == JavaSyntaxKind::Identifier
                    && colon.kind() == JavaSyntaxKind::Colon
                    && Statement::can_cast(statement.kind())
        )
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

impl AssertStatement {
    pub fn expressions(&self) -> impl Iterator<Item = Expression> + '_ {
        children_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        match elements.as_slice() {
            [assert_kw, expression, semicolon] => {
                assert_kw.kind() == JavaSyntaxKind::AssertKw
                    && Expression::can_cast(expression.kind())
                    && semicolon.kind() == JavaSyntaxKind::Semicolon
            }
            [assert_kw, expression, colon, detail, semicolon] => {
                assert_kw.kind() == JavaSyntaxKind::AssertKw
                    && Expression::can_cast(expression.kind())
                    && colon.kind() == JavaSyntaxKind::Colon
                    && Expression::can_cast(detail.kind())
                    && semicolon.kind() == JavaSyntaxKind::Semicolon
            }
            _ => false,
        }
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
                        | JavaSyntaxKind::ObjectCreationExpression
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

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [while_kw, left, condition, right, body]
                if while_kw.kind() == JavaSyntaxKind::WhileKw
                    && left.kind() == JavaSyntaxKind::LParen
                    && Expression::can_cast(condition.kind())
                    && right.kind() == JavaSyntaxKind::RParen
                    && Statement::can_cast(body.kind())
        )
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

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [do_kw, body, while_kw, left, condition, right, semicolon]
                if do_kw.kind() == JavaSyntaxKind::DoKw
                    && Statement::can_cast(body.kind())
                    && while_kw.kind() == JavaSyntaxKind::WhileKw
                    && left.kind() == JavaSyntaxKind::LParen
                    && Expression::can_cast(condition.kind())
                    && right.kind() == JavaSyntaxKind::RParen
                    && semicolon.kind() == JavaSyntaxKind::Semicolon
        )
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

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [synchronized_kw, left, expression, right, body]
                if synchronized_kw.kind() == JavaSyntaxKind::SynchronizedKw
                    && left.kind() == JavaSyntaxKind::LParen
                    && Expression::can_cast(expression.kind())
                    && right.kind() == JavaSyntaxKind::RParen
                    && body.kind() == JavaSyntaxKind::Block
        )
    }
}

impl TryStatement {
    #[must_use]
    pub fn try_with_resources(&self) -> Option<TryWithResourcesStatement> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Block> {
        child(&self.syntax)
    }

    pub fn catches(&self) -> impl Iterator<Item = CatchClause> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn finally_clause(&self) -> Option<FinallyClause> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let [try_kw, body, rest @ ..] = elements.as_slice() else {
            return false;
        };
        if try_kw.kind() != JavaSyntaxKind::TryKw || body.kind() != JavaSyntaxKind::Block {
            return false;
        }

        let mut saw_catch = false;
        let mut saw_finally = false;
        for element in rest {
            match element.kind() {
                JavaSyntaxKind::CatchClause if !saw_finally => saw_catch = true,
                JavaSyntaxKind::FinallyClause if !saw_finally => saw_finally = true,
                _ => return false,
            }
        }

        saw_catch || saw_finally
    }
}

impl TryWithResourcesStatement {
    #[must_use]
    pub fn resources(&self) -> Option<ResourceSpecification> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Block> {
        child(&self.syntax)
    }

    pub fn catches(&self) -> impl Iterator<Item = CatchClause> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn finally_clause(&self) -> Option<FinallyClause> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let [try_kw, resources, body, rest @ ..] = elements.as_slice() else {
            return false;
        };
        if try_kw.kind() != JavaSyntaxKind::TryKw
            || resources.kind() != JavaSyntaxKind::ResourceSpecification
            || body.kind() != JavaSyntaxKind::Block
        {
            return false;
        }

        let mut saw_finally = false;
        for element in rest {
            match element.kind() {
                JavaSyntaxKind::CatchClause if !saw_finally => {}
                JavaSyntaxKind::FinallyClause if !saw_finally => saw_finally = true,
                _ => return false,
            }
        }

        true
    }
}

impl ResourceSpecification {
    #[must_use]
    pub fn resources(&self) -> Option<ResourceList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_trailing_semicolon(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .any(|element| element.kind() == JavaSyntaxKind::Semicolon)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [left, resources, right]
                if left.kind() == JavaSyntaxKind::LParen
                    && resources.kind() == JavaSyntaxKind::ResourceList
                    && right.kind() == JavaSyntaxKind::RParen
        ) || matches!(
            elements.as_slice(),
            [left, resources, semicolon, right]
                if left.kind() == JavaSyntaxKind::LParen
                    && resources.kind() == JavaSyntaxKind::ResourceList
                    && semicolon.kind() == JavaSyntaxKind::Semicolon
                    && right.kind() == JavaSyntaxKind::RParen
        )
    }
}

impl ResourceList {
    pub fn resources(&self) -> impl Iterator<Item = Resource> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let mut expect_resource = true;
        let mut saw_resource = false;
        for element in self.syntax.children_with_tokens() {
            match (expect_resource, element.kind()) {
                (true, JavaSyntaxKind::Resource) => {
                    expect_resource = false;
                    saw_resource = true;
                }
                (false, JavaSyntaxKind::Semicolon) => expect_resource = true,
                _ => return false,
            }
        }

        saw_resource && !expect_resource
    }
}

impl Resource {
    #[must_use]
    pub fn local_variable_declaration(&self) -> Option<LocalVariableDeclaration> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn variable_access(&self) -> Option<VariableAccess> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [resource]
                if resource.kind() == JavaSyntaxKind::LocalVariableDeclaration
                    || resource.kind() == JavaSyntaxKind::VariableAccess
        )
    }
}

impl VariableAccess {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(elements.as_slice(), [expression] if Expression::can_cast(expression.kind()))
    }
}

impl CatchClause {
    #[must_use]
    pub fn parameter(&self) -> Option<CatchParameter> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn body(&self) -> Option<Block> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [catch_kw, left, parameter, right, body]
                if catch_kw.kind() == JavaSyntaxKind::CatchKw
                    && left.kind() == JavaSyntaxKind::LParen
                    && parameter.kind() == JavaSyntaxKind::CatchParameter
                    && right.kind() == JavaSyntaxKind::RParen
                    && body.kind() == JavaSyntaxKind::Block
        )
    }
}

impl CatchParameter {
    pub fn annotations(&self) -> impl Iterator<Item = Annotation> + '_ {
        self.syntax
            .children()
            .take_while(|node| node.kind() == JavaSyntaxKind::Annotation)
            .filter_map(Annotation::cast)
    }

    #[must_use]
    pub fn final_token(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::FinalKw)
    }

    #[must_use]
    pub fn ty(&self) -> Option<CatchTypeList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn name(&self) -> Option<JavaSyntaxToken> {
        child_token_in(
            &self.syntax,
            &[JavaSyntaxKind::Identifier, JavaSyntaxKind::UnderscoreKw],
        )
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let kinds = self
            .syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .collect::<Vec<_>>();

        let mut index = 0;
        while kinds.get(index) == Some(&JavaSyntaxKind::Annotation) {
            index += 1;
        }
        if kinds.get(index) == Some(&JavaSyntaxKind::FinalKw) {
            index += 1;
        }
        kinds.get(index) == Some(&JavaSyntaxKind::CatchTypeList)
            && matches!(
                kinds.get(index + 1),
                Some(JavaSyntaxKind::Identifier | JavaSyntaxKind::UnderscoreKw)
            )
            && index + 2 == kinds.len()
    }
}

impl CatchTypeList {
    #[must_use]
    pub fn ty(&self) -> Option<Type> {
        child_family(&self.syntax)
    }

    pub fn types(&self) -> impl Iterator<Item = Type> + '_ {
        children_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let mut expect_type = true;
        let mut saw_type = false;

        for element in self.syntax.children_with_tokens() {
            let kind = element.kind();
            if expect_type {
                if !Type::can_cast(kind) {
                    return false;
                }
                saw_type = true;
                expect_type = false;
            } else if kind == JavaSyntaxKind::Bar {
                expect_type = true;
            } else {
                return false;
            }
        }

        saw_type && !expect_type
    }
}

impl FinallyClause {
    #[must_use]
    pub fn body(&self) -> Option<Block> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [finally_kw, body]
                if finally_kw.kind() == JavaSyntaxKind::FinallyKw
                    && body.kind() == JavaSyntaxKind::Block
        )
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

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [statement]
                if statement.kind() == JavaSyntaxKind::BasicForStatement
                    || statement.kind() == JavaSyntaxKind::EnhancedForStatement
        )
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

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let [for_kw, left, rest @ .., right, body] = elements.as_slice() else {
            return false;
        };
        if for_kw.kind() != JavaSyntaxKind::ForKw
            || left.kind() != JavaSyntaxKind::LParen
            || right.kind() != JavaSyntaxKind::RParen
            || !Statement::can_cast(body.kind())
        {
            return false;
        }

        let mut index = 0;
        if rest
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::ForInitializer)
        {
            index += 1;
        }
        if !rest
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::Semicolon)
        {
            return false;
        }
        index += 1;
        if rest
            .get(index)
            .is_some_and(|element| Expression::can_cast(element.kind()))
        {
            index += 1;
        }
        if !rest
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::Semicolon)
        {
            return false;
        }
        index += 1;
        if rest
            .get(index)
            .is_some_and(|element| element.kind() == JavaSyntaxKind::ForUpdate)
        {
            index += 1;
        }

        index == rest.len()
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

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [for_kw, left, variable, colon, iterable, right, body]
                if for_kw.kind() == JavaSyntaxKind::ForKw
                    && left.kind() == JavaSyntaxKind::LParen
                    && variable.kind() == JavaSyntaxKind::LocalVariableDeclaration
                    && colon.kind() == JavaSyntaxKind::Colon
                    && Expression::can_cast(iterable.kind())
                    && right.kind() == JavaSyntaxKind::RParen
                    && Statement::can_cast(body.kind())
        )
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

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [initializer]
                if initializer.kind() == JavaSyntaxKind::LocalVariableDeclaration
                    || initializer.kind() == JavaSyntaxKind::StatementExpressionList
        )
    }
}

impl ForUpdate {
    #[must_use]
    pub fn expressions(&self) -> Option<StatementExpressionList> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .eq([JavaSyntaxKind::StatementExpressionList])
    }
}

impl StatementExpressionList {
    pub fn expressions(&self) -> impl Iterator<Item = Expression> + '_ {
        children_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let Some(first) = elements.first() else {
            return false;
        };
        if !Expression::can_cast(first.kind()) {
            return false;
        }

        let mut expect_comma = true;
        for element in &elements[1..] {
            if expect_comma {
                if element.kind() != JavaSyntaxKind::Comma {
                    return false;
                }
            } else if !Expression::can_cast(element.kind()) {
                return false;
            }
            expect_comma = !expect_comma;
        }

        expect_comma
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

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [switch_kw, left, selector, right, block]
                if switch_kw.kind() == JavaSyntaxKind::SwitchKw
                    && left.kind() == JavaSyntaxKind::LParen
                    && Expression::can_cast(selector.kind())
                    && right.kind() == JavaSyntaxKind::RParen
                    && block.kind() == JavaSyntaxKind::SwitchBlock
        )
    }
}

pub enum SwitchBlockItem {
    StatementGroup(SwitchBlockStatementGroup),
    Rule(SwitchRule),
    BlockStatement(BlockStatement),
}

impl SwitchBlock {
    pub fn items(&self) -> impl Iterator<Item = SwitchBlockItem> + '_ {
        self.syntax.children_with_tokens().filter_map(|element| {
            let node = element.into_node()?;
            match node.kind() {
                JavaSyntaxKind::SwitchBlockStatementGroup => {
                    SwitchBlockStatementGroup::cast(node).map(SwitchBlockItem::StatementGroup)
                }
                JavaSyntaxKind::SwitchRule => SwitchRule::cast(node).map(SwitchBlockItem::Rule),
                JavaSyntaxKind::BlockStatement => {
                    BlockStatement::cast(node).map(SwitchBlockItem::BlockStatement)
                }
                _ => None,
            }
        })
    }

    pub fn statement_groups(&self) -> impl Iterator<Item = SwitchBlockStatementGroup> + '_ {
        children(&self.syntax)
    }

    pub fn rules(&self) -> impl Iterator<Item = SwitchRule> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let Some(first) = elements.first() else {
            return false;
        };
        let Some(last) = elements.last() else {
            return false;
        };
        first.kind() == JavaSyntaxKind::LBrace
            && last.kind() == JavaSyntaxKind::RBrace
            && elements[1..elements.len().saturating_sub(1)]
                .iter()
                .all(|element| {
                    matches!(
                        element.kind(),
                        JavaSyntaxKind::SwitchBlockStatementGroup
                            | JavaSyntaxKind::SwitchRule
                            | JavaSyntaxKind::BlockStatement
                    )
                })
    }
}

impl SwitchBlockStatementGroup {
    pub fn labels(&self) -> impl Iterator<Item = SwitchLabel> + '_ {
        children(&self.syntax)
    }

    pub fn colons(&self) -> impl Iterator<Item = JavaSyntaxToken> + '_ {
        self.syntax
            .children_with_tokens()
            .filter_map(jolt_syntax::SyntaxElement::into_token)
            .filter(|token| token.kind() == JavaSyntaxKind::Colon)
            .map(|syntax| JavaSyntaxToken { syntax })
    }

    pub fn block_statements(&self) -> impl Iterator<Item = BlockStatement> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let mut index = 0;
        let mut label_count = 0;
        while index + 1 < elements.len()
            && elements[index].kind() == JavaSyntaxKind::SwitchLabel
            && elements[index + 1].kind() == JavaSyntaxKind::Colon
        {
            label_count += 1;
            index += 2;
        }

        label_count > 0
            && elements[index..]
                .iter()
                .all(|element| element.kind() == JavaSyntaxKind::BlockStatement)
    }
}

pub enum SwitchRuleBody {
    Block(Block),
    Expression(Expression),
    Throw(ThrowStatement),
}

impl SwitchRule {
    #[must_use]
    pub fn label(&self) -> Option<SwitchLabel> {
        child(&self.syntax)
    }

    #[must_use]
    pub fn arrow(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::Arrow)
    }

    #[must_use]
    pub fn body(&self) -> Option<SwitchRuleBody> {
        self.syntax.children_with_tokens().find_map(|element| {
            let node = element.into_node()?;
            match node.kind() {
                JavaSyntaxKind::Block => Block::cast(node).map(SwitchRuleBody::Block),
                JavaSyntaxKind::ThrowStatement => {
                    ThrowStatement::cast(node).map(SwitchRuleBody::Throw)
                }
                kind if Expression::can_cast(kind) => {
                    Expression::cast(node).map(SwitchRuleBody::Expression)
                }
                _ => None,
            }
        })
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(
            elements.as_slice(),
            [label, arrow, body]
                if label.kind() == JavaSyntaxKind::SwitchLabel
                    && arrow.kind() == JavaSyntaxKind::Arrow
                    && matches!(body.kind(), JavaSyntaxKind::Block | JavaSyntaxKind::ThrowStatement)
        ) || matches!(
            elements.as_slice(),
            [label, arrow, body, semicolon]
                if label.kind() == JavaSyntaxKind::SwitchLabel
                    && arrow.kind() == JavaSyntaxKind::Arrow
                    && Expression::can_cast(body.kind())
                    && semicolon.kind() == JavaSyntaxKind::Semicolon
        )
    }
}

impl SwitchLabel {
    pub fn items(&self) -> impl Iterator<Item = SwitchLabelItem> {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        let mut items = Vec::new();
        let mut index = 0;

        while let Some(element) = elements.get(index) {
            match element.kind() {
                JavaSyntaxKind::CaseConstant => {
                    if let Some(item) = element
                        .clone()
                        .into_node()
                        .and_then(CaseConstant::cast)
                        .map(SwitchLabelItem::Constant)
                    {
                        items.push(item);
                    }
                }
                JavaSyntaxKind::CasePattern => {
                    let pattern = element.clone().into_node().and_then(CasePattern::cast);
                    let guard = elements
                        .get(index + 1)
                        .filter(|next| next.kind() == JavaSyntaxKind::Guard)
                        .and_then(|next| next.clone().into_node().and_then(Guard::cast));
                    if guard.is_some() {
                        index += 1;
                    }
                    if let Some(pattern) = pattern {
                        items.push(SwitchLabelItem::Pattern(pattern, guard));
                    }
                }
                JavaSyntaxKind::DefaultKw => {
                    if let Some(syntax) = element.clone().into_token() {
                        items.push(SwitchLabelItem::Default(JavaSyntaxToken { syntax }));
                    }
                }
                _ => {}
            }
            index += 1;
        }

        items.into_iter()
    }

    pub fn constants(&self) -> impl Iterator<Item = CaseConstant> + '_ {
        children(&self.syntax)
    }

    pub fn patterns(&self) -> impl Iterator<Item = CasePattern> + '_ {
        children(&self.syntax)
    }

    #[must_use]
    pub fn default_token(&self) -> Option<JavaSyntaxToken> {
        child_token(&self.syntax, JavaSyntaxKind::DefaultKw)
    }

    #[must_use]
    pub fn has_default_only_layout_shape(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .map(|element| element.kind())
            .eq([JavaSyntaxKind::DefaultKw])
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        match elements.as_slice() {
            [default_kw] => default_kw.kind() == JavaSyntaxKind::DefaultKw,
            [case_kw, rest @ ..] if case_kw.kind() == JavaSyntaxKind::CaseKw => {
                has_supported_switch_label_items(rest)
            }
            _ => false,
        }
    }
}

impl CaseConstant {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        self.expression().is_some()
    }
}

impl CasePattern {
    #[must_use]
    pub fn pattern(&self) -> Option<Pattern> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        matches!(elements.as_slice(), [pattern] if Pattern::can_cast(pattern.kind()))
    }
}

impl Guard {
    #[must_use]
    pub fn expression(&self) -> Option<Expression> {
        child_family(&self.syntax)
    }

    #[must_use]
    pub fn has_supported_layout_shape(&self) -> bool {
        let elements = self.syntax.children_with_tokens().collect::<Vec<_>>();
        match elements.as_slice() {
            [when_kw, expression] => {
                let is_when = when_kw.clone().into_token().is_some_and(|token| {
                    token.kind() == JavaSyntaxKind::Identifier && token.text() == "when"
                });
                is_when && Expression::can_cast(expression.kind())
            }
            _ => false,
        }
    }
}

fn has_supported_switch_label_items(
    elements: &[jolt_syntax::SyntaxElement<crate::language::JavaLanguage>],
) -> bool {
    let mut index = 0;
    let mut expect_item = true;
    let mut saw_item = false;

    while let Some(element) = elements.get(index) {
        if expect_item {
            match element.kind() {
                JavaSyntaxKind::CaseConstant | JavaSyntaxKind::DefaultKw => {}
                JavaSyntaxKind::CasePattern => {
                    if elements
                        .get(index + 1)
                        .is_some_and(|next| next.kind() == JavaSyntaxKind::Guard)
                    {
                        index += 1;
                    }
                }
                _ => return false,
            }
            saw_item = true;
            expect_item = false;
        } else if element.kind() == JavaSyntaxKind::Comma {
            expect_item = true;
        } else {
            return false;
        }
        index += 1;
    }

    saw_item && !expect_item
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
            ] | [JavaSyntaxKind::LocalClassOrInterfaceDeclaration
                | JavaSyntaxKind::Block
                | JavaSyntaxKind::EmptyStatement
                | JavaSyntaxKind::ReturnStatement
                | JavaSyntaxKind::ThrowStatement
                | JavaSyntaxKind::YieldStatement
                | JavaSyntaxKind::ExpressionStatement
                | JavaSyntaxKind::IfStatement
                | JavaSyntaxKind::LabeledStatement
                | JavaSyntaxKind::AssertStatement
                | JavaSyntaxKind::SwitchStatement
                | JavaSyntaxKind::WhileStatement
                | JavaSyntaxKind::DoStatement
                | JavaSyntaxKind::ForStatement
                | JavaSyntaxKind::SynchronizedStatement
                | JavaSyntaxKind::TryStatement
                | JavaSyntaxKind::BreakStatement
                | JavaSyntaxKind::ContinueStatement]
        )
    }
}

impl LocalClassOrInterfaceDeclaration {
    #[must_use]
    pub fn declaration(&self) -> Option<TypeDeclaration> {
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
            [JavaSyntaxKind::ClassDeclaration
                | JavaSyntaxKind::RecordDeclaration
                | JavaSyntaxKind::EnumDeclaration
                | JavaSyntaxKind::InterfaceDeclaration
                | JavaSyntaxKind::AnnotationInterfaceDeclaration]
        )
    }
}
