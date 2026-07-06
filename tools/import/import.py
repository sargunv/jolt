#!/usr/bin/env python3
"""Import pinned upstream files."""

import contextlib
import shutil
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
IMPORTS = ROOT / "tools/import/.imports"

IMPORTS_CONFIG = [
    {
        "name": "google-java-format/input",
        "repo": "google/google-java-format",
        "tag": "v1.35.0",
        "commit": "cdd8a84012838205747cfd54b389a37397bdb701",
        "path": "core/src/test/resources/com/google/googlejavaformat/java/testdata",
        "globs": ["*.input"],
        "rename": "{stem}.java",
    },
    {
        "name": "palantir-java-format/input",
        "repo": "palantir/palantir-java-format",
        "tag": "2.94.0",
        "commit": "df34ef135c326a290046f793deb945ac714359f4",
        "path": "palantir-java-format/src/test/resources/com/palantir/javaformat/java/testdata",
        "globs": ["*.input"],
        "rename": "{stem}.java",
    },
    {
        "name": "prettier-java/input",
        "repo": "jhipster/prettier-java",
        "tag": "prettier-plugin-java@2.10.2",
        "commit": "34174e31d237eed8d70f63f4fa9bdaf598887576",
        "path": "test/unit-test",
        "globs": ["**/_input.java"],
        "rename": "{parent}.java",
        "keep_path": True,
    },
    {
        "name": "spring-framework",
        "repo": "spring-projects/spring-framework",
        "tag": "v7.0.8",
        "commit": "9e8cea3ef8ae02efb7956b071cd7bbef7c22cb82",
        "path": ".",
        "globs": ["**/*.java"],
        "keep_path": True,
    },
    {
        "name": "ktfmt/source",
        "repo": "facebook/ktfmt",
        "tag": "v0.64",
        "commit": "682a5cd32d741bd7126183714aa4ee2e3246defe",
        "path": ".",
        "globs": ["**/*.kt", "**/*.kts"],
        "keep_path": True,
    },
    {
        "name": "maplibre-compose/source",
        "repo": "maplibre/maplibre-compose",
        "tag": "v0.13.0",
        "commit": "db32283a8fdb7838ffc6e3fc333b7b10b57df0f5",
        "path": ".",
        "globs": ["**/*.kt", "**/*.kts"],
        "keep_path": True,
    },
]


def main() -> int:
    sync_repos(IMPORTS_CONFIG)
    clear_imports(IMPORTS_CONFIG)
    materialize(IMPORTS_CONFIG)
    return 0


def sync_repos(imports: list[dict]) -> None:
    repos = {item["repo"]: item for item in imports}
    for item in sorted(repos.values(), key=lambda item: item["repo"]):
        with repo_lock(item):
            destination = repo_dir(item)
            log(f"syncing {item['repo']} {item['tag']} ({item['commit']})")
            if not destination.exists():
                destination.parent.mkdir(parents=True, exist_ok=True)
                run(
                    "git",
                    "clone",
                    "--filter=blob:none",
                    "--no-checkout",
                    f"https://github.com/{item['repo']}.git",
                    str(destination),
                )
            run(
                "git",
                "fetch",
                "--depth",
                "1",
                "origin",
                f"refs/tags/{item['tag']}",
                cwd=destination,
            )
            run("git", "checkout", "--detach", item["commit"], cwd=destination)
            run("git", "clean", "-fdx", cwd=destination)
            actual = capture("git", "rev-parse", "HEAD", cwd=destination)
            if actual != item["commit"]:
                raise RuntimeError(
                    f"{item['repo']} resolved to {actual}, expected {item['commit']}"
                )


def clear_imports(imports: list[dict]) -> None:
    IMPORTS.mkdir(parents=True, exist_ok=True)
    for child in IMPORTS.iterdir():
        if child.name.startswith("."):
            continue
        if child.is_dir():
            shutil.rmtree(child)
        else:
            child.unlink()
    for item in imports:
        target = IMPORTS / item["name"]
        target.mkdir(parents=True)


def materialize(imports: list[dict]) -> None:
    for item in imports:
        source_dir = repo_dir(item) / item["path"]
        paths = source_paths(source_dir, item["globs"])
        target_dir = IMPORTS / item["name"]
        written = 0
        seen: set[Path] = set()

        for path in paths:
            relative = path.relative_to(source_dir)
            output = output_path(item, relative)
            destination = target_dir / output
            if destination in seen:
                raise RuntimeError(f"duplicate materialized path: {output}")
            seen.add(destination)
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, destination)
            written += 1

        log(f"imported {written} {item['name']} file(s)")


def source_paths(source_dir: Path, globs: list[str]) -> list[Path]:
    if not source_dir.is_dir():
        raise RuntimeError(f"missing source directory: {source_dir}")
    paths = {
        path
        for glob in globs
        for path in source_dir.glob(glob)
        if path.is_file()
    }
    if not paths:
        raise RuntimeError(
            f"no source files matched under {source_dir}: {globs}"
        )
    return sorted(paths)


def output_path(item: dict, relative: Path) -> Path:
    template = item.get("rename", "{name}")
    name = template.format(
        name=relative.name,
        stem=relative.stem,
        suffix=relative.suffix,
        parent=relative.parent.name,
    )
    if item.get("keep_path"):
        return relative.parent / name
    return Path(name)


def repo_dir(item: dict) -> Path:
    return IMPORTS / ".repos" / item["repo"].replace("/", "__")


@contextlib.contextmanager
def repo_lock(item: dict):
    lock = IMPORTS / ".locks" / (item["repo"].replace("/", "__") + ".lock")
    lock.parent.mkdir(parents=True, exist_ok=True)
    while True:
        try:
            lock.mkdir()
            break
        except FileExistsError:
            time.sleep(0.1)
    try:
        yield
    finally:
        shutil.rmtree(lock, ignore_errors=True)


def run(*command: str, cwd: Path | None = None) -> None:
    print("+ " + " ".join(command), file=sys.stderr)
    subprocess.run(command, cwd=cwd, check=True)


def capture(*command: str, cwd: Path | None = None) -> str:
    print("+ " + " ".join(command), file=sys.stderr)
    return subprocess.check_output(
        command,
        cwd=cwd,
        text=True,
    ).strip()


def log(message: str) -> None:
    print(message, file=sys.stderr)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except subprocess.CalledProcessError as error:
        raise SystemExit(error.returncode) from error
