#!/usr/bin/env python3
"""Run formatter benchmarks over imported benchmark corpora."""

import argparse
import fnmatch
import json
import os
import platform
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Literal

ROOT = Path(__file__).resolve().parents[2]
IMPORTS = ROOT / "tools/import/.imports"
WORK = ROOT / "target/bench"
REPORTS = ROOT / "tools/bench/reports"
JOLT = ROOT / "target/release/jolt"
DPRINT_PLUGIN = (
    ROOT / "target/wasm32-unknown-unknown/release/jolt_fmt_dprint.wasm"
)
HYPERFINE_MIN_RUNS = 3
HYPERFINE_WARMUP = 1

CORPORA = {
    "adversarial": {
        "description": "google-java-format formatter test inputs",
        "language": "java",
        "extensions": (".java",),
        "source": IMPORTS / "google-java-format/input",
        "exclude": [
            # Intentionally invalid upstream Java.
            "B26952926.java",
        ],
        "tool_exclude": {
            # Uses an annotation expression accepted by google-java-format and
            # Jolt but rejected by prettier-plugin-java 2.10.2.
            "prettier-java": ["B38352414.java"],
        },
    },
    "realistic": {
        "description": "Spring Framework Java sources",
        "language": "java",
        "extensions": (".java",),
        "source": IMPORTS / "spring-framework",
        "exclude": [],
    },
    "kotlin-realistic": {
        "description": "MapLibre Compose Kotlin sources",
        "language": "kotlin",
        "extensions": (".kt", ".kts"),
        "source": IMPORTS / "maplibre-compose/source",
        "exclude": [],
    },
}


def q(path: str | Path) -> str:
    return "'" + str(path).replace("'", "'\"'\"'") + "'"


ToolKey = Literal["jolt", "dprint-jolt", "google-java-format", "prettier-java"]
DEFAULT_TOOLS: tuple[ToolKey, ...] = ("jolt", "dprint-jolt")


@dataclass(frozen=True)
class Tool:
    label: str
    benchmark_command: Callable[[str], str]
    version_command: str
    requirements: tuple[tuple[Path, str], ...] = ()
    reset_commands: Callable[[str], list[str]] = lambda _corpus: []
    languages: frozenset[str] = frozenset({"java", "kotlin"})


TOOLS: dict[ToolKey, Tool] = {
    "jolt": Tool(
        "jolt fmt",
        lambda corpus: f"{q(JOLT)} fmt {q(tool_dir(corpus, 'jolt'))}",
        f"{q(JOLT)} --version",
        ((JOLT, "release Jolt CLI"),),
    ),
    "dprint-jolt": Tool(
        "dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format",
        lambda corpus: (
            f"cd {q(tool_dir(corpus, 'dprint-jolt'))} && dprint --plugins={q(DPRINT_PLUGIN)} fmt --incremental=false --skip-stable-format ."
        ),
        "dprint --version",
        ((DPRINT_PLUGIN, "release dprint plugin"),),
        lambda corpus: [
            f"cp {q(WORK / corpus / 'dprint.json')} {q(tool_dir(corpus, 'dprint-jolt') / 'dprint.json')}"
        ],
    ),
    "google-java-format": Tool(
        "google-java-format --replace",
        lambda corpus: (
            f"google-java-format --replace --skip-removing-unused-imports @{q(WORK / corpus / 'google-java-format.args')}"
        ),
        "google-java-format --version",
        reset_commands=lambda corpus: [
            f"find {q(tool_dir(corpus, 'google-java-format'))} -name '*.java' -print > {q(WORK / corpus / 'google-java-format.args')}"
        ],
        languages=frozenset({"java"}),
    ),
    "prettier-java": Tool(
        "prettier --write --plugin prettier-plugin-java",
        lambda corpus: (
            f"pnpm exec prettier --write {q(str(tool_dir(corpus, 'prettier-java') / '**/*.java'))} --plugin prettier-plugin-java --print-width 80 --tab-width 2 --ignore-path {q(WORK / corpus / 'prettier.ignore')} --log-level silent"
        ),
        "pnpm exec prettier --version",
        ((ROOT / "node_modules/.bin/prettier", "Prettier install"),),
        languages=frozenset({"java"}),
    ),
}


def main() -> int:
    tool_keys = parse_args(sys.argv[1:])
    benchmark(tool_keys)
    return 0


def parse_args(argv: list[str]) -> tuple[ToolKey, ...]:
    parser = argparse.ArgumentParser(
        description="Run formatter benchmarks over imported benchmark corpora."
    )
    parser.add_argument(
        "tools",
        nargs="*",
        choices=(*TOOLS, "all"),
        help=f"tools to benchmark (default: {' '.join(DEFAULT_TOOLS)})",
    )
    args = parser.parse_args(argv)

    if not args.tools:
        return DEFAULT_TOOLS

    if args.tools == ["all"]:
        return tuple(TOOLS)

    if "all" in args.tools:
        parser.error("'all' cannot be combined with explicit tools")

    return tuple(dict.fromkeys(args.tools))


def prepare_baseline(name: str, tool_keys: tuple[ToolKey, ...]) -> None:
    corpus = CORPORA[name]
    source = corpus["source"]
    if not source.is_dir():
        raise RuntimeError(f"missing benchmark import: {source}")

    copy_corpus(corpus, baseline_dir(name))
    (WORK / name).mkdir(parents=True, exist_ok=True)
    if "prettier-java" in tool_keys:
        (WORK / name / "prettier.ignore").write_text(
            "\n".join(corpus.get("tool_exclude", {}).get("prettier-java", ())),
            encoding="utf-8",
        )
    if "dprint-jolt" in tool_keys:
        (WORK / name / "dprint.json").write_text(
            json.dumps(
                {
                    "lineWidth": 80,
                    "indentWidth": 2,
                    "useTabs": False,
                },
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )


def copy_corpus(corpus: dict, dest: Path) -> None:
    if dest.exists():
        shutil.rmtree(dest)
    dest.mkdir(parents=True)

    for source in corpus_files(corpus, corpus["source"]):
        relative = source.relative_to(corpus["source"])
        if is_excluded(relative, corpus["exclude"]):
            continue
        target = dest / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)


def is_excluded(path: Path, patterns: list[str]) -> bool:
    posix = path.as_posix()
    return any(fnmatch.fnmatchcase(posix, pattern) for pattern in patterns)


def benchmark(tool_keys: tuple[ToolKey, ...]) -> None:
    for key in tool_keys:
        for path, label in TOOLS[key].requirements:
            require(path, label)

    for name, corpus in CORPORA.items():
        corpus_tool_keys = applicable_tools(corpus, tool_keys)
        if not corpus_tool_keys:
            log(
                f"skipping {name}: none of the selected tools support {corpus['language']}"
            )
            continue
        prepare_baseline(name, corpus_tool_keys)
        summarize(name, corpus)
        versions = version_strings(corpus_tool_keys)
        rows = report_rows(name, corpus_tool_keys, versions)
        args = [
            "hyperfine",
            "--warmup",
            str(HYPERFINE_WARMUP),
            "--min-runs",
            str(HYPERFINE_MIN_RUNS),
            "--prepare",
            reset_command(name, corpus_tool_keys),
        ]
        for key in corpus_tool_keys:
            tool = TOOLS[key]
            args += [
                "-n",
                tool.label,
                tool.benchmark_command(name),
            ]
        output = run_capture(*args, cwd=ROOT)
        write_report(name, rows, output)


def write_report(
    name: str, rows: list[dict[str, str | int]], output: str
) -> None:
    REPORTS.mkdir(parents=True, exist_ok=True)
    report = REPORTS / f"{name}.txt"
    log(f"writing benchmark report: {report}")
    contents = "\n".join(
        [
            format_rows(rows),
            f"System: {system_info()}",
            f"Hyperfine: adaptive runs, min {HYPERFINE_MIN_RUNS}, {HYPERFINE_WARMUP} warmup",
            "",
            output.rstrip(),
            "",
        ]
    )
    report.write_text(contents, encoding="utf-8")


def summarize(name: str, corpus: dict) -> None:
    files = corpus_files(corpus, baseline_dir(name))
    total_bytes = sum(path.stat().st_size for path in files)
    log(
        f"benchmarking {name} ({corpus['description']}): {len(files)} file(s), {total_bytes} byte(s)"
    )


def report_rows(
    name: str, tool_keys: tuple[ToolKey, ...], versions: dict[ToolKey, str]
) -> list[dict[str, str | int]]:
    log(f"collecting report rows for {name}")
    rows = []

    run_shell(reset_command(name, tool_keys), cwd=ROOT)
    for key in tool_keys:
        run_shell(TOOLS[key].benchmark_command(name), cwd=ROOT)
        input_files = len(corpus_files(CORPORA[name], tool_dir(name, key)))
        modified_files = count_modified_files(
            CORPORA[name], baseline_dir(name), tool_dir(name, key)
        )
        if input_files > 0 and modified_files == 0:
            raise RuntimeError(f"{key} did not modify any files for {name}")
        rows.append(
            {
                "tool": key,
                "version": versions[key],
                "input_files": input_files,
                "modified_files": modified_files,
            }
        )

    return rows


def version_strings(tool_keys: tuple[ToolKey, ...]) -> dict[ToolKey, str]:
    versions = {}
    for key in tool_keys:
        tool = TOOLS[key]
        try:
            output = run_shell_capture(tool.version_command, cwd=ROOT)
            versions[key] = " ".join(output.split()) or "unknown"
        except subprocess.CalledProcessError:
            versions[key] = "unknown"
    return versions


def format_rows(rows: list[dict[str, str | int]]) -> str:
    lines = ["tool\tversion\tinput_files\tmodified_files"]
    for row in rows:
        lines.append(
            f"{row['tool']}\t{row['version']}\t{row['input_files']}\t{row['modified_files']}"
        )
    return "\n".join(lines)


def corpus_files(corpus: dict, directory: Path) -> list[Path]:
    return sorted(
        path
        for extension in corpus["extensions"]
        for path in directory.rglob(f"*{extension}")
    )


def applicable_tools(
    corpus: dict, tool_keys: tuple[ToolKey, ...]
) -> tuple[ToolKey, ...]:
    language = corpus["language"]
    return tuple(key for key in tool_keys if language in TOOLS[key].languages)


def count_modified_files(corpus: dict, baseline: Path, formatted: Path) -> int:
    count = 0
    for path in corpus_files(corpus, baseline):
        relative = path.relative_to(baseline)
        formatted_path = formatted / relative
        if (
            not formatted_path.exists()
            or path.read_bytes() != formatted_path.read_bytes()
        ):
            count += 1
    return count


def system_info() -> str:
    uname = platform.uname()
    parts = [
        f"{uname.system} {uname.release}",
        uname.machine,
    ]
    logical_cpus = os.cpu_count()
    if logical_cpus is not None:
        parts.append(f"{logical_cpus} logical CPUs")
    return ", ".join(parts)


def tool_dir(corpus: str, tool: str) -> Path:
    return WORK / corpus / tool


def baseline_dir(corpus: str) -> Path:
    return WORK / corpus / "baseline"


def reset_command(corpus: str, tool_keys: tuple[ToolKey, ...]) -> str:
    baseline = q(baseline_dir(corpus))
    parts = []
    for key in tool_keys:
        parts.append(f"rm -rf {q(tool_dir(corpus, key))}")
        parts.append(f"cp -R {baseline} {q(tool_dir(corpus, key))}")
        parts.extend(TOOLS[key].reset_commands(corpus))
    return " && ".join(parts)


def require(path: Path, label: str) -> None:
    if not path.exists():
        raise RuntimeError(f"missing {label}: {path}")


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


def run_shell(command: str, cwd: Path | None = None) -> None:
    print("+ " + command, file=sys.stderr)
    subprocess.run(command, cwd=cwd, shell=True, check=True)


def run_shell_capture(command: str, cwd: Path | None = None) -> str:
    print("+ " + command, file=sys.stderr)
    output = subprocess.check_output(
        command,
        cwd=cwd,
        shell=True,
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
