use jolt_fmt_ir::{FormatOptions, SyntaxErrorPolicy};
use jolt_kotlin_fmt::format_source_to_sink;
use jolt_kotlin_syntax::parse_kotlin_file;
use jolt_test_support::{
    CorpusLanguage, CorpusParseFacts, corpus_parse_facts, format_source_or_panic,
};

pub(crate) struct KotlinCorpus;

impl CorpusLanguage for KotlinCorpus {
    fn language_name(&self) -> &'static str {
        "Kotlin"
    }

    fn parse_facts(&self, source: &str) -> CorpusParseFacts {
        let parse = parse_kotlin_file(source);
        let tokens = parse
            .syntax()
            .map(|syntax| syntax.token_iter().collect::<Vec<_>>())
            .unwrap_or_default();
        corpus_parse_facts(parse.syntax().is_some(), parse.diagnostics(), tokens)
    }

    fn format(&self, source: &str, label: &str) -> String {
        format_source_or_panic(
            |source, options, sink| {
                format_source_to_sink(source, options, SyntaxErrorPolicy::Format, sink)
            },
            source,
            &FormatOptions::default(),
            label,
        )
    }

    fn expects_parser_diagnostics(&self, relative: &str) -> bool {
        let Some(name) = relative.strip_prefix("syntax/parser/") else {
            return false;
        };
        name.starts_with("diagnoses-")
            || name.starts_with("recovers-")
            || name == "parses-destructuring-square-preview.kt"
    }
}
