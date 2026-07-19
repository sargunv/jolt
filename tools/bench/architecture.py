#!/usr/bin/env python3
"""Measure formatter architecture costs on the realistic corpora."""

from __future__ import annotations

import hashlib
import json
import os
import platform
import re
import shutil
import statistics
import subprocess
import sys
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from tools.corpora import CORPORA, REALISTIC_CORPUS_KEYS, Corpus

ROOT = Path(__file__).resolve().parents[2]
WORK = ROOT / "target/bench/architecture"
REPORTS = ROOT / "tools/bench/reports/machines"
DRIVER = ROOT / "target/release/jolt_bench_driver"
JOLT = ROOT / "target/release/jolt"
DPRINT_PLUGIN = (
    ROOT / "target/wasm32-unknown-unknown/release/jolt_fmt_dprint.wasm"
)
TIMING_DRIVER = WORK / "jolt_bench_driver-timing"
ALLOCATION_DRIVER = WORK / "jolt_bench_driver-allocations"
SITE_REPORT = ROOT / "tools/bench/reports/site.json"
HARNESS_GENERATION = 3
CORPUS_KEYS = REALISTIC_CORPUS_KEYS
MODES = ("parse", "format", "end-to-end")
CLI_SAMPLES = 5
CLI_WARMUPS = 1


def benchmark() -> int:
    machine = machine_metadata()
    snapshot = measure(machine, samples=20, warmups=2)
    output = report_path(str(machine["id"]))
    write_json(output, snapshot)
    write_json(SITE_REPORT, site_report(snapshot))
    run("dprint", "fmt", output, SITE_REPORT)
    print_summary(snapshot)
    print(f"report: {output}")
    return 0


def main() -> int:
    return benchmark()


def measure(
    machine: dict[str, str | int | None], samples: int, warmups: int
) -> dict[str, Any]:
    initial_harness = harness_digest()
    initial_source = source_state()
    build_artifacts()
    corpora: dict[str, Any] = {}
    for key in CORPUS_KEYS:
        corpus = CORPORA[key]
        manifest = corpus_manifest(corpus)
        stages = {
            mode: measure_stage(corpus, manifest, mode, samples, warmups)
            for mode in MODES
        }
        structure = stages["format"].pop("structure")
        syntax = structure["syntax"]
        document = structure["document"]
        if corpus_manifest(corpus) != manifest:
            raise RuntimeError(f"{key} corpus changed during measurement")
        corpora[key] = {
            "manifest": manifest,
            "stages": stages,
            "whole_cli": measure_whole_cli(corpus, CLI_SAMPLES, CLI_WARMUPS),
            "structure": {
                "syntax": syntax,
                "document": document,
                "normalized": {
                    **{
                        f"{mode.replace('-', '_')}_ns_per_token": ratio(
                            timing_median(stages[mode]), syntax["tokens"]
                        )
                        for mode in MODES
                    },
                    "tree_reserved_bytes_per_token": ratio(
                        syntax["reserved_bytes"], syntax["tokens"]
                    ),
                    "tree_reserved_bytes_per_node": ratio(
                        syntax["reserved_bytes"], syntax["nodes"]
                    ),
                    "document_nodes_per_token": ratio(
                        document["nodes"], syntax["tokens"]
                    ),
                },
            },
        }
    if harness_digest() != initial_harness or source_state() != initial_source:
        raise RuntimeError(
            "source or benchmark harness changed during measurement"
        )
    return {
        "schema_version": 1,
        "harness_generation": HARNESS_GENERATION,
        "harness_sha256": initial_harness,
        "recorded_at": datetime.now(UTC).isoformat(),
        "machine": machine,
        "subject": initial_source,
        "build": build_metadata(),
        "corpora": corpora,
    }


def build_artifacts() -> None:
    WORK.mkdir(parents=True, exist_ok=True)
    run(
        "cargo",
        "build",
        "--release",
        "--package",
        "jolt_bench_driver",
        "--no-default-features",
    )
    shutil.copy2(DRIVER, TIMING_DRIVER)
    run(
        "cargo",
        "build",
        "--release",
        "--package",
        "jolt_bench_driver",
        "--features",
        "allocations",
    )
    shutil.copy2(DRIVER, ALLOCATION_DRIVER)
    run("cargo", "build", "--release", "--package", "jolt_cli")
    run(ROOT / "tools/wasm/build_optimized.sh")


def measure_stage(
    corpus: Corpus,
    manifest: dict[str, int | str],
    mode: str,
    samples: int,
    warmups: int,
) -> dict[str, Any]:
    common = (
        "--corpus",
        str(corpus.source),
        "--language",
        corpus.language,
        "--mode",
        mode,
    )
    timing_command = (
        TIMING_DRIVER,
        *common,
        "--measurement",
        "timing",
        "--samples",
        str(samples),
        "--warmups",
        str(warmups),
    )
    timing = run_json(
        *timing_command,
    )
    allocation_command = (
        ALLOCATION_DRIVER,
        *common,
        "--measurement",
        "allocations",
        "--samples",
        "3",
        "--warmups",
        "1",
    )
    allocations = run_json(
        *allocation_command,
    )
    memory_command = (
        TIMING_DRIVER,
        *common,
        "--measurement",
        "memory",
    )
    memory = run_json_with_peak_rss(*memory_command)
    for result, measurement in (
        (timing, "timing"),
        (allocations, "allocations"),
        (memory, "memory"),
    ):
        if result["corpus"] != {
            "files": manifest["files"],
            "source_bytes": manifest["source_bytes"],
        }:
            raise RuntimeError("driver corpus does not match the manifest")
        if (
            result["schema_version"] != 1
            or result["language"] != corpus.language
            or result["mode"] != mode
            or result["measurement"] != measurement
        ):
            raise RuntimeError("driver result does not match the request")
        for key in (
            "schema_version",
            "language",
            "mode",
            "measurement",
            "corpus",
        ):
            result.pop(key)
    structure = None
    if mode == "format":
        structure = {
            "syntax": timing.pop("syntax"),
            "document": timing.pop("document"),
        }
    elif "syntax" in timing or "document" in timing:
        raise RuntimeError("driver returned structure for a non-format stage")

    timing = timing["timing"]
    timing["summary"] = timing_summary(timing["samples_ns"])
    allocations = allocations["allocations"]
    allocations["summary"] = allocation_summary(allocations["samples"])
    result = {
        "commands": {
            "timing": command_string(timing_command),
            "allocations": command_string(allocation_command),
            "memory": command_string(memory_command),
        },
        "timing": timing,
        "allocations": allocations,
        "peak_rss_bytes": memory["peak_rss_bytes"],
    }
    if structure is not None:
        result["structure"] = structure
    return result


def measure_whole_cli(
    corpus: Corpus, samples: int, warmups: int
) -> dict[str, Any]:
    tools = ["jolt-native", "jolt-dprint"]
    if corpus.language == "java":
        tools.extend(("google-java-format", "prettier-java"))
    return {
        "samples": samples,
        "warmups": warmups,
        "tools": {
            tool: measure_cli_tool(corpus, tool, samples, warmups)
            for tool in tools
        },
    }


def measure_cli_tool(
    corpus: Corpus, tool: str, samples: int, warmups: int
) -> dict[str, Any]:
    target = WORK / "cli" / corpus.language / tool / "source"
    command, cwd = cli_command(tool, target)
    samples_ns: list[int] = []
    for index in range(warmups + samples):
        prepare_cli_corpus(corpus, target, tool)
        phase = (
            f"warmup {index + 1}/{warmups}"
            if index < warmups
            else f"sample {index - warmups + 1}/{samples}"
        )
        print(
            f"+ whole-cli {corpus.language} {tool} {phase}",
            file=sys.stderr,
        )
        start = time.perf_counter_ns()
        completed = subprocess.run(
            [str(part) for part in command],
            cwd=cwd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            check=False,
        )
        elapsed = time.perf_counter_ns() - start
        if completed.returncode != 0:
            raise RuntimeError(
                f"{tool} exited with {completed.returncode}:\n"
                f"{completed.stdout}{completed.stderr}"
            )
        if index >= warmups:
            samples_ns.append(elapsed)
    modified = sum(
        source.read_bytes()
        != (target / source.relative_to(corpus.source)).read_bytes()
        for source in corpus.files()
    )
    if modified == 0:
        raise RuntimeError(f"{tool} did not modify any {corpus.language} files")
    return {
        "label": cli_label(tool),
        "version": cli_version(tool),
        "command": command_string(command),
        "cwd": str(cwd),
        "modified_files": modified,
        "timing": {
            "samples_ns": samples_ns,
            "summary": timing_summary(samples_ns),
        },
    }


def prepare_cli_corpus(corpus: Corpus, target: Path, tool: str) -> None:
    shutil.rmtree(target, ignore_errors=True)
    target.mkdir(parents=True)
    for source in corpus.files():
        destination = target / source.relative_to(corpus.source)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, destination)
    if tool == "jolt-dprint":
        (target / "dprint.json").write_text(
            json.dumps(
                {"lineWidth": 80, "indentWidth": 2, "useTabs": False},
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )
    elif tool == "google-java-format":
        arguments = target.parent / "google-java-format.args"
        arguments.write_text(
            "\n".join(str(path) for path in corpus.files(target)) + "\n",
            encoding="utf-8",
        )
    elif tool == "prettier-java":
        (target.parent / "prettier.ignore").write_text("", encoding="utf-8")


def cli_command(tool: str, target: Path) -> tuple[tuple[str | Path, ...], Path]:
    if tool == "jolt-native":
        return (JOLT, "fmt", target), ROOT
    if tool == "jolt-dprint":
        return (
            (
                "dprint",
                f"--plugins={DPRINT_PLUGIN}",
                "fmt",
                "--incremental=false",
                "--skip-stable-format",
                ".",
            ),
            target,
        )
    if tool == "google-java-format":
        return (
            (
                "google-java-format",
                "--replace",
                "--skip-removing-unused-imports",
                f"@{target.parent / 'google-java-format.args'}",
            ),
            ROOT,
        )
    if tool == "prettier-java":
        return (
            (
                "pnpm",
                "exec",
                "prettier",
                "--write",
                target / "**/*.java",
                "--plugin",
                "prettier-plugin-java",
                "--print-width",
                "80",
                "--tab-width",
                "2",
                "--ignore-path",
                target.parent / "prettier.ignore",
                "--log-level",
                "silent",
            ),
            ROOT,
        )
    raise RuntimeError(f"unknown whole-CLI benchmark tool: {tool}")


def cli_label(tool: str) -> str:
    return {
        "jolt-native": "jolt (native)",
        "jolt-dprint": "jolt (dprint)",
        "google-java-format": "google-java-format",
        "prettier-java": "prettier-java",
    }[tool]


def cli_version(tool: str) -> str:
    if tool == "jolt-native":
        return capture_version(str(JOLT), "--version")
    if tool == "jolt-dprint":
        dprint = capture_version("dprint", "--version")
        commit = capture("git", "rev-parse", "--short", "HEAD").strip()
        return f"{dprint}; jolt {commit}"
    if tool == "google-java-format":
        return capture_version("google-java-format", "--version")
    if tool == "prettier-java":
        prettier = capture_version("pnpm", "exec", "prettier", "--version")
        package = json.loads(
            (ROOT / "node_modules/prettier-plugin-java/package.json").read_text(
                encoding="utf-8"
            )
        )
        return f"prettier {prettier}; prettier-plugin-java {package['version']}"
    raise RuntimeError(f"unknown whole-CLI benchmark tool: {tool}")


def timing_summary(samples: list[int]) -> dict[str, float]:
    median = float(statistics.median(samples))
    return {
        "median_ns": median,
        "median_absolute_deviation_ns": float(
            statistics.median(abs(sample - median) for sample in samples)
        ),
    }


def allocation_summary(samples: list[dict[str, int]]) -> dict[str, float]:
    return {
        key: float(statistics.median(sample[key] for sample in samples))
        for key in ("count_total", "count_max", "bytes_total", "bytes_max")
    }


def command_string(command: tuple[str | Path, ...]) -> str:
    return " ".join(map(str, command))


def ratio(numerator: int, denominator: int) -> float:
    if denominator == 0:
        raise RuntimeError(
            "cannot normalize metrics for a corpus without tokens"
        )
    return numerator / denominator


def harness_digest() -> str:
    paths = [
        Path("mise.toml"),
        Path("tools/corpora.py"),
        Path("tools/bench/architecture.py"),
        Path("tools/bench/driver/Cargo.toml"),
    ]
    paths.extend(sorted(Path("tools/bench/driver/src").rglob("*.rs")))
    digest = hashlib.sha256()
    for relative in paths:
        contents = (ROOT / relative).read_bytes()
        encoded = relative.as_posix().encode()
        digest.update(len(encoded).to_bytes(8, "big"))
        digest.update(encoded)
        digest.update(len(contents).to_bytes(8, "big"))
        digest.update(contents)
    return digest.hexdigest()


def corpus_manifest(corpus: Corpus) -> dict[str, int | str]:
    digest = hashlib.sha256()
    files = corpus.files()
    source_bytes = 0
    for path in files:
        contents = path.read_bytes()
        relative = path.relative_to(corpus.source).as_posix().encode()
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        digest.update(len(contents).to_bytes(8, "big"))
        digest.update(contents)
        source_bytes += len(contents)
    return {
        "sha256": digest.hexdigest(),
        "files": len(files),
        "source_bytes": source_bytes,
    }


def machine_metadata() -> dict[str, str | int | None]:
    uname = platform.uname()
    specs: dict[str, str | int | None] = {
        "system": uname.system,
        "architecture": uname.machine,
        "processor": processor_name(),
        "logical_cpus": os.cpu_count(),
        "memory_bytes": total_memory_bytes(),
    }
    encoded = json.dumps(specs, sort_keys=True, separators=(",", ":")).encode()
    prefix = re.sub(
        r"[^a-z0-9]+", "-", f"{uname.system}-{uname.machine}".lower()
    ).strip("-")
    return {
        "id": f"{prefix}-{hashlib.sha256(encoded).hexdigest()[:12]}",
        **specs,
        "release": uname.release,
    }


def total_memory_bytes() -> int | None:
    if sys.platform == "darwin":
        try:
            memory = int(capture("sysctl", "-n", "hw.memsize").strip())
            return memory if memory > 0 else None
        except (subprocess.CalledProcessError, ValueError):
            return None
    try:
        pages = os.sysconf("SC_PHYS_PAGES")
        page_size = os.sysconf("SC_PAGE_SIZE")
        return pages * page_size if pages > 0 and page_size > 0 else None
    except (AttributeError, OSError, ValueError):
        return None


def processor_name() -> str:
    if sys.platform == "darwin":
        try:
            return capture("sysctl", "-n", "machdep.cpu.brand_string").strip()
        except subprocess.CalledProcessError:
            pass
    if sys.platform.startswith("linux"):
        cpuinfo = Path("/proc/cpuinfo")
        if cpuinfo.is_file():
            for line in cpuinfo.read_text(encoding="utf-8").splitlines():
                if line.startswith("model name"):
                    return line.partition(":")[2].strip()
    return platform.processor() or "unknown"


def source_state() -> dict[str, str | bool | None]:
    commit = capture("git", "rev-parse", "HEAD").strip()
    diff = subprocess.check_output(
        [
            "git",
            "diff",
            "--binary",
            "HEAD",
            "--",
            ".",
            ":(exclude)tools/bench/reports",
        ],
        cwd=ROOT,
    )
    untracked = capture(
        "git",
        "ls-files",
        "--others",
        "--exclude-standard",
        "--",
        ".",
        ":(exclude)tools/bench/reports",
    ).splitlines()
    digest = hashlib.sha256()
    digest.update(diff)
    for relative in sorted(untracked):
        path = ROOT / relative
        if path.is_file():
            digest.update(relative.encode())
            digest.update(path.read_bytes())
    dirty = bool(diff or untracked)
    return {
        "commit": commit,
        "dirty": dirty,
        "worktree_sha256": digest.hexdigest() if dirty else None,
    }


def build_metadata() -> dict[str, str]:
    rustc = capture("rustc", "-vV").strip()
    host = next(
        line.removeprefix("host: ")
        for line in rustc.splitlines()
        if line.startswith("host: ")
    )
    return {"profile": "release", "rustc": rustc, "target": host}


def timing_median(stage: dict[str, Any]) -> float:
    samples = stage["timing"]["samples_ns"]
    if not isinstance(samples, list) or not samples:
        raise RuntimeError("timing result has no samples_ns")
    return float(statistics.median(samples))


def site_report(snapshot: dict[str, Any]) -> dict[str, Any]:
    spring = snapshot["corpora"]["realistic"]
    tools = spring["whole_cli"]["tools"]
    return {
        "schema_version": 1,
        "recorded_at": snapshot["recorded_at"],
        "subject": snapshot["subject"],
        "machine": snapshot["machine"],
        "corpus": spring["manifest"],
        "tools": [
            {
                "id": key,
                "label": tool["label"],
                "version": tool["version"],
                "median_seconds": timing_median(tool) / 1_000_000_000,
                "median_absolute_deviation_seconds": tool["timing"]["summary"][
                    "median_absolute_deviation_ns"
                ]
                / 1_000_000_000,
            }
            for key, tool in tools.items()
        ],
    }


def print_summary(snapshot: dict[str, Any]) -> None:
    print(f"subject: {snapshot['subject']['commit']}")
    for corpus in CORPUS_KEYS:
        for mode in MODES:
            stage = snapshot["corpora"][corpus]["stages"][mode]
            median_ms = timing_median(stage) / 1_000_000
            print(f"{corpus:18} {mode:10} {median_ms:10.3f} ms")
        normalized = snapshot["corpora"][corpus]["structure"]["normalized"]
        print(
            f"{corpus:18} {'structure':10} "
            f"{normalized['parse_ns_per_token']:.2f} parse ns/token, "
            f"{normalized['format_ns_per_token']:.2f} format ns/token, "
            f"{normalized['end_to_end_ns_per_token']:.2f} e2e ns/token, "
            f"{normalized['tree_reserved_bytes_per_token']:.2f} tree bytes/token, "
            f"{normalized['tree_reserved_bytes_per_node']:.2f} tree bytes/node, "
            f"{normalized['document_nodes_per_token']:.2f} doc nodes/token"
        )
        for tool in snapshot["corpora"][corpus]["whole_cli"]["tools"].values():
            median_ms = timing_median(tool) / 1_000_000
            print(
                f"{corpus:18} {'whole-cli':10} {tool['label']:20} "
                f"{median_ms:10.3f} ms"
            )


def report_path(machine: str) -> Path:
    return REPORTS / f"{machine}.json"


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    temporary.replace(path)


def run(*command: str | Path) -> None:
    print("+ " + " ".join(map(str, command)), file=sys.stderr)
    subprocess.run([str(part) for part in command], cwd=ROOT, check=True)


def capture(*command: str) -> str:
    return subprocess.check_output(command, cwd=ROOT, text=True)


def capture_version(*command: str) -> str:
    completed = subprocess.run(
        command,
        cwd=ROOT,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return (completed.stdout or completed.stderr).strip()


def run_json(program: Path, *arguments: str) -> dict[str, Any]:
    command = [str(program), *arguments]
    print("+ " + " ".join(command), file=sys.stderr)
    output = subprocess.check_output(command, cwd=ROOT, text=True)
    return json.loads(output)


def run_json_with_peak_rss(program: Path, *arguments: str) -> dict[str, Any]:
    command = [str(program), *arguments]
    if sys.platform == "darwin":
        wrapped = ["/usr/bin/time", "-l", *command]
        pattern = re.compile(
            r"^\s*(\d+)\s+maximum resident set size$", re.MULTILINE
        )
        scale = 1
    elif sys.platform.startswith("linux"):
        wrapped = ["/usr/bin/time", "-v", *command]
        pattern = re.compile(
            r"^\s*Maximum resident set size \(kbytes\):\s*(\d+)\s*$",
            re.MULTILINE,
        )
        scale = 1024
    else:
        raise RuntimeError("peak RSS measurement supports Linux and macOS")
    print("+ " + " ".join(wrapped), file=sys.stderr)
    completed = subprocess.run(
        wrapped,
        cwd=ROOT,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"benchmark driver exited with {completed.returncode}:\n{completed.stderr}"
        )
    match = pattern.search(completed.stderr)
    if match is None:
        raise RuntimeError(
            f"could not parse peak RSS from /usr/bin/time:\n{completed.stderr}"
        )
    result = json.loads(completed.stdout)
    result["peak_rss_bytes"] = int(match.group(1)) * scale
    return result


if __name__ == "__main__":
    raise SystemExit(main())
