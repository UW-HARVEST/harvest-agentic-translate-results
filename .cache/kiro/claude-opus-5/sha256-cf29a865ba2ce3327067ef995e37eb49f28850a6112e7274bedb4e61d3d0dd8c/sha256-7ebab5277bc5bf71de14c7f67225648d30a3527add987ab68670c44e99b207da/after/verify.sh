#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth for every build-time
# configuration.
#
#   * enumerates every feature combination declared in translation/Cargo.toml
#     (the crate currently declares none, so the matrix is the single default
#     configuration, exercised both with and without --no-default-features)
#   * cargo check for each combination
#   * builds the C shared library
#   * builds the Rust cdylib and the differential-test helper
#   * compares exported symbols and runs the differential tests
#
# Usage: ./verify.sh [--quick]     (--quick skips the multi-million-input fuzz)

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
C_SRC="$ROOT/c_src"
RUST="$ROOT/translation"
QUICK=0
[[ "${1:-}" == "--quick" ]] && QUICK=1

fail=0
step() { printf '\n=== %s ===\n' "$1"; }
ok()   { printf '  [ok]   %s\n' "$1"; }
bad()  { printf '  [FAIL] %s\n' "$1"; fail=1; }

# ---------------------------------------------------------------- feature matrix
step "Enumerating feature combinations"
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /=/   { split($0, a, "="); gsub(/[ \t"]/, "", a[1]);
                      if (a[1] != "default" && a[1] != "") print a[1] }
  ' "$RUST/Cargo.toml"
)
n=${#FEATURES[@]}
echo "  declared non-default features: ${n} (${FEATURES[*]:-none})"

COMBOS=()
if (( n == 0 )); then
  COMBOS=("")
else
  for (( mask = 0; mask < (1 << n); mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi
echo "  feature combinations to verify: ${#COMBOS[@]}"

# ------------------------------------------------------------------- cargo check
step "cargo check for every combination"
for combo in "${COMBOS[@]}"; do
  label="--no-default-features --features '${combo}'"
  if timeout 600 cargo check --manifest-path "$RUST/Cargo.toml" \
       --all-targets --no-default-features ${combo:+--features "$combo"} \
       >/tmp/check.log 2>&1; then
    ok "cargo check $label"
  else
    bad "cargo check $label"; tail -n 25 /tmp/check.log
  fi
done
# The default configuration too, in case a default feature exists.
if timeout 600 cargo check --manifest-path "$RUST/Cargo.toml" --all-targets \
     >/tmp/check.log 2>&1; then
  ok "cargo check (default features)"
else
  bad "cargo check (default features)"; tail -n 25 /tmp/check.log
fi

# ------------------------------------------------------------------- build the C
step "Building the C shared library"
mkdir -p "$C_SRC/build"
if (cd "$C_SRC/build" \
      && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/cmake.log 2>&1 \
      && timeout 600 cmake --build . >>/tmp/cmake.log 2>&1); then
  ok "libdriver.so built at c_src/build/libdriver.so"
else
  bad "C build failed"; tail -n 25 /tmp/cmake.log; exit 1
fi

# ---------------------------------------------------- per-combination verification
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  step "Verifying combination: $label"

  if timeout 600 cargo build --manifest-path "$RUST/Cargo.toml" --release \
       --lib --examples --no-default-features ${combo:+--features "$combo"} \
       >/tmp/build.log 2>&1; then
    ok "cargo build --release"
  else
    bad "cargo build --release ($label)"; tail -n 25 /tmp/build.log; continue
  fi

  # Exported-symbol parity: every symbol the C .so defines, the Rust .so must too.
  c_syms=$(nm -D --defined-only "$C_SRC/build/libdriver.so" | awk '{print $NF}' | sort -u)
  r_syms=$(nm -D --defined-only "$RUST/target/release/libdriver.so" | awk '{print $NF}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
  if [[ -z "$missing" ]]; then
    ok "symbol parity ($(echo "$c_syms" | wc -l) C symbol(s) all present)"
  else
    bad "Rust .so missing exported symbols: $(echo "$missing" | tr '\n' ' ')"
  fi

  # Discover every integration-test target so new files are picked up
  # automatically; --quick drops the multi-million-input sweeps.
  targets=()
  for t in "$RUST"/tests/*.rs; do
    name=$(basename "$t" .rs)
    (( QUICK )) && [[ "$name" == "driver_fuzz" ]] && continue
    targets+=(--test "$name")
  done
  if timeout 600 cargo test --manifest-path "$RUST/Cargo.toml" --release \
       --no-default-features ${combo:+--features "$combo"} "${targets[@]}" \
       >/tmp/test.log 2>&1; then
    ok "cargo test ($(grep -c ' \.\.\. ok$' /tmp/test.log) tests passed across ${#targets[@]} targets)"
  else
    bad "cargo test ($label)"; grep -E "^test |panicked|mismatch|input [0-9]" /tmp/test.log | head -n 40
  fi
done

step "Result"
if (( fail )); then
  echo "  FAILURES present"
  exit 1
fi
echo "  all configurations verified"
