#[path = "identifiers.rs"]
mod identifiers;
pub(in crate::parser::grammar) use identifiers::is_identifier_like_kind;
#[path = "lookahead.rs"]
mod lookahead;
#[path = "recovery.rs"]
mod recovery;
#[path = "semi.rs"]
mod semi;
#[path = "soft_keywords.rs"]
mod soft_keywords;
pub(in crate::parser::grammar) mod token_sets;
