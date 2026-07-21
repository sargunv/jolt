use std::ops::Range;

use jolt_fmt_ir::{Doc, DocBuilder};
use jolt_kotlin_syntax::{
    ImportAlias, ImportDirective, ImportDirectiveList, ImportOnDemandSuffix, KotlinMalformedSyntax,
    KotlinMissingSyntax, KotlinSyntaxField, KotlinSyntaxListPart, KotlinSyntaxToken,
    KotlinSyntaxView, ReorderClaim,
};

use crate::helpers::comments::{
    LeadingTrivia, TrailingTrivia, format_terminator_list, format_token,
};
use crate::helpers::formatter_ignore::{
    FormatterIgnoreRun, FormatterIgnoreSplice, for_each_formatter_ignore_splice,
    formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs,
    relative_token_range_between,
};
use crate::helpers::recovery::{
    format_malformed, format_missing, format_optional_field, format_required_field,
};
use crate::rules::names::{NameSortKey, format_name, format_qualified_name};

pub(crate) fn format_import_list<'source>(
    doc: &mut DocBuilder<'source>,
    list: &ImportDirectiveList<'source>,
) -> Doc<'source> {
    let entries = list
        .parts()
        .map(|part| match part {
            Ok(KotlinSyntaxListPart::Item(import)) => ImportEntry::Directive(import),
            Ok(KotlinSyntaxListPart::Separator(token)) => ImportEntry::Token(token),
            Ok(KotlinSyntaxListPart::Malformed(malformed)) => ImportEntry::Malformed(malformed),
            Ok(KotlinSyntaxListPart::Missing(missing)) => ImportEntry::Missing(missing),
            Err(error) => {
                doc.block_on_invariant(error.to_string());
                ImportEntry::Invariant
            }
        })
        .collect::<Vec<_>>();
    let base = list.text_range().start().get();
    let ignored = formatter_ignore_ranges(list.source_text(), base, list.token_iter());
    if ignored.is_empty() {
        return format_import_entries(doc, entries);
    }
    let ranges = entries
        .iter()
        .map(|entry| entry.range(base))
        .collect::<Vec<_>>();
    let runs = formatter_ignore_runs(&ignored, &ranges);
    if runs.is_empty() {
        format_import_entries(doc, entries)
    } else {
        format_import_entries_with_ignored(doc, entries, &runs)
    }
}

enum ImportEntry<'source> {
    Directive(ImportDirective<'source>),
    Token(KotlinSyntaxToken<'source>),
    Malformed(KotlinMalformedSyntax<'source>),
    Missing(KotlinMissingSyntax<'source>),
    Invariant,
}

impl ImportEntry<'_> {
    fn range(&self, base: usize) -> Option<Range<usize>> {
        match self {
            Self::Directive(import) => Some(relative_token_range_between(
                &import.first_token()?,
                &import.last_token()?,
                base,
            )),
            Self::Token(token) => Some(relative_token_range_between(token, token, base)),
            Self::Malformed(malformed) => {
                let syntax = malformed.syntax_node()?;
                Some(relative_token_range_between(
                    &syntax.first_token()?,
                    &syntax.last_token()?,
                    base,
                ))
            }
            Self::Missing(_) | Self::Invariant => None,
        }
    }
}

fn format_import_entries_with_ignored<'source>(
    doc: &mut DocBuilder<'source>,
    entries: Vec<ImportEntry<'source>>,
    runs: &[FormatterIgnoreRun<'source>],
) -> Doc<'source> {
    let mut sections = Vec::new();
    let mut retained = Vec::new();
    let mut entries = entries.into_iter().map(Some).collect::<Vec<_>>();
    for_each_formatter_ignore_splice(entries.len(), runs, |event| match event {
        FormatterIgnoreSplice::Ignore(run) => {
            if !retained.is_empty() {
                sections.push(format_import_entries(doc, std::mem::take(&mut retained)));
            }
            sections.push(formatter_ignore_run_doc(run, doc));
        }
        FormatterIgnoreSplice::Item { index, .. } => {
            if let Some(entry) = entries[index].take() {
                retained.push(entry);
            }
        }
    });
    if !retained.is_empty() {
        sections.push(format_import_entries(doc, retained));
    }
    doc.concat_list(|docs| {
        for section in sections {
            if !docs.is_empty() {
                let line = docs.hard_line();
                docs.push(line);
            }
            docs.push(section);
        }
    })
}

fn format_import_entries<'source>(
    doc: &mut DocBuilder<'source>,
    entries: Vec<ImportEntry<'source>>,
) -> Doc<'source> {
    let mut sections = Vec::new();
    let mut sortable = Vec::new();
    for entry in entries {
        match entry {
            ImportEntry::Directive(import) => {
                if let Some(formatted) = FormattedImport::new(import) {
                    if formatted
                        .import
                        .first_token()
                        .is_some_and(|token| !token.leading_comments().is_empty())
                    {
                        flush_sortable(doc, &mut sortable, &mut sections);
                        sections.push(ImportSection {
                            doc: formatted.into_doc(doc),
                            starts_comment_barrier: true,
                        });
                    } else {
                        sortable.push(formatted);
                    }
                } else {
                    flush_sortable(doc, &mut sortable, &mut sections);
                    sections.push(ImportSection {
                        doc: format_import(doc, &import),
                        starts_comment_barrier: false,
                    });
                }
            }
            ImportEntry::Token(separator) => {
                flush_sortable(doc, &mut sortable, &mut sections);
                sections.push(ImportSection {
                    doc: format_token(
                        doc,
                        &separator,
                        LeadingTrivia::Preserve,
                        TrailingTrivia::Preserve,
                    ),
                    starts_comment_barrier: false,
                });
            }
            ImportEntry::Malformed(malformed) => {
                flush_sortable(doc, &mut sortable, &mut sections);
                sections.push(ImportSection {
                    doc: format_malformed(&malformed, doc),
                    starts_comment_barrier: false,
                });
            }
            ImportEntry::Missing(missing) => {
                flush_sortable(doc, &mut sortable, &mut sections);
                sections.push(ImportSection {
                    doc: format_missing(&missing, doc),
                    starts_comment_barrier: false,
                });
            }
            ImportEntry::Invariant => {}
        }
    }
    flush_sortable(doc, &mut sortable, &mut sections);
    doc.concat_list(|docs| {
        for (index, section) in sections.into_iter().enumerate() {
            if index != 0 {
                let separator = if section.starts_comment_barrier {
                    docs.empty_line()
                } else {
                    docs.hard_line()
                };
                docs.push(separator);
            }
            docs.push(section.doc);
        }
    })
}

struct ImportSection<'source> {
    doc: Doc<'source>,
    starts_comment_barrier: bool,
}

fn flush_sortable<'source>(
    doc: &mut DocBuilder<'source>,
    imports: &mut Vec<FormattedImport<'source>>,
    sections: &mut Vec<ImportSection<'source>>,
) {
    if imports.is_empty() {
        return;
    }
    // Each comment- or recovery-delimited run has `r <= represented tokens`.
    // Stable sorting is O(r log r) time and O(r) scratch with no layout search.
    imports.sort_by(|left, right| left.path.cmp(&right.path));
    sections.extend(
        std::mem::take(imports)
            .into_iter()
            .map(|import| ImportSection {
                doc: import.into_doc(doc),
                starts_comment_barrier: false,
            }),
    );
}

struct FormattedImport<'source> {
    import: ImportDirective<'source>,
    reorder: ReorderClaim<'source>,
    path: NameSortKey<'source>,
}

impl<'source> FormattedImport<'source> {
    fn new(import: ImportDirective<'source>) -> Option<Self> {
        use KotlinSyntaxField::{Missing, Present};

        let reorder = import.canonical_reorder_claim()?;
        if !matches!(import.import_token(), Ok(Present(_)))
            || !matches!(import.on_demand(), Ok(Present(_) | Missing(_)))
            || !matches!(import.alias(), Ok(Present(_) | Missing(_)))
            || !matches!(import.suffix(), Ok(Missing(_)))
            || !matches!(import.terminators(), Ok(Present(_)))
        {
            return None;
        }
        let Present(name) = import.name().ok()? else {
            return None;
        };
        let on_demand = matches!(import.on_demand(), Ok(Present(_)));
        let path = NameSortKey::new(&name, on_demand)?;
        Some(Self {
            import,
            reorder,
            path,
        })
    }

    fn into_doc(self, doc: &mut DocBuilder<'source>) -> Doc<'source> {
        let formatted = format_import(doc, &self.import);
        doc.reordered_source(formatted, self.reorder)
    }
}

fn format_import<'source>(
    doc: &mut DocBuilder<'source>,
    import: &ImportDirective<'source>,
) -> Doc<'source> {
    let keyword = format_required_field(import.import_token(), doc, |token, doc| {
        format_token(
            doc,
            &token,
            LeadingTrivia::Preserve,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    });
    let name = format_required_field(import.name(), doc, |name, doc| {
        let has_token = name.first_token().is_some();
        let name = format_qualified_name(doc, &name);
        if has_token {
            let space = doc.space();
            doc.concat([space, name])
        } else {
            name
        }
    });
    let on_demand = format_optional_field(import.on_demand(), doc, |suffix, doc| {
        format_import_on_demand(doc, &suffix)
    });
    let alias = format_optional_field(import.alias(), doc, |alias, doc| {
        let space = doc.space();
        let alias = format_import_alias(doc, &alias);
        doc.concat([space, alias])
    });
    let suffix = format_optional_field(import.suffix(), doc, |suffix, doc| {
        format_malformed(&suffix, doc)
    });
    let terminators = format_required_field(import.terminators(), doc, |terminators, doc| {
        format_terminator_list(doc, &terminators, true)
    });
    doc.concat([keyword, name, on_demand, alias, suffix, terminators])
}

fn format_import_on_demand<'source>(
    doc: &mut DocBuilder<'source>,
    suffix: &ImportOnDemandSuffix<'source>,
) -> Doc<'source> {
    let dot = format_required_field(suffix.dot(), doc, |dot, doc| format_path_token(doc, &dot));
    let star = format_required_field(suffix.star(), doc, |star, doc| {
        format_path_token(doc, &star)
    });
    doc.concat([dot, star])
}

fn format_import_alias<'source>(
    doc: &mut DocBuilder<'source>,
    alias: &ImportAlias<'source>,
) -> Doc<'source> {
    let keyword = format_required_field(alias.alias_keyword(), doc, |token, doc| {
        format_token(
            doc,
            &token,
            LeadingTrivia::SuppressAlreadyHandled,
            TrailingTrivia::RelocatedToEnclosingContext,
        )
    });
    let name = format_required_field(alias.name(), doc, |name, doc| {
        let has_token = name.first_token().is_some();
        let name = format_name(doc, &name);
        if has_token {
            let space = doc.space();
            doc.concat([space, name])
        } else {
            name
        }
    });
    doc.concat([keyword, name])
}

fn format_path_token<'source>(
    doc: &mut DocBuilder<'source>,
    token: &KotlinSyntaxToken<'source>,
) -> Doc<'source> {
    format_token(
        doc,
        token,
        LeadingTrivia::Preserve,
        TrailingTrivia::Preserve,
    )
}
