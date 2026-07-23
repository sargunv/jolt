use std::path::PathBuf;

use jolt_fmt_ir::{FormatOptions, FormatSinkResult};
use jolt_kotlin_fmt::format_source_to_sink;
use jolt_kotlin_syntax::parse_kotlin_file;
use jolt_test_support::{StringSink, collect_kotlin_files, read_to_string, workspace_root};

#[test]
fn imported_fixture_inputs_format_idempotently_and_parse() {
    for suite in ["ktfmt", "maplibre-compose"] {
        let root = fixture_root(suite);
        let options = FormatOptions::default();

        for path in collect_kotlin_files(&root) {
            let source = read_to_string(&path);
            let parse = parse_kotlin_file(&source);
            let syntax = parse.syntax().unwrap_or_else(|| {
                panic!(
                    "parser produced no represented tree for {}: {:#?}",
                    path.display(),
                    parse.diagnostics()
                )
            });
            assert_eq!(
                syntax.source_text(),
                source,
                "syntax tree did not reconstruct exactly for {}",
                path.display()
            );
            assert!(
                parse.diagnostics().is_empty(),
                "imported Kotlin source produced diagnostics for {}: {:#?}",
                path.display(),
                parse.diagnostics()
            );

            let formatted = match format_source(&source, options) {
                Ok(formatted) => formatted,
                Err(diagnostics) => panic!(
                    "formatter refused clean imported input {}: {diagnostics:#?}",
                    path.display()
                ),
            };

            let formatted_parse = parse_kotlin_file(&formatted);
            assert!(
                formatted_parse.diagnostics().is_empty(),
                "formatted output did not parse cleanly for {}: {:#?}\n{}",
                path.display(),
                formatted_parse.diagnostics(),
                formatted
            );
            let formatted_syntax = formatted_parse.syntax().unwrap_or_else(|| {
                panic!(
                    "formatted output produced no syntax tree for {}",
                    path.display()
                )
            });
            assert_eq!(
                formatted_syntax.source_text(),
                formatted,
                "formatted output did not reconstruct exactly for {}",
                path.display()
            );

            let formatted_again =
                format_source(&formatted, options).unwrap_or_else(|diagnostics| {
                    panic!(
                        "formatted output was not accepted by formatter for {}: {diagnostics:#?}",
                        path.display()
                    )
                });
            assert_eq!(
                formatted_again,
                formatted,
                "formatted output was not idempotent for {}",
                path.display()
            );

            let repeated = format_source(&source, options).unwrap_or_else(|diagnostics| {
                panic!(
                    "repeated formatting produced diagnostic(s) for {}: {diagnostics:#?}",
                    path.display()
                )
            });
            assert_eq!(
                repeated,
                formatted,
                "formatting was not deterministic for {}",
                path.display()
            );
        }
    }
}

#[test]
fn deeply_nested_recovery_formats_idempotently_and_keeps_following_syntax() {
    let depth = 4096;
    let sources = [
        format!(
            "typealias Deep = {}Leaf{}\nclass Following\n",
            "Box<".repeat(depth),
            ">".repeat(depth)
        ),
        format!(
            "fun value() = {}true\nval following = 1\nclass Following\n",
            "! ".repeat(depth)
        ),
        format!(
            "typealias Deep = {}Leaf{}\nclass Following\n",
            "Box<@A(value = input as ".repeat(depth),
            ") Annotated>".repeat(depth)
        ),
        format!(
            "typealias Deep = {}suspend () -> Unit{}\nclass Following\n",
            "Box<".repeat(127),
            ">".repeat(127)
        ),
        format!(
            "typealias Deep = {}context() () -> Unit{}\nclass Following\n",
            "Box<".repeat(127),
            ">".repeat(127)
        ),
        format!(
            "fun value() = {}leaf\nclass Following\n",
            "target = ".repeat(depth)
        ),
        format!(
            "fun value() = {}input{}\nclass Following\n",
            "(".repeat(depth),
            ")".repeat(depth)
        ),
    ];

    for source in sources {
        let parse = parse_kotlin_file(&source);
        assert_eq!(
            parse
                .syntax()
                .expect("represented deep input")
                .source_text(),
            source
        );

        let formatted = format_source(&source, FormatOptions::default())
            .unwrap_or_else(|diagnostics| panic!("formatter blocked: {diagnostics:#?}"));
        assert!(formatted.contains("class Following"));
        let reparsed = parse_kotlin_file(&formatted);
        assert_eq!(
            reparsed
                .syntax()
                .expect("represented formatted output")
                .source_text(),
            formatted
        );
        let formatted_again = format_source(&formatted, FormatOptions::default())
            .unwrap_or_else(|diagnostics| panic!("second format blocked: {diagnostics:#?}"));
        assert_eq!(formatted_again, formatted);
    }
}

fn format_source(
    source: &str,
    options: FormatOptions,
) -> Result<String, Vec<jolt_diagnostics::Diagnostic>> {
    let mut sink = StringSink::default();
    match format_source_to_sink(source, &options, &mut sink) {
        FormatSinkResult::Complete => Ok(sink.into_string()),
        FormatSinkResult::Halted => panic!("formatter unexpectedly halted with StringSink"),
        FormatSinkResult::Blocked { diagnostic } => Err(vec![diagnostic]),
    }
}

fn fixture_root(suite: &str) -> PathBuf {
    workspace_root(env!("CARGO_MANIFEST_DIR"))
        .join("tools/import/.imports")
        .join(suite)
        .join("source")
}
