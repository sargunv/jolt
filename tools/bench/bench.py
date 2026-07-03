#!/usr/bin/env python3
"""Run formatter benchmarks over imported benchmark corpora."""

import argparse
import json
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = Path(__file__).resolve()
IMPORTS = ROOT / "tools/import/imports.toml"
CORPORA = ROOT / ".bench/corpora"
WORK = ROOT / "target/bench"
REPORTS = ROOT / "tools/bench/reports"
JOLT = ROOT / "target/release/jolt"
DPRINT_PLUGIN = (
    ROOT / "target/wasm32-unknown-unknown/release/jolt_fmt_dprint.wasm"
)
TOOLS = ("jolt", "dprint", "google-java-format", "prettier-java")

COMMANDS = (
    ("jolt fmt", "{jolt} fmt {jolt_dir}"),
    (
        "dprint fmt --incremental=false --skip-stable-format",
        "cd {dprint_dir} && dprint fmt --incremental=false --skip-stable-format .",
    ),
    (
        "google-java-format --replace",
        "xargs -0 google-java-format --replace --skip-removing-unused-imports < {gjf_files}",
    ),
    (
        "prettier --write --plugin prettier-plugin-java",
        "pnpm exec prettier --write {prettier_glob} --plugin prettier-plugin-java --print-width 80 --tab-width 2",
    ),
)


def main() -> int:
    if sys.argv[1:2] == ["--prepare"]:
        prepare(sys.argv[2])
        return 0

    parser = argparse.ArgumentParser()
    parser.add_argument("--corpus", action="append", choices=corpus_names())
    args, hyperfine_args = parser.parse_known_args()

    corpora = load_corpora()
    selected = (
        corpora
        if not args.corpus
        else {name: corpora[name] for name in args.corpus}
    )
    benchmark(selected, strip_separator(hyperfine_args))
    return 0


def load_corpora() -> dict[str, dict]:
    with IMPORTS.open("rb") as file:
        config = tomllib.load(file)
    corpora = {}
    for source in config["sources"]:
        for copy in source.get("copy", []):
            if copy["kind"] != "bench":
                continue
            name = copy.get("name") or Path(copy["to"]).name
            corpora[name] = {
                "description": copy.get("description", source["name"]),
            }
    return corpora


def corpus_names() -> tuple[str, ...]:
    return tuple(load_corpora())


def prepare(name: str) -> None:
    corpus = CORPORA / name
    if not corpus.is_dir():
        raise RuntimeError(f"missing benchmark corpus: {corpus}")

    for tool in TOOLS:
        dest = tool_dir(name, tool)
        if dest.exists():
            shutil.rmtree(dest)
        shutil.copytree(corpus, dest)

    (tool_dir(name, "dprint") / "dprint.json").write_text(
        json.dumps(
            {
                "lineWidth": 80,
                "indentWidth": 2,
                "useTabs": False,
                "plugins": [str(DPRINT_PLUGIN)],
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )

    java_files = sorted(tool_dir(name, "google-java-format").rglob("*.java"))
    (WORK / name).mkdir(parents=True, exist_ok=True)
    (WORK / name / "google-java-format.java-files.nul").write_bytes(
        b"".join(str(path).encode() + b"\0" for path in java_files)
    )


def benchmark(corpora: dict[str, dict], hyperfine_args: list[str]) -> None:
    require(JOLT, "release Jolt CLI")
    require(DPRINT_PLUGIN, "release dprint plugin")
    require(ROOT / "node_modules/.bin/prettier", "Prettier install")

    for name, corpus in corpora.items():
        summarize(name, corpus)
        args = [
            "hyperfine",
            "--prepare",
            f"{q(sys.executable)} {q(SCRIPT)} --prepare {name}",
            *hyperfine_args,
        ]
        for label, command in COMMANDS:
            args += ["-n", label, command.format_map(context(name))]
        if exports_report(hyperfine_args):
            run(*args, cwd=ROOT)
        else:
            write_report(name, corpus, run_capture(*args, cwd=ROOT))


def context(name: str) -> dict[str, str]:
    return {
        "jolt": q(JOLT),
        "jolt_dir": q(tool_dir(name, "jolt")),
        "dprint_dir": q(tool_dir(name, "dprint")),
        "gjf_files": q(WORK / name / "google-java-format.java-files.nul"),
        "prettier_glob": q(str(tool_dir(name, "prettier-java") / "**/*.java")),
    }


def exports_report(hyperfine_args: list[str]) -> bool:
    return any(arg.startswith("--export-") for arg in hyperfine_args)


def write_report(name: str, corpus: dict, output: str) -> None:
    REPORTS.mkdir(parents=True, exist_ok=True)
    report = REPORTS / f"{name}.md"
    log(f"writing hyperfine report: {report}")
    report.write_text(
        f"# {name}\n\n"
        f"{corpus['description']}.\n\n"
        "```text\n"
        f"{output.rstrip()}\n"
        "```\n",
        encoding="utf-8",
    )
    run("dprint", "fmt", report, cwd=ROOT)


def summarize(name: str, corpus: dict) -> None:
    files = list((CORPORA / name).rglob("*.java"))
    total_bytes = sum(path.stat().st_size for path in files)
    log(
        f"benchmarking {name} ({corpus['description']}): "
        f"{len(files)} file(s), {total_bytes} byte(s)"
    )


def tool_dir(corpus: str, tool: str) -> Path:
    return WORK / corpus / tool


def strip_separator(args: list[str]) -> list[str]:
    return args[1:] if args[:1] == ["--"] else args


def require(path: Path, label: str) -> None:
    if not path.exists():
        raise RuntimeError(f"missing {label}: {path}")


def q(path: str | Path) -> str:
    return "'" + str(path).replace("'", "'\"'\"'") + "'"


def run(*command: str | Path, cwd: Path | None = None) -> None:
    args = [str(arg) for arg in command]
    print("+ " + " ".join(args), file=sys.stderr)
    subprocess.run(args, cwd=cwd, check=True)


def run_capture(*command: str | Path, cwd: Path | None = None) -> str:
    args = [str(arg) for arg in command]
    print("+ " + " ".join(args), file=sys.stderr)
    output = subprocess.check_output(
        args,
        cwd=cwd,
        stderr=subprocess.STDOUT,
        text=True,
    )
    print(output, end="", file=sys.stderr)
    return output


def log(message: str) -> None:
    print(message, file=sys.stderr)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except subprocess.CalledProcessError as error:
        raise SystemExit(error.returncode) from error
