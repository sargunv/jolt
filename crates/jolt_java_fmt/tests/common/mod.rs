use jolt_fmt_ir::FormatOptions;
use jolt_java_fmt::format_source_to_sink;
use jolt_java_syntax::parse_compilation_unit;
use jolt_test_support::{
    CorpusLanguage, CorpusParseFacts, RepresentedTokenRemoval, corpus_parse_facts,
    format_source_or_panic,
};

const REMOVE_ONE_SEMICOLON: &[RepresentedTokenRemoval] = &[RepresentedTokenRemoval {
    source: ";",
    count: 1,
}];
const REMOVE_TWO_SEMICOLONS: &[RepresentedTokenRemoval] = &[RepresentedTokenRemoval {
    source: ";",
    count: 2,
}];
const REMOVE_THREE_SEMICOLONS: &[RepresentedTokenRemoval] = &[RepresentedTokenRemoval {
    source: ";",
    count: 3,
}];
const REMOVE_FOUR_SEMICOLONS: &[RepresentedTokenRemoval] = &[RepresentedTokenRemoval {
    source: ";",
    count: 4,
}];
const REMOVE_EIGHT_SEMICOLONS: &[RepresentedTokenRemoval] = &[RepresentedTokenRemoval {
    source: ";",
    count: 8,
}];

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

    fn allowed_clean_removals(&self, relative: &str) -> &'static [RepresentedTokenRemoval] {
        match relative {
            "style/compact-compilation-units/mixed-declarations-preserve-order.java"
            | "style/compact-compilation-units/mixed-declarations-second-order.java"
            | "style/program/top-level-blank-lines.java"
            | "style/statements/while-empty-body.java"
            | "syntax/parser/try-with-resources-has-resource-specification-boundary.java"
            | "trivia/symbol-crevices.java" => REMOVE_ONE_SEMICOLON,
            "style/statements/labels-and-loop-bodies.java"
            | "style/statements/try-with-resources.java"
            | "syntax/parser/parses-block-local-declarations-and-statement-forms.java" => {
                REMOVE_TWO_SEMICOLONS
            }
            "style/program/removes-top-level-semicolons.java" => REMOVE_THREE_SEMICOLONS,
            "style/statements/remove-empty-block-statements.java"
            | "syntax/parser/parses-empty-declaration-alternatives.java" => REMOVE_FOUR_SEMICOLONS,
            "style/declarations/remove-empty-type-body-declarations.java" => {
                REMOVE_EIGHT_SEMICOLONS
            }
            "style/declarations/enum-constant-comments.java" => &[RepresentedTokenRemoval {
                source: ",",
                count: 1,
            }],
            "style/expressions/calls-and-arguments.java"
            | "syntax/parser/switch-case-pattern-label-items-are-structured-with-guards.java" => &[
                RepresentedTokenRemoval {
                    source: "(",
                    count: 1,
                },
                RepresentedTokenRemoval {
                    source: ")",
                    count: 1,
                },
            ],
            "style/expressions/lambdas.java" => &[
                RepresentedTokenRemoval {
                    source: "(",
                    count: 2,
                },
                RepresentedTokenRemoval {
                    source: ")",
                    count: 2,
                },
            ],
            _ => &[],
        }
    }
}
