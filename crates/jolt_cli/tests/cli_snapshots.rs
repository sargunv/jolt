use std::{
    fs,
    path::{Path, PathBuf},
    process::Output,
};

use assert_cmd::{Command, cargo::cargo_bin};
use tempfile::TempDir;

#[test]
fn top_level_help_describes_product_cli() {
    let temp = TempDir::new().expect("tempdir should be created");

    let output = jolt(temp.path(), ["--help"], "");

    insta::assert_snapshot!(
        "top_level_help_describes_product_cli",
        snapshot(&output, &[])
    );
}

#[test]
fn fmt_help_describes_formatter_contract() {
    let temp = TempDir::new().expect("tempdir should be created");

    let output = jolt(temp.path(), ["fmt", "--help"], "");

    insta::assert_snapshot!(
        "fmt_help_describes_formatter_contract",
        snapshot(&output, &[])
    );
}

#[test]
fn stdin_formats_to_stdout() {
    let temp = TempDir::new().expect("tempdir should be created");

    let output = jolt(temp.path(), ["fmt", "-"], "class A {}\n");

    insta::assert_snapshot!("stdin_formats_to_stdout", snapshot(&output, &[]));
}

#[test]
fn write_mode_rewrites_changed_file_and_reports_summary() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("A.java"), "class A {}\n");

    let output = jolt(temp.path(), ["fmt", "A.java"], "");

    insta::assert_snapshot!(
        "write_mode_rewrites_changed_file_and_reports_summary",
        snapshot(&output, &[temp.path().join("A.java")])
    );
}

#[test]
fn write_mode_counts_clean_files_as_formatted() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("A.java"), "class A {\n}\n");

    let output = jolt(temp.path(), ["fmt", "A.java"], "");

    insta::assert_snapshot!(
        "write_mode_counts_clean_files_as_formatted",
        snapshot(&output, &[temp.path().join("A.java")])
    );
}

#[test]
fn check_mode_reports_changed_file_without_writing() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("A.java"), "class A {}\n");

    let output = jolt(temp.path(), ["fmt", "--check", "A.java"], "");

    insta::assert_snapshot!(
        "check_mode_reports_changed_file_without_writing",
        snapshot(&output, &[temp.path().join("A.java")])
    );
}

#[test]
fn check_mode_reports_changed_stdin_filename() {
    let temp = TempDir::new().expect("tempdir should be created");

    let output = jolt(
        temp.path(),
        ["fmt", "--check", "--stdin-filename", "src/Main.java", "-"],
        "class A {}\n",
    );

    insta::assert_snapshot!(
        "check_mode_reports_changed_stdin_filename",
        snapshot(&output, &[])
    );
}

#[test]
fn check_mode_reports_all_changed_files_with_summary_count() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("A.java"), "class A {}\n");
    write(temp.path().join("B.java"), "class B {\n}\n");
    write(temp.path().join("C.java"), "class C {}\n");

    let output = jolt(temp.path(), ["fmt", "--check", "."], "");

    insta::assert_snapshot!(
        "check_mode_reports_all_changed_files_with_summary_count",
        snapshot(
            &output,
            &[
                temp.path().join("A.java"),
                temp.path().join("B.java"),
                temp.path().join("C.java"),
            ],
        )
    );
}

#[test]
fn check_mode_reports_mixed_changed_and_failed_files() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("Changed.java"), "class Changed {}\n");
    write(temp.path().join("Bad.java"), "class {\n");

    let output = jolt(temp.path(), ["fmt", "--check", "."], "");

    insta::assert_snapshot!(
        "check_mode_reports_mixed_changed_and_failed_files",
        snapshot(
            &output,
            &[
                temp.path().join("Bad.java"),
                temp.path().join("Changed.java"),
            ],
        )
    );
}

#[test]
fn check_mode_accepts_formatted_file_and_reports_summary() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("A.java"), "class A {\n}\n");

    let output = jolt(temp.path(), ["fmt", "--check", "A.java"], "");

    insta::assert_snapshot!(
        "check_mode_accepts_formatted_file_and_reports_summary",
        snapshot(&output, &[temp.path().join("A.java")])
    );
}

#[test]
fn invalid_flags_report_usage_errors() {
    let temp = TempDir::new().expect("tempdir should be created");

    let output = jolt(temp.path(), ["fmt", "--threads", "0", "-"], "");

    insta::assert_snapshot!("invalid_flags_report_usage_errors", snapshot(&output, &[]));
}

#[test]
fn config_errors_report_diagnostics() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(
        temp.path().join("jolt.toml"),
        "[format]\nindent-width = 0\n",
    );

    let output = jolt(temp.path(), ["fmt", "-"], "class A {}\n");

    insta::assert_snapshot!(
        "config_errors_report_diagnostics",
        snapshot_normalized(&output, &[], temp.path())
    );
}

#[test]
fn parse_errors_report_diagnostics_and_do_not_write() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("A.java"), "class {\n");

    let output = jolt(temp.path(), ["fmt", "A.java"], "");

    insta::assert_snapshot!(
        "parse_errors_report_diagnostics_and_do_not_write",
        snapshot(&output, &[temp.path().join("A.java")])
    );
}

#[test]
fn check_mode_parse_errors_report_diagnostics_and_do_not_write() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("A.java"), "class {\n");

    let output = jolt(temp.path(), ["fmt", "--check", "A.java"], "");

    insta::assert_snapshot!(
        "check_mode_parse_errors_report_diagnostics_and_do_not_write",
        snapshot(&output, &[temp.path().join("A.java")])
    );
}

#[test]
fn stdin_parse_errors_use_stdin_filename_in_diagnostics() {
    let temp = TempDir::new().expect("tempdir should be created");

    let output = jolt(
        temp.path(),
        ["fmt", "--stdin-filename", "src/Main.java", "-"],
        "class {\n",
    );

    insta::assert_snapshot!(
        "stdin_parse_errors_use_stdin_filename_in_diagnostics",
        snapshot(&output, &[])
    );
}

#[test]
fn explicit_unknown_extension_file_is_processed_as_java() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("README.md"), "class Readme {}\n");

    let output = jolt(temp.path(), ["fmt", "README.md"], "");

    insta::assert_snapshot!(
        "explicit_unknown_extension_file_is_processed_as_java",
        snapshot(&output, &[temp.path().join("README.md")])
    );
}

#[test]
fn missing_path_reports_error() {
    let temp = TempDir::new().expect("tempdir should be created");

    let output = jolt(temp.path(), ["fmt", "Missing.java"], "");

    insta::assert_snapshot!("missing_path_reports_error", snapshot(&output, &[]));
}

fn jolt<const N: usize>(dir: &Path, args: [&str; N], stdin: &str) -> Output {
    let mut command = Command::new(cargo_bin("jolt"));
    command.current_dir(dir).args(args);
    if !stdin.is_empty() {
        command.write_stdin(stdin);
    }
    command.output().expect("jolt command should run")
}

fn snapshot(output: &Output, files: &[PathBuf]) -> String {
    let mut rendered = String::new();
    rendered.push_str(&format!("status: {}\n", output.status.code().unwrap_or(-1)));
    push_stream(&mut rendered, "stdout", &output.stdout);
    push_stream(&mut rendered, "stderr", &output.stderr);
    rendered.push_str("files:\n");
    if files.is_empty() {
        rendered.push_str("<none>\n");
    } else {
        for file in files {
            rendered.push_str(&format!(
                "-- {} --\n",
                file.file_name()
                    .expect("snapshot file should have a file name")
                    .to_string_lossy()
            ));
            rendered.push_str(
                &fs::read_to_string(file)
                    .unwrap_or_else(|error| panic!("file should be readable: {error}")),
            );
        }
    }
    rendered
}

fn snapshot_normalized(output: &Output, files: &[PathBuf], temp: &Path) -> String {
    snapshot(output, files).replace(&temp.display().to_string(), "$TEMP")
}

fn push_stream(rendered: &mut String, label: &str, bytes: &[u8]) {
    rendered.push_str(label);
    rendered.push_str(":\n");
    if bytes.is_empty() {
        rendered.push_str("<empty>\n");
        return;
    }
    let text = String::from_utf8_lossy(bytes);
    if label == "stderr" {
        rendered.push_str(&normalize_summary_duration(&text));
    } else {
        rendered.push_str(&text);
    }
}

fn normalize_summary_duration(text: &str) -> String {
    let mut normalized = String::new();
    for line in text.lines() {
        if let Some((prefix, _)) = line.rsplit_once(" in ")
            && (prefix.starts_with("Formatted ") || prefix.starts_with("Checked "))
        {
            normalized.push_str(prefix);
            normalized.push_str(" in $TIME\n");
            continue;
        }
        normalized.push_str(line);
        normalized.push('\n');
    }
    normalized
}

fn write(path: impl AsRef<Path>, contents: &str) {
    fs::write(path, contents).expect("file should be written");
}
