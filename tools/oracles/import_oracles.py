#!/usr/bin/env python3
"""Import formatter oracle inputs."""

import shutil
import subprocess
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / ".oracles"
PINS = ROOT / "tools" / "oracles" / "oracle-pins.toml"


@dataclass(frozen=True)
class JavaSuite:
    repo: str
    suite: str
    input_dir: str


@dataclass(frozen=True)
class RepoPin:
    repo: str
    tag: str
    commit: str


JAVA_SUITES = (
    JavaSuite(
        repo="google/google-java-format",
        suite="google-java-format",
        input_dir="core/src/test/resources/com/google/googlejavaformat/java/testdata",
    ),
    JavaSuite(
        repo="palantir/palantir-java-format",
        suite="palantir-java-format",
        input_dir=(
            "palantir-java-format/src/test/resources/com/palantir/javaformat/java/testdata"
        ),
    ),
)


def main() -> int:
    output_root = OUTPUT
    repos_root = output_root / "repos"
    fixtures_root = output_root / "fixtures"

    pins = load_pins()
    output_root.mkdir(parents=True, exist_ok=True)
    repos_root.mkdir(parents=True, exist_ok=True)
    if fixtures_root.exists():
        log(f"clearing {fixtures_root}")
        shutil.rmtree(fixtures_root)
    fixtures_root.mkdir(parents=True, exist_ok=True)

    for repo, pin in pins.items():
        log(f"syncing {repo} {pin.tag} ({pin.commit})")
        sync_repo(pin, repos_root / repo.replace("/", "__"))

    fixture_count = 0
    for suite in JAVA_SUITES:
        repo_dir = repos_root / suite.repo.replace("/", "__")
        source_dir = repo_dir / suite.input_dir
        if not source_dir.is_dir():
            raise RuntimeError(
                f"missing upstream fixture directory: {source_dir}"
            )
        suite_count = 0
        for input_path in sorted(source_dir.glob("*.input")):
            materialized_input = (
                fixtures_root
                / suite.suite
                / "input"
                / f"{input_path.stem}.java"
            )
            materialized_input.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(input_path, materialized_input)
            fixture_count += 1
            suite_count += 1
        log(f"imported {suite_count} {suite.suite} input fixture(s)")

    print(f"imported {fixture_count} oracle inputs under {output_root}")
    return 0


def load_pins() -> dict[str, RepoPin]:
    with PINS.open("rb") as file:
        raw_pins = tomllib.load(file)

    pins = {}
    for repo, values in sorted(raw_pins.items()):
        if not isinstance(values, dict):
            raise TypeError(f"pin for {repo} must contain tag and commit")
        tag = values.get("tag")
        commit = values.get("commit")
        if not isinstance(tag, str) or not isinstance(commit, str):
            raise TypeError(
                f"pin for {repo} must contain string tag and commit"
            )
        pins[str(repo)] = RepoPin(repo=str(repo), tag=tag, commit=commit)
    return pins


def sync_repo(pin: RepoPin, destination: Path) -> None:
    url = f"https://github.com/{pin.repo}.git"
    if not destination.exists():
        run(
            (
                "git",
                "clone",
                "--filter=blob:none",
                "--no-checkout",
                url,
                str(destination),
            )
        )
    run(
        ("git", "fetch", "--depth", "1", "origin", f"refs/tags/{pin.tag}"),
        cwd=destination,
    )
    run(("git", "checkout", "--detach", pin.commit), cwd=destination)
    actual_commit = capture(("git", "rev-parse", "HEAD"), cwd=destination)
    if actual_commit != pin.commit:
        raise RuntimeError(
            f"{pin.repo} {pin.tag} resolved to {actual_commit}, expected {pin.commit}"
        )


def run(command: tuple[str, ...], cwd: Path | None = None) -> None:
    print("+ " + " ".join(command), file=sys.stderr)
    subprocess.run(command, cwd=cwd, check=True)


def capture(command: tuple[str, ...], cwd: Path | None = None) -> str:
    print("+ " + " ".join(command), file=sys.stderr)
    completed = subprocess.run(
        command,
        cwd=cwd,
        check=True,
        stdout=subprocess.PIPE,
        text=True,
    )
    return completed.stdout.strip()


def log(message: str) -> None:
    print(message, file=sys.stderr)


if __name__ == "__main__":
    raise SystemExit(main())
