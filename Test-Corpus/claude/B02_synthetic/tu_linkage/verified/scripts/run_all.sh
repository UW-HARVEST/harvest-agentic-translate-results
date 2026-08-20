#!/usr/bin/env bash
# Full verification run.
#
# 1. Enumerate every cargo feature combination mechanically from Cargo.toml.
#    (This crate has no [features] section, so the set is exactly one element:
#    the empty combination == default. The loop is written generically anyway.)
# 2. For each combination: cargo check, cargo build --release, cargo test.
# 3. Re-run the whole differential suite against the *debug* cdylib, where
#    Rust's integer overflow checks are on (any non-wrapping arithmetic panics).
# 4. Symbol parity + the shell level CLI differential script.
set -uo pipefail
HERE="$(cd "$(dirname "$0")/.." && pwd)"
cd "$HERE"
fails=0

# ---- 0. build every artifact (cmake exe, both C .so, release + debug Rust) ---
bash scripts/build_all.sh || { echo "FAIL build_all"; exit 1; }

# ---- 1. feature combinations ------------------------------------------------
mapfile -t FEATURES < <(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/ {inf=0}
  inf && /^[A-Za-z0-9_-]+[ \t]*=/ {sub(/[ \t]*=.*/,""); print}
' Cargo.toml)
echo "declared cargo features: ${#FEATURES[@]} (${FEATURES[*]:-none})"

COMBOS=("")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  total=$((1 << n))
  COMBOS=()
  for ((mask = 0; mask < total; mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( (mask >> i) & 1 )); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "feature combinations to verify: ${#COMBOS[@]}"

LOG="${TMPDIR:-/tmp}/driver_verify.$$.log"
for combo in "${COMBOS[@]}"; do
  label=${combo:-<none>}
  echo
  echo "############ feature combination: $label ############"
  for step in "check" "build --release" "test"; do
    # shellcheck disable=SC2086
    if timeout 600 cargo $step --no-default-features ${combo:+--features "$combo"} >"$LOG" 2>&1; then
      echo "ok   cargo $step ($label)"
    else
      echo "FAIL cargo $step ($label)"; fails=$((fails+1)); tail -30 "$LOG"
    fi
    grep -E "^(test |test result|error|warning: unused)" "$LOG" | grep -v "^test .* ok$" || true
    grep -c "^test .* \.\.\. ok$" "$LOG" | sed 's/^/     passing tests: /'
  done
done

# ---- 3. same suite against the debug cdylib (overflow checks enabled) -------
echo
echo "############ differential suite vs the DEBUG cdylib ############"
timeout 600 cargo build >/dev/null 2>&1
if DRIVER_RUST_SO=target/debug/libdriver.so DRIVER_RUST_EXE=target/debug/driver \
     timeout 600 cargo test >"$LOG" 2>&1; then
  echo "ok   debug-cdylib test run"
else
  echo "FAIL debug-cdylib test run"; fails=$((fails+1)); tail -30 "$LOG"
fi
grep -E "^test result" "$LOG" || true
grep -c "^test .* \.\.\. ok$" "$LOG" | sed 's/^/     passing tests: /' 

# ---- 4. symbol parity + CLI script -----------------------------------------
echo
echo "############ symbol parity ############"
bash scripts/symbol_parity.sh || fails=$((fails+1))
bash scripts/symbol_parity.sh c_src/build/libdriver_c.so target/release/libdriver.so || fails=$((fails+1))

echo
echo "############ shell CLI differential ############"
bash scripts/cli_diff.sh || fails=$((fails+1))

echo
if [ "$fails" = 0 ]; then
  echo "ALL VERIFICATION STEPS PASSED"
else
  echo "$fails verification step(s) FAILED"
fi
exit "$fails"
