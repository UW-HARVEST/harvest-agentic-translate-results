#!/usr/bin/env bash
# Phase D driver: rebuild both .so files, diff their exported symbols, and run
# the whole differential suite under EVERY cargo feature combination declared
# in Cargo.toml (plus the no-default-features case).
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CDIR="$ROOT/c_src"
RDIR="$ROOT/translation"
fail=0

echo "=== 1. build the C shared library ==="
mkdir -p "$CDIR/build"
( cd "$CDIR/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
CSO="$CDIR/build/libjansson.so"
echo "    $CSO"

echo
echo "=== 2. enumerate cargo feature combinations ==="
# Extract feature names from [features] in Cargo.toml (excluding "default").
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[ ]*=/{
        sub(/[ ]*=.*/,""); if ($0 != "default") print }' "$RDIR/Cargo.toml"
)
NFEAT=${#FEATURES[@]}
echo "    declared features: ${NFEAT} ${FEATURES[*]:-(none)}"

COMBOS=()
COMBOS+=("--features=")                       # default features
COMBOS+=("--no-default-features")             # nothing
if [ "$NFEAT" -gt 0 ]; then
  # full power set of the declared features, with and without defaults
  total=$(( 1 << NFEAT ))
  for (( m = 0; m < total; m++ )); do
    sel=()
    for (( b = 0; b < NFEAT; b++ )); do
      if (( (m >> b) & 1 )); then sel+=("${FEATURES[$b]}"); fi
    done
    joined=$(IFS=,; echo "${sel[*]:-}")
    COMBOS+=("--features=$joined")
    COMBOS+=("--no-default-features --features=$joined")
  done
fi
# de-duplicate
mapfile -t COMBOS < <(printf '%s\n' "${COMBOS[@]}" | awk '!seen[$0]++')
echo "    combinations to verify: ${#COMBOS[@]}"

for combo in "${COMBOS[@]}"; do
  echo
  echo "=================================================================="
  echo "=== combination: [${combo}]"
  echo "=================================================================="

  echo "--- cargo check"
  # shellcheck disable=SC2086
  if ! ( cd "$RDIR" && timeout 600 cargo check --release $combo 2>&1 | tail -3 ); then
    echo "    cargo check FAILED"; fail=1; continue
  fi

  echo "--- cargo build --release"
  # shellcheck disable=SC2086
  if ! ( cd "$RDIR" && timeout 600 cargo build --release $combo >/dev/null 2>&1 ); then
    echo "    build FAILED"; fail=1; continue
  fi
  RSO="$RDIR/target/release/libjansson.so"

  echo "--- nm -D symbol diff (C vs Rust)"
  nm -D --defined-only "$CSO" | awk '{print $3}' | sort -u > /tmp/pd_c.txt
  nm -D --defined-only "$RSO" | awk '{print $3}' | sort -u > /tmp/pd_r.txt
  missing=$(comm -23 /tmp/pd_c.txt /tmp/pd_r.txt)
  extra=$(comm -13 /tmp/pd_c.txt /tmp/pd_r.txt)
  echo "    C symbols:    $(wc -l < /tmp/pd_c.txt)"
  echo "    Rust symbols: $(wc -l < /tmp/pd_r.txt)"
  if [ -n "$missing" ]; then
    echo "    MISSING IN RUST:"; echo "$missing" | sed 's/^/      /'; fail=1
  else
    echo "    missing in Rust: 0"
  fi
  if [ -n "$extra" ]; then
    echo "    extra in Rust:"; echo "$extra" | sed 's/^/      /'
  else
    echo "    extra in Rust:   0"
  fi

  echo "--- undefined non-libc symbols in the Rust .so"
  und=$(nm -D --undefined-only "$RSO" | awk '{print $2}' | sort -u \
        | grep -v '@GLIBC\|@GCC\|^_ITM_\|^__gmon_start__\|^_Unwind_\|^__cxa_\|^__tls_get_addr' || true)
  if [ -n "$und" ]; then
    echo "    UNRESOLVED:"; echo "$und" | sed 's/^/      /'; fail=1
  else
    echo "    unresolved non-libc: 0"
  fi

  echo "--- cargo test --release (full differential suite)"
  # shellcheck disable=SC2086
  out=$( cd "$RDIR" && timeout 600 cargo test --release $combo 2>&1 )
  echo "$out" | grep -E '^test result:' | sed 's/^/    /'
  if echo "$out" | grep -qE 'FAILED|panicked|error\['; then
    echo "    TESTS FAILED"
    echo "$out" | grep -E 'FAILED|panicked|^error' | head -20 | sed 's/^/      /'
    fail=1
  fi
done

echo
echo "=================================================================="
if [ "$fail" -eq 0 ]; then
  echo "PHASE D: PASS — symbol parity empty and all tests green in every combination"
else
  echo "PHASE D: FAIL"
fi
exit "$fail"
