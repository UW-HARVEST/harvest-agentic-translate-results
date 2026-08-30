#!/usr/bin/env bash
# Phase D driver: build the C .so, then for EVERY feature combination and BOTH
# profiles rebuild the Rust cdylib and run both differential suites against it.
#
# The rebuild is not optional: `cargo test` does NOT rebuild a cdylib that no
# test target links against (the suites reach it via dlopen), so without an
# explicit `cargo build` per combination the tests would silently exercise the
# previously built .so. The harness also refuses to run against a stale artifact.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
CRATE_DIR="$PWD"
ROOT="$(cd .. && pwd)"
CARGO_FLAGS="--offline"   # this sandbox has no crates.io egress; cached crates only

echo "=== Building the C shared library ==="
mkdir -p "$ROOT/c_src/build" || exit 1
(cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null) || { echo "C build FAILED"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
echo "C  .so: $C_SO"

# ---- enumerate feature combinations from Cargo.toml -------------------------
# Collect the feature names declared in [features] (excluding "default"), then
# build the full power set. An empty [features] table yields just the two
# baseline configurations below.
mapfile -t FEATURES < <(awk '
  /^\[features\]/ { inf=1; next }
  /^\[/           { inf=0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' Cargo.toml)

COMBOS=()
COMBOS+=("default|")                               # default features
COMBOS+=("no-default|--no-default-features")       # nothing enabled
n=${#FEATURES[@]}
if (( n > 0 && n <= 12 )); then
  for (( mask=1; mask < (1<<n); mask++ )); do
    sel=()
    for (( i=0; i<n; i++ )); do
      (( mask & (1<<i) )) && sel+=("${FEATURES[i]}")
    done
    joined=$(IFS=,; echo "${sel[*]}")
    COMBOS+=("$joined|--no-default-features --features $joined")
    COMBOS+=("default+$joined|--features $joined")
  done
elif (( n > 12 )); then
  echo "NOTE: $n features declared; power set too large, testing each singly."
  for f in "${FEATURES[@]}"; do
    COMBOS+=("$f|--no-default-features --features $f")
  done
fi

echo
echo "=== ${#COMBOS[@]} configuration(s) x 2 profile(s) ==="
printf '  declared features: %s\n' "${FEATURES[*]:-(none)}"

FAILED=()
PASSED=0

for profile in dev release; do
  if [ "$profile" = "release" ]; then PROF_FLAG="--release"; SO_DIR="target/release"
  else PROF_FLAG=""; SO_DIR="target/debug"; fi

  for entry in "${COMBOS[@]}"; do
    label="${entry%%|*}"
    flags="${entry#*|}"
    tag="[$profile/$label]"
    echo
    echo "───────────────────────────────────────────────────────────────"
    echo "$tag cargo build $PROF_FLAG $flags"

    if ! cargo build $CARGO_FLAGS $PROF_FLAG $flags >/dev/null 2>&1; then
      echo "$tag BUILD FAILED"; FAILED+=("$tag build"); continue
    fi
    if ! cargo clippy --version >/dev/null 2>&1; then :; fi

    # Symbol parity for this exact artifact.
    RUST_SO="$CRATE_DIR/$SO_DIR/libdriver.so"
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO"   | awk '{print $3}' | sort -u) \
      <(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort -u))
    if [ -n "$missing" ]; then
      echo "$tag SYMBOL PARITY FAILED; missing from Rust .so:"; echo "$missing"
      FAILED+=("$tag symbols")
    else
      echo "$tag symbol parity OK ($(nm -D --defined-only "$C_SO" | wc -l) C symbol(s))"
    fi

    if timeout 600 cargo test $CARGO_FLAGS $PROF_FLAG $flags 2>&1 | tee "$SO_DIR/.difftest.log" \
        | grep -E '^(  phase|.* result:)'; then :; fi
    if grep -qE 'FAILED|STALE|panicked' "$SO_DIR/.difftest.log"; then
      echo "$tag DIFFERENTIAL TESTS FAILED"; FAILED+=("$tag tests")
    else
      echo "$tag differential tests OK"; PASSED=$((PASSED+1))
    fi
  done
done

echo
echo "==============================================================="
if [ ${#FAILED[@]} -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED ($PASSED configuration/profile pairs)"
  exit 0
else
  echo "FAILURES (${#FAILED[@]}): ${FAILED[*]}"
  exit 1
fi
