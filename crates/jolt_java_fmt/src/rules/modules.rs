use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_java_syntax::{
    ExportsDirective, JavaMalformedSyntax, JavaMissingSyntax, JavaSyntaxField, JavaSyntaxToken,
    JavaSyntaxView, ModuleDeclaration, ModuleDirective, ModuleImplementationClause, ModuleNameList,
    ModuleTargetClause, NameSyntax, OpensDirective, ProvidesDirective, ReorderClaim,
    RequiresDirective, UsesDirective,
};

use crate::helpers::blocks::join_empty_lines;
use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_comment, format_inline_trailing_comment_list,
    format_separator_with_comments, format_token_after_relocated_leading_comments,
    format_token_before_relocated_trailing_comments, format_token_with_comments,
    token_has_comments,
};
use crate::helpers::formatter_ignore::{
    FormatterIgnoreItemRange, FormatterIgnoreRun, FormatterIgnoreSplice,
    for_each_formatter_ignore_splice, formatter_ignore_content_range, formatter_ignore_run_doc,
};
use crate::helpers::recovery::{
    JavaFormatListPart, format_malformed, format_optional_field, format_required_field,
    resolve_list_part,
};
use crate::rules::annotations::format_required_annotation_lines;
use crate::rules::names::{NameSortKey, format_name};

pub(crate) fn format_module_declaration<'source>(
    module: &ModuleDeclaration<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    {
        let (annotations, annotations_visible) =
            format_required_annotation_lines(module.annotations(), doc);
        let open = format_optional_field(module.open_keyword(), doc, |token, doc| {
            doc_concat!(doc, [format_token_with_comments(doc, &token), doc.space()])
        });
        let keyword = format_required_field(module.module_keyword(), doc, |token, doc| {
            doc_concat!(doc, [format_token_with_comments(doc, &token), doc.space()])
        });
        let name = format_required_field(module.name(), doc, |name, doc| format_name(&name, doc));
        let open_brace = format_required_field(module.open_brace(), doc, |token, doc| {
            doc_concat!(doc, [doc.space(), format_token_with_comments(doc, &token)])
        });
        let (directives, directives_visible) = match module.directives() {
            Ok(JavaSyntaxField::Present(list)) => format_module_directives(module, &list, doc),
            Ok(JavaSyntaxField::Malformed(malformed)) => {
                let visible = malformed.first_token().is_some();
                (format_malformed(&malformed, doc), visible)
            }
            Ok(JavaSyntaxField::Missing(missing)) => (
                crate::helpers::recovery::format_missing(&missing, doc),
                false,
            ),
            Err(error) => {
                doc.block_on_invariant(error.to_string());
                (Doc::nil(), false)
            }
        };
        let close_brace = format_required_field(module.close_brace(), doc, |token, doc| {
            format_token_with_comments(doc, &token)
        });
        let head = doc_concat!(doc, [open, keyword, name, open_brace]);
        let body = if directives_visible {
            doc_indent!(doc, doc_concat!(doc, [doc.hard_line(), directives]))
        } else {
            directives
        };
        let declaration = doc_concat!(doc, [head, body, doc.hard_line(), close_brace]);
        if annotations_visible {
            doc_concat!(doc, [annotations, doc.hard_line(), declaration])
        } else {
            doc_concat!(doc, [annotations, declaration])
        }
    }
}

fn format_module_directives<'source>(
    module: &ModuleDeclaration<'source>,
    list: &jolt_java_syntax::ModuleDirectiveList<'source>,
    doc: &mut DocBuilder<'source>,
) -> (Doc<'source>, bool) {
    {
        let parts = list.parts();
        let mut entries = Vec::with_capacity(parts.size_hint().0);
        for part in parts {
            match part {
                Ok(jolt_java_syntax::JavaSyntaxListPart::Item(item)) => {
                    entries.push(DirectiveEntry::Node(item));
                }
                Ok(jolt_java_syntax::JavaSyntaxListPart::Separator(token)) => {
                    entries.push(DirectiveEntry::Token(token));
                }
                Ok(jolt_java_syntax::JavaSyntaxListPart::Malformed(malformed)) => {
                    entries.push(DirectiveEntry::Malformed(malformed));
                }
                Ok(jolt_java_syntax::JavaSyntaxListPart::Missing(missing)) => {
                    entries.push(DirectiveEntry::Missing(missing));
                }
                Err(error) => doc.block_on_invariant(error.to_string()),
            }
        }
        let visible = entries.iter().any(DirectiveEntry::is_visible);
        let open = match module.open_brace() {
            Ok(JavaSyntaxField::Present(token)) => Some(token),
            _ => None,
        };
        let close = match module.close_brace() {
            Ok(JavaSyntaxField::Present(token)) => Some(token),
            _ => None,
        };
        let container = formatter_ignore_content_range(list.text_range(), open, close);
        let runs =
            doc.formatter_ignore_runs(container, entries.iter().map(DirectiveEntry::ignore_range));
        (
            format_directive_entries_with_ignored(entries, &runs, doc),
            visible || !runs.is_empty(),
        )
    }
}

enum DirectiveEntry<'source> {
    Node(ModuleDirective<'source>),
    Token(JavaSyntaxToken<'source>),
    Malformed(JavaMalformedSyntax<'source>),
    Missing(JavaMissingSyntax<'source>),
}

impl DirectiveEntry<'_> {
    fn is_visible(&self) -> bool {
        match self {
            Self::Node(node) => node.first_token().is_some(),
            Self::Token(_) => true,
            Self::Malformed(malformed) => malformed.first_token().is_some(),
            Self::Missing(_) => false,
        }
    }

    fn ignore_range(&self) -> Option<FormatterIgnoreItemRange> {
        match self {
            Self::Node(node) => Some(FormatterIgnoreItemRange::between(
                &node.first_token()?,
                &node.last_token()?,
            )),
            Self::Token(token) => Some(FormatterIgnoreItemRange::between(token, token)),
            Self::Malformed(malformed) => {
                let syntax = malformed.syntax_node()?;
                Some(FormatterIgnoreItemRange::between(
                    &syntax.first_token()?,
                    &syntax.last_token()?,
                ))
            }
            Self::Missing(_) => None,
        }
    }
}

fn format_directive_entries_with_ignored<'source>(
    entries: Vec<DirectiveEntry<'source>>,
    runs: &[FormatterIgnoreRun<'source>],
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let entry_count = entries.len();
    let mut sections = Vec::with_capacity(entry_count.saturating_add(runs.len()));
    let mut retained = Vec::with_capacity(entry_count);
    let mut entries = entries.into_iter().map(Some).collect::<Vec<_>>();
    for_each_formatter_ignore_splice(entries.len(), runs, |event| match event {
        FormatterIgnoreSplice::Ignore(run) => {
            if !retained.is_empty() {
                let visible = retained.iter().any(DirectiveEntry::is_visible);
                sections.push((
                    format_directive_entries(std::mem::take(&mut retained), doc),
                    false,
                    visible,
                ));
            }
            sections.push((formatter_ignore_run_doc(run, doc), true, true));
        }
        FormatterIgnoreSplice::Item { index, .. } => {
            if let Some(entry) = entries[index].take() {
                retained.push(entry);
            }
        }
    });
    if !retained.is_empty() {
        let visible = retained.iter().any(DirectiveEntry::is_visible);
        sections.push((format_directive_entries(retained, doc), false, visible));
    }
    doc.concat_list(|joined| {
        let mut has_visible_section = false;
        let mut previous_was_ignored = false;
        for (section, ignored, visible) in sections {
            if visible && has_visible_section {
                let separator = if previous_was_ignored {
                    joined.hard_line()
                } else {
                    joined.empty_line()
                };
                joined.push(separator);
            }
            joined.push(section);
            has_visible_section |= visible;
            if visible {
                previous_was_ignored = ignored;
            }
        }
    })
}

fn format_directive_entries<'source>(
    entries: Vec<DirectiveEntry<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut sections = Vec::new();
    let mut run = Vec::new();
    for entry in entries {
        match entry {
            DirectiveEntry::Node(node) => {
                let Some(formatted) = FormattedDirective::new(node) else {
                    flush_directives(&mut run, &mut sections, doc);
                    sections.push(format_directive_node(&node, doc));
                    continue;
                };
                if node
                    .first_token()
                    .is_some_and(|token| !token.leading_comments().is_empty())
                {
                    flush_directives(&mut run, &mut sections, doc);
                    sections.push(format_directive_node(&node, doc));
                } else {
                    run.push(formatted);
                }
            }
            DirectiveEntry::Token(token) => {
                flush_directives(&mut run, &mut sections, doc);
                sections.push(format_token_with_comments(doc, &token));
            }
            DirectiveEntry::Malformed(malformed) => {
                flush_directives(&mut run, &mut sections, doc);
                sections.push(format_malformed(&malformed, doc));
            }
            DirectiveEntry::Missing(missing) => {
                flush_directives(&mut run, &mut sections, doc);
                sections.push(crate::helpers::recovery::format_missing(&missing, doc));
            }
        }
    }
    flush_directives(&mut run, &mut sections, doc);
    join_empty_lines(doc, sections)
}

fn flush_directives<'source>(
    directives: &mut Vec<FormattedDirective<'source>>,
    sections: &mut Vec<Doc<'source>>,
    doc: &mut DocBuilder<'source>,
) {
    if !directives.is_empty() {
        sort_directives_if_needed(directives);
        sections.push(format_sorted_directives(directives.drain(..), doc));
    }
}

fn sort_directives_if_needed(directives: &mut [FormattedDirective<'_>]) {
    let compare = |left: &FormattedDirective<'_>, right: &FormattedDirective<'_>| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.key.cmp(&right.key))
    };
    if !directives
        .windows(2)
        .all(|pair| compare(&pair[0], &pair[1]).is_le())
    {
        directives.sort_by(compare);
    }
}

fn format_sorted_directives<'source>(
    directives: impl IntoIterator<Item = FormattedDirective<'source>>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let mut previous = None;
    doc.concat_list(|docs| {
        for directive in directives {
            if previous.is_some() {
                let separator = if previous == Some(directive.kind) {
                    docs.hard_line()
                } else {
                    docs.empty_line()
                };
                docs.push(separator);
            }
            previous = Some(directive.kind);
            let formatted = format_directive_node(&directive.node, docs);
            let directive = docs.reordered_source(formatted, directive.reorder);
            docs.push(directive);
        }
    })
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum DirectiveKind {
    Requires,
    Exports,
    Opens,
    Uses,
    Provides,
}

struct FormattedDirective<'source> {
    node: ModuleDirective<'source>,
    reorder: ReorderClaim<'source>,
    kind: DirectiveKind,
    key: NameSortKey<'source>,
}

impl<'source> FormattedDirective<'source> {
    fn new(node: ModuleDirective<'source>) -> Option<Self> {
        let reorder = node.canonical_reorder_claim()?;
        let (kind, key) = match &node {
            ModuleDirective::RequiresDirective(value) if requires_is_sortable(value) => {
                (DirectiveKind::Requires, name_key(value.module())?)
            }
            ModuleDirective::ExportsDirective(value) if exports_is_sortable(value) => {
                (DirectiveKind::Exports, name_key(value.package())?)
            }
            ModuleDirective::OpensDirective(value) if opens_is_sortable(value) => {
                (DirectiveKind::Opens, name_key(value.package())?)
            }
            ModuleDirective::UsesDirective(value) if uses_is_sortable(value) => {
                (DirectiveKind::Uses, name_key(value.service())?)
            }
            ModuleDirective::ProvidesDirective(value) if provides_is_sortable(value) => {
                (DirectiveKind::Provides, name_key(value.service())?)
            }
            _ => return None,
        };
        Some(Self {
            node,
            reorder,
            kind,
            key,
        })
    }
}

#[allow(clippy::needless_pass_by_value)]
fn name_key<'source>(
    field: Result<
        jolt_java_syntax::JavaSyntaxField<'source, NameSyntax<'source>>,
        jolt_java_syntax::JavaSyntaxInvariantError,
    >,
) -> Option<NameSortKey<'source>> {
    match field.ok()? {
        jolt_java_syntax::JavaSyntaxField::Present(name) if name.is_recovery_free() => {
            NameSortKey::new(&name, false)
        }
        _ => None,
    }
}

#[allow(clippy::needless_pass_by_value)]
fn required<T>(
    field: Result<
        jolt_java_syntax::JavaSyntaxField<'_, T>,
        jolt_java_syntax::JavaSyntaxInvariantError,
    >,
) -> bool {
    matches!(field, Ok(jolt_java_syntax::JavaSyntaxField::Present(_)))
}

fn requires_is_sortable(value: &RequiresDirective<'_>) -> bool {
    required(value.requires_keyword())
        && matches!(value.modifiers(), Ok(jolt_java_syntax::JavaSyntaxField::Present(ref list)) if list.is_recovery_free())
        && name_key(value.module()).is_some()
        && required(value.semicolon())
}

fn exports_is_sortable(value: &ExportsDirective<'_>) -> bool {
    required(value.exports_keyword())
        && name_key(value.package()).is_some()
        && optional_target_is_sortable(value.targets())
        && required(value.semicolon())
}

fn opens_is_sortable(value: &OpensDirective<'_>) -> bool {
    required(value.opens_keyword())
        && name_key(value.package()).is_some()
        && optional_target_is_sortable(value.targets())
        && required(value.semicolon())
}

fn uses_is_sortable(value: &UsesDirective<'_>) -> bool {
    required(value.uses_keyword())
        && name_key(value.service()).is_some()
        && required(value.semicolon())
}

fn provides_is_sortable(value: &ProvidesDirective<'_>) -> bool {
    required(value.provides_keyword())
        && name_key(value.service()).is_some()
        && matches!(value.implementation(), Ok(jolt_java_syntax::JavaSyntaxField::Present(ref clause)) if clause.is_recovery_free())
        && required(value.semicolon())
}

#[allow(clippy::needless_pass_by_value)]
fn optional_target_is_sortable(
    field: Result<
        jolt_java_syntax::JavaSyntaxField<'_, ModuleTargetClause<'_>>,
        jolt_java_syntax::JavaSyntaxInvariantError,
    >,
) -> bool {
    match field {
        Ok(jolt_java_syntax::JavaSyntaxField::Present(value)) => value.is_recovery_free(),
        Ok(jolt_java_syntax::JavaSyntaxField::Missing(_)) => true,
        Ok(jolt_java_syntax::JavaSyntaxField::Malformed(_)) | Err(_) => false,
    }
}

fn format_directive_node<'source>(
    node: &ModuleDirective<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    {
        let first = node.first_token();
        let last = node.last_token();
        let body = match node {
            ModuleDirective::RequiresDirective(value) => format_requires(value, doc),
            ModuleDirective::ExportsDirective(value) => format_exports(value, doc),
            ModuleDirective::OpensDirective(value) => format_opens(value, doc),
            ModuleDirective::UsesDirective(value) => format_uses(value, doc),
            ModuleDirective::ProvidesDirective(value) => format_provides(value, doc),
            ModuleDirective::BogusModuleDirective(bogus) => return format_malformed(bogus, doc),
        };
        let trailing = last.map_or_else(Doc::nil, |token| {
            format_inline_trailing_comment_list(doc, token.trailing_comments())
        });
        if first
            .as_ref()
            .is_some_and(|token| !token.leading_comments().is_empty())
        {
            let leading = doc.concat_list(|comments| {
                for comment in first.into_iter().flat_map(|token| token.leading_comments()) {
                    if !comments.is_empty() {
                        let line = comments.hard_line();
                        comments.push(line);
                    }
                    let comment = format_comment(comments, &comment);
                    comments.push(comment);
                }
            });
            doc_concat!(doc, [leading, doc.hard_line(), body, trailing])
        } else {
            doc_concat!(doc, [body, trailing])
        }
    }
}

fn keyword<'source>(
    field: Result<
        jolt_java_syntax::JavaSyntaxField<'source, jolt_java_syntax::JavaSyntaxToken<'source>>,
        jolt_java_syntax::JavaSyntaxInvariantError,
    >,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_required_field(field, doc, |token, doc| {
        doc_concat!(
            doc,
            [
                format_token_after_relocated_leading_comments(
                    doc,
                    &token,
                    TrailingTrivia::Preserve
                ),
                doc.space(),
            ]
        )
    })
}

fn semicolon<'source>(
    field: Result<
        jolt_java_syntax::JavaSyntaxField<'source, jolt_java_syntax::JavaSyntaxToken<'source>>,
        jolt_java_syntax::JavaSyntaxInvariantError,
    >,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_required_field(field, doc, |token, doc| {
        format_token_before_relocated_trailing_comments(doc, &token, LeadingTrivia::Preserve)
    })
}

fn format_requires<'source>(
    value: &RequiresDirective<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    {
        let has_module = matches!(
            value.module(),
            Ok(jolt_java_syntax::JavaSyntaxField::Present(ref module))
                if module.first_token().is_some()
        );
        let modifiers = format_required_field(value.modifiers(), doc, |list, doc| {
            let mut modifiers = list
                .parts()
                .map(|part| resolve_list_part(part, doc))
                .collect::<Vec<_>>();
            let authorization = list.canonical_reorder_claim();
            if authorization.is_some() {
                sort_requires_modifier_runs(&mut modifiers);
            }
            let formatted = doc.concat_list(|parts| {
                let mut previous_was_structured = false;
                for part in modifiers {
                    let (part, structured) = match part {
                        JavaFormatListPart::Item(item) => {
                            let Some(token) = item.token() else {
                                parts.block_on_invariant("requires modifier was not a token");
                                continue;
                            };
                            (format_token_with_comments(parts, &token), true)
                        }
                        JavaFormatListPart::Separator(token) => {
                            (format_token_with_comments(parts, &token), false)
                        }
                        JavaFormatListPart::Malformed(raw) => (raw, false),
                    };
                    if previous_was_structured && structured {
                        let space = parts.space();
                        parts.push(space);
                    }
                    parts.push(part);
                    previous_was_structured = structured;
                }
                if previous_was_structured && has_module {
                    let space = parts.space();
                    parts.push(space);
                }
            });
            authorization.map_or(formatted, |claim| doc.reordered_source(formatted, claim))
        });
        let module =
            format_required_field(value.module(), doc, |name, doc| format_name(&name, doc));
        doc_concat!(
            doc,
            [
                keyword(value.requires_keyword(), doc),
                modifiers,
                module,
                semicolon(value.semicolon(), doc)
            ]
        )
    }
}

fn sort_requires_modifier_runs<'source>(
    parts: &mut [JavaFormatListPart<'source, jolt_java_syntax::RequiresModifier<'source>>],
) {
    let sortable = parts.iter().all(|part| match part {
        JavaFormatListPart::Item(modifier) => modifier
            .token()
            .is_some_and(|token| !token_has_comments(&token)),
        JavaFormatListPart::Separator(_) | JavaFormatListPart::Malformed(_) => false,
    });
    if sortable {
        parts.sort_by_key(requires_modifier_order);
    }
}

fn requires_modifier_order(
    part: &JavaFormatListPart<'_, jolt_java_syntax::RequiresModifier<'_>>,
) -> u8 {
    match part {
        JavaFormatListPart::Item(modifier)
            if modifier.token().is_some_and(|token| {
                token.kind() == jolt_java_syntax::JavaSyntaxKind::StaticKw
            }) =>
        {
            0
        }
        JavaFormatListPart::Item(_) => 1,
        JavaFormatListPart::Separator(_) | JavaFormatListPart::Malformed(_) => 2,
    }
}

fn format_exports<'source>(
    value: &ExportsDirective<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_named_list_directive(
        value.exports_keyword(),
        value.package(),
        value.targets(),
        value.semicolon(),
        doc,
    )
}

fn format_opens<'source>(
    value: &OpensDirective<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    format_named_list_directive(
        value.opens_keyword(),
        value.package(),
        value.targets(),
        value.semicolon(),
        doc,
    )
}

fn format_uses<'source>(
    value: &UsesDirective<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    {
        doc_concat!(
            doc,
            [
                keyword(value.uses_keyword(), doc),
                format_required_field(value.service(), doc, |name, doc| format_name(&name, doc)),
                semicolon(value.semicolon(), doc),
            ]
        )
    }
}

fn format_provides<'source>(
    value: &ProvidesDirective<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    {
        let service =
            format_required_field(value.service(), doc, |name, doc| format_name(&name, doc));
        let implementation = format_required_field(value.implementation(), doc, |clause, doc| {
            format_module_implementation_clause(&clause, doc)
        });
        doc_concat!(
            doc,
            [
                keyword(value.provides_keyword(), doc),
                service,
                implementation,
                semicolon(value.semicolon(), doc),
            ]
        )
    }
}

fn format_named_list_directive<'source>(
    keyword_field: Result<
        jolt_java_syntax::JavaSyntaxField<'source, jolt_java_syntax::JavaSyntaxToken<'source>>,
        jolt_java_syntax::JavaSyntaxInvariantError,
    >,
    subject_field: Result<
        jolt_java_syntax::JavaSyntaxField<'source, NameSyntax<'source>>,
        jolt_java_syntax::JavaSyntaxInvariantError,
    >,
    targets_field: Result<
        jolt_java_syntax::JavaSyntaxField<'source, ModuleTargetClause<'source>>,
        jolt_java_syntax::JavaSyntaxInvariantError,
    >,
    semicolon_field: Result<
        jolt_java_syntax::JavaSyntaxField<'source, jolt_java_syntax::JavaSyntaxToken<'source>>,
        jolt_java_syntax::JavaSyntaxInvariantError,
    >,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    {
        let subject =
            format_required_field(subject_field, doc, |name, doc| format_name(&name, doc));
        let targets = format_optional_field(targets_field, doc, |clause, doc| {
            format_module_target_clause(&clause, doc)
        });
        doc_concat!(
            doc,
            [
                keyword(keyword_field, doc),
                subject,
                targets,
                semicolon(semicolon_field, doc)
            ]
        )
    }
}

fn format_module_target_clause<'source>(
    clause: &ModuleTargetClause<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let to = format_required_field(clause.to_keyword(), doc, |token, doc| {
        doc_concat!(doc, [doc.space(), format_token_with_comments(doc, &token)])
    });
    let targets = format_required_field(clause.targets(), doc, |list, doc| {
        format_name_list(&list, doc)
    });
    doc_concat!(doc, [to, targets])
}

fn format_module_implementation_clause<'source>(
    clause: &ModuleImplementationClause<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    let with = format_required_field(clause.with_keyword(), doc, |token, doc| {
        doc_concat!(doc, [doc.space(), format_token_with_comments(doc, &token)])
    });
    let implementations = format_required_field(clause.implementations(), doc, |list, doc| {
        format_name_list(&list, doc)
    });
    doc_concat!(doc, [with, implementations])
}

fn format_name_list<'source>(
    list: &ModuleNameList<'source>,
    doc: &mut DocBuilder<'source>,
) -> Doc<'source> {
    {
        let syntax_parts = list.parts();
        let (lower, _) = syntax_parts.size_hint();
        let mut parts = Vec::with_capacity(lower);
        for part in syntax_parts {
            match part {
                Ok(jolt_java_syntax::JavaSyntaxListPart::Item(name)) => {
                    parts.push(NameListPart::Name(format_name(&name, doc)));
                }
                Ok(jolt_java_syntax::JavaSyntaxListPart::Separator(comma)) => {
                    parts.push(NameListPart::Comma(comma));
                }
                Ok(jolt_java_syntax::JavaSyntaxListPart::Malformed(malformed)) => {
                    parts.push(NameListPart::Malformed(malformed));
                }
                Ok(jolt_java_syntax::JavaSyntaxListPart::Missing(missing)) => {
                    parts.push(NameListPart::Missing(missing));
                }
                Err(error) => doc.block_on_invariant(error.to_string()),
            }
        }
        if parts.is_empty() {
            return Doc::nil();
        }
        doc_indent!(
            doc,
            doc_group!(
                doc,
                doc.concat_list(|docs| {
                    let line = docs.line();
                    docs.push(line);
                    for part in parts {
                        match part {
                            NameListPart::Name(name) => docs.push(name),
                            NameListPart::Comma(comma) => {
                                let line = docs.line();
                                let comma = format_separator_with_comments(docs, &comma, line);
                                docs.push(comma);
                            }
                            NameListPart::Malformed(malformed) => {
                                let malformed = format_malformed(&malformed, docs);
                                docs.push(malformed);
                            }
                            NameListPart::Missing(missing) => {
                                let missing =
                                    crate::helpers::recovery::format_missing(&missing, docs);
                                docs.push(missing);
                            }
                        }
                    }
                })
            )
        )
    }
}

enum NameListPart<'source> {
    Name(Doc<'source>),
    Comma(jolt_java_syntax::JavaSyntaxToken<'source>),
    Malformed(JavaMalformedSyntax<'source>),
    Missing(JavaMissingSyntax<'source>),
}
