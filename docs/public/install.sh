#!/bin/sh
set -eu

# GitHub Pages cannot issue configurable HTTP redirects, so this stable
# endpoint downloads and runs the installer attached to the latest release.
installer_url='https://github.com/sargunv/jolt/releases/latest/download/jolt_cli-installer.sh'
installer=$(mktemp "${TMPDIR:-/tmp}/jolt-installer.XXXXXX")
trap 'rm -f "$installer"' 0 1 2 15

curl --proto '=https' --tlsv1.2 --location --silent --show-error --fail \
  --output "$installer" "$installer_url"
sh "$installer" "$@"
