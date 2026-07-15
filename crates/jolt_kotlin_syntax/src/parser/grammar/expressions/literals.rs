use jolt_syntax::CompletedMarker;

use crate::KotlinSyntaxKind as K;

use super::super::Parser;

impl Parser<'_> {
    pub(super) fn parse_lambda_expression(&mut self) -> CompletedMarker {
        let marker = self.start();
        let body = self.start();
        self.expect(K::LBrace, "expected lambda");
        if self.lambda_has_parameter_arrow() {
            let params = self.start();
            let parameters = self.start();
            let mut expect_parameter = true;
            while !matches!(self.current_kind(), K::Arrow | K::RBrace | K::Eof) {
                let before = self.position();
                if self.eat(K::Comma) {
                    if expect_parameter
                        && !matches!(self.current_kind(), K::Arrow | K::RBrace | K::Eof)
                    {
                        self.unexpected_here("expected lambda parameter between commas");
                        let error = self.start();
                        self.complete(error, K::ErrorNode);
                    }
                    expect_parameter = true;
                    continue;
                }
                let parameter = self.start();
                self.parse_name_or_destructuring();
                if self.eat(K::Colon) {
                    self.parse_type_reference_until(&[K::Comma, K::Arrow]);
                }
                self.complete(parameter, K::LambdaParameter);
                expect_parameter = false;
                self.ensure_progress(before, "expected lambda parameter");
            }
            self.complete(parameters, K::LambdaParameterSeparatedList);
            self.expect(K::Arrow, "expected '->' after lambda parameters");
            self.complete(params, K::LambdaParameterList);
        }
        let items = self.start();
        while !matches!(self.current_kind(), K::RBrace | K::Eof) {
            let before = self.position();
            self.parse_declaration_or_statement();
            self.ensure_progress(before, "expected lambda body statement");
        }
        self.complete(items, K::LambdaBodyItemList);
        self.expect(K::RBrace, "expected '}' after lambda");
        self.complete(body, K::LambdaBody);
        self.complete(marker, K::LambdaExpression)
    }

    pub(super) fn parse_labeled_lambda_expression(&mut self) -> CompletedMarker {
        let marker = self.start();
        let labeled = self.start();
        self.parse_optional_label_definition();
        self.parse_lambda_expression();
        self.complete(labeled, K::LabeledLambdaExpression);
        self.complete(marker, K::LambdaExpression)
    }

    pub(super) fn parse_collection_literal_expression(&mut self) -> CompletedMarker {
        let marker = self.start();
        self.expect(K::LBracket, "expected collection literal");
        self.parse_value_arguments_until(K::RBracket, K::ValueArgumentSeparatedList);
        self.expect(K::RBracket, "expected ']' after collection literal");
        self.complete(marker, K::CollectionLiteralExpression)
    }

    fn lambda_has_parameter_arrow(&mut self) -> bool {
        const MAX_LAMBDA_PARAMETER_LOOKAHEAD: usize = 256;

        let mut depth = 0usize;
        for index in (self.position()..).take(MAX_LAMBDA_PARAMETER_LOOKAHEAD) {
            match self.kind_at(index) {
                K::Arrow if depth == 0 => return true,
                K::RBrace if depth == 0 => return false,
                kind if depth == 0 && is_lambda_body_declaration_start(kind) => return false,
                K::LParen | K::LBracket | K::LBrace => depth += 1,
                K::RParen | K::RBracket | K::RBrace => depth = depth.saturating_sub(1),
                K::Eof => return false,
                _ => {}
            }
        }

        false
    }
}

fn is_lambda_body_declaration_start(kind: K) -> bool {
    matches!(
        kind,
        K::ClassKw | K::InterfaceKw | K::ObjectKw | K::FunKw | K::ValKw | K::VarKw | K::TypeAliasKw
    )
}
