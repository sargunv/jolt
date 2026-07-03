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
HYPERFINE_RUNS = 3
HYPERFINE_WARMUP = 1

CORPORA = {
    "adversarial": {
        "description": "google-java-format formatter test inputs",
        "source": IMPORTS / "google-java-format/input",
        "exclude": [
            # Intentionally invalid upstream Java.
            "B26952926.java",
            # Uses an annotation expression accepted by google-java-format but
            # rejected by prettier-plugin-java 2.10.2.
            "B38352414.java",
        ],
    },
    "realistic": {
        "description": "Spring Framework Java sources",
        "source": IMPORTS / "spring-framework",
        "exclude": [
            # Jolt parser gap: generic array constructor references, for
            # example `classes.toArray(Class<?>[]::new)`.
            "spring-aop/src/main/java/org/springframework/aop/framework/AopProxyUtils.java",
            "spring-context/src/main/java/org/springframework/context/aot/ReflectiveProcessorAotContributionBuilder.java",
            "spring-context/src/main/java/org/springframework/context/aot/ReflectiveProcessorBeanFactoryInitializationAotProcessor.java",
            # Jolt parser gap: explicit constructor invocations using
            # `this(...)` or `super(...)`.
            "spring-context/src/main/java/org/springframework/context/support/ClassPathXmlApplicationContext.java",
            "spring-context/src/main/java/org/springframework/context/support/DefaultMessageSourceResolvable.java",
            "spring-context/src/main/java/org/springframework/context/support/FileSystemXmlApplicationContext.java",
            "spring-core/src/test/java/org/springframework/core/io/support/SpringFactoriesLoaderTests.java",
            "spring-jdbc/src/test/java/org/springframework/jdbc/object/SqlUpdateTests.java",
            "spring-web/src/main/java/org/springframework/web/HttpMediaTypeNotAcceptableException.java",
            "spring-web/src/main/java/org/springframework/web/HttpMediaTypeNotSupportedException.java",
            "spring-web/src/main/java/org/springframework/web/bind/MissingMatrixVariableException.java",
            "spring-web/src/main/java/org/springframework/web/bind/MissingPathVariableException.java",
            "spring-web/src/main/java/org/springframework/web/bind/MissingRequestCookieException.java",
            "spring-web/src/main/java/org/springframework/web/bind/MissingRequestHeaderException.java",
            "spring-web/src/main/java/org/springframework/web/bind/MissingServletRequestParameterException.java",
            "spring-web/src/main/java/org/springframework/web/bind/UnsatisfiedServletRequestParameterException.java",
            "spring-web/src/main/java/org/springframework/web/server/MethodNotAllowedException.java",
            "spring-web/src/main/java/org/springframework/web/server/MissingRequestValueException.java",
            "spring-web/src/main/java/org/springframework/web/server/NotAcceptableStatusException.java",
            "spring-web/src/main/java/org/springframework/web/server/ServerErrorException.java",
            "spring-web/src/main/java/org/springframework/web/server/UnsatisfiedRequestParameterException.java",
            "spring-web/src/main/java/org/springframework/web/server/UnsupportedMediaTypeStatusException.java",
            "spring-webflux/src/main/java/org/springframework/web/reactive/resource/NoResourceFoundException.java",
            "spring-webmvc/src/test/java/org/springframework/web/servlet/view/freemarker/FreeMarkerViewTests.java",
            # Jolt parser gap: `yield` statements in switch expression block
            # rules.
            "spring-core/src/main/java/org/springframework/aot/nativex/BasicJsonWriter.java",
            "spring-jdbc/src/main/java/org/springframework/jdbc/support/SQLErrorCodeSQLExceptionTranslator.java",
            "spring-test/src/main/java/org/springframework/mock/web/MockPageContext.java",
            "spring-web/src/testFixtures/java/org/springframework/web/testfixture/servlet/MockPageContext.java",
            # Jolt parser gap: array types and pattern variables in
            # `instanceof`.
            "spring-beans/src/test/java/org/springframework/beans/AbstractPropertyAccessorTests.java",
            "spring-core/src/main/java/org/springframework/asm/AnnotationWriter.java",
            "spring-core/src/main/java/org/springframework/core/annotation/TypeMappedAnnotation.java",
            "spring-core/src/main/java/org/springframework/core/convert/support/ByteBufferConverter.java",
            "spring-core/src/main/java/org/springframework/core/io/support/ResourceArrayPropertyEditor.java",
            "spring-core/src/test/java/org/springframework/util/xml/AbstractStaxXMLReaderTests.java",
            "spring-jdbc/src/main/java/org/springframework/jdbc/core/support/SqlLobValue.java",
            "spring-messaging/src/test/java/org/springframework/messaging/simp/stomp/AbstractStompBrokerRelayIntegrationTests.java",
            "spring-test/src/main/java/org/springframework/test/web/client/match/ContentRequestMatchers.java",
            "spring-web/src/main/java/org/springframework/web/multipart/support/ByteArrayMultipartFileEditor.java",
            # Jolt parser gap: relational expressions whose left side is a
            # selector, array access, or call, for example
            # `this.index < this.items.size()`.
            "spring-aop/src/main/java/org/springframework/aop/target/dynamic/AbstractRefreshableTargetSource.java",
            "spring-beans/src/testFixtures/java/org/springframework/beans/testfixture/beans/DerivedTestBean.java",
            "spring-context/src/main/java/org/springframework/scripting/support/ResourceScriptSource.java",
            "spring-core/src/jmh/java/org/springframework/util/ConcurrentLruCacheBenchmark.java",
            "spring-core/src/main/java/org/springframework/cglib/proxy/MethodProxy.java",
            "spring-core/src/main/java/org/springframework/core/MethodParameter.java",
            "spring-core/src/main/java/org/springframework/core/annotation/TypeMappedAnnotations.java",
            "spring-core/src/main/java/org/springframework/core/convert/support/GenericConversionService.java",
            "spring-core/src/main/java/org/springframework/core/io/buffer/LimitedDataBufferList.java",
            "spring-core/src/main/java/org/springframework/core/io/buffer/NettyDataBuffer.java",
            "spring-core/src/main/java/org/springframework/util/FastByteArrayOutputStream.java",
            "spring-core/src/main/java/org/springframework/util/backoff/ExponentialBackOff.java",
            "spring-core/src/main/java/org/springframework/util/unit/DataSize.java",
            "spring-core/src/main/java/org/springframework/util/xml/ListBasedXMLEventReader.java",
            "spring-expression/src/main/java/org/springframework/expression/spel/ast/CompoundExpression.java",
            "spring-expression/src/main/java/org/springframework/expression/spel/ast/OpMinus.java",
            "spring-expression/src/main/java/org/springframework/expression/spel/ast/OpPlus.java",
            "spring-expression/src/main/java/org/springframework/expression/spel/ast/TypeReference.java",
            "spring-expression/src/main/java/org/springframework/expression/spel/standard/Tokenizer.java",
            "spring-jdbc/src/main/java/org/springframework/jdbc/core/JdbcTemplate.java",
            "spring-jdbc/src/main/java/org/springframework/jdbc/core/simple/AbstractJdbcInsert.java",
            "spring-jdbc/src/main/java/org/springframework/jdbc/support/incrementer/AbstractIdentityColumnMaxValueIncrementer.java",
            "spring-messaging/src/main/java/org/springframework/messaging/simp/stomp/StompHeaderAccessor.java",
            "spring-messaging/src/main/java/org/springframework/messaging/support/MessageHeaderAccessor.java",
            "spring-web/src/main/java/org/springframework/http/client/support/ProxyFactoryBean.java",
            "spring-web/src/main/java/org/springframework/http/codec/protobuf/ProtobufDecoder.java",
            "spring-web/src/main/java/org/springframework/web/util/HtmlCharacterEntityDecoder.java",
            "spring-web/src/main/java/org/springframework/web/util/RfcUriParser.java",
            "spring-web/src/main/java/org/springframework/web/util/WhatWgUrlParser.java",
            "spring-web/src/main/java/org/springframework/web/util/pattern/InternalPathPatternParser.java",
            "spring-webflux/src/main/java/org/springframework/web/reactive/socket/adapter/JettyWebSocketSession.java",
            "spring-webflux/src/test/java/org/springframework/web/reactive/result/method/annotation/MessageReaderArgumentResolverTests.java",
        ],
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
            f"cd {q(tool_dir(corpus, 'dprint-jolt'))} && "
            f"dprint --plugins={q(DPRINT_PLUGIN)} fmt --incremental=false "
            "--skip-stable-format ."
        ),
        "dprint --version",
        ((DPRINT_PLUGIN, "release dprint plugin"),),
        lambda corpus: [
            f"cp {q(WORK / corpus / 'dprint.json')} "
            f"{q(tool_dir(corpus, 'dprint-jolt') / 'dprint.json')}"
        ],
    ),
    "google-java-format": Tool(
        "google-java-format --replace",
        lambda corpus: (
            "google-java-format --replace --skip-removing-unused-imports "
            f"@{q(WORK / corpus / 'google-java-format.args')}"
        ),
        "google-java-format --version",
        reset_commands=lambda corpus: [
            f"find {q(tool_dir(corpus, 'google-java-format'))} "
            f"-name '*.java' -print > {q(WORK / corpus / 'google-java-format.args')}"
        ],
    ),
    "prettier-java": Tool(
        "prettier --write --plugin prettier-plugin-java",
        lambda corpus: (
            "pnpm exec prettier --write "
            f"{q(str(tool_dir(corpus, 'prettier-java') / '**/*.java'))} "
            "--plugin prettier-plugin-java --print-width 80 --tab-width 2 "
            f"--ignore-path {q(WORK / corpus / 'prettier.ignore')} "
            "--log-level silent"
        ),
        "pnpm exec prettier --version",
        ((ROOT / "node_modules/.bin/prettier", "Prettier install"),),
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
        (WORK / name / "prettier.ignore").write_text("", encoding="utf-8")
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

    for source in sorted(corpus["source"].rglob("*.java")):
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
        prepare_baseline(name, tool_keys)
        summarize(name, corpus)
        versions = version_strings(tool_keys)
        rows = report_rows(name, tool_keys, versions)
        args = [
            "hyperfine",
            "--warmup",
            str(HYPERFINE_WARMUP),
            "--runs",
            str(HYPERFINE_RUNS),
            "--prepare",
            reset_command(name, tool_keys),
        ]
        for key in tool_keys:
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
            f"Hyperfine: {HYPERFINE_RUNS} runs, {HYPERFINE_WARMUP} warmup",
            "",
            output.rstrip(),
            "",
        ]
    )
    report.write_text(contents, encoding="utf-8")


def summarize(name: str, corpus: dict) -> None:
    files = list(baseline_dir(name).rglob("*.java"))
    total_bytes = sum(path.stat().st_size for path in files)
    log(
        f"benchmarking {name} ({corpus['description']}): "
        f"{len(files)} file(s), {total_bytes} byte(s)"
    )


def report_rows(
    name: str, tool_keys: tuple[ToolKey, ...], versions: dict[ToolKey, str]
) -> list[dict[str, str | int]]:
    log(f"collecting report rows for {name}")
    rows = []

    run_shell(reset_command(name, tool_keys), cwd=ROOT)
    for key in tool_keys:
        run_shell(TOOLS[key].benchmark_command(name), cwd=ROOT)
        input_files = len(java_files(tool_dir(name, key)))
        modified_files = count_modified_files(
            baseline_dir(name), tool_dir(name, key)
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
            f"{row['tool']}\t{row['version']}\t"
            f"{row['input_files']}\t{row['modified_files']}"
        )
    return "\n".join(lines)


def java_files(directory: Path) -> list[Path]:
    return sorted(directory.rglob("*.java"))


def count_modified_files(baseline: Path, formatted: Path) -> int:
    count = 0
    for path in java_files(baseline):
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
