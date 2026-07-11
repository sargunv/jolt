#!/usr/bin/env python3
"""Build the target-specific PGO binary expected by cargo-dist."""

import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

PACKAGE_ROOT = Path(__file__).resolve().parent
ROOT = PACKAGE_ROOT.parents[1]
IMPORT_MANIFEST = ROOT / "tools/import/.imports/manifest.json"
OUT = PACKAGE_ROOT / "out"


def main() -> None:
    target = os.environ.get("CARGO_DIST_TARGET")
    if not target:
        raise RuntimeError("cargo-dist did not set CARGO_DIST_TARGET")
    validate_versions()

    run("rustup", "component", "add", "llvm-tools-preview")
    run("rustup", "target", "add", target)
    if not IMPORT_MANIFEST.is_file():
        run(sys.executable, ROOT / "tools/import/import.py")

    suffix = ".exe" if "windows" in target else ""
    host = rust_host()
    if target_arch(host) == target_arch(target):
        run(sys.executable, "-m", "tools.pgo.build", "--target", target)
        source = ROOT / f"target/pgo/optimized/{target}/release/jolt{suffix}"
        should_smoke = True
    else:
        # cargo-dist assigns ARM64 Windows to an x64 Windows runner. Do not
        # reuse indexed profiles across architectures.
        run(
            "cargo",
            "build",
            "--release",
            "--package",
            "jolt_cli",
            "--target",
            target,
        )
        source = ROOT / f"target/{target}/release/jolt{suffix}"
        should_smoke = False
    if not source.is_file():
        raise RuntimeError(
            f"release build did not produce expected binary: {source}"
        )
    if should_smoke:
        smoke_binary(source)

    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir()
    shutil.copy2(source, OUT / f"jolt{suffix}")


def rust_host() -> str:
    line = next(
        line
        for line in capture("rustc", "-vV").splitlines()
        if line.startswith("host: ")
    )
    return line.removeprefix("host: ")


def target_arch(target: str) -> str:
    return target.split("-", 1)[0]


def smoke_binary(binary: Path) -> None:
    run(binary, "--version")
    result = subprocess.run(
        [str(binary), "fmt", "-"],
        cwd=ROOT,
        check=True,
        input="class Smoke{}\n",
        text=True,
        stdout=subprocess.PIPE,
    )
    if result.stdout != "class Smoke {\n}\n":
        raise RuntimeError(
            f"release smoke test produced unexpected output: {result.stdout!r}"
        )


def validate_versions() -> None:
    metadata = json.loads(
        capture("cargo", "metadata", "--no-deps", "--format-version=1")
    )
    cargo_version = next(
        package["version"]
        for package in metadata["packages"]
        if package["name"] == "jolt_cli"
    )
    dist_manifest = (PACKAGE_ROOT / "dist.toml").read_text(encoding="utf-8")
    match = re.search(r'^version = "([^"]+)"$', dist_manifest, re.MULTILINE)
    if match is None:
        raise RuntimeError("release package manifest has no version")
    dist_version = match.group(1)
    if cargo_version != dist_version:
        raise RuntimeError(
            "release package version does not match jolt_cli: "
            f"{dist_version} != {cargo_version}"
        )


def run(*args: str | Path) -> None:
    command = [str(arg) for arg in args]
    print("+", " ".join(command), flush=True)
    subprocess.run(command, cwd=ROOT, check=True)


def capture(*args: str | Path) -> str:
    return subprocess.run(
        [str(arg) for arg in args],
        cwd=ROOT,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    ).stdout


if __name__ == "__main__":
    main()
