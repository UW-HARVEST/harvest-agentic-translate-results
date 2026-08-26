#!/bin/sh
# Phase A / Phase D — enumerate every build-time configuration and check + test
# each one.
#
# The feature list is extracted from Cargo.toml rather than hard-coded, so this
# keeps working if features are ever added. Today the [features] table is absent,
# which means the only configuration is the empty feature set; the script still
# runs the --no-default-features and --all-features spellings explicitly so the
# claim "there is exactly one configuration" is verified instead of assumed.

set -eu

cd "$(dirname "$0")"

CARGO_FLAGS="--offline"

features=$(awk '
  /^\[features\]/ { inside = 1; next }
  /^\[/           { inside = 0 }
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }
' Cargo.toml)

echo "== declared features =="
if [ -z "$features" ]; then
  echo "(none — the crate declares no [features], so there is one configuration)"
else
  echo "$features"
fi

# Build the list of combinations: always the empty set, plus the power set of any
# declared features.
combos_file=$(mktemp "${TMPDIR:-/tmp}/combos.XXXXXX")
printf '%s\n' "" >"$combos_file"

if [ -n "$features" ]; then
  set -- $features
  n=$#
  total=$((1 << n))
  i=1
  while [ "$i" -lt "$total" ]; do
    combo=""
    j=0
    for f in $features; do
      if [ $(( (i >> j) & 1 )) -eq 1 ]; then
        combo="${combo:+$combo,}$f"
      fi
      j=$((j + 1))
    done
    printf '%s\n' "$combo" >>"$combos_file"
    i=$((i + 1))
  done
fi

status=0

run() {
  label="$1"; shift
  printf '\n---- %s ----\n' "$label"
  if "$@"; then
    echo "PASS: $label"
  else
    echo "FAIL: $label"
    status=1
  fi
}

# `cargo test` alone never builds the cdylib (no test target depends on it), so
# every combination is built first: that way the differential tests load the real
# `target/<profile>/libdriver.so` artifact rather than build.rs's fallback copy.
while IFS= read -r combo; do
  if [ -z "$combo" ]; then
    run "cargo check --no-default-features" \
        cargo check $CARGO_FLAGS --no-default-features --all-targets
    run "cargo build --no-default-features" \
        cargo build $CARGO_FLAGS --no-default-features
    run "cargo test  --no-default-features" \
        cargo test $CARGO_FLAGS --no-default-features
  else
    run "cargo check --no-default-features --features $combo" \
        cargo check $CARGO_FLAGS --no-default-features --features "$combo" --all-targets
    run "cargo build --no-default-features --features $combo" \
        cargo build $CARGO_FLAGS --no-default-features --features "$combo"
    run "cargo test  --no-default-features --features $combo" \
        cargo test $CARGO_FLAGS --no-default-features --features "$combo"
  fi
done <"$combos_file"

rm -f "$combos_file"

# The default and --all-features spellings, which for this crate are the same
# configuration as the empty set but are checked rather than assumed.
run "cargo check (default features)"  cargo check $CARGO_FLAGS --all-targets
run "cargo build (default features)"  cargo build $CARGO_FLAGS
run "cargo test  (default features)"  cargo test  $CARGO_FLAGS
run "cargo check --all-features"      cargo check $CARGO_FLAGS --all-features --all-targets
run "cargo build --all-features"      cargo build $CARGO_FLAGS --all-features
run "cargo test  --all-features"      cargo test  $CARGO_FLAGS --all-features

# The release profile is a distinct code-generation configuration (panic =
# "abort", optimisations on); make sure the translation is correct there too.
run "cargo build --release"           cargo build $CARGO_FLAGS --release
run "cargo test --release"            cargo test  $CARGO_FLAGS --release

printf '\n==== overall: %s ====\n' "$([ $status -eq 0 ] && echo PASS || echo FAIL)"
exit $status
