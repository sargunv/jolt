use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const IMPLEMENTATION_BASELINE: &str = "2197128";
const MAX_IMPLEMENTATION_NET_DELTA: usize = 8_489;

#[test]
fn forbidden_architecture_patterns_do_not_regress() {
    let workspace = workspace_root();
    let production = production_rust(&workspace);
    let mut failures = Vec::new();

    for path in &production {
        let source = read(path);
        let relative = relative(&workspace, path);
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

        if relative.starts_with("crates/jolt_java_syntax/src/parser/")
            || relative.starts_with("crates/jolt_kotlin_syntax/src/parser/")
        {
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

/// Enforces the roadmap's formal implementation-size projection. The explicit
/// pathspec includes production, tests, test support, and benchmark/import
/// tooling while excluding fixtures, snapshots, reports, and documentation by
/// construction. Untracked implementation files are added to the projection so
/// a local `mise run test` cannot evade the gate before staging them.
#[test]
fn implementation_projection_stays_within_phase_twenty_eight_budget() {
    let workspace = workspace_root();
    let (additions, deletions) = implementation_projection(&workspace);
    let net = additions.saturating_sub(deletions);

    assert!(
        net <= MAX_IMPLEMENTATION_NET_DELTA,
        "Phase 28 implementation projection against {IMPLEMENTATION_BASELINE} is \
         +{additions}/-{deletions}, net +{net}; maximum net delta is \
         +{MAX_IMPLEMENTATION_NET_DELTA}. The projection includes crates/**/*.rs and \
         tools/**/*.py, including tests and test support."
    );
}

fn implementation_projection(workspace: &Path) -> (usize, usize) {
    let output = Command::new("git")
        .current_dir(workspace)
        .args([
            "diff",
            "--numstat",
            IMPLEMENTATION_BASELINE,
            "--",
            ":(glob)crates/**/*.rs",
            ":(glob)tools/**/*.py",
        ])
        .output()
        .expect("architecture size gate requires git");
    assert!(
        output.status.success(),
        "git diff failed for architecture size gate: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let mut additions = 0;
    let mut deletions = 0;
    for line in String::from_utf8(output.stdout)
        .expect("git numstat must be UTF-8")
        .lines()
    {
        let mut fields = line.split('\t');
        additions += fields
            .next()
            .expect("numstat addition field")
            .parse::<usize>()
            .expect("implementation files must have textual numstat additions");
        deletions += fields
            .next()
            .expect("numstat deletion field")
            .parse::<usize>()
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
        .filter(|path| path.ends_with(".rs") || path.ends_with(".py"))
    {
        additions += read(&workspace.join(relative)).lines().count();
    }

    (additions, deletions)
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
