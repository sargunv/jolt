#!/usr/bin/env python3
"""Build Jolt with profile-guided optimization using benchmark corpora."""

import argparse
import os
import shutil
import subprocess
from pathlib import Path

from tools.corpora import CORPORA

ROOT = Path(__file__).resolve().parents[2]
WORK = ROOT / "target/pgo"
PROFILES = WORK / "profiles"
INSTRUMENTED_TARGET = WORK / "instrumented"
OPTIMIZED_TARGET = WORK / "optimized"
TRAINING = WORK / "training"
MERGED_PROFILE = WORK / "jolt.profdata"
EXE_SUFFIX = ".exe" if os.name == "nt" else ""


def main() -> None:
    target = parse_args().target
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
        target,
    )
    optimized_jolt = optimized_binary(target)
    require_file(optimized_jolt, "PGO-optimized Jolt CLI")
    if target is None or target == rust_host():
        run(optimized_jolt, "--version")
    print(f"PGO-optimized CLI: {optimized_jolt}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--target",
        help="Rust target triple for the optimized build (training runs on the host)",
    )
    return parser.parse_args()


def optimized_binary(target: str | None) -> Path:
    directory = OPTIMIZED_TARGET
    if target:
        directory /= target
    suffix = ".exe" if target and "windows" in target else EXE_SUFFIX
    return directory / f"release/jolt{suffix}"


def find_llvm_profdata() -> Path:
    override = os.environ.get("LLVM_PROFDATA")
    if override:
        tool = Path(override)
        require_file(tool, "LLVM_PROFDATA")
        return tool

    sysroot = capture("rustc", "--print", "sysroot")
    host = rust_host()
    tool = (
        Path(sysroot) / "lib/rustlib" / host / f"bin/llvm-profdata{EXE_SUFFIX}"
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

    for name, corpus in CORPORA.items():
        source = corpus.source
        if not source.is_dir():
            raise RuntimeError(
                f"missing benchmark corpus: {source}; run `mise install` to import fixtures"
            )
        files = corpus.files()
        if not files:
            raise RuntimeError(
                f"benchmark corpus contains no source files: {source}"
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


def rust_host() -> str:
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
    return host_line.removeprefix("host: ")


def build(target_dir: Path, rustflags: str, target: str | None = None) -> None:
    command = [
        "cargo",
        "build",
        "--release",
        "--package",
        "jolt_cli",
    ]
    if target:
        command += ["--target", target]
    run(
        *command,
        env={"CARGO_TARGET_DIR": target_dir.as_posix(), "RUSTFLAGS": rustflags},
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
