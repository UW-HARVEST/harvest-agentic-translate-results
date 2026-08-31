#!/usr/bin/env bash
# Enumerate every valid Cargo feature combination and `cargo check` each one.
#
# The crate declares no [features] section, so there is exactly one valid
# configuration: the default (empty) feature set. This matches the C build,
# whose CMakeLists.txt fixes PCRE2_CODE_UNIT_WIDTH=8 and SUPPORT_UNICODE and
# whose src/config.h fixes LINK_SIZE=2, NEWLINE_DEFAULT=2, PARENS_NEST_LIMIT=250
# and leaves SUPPORT_JIT / EBCDIC / BSR_ANYCRLF undefined.
set -euo pipefail

cd "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Extract feature names from the [features] table, ignoring "default".
features=$(awk '
  /^\[features\]/ { inside=1; next }
  /^\[/           { inside=0 }
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' Cargo.toml)

if [ -z "$features" ]; then
  echo "Cargo.toml declares no [features]: 1 valid combination (default)."
  echo "==> cargo check --no-default-features"
  timeout 600 cargo check --no-default-features 2>&1 | tail -n 3
  echo "==> cargo check (default features)"
  timeout 600 cargo check 2>&1 | tail -n 3
  echo "==> cargo build --release"
  timeout 600 cargo build --release 2>&1 | tail -n 3
  exit 0
fi

# Otherwise check the power set of the declared features.
mapfile -t list <<< "$features"
n=${#list[@]}
echo "Found $n features: ${list[*]} -> $((1 << n)) combinations"
for (( mask = 0; mask < (1 << n); mask++ )); do
  combo=""
  for (( i = 0; i < n; i++ )); do
    if (( mask & (1 << i) )); then
      combo="${combo:+$combo,}${list[$i]}"
    fi
  done
  echo "==> cargo check --no-default-features --features '${combo}'"
  timeout 600 cargo check --no-default-features --features "$combo" 2>&1 | tail -n 2
done
