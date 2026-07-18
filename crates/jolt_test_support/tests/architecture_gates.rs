use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const IMPLEMENTATION_BASELINE: &str = "2197128";
const IMPLEMENTATION_PATHS: &[&str] = &[":(glob)crates/**/*.rs", ":(glob)tools/**/*.py"];
const MACRO_SCHEMA_PATHS: &[&str] = &[
    "crates/jolt_java_syntax/src/schema.rs",
    "crates/jolt_kotlin_syntax/src/schema.rs",
    "crates/jolt_syntax/src/projection.rs",
    "crates/jolt_syntax/src/schema.rs",
];
const GENERATED_CONSUMER_PATHS: &[&str] = &[
    "crates/jolt_java_syntax/src/kind.rs",
    "crates/jolt_java_syntax/src/nodes/mod.rs",
    "crates/jolt_java_syntax/src/shape.rs",
    "crates/jolt_kotlin_syntax/src/kind.rs",
    "crates/jolt_kotlin_syntax/src/nodes/mod.rs",
    "crates/jolt_kotlin_syntax/src/shape.rs",
];
const AUDIT_PROOF_PATHS: &[&str] = &[
    "crates/jolt_fmt_ir/src/document.rs",
    "crates/jolt_fmt_ir/src/formatter_ignore.rs",
    "crates/jolt_fmt_ir/src/render.rs",
    "crates/jolt_fmt_ir/src/source_fragment.rs",
    "crates/jolt_java_fmt/tests/normalization_authority.rs",
    "crates/jolt_java_syntax/src/normalization.rs",
    "crates/jolt_java_syntax/src/schema_audit.rs",
    "crates/jolt_java_syntax/tests/normalization.rs",
    "crates/jolt_kotlin_fmt/tests/normalization_authority.rs",
    "crates/jolt_kotlin_syntax/src/normalization.rs",
    "crates/jolt_kotlin_syntax/src/schema_audit.rs",
    "crates/jolt_kotlin_syntax/tests/normalization.rs",
    "crates/jolt_syntax/src/conservation.rs",
    "crates/jolt_syntax/src/normalization.rs",
    "crates/jolt_test_support/src/diagnostic_ownership.rs",
    "crates/jolt_test_support/src/schema_audit.rs",
    "crates/jolt_test_support/tests/architecture_gates.rs",
];
const MAX_MACRO_SCHEMA_NET_DELTA: isize = 3_490;
const MAX_GENERATED_CONSUMER_NET_DELTA: isize = -24;
const MAX_AUDIT_PROOF_NET_DELTA: isize = 5_768;

#[test]
fn forbidden_architecture_patterns_do_not_regress() {
    let workspace = workspace_root();
    let production = production_rust(&workspace);
    let mut failures = Vec::new();

    for path in &production {
        scan_production_file(&workspace, path, &mut failures);
    }

    let audit = read(&workspace.join("crates/jolt_test_support/src/schema_audit.rs"));
    for pattern in ["RawChildren", "pub fn new_raw"] {
        if audit.contains(pattern) {
            failures.push(format!(
                "crates/jolt_test_support/src/schema_audit.rs: deleted raw-schema carrier \
                 {pattern:?} was reintroduced"
            ));
        }
    }

    let formatter_ignore = read(&workspace.join("crates/jolt_fmt_ir/src/formatter_ignore.rs"));
    for pattern in [
        "Vec<SourceIdentity",
        "populate_claims",
        "range_separators(",
        "source_document(",
    ] {
        if formatter_ignore.contains(pattern) {
            failures.push(format!(
                "crates/jolt_fmt_ir/src/formatter_ignore.rs: proof-carrying ignore ranges forbid \
                 manual bookkeeping {pattern:?}"
            ));
        }
    }

    let conservation = read(&workspace.join("crates/jolt_syntax/src/conservation.rs"));
    if conservation.contains("for index in 0..self.tree.token_count()") {
        failures.push(
            "crates/jolt_syntax/src/conservation.rs: source-range claims must visit only their \
             bounded token interval"
                .to_owned(),
        );
    }

    for relative in [
        "crates/jolt_java_fmt/src/format.rs",
        "crates/jolt_kotlin_fmt/src/format.rs",
    ] {
        let source = read(&workspace.join(relative));
        for pattern in ["RenderProof", "conservation_tracker()"] {
            if source.contains(pattern) {
                failures.push(format!(
                    "{relative}: source render entry points must not own proof bookkeeping \
                     {pattern:?}"
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "forbidden architecture patterns found:\n{}",
        failures.join("\n")
    );
}

fn scan_production_file(workspace: &Path, path: &Path, failures: &mut Vec<String>) {
    let source = read(path);
    let relative = relative(workspace, path);
    for forbidden in [
        "DiagnosticMarker",
        "FormatterInsertedToken",
        "ReferenceSyntaxFactory",
        "build_reference_syntax_tree",
        "build_syntax_tree_with_factory_and_diagnostic_owners",
        "completed_is_error_node",
        "directive_reorder_claim",
        "error_node_kind",
        "ErrorNode",
        "expect_owned(",
        "expected_here(",
        "expected_owned_",
        "own_diagnostic(",
        "modifier_reorder_claim",
        "represented_range_is_trivia",
        "tokens_between",
        "unexpected_here(",
        "unexpected_owned_",
        "canonical_reorder_claim().is_none()",
        "canonical_reorder_claim().is_some()",
        "claimed_source(",
        "claimed_trivia(",
        "render_to_tracked(",
        "rendered_fragments(",
        "RenderedSourceFragment",
        "let _authorization =",
    ] {
        if source.contains(forbidden) {
            failures.push(format!("{relative}: forbidden pattern {forbidden:?}"));
        }
    }

    if !(relative.starts_with("crates/jolt_java_syntax/src/parser/")
        || relative.starts_with("crates/jolt_kotlin_syntax/src/parser/"))
    {
        return;
    }
    for forbidden in ["UnresolvedDiagnosticOwner", "complete_owned_"] {
        if source.contains(forbidden) {
            failures.push(format!(
                "{relative}: language atomic recovery migration forbids {forbidden:?}"
            ));
        }
    }
    if relative.contains("/grammar/") {
        for forbidden in [
            "ensure_progress(",
            "error_here(",
            "expected_here(",
            "unexpected_here(",
            "self.expect(",
            "self.expect_contextual(",
            "self.expect_variable_identifier(",
            "self.expect_named_variable_identifier(",
            "self.consume_qualified_name(",
            "report_non_structural(",
        ] {
            if source.contains(forbidden) {
                failures.push(format!(
                    "{relative}: language grammar must classify structural diagnostics; \
                     forbidden ownerless path {forbidden:?}"
                ));
            }
        }
    }
}

/// Enforces the roadmap's formal implementation-size projection. The explicit
/// pathspec includes production, tests, test support, and benchmark/import
/// tooling while excluding fixtures, snapshots, reports, and documentation by
/// construction. Untracked implementation files are added to the projection so
/// a local `mise run test` cannot evade the gate before staging them.
#[test]
fn implementation_projection_has_bounded_architecture_and_negative_ordinary_code() {
    let workspace = workspace_root();
    let total = implementation_projection(&workspace, IMPLEMENTATION_PATHS);
    let macro_schema = implementation_projection(&workspace, MACRO_SCHEMA_PATHS);
    let generated_consumer = implementation_projection(&workspace, GENERATED_CONSUMER_PATHS);
    let audit_proof = implementation_projection(&workspace, AUDIT_PROOF_PATHS);
    let categorized = macro_schema + generated_consumer + audit_proof;
    let ordinary = total - categorized;

    assert!(
        macro_schema.net() <= MAX_MACRO_SCHEMA_NET_DELTA
            && generated_consumer.net() <= MAX_GENERATED_CONSUMER_NET_DELTA
            && audit_proof.net() <= MAX_AUDIT_PROOF_NET_DELTA
            && ordinary.net() < 0,
        "Phase 29 projection against {IMPLEMENTATION_BASELINE}: total {total}, \
         macro schema/projection {macro_schema} (maximum {MAX_MACRO_SCHEMA_NET_DELTA:+}), \
         generated consumers {generated_consumer} (maximum \
         {MAX_GENERATED_CONSUMER_NET_DELTA:+}), audit/proof {audit_proof} (maximum \
         {MAX_AUDIT_PROOF_NET_DELTA:+}), ordinary implementation {ordinary} (must be \
         negative). The projection includes crates/**/*.rs and tools/**/*.py, including \
         tests and test support."
    );
}

#[derive(Clone, Copy)]
struct Projection {
    additions: isize,
    deletions: isize,
}

impl Projection {
    const fn net(self) -> isize {
        self.additions - self.deletions
    }
}

impl std::fmt::Display for Projection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "+{}/-{}, net {:+}",
            self.additions,
            self.deletions,
            self.net()
        )
    }
}

impl std::ops::Add for Projection {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            additions: self.additions + other.additions,
            deletions: self.deletions + other.deletions,
        }
    }
}

impl std::ops::Sub for Projection {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            additions: self.additions - other.additions,
            deletions: self.deletions - other.deletions,
        }
    }
}

fn implementation_projection(workspace: &Path, paths: &[&str]) -> Projection {
    let mut arguments = vec!["diff", "--numstat", IMPLEMENTATION_BASELINE, "--"];
    arguments.extend_from_slice(paths);
    let output = Command::new("git")
        .current_dir(workspace)
        .args(arguments)
        .output()
        .expect("architecture size gate requires git");
    assert!(
        output.status.success(),
        "git diff failed for architecture size gate: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let mut additions = 0_isize;
    let mut deletions = 0_isize;
    for line in String::from_utf8(output.stdout)
        .expect("git numstat must be UTF-8")
        .lines()
    {
        let mut fields = line.split('\t');
        additions += fields
            .next()
            .expect("numstat addition field")
            .parse::<isize>()
            .expect("implementation files must have textual numstat additions");
        deletions += fields
            .next()
            .expect("numstat deletion field")
            .parse::<isize>()
            .expect("implementation files must have textual numstat deletions");
    }

    let untracked = Command::new("git")
        .current_dir(workspace)
        .args([
            "ls-files",
            "--others",
            "--exclude-standard",
            "--",
            "crates",
            "tools",
        ])
        .output()
        .expect("architecture size gate requires git");
    assert!(
        untracked.status.success(),
        "git ls-files failed for architecture size gate: {}",
        String::from_utf8_lossy(&untracked.stderr)
    );
    for relative in String::from_utf8(untracked.stdout)
        .expect("git file names must be UTF-8")
        .lines()
        .filter(|path| projection_includes_untracked(path, paths))
    {
        additions += isize::try_from(read(&workspace.join(relative)).lines().count())
            .expect("implementation line count fits isize");
    }

    Projection {
        additions,
        deletions,
    }
}

fn projection_includes_untracked(relative: &str, paths: &[&str]) -> bool {
    if paths == IMPLEMENTATION_PATHS {
        let extension = Path::new(relative).extension();
        return extension.is_some_and(|extension| extension == "rs" || extension == "py");
    }
    paths.contains(&relative)
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("test-support crate must be inside the workspace")
        .to_owned()
}

fn production_rust(workspace: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let crates = workspace.join("crates");
    for entry in fs::read_dir(&crates)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", crates.display()))
    {
        let crate_root = entry.expect("crate directory entry").path();
        let source = crate_root.join("src");
        if source.is_dir() {
            collect_rust(&source, &mut files);
        }
    }
    files.sort();
    files
}

fn collect_rust(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", directory.display()))
    {
        let path = entry.expect("source directory entry").path();
        if path.is_dir() {
            collect_rust(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

fn read(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn relative(workspace: &Path, path: &Path) -> String {
    path.strip_prefix(workspace)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
