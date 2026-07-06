use std::{
    convert::Infallible,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

use jolt_fmt_ir::{RenderControl, RenderSink};
use jolt_formatter::{FormatOptions, FormatSinkResult, Language, format_source_to_sink};
use tempfile::TempDir;

static PLUGIN_PATH: OnceLock<PathBuf> = OnceLock::new();

#[test]
fn dprint_fmt_loads_local_wasm_plugin_and_formats_java() {
    let project = DprintProject::new();
    project.write_config("");
    project.write_file("A.java", "class A {}");

    project.run_dprint(["fmt", "A.java"]).assert_success();

    assert_eq!(
        project.read_file("A.java"),
        direct_java_format("class A {}")
    );
}

#[test]
fn dprint_check_loads_local_wasm_plugin_and_fails_on_unformatted_java() {
    let project = DprintProject::new();
    project.write_config("");
    project.write_file("A.java", "class A {}");

    project.run_dprint(["check", "A.java"]).assert_failure();

    assert_eq!(project.read_file("A.java"), "class A {}");
}

#[test]
fn dprint_configuration_influences_line_width_indent_width_and_tabs() {
    let project = DprintProject::new();
    project.write_config(r#""jolt": { "lineWidth": 35, "indentWidth": 4, "useTabs": false }"#);
    project.write_file(
        "A.java",
        "class A { void call() { target.alpha(beta, gamma, delta); } }",
    );

    project.run_dprint(["fmt", "A.java"]).assert_success();
    let spaces_output = project.read_file("A.java");

    assert!(spaces_output.contains("    void call()"));
    assert!(spaces_output.contains("target.alpha(\n"));

    project.write_config(r#""jolt": { "lineWidth": 35, "indentWidth": 4, "useTabs": true }"#);
    project.write_file(
        "A.java",
        "class A { void call() { target.alpha(beta, gamma, delta); } }",
    );

    project.run_dprint(["fmt", "A.java"]).assert_success();
    let tabs_output = project.read_file("A.java");

    assert!(tabs_output.contains("\tvoid call()"));
    assert_ne!(spaces_output, tabs_output);
}

#[test]
fn dprint_smoke_formats_a_committed_fixture_input() {
    let fixture = repo_root().join("fixtures/java/style/declarations/body-blank-lines.java");
    assert!(
        fixture.is_file(),
        "required committed fixture is missing: {}",
        fixture.display()
    );

    let project = DprintProject::new();
    project.write_config("");
    fs::copy(&fixture, project.path("Fixture.java")).unwrap_or_else(|error| {
        panic!(
            "failed to copy required fixture {}: {error}",
            fixture.display()
        )
    });
    let before = project.read_file("Fixture.java");

    project.run_dprint(["fmt", "Fixture.java"]).assert_success();

    assert_ne!(project.read_file("Fixture.java"), before);
}

struct DprintProject {
    temp_dir: TempDir,
}

impl DprintProject {
    fn new() -> Self {
        Self {
            temp_dir: TempDir::new().expect("failed to create temp dir"),
        }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.temp_dir.path().join(relative)
    }

    fn write_config(&self, jolt_config: &str) {
        let plugin_path = local_wasm_plugin();
        let comma = if jolt_config.trim().is_empty() {
            ""
        } else {
            ","
        };
        let contents = format!(
            r#"{{
  "plugins": ["{}"]{}{jolt_config}
}}
"#,
            plugin_path.display(),
            comma
        );
        fs::write(self.path("dprint.json"), contents).expect("failed to write dprint config");
    }

    fn write_file(&self, relative: &str, contents: &str) {
        fs::write(self.path(relative), contents).expect("failed to write source file");
    }

    fn read_file(&self, relative: &str) -> String {
        fs::read_to_string(self.path(relative)).expect("failed to read source file")
    }

    fn run_dprint<const N: usize>(&self, args: [&str; N]) -> CommandOutput {
        let output = Command::new("dprint")
            .args(args)
            .current_dir(self.temp_dir.path())
            .output()
            .expect("failed to run required dprint binary");
        CommandOutput { output }
    }
}

struct CommandOutput {
    output: std::process::Output,
}

fn direct_java_format(source: &str) -> String {
    let mut sink = StringSink::default();
    match format_source_to_sink(source, Language::Java, &FormatOptions::default(), &mut sink) {
        FormatSinkResult::Complete | FormatSinkResult::Halted => sink.text,
        FormatSinkResult::Blocked { diagnostics } => {
            panic!("direct Java formatting blocked: {diagnostics:?}")
        }
        FormatSinkResult::SinkError { error } => match error {},
    }
}

#[derive(Default)]
struct StringSink {
    text: String,
}

impl RenderSink for StringSink {
    type Error = Infallible;

    fn write_str(&mut self, text: &str) -> Result<RenderControl, Self::Error> {
        self.text.push_str(text);
        Ok(RenderControl::Continue)
    }
}

impl CommandOutput {
    fn assert_success(&self) {
        assert!(
            self.output.status.success(),
            "command failed\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            self.output.status,
            String::from_utf8_lossy(&self.output.stdout),
            String::from_utf8_lossy(&self.output.stderr)
        );
    }

    fn assert_failure(&self) {
        assert!(
            !self.output.status.success(),
            "command unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&self.output.stdout),
            String::from_utf8_lossy(&self.output.stderr)
        );
    }
}

fn local_wasm_plugin() -> &'static Path {
    PLUGIN_PATH.get_or_init(|| {
        let status = Command::new("cargo")
            .args([
                "build",
                "--target",
                "wasm32-unknown-unknown",
                "--package",
                "jolt_fmt_dprint",
                "--features",
                "wasm",
            ])
            .current_dir(repo_root())
            .status()
            .expect("failed to run cargo to build required dprint wasm plugin");
        assert!(
            status.success(),
            "failed to build required dprint wasm plugin"
        );

        let plugin_path =
            repo_root().join("target/wasm32-unknown-unknown/debug/jolt_fmt_dprint.wasm");
        assert!(
            plugin_path.is_file(),
            "required dprint wasm plugin is missing after build: {}",
            plugin_path.display()
        );
        plugin_path
    })
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crate should live below the repository root")
        .to_path_buf()
}
