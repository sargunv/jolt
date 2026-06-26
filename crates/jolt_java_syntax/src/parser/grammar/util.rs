impl Parser<'_> {
    fn expect_type_identifier(&mut self, message: &str) {
        if self.at_type_identifier() {
            self.bump();
        } else if self.at_name_segment() {
            let error = self.start();
            self.error_here(message);
            self.bump();
            self.complete(error, JavaSyntaxKind::ErrorNode);
        } else {
            self.error_here(message);
        }
    }

    fn expect_method_identifier(&mut self, message: &str) {
        if self.at_name_segment() {
            self.bump();
        } else {
            self.error_here(message);
        }
    }

    fn expect_variable_identifier(&mut self, message: &str) {
        if self.at_variable_identifier() {
            self.bump();
        } else {
            self.error_here(message);
        }
    }

    fn expect_named_variable_identifier(&mut self, message: &str) {
        if self.at_name_segment() {
            self.bump();
        } else {
            self.error_here(message);
        }
    }

    fn consume_qualified_name(&mut self) -> bool {
        if !self.at_name_segment() {
            self.error_here("expected identifier");
            return false;
        }

        let name = self.start();
        self.bump();
        let mut qualified = false;
        while self.at(JavaSyntaxKind::Dot) && self.nth_is_name_segment(1) {
            qualified = true;
            self.bump();
            self.bump();
        }
        self.complete(
            name,
            if qualified {
                JavaSyntaxKind::QualifiedName
            } else {
                JavaSyntaxKind::Name
            },
        );
        true
    }

    fn consume_balanced_delimited(&mut self, open: JavaSyntaxKind, close: JavaSyntaxKind) {
        if !self.at(open) {
            return;
        }

        let mut depth = 0usize;
        while !self.at_eof() {
            if self.at(open) {
                depth += 1;
            } else if self.at(close) {
                depth = depth.saturating_sub(1);
                self.bump();
                if depth == 0 {
                    return;
                }
                continue;
            }
            self.bump();
        }
    }

    fn error_unexpected_top_level_token(&mut self) {
        let error = self.start();
        self.error_here("unexpected token at top level");
        self.recover_top_level();
        self.complete(error, JavaSyntaxKind::ErrorNode);
    }

    fn error_unexpected_module_token(&mut self) {
        let error = self.start();
        self.error_here("unexpected token in module declaration");
        self.recover_module_directive();
        self.complete(error, JavaSyntaxKind::ErrorNode);
    }

    fn recover_top_level(&mut self) {
        if self.at_eof() {
            return;
        }

        self.bump();
        while !self.at_eof()
            && !self.at(JavaSyntaxKind::Semicolon)
            && !self.at(JavaSyntaxKind::ImportKw)
            && !self.starts_module_declaration()
            && !self.starts_top_level_type_declaration()
        {
            self.bump();
        }

        self.eat(JavaSyntaxKind::Semicolon);
    }

    fn recover_module_directive(&mut self) {
        if self.at_eof() || self.at(JavaSyntaxKind::RBrace) {
            return;
        }

        self.bump();
        while !self.at_eof()
            && !self.at(JavaSyntaxKind::Semicolon)
            && !self.at(JavaSyntaxKind::RBrace)
        {
            self.bump();
        }

        self.eat(JavaSyntaxKind::Semicolon);
    }

    fn at_header_clause_end(&self) -> bool {
        matches!(
            self.current_kind(),
            JavaSyntaxKind::LBrace
                | JavaSyntaxKind::LParen
                | JavaSyntaxKind::Semicolon
                | JavaSyntaxKind::ImplementsKw
                | JavaSyntaxKind::ExtendsKw
        ) || self.at_contextual("permits")
    }

    fn at_type_argument_close(&self) -> bool {
        matches!(
            self.current_kind(),
            JavaSyntaxKind::Gt | JavaSyntaxKind::RShift | JavaSyntaxKind::UnsignedRShift
        )
    }

    fn eat_type_argument_close(&mut self) -> bool {
        if self.at_type_argument_close() {
            self.bump_split_gt();
            true
        } else {
            false
        }
    }

    fn at_primitive_type(&self) -> bool {
        matches!(
            self.current_kind(),
            JavaSyntaxKind::BooleanKw
                | JavaSyntaxKind::ByteKw
                | JavaSyntaxKind::CharKw
                | JavaSyntaxKind::DoubleKw
                | JavaSyntaxKind::FloatKw
                | JavaSyntaxKind::IntKw
                | JavaSyntaxKind::LongKw
                | JavaSyntaxKind::ShortKw
        )
    }

    fn starts_array_dimensions(&self) -> bool {
        let index = self.skip_annotations_from(self.position());
        self.kind_at(index) == JavaSyntaxKind::LBracket
            && self.kind_at(index + 1) == JavaSyntaxKind::RBracket
    }

    fn starts_dim_expression(&self) -> bool {
        let index = self.skip_annotations_from(self.position());
        self.kind_at(index) == JavaSyntaxKind::LBracket
            && self.kind_at(index + 1) != JavaSyntaxKind::RBracket
    }

    fn starts_constructor(&self, type_name: Option<&str>) -> bool {
        let mut index = self.skip_type_modifiers_from(self.position());
        if self.kind_at(index) == JavaSyntaxKind::Lt {
            index = self.skip_balanced_type_arguments_from(index);
        }

        (matches!(type_name, Some(name) if self.text_at(index) == Some(name))
            || (self.is_name_segment_at_offset(index)
                && self.kind_at(index + 1) == JavaSyntaxKind::LParen
                && self.member_header_ends_with_block()))
            && self.kind_at(index + 1) == JavaSyntaxKind::LParen
    }

    fn starts_compact_constructor(&self, type_name: Option<&str>) -> bool {
        let index = self.skip_type_modifiers_from(self.position());
        matches!(type_name, Some(name) if self.text_at(index) == Some(name))
            && self.kind_at(index + 1) == JavaSyntaxKind::LBrace
    }

    fn starts_method_declaration(&self) -> bool {
        let mut index = self.skip_type_modifiers_from(self.position());
        if self.kind_at(index) == JavaSyntaxKind::Lt {
            index = self.skip_balanced_type_arguments_from(index);
            index = self.skip_annotations_from(index);
        }

        if self.kind_at(index) == JavaSyntaxKind::VoidKw {
            return self.is_name_segment_at_offset(index + 1)
                && self.kind_at(index + 2) == JavaSyntaxKind::LParen;
        }

        if !self.is_type_start_at(index) {
            return false;
        }

        let after_type = self.skip_type_from(index);
        self.is_name_segment_at_offset(after_type)
            && self.kind_at(after_type + 1) == JavaSyntaxKind::LParen
    }

    fn starts_annotation_element(&self) -> bool {
        let index = self.skip_type_modifiers_from(self.position());
        if !self.is_non_void_type_start_at(index) {
            return false;
        }

        let after_type = self.skip_type_from(index);
        self.is_name_segment_at_offset(after_type)
            && self.kind_at(after_type + 1) == JavaSyntaxKind::LParen
            && self.kind_at(after_type + 2) == JavaSyntaxKind::RParen
    }

    fn starts_receiver_parameter(&self) -> bool {
        let mut index = self.position();
        while !matches!(
            self.kind_at(index),
            JavaSyntaxKind::Eof
                | JavaSyntaxKind::Comma
                | JavaSyntaxKind::RParen
                | JavaSyntaxKind::Semicolon
        ) {
            if self.kind_at(index) == JavaSyntaxKind::ThisKw {
                return true;
            }
            index += 1;
        }
        false
    }

    fn starts_local_class_or_interface_declaration(&self) -> bool {
        let index = self.skip_type_modifiers_from(self.position());
        matches!(
            self.kind_at(index),
            JavaSyntaxKind::ClassKw | JavaSyntaxKind::InterfaceKw | JavaSyntaxKind::EnumKw
        ) || self.text_at(index) == Some("record")
            || (self.kind_at(index) == JavaSyntaxKind::At
                && self.kind_at(index + 1) == JavaSyntaxKind::InterfaceKw)
    }

    fn starts_local_variable_declaration(&self) -> bool {
        let index = self.skip_variable_modifiers_from(self.position());

        if self.text_at(index) == Some("yield") && self.kind_at(index + 1) != JavaSyntaxKind::Dot {
            return false;
        }

        if self.text_at(index) == Some("var") && self.kind_at(index + 1) != JavaSyntaxKind::Dot {
            return self.is_variable_identifier_at_offset(index + 1)
                && !matches!(
                    self.kind_at(index + 2),
                    JavaSyntaxKind::LParen | JavaSyntaxKind::Dot
                );
        }

        if !self.is_non_void_type_start_at(index) {
            return false;
        }

        let after_type = self.skip_type_from(index);
        self.is_variable_identifier_at_offset(after_type)
            && !matches!(self.kind_at(after_type + 1), JavaSyntaxKind::LParen)
    }

    fn starts_resource_local_variable_declaration(&self) -> bool {
        if !self.starts_local_variable_declaration() {
            return false;
        }

        let mut index = self.position();
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        while self.kind_at(index) != JavaSyntaxKind::Eof {
            match self.kind_at(index) {
                JavaSyntaxKind::Assign if paren_depth == 0 && bracket_depth == 0 => return true,
                JavaSyntaxKind::Semicolon | JavaSyntaxKind::RParen
                    if paren_depth == 0 && bracket_depth == 0 =>
                {
                    return false;
                }
                JavaSyntaxKind::LParen => paren_depth += 1,
                JavaSyntaxKind::RParen => {
                    if paren_depth == 0 {
                        return false;
                    }
                    paren_depth -= 1;
                }
                JavaSyntaxKind::LBracket => bracket_depth += 1,
                JavaSyntaxKind::RBracket => {
                    if bracket_depth == 0 {
                        return false;
                    }
                    bracket_depth -= 1;
                }
                _ => {}
            }
            index += 1;
        }

        false
    }

    fn skip_variable_modifiers_from(&self, mut index: usize) -> usize {
        loop {
            let after_annotations = self.skip_annotations_from(index);
            if after_annotations != index {
                index = after_annotations;
            } else if self.kind_at(index) == JavaSyntaxKind::FinalKw {
                index += 1;
            } else {
                return index;
            }
        }
    }

    fn starts_labeled_statement(&self) -> bool {
        self.current_kind() == JavaSyntaxKind::Identifier
            && self.nth_kind(1) == JavaSyntaxKind::Colon
    }

    fn starts_yield_statement(&self) -> bool {
        self.at_contextual("yield")
            && !matches!(
                self.nth_kind(1),
                JavaSyntaxKind::LParen
                    | JavaSyntaxKind::LBracket
                    | JavaSyntaxKind::Dot
                    | JavaSyntaxKind::Assign
                    | JavaSyntaxKind::PlusPlus
                    | JavaSyntaxKind::MinusMinus
                    | JavaSyntaxKind::PlusEq
                    | JavaSyntaxKind::MinusEq
                    | JavaSyntaxKind::StarEq
                    | JavaSyntaxKind::SlashEq
                    | JavaSyntaxKind::AmpEq
                    | JavaSyntaxKind::BarEq
                    | JavaSyntaxKind::CaretEq
                    | JavaSyntaxKind::PercentEq
                    | JavaSyntaxKind::LShiftEq
                    | JavaSyntaxKind::RShiftEq
                    | JavaSyntaxKind::UnsignedRShiftEq
                    | JavaSyntaxKind::Semicolon
            )
    }

    fn starts_parenthesized_lambda_expression(&self) -> bool {
        if self.current_kind() != JavaSyntaxKind::LParen {
            return false;
        }

        let after_parameters = self.skip_balanced_from(
            self.position(),
            JavaSyntaxKind::LParen,
            JavaSyntaxKind::RParen,
        );
        self.kind_at(after_parameters) == JavaSyntaxKind::Arrow
    }

    fn starts_lambda_expression(&self) -> bool {
        self.starts_parenthesized_lambda_expression()
            || ((self.current_kind() == JavaSyntaxKind::Identifier
                || self.current_kind() == JavaSyntaxKind::UnderscoreKw)
                && self.nth_kind(1) == JavaSyntaxKind::Arrow)
    }

    fn at_assignment_operator(&self) -> bool {
        matches!(
            self.current_kind(),
            JavaSyntaxKind::Assign
                | JavaSyntaxKind::PlusEq
                | JavaSyntaxKind::MinusEq
                | JavaSyntaxKind::StarEq
                | JavaSyntaxKind::SlashEq
                | JavaSyntaxKind::AmpEq
                | JavaSyntaxKind::BarEq
                | JavaSyntaxKind::CaretEq
                | JavaSyntaxKind::PercentEq
                | JavaSyntaxKind::LShiftEq
                | JavaSyntaxKind::RShiftEq
                | JavaSyntaxKind::UnsignedRShiftEq
        )
    }

    fn binary_operator_precedence(&self) -> Option<u8> {
        Some(match self.current_kind() {
            JavaSyntaxKind::OrOr => 1,
            JavaSyntaxKind::AndAnd => 2,
            JavaSyntaxKind::Bar => 3,
            JavaSyntaxKind::Caret => 4,
            JavaSyntaxKind::Amp => 5,
            JavaSyntaxKind::EqEq | JavaSyntaxKind::BangEq => 6,
            JavaSyntaxKind::Lt
            | JavaSyntaxKind::Gt
            | JavaSyntaxKind::LtEq
            | JavaSyntaxKind::GtEq
            | JavaSyntaxKind::InstanceofKw => 7,
            JavaSyntaxKind::LShift | JavaSyntaxKind::RShift | JavaSyntaxKind::UnsignedRShift => 8,
            JavaSyntaxKind::Plus | JavaSyntaxKind::Minus => 9,
            JavaSyntaxKind::Star | JavaSyntaxKind::Slash | JavaSyntaxKind::Percent => 10,
            _ => return None,
        })
    }

    fn starts_cast_expression(&self) -> bool {
        if self.current_kind() != JavaSyntaxKind::LParen
            || self.starts_parenthesized_lambda_expression()
        {
            return false;
        }

        let type_start = self.skip_annotations_from(self.position() + 1);
        let close = self.skip_cast_type_from(self.position() + 1);
        if self.kind_at(close) != JavaSyntaxKind::RParen
            || self.kind_at(close + 1) == JavaSyntaxKind::Arrow
        {
            return false;
        }

        if self.is_primitive_type_start_at(type_start)
            && self.kind_at(type_start + 1) == JavaSyntaxKind::RParen
        {
            self.starts_expression_at(close + 1)
        } else {
            self.starts_expression_not_plus_minus_at(close + 1)
        }
    }

    fn skip_cast_type_from(&self, mut index: usize) -> usize {
        index = self.skip_annotations_from(index);
        if !self.is_type_start_at(index) {
            return index;
        }

        index = self.skip_type_from(index);
        while self.kind_at(index) == JavaSyntaxKind::Amp && self.is_type_start_at(index + 1) {
            index = self.skip_type_from(index + 1);
        }
        index
    }

    fn type_arguments_are_followed_by_double_colon(&self) -> bool {
        let index = self.skip_balanced_type_arguments_from(self.position());
        self.kind_at(index) == JavaSyntaxKind::DoubleColon
    }

    fn starts_expression_at(&self, index: usize) -> bool {
        matches!(
            self.kind_at(index),
            JavaSyntaxKind::Identifier
                | JavaSyntaxKind::UnderscoreKw
                | JavaSyntaxKind::IntegerLiteral
                | JavaSyntaxKind::FloatingPointLiteral
                | JavaSyntaxKind::BooleanLiteral
                | JavaSyntaxKind::CharacterLiteral
                | JavaSyntaxKind::StringLiteral
                | JavaSyntaxKind::TextBlockLiteral
                | JavaSyntaxKind::NullLiteral
                | JavaSyntaxKind::ThisKw
                | JavaSyntaxKind::SuperKw
                | JavaSyntaxKind::SwitchKw
                | JavaSyntaxKind::NewKw
                | JavaSyntaxKind::LParen
                | JavaSyntaxKind::PlusPlus
                | JavaSyntaxKind::MinusMinus
                | JavaSyntaxKind::Plus
                | JavaSyntaxKind::Minus
                | JavaSyntaxKind::Bang
                | JavaSyntaxKind::Tilde
        ) || self.starts_primitive_or_void_class_literal_at(index)
    }

    fn starts_expression_not_plus_minus_at(&self, index: usize) -> bool {
        self.starts_expression_at(index)
            && !matches!(
                self.kind_at(index),
                JavaSyntaxKind::Plus | JavaSyntaxKind::Minus
            )
    }

    fn starts_primitive_or_void_class_literal(&self) -> bool {
        self.starts_primitive_or_void_class_literal_at(self.position())
    }

    fn starts_primitive_or_void_class_literal_at(&self, mut index: usize) -> bool {
        if !matches!(
            self.kind_at(index),
            JavaSyntaxKind::BooleanKw
                | JavaSyntaxKind::ByteKw
                | JavaSyntaxKind::CharKw
                | JavaSyntaxKind::DoubleKw
                | JavaSyntaxKind::FloatKw
                | JavaSyntaxKind::IntKw
                | JavaSyntaxKind::LongKw
                | JavaSyntaxKind::ShortKw
                | JavaSyntaxKind::VoidKw
        ) {
            return false;
        }

        index += 1;
        while self.kind_at(index) == JavaSyntaxKind::LBracket
            && self.kind_at(index + 1) == JavaSyntaxKind::RBracket
        {
            index += 2;
        }

        self.kind_at(index) == JavaSyntaxKind::Dot
            && self.kind_at(index + 1) == JavaSyntaxKind::ClassKw
    }

    fn starts_typed_lambda_parameter(&self) -> bool {
        if self.text_at(self.position()) == Some("var") && self.nth_kind(1) != JavaSyntaxKind::Dot {
            return self.is_variable_identifier_at_offset(self.position() + 1);
        }

        if !self.is_type_start_at(self.position()) {
            return false;
        }

        let after_type = self.skip_type_from(self.position());
        let after_annotations = self.skip_annotations_from(after_type);
        let name = if self.kind_at(after_annotations) == JavaSyntaxKind::Ellipsis {
            after_annotations + 1
        } else {
            after_annotations
        };

        self.is_variable_identifier_at_offset(name)
    }

    fn for_header_has_top_level_colon(&self) -> bool {
        let mut index = self.position();
        while self.kind_at(index) != JavaSyntaxKind::Eof
            && self.kind_at(index) != JavaSyntaxKind::LParen
        {
            index += 1;
        }

        if self.kind_at(index) != JavaSyntaxKind::LParen {
            return false;
        }

        index += 1;
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut conditional_depth = 0usize;
        while self.kind_at(index) != JavaSyntaxKind::Eof {
            match self.kind_at(index) {
                JavaSyntaxKind::Question if paren_depth == 0 && bracket_depth == 0 => {
                    conditional_depth += 1;
                }
                JavaSyntaxKind::Colon
                    if paren_depth == 0 && bracket_depth == 0 && conditional_depth > 0 =>
                {
                    conditional_depth -= 1;
                }
                JavaSyntaxKind::Colon if paren_depth == 0 && bracket_depth == 0 => return true,
                JavaSyntaxKind::Semicolon | JavaSyntaxKind::RParen
                    if paren_depth == 0 && bracket_depth == 0 =>
                {
                    return false;
                }
                JavaSyntaxKind::LParen => paren_depth += 1,
                JavaSyntaxKind::RParen => {
                    if paren_depth == 0 {
                        return false;
                    }
                    paren_depth -= 1;
                }
                JavaSyntaxKind::LBracket => bracket_depth += 1,
                JavaSyntaxKind::RBracket => {
                    if bracket_depth == 0 {
                        return false;
                    }
                    bracket_depth -= 1;
                }
                _ => {}
            }
            index += 1;
        }

        false
    }

    fn starts_switch_label(&self) -> bool {
        matches!(
            self.current_kind(),
            JavaSyntaxKind::CaseKw | JavaSyntaxKind::DefaultKw
        )
    }

    fn switch_label_is_rule(&self) -> bool {
        let mut index = self.position();
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        while self.kind_at(index) != JavaSyntaxKind::Eof {
            match self.kind_at(index) {
                JavaSyntaxKind::Arrow if paren_depth == 0 && bracket_depth == 0 => return true,
                JavaSyntaxKind::Colon | JavaSyntaxKind::RBrace
                    if paren_depth == 0 && bracket_depth == 0 =>
                {
                    return false;
                }
                JavaSyntaxKind::LParen => paren_depth += 1,
                JavaSyntaxKind::RParen => {
                    if paren_depth == 0 {
                        return false;
                    }
                    paren_depth -= 1;
                }
                JavaSyntaxKind::LBracket => bracket_depth += 1,
                JavaSyntaxKind::RBracket => {
                    if bracket_depth == 0 {
                        return false;
                    }
                    bracket_depth -= 1;
                }
                _ => {}
            }
            index += 1;
        }

        false
    }

    fn starts_case_type_pattern(&self) -> bool {
        let index = self.skip_variable_modifiers_from(self.position());
        if !self.is_non_void_type_start_at(index) || self.starts_literal_expression_at(index) {
            return false;
        }

        let after_type = self.skip_type_from(index);
        self.is_variable_identifier_at_offset(after_type)
    }

    fn starts_pattern(&self) -> bool {
        self.starts_case_type_pattern() || self.starts_record_pattern()
    }

    fn starts_record_pattern(&self) -> bool {
        let index = self.skip_variable_modifiers_from(self.position());
        if !self.is_non_void_type_start_at(index) || self.starts_literal_expression_at(index) {
            return false;
        }

        let after_type = self.skip_type_from(index);
        self.kind_at(after_type) == JavaSyntaxKind::LParen
    }

    fn starts_literal_expression(&self) -> bool {
        self.starts_literal_expression_at(self.position())
    }

    fn starts_literal_expression_at(&self, index: usize) -> bool {
        matches!(
            self.kind_at(index),
            JavaSyntaxKind::IntegerLiteral
                | JavaSyntaxKind::FloatingPointLiteral
                | JavaSyntaxKind::BooleanLiteral
                | JavaSyntaxKind::CharacterLiteral
                | JavaSyntaxKind::StringLiteral
                | JavaSyntaxKind::TextBlockLiteral
                | JavaSyntaxKind::NullLiteral
        )
    }

    fn starts_constructor_invocation_statement(&self) -> bool {
        let mut index = self.position();
        let mut saw_constructor_keyword = false;
        while !matches!(
            self.kind_at(index),
            JavaSyntaxKind::Eof | JavaSyntaxKind::Semicolon | JavaSyntaxKind::RBrace
        ) {
            if matches!(
                self.kind_at(index),
                JavaSyntaxKind::LBrace | JavaSyntaxKind::RParen | JavaSyntaxKind::RBracket
            ) && index == self.position()
            {
                return false;
            }

            if matches!(
                self.kind_at(index),
                JavaSyntaxKind::ThisKw | JavaSyntaxKind::SuperKw
            ) && self.kind_at(index + 1) == JavaSyntaxKind::LParen
            {
                saw_constructor_keyword = true;
            }
            index += 1;
        }
        saw_constructor_keyword && self.kind_at(index) == JavaSyntaxKind::Semicolon
    }

    fn starts_expression_name_qualified_constructor_invocation(&self) -> bool {
        if !self.at_name_segment() {
            return false;
        }

        let mut index = self.position() + 1;
        while self.kind_at(index) == JavaSyntaxKind::Dot
            && self.is_name_segment_at_offset(index + 1)
        {
            index += 2;
        }

        if self.kind_at(index) != JavaSyntaxKind::Dot {
            return false;
        }

        self.index_starts_constructor_super_suffix(index)
    }

    fn dot_starts_constructor_super_suffix(&self) -> bool {
        self.current_kind() == JavaSyntaxKind::Dot
            && self.index_starts_constructor_super_suffix(self.position())
    }

    fn index_starts_constructor_super_suffix(&self, dot_index: usize) -> bool {
        let mut index = dot_index + 1;
        if self.kind_at(index) == JavaSyntaxKind::Lt {
            index = self.skip_balanced_type_arguments_from(index);
        }

        self.kind_at(index) == JavaSyntaxKind::SuperKw
            && self.kind_at(index + 1) == JavaSyntaxKind::LParen
    }

    fn skip_type_from(&self, mut index: usize) -> usize {
        index = self.skip_type_base_from(index);

        loop {
            index = self.skip_annotations_from(index);
            if self.kind_at(index) == JavaSyntaxKind::LBracket
                && self.kind_at(index + 1) == JavaSyntaxKind::RBracket
            {
                index += 2;
            } else {
                break;
            }
        }

        index
    }

    fn skip_type_base_from(&self, mut index: usize) -> usize {
        index = self.skip_annotations_from(index);

        if matches!(
            self.kind_at(index),
            JavaSyntaxKind::BooleanKw
                | JavaSyntaxKind::ByteKw
                | JavaSyntaxKind::CharKw
                | JavaSyntaxKind::DoubleKw
                | JavaSyntaxKind::FloatKw
                | JavaSyntaxKind::IntKw
                | JavaSyntaxKind::LongKw
                | JavaSyntaxKind::ShortKw
        ) {
            return index + 1;
        }

        if self.is_name_segment_at_offset(index) {
            index += 1;
            if self.kind_at(index) == JavaSyntaxKind::Lt {
                index = self.skip_balanced_type_arguments_from(index);
            }
            while self.kind_at(index) == JavaSyntaxKind::Dot {
                let after_dot = self.skip_annotations_from(index + 1);
                if !self.is_name_segment_at_offset(after_dot) {
                    break;
                }
                index = after_dot + 1;
                if self.kind_at(index) == JavaSyntaxKind::Lt {
                    index = self.skip_balanced_type_arguments_from(index);
                }
            }
        }

        index
    }

    fn new_expression_is_array_creation(&self) -> bool {
        if self.current_kind() != JavaSyntaxKind::NewKw {
            return false;
        }

        let mut index = self.position() + 1;
        if self.kind_at(index) == JavaSyntaxKind::Lt {
            index = self.skip_balanced_type_arguments_from(index);
        }

        index = self.skip_type_base_from(index);
        index = self.skip_annotations_from(index);
        self.kind_at(index) == JavaSyntaxKind::LBracket
    }

    fn skip_balanced_type_arguments_from(&self, mut index: usize) -> usize {
        if self.kind_at(index) != JavaSyntaxKind::Lt {
            return index;
        }

        let mut depth = 0usize;
        while self.kind_at(index) != JavaSyntaxKind::Eof {
            match self.kind_at(index) {
                JavaSyntaxKind::Lt => {
                    depth += 1;
                    index += 1;
                }
                JavaSyntaxKind::Gt => {
                    depth = depth.saturating_sub(1);
                    index += 1;
                    if depth == 0 {
                        return index;
                    }
                }
                JavaSyntaxKind::RShift => {
                    if depth < 2 {
                        return index;
                    }

                    depth -= 2;
                    index += 1;
                    if depth == 0 {
                        return index;
                    }
                }
                JavaSyntaxKind::UnsignedRShift => {
                    if depth < 3 {
                        return index;
                    }

                    depth -= 3;
                    index += 1;
                    if depth == 0 {
                        return index;
                    }
                }
                _ => index += 1,
            }
        }
        index
    }

    fn is_name_segment_at_offset(&self, index: usize) -> bool {
        self.kind_at(index) == JavaSyntaxKind::Identifier
    }

    fn at_variable_identifier(&self) -> bool {
        self.is_variable_identifier_at_offset(self.position())
    }

    fn is_variable_identifier_at_offset(&self, index: usize) -> bool {
        matches!(
            self.kind_at(index),
            JavaSyntaxKind::Identifier | JavaSyntaxKind::UnderscoreKw
        )
    }

    fn starts_package_declaration(&self) -> bool {
        let index = self.skip_annotations_from(self.position());
        self.kind_at(index) == JavaSyntaxKind::PackageKw
    }

    fn starts_module_declaration(&self) -> bool {
        let mut index = self.skip_annotations_from(self.position());
        if self.text_at(index) == Some("open") {
            index += 1;
        }

        self.text_at(index) == Some("module")
    }

    fn starts_misspelled_non_sealed_type_declaration(&self) -> bool {
        self.current_text() == Some("non")
            && self.nth_kind(1) == JavaSyntaxKind::Minus
            && matches!(self.text_at(self.position() + 2), Some(text) if text.starts_with("sealed"))
            && self.nth_is_name_segment(3)
    }

    fn starts_top_level_type_declaration(&self) -> bool {
        let index = self.skip_type_modifiers_from(self.position());
        matches!(
            self.kind_at(index),
            JavaSyntaxKind::ClassKw | JavaSyntaxKind::InterfaceKw | JavaSyntaxKind::EnumKw
        ) || self.text_at(index) == Some("record")
            || (self.kind_at(index) == JavaSyntaxKind::At
                && self.kind_at(index + 1) == JavaSyntaxKind::InterfaceKw)
    }

    fn starts_compact_member_declaration(&self) -> bool {
        let index = self.skip_type_modifiers_from(self.position());
        self.is_non_void_type_start_at(index)
            || (self.kind_at(index) == JavaSyntaxKind::VoidKw
                && self.is_name_segment_at_offset(index + 1)
                && self.kind_at(index + 2) == JavaSyntaxKind::LParen)
    }

    fn is_type_start_at(&self, index: usize) -> bool {
        self.is_non_void_type_start_at(index) || self.kind_at(index) == JavaSyntaxKind::VoidKw
    }

    fn is_non_void_type_start_at(&self, index: usize) -> bool {
        self.is_name_segment_at_offset(index) || self.is_primitive_type_start_at(index)
    }

    fn is_primitive_type_start_at(&self, index: usize) -> bool {
        matches!(
            self.kind_at(index),
            JavaSyntaxKind::BooleanKw
                | JavaSyntaxKind::ByteKw
                | JavaSyntaxKind::CharKw
                | JavaSyntaxKind::DoubleKw
                | JavaSyntaxKind::FloatKw
                | JavaSyntaxKind::IntKw
                | JavaSyntaxKind::LongKw
                | JavaSyntaxKind::ShortKw
        )
    }

    fn at_type_identifier(&self) -> bool {
        self.current_kind() == JavaSyntaxKind::Identifier
            && !matches!(
                self.current_text(),
                Some("permits" | "record" | "sealed" | "var" | "yield")
            )
    }

    fn member_header_ends_with_block(&self) -> bool {
        let mut index = self.position();
        while self.kind_at(index) != JavaSyntaxKind::Eof {
            match self.kind_at(index) {
                JavaSyntaxKind::LBrace => return true,
                JavaSyntaxKind::Semicolon => return false,
                JavaSyntaxKind::LParen => {
                    index = self.skip_balanced_from(
                        index,
                        JavaSyntaxKind::LParen,
                        JavaSyntaxKind::RParen,
                    );
                }
                JavaSyntaxKind::LBracket => {
                    index = self.skip_balanced_from(
                        index,
                        JavaSyntaxKind::LBracket,
                        JavaSyntaxKind::RBracket,
                    );
                }
                _ => index += 1,
            }
        }

        false
    }

    fn at_module_directive_start(&self) -> bool {
        matches!(
            self.current_text(),
            Some("requires" | "exports" | "opens" | "uses" | "provides")
        )
    }
}
