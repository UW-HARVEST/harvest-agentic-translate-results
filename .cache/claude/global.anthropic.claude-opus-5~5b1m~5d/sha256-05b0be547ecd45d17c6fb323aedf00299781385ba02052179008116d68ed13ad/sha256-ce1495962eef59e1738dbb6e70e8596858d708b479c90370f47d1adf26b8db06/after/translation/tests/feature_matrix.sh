#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY feature combination
# and under both build profiles (the dev profile the tests default to, and the
# release profile that actually ships: opt-level 3 + panic = "abort").
#
# The feature list is extracted from Cargo.toml rather than hard-coded, so this
# stays correct if features are ever added.
set -u -o pipefail

cd "$(dirname "$0")/.." || exit 1
ROOT=$(pwd)

echo "=== enumerating features from Cargo.toml ==="
FEATURES=$(awk '
  /^\[features\]/ { inf=1; next }
  /^\[/           { inf=0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { sub(/[[:space:]]*=.*/,""); print }
' Cargo.toml | grep -v '^default$' | sort -u)

if [ -z "$FEATURES" ]; then
  echo "no [features] declared -> the feature power set is exactly one element (default)"
else
  echo "features: $FEATURES"
fi

# Build the power set of the declared features.
COMBOS=()
if [ -z "$FEATURES" ]; then
  COMBOS+=("")                       # default
  COMBOS+=("--no-default-features")  # explicitly empty
  COMBOS+=("--all-features")         # equals default here, but verified anyway
else
  mapfile -t FARR <<<"$FEATURES"
  n=${#FARR[@]}
  total=$((1 << n))
  for ((mask = 0; mask < total; mask++)); do
    set=""
    for ((i = 0; i < n; i++)); do
      if (( (mask >> i) & 1 )); then
        set="${set:+$set,}${FARR[$i]}"
      fi
    done
    COMBOS+=("--no-default-features${set:+ --features $set}")
  done
  COMBOS+=("")
  COMBOS+=("--all-features")
fi

FAIL=0
run() {
  local desc="$1"; shift
  echo
  echo "--------------------------------------------------------------"
  echo ">>> $desc"
  echo "    cargo $*"
  echo "--------------------------------------------------------------"
  # `tail` must be generous enough to keep every `test result:` line of every
  # test binary, otherwise a suite could silently vanish from the report.
  if ! timeout 600 cargo "$@" 2>&1 | tail -n 400; then
    echo "!!! FAILED: $desc"
    FAIL=1
  fi
}

for combo in "${COMBOS[@]}"; do
  label="${combo:-<default features>}"
  # shellcheck disable=SC2086
  run "check [$label]" check --tests $combo
  # `cargo test` does not build a cdylib, so build the .so under test explicitly
  # for each profile; the harness refuses to fall back to the other profile.
  # shellcheck disable=SC2086
  run "build dev cdylib [$label]" build $combo
  # shellcheck disable=SC2086
  run "dev-profile differential suite [$label]" test $combo -- --test-threads=4
  # shellcheck disable=SC2086
  run "build release cdylib [$label]" build --release $combo
  # shellcheck disable=SC2086
  run "release-profile differential suite [$label]" test --release $combo -- --test-threads=4
done

echo
echo "=== symbol parity for every profile's cdylib ==="
C_SO=$(ls "$ROOT"/../c_src/build/lib*.so | head -n1)
for prof in debug release; do
  R_SO="$ROOT/target/$prof/libbin2hex_lib.so"
  [ -f "$R_SO" ] || { echo "($prof cdylib not built, skipping)"; continue; }
  cdefs=$(nm -D --defined-only "$C_SO" | awk '$1!="w"{print $NF}' | sed 's/@.*//' | sort -u)
  rdefs=$(nm -D --defined-only "$R_SO" | awk '$1!="w"{print $NF}' | sed 's/@.*//' | sort -u)
  missing=$(comm -23 <(echo "$cdefs") <(echo "$rdefs"))
  echo "profile=$prof  C symbols: $(echo "$cdefs" | wc -l)  missing from Rust: $(echo -n "$missing" | grep -c . || true)"
  if [ -n "$missing" ]; then
    echo "!!! MISSING: $missing"
    FAIL=1
  fi
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "SOME COMBINATIONS FAILED"
fi
exit "$FAIL"
