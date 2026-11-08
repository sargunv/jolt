#!/bin/sh
# The formatter's output at a baseline revision (default origin/main) over the
# imported Java corpora, to review what a change does to real code.
#
#   tools/baseline.sh build [revision]
#     Recreates tools/import/.imports/.baseline: builds the CLI at the revision
#     in a temporary worktree into `jolt`, copies the corpora into `input/`,
#     and formats them into `output/` (width 80) and `output-w100/` (Spring at
#     width 100). `bench/spring-framework` is a copy of Spring for timing.
#     Needs the corpora (`mise run` imports them).
#
#   tools/baseline.sh compare <jolt binary> [suffix]
#     Formats the inputs with another binary into `output-<suffix>/` and
#     prints how many files differ from the baseline per corpus
#     (`mise run jref --input ... --expected ...` reports them file by file).
set -eu
cd "$(dirname "$0")/.."
root=$PWD

imports=$root/tools/import/.imports
base=$imports/.baseline
corpora="google-java-format palantir-java-format prettier-java spring-framework"

build() {
  revision=$(git rev-parse --verify "${1:-origin/main}^{commit}")
  for corpus in $corpora; do
    [ -d "$imports/$corpus" ] || {
      echo "missing corpus $imports/$corpus" >&2
      exit 1
    }
  done
  tree=$(mktemp -d)
  trap 'git worktree remove --force "$tree"' EXIT
  git worktree add --detach "$tree" "$revision"
  # Builds into this checkout's target directory, which keeps the build
  # incremental and out of the shared /tmp.
  (cd "$tree" && CARGO_TARGET_DIR="$root/target/baseline" cargo build --release -p jolt_cli)
  rm -rf "$base"
  mkdir -p "$base/bench"
  cp target/baseline/release/jolt "$base/jolt"
  echo "$revision" >"$base/revision"
  for corpus in $corpora; do
    mkdir -p "$base/input/$corpus"
    cp -a "$imports/$corpus/." "$base/input/$corpus/"
  done
  format "$base/jolt" output
  cp -a "$imports/spring-framework" "$base/bench/spring-framework"
}

# Formats the inputs with $1 into `$2/` and `$2-w100/`. Files with syntax
# errors are left untouched, and the CLI exits 1 for them.
format() {
  for corpus in $corpora; do
    out=$base/$2/$corpus
    rm -rf "$out"
    mkdir -p "$out"
    cp -a "$base/input/$corpus/." "$out/"
    "$1" format --no-config "$out" >"$base/logs-$2-$corpus.txt" 2>&1 || true
  done
  out=$base/$2-w100/spring-framework
  rm -rf "$out"
  mkdir -p "$out"
  cp -a "$base/input/spring-framework/." "$out/"
  "$1" format --no-config --line-width 100 "$out" \
    >"$base/logs-$2-spring-w100.txt" 2>&1 || true
}

compare() {
  suffix=output-${2:-new}
  format "$(realpath "$1")" "$suffix"
  for corpus in $corpora; do
    echo "$corpus: $(diff -rq "$base/output/$corpus" "$base/$suffix/$corpus" | wc -l) files differ"
  done
  echo "spring-framework (w100): $(diff -rq "$base/output-w100/spring-framework" "$base/$suffix-w100/spring-framework" | wc -l) files differ"
}

case ${1:-} in
build) build "${2:-}" ;;
compare)
  [ $# -ge 2 ] || {
    echo "usage: $0 compare <jolt binary> [suffix]" >&2
    exit 2
  }
  compare "$2" "${3:-new}"
  ;;
*)
  echo "usage: $0 build [revision] | compare <jolt binary> [suffix]" >&2
  exit 2
  ;;
esac
