#!/bin/sh
# Builds the release dprint plugin, shrinks it with wasm-opt (about 17%), and
# checks that dprint loads it. Writes $1, by default target/plugin.wasm.
#
# wasm-opt and dprint are the mise.toml tools. The release job that builds
# plugin.wasm has no mise, so there the Linux x64 builds that mise.lock pins
# are downloaded and checked against its checksums.
set -eu
cd "$(dirname "$0")/.."

input=target/wasm32-unknown-unknown/release/jolt_fmt_dprint.wasm
output=${1:-target/plugin.wasm}

# fetch <mise tool> <binary>: prints the path of the tool's binary.
fetch() {
  if command -v "$2" >/dev/null 2>&1; then
    command -v "$2"
    return
  fi
  dir=$PWD/target/tools/$(printf %s "$1" | tr ':/' '__')
  if [ ! -d "$dir" ]; then
    lock() {
      awk -v section="[tools.\"$1\".\"platforms.linux-x64\"]" -v key="$2" '
        $0 == section { found = 1; next }
        /^\[/ { found = 0 }
        found && $1 == key { gsub(/"/, "", $3); print $3 }
      ' mise.lock
    }
    url=$(lock "$1" url)
    checksum=$(lock "$1" checksum)
    [ -n "$url" ] && [ -n "$checksum" ] || {
      echo "no linux-x64 url and checksum for $1 in mise.lock" >&2
      exit 1
    }
    # Extract next to the final directory and rename it into place, so an
    # interrupted download or extract never leaves a partial directory.
    partial=$dir.partial
    rm -rf "$partial" "$dir.archive"
    mkdir -p "$partial"
    curl --proto '=https' --tlsv1.2 -LsSf "$url" -o "$dir.archive"
    printf '%s  %s\n' "${checksum#sha256:}" "$dir.archive" | sha256sum -c - >&2
    case $url in
      *.zip) unzip -q "$dir.archive" -d "$partial" ;;
      *) tar -xzf "$dir.archive" -C "$partial" ;;
    esac
    rm "$dir.archive"
    mv "$partial" "$dir"
  fi
  binary=$(find "$dir" -name "$2" -type f | head -n 1)
  [ -n "$binary" ] || {
    echo "no $2 in $dir; delete it and rerun" >&2
    exit 1
  }
  printf '%s\n' "$binary"
}

wasm_opt=$(fetch aqua:WebAssembly/binaryen wasm-opt)
dprint=$(fetch aqua:dprint/dprint dprint)
command -v rustup >/dev/null 2>&1 && rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown --package jolt_fmt_dprint

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
"$wasm_opt" -O3 "$input" -o "$work/plugin.wasm"
printf 'class Smoke{}\n' > "$work/Smoke.java"
(cd "$work" && "$dprint" --plugins="$work/plugin.wasm" fmt Smoke.java >/dev/null)
grep -q 'class Smoke {' "$work/Smoke.java"
cp "$work/plugin.wasm" "$output"
