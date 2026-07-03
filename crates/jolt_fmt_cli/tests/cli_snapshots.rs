use std::{
    fs,
    path::{Path, PathBuf},
    process::Output,
};

use assert_cmd::{Command, cargo::cargo_bin};
use tempfile::TempDir;

#[test]
fn write_mode_rewrites_changed_file_without_user_output() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("A.java"), "class A {}\n");

    let output = jolt(temp.path(), ["fmt", "A.java"], "");

    insta::assert_snapshot!(
        "write_mode_rewrites_changed_file_without_user_output",
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
fn check_mode_accepts_formatted_file_without_user_output() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("A.java"), "class A {\n}\n");

    let output = jolt(temp.path(), ["fmt", "--check", "A.java"], "");

    insta::assert_snapshot!(
        "check_mode_accepts_formatted_file_without_user_output",
        snapshot(&output, &[temp.path().join("A.java")])
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

fn push_stream(rendered: &mut String, label: &str, bytes: &[u8]) {
    rendered.push_str(label);
    rendered.push_str(":\n");
    if bytes.is_empty() {
        rendered.push_str("<empty>\n");
        return;
    }
    rendered.push_str(&String::from_utf8_lossy(bytes));
}

fn write(path: impl AsRef<Path>, contents: &str) {
    fs::write(path, contents).expect("file should be written");
}
