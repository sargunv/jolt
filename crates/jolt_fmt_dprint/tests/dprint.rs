//! Runs the plugin in the real dprint CLI.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::OnceLock,
};

use tempfile::TempDir;

const UNFORMATTED: &str = "class A { void call() { target.alpha(beta, gamma, delta); } }";

#[test]
fn fmt_formats_and_check_fails_on_unformatted_files() {
    let project = Project::new("");
    let check = project.dprint("check");
    assert!(!check.status.success());
    assert_eq!(project.read(), UNFORMATTED);

    let fmt = project.dprint("fmt");
    assert!(
        fmt.status.success(),
        "{}",
        String::from_utf8_lossy(&fmt.stderr)
    );
    assert!(project.read().contains("\n  void call() {\n"));
}

#[test]
fn global_and_plugin_options_apply() {
    let project = Project::new(r#""lineWidth": 35, "indentWidth": 4, "jolt": { "useTabs": true }"#);
    assert!(project.dprint("fmt").status.success());
    assert!(
        project
            .read()
            .contains("\n\tvoid call() {\n\t\ttarget.alpha(\n")
    );

    let project =
        Project::new(r#""useTabs": true, "jolt": { "useTabs": false, "indentWidth": 4 }"#);
    assert!(project.dprint("fmt").status.success());
    assert!(project.read().contains("\n    void call() {\n"));
}

#[test]
fn syntax_errors_fail_without_writing() {
    let project = Project::new("");
    fs::write(project.file(), "class {\n").unwrap();
    let output = project.dprint("fmt");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("1:7: java.parse.expected_syntax"));
    assert_eq!(project.read(), "class {\n");
}

struct Project(TempDir);

impl Project {
    fn new(config: &str) -> Self {
        let dir = TempDir::new().unwrap();
        let separator = if config.is_empty() { "" } else { ", " };
        let config = format!(
            r#"{{ "plugins": ["{}"]{separator}{config} }}"#,
            plugin().display()
        );
        fs::write(dir.path().join("dprint.json"), config).unwrap();
        fs::write(dir.path().join("A.java"), UNFORMATTED).unwrap();
        Self(dir)
    }

    fn file(&self) -> PathBuf {
        self.0.path().join("A.java")
    }

    fn read(&self) -> String {
        fs::read_to_string(self.file()).unwrap()
    }

    fn dprint(&self, command: &str) -> Output {
        Command::new("dprint")
            .args([command, "A.java"])
            .current_dir(self.0.path())
            .output()
            .expect("the dprint CLI is required; run `mise install`")
    }
}

/// Builds the debug plugin once per test run.
fn plugin() -> &'static Path {
    static PLUGIN: OnceLock<PathBuf> = OnceLock::new();
    PLUGIN.get_or_init(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let status = Command::new(env!("CARGO"))
            .args([
                "build",
                "--target",
                "wasm32-unknown-unknown",
                "-p",
                "jolt_fmt_dprint",
            ])
            .current_dir(&root)
            .status()
            .expect("cargo runs");
        assert!(status.success(), "building the plugin failed");
        root.join("target/wasm32-unknown-unknown/debug/jolt_fmt_dprint.wasm")
    })
}
