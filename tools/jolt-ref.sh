#!/bin/sh
# The formatter's regression reference: the output of the formatter as it was
# at commit de12e2cb, before the rewrite, over the imported Java corpora.
#
#   tools/jolt-ref.sh build
#     Recreates tools/import/.imports/.jolt-ref: builds `jolt-old` from
#     de12e2cb in a temporary worktree, copies the corpora into `input/`, and
#     formats them into `old-output/` (width 80) and `old-output-w100/`
#     (Spring at width 100). `bench/spring-framework` is a copy of Spring for
#     timing. Needs the corpora (`mise run` imports them) and the commit.
#
#   tools/jolt-ref.sh compare <jolt binary> [suffix]
#     Formats the inputs with another binary into `output-<suffix>/` and
#     prints how many files differ from the reference per corpus. Differences
#     must be explained by "Deliberate changes" in docs/internals/java-style.md
#     (`mise run jref` reports them file by file).
set -eu
cd "$(dirname "$0")/.."

commit=de12e2cb7f106d0d7c0350d0da31fe8eca01981c
imports=$PWD/tools/import/.imports
ref=$imports/.jolt-ref
corpora="google-java-format palantir-java-format prettier-java spring-framework"

build() {
  for corpus in $corpora; do
    [ -d "$imports/$corpus" ] || {
      echo "missing corpus $imports/$corpus" >&2
      exit 1
    }
  done
  tree=$(mktemp -d)
  trap 'git worktree remove --force "$tree"' EXIT
  git worktree add --detach "$tree" "$commit"
  (cd "$tree" && cargo build --release -p jolt_cli)
  rm -rf "$ref"
  mkdir -p "$ref/bench"
  cp "$tree/target/release/jolt" "$ref/jolt-old"
  for corpus in $corpora; do
    mkdir -p "$ref/input/$corpus" "$ref/old-output/$corpus"
    cp -a "$imports/$corpus/." "$ref/input/$corpus/"
    cp -a "$imports/$corpus/." "$ref/old-output/$corpus/"
    # The reference leaves files with syntax errors untouched and exits 1.
    "$ref/jolt-old" format --no-config "$ref/old-output/$corpus" \
      >"$ref/logs-$corpus.txt" 2>&1 || true
  done
  mkdir -p "$ref/old-output-w100/spring-framework"
  cp -a "$imports/spring-framework/." "$ref/old-output-w100/spring-framework/"
  "$ref/jolt-old" format --no-config --line-width 100 \
    "$ref/old-output-w100/spring-framework" >"$ref/logs-spring-w100.txt" 2>&1 || true
  cp -a "$imports/spring-framework" "$ref/bench/spring-framework"
}

compare() {
  binary=$(realpath "$1")
  suffix=${2:-new}
  for corpus in $corpora; do
    out=$ref/output-$suffix/$corpus
    rm -rf "$out"
    mkdir -p "$out"
    cp -a "$ref/input/$corpus/." "$out/"
    "$binary" format --no-config "$out" >"$ref/logs-$suffix-$corpus.txt" 2>&1 || true
    echo "$corpus: $(diff -rq "$ref/old-output/$corpus" "$out" | wc -l) files differ"
  done
  out=$ref/output-$suffix-w100/spring-framework
  rm -rf "$out"
  mkdir -p "$out"
  cp -a "$ref/input/spring-framework/." "$out/"
  "$binary" format --no-config --line-width 100 "$out" \
    >"$ref/logs-$suffix-spring-w100.txt" 2>&1 || true
  echo "spring-framework (w100): $(diff -rq "$ref/old-output-w100/spring-framework" "$out" | wc -l) files differ"
}

case ${1:-} in
build) build ;;
compare)
  [ $# -ge 2 ] || {
    echo "usage: $0 compare <jolt binary> [suffix]" >&2
    exit 2
  }
  compare "$2" "${3:-new}"
  ;;
*)
  echo "usage: $0 build | compare <jolt binary> [suffix]" >&2
  exit 2
  ;;
esac
