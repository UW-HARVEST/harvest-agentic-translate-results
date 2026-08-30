#!/usr/bin/env bash
# Build the C reference library, enumerate every feature combination declared in
# Cargo.toml, then `cargo check` and `cargo test` each one.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
CSRC="$ROOT/c_src"

echo "== building C reference library =="
cmake -S "$CSRC" -B "$CSRC/build" -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null || exit 1
cmake --build "$CSRC/build" >/dev/null || exit 1
ls -l "$CSRC/build/libdriver.so"

# --- enumerate features -----------------------------------------------------
# Every non-default entry under [features]; "default" is covered separately.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$CRATE/Cargo.toml"
)

# Power set of FEATURES, expressed as comma-separated --features arguments.
COMBOS=("")
n=${#FEATURES[@]}
if (( n > 0 )); then
  COMBOS=()
  for (( mask = 0; mask < (1 << n); mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo
echo "== ${#COMBOS[@]} feature combination(s) discovered =="
for c in "${COMBOS[@]}"; do echo "  - '${c:-<none>}'"; done

# --- check + test each combination -----------------------------------------
status=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  echo
  echo "=============================================================="
  echo "  features: $label"
  echo "=============================================================="

  for phase in check test; do
    echo "-- cargo $phase --"
    if ! timeout 600 cargo "$phase" --manifest-path "$CRATE/Cargo.toml" \
        --no-default-features ${combo:+--features "$combo"} 2>&1 | tail -25; then
      echo "FAILED: cargo $phase (features: $label)"
      status=1
    fi
  done

  # Symbol parity between the C .so and this configuration's Rust .so.
  echo "-- nm -D symbol parity --"
  timeout 600 cargo build --manifest-path "$CRATE/Cargo.toml" \
      --no-default-features ${combo:+--features "$combo"} >/dev/null 2>&1
  c_syms=$(nm -D --defined-only "$CSRC/build/libdriver.so" | awk '{print $NF}' | sort -u)
  r_syms=$(nm -D --defined-only "$CRATE/target/debug/libdriver.so" | awk '{print $NF}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
  if [[ -n "$missing" ]]; then
    echo "FAILED: Rust .so is missing C exports:"; echo "$missing"
    status=1
  else
    echo "ok: Rust .so exports all $(echo "$c_syms" | wc -l) C symbol(s): $(echo $c_syms)"
  fi
done

echo
if (( status == 0 )); then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "SOME CHECKS FAILED"
fi
exit $status
