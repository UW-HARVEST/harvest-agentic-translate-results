#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY feature combination.
#
# Feature names are extracted mechanically from Cargo.toml, then the full
# power set is enumerated (capped at 2^8 combinations). For each combination the
# cdylib is REBUILT (so the .so under test always matches the feature set) and
# the suite is run against it.
set -uo pipefail
cd "$(dirname "$0")"
HTMP=${TMPDIR:-/tmp}; mkdir -p "$HTMP"
ulimit -c 0 2>/dev/null || true

CARGO_FLAGS="--offline"

# --- 1. build the C reference .so ------------------------------------------
CSRC=../c_src
if [ ! -d "$CSRC/build" ] || [ -z "$(find "$CSRC/build" -maxdepth 1 -name 'lib*.so' -print -quit)" ]; then
  echo "== building C shared library =="
  ( mkdir -p "$CSRC/build" && cd "$CSRC/build" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi
C_SO=$(find "$CSRC/build" -maxdepth 1 -name 'lib*.so' | head -1)
echo "C .so: $C_SO"

# --- 2. enumerate features -------------------------------------------------
# Everything between a line `[features]` and the next `[section]`, taking the
# key on the left of each `=`.
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /=/      {split($0,a,"="); gsub(/[ \t"]/,"",a[1]); if (a[1] != "" && a[1] !~ /^#/) print a[1]}
' Cargo.toml | sort -u)

FEAT_ARR=()
if [ -n "$FEATURES" ]; then
  while IFS= read -r f; do FEAT_ARR+=("$f"); done <<< "$FEATURES"
fi
N=${#FEAT_ARR[@]}
echo "features declared in Cargo.toml: $N ${FEAT_ARR[*]:-(none)}"

COMBOS=()
if [ "$N" -eq 0 ]; then
  # No [features] table => the only two build configurations are the default
  # (empty) feature set and --no-default-features (also empty).
  COMBOS+=("")
  COMBOS+=("--no-default-features")
  COMBOS+=("--all-features")
else
  if [ "$N" -gt 8 ]; then echo "capping power set at the first 8 features"; N=8; fi
  for ((mask=0; mask<(1<<N); mask++)); do
    sel=""
    for ((i=0; i<N; i++)); do
      if (( (mask>>i) & 1 )); then sel="$sel,${FEAT_ARR[$i]}"; fi
    done
    sel="${sel#,}"
    COMBOS+=("--no-default-features --features $sel")
    COMBOS+=("--features $sel")
  done
  COMBOS+=("--all-features")
fi

# --- 3. check + build + test every combination -----------------------------
fails=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  echo
  echo "=============================================================="
  echo "== feature combination: $label"
  echo "=============================================================="

  if ! cargo check $CARGO_FLAGS $combo --tests > $HTMP/.hatch_check.log 2>&1; then
    echo "  cargo check FAILED"; tail -20 $HTMP/.hatch_check.log; fails=$((fails+1)); continue
  fi
  echo "  cargo check ok"

  # Rebuild the cdylib for THIS feature set and point the harness at it.
  if ! cargo build $CARGO_FLAGS $combo --release > $HTMP/.hatch_build.log 2>&1; then
    echo "  cargo build --release FAILED"; tail -20 $HTMP/.hatch_build.log; fails=$((fails+1)); continue
  fi
  R_SO=target/release/libhatch_lib.so
  if [ ! -f "$R_SO" ]; then echo "  missing $R_SO"; fails=$((fails+1)); continue; fi
  echo "  cdylib: $R_SO ($(nm -D --defined-only "$R_SO" | wc -l) exported symbols)"

  # Symbol diff must be empty for every combination.
  nm -D --defined-only "$C_SO"  | awk '{print $3}' | sort > $HTMP/.hatch_c.syms
  nm -D --defined-only "$R_SO"  | awk '{print $3}' | sort > $HTMP/.hatch_r.syms
  missing=$(comm -23 $HTMP/.hatch_c.syms $HTMP/.hatch_r.syms)
  if [ -n "$missing" ]; then
    echo "  SYMBOL DIFF NOT EMPTY: $missing"; fails=$((fails+1)); continue
  fi
  echo "  symbol diff: empty"

  if HATCH_C_SO="$(cd "$(dirname "$C_SO")" && pwd)/$(basename "$C_SO")" \
     HATCH_RUST_SO="$PWD/$R_SO" \
     timeout 600 cargo test $CARGO_FLAGS $combo -- --test-threads=1 > $HTMP/.hatch_test.log 2>&1; then
    grep -hE '^test result:' $HTMP/.hatch_test.log | sed 's/^/  /'
  else
    echo "  TESTS FAILED"
    grep -E 'FAILED|panicked|^test result:' $HTMP/.hatch_test.log | head -40 | sed 's/^/  /'
    fails=$((fails+1))
  fi
done

# --- 4. build-PROFILE axis --------------------------------------------------
# The C reference is compiled at -O0 (CMAKE_BUILD_TYPE is unset), and the C code
# relies on signed-overflow wrap-around. Verify the Rust cdylib matches at BOTH
# optimisation levels, since -O3 is where a mistranslated wrap would show up.
for prof in debug release; do
  echo
  echo "=============================================================="
  echo "== build profile: $prof"
  echo "=============================================================="
  if [ "$prof" = release ]; then
    cargo build $CARGO_FLAGS --release > $HTMP/.hatch_build.log 2>&1 || { echo "  build FAILED"; fails=$((fails+1)); continue; }
  else
    cargo build $CARGO_FLAGS > $HTMP/.hatch_build.log 2>&1 || { echo "  build FAILED"; fails=$((fails+1)); continue; }
  fi
  R_SO="target/$prof/libhatch_lib.so"
  if [ ! -f "$R_SO" ]; then echo "  missing $R_SO"; fails=$((fails+1)); continue; fi
  echo "  cdylib: $R_SO ($(nm -D --defined-only "$R_SO" | wc -l) exported symbols)"
  nm -D --defined-only "$C_SO" | awk '{print $3}' | sort > $HTMP/.hatch_c.syms
  nm -D --defined-only "$R_SO" | awk '{print $3}' | sort > $HTMP/.hatch_r.syms
  missing=$(comm -23 $HTMP/.hatch_c.syms $HTMP/.hatch_r.syms)
  if [ -n "$missing" ]; then echo "  SYMBOL DIFF NOT EMPTY: $missing"; fails=$((fails+1)); continue; fi
  echo "  symbol diff: empty"
  if HATCH_C_SO="$(cd "$(dirname "$C_SO")" && pwd)/$(basename "$C_SO")" \
     HATCH_RUST_SO="$PWD/$R_SO" \
     timeout 600 cargo test $CARGO_FLAGS -- --test-threads=1 > $HTMP/.hatch_test.log 2>&1; then
    grep -hE '^test result:' $HTMP/.hatch_test.log | sed 's/^/  /'
  else
    echo "  TESTS FAILED"
    grep -E 'FAILED|panicked|^test result:' $HTMP/.hatch_test.log | head -40 | sed 's/^/  /'
    fails=$((fails+1))
  fi
done

echo
if [ "$fails" -eq 0 ]; then
  echo "ALL ${#COMBOS[@]} FEATURE COMBINATIONS + 2 BUILD PROFILES PASSED"
else
  echo "$fails CONFIGURATION(S) FAILED"
fi
exit "$fails"
