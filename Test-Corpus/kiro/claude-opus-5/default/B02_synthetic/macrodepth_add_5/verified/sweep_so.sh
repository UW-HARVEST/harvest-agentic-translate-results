#!/usr/bin/env bash
# Run Phases B, C and D for every valid feature combination.
#
# Valid combinations mirror the two CMake cache variables in c_src/CMakeLists.txt:
#   OP     = add | sub | mul
#   REPEAT = 0 .. 7
# giving 24 combinations, plus the two degenerate ones the C also accepts:
#   * no features at all  -> mdmacros.h's `#ifndef` fallbacks (add / 5)
#   * every feature on    -> the documented precedence (mul / 0)
#
# For each combination: build the Rust cdylib + bin, build the matching C .so and
# reference executable, then run all four test targets. Each cargo invocation is
# wrapped in `timeout` so no single step can hang the sweep.
set -u
cd "$(dirname "$0")"
root="$(cd .. && pwd)"

PROFILE_FLAG=""
PROFILE_DIR="debug"
if [[ "${1:-}" == "--release" ]]; then
  PROFILE_FLAG="--release"
  PROFILE_DIR="release"
  shift
fi
FILTER="${1:-}"

TESTS=(phase_b_valid phase_b_exe phase_c_errors phase_d_symbols)

fail=0
declare -a failed_combos=()

run_combo() {
  local label="$1"; shift
  local -a feat_args=("$@")

  local so
  if ! so=$(timeout 600 ./build_so.sh $PROFILE_FLAG "${feat_args[@]}" 2>/tmp/sweep_build.log); then
    echo "BUILD FAIL  $label"
    grep -E '^error' /tmp/sweep_build.log | head -10 | sed 's/^/    /'
    failed_combos+=("$label(build)")
    fail=1
    return
  fi
  if [[ ! -f "$so" ]]; then
    echo "NO CDYLIB   $label ($so missing)"
    failed_combos+=("$label(no-cdylib)")
    fail=1
    return
  fi

  local combo_ok=1
  for t in "${TESTS[@]}"; do
    if [[ -n "$FILTER" && "$t" != *"$FILTER"* ]]; then continue; fi
    if MD_RUST_SO="$so" timeout 600 cargo test $PROFILE_FLAG "${feat_args[@]}" --test "$t" \
         >/tmp/sweep_test.log 2>&1; then
      printf '  ok   %-16s %s\n' "$t" "$label"
    else
      printf '  FAIL %-16s %s\n' "$t" "$label"
      grep -E '^(test .*FAILED|thread .* panicked|assertion|  left:| right:|\[)' \
        /tmp/sweep_test.log | head -20 | sed 's/^/       /'
      combo_ok=0
      fail=1
    fi
  done
  [[ $combo_ok -eq 1 ]] || failed_combos+=("$label")
}

# Make sure every C reference artifact exists up front (the tests can also build
# them on demand, but doing it once keeps the per-combo output clean).
if [[ ! -f "$root/cbuild/libcdriver_add_5.so" ]]; then
  "$root/build_c.sh" >/dev/null || { echo "C reference build failed"; exit 1; }
fi

echo "=== 24 OP x REPEAT combinations ==="
for OP in add sub mul; do
  for R in 0 1 2 3 4 5 6 7; do
    echo "--- $OP / $R ---"
    run_combo "$OP/$R" --no-default-features --features "$OP,$R"
  done
done

echo "=== degenerate combinations ==="
echo "--- no features (mdmacros.h #ifndef fallbacks -> add/5) ---"
run_combo "no-features" --no-default-features
echo "--- default features (cmake defaults -> add/5) ---"
run_combo "default"
echo "--- all features (documented precedence -> mul/0) ---"
run_combo "all-features" --all-features

echo
if [[ $fail -eq 0 ]]; then
  echo "SWEEP PASSED: all 26 configurations green across ${#TESTS[@]} test targets"
else
  echo "SWEEP FAILED in: ${failed_combos[*]}"
fi
exit $fail
