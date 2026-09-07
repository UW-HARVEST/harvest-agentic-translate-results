#!/usr/bin/env bash
# Phase D: run the full differential suite under EVERY feature combination and
# under both codegen profiles (debug and release produce different Rust .so
# binaries, so both are exercised).
#
# Usage: tests/feature_matrix.sh
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
CRATE_DIR="$PWD"
ROOT="$(cd .. && pwd)"

echo "== crate: $CRATE_DIR"

# ---------------------------------------------------------------------------
# 1. Make sure the C .so exists.
# ---------------------------------------------------------------------------
if ! compgen -G "$ROOT/c_src/build/lib*.so" > /dev/null; then
  echo "-- building the C shared library"
  ( mkdir -p "$ROOT/c_src/build" \
    && cd "$ROOT/c_src/build" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi
C_SO="$(ls "$ROOT"/c_src/build/lib*.so)"
echo "-- C .so: $C_SO"

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations from Cargo.toml (powerset of [features]).
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t"]/, "", a[1]);
                      if (a[1] != "default" && a[1] != "") print a[1] }
  ' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "-- no [features] in Cargo.toml: only the default configuration exists"
  COMBOS+=("DEFAULT")
  COMBOS+=("NODEFAULT")
else
  n=${#FEATURES[@]}
  total=$(( 1 << n ))
  COMBOS+=("DEFAULT")
  for (( mask = 0; mask < total; mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("NODEFAULT:$combo")
  done
fi

# ---------------------------------------------------------------------------
# 3. Run cargo check + the full differential suite for each combo x profile.
# ---------------------------------------------------------------------------
FAIL=0
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    DEFAULT)      FLAGS=() ; label="default features" ;;
    NODEFAULT)    FLAGS=(--no-default-features) ; label="--no-default-features" ;;
    NODEFAULT:)   FLAGS=(--no-default-features) ; label="--no-default-features" ;;
    NODEFAULT:*)  FLAGS=(--no-default-features --features "${combo#NODEFAULT:}")
                  label="--no-default-features --features ${combo#NODEFAULT:}" ;;
  esac

  for profile in debug release; do
    if [ "$profile" = release ]; then PF=(--release); else PF=(); fi
    echo
    echo "=============================================================="
    echo "== [$profile] $label"
    echo "=============================================================="

    if ! cargo check --offline "${FLAGS[@]}" "${PF[@]}" 2>&1 | tail -n 3; then
      echo "!! cargo check FAILED"; FAIL=1; continue
    fi
    # Build the cdylib for this profile so the tests can dlopen it.
    if ! cargo build --offline "${FLAGS[@]}" "${PF[@]}" 2>&1 | tail -n 3; then
      echo "!! cargo build FAILED"; FAIL=1; continue
    fi
    if ! timeout 600 cargo test --offline "${FLAGS[@]}" "${PF[@]}" 2>&1 | tail -n 6; then
      echo "!! cargo test FAILED"; FAIL=1
    fi
  done
done

# ---------------------------------------------------------------------------
# 4. Symbol parity (Phase D).
# ---------------------------------------------------------------------------
echo
echo "=============================================================="
echo "== symbol parity: nm -D"
echo "=============================================================="
for profile in debug release; do
  RUST_SO="$CRATE_DIR/target/$profile/libldexp_q2_lib.so"
  [ -f "$RUST_SO" ] || continue
  echo "-- $profile"
  diffout=$(diff \
    <(nm -D --defined-only "$C_SO"    | awk '{print $3}' | grep -vE '^(_init|_fini|__)' | sort -u) \
    <(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | grep -vE '^(_init|_fini|__)' | sort -u))
  # Lines starting with '<' are symbols present in C but MISSING from Rust.
  if echo "$diffout" | grep -q '^<'; then
    echo "!! symbols missing from the Rust .so:"
    echo "$diffout" | grep '^<'
    FAIL=1
  else
    echo "   OK: 0 C symbols missing from the Rust .so"
  fi
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$FAIL"
