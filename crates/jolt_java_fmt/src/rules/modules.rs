use std::ops::Range;

use jolt_fmt_ir::{Doc, concat, empty_line, hard_line, text};
use jolt_java_syntax::{
    ModuleDeclaration, ModuleDirective, ModuleDirectiveRole, ModuleNameListEntry, NameSyntax,
};

use crate::helpers::blocks::{join_empty_lines, join_hard_lines};
use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_comment, format_construct_leading_comments,
    format_inline_trailing_comment_list, format_separator_with_comments,
    format_token_after_relocated_leading_comments, format_token_before_relocated_trailing_comments,
    format_token_with_comments, split_leading_comment_barrier_runs, token_has_comments,
};
use crate::helpers::formatter_ignore::{
    formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs,
    relative_token_range_between,
};
use crate::rules::names::{format_name, name_key};

pub(crate) fn format_module_declaration<'source>(
    module: &ModuleDeclaration<'source>,
) -> Doc<'source> {
    concat([
        module
            .open_token()
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, |token| {
                concat([format_token_with_comments(token), text(" ")])
            }),
        module
            .module_token()
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, |token| {
                concat([format_token_with_comments(token), text(" ")])
            }),
        module
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_name(&name)),
        module
            .open_brace()
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, |token| {
                concat([text(" "), format_token_with_comments(token)])
            }),
        indent_module_body(format_module_directives(module)),
        hard_line(),
        module
            .close_brace()
            .as_ref()
            .map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
    ])
}

fn indent_module_body(directives: Option<Doc<'_>>) -> Doc<'_> {
    directives.map_or_else(jolt_fmt_ir::nil, |directives| {
        jolt_fmt_ir::indent(concat([hard_line(), directives]))
    })
}

fn format_module_directives<'source>(module: &ModuleDeclaration<'source>) -> Option<Doc<'source>> {
    let directives = module.directives().collect::<Vec<_>>();
    if directives.is_empty() {
        return None;
    }

    let ignored_ranges = formatter_ignore_ranges(
        module.source_text(),
        module.text_range().start().get(),
        module.token_iter(),
    );
    let directive_ranges = directives
        .iter()
        .map(|directive| module_directive_token_range(directive, module.text_range().start().get()))
        .collect::<Vec<_>>();
    let ignored_runs = formatter_ignore_runs(&ignored_ranges, &directive_ranges);
    if !ignored_runs.is_empty() {
        return Some(format_module_directives_with_ignored(
            directives,
            &ignored_runs,
        ));
    }

    Some(format_module_directive_segments(directives))
}

fn format_module_directives_with_ignored<'source>(
    directives: Vec<ModuleDirective<'source>>,
    ignored_runs: &[crate::helpers::formatter_ignore::FormatterIgnoreRun<'source>],
) -> Doc<'source> {
    let mut sections = Vec::new();
    let mut segment = Vec::new();
    let mut ignored_index = 0;
    let mut skip_index = 0;

    for (directive_index, directive) in directives.into_iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == directive_index
        {
            push_module_directive_segment(&mut sections, &mut segment);
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

    push_module_directive_segment(&mut sections, &mut segment);
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

fn push_module_directive_segment<'source>(
    sections: &mut Vec<ModuleDirectiveSection<'source>>,
    segment: &mut Vec<ModuleDirective<'source>>,
) {
    if segment.is_empty() {
        return;
    }
    sections.push(ModuleDirectiveSection {
        doc: format_module_directive_segments(std::mem::take(segment)),
        hard_line_after: false,
    });
}

fn join_module_directive_sections(sections: Vec<ModuleDirectiveSection<'_>>) -> Doc<'_> {
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

struct ModuleDirectiveSection<'source> {
    doc: Doc<'source>,
    hard_line_after: bool,
}

fn format_module_directive_segments(directives: Vec<ModuleDirective<'_>>) -> Doc<'_> {
    let runs = split_leading_comment_barrier_runs(directives, |directive| {
        directive
            .first_token()
            .is_some_and(|token| !token.leading_comments().is_empty())
    });

    join_empty_lines(runs.into_iter().map(|run| {
        format_module_directive_run(
            run.into_iter()
                .map(|directive| FormattedModuleDirective::from_directive(&directive))
                .collect(),
        )
    }))
}

fn module_directive_token_range(
    directive: &ModuleDirective<'_>,
    module_start: usize,
) -> Option<Range<usize>> {
    Some(relative_token_range_between(
        &directive.first_token()?,
        &directive.last_token()?,
        module_start,
    ))
}

fn format_module_directive_run(directives: Vec<FormattedModuleDirective<'_>>) -> Doc<'_> {
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

struct FormattedModuleDirective<'source> {
    first_token: Option<jolt_java_syntax::JavaSyntaxToken<'source>>,
    last_token: Option<jolt_java_syntax::JavaSyntaxToken<'source>>,
    kind_order: ModuleDirectiveKindOrder,
    primary_name: String,
    doc: Doc<'source>,
}

impl<'source> FormattedModuleDirective<'source> {
    fn from_directive(directive: &ModuleDirective<'source>) -> Self {
        let role = directive
            .directive_role()
            .expect("clean module directive should expose a directive role");
        let primary_name = module_directive_primary_name(&role);
        let kind_order = module_directive_kind_order(&role);
        let doc = format_module_directive_doc(directive, &role);

        Self {
            first_token: directive.first_token(),
            last_token: directive.last_token(),
            kind_order,
            primary_name,
            doc,
        }
    }

    fn into_doc(self) -> Doc<'source> {
        let doc = concat([
            self.doc,
            self.last_token.map_or_else(jolt_fmt_ir::nil, |token| {
                format_inline_trailing_comment_list(token.trailing_comments())
            }),
        ]);
        if self
            .first_token
            .as_ref()
            .is_none_or(|token| token.leading_comments().is_empty())
        {
            doc
        } else {
            let leading_comments = self
                .first_token
                .into_iter()
                .flat_map(|token| token.leading_comments());
            concat([
                join_hard_lines(leading_comments.map(|comment| format_comment(&comment))),
                hard_line(),
                doc,
            ])
        }
    }
}

fn format_module_directive_doc<'source>(
    directive: &ModuleDirective<'source>,
    role: &ModuleDirectiveRole<'source>,
) -> Doc<'source> {
    match (directive, role) {
        (
            ModuleDirective::RequiresDirective(requires),
            ModuleDirectiveRole::Requires {
                module,
                is_static,
                is_transitive,
            },
        ) => {
            let mut parts = vec![format_directive_head(requires.requires_token().as_ref())];
            if *is_static {
                parts.push(format_directive_middle_token(
                    requires.static_token().as_ref(),
                ));
            }
            if *is_transitive {
                parts.push(format_directive_middle_token(
                    requires.transitive_token().as_ref(),
                ));
            }
            parts.push(format_name(module));
            parts.push(format_directive_semicolon(requires.semicolon().as_ref()));
            concat(parts)
        }
        (
            ModuleDirective::ExportsDirective(exports),
            ModuleDirectiveRole::Exports { package, .. },
        ) => format_module_name_list_directive(
            exports.exports_token().as_ref(),
            package,
            exports.to_token().as_ref(),
            exports.target_entries().collect(),
            exports.semicolon().as_ref(),
        ),
        (ModuleDirective::OpensDirective(opens), ModuleDirectiveRole::Opens { package, .. }) => {
            format_module_name_list_directive(
                opens.opens_token().as_ref(),
                package,
                opens.to_token().as_ref(),
                opens.target_entries().collect(),
                opens.semicolon().as_ref(),
            )
        }
        (ModuleDirective::UsesDirective(uses), ModuleDirectiveRole::Uses { service }) => concat([
            format_directive_head(uses.uses_token().as_ref()),
            format_name(service),
            format_directive_semicolon(uses.semicolon().as_ref()),
        ]),
        (
            ModuleDirective::ProvidesDirective(provides),
            ModuleDirectiveRole::Provides { service, .. },
        ) => format_module_name_list_directive(
            provides.provides_token().as_ref(),
            service,
            provides.with_token().as_ref(),
            provides.implementation_entries().collect(),
            provides.semicolon().as_ref(),
        ),
        _ => unreachable!("module directive role should match directive variant"),
    }
}

fn format_directive_head<'source>(
    token: Option<&jolt_java_syntax::JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    token.map_or_else(jolt_fmt_ir::nil, |token| {
        concat([
            format_token_after_relocated_leading_comments(token, TrailingTrivia::Preserve),
            text(" "),
        ])
    })
}

fn format_directive_middle_token<'source>(
    token: Option<&jolt_java_syntax::JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    token.map_or_else(jolt_fmt_ir::nil, |token| {
        concat([format_token_with_comments(token), text(" ")])
    })
}

fn format_directive_semicolon<'source>(
    token: Option<&jolt_java_syntax::JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    token.map_or_else(jolt_fmt_ir::nil, |token| {
        format_token_before_relocated_trailing_comments(token, LeadingTrivia::Preserve)
    })
}

fn format_module_name_list_directive<'source>(
    keyword: Option<&jolt_java_syntax::JavaSyntaxToken<'source>>,
    subject: &NameSyntax<'source>,
    connective: Option<&jolt_java_syntax::JavaSyntaxToken<'source>>,
    entries: Vec<ModuleNameListEntry<'source>>,
    semicolon: Option<&jolt_java_syntax::JavaSyntaxToken<'source>>,
) -> Doc<'source> {
    if entries.is_empty() {
        return concat([
            format_directive_head(keyword),
            format_name(subject),
            format_directive_semicolon(semicolon),
        ]);
    }

    concat([
        format_directive_head(keyword),
        format_name(subject),
        text(" "),
        connective.map_or_else(jolt_fmt_ir::nil, format_token_with_comments),
        format_module_name_list(entries),
        format_directive_semicolon(semicolon),
    ])
}

fn format_module_name_list(entries: Vec<ModuleNameListEntry<'_>>) -> Doc<'_> {
    let should_break = entries.iter().any(|entry| {
        name_has_leading_comments(&entry.name)
            || entry.comma.as_ref().is_some_and(token_has_comments)
    });

    if should_break {
        return jolt_fmt_ir::indent(concat([
            hard_line(),
            format_module_name_entries_broken(entries),
        ]));
    }

    concat([text(" "), format_module_name_entries_inline(entries)])
}

fn format_module_name_entries_inline(entries: Vec<ModuleNameListEntry<'_>>) -> Doc<'_> {
    let mut docs = Vec::new();

    for entry in entries {
        docs.push(format_name(&entry.name));
        if let Some(comma) = entry.comma {
            docs.push(format_separator_with_comments(&comma, text(" ")));
        }
    }

    concat(docs)
}

fn format_module_name_entries_broken(entries: Vec<ModuleNameListEntry<'_>>) -> Doc<'_> {
    let mut docs = Vec::new();
    let entries_len = entries.len();

    for (index, entry) in entries.into_iter().enumerate() {
        docs.push(concat([
            format_construct_leading_comments(entry.name.first_token().as_ref()),
            format_name(&entry.name),
        ]));
        if let Some(comma) = entry.comma {
            docs.push(format_separator_with_comments(&comma, jolt_fmt_ir::line()));
        } else if index + 1 < entries_len {
            docs.push(hard_line());
        }
    }

    concat(docs)
}

fn name_has_leading_comments(name: &NameSyntax<'_>) -> bool {
    name.first_token()
        .is_some_and(|token| !token.leading_comments().is_empty())
}

fn module_directive_primary_name(role: &ModuleDirectiveRole<'_>) -> String {
    match role {
        ModuleDirectiveRole::Requires { module, .. } => name_key(module),
        ModuleDirectiveRole::Exports { package, .. }
        | ModuleDirectiveRole::Opens { package, .. } => name_key(package),
        ModuleDirectiveRole::Uses { service } | ModuleDirectiveRole::Provides { service, .. } => {
            name_key(service)
        }
    }
}

const fn module_directive_kind_order(role: &ModuleDirectiveRole<'_>) -> ModuleDirectiveKindOrder {
    match role {
        ModuleDirectiveRole::Requires { .. } => ModuleDirectiveKindOrder::Requires,
        ModuleDirectiveRole::Exports { .. } => ModuleDirectiveKindOrder::Exports,
        ModuleDirectiveRole::Opens { .. } => ModuleDirectiveKindOrder::Opens,
        ModuleDirectiveRole::Uses { .. } => ModuleDirectiveKindOrder::Uses,
        ModuleDirectiveRole::Provides { .. } => ModuleDirectiveKindOrder::Provides,
    }
}
