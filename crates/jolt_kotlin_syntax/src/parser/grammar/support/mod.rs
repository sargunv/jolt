#[path = "identifiers.rs"]
mod identifiers;
#[path = "lookahead.rs"]
mod lookahead;
#[path = "recovery.rs"]
mod recovery;
#[path = "semi.rs"]
mod semi;
#[path = "soft_keywords.rs"]
mod soft_keywords;
pub(in crate::parser::grammar) mod token_sets;
