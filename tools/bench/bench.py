#!/usr/bin/env python3
"""Run formatter benchmarks over imported benchmark corpora."""

import fnmatch
import json
import os
import platform
import shutil
import subprocess
import sys
from pathlib import Path, PurePosixPath
from typing import Literal, NamedTuple

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

ToolKey = Literal["jolt", "dprint-jolt", "google-java-format", "prettier-java"]


class Tool(NamedTuple):
    label: str
    benchmark_command: str
    version_command: str


TOOLS: dict[ToolKey, Tool] = {
    "jolt": Tool(
        "jolt fmt",
        "{jolt} fmt {jolt_dir}",
        "{jolt} --version",
    ),
    "dprint-jolt": Tool(
        "dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format",
        "cd {dprint_jolt_dir} && dprint --plugins={dprint_plugin} fmt --incremental=false --skip-stable-format .",
        "dprint --version",
    ),
    # DISABLED to speed up iteration optimizing jolt itself
    # "google-java-format": Tool(
    #     "google-java-format --replace",
    #     "google-java-format --replace --skip-removing-unused-imports @{gjf_args}",
    #     "google-java-format --version",
    # ),
    # "prettier-java": Tool(
    #     "prettier --write --plugin prettier-plugin-java",
    #     "pnpm exec prettier --write {prettier_glob} --plugin prettier-plugin-java --print-width 80 --tab-width 2 --ignore-path {prettier_ignore} --log-level silent",
    #     "pnpm exec prettier --version",
    # ),
}


def main() -> int:
    if len(sys.argv) != 1:
        raise RuntimeError("benchmark does not take arguments")
    benchmark()
    return 0


def prepare_baseline(name: str) -> None:
    corpus = CORPORA[name]
    source = corpus["source"]
    if not source.is_dir():
        raise RuntimeError(f"missing benchmark import: {source}")

    copy_corpus(corpus, baseline_dir(name))
    (WORK / name).mkdir(parents=True, exist_ok=True)
    (WORK / name / "prettier.ignore").write_text("", encoding="utf-8")
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
    posix = PurePosixPath(path.as_posix())
    return any(
        posix.match(pattern) or fnmatch.fnmatchcase(posix.as_posix(), pattern)
        for pattern in patterns
    )


def benchmark() -> None:
    require(JOLT, "release Jolt CLI")
    require(DPRINT_PLUGIN, "release dprint plugin")
    require(ROOT / "node_modules/.bin/prettier", "Prettier install")

    for name, corpus in CORPORA.items():
        prepare_baseline(name)
        summarize(name, corpus)
        metadata = collect_metadata(name)
        args = [
            "hyperfine",
            "--warmup",
            str(HYPERFINE_WARMUP),
            "--runs",
            str(HYPERFINE_RUNS),
            "--prepare",
            reset_command(name),
        ]
        for tool in TOOLS.values():
            args += [
                "-n",
                tool.label,
                tool.benchmark_command.format_map(context(name)),
            ]
        write_report(name, corpus, metadata, run_capture(*args, cwd=ROOT))


def context(name: str) -> dict[str, str]:
    return {
        "jolt": q(JOLT),
        "jolt_dir": q(tool_dir(name, "jolt")),
        "dprint_jolt_dir": q(tool_dir(name, "dprint-jolt")),
        "dprint_plugin": q(DPRINT_PLUGIN),
        "gjf_args": q(WORK / name / "google-java-format.args"),
        "prettier_glob": q(str(tool_dir(name, "prettier-java") / "**/*.java")),
        "prettier_ignore": q(WORK / name / "prettier.ignore"),
    }


def write_report(name: str, corpus: dict, metadata: dict, output: str) -> None:
    REPORTS.mkdir(parents=True, exist_ok=True)
    report = REPORTS / f"{name}.md"
    log(f"writing hyperfine report: {report}")
    contents = (
        f"# {name}\n\n"
        f"{corpus['description']}.\n\n"
        f"{format_metadata(metadata)}\n"
        "```text\n"
        f"{output.rstrip()}\n"
        "```\n"
    )
    report.write_text(contents, encoding="utf-8")
    run("dprint", "fmt", report, cwd=ROOT)


def summarize(name: str, corpus: dict) -> None:
    files = list(baseline_dir(name).rglob("*.java"))
    total_bytes = sum(path.stat().st_size for path in files)
    log(
        f"benchmarking {name} ({corpus['description']}): "
        f"{len(files)} file(s), {total_bytes} byte(s)"
    )


def collect_metadata(name: str) -> dict:
    log(f"collecting metadata for {name}")
    versions = version_strings(name)
    rows = []

    run_shell(reset_command(name), cwd=ROOT)
    for key, tool in TOOLS.items():
        input_files = len(java_files(tool_dir(name, key)))
        run_shell(tool.benchmark_command.format_map(context(name)), cwd=ROOT)
        rows.append(
            {
                "tool": key,
                "version": versions[key],
                "input_files": input_files,
                "modified_files": count_modified_files(
                    baseline_dir(name), tool_dir(name, key)
                ),
            }
        )

    return {
        "rows": rows,
        "system": system_info(),
        "hyperfine": {
            "runs": HYPERFINE_RUNS,
            "warmup": HYPERFINE_WARMUP,
        },
    }


def version_strings(name: str) -> dict[str, str]:
    versions = {}
    for key, tool in TOOLS.items():
        try:
            output = run_shell_capture(
                tool.version_command.format_map(context(name)), cwd=ROOT
            )
            versions[key] = " ".join(output.split()) or "unknown"
        except subprocess.CalledProcessError:
            versions[key] = "unknown"
    return versions


def format_metadata(metadata: dict) -> str:
    lines = [
        "## Metadata",
        "",
        "| Tool | Version | Input files | Modified files |",
        "| --- | --- | ---: | ---: |",
    ]
    for row in metadata["rows"]:
        lines.append(
            f"| {row['tool']} | {row['version']} | "
            f"{row['input_files']} | {row['modified_files']} |"
        )
    system = metadata["system"]
    hyperfine = metadata["hyperfine"]
    lines += [
        "",
        f"System: {system}.",
        f"Hyperfine: {hyperfine['runs']} runs, {hyperfine['warmup']} warmup.",
        "",
    ]
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
    parts = [
        f"{platform.system()} {platform.release()}",
        platform.machine(),
    ]
    cpu = cpu_name()
    if cpu:
        parts.append(cpu)
    logical_cpus = os.cpu_count()
    if logical_cpus is not None:
        parts.append(f"{logical_cpus} logical CPUs")
    memory = memory_gb()
    if memory is not None:
        parts.append(f"{memory:.0f} GB memory")
    return ", ".join(parts)


def cpu_name() -> str | None:
    if platform.system() == "Linux":
        cpuinfo = Path("/proc/cpuinfo")
        if cpuinfo.exists():
            for line in cpuinfo.read_text(encoding="utf-8").splitlines():
                if line.startswith("model name"):
                    return line.split(":", 1)[1].strip()
    if platform.system() == "Darwin":
        try:
            return run_shell_capture(
                "sysctl -n machdep.cpu.brand_string"
            ).strip()
        except subprocess.CalledProcessError:
            pass
    return platform.processor() or None


def memory_gb() -> float | None:
    if platform.system() == "Linux":
        meminfo = Path("/proc/meminfo")
        if meminfo.exists():
            for line in meminfo.read_text(encoding="utf-8").splitlines():
                if line.startswith("MemTotal:"):
                    kib = int(line.split()[1])
                    return kib / 1024 / 1024
    if platform.system() == "Darwin":
        try:
            bytes_total = int(run_shell_capture("sysctl -n hw.memsize").strip())
            return bytes_total / 1024 / 1024 / 1024
        except subprocess.CalledProcessError:
            pass
    return None


def tool_dir(corpus: str, tool: str) -> Path:
    return WORK / corpus / tool


def baseline_dir(corpus: str) -> Path:
    return WORK / corpus / "baseline"


def reset_command(corpus: str) -> str:
    baseline = q(baseline_dir(corpus))
    dprint_config = q(WORK / corpus / "dprint.json")
    gjf_args = q(WORK / corpus / "google-java-format.args")
    parts = []
    for tool in TOOLS:
        parts.append(f"rm -rf {q(tool_dir(corpus, tool))}")
        parts.append(f"cp -R {baseline} {q(tool_dir(corpus, tool))}")
    parts.append(
        f"cp {dprint_config} {q(tool_dir(corpus, 'dprint-jolt') / 'dprint.json')}"
    )
    parts.append(
        f"find {q(tool_dir(corpus, 'google-java-format'))} -name '*.java' -print > {gjf_args}"
    )
    return " && ".join(parts)


def require(path: Path, label: str) -> None:
    if not path.exists():
        raise RuntimeError(f"missing {label}: {path}")


def q(path: str | Path) -> str:
    return "'" + str(path).replace("'", "'\"'\"'") + "'"


def run(*command: str | Path, cwd: Path | None = None) -> None:
    args = [str(arg) for arg in command]
    print("+ " + " ".join(args), file=sys.stderr)
    subprocess.run(args, cwd=cwd, check=True)


def run_shell(command: str, cwd: Path | None = None) -> None:
    print("+ " + command, file=sys.stderr)
    subprocess.run(command, cwd=cwd, shell=True, check=True)


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
