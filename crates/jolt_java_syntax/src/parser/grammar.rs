use crate::JavaSyntaxKind;

use super::source::{ParseEvents, Parser};

#[path = "grammar/compilation_unit.rs"]
mod compilation_unit;
#[path = "grammar/declarations.rs"]
mod declarations;
#[path = "grammar/expressions.rs"]
mod expressions;
#[path = "grammar/patterns.rs"]
mod patterns;
#[path = "grammar/statements.rs"]
mod statements;
#[path = "grammar/types.rs"]
mod types;
#[path = "grammar/util.rs"]
mod util;
