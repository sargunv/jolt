//! Type, record and match-all patterns.

use jolt_fmt::{Doc, cat};
use jolt_java_syntax::{
    ComponentPattern, ComponentPatternList, MatchAllPattern, RecordPattern, TypePattern,
};
use jolt_syntax::AstNode;

use crate::JavaFormatter;
use crate::helpers::lists::{ListLayout, delimited};
use crate::rules::types::inline_modifiers;

/// `modifiers Type name dims`, modifiers inline.
pub(crate) fn type_pattern<'a>(f: &mut JavaFormatter<'a>, node: TypePattern<'a>) -> Doc {
    let modifiers = inline_modifiers(f, node.modifiers());
    let ty = f.fmt_opt(node.r#type());
    let name = f.token_opt(node.name());
    let dimensions = f.fmt_opt(node.dimensions());
    cat![f; modifiers, ty, f.space(), name, dimensions]
}

/// `Type(component, …)` with the components as a break-all list.
pub(crate) fn record_pattern<'a>(f: &mut JavaFormatter<'a>, node: RecordPattern<'a>) -> Doc {
    let ty = f.fmt_opt(node.r#type());
    let components = delimited(
        f,
        node.syntax(),
        node.open_paren(),
        node.components()
            .into_iter()
            .flat_map(ComponentPatternList::elements),
        node.close_paren(),
        ListLayout::Plain,
    );
    cat![f; ty, components]
}

pub(crate) fn component_pattern<'a>(f: &mut JavaFormatter<'a>, node: ComponentPattern<'a>) -> Doc {
    f.fmt_opt(node.pattern())
}

pub(crate) fn match_all_pattern<'a>(f: &mut JavaFormatter<'a>, node: MatchAllPattern<'a>) -> Doc {
    f.token_opt(node.underscore())
}
