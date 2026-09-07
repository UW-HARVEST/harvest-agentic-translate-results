#!/usr/bin/env bash
# Full verification run: builds both shared objects, diffs their exported
# symbols, and runs the differential suite under EVERY feature combination
# declared in Cargo.toml (plus --no-default-features and --all-features).
#
# Usage:  cd translation && ./run_all.sh
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$CRATE_DIR")"
FAIL=0

step() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------- build C -----
step "Building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }

C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | sort | head -n1)"
[ -n "$C_SO" ] || { echo "no C .so produced"; exit 1; }
echo "C  .so: $C_SO"

# ------------------------------------------------------- enumerate features ---
step "Enumerating feature combinations from Cargo.toml"
# Extract the feature names from the [features] table, ignoring "default".
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$CRATE_DIR/Cargo.toml"
)

if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "Cargo.toml declares no [features]; the complete set of feature"
  echo "combinations is the single default (empty) one."
  # Still exercise both spellings so the claim is demonstrated, not asserted.
  COMBOS=( "<default>" "--no-default-features" "--all-features" )
else
  echo "features: ${FEATURES[*]}"
  COMBOS=( "<default>" "--no-default-features" "--all-features" )
  # Full power set of the declared features, with default features off.
  n=${#FEATURES[@]}
  for (( mask = 1; mask < (1 << n); mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( mask & (1 << i) )); then combo+="${FEATURES[i]},"; fi
    done
    COMBOS+=( "--no-default-features --features ${combo%,}" )
  done
fi
printf 'combinations to verify: %d\n' "${#COMBOS[@]}"

# ------------------------------------------------------- per-combo cargo ------
cd "$CRATE_DIR"
for combo in "${COMBOS[@]}"; do
  if [ "$combo" = "<default>" ]; then flags=(); label="(default)"; else
    # shellcheck disable=SC2206
    flags=( $combo ); label="$combo"
  fi

  step "cargo check  $label"
  timeout 600 cargo check "${flags[@]}" 2>&1 | tail -n 5 || FAIL=1

  step "cargo build --release  $label"
  timeout 600 cargo build --release "${flags[@]}" 2>&1 | tail -n 3 || FAIL=1

  RUST_SO="$CRATE_DIR/target/release/libldexp_q2_lib.so"
  [ -f "$RUST_SO" ] || { echo "no Rust .so produced for $label"; FAIL=1; continue; }

  step "symbol diff  $label"
  cdefs=$(nm -D --defined-only "$C_SO"   | awk '{print $NF}' | sort -u)
  rdefs=$(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u)
  missing=$(comm -23 <(printf '%s\n' "$cdefs") <(printf '%s\n' "$rdefs"))
  printf 'C exports (%s):\n%s\n' "$(printf '%s\n' "$cdefs" | grep -c .)" "$cdefs"
  if [ -n "$missing" ]; then
    echo "MISSING from the Rust .so:"; printf '%s\n' "$missing"; FAIL=1
  else
    echo "symbol diff EMPTY -- full parity"
  fi

  step "cargo test --release  $label"
  C_SO_PATH="$C_SO" RUST_SO_PATH="$RUST_SO" \
    timeout 600 cargo test --release "${flags[@]}" -- --nocapture 2>&1 \
    | grep -E 'running|test result|FAILED|panicked|probes' || FAIL=1
  # propagate the real test exit status
  C_SO_PATH="$C_SO" RUST_SO_PATH="$RUST_SO" \
    timeout 600 cargo test --release "${flags[@]}" >/dev/null 2>&1 || FAIL=1
done

step "RESULT"
if [ "$FAIL" -eq 0 ]; then
  echo "ALL PHASES PASSED for all ${#COMBOS[@]} feature combination(s)."
else
  echo "FAILURES detected -- see above."
fi
exit "$FAIL"
