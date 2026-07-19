use std::process::Command;

fn main() {
    watch_git_path("HEAD");
    if let Some(reference) = git_output(&["symbolic-ref", "--quiet", "HEAD"]) {
        watch_git_path(&reference);
    }

    let commit =
        git_output(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=JOLT_COMMIT_SHORT={commit}");
}

fn watch_git_path(path: &str) {
    if let Some(path) = git_output(&["rev-parse", "--git-path", path]) {
        println!("cargo:rerun-if-changed={path}");
    }
}

fn git_output(arguments: &[&str]) -> Option<String> {
    Command::new("git")
        .args(arguments)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|output| output.trim().to_owned())
        .filter(|output| !output.is_empty())
}
