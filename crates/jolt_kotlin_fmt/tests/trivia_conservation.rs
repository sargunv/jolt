use std::collections::BTreeMap;
use std::path::PathBuf;

use jolt_kotlin_fmt::{KotlinFormatOptions, KotlinFormatSinkResult, format_source_to_sink};
use jolt_kotlin_syntax::parse_kotlin_file;
use jolt_test_support::{StringSink, collect_kotlin_files, kotlin_fixture_root, read_to_string};

#[test]
fn trivia_markers_are_conserved_by_formatter() {
    let root = kotlin_fixture_root(env!("CARGO_MANIFEST_DIR")).join("trivia");
    for path in collect_kotlin_files(&root) {
        let source = read_to_string(&path);
        let expected = trivia_markers(&source);
        assert!(
            !expected.is_empty(),
            "expected trivia fixture to contain at least one marker: {}",
            path.display()
        );

        let parse = parse_kotlin_file(&source);
        assert!(
            parse.diagnostics().is_empty(),
            "trivia fixture must parse cleanly before formatting: {}\n{:#?}",
            path.display(),
            parse.diagnostics()
        );

        let formatted = format_or_panic(&source, &path);
        assert_eq!(
            trivia_markers(&formatted),
            expected,
            "formatter must conserve trivia markers in {}",
            path.display()
        );

        let formatted_again = format_or_panic(&formatted, &path);
        assert_eq!(
            formatted_again,
            formatted,
            "formatter output must be idempotent for {}",
            path.display()
        );
    }
}

fn format_or_panic(source: &str, path: &PathBuf) -> String {
    let mut sink = StringSink::default();
    match format_source_to_sink(source, &KotlinFormatOptions::default(), &mut sink) {
        KotlinFormatSinkResult::Complete | KotlinFormatSinkResult::Halted => sink.into_string(),
        KotlinFormatSinkResult::Blocked { diagnostics } => {
            panic!(
                "formatter diagnostics in {}: {diagnostics:#?}",
                path.display()
            )
        }
    }
}

fn trivia_markers(source: &str) -> BTreeMap<String, usize> {
    let mut markers = BTreeMap::new();
    for (start, _) in source.match_indices("JOLT-TRIVIA:") {
        let marker = source[start..]
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '_' | '-'))
            .collect::<String>();
        *markers.entry(marker).or_insert(0) += 1;
    }
    markers
}
