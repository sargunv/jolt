use jolt_fmt_ir::{Doc, concat, hard_line, text};

use crate::policy::JavaFormatPolicy;

pub(crate) struct ImportDeclarationLayout {
    pub(crate) is_module: bool,
    pub(crate) is_static: bool,
    pub(crate) name: Doc,
    pub(crate) is_on_demand: bool,
}

pub(crate) struct ImportSectionItem {
    doc: Doc,
    kind: ImportKind,
    top_level: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ImportKind {
    Static,
    Module,
    Normal,
}

impl ImportSectionItem {
    pub(crate) fn new(
        doc: Doc,
        is_module: bool,
        is_static: bool,
        top_level: Option<String>,
    ) -> Self {
        Self {
            doc,
            kind: ImportKind::from_flags(is_module, is_static),
            top_level,
        }
    }

    fn group(&self) -> Option<ImportGroup> {
        match self.kind {
            ImportKind::Static => Some(ImportGroup::Static),
            ImportKind::Module => Some(ImportGroup::Module),
            ImportKind::Normal => self.top_level.clone().map(ImportGroup::TopLevel),
        }
    }
}

impl ImportKind {
    fn from_flags(is_module: bool, is_static: bool) -> Self {
        if is_module {
            Self::Module
        } else if is_static {
            Self::Static
        } else {
            Self::Normal
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ImportGroup {
    Static,
    Module,
    TopLevel(String),
}

pub(crate) fn import_section(imports: Vec<ImportSectionItem>, policy: JavaFormatPolicy) -> Doc {
    let mut imports = imports.into_iter();
    let Some(first) = imports.next() else {
        return text("");
    };

    let mut previous_group = first.group();
    let mut docs = vec![first.doc];
    for import in imports {
        let group = import.group();
        let separator = if separates_import_groups(previous_group.as_ref(), group.as_ref(), policy)
        {
            concat([hard_line(), hard_line()])
        } else {
            hard_line()
        };
        docs.push(separator);
        docs.push(import.doc);
        previous_group = group;
    }

    concat(docs)
}

fn separates_import_groups(
    previous_group: Option<&ImportGroup>,
    group: Option<&ImportGroup>,
    policy: JavaFormatPolicy,
) -> bool {
    policy.separates_static_import_section()
        && previous_group.is_some()
        && group.is_some()
        && previous_group != group
}

pub(crate) fn import_declaration(import: ImportDeclarationLayout) -> Doc {
    let mut parts = vec![text("import ")];
    if import.is_module {
        parts.push(text("module "));
    }
    if import.is_static {
        parts.push(text("static "));
    }
    parts.push(import.name);
    if import.is_on_demand {
        parts.push(text(".*"));
    }
    parts.push(text(";"));
    concat(parts)
}
