use std::ops::Range;

use jolt_fmt_ir::{Doc, concat, empty_line, hard_line, text};
use jolt_java_syntax::{
    JavaSyntaxToken, ModuleDeclaration, ModuleDirective, ModuleDirectiveRole, ModuleNameListEntry,
    NameSyntax,
};

use crate::context::JavaFormatter;
use crate::helpers::blocks::{join_empty_lines, join_hard_lines};
use crate::helpers::comments::{
    comment_forces_line, format_comment, format_leading_comments,
    format_trailing_comments_before_line_break,
};
use crate::helpers::formatter_ignore::{
    formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs,
};
use crate::rules::names::{format_name, name_key};

pub(crate) fn format_module_declaration(
    module: &ModuleDeclaration,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    concat([
        if module.is_open() {
            text("open module ")
        } else {
            text("module ")
        },
        module
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_name(&name)),
        text(" {"),
        indent_module_body(format_module_directives(module, formatter)),
        hard_line(),
        text("}"),
    ])
}

fn indent_module_body(directives: Option<Doc>) -> Doc {
    directives.map_or_else(jolt_fmt_ir::nil, |directives| {
        jolt_fmt_ir::indent(concat([hard_line(), directives]))
    })
}

fn format_module_directives(
    module: &ModuleDeclaration,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc> {
    let directives = module.directives().collect::<Vec<_>>();
    if directives.is_empty() {
        return None;
    }

    let ignored_ranges = formatter_ignore_ranges(&module.source_text());
    let directive_ranges = directives
        .iter()
        .map(|directive| module_directive_token_range(directive, module.text_range().start().get()))
        .collect::<Vec<_>>();
    let ignored_runs = formatter_ignore_runs(&ignored_ranges, &directive_ranges);
    if !ignored_runs.is_empty() {
        return Some(format_module_directives_with_ignored(
            directives,
            &ignored_runs,
            formatter,
        ));
    }

    Some(format_module_directive_segments(directives, formatter))
}

fn format_module_directives_with_ignored(
    directives: Vec<ModuleDirective>,
    ignored_runs: &[crate::helpers::formatter_ignore::FormatterIgnoreRun],
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let mut sections = Vec::new();
    let mut segment = Vec::new();
    let mut ignored_index = 0;
    let mut skip_index = 0;

    for (directive_index, directive) in directives.into_iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == directive_index
        {
            push_module_directive_segment(&mut sections, &mut segment, formatter);
            let run = &ignored_runs[ignored_index];
            sections.push(ModuleDirectiveSection {
                doc: formatter_ignore_run_doc(run),
                hard_line_after: !run.include_on_marker,
            });
            ignored_index += 1;
        }

        while skip_index < ignored_runs.len()
            && ignored_runs[skip_index].skip_end <= directive_index
        {
            skip_index += 1;
        }

        if skip_index < ignored_runs.len() && ignored_runs[skip_index].skips(directive_index) {
            continue;
        }

        segment.push(directive);
    }

    push_module_directive_segment(&mut sections, &mut segment, formatter);
    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        sections.push(ModuleDirectiveSection {
            doc: formatter_ignore_run_doc(run),
            hard_line_after: !run.include_on_marker,
        });
        ignored_index += 1;
    }

    join_module_directive_sections(sections)
}

fn push_module_directive_segment(
    sections: &mut Vec<ModuleDirectiveSection>,
    segment: &mut Vec<ModuleDirective>,
    formatter: &JavaFormatter<'_>,
) {
    if segment.is_empty() {
        return;
    }
    sections.push(ModuleDirectiveSection {
        doc: format_module_directive_segments(std::mem::take(segment), formatter),
        hard_line_after: false,
    });
}

fn join_module_directive_sections(sections: Vec<ModuleDirectiveSection>) -> Doc {
    let mut joined = Vec::new();
    let mut previous_hard_line_after = false;
    for section in sections {
        if !joined.is_empty() {
            joined.push(if previous_hard_line_after {
                hard_line()
            } else {
                empty_line()
            });
        }
        joined.push(section.doc);
        previous_hard_line_after = section.hard_line_after;
    }
    concat(joined)
}

struct ModuleDirectiveSection {
    doc: Doc,
    hard_line_after: bool,
}

fn format_module_directive_segments(
    directives: Vec<ModuleDirective>,
    formatter: &JavaFormatter<'_>,
) -> Doc {
    let mut runs: Vec<Vec<FormattedModuleDirective>> = Vec::new();
    let mut current_run = Vec::new();

    for directive in directives {
        let tokens = directive.tokens();
        if formatter.comments().has_leading_comment_for_tokens(&tokens) && !current_run.is_empty() {
            runs.push(current_run);
            current_run = Vec::new();
        }
        current_run.push(FormattedModuleDirective::from_directive(
            &directive, formatter,
        ));
    }
    if !current_run.is_empty() {
        runs.push(current_run);
    }

    join_empty_lines(runs.into_iter().map(format_module_directive_run).collect())
}

fn module_directive_token_range(
    directive: &ModuleDirective,
    module_start: usize,
) -> Option<Range<usize>> {
    let tokens = directive.tokens();
    let first = tokens.first()?;
    let last = tokens.last()?;
    Some(
        first.token_text_range().start().get() - module_start
            ..last.token_text_range().end().get() - module_start,
    )
}

fn format_module_directive_run(directives: Vec<FormattedModuleDirective>) -> Doc {
    let mut directives = directives;
    directives.sort_by(|lhs, rhs| {
        lhs.kind_order
            .cmp(&rhs.kind_order)
            .then_with(|| lhs.primary_name.cmp(&rhs.primary_name))
    });

    let mut groups = Vec::new();
    let mut current_kind = None;
    let mut current_group = Vec::new();

    for directive in directives {
        if current_kind.is_some_and(|kind| kind != directive.kind_order) {
            groups.push(join_hard_lines(current_group));
            current_group = Vec::new();
        }
        current_kind = Some(directive.kind_order);
        current_group.push(directive.into_doc());
    }
    if !current_group.is_empty() {
        groups.push(join_hard_lines(current_group));
    }

    join_empty_lines(groups)
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum ModuleDirectiveKindOrder {
    Requires,
    Exports,
    Opens,
    Uses,
    Provides,
}

struct FormattedModuleDirective {
    leading_comments: Vec<jolt_java_syntax::JavaComment>,
    trailing_comments: Vec<jolt_java_syntax::JavaComment>,
    kind_order: ModuleDirectiveKindOrder,
    primary_name: String,
    doc: Doc,
}

impl FormattedModuleDirective {
    fn from_directive(directive: &ModuleDirective, formatter: &JavaFormatter<'_>) -> Self {
        let role = directive
            .directive_role()
            .expect("clean module directive should expose a directive role");
        let primary_name = module_directive_primary_name(&role);
        let kind_order = module_directive_kind_order(&role);
        let doc = format_module_directive_doc(directive, &role);

        let tokens = directive.tokens();
        Self {
            leading_comments: formatter
                .comments()
                .leading_comments_for_tokens(&tokens)
                .to_vec(),
            trailing_comments: formatter
                .comments()
                .trailing_comments_for_tokens(&tokens)
                .to_vec(),
            kind_order,
            primary_name,
            doc,
        }
    }

    fn into_doc(self) -> Doc {
        let doc = concat([
            self.doc,
            format_inline_trailing_comments(&self.trailing_comments),
        ]);
        if self.leading_comments.is_empty() {
            doc
        } else {
            concat([
                join_hard_lines(self.leading_comments.iter().map(format_comment).collect()),
                hard_line(),
                doc,
            ])
        }
    }
}

fn format_module_directive_doc(directive: &ModuleDirective, role: &ModuleDirectiveRole) -> Doc {
    match (directive, role) {
        (
            ModuleDirective::RequiresDirective(_),
            ModuleDirectiveRole::Requires {
                module,
                is_static,
                is_transitive,
            },
        ) => {
            let mut parts = vec![text("requires ")];
            if *is_static {
                parts.push(text("static "));
            }
            if *is_transitive {
                parts.push(text("transitive "));
            }
            parts.push(format_name(module));
            parts.push(text(";"));
            concat(parts)
        }
        (
            ModuleDirective::ExportsDirective(exports),
            ModuleDirectiveRole::Exports { package, .. },
        ) => format_module_name_list_directive(
            "exports",
            package,
            "to",
            exports.target_entries().collect(),
        ),
        (ModuleDirective::OpensDirective(opens), ModuleDirectiveRole::Opens { package, .. }) => {
            format_module_name_list_directive(
                "opens",
                package,
                "to",
                opens.target_entries().collect(),
            )
        }
        (ModuleDirective::UsesDirective(_), ModuleDirectiveRole::Uses { service }) => {
            concat([text("uses "), format_name(service), text(";")])
        }
        (
            ModuleDirective::ProvidesDirective(provides),
            ModuleDirectiveRole::Provides { service, .. },
        ) => format_module_name_list_directive(
            "provides",
            service,
            "with",
            provides.implementation_entries().collect(),
        ),
        _ => unreachable!("module directive role should match directive variant"),
    }
}

fn format_inline_trailing_comments(comments: &[jolt_java_syntax::JavaComment]) -> Doc {
    concat(
        comments
            .iter()
            .map(|comment| concat([text(" "), format_comment(comment)]))
            .collect::<Vec<_>>(),
    )
}

fn format_module_name_list_directive(
    keyword: &str,
    subject: &NameSyntax,
    connective: &str,
    entries: Vec<ModuleNameListEntry>,
) -> Doc {
    if entries.is_empty() {
        return concat([
            text(keyword.to_owned()),
            text(" "),
            format_name(subject),
            text(";"),
        ]);
    }

    concat([
        text(keyword.to_owned()),
        text(" "),
        format_name(subject),
        text(" "),
        text(connective.to_owned()),
        format_module_name_list(entries),
        text(";"),
    ])
}

fn format_module_name_list(entries: Vec<ModuleNameListEntry>) -> Doc {
    let should_break = entries.iter().any(|entry| {
        name_has_leading_comments(&entry.name)
            || entry
                .comma
                .as_ref()
                .is_some_and(separator_token_has_comments)
    });

    if should_break {
        return jolt_fmt_ir::indent(concat([
            hard_line(),
            format_module_name_entries_broken(entries),
        ]));
    }

    concat([text(" "), format_module_name_entries_inline(entries)])
}

fn format_module_name_entries_inline(entries: Vec<ModuleNameListEntry>) -> Doc {
    let mut docs = Vec::new();

    for entry in entries {
        docs.push(format_name(&entry.name));
        if let Some(comma) = entry.comma {
            docs.push(format_module_name_separator_inline(&comma));
        }
    }

    concat(docs)
}

fn format_module_name_entries_broken(entries: Vec<ModuleNameListEntry>) -> Doc {
    let mut docs = Vec::new();
    let entries_len = entries.len();

    for (index, entry) in entries.into_iter().enumerate() {
        docs.push(concat([
            format_construct_leading_comments(&entry.name.tokens()),
            format_name(&entry.name),
        ]));
        if let Some(comma) = entry.comma {
            docs.push(format_module_name_separator_broken(&comma));
        } else if index + 1 < entries_len {
            docs.push(hard_line());
        }
    }

    concat(docs)
}

fn format_module_name_separator_inline(comma: &JavaSyntaxToken) -> Doc {
    concat([
        format_leading_comments(comma),
        text(","),
        format_trailing_comments_before_line_break(comma),
        text(" "),
    ])
}

fn format_module_name_separator_broken(comma: &JavaSyntaxToken) -> Doc {
    concat([
        format_leading_comments(comma),
        text(","),
        format_trailing_comments_before_line_break(comma),
        if comma.trailing_comments().iter().any(comment_forces_line) {
            hard_line()
        } else {
            jolt_fmt_ir::line()
        },
    ])
}

fn format_construct_leading_comments(tokens: &[JavaSyntaxToken]) -> Doc {
    tokens
        .first()
        .map_or_else(jolt_fmt_ir::nil, format_leading_comments)
}

fn separator_token_has_comments(token: &JavaSyntaxToken) -> bool {
    !token.leading_comments().is_empty() || !token.trailing_comments().is_empty()
}

fn name_has_leading_comments(name: &NameSyntax) -> bool {
    name.tokens()
        .first()
        .is_some_and(|token| !token.leading_comments().is_empty())
}

fn module_directive_primary_name(role: &ModuleDirectiveRole) -> String {
    match role {
        ModuleDirectiveRole::Requires { module, .. } => name_key(module),
        ModuleDirectiveRole::Exports { package, .. }
        | ModuleDirectiveRole::Opens { package, .. } => name_key(package),
        ModuleDirectiveRole::Uses { service } | ModuleDirectiveRole::Provides { service, .. } => {
            name_key(service)
        }
    }
}

const fn module_directive_kind_order(role: &ModuleDirectiveRole) -> ModuleDirectiveKindOrder {
    match role {
        ModuleDirectiveRole::Requires { .. } => ModuleDirectiveKindOrder::Requires,
        ModuleDirectiveRole::Exports { .. } => ModuleDirectiveKindOrder::Exports,
        ModuleDirectiveRole::Opens { .. } => ModuleDirectiveKindOrder::Opens,
        ModuleDirectiveRole::Uses { .. } => ModuleDirectiveKindOrder::Uses,
        ModuleDirectiveRole::Provides { .. } => ModuleDirectiveKindOrder::Provides,
    }
}
