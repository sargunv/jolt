use jolt_syntax::CompletedMarker;

use crate::KotlinSyntaxKind as K;

use super::super::Parser;

impl Parser<'_> {
    pub(super) fn parse_lambda_expression(&mut self) -> CompletedMarker {
        let marker = self.start();
        let body = self.start();
        self.eat_asserted(K::LBrace);
        if self.lambda_has_parameter_arrow() {
            let params = self.start();
            let parameters = self.start();
            let mut expect_parameter = true;
            while !matches!(self.current_kind(), K::Arrow | K::RBrace | K::Eof) {
                let before = self.position();
                if self.at(K::Comma) {
                    if expect_parameter {
                        let error = self.start();
                        let diagnostic =
                            self.pending_unexpected("expected lambda parameter between commas");

                        self.complete_recovery(error, K::BogusLambdaParameter, [diagnostic]);
                    }
                    self.bump();
                    expect_parameter = true;
                    continue;
                }
                let parameter = self.start();
                let binding = self.start();
                self.parse_name_or_destructuring();
                self.complete(binding, K::LambdaParameterBinding);
                if self.eat(K::Colon) {
                    self.parse_type_reference_until(&[K::Comma, K::Arrow]);
                }
                self.complete(parameter, K::LambdaParameter);
                expect_parameter = false;
                debug_assert!(self.position() > before);
            }
            self.complete(parameters, K::LambdaParameterSeparatedList);
            if !self.eat(K::Arrow) {
                let diagnostic = self.pending_expected("expected '->' after lambda parameters");
                self.missing_required_slot(
                    params.anchor(),
                    crate::shape::lambda_parameter_list::Slot::arrow as u16,
                    [diagnostic],
                );
            }
            self.complete(params, K::LambdaParameterList);
        }
        let items = self.start();
        while !matches!(self.current_kind(), K::RBrace | K::Eof) {
            let before = self.position();
            self.parse_declaration_or_statement();
            debug_assert!(self.position() > before);
        }
        self.complete(items, K::LambdaBodyItemList);
        if !self.eat(K::RBrace) {
            let diagnostic = self.pending_expected("expected '}' after lambda");
            self.missing_required_slot(
                body.anchor(),
                crate::shape::lambda_body::Slot::close_brace as u16,
                [diagnostic],
            );
        }
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
        self.eat_asserted(K::LBracket);
        self.parse_value_arguments_until(K::RBracket, K::ValueArgumentEntryList);
        if !self.eat(K::RBracket) {
            let diagnostic = self.pending_expected("expected ']' after collection literal");
            self.missing_required_slot(
                marker.anchor(),
                crate::shape::collection_literal_expression::Slot::close_bracket as u16,
                [diagnostic],
            );
        }
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
