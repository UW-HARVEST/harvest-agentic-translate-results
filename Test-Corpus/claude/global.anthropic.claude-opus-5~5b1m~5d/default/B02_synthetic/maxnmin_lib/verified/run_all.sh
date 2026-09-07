#!/usr/bin/env bash
# Full verification driver: builds both libraries, diffs the exported symbol
# tables, and runs the whole differential suite against every feature
# combination and both build profiles of the Rust cdylib.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
FAIL=0

step() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------- build C -----
step "Building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | sort | head -n1)"
echo "C  .so: $C_SO"

# ------------------------------------------------- enumerate feature combos ---
# Mechanically extract the crate's features and build their power set. This
# crate declares no [features], so the only combination is the default one.
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
echo "declared non-default features: ${#FEATURES[@]} (${FEATURES[*]-none})"

COMBOS=()
COMBOS+=("--offline")                                    # default features
if [ "${#FEATURES[@]}" -gt 0 ]; then
  COMBOS+=("--offline --no-default-features")
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[i]}")
    done
    COMBOS+=("--offline --no-default-features --features $(
      IFS=,; echo "${sel[*]}")")
  done
fi

# ------------------------------------------------------ per-combo, per-profile -
for combo in "${COMBOS[@]}"; do
  for profile in release debug; do
    step "features: [$combo]  profile: $profile"

    if [ "$profile" = release ]; then
      # shellcheck disable=SC2086
      ( cd "$CRATE" && cargo build --release $combo ) || { FAIL=1; continue; }
      RUST_SO="$CRATE/target/release/libmaxnmin_lib.so"
    else
      # shellcheck disable=SC2086
      ( cd "$CRATE" && cargo build $combo ) || { FAIL=1; continue; }
      RUST_SO="$CRATE/target/debug/libmaxnmin_lib.so"
    fi
    echo "Rust .so: $RUST_SO"

    # --- symbol parity (Phase D) ---
    diff <(nm -D --defined-only "$C_SO"   | awk '{print $3}' | sort) \
         <(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort) \
      && echo "symbol diff: EMPTY (parity OK)" \
      || { echo "SYMBOL PARITY FAILED"; FAIL=1; }

    # --- unresolved non-libc symbols in the Rust .so ---
    undef="$(nm -D --undefined-only "$RUST_SO" | awk '{print $2}' \
             | grep -vE '^(memcpy|memset|memmove|memcmp|_ITM_|__cxa_|__gxx_|__tls_get_addr|_Unwind_|rust_eh_personality|__rust_|abort|__stack_chk)' \
             | grep -E '^(add_node|find_node_by_id|get_children_count|calculate_subtree_sum|process_string|safe_double_to_int|maxnmin)$' || true)"
    if [ -n "$undef" ]; then
      echo "UNRESOLVED project symbols: $undef"; FAIL=1
    else
      echo "unresolved project symbols: none"
    fi

    # --- the differential suite ---
    # shellcheck disable=SC2086
    ( cd "$CRATE" && DIFF_RUST_SO="$RUST_SO" \
        timeout 600 cargo test $combo -- --test-threads 8 ) \
      || { echo "TESTS FAILED for [$combo] $profile"; FAIL=1; }
  done
done

step "SUMMARY"
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$FAIL"
