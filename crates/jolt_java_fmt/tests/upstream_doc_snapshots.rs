use std::fs;
use std::path::{Path, PathBuf};

use jolt_java_fmt::JavaFormatOptions;

mod support;
use support::format_source;

#[test]
fn upstream_doc_snippets_format_snapshots() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/upstream-docs");
    let mut cases = collect_cases(&root);
    let options = JavaFormatOptions {
        indent_width: 4,
        ..JavaFormatOptions::default()
    };
    assert!(
        !cases.is_empty(),
        "expected at least one upstream-doc Java fixture under {}",
        root.display()
    );

    cases.sort();
    for case in cases {
        let source = fs::read_to_string(&case)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", case.display()));
        let result = format_source(&source, &options);
        assert!(
            result.diagnostics.is_empty(),
            "formatter diagnostics in {}: {:#?}",
            case.display(),
            result.diagnostics
        );

        let name = case
            .file_name()
            .and_then(|file_name| file_name.to_str())
            .and_then(|file_name| file_name.strip_suffix(".input.java"))
            .expect("upstream-doc fixture names must end in .input.java");
        let formatted = result
            .formatted_source
            .unwrap_or_else(|| panic!("formatter blocked without output for {}", case.display()));
        insta::assert_snapshot!(name, formatted);
    }
}

fn collect_cases(root: &Path) -> Vec<PathBuf> {
    assert!(
        root.is_dir(),
        "required upstream-doc fixture directory is missing: {}",
        root.display()
    );

    fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
        .map(|entry| entry.expect("valid directory entry").path())
        .filter(|path| {
            path.file_name()
                .and_then(|file_name| file_name.to_str())
                .is_some_and(|file_name| file_name.ends_with(".input.java"))
        })
        .collect()
}
