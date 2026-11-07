#!/usr/bin/env python3
"""Time formatting the Spring Framework sources with hyperfine and write the
medians to docs/bench.json, which the docs read.

Run with `mise run bench`, which builds the release CLI and plugin first.
Each run formats a fresh copy of the corpus; copying is not timed. Numbers
from different machines are not comparable, so commit a rerun from the
machine named in docs/bench.json, or replace every number.
"""

import argparse
import datetime
import json
import os
import platform
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CORPUS = ROOT / "tools/import/.imports/spring-framework"
JOLT = ROOT / "target/release/jolt"
PLUGIN = ROOT / "target/plugin.wasm"
OUTPUT = ROOT / "docs/bench.json"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--compare",
        action="store_true",
        help="also time google-java-format and prettier-java",
    )
    parser.add_argument("--runs", type=int, default=10)
    args = parser.parse_args()

    files = sorted(CORPUS.rglob("*.java"))
    if not files:
        raise SystemExit(
            f"no Java files under {CORPUS}; run `mise install` to import the corpora"
        )

    with tempfile.TemporaryDirectory() as work:
        work = Path(work)
        copy = work / "copy"
        (work / "dprint.json").write_text("{}\n")
        (work / "files.txt").write_text(
            "".join(f"{copy / path.relative_to(CORPUS)}\n" for path in files)
        )
        (work / "empty").write_text("")
        commands = {
            "jolt": f"{JOLT} format --no-config {copy}",
            "jolt, 1 thread": f"{JOLT} format --no-config --threads 1 {copy}",
            "jolt (dprint)": f"cd {copy} && dprint fmt --config {work / 'dprint.json'} --plugins {PLUGIN} "
            "--incremental=false --skip-stable-format '**/*.java'",
        }
        if args.compare:
            commands["google-java-format"] = (
                f"google-java-format --replace @{work / 'files.txt'}"
            )
            commands["prettier-java"] = (
                f"cd {ROOT} && pnpm exec prettier --write --plugin prettier-plugin-java "
                f"--ignore-path {work / 'empty'} --log-level silent '{copy}/**/*.java'"
            )
        hyperfine = [
            "hyperfine",
            "--shell=default",
            "--warmup=1",
            f"--runs={args.runs}",
            f"--prepare=rm -rf {copy} && cp -R {CORPUS} {copy}",
            f"--export-json={work / 'results.json'}",
        ]
        for name, command in commands.items():
            hyperfine += [f"--command-name={name}", command]
        subprocess.run(hyperfine, check=True)
        results = json.loads((work / "results.json").read_text())["results"]

    OUTPUT.write_text(
        json.dumps(
            {
                "corpus": "spring-framework",
                "files": len(files),
                "bytes": sum(path.stat().st_size for path in files),
                "machine": machine(),
                "commit": capture("git", "rev-parse", "--short", "HEAD"),
                "date": datetime.date.today().isoformat(),
                "medianSeconds": {
                    name: round(
                        result["summary"]["time_wall_clock"]["median"], 4
                    )
                    for name, result in zip(commands, results, strict=True)
                },
            },
            indent=2,
        )
        + "\n"
    )
    print(f"Wrote {OUTPUT.relative_to(ROOT)}")


def machine() -> str:
    cpu = platform.processor()
    cpuinfo = Path("/proc/cpuinfo")
    if cpuinfo.exists():
        cpu = next(
            (
                line.split(":", 1)[1].strip()
                for line in cpuinfo.read_text().splitlines()
                if line.startswith("model name")
            ),
            cpu,
        )
    elif platform.system() == "Darwin":
        cpu = capture("sysctl", "-n", "machdep.cpu.brand_string")
    return f"{cpu}, {os.cpu_count()} threads, {platform.system()}"


def capture(*args: str) -> str:
    return subprocess.run(
        args, cwd=ROOT, check=True, text=True, capture_output=True
    ).stdout.strip()


if __name__ == "__main__":
    main()
