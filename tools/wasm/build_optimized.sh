#!/bin/sh
set -eu

output=${1:-target/wasm32-unknown-unknown/release/jolt_fmt_dprint.opt.wasm}
input=target/wasm32-unknown-unknown/release/jolt_fmt_dprint.wasm

rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown --package jolt_fmt_dprint --features wasm

if command -v wasm-opt >/dev/null 2>&1; then
  wasm_opt=wasm-opt
else
  if [ "$(uname -s)" != Linux ] || [ "$(uname -m)" != x86_64 ]; then
    echo "wasm-opt is required; run 'mise install' before this task" >&2
    exit 1
  fi
  binaryen=target/binaryen-version_130
  archive=target/binaryen-version_130-x86_64-linux.tar.gz
  if [ ! -x "$binaryen/bin/wasm-opt" ]; then
    curl --proto '=https' --tlsv1.2 -LsSf \
      https://github.com/WebAssembly/binaryen/releases/download/version_130/binaryen-version_130-x86_64-linux.tar.gz \
      -o "$archive"
    printf '%s  %s\n' \
      0a18362361ad05465118cd8eeb72edaeec89de6894bc283576ef4e07aa3babcc \
      "$archive" | sha256sum -c -
    tar -xzf "$archive" -C target
  fi
  wasm_opt=$binaryen/bin/wasm-opt
fi

case $("$wasm_opt" --version) in
  *"version 130"*) ;;
  *) echo "expected wasm-opt version 130: $wasm_opt" >&2; exit 1 ;;
esac

"$wasm_opt" -O3 "$input" -o "$output"

if command -v dprint >/dev/null 2>&1; then
  dprint=$(command -v dprint)
elif [ "$(uname -s)" = Linux ] && [ "$(uname -m)" = x86_64 ]; then
  dprint_dir=target/dprint-0.54.0
  dprint=$PWD/$dprint_dir/dprint
  if [ ! -x "$dprint" ]; then
    archive=target/dprint-0.54.0-x86_64-linux-musl.zip
    curl --proto '=https' --tlsv1.2 -LsSf \
      https://github.com/dprint/dprint/releases/download/0.54.0/dprint-x86_64-unknown-linux-musl.zip \
      -o "$archive"
    printf '%s  %s\n' \
      859ae94e596105201faa59a3fb4bedc8316e226e3e154ae410f9373461e1e41c \
      "$archive" | sha256sum -c -
    mkdir -p "$dprint_dir"
    unzip -q -o "$archive" -d "$dprint_dir"
  fi
else
  echo "dprint 0.54.0 is required; run 'mise install' before this task" >&2
  exit 1
fi

case $("$dprint" --version) in
  *"0.54.0"*) ;;
  *) echo "expected dprint 0.54.0: $dprint" >&2; exit 1 ;;
esac

plugin=$(cd "$(dirname "$output")" && pwd)/$(basename "$output")
smoke=$(mktemp -d)
trap 'rm -rf "$smoke"' EXIT HUP INT TERM
printf 'class Smoke{}\n' > "$smoke/Smoke.java"
printf 'class Smoke{val answer=42}\n' > "$smoke/Smoke.kt"
(
  cd "$smoke"
  "$dprint" --plugins="$plugin" fmt Smoke.java Smoke.kt >/dev/null
)
grep -q 'class Smoke {' "$smoke/Smoke.java"
grep -q 'val answer = 42' "$smoke/Smoke.kt"
