#!/usr/bin/env python3
"""Build the jolt CLI with profile-guided optimization.

Training formats a copy of fixtures/java. PGO makes `jolt format --check` on
the Spring Framework sources about 18% faster single-threaded (measured with
hyperfine, 2026-11-07); training on fixtures/java gets nearly all of that,
so releases need no imported corpus.

Run by `mise run build:jolt --mode release`, which writes target/release/jolt,
and by cargo-dist (dist/jolt/dist.toml), which sets CARGO_DIST_TARGET and wants
the binary in dist/jolt/out.
"""

import os
import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WORK = ROOT / "target/pgo"


def main() -> None:
    dist_target = os.environ.get("CARGO_DIST_TARGET")
    host = capture("rustc", "-vV").split("host: ")[1].split()[0]
    target = dist_target or host
    exe = ".exe" if "windows" in target else ""

    if dist_target:
        run("rustup", "component", "add", "llvm-tools-preview")
        run("rustup", "target", "add", target)

    if target.split("-")[0] == host.split("-")[0]:
        binary = pgo_build(host, target, exe)
        run(binary, "--version")
        smoke = subprocess.run(
            [binary, "fmt", "-"],
            input="class Smoke{}\n",
            text=True,
            check=True,
            capture_output=True,
        )
        if not smoke.stdout.startswith("class Smoke {"):
            raise SystemExit(f"smoke test produced {smoke.stdout!r}")
    else:
        # The ARM64 Windows release builds on an x64 runner, where the
        # instrumented binary cannot run to train.
        run(
            "cargo",
            "build",
            "--release",
            "--package",
            "jolt_cli",
            "--target",
            target,
        )
        binary = ROOT / f"target/{target}/release/jolt{exe}"

    out = ROOT / "dist/jolt/out" if dist_target else ROOT / "target/release"
    out.mkdir(parents=True, exist_ok=True)
    shutil.copy2(binary, out / f"jolt{exe}")
    print(f"Built {out / f'jolt{exe}'}")


def pgo_build(host: str, target: str, exe: str) -> Path:
    shutil.rmtree(WORK, ignore_errors=True)
    profiles = WORK / "profiles"
    # With an explicit --target, RUSTFLAGS skip build scripts, so only the
    # CLI is instrumented.
    cargo_build(WORK / "instrumented", f"-Cprofile-generate={profiles}", host)
    with tempfile.TemporaryDirectory() as training:
        shutil.copytree(ROOT / "fixtures/java", training, dirs_exist_ok=True)
        subprocess.run(
            [
                WORK / f"instrumented/{host}/release/jolt{exe}",
                "fmt",
                "--format-with-errors",
                training,
            ],
            env={
                **os.environ,
                "LLVM_PROFILE_FILE": str(profiles / "jolt-%m-%p.profraw"),
            },
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    raw = sorted(profiles.glob("*.profraw"))
    if not raw:
        raise SystemExit("training wrote no profiles")
    sysroot = Path(capture("rustc", "--print", "sysroot").strip())
    profdata = WORK / "jolt.profdata"
    host_exe = ".exe" if os.name == "nt" else ""
    run(
        sysroot / f"lib/rustlib/{host}/bin/llvm-profdata{host_exe}",
        "merge",
        "-o",
        profdata,
        *raw,
    )
    cargo_build(WORK / "optimized", f"-Cprofile-use={profdata}", target)
    return WORK / f"optimized/{target}/release/jolt{exe}"


def cargo_build(target_dir: Path, rustflags: str, target: str) -> None:
    run(
        "cargo",
        "build",
        "--release",
        "--package",
        "jolt_cli",
        "--target",
        target,
        env={"CARGO_TARGET_DIR": str(target_dir), "RUSTFLAGS": rustflags},
    )


def capture(*args: str) -> str:
    return subprocess.run(
        args, check=True, text=True, capture_output=True
    ).stdout


def run(*args: str | Path, env: dict[str, str] | None = None) -> None:
    print("+", " ".join(map(str, args)), flush=True)
    subprocess.run(
        [str(arg) for arg in args],
        cwd=ROOT,
        check=True,
        env={**os.environ, **(env or {})},
    )


if __name__ == "__main__":
    main()
