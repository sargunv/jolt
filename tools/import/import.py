#!/usr/bin/env python3
"""Import pinned upstream files.

An import copies the files its globs match, or with `files` exactly the paths
listed in that file (fetched with a shallow, sparse checkout of the pinned
commit, so a large repository costs only those files). An import with
`generate` writes the files a generator derives from the matched files
instead of copying them; that keeps derived files out of the repository when
their licence asks for attribution or share-alike.
"""

import contextlib
import json
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

if os.name == "nt":
    import msvcrt
else:
    import fcntl

ROOT = Path(__file__).resolve().parents[2]
IMPORTS = ROOT / "tools/import/.imports"
# Records what was imported; mise reruns the import when it is missing.
MANIFEST = IMPORTS / "manifest.json"
LOCK = IMPORTS / ".locks/import.lock"
LOCK_TIMEOUT_SECONDS = 300
LOCK_RETRY_SECONDS = 0.1

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
    # The Markdown (`///`) doc comments of the JDK, and a sample of `/**`
    # comments with `{@snippet}`, `<!-- -->` and content lines without a
    # leading `*`.
    {
        "name": "openjdk",
        "repo": "openjdk/jdk",
        "tag": "jdk-27-ga",
        "commit": "815ff4dc327fe17f2433c7d115a5a503af10f3c4",
        "path": ".",
        "files": "tools/import/openjdk-files.txt",
        "keep_path": True,
    },
    # The CommonMark spec's examples, each as a `///` comment on a method.
    # The spec is CC-BY-SA 4.0, so the derived files are generated here and
    # never committed.
    {
        "name": "commonmark-spec/input",
        "repo": "commonmark/commonmark-spec",
        "tag": "0.31.2",
        "commit": "9103e341a973013013bb1a80e13567007c5cef6f",
        "path": ".",
        "globs": ["spec.txt"],
        "generate": "commonmark_examples",
    },
]


def main() -> int:
    # Git hooks run with GIT_DIR, GIT_INDEX_FILE, ... pointing at this repo
    # (mise runs this import from the pre-commit hook). Drop them so git
    # commands act on the corpus clones instead.
    for name in subprocess.check_output(
        ["git", "rev-parse", "--local-env-vars"], text=True
    ).split():
        os.environ.pop(name, None)
    with import_lock():
        sync_repos(IMPORTS_CONFIG)
        clear_imports(IMPORTS_CONFIG)
        manifest = materialize(IMPORTS_CONFIG)
        write_manifest(manifest)
    return 0


def sync_repos(imports: list[dict]) -> None:
    repos = {item["repo"]: item for item in imports}
    for item in sorted(repos.values(), key=lambda item: item["repo"]):
        destination = repo_dir(item)
        log(f"syncing {item['repo']} {item['tag']} ({item['commit']})")
        if "files" in item:
            sync_sparse(item, destination)
            continue
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
        # A full filtered clone already contains reachable commit objects.
        # Avoid a redundant exact-tag fetch: remote tag advertisement may
        # transiently lag even though the pinned commit is available locally.
        if not has_commit(destination, item["commit"]):
            run_with_retries(
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


def sync_sparse(item: dict, destination: Path) -> None:
    """Checks out only the listed files of the pinned commit, fetched
    without history."""
    if not destination.exists():
        destination.mkdir(parents=True)
        run("git", "init", "--quiet", cwd=destination)
        run(
            "git",
            "remote",
            "add",
            "origin",
            f"https://github.com/{item['repo']}.git",
            cwd=destination,
        )
    if not has_commit(destination, item["commit"]):
        run_with_retries(
            "git",
            "fetch",
            "--depth",
            "1",
            "--filter=blob:none",
            "origin",
            f"refs/tags/{item['tag']}",
            cwd=destination,
        )
    patterns = "".join(f"/{path}\n" for path in listed_files(item))
    print("+ git sparse-checkout set --no-cone --stdin", file=sys.stderr)
    subprocess.run(
        ["git", "sparse-checkout", "set", "--no-cone", "--stdin"],
        cwd=destination,
        input=patterns,
        text=True,
        check=True,
    )
    run(
        "git",
        "checkout",
        "--quiet",
        "--detach",
        item["commit"],
        cwd=destination,
    )
    actual = capture("git", "rev-parse", "HEAD", cwd=destination)
    if actual != item["commit"]:
        raise RuntimeError(
            f"{item['repo']} resolved to {actual}, expected {item['commit']}"
        )


def listed_files(item: dict) -> list[str]:
    text = (ROOT / item["files"]).read_text()
    return [line for line in text.splitlines() if line.strip()]


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


def materialize(imports: list[dict]) -> list[dict]:
    manifest = []
    for item in imports:
        source_dir = repo_dir(item) / item["path"]
        if "files" in item:
            paths = [source_dir / path for path in listed_files(item)]
            missing = [path for path in paths if not path.is_file()]
            if missing:
                raise RuntimeError(f"missing listed files: {missing}")
        else:
            paths = source_paths(source_dir, item["globs"])
        target_dir = IMPORTS / item["name"]
        seen: set[Path] = set()

        if "generate" in item:
            for path in paths:
                for name, content in GENERATORS[item["generate"]](path):
                    destination = target_dir / name
                    if destination in seen:
                        raise RuntimeError(f"duplicate generated path: {name}")
                    seen.add(destination)
                    destination.write_text(content)
            paths = []

        for path in paths:
            relative = path.relative_to(source_dir)
            output = output_path(item, relative)
            destination = target_dir / output
            if destination in seen:
                raise RuntimeError(f"duplicate materialized path: {output}")
            seen.add(destination)
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, destination)

        log(f"imported {len(seen)} {item['name']} file(s)")
        manifest.append(
            {
                "name": item["name"],
                "repo": item["repo"],
                "tag": item["tag"],
                "commit": item["commit"],
                "source_path": item["path"],
                "globs": item.get("globs", []),
                "files": item.get("files"),
                "file_count": len(seen),
            }
        )
    return manifest


def write_manifest(manifest: list[dict]) -> None:
    MANIFEST.write_text(
        json.dumps(
            {
                "version": 1,
                "suites": manifest,
            },
            indent=2,
            sort_keys=True,
        )
        + "\n"
    )
    log(f"wrote {MANIFEST.relative_to(ROOT)}")


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


def commonmark_examples(spec: Path):
    """Yields one Java file per example of the CommonMark spec: the example's
    Markdown as a `///` comment on a method. Examples a `///` comment can't
    carry unchanged are skipped: a line starting with `/` would make a `////`
    line, and Java reads `\\u` as a Unicode escape even in a comment."""
    fence = "`" * 32
    section = ""
    number = 0
    lines = iter(spec.read_text().splitlines())
    for line in lines:
        if line.startswith("#"):
            section = line.lstrip("#").strip()
        if line != f"{fence} example":
            continue
        number += 1
        markdown = []
        for line in lines:
            if line == ".":
                break
            markdown.append(line.replace("\u2192", "\t"))
        for line in lines:
            if line == fence:
                break
        if any(line.startswith("/") for line in markdown) or any(
            "\\u" in line for line in markdown
        ):
            continue
        slug = re.sub(r"[^a-z0-9]+", "-", section.lower()).strip("-")
        body = "".join(f"  ///{line}\n" for line in markdown)
        yield (
            f"example-{number:03}-{slug}.java",
            f"// CommonMark spec 0.31.2, example {number} ({section}).\n"
            "// Copyright (C) 2014-16 John MacFarlane, CC-BY-SA 4.0.\n"
            f"class Example{number} {{\n{body}  void example() {{}}\n}}\n",
        )


GENERATORS = {"commonmark_examples": commonmark_examples}


def repo_dir(item: dict) -> Path:
    return IMPORTS / ".repos" / item["repo"].replace("/", "__")


@contextlib.contextmanager
def import_lock():
    LOCK.parent.mkdir(parents=True, exist_ok=True)
    with LOCK.open("a+b") as lock_file:
        if os.name == "nt":
            if lock_file.tell() == 0:
                lock_file.write(b"\0")
                lock_file.flush()
            deadline = time.monotonic() + LOCK_TIMEOUT_SECONDS
            while True:
                try:
                    lock_file.seek(0)
                    msvcrt.locking(lock_file.fileno(), msvcrt.LK_NBLCK, 1)
                    break
                except OSError as error:
                    if time.monotonic() >= deadline:
                        raise TimeoutError(
                            f"timed out waiting for import lock: {LOCK}"
                        ) from error
                    time.sleep(LOCK_RETRY_SECONDS)
        else:
            fcntl.flock(lock_file, fcntl.LOCK_EX)
        try:
            yield
        finally:
            if os.name == "nt":
                lock_file.seek(0)
                msvcrt.locking(lock_file.fileno(), msvcrt.LK_UNLCK, 1)


def run(*command: str, cwd: Path | None = None) -> None:
    print("+ " + " ".join(command), file=sys.stderr)
    subprocess.run(command, cwd=cwd, check=True)


def run_with_retries(
    *command: str,
    cwd: Path | None = None,
    attempts: int = 3,
) -> None:
    for attempt in range(1, attempts + 1):
        try:
            run(*command, cwd=cwd)
            return
        except subprocess.CalledProcessError:
            if attempt == attempts:
                raise
            delay = attempt
            log(
                f"command failed (attempt {attempt}/{attempts}); "
                f"retrying in {delay}s"
            )
            time.sleep(delay)


def has_commit(repository: Path, commit: str) -> bool:
    return (
        subprocess.run(
            ["git", "cat-file", "-e", f"{commit}^{{commit}}"],
            cwd=repository,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        ).returncode
        == 0
    )


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
