use std::{
    fmt::Write as _,
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
fn config_help_describes_inspection_commands() {
    let temp = TempDir::new().expect("tempdir should be created");

    let output = jolt(temp.path(), ["config", "--help"], "");

    insta::assert_snapshot!(
        "config_help_describes_inspection_commands",
        snapshot(&output, &[])
    );
}

#[test]
fn config_list_reports_discovered_configs() {
    let temp = TempDir::new().expect("tempdir should be created");
    fs::create_dir_all(temp.path().join("src/nested")).expect("nested dir should be created");
    write(temp.path().join("jolt.toml"), "root = true\n");
    write(
        temp.path().join("src/jolt.toml"),
        "[format]\nline-width = 100\n",
    );

    let output = jolt(temp.path(), ["config", "list", "src/nested/A.java"], "");

    insta::assert_snapshot!(
        "config_list_reports_discovered_configs",
        snapshot_normalized(&output, &[], temp.path())
    );
}

#[test]
fn config_list_reports_no_configs() {
    let temp = TempDir::new().expect("tempdir should be created");

    let output = jolt(temp.path(), ["config", "list"], "");

    insta::assert_snapshot!("config_list_reports_no_configs", snapshot(&output, &[]));
}

#[test]
fn config_resolve_reports_effective_config() {
    let temp = TempDir::new().expect("tempdir should be created");
    fs::create_dir_all(temp.path().join("src/nested")).expect("nested dir should be created");
    write(
        temp.path().join("jolt.toml"),
        "root = true\n[format]\nline-width = 100\n[files]\nexclude = [\"generated/**\"]\n",
    );
    write(
        temp.path().join("src/jolt.toml"),
        "[format]\nuse-tabs = true\n[files]\ninclude = [\"**/*.java\"]\nexclude = [\"**/Internal.java\"]\n",
    );

    let output = jolt(
        temp.path(),
        ["config", "resolve", "src/nested/Internal.java"],
        "",
    );

    insta::assert_snapshot!(
        "config_resolve_reports_effective_config",
        snapshot_normalized(&output, &[], temp.path())
    );
}

#[test]
fn config_schema_reports_jolt_schema() {
    let temp = TempDir::new().expect("tempdir should be created");

    let output = jolt(temp.path(), ["config", "schema"], "");

    insta::assert_snapshot!("config_schema_reports_jolt_schema", snapshot(&output, &[]));
}

#[test]
fn config_schema_reports_dprint_schema() {
    let temp = TempDir::new().expect("tempdir should be created");

    let output = jolt(temp.path(), ["config", "schema", "--dprint"], "");

    insta::assert_snapshot!(
        "config_schema_reports_dprint_schema",
        snapshot(&output, &[])
    );
}

#[test]
fn completions_help_describes_supported_shells() {
    let temp = TempDir::new().expect("tempdir should be created");

    let output = jolt(temp.path(), ["completions", "--help"], "");

    insta::assert_snapshot!(
        "completions_help_describes_supported_shells",
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
fn format_command_formats_stdin_to_stdout() {
    let temp = TempDir::new().expect("tempdir should be created");

    let output = jolt(temp.path(), ["format", "-"], "class A {}\n");

    insta::assert_snapshot!(
        "format_command_formats_stdin_to_stdout",
        snapshot(&output, &[])
    );
}

#[test]
fn completions_generate_shell_script() {
    let temp = TempDir::new().expect("tempdir should be created");

    let output = jolt(temp.path(), ["completions", "bash"], "");

    insta::assert_snapshot!("completions_generate_shell_script", snapshot(&output, &[]));
}

#[cfg(feature = "docs-generation")]
#[test]
fn docs_manpages_generate_cohesive_set() {
    let temp = TempDir::new().expect("tempdir should be created");
    let out_dir = temp.path().join("man1");
    let out_dir_arg = out_dir
        .to_str()
        .expect("temp path should be valid unicode for test args");

    let output = jolt(temp.path(), ["__docs", "manpages", out_dir_arg], "");
    let mut files = fs::read_dir(&out_dir)
        .expect("manpage directory should be readable")
        .map(|entry| entry.expect("manpage entry should be readable").path())
        .collect::<Vec<_>>();
    files.sort();

    insta::assert_snapshot!(
        "docs_manpages_generate_cohesive_set",
        snapshot_without_trailing_whitespace(&output, &files)
    );
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
fn check_mode_reports_mixed_changed_and_invalid_files() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("Changed.java"), "class Changed {}\n");
    write(temp.path().join("Bad.java"), "class {\n");

    let output = jolt(temp.path(), ["fmt", "--check", "."], "");

    insta::assert_snapshot!(
        "check_mode_reports_mixed_changed_and_invalid_files",
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
fn syntax_error_is_rejected_without_writing_file() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("A.java"), "class {\n");

    let output = jolt(temp.path(), ["fmt", "A.java"], "");

    insta::assert_snapshot!(
        "syntax_error_is_rejected_without_writing_file",
        snapshot(&output, &[temp.path().join("A.java")])
    );
}

#[test]
fn format_with_errors_formats_and_writes_recovered_parse() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("A.java"), "class {\n");

    let output = jolt(temp.path(), ["fmt", "--format-with-errors", "A.java"], "");

    insta::assert_snapshot!(
        "format_with_errors_formats_and_writes_recovered_parse",
        snapshot(&output, &[temp.path().join("A.java")])
    );
}

#[test]
fn check_mode_reports_syntax_error_as_failed() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("A.java"), "class {\n");

    let output = jolt(temp.path(), ["fmt", "--check", "A.java"], "");

    insta::assert_snapshot!(
        "check_mode_reports_syntax_error_as_failed",
        snapshot(&output, &[temp.path().join("A.java")])
    );
}

#[test]
fn stdin_syntax_error_is_rejected_without_stdout() {
    let temp = TempDir::new().expect("tempdir should be created");

    let output = jolt(
        temp.path(),
        ["fmt", "--stdin-filename", "src/Main.java", "-"],
        "class {\n",
    );

    insta::assert_snapshot!(
        "stdin_syntax_error_is_rejected_without_stdout",
        snapshot(&output, &[])
    );
}

#[test]
fn explicit_unknown_extension_file_reports_unsupported_extension() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("README.md"), "class Readme {}\n");

    let output = jolt(temp.path(), ["fmt", "README.md"], "");

    insta::assert_snapshot!(
        "explicit_unknown_extension_file_reports_unsupported_extension",
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
    writeln!(rendered, "status: {}", output.status.code().unwrap_or(-1))
        .expect("writing to a String should not fail");
    push_stream(&mut rendered, "stdout", &output.stdout);
    push_stream(&mut rendered, "stderr", &output.stderr);
    rendered.push_str("files:\n");
    if files.is_empty() {
        rendered.push_str("<none>\n");
    } else {
        for file in files {
            writeln!(
                rendered,
                "-- {} --",
                file.file_name()
                    .expect("snapshot file should have a file name")
                    .to_string_lossy()
            )
            .expect("writing to a String should not fail");
            rendered.push_str(&normalize_commit_hashes(
                &fs::read_to_string(file)
                    .unwrap_or_else(|error| panic!("file should be readable: {error}")),
            ));
        }
    }
    rendered
}

fn snapshot_normalized(output: &Output, files: &[PathBuf], temp: &Path) -> String {
    let mut rendered = snapshot(output, files);
    if let Ok(canonical) = fs::canonicalize(temp) {
        rendered = rendered.replace(&canonical.display().to_string(), "$TEMP");
    }
    rendered.replace(&temp.display().to_string(), "$TEMP")
}

fn snapshot_without_trailing_whitespace(output: &Output, files: &[PathBuf]) -> String {
    let mut normalized = String::new();
    for line in snapshot(output, files).lines() {
        writeln!(normalized, "{}", line.trim_end()).expect("writing to a String should not fail");
    }
    normalized
}

fn push_stream(rendered: &mut String, label: &str, bytes: &[u8]) {
    rendered.push_str(label);
    rendered.push_str(":\n");
    if bytes.is_empty() {
        rendered.push_str("<empty>\n");
        return;
    }
    let text = String::from_utf8_lossy(bytes);
    let text = if label == "stderr" {
        normalize_summary_duration(&text)
    } else {
        text.into_owned()
    };
    rendered.push_str(&normalize_commit_hashes(&text));
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

fn normalize_commit_hashes(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('(') {
        normalized.push_str(&rest[..start]);
        let after_open = &rest[start + 1..];
        if let Some(end) = after_open.find(')') {
            let candidate = &after_open[..end];
            if (7..=40).contains(&candidate.len())
                && candidate.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                normalized.push_str("($COMMIT)");
                rest = &after_open[end + 1..];
                continue;
            }
        }
        normalized.push('(');
        rest = after_open;
    }
    normalized.push_str(rest);
    normalized
}

fn write(path: impl AsRef<Path>, contents: &str) {
    fs::write(path, contents).expect("file should be written");
}
