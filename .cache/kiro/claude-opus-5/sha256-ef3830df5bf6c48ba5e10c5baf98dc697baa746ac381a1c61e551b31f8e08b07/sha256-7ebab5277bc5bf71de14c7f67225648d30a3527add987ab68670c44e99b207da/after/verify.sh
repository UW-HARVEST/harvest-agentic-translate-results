#!/usr/bin/env bash
# Full verification matrix: enumerate every Cargo feature combination, check it
# compiles, then run the C-vs-Rust differential tests through both .so files.
#
# Usage: ./verify.sh          (run from the repo root)
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
LOG=/tmp/verify.log
: > "$LOG"
FAILED=0

run() { # run <label> <cmd...>
  local label="$1"; shift
  printf '  %-52s' "$label"
  if timeout 600 "$@" >>"$LOG" 2>&1; then
    echo "PASS"
  else
    echo "FAIL  (see $LOG)"
    FAILED=1
  fi
}

# --- 1. Enumerate feature combinations ------------------------------------
# Parse the [features] table of Cargo.toml. Every subset of the non-`default`
# features is a valid combination.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$CRATE/Cargo.toml"
)

COMBOS=()
n=${#FEATURES[@]}
for ((mask = 0; mask < (1 << n); mask++)); do
  combo=""
  for ((b = 0; b < n; b++)); do
    if (((mask >> b) & 1)); then combo+="${FEATURES[b]},"; fi
  done
  COMBOS+=("${combo%,}")
done

echo "Features declared: ${n} (${FEATURES[*]:-none})"
echo "Combinations to verify: ${#COMBOS[@]}"

# --- 2. Build the C reference library -------------------------------------
echo
echo "Building C reference library (default configuration)"
run "cmake configure" bash -c "mkdir -p '$ROOT/c_src/build' && cd '$ROOT/c_src/build' && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON"
run "cmake build" cmake --build "$ROOT/c_src/build"
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | sort | head -1)"
echo "  C .so: $C_SO"

# --- 3/4. Per-combination: check, build, symbol diff, differential tests ---
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default/none>}"
  echo
  echo "=== features: $label ==="
  FLAGS=(--no-default-features)
  [[ -n "$combo" ]] && FLAGS+=(--features "$combo")

  run "cargo check" cargo check --manifest-path "$CRATE/Cargo.toml" "${FLAGS[@]}"

  for profile in dev release; do
    PF=()
    [[ "$profile" == release ]] && PF=(--release)
    outdir="$CRATE/target/$([[ $profile == release ]] && echo release || echo debug)"

    run "cargo build ($profile)" cargo build --manifest-path "$CRATE/Cargo.toml" "${FLAGS[@]}" "${PF[@]}"

    # Every symbol the C .so exports must also be exported by the Rust .so.
    RS_SO="$outdir/libnormalize_lib.so"
    run "symbol parity ($profile)" bash -c "
      missing=\$(comm -23 \
        <(nm -D --defined-only '$C_SO'  | awk '{print \$3}' | sort -u) \
        <(nm -D --defined-only '$RS_SO' | awk '{print \$3}' | sort -u))
      if [ -n \"\$missing\" ]; then echo \"missing exports: \$missing\"; exit 1; fi"

    run "cargo test ($profile, C -O0)" cargo test --manifest-path "$CRATE/Cargo.toml" "${FLAGS[@]}" "${PF[@]}"
    C_SO_PATH="${ALT_C_SO:-}" # optional optimized C build
    if [[ -n "$C_SO_PATH" ]]; then
      run "cargo test ($profile, C -O3)" env C_SO_PATH="$C_SO_PATH" \
        cargo test --manifest-path "$CRATE/Cargo.toml" "${FLAGS[@]}" "${PF[@]}"
    fi
  done
done

echo
if ((FAILED)); then
  echo "RESULT: FAILURES present -- inspect $LOG"
  exit 1
fi
echo "RESULT: all ${#COMBOS[@]} feature combination(s) verified against C."
