//! Runs the `jolt` binary in temporary projects. Each snapshot shows the exit
//! status, the output, and every file the run created or changed.

use std::{
    collections::BTreeMap,
    fmt::Write as _,
    fs,
    io::Write as _,
    path::Path,
    process::{Command, Stdio},
};

use tempfile::TempDir;

/// Formats to different text with each indentation option.
const NESTED: &str = "class A { void f() { if (ready) { go(); } } }\n";
const FORMATTED: &str = "class A {\n  void f() {\n    go();\n  }\n}\n";

/// A temporary project: write files, run `jolt`, snapshot the result.
struct Project {
    dir: TempDir,
}

impl Project {
    fn new(files: &[(&str, &str)]) -> Self {
        let project = Self {
            dir: TempDir::new().unwrap(),
        };
        for (path, contents) in files {
            project.write(path, contents);
        }
        project
    }

    fn write(&self, path: &str, contents: &str) {
        let path = self.dir.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn files(&self) -> BTreeMap<String, String> {
        let mut files = BTreeMap::new();
        collect(self.dir.path(), self.dir.path(), &mut files);
        files
    }

    /// Runs `jolt` in `cwd` (relative to the project) and renders the result.
    fn run_in(&self, cwd: &str, args: &[&str], stdin: &str) -> String {
        let before = self.files();
        let mut child = Command::new(env!("CARGO_BIN_EXE_jolt"))
            .args(args)
            .current_dir(self.dir.path().join(cwd))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        // A run that fails before reading stdin closes the pipe early.
        let _ = child.stdin.take().unwrap().write_all(stdin.as_bytes());
        let output = child.wait_with_output().unwrap();

        let mut rendered = format!(
            "$ jolt {}\nstatus: {}\n",
            args.join(" "),
            output.status.code().unwrap_or(-1)
        );
        for (label, bytes) in [("stdout", &output.stdout), ("stderr", &output.stderr)] {
            if !bytes.is_empty() {
                let _ = write!(rendered, "--- {label}\n{}", String::from_utf8_lossy(bytes));
            }
        }
        for (path, contents) in self.files() {
            if before.get(&path) != Some(&contents) {
                let _ = write!(rendered, "--- wrote {path}\n{contents}");
            }
        }
        self.normalize(&rendered)
    }

    fn run(&self, args: &[&str], stdin: &str) -> String {
        self.run_in(".", args, stdin)
    }

    fn normalize(&self, text: &str) -> String {
        let mut text = text.to_owned();
        for dir in [
            fs::canonicalize(self.dir.path()).unwrap(),
            self.dir.path().to_path_buf(),
        ] {
            text = text.replace(&dir.display().to_string(), "$TEMP");
        }
        text.lines()
            .map(|line| match line.rsplit_once(" in ") {
                Some((summary, _))
                    if summary.starts_with("Reformatted ") || summary.starts_with("Checked ") =>
                {
                    format!("{summary} in $TIME\n")
                }
                _ => format!("{line}\n"),
            })
            .collect()
    }
}

fn collect(root: &Path, dir: &Path, files: &mut BTreeMap<String, String>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect(root, &path, files);
        } else {
            let name = path.strip_prefix(root).unwrap().display().to_string();
            files.insert(name, fs::read_to_string(&path).unwrap_or_default());
        }
    }
}

#[test]
fn write_mode() {
    let project = Project::new(&[
        ("A.java", NESTED),
        ("src/B.java", FORMATTED),
        ("README.md", NESTED),
    ]);
    insta::assert_snapshot!(project.run(&["fmt"], ""));
}

#[cfg(unix)]
#[test]
fn write_mode_rewrites_files_in_place() {
    use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};

    let project = Project::new(&[("A.java", NESTED)]);
    let path = project.dir.path().join("A.java");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o754)).unwrap();
    let before = fs::metadata(&path).unwrap();
    assert!(project.run(&["fmt", "A.java"], "").contains("status: 0"));
    let after = fs::metadata(&path).unwrap();
    assert_eq!(after.mode() & 0o777, 0o754);
    assert_eq!(after.ino(), before.ino());
}

#[test]
fn check_mode() {
    let project = Project::new(&[
        ("src/B.java", NESTED),
        ("src/A.java", NESTED),
        ("src/C.java", FORMATTED),
        ("Bad.java", "class {\n"),
    ]);
    insta::assert_snapshot!(project.run(&["fmt", "--check", "--threads", "2", "."], ""));
}

#[test]
fn check_mode_exits_1_for_unformatted_files_and_2_for_errors() {
    let project = Project::new(&[("A.java", NESTED), ("B.java", FORMATTED)]);
    insta::assert_snapshot!(project.run(&["fmt", "--check", "B.java"], ""));
    insta::assert_snapshot!(project.run(&["fmt", "--check"], ""));
    project.write("Bad.java", "class {\n");
    insta::assert_snapshot!(project.run(&["fmt", "--check"], ""));
}

#[test]
fn syntax_errors() {
    let project = Project::new(&[("A.java", "class {\n"), ("B.java", NESTED)]);
    insta::assert_snapshot!(project.run(&["fmt", "--threads", "2", "."], ""));
    insta::assert_snapshot!(project.run(&["fmt", "--format-with-errors", "A.java"], ""));
}

#[test]
fn stdin() {
    let project = Project::new(&[]);
    insta::assert_snapshot!(project.run(&["format", "-"], NESTED));
    insta::assert_snapshot!(project.run(
        &["fmt", "--check", "--stdin-filename", "src/Main.java", "-"],
        NESTED
    ));
    insta::assert_snapshot!(project.run(
        &["fmt", "--stdin-filename", "src/Main.java", "-"],
        "class {\n"
    ));
}

#[test]
fn path_errors() {
    let project = Project::new(&[("README.md", NESTED)]);
    insta::assert_snapshot!(project.run(&["fmt", "README.md"], ""));
    insta::assert_snapshot!(project.run(&["fmt", "Missing.java"], ""));
    insta::assert_snapshot!(project.run(&["fmt", "-", "README.md"], ""));
}

#[test]
fn format_flags_override_config() {
    let project = Project::new(&[("jolt.toml", "[format]\nindent-width = 4\nuse-tabs = true\n")]);
    insta::assert_snapshot!(project.run(&["fmt", "-"], NESTED));
    insta::assert_snapshot!(project.run(&["fmt", "--use-tabs=false", "-"], NESTED));
    insta::assert_snapshot!(project.run(
        &[
            "fmt",
            "--line-width",
            "20",
            "--indent-width",
            "3",
            "--use-tabs=false",
            "-"
        ],
        NESTED
    ));
}

#[test]
fn bare_use_tabs_means_true() {
    let project = Project::new(&[("A.java", NESTED)]);
    insta::assert_snapshot!(project.run(&["fmt", "--use-tabs", "A.java"], ""));
}

#[test]
fn nested_configs_layer_up_to_the_project_root() {
    let project = Project::new(&[
        ("jolt.toml", "[format]\nuse-tabs = true\n"),
        ("project/.git/HEAD", ""),
        ("project/jolt.toml", "[format]\nindent-width = 4\n"),
        ("project/Root.java", NESTED),
        (
            "project/module/.config/jolt.toml",
            "[format]\nindent-width = 3\n",
        ),
        ("project/module/Module.java", NESTED),
        ("project/module/lib/jolt.toml", "root = true\n"),
        ("project/module/lib/Lib.java", NESTED),
    ]);
    insta::assert_snapshot!(project.run_in("project/module", &["fmt", "--threads", "2", ".."], ""));
}

#[test]
fn without_a_project_root_targets_under_the_invocation_directory_read_its_config() {
    let project = Project::new(&[
        ("jolt.toml", "[format]\nindent-width = 4\n"),
        ("a/A.java", NESTED),
    ]);
    insta::assert_snapshot!(project.run(&["fmt", "-", "--stdin-filename", "a/C.java"], NESTED));
    insta::assert_snapshot!(project.run(&["config", "resolve", "a/A.java"], ""));
    insta::assert_snapshot!(project.run(&["fmt", "a/A.java"], ""));
}

#[test]
fn config_locations_in_one_directory() {
    let project = Project::new(&[
        (
            ".config/jolt/config.toml",
            "[format]\nindent-width = 4\nline-width = 20\n",
        ),
        (".config/jolt.toml", "[format]\nindent-width = 3\n"),
    ]);
    insta::assert_snapshot!(project.run(&["fmt", "-"], NESTED));
    project.write("jolt.toml", "[format]\nindent-width = 2\n");
    insta::assert_snapshot!(project.run(&["fmt", "-"], NESTED));
}

#[test]
fn explicit_and_disabled_config() {
    let project = Project::new(&[
        ("jolt.toml", "[format]\nindent-width = 4\n"),
        (
            "conf/explicit.toml",
            "[format]\nindent-width = 3\n[files]\nexclude = [\"B.java\"]\n",
        ),
        ("conf/B.java", NESTED),
        ("A.java", NESTED),
    ]);
    insta::assert_snapshot!(project.run(&["fmt", "--no-config", "-"], NESTED));
    insta::assert_snapshot!(project.run(&["fmt", "--config", "conf/explicit.toml", "."], ""));
    insta::assert_snapshot!(project.run(&["fmt", "--config", "missing.toml", "-"], NESTED));
    insta::assert_snapshot!(project.run(
        &["fmt", "--config", "conf/explicit.toml", "--no-config", "-"],
        NESTED
    ));
}

#[test]
fn config_errors() {
    let project = Project::new(&[]);
    for config in [
        "[format]\nline-wdith = 80\n",
        "[format]\nindent-width = 0\n",
        "[files]\ninclude = [\"[\"]\n",
    ] {
        project.write("jolt.toml", config);
        insta::assert_snapshot!(project.run(&["fmt", "-"], NESTED));
    }
}

#[test]
fn file_selection() {
    let project = Project::new(&[
        (
            "jolt.toml",
            "[files]\ninclude = [\"src/**/*.java\"]\nexclude = [\"src/gen/**\"]\n",
        ),
        ("src/A.java", NESTED),
        ("src/Internal.java", NESTED),
        ("src/gen/G.java", NESTED),
        ("src/ignored/I.java", NESTED),
        ("src/.ignore", "ignored/\n"),
        ("other/O.java", NESTED),
    ]);
    insta::assert_snapshot!(project.run(&["fmt", "--exclude", "src/Internal.java"], ""));
    insta::assert_snapshot!(project.run(&["fmt", "--include", "other/*.java", "--check"], ""));
}

/// Hooks pass changed files by name: the exclude globs still apply to them,
/// and the include globs don't.
#[test]
fn explicit_files_respect_excludes_but_not_includes() {
    let project = Project::new(&[
        (
            "jolt.toml",
            "[files]\ninclude = [\"src/**/*.java\"]\nexclude = [\"src/gen/**\"]\n",
        ),
        ("src/A.java", NESTED),
        ("src/gen/G.java", NESTED),
        ("other/O.java", NESTED),
    ]);
    insta::assert_snapshot!(project.run(
        &[
            "fmt",
            "--check",
            "src/A.java",
            "src/gen/G.java",
            "other/O.java"
        ],
        ""
    ));
    insta::assert_snapshot!(project.run(&["fmt", "--exclude", "other/**", "other/O.java"], ""));
}

#[test]
fn config_init() {
    let project = Project::new(&[]);
    insta::assert_snapshot!(project.run(&["config", "init"], ""));
    insta::assert_snapshot!(project.run(&["config", "init"], ""));
}

#[test]
fn config_list_and_resolve() {
    let project = Project::new(&[
        (
            "jolt.toml",
            "root = true\n[format]\nline-width = 100\n[files]\nexclude = [\"generated/**\"]\n",
        ),
        ("src/.config/jolt.toml", "[format]\nuse-tabs = true\n"),
        (
            "src/jolt.toml",
            "[files]\nexclude = [\"**/Internal.java\"]\n",
        ),
    ]);
    insta::assert_snapshot!(project.run(&["config", "list", "src/nested/A.java"], ""));
    insta::assert_snapshot!(project.run(&["config", "resolve", "src/nested/Internal.java"], ""));
    insta::assert_snapshot!(project.run_in("src", &["config", "resolve"], ""));
}

#[test]
fn config_schemas() {
    let project = Project::new(&[]);
    insta::assert_snapshot!(project.run(&["config", "schema"], ""));
    insta::assert_snapshot!(project.run(&["config", "schema", "--dprint"], ""));
}
