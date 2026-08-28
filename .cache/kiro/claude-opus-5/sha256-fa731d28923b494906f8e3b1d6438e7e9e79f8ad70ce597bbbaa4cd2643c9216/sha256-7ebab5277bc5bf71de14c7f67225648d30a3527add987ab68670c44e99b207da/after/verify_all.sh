#!/usr/bin/env bash
#
# Verify the translation against the C ground truth for EVERY build-time
# configuration.
#
#   ./verify_all.sh
#
# What it does:
#   1. Enumerates every valid feature combination from translation/Cargo.toml.
#   2. `cargo check` for each combination.
#   3. Builds the C shared library with CMake (and, additionally, at several
#      optimisation levels, since the C's out-of-table reads are layout
#      dependent and should be checked against more than one code generation).
#   4. `cargo test` for each combination, against each C build.
#   5. Compares `nm -D` exports of the C .so and the Rust .so.
#
# Run from the workspace root (the directory holding c_src/ and translation/).

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
CSRC="$ROOT/c_src"
LOGDIR="${TMPDIR:-/tmp}/hdr_bitrate_verify"
mkdir -p "$LOGDIR"

TIMEOUT=600
fail=0

note() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  [ok]   %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; fail=1; }

# --------------------------------------------------------------------------
# 1. Enumerate feature combinations
# --------------------------------------------------------------------------
note "Enumerating feature combinations from translation/Cargo.toml"

# Read the feature names declared under [features]. `default` is handled
# separately: it is a meta-feature, not an independent knob.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inblock = 1; next }
    /^\[/           { inblock = 0 }
    inblock && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "=");
      gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' "$CRATE/Cargo.toml"
)

if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "  translation/Cargo.toml declares no [features]."
  echo "  The only valid configuration is the default (empty) feature set."
  COMBOS=("")
else
  echo "  features: ${FEATURES[*]}"
  # Power set of the declared features.
  COMBOS=()
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi
echo "  ${#COMBOS[@]} combination(s) to verify"

# --------------------------------------------------------------------------
# 2. cargo check for every combination
# --------------------------------------------------------------------------
note "cargo check for every feature combination"
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  log="$LOGDIR/check_${combo//,/_}.log"
  if timeout "$TIMEOUT" cargo check --manifest-path "$CRATE/Cargo.toml" \
      --all-targets --no-default-features \
      ${combo:+--features "$combo"} >"$log" 2>&1; then
    ok "cargo check --no-default-features --features '$label'"
  else
    bad "cargo check --no-default-features --features '$label' (see $log)"
    tail -n 20 "$log"
  fi
done

# --------------------------------------------------------------------------
# 3. Build the C library
# --------------------------------------------------------------------------
note "Building the C shared library (CMake, default configuration)"
if ( mkdir -p "$CSRC/build" && cd "$CSRC/build" \
     && timeout "$TIMEOUT" cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
     && timeout "$TIMEOUT" cmake --build . ) >"$LOGDIR/cmake.log" 2>&1; then
  C_SO="$(find "$CSRC/build" -maxdepth 1 -name '*.so' | sort | head -n1)"
  ok "C .so: $C_SO"
else
  bad "CMake build failed (see $LOGDIR/cmake.log)"
  tail -n 30 "$LOGDIR/cmake.log"
  exit 1
fi

note "Building extra C variants at different optimisation levels"
VARIANT_DIR="$LOGDIR/c_variants"
mkdir -p "$VARIANT_DIR"
C_SOS=("$C_SO")
for opt in -O0 -O1 -O2 -O3 -Os; do
  out="$VARIANT_DIR/lib${opt}.so"
  if cc "$opt" -fPIC -shared -I "$CSRC/include" -o "$out" "$CSRC/src/lib.c" \
      >>"$LOGDIR/c_variants.log" 2>&1; then
    ok "cc $opt -> $out"
    C_SOS+=("$out")
  else
    echo "  [skip] cc $opt failed (see $LOGDIR/c_variants.log)"
  fi
done

# --------------------------------------------------------------------------
# 4. cargo test for every combination, against every C build
# --------------------------------------------------------------------------
note "cargo test for every feature combination x every C build"
for combo in "${COMBOS[@]}"; do
  for c_so in "${C_SOS[@]}"; do
    label="${combo:-<no features>} vs $(basename "$c_so")"
    log="$LOGDIR/test_${combo//,/_}_$(basename "$c_so").log"
    if HDR_BITRATE_C_SO="$c_so" \
       HDR_BITRATE_NO_DEFAULT_FEATURES=1 \
       HDR_BITRATE_FEATURES="$combo" \
       timeout "$TIMEOUT" cargo test --manifest-path "$CRATE/Cargo.toml" \
         --no-default-features ${combo:+--features "$combo"} \
         >"$log" 2>&1; then
      ok "cargo test $label"
    else
      bad "cargo test $label (see $log)"
      grep -E '^(test |error|thread)' "$log" | tail -n 30
    fi
  done
done

# --------------------------------------------------------------------------
# 5. Export parity (nm -D)
# --------------------------------------------------------------------------
note "Export parity: nm -D on the C .so vs the Rust .so"
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  timeout "$TIMEOUT" cargo build --manifest-path "$CRATE/Cargo.toml" --lib \
    --no-default-features ${combo:+--features "$combo"} \
    >"$LOGDIR/build_lib.log" 2>&1
  RUST_SO="$(find "$CRATE/target/debug" -maxdepth 1 -name 'libhdr_bitrate_lib.so' | head -n1)"
  if [ -z "$RUST_SO" ]; then
    bad "no Rust .so produced for features '$label'"
    continue
  fi

  nm -D --defined-only "$C_SO"    | awk '{print $NF}' | sort -u >"$LOGDIR/c.syms"
  nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u >"$LOGDIR/rust.syms"

  missing="$(comm -23 "$LOGDIR/c.syms" "$LOGDIR/rust.syms" \
    | grep -vE '^(_init|_fini|__bss_start|_edata|_end|__gmon_start__|_ITM_(de)?registerTMCloneTable|__cxa_finalize)$')"
  if [ -z "$missing" ]; then
    ok "features '$label': Rust .so exports every C symbol"
  else
    bad "features '$label': Rust .so missing: $(echo "$missing" | tr '\n' ' ')"
  fi
done

note "Summary"
if [ "$fail" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "SOME CHECKS FAILED"
fi
exit "$fail"
