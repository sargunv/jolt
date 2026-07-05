use std::{fs, path::Path, process::Output};

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin;
use tempfile::TempDir;

const SIMPLE_INPUT: &str = "class A {}\n";
const SIMPLE_FORMATTED: &str = "class A {\n}\n";
const NESTED_INPUT: &str = "class A { void f(){ if (true) { System.out.println(1); } } }\n";

#[test]
fn stdin_formats_to_stdout() {
    let temp = TempDir::new().expect("tempdir should be created");
    let output = jolt(temp.path(), ["fmt", "-"], SIMPLE_INPUT);

    assert_success(&output);
    assert_eq!(stdout(&output), SIMPLE_FORMATTED);
}

#[test]
fn stdin_filename_is_used_for_diagnostics() {
    let temp = TempDir::new().expect("tempdir should be created");
    let output = jolt(
        temp.path(),
        ["fmt", "--stdin-filename", "src/Main.java", "-"],
        "class {\n",
    );

    assert_failure(&output);
    assert!(stderr(&output).contains("src/Main.java:1:7"));
}

#[test]
fn write_mode_rewrites_changed_java_files() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("A.java"), SIMPLE_INPUT);

    let output = jolt(temp.path(), ["fmt", "A.java"], "");

    assert_success(&output);
    assert_eq!(read(temp.path().join("A.java")), SIMPLE_FORMATTED);
}

#[cfg(unix)]
#[test]
fn write_mode_in_place_rewrite_preserves_unix_mode_bits() {
    use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};

    let temp = TempDir::new().expect("tempdir should be created");
    let path = temp.path().join("A.java");
    write(&path, SIMPLE_INPUT);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o754)).expect("mode should be set");
    let before = fs::metadata(&path).expect("metadata should be readable");

    let output = jolt(temp.path(), ["fmt", "A.java"], "");

    assert_success(&output);
    assert_eq!(read(&path), SIMPLE_FORMATTED);
    let after = fs::metadata(&path).expect("metadata should be readable");
    assert_eq!(after.mode() & 0o777, 0o754);
    assert_eq!(after.ino(), before.ino());
}

#[test]
fn write_mode_formats_multiple_files_with_threads() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("A.java"), SIMPLE_INPUT);
    write(temp.path().join("B.java"), NESTED_INPUT);

    let output = jolt(temp.path(), ["fmt", "--threads", "2", "."], "");

    assert_success(&output);
    assert_eq!(read(temp.path().join("A.java")), SIMPLE_FORMATTED);
    assert_nested_uses_two_spaces(&read(temp.path().join("B.java")));
}

#[test]
fn write_mode_leaves_unchanged_java_files_untouched() {
    let temp = TempDir::new().expect("tempdir should be created");
    let path = temp.path().join("A.java");
    write(&path, SIMPLE_FORMATTED);
    let mut permissions = fs::metadata(&path)
        .expect("metadata should be readable")
        .permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&path, permissions).expect("file should be made readonly");

    let output = jolt(temp.path(), ["fmt", "A.java"], "");

    assert_success(&output);
    assert_eq!(read(path), SIMPLE_FORMATTED);
}

#[test]
fn check_mode_succeeds_when_files_are_formatted() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("A.java"), SIMPLE_FORMATTED);

    let output = jolt(temp.path(), ["fmt", "--check", "."], "");

    assert_success(&output);
    assert_eq!(stdout(&output), "");
}

#[test]
fn check_mode_fails_when_files_would_change_without_writing() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("A.java"), SIMPLE_INPUT);

    let output = jolt(temp.path(), ["fmt", "--check", "."], "");

    assert_failure(&output);
    assert_eq!(stdout(&output), "A.java\n");
    assert_eq!(read(temp.path().join("A.java")), SIMPLE_INPUT);
}

#[test]
fn check_mode_with_threads_prints_changed_paths_in_order_without_writing() {
    let temp = TempDir::new().expect("tempdir should be created");
    fs::create_dir_all(temp.path().join("src")).expect("src dir should be created");
    write(temp.path().join("src/B.java"), SIMPLE_INPUT);
    write(temp.path().join("src/A.java"), SIMPLE_INPUT);

    let output = jolt(temp.path(), ["fmt", "--check", "--threads", "2", "."], "");

    assert_failure(&output);
    assert_eq!(stdout(&output), "src/A.java\nsrc/B.java\n");
    assert_eq!(read(temp.path().join("src/A.java")), SIMPLE_INPUT);
    assert_eq!(read(temp.path().join("src/B.java")), SIMPLE_INPUT);
}

#[test]
fn parse_errors_do_not_write_files() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("A.java"), "class {\n");

    let output = jolt(temp.path(), ["fmt", "A.java"], "");

    assert_failure(&output);
    assert!(stderr(&output).contains("A.java:1:7"));
    assert_eq!(read(temp.path().join("A.java")), "class {\n");
}

#[test]
fn parse_errors_with_threads_do_not_stop_other_files() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("Bad.java"), "class {\n");
    write(temp.path().join("Good.java"), SIMPLE_INPUT);

    let output = jolt(temp.path(), ["fmt", "--threads", "2", "."], "");

    assert_failure(&output);
    assert!(stderr(&output).contains("Bad.java:1:7"));
    assert_eq!(read(temp.path().join("Bad.java")), "class {\n");
    assert_simple_formatted(&read(temp.path().join("Good.java")));
}

#[test]
fn cli_format_options_reach_core() {
    let temp = TempDir::new().expect("tempdir should be created");

    let line_width = jolt(
        temp.path(),
        ["fmt", "--line-width", "30", "-"],
        "class A { String s = first.second.third.fourth.fifth.sixth; }\n",
    );
    assert_success(&line_width);
    assert!(stdout(&line_width).contains("String s =\n    first.second"));

    let indent_width = jolt(
        temp.path(),
        ["fmt", "--indent-width", "4", "-"],
        NESTED_INPUT,
    );
    assert_success(&indent_width);
    assert_nested_uses_four_spaces(&stdout(&indent_width));

    let tabs = jolt(temp.path(), ["fmt", "--tabs", "-"], NESTED_INPUT);
    assert_success(&tabs);
    assert_nested_uses_tabs(&stdout(&tabs));

    write(temp.path().join("jolt.toml"), "[format]\ntabs = true\n");
    let spaces = jolt(temp.path(), ["fmt", "--spaces", "-"], NESTED_INPUT);
    assert_success(&spaces);
    assert_nested_uses_two_spaces(&stdout(&spaces));
}

#[test]
fn config_options_apply_and_cli_options_override_them() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(
        temp.path().join("jolt.toml"),
        "[format]\nindent-width = 4\n",
    );

    let configured = jolt(temp.path(), ["fmt", "-"], NESTED_INPUT);
    assert_success(&configured);
    assert_nested_uses_four_spaces(&stdout(&configured));

    let overridden = jolt(
        temp.path(),
        ["fmt", "--indent-width", "2", "-"],
        NESTED_INPUT,
    );
    assert_success(&overridden);
    assert_nested_uses_two_spaces(&stdout(&overridden));
}

#[test]
fn nested_configs_override_parent_configs() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(
        temp.path().join("jolt.toml"),
        "[format]\nindent-width = 4\n",
    );
    fs::create_dir_all(temp.path().join("module")).expect("module dir should be created");
    write(
        temp.path().join("module/jolt.toml"),
        "[format]\nindent-width = 2\n",
    );
    write(temp.path().join("Root.java"), NESTED_INPUT);
    write(temp.path().join("module/Module.java"), NESTED_INPUT);

    let output = jolt(temp.path(), ["fmt", "."], "");

    assert_success(&output);
    assert_nested_uses_four_spaces(&read(temp.path().join("Root.java")));
    assert_nested_uses_two_spaces(&read(temp.path().join("module/Module.java")));
}

#[test]
fn nested_configs_apply_with_threads() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(
        temp.path().join("jolt.toml"),
        "[format]\nindent-width = 4\n",
    );
    fs::create_dir_all(temp.path().join("module")).expect("module dir should be created");
    write(
        temp.path().join("module/jolt.toml"),
        "[format]\nindent-width = 2\n",
    );
    write(temp.path().join("Root.java"), NESTED_INPUT);
    write(temp.path().join("module/Module.java"), NESTED_INPUT);

    let output = jolt(temp.path(), ["fmt", "--threads", "2", "."], "");

    assert_success(&output);
    assert_nested_uses_four_spaces(&read(temp.path().join("Root.java")));
    assert_nested_uses_two_spaces(&read(temp.path().join("module/Module.java")));
}

#[test]
fn child_invocations_discover_configs_up_to_vcs_root() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("jolt.toml"), "[format]\ntabs = true\n");
    fs::create_dir_all(temp.path().join("project/.git")).expect("git dir should be created");
    fs::create_dir_all(temp.path().join("project/module")).expect("module dir should be created");
    write(
        temp.path().join("project/jolt.toml"),
        "[format]\nindent-width = 4\n",
    );

    let output = jolt(
        &temp.path().join("project/module"),
        ["fmt", "-"],
        NESTED_INPUT,
    );

    assert_success(&output);
    assert_nested_uses_four_spaces(&stdout(&output));
}

#[test]
fn root_true_config_defines_project_boundary_without_vcs_marker() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("jolt.toml"), "[format]\ntabs = true\n");
    fs::create_dir_all(temp.path().join("project/module")).expect("module dir should be created");
    write(
        temp.path().join("project/jolt.toml"),
        "root = true\n[format]\nindent-width = 4\n",
    );

    let output = jolt(
        &temp.path().join("project/module"),
        ["fmt", "-"],
        NESTED_INPUT,
    );

    assert_success(&output);
    assert_nested_uses_four_spaces(&stdout(&output));

    let dot_config = TempDir::new().expect("tempdir should be created");
    write(
        dot_config.path().join("jolt.toml"),
        "[format]\ntabs = true\n",
    );
    fs::create_dir_all(dot_config.path().join("project/.config/jolt"))
        .expect("config dir should be created");
    fs::create_dir_all(dot_config.path().join("project/module"))
        .expect("module dir should be created");
    write(
        dot_config.path().join("project/.config/jolt/config.toml"),
        "root = true\n[format]\nindent-width = 4\n",
    );

    let output = jolt(
        &dot_config.path().join("project/module"),
        ["fmt", "-"],
        NESTED_INPUT,
    );

    assert_success(&output);
    assert_nested_uses_four_spaces(&stdout(&output));
}

#[test]
fn root_and_dot_config_locations_are_discovered() {
    let root_config = TempDir::new().expect("tempdir should be created");
    write(
        root_config.path().join("jolt.toml"),
        "[format]\nindent-width = 4\n",
    );
    let output = jolt(root_config.path(), ["fmt", "-"], NESTED_INPUT);
    assert_success(&output);
    assert_nested_uses_four_spaces(&stdout(&output));

    let dot_config = TempDir::new().expect("tempdir should be created");
    fs::create_dir_all(dot_config.path().join(".config")).expect("config dir should be created");
    write(
        dot_config.path().join(".config/jolt.toml"),
        "[format]\nindent-width = 4\n",
    );
    let output = jolt(dot_config.path(), ["fmt", "-"], NESTED_INPUT);
    assert_success(&output);
    assert_nested_uses_four_spaces(&stdout(&output));

    let xdg_config = TempDir::new().expect("tempdir should be created");
    fs::create_dir_all(xdg_config.path().join(".config/jolt"))
        .expect("config dir should be created");
    write(
        xdg_config.path().join(".config/jolt/config.toml"),
        "[format]\nindent-width = 4\n",
    );
    let output = jolt(xdg_config.path(), ["fmt", "-"], NESTED_INPUT);
    assert_success(&output);
    assert_nested_uses_four_spaces(&stdout(&output));

    write(
        xdg_config.path().join("jolt.toml"),
        "[format]\nindent-width = 2\n",
    );
    let output = jolt(xdg_config.path(), ["fmt", "-"], NESTED_INPUT);
    assert_success(&output);
    assert_nested_uses_two_spaces(&stdout(&output));
}

#[test]
fn config_file_errors_and_no_config_behavior() {
    let temp = TempDir::new().expect("tempdir should be created");

    let missing = jolt(
        temp.path(),
        ["fmt", "--config", "missing.toml", "-"],
        SIMPLE_INPUT,
    );
    assert_failure(&missing);
    assert!(stderr(&missing).contains("missing.toml"));

    write(
        temp.path().join("jolt.toml"),
        "[format]\nindent-width = 4\n",
    );
    let no_config = jolt(temp.path(), ["fmt", "--no-config", "-"], NESTED_INPUT);
    assert_success(&no_config);
    assert_nested_uses_two_spaces(&stdout(&no_config));

    write(temp.path().join("jolt.toml"), "[format]\nline-wdith = 80\n");
    let unknown_key = jolt(temp.path(), ["fmt", "-"], SIMPLE_INPUT);
    assert_failure(&unknown_key);
    assert!(stderr(&unknown_key).contains("unknown field"));

    write(
        temp.path().join("jolt.toml"),
        "[format]\nindent-width = 0\n",
    );
    let invalid_range = jolt(temp.path(), ["fmt", "-"], SIMPLE_INPUT);
    assert_failure(&invalid_range);
    assert!(stderr(&invalid_range).contains("indent-width must be greater than zero"));

    write(
        temp.path().join("jolt.toml"),
        "[format]\ninclude = [\"[\"]\n",
    );
    let invalid_glob = jolt(temp.path(), ["fmt", "-"], SIMPLE_INPUT);
    assert_failure(&invalid_glob);
    assert!(stderr(&invalid_glob).contains("jolt.toml: invalid glob pattern"));
}

#[test]
fn threads_zero_fails_argument_parsing() {
    let temp = TempDir::new().expect("tempdir should be created");

    let output = jolt(temp.path(), ["fmt", "--threads", "0", "-"], SIMPLE_INPUT);

    assert_failure(&output);
    assert!(stderr(&output).contains("--threads"));
}

#[test]
fn stdin_formatting_accepts_threads_but_stays_serial() {
    let temp = TempDir::new().expect("tempdir should be created");

    let output = jolt(temp.path(), ["fmt", "--threads", "2", "-"], SIMPLE_INPUT);

    assert_success(&output);
    assert_simple_formatted(&stdout(&output));
}

#[test]
fn include_replacement_selects_candidates() {
    let temp = TempDir::new().expect("tempdir should be created");
    fs::create_dir_all(temp.path().join("selected")).expect("selected dir should be created");
    fs::create_dir_all(temp.path().join("other")).expect("other dir should be created");
    write(
        temp.path().join("jolt.toml"),
        "[format]\ninclude = [\"selected/**/*.java\"]\n",
    );
    write(temp.path().join("selected/A.java"), SIMPLE_INPUT);
    write(temp.path().join("other/B.java"), SIMPLE_INPUT);

    let configured = jolt(temp.path(), ["fmt", "."], "");
    assert_success(&configured);
    assert_simple_formatted(&read(temp.path().join("selected/A.java")));
    assert_eq!(read(temp.path().join("other/B.java")), SIMPLE_INPUT);

    let cli_replacement = jolt(
        temp.path(),
        ["fmt", "--include", "other/**/*.java", "."],
        "",
    );
    assert_success(&cli_replacement);
    assert_simple_formatted(&read(temp.path().join("other/B.java")));
}

#[test]
fn exclude_patterns_stack_across_config_and_cli() {
    let temp = TempDir::new().expect("tempdir should be created");
    fs::create_dir_all(temp.path().join("src")).expect("src dir should be created");
    fs::create_dir_all(temp.path().join("generated")).expect("generated dir should be created");
    write(
        temp.path().join("jolt.toml"),
        "[format]\nexclude = [\"generated/**\"]\n",
    );
    write(temp.path().join("src/A.java"), SIMPLE_INPUT);
    write(temp.path().join("src/Internal.java"), SIMPLE_INPUT);
    write(temp.path().join("generated/B.java"), SIMPLE_INPUT);

    let output = jolt(
        temp.path(),
        ["fmt", "--exclude", "src/Internal.java", "."],
        "",
    );

    assert_success(&output);
    assert_simple_formatted(&read(temp.path().join("src/A.java")));
    assert_eq!(read(temp.path().join("src/Internal.java")), SIMPLE_INPUT);
    assert_eq!(read(temp.path().join("generated/B.java")), SIMPLE_INPUT);
}

#[test]
fn gitignore_and_ignore_files_are_respected() {
    let temp = TempDir::new().expect("tempdir should be created");
    fs::create_dir_all(temp.path().join("gitignored")).expect("ignored dir should be created");
    fs::create_dir_all(temp.path().join("ignored")).expect("ignored dir should be created");
    write(temp.path().join(".gitignore"), "gitignored/\n");
    write(temp.path().join(".ignore"), "ignored/\n");
    write(temp.path().join("A.java"), SIMPLE_INPUT);
    write(temp.path().join("gitignored/B.java"), SIMPLE_INPUT);
    write(temp.path().join("ignored/C.java"), SIMPLE_INPUT);

    let output = jolt(temp.path(), ["fmt", "."], "");

    assert_success(&output);
    assert_simple_formatted(&read(temp.path().join("A.java")));
    assert_eq!(read(temp.path().join("gitignored/B.java")), SIMPLE_INPUT);
    assert_eq!(read(temp.path().join("ignored/C.java")), SIMPLE_INPUT);
}

#[test]
fn unknown_extensions_are_formatted_when_explicit_and_ignored_when_recursive() {
    let temp = TempDir::new().expect("tempdir should be created");
    write(temp.path().join("README.md"), SIMPLE_INPUT);
    write(temp.path().join("A.java"), SIMPLE_INPUT);

    let explicit = jolt(temp.path(), ["fmt", "README.md"], "");
    assert_success(&explicit);
    assert_simple_formatted(&read(temp.path().join("README.md")));

    write(temp.path().join("README.md"), SIMPLE_INPUT);

    let recursive = jolt(temp.path(), ["fmt", "."], "");
    assert_success(&recursive);
    assert_eq!(read(temp.path().join("README.md")), SIMPLE_INPUT);
    assert_simple_formatted(&read(temp.path().join("A.java")));
}

fn jolt<const N: usize>(dir: &Path, args: [&str; N], stdin: &str) -> Output {
    let mut command = Command::new(cargo_bin("jolt"));
    command.current_dir(dir).args(args);
    if !stdin.is_empty() {
        command.write_stdin(stdin);
    }
    command.output().expect("jolt command should run")
}

fn write(path: impl AsRef<Path>, contents: &str) {
    fs::write(path, contents).expect("file should be written");
}

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).expect("file should be read")
}

fn assert_simple_formatted(contents: &str) {
    assert_ne!(contents, SIMPLE_INPUT);
    assert!(contents.contains("class A"));
    assert!(contents.ends_with('\n'));
}

fn assert_nested_uses_two_spaces(contents: &str) {
    assert!(contents.contains("\n  void f()"));
    assert!(contents.contains("\n    if (true)"));
}

fn assert_nested_uses_four_spaces(contents: &str) {
    assert!(contents.contains("\n    void f()"));
    assert!(contents.contains("\n        if (true)"));
}

fn assert_nested_uses_tabs(contents: &str) {
    assert!(contents.contains("\n\tvoid f()"));
    assert!(contents.contains("\n\t\tif (true)"));
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be UTF-8")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success\nstdout:\n{}\nstderr:\n{}",
        stdout(output),
        stderr(output)
    );
}

fn assert_failure(output: &Output) {
    assert!(
        !output.status.success(),
        "expected failure\nstdout:\n{}\nstderr:\n{}",
        stdout(output),
        stderr(output)
    );
}
