use std::ops::Range;

use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{
    ModuleDeclaration, ModuleDirective, ModuleDirectiveRole, ModuleNameListEntry, NameSyntax,
    RecoveredSeparatedListEntry,
};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_comment, format_construct_leading_comments,
    format_inline_trailing_comment_list, format_leading_comment_runs,
    format_separator_with_comments, format_token_after_relocated_leading_comments,
    format_token_before_relocated_trailing_comments, format_token_sequence,
    format_token_with_comments, token_has_comments,
};
use crate::helpers::formatter_ignore::{
    formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs,
    relative_token_range_between,
};
use crate::rules::annotations::format_annotation;
use crate::rules::names::{NameSortKey, format_name};

pub(crate) fn format_module_declaration<'source>(
    module: &ModuleDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc_concat!(
        doc,
        [
            doc.concat_list(|annotations| {
                for annotation in module.annotations() {
                    let annotation = format_annotation(&annotation, annotations);
                    annotations.push(annotation);
                    let hard_line = annotations.hard_line();
                    annotations.push(hard_line);
                }
            }),
            module
                .open_token()
                .as_ref()
                .map_or_else(Doc::nil, |token| doc_concat!(
                    doc,
                    [format_token_with_comments(doc, token), doc.space()]
                ),),
            module
                .module_token()
                .as_ref()
                .map_or_else(Doc::nil, |token| doc_concat!(
                    doc,
                    [format_token_with_comments(doc, token), doc.space()]
                ),),
            module
                .name()
                .map_or_else(Doc::nil, |name| format_name(&name, doc)),
            module
                .open_brace()
                .as_ref()
                .map_or_else(Doc::nil, |token| doc_concat!(
                    doc,
                    [doc.space(), format_token_with_comments(doc, token)]
                ),),
            indent_module_body(format_module_directives(module, doc), doc),
            doc.hard_line(),
            module
                .close_brace()
                .as_ref()
                .map_or_else(Doc::nil, |token| format_token_with_comments(doc, token)),
        ]
    )
}

fn indent_module_body<'source>(
    directives: Option<Doc<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    directives.map_or_else(Doc::nil, |directives| {
        doc_indent!(doc, doc_concat!(doc, [doc.hard_line(), directives]))
    })
}

fn format_module_directives<'source>(
    module: &ModuleDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    let directives = module.directives_with_recovered().collect::<Vec<_>>();
    if directives.is_empty() {
        return None;
    }

    let ignored_ranges = formatter_ignore_ranges(
        module.source_text(),
        module.text_range().start().get(),
        module.token_iter(),
    );
    if ignored_ranges.is_empty() {
        return Some(format_module_directive_entries(directives, doc));
    }
    let directive_ranges = directives
        .iter()
        .map(|directive| {
            module_directive_entry_token_range(directive, module.text_range().start().get())
        })
        .collect::<Vec<_>>();
    let ignored_runs = formatter_ignore_runs(&ignored_ranges, &directive_ranges);
    Some(format_module_directives_with_ignored(
        directives,
        &ignored_runs,
        doc,
    ))
}

fn format_module_directives_with_ignored<'source>(
    directives: Vec<RecoveredSeparatedListEntry<'source, ModuleDirective<'source>>>,
    ignored_runs: &[crate::helpers::formatter_ignore::FormatterIgnoreRun<'source>],
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut sections = Vec::with_capacity(directives.len().saturating_add(ignored_runs.len()));
    let mut segment = Vec::with_capacity(directives.len());
    let mut ignored_index = 0;
    let mut skip_index = 0;

    for (directive_index, directive) in directives.into_iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == directive_index
        {
            push_module_directive_segment(&mut sections, &mut segment, doc);
            let run = &ignored_runs[ignored_index];
            sections.push(ModuleDirectiveSection {
                doc: formatter_ignore_run_doc(run, doc),
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

        match directive {
            RecoveredSeparatedListEntry::Entry(directive) => segment.push(directive),
            recovered => {
                push_module_directive_segment(&mut sections, &mut segment, doc);
                sections.push(ModuleDirectiveSection {
                    doc: format_recovered_module_directive_entry(recovered, doc),
                    hard_line_after: false,
                });
            }
        }
    }

    push_module_directive_segment(&mut sections, &mut segment, doc);
    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        sections.push(ModuleDirectiveSection {
            doc: formatter_ignore_run_doc(run, doc),
            hard_line_after: !run.include_on_marker,
        });
        ignored_index += 1;
    }

    join_module_directive_sections(sections, doc)
}

fn push_module_directive_segment<'source>(
    sections: &mut Vec<ModuleDirectiveSection<'source>>,
    segment: &mut Vec<ModuleDirective<'source>>,
    doc: &mut DocBuilder<'source>,
) {
    if segment.is_empty() {
        return;
    }
    sections.push(ModuleDirectiveSection {
        doc: format_module_directive_segments(std::mem::take(segment), doc),
        hard_line_after: false,
    });
}

fn join_module_directive_sections<'source>(
    sections: Vec<ModuleDirectiveSection<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut previous_hard_line_after = false;
    doc.concat_list(|joined| {
        for section in sections {
            if !joined.is_empty() {
                let separator = if previous_hard_line_after {
                    joined.hard_line()
                } else {
                    joined.empty_line()
                };
                joined.push(separator);
            }
            joined.push(section.doc);
            previous_hard_line_after = section.hard_line_after;
        }
    })
}

struct ModuleDirectiveSection<'source> {
    doc: Doc<'source>,
    hard_line_after: bool,
}

fn format_module_directive_segments<'source>(
    directives: Vec<ModuleDirective<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let directives = directives
        .into_iter()
        .map(|directive| FormattedModuleDirective::from_directive(&directive, doc))
        .collect::<Vec<_>>();
    format_leading_comment_runs(
        doc,
        directives,
        FormattedModuleDirective::has_leading_comments,
        format_module_directive_run,
    )
}

fn format_module_directive_entries<'source>(
    entries: Vec<RecoveredSeparatedListEntry<'source, ModuleDirective<'source>>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut sections = Vec::with_capacity(entries.len());
    let mut segment = Vec::with_capacity(entries.len());

    for entry in entries {
        match entry {
            RecoveredSeparatedListEntry::Entry(directive) => segment.push(directive),
            recovered => {
                push_module_directive_segment(&mut sections, &mut segment, doc);
                sections.push(ModuleDirectiveSection {
                    doc: format_recovered_module_directive_entry(recovered, doc),
                    hard_line_after: false,
                });
            }
        }
    }

    push_module_directive_segment(&mut sections, &mut segment, doc);
    join_module_directive_sections(sections, doc)
}

fn format_recovered_module_directive_entry<'source>(
    entry: RecoveredSeparatedListEntry<'source, ModuleDirective<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match entry {
        RecoveredSeparatedListEntry::Entry(directive) => {
            FormattedModuleDirective::from_directive(&directive, doc).into_doc(doc)
        }
        RecoveredSeparatedListEntry::Token(token) => {
            format_token_sequence(doc, std::iter::once(token), LeadingTrivia::Preserve)
        }
        RecoveredSeparatedListEntry::Error(error) => {
            format_token_sequence(doc, error.token_iter(), LeadingTrivia::Preserve)
        }
        RecoveredSeparatedListEntry::Node(node) => {
            format_token_sequence(doc, node.token_iter(), LeadingTrivia::Preserve)
        }
    }
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

fn module_directive_entry_token_range(
    entry: &RecoveredSeparatedListEntry<'_, ModuleDirective<'_>>,
    module_start: usize,
) -> Option<Range<usize>> {
    match entry {
        RecoveredSeparatedListEntry::Entry(directive) => {
            module_directive_token_range(directive, module_start)
        }
        RecoveredSeparatedListEntry::Token(token) => {
            Some(relative_token_range_between(token, token, module_start))
        }
        RecoveredSeparatedListEntry::Error(error) => Some(relative_token_range_between(
            &error.first_token()?,
            &error.last_token()?,
            module_start,
        )),
        RecoveredSeparatedListEntry::Node(node) => Some(relative_token_range_between(
            &node.first_token()?,
            &node.last_token()?,
            module_start,
        )),
    }
}

fn format_module_directive_run<'source>(
    directives: Vec<FormattedModuleDirective<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut directives = directives;
    directives.sort_by(|lhs, rhs| {
        lhs.kind_order
            .cmp(&rhs.kind_order)
            .then_with(|| lhs.primary_name.cmp(&rhs.primary_name))
    });

    let mut directives = directives.into_iter().peekable();
    doc.concat_list(|docs| {
        while let Some(directive) = directives.next() {
            if !docs.is_empty() {
                let empty_line = docs.empty_line();
                docs.push(empty_line);
            }
            let kind = directive.kind_order;
            let group = docs.concat_list(|group| {
                let directive = directive.into_doc(group);
                group.push(directive);
                while directives
                    .peek()
                    .is_some_and(|directive| directive.kind_order == kind)
                {
                    let hard_line = group.hard_line();
                    group.push(hard_line);
                    let directive = directives.next().expect("peeked module directive exists");
                    let directive = directive.into_doc(group);
                    group.push(directive);
                }
            });
            docs.push(group);
        }
    })
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
    primary_name: NameSortKey<'source>,
    doc: Doc<'source>,
}

impl<'source> FormattedModuleDirective<'source> {
    fn from_directive(directive: &ModuleDirective<'source>, doc: &mut DocBuilder<'source>) -> Self {
        let Some(role) = directive.directive_role() else {
            return Self {
                first_token: None,
                last_token: None,
                kind_order: ModuleDirectiveKindOrder::Requires,
                primary_name: NameSortKey::recovered(),
                doc: format_token_sequence(doc, directive.token_iter(), LeadingTrivia::Preserve),
            };
        };
        let primary_name = module_directive_primary_name(&role);
        let kind_order = module_directive_kind_order(&role);
        let directive_doc = format_module_directive_doc(directive, &role, doc);

        Self {
            first_token: directive.first_token(),
            last_token: directive.last_token(),
            kind_order,
            primary_name,
            doc: directive_doc,
        }
    }

    fn has_leading_comments(&self) -> bool {
        self.first_token
            .as_ref()
            .is_some_and(|token| !token.leading_comments().is_empty())
    }

    fn into_doc(self, builder: &mut DocBuilder<'source>) -> Doc<'source> {
        let doc = doc_concat!(
            builder,
            [
                self.doc,
                self.last_token.map_or_else(Doc::nil, |token| {
                    format_inline_trailing_comment_list(builder, token.trailing_comments())
                },),
            ]
        );
        if self
            .first_token
            .as_ref()
            .is_none_or(|token| token.leading_comments().is_empty())
        {
            doc
        } else {
            let leading_comments = builder.concat_list(|leading_comments| {
                for comment in self
                    .first_token
                    .into_iter()
                    .flat_map(|token| token.leading_comments())
                {
                    if !leading_comments.is_empty() {
                        let hard_line = leading_comments.hard_line();
                        leading_comments.push(hard_line);
                    }
                    let comment = format_comment(leading_comments, &comment);
                    leading_comments.push(comment);
                }
            });
            doc_concat!(builder, [leading_comments, builder.hard_line(), doc,])
        }
    }
}

fn format_module_directive_doc<'source>(
    directive: &ModuleDirective<'source>,
    role: &ModuleDirectiveRole<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    match (directive, role) {
        (
            ModuleDirective::RequiresDirective(requires),
            ModuleDirectiveRole::Requires {
                module,
                is_static,
                is_transitive,
            },
        ) => doc.concat_list(|parts| {
            let head = format_directive_head(requires.requires_token().as_ref(), parts);
            parts.push(head);
            if *is_static {
                for token in requires.static_tokens() {
                    let static_token = format_directive_middle_token(Some(&token), parts);
                    parts.push(static_token);
                }
            }
            if *is_transitive {
                let transitive =
                    format_directive_middle_token(requires.transitive_token().as_ref(), parts);
                parts.push(transitive);
            }
            let module = format_name(module, parts);
            parts.push(module);
            let semicolon = format_directive_semicolon(requires.semicolon().as_ref(), parts);
            parts.push(semicolon);
        }),
        (
            ModuleDirective::ExportsDirective(exports),
            ModuleDirectiveRole::Exports { package, .. },
        ) => format_module_name_list_directive(
            exports.exports_token().as_ref(),
            package,
            exports.to_token().as_ref(),
            exports.target_entries_with_recovered(),
            exports.semicolon().as_ref(),
            doc,
        ),
        (ModuleDirective::OpensDirective(opens), ModuleDirectiveRole::Opens { package, .. }) => {
            format_module_name_list_directive(
                opens.opens_token().as_ref(),
                package,
                opens.to_token().as_ref(),
                opens.target_entries_with_recovered(),
                opens.semicolon().as_ref(),
                doc,
            )
        }
        (ModuleDirective::UsesDirective(uses), ModuleDirectiveRole::Uses { service }) => {
            doc_concat!(
                doc,
                [
                    format_directive_head(uses.uses_token().as_ref(), doc),
                    format_name(service, doc),
                    format_directive_semicolon(uses.semicolon().as_ref(), doc),
                ]
            )
        }
        (
            ModuleDirective::ProvidesDirective(provides),
            ModuleDirectiveRole::Provides { service, .. },
        ) => format_module_name_list_directive(
            provides.provides_token().as_ref(),
            service,
            provides.with_token().as_ref(),
            provides.implementation_entries_with_recovered(),
            provides.semicolon().as_ref(),
            doc,
        ),
        _ => format_token_sequence(doc, directive.token_iter(), LeadingTrivia::Preserve),
    }
}

fn format_directive_head<'source>(
    token: Option<&jolt_java_syntax::JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    token.map_or_else(Doc::nil, |token| {
        doc_concat!(
            doc,
            [
                format_token_after_relocated_leading_comments(doc, token, TrailingTrivia::Preserve),
                doc.space(),
            ]
        )
    })
}

fn format_directive_middle_token<'source>(
    token: Option<&jolt_java_syntax::JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    token.map_or_else(Doc::nil, |token| {
        doc_concat!(doc, [format_token_with_comments(doc, token), doc.space()])
    })
}

fn format_directive_semicolon<'source>(
    token: Option<&jolt_java_syntax::JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    token.map_or_else(Doc::nil, |token| {
        format_token_before_relocated_trailing_comments(doc, token, LeadingTrivia::Preserve)
    })
}

fn format_module_name_list_directive<'source>(
    keyword: Option<&jolt_java_syntax::JavaSyntaxToken<'source>>,
    subject: &NameSyntax<'source>,
    connective: Option<&jolt_java_syntax::JavaSyntaxToken<'source>>,
    entries: impl IntoIterator<
        Item = RecoveredSeparatedListEntry<'source, ModuleNameListEntry<'source>>,
    >,
    semicolon: Option<&jolt_java_syntax::JavaSyntaxToken<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let Some(entries) = format_module_name_list(entries, doc) else {
        return doc_concat!(
            doc,
            [
                format_directive_head(keyword, doc),
                format_name(subject, doc),
                connective.map_or_else(Doc::nil, |connective| doc_concat!(
                    doc,
                    [doc.space(), format_token_with_comments(doc, connective)]
                ),),
                format_directive_semicolon(semicolon, doc),
            ]
        );
    };

    doc_concat!(
        doc,
        [
            format_directive_head(keyword, doc),
            format_name(subject, doc),
            doc.space(),
            connective.map_or_else(Doc::nil, |connective| format_token_with_comments(
                doc, connective
            ),),
            entries,
            format_directive_semicolon(semicolon, doc),
        ]
    )
}

fn format_module_name_list<'source>(
    entries: impl IntoIterator<
        Item = RecoveredSeparatedListEntry<'source, ModuleNameListEntry<'source>>,
    >,
    doc: &mut DocBuilder<'source>,
) -> Option<Doc<'source>> {
    let mut should_break = false;
    let mut has_recovered = false;
    let entries = entries.into_iter();
    let (lower, _) = entries.size_hint();
    let mut items = Vec::with_capacity(lower);

    for entry in entries {
        match entry {
            RecoveredSeparatedListEntry::Entry(entry) => {
                should_break |= name_has_leading_comments(&entry.name)
                    || entry.comma.as_ref().is_some_and(token_has_comments);
                items.push(FormattedModuleNamePart::Entry(FormattedModuleNameEntry {
                    leading_comments: format_construct_leading_comments(
                        doc,
                        entry.name.first_token().as_ref(),
                    ),
                    name: format_name(&entry.name, doc),
                    comma: entry.comma,
                }));
            }
            RecoveredSeparatedListEntry::Token(token) => {
                has_recovered = true;
                items.push(FormattedModuleNamePart::Recovered(
                    format_token_with_comments(doc, &token),
                ));
            }
            RecoveredSeparatedListEntry::Error(error) => {
                has_recovered = true;
                items.push(FormattedModuleNamePart::Recovered(format_token_sequence(
                    doc,
                    error.token_iter(),
                    LeadingTrivia::Preserve,
                )));
            }
            RecoveredSeparatedListEntry::Node(node) => {
                has_recovered = true;
                items.push(FormattedModuleNamePart::Recovered(format_token_sequence(
                    doc,
                    node.token_iter(),
                    LeadingTrivia::Preserve,
                )));
            }
        }
    }

    if items.is_empty() {
        return None;
    }

    Some(if should_break || has_recovered {
        doc_indent!(
            doc,
            doc_concat!(
                doc,
                [
                    doc.hard_line(),
                    format_module_name_entries_broken(items, doc),
                ]
            )
        )
    } else {
        doc_concat!(
            doc,
            [doc.space(), format_module_name_entries_inline(items, doc)]
        )
    })
}

enum FormattedModuleNamePart<'source> {
    Entry(FormattedModuleNameEntry<'source>),
    Recovered(Doc<'source>),
}

struct FormattedModuleNameEntry<'source> {
    leading_comments: Doc<'source>,
    name: Doc<'source>,
    comma: Option<jolt_java_syntax::JavaSyntaxToken<'source>>,
}

fn format_module_name_entries_inline<'source>(
    entries: Vec<FormattedModuleNamePart<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    doc.concat_list(|docs| {
        for entry in entries {
            match entry {
                FormattedModuleNamePart::Entry(entry) => {
                    docs.push(entry.name);
                    if let Some(comma) = entry.comma {
                        let separator = docs.space();
                        let comma = format_separator_with_comments(docs, &comma, separator);
                        docs.push(comma);
                    }
                }
                FormattedModuleNamePart::Recovered(recovered) => docs.push(recovered),
            }
        }
    })
}

fn format_module_name_entries_broken<'source>(
    entries: Vec<FormattedModuleNamePart<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let entries_len = entries.len();
    doc.concat_list(|docs| {
        for (index, entry) in entries.into_iter().enumerate() {
            match entry {
                FormattedModuleNamePart::Entry(entry) => {
                    let entry_doc = doc_concat!(docs, [entry.leading_comments, entry.name]);
                    docs.push(entry_doc);
                    if let Some(comma) = entry.comma {
                        let separator = docs.line();
                        let comma = format_separator_with_comments(docs, &comma, separator);
                        docs.push(comma);
                    } else if index + 1 < entries_len {
                        let hard_line = docs.hard_line();
                        docs.push(hard_line);
                    }
                }
                FormattedModuleNamePart::Recovered(recovered) => {
                    docs.push(recovered);
                    if index + 1 < entries_len {
                        let hard_line = docs.hard_line();
                        docs.push(hard_line);
                    }
                }
            }
        }
    })
}

fn name_has_leading_comments(name: &NameSyntax<'_>) -> bool {
    name.first_token()
        .is_some_and(|token| !token.leading_comments().is_empty())
}

fn module_directive_primary_name<'source>(
    role: &ModuleDirectiveRole<'source>,
) -> NameSortKey<'source> {
    match role {
        ModuleDirectiveRole::Requires { module, .. } => NameSortKey::new(module, false),
        ModuleDirectiveRole::Exports { package, .. }
        | ModuleDirectiveRole::Opens { package, .. } => NameSortKey::new(package, false),
        ModuleDirectiveRole::Uses { service } | ModuleDirectiveRole::Provides { service, .. } => {
            NameSortKey::new(service, false)
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
