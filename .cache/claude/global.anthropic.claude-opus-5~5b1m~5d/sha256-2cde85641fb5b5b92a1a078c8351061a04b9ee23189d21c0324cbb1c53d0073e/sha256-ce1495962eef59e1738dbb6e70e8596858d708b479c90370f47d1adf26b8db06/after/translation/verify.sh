#!/usr/bin/env bash
# Full verification sweep: symbol parity + every phase, across every cargo
# feature combination and both profiles.
#
# Feature combinations are extracted from Cargo.toml rather than hard-coded, so
# adding a [features] table automatically widens the sweep.
set -uo pipefail
cd "$(dirname "$0")" || exit 1

fail=0
run() { # run <label> <cmd...>
  local label="$1"; shift
  printf '\n=== %s ===\n' "$label"
  if timeout 600 "$@"; then
    printf '  PASS: %s\n' "$label"
  else
    printf '  FAIL: %s\n' "$label"
    fail=1
  fi
}

# --- 1. build the C ground truth ------------------------------------------
printf '=== building C reference ===\n'
mkdir -p ../c_src/build
( cd ../c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "  FAIL: C build"; exit 1; }
echo "  ok: ../c_src/build/libdriver.so"

# --- 2. enumerate feature combinations -----------------------------------
# Every declared feature name (empty if there is no [features] table).
features=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{print $1}' Cargo.toml)

combos=("__default__")
if [ -n "$features" ]; then
  # shellcheck disable=SC2206
  farr=($features)
  n=${#farr[@]}
  # power set of the declared features, driven with --no-default-features
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=""
    for ((b = 0; b < n; b++)); do
      (((mask >> b) & 1)) && sel="$sel,${farr[$b]}"
    done
    combos+=("${sel#,}")
  done
fi
printf 'feature combinations to verify: %s\n' "${combos[*]}"

# --- 3. cargo check, then the full suite, per combo x per profile --------
for combo in "${combos[@]}"; do
  for profile in dev release; do
    args=(test)
    [ "$profile" = release ] && args+=(--release)
    label="features=$combo profile=$profile"
    if [ "$combo" != "__default__" ]; then
      args+=(--no-default-features)
      [ -n "$combo" ] && args+=(--features "$combo")
    fi
    run "cargo check ($label)" cargo check "${args[@]:1}" --tests
    run "cargo test  ($label)" cargo "${args[@]}"
  done
done

# --- 4. the symbol diff, independently of the test harness ---------------
printf '\n=== nm -D symbol diff (C vs Rust) ===\n'
c_syms=$(nm -D --defined-only ../c_src/build/libdriver.so | awk '{print $3}' | sort)
r_syms=$(nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort)
diff <(echo "$c_syms") <(echo "$r_syms") && echo "  PASS: symbol sets identical" || {
  echo "  FAIL: symbol sets differ"; fail=1; }

printf '\n========================================\n'
[ "$fail" -eq 0 ] && echo "ALL VERIFICATION PASSED" || echo "VERIFICATION FAILED"
exit "$fail"
