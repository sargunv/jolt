use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use jolt_java_syntax::{JavaLexer, JavaSyntaxKind, LexerDiagnostic, Token};

#[test]
fn oracle_java_inputs_lex_without_loss() {
    let google_summary = assert_corpus("google-java-format", 209);
    let palantir_summary = assert_corpus("palantir-java-format", 226);

    insta::assert_snapshot!("google_java_format_lexer_summary", google_summary);
    insta::assert_snapshot!("palantir_java_format_lexer_summary", palantir_summary);
}

fn assert_corpus(suite: &str, expected_files: usize) -> String {
    let root = fixture_root(suite);
    let mut files = Vec::new();
    collect_java_files(&root, &mut files);

    files.sort();
    assert_eq!(
        files.len(),
        expected_files,
        "expected the pinned {suite} Java input fixture corpus"
    );

    let mut summary = CorpusSummary::new(suite, files.len());
    for path in files {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let lexed = lex(&source);
        assert_reconstructs(&path, &source, &lexed.tokens);
        summary.record(&lexed);

        assert!(
            lexed.diagnostics.is_empty(),
            "lexer diagnostic(s) in {}: {:#?}",
            path.display(),
            lexed.diagnostics
        );
    }

    summary.render()
}

struct Lexed {
    tokens: Vec<Token>,
    diagnostics: Vec<LexerDiagnostic>,
}

fn lex(source: &str) -> Lexed {
    let mut lexer = JavaLexer::new(source);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        let at_eof = token.kind == JavaSyntaxKind::Eof;
        tokens.push(token);
        if at_eof {
            break;
        }
    }
    let diagnostics = lexer.finish();
    Lexed {
        tokens,
        diagnostics,
    }
}

fn fixture_root(suite: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".oracles/fixtures")
        .join(suite)
        .join("input")
}

fn collect_java_files(root: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).unwrap_or_else(|error| {
        panic!(
            "failed to read fixture directory {}: {error}",
            root.display()
        )
    }) {
        let path = entry.expect("valid directory entry").path();
        if path.is_dir() {
            collect_java_files(&path, files);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "java")
        {
            files.push(path);
        }
    }
}

fn assert_reconstructs(path: &Path, source: &str, tokens: &[jolt_java_syntax::Token]) {
    let mut cursor = 0usize;
    let mut reconstructed = String::with_capacity(source.len());

    for token in tokens {
        for trivia in &token.leading {
            append_range(path, source, &mut reconstructed, &mut cursor, trivia.range);
        }

        if token.kind == JavaSyntaxKind::Eof {
            assert_eq!(
                token.range.start().get(),
                source.len(),
                "EOF token range must start at source end in {}",
                path.display()
            );
            assert_eq!(
                token.range.end().get(),
                source.len(),
                "EOF token range must end at source end in {}",
                path.display()
            );
        } else {
            append_range(path, source, &mut reconstructed, &mut cursor, token.range);
        }

        for trivia in &token.trailing {
            append_range(path, source, &mut reconstructed, &mut cursor, trivia.range);
        }
    }

    assert_eq!(
        cursor,
        source.len(),
        "token/trivia ranges did not consume all source in {}",
        path.display()
    );
    assert_eq!(
        reconstructed,
        source,
        "token/trivia reconstruction changed source in {}",
        path.display()
    );
}

fn append_range(
    path: &Path,
    source: &str,
    reconstructed: &mut String,
    cursor: &mut usize,
    range: jolt_text::TextRange,
) {
    assert_eq!(
        range.start().get(),
        *cursor,
        "non-contiguous token/trivia range in {}",
        path.display()
    );
    assert!(
        range.end().get() <= source.len(),
        "token/trivia range extends past source end in {}",
        path.display()
    );
    reconstructed.push_str(&source[range.start().get()..range.end().get()]);
    *cursor = range.end().get();
}

struct CorpusSummary {
    suite: String,
    files: usize,
    tokens: BTreeMap<String, usize>,
    trivia: BTreeMap<String, usize>,
    diagnostics: BTreeMap<String, usize>,
}

impl CorpusSummary {
    fn new(suite: &str, files: usize) -> Self {
        Self {
            suite: suite.to_owned(),
            files,
            tokens: BTreeMap::new(),
            trivia: BTreeMap::new(),
            diagnostics: BTreeMap::new(),
        }
    }

    fn record(&mut self, lexed: &Lexed) {
        for token in &lexed.tokens {
            increment(&mut self.tokens, format!("{:?}", token.kind));

            for trivia in &token.leading {
                increment(&mut self.trivia, format!("{:?}", trivia.kind));
            }
            for trivia in &token.trailing {
                increment(&mut self.trivia, format!("{:?}", trivia.kind));
            }
        }

        for diagnostic in &lexed.diagnostics {
            increment(&mut self.diagnostics, format!("{:?}", diagnostic.kind));
        }
    }

    fn render(&self) -> String {
        let mut output = String::new();
        writeln!(&mut output, "suite: {}", self.suite).expect("write summary");
        writeln!(&mut output, "files: {}", self.files).expect("write summary");
        output.push_str("\ntokens:\n");
        push_counts(&mut output, &self.tokens);
        output.push_str("\ntrivia:\n");
        push_counts(&mut output, &self.trivia);
        output.push_str("\ndiagnostics:\n");
        push_counts(&mut output, &self.diagnostics);
        output
    }
}

fn increment(counts: &mut BTreeMap<String, usize>, key: String) {
    *counts.entry(key).or_default() += 1;
}

fn push_counts(output: &mut String, counts: &BTreeMap<String, usize>) {
    if counts.is_empty() {
        output.push_str("  <none>: 0\n");
        return;
    }

    for (kind, count) in counts {
        writeln!(output, "  {kind}: {count}").expect("write summary");
    }
}
