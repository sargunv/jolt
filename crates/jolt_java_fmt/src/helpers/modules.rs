use jolt_fmt_ir::{Doc, concat, empty_line, hard_line, indent_by, join, text};

use crate::layout as wrap;
use crate::policy::JavaFormatPolicy;

pub(crate) enum ModuleDirectiveLayout {
    Requires {
        is_transitive: bool,
        is_static: bool,
        name: Doc,
    },
    Exports {
        package_name: Doc,
        targets: Vec<Doc>,
    },
    Opens {
        package_name: Doc,
        targets: Vec<Doc>,
    },
    Uses {
        service_name: Doc,
    },
    Provides {
        service_name: Doc,
        implementations: Vec<Doc>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ModuleDirectiveGroup {
    Requires,
    Exports,
    Opens,
    Uses,
    Provides,
}

impl ModuleDirectiveLayout {
    fn group(&self) -> ModuleDirectiveGroup {
        match self {
            Self::Requires { .. } => ModuleDirectiveGroup::Requires,
            Self::Exports { .. } => ModuleDirectiveGroup::Exports,
            Self::Opens { .. } => ModuleDirectiveGroup::Opens,
            Self::Uses { .. } => ModuleDirectiveGroup::Uses,
            Self::Provides { .. } => ModuleDirectiveGroup::Provides,
        }
    }
}

pub(crate) fn module_declaration(
    is_open: bool,
    name: Doc,
    directives: Vec<ModuleDirectiveLayout>,
    policy: JavaFormatPolicy,
) -> Doc {
    let mut header = Vec::new();
    if is_open {
        header.push(text("open "));
    }
    header.push(text("module "));
    header.push(name);
    header.push(text(" "));

    concat([concat(header), module_body(directives, policy)])
}

fn module_body(directives: Vec<ModuleDirectiveLayout>, policy: JavaFormatPolicy) -> Doc {
    let mut separators = Vec::with_capacity(directives.len().saturating_sub(1));
    let mut previous_group = None;
    for directive in &directives {
        if let Some(previous_group) = previous_group {
            separators.push(if previous_group == directive.group() {
                hard_line()
            } else {
                empty_line()
            });
        }
        previous_group = Some(directive.group());
    }

    wrap::braced_block_with_separators(
        directives
            .into_iter()
            .map(|directive| module_directive(directive, policy)),
        separators,
    )
}

fn module_directive(directive: ModuleDirectiveLayout, policy: JavaFormatPolicy) -> Doc {
    match directive {
        ModuleDirectiveLayout::Requires {
            is_transitive,
            is_static,
            name,
        } => requires_directive(is_transitive, is_static, name),
        ModuleDirectiveLayout::Exports {
            package_name,
            targets,
        } => package_target_directive("exports", package_name, targets, "to", policy),
        ModuleDirectiveLayout::Opens {
            package_name,
            targets,
        } => package_target_directive("opens", package_name, targets, "to", policy),
        ModuleDirectiveLayout::Uses { service_name } => {
            concat([text("uses "), service_name, text(";")])
        }
        ModuleDirectiveLayout::Provides {
            service_name,
            implementations,
        } => package_target_directive("provides", service_name, implementations, "with", policy),
    }
}

fn requires_directive(is_transitive: bool, is_static: bool, name: Doc) -> Doc {
    let mut parts = vec![text("requires ")];
    if is_transitive {
        parts.push(text("transitive "));
    }
    if is_static {
        parts.push(text("static "));
    }
    parts.push(name);
    parts.push(text(";"));
    concat(parts)
}

fn package_target_directive(
    keyword: &'static str,
    name: Doc,
    targets: Vec<Doc>,
    target_keyword: &'static str,
    policy: JavaFormatPolicy,
) -> Doc {
    let mut parts = vec![text(format!("{keyword} ")), name];
    if !targets.is_empty() {
        parts.push(text(format!(" {target_keyword}")));
        parts.push(indent_by(
            policy.continuation_indent_levels(),
            concat([hard_line(), join(concat([text(","), hard_line()]), targets)]),
        ));
    }
    parts.push(text(";"));
    concat(parts)
}
