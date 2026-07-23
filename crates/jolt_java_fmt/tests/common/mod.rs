use jolt_fmt_ir::FormatOptions;
use jolt_java_fmt::format_source_to_sink;
use jolt_java_syntax::parse_compilation_unit;
use jolt_test_support::{
    CorpusLanguage, CorpusParseFacts, corpus_parse_facts, format_source_or_panic,
};

pub(crate) struct JavaCorpus;

impl CorpusLanguage for JavaCorpus {
    fn language_name(&self) -> &'static str {
        "Java"
    }

    fn parse_facts(&self, source: &str) -> CorpusParseFacts {
        let parse = parse_compilation_unit(source);
        let tokens = parse
            .syntax()
            .map(|syntax| syntax.token_iter().collect::<Vec<_>>())
            .unwrap_or_default();
        corpus_parse_facts(parse.syntax().is_some(), parse.diagnostics(), tokens)
    }

    fn format(&self, source: &str, label: &str) -> String {
        format_source_or_panic(
            format_source_to_sink,
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
            || name == "disambiguates-when-in-switch-labels--invalid-guard.java"
    }
}
