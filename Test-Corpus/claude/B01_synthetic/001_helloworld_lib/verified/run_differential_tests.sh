#!/usr/bin/env bash
# Full differential verification: build the C .so, build the Rust cdylib for
# every feature combination and profile, and run Phases B, C and D against both.
#
#   ./run_differential_tests.sh
set -uo pipefail
cd "$(dirname "$0")"

CARGO_FLAGS=(--offline)
TEST_ARGS=(--nocapture --test-threads=1)

echo "################ building the C shared library ################"
if [ ! -f c_src/build/libhello.so ]; then
  (mkdir -p c_src/build && cd c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null) || { echo "C build FAILED"; exit 1; }
fi
ls -l c_src/build/libhello.so

# --- feature powerset, extracted mechanically from Cargo.toml ----------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0}
       f && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/{
         split($0,a,"="); gsub(/[[:space:]]/,"",a[1]);
         if (a[1] != "default") print a[1] }' Cargo.toml
)
N=${#FEATURES[@]}
echo
echo "features declared: $N ${FEATURES[*]:-(none)} -> $((1 << N)) combination(s) + default"

COMBOS=()
for ((mask = 0; mask < (1 << N); mask++)); do
  c=""
  for ((i = 0; i < N; i++)); do
    (((mask >> i) & 1)) && c="${c:+$c,}${FEATURES[$i]}"
  done
  COMBOS+=("$c")
done

FAIL=0
run_combo() {           # $1 = human label, rest = cargo feature flags
  local label="$1"; shift
  local flags=("$@")
  echo
  echo "################ feature combination: $label ################"

  for profile in dev release; do
    if [ "$profile" = release ]; then
      timeout 600 cargo build "${CARGO_FLAGS[@]}" --release "${flags[@]}" || {
        echo "FAIL: release build [$label]"; FAIL=1; return; }
    else
      timeout 600 cargo build "${CARGO_FLAGS[@]}" "${flags[@]}" || {
        echo "FAIL: dev build [$label]"; FAIL=1; return; }
    fi
  done
  ls -l target/debug/libhello.so target/release/libhello.so

  local out rc
  for t in phase_b phase_c phase_d; do
    echo "---- $t [$label] ----"
    out=$(timeout 600 cargo test "${CARGO_FLAGS[@]}" "${flags[@]}" --test "$t" \
            -- "${TEST_ARGS[@]}" 2>&1)
    rc=$?
    printf '%s\n' "$out" | grep -E '^\s*\[|^===|test result'
    if [ "$rc" != 0 ]; then
      echo "FAIL: $t [$label] (exit $rc)"
      printf '%s\n' "$out" | tail -40
      FAIL=1
    fi
  done
}

for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    run_combo "<none> (--no-default-features)" --no-default-features
  else
    run_combo "$combo" --no-default-features --features "$combo"
  fi
done
run_combo "default features"

echo
if [ "$FAIL" = 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED (Phases B, C, D)"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit $FAIL
