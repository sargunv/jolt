use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use jolt_java_fmt::{JavaFormatOptions, format_source};

#[test]
fn style_rule_fixtures_match_expected_output() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/style");
    let cases = collect_cases(&root);

    assert!(
        !cases.is_empty(),
        "expected at least one Java style fixture under {}",
        root.display()
    );

    let mut failures: Vec<String> = Vec::new();

    for case in cases {
        let input = read_to_string(&case.input_path);
        let expected = read_to_string(&case.expected_path);
        let formatted = match format_or_error(&case.input_path, &input) {
            Ok(formatted) => formatted,
            Err(error) => {
                failures.push(format!("{}: {error}", case.name));
                continue;
            }
        };

        let formatted_matches_expected = formatted == expected;
        if !formatted_matches_expected {
            failures.push(format!("{}: formatted output differed", case.name));
        }

        match format_or_error(&case.expected_path, &expected) {
            Ok(formatted_expected) if formatted_matches_expected => {
                if formatted_expected != expected {
                    failures.push(format!("{}: expected output was not idempotent", case.name));
                }
            }
            Ok(_) => {}
            Err(error) => failures.push(format!("{}: {error}", case.name)),
        }

        match format_or_error(&case.input_path, &input) {
            Ok(repeated) => {
                if repeated != formatted {
                    failures.push(format!("{}: formatting was not deterministic", case.name));
                }
            }
            Err(error) => failures.push(format!("{}: {error}", case.name)),
        }
    }

    if !failures.is_empty() {
        let mut message = format!("{} Java style fixture failure(s):", failures.len());
        for failure in failures {
            write!(message, "\n- {failure}").expect("write to string");
        }
        panic!("{message}");
    }
}

fn collect_cases(root: &Path) -> Vec<FixtureCase> {
    assert!(
        root.is_dir(),
        "required Java style fixture directory is missing: {}",
        root.display()
    );

    let mut input_paths = Vec::new();
    collect_input_paths(root, &mut input_paths);
    input_paths.sort();

    input_paths
        .into_iter()
        .map(|input_path| {
            let expected_path = expected_path_for(&input_path);
            assert!(
                expected_path.is_file(),
                "missing expected fixture for {}: {}",
                input_path.display(),
                expected_path.display()
            );

            FixtureCase {
                name: input_path
                    .strip_prefix(root)
                    .expect("fixture path should be under root")
                    .display()
                    .to_string(),
                input_path,
                expected_path,
            }
        })
        .collect()
}

fn collect_input_paths(root: &Path, input_paths: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
    {
        let path = entry.expect("valid directory entry").path();
        if path.is_dir() {
            collect_input_paths(&path, input_paths);
        } else if path
            .file_name()
            .and_then(|file_name| file_name.to_str())
            .is_some_and(|file_name| file_name.ends_with(".input.java"))
        {
            input_paths.push(path);
        }
    }
}

fn expected_path_for(input_path: &Path) -> PathBuf {
    let file_name = input_path
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .expect("fixture path must have a UTF-8 file name");
    let expected_name = file_name
        .strip_suffix(".input.java")
        .expect("input fixture must end in .input.java")
        .to_owned()
        + ".expected.java";

    input_path.with_file_name(expected_name)
}

fn read_to_string(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn format_or_error(path: &Path, source: &str) -> Result<String, String> {
    let result = format_source(source, &JavaFormatOptions::default());
    if !result.diagnostics.is_empty() {
        return Err(format!(
            "formatter diagnostics in {}: {:#?}",
            path.display(),
            result.diagnostics
        ));
    }

    result
        .formatted_source
        .ok_or_else(|| format!("formatter blocked without output for {}", path.display()))
}

struct FixtureCase {
    name: String,
    input_path: PathBuf,
    expected_path: PathBuf,
}
