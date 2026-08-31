#!/bin/sh
# Phase D: run the whole differential suite under EVERY cargo feature
# combination declared in Cargo.toml.
#
# The crate declares no [features] section at all, so the cross-product is the
# single default configuration; the loop below is derived from Cargo.toml rather
# than hard-coded, so it stays correct if features are ever added.
set -eu
cd "$(dirname "$0")"

FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, ""); if ($0 != "default") print
  }' Cargo.toml)

echo "features declared in Cargo.toml: [${FEATURES:-none}]"

run() {
  timeout 900 cargo build --offline   # the cdylib is loaded via dlsym, so build it explicitly
  echo "=============================================================="
  echo "=== cargo test --offline $*"
  echo "=============================================================="
  timeout 3600 cargo test --offline "$@" -- --test-threads=4
}

# 1. default configuration
run

if [ -n "$FEATURES" ]; then
  # 2. no default features
  run --no-default-features
  # 3. every non-empty subset of the declared features
  set -- $FEATURES
  n=$#
  total=$((1 << n))
  i=1
  while [ "$i" -lt "$total" ]; do
    combo=""
    j=0
    for f in $FEATURES; do
      if [ $(((i >> j) & 1)) -eq 1 ]; then
        combo="$combo,$f"
      fi
      j=$((j + 1))
    done
    run --no-default-features --features "${combo#,}"
    i=$((i + 1))
  done
  # 4. all features at once
  run --all-features
fi

echo
echo "ALL FEATURE COMBINATIONS PASSED"
