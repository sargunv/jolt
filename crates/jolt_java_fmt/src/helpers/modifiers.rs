use jolt_fmt_ir::{Doc, concat, hard_line, text};
use jolt_java_syntax::{JavaSyntaxKind, JavaSyntaxToken, ModifierEntry};

pub(crate) fn modifier_prefix_from_docs(
    annotation_docs: Vec<Doc>,
    modifier_entries: Vec<ModifierEntry>,
) -> Doc {
    let modifier_entries = sorted_modifier_entries(modifier_entries);

    let mut docs = Vec::new();
    for annotation in annotation_docs {
        docs.push(annotation);
        docs.push(hard_line());
    }
    if !modifier_entries.is_empty() {
        docs.push(jolt_fmt_ir::join(
            text(" "),
            modifier_entries
                .into_iter()
                .map(|entry| text(modifier_entry_text(&entry))),
        ));
        docs.push(text(" "));
    }

    concat(docs)
}

pub(crate) fn inline_modifier_prefix_from_docs(
    annotation_docs: Vec<Doc>,
    modifier_tokens: Vec<JavaSyntaxToken>,
) -> Doc {
    let modifier_tokens = sorted_modifier_tokens(modifier_tokens);
    let mut docs = annotation_docs;
    docs.extend(
        modifier_tokens
            .into_iter()
            .map(|token| text(token.text().to_owned())),
    );

    if docs.is_empty() {
        jolt_fmt_ir::nil()
    } else {
        concat([jolt_fmt_ir::join(text(" "), docs), text(" ")])
    }
}

fn sorted_modifier_tokens(mut tokens: Vec<JavaSyntaxToken>) -> Vec<JavaSyntaxToken> {
    tokens.sort_by_key(|token| modifier_order(token.kind()));
    tokens
}

fn sorted_modifier_entries(mut entries: Vec<ModifierEntry>) -> Vec<ModifierEntry> {
    entries.sort_by_key(modifier_entry_order);
    entries
}

fn modifier_entry_order(entry: &ModifierEntry) -> u8 {
    match modifier_entry_text(entry).as_str() {
        "sealed" => 11,
        "non-sealed" => 12,
        _ => entry
            .tokens
            .first()
            .map_or(u8::MAX, |token| modifier_order(token.kind())),
    }
}

fn modifier_entry_text(entry: &ModifierEntry) -> String {
    entry.tokens.iter().map(JavaSyntaxToken::text).collect()
}

const fn modifier_order(kind: JavaSyntaxKind) -> u8 {
    match kind {
        JavaSyntaxKind::PublicKw => 0,
        JavaSyntaxKind::ProtectedKw => 1,
        JavaSyntaxKind::PrivateKw => 2,
        JavaSyntaxKind::AbstractKw => 3,
        JavaSyntaxKind::DefaultKw => 4,
        JavaSyntaxKind::StaticKw => 5,
        JavaSyntaxKind::FinalKw => 6,
        JavaSyntaxKind::TransientKw => 7,
        JavaSyntaxKind::VolatileKw => 8,
        JavaSyntaxKind::SynchronizedKw => 9,
        JavaSyntaxKind::NativeKw => 10,
        JavaSyntaxKind::StrictfpKw => 13,
        _ => u8::MAX,
    }
}
