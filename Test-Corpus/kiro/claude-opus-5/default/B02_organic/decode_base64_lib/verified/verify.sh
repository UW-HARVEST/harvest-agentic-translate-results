#!/usr/bin/env bash
# Full verification driver.
#
# Rebuilds the C .so and the Rust cdylib, then runs the differential suite for
# EVERY feature combination declared in Cargo.toml. The explicit
# `cargo build` before each `cargo test` is mandatory: cargo does not rebuild a
# `crate-type = ["cdylib"]` artifact as part of `cargo test`, so without it the
# suite would load a stale .so (the tests' freshness guard also catches this).
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(pwd)"
rc=0

echo "=== building C shared library ==="
( cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C BUILD FAILED"; exit 1; }
ls -l ../c_src/build/libdriver.so

# ---------------------------------------------------------------------------
# Enumerate feature combinations from Cargo.toml.
# ---------------------------------------------------------------------------
FEATURES=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /=/   {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default") print a[1]}
' Cargo.toml)

# Build the list of flag-sets to test: default, no-default, then each feature and
# the full powerset if any features exist.
COMBOS=()
COMBOS+=("")                        # default features
COMBOS+=("--no-default-features")
if [ -n "$FEATURES" ]; then
  mapfile -t FLIST <<< "$FEATURES"
  n=${#FLIST[@]}
  for (( mask=1; mask < (1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FLIST[$i]}"; fi
    done
    COMBOS+=("--no-default-features --features $combo")
  done
else
  echo "note: Cargo.toml declares no [features] table -> the default build is"
  echo "      the only configuration; --no-default-features is equivalent."
fi

# ---------------------------------------------------------------------------
# Run every combination in both profiles. Debug matters: it enables Rust's
# arithmetic-overflow checks, which C does not have, so a shift/add that the C
# wraps silently would abort the Rust .so there.
# ---------------------------------------------------------------------------
for profile in release debug; do
  if [ "$profile" = release ]; then PFLAG="--release"; else PFLAG=""; fi
  for combo in "${COMBOS[@]}"; do
    label="profile=$profile features=[${combo:-default}]"
    echo
    echo "=== $label : build ==="
    # shellcheck disable=SC2086
    if ! timeout 600 cargo build $PFLAG $combo 2>&1 | tail -2; then
      echo "BUILD FAILED: $label"; rc=1; continue
    fi
    # Point the harness at the profile under test explicitly, so the .so it
    # loads is unambiguous without deleting any artifact.
    SO="$ROOT/target/$profile/libdriver.so"
    export DRIVER_SO="$SO"

    echo "=== $label : nm -D parity ==="
    diff <(nm -D --defined-only ../c_src/build/libdriver.so | awk '$2=="T"{print $3}' | sort) \
         <(nm -D --defined-only "$SO"                        | awk '$2=="T"{print $3}' | sort) \
      && echo "  symbol diff EMPTY (parity)" \
      || { echo "  SYMBOL PARITY FAILED: $label"; rc=1; }

    echo "=== $label : test ==="
    # shellcheck disable=SC2086
    if timeout 600 cargo test $PFLAG $combo 2>&1 | tail -4; then :; else
      echo "TESTS FAILED: $label"; rc=1
    fi
  done
done

echo
if [ $rc -eq 0 ]; then echo "ALL COMBINATIONS PASSED"; else echo "FAILURES PRESENT (rc=$rc)"; fi
exit $rc
