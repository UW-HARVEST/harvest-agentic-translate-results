#!/usr/bin/env bash
# Full verification driver: enumerates every valid feature combination from
# Cargo.toml, builds the C reference .so, then runs `cargo check` + the whole
# differential test suite for each combination, and finally diffs `nm -D`.
set -uo pipefail
cd "$(dirname "$0")"

CARGO_FLAGS=${CARGO_FLAGS:---offline}
fail=0

echo "=============================================================="
echo "0. Enumerate feature combinations from Cargo.toml"
echo "=============================================================="
# Extract feature names from the [features] section (excluding "default").
FEATURES=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml)

COMBOS=()
if [ -z "$FEATURES" ]; then
  echo "No [features] section -> exactly ONE valid combination: the empty set."
  COMBOS+=("")
else
  # power set of all declared features
  feats=($FEATURES)
  n=${#feats[@]}
  echo "Declared features (${n}): ${feats[*]}"
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${feats[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "Combinations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "  - '${c:-<empty>}'"; done

echo
echo "=============================================================="
echo "1. Build the C reference shared library"
echo "=============================================================="
mkdir -p c_src/build
(cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build .) \
  || { echo "FAIL: C build"; exit 1; }
C_SO=$(ls c_src/build/lib*.so | head -1)
echo "C .so: $C_SO"

echo
echo "=============================================================="
echo "2. cargo check + cargo test for every feature combination"
echo "=============================================================="
for combo in "${COMBOS[@]}"; do
 for prof in dev release; do
  label="${combo:-<empty>} / $prof"
  echo
  echo "---- combination: $label ----"
  if [ -z "$combo" ]; then
    FSET=(--no-default-features)
  else
    FSET=(--no-default-features --features "$combo")
  fi
  if [ "$prof" = release ]; then
    FSET+=(--release)
    PROFDIR=release
  else
    PROFDIR=debug
  fi

  echo "\$ cargo check ${FSET[*]}"
  if ! timeout 600 cargo check $CARGO_FLAGS "${FSET[@]}" 2>&1 | tail -5; then
    echo "FAIL: cargo check ($label)"; fail=1; continue
  fi

  echo "\$ cargo build ${FSET[*]}   (produces the Rust cdylib under test)"
  if ! timeout 600 cargo build $CARGO_FLAGS "${FSET[@]}" 2>&1 | tail -5; then
    echo "FAIL: cargo build ($label)"; fail=1; continue
  fi

  echo "\$ cargo test ${FSET[*]}"
  TLOG="${TMPDIR:-/tmp}/cargo-test-$$.log"
  if timeout 600 cargo test $CARGO_FLAGS --no-fail-fast "${FSET[@]}" > "$TLOG" 2>&1; then
    grep -E '^(test |     Running|test result)' "$TLOG"
    echo "    total: $(grep -c '^test .* ok$' "$TLOG") passing tests, \
$(grep -c '^test .* FAILED$' "$TLOG") failures"
  else
    echo "FAIL: cargo test ($label)"; tail -60 "$TLOG"; fail=1
  fi
  rm -f "$TLOG"

  echo "-- nm -D diff for $label --"
  R_SO=target/$PROFDIR/libhdr_compare_lib.so
  SYMDIFF="${TMPDIR:-/tmp}/symdiff.$$"
  diff <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u) \
       <(nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u) \
       > "$SYMDIFF" 2>&1
  if [ ! -f "$SYMDIFF" ]; then
    echo "FAIL: could not write $SYMDIFF (set TMPDIR to a writable dir)"; fail=1
  elif grep -q '^<' "$SYMDIFF"; then
    echo "FAIL: symbols missing from the Rust .so:"; grep '^<' "$SYMDIFF"; fail=1
  else
    echo "OK: every C symbol is exported by the Rust .so"
    echo "    C exports  : $(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u | tr '\n' ' ')"
    echo "    Rust-only  : $(grep '^>' "$SYMDIFF" | sed 's/^> //' | tr '\n' ' ')"
  fi
  rm -f "$SYMDIFF"
 done
done

echo
echo "=============================================================="
if [ "$fail" -eq 0 ]; then echo "ALL COMBINATIONS PASSED"; else echo "FAILURES DETECTED"; fi
echo "=============================================================="
exit $fail
