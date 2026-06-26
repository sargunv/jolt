use super::{JavaSyntaxKind, Parser};

mod declaration_predicates;
mod delimiters;
mod expression_predicates;
mod identifiers;
mod lookahead;
mod pattern_predicates;
mod recovery;
mod statement_predicates;
mod token_predicates;
mod type_arguments;

pub(in crate::parser::grammar) use lookahead::JavaLookahead;
