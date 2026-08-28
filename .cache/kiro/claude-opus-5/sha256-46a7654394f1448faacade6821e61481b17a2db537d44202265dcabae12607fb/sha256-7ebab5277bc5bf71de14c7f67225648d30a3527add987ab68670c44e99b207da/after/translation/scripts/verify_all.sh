#!/usr/bin/env bash
# Enumerates every valid Cargo feature combination and runs
# `cargo check` + `cargo test` for each, comparing exported symbols
# against the C reference .so.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
ROOT="$(cd .. && pwd)"
CRATE="$PWD"
LOGDIR=/tmp/cmode-sweep
mkdir -p "$LOGDIR"

# --- 1. enumerate features from Cargo.toml -------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t]/, "", a[1]);
                      if (a[1] != "default" && a[1] != "") print a[1] }
  ' Cargo.toml
)

echo "features declared: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# --- 2. build the power set ---------------------------------------------
COMBOS=("")   # empty combo == --no-default-features with nothing enabled
n=${#FEATURES[@]}
if (( n > 0 )); then
  for (( mask=1; mask < (1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then
        combo="${combo:+$combo,}${FEATURES[i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "combinations to verify: ${#COMBOS[@]} (plus the default feature set)"

# --- 3. C reference library ---------------------------------------------
( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    && cmake --build . ) > "$LOGDIR/c-build.log" 2>&1 \
  || { echo "C build FAILED, see $LOGDIR/c-build.log"; exit 1; }
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | head -1)"
echo "C .so: $C_SO"

nm -D --defined-only "$C_SO" \
  | awk '$2=="T"||$2=="W"||$2=="D"||$2=="B"{print $3}' | sort > "$LOGDIR/c_syms.txt"

FAIL=0

run_combo() {
  local label="$1"; shift
  local -a flags=("$@")
  local safe_label="${label//[^a-zA-Z0-9]/_}"

  echo "=============================================================="
  echo "== combo: $label"

  if ! timeout 600 cargo check "${flags[@]}" > "$LOGDIR/check-$safe_label.log" 2>&1; then
    echo "   cargo check FAILED (see $LOGDIR/check-$safe_label.log)"
    tail -20 "$LOGDIR/check-$safe_label.log"
    FAIL=1
    return
  fi
  echo "   cargo check ok"

  # Build the cdylib for this combo and diff exported symbols against C.
  if ! timeout 600 cargo build --lib --release "${flags[@]}" \
        --target-dir "$CRATE/target/so-build" > "$LOGDIR/build-$safe_label.log" 2>&1; then
    echo "   cargo build FAILED (see $LOGDIR/build-$safe_label.log)"
    FAIL=1
    return
  fi
  local rs_so="$CRATE/target/so-build/release/libcomplexmode_lib.so"
  nm -D --defined-only "$rs_so" \
    | awk '$2=="T"||$2=="W"||$2=="D"||$2=="B"{print $3}' | sort > "$LOGDIR/r_syms.txt"
  local missing
  missing="$(comm -23 "$LOGDIR/c_syms.txt" "$LOGDIR/r_syms.txt")"
  if [[ -n "$missing" ]]; then
    echo "   MISSING EXPORTS in Rust .so:"
    echo "$missing" | sed 's/^/     /'
    FAIL=1
  else
    echo "   symbol parity ok ($(wc -l < "$LOGDIR/c_syms.txt") C symbols)"
  fi

  # Differential tests (the harness rebuilds the .so with the same flags).
  if [[ " ${flags[*]} " == *" --no-default-features "* ]]; then
    export RUST_SO_NO_DEFAULT_FEATURES=1
  else
    unset RUST_SO_NO_DEFAULT_FEATURES
  fi
  local feats=""
  for (( i=0; i<${#flags[@]}; i++ )); do
    if [[ "${flags[i]}" == "--features" ]]; then feats="${flags[i+1]}"; fi
  done
  export RUST_SO_FEATURES="$feats"

  if timeout 600 cargo test "${flags[@]}" > "$LOGDIR/test-$safe_label.log" 2>&1; then
    echo "   cargo test ok"
  else
    echo "   cargo test FAILED (see $LOGDIR/test-$safe_label.log)"
    grep -E 'FAILED|assertion|panicked' "$LOGDIR/test-$safe_label.log" | head -20
    FAIL=1
  fi
}

# Default feature set (what an ordinary `cargo build` produces).
run_combo "default" 

# Every explicit combination.
for combo in "${COMBOS[@]}"; do
  if [[ -z "$combo" ]]; then
    run_combo "no-default-features" --no-default-features
  else
    run_combo "$combo" --no-default-features --features "$combo"
  fi
done

echo "=============================================================="
if (( FAIL )); then
  echo "RESULT: FAILURES present"
  exit 1
fi
echo "RESULT: all feature combinations verified"
