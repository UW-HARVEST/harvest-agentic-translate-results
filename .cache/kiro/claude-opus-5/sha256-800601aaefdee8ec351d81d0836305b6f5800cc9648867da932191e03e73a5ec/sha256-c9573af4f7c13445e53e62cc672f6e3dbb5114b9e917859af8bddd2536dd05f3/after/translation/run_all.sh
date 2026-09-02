#!/usr/bin/env bash
# Runs the full differential suite (Phases B, C, D) against every feature
# combination and every build profile of the Rust cdylib.
#
# Usage: cd translation && ./run_all.sh
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/translation"
cd "$CRATE"

FAIL=0

# --- build the C library --------------------------------------------------
if ! ls "$ROOT"/c_src/build/lib*.so >/dev/null 2>&1; then
  echo "== building the C shared library =="
  ( mkdir -p "$ROOT/c_src/build" && cd "$ROOT/c_src/build" \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi
C_SO="$(ls "$ROOT"/c_src/build/lib*.so | head -1)"
echo "C  .so: $C_SO"

# --- enumerate feature combinations from Cargo.toml -----------------------
# Every key in [features] except "default".
FEATURES="$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/ {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' Cargo.toml)"

COMBOS=()
if [ -z "$FEATURES" ]; then
  echo "no [features] in Cargo.toml -> the only configuration is the default one"
  COMBOS+=("__default__" "__none__")
else
  COMBOS+=("__default__" "__none__")
  # Full powerset of the declared features.
  FARR=($FEATURES)
  n=${#FARR[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FARR[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "feature combinations: ${COMBOS[*]}"

run_suite () {           # $1 = label, $2 = cargo feature flags, $3 = profile flags
  local label="$1" fflags="$2" pflags="$3"
  echo
  echo "===== $label ====="
  # Build the cdylib for this combination/profile so the .so under test matches.
  # shellcheck disable=SC2086
  if ! timeout 600 cargo build $pflags $fflags >/dev/null 2>&1; then
    echo "  cargo build FAILED for $label"; FAIL=1; return
  fi
  local prof=debug
  [[ "$pflags" == *--release* ]] && prof=release
  export HARVEST_RUST_SO="$CRATE/target/$prof/librgb_to_hsv_lib.so"
  if [ ! -f "$HARVEST_RUST_SO" ]; then
    echo "  missing $HARVEST_RUST_SO"; FAIL=1; return
  fi
  echo "  Rust .so: $HARVEST_RUST_SO"
  # nm parity for this exact artifact.
  local missing
  missing="$(comm -23 \
      <(nm -D --defined-only "$C_SO"            | awk '{print $NF}' | sort -u) \
      <(nm -D --defined-only "$HARVEST_RUST_SO" | awk '{print $NF}' | sort -u))"
  if [ -n "$missing" ]; then
    echo "  SYMBOL PARITY FAILED, missing: $missing"; FAIL=1
  else
    echo "  symbol parity: OK (0 missing)"
  fi
  # shellcheck disable=SC2086
  if timeout 600 cargo test $fflags -- --test-threads=4 2>&1 | tail -n 6; then
    :
  fi
  # shellcheck disable=SC2086
  if ! timeout 600 cargo test $fflags >/dev/null 2>&1; then
    echo "  TESTS FAILED for $label"; FAIL=1
  else
    echo "  tests: OK"
  fi
  unset HARVEST_RUST_SO
}

for combo in "${COMBOS[@]}"; do
  case "$combo" in
    __default__) fflags="" ; name="default features" ;;
    __none__)    fflags="--no-default-features"; name="--no-default-features" ;;
    *)           fflags="--no-default-features --features $combo"; name="features=$combo" ;;
  esac
  run_suite "$name / debug cdylib"   "$fflags" ""
  run_suite "$name / release cdylib" "$fflags" "--release"
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "ALL COMBINATIONS PASSED"
else
  echo "SOME COMBINATIONS FAILED"
fi
exit "$FAIL"
