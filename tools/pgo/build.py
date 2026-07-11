#!/usr/bin/env python3
"""Build Jolt with profile-guided optimization using benchmark corpora."""

import os
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
IMPORTS = ROOT / "tools/import/.imports"
WORK = ROOT / "target/pgo"
PROFILES = WORK / "profiles"
INSTRUMENTED_TARGET = WORK / "instrumented"
OPTIMIZED_TARGET = WORK / "optimized"
TRAINING = WORK / "training"
MERGED_PROFILE = WORK / "jolt.profdata"
EXE_SUFFIX = ".exe" if os.name == "nt" else ""

# Keep this aligned with the corpora used by tools/bench/bench.py. The excluded
# input is intentionally invalid upstream Java.
CORPORA = {
    "adversarial": (IMPORTS / "google-java-format/input", {"B26952926.java"}),
    "realistic": (IMPORTS / "spring-framework", set()),
}


def main() -> None:
    reset_build_outputs()
    llvm_profdata = find_llvm_profdata()
    prepare_training_corpora()

    build(
        INSTRUMENTED_TARGET,
        f"-Cprofile-generate={PROFILES.as_posix()}",
    )
    instrumented_jolt = INSTRUMENTED_TARGET / f"release/jolt{EXE_SUFFIX}"
    require_file(instrumented_jolt, "instrumented Jolt CLI")

    # Cargo also instruments host build scripts through RUSTFLAGS. Discard any
    # profiles they emitted so compilation behavior cannot bias CLI training.
    shutil.rmtree(PROFILES)
    PROFILES.mkdir()

    training_paths = [TRAINING / name for name in CORPORA]
    run(
        instrumented_jolt,
        "fmt",
        *training_paths,
        env={"LLVM_PROFILE_FILE": (PROFILES / "jolt-%m-%p.profraw").as_posix()},
    )

    raw_profiles = sorted(PROFILES.glob("*.profraw"))
    if not raw_profiles:
        raise RuntimeError(f"training produced no raw profiles in {PROFILES}")
    run(llvm_profdata, "merge", "-o", MERGED_PROFILE, *raw_profiles)
    require_file(MERGED_PROFILE, "merged PGO profile")

    build(
        OPTIMIZED_TARGET,
        f"-Cprofile-use={MERGED_PROFILE.as_posix()}",
    )
    optimized_jolt = OPTIMIZED_TARGET / f"release/jolt{EXE_SUFFIX}"
    require_file(optimized_jolt, "PGO-optimized Jolt CLI")
    run(optimized_jolt, "--version")
    print(f"PGO-optimized CLI: {optimized_jolt}")


def find_llvm_profdata() -> Path:
    override = os.environ.get("LLVM_PROFDATA")
    if override:
        tool = Path(override)
        require_file(tool, "LLVM_PROFDATA")
        return tool

    sysroot = capture("rustc", "--print", "sysroot")
    host_line = next(
        (
            line
            for line in capture("rustc", "-vV").splitlines()
            if line.startswith("host: ")
        ),
        None,
    )
    if host_line is None:
        raise RuntimeError("rustc -vV did not report its host triple")
    tool = (
        Path(sysroot)
        / "lib/rustlib"
        / host_line.removeprefix("host: ")
        / f"bin/llvm-profdata{EXE_SUFFIX}"
    )
    if not tool.is_file():
        raise RuntimeError(
            "matching llvm-profdata is unavailable; run `mise install` to install "
            "Rust's llvm-tools-preview component, or set LLVM_PROFDATA"
        )
    return tool


def prepare_training_corpora() -> None:
    if TRAINING.exists():
        shutil.rmtree(TRAINING)

    for name, (source, excluded) in CORPORA.items():
        if not source.is_dir():
            raise RuntimeError(
                f"missing benchmark corpus: {source}; run `mise install` to import fixtures"
            )
        files = [
            path
            for path in sorted(source.rglob("*.java"))
            if path.relative_to(source).as_posix() not in excluded
        ]
        if not files:
            raise RuntimeError(
                f"benchmark corpus contains no Java files: {source}"
            )
        for path in files:
            destination = TRAINING / name / path.relative_to(source)
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, destination)
        print(f"Training corpus {name}: {len(files)} files")


def reset_build_outputs() -> None:
    for path in (PROFILES, INSTRUMENTED_TARGET, OPTIMIZED_TARGET):
        if path.exists():
            shutil.rmtree(path)
    if MERGED_PROFILE.exists():
        MERGED_PROFILE.unlink()
    PROFILES.mkdir(parents=True)


def build(target: Path, rustflags: str) -> None:
    run(
        "cargo",
        "build",
        "--release",
        "--package",
        "jolt_cli",
        env={"CARGO_TARGET_DIR": target.as_posix(), "RUSTFLAGS": rustflags},
    )


def require_file(path: Path, label: str) -> None:
    if not path.is_file():
        raise RuntimeError(f"missing {label}: {path}")


def capture(*args: str) -> str:
    return subprocess.run(
        args, cwd=ROOT, check=True, text=True, stdout=subprocess.PIPE
    ).stdout.strip()


def run(*args: str | Path, env: dict[str, str] | None = None) -> None:
    command = [str(arg) for arg in args]
    print("+", " ".join(command), flush=True)
    process_env = os.environ.copy()
    if env:
        process_env.update(env)
    subprocess.run(command, cwd=ROOT, check=True, env=process_env)


if __name__ == "__main__":
    main()
