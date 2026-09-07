#!/usr/bin/env bash
# Phase D driver: rebuild both libraries, diff exported symbols, and run the
# full differential suite under EVERY cargo feature combination and profile.
#
# Usage: ./scripts/verify.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CRATE="$ROOT/translation"
FAIL=0

note() { printf '\n=== %s ===\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
note "1. Build the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . >/dev/null ) || fail "C build"
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | sort | head -1)"
[ -n "$C_SO" ] || fail "no C .so produced"
echo "C  .so: $C_SO"

# ---------------------------------------------------------------------------
note "2. Enumerate cargo feature combinations"
# Extract feature names from the [features] table (excluding "default").
FEATURES=$(awk '
  /^\[features\]/ { inf=1; next }
  /^\[/           { inf=0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' "$CRATE/Cargo.toml")
if [ -z "$FEATURES" ]; then
  echo "no [features] declared -> configurations: default, --no-default-features"
  COMBOS=("" "--no-default-features")
else
  echo "features: $FEATURES"
  # Full power set of the declared features, with and without defaults.
  mapfile -t FARR <<<"$FEATURES"
  N=${#FARR[@]}
  COMBOS=("")
  for ((mask = 0; mask < (1 << N); mask++)); do
    sel=""
    for ((i = 0; i < N; i++)); do
      if (( mask & (1 << i) )); then sel="${sel:+$sel,}${FARR[$i]}"; fi
    done
    if [ -z "$sel" ]; then
      COMBOS+=("--no-default-features")
    else
      COMBOS+=("--no-default-features --features $sel")
      COMBOS+=("--features $sel")
    fi
  done
fi
printf 'configurations to verify: %d\n' "${#COMBOS[@]}"

# ---------------------------------------------------------------------------
for PROFILE in release debug; do
  if [ "$PROFILE" = release ]; then PFLAG="--release"; PDIR=release; else PFLAG=""; PDIR=debug; fi
  for COMBO in "${COMBOS[@]}"; do
    LABEL="profile=$PROFILE features='${COMBO:-<default>}'"
    note "3. cargo check  [$LABEL]"
    # shellcheck disable=SC2086
    ( cd "$CRATE" && timeout 600 cargo check $PFLAG $COMBO 2>&1 | tail -3 ) \
      || fail "cargo check [$LABEL]"

    note "4. Build Rust cdylib + symbol diff  [$LABEL]"
    # shellcheck disable=SC2086
    ( cd "$CRATE" && timeout 600 cargo build $PFLAG $COMBO 2>&1 | tail -3 ) \
      || fail "cargo build [$LABEL]"
    RS_SO="$CRATE/target/$PDIR/libgaussian_kernel_lib.so"
    if [ ! -f "$RS_SO" ]; then
      fail "no Rust .so at $RS_SO [$LABEL]"
      continue
    fi

    nm -D --defined-only "$C_SO"  | awk '{print $NF}' | sort -u > /tmp/c_syms.txt
    nm -D --defined-only "$RS_SO" | awk '{print $NF}' | sort -u > /tmp/rs_syms.txt
    MISSING=$(comm -23 /tmp/c_syms.txt /tmp/rs_syms.txt)
    if [ -n "$MISSING" ]; then
      fail "symbols exported by C but MISSING from Rust [$LABEL]:"
      echo "$MISSING"
    else
      echo "symbol diff EMPTY ($(wc -l < /tmp/c_syms.txt) C symbols all present in Rust)"
    fi

    # Undefined non-libc / non-runtime symbols in the Rust .so.
    UNDEF=$(ldd -r "$RS_SO" 2>&1 | grep -i 'undefined symbol' || true)
    if [ -n "$UNDEF" ]; then
      fail "unresolved symbols in Rust .so [$LABEL]: $UNDEF"
    else
      echo "Rust .so: 0 unresolved symbols"
    fi

    note "5. Differential test suite  [$LABEL]"
    LOG=$(mktemp)
    # shellcheck disable=SC2086
    ( cd "$CRATE" && timeout 600 cargo test $PFLAG $COMBO ) > "$LOG" 2>&1 \
      || fail "cargo test [$LABEL]"
    grep -E '^test result|^running|FAILED|panicked' "$LOG"
    if grep -q 'FAILED' "$LOG"; then fail "test failures [$LABEL]"; fi
    PASSED=$(awk '/^test result: ok\./ { for (i = 1; i <= NF; i++) if ($i == "passed;") s += $(i-1) } END { print s + 0 }' "$LOG")
    echo "total passing tests: ${PASSED:-0}"
    if [ "${PASSED:-0}" -lt 40 ]; then
      fail "expected >=40 passing tests, got ${PASSED:-0} [$LABEL]"
    fi
    rm -f "$LOG"
  done
done

# ---------------------------------------------------------------------------
note "6. C optimization-level robustness matrix"
# The ground truth is c_src/CMakeLists.txt with default flags (no -O), already
# verified above. This step additionally rebuilds the SAME C source (read-only,
# outside c_src/) at each -O level and re-runs the suite, to prove the Rust is
# not accidentally matching one particular gcc constant-folding choice.
#
# -Ofast / -ffast-math is EXPECTED to diverge (it licenses reassociation and
# reciprocal approximation of `1.0f/sum`, giving 1-ULP differences) and is
# reported as informational only, not a failure.
OPTDIR=$(mktemp -d)
for O in 0 1 2 3 s; do
  gcc "-O$O" -fPIC -shared -I"$ROOT/c_src/include" "$ROOT/c_src/src/lib.c" -lm \
      -o "$OPTDIR/lib_O$O.so" 2>/dev/null || { fail "gcc -O$O build"; continue; }
  LOG=$(mktemp)
  ( cd "$CRATE" && HARVEST_C_SO="$OPTDIR/lib_O$O.so" SWEEP_STRIDE=8191 \
      timeout 600 cargo test --release --no-fail-fast ) > "$LOG" 2>&1
  R=$(awk '/^test result/ { for (i=1;i<=NF;i++) { if ($i=="passed;") p+=$(i-1); if ($i=="failed;") f+=$(i-1) } } END { printf "passed=%d failed=%d", p, f }' "$LOG")
  echo "  C -O$O : $R"
  if grep -q 'FAILED' "$LOG"; then
    fail "divergence against C built with -O$O"
    grep -m3 'DIVERGENCE' "$LOG"
  fi
  rm -f "$LOG"
done
# informational: -ffast-math
gcc -O3 -ffast-math -fPIC -shared -I"$ROOT/c_src/include" "$ROOT/c_src/src/lib.c" -lm \
    -o "$OPTDIR/lib_Ofast.so" 2>/dev/null && {
  LOG=$(mktemp)
  ( cd "$CRATE" && HARVEST_C_SO="$OPTDIR/lib_Ofast.so" SWEEP_STRIDE=8191 \
      timeout 600 cargo test --release --no-fail-fast ) > "$LOG" 2>&1
  R=$(awk '/^test result/ { for (i=1;i<=NF;i++) { if ($i=="passed;") p+=$(i-1); if ($i=="failed;") f+=$(i-1) } } END { printf "passed=%d failed=%d", p, f }' "$LOG")
  echo "  C -Ofast (INFORMATIONAL, divergence expected) : $R"
  grep -m1 'DIVERGENCE' "$LOG" || true
  rm -f "$LOG"
}
rm -rf "$OPTDIR"

# ---------------------------------------------------------------------------
note "7. No binary driver to compare"
if grep -q 'add_executable' "$ROOT/c_src/CMakeLists.txt"; then
  fail "c_src builds an executable; stdout comparison required but not implemented"
else
  echo "c_src/CMakeLists.txt has no add_executable"
fi
if grep -qE '^\[\[bin\]\]' "$CRATE/Cargo.toml"; then
  fail "translation declares a [[bin]]; stdout comparison required"
else
  echo "translation/Cargo.toml declares no [[bin]]"
fi

# ---------------------------------------------------------------------------
note "RESULT"
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "SOME CHECKS FAILED"
fi
exit "$FAIL"
