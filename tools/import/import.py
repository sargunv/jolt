#!/usr/bin/env python3
"""Import pinned fixture and benchmark corpora."""

import argparse
import contextlib
import fnmatch
import shutil
import subprocess
import sys
import time
import tomllib
from pathlib import Path, PurePosixPath


ROOT = Path(__file__).resolve().parents[2]
CONFIG = ROOT / "tools/import/imports.toml"
CACHE = ROOT / "tools/import/.imports"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("kind", choices=("test", "bench", "all"))
    args = parser.parse_args()

    config = load_config()
    sources = selected_sources(config["sources"], args.kind)
    sync_repos(sources)
    clear_outputs(config["outputs"], sources)
    materialize(config["outputs"], sources)
    return 0


def load_config() -> dict:
    with CONFIG.open("rb") as file:
        return tomllib.load(file)


def selected_sources(sources: list[dict], kind: str) -> list[dict]:
    selected = []
    for source in sources:
        copies = [
            copy
            for copy in source.get("copy", [])
            if kind == "all" or copy["kind"] == kind
        ]
        if copies:
            selected.append({**source, "copy": copies})
    if not selected:
        raise RuntimeError(f"no imports configured for {kind}")
    return selected


def sync_repos(sources: list[dict]) -> None:
    for source in sorted(sources, key=lambda item: item["repo"]):
        with repo_lock(source):
            destination = repo_dir(source)
            log(
                f"syncing {source['repo']} {source['tag']} ({source['commit']})"
            )
            if not destination.exists():
                destination.parent.mkdir(parents=True, exist_ok=True)
                run(
                    "git",
                    "clone",
                    "--filter=blob:none",
                    "--no-checkout",
                    f"https://github.com/{source['repo']}.git",
                    str(destination),
                )
            run(
                "git",
                "fetch",
                "--depth",
                "1",
                "origin",
                f"refs/tags/{source['tag']}",
                cwd=destination,
            )
            run(
                "git", "checkout", "--detach", source["commit"], cwd=destination
            )
            run("git", "clean", "-fdx", cwd=destination)
            actual = capture("git", "rev-parse", "HEAD", cwd=destination)
            if actual != source["commit"]:
                raise RuntimeError(
                    f"{source['repo']} resolved to {actual}, expected {source['commit']}"
                )


def clear_outputs(outputs: dict[str, str], sources: list[dict]) -> None:
    for kind in sorted(
        {copy["kind"] for source in sources for copy in source["copy"]}
    ):
        root = ROOT / outputs[kind]
        if root.exists():
            shutil.rmtree(root)
        root.mkdir(parents=True)


def materialize(outputs: dict[str, str], sources: list[dict]) -> None:
    for source in sources:
        source_dir = repo_dir(source) / source["path"]
        paths = source_paths(source_dir, source["globs"])
        for copy in source["copy"]:
            target_dir = ROOT / outputs[copy["kind"]] / copy["to"]
            target_dir.mkdir(parents=True, exist_ok=True)

            written = excluded = 0
            seen: set[Path] = set()
            for path in paths:
                relative = path.relative_to(source_dir)
                output = output_path(copy, relative)
                if is_excluded(copy, output):
                    excluded += 1
                    continue
                destination = target_dir / output
                if destination in seen:
                    raise RuntimeError(f"duplicate materialized path: {output}")
                seen.add(destination)
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(path, destination)
                written += 1

            label = copy.get("name") or copy["to"]
            suffix = f", excluded {excluded}" if excluded else ""
            log(f"imported {written} {label} file(s){suffix}")


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


def output_path(copy: dict, relative: Path) -> Path:
    template = copy.get("rename", "{name}")
    name = template.format(
        name=relative.name,
        stem=relative.stem,
        suffix=relative.suffix,
        parent=relative.parent.name,
    )
    if copy.get("keep_path"):
        return relative.parent / name
    return Path(name)


def is_excluded(copy: dict, relative: Path) -> bool:
    path = PurePosixPath(relative.as_posix())
    return any(
        path.match(pattern) or fnmatch.fnmatchcase(path.as_posix(), pattern)
        for pattern in copy.get("exclude", [])
    )


def repo_dir(source: dict) -> Path:
    return CACHE / source["repo"].replace("/", "__")


@contextlib.contextmanager
def repo_lock(source: dict):
    lock = CACHE / ".locks" / (source["repo"].replace("/", "__") + ".lock")
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
